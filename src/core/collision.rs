//! Collision avoidance for RTS units.
//!
//! Calculation and application are separated into two systems:
//! - `calc_collision_avoidance` — reads only, fires `CollisionAvoidanceEvent`
//! - `apply_collision_avoidance` — sole writer of Movement/Transform for collision response

use crate::core::components::*;
use crate::core::resources::SpatialGrids;
use crate::core::spatial_grid::GridCoord;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use constants::*;

// --- SystemParam query structs ---

#[derive(SystemParam)]
struct SpatiallyTrackedUnits<'w, 's> {
    query: Query<'w, 's, (Entity, &'static Transform, &'static CollisionRadius, &'static mut SpatialGridPosition), With<RTSUnit>>,
}

#[derive(SystemParam)]
struct MovingUnits<'w, 's> {
    query: Query<'w, 's, (Entity, &'static Transform, &'static Movement, &'static CollisionRadius), With<RTSUnit>>,
}

#[derive(SystemParam)]
struct StaticObstacles<'w, 's> {
    buildings: Query<'w, 's, (&'static Position, &'static CollisionRadius), (With<Building>, Without<Movement>)>,
    env: Query<'w, 's, (&'static Transform, &'static CollisionRadius), (With<EnvironmentObject>, Without<Movement>)>,
}

// --- Plugin ---

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialGrids>()
            .add_event::<CollisionAvoidanceEvent>()
            .add_systems(
            Update,
            (
                spatial_grid_update_system,
                calc_collision_avoidance,
                apply_collision_avoidance,
            )
                .chain(),
        );
    }
}

// --- Events ---

/// Fired per unit per frame when collision avoidance forces are non-trivial.
/// Applied by `apply_collision_avoidance`, which is the sole writer of
/// Movement.current_velocity and Transform.translation for collision response.
#[derive(Event)]
pub struct CollisionAvoidanceEvent {
    pub entity: Entity,
    pub velocity_delta: Vec3,
    pub position_push: Vec3,
}

// --- Internal types ---

struct CollisionInfo {
    direction: Vec3,
    overlap: f32,
    distance: f32,
    min_distance: f32,
}

/// Snapshot of a unit's relevant collision state for helper functions.
struct UnitState {
    entity: Entity,
    pos: Vec3,
    radius: f32,
    velocity: Vec3,
    max_speed: f32,
}

// --- Constants ---

mod constants {
    pub const MAX_DELTA_TIME: f32 = 0.1;
    pub const MOVING_VELOCITY_THRESHOLD: f32 = 2.0;
    pub const BUILDING_BUFFER: f32 = 1.5;
    pub const UNIT_BUFFER: f32 = 0.8;
    pub const OVERLAP_THRESHOLD: f32 = 0.3;
    pub const AVOIDANCE_FORCE_BASE: f32 = 40.0;
    pub const SERIOUS_OVERLAP_RATIO: f32 = 0.85;
    pub const PUSH_MULTIPLIER: f32 = 0.3;
    pub const UNIT_OVERLAP_THRESHOLD: f32 = 0.8;
    pub const CONVERGENCE_THRESHOLD: f32 = 2.0;
    pub const CONVERGING_FORCE: f32 = 3.0;
    pub const MOVING_FORCE: f32 = 1.0;
    pub const STATIONARY_FORCE: f32 = 0.5;
    pub const SERIOUS_UNIT_OVERLAP_RATIO: f32 = 0.6;
    pub const GENTLE_PUSH_MULTIPLIER: f32 = 0.05;
    pub const MIN_AVOIDANCE_MAGNITUDE: f32 = 0.1;
    pub const VELOCITY_ADJUSTMENT_FACTOR: f32 = 0.3;
    pub const MAX_SPEED_MULTIPLIER: f32 = 1.2;
    pub const STATIONARY_ADJUSTMENT_FACTOR: f32 = 0.5;
    pub const VELOCITY_DAMPING: f32 = 0.98;
}

// --- Systems ---

/// Incrementally syncs moving units into the spatial grid.
fn spatial_grid_update_system(mut grids: ResMut<SpatialGrids>, mut units: SpatiallyTrackedUnits) {
    let live: HashSet<Entity> = units.query.iter().map(|(e, _, _, _)| e).collect();
    let stale: Vec<Entity> = grids
        .entity_grid
        .entity_positions
        .keys()
        .copied()
        .filter(|e| !live.contains(e))
        .collect();
    for entity in stale {
        grids.entity_grid.remove_item(entity);
    }

    for (entity, transform, radius, mut grid_pos) in units.query.iter_mut() {
        let coord = GridCoord::from_world_pos(transform.translation, grids.entity_grid.cell_size);
        if grid_pos.dirty || grid_pos.last_grid_coord != Some(coord) {
            grids.entity_grid.update_entity(entity, transform.translation, radius.radius);
            grid_pos.last_grid_coord = Some(coord);
            grid_pos.dirty = false;
        }
    }
}

/// Calculates collision avoidance for all units. Read-only — fires `CollisionAvoidanceEvent`.
fn calc_collision_avoidance(
    units: MovingUnits,
    obstacles: StaticObstacles,
    grids: Res<SpatialGrids>,
    time: Res<Time>,
    mut events: EventWriter<CollisionAvoidanceEvent>,
) {
    let dt = time.delta_secs().min(MAX_DELTA_TIME);
    let velocities: HashMap<Entity, Vec3> =
        units.query.iter().map(|(e, _, m, _)| (e, m.current_velocity)).collect();

    for (entity, transform, movement, radius) in units.query.iter() {
        let unit = UnitState {
            entity,
            pos: transform.translation,
            radius: radius.radius,
            velocity: movement.current_velocity,
            max_speed: movement.max_speed,
        };
        let (force, push) = unit_collision_forces(&unit, &obstacles, &grids, &velocities);
        let is_moving = unit.velocity.length() > MOVING_VELOCITY_THRESHOLD;
        let (velocity_delta, position_push) = resolve_avoidance(force, push, &unit, is_moving, dt);

        if velocity_delta != Vec3::ZERO || position_push != Vec3::ZERO {
            events.send(CollisionAvoidanceEvent { entity, velocity_delta, position_push });
        }
    }
}

/// Applies `CollisionAvoidanceEvent` to Movement and Transform.
/// Sole writer of Movement.current_velocity and Transform.translation for collision response.
fn apply_collision_avoidance(
    mut events: EventReader<CollisionAvoidanceEvent>,
    mut units: Query<(&mut Movement, &mut Transform)>,
) {
    for event in events.read() {
        let Ok((mut movement, mut transform)) = units.get_mut(event.entity) else {
            continue;
        };
        movement.current_velocity += event.velocity_delta;
        transform.translation += event.position_push;
    }
}

// --- Helpers ---

/// Accumulates total avoidance (force, push) for one unit against all obstacles and nearby units.
fn unit_collision_forces(
    unit: &UnitState,
    obstacles: &StaticObstacles,
    grids: &SpatialGrids,
    velocities: &HashMap<Entity, Vec3>,
) -> (Vec3, Vec3) {
    let mut force = Vec3::ZERO;
    let mut push = Vec3::ZERO;

    for (pos, radius) in obstacles.buildings.iter() {
        let (f, p) = obstacle_forces(unit.pos, unit.radius, pos.translation, radius.radius, BUILDING_BUFFER);
        force += f;
        push += p;
    }
    for (tf, radius) in obstacles.env.iter() {
        let (f, p) = obstacle_forces(unit.pos, unit.radius, tf.translation, radius.radius, BUILDING_BUFFER);
        force += f;
        push += p;
    }

    let nearby = grids.entity_grid.query_nearby_entities(unit.pos, unit.radius * 5.0, Some(unit.entity));
    for (other, other_pos, other_radius) in nearby {
        let other_vel = velocities.get(&other).copied().unwrap_or(Vec3::ZERO);
        let (f, p) = unit_unit_forces(unit, other_pos, other_radius, other_vel);
        force += f;
        push += p;
    }

    (force, push)
}

/// Avoidance force and immediate push-out for a unit vs. a static obstacle.
fn obstacle_forces(pos: Vec3, radius: f32, obs_pos: Vec3, obs_radius: f32, buffer: f32) -> (Vec3, Vec3) {
    let Some(col) = check_collision(pos, radius, obs_pos, obs_radius, buffer) else {
        return (Vec3::ZERO, Vec3::ZERO);
    };
    if col.overlap <= OVERLAP_THRESHOLD {
        return (Vec3::ZERO, Vec3::ZERO);
    }
    let magnitude = (col.overlap / col.min_distance).min(1.0);
    let force = col.direction * magnitude * AVOIDANCE_FORCE_BASE;
    let push = if col.distance < col.min_distance * SERIOUS_OVERLAP_RATIO {
        col.direction * col.overlap * PUSH_MULTIPLIER
    } else {
        Vec3::ZERO
    };
    (force, push)
}

/// Avoidance force and push-out for a unit vs. another unit.
fn unit_unit_forces(unit: &UnitState, other_pos: Vec3, other_radius: f32, other_vel: Vec3) -> (Vec3, Vec3) {
    let Some(col) = check_collision(unit.pos, unit.radius, other_pos, other_radius, UNIT_BUFFER) else {
        return (Vec3::ZERO, Vec3::ZERO);
    };
    if col.overlap <= UNIT_OVERLAP_THRESHOLD {
        return (Vec3::ZERO, Vec3::ZERO);
    }
    let is_moving = unit.velocity.length() > MOVING_VELOCITY_THRESHOLD;
    let is_converging = (unit.velocity - other_vel).length() > CONVERGENCE_THRESHOLD;
    let multiplier = if is_converging { CONVERGING_FORCE } else if is_moving { MOVING_FORCE } else { STATIONARY_FORCE };
    let magnitude = (col.overlap / col.min_distance).min(1.0);
    let force = col.direction * magnitude * multiplier;
    let push = if col.overlap > col.min_distance * SERIOUS_UNIT_OVERLAP_RATIO {
        col.direction * col.overlap * GENTLE_PUSH_MULTIPLIER
    } else {
        Vec3::ZERO
    };
    (force, push)
}

/// Converts accumulated avoidance force into (velocity_delta, position_push) for the event.
fn resolve_avoidance(force: Vec3, push: Vec3, unit: &UnitState, is_moving: bool, dt: f32) -> (Vec3, Vec3) {
    if force.length() <= MIN_AVOIDANCE_MAGNITUDE {
        return (Vec3::ZERO, push);
    }
    let velocity_delta = if is_moving {
        let new_vel = unit.velocity + force * dt * VELOCITY_ADJUSTMENT_FACTOR;
        let max_speed = unit.max_speed * MAX_SPEED_MULTIPLIER;
        let clamped = if new_vel.length() > max_speed { new_vel.normalize() * max_speed } else { new_vel };
        clamped * VELOCITY_DAMPING - unit.velocity
    } else {
        unit.velocity * (VELOCITY_DAMPING - 1.0)
    };
    let position_push = if is_moving {
        push
    } else {
        push + force.normalize() * dt * STATIONARY_ADJUSTMENT_FACTOR
    };
    (velocity_delta, position_push)
}

/// Returns collision geometry if two circles overlap, `None` otherwise.
fn check_collision(pos1: Vec3, radius1: f32, pos2: Vec3, radius2: f32, buffer: f32) -> Option<CollisionInfo> {
    let distance = pos1.distance(pos2);
    let min_distance = radius1 + radius2 + buffer;
    if distance < min_distance && distance > 0.001 {
        Some(CollisionInfo {
            direction: (pos1 - pos2).normalize(),
            overlap: min_distance - distance,
            distance,
            min_distance,
        })
    } else {
        None
    }
}
