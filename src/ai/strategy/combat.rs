//! Combat goal generation — orders idle AI units to attack the player.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::seq::SliceRandom;
use crate::core::components::{Building, BuildingType, Combat, Dying, RTSHealth, RTSUnit, UnitState};
use crate::ai::goals::types::{GlobalGoalManager, UnifiedGoal};

/// Fraction of idle AI units committed to each attack wave.
const WAVE_COMMIT_FRACTION: f32 = 0.75;
const ATTACK_PRIORITY: f32 = 70.0;

#[derive(SystemParam)]
pub(crate) struct CombatParams<'w, 's> {
    pub(crate) idle_ai: Query<'w, 's, (Entity, &'static RTSUnit, &'static UnitState, &'static Combat)>,
    #[allow(clippy::type_complexity)]
    pub(crate) player_buildings: Query<
        'w,
        's,
        (Entity, &'static Building),
        (With<RTSHealth>, Without<Dying>),
    >,
    #[allow(clippy::type_complexity)]
    pub(crate) player_units: Query<
        'w,
        's,
        (Entity, &'static RTSUnit),
        (With<RTSHealth>, Without<Dying>, Without<Building>),
    >,
}

/// Orders idle AI units to attack the player's base or nearest player unit.
pub fn generate_combat_goals(goals: &mut GlobalGoalManager, params: &CombatParams) {
    let Some(target) = find_player_target(params) else {
        return;
    };

    let mut candidates: Vec<Entity> = params
        .idle_ai
        .iter()
        .filter(|(entity, unit, state, combat)| {
            unit.player_id >= 2
                && **state == UnitState::Idle
                && combat.target.is_none()
                && !goals.has_goal_for(*entity)
        })
        .map(|(entity, _, _, _)| entity)
        .collect();

    candidates.shuffle(&mut rand::thread_rng());
    let commit_count = ((candidates.len() as f32) * WAVE_COMMIT_FRACTION).ceil() as usize;
    for attacker in candidates.into_iter().take(commit_count) {
        goals.push(ATTACK_PRIORITY, UnifiedGoal::AttackTarget { attacker, target });
    }
}

fn find_player_target(params: &CombatParams) -> Option<Entity> {
    let queen = params
        .player_buildings
        .iter()
        .find(|(_, b)| b.player_id == 1 && b.building_type == BuildingType::Queen);
    if let Some((entity, _)) = queen {
        return Some(entity);
    }
    params
        .player_units
        .iter()
        .find(|(_, u)| u.player_id == 1)
        .map(|(e, _)| e)
}
