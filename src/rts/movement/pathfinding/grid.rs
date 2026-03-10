//! Terrain passability grid and A* pathfinding over it.

use bevy::prelude::*;
use hashbrown::HashMap;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;

use crate::world::static_terrain::StaticTerrainHeights;

/// World units represented by one grid cell.
pub const GRID_RESOLUTION: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum CellState {
    Passable,
    Blocked,
}

/// Terrain passability grid. Built once at startup, shared across async tasks via `Arc`.
#[derive(Resource, Clone)]
pub struct TerrainPathfindingGrid {
    cells: Vec<Vec<CellState>>,
    width: usize,
    height: usize,
    world_min: Vec2,
}

impl TerrainPathfindingGrid {
    pub fn from_terrain(terrain: &StaticTerrainHeights, world_size: f32) -> Self {
        let grid_size = (world_size / GRID_RESOLUTION) as usize;
        let half = world_size * 0.5;

        const SAMPLE_OFFSETS: [(f32, f32); 9] = [
            (0.0, 0.0),
            (-0.4, -0.4), (-0.4, 0.4), (0.4, -0.4), (0.4, 0.4),
            (-0.4, 0.0),  (0.4, 0.0),  (0.0, -0.4), (0.0, 0.4),
        ];

        let mut cells = vec![vec![CellState::Passable; grid_size]; grid_size];
        for (x, row) in cells.iter_mut().enumerate() {
            for (z, cell) in row.iter_mut().enumerate() {
                let wx = (x as f32 * GRID_RESOLUTION) - half;
                let wz = (z as f32 * GRID_RESOLUTION) - half;
                let blocked = SAMPLE_OFFSETS.iter().any(|(ox, oz)| {
                    !terrain.is_passable(wx + ox * GRID_RESOLUTION, wz + oz * GRID_RESOLUTION)
                });
                if blocked {
                    *cell = CellState::Blocked;
                }
            }
        }

        Self { cells, width: grid_size, height: grid_size, world_min: Vec2::new(-half, -half) }
    }

    pub fn world_to_grid(&self, pos: Vec3) -> Option<(i32, i32)> {
        let x = ((pos.x - self.world_min.x) / GRID_RESOLUTION) as i32;
        let z = ((pos.z - self.world_min.y) / GRID_RESOLUTION) as i32;
        if x >= 0 && x < self.width as i32 && z >= 0 && z < self.height as i32 {
            Some((x, z))
        } else {
            None
        }
    }

    pub fn is_passable(&self, gx: i32, gz: i32) -> bool {
        if gx < 0 || gx >= self.width as i32 || gz < 0 || gz >= self.height as i32 {
            return false;
        }
        matches!(self.cells[gx as usize][gz as usize], CellState::Passable)
    }

    /// A* search from `start` to `goal`. Returns `None` if no path exists.
    pub fn find_path(&self, start: Vec3, goal: Vec3, terrain: &StaticTerrainHeights) -> Option<Vec<Vec3>> {
        let state = init_search(self, start, goal)?;
        let result = run_astar(self, state)?;
        Some(build_world_path(self, result, terrain))
    }

    /// Spirals outward from `target` to find the nearest passable world position.
    pub fn find_nearest_passable(&self, target: Vec3, terrain: &StaticTerrainHeights) -> Option<Vec3> {
        if terrain.is_passable(target.x, target.z) {
            if let Some(g) = self.world_to_grid(target) {
                if self.is_passable(g.0, g.1) {
                    return Some(target);
                }
            }
        }
        for r in 1..=40_i32 {
            for i in 0..(8 * r) {
                let angle = (i as f32 / (8 * r) as f32) * std::f32::consts::TAU;
                let wx = target.x + angle.cos() * r as f32 * GRID_RESOLUTION;
                let wz = target.z + angle.sin() * r as f32 * GRID_RESOLUTION;
                if !terrain.is_passable(wx, wz) {
                    continue;
                }
                if let Some(g) = self.world_to_grid(Vec3::new(wx, 0.0, wz)) {
                    if self.is_passable(g.0, g.1) {
                        return Some(Vec3::new(wx, terrain.get_height(wx, wz), wz));
                    }
                }
            }
        }
        None
    }

    /// Returns `false` if any sampled waypoint maps to a blocked grid cell.
    pub(super) fn is_cached_path_valid(&self, path: &[Vec3]) -> bool {
        if path.is_empty() {
            return false;
        }
        let last = path.len() - 1;
        (0..path.len())
            .step_by(4)
            .chain(std::iter::once(last))
            .all(|i| {
                self.world_to_grid(path[i])
                    .map(|(gx, gz)| self.is_passable(gx, gz))
                    .unwrap_or(false)
            })
    }

    fn grid_to_world(&self, gx: i32, gz: i32, terrain: &StaticTerrainHeights) -> Vec3 {
        let wx = self.world_min.x + gx as f32 * GRID_RESOLUTION + GRID_RESOLUTION * 0.5;
        let wz = self.world_min.y + gz as f32 * GRID_RESOLUTION + GRID_RESOLUTION * 0.5;
        Vec3::new(wx, terrain.get_height(wx, wz), wz)
    }

    fn neighbors(&self, gx: i32, gz: i32) -> impl Iterator<Item = (i32, i32, f32)> + '_ {
        const DIRS: [(i32, i32, f32); 8] = [
            (-1, -1, 1.414), (-1, 0, 1.0), (-1, 1, 1.414),
            ( 0, -1, 1.0),                  ( 0, 1, 1.0),
            ( 1, -1, 1.414), ( 1, 0, 1.0), ( 1, 1, 1.414),
        ];
        DIRS.iter().filter_map(move |&(dx, dz, cost)| {
            let nx = gx + dx;
            let nz = gz + dz;
            self.is_passable(nx, nz).then_some((nx, nz, cost))
        })
    }

    fn heuristic(&self, x1: i32, z1: i32, x2: i32, z2: i32) -> f32 {
        ((x2 - x1).abs() + (z2 - z1).abs()) as f32
    }
}

/// Arc-wrapped grid + terrain shared cheaply across async tasks.
#[derive(Resource, Clone)]
pub struct PathfindingGridResource {
    pub grid: Arc<TerrainPathfindingGrid>,
    pub(super) terrain: Arc<StaticTerrainHeights>,
}

// --- A* internals ---

#[derive(Clone, Copy)]
struct Node {
    x: i32,
    z: i32,
    g: f32,
    f: f32,
    parent: Option<(i32, i32)>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool { self.f == other.f }
}
impl Eq for Node {}
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.partial_cmp(&self.f).unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

struct SearchState {
    start: (i32, i32),
    goal: (i32, i32),
}

struct SearchResult {
    goal_node: Node,
    closed: HashMap<(i32, i32), Node>,
}

const MAX_NODES: usize = 500_000;

fn init_search(grid: &TerrainPathfindingGrid, start: Vec3, goal: Vec3) -> Option<SearchState> {
    let sg = grid.world_to_grid(start)?;
    let gg = grid.world_to_grid(goal)?;
    if !grid.is_passable(sg.0, sg.1) || !grid.is_passable(gg.0, gg.1) {
        return None;
    }
    Some(SearchState { start: sg, goal: gg })
}

fn run_astar(grid: &TerrainPathfindingGrid, state: SearchState) -> Option<SearchResult> {
    let mut open: BinaryHeap<Node> = BinaryHeap::new();
    let mut closed: HashMap<(i32, i32), Node> = HashMap::new();
    let mut explored = 0usize;

    let h0 = grid.heuristic(state.start.0, state.start.1, state.goal.0, state.goal.1);
    open.push(Node { x: state.start.0, z: state.start.1, g: 0.0, f: h0, parent: None });

    while let Some(cur) = open.pop() {
        explored += 1;
        if explored > MAX_NODES {
            return None;
        }
        if closed.contains_key(&(cur.x, cur.z)) {
            continue;
        }
        closed.insert((cur.x, cur.z), cur);
        if cur.x == state.goal.0 && cur.z == state.goal.1 {
            return Some(SearchResult { goal_node: cur, closed });
        }
        for (nx, nz, cost) in grid.neighbors(cur.x, cur.z) {
            if closed.contains_key(&(nx, nz)) {
                continue;
            }
            let g = cur.g + cost;
            let h = grid.heuristic(nx, nz, state.goal.0, state.goal.1);
            open.push(Node { x: nx, z: nz, g, f: g + h, parent: Some((cur.x, cur.z)) });
        }
    }
    None
}

fn build_world_path(grid: &TerrainPathfindingGrid, result: SearchResult, terrain: &StaticTerrainHeights) -> Vec<Vec3> {
    trace_grid_path(result)
        .into_iter()
        .map(|(gx, gz)| grid.grid_to_world(gx, gz, terrain))
        .collect()
}

fn trace_grid_path(result: SearchResult) -> Vec<(i32, i32)> {
    let mut coords = Vec::new();
    let mut cur = (result.goal_node.x, result.goal_node.z);
    loop {
        coords.push(cur);
        let Some(node) = result.closed.get(&cur) else { break };
        let Some(parent) = node.parent else { break };
        cur = parent;
    }
    coords.reverse();
    coords
}
