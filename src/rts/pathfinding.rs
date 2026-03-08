//! Grid-based pathfinding system for RTS terrain navigation.
//!
//! Builds a passability grid from terrain data at startup.

use bevy::prelude::*;
use tracing::instrument;

/// Grid resolution in world units
pub const GRID_RESOLUTION: f32 = 2.0;

/// Marker resource indicating the pathfinding grid has been initialized.
#[derive(Resource)]
pub struct TerrainPathfindingGrid;

impl TerrainPathfindingGrid {
    /// Build the passability grid from terrain data.
    fn from_terrain(
        terrain_heights: &crate::world::static_terrain::StaticTerrainHeights,
        world_size: f32,
    ) -> Self {
        let grid_size = (world_size / GRID_RESOLUTION) as usize;
        let world_half_size = world_size * 0.5;

        let sample_offsets = [
            (0.0_f32, 0.0_f32),
            (-0.4, -0.4),
            (-0.4, 0.4),
            (0.4, -0.4),
            (0.4, 0.4),
            (-0.4, 0.0),
            (0.4, 0.0),
            (0.0, -0.4),
            (0.0, 0.4),
        ];

        let mut blocked_count: usize = 0;
        for x in 0..grid_size {
            for z in 0..grid_size {
                let cell_world_x = (x as f32 * GRID_RESOLUTION) - world_half_size;
                let cell_world_z = (z as f32 * GRID_RESOLUTION) - world_half_size;

                if sample_offsets.iter().any(|(ox, oz)| {
                    !terrain_heights.is_passable(
                        cell_world_x + ox * GRID_RESOLUTION,
                        cell_world_z + oz * GRID_RESOLUTION,
                    )
                }) {
                    blocked_count += 1;
                }
            }
        }

        debug!("Pathfinding grid built: {}x{}, {} blocked cells", grid_size, grid_size, blocked_count);
        Self
    }
}

/// Plugin to add pathfinding systems
pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup_pathfinding_grid);
    }
}

/// Initialize pathfinding grid from terrain data
#[instrument(skip_all)]
fn setup_pathfinding_grid(
    mut commands: Commands,
    terrain_heights: Option<Res<crate::world::static_terrain::StaticTerrainHeights>>,
    pathfinding_grid: Option<Res<TerrainPathfindingGrid>>,
    mut grid_initialized: Local<bool>,
) {
    if *grid_initialized || pathfinding_grid.is_some() {
        return;
    }

    let Some(terrain) = terrain_heights else {
        return;
    };

    let world_size = crate::core::constants::movement::MAP_BOUNDARY * 2.0;
    let grid = TerrainPathfindingGrid::from_terrain(&terrain, world_size);

    commands.insert_resource(grid);
    *grid_initialized = true;
}
