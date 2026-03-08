//! Resource gathering state management.
//!
//! Owns `ResourceGatherer` and `GatheringState` components.
//! All mutations go through events defined in `events`.

pub mod construction;
pub mod events;

use crate::core::components::*;
use bevy::prelude::*;
use self::events::*;

pub struct ResourceStatePlugin;

impl Plugin for ResourceStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ClearTargetResourceEvent>()
            .add_event::<SetTargetResourceEvent>()
            .add_event::<ClearMovementEvent>()
            .add_event::<ResetCargoEvent>()
            .add_systems(
                Update,
                (
                    add_gathering_state_to_gatherers,
                    resource_state_system,
                    update_gathering_states,
                )
                    .chain(),
            );
    }
}

/// Inserts `GatheringState` on newly spawned gatherers.
fn add_gathering_state_to_gatherers(
    mut commands: Commands,
    new_gatherers: Query<Entity, Added<ResourceGatherer>>,
) {
    for entity in new_gatherers.iter() {
        commands.entity(entity).insert(GatheringState::default());
    }
}

/// Sole writer of `ResourceGatherer` — applies all mutations via events.
/// Movement/pathfinding clearing is handled by MovementPlugin (ClearMovementEvent consumer).
pub fn resource_state_system(
    mut gatherers: Query<&mut ResourceGatherer>,
    mut clear_target_events: EventReader<ClearTargetResourceEvent>,
    mut set_target_events: EventReader<SetTargetResourceEvent>,
    mut reset_cargo_events: EventReader<ResetCargoEvent>,
) {
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
        gatherer.target_resource = None;
    }
}

/// Derives `GatheringState` from `ResourceGatherer` each frame.
/// Sole writer of `GatheringState`.
fn update_gathering_states(
    mut query: Query<(&mut GatheringState, &ResourceGatherer, Option<&Movement>)>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for (mut state, gatherer, movement) in query.iter_mut() {
        let new_state = derive_gathering_state(gatherer, movement);
        if new_state != state.state {
            state.state = new_state;
            state.last_state_change = now;
        }
    }
}

fn derive_gathering_state(
    gatherer: &ResourceGatherer,
    movement: Option<&Movement>,
) -> GatheringStateType {
    let is_moving = movement.is_some_and(|m| m.target_position.is_some());

    if gatherer.carried_amount > 0.0
        && (gatherer.carried_amount >= gatherer.capacity || gatherer.target_resource.is_none())
    {
        return if is_moving {
            GatheringStateType::ReturningToBase
        } else {
            GatheringStateType::DeliveringResources
        };
    }

    if gatherer.target_resource.is_some() {
        return if is_moving {
            GatheringStateType::MovingToResource
        } else {
            GatheringStateType::Gathering
        };
    }

    GatheringStateType::Idle
}
