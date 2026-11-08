//! Proximity grid: answers "which entities are near this point?"
//!
//! A sparse hash grid of moving entities, updated incrementally as they change cells.
//! Used for unit separation and combat target acquisition. Terrain walkability is a
//! separate concern, handled by the navigation grid (`NavGrid`).

use bevy::prelude::*;
use hashbrown::HashMap;

/// Default grid cell size in world units
pub const DEFAULT_GRID_CELL_SIZE: f32 = 50.0;

/// Sparse grid of keyed items, updated incrementally as items move between cells.
#[derive(Debug)]
pub struct ProximityGrid<K, T> 
where
    K: Copy + Eq + std::hash::Hash,
{
    pub cells: HashMap<ProximityCell, Vec<(K, T)>>,
    pub entity_positions: HashMap<K, ProximityCell>,
    pub cell_size: f32,
}

/// Grid coordinate representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProximityCell {
    pub x: i32,
    pub z: i32,
}

impl ProximityCell {
    /// Create a grid coordinate from a world position
    pub fn from_world_pos(pos: Vec3, cell_size: f32) -> Self {
        Self {
            x: (pos.x / cell_size).floor() as i32,
            z: (pos.z / cell_size).floor() as i32,
        }
    }
    
    /// Get all neighboring coordinates (including self) in a 3x3 grid
    pub fn get_neighboring_coords(self) -> [ProximityCell; 9] {
        [
            ProximityCell { x: self.x - 1, z: self.z - 1 },
            ProximityCell { x: self.x, z: self.z - 1 },
            ProximityCell { x: self.x + 1, z: self.z - 1 },
            ProximityCell { x: self.x - 1, z: self.z },
            self, // center
            ProximityCell { x: self.x + 1, z: self.z },
            ProximityCell { x: self.x - 1, z: self.z + 1 },
            ProximityCell { x: self.x, z: self.z + 1 },
            ProximityCell { x: self.x + 1, z: self.z + 1 },
        ]
    }
}


impl<K, T> ProximityGrid<K, T> 
where
    K: Copy + Eq + std::hash::Hash,
{
    /// Create an empty grid with the given cell size.
    pub fn new(cell_size: f32) -> Self {
        Self {
            cells: HashMap::new(),
            entity_positions: HashMap::new(),
            cell_size,
        }
    }
    
    /// Create an empty grid with the default cell size.
    pub fn with_default_size() -> Self {
        Self::new(DEFAULT_GRID_CELL_SIZE)
    }
    
    
    /// Insert or update an item's position. Returns true if the item changed grid cells.
    pub fn update_item(&mut self, key: K, position: Vec3, item: T) -> bool {
        let new_coord = ProximityCell::from_world_pos(position, self.cell_size);
        
        // Check if this is a position update
        if let Some(&old_coord) = self.entity_positions.get(&key) {
            if old_coord == new_coord {
                // Same cell, just update the item data in place
                if let Some(cell) = self.cells.get_mut(&new_coord) {
                    if let Some(entry) = cell.iter_mut().find(|(k, _)| *k == key) {
                        entry.1 = item;
                    }
                }
                return false;
            }
            // Different cell, remove from old cell
            self.remove_from_cell(key, old_coord);
        }
        
        // Add to new cell
        self.cells.entry(new_coord).or_default().push((key, item));
        self.entity_positions.insert(key, new_coord);
        true
    }
    
    /// Remove an item from the grid
    pub fn remove_item(&mut self, key: K) -> bool {
        if let Some(coord) = self.entity_positions.remove(&key) {
            self.remove_from_cell(key, coord);
            true
        } else {
            false
        }
    }
    
    /// Remove item from a specific cell
    fn remove_from_cell(&mut self, key: K, coord: ProximityCell) {
        if let Some(cell) = self.cells.get_mut(&coord) {
            cell.retain(|(k, _)| *k != key);
            if cell.is_empty() {
                self.cells.remove(&coord);
            }
        }
    }
    
    
    /// Get all items with keys in the cell and neighboring cells around the given position
    pub fn query_nearby_with_keys(&self, position: Vec3) -> Vec<(K, &T)> {
        let coord = ProximityCell::from_world_pos(position, self.cell_size);
        let neighboring_coords = coord.get_neighboring_coords();
        
        let mut results = Vec::new();
        for &neighbor_coord in &neighboring_coords {
            if let Some(items) = self.cells.get(&neighbor_coord) {
                for &(key, ref item) in items {
                    results.push((key, item));
                }
            }
        }
        results
    }
}

/// Proximity grid of units: entity → (position, collision radius).
pub type UnitProximityGrid = ProximityGrid<Entity, (Vec3, f32)>;


impl UnitProximityGrid {
    /// Update an entity's position and radius. Returns true if it changed grid cells.
    pub fn update_entity(&mut self, entity: Entity, position: Vec3, radius: f32) -> bool {
        self.update_item(entity, position, (position, radius))
    }
    
    /// Query nearby entities within a certain radius, excluding the given entity
    pub fn query_nearby_entities(&self, position: Vec3, radius: f32, exclude_entity: Option<Entity>) -> Vec<(Entity, Vec3, f32)> {
        self.query_nearby_with_keys(position)
            .into_iter()
            .filter(|(entity, (entity_pos, entity_radius))| {
                if let Some(exclude) = exclude_entity {
                    if *entity == exclude {
                        return false;
                    }
                }
                position.distance(*entity_pos) <= radius + entity_radius + self.cell_size
            })
            .map(|(entity, (pos, radius))| (entity, *pos, *radius))
            .collect()
    }
}