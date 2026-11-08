//! Unit separation system.
//!
//! Owns: separation-driven corrections to `Transform.translation`.
//! Runs after `GameSet::RtsUpdate` so movement always resolves first.
//!
//! Design:
//! - Flow fields handle routing around impassable terrain.
//! - Building/environment separation always applies (units must not clip geometry).
//! - Idle units are pushed apart fully; moving units only gently, so separation
//!   spreads a column out without fighting flow-field steering.
//! - Units squeezed deep into each other crawl over: the smaller one rides up the
//!   back of a grounded neighbour (never more than one body high), tilts to its
//!   slope, and slides off the far side.

use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use hashbrown::HashMap;

use crate::core::components::*;
use crate::core::resources::UnitProximity;
use crate::core::proximity_grid::ProximityCell;
use crate::core::GameSet;

/// Push strength per unit of overlap (world units out per world unit in).
const SEPARATION_STRENGTH: f32 = 1.0;
/// Extra gap kept between two idle units (on top of their summed radii).
const UNIT_BUFFER: f32 = 0.5;
/// Extra gap kept between a unit and a building or environment object.
const BUILDING_BUFFER: f32 = 1.5;
/// Fraction of the idle push applied to moving units.
const MOVING_SEPARATION: f32 = 0.25;
/// Units climb only when closer than this fraction of their summed radii.
const CLIMB_OVERLAP: f32 = 0.6;
/// Height of a unit's back above its feet, as a fraction of its collision radius.
const BODY_HEIGHT_RATIO: f32 = 0.9;
/// How quickly a unit eases up onto a back, and back down, per second.
const RISE_RATE: f32 = 12.0;
const FALL_RATE: f32 = 6.0;
/// Speed at which riders slide off the back they're on, world units per second.
const SLIDE_OFF_SPEED: f32 = 20.0;
/// How far riders lean with the slope of the back.
const CLIMB_TILT: f32 = 1.5;
/// Below this height a unit counts as standing on the ground.
const GROUNDED: f32 = 0.01;

/// A unit's state at the start of separation, read by its neighbours.
#[derive(Clone, Copy)]
struct Body {
    pos: Vec3,
    radius: f32,
    /// Height above the terrain from climbing (last frame).
    climb: f32,
    idle: bool,
}

/// Where a climber rests on the back beneath it.
struct Perch {
    /// Height above the climber's terrain.
    height: f32,
    /// Flat direction from the back's centre out to the climber.
    out: Vec3,
    /// How far out from the centre, 0 (top) to 1 (edge).
    offset: f32,
}

// --- SystemParam query structs ---

type SeparationData = (
    Entity,
    &'static mut Transform,
    &'static CollisionRadius,
    &'static Movement,
    Option<&'static Climbing>,
);
/// Units that move (excludes buildings and environment objects, which are also obstacles).
type Mobile = (With<RTSUnit>, Without<Building>, Without<EnvironmentObject>);

/// RTSUnit entities eligible for separation.
#[derive(SystemParam)]
struct MobileUnits<'w, 's> {
    query: Query<'w, 's, SeparationData, Mobile>,
}

#[derive(SystemParam)]
struct SpatiallyTrackedUnits<'w, 's> {
    query: Query<
        'w,
        's,
        (
            Entity,
            &'static Transform,
            &'static CollisionRadius,
            &'static mut ProximityCellTracker,
        ),
        With<RTSUnit>,
    >,
}

type StaticBuilding = (With<Building>, Without<Movement>);
type StaticEnvironment = (With<EnvironmentObject>, Without<Movement>);

/// Buildings and environment objects units are pushed away from.
#[derive(SystemParam)]
struct StaticObstacles<'w, 's> {
    buildings: Query<'w, 's, (&'static Position, &'static CollisionRadius), StaticBuilding>,
    env: Query<'w, 's, (&'static Transform, &'static CollisionRadius), StaticEnvironment>,
}

// --- Plugin ---

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnitProximity>().add_systems(
            Update,
            (update_unit_proximity, separate_units)
                .chain()
                .after(GameSet::RtsUpdate),
        );
    }
}

// --- Systems ---

/// Incrementally syncs unit positions into the proximity grid.
fn update_unit_proximity(
    mut grids: ResMut<UnitProximity>,
    mut units: SpatiallyTrackedUnits,
) {
    use std::collections::HashSet;
    let live: HashSet<Entity> = units.query.iter().map(|(e, _, _, _)| e).collect();
    let stale: Vec<Entity> = grids
        .grid
        .entity_positions
        .keys()
        .copied()
        .filter(|e| !live.contains(e))
        .collect();
    for entity in stale {
        grids.grid.remove_item(entity);
    }
    for (entity, transform, radius, mut grid_pos) in units.query.iter_mut() {
        let coord =
            ProximityCell::from_world_pos(transform.translation, grids.grid.cell_size);
        if grid_pos.dirty || grid_pos.last_cell != Some(coord) {
            grids.grid.update_entity(
                entity,
                transform.translation,
                radius.radius,
            );
            grid_pos.last_cell = Some(coord);
            grid_pos.dirty = false;
        }
    }
}

/// Pushes overlapping units apart and lifts units that crawl over each other.
/// Direct position correction — no events, no velocity changes.
fn separate_units(
    grids: Res<UnitProximity>,
    mut units: MobileUnits,
    obstacles: StaticObstacles,
    time: Res<Time>,
    mut commands: Commands,
) {
    let bodies: HashMap<Entity, Body> = units
        .query
        .iter()
        .map(|(entity, tf, radius, mv, climbing)| {
            let climb = climbing.map_or(0.0, |c| c.height);
            let idle = mv.target_position.is_none();
            (entity, Body { pos: tf.translation, radius: radius.radius, climb, idle })
        })
        .collect();
    let dt = time.delta_secs();
    for (entity, mut tf, ..) in units.query.iter_mut() {
        let Some(&body) = bodies.get(&entity) else { continue };
        let neighbors = neighbors_of(entity, &body, &grids, &bodies);
        let perch = perch_on(entity, &body, &neighbors);
        let climb = ease_climb(body.climb, perch.as_ref().map_or(0.0, |p| p.height), dt);
        let slide = perch.as_ref().map_or(Vec3::ZERO, |p| p.out * SLIDE_OFF_SPEED * dt);
        let push = obstacle_separation(body.pos, body.radius, &obstacles)
            + crowd_push(&body, &neighbors)
            + slide;
        tf.translation += Vec3::new(push.x, climb, push.z);
        update_climbing(&mut commands, entity, body.climb > 0.0, climb, perch);
    }
}

// --- Helpers ---

fn obstacle_separation(pos: Vec3, radius: f32, obstacles: &StaticObstacles) -> Vec3 {
    let mut push = Vec3::ZERO;
    for (bpos, bradius) in obstacles.buildings.iter() {
        push += push_away(pos, radius, bpos.translation, bradius.radius, BUILDING_BUFFER);
    }
    for (etf, eradius) in obstacles.env.iter() {
        push += push_away(pos, radius, etf.translation, eradius.radius, BUILDING_BUFFER);
    }
    push
}

/// Mobile units near `body`, with their current state.
fn neighbors_of(
    entity: Entity,
    body: &Body,
    grids: &UnitProximity,
    bodies: &HashMap<Entity, Body>,
) -> Vec<(Entity, Body)> {
    grids
        .grid
        .query_nearby_entities(body.pos, body.radius * 5.0, Some(entity))
        .into_iter()
        .filter_map(|(other, _, _)| bodies.get(&other).map(|b| (other, *b)))
        .collect()
}

/// Push away from crowding neighbours: full strength when idle, gentle while moving.
fn crowd_push(body: &Body, neighbors: &[(Entity, Body)]) -> Vec3 {
    let strength = if body.idle { 1.0 } else { MOVING_SEPARATION };
    let push: Vec3 = neighbors
        .iter()
        .map(|(_, other)| push_away(body.pos, body.radius, other.pos, other.radius, UNIT_BUFFER))
        .sum();
    push * strength
}

/// The highest back `body` is squeezed onto, if any. Only the smaller unit climbs
/// (ties broken by entity), and only onto a grounded unit, so stacks stay one high.
/// Each back is a dome: the closer to its centre, the higher the climber sits.
fn perch_on(entity: Entity, body: &Body, neighbors: &[(Entity, Body)]) -> Option<Perch> {
    neighbors
        .iter()
        .filter(|(other, below)| below.climb < GROUNDED && (body.radius, entity) < (below.radius, *other))
        .filter_map(|(_, below)| {
            let away = Vec3::new(body.pos.x - below.pos.x, 0.0, body.pos.z - below.pos.z);
            let offset = away.length() / (CLIMB_OVERLAP * (body.radius + below.radius));
            let dome = below.radius * BODY_HEIGHT_RATIO * (1.0 - offset * offset).max(0.0).sqrt();
            let height = (below.pos.y - body.pos.y + dome).max(0.0);
            (offset < 1.0).then_some(Perch { height, out: away.normalize_or_zero(), offset })
        })
        .max_by(|a, b| a.height.total_cmp(&b.height))
}

/// Moves the climb height toward `target`: quick to step up, slower to step down.
fn ease_climb(current: f32, target: f32, dt: f32) -> f32 {
    let rate = if target > current { RISE_RATE } else { FALL_RATE };
    current + (target - current) * (1.0 - (-rate * dt).exp())
}

/// Adds or refreshes `Climbing` while the unit is off the ground; removes it on landing.
fn update_climbing(commands: &mut Commands, entity: Entity, was_climbing: bool, climb: f32, perch: Option<Perch>) {
    if climb < GROUNDED {
        if was_climbing {
            commands.entity(entity).remove::<Climbing>();
        }
        return;
    }
    // Lean with the dome's slope, more the higher up the unit has climbed.
    let lean = perch.map_or(Vec3::ZERO, |p| p.out * p.offset * (climb / p.height).min(1.0) * CLIMB_TILT);
    commands.entity(entity).try_insert(Climbing { height: climb, normal: (Vec3::Y + lean).normalize() });
}

/// Returns how far `pos` should be pushed away from `other_pos` to resolve overlap.
/// Returns `Vec3::ZERO` if there is no overlap.
fn push_away(
    pos: Vec3,
    radius: f32,
    other_pos: Vec3,
    other_radius: f32,
    buffer: f32,
) -> Vec3 {
    let diff = Vec3::new(pos.x - other_pos.x, 0.0, pos.z - other_pos.z);
    let dist = diff.length();
    let min_dist = radius + other_radius + buffer;
    if dist >= min_dist || dist < 0.001 {
        return Vec3::ZERO;
    }
    diff.normalize() * (min_dist - dist) * SEPARATION_STRENGTH
}
