//! Production goal generation — queues workers and military units at a fixed 5:1 ratio.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::core::components::{Building, BuildingType, ProductionQueue, UnitType};
use crate::core::resources::Stockpiles;
use crate::ai::goals::types::{GlobalGoalManager, UnifiedGoal};

const WORKER_PRIORITY: f32 = 5.0;
const MILITARY_PRIORITY: f32 = 60.0;
const WORKERS_PER_EVAL: u32 = 5;

#[derive(SystemParam)]
pub(crate) struct ProductionParams<'w, 's> {
    pub(crate) buildings: Query<'w, 's, (Entity, &'static Building), With<ProductionQueue>>,
    pub(crate) stockpiles: Res<'w, Stockpiles>,
}

/// Generates `BuildUnit` goals at a 5:1 worker:military ratio per eval tick.
pub fn generate_production_goals(goals: &mut GlobalGoalManager, params: &ProductionParams) {
    for (player_id, stockpile) in params.stockpiles.0.iter() {
        if *player_id < 2 {
            continue;
        }
        push_unit_goals(goals, &params.buildings, *player_id, stockpile.nectar);
    }
}

fn push_unit_goals(
    goals: &mut GlobalGoalManager,
    buildings: &Query<(Entity, &Building), With<ProductionQueue>>,
    player_id: u8,
    available_nectar: f32,
) {
    let worker_type = UnitType::Fourmi;
    if available_nectar >= worker_type.build_cost_nectar() {
        if let Some(entity) = find_building(buildings, player_id, worker_type.required_building()) {
            for _ in 0..WORKERS_PER_EVAL {
                goals.push(WORKER_PRIORITY, UnifiedGoal::BuildUnit {
                    building: entity,
                    unit_type: worker_type.clone(),
                    player_id,
                });
            }
        }
    }

    for unit_type in early_military_roster() {
        let Some(entity) = find_building(buildings, player_id, unit_type.required_building()) else {
            continue;
        };
        if available_nectar < unit_type.build_cost_nectar() {
            continue;
        }
        goals.push(MILITARY_PRIORITY, UnifiedGoal::BuildUnit {
            building: entity,
            unit_type,
            player_id,
        });
        break;
    }
}

fn find_building(
    buildings: &Query<(Entity, &Building), With<ProductionQueue>>,
    player_id: u8,
    building_type: BuildingType,
) -> Option<Entity> {
    buildings
        .iter()
        .find(|(_, b)| b.player_id == player_id && b.is_complete && b.building_type == building_type)
        .map(|(e, _)| e)
}

fn early_military_roster() -> Vec<UnitType> {
    vec![
        UnitType::Scorpion,
        UnitType::RolyPoly,
        UnitType::GoliathBirdeater,
        UnitType::RhinoBeetle,
        UnitType::CairnsBirdwing,
        UnitType::Dragonfly,
    ]
}
