use bevy::prelude::*;

/// Fired to assign a movement destination to a unit.
/// Consumed by MovementPlugin (apply_movement_targets).
// Owned by: MovementPlugin (apply_movement_targets)
#[derive(Event, Debug, Clone)]
pub struct MovementTargetEvent {
    pub entity: Entity,
    pub target_position: Vec3,
}
