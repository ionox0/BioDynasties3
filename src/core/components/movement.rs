use bevy::prelude::*;
use std::sync::Arc;
use crate::rts::movement::navigation::flow_field::FlowField;

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
// Owned by: MovementPlugin (apply_movement_targets, poll_flow_fields)
#[derive(Component, Clone, Default)]
pub enum Route {
    /// Straight line to the target (clear line of sight, or no navigation grid yet).
    #[default]
    Direct,
    /// Waiting for its move order's flow field to finish building.
    Pending,
    /// Following a flow field shared by every unit in the same move order.
    Field(Arc<FlowField>),
}

/// The proximity-grid cell a unit was last filed under, so it's only re-filed when it changes cell.
// Owned by: CollisionPlugin (update_unit_proximity)
#[derive(Component, Debug, Clone)]
pub struct ProximityCellTracker {
    pub last_cell: Option<crate::core::proximity_grid::ProximityCell>,
    /// Set to true on spawn and whenever the entity moves to a new cell.
    pub dirty: bool,
}

impl Default for ProximityCellTracker {
    fn default() -> Self {
        Self { last_cell: None, dirty: true }
    }
}

// Owned by: CollisionPlugin (update_unit_proximity, separate_units)
#[derive(Component, Debug, Clone)]
pub struct CollisionRadius {
    pub radius: f32,
}

impl Default for CollisionRadius {
    fn default() -> Self {
        Self { radius: 2.5 }
    }
}
