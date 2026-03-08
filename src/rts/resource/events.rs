use bevy::prelude::*;
use crate::core::components::ResourceType;

/// Event sent to clear a gatherer's target resource (used when resource is depleted/despawned)
#[derive(Event, Debug, Clone)]
pub struct ClearTargetResourceEvent {
    pub gatherer: Entity,
    /// Whether to also clear resource_type (only if gatherer has no cargo)
    pub clear_resource_type: bool,
}

/// Event sent to set a gatherer's target resource
#[derive(Event, Debug, Clone)]
pub struct SetTargetResourceEvent {
    pub gatherer: Entity,
    pub target_resource: Entity,
    pub resource_type: ResourceType,
}

/// Event sent to clear a gatherer's movement and pathfinding state
#[derive(Event, Debug, Clone)]
pub struct ClearMovementEvent {
    pub gatherer: Entity,
}

/// Event sent to reset a gatherer's cargo after delivery
#[derive(Event, Debug, Clone)]
pub struct ResetCargoEvent {
    pub gatherer: Entity,
}
