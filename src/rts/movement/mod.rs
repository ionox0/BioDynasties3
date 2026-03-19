//! Unit movement system.
//!
//! Owns: `Movement.target_position / current_velocity`, `PathfindingState.path_index`
//! Inputs: `MovementTargetEvent`, `StopMovementEvent`
//! Outputs: `UnitArrivedEvent` (fired when a unit exhausts its path)
//!
//! System order each Update frame:
//!   stop_unit_movement → apply_movement_targets → request_paths → poll_path_tasks → move_units → sync_position_component

pub mod events;
pub mod formation;
pub mod formation_events;
pub mod pathfinding;
pub mod unit_commands;

use bevy::prelude::*;
use hashbrown::HashMap;
use crate::core::components::*;
use crate::core::constants::movement as mc;
use crate::core::GameSet;
use crate::world::static_terrain::{StaticTerrainHeights, TerrainNormals};
use self::events::{MovementTargetEvent, StopMovementEvent, UnitArrivedEvent};
use self::pathfinding::{request_paths, poll_path_tasks};
use self::pathfinding::systems::PathTask;

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MovementTargetEvent>()
            .add_event::<StopMovementEvent>()
            .add_event::<UnitArrivedEvent>()
            .add_systems(
                Update,
                (
                    stop_unit_movement,
                    apply_movement_targets,
                    request_paths,
                    poll_path_tasks,
                    move_units,
                    sync_position_component,
                )
                    .chain()
                    .in_set(GameSet::RtsUpdate),
            );
    }
}

// ---------------------------------------------------------------------------
// Target management
// ---------------------------------------------------------------------------

/// Sole writer of `Movement.target_position` — applies `MovementTargetEvent`.
fn apply_movement_targets(
    mut commands: Commands,
    mut units: Query<(&mut Movement, &mut PathfindingState)>,
    mut events: EventReader<MovementTargetEvent>,
) {
    for ev in events.read() {
        let Ok((mut mv, mut pf)) = units.get_mut(ev.entity) else { continue };
        commands.entity(ev.entity).remove::<PathTask>();
        mv.target_position = Some(ev.target_position);
        pf.path.clear();
        pf.path_index = 0;
        pf.last_pathfinding_failure = f32::NEG_INFINITY;
        pf.last_failed_target = None;
    }
}

/// Clears movement when `StopMovementEvent` fires (sent by combat system).
fn stop_unit_movement(
    mut units: Query<(&mut Movement, &mut PathfindingState)>,
    mut events: EventReader<StopMovementEvent>,
) {
    for ev in events.read() {
        let Ok((mut mv, mut pf)) = units.get_mut(ev.entity) else { continue };
        mv.target_position = None;
        mv.current_velocity = Vec3::ZERO;
        pf.path.clear();
        pf.path_index = 0;
    }
}

// ---------------------------------------------------------------------------
// Movement
// ---------------------------------------------------------------------------

struct StepCtx<'a> {
    terrain: &'a StaticTerrainHeights,
    dt: f32,
}

/// Give up waiting for a path after this many seconds of consecutive failures.
const PATHFINDING_GIVE_UP_SECS: f32 = 6.0;
/// Frames a unit must be stuck at the same waypoint before skipping ahead.
const STUCK_FRAME_THRESHOLD: u32 = 60;

/// Moves all units each frame, following their A* path.
/// Fires `UnitArrivedEvent` when a unit exhausts its path and clears its target.
///
/// `stuck_tracker` maps each entity to the number of consecutive frames it has
/// remained on the same waypoint. When the threshold is reached the unit skips
/// two waypoints, keeping its original destination intact.
fn move_units(
    mut units: Query<(Entity, &mut Transform, &mut Movement, &mut PathfindingState, &RTSUnit)>,
    terrain: Res<StaticTerrainHeights>,
    terrain_normals: Res<TerrainNormals>,
    time: Res<Time>,
    mut arrived: EventWriter<UnitArrivedEvent>,
    mut stuck_tracker: Local<HashMap<Entity, u32>>,
) {
    let dt = time.delta_secs().min(0.033);
    let ctx = StepCtx { terrain: &terrain, dt };
    for (entity, mut tf, mut mv, mut pf, rts_unit) in units.iter_mut() {
        if mv.target_position.is_none() {
            mv.current_velocity = Vec3::ZERO;
            snap_to_terrain(&mut tf, ctx.terrain);
            stuck_tracker.remove(&entity);
            continue;
        }
        // Give up if pathfinding has been failing too long with no path to follow.
        let now = time.elapsed_secs();
        if pf.path.is_empty()
            && pf.last_pathfinding_failure.is_finite()
            && now - pf.last_pathfinding_failure > PATHFINDING_GIVE_UP_SECS
        {
            mv.target_position = None;
            mv.current_velocity = Vec3::ZERO;
            pf.path_index = 0;
            pf.last_pathfinding_failure = f32::NEG_INFINITY;
            arrived.send(UnitArrivedEvent { entity });
            stuck_tracker.remove(&entity);
            continue;
        }
        let prev_index = pf.path_index;
        if let Some(dir) = step_path(&mut tf, &mut mv, &mut pf, &ctx) {
            update_rotation(&mut tf, dir, rts_unit, &terrain_normals, dt);
        }
        // target_position cleared inside step_path means path was exhausted — unit arrived.
        if mv.target_position.is_none() {
            arrived.send(UnitArrivedEvent { entity });
            stuck_tracker.remove(&entity);
            continue;
        }
        // Track stuck state: only when we have a live path and didn't just advance.
        if !pf.path.is_empty() {
            if pf.path_index > prev_index {
                stuck_tracker.remove(&entity);
            } else {
                let count = stuck_tracker.entry(entity).or_insert(0);
                *count += 1;
                if *count >= STUCK_FRAME_THRESHOLD {
                    // Skip 2 waypoints ahead without changing the destination.
                    pf.path_index = (pf.path_index + 2).min(pf.path.len());
                    stuck_tracker.remove(&entity);
                }
            }
        } else {
            // Waiting for a path — don't count as stuck.
            stuck_tracker.remove(&entity);
        }
    }
}

/// Ground units: follow A* path waypoints. Returns None while waiting for path.
fn step_path(
    tf: &mut Transform,
    mv: &mut Movement,
    pf: &mut PathfindingState,
    ctx: &StepCtx,
) -> Option<Vec3> {
    // Skip past waypoints we've already reached.
    while pf.path_index < pf.path.len() {
        if flat_dist(tf.translation, pf.path[pf.path_index]) >= mc::ARRIVAL_THRESHOLD {
            break;
        }
        pf.path_index += 1;
    }

    if pf.path_index >= pf.path.len() {
        // Path exhausted → destination reached.
        if !pf.path.is_empty() {
            mv.target_position = None;
            mv.current_velocity = Vec3::ZERO;
            pf.path.clear();
            pf.path_index = 0;
        }
        // path is empty → waiting for pathfinding system to compute one.
        return None;
    }

    let wp = pf.path[pf.path_index];
    let dir = flat_dir(tf.translation, wp);
    mv.current_velocity = dir * mv.max_speed;
    tf.translation.x += mv.current_velocity.x * ctx.dt;
    tf.translation.z += mv.current_velocity.z * ctx.dt;
    snap_to_terrain(tf, ctx.terrain);
    clamp_to_map(tf);
    Some(dir)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn flat_dist(a: Vec3, b: Vec3) -> f32 {
    let dx = a.x - b.x;
    let dz = a.z - b.z;
    (dx * dx + dz * dz).sqrt()
}

fn flat_dir(from: Vec3, to: Vec3) -> Vec3 {
    Vec3::new(to.x - from.x, 0.0, to.z - from.z).normalize_or_zero()
}

fn snap_to_terrain(tf: &mut Transform, terrain: &StaticTerrainHeights) {
    tf.translation.y =
        terrain.get_height(tf.translation.x, tf.translation.z) + mc::DEFAULT_SPAWN_HEIGHT;
}

fn clamp_to_map(tf: &mut Transform) {
    let b = mc::MAP_BOUNDARY;
    tf.translation.x = tf.translation.x.clamp(-b, b);
    tf.translation.z = tf.translation.z.clamp(-b, b);
}

/// Rotates the unit to face its direction of travel, tilted to match the terrain normal.
fn update_rotation(
    tf: &mut Transform,
    dir: Vec3,
    rts_unit: &RTSUnit,
    normals: &TerrainNormals,
    dt: f32,
) {
    if dir.length_squared() <= mc::DIRECTION_THRESHOLD * mc::DIRECTION_THRESHOLD {
        return;
    }
    // Fourmi/CairnsBirdwing GLBs face backward — negate the formula to compensate.
    // Dragonfly GLB is rotated 90° CCW from the forward direction.
    let yaw = match rts_unit.unit_type.as_ref() {
        Some(UnitType::Fourmi | UnitType::CairnsBirdwing) => {
            Quat::from_rotation_y(-dir.x.atan2(-dir.z))
        }
        Some(UnitType::Dragonfly) => {
            Quat::from_rotation_y(dir.x.atan2(dir.z) + std::f32::consts::FRAC_PI_2)
        }
        _ => Quat::from_rotation_y(dir.x.atan2(dir.z)),
    };
    let normal = normals.get_normal(tf.translation.x, tf.translation.z);
    let tilt = Quat::from_rotation_arc(Vec3::Y, normal);
    let turn = (mc::MAX_TURN_SPEED * dt * 10.0).min(1.0);
    tf.rotation = tf.rotation.slerp(tilt * yaw, turn);
}

/// Keeps `Position` in sync with `Transform` so spatial grid readers stay current.
fn sync_position_component(mut units: Query<(&Transform, &mut Position)>) {
    for (tf, mut pos) in units.iter_mut() {
        pos.translation = tf.translation;
    }
}
