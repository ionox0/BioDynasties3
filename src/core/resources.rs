use bevy::prelude::*;
use crate::core::spatial_grid::IncrementalEntitySpatialGrid;

/// Persistent spatial grids for efficient collision and movement queries
#[derive(Resource, Debug)]
pub struct SpatialGrids {
    pub entity_grid: IncrementalEntitySpatialGrid,
}

impl Default for SpatialGrids {
    fn default() -> Self {
        Self {
            entity_grid: IncrementalEntitySpatialGrid::with_default_size(),
        }
    }
}

