//! Per-entity path cache: lookup, validation, eviction, and resume helpers.

use bevy::prelude::Vec3;
use std::cmp::Ordering;

use crate::core::components::PathfindingState;
use super::grid::TerrainPathfindingGrid;

pub(super) const CACHE_DURATION: f32 = 50.0;
pub(super) const MAX_CACHE_SIZE: usize = 20;

/// Returns a slice of `cached_path` starting at the waypoint nearest to `current_pos`.
pub(super) fn resume_from_cache(cached_path: &[Vec3], current_pos: Vec3) -> Vec<Vec3> {
    let start = cached_path
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            current_pos
                .distance_squared(**a)
                .partial_cmp(&current_pos.distance_squared(**b))
                .unwrap_or(Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);
    cached_path[start..].to_vec()
}

/// Returns a valid cached path for `raw_target`, or `None` on miss / expiry.
/// Evicts stale or invalid entries from the cache.
pub(super) fn try_cached_path(
    grid: &TerrainPathfindingGrid,
    pf: &mut PathfindingState,
    raw_target: Vec3,
    current_pos: Vec3,
    now: f32,
) -> Option<Vec<Vec3>> {
    let goal_grid = grid.world_to_grid(raw_target)?;

    // Check validity without a long-lived borrow so we can mutate `pf` afterward.
    let is_valid = pf
        .path_cache
        .get(&goal_grid)
        .map(|(cached, stamp)| now - stamp < CACHE_DURATION && grid.is_cached_path_valid(cached));

    match is_valid? {
        false => {
            pf.path_cache.remove(&goal_grid);
            None
        }
        true => {
            let (cached, _) = pf.path_cache.get(&goal_grid)?;
            let path = resume_from_cache(cached, current_pos);
            if path.is_empty() { None } else { Some(path) }
        }
    }
}

/// Inserts `path` into the cache, evicting the oldest entry if at capacity.
pub(super) fn update_path_cache(
    pf: &mut PathfindingState,
    goal_grid: (i32, i32),
    path: &[Vec3],
    now: f32,
) {
    if pf.path_cache.len() >= MAX_CACHE_SIZE {
        let oldest = pf
            .path_cache
            .iter()
            .min_by(|(_, (_, ta)), (_, (_, tb))| ta.partial_cmp(tb).unwrap_or(Ordering::Equal))
            .map(|(k, _)| *k);
        if let Some(k) = oldest {
            pf.path_cache.remove(&k);
        }
    }
    pf.path_cache.insert(goal_grid, (path.to_vec(), now));
}
