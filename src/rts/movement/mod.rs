//! Unit movement system.
//!
//! Owns: `Movement.target_position / current_velocity`, `PathfindingState.route`
//! Inputs: `MovementTargetEvent`, `StopMovementEvent`
//! Outputs: `UnitArrivedEvent` (fired when a unit reaches its target)
//!
//! System order each Update frame:
//!   stop_unit_movement → apply_movement_targets → poll_flow_fields → move_units → sync_position_component

pub mod events;
pub mod formation;
pub mod formation_events;
pub mod pathfinding;
pub mod unit_commands;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::utils::Parallel;
use hashbrown::HashMap;
use tracing::instrument;
use crate::core::components::*;
use crate::core::constants::movement as mc;
use crate::core::GameSet;
use crate::world::static_terrain::{StaticTerrainHeights, TerrainNormals};
use self::events::{MovementTargetEvent, StopMovementEvent, UnitArrivedEvent};
use self::pathfinding::flow_field::has_clear_line;
use self::pathfinding::grid::CELL_SIZE;
use self::pathfinding::systems::{poll_flow_fields, NavGridResource, PendingFlowFields, RouteRequest};

/// Units this close to their target steer straight at it instead of following the field.
const DIRECT_STEER_RANGE: f32 = CELL_SIZE * 2.0;
/// Longest straight line checked for a direct route (longer orders always use a flow field).
const LINE_CHECK_RANGE: f32 = 1000.0;

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<MovementTargetEvent>()
            .add_event::<StopMovementEvent>()
            .add_event::<UnitArrivedEvent>()
            .init_resource::<PendingFlowFields>()
            .add_systems(
                Update,
                (
                    stop_unit_movement,
                    apply_movement_targets,
                    poll_flow_fields,
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

/// Units receiving move orders.
#[derive(SystemParam)]
pub(crate) struct MoveOrderUnits<'w, 's> {
    query: Query<'w, 's, (&'static Transform, &'static mut Movement, &'static mut PathfindingState)>,
}

/// Sole writer of `Movement.target_position` — applies `MovementTargetEvent`.
/// Units with a clear straight line go directly; the rest of each order shares one flow field.
#[instrument(skip_all)]
fn apply_movement_targets(
    mut units: MoveOrderUnits,
    mut events: EventReader<MovementTargetEvent>,
    grid: Option<Res<NavGridResource>>,
    mut pending: ResMut<PendingFlowFields>,
) {
    let mut orders: HashMap<IVec2, (Vec3, Vec<RouteRequest>)> = HashMap::new();
    for ev in events.read() {
        let Ok((tf, mut mv, mut pf)) = units.query.get_mut(ev.entity) else { continue };
        mv.target_position = Some(ev.target_position);
        pf.route = Route::Direct;
        let Some(grid) = grid.as_ref() else { continue };
        let start = tf.translation;
        let is_short_clear = flat_dist(start, ev.target_position) <= LINE_CHECK_RANGE
            && has_clear_line(&grid.0, start, ev.target_position);
        if is_short_clear {
            continue;
        }
        pf.route = Route::Pending;
        let key = grid.0.cell_of(ev.group_goal).unwrap_or(IVec2::MIN);
        orders
            .entry(key)
            .or_insert_with(|| (ev.group_goal, Vec::new()))
            .1
            .push(RouteRequest { entity: ev.entity, start, target: ev.target_position });
    }
    let Some(grid) = grid else { return };
    for (goal, requests) in orders.into_values() {
        pending.spawn(&grid.0, goal, requests);
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
        pf.route = Route::Direct;
    }
}

// ---------------------------------------------------------------------------
// Movement
// ---------------------------------------------------------------------------

/// Every unit that can move.
#[derive(SystemParam)]
pub(crate) struct MovingUnits<'w, 's> {
    query: Query<'w, 's, (Entity, &'static mut Transform, &'static mut Movement, &'static PathfindingState, &'static RTSUnit)>,
}

/// Terrain lookups movement needs: surface height and slope.
#[derive(SystemParam)]
pub(crate) struct TerrainSurface<'w> {
    heights: Res<'w, StaticTerrainHeights>,
    normals: Res<'w, TerrainNormals>,
}

struct StepCtx<'a> {
    terrain: &'a StaticTerrainHeights,
    normals: &'a TerrainNormals,
    dt: f32,
}

/// Moves all units in parallel. Fires `UnitArrivedEvent` when a unit reaches its target.
#[instrument(skip_all)]
fn move_units(
    mut units: MovingUnits,
    terrain: TerrainSurface,
    time: Res<Time>,
    mut arrived: EventWriter<UnitArrivedEvent>,
    mut arrivals: Local<Parallel<Vec<Entity>>>,
) {
    let ctx = StepCtx { terrain: &terrain.heights, normals: &terrain.normals, dt: time.delta_secs().min(0.033) };
    units.query.par_iter_mut().for_each_init(
        || arrivals.borrow_local_mut(),
        |local, (entity, mut tf, mut mv, pf, rts_unit)| {
            if step_unit(&mut tf, &mut mv, pf, &ctx) {
                local.push(entity);
            }
            if let Some(dir) = mv.current_velocity.try_normalize() {
                update_rotation(&mut tf, dir, rts_unit, ctx.normals, ctx.dt);
            }
        },
    );
    arrived.send_batch(arrivals.drain().map(|entity| UnitArrivedEvent { entity }));
}

/// Advances one unit toward its target. Returns true on the frame it arrives.
fn step_unit(tf: &mut Transform, mv: &mut Movement, pf: &PathfindingState, ctx: &StepCtx) -> bool {
    mv.current_velocity = Vec3::ZERO;
    let Some(target) = mv.target_position else {
        snap_to_terrain(tf, ctx.terrain);
        return false;
    };
    let distance = flat_dist(tf.translation, target);
    if distance < mc::ARRIVAL_THRESHOLD {
        mv.target_position = None;
        return true;
    }
    let Some(dir) = steering_direction(tf.translation, target, distance, &pf.route) else {
        snap_to_terrain(tf, ctx.terrain);
        return false;
    };
    mv.current_velocity = dir * mv.max_speed;
    tf.translation.x += mv.current_velocity.x * ctx.dt;
    tf.translation.z += mv.current_velocity.z * ctx.dt;
    snap_to_terrain(tf, ctx.terrain);
    clamp_to_map(tf);
    false
}

/// Direction to travel this frame, or `None` while the unit's flow field is still building.
fn steering_direction(pos: Vec3, target: Vec3, distance: f32, route: &Route) -> Option<Vec3> {
    match route {
        Route::Pending => None,
        Route::Direct => Some(flat_dir(pos, target)),
        Route::Field(_) if distance < DIRECT_STEER_RANGE => Some(flat_dir(pos, target)),
        Route::Field(field) => Some(field.direction(pos).unwrap_or_else(|| flat_dir(pos, target))),
    }
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
    // CairnsBirdwing GLB faces backward — negate the formula to compensate.
    // Dragonfly GLB is rotated 90° CCW from the forward direction.
    let yaw = match rts_unit.unit_type.as_ref() {
        Some(UnitType::CairnsBirdwing) => {
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
