//! AI goal generator — the single Bevy system that owns all AI timing logic.
//!
//! Runs every frame but only calls each strategy generator when its jittered
//! interval has elapsed. This keeps timing logic in one place and lets the
//! strategy modules be plain functions with no scheduling awareness.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::Rng;
use crate::ai::goals::types::GlobalGoalManager;
use crate::ai::strategy::{
    BuildingParams, generate_building_goals,
    CombatParams, generate_combat_goals,
    ProductionParams, generate_production_goals,
    WorkerParams, generate_worker_goals,
};

const WORKER_EVAL_MIN: f32 = 3.0;
const WORKER_EVAL_MAX: f32 = 8.0;
const PRODUCTION_EVAL_MIN: f32 = 5.0;
const PRODUCTION_EVAL_MAX: f32 = 10.0;
const COMBAT_EVAL_MIN: f32 = 20.0;
const COMBAT_EVAL_MAX: f32 = 45.0;
/// Combat waves don't begin until this many seconds into the game.
const COMBAT_INITIAL_DELAY: f32 = 30.0;
const BUILDING_EVAL_MIN: f32 = 10.0;
const BUILDING_EVAL_MAX: f32 = 20.0;

pub(crate) struct GoalGeneratorState {
    next_worker_eval: f32,
    next_production_eval: f32,
    next_combat_eval: f32,
    next_building_eval: f32,
}

impl Default for GoalGeneratorState {
    fn default() -> Self {
        Self {
            // Workers and production activate on the first tick.
            next_worker_eval: 0.0,
            next_production_eval: 0.0,
            // Combat waits for the initial delay.
            next_combat_eval: COMBAT_INITIAL_DELAY,
            // Building check activates on the first tick; suppressed if building already exists.
            next_building_eval: 0.0,
        }
    }
}

#[derive(SystemParam)]
pub(crate) struct GeneratorQueries<'w, 's> {
    worker: WorkerParams<'w, 's>,
    production: ProductionParams<'w, 's>,
    combat: CombatParams<'w, 's>,
    building: BuildingParams<'w, 's>,
}

/// Drives all AI goal generation on per-category jittered intervals.
pub(crate) fn goal_generator(
    mut goals: ResMut<GlobalGoalManager>,
    mut state: Local<GoalGeneratorState>,
    time: Res<Time>,
    mut queries: GeneratorQueries,
) {
    let now = time.elapsed_secs();
    let mut rng = rand::thread_rng();

    if now >= state.next_worker_eval {
        generate_worker_goals(&mut goals, &queries.worker);
        state.next_worker_eval = now + rng.gen_range(WORKER_EVAL_MIN..WORKER_EVAL_MAX);
    }

    if now >= state.next_production_eval {
        generate_production_goals(&mut goals, &queries.production);
        state.next_production_eval = now + rng.gen_range(PRODUCTION_EVAL_MIN..PRODUCTION_EVAL_MAX);
    }

    if now >= state.next_combat_eval {
        generate_combat_goals(&mut goals, &queries.combat);
        state.next_combat_eval = now + rng.gen_range(COMBAT_EVAL_MIN..COMBAT_EVAL_MAX);
    }

    if now >= state.next_building_eval {
        generate_building_goals(&mut goals, &queries.building);
        state.next_building_eval = now + rng.gen_range(BUILDING_EVAL_MIN..BUILDING_EVAL_MAX);
    }
}
