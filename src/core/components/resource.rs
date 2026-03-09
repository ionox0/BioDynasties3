use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Nectar,
    Chitin,
    Minerals,
    Pheromones,
}

/// Per-unit cargo and targeting state for resource gathering.
/// See `rts::resource::gathering` for the full ownership table and gather-cycle diagram.
#[derive(Component, Debug, Clone)]
pub struct ResourceGatherer {
    pub gather_rate: f32,
    pub capacity: f32,
    pub carried_amount: f32,
    pub resource_type: Option<ResourceType>,
    pub target_resource: Option<Entity>,
}

/// Finite resource pool in the world. Depleted by `gathering_system`.
// Owned by: GatheringPlugin (gathering_system) — amount decremented on gather, despawned when depleted
#[derive(Component, Debug, Clone)]
pub struct ResourceSource {
    pub resource_type: ResourceType,
    pub amount: f32,
}
