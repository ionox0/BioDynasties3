//! Building occupancy grid — tracks exclusion zones around placed buildings.
//!
//! Separate from `TerrainPathfindingGrid` so A* routing is never affected by
//! building placement. Used only to find clear spawn positions and validate
//! AI building placements.

use bevy::prelude::*;
use crate::world::static_terrain::StaticTerrainHeights;
use crate::core::constants::movement::TERRAIN_SIZE;

const GRID_RESOLUTION: f32 = 2.0;

/// Tracks cells occupied by building exclusion zones.
/// Populated incrementally as buildings spawn; never reset.
#[derive(Resource)]
pub struct BuildingGrid {
    cells: Vec<bool>,
    width: usize,
    world_min: Vec2,
}

impl BuildingGrid {
    pub fn new() -> Self {
        let world_size = TERRAIN_SIZE * 2.0;
        let width = (world_size / GRID_RESOLUTION) as usize;
        Self {
            cells: vec![false; width * width],
            width,
            world_min: Vec2::splat(-TERRAIN_SIZE),
        }
    }

    /// Marks all cells within `world_radius` of `center` as occupied.
    pub fn mark_circle(&mut self, center: Vec3, world_radius: f32) {
        let cell_radius = (world_radius / GRID_RESOLUTION).ceil() as i32;
        let Some((cx, cz)) = self.world_to_cell(center) else { return };
        let w = self.width as i32;
        for dx in -cell_radius..=cell_radius {
            for dz in -cell_radius..=cell_radius {
                if dx * dx + dz * dz <= cell_radius * cell_radius {
                    let gx = cx + dx;
                    let gz = cz + dz;
                    if gx >= 0 && gx < w && gz >= 0 && gz < w {
                        self.cells[gz as usize * self.width + gx as usize] = true;
                    }
                }
            }
        }
    }

    /// Returns `true` if `pos` falls inside any building's exclusion zone.
    pub fn is_occupied(&self, pos: Vec3) -> bool {
        self.world_to_cell(pos)
            .map(|(x, z)| self.cells[z as usize * self.width + x as usize])
            .unwrap_or(false)
    }

    /// Spirals outward from `pos` to find the nearest position that is both
    /// terrain-passable and outside all building exclusion zones.
    pub fn find_clear_position(&self, pos: Vec3, terrain: &StaticTerrainHeights) -> Option<Vec3> {
        if terrain.is_passable(pos.x, pos.z) && !self.is_occupied(pos) {
            return Some(Vec3::new(pos.x, terrain.get_height(pos.x, pos.z), pos.z));
        }
        for r in 1..=200_i32 {
            let samples = (8 * r) as usize;
            for i in 0..samples {
                let angle = (i as f32 / samples as f32) * std::f32::consts::TAU;
                let wx = pos.x + angle.cos() * r as f32 * GRID_RESOLUTION;
                let wz = pos.z + angle.sin() * r as f32 * GRID_RESOLUTION;
                let candidate = Vec3::new(wx, 0.0, wz);
                if terrain.is_passable(wx, wz) && !self.is_occupied(candidate) {
                    return Some(Vec3::new(wx, terrain.get_height(wx, wz), wz));
                }
            }
        }
        None
    }

    fn world_to_cell(&self, pos: Vec3) -> Option<(i32, i32)> {
        let x = ((pos.x - self.world_min.x) / GRID_RESOLUTION) as i32;
        let z = ((pos.z - self.world_min.y) / GRID_RESOLUTION) as i32;
        let w = self.width as i32;
        if x >= 0 && x < w && z >= 0 && z < w {
            Some((x, z))
        } else {
            None
        }
    }
}
