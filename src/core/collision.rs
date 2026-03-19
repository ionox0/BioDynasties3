//! Unit separation system.
//!
//! Owns: separation-driven corrections to `Transform.translation`.
//! Runs after `GameSet::RtsUpdate` so movement always resolves first.
//!
//! Design:
//! - A* handles routing around static obstacles.
//! - Building/environment separation always applies (units must not clip geometry).
//! - Unit-unit separation only applies to **idle** units (no active target_position).
//!   Moving units pass through each other briefly; they spread out once they arrive.
//!   This prevents separation forces from fighting path-following and launching lead
//!   units past their waypoints.

use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use hashbrown::HashMap;

use crate::core::components::*;
use crate::core::resources::SpatialGrids;
use crate::core::spatial_grid::GridCoord;
use crate::core::GameSet;

/// Push strength per unit of overlap (world units out per world unit in).
const SEPARATION_STRENGTH: f32 = 1.0;
/// Extra gap kept between two idle units (on top of their summed radii).
const UNIT_BUFFER: f32 = 0.5;
/// Extra gap kept between a unit and a building or environment object.
const BUILDING_BUFFER: f32 = 1.5;

// --- SystemParam query structs ---

/// RTSUnit entities eligible for separation (excludes static obstacle types).
#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
struct MobileUnits<'w, 's> {
    query: Query<
        'w,
        's,
        (Entity, &'static mut Transform, &'static CollisionRadius, &'static Movement),
        (With<RTSUnit>, Without<Building>, Without<EnvironmentObject>),
    >,
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
            &'static mut SpatialGridPosition,
        ),
        With<RTSUnit>,
    >,
}

#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
struct StaticObstacles<'w, 's> {
    buildings: Query<
        'w,
        's,
        (&'static Position, &'static CollisionRadius),
        (With<Building>, Without<Movement>),
    >,
    env: Query<
        'w,
        's,
        (&'static Transform, &'static CollisionRadius),
        (With<EnvironmentObject>, Without<Movement>),
    >,
}

// --- Plugin ---

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialGrids>().add_systems(
            Update,
            (spatial_grid_update_system, separate_units)
                .chain()
                .after(GameSet::RtsUpdate),
        );
    }
}

// --- Systems ---

/// Incrementally syncs unit positions into the spatial grid.
fn spatial_grid_update_system(
    mut grids: ResMut<SpatialGrids>,
    mut units: SpatiallyTrackedUnits,
) {
    use std::collections::HashSet;
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
        let coord =
            GridCoord::from_world_pos(transform.translation, grids.entity_grid.cell_size);
        if grid_pos.dirty || grid_pos.last_grid_coord != Some(coord) {
            grids.entity_grid.update_entity(
                entity,
                transform.translation,
                radius.radius,
            );
            grid_pos.last_grid_coord = Some(coord);
            grid_pos.dirty = false;
        }
    }
}

/// Pushes overlapping units apart. Direct position correction — no events, no velocity changes.
/// Unit-unit separation is only applied to idle units to avoid fighting path-following.
fn separate_units(
    grids: Res<SpatialGrids>,
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

fn unit_separation(entity: Entity, pos: Vec3, radius: f32, grids: &SpatialGrids) -> Vec3 {
    grids
        .entity_grid
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
