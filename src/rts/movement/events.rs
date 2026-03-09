use bevy::prelude::*;

/// Fired to assign a movement destination to a unit.
/// Consumed by MovementPlugin (apply_movement_targets).
// Owned by: MovementPlugin (apply_movement_targets)
#[derive(Event, Debug, Clone)]
pub struct MovementTargetEvent {
    pub entity: Entity,
    pub target_position: Vec3,
}

/// Fired to halt a unit in place (clear target_position and velocity).
/// Consumed by MovementPlugin (stop_unit_movement).
// Owned by: MovementPlugin (stop_unit_movement)
#[derive(Event, Debug, Clone)]
pub struct StopMovementEvent {
    pub entity: Entity,
}
