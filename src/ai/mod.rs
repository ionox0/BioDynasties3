//! AI player controller.
//!
//! ## System order (each Update frame)
//!
//! ```text
//! goal_generator          (fires each strategy generator on a jittered interval)
//!   ├─ generate_worker_goals     → AssignWorkerToResource goals
//!   ├─ generate_production_goals → BuildUnit goals
//!   └─ generate_combat_goals     → AttackTarget goals
//!   ↓
//! execute_ai_goals_system (drains GlobalGoalManager → fires events, deducts costs)
//! ```
//!
//! Unit spawning is handled by `ProductionPlugin` in `rts/production.rs`,
//! which consumes `QueueProductionEvent` independently of the AI tick.

pub mod goals;
pub mod strategy;
mod goal_generator;

use bevy::prelude::*;
use goals::types::{GlobalGoalManager, GoalQueueSnapshot};
use goals::goal_executor;
use goal_generator::goal_generator;

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobalGoalManager>()
            .init_resource::<GoalQueueSnapshot>()
            .add_systems(
                Update,
                (goal_generator, snapshot_goals, goal_executor).chain(),
            );
    }
}

fn snapshot_goals(goals: Res<GlobalGoalManager>, mut snapshot: ResMut<GoalQueueSnapshot>) {
    if !goals.goals.is_empty() {
        snapshot.goals = goals.goals.clone();
    }
}
