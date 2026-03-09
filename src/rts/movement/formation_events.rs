//! Formation movement events.

use bevy::prelude::*;

/// Fired to move a group of units in a grid formation to a destination.
/// Consumed by `FormationPlugin` (apply_formation_move).
#[derive(Event, Debug, Clone)]
pub struct FormationMoveEvent {
    /// Units to include in the formation.
    pub units: Vec<Entity>,
    /// World-space destination for the formation centre.
    pub target: Vec3,
}
