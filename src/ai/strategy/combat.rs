//! Combat goal generation — orders idle AI units to attack enemy players.

use std::collections::HashMap;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::seq::SliceRandom;
use crate::core::components::{Building, Combat, Dying, RTSHealth, RTSUnit, UnitState};
use crate::ai::goals::types::{GlobalGoalManager, UnifiedGoal};

/// Fraction of idle AI units committed to each attack wave.
const WAVE_COMMIT_FRACTION: f32 = 0.75;
const ATTACK_PRIORITY: f32 = 70.0;

#[derive(SystemParam)]
pub(crate) struct CombatParams<'w, 's> {
    pub(crate) idle_ai: Query<'w, 's, (Entity, &'static RTSUnit, &'static UnitState, &'static Combat)>,
    #[allow(clippy::type_complexity)]
    pub(crate) player_units: Query<
        'w,
        's,
        (Entity, &'static RTSUnit, &'static UnitState),
        (With<RTSHealth>, Without<Dying>, Without<Building>),
    >,
}

/// Orders idle AI units to attack an enemy player's units.
pub fn generate_combat_goals(goals: &mut GlobalGoalManager, params: &CombatParams) {
    let mut by_player: HashMap<u8, Vec<Entity>> = HashMap::new();
    for (entity, unit, state, combat) in params.idle_ai.iter() {
        if unit.player_id >= 2
            && *state == UnitState::Idle
            && combat.target.is_none()
            && !goals.has_goal_for(entity)
        {
            by_player.entry(unit.player_id).or_default().push(entity);
        }
    }

    for (player_id, mut candidates) in by_player {
        let Some(target) = find_enemy_target(player_id, params) else { continue };
        candidates.shuffle(&mut rand::thread_rng());
        let commit_count = ((candidates.len() as f32) * WAVE_COMMIT_FRACTION).ceil() as usize;
        for attacker in candidates.into_iter().take(commit_count) {
            goals.push(ATTACK_PRIORITY, UnifiedGoal::AttackTarget { attacker, target });
        }
    }
}

fn find_enemy_target(attacker_id: u8, params: &CombatParams) -> Option<Entity> {
    params.player_units.iter()
        .find(|(_, u, state)| u.player_id != attacker_id && **state == UnitState::Idle)
        .map(|(e, _, _)| e)
}
