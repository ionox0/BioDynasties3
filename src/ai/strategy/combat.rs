//! AI combat goal generation.
//!
//! Periodically orders idle AI units to attack the player's base (or nearest
//! player unit if the base is gone). A wave fires at most once every
//! `ATTACK_WAVE_INTERVAL` seconds, and only targets units that are currently
//! idle with no active combat target.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::core::components::{Building, BuildingType, Combat, Dying, RTSHealth, RTSUnit, UnitState};
use crate::ai::goals::types::{GlobalGoalManager, UnifiedGoal};

const ATTACK_WAVE_INTERVAL: f32 = 30.0;
/// Fraction of idle AI units to commit to each wave (rest stay as passive defenders).
const WAVE_COMMIT_FRACTION: f32 = 0.75;
const ATTACK_PRIORITY: f32 = 70.0;

#[derive(SystemParam)]
pub(crate) struct IdleAiUnits<'w, 's> {
    query: Query<'w, 's, (Entity, &'static RTSUnit, &'static UnitState, &'static Combat)>,
}

#[derive(SystemParam)]
pub(crate) struct PlayerTargets<'w, 's> {
    #[allow(clippy::type_complexity)]
    buildings: Query<
        'w,
        's,
        (Entity, &'static Building),
        (With<RTSHealth>, Without<Dying>),
    >,
    #[allow(clippy::type_complexity)]
    units: Query<
        'w,
        's,
        (Entity, &'static RTSUnit),
        (With<RTSHealth>, Without<Dying>, Without<Building>),
    >,
}

impl PlayerTargets<'_, '_> {
    /// Returns the player's Queen building entity, or the first living player
    /// unit if the Queen is gone.
    fn find_target(&self) -> Option<Entity> {
        // Prefer the Queen building.
        let queen = self.buildings.iter().find(|(_, b)| {
            b.player_id == 1 && b.building_type == BuildingType::Queen
        });
        if let Some((entity, _)) = queen {
            return Some(entity);
        }
        // Fall back to any player unit.
        self.units
            .iter()
            .find(|(_, u)| u.player_id == 1)
            .map(|(e, _)| e)
    }
}

/// Generates attack-order goals for idle AI units on a timed wave interval.
pub fn combat_goal_system(
    mut goals: ResMut<GlobalGoalManager>,
    mut last_wave: Local<f32>,
    time: Res<Time>,
    idle_ai: IdleAiUnits,
    player_targets: PlayerTargets,
) {
    let now = time.elapsed_secs();
    if now - *last_wave < ATTACK_WAVE_INTERVAL {
        return;
    }

    let Some(target) = player_targets.find_target() else {
        return;
    };

    let candidates: Vec<Entity> = idle_ai
        .query
        .iter()
        .filter(|(_, unit, state, combat)| {
            unit.player_id >= 2 && **state == UnitState::Idle && combat.target.is_none()
        })
        .map(|(entity, _, _, _)| entity)
        .collect();

    if candidates.is_empty() {
        return;
    }

    *last_wave = now;

    let commit_count = ((candidates.len() as f32) * WAVE_COMMIT_FRACTION).ceil() as usize;
    for attacker in candidates.into_iter().take(commit_count) {
        goals.push(ATTACK_PRIORITY, UnifiedGoal::AttackTarget { attacker, target });
    }
}
