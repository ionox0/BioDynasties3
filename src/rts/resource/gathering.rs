use bevy::prelude::*;

use crate::core::components::*;
use crate::core::constants::resource_interaction::{
    GATHERING_DISTANCE,
    DROPOFF_TRAVEL_DISTANCE,
};
use crate::core::resources::Stockpiles;
use crate::rts::movement::events::MovementTargetEvent;

pub fn gathering_system(
    mut gatherers: Query<(Entity, &mut ResourceGatherer, &Transform, &RTSUnit)>,
    mut resources: Query<(&mut ResourceSource, &Transform), Without<RTSUnit>>,
    buildings: Query<(Entity, &Transform, &Building)>,
    time: Res<Time>,
    mut stockpiles: ResMut<Stockpiles>,
    mut move_events: EventWriter<MovementTargetEvent>,
) {
    for (entity, mut gatherer, transform, unit) in gatherers.iter_mut() {
        let Some(resource_entity) = gatherer.target_resource else {
            continue;
        };

        let Ok((mut resource, resource_tf)) = resources.get_mut(resource_entity) else {
            // resource disappeared
            gatherer.target_resource = None;
            continue;
        };

        let pos = transform.translation;
        let resource_pos = resource_tf.translation;

        //
        // STATE 1: Returning to base
        //
        if gatherer.carried_amount >= gatherer.capacity {
            let Some((_, building_pos)) =
                find_nearest_dropoff(unit.player_id, pos, &buildings)
            else {
                continue;
            };

            let dist = pos.distance(building_pos);

            if dist > DROPOFF_TRAVEL_DISTANCE {
                move_events.send(MovementTargetEvent {
                    entity,
                    target_position: building_pos,
                });
                continue;
            }

            // deposit
            if let Some(resource_type) = &gatherer.resource_type {
                stockpiles
                    .get_or_insert_mut(unit.player_id)
                    .add(resource_type, gatherer.carried_amount);
            }

            gatherer.carried_amount = 0.0;

            // worker will automatically go back to resource next frame
            continue;
        }

        //
        // STATE 2: Moving to resource
        //
        let dist = pos.distance(resource_pos);

        if dist > GATHERING_DISTANCE {
            move_events.send(MovementTargetEvent {
                entity,
                target_position: resource_pos,
            });
            continue;
        }

        //
        // STATE 3: Gathering
        //
        let dt = time.delta_secs();

        let gather_amt = (gatherer.gather_rate * dt)
            .min(gatherer.capacity - gatherer.carried_amount)
            .min(resource.amount);

        if gather_amt <= 0.0 {
            continue;
        }

        gatherer.carried_amount += gather_amt;
        gatherer.resource_type = Some(resource.resource_type.clone());
        resource.amount -= gather_amt;

        // resource depleted
        if resource.amount <= 0.0 {
            resource.amount = 0.0;
            gatherer.target_resource = None;
        }
    }
}

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
                && matches!(
                    b.building_type,
                    BuildingType::Queen | BuildingType::Nursery
                )
        })
        .map(|(e, tf, _)| (e, tf.translation))
        .min_by(|a, b| {
            a.1.distance(pos)
                .partial_cmp(&b.1.distance(pos))
                .unwrap()
        })
}