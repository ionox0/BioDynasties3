//! Unit movement system.
//!
//! Owns: `Movement.target_position / current_velocity`, `Route`
//! Inputs: `MovementTargetEvent`, `StopMovementEvent`
//! Outputs: `UnitArrivedEvent` (fired when a unit reaches its target)
//!
//! System order each Update frame:
//!   stop_unit_movement → apply_movement_targets → poll_flow_fields → move_units → sync_position_component

pub mod events;
pub mod formation;
pub mod formation_events;
pub mod navigation;
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
use self::navigation::flow_field::has_clear_line;
use self::navigation::nav_grid::CELL_SIZE;
use self::navigation::systems::{poll_flow_fields, NavGridResource, PendingFlowFields, RouteRequest};

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
    query: Query<'w, 's, (&'static Transform, &'static mut Movement, &'static mut Route)>,
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
        let Ok((tf, mut mv, mut route)) = units.query.get_mut(ev.entity) else { continue };
        mv.target_position = Some(ev.target_position);
        *route = Route::Direct;
        let Some(grid) = grid.as_ref() else { continue };
        let start = tf.translation;
        let is_short_clear = flat_dist(start, ev.target_position) <= LINE_CHECK_RANGE
            && has_clear_line(&grid.0, start, ev.target_position);
        if is_short_clear {
            continue;
        }
        *route = Route::Pending;
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
    mut units: Query<(&mut Movement, &mut Route)>,
    mut events: EventReader<StopMovementEvent>,
) {
    for ev in events.read() {
        let Ok((mut mv, mut route)) = units.get_mut(ev.entity) else { continue };
        mv.target_position = None;
        mv.current_velocity = Vec3::ZERO;
        *route = Route::Direct;
    }
}

// ---------------------------------------------------------------------------
// Movement
// ---------------------------------------------------------------------------

type MovingUnitData = (
    Entity,
    &'static mut Transform,
    &'static mut Movement,
    &'static Route,
    &'static RTSUnit,
    Option<&'static Climbing>,
    Option<&'static CombatState>,
);

/// Every unit that can move.
#[derive(SystemParam)]
pub(crate) struct MovingUnits<'w, 's> {
    query: Query<'w, 's, MovingUnitData>,
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
    elapsed: f32,
}

/// Peak heading wobble either side of the route, in radians.
const WANDER_ANGLE: f32 = 0.6;
/// How strongly each unit favours its own branch of the flow field (0 = none, below 1).
const BRANCH_PREFERENCE: f32 = 0.98;
/// Each unit's fixed pace differs from its max speed by up to this fraction either way.
const SPEED_VARIATION: f32 = 0.2;
/// While moving, each unit's speed surges and slows by up to this fraction either way.
const SPEED_SURGE: f32 = 0.35;

/// Per-unit variation that keeps crowds from moving in lockstep straight lines.
struct Wander {
    /// Heading offset this frame, in radians.
    angle: f32,
    /// Multiplier on the unit's max speed.
    speed: f32,
    /// Slowly turning bias toward one of the flow field's downhill branches.
    preference: Vec2,
}

impl Wander {
    /// Deterministic per entity: each unit gets its own phase, rhythm and pace.
    fn of(entity: Entity, elapsed: f32) -> Self {
        let seed = |salt: u32| (entity.index().wrapping_add(salt).wrapping_mul(2_654_435_761) >> 8) as f32 / (1 << 24) as f32;
        let phase = elapsed * (1.0 + seed(1) * 1.5) + seed(2) * std::f32::consts::TAU;
        let angle = WANDER_ANGLE * (phase.sin() + 0.5 * (phase * 2.3).sin()) / 1.5;
        let pace = SPEED_VARIATION * (seed(3) * 2.0 - 1.0);
        let surge = SPEED_SURGE * (elapsed * (0.7 + seed(4)) + seed(5) * std::f32::consts::TAU).sin();
        let turn = elapsed * (0.15 + seed(6) * 0.3) + seed(7) * std::f32::consts::TAU;
        let preference = Vec2::from_angle(turn) * BRANCH_PREFERENCE;
        Self { angle, speed: 1.0 + pace + surge, preference }
    }
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
    let ctx = StepCtx {
        terrain: &terrain.heights,
        normals: &terrain.normals,
        dt: time.delta_secs().min(0.033),
        elapsed: time.elapsed_secs(),
    };
    units.query.par_iter_mut().for_each_init(
        || arrivals.borrow_local_mut(),
        |local, (entity, mut tf, mut mv, route, rts_unit, climbing, combat_state)| {
            if step_unit(&mut tf, &mut mv, route, &ctx, Wander::of(entity, ctx.elapsed)) {
                local.push(entity);
            }
            let normal = climbing.map_or_else(
                || ctx.normals.get_normal(tf.translation.x, tf.translation.z),
                |c| c.normal,
            );
            let heading = mv.current_velocity.try_normalize().or_else(|| direction_to_target(&tf, combat_state));
            update_rotation(&mut tf, heading, rts_unit, normal, ctx.dt);
        },
    );
    arrived.send_batch(arrivals.drain().map(|entity| UnitArrivedEvent { entity }));
}

/// Advances one unit toward its target. Returns true on the frame it arrives.
fn step_unit(tf: &mut Transform, mv: &mut Movement, route: &Route, ctx: &StepCtx, wander: Wander) -> bool {
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
    let Some(dir) = steering_direction(tf.translation, target, distance, route, wander.preference) else {
        snap_to_terrain(tf, ctx.terrain);
        return false;
    };
    mv.current_velocity = Quat::from_rotation_y(wander.angle) * dir * mv.max_speed * wander.speed;
    tf.translation.x += mv.current_velocity.x * ctx.dt;
    tf.translation.z += mv.current_velocity.z * ctx.dt;
    snap_to_terrain(tf, ctx.terrain);
    clamp_to_map(tf);
    false
}

/// Direction to travel this frame, or `None` while the unit's flow field is still building.
fn steering_direction(pos: Vec3, target: Vec3, distance: f32, route: &Route, preference: Vec2) -> Option<Vec3> {
    match route {
        Route::Pending => None,
        Route::Direct => Some(flat_dir(pos, target)),
        Route::Field(_) if distance < DIRECT_STEER_RANGE => Some(flat_dir(pos, target)),
        Route::Field(field) => Some(field.direction(pos, preference).unwrap_or_else(|| flat_dir(pos, target))),
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

/// Flat direction from a stationary unit to its combat target, so it faces what it attacks.
fn direction_to_target(tf: &Transform, combat_state: Option<&CombatState>) -> Option<Vec3> {
    let offset = combat_state?.target_position? - tf.translation;
    Vec3::new(offset.x, 0.0, offset.z).try_normalize()
}

/// Tilts the unit to match the surface `normal`, turning to face `heading` if it has one.
fn update_rotation(tf: &mut Transform, heading: Option<Vec3>, rts_unit: &RTSUnit, normal: Vec3, dt: f32) {
    let yaw = match heading {
        Some(dir) => facing_yaw(dir, rts_unit),
        // Idle: keep the current facing, with the old tilt removed.
        None => Quat::from_rotation_arc(tf.rotation * Vec3::Y, Vec3::Y) * tf.rotation,
    };
    let tilt = Quat::from_rotation_arc(Vec3::Y, normal);
    let turn = (mc::MAX_TURN_SPEED * dt * 10.0).min(1.0);
    tf.rotation = tf.rotation.slerp(tilt * yaw, turn);
}

/// Yaw that faces `dir`, corrected for models authored facing another way.
fn facing_yaw(dir: Vec3, rts_unit: &RTSUnit) -> Quat {
    match rts_unit.unit_type.as_ref() {
        // CairnsBirdwing GLB faces backward.
        Some(UnitType::CairnsBirdwing) => Quat::from_rotation_y(-dir.x.atan2(-dir.z)),
        // Dragonfly GLB is rotated 90° CCW from the forward direction.
        Some(UnitType::Dragonfly) => Quat::from_rotation_y(dir.x.atan2(dir.z) + std::f32::consts::FRAC_PI_2),
        _ => Quat::from_rotation_y(dir.x.atan2(dir.z)),
    }
}

/// Keeps `Position` in sync with `Transform` so proximity-grid readers stay current.
fn sync_position_component(mut units: Query<(&Transform, &mut Position)>) {
    for (tf, mut pos) in units.iter_mut() {
        pos.translation = tf.translation;
    }
}
