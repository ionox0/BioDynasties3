//! Unit movement system.
//!
//! Owns: `Movement.target_position / current_velocity`, `PathfindingState.path_index`
//! Inputs: `MovementTargetEvent`, `StopMovementEvent`
//! Outputs: `UnitArrivedEvent` (fired when a unit exhausts its path)
//!
//! System order each Update frame:
//!   apply_movement_targets → stop_unit_movement → pathfinding_system → move_units → sync_position_component

pub mod events;
pub mod formation;
pub mod formation_events;
pub mod pathfinding;
pub mod unit_commands;
pub mod unstuck;

use bevy::prelude::*;
use crate::core::components::*;
use crate::core::constants::movement as mc;
use crate::world::static_terrain::StaticTerrainHeights;
use self::events::{MovementTargetEvent, StopMovementEvent, UnitArrivedEvent};
use self::pathfinding::pathfinding_system;
use self::unstuck::{add_stuck_detection, unstuck_system};

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MovementTargetEvent>()
            .add_event::<StopMovementEvent>()
            .add_event::<UnitArrivedEvent>()
            .add_systems(
                Update,
                (
                    add_stuck_detection,
                    apply_movement_targets,
                    stop_unit_movement,
                    pathfinding_system,
                    move_units,
                    sync_position_component,
                    unstuck_system,
                )
                    .chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Target management
// ---------------------------------------------------------------------------

/// Sole writer of `Movement.target_position` — applies `MovementTargetEvent`.
fn apply_movement_targets(
    mut units: Query<(&mut Movement, &mut PathfindingState)>,
    mut events: EventReader<MovementTargetEvent>,
) {
    for ev in events.read() {
        let Ok((mut mv, mut pf)) = units.get_mut(ev.entity) else { continue };
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

/// Moves all units each frame, following their A* path.
/// Fires `UnitArrivedEvent` when a unit exhausts its path and clears its target.
fn move_units(
    mut units: Query<(Entity, &mut Transform, &mut Movement, &mut PathfindingState, &RTSUnit)>,
    terrain: Res<StaticTerrainHeights>,
    time: Res<Time>,
    mut arrived: EventWriter<UnitArrivedEvent>,
) {
    let ctx = StepCtx { terrain: &terrain, dt: time.delta_secs().min(0.033) };
    for (entity, mut tf, mut mv, mut pf, rts_unit) in units.iter_mut() {
        if mv.target_position.is_none() {
            mv.current_velocity = Vec3::ZERO;
            snap_to_terrain(&mut tf, ctx.terrain);
            continue;
        }
        if let Some(dir) = step_path(&mut tf, &mut mv, &mut pf, &ctx) {
            update_rotation(&mut tf, dir, rts_unit, ctx.dt);
        }
        // target_position cleared inside step_path means path was exhausted — unit arrived.
        if mv.target_position.is_none() {
            arrived.send(UnitArrivedEvent { entity });
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
        // path is empty → waiting for pathfinding_system to compute one.
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
    let floor = terrain.get_height(tf.translation.x, tf.translation.z) + mc::DEFAULT_SPAWN_HEIGHT;
    if tf.translation.y < floor {
        tf.translation.y = floor;
    }
}

fn clamp_to_map(tf: &mut Transform) {
    let b = mc::MAP_BOUNDARY;
    tf.translation.x = tf.translation.x.clamp(-b, b);
    tf.translation.z = tf.translation.z.clamp(-b, b);
}

/// Rotates the unit to face its direction of travel.
fn update_rotation(tf: &mut Transform, dir: Vec3, rts_unit: &RTSUnit, dt: f32) {
    if dir.length_squared() <= mc::DIRECTION_THRESHOLD * mc::DIRECTION_THRESHOLD {
        return;
    }
    // Ant/butterfly GLBs face backward — negate the formula to compensate.
    let target_rot = match rts_unit.unit_type.as_ref() {
        Some(UnitType::WorkerAnt | UnitType::ScoutAnt) => {
            Quat::from_rotation_y(-dir.x.atan2(-dir.z))
        }
        _ => Quat::from_rotation_y(dir.x.atan2(dir.z)),
    };
    let turn = (mc::MAX_TURN_SPEED * dt * 10.0).min(1.0);
    tf.rotation = tf.rotation.slerp(target_rot, turn);
}

/// Keeps `Position` in sync with `Transform` so spatial grid readers stay current.
fn sync_position_component(mut units: Query<(&Transform, &mut Position)>) {
    for (tf, mut pos) in units.iter_mut() {
        pos.translation = tf.translation;
    }
}
