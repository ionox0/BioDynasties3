//! Production goal generation — queues workers and military units at a fixed 5:1 ratio.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::seq::SliceRandom;
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
        push_unit_goals(goals, &params.buildings, *player_id, stockpile.nectar, worker_type_for_player(*player_id));
    }
}

fn worker_type_for_player(player_id: u8) -> UnitType {
    match player_id {
        2 => UnitType::Bee,
        _ => UnitType::Fourmi,
    }
}

fn push_unit_goals(
    goals: &mut GlobalGoalManager,
    buildings: &Query<(Entity, &Building), With<ProductionQueue>>,
    player_id: u8,
    available_nectar: f32,
    worker_type: UnitType,
) {
    let mut budget = available_nectar;

    // Military executes first (higher priority), so deduct it from budget first.
    let mut roster = early_military_roster();
    roster.shuffle(&mut rand::thread_rng());
    for unit_type in roster {
        let cost = unit_type.build_cost_nectar();
        if budget < cost { continue; }
        let Some(entity) = find_building(buildings, player_id, unit_type.required_building()) else { continue };
        goals.push(MILITARY_PRIORITY, UnifiedGoal::BuildUnit { building: entity, unit_type, player_id });
        budget -= cost;
        break;
    }

    // Workers get whatever budget remains.
    let worker_cost = worker_type.build_cost_nectar();
    if let Some(entity) = find_building(buildings, player_id, worker_type.required_building()) {
        let affordable = ((budget / worker_cost) as u32).min(WORKERS_PER_EVAL);
        for _ in 0..affordable {
            goals.push(WORKER_PRIORITY, UnifiedGoal::BuildUnit {
                building: entity,
                unit_type: worker_type.clone(),
                player_id,
            });
        }
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
