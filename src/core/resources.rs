use std::collections::HashMap;
use bevy::prelude::*;
use crate::core::spatial_grid::IncrementalEntitySpatialGrid;
use crate::core::components::ResourceType;

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

/// Per-player accumulated resources. Keyed by player_id (1 = player, 2+ = AI).
#[derive(Resource, Default, Debug)]
pub struct Stockpiles(pub HashMap<u8, Stockpile>);

impl Stockpiles {
    pub fn get_or_insert_mut(&mut self, player_id: u8) -> &mut Stockpile {
        self.0.entry(player_id).or_default()
    }
}

#[derive(Default, Debug, Clone)]
pub struct Stockpile {
    pub nectar: f32,
    pub chitin: f32,
    pub minerals: f32,
    pub pheromones: f32,
}

impl Stockpile {
    /// Returns a stockpile pre-filled with standard starting resources.
    pub fn starting() -> Self {
        use crate::core::constants::resources as rc;
        Self {
            nectar: rc::STARTING_NECTAR,
            chitin: rc::STARTING_CHITIN,
            minerals: rc::STARTING_MINERALS,
            pheromones: rc::STARTING_PHEROMONES,
        }
    }

    pub fn add(&mut self, resource_type: &ResourceType, amount: f32) {
        match resource_type {
            ResourceType::Nectar => self.nectar += amount,
            ResourceType::Chitin => self.chitin += amount,
            ResourceType::Minerals => self.minerals += amount,
            ResourceType::Pheromones => self.pheromones += amount,
        }
    }
}
