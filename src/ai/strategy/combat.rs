//! Combat goal generation — orders idle AI units to attack enemy players.

use std::collections::HashMap;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::seq::SliceRandom;
use crate::core::components::{Combat, RTSUnit, UnitState};
use crate::ai::goals::types::{GlobalGoalManager, UnifiedGoal};
use crate::rts::combat::TargetGrid;

/// Fraction of idle AI units committed to each attack wave.
const WAVE_COMMIT_FRACTION: f32 = 0.75;
const ATTACK_PRIORITY: f32 = 70.0;

#[derive(SystemParam)]
pub(crate) struct CombatParams<'w, 's> {
    #[allow(clippy::type_complexity)]
    pub(crate) idle_ai: Query<'w, 's, (Entity, &'static RTSUnit, &'static UnitState, &'static Combat, &'static Transform)>,
    pub(crate) target_grid: Res<'w, TargetGrid>,
}

/// Orders idle AI units to attack an enemy player's units.
pub fn generate_combat_goals(goals: &mut GlobalGoalManager, params: &CombatParams) {
    let mut by_player: HashMap<u8, Vec<(Entity, Vec3)>> = HashMap::new();
    for (entity, unit, state, combat, transform) in params.idle_ai.iter() {
        if unit.player_id >= 2
            && *state == UnitState::Idle
            && combat.target.is_none()
            && !goals.has_goal_for(entity)
        {
            by_player.entry(unit.player_id).or_default().push((entity, transform.translation));
        }
    }

    for (player_id, mut candidates) in by_player {
        candidates.shuffle(&mut rand::thread_rng());
        let commit_count = ((candidates.len() as f32) * WAVE_COMMIT_FRACTION).ceil() as usize;
        for (attacker, pos) in candidates.into_iter().take(commit_count) {
            let nearby = params.target_grid.query_nearby(pos, f32::MAX);
            let target = nearby.into_iter().find(|(_, _, pid)| *pid != player_id).map(|(e, _, _)| e)
                .or_else(|| params.target_grid.0.cells.values().flatten()
                    .find(|(_, (_, pid))| *pid != player_id).map(|(e, _)| *e));
            let Some(target) = target else { continue };
            goals.push(ATTACK_PRIORITY, UnifiedGoal::AttackTarget { attacker, target });
        }
    }
}
