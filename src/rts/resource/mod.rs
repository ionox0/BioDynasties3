pub mod events;
pub mod gathering;

use crate::core::components::*;
use crate::core::resources::Stockpiles;
use bevy::prelude::*;
use self::events::*;
use self::gathering::gathering_system;

/// Supplementary state for a gathering unit (timing fields only).
/// Top-level activity is tracked via `UnitState`.
// Owned by: ResourceStatePlugin (update_gathering_states)
#[derive(Component, Debug, Clone, PartialEq)]
pub struct GatheringState {
    pub return_building: Option<Entity>,
    pub gather_start_time: f32,
    pub last_state_change: f32,
}

impl Default for GatheringState {
    fn default() -> Self {
        Self {
            return_building: None,
            gather_start_time: 0.0,
            last_state_change: 0.0,
        }
    }
}

pub struct ResourceStatePlugin;

impl Plugin for ResourceStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Stockpiles>()
            .add_event::<ClearTargetResourceEvent>()
            .add_event::<SetTargetResourceEvent>()
            .add_event::<ResetCargoEvent>()
            .add_event::<ResourceDepletedEvent>()
            .add_systems(
                Update,
                (
                    add_gathering_state_to_gatherers,
                    resource_state_system,
                    update_gathering_states,
                    gathering_system,
                )
                    .chain(),
            );
    }
}

/// Inserts `GatheringState` and `UnitState::Idle` on newly spawned gatherers.
fn add_gathering_state_to_gatherers(
    mut commands: Commands,
    new_gatherers: Query<Entity, Added<ResourceGatherer>>,
) {
    for entity in new_gatherers.iter() {
        commands.entity(entity).insert((GatheringState::default(), UnitState::Idle));
    }
}

/// Sole writer of `ResourceGatherer.target_resource` — applies all mutations via events.
pub fn resource_state_system(
    mut gatherers: Query<&mut ResourceGatherer>,
    mut clear_target_events: EventReader<ClearTargetResourceEvent>,
    mut set_target_events: EventReader<SetTargetResourceEvent>,
    mut reset_cargo_events: EventReader<ResetCargoEvent>,
    mut resource_depleted_events: EventReader<ResourceDepletedEvent>,
) {
    for event in resource_depleted_events.read() {
        for mut gatherer in gatherers.iter_mut() {
            if gatherer.target_resource == Some(event.resource_entity) {
                gatherer.target_resource = None;
                if gatherer.carried_amount == 0.0 {
                    gatherer.resource_type = None;
                }
            }
        }
    }

    for event in clear_target_events.read() {
        let Ok(mut gatherer) = gatherers.get_mut(event.gatherer) else {
            continue;
        };
        gatherer.target_resource = None;
        if event.clear_resource_type && gatherer.carried_amount == 0.0 {
            gatherer.resource_type = None;
        }
    }

    for event in set_target_events.read() {
        let Ok(mut gatherer) = gatherers.get_mut(event.gatherer) else {
            continue;
        };
        gatherer.target_resource = Some(event.target_resource);
        gatherer.resource_type = Some(event.resource_type.clone());
    }

    for event in reset_cargo_events.read() {
        let Ok(mut gatherer) = gatherers.get_mut(event.gatherer) else {
            continue;
        };
        gatherer.carried_amount = 0.0;
        gatherer.resource_type = None;
        // target_resource is intentionally preserved — tick_gathering uses it to send the
        // worker back to the resource automatically. It is cleared separately via
        // ClearTargetResourceEvent (e.g. when the resource is depleted).
    }
}

/// Derives `UnitState` from `ResourceGatherer` each frame.
/// Sole gathering-domain writer of `UnitState`. Skips if `UnitState` is a combat or Moving variant.
fn update_gathering_states(
    mut commands: Commands,
    mut query: Query<(Entity, &mut GatheringState, &ResourceGatherer, Option<&Movement>, &UnitState), Without<Dying>>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for (entity, mut gs, gatherer, movement, current_state) in query.iter_mut() {
        if is_combat_or_moving_state(current_state) {
            continue;
        }
        let new_state = derive_gathering_state(gatherer, movement);
        if new_state != *current_state {
            gs.last_state_change = now;
            commands.entity(entity).insert(new_state);
        }
    }
}

fn is_combat_or_moving_state(state: &UnitState) -> bool {
    matches!(
        state,
        UnitState::Moving
            | UnitState::InCombat
            | UnitState::MovingToAttack
            | UnitState::MovingToCombat
    )
}

fn derive_gathering_state(
    gatherer: &ResourceGatherer,
    movement: Option<&Movement>,
) -> UnitState {
    let is_moving = movement.is_some_and(|m| m.target_position.is_some());

    if gatherer.carried_amount > 0.0
        && (gatherer.carried_amount >= gatherer.capacity || gatherer.target_resource.is_none())
    {
        return if is_moving {
            UnitState::ReturningToBase
        } else {
            UnitState::DeliveringResources
        };
    }

    if gatherer.target_resource.is_some() {
        return if is_moving {
            UnitState::MovingToResource
        } else {
            UnitState::Gathering
        };
    }

    UnitState::Idle
}
