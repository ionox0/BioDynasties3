use bevy::prelude::*;
use std::sync::Arc;
use crate::rts::movement::pathfinding::flow_field::FlowField;

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

/// How a unit gets to its `Movement.target_position`.
#[derive(Clone, Default)]
pub enum Route {
    /// Straight line to the target (clear line of sight, or no navigation grid yet).
    #[default]
    Direct,
    /// Waiting for its move order's flow field to finish building.
    Pending,
    /// Following a flow field shared by every unit in the same move order.
    Field(Arc<FlowField>),
}

// Owned by: MovementPlugin (apply_movement_targets, poll_flow_fields)
#[derive(Component, Clone, Default)]
pub struct PathfindingState {
    pub route: Route,
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
