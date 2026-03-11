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

/// Event sent to reset a gatherer's cargo after delivery
#[derive(Event, Debug, Clone)]
pub struct ResetCargoEvent {
    pub gatherer: Entity,
}

/// Broadcast when a resource node is fully depleted.
/// `resource_state_system` clears `target_resource` on every gatherer that targets this entity.
#[derive(Event, Debug, Clone)]
pub struct ResourceDepletedEvent {
    pub resource_entity: Entity,
}
