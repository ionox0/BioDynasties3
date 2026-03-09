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
use crate::core::components::*;
use goals::types::GlobalGoalManager;
use goals::execute_ai_goals_system;
use strategy::{combat_goal_system, production_goal_system, worker_goal_system};

pub struct AIPlugin;

impl Plugin for AIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobalGoalManager>()
            .add_systems(Startup, spawn_ai_units)
            .add_systems(
                Update,
                (worker_goal_system, production_goal_system, combat_goal_system, execute_ai_goals_system).chain(),
            );
    }
}

/// Spawns the initial AI base and one worker ant (player_id = 2).
fn spawn_ai_units(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        SceneRoot(asset_server.load("models/objects/anthill.glb#Scene0")),
        Transform::from_xyz(-200.0, 0.0, -300.0).with_scale(Vec3::splat(20.0)),
        Building {
            player_id: 2,
            building_type: BuildingType::Queen,
            construction_progress: 100.0,
            max_construction: 100.0,
            is_complete: true,
        },
        Position { translation: Vec3::new(-200.0, 0.0, -300.0) },
        CollisionRadius { radius: 20.0 },
        ProductionQueue::default(),
    ));

    commands.spawn((
        SceneRoot(asset_server.load("models/insects/fourmi.glb#Scene0")),
        Transform::from_xyz(-170.0, 1.0, -300.0).with_scale(Vec3::splat(15.0)),
        RTSUnit { player_id: 2, unit_type: Some(UnitType::WorkerAnt) },
        Movement { max_speed: 80.0, current_velocity: Vec3::ZERO, target_position: None },
        PathfindingState::default(),
        Position { translation: Vec3::new(-170.0, 1.0, -300.0) },
        CollisionRadius { radius: 6.0 },
        SpatialGridPosition::default(),
        RTSHealth { current: 100.0, max: 100.0, ..RTSHealth::default() },
        ResourceGatherer {
            gather_rate: 5.0,
            capacity: 10.0,
            carried_amount: 0.0,
            resource_type: None,
            target_resource: None,
        },
    ));
}
