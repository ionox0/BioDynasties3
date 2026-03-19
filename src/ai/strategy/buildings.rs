//! Building goal generation — spawns new AI buildings when needed.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::Rng;
use crate::core::components::{Building, BuildingType};
use crate::world::static_terrain::StaticTerrainHeights;
use crate::ai::goals::types::{GlobalGoalManager, UnifiedGoal};

/// Radius from the AI Queen at which new buildings are placed.
pub const BUILDING_PLACEMENT_RADIUS: f32 = 160.0;

const NURSERY_PRIORITY: f32 = 30.0;
const WARRIOR_CHAMBER_PRIORITY: f32 = 45.0;

#[derive(SystemParam)]
pub(crate) struct BuildingParams<'w, 's> {
    buildings: Query<'w, 's, (Entity, &'static Building, &'static Transform)>,
    terrain: Res<'w, StaticTerrainHeights>,
}

/// Pushes `BuildBuilding` goals for Nursery and WarriorChamber on each eval tick.
/// No existence check — priorities and nectar cost act as the natural rate limiter.
/// Placement is offset from a randomly chosen existing AI building so the base
/// can expand outward rather than always clustering around the Queen.
pub fn generate_building_goals(goals: &mut GlobalGoalManager, params: &BuildingParams) {
    let ai_buildings: Vec<Vec3> = params
        .buildings
        .iter()
        .filter(|(_, b, _)| b.player_id == 2)
        .map(|(_, _, tf)| tf.translation)
        .collect();

    if ai_buildings.is_empty() {
        return;
    }

    goals.push(
        NURSERY_PRIORITY,
        UnifiedGoal::BuildBuilding {
            building_type: BuildingType::Nursery,
            position: placement_near_random(&ai_buildings, BUILDING_PLACEMENT_RADIUS, &params.terrain),
            player_id: 2,
        },
    );
    goals.push(
        WARRIOR_CHAMBER_PRIORITY,
        UnifiedGoal::BuildBuilding {
            building_type: BuildingType::WarriorChamber,
            position: placement_near_random(&ai_buildings, BUILDING_PLACEMENT_RADIUS, &params.terrain),
            player_id: 2,
        },
    );
}

fn placement_near_random(buildings: &[Vec3], radius: f32, terrain: &StaticTerrainHeights) -> Vec3 {
    let idx = rand::thread_rng().gen_range(0..buildings.len());
    placement_position(buildings[idx], radius, terrain)
}

fn placement_position(origin: Vec3, radius: f32, terrain: &StaticTerrainHeights) -> Vec3 {
    let angle = rand::thread_rng().gen_range(0.0..std::f32::consts::TAU);
    let candidate = Vec2::new(origin.x + angle.cos() * radius, origin.z + angle.sin() * radius);
    let passable = terrain.find_passable_near(candidate);
    let y = terrain.get_height(passable.x, passable.y);
    Vec3::new(passable.x, y, passable.y)
}
