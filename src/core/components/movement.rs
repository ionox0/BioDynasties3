use bevy::prelude::*;
use hashbrown::HashMap;

// Owned by: MovementSystem (position sync)
#[derive(Component, Debug, Clone)]
pub struct Position {
    pub translation: Vec3,
}

impl Default for Position {
    fn default() -> Self {
        Self { translation: Vec3::ZERO }
    }
}

// Owned by: MovementSystem (collision, animation)
#[derive(Component, Debug, Clone)]
pub struct Movement {
    pub max_speed: f32,
    pub current_velocity: Vec3,
    pub target_position: Option<Vec3>,
}

impl Default for Movement {
    fn default() -> Self {
        Self { max_speed: 200.0, current_velocity: Vec3::ZERO, target_position: None }
    }
}

// Owned by: PathfindingPlugin (pathfinding_system) and MovementPlugin (path_index advancement)
//   path_cache: pathfinding_system only
#[derive(Component, Debug, Clone)]
pub struct PathfindingState {
    /// World-space waypoints produced by A*.
    pub path: Vec<Vec3>,
    /// Index of the next waypoint to head toward.
    pub path_index: usize,
    /// Bevy elapsed time of the last pathfinding failure (NEG_INFINITY = no failure yet).
    pub last_pathfinding_failure: f32,
    /// Target that triggered the last failure — cleared when a new target arrives.
    pub last_failed_target: Option<Vec3>,
    /// Per-destination path cache. Key = goal grid coords. Value = (path, game-time stamp).
    pub path_cache: HashMap<(i32, i32), (Vec<Vec3>, f32)>,
}

impl Default for PathfindingState {
    fn default() -> Self {
        Self {
            path: Vec::new(),
            path_index: 0,
            last_pathfinding_failure: f32::NEG_INFINITY,
            last_failed_target: None,
            path_cache: HashMap::new(),
        }
    }
}

/// Tracks last known spatial grid cell for incremental updates.
// Owned by: CollisionPlugin (spatial_grid_update_system)
#[derive(Component, Debug, Clone)]
pub struct SpatialGridPosition {
    pub last_grid_coord: Option<crate::core::spatial_grid::GridCoord>,
    /// Set to true on spawn and whenever the entity moves to a new cell.
    pub dirty: bool,
}

impl Default for SpatialGridPosition {
    fn default() -> Self {
        Self { last_grid_coord: None, dirty: true }
    }
}

// Owned by: CollisionPlugin (spatial_grid_update_system, unit_collision_avoidance_system)
#[derive(Component, Debug, Clone)]
pub struct CollisionRadius {
    pub radius: f32,
}

impl Default for CollisionRadius {
    fn default() -> Self {
        Self { radius: 2.5 }
    }
}
