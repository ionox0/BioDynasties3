//! Grid-based async A* pathfinding for RTS terrain navigation.
//!
//! The terrain grid is built once at startup by `PathfindingPlugin`. Per frame:
//! - `request_paths`   — cache check + spawn background A* tasks (registered by `MovementPlugin`)
//! - `poll_path_tasks` — collect completed tasks, write paths to `PathfindingState`

mod cache;
pub mod grid;
pub mod systems;

pub use systems::{poll_path_tasks, request_paths};

use bevy::prelude::*;
use std::sync::Arc;
use tracing::instrument;

use crate::world::static_terrain::{MapSeed, StaticTerrainHeights};

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup_pathfinding_grid);
    }
}

#[instrument(skip_all)]
fn setup_pathfinding_grid(
    mut commands: Commands,
    terrain: Option<Res<StaticTerrainHeights>>,
    map_seed: Option<Res<MapSeed>>,
    existing_grid: Option<Res<grid::TerrainPathfindingGrid>>,
    mut initialized: Local<bool>,
) {
    if *initialized || existing_grid.is_some() {
        return;
    }
    let Some(terrain) = terrain else { return };
    let Some(map_seed) = map_seed else { return };

    let world_size = crate::core::constants::movement::MAP_BOUNDARY * 2.0;
    let grid = grid::TerrainPathfindingGrid::from_terrain(&terrain, world_size);
    let grid_arc = Arc::new(grid.clone());
    let terrain_arc = Arc::new(StaticTerrainHeights::from_seed(map_seed.0));
    commands.insert_resource(grid);
    commands.insert_resource(grid::PathfindingGridResource { grid: grid_arc, terrain: terrain_arc });
    *initialized = true;
}
