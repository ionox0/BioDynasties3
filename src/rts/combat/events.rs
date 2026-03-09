//! Combat events.
//!
//! All cross-system combat state changes flow through these events.

use bevy::prelude::*;
use crate::core::components::DamageType;

/// Fired by `combat_execution_system` when an attacker lands a hit.
/// Consumed by `damage_resolution_system`.
#[derive(Event, Debug, Clone)]
pub struct DamageEvent {
    #[allow(dead_code)]
    pub attacker: Entity,
    pub target: Entity,
    pub damage: f32,
    pub damage_type: DamageType,
}

/// Fired by `damage_resolution_system` when a unit's health reaches zero.
/// Consumed by `death_system` (clear targets, despawn).
#[derive(Event, Debug, Clone)]
pub struct DeathEvent {
    pub entity: Entity,
}

/// Fired by external systems (AI, UI) to assign a combat target.
/// Consumed by `combat_target_handler`.
#[derive(Event, Debug, Clone)]
pub struct CombatTargetEvent {
    pub attacker: Entity,
    pub target: Entity,
    /// If true, also send a movement event to get in range.
    pub move_to_range: bool,
}

/// Fired to clear a unit's combat target and halt movement.
/// Consumed by `combat_stop_handler`.
#[derive(Event, Debug, Clone)]
pub struct CombatStopEvent {
    pub entity: Entity,
}
