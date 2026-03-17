//! Production goal generation — queues workers and military units when the AI can afford them.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::core::components::{Building, ProductionQueue, RTSUnit, UnitType};
use crate::core::resources::Stockpiles;
use crate::ai::goals::types::{GlobalGoalManager, UnifiedGoal};

const MAX_AI_WORKERS: u32 = 5000;
const BUILD_UNIT_PRIORITY: f32 = 5.0;
const COMBAT_GOAL_PRIORITY_BASE: f32 = 60.0;

#[derive(SystemParam)]
pub(crate) struct ProductionParams<'w, 's> {
    pub(crate) units: Query<'w, 's, &'static RTSUnit>,
    pub(crate) buildings: Query<'w, 's, (Entity, &'static Building), With<ProductionQueue>>,
    pub(crate) stockpiles: Res<'w, Stockpiles>,
}

/// Generates `BuildUnit` goals for all AI-controlled unit types — workers and military.
pub fn generate_production_goals(goals: &mut GlobalGoalManager, params: &ProductionParams) {
    for (player_id, stockpile) in params.stockpiles.0.iter() {
        if *player_id < 2 {
            continue;
        }
        push_worker_goal(goals, &params.units, &params.buildings, *player_id, stockpile.nectar);
        let military_count = count_military_units(&params.units, *player_id);
        push_military_goals(
            goals,
            &params.buildings,
            *player_id,
            military_count,
            &early_military_roster(),
            stockpile.nectar,
        );
    }
}

fn push_worker_goal(
    goals: &mut GlobalGoalManager,
    units: &Query<&RTSUnit>,
    buildings: &Query<(Entity, &Building), With<ProductionQueue>>,
    player_id: u8,
    available_nectar: f32,
) {
    let unit_type = UnitType::Fourmi;
    if available_nectar < unit_type.build_cost_nectar() {
        return;
    }
    let worker_count = units
        .iter()
        .filter(|u| u.player_id == player_id && u.unit_type.as_ref().is_some_and(UnitType::is_worker))
        .count() as u32;
    if worker_count >= MAX_AI_WORKERS {
        return;
    }
    let Some((building_entity, _)) = buildings
        .iter()
        .find(|(_, b)| b.player_id == player_id && b.is_complete && b.building_type == unit_type.required_building())
    else {
        return;
    };
    goals.push(
        BUILD_UNIT_PRIORITY,
        UnifiedGoal::BuildUnit { building: building_entity, unit_type, player_id },
    );
}

fn push_military_goals(
    goals: &mut GlobalGoalManager,
    buildings: &Query<(Entity, &Building), With<ProductionQueue>>,
    player_id: u8,
    military_count: u32,
    roster: &[UnitType],
    available_nectar: f32,
) {
    let (targets, priority_base) = phase_config(military_count);

    for (i, unit_type) in roster.iter().enumerate().take(targets.len()) {
        let target_qty = targets[i];
        let cost = unit_type.build_cost_nectar();
        if available_nectar < cost {
            continue;
        }
        let building_type = unit_type.required_building();
        let Some((building_entity, _)) = buildings
            .iter()
            .find(|(_, b)| b.player_id == player_id && b.is_complete && b.building_type == building_type)
        else {
            continue;
        };
        let priority = priority_base - i as f32 * 1.5;
        for _ in 0..target_qty {
            goals.push(priority, UnifiedGoal::BuildUnit {
                building: building_entity,
                unit_type: unit_type.clone(),
                player_id,
            });
        }
    }
}

fn phase_config(military_count: u32) -> (&'static [u32], f32) {
    if military_count < 15 {
        (&[8, 6, 5, 4], COMBAT_GOAL_PRIORITY_BASE)
    } else if military_count < 30 {
        (&[12, 10, 8, 6, 5, 4, 3, 3], COMBAT_GOAL_PRIORITY_BASE + 28.0)
    } else {
        (&[20, 18, 15, 12, 10, 8, 6, 5, 4, 4], COMBAT_GOAL_PRIORITY_BASE + 27.0)
    }
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

fn count_military_units(units: &Query<&RTSUnit>, player_id: u8) -> u32 {
    units
        .iter()
        .filter(|u| u.player_id == player_id && u.unit_type.as_ref().is_some_and(|t| !t.is_worker()))
        .count() as u32
}
