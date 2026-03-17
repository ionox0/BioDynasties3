//! Goal executors — consume `GlobalGoalManager` and fire events.
//! No direct component mutations; all changes go through existing events.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::core::components::ResourceSource;
use crate::core::resources::Stockpiles;
use crate::rts::resource::events::SetTargetResourceEvent;
use crate::rts::movement::events::MovementTargetEvent;
use crate::rts::production::QueueProductionEvent;
use crate::rts::combat::events::CombatTargetEvent;
use super::types::{GlobalGoalManager, UnifiedGoal};

/// Grouped event writers used by executor helpers.
#[derive(SystemParam)]
pub struct AiGoalWriters<'w> {
    pub set_target: EventWriter<'w, SetTargetResourceEvent>,
    pub move_events: EventWriter<'w, MovementTargetEvent>,
    pub queue_production: EventWriter<'w, QueueProductionEvent>,
    pub combat_target: EventWriter<'w, CombatTargetEvent>,
}

/// Drains `GlobalGoalManager` each frame and executes every goal via events.
pub fn execute_ai_goals_system(
    mut goals: ResMut<GlobalGoalManager>,
    mut writers: AiGoalWriters,
    mut stockpiles: ResMut<Stockpiles>,
    resources: Query<&ResourceSource>,
) {
    for pg in goals.drain_sorted() {
        match &pg.goal {
            UnifiedGoal::AssignWorkerToResource { .. } => {
                execute_assign_worker(&pg.goal, &mut writers, &resources);
            }
            UnifiedGoal::BuildUnit { .. } => {
                execute_build_unit(&pg.goal, &mut writers, &mut stockpiles);
            }
            UnifiedGoal::AttackTarget { attacker, target } => {
                writers.combat_target.send(CombatTargetEvent {
                    attacker: *attacker,
                    target: *target,
                    move_to_range: true,
                });
            }
        }
    }
}

fn execute_assign_worker(
    goal: &UnifiedGoal,
    writers: &mut AiGoalWriters,
    resources: &Query<&ResourceSource>,
) {
    let UnifiedGoal::AssignWorkerToResource {
        worker,
        resource_entity,
        resource_type,
        resource_pos,
    } = goal
    else {
        return;
    };
    let is_available = resources
        .get(*resource_entity)
        .map(|r| r.amount > 0.0)
        .unwrap_or(false);
    if !is_available {
        return;
    }
    writers.set_target.send(SetTargetResourceEvent {
        gatherer: *worker,
        target_resource: *resource_entity,
        resource_type: resource_type.clone(),
    });
    writers.move_events.send(MovementTargetEvent {
        entity: *worker,
        target_position: *resource_pos,
    });
}

fn execute_build_unit(
    goal: &UnifiedGoal,
    writers: &mut AiGoalWriters,
    stockpiles: &mut Stockpiles,
) {
    let UnifiedGoal::BuildUnit { building, unit_type, player_id } = goal else { return };
    let cost = unit_type.build_cost_nectar();
    let stockpile = stockpiles.get_or_insert_mut(*player_id);
    if stockpile.nectar < cost {
        return;
    }
    stockpile.nectar -= cost;
    writers.queue_production.send(QueueProductionEvent {
        building: *building,
        unit_type: unit_type.clone(),
    });
}
