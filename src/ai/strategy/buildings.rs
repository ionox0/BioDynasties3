//! Building goal generation — spawns new AI buildings when needed.

use std::collections::HashMap;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::Rng;
use crate::core::components::{Building, BuildingType};
use crate::world::building_grid::BuildingGrid;
use crate::world::static_terrain::StaticTerrainHeights;
use crate::ai::goals::types::{GlobalGoalManager, UnifiedGoal};

/// Radius from the chosen anchor building at which new buildings are placed.
pub const BUILDING_PLACEMENT_RADIUS: f32 = 280.0;

const NURSERY_PRIORITY: f32 = 65.0;
const WARRIOR_CHAMBER_PRIORITY: f32 = 70.0;

#[derive(SystemParam)]
pub(crate) struct BuildingParams<'w, 's> {
    buildings: Query<'w, 's, (Entity, &'static Building, &'static Transform)>,
    terrain: Res<'w, StaticTerrainHeights>,
    building_grid: Res<'w, BuildingGrid>,
}

/// Pushes `BuildBuilding` goals for Nursery and WarriorChamber on each eval tick.
/// No existence check — priorities and nectar cost act as the natural rate limiter.
/// Placement is offset from a randomly chosen existing AI building so the base
/// can expand outward rather than always clustering around the Queen.
pub fn generate_building_goals(goals: &mut GlobalGoalManager, params: &BuildingParams) {
    let mut by_player: HashMap<u8, Vec<Vec3>> = HashMap::new();
    for (_, b, tf) in params.buildings.iter() {
        if b.player_id >= 2 {
            by_player.entry(b.player_id).or_default().push(tf.translation);
        }
    }

    for (player_id, positions) in &by_player {
        goals.push(NURSERY_PRIORITY, UnifiedGoal::BuildBuilding {
            building_type: BuildingType::Nursery,
            position: placement_near_random(positions, BUILDING_PLACEMENT_RADIUS, &params.terrain, &params.building_grid),
            player_id: *player_id,
        });
        goals.push(WARRIOR_CHAMBER_PRIORITY, UnifiedGoal::BuildBuilding {
            building_type: BuildingType::WarriorChamber,
            position: placement_near_random(positions, BUILDING_PLACEMENT_RADIUS, &params.terrain, &params.building_grid),
            player_id: *player_id,
        });
    }
}

fn placement_near_random(buildings: &[Vec3], radius: f32, terrain: &StaticTerrainHeights, building_grid: &BuildingGrid) -> Vec3 {
    let idx = rand::thread_rng().gen_range(0..buildings.len());
    placement_position(buildings[idx], radius, terrain, building_grid)
}

fn placement_position(origin: Vec3, radius: f32, terrain: &StaticTerrainHeights, building_grid: &BuildingGrid) -> Vec3 {
    use crate::core::constants::movement::TERRAIN_SIZE;
    const MARGIN: f32 = 100.0;
    let limit = TERRAIN_SIZE - MARGIN;
    let angle = rand::thread_rng().gen_range(0.0..std::f32::consts::TAU);
    let candidate = Vec3::new(
        (origin.x + angle.cos() * radius).clamp(-limit, limit),
        0.0,
        (origin.z + angle.sin() * radius).clamp(-limit, limit),
    );
    if let Some(pos) = building_grid.find_clear_position(candidate, terrain) {
        return pos;
    }
    let passable = terrain.find_passable_near(candidate.xz());
    let y = terrain.get_height(passable.x, passable.y);
    Vec3::new(passable.x, y, passable.y)
}
