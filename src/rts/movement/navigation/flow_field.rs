//! Shared flow fields: one Dijkstra search per move order, sampled by every unit in it.

use bevy::prelude::*;
use hashbrown::{HashMap, HashSet};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::Arc;

use super::nav_grid::NavGrid;

const ORTHOGONAL_COST: u32 = 10;
const DIAGONAL_COST: u32 = 14;
const NEIGHBORS: [(i32, i32, u32); 8] = [
    (1, 0, ORTHOGONAL_COST), (-1, 0, ORTHOGONAL_COST), (0, 1, ORTHOGONAL_COST), (0, -1, ORTHOGONAL_COST),
    (1, 1, DIAGONAL_COST), (1, -1, DIAGONAL_COST), (-1, 1, DIAGONAL_COST), (-1, -1, DIAGONAL_COST),
];
/// How far (in cells) start and goal cells may be nudged to reach passable terrain.
const SNAP_RING: i32 = 6;

/// Cost-to-goal for every cell the search reached. Sparse, so short trips stay cheap.
pub struct FlowField {
    costs: HashMap<usize, u32>,
    grid: Arc<NavGrid>,
}

impl FlowField {
    /// Dijkstra outward from `goal`, stopping once every reachable start cell is settled.
    pub fn build(grid: Arc<NavGrid>, goal: Vec3, starts: &[Vec3]) -> Self {
        let mut costs = HashMap::new();
        let Some(goal_cell) = grid.cell_of(goal).and_then(|c| grid.nearest_passable(c, SNAP_RING)) else {
            return Self { costs, grid };
        };
        let mut remaining: HashSet<usize> = starts
            .iter()
            .filter_map(|&s| grid.cell_of(s).and_then(|c| grid.nearest_passable(c, SNAP_RING)))
            .filter(|&c| grid.is_connected(c, goal_cell)) // unreachable starts would flood the whole map
            .map(|c| grid.index(c))
            .collect();
        let mut open = BinaryHeap::from([Reverse((0u32, goal_cell.x, goal_cell.y))]);
        costs.insert(grid.index(goal_cell), 0);
        while let Some(Reverse((cost, x, z))) = open.pop() {
            let cell = IVec2::new(x, z);
            if costs.get(&grid.index(cell)).is_some_and(|&best| cost > best) {
                continue;
            }
            remaining.remove(&grid.index(cell));
            if remaining.is_empty() {
                break;
            }
            for (next, step) in passable_neighbors(&grid, cell) {
                let next_cost = cost + step;
                let entry = costs.entry(grid.index(next)).or_insert(u32::MAX);
                if next_cost < *entry {
                    *entry = next_cost;
                    open.push(Reverse((next_cost, next.x, next.y)));
                }
            }
        }
        Self { costs, grid }
    }

    /// Direction toward the cheapest neighboring cell, or `None` at the goal / outside the field.
    /// Units standing on blocked cells first head for the nearest open cell.
    pub fn direction(&self, pos: Vec3) -> Option<Vec3> {
        let cell = self.grid.cell_of(pos)?;
        let next = if self.grid.is_passable(cell) {
            self.cheapest_neighbor(cell)?
        } else {
            self.grid.nearest_passable(cell, SNAP_RING)?
        };
        let center = self.grid.cell_center(next);
        Some(Vec3::new(center.x - pos.x, 0.0, center.y - pos.z).normalize_or_zero())
    }

    fn cheapest_neighbor(&self, cell: IVec2) -> Option<IVec2> {
        let own = self.costs.get(&self.grid.index(cell)).copied().unwrap_or(u32::MAX);
        passable_neighbors(&self.grid, cell)
            .filter_map(|(next, _)| Some((next, *self.costs.get(&self.grid.index(next))?)))
            .filter(|&(_, cost)| cost < own)
            .min_by_key(|&(_, cost)| cost)
            .map(|(next, _)| next)
    }
}

/// Passable 8-neighbors; diagonals require both adjacent orthogonal cells open (no corner cutting).
fn passable_neighbors(grid: &NavGrid, cell: IVec2) -> impl Iterator<Item = (IVec2, u32)> + '_ {
    NEIGHBORS.iter().filter_map(move |&(dx, dz, cost)| {
        let next = cell + IVec2::new(dx, dz);
        let is_open = grid.is_passable(next)
            && (dx == 0 || dz == 0
                || (grid.is_passable(cell + IVec2::new(dx, 0)) && grid.is_passable(cell + IVec2::new(0, dz))));
        is_open.then_some((next, cost))
    })
}

/// True if the straight segment between two points crosses only passable cells.
pub fn has_clear_line(grid: &NavGrid, from: Vec3, to: Vec3) -> bool {
    let delta = Vec2::new(to.x - from.x, to.z - from.z);
    let samples = (delta.length() / (super::nav_grid::CELL_SIZE * 0.5)).ceil() as usize;
    (0..=samples).all(|i| {
        let t = i as f32 / samples.max(1) as f32;
        let point = from + Vec3::new(delta.x * t, 0.0, delta.y * t);
        grid.cell_of(point).is_some_and(|c| grid.is_passable(c))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rts::movement::navigation::nav_grid::CELL_SIZE;

    /// 20×20 grid with a wall along x = 10, open only at z = 18.
    fn walled_grid() -> Arc<NavGrid> {
        let width = 20;
        let passable = (0..width * width).map(|i| i % width != 10 || i / width == 18).collect();
        Arc::new(NavGrid::from_cells(passable, width, 0.0))
    }

    fn world(cell: IVec2) -> Vec3 {
        Vec3::new((cell.x as f32 + 0.5) * CELL_SIZE, 0.0, (cell.y as f32 + 0.5) * CELL_SIZE)
    }

    #[test]
    fn units_route_through_the_gap_without_entering_walls() {
        let grid = walled_grid();
        let (start, goal) = (world(IVec2::new(2, 2)), world(IVec2::new(17, 2)));
        let field = FlowField::build(Arc::clone(&grid), goal, &[start]);
        let mut pos = start;
        for _ in 0..2000 {
            let Some(dir) = field.direction(pos) else { break };
            pos += dir * CELL_SIZE * 0.25;
            assert!(grid.cell_of(pos).is_some_and(|c| grid.is_passable(c)), "entered a wall at {pos:?}");
        }
        assert_eq!(grid.cell_of(pos), grid.cell_of(goal), "did not reach the goal cell");
    }

    #[test]
    fn clear_line_detects_walls() {
        let grid = walled_grid();
        assert!(has_clear_line(&grid, world(IVec2::new(2, 2)), world(IVec2::new(8, 15))));
        assert!(!has_clear_line(&grid, world(IVec2::new(2, 2)), world(IVec2::new(17, 2))));
    }
}
