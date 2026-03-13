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
use crate::core::resources::{Stockpile, Stockpiles};
use crate::world::static_terrain::StaticTerrainHeights;
use goals::types::GlobalGoalManager;
use goals::execute_ai_goals_system;
use strategy::{combat_goal_system, production_goal_system, worker_goal_system};

/// East edge: 85 % of MAP_BOUNDARY — AI player 2 spawn.
const AI_SPAWN: Vec3 = Vec3::new(2550.0, 0.0, 0.0);

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

/// Spawns the initial AI base and one worker ant (player_id = 2) at the east edge.
fn spawn_ai_units(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut stockpiles: ResMut<Stockpiles>,
    terrain: Res<StaticTerrainHeights>,
) {
    *stockpiles.get_or_insert_mut(2) = Stockpile::starting();

    let passable = terrain.find_passable_near(Vec2::new(AI_SPAWN.x, AI_SPAWN.z));
    let ground_y = terrain.get_height(passable.x, passable.y);
    let base_pos = Vec3::new(passable.x, ground_y, passable.y);

    commands.spawn((
        SceneRoot(asset_server.load("models/objects/anthill.glb#Scene0")),
        Transform::from_translation(base_pos).with_scale(Vec3::splat(20.0)),
        Building {
            player_id: 2,
            building_type: BuildingType::Queen,
            construction_progress: 100.0,
            max_construction: 100.0,
            is_complete: true,
        },
        Position { translation: base_pos },
        CollisionRadius { radius: 20.0 },
        Selectable { is_selected: false, selection_radius: 10.0 },
        ProductionQueue::default(),
    ));

    let wp = base_pos + Vec3::new(-30.0, 0.0, 0.0);
    let worker_pos = Vec3::new(wp.x, terrain.get_height(wp.x, wp.z) + 1.0, wp.z);
    commands.spawn((
        SceneRoot(asset_server.load("models/insects/good/fourmi.glb#Scene0")),
        Transform::from_translation(worker_pos).with_scale(Vec3::splat(3.75)),
        RTSUnit { player_id: 2, unit_type: Some(UnitType::Fourmi) },
        Movement { max_speed: 80.0, current_velocity: Vec3::ZERO, target_position: None },
        PathfindingState::default(),
        Position { translation: worker_pos },
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
