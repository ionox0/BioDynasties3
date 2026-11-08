//! Unit separation system.
//!
//! Owns: separation-driven corrections to `Transform.translation`.
//! Runs after `GameSet::RtsUpdate` so movement always resolves first.
//!
//! Design:
//! - Flow fields handle routing around impassable terrain.
//! - Building/environment separation always applies (units must not clip geometry).
//! - Unit-unit separation only applies to **idle** units (no active target_position).
//!   Moving units pass through each other briefly; they spread out once they arrive.
//!   This prevents separation forces from fighting flow-field steering.

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

// --- SystemParam query structs ---

type SeparationData = (Entity, &'static mut Transform, &'static CollisionRadius, &'static Movement);
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

/// Pushes overlapping units apart. Direct position correction — no events, no velocity changes.
/// Unit-unit separation is only applied to idle units to avoid fighting path-following.
fn separate_units(
    grids: Res<UnitProximity>,
    mut units: MobileUnits,
    obstacles: StaticObstacles,
) {
    // Snapshot which units are idle so unit-unit separation knows who to push.
    let idle: std::collections::HashSet<Entity> = units
        .query
        .iter()
        .filter(|(_, _, _, mv)| mv.target_position.is_none())
        .map(|(e, _, _, _)| e)
        .collect();

    // First pass (read-only): compute per-entity separation push.
    let pushes: HashMap<Entity, Vec3> = units
        .query
        .iter()
        .filter_map(|(entity, tf, radius, _)| {
            let mut push = obstacle_separation(tf.translation, radius.radius, &obstacles);
            if idle.contains(&entity) {
                push += unit_separation(entity, tf.translation, radius.radius, &grids);
            }
            if push != Vec3::ZERO { Some((entity, push)) } else { None }
        })
        .collect();

    // Second pass (mutable): apply pushes.
    for (entity, mut tf, _, _) in units.query.iter_mut() {
        let Some(&push) = pushes.get(&entity) else { continue };
        tf.translation.x += push.x;
        tf.translation.z += push.z;
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

fn unit_separation(entity: Entity, pos: Vec3, radius: f32, grids: &UnitProximity) -> Vec3 {
    grids
        .grid
        .query_nearby_entities(pos, radius * 5.0, Some(entity))
        .into_iter()
        .map(|(_, other_pos, other_radius)| {
            push_away(pos, radius, other_pos, other_radius, UNIT_BUFFER)
        })
        .fold(Vec3::ZERO, |a, b| a + b)
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
