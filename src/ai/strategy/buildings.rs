//! Building goal generation — spawns new AI buildings when needed.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::Rng;
use crate::core::components::{Building, BuildingType};
use crate::world::static_terrain::StaticTerrainHeights;
use crate::ai::goals::types::{GlobalGoalManager, UnifiedGoal};

/// Radius from the AI Queen at which new buildings are placed.
pub const BUILDING_PLACEMENT_RADIUS: f32 = 80.0;

const BUILDING_PRIORITY: f32 = 50.0;

#[derive(SystemParam)]
pub(crate) struct BuildingParams<'w, 's> {
    buildings: Query<'w, 's, (Entity, &'static Building, &'static Transform)>,
    terrain: Res<'w, StaticTerrainHeights>,
}

/// Pushes a `BuildBuilding` goal if the AI has no Nursery yet.
pub fn generate_building_goals(goals: &mut GlobalGoalManager, params: &BuildingParams) {
    let has_nursery = params
        .buildings
        .iter()
        .any(|(_, b, _)| b.player_id == 2 && b.building_type == BuildingType::Nursery);
    if has_nursery {
        return;
    }

    let Some((_, _, queen_tf)) = params
        .buildings
        .iter()
        .find(|(_, b, _)| b.player_id == 2 && b.building_type == BuildingType::Queen)
    else {
        return;
    };

    let position = placement_position(queen_tf.translation, BUILDING_PLACEMENT_RADIUS, &params.terrain);
    goals.push(
        BUILDING_PRIORITY,
        UnifiedGoal::BuildBuilding {
            building_type: BuildingType::Nursery,
            position,
            player_id: 2,
        },
    );
}

fn placement_position(origin: Vec3, radius: f32, terrain: &StaticTerrainHeights) -> Vec3 {
    let angle = rand::thread_rng().gen_range(0.0..std::f32::consts::TAU);
    let candidate = Vec2::new(origin.x + angle.cos() * radius, origin.z + angle.sin() * radius);
    let passable = terrain.find_passable_near(candidate);
    let y = terrain.get_height(passable.x, passable.y);
    Vec3::new(passable.x, y, passable.y)
}
