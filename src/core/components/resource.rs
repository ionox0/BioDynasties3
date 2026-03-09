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

/// Derived state for a gathering unit. Recomputed every frame — never set directly.
// Owned by: ResourceStatePlugin (update_gathering_states)
#[derive(Component, Debug, Clone, PartialEq)]
pub struct GatheringState {
    pub state: GatheringStateType,
    pub return_building: Option<Entity>,
    pub gather_start_time: f32,
    pub last_state_change: f32,
}

impl Default for GatheringState {
    fn default() -> Self {
        Self {
            state: GatheringStateType::Idle,
            return_building: None,
            gather_start_time: 0.0,
            last_state_change: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GatheringStateType {
    /// Unit is idle, waiting for work assignment
    Idle,
    /// Moving to a resource to start gathering
    MovingToResource,
    /// Actively gathering from a resource
    Gathering,
    /// Moving back to base with gathered resources
    ReturningToBase,
    /// Delivering resources to a building
    DeliveringResources,
}
