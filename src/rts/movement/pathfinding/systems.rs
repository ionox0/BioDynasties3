//! Bevy systems for async A* path requests and result collection.
//!
//! Ownership: `PathfindingState.path` and `path_index` (written here on task completion).

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on};
use bevy::tasks::futures_lite::future;

use crate::core::components::{Movement, PathfindingState, RTSUnit};
use crate::world::static_terrain::StaticTerrainHeights;
use super::cache::{try_cached_path, update_path_cache};
use super::grid::{PathfindingGridResource, TerrainPathfindingGrid};

const PATHFINDING_RETRY_COOLDOWN: f32 = 2.0;

/// Attached to a unit while an async A* search is in progress.
/// Removed via `Commands` when the task completes or the unit's target changes.
#[derive(Component)]
pub struct PathTask {
    task: Task<Option<Vec<Vec3>>>,
    raw_target: Vec3,
}

fn unit_needs_path(mv: &Movement, pf: &PathfindingState) -> bool {
    mv.target_position.is_some() && (pf.path.is_empty() || pf.path_index >= pf.path.len())
}

fn should_retry(now: f32, pf: &PathfindingState) -> bool {
    now - pf.last_pathfinding_failure >= PATHFINDING_RETRY_COOLDOWN
}

fn resolve_target(raw: Vec3, grid: &TerrainPathfindingGrid, terrain: &StaticTerrainHeights) -> Option<Vec3> {
    if grid.world_to_grid(raw).map(|g| grid.is_passable(g.0, g.1)).unwrap_or(false) {
        Some(raw)
    } else {
        grid.find_nearest_passable(raw, terrain)
    }
}

/// Checks the path cache, then spawns an async A* task for each unit that needs a route.
/// Units with an in-flight `PathTask` are skipped (`Without<PathTask>` filter).
pub fn request_paths(
    mut commands: Commands,
    mut units: Query<
        (Entity, &Transform, &Movement, &mut PathfindingState),
        (With<RTSUnit>, Without<PathTask>),
    >,
    grid_res: Option<Res<PathfindingGridResource>>,
    time: Res<Time>,
) {
    let Some(grid_res) = grid_res else { return };
    let now = time.elapsed_secs();
    let task_pool = AsyncComputeTaskPool::get();

    for (entity, transform, movement, mut pf) in units.iter_mut() {
        if !unit_needs_path(movement, &pf) || !should_retry(now, &pf) {
            continue;
        }
        let Some(raw_target) = movement.target_position else { continue };

        // Cache hit — apply immediately with no async overhead.
        if let Some(path) = try_cached_path(&grid_res.grid, &mut pf, raw_target, transform.translation, now) {
            pf.path = path;
            pf.path_index = 0;
            continue;
        }

        // Cache miss — run A* on a background thread.
        let grid = grid_res.grid.clone();
        let terrain = grid_res.terrain.clone();
        let start = transform.translation;
        let task = task_pool.spawn(async move {
            let target = resolve_target(raw_target, &grid, &terrain)?;
            grid.find_path(start, target, &terrain)
        });
        commands.entity(entity).insert(PathTask { task, raw_target });
    }
}

/// Polls in-flight `PathTask`s and writes completed results to `PathfindingState`.
pub fn poll_path_tasks(
    mut commands: Commands,
    mut units: Query<(Entity, &Movement, &mut PathfindingState, &mut PathTask)>,
    grid_res: Option<Res<PathfindingGridResource>>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();

    for (entity, movement, mut pf, mut path_task) in units.iter_mut() {
        let Some(result) = block_on(future::poll_once(&mut path_task.task)) else { continue };

        let raw_target = path_task.raw_target;
        commands.entity(entity).remove::<PathTask>();

        // Discard stale result if the target changed while the task was in flight.
        if movement.target_position != Some(raw_target) {
            continue;
        }

        match result {
            Some(path) if !path.is_empty() => {
                if let Some(grid) = grid_res.as_ref() {
                    if let Some(goal_grid) = grid.grid.world_to_grid(raw_target) {
                        update_path_cache(&mut pf, goal_grid, &path, now);
                    }
                }
                pf.path = path;
                pf.path_index = 0;
            }
            _ => {
                pf.last_pathfinding_failure = now;
                pf.last_failed_target = Some(raw_target);
            }
        }
    }
}
