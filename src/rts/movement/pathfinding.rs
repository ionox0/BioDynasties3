//! Grid-based A* pathfinding for RTS terrain navigation.
//!
//! `TerrainPathfindingGrid` is built once at startup from terrain passability data.
//! `pathfinding_system` (pub, called from MovementPlugin) fills `PathfindingState.path`
//! each frame for ground units that need a new route.

use bevy::prelude::*;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use tracing::instrument;

use crate::core::components::{Movement, PathfindingState, RTSUnit};
use crate::world::static_terrain::StaticTerrainHeights;

/// World units represented by one grid cell.
pub const GRID_RESOLUTION: f32 = 2.0;

/// Retry cooldown in Bevy elapsed seconds after a pathfinding failure.
const PATHFINDING_RETRY_COOLDOWN: f32 = 2.0;

// ---------------------------------------------------------------------------
// Grid resource
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum CellState {
    Passable,
    Blocked,
}

/// A* grid built from static terrain data. Inserted as a `Resource` at startup.
#[derive(Resource)]
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

        let sample_offsets: [(f32, f32); 9] = [
            (0.0, 0.0),
            (-0.4, -0.4), (-0.4, 0.4), (0.4, -0.4), (0.4, 0.4),
            (-0.4, 0.0), (0.4, 0.0), (0.0, -0.4), (0.0, 0.4),
        ];

        let mut cells = vec![vec![CellState::Passable; grid_size]; grid_size];

        for (x, row) in cells.iter_mut().enumerate() {
            for (z, cell) in row.iter_mut().enumerate() {
                let wx = (x as f32 * GRID_RESOLUTION) - half;
                let wz = (z as f32 * GRID_RESOLUTION) - half;
                let blocked = sample_offsets.iter().any(|(ox, oz)| {
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

    /// A* search. Returns `None` if no path exists or goal is outside / blocked.
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
        let max_r = 40;
        for r in 1..=max_r {
            for i in 0..(8 * r) {
                let angle = (i as f32 / (8 * r) as f32) * std::f32::consts::TAU;
                let wx = target.x + angle.cos() * r as f32 * GRID_RESOLUTION;
                let wz = target.z + angle.sin() * r as f32 * GRID_RESOLUTION;
                if !terrain.is_passable(wx, wz) {
                    continue;
                }
                let probe = Vec3::new(wx, 0.0, wz);
                if let Some(g) = self.world_to_grid(probe) {
                    if self.is_passable(g.0, g.1) {
                        return Some(Vec3::new(wx, terrain.get_height(wx, wz), wz));
                    }
                }
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// A* internals
// ---------------------------------------------------------------------------

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

const MAX_NODES: usize = 50_000;

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

fn build_world_path(
    grid: &TerrainPathfindingGrid,
    result: SearchResult,
    terrain: &StaticTerrainHeights,
) -> Vec<Vec3> {
    let grid_path = trace_grid_path(result);
    grid_path.into_iter().map(|(gx, gz)| grid.grid_to_world(gx, gz, terrain)).collect()
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

// ---------------------------------------------------------------------------
// Pathfinding system (pub — registered by MovementPlugin, runs before move_units)
// ---------------------------------------------------------------------------

fn unit_needs_path(mv: &Movement, pf: &PathfindingState) -> bool {
    mv.target_position.is_some() && (pf.path.is_empty() || pf.path_index >= pf.path.len())
}

fn should_retry(now: f32, pf: &PathfindingState) -> bool {
    now - pf.last_pathfinding_failure >= PATHFINDING_RETRY_COOLDOWN
}

/// Computes A* paths for ground units that need one. Registered by `MovementPlugin`.
pub fn pathfinding_system(
    mut units: Query<(Entity, &Transform, &Movement, &mut PathfindingState), With<RTSUnit>>,
    grid: Option<Res<TerrainPathfindingGrid>>,
    terrain: Res<StaticTerrainHeights>,
    time: Res<Time>,
) {
    let Some(grid) = grid else { return };
    let now = time.elapsed_secs();

    for (_entity, transform, movement, mut pf) in units.iter_mut() {
        if !unit_needs_path(movement, &pf) || !should_retry(now, &pf) {
            continue;
        }
        let Some(raw_target) = movement.target_position else { continue };

        let target = if grid.world_to_grid(raw_target).map(|g| grid.is_passable(g.0, g.1)).unwrap_or(false) {
            raw_target
        } else {
            match grid.find_nearest_passable(raw_target, &terrain) {
                Some(t) => t,
                None => {
                    pf.last_pathfinding_failure = now;
                    pf.last_failed_target = Some(raw_target);
                    continue;
                }
            }
        };

        match grid.find_path(transform.translation, target, &terrain) {
            Some(path) if !path.is_empty() => {
                pf.path = path;
                pf.path_index = 0;
            }
            _ => {
                pf.last_pathfinding_failure = now;
                pf.last_failed_target = Some(raw_target);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin (grid setup only — pathfinding_system is registered by MovementPlugin)
// ---------------------------------------------------------------------------

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
    grid: Option<Res<TerrainPathfindingGrid>>,
    mut initialized: Local<bool>,
) {
    if *initialized || grid.is_some() {
        return;
    }
    let Some(terrain) = terrain else { return };

    let world_size = crate::core::constants::movement::MAP_BOUNDARY * 2.0;
    commands.insert_resource(TerrainPathfindingGrid::from_terrain(&terrain, world_size));
    *initialized = true;
}
