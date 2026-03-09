//! Production goal generation — queues new workers when the AI can afford them.

use bevy::prelude::*;
use crate::core::components::{Building, ProductionQueue, RTSUnit, UnitType};
use crate::core::resources::Stockpiles;
use crate::ai::goals::types::{GlobalGoalManager, UnifiedGoal};

const MAX_AI_WORKERS: u32 = 5;
const BUILD_UNIT_PRIORITY: f32 = 5.0;

/// Generates `BuildUnit` goals for AI players that can afford a new worker
/// and haven't yet reached the worker cap.
pub fn production_goal_system(
    mut goals: ResMut<GlobalGoalManager>,
    units: Query<&RTSUnit>,
    buildings: Query<(Entity, &Building), With<ProductionQueue>>,
    stockpiles: Res<Stockpiles>,
) {
    for (player_id, stockpile) in stockpiles.0.iter() {
        if *player_id < 2 {
            continue;
        }
        let unit_type = UnitType::WorkerAnt;
        if stockpile.nectar < unit_type.build_cost_nectar() {
            continue;
        }
        let worker_count = units
            .iter()
            .filter(|u| u.player_id == *player_id && u.unit_type.as_ref().is_some_and(UnitType::is_worker))
            .count() as u32;
        if worker_count >= MAX_AI_WORKERS {
            continue;
        }
        let Some((building_entity, _)) = buildings
            .iter()
            .find(|(_, b)| b.player_id == *player_id && b.is_complete && b.building_type == unit_type.required_building())
        else {
            continue;
        };
        goals.push(
            BUILD_UNIT_PRIORITY,
            UnifiedGoal::BuildUnit { building: building_entity, unit_type, player_id: *player_id },
        );
    }
}
