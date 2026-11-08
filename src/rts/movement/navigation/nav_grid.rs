//! Coarse terrain passability grid shared by all flow-field builds.

use bevy::prelude::*;

use crate::world::static_terrain::StaticTerrainHeights;

/// World units covered by one navigation cell.
pub const CELL_SIZE: f32 = 8.0;
/// Passability samples per cell side (every 2 world units).
const SAMPLES_PER_SIDE: usize = 4;

/// Passability per coarse cell. Built once at startup, shared across async tasks via `Arc`.
pub struct NavGrid {
    passable: Vec<bool>,
    /// Connected-region id per passable cell (cells with different ids can't reach each other).
    regions: Vec<u32>,
    pub(super) width: i32,
    world_min: f32,
}

impl NavGrid {
    /// A cell is blocked if any 2-unit sample inside it is impassable terrain.
    pub fn from_terrain(terrain: &StaticTerrainHeights, world_size: f32) -> Self {
        let width = (world_size / CELL_SIZE) as i32;
        let world_min = -world_size * 0.5;
        let sample_step = CELL_SIZE / SAMPLES_PER_SIDE as f32;
        let passable = (0..width * width)
            .map(|i| {
                let (x0, z0) = (world_min + (i % width) as f32 * CELL_SIZE, world_min + (i / width) as f32 * CELL_SIZE);
                (0..SAMPLES_PER_SIDE * SAMPLES_PER_SIDE).all(|s| {
                    let wx = x0 + ((s % SAMPLES_PER_SIDE) as f32 + 0.5) * sample_step;
                    let wz = z0 + ((s / SAMPLES_PER_SIDE) as f32 + 0.5) * sample_step;
                    terrain.is_passable(wx, wz)
                })
            })
            .collect();
        Self::with_regions(passable, width, world_min)
    }

    /// Grid from explicit passability (row-major, `width` × `width`), for tests.
    #[cfg(test)]
    pub(super) fn from_cells(passable: Vec<bool>, width: i32, world_min: f32) -> Self {
        Self::with_regions(passable, width, world_min)
    }

    fn with_regions(passable: Vec<bool>, width: i32, world_min: f32) -> Self {
        let mut grid = Self { regions: vec![u32::MAX; passable.len()], passable, width, world_min };
        let mut next_region = 0;
        for start in 0..grid.passable.len() {
            if grid.passable[start] && grid.regions[start] == u32::MAX {
                grid.flood_region(start, next_region);
                next_region += 1;
            }
        }
        grid
    }

    /// Labels every passable cell orthogonally connected to `start` with `region`.
    fn flood_region(&mut self, start: usize, region: u32) {
        let mut stack = vec![start];
        self.regions[start] = region;
        while let Some(i) = stack.pop() {
            let cell = IVec2::new(i as i32 % self.width, i as i32 / self.width);
            for step in [IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y] {
                let next = cell + step;
                if self.is_passable(next) && self.regions[self.index(next)] == u32::MAX {
                    let index = self.index(next);
                    self.regions[index] = region;
                    stack.push(index);
                }
            }
        }
    }

    /// Whether two passable cells can reach each other.
    pub fn is_connected(&self, a: IVec2, b: IVec2) -> bool {
        self.regions[self.index(a)] == self.regions[self.index(b)]
    }

    pub fn cell_of(&self, pos: Vec3) -> Option<IVec2> {
        let cell = IVec2::new(
            ((pos.x - self.world_min) / CELL_SIZE).floor() as i32,
            ((pos.z - self.world_min) / CELL_SIZE).floor() as i32,
        );
        self.contains(cell).then_some(cell)
    }

    pub fn cell_center(&self, cell: IVec2) -> Vec2 {
        Vec2::new(
            self.world_min + (cell.x as f32 + 0.5) * CELL_SIZE,
            self.world_min + (cell.y as f32 + 0.5) * CELL_SIZE,
        )
    }

    pub(super) fn contains(&self, cell: IVec2) -> bool {
        cell.x >= 0 && cell.y >= 0 && cell.x < self.width && cell.y < self.width
    }

    pub(super) fn index(&self, cell: IVec2) -> usize {
        (cell.y * self.width + cell.x) as usize
    }

    pub fn is_passable(&self, cell: IVec2) -> bool {
        self.contains(cell) && self.passable[self.index(cell)]
    }

    /// Nearest passable cell by expanding square rings, up to `max_ring` cells away.
    pub fn nearest_passable(&self, cell: IVec2, max_ring: i32) -> Option<IVec2> {
        if self.is_passable(cell) {
            return Some(cell);
        }
        (1..=max_ring).find_map(|ring| {
            (-ring..=ring)
                .flat_map(|dx| (-ring..=ring).map(move |dz| IVec2::new(dx, dz)))
                .filter(|d| d.x.abs() == ring || d.y.abs() == ring)
                .map(|d| cell + d)
                .find(|&c| self.is_passable(c))
        })
    }
}
