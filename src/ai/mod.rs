//! AI player controller.
//!
//! ## System order (each Update frame)
//!
//! ```text
//! worker_goal_system          (idle workers → AssignWorkerToResource goals)
//! production_goal_system      (can-afford + under-cap → BuildUnit goals)
//!   ↓
//! execute_ai_goals_system     (drains GlobalGoalManager → fires events, deducts costs)
//! ```
//!
//! Unit spawning is handled by `ProductionPlugin` in `rts/production.rs`,
//! which consumes `QueueProductionEvent` independently of the AI tick.

pub mod goals;
pub mod strategy;

use bevy::prelude::*;
use goals::types::GlobalGoalManager;
use goals::execute_ai_goals_system;
use strategy::{combat_goal_system, production_goal_system, worker_goal_system};

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobalGoalManager>()
            .add_systems(
                Update,
                (worker_goal_system, production_goal_system, combat_goal_system, execute_ai_goals_system).chain(),
            );
    }
}
