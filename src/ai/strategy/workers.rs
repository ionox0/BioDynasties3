//! Worker goal generation — assigns idle AI workers to resource nodes.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::seq::SliceRandom;
use crate::core::components::{ResourceGatherer, ResourceSource, RTSUnit, UnitType};
use crate::rts::resource::{GatheringState, GatheringStateType};
use crate::core::components::ResourceType;
use crate::ai::goals::types::{GlobalGoalManager, UnifiedGoal};

const WORKER_ASSIGNMENT_PRIORITY: f32 = 10.0;

#[derive(SystemParam)]
pub(crate) struct IdleWorkers<'w, 's> {
    query: Query<'w, 's, (Entity, &'static RTSUnit, &'static GatheringState, &'static Transform), With<ResourceGatherer>>,
}

#[derive(SystemParam)]
pub(crate) struct AvailableResources<'w, 's> {
    query: Query<'w, 's, (Entity, &'static ResourceSource, &'static Transform), Without<RTSUnit>>,
}

/// Finds idle AI workers and generates `AssignWorkerToResource` goals.
pub fn worker_goal_system(
    mut goals: ResMut<GlobalGoalManager>,
    workers: IdleWorkers,
    resources: AvailableResources,
) {
    for (entity, unit, state, transform) in workers.query.iter() {
        if unit.player_id < 2 {
            continue;
        }
        if !unit.unit_type.as_ref().is_some_and(UnitType::is_worker) {
            continue;
        }
        if state.state != GatheringStateType::Idle {
            continue;
        }
        let Some((resource_entity, resource_type, resource_pos)) =
            find_nearest_resource(transform.translation, &resources.query)
        else {
            println!("Idle worker has no available resources to gather");
            continue;
        };
        goals.push(
            WORKER_ASSIGNMENT_PRIORITY,
            UnifiedGoal::AssignWorkerToResource {
                worker: entity,
                resource_entity,
                resource_type,
                resource_pos,
            },
        );
    }
}

fn find_nearest_resource(
    pos: Vec3,
    resources: &Query<(Entity, &'static ResourceSource, &'static Transform), Without<RTSUnit>>,
) -> Option<(Entity, ResourceType, Vec3)> {
    let mut candidates: Vec<_> = resources
        .iter()
        .filter(|(_, r, _)| r.amount > 0.0)
        .collect();
    candidates.sort_unstable_by(|a, b| {
        a.2.translation
            .distance(pos)
            .partial_cmp(&b.2.translation.distance(pos))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(5);
    let (e, r, tf) = candidates.choose(&mut rand::thread_rng())?;
    Some((*e, r.resource_type.clone(), tf.translation))
}
