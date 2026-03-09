//! Worker goal generation — assigns idle AI workers to resource nodes.

use bevy::prelude::*;
use crate::core::components::{
    GatheringState, GatheringStateType, ResourceGatherer, ResourceSource, RTSUnit, UnitType,
};
use crate::core::components::ResourceType;
use crate::ai::goals::types::{GlobalGoalManager, UnifiedGoal};

const WORKER_ASSIGNMENT_PRIORITY: f32 = 10.0;

/// Finds idle AI workers and generates `AssignWorkerToResource` goals.
pub fn worker_goal_system(
    mut goals: ResMut<GlobalGoalManager>,
    workers: Query<(Entity, &RTSUnit, &GatheringState, &Transform), With<ResourceGatherer>>,
    resources: Query<(Entity, &ResourceSource, &Transform), Without<RTSUnit>>,
) {
    for (entity, unit, state, transform) in workers.iter() {
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
            find_nearest_resource(transform.translation, &resources)
        else {
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
    resources: &Query<(Entity, &ResourceSource, &Transform), Without<RTSUnit>>,
) -> Option<(Entity, ResourceType, Vec3)> {
    resources
        .iter()
        .filter(|(_, r, _)| r.amount > 0.0)
        .min_by(|a, b| {
            a.2.translation
                .distance(pos)
                .partial_cmp(&b.2.translation.distance(pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(e, r, tf)| (e, r.resource_type.clone(), tf.translation))
}
