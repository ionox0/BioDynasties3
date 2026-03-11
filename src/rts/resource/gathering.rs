//! Resource gathering subsystem.
//!
//! ## Component ownership
//!
//! | Component field                    | Owner                     | Mutation mechanism       |
//! |------------------------------------|---------------------------|--------------------------|
//! | `ResourceGatherer.target_resource` | `resource_state_system`   | Events (Set/ClearTarget) |
//! | `ResourceGatherer.carried_amount`  | `gathering_system`        | Direct write             |
//! | `ResourceGatherer.resource_type`   | `gathering_system`        | Direct write             |
//! | `GatheringState`                   | `update_gathering_states` | Derived each frame       |
//! | `ResourceSource.amount`            | `gathering_system`        | Direct write             |
//!
//! ## Gather cycle
//!
//! The trigger is the only player/AI difference — the loop below is shared.
//!
//! ```text
//!  [unit_commands right-click  OR  AI goal system]
//!  SetTargetResourceEvent + MovementTargetEvent(resource_pos)
//!                    │
//!                    ▼
//!          ┌──────────────────┐ ◄─────────────────────────────────────────┐
//!          │ MovingToResource │                                           │
//!          └────────┬─────────┘                                           │
//!                   │ movement cleared on arrival                         │
//!                   ▼                                                     │
//!            ┌────────────┐                                               │
//!            │  Gathering │  carried_amount += gather_rate * dt           │
//!            └──────┬─────┘  resource.amount  -= amount                   │
//!                   │                                                     │
//!           ┌───────┴────────────┐                                        │
//!     at capacity             depleted                                    │
//!           │                    └──► ClearTargetResourceEvent            │
//!           │                                  │                          │
//!           │                                  ▼                          │
//!           │                              ┌──────┐                       │
//!           │                              │ Idle │                       │
//!           │                              └──────┘                       │
//!           │ MovementTargetEvent(building_pos)                           │
//!           ▼                                                             │
//!  ┌─────────────────┐                                                    │
//!  │ ReturningToBase │                                                    │
//!  └────────┬────────┘                                                    │
//!           │ movement cleared on arrival                                 │
//!           ▼                                                             │
//!  ┌──────────────────────┐                                               │
//!  │  DeliveringResources │  Stockpiles += carried_amount                 │
//!  └──────────┬───────────┘  ResetCargoEvent  (cargo → 0)                 │
//!             │              target_resource preserved                    │
//!             │ MovementTargetEvent(resource_pos)                         │
//!             └───────────────────────────────────────────────────────────┘
//! ```
//!
//! `GatheringState` is **derived each frame** from `ResourceGatherer` + `Movement` —
//! never written directly. The state drives which branch of `gathering_system` runs.
//!
//! ## System order (each Update frame)
//!
//! ```text
//! add_gathering_state_to_gatherers
//!   → resource_state_system      (applies Set/Clear/Reset events → writes ResourceGatherer)
//!   → update_gathering_states    (derives GatheringState from ResourceGatherer + Movement)
//!   → gathering_system           (reads GatheringState, accumulates cargo, sends movement events)
//! ```

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::core::components::*;
use super::{GatheringState, GatheringStateType};
use crate::core::constants::resource_interaction::{GATHERING_DISTANCE, DROPOFF_TRAVEL_DISTANCE};
use crate::core::resources::Stockpiles;
use super::events::{ClearTargetResourceEvent, ResetCargoEvent, ResourceDepletedEvent};
use crate::rts::movement::events::MovementTargetEvent;

type GathererQuery<'w, 's> = Query<
    'w, 's,
    (Entity, &'static mut ResourceGatherer, &'static GatheringState, &'static Transform, &'static RTSUnit),
>;

type ResourceQuery<'w, 's> = Query<
    'w, 's,
    (&'static mut ResourceSource, &'static Transform),
    Without<RTSUnit>,
>;

#[derive(SystemParam)]
pub(super) struct GatheringCtx<'w, 's> {
    stockpiles: ResMut<'w, Stockpiles>,
    time: Res<'w, Time>,
    buildings: Query<'w, 's, (Entity, &'static Transform, &'static Building)>,
    move_events: EventWriter<'w, MovementTargetEvent>,
    clear_target_events: EventWriter<'w, ClearTargetResourceEvent>,
    reset_cargo_events: EventWriter<'w, ResetCargoEvent>,
    resource_depleted_events: EventWriter<'w, ResourceDepletedEvent>,
}

/// Core gathering loop — runs each frame for all active gatherers.
pub(super) fn gathering_system(
    mut gatherers: GathererQuery,
    mut resources: ResourceQuery,
    mut ctx: GatheringCtx,
) {
    for (entity, mut gatherer, state, transform, unit) in gatherers.iter_mut() {
        match state.state {
            GatheringStateType::Gathering => {
                tick_gathering(entity, &mut gatherer, transform, &mut resources, &mut ctx);
            }
            GatheringStateType::DeliveringResources => {
                tick_delivery(entity, &gatherer, unit, transform, &mut ctx);
            }
            _ => {}
        }
    }
}

/// Accumulates cargo from the target resource each frame.
/// If the worker is too far away, sends a MovementTargetEvent to approach.
fn tick_gathering(
    entity: Entity,
    gatherer: &mut ResourceGatherer,
    transform: &Transform,
    resources: &mut ResourceQuery,
    ctx: &mut GatheringCtx,
) {
    let Some(resource_entity) = gatherer.target_resource else { return };
    let Ok((mut resource, resource_tf)) = resources.get_mut(resource_entity) else {
        ctx.clear_target_events.send(ClearTargetResourceEvent {
            gatherer: entity,
            clear_resource_type: gatherer.carried_amount == 0.0,
        });
        return;
    };

    let dist = (transform.translation.xz() - resource_tf.translation.xz()).length();
    if dist > GATHERING_DISTANCE {
        // Not at resource yet — send move command (handles post-delivery return too).
        ctx.move_events.send(MovementTargetEvent { entity, target_position: resource_tf.translation });
        return;
    }
    if gatherer.carried_amount >= gatherer.capacity { return; }

    let dt = ctx.time.delta_secs();
    let gather_amt = (gatherer.gather_rate * dt)
        .min(gatherer.capacity - gatherer.carried_amount)
        .min(resource.amount);

    gatherer.carried_amount += gather_amt;
    gatherer.resource_type = Some(resource.resource_type.clone());
    resource.amount -= gather_amt;

    if resource.amount <= 0.0 {
        resource.amount = 0.0;
        ctx.resource_depleted_events.send(ResourceDepletedEvent { resource_entity });
    }
}

/// Deposits cargo into the player's stockpile when the worker reaches a dropoff building.
/// If not yet at the building, sends a MovementTargetEvent to travel there.
fn tick_delivery(
    entity: Entity,
    gatherer: &ResourceGatherer,
    unit: &RTSUnit,
    transform: &Transform,
    ctx: &mut GatheringCtx,
) {
    let Some((_, building_pos)) = find_nearest_dropoff(unit.player_id, transform.translation, &ctx.buildings) else {
        return; // No valid building yet — worker waits.
    };

    let dist = (transform.translation.xz() - building_pos.xz()).length();
    if dist > DROPOFF_TRAVEL_DISTANCE {
        ctx.move_events.send(MovementTargetEvent { entity, target_position: building_pos });
        return;
    }

    // At building — deposit and reset cargo.
    // Worker will be in Gathering state next frame (cargo=0, target_resource still set),
    // and tick_gathering will send them back to the resource automatically.
    if let Some(resource_type) = &gatherer.resource_type {
        ctx.stockpiles
            .get_or_insert_mut(unit.player_id)
            .add(resource_type, gatherer.carried_amount);
    }
    ctx.reset_cargo_events.send(ResetCargoEvent { gatherer: entity });
}

/// Returns the nearest complete dropoff building owned by `player_id`, if any.
fn find_nearest_dropoff(
    player_id: u8,
    pos: Vec3,
    buildings: &Query<(Entity, &Transform, &Building)>,
) -> Option<(Entity, Vec3)> {
    buildings
        .iter()
        .filter(|(_, _, b)| {
            b.player_id == player_id
                && b.is_complete
                && matches!(b.building_type, BuildingType::Queen | BuildingType::Nursery)
        })
        .map(|(e, tf, _)| (e, tf.translation))
        .min_by(|a, b| {
            a.1.distance(pos)
                .partial_cmp(&b.1.distance(pos))
                .unwrap()
        })
}
