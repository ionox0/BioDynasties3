//! Centralized entity spawn factory.
//!
//! All unit and building spawns should go through `EntityFactory::spawn_unit` or
//! `EntityFactory::spawn_building` to ensure consistent component bundles.

use bevy::prelude::*;
use crate::core::components::*;
use crate::core::unit_stats;

/// Centralized factory for spawning units and buildings.
pub struct EntityFactory;

impl EntityFactory {
    /// Spawn a unit at the given position for the given player.
    pub fn spawn_unit(
        commands: &mut Commands,
        asset_server: &AssetServer,
        unit_type: UnitType,
        position: Vec3,
        player_id: u8,
    ) -> Entity {
        let stats = unit_stats::get_unit_stats(&unit_type);
        let model_path = unit_model_path(&unit_type);
        let scale = unit_model_scale(&unit_type);
        let rotation = unit_model_rotation(&unit_type);

        let mut entity = commands.spawn((
            SceneRoot(asset_server.load(model_path)),
            Transform::from_translation(position).with_rotation(rotation).with_scale(Vec3::splat(scale)),
            RTSUnit { player_id, unit_type: Some(unit_type.clone()) },
            Position { translation: position },
            Movement {
                max_speed: stats.movement.max_speed,
                current_velocity: Vec3::ZERO,
                target_position: None,
            },
            PathfindingState::default(),
            CollisionRadius { radius: stats.collision_radius },
            SpatialGridPosition::default(),
            Selectable::default(),
            RTSHealth {
                current: stats.health.current,
                max: stats.health.max,
                armor: stats.health.armor,
                regeneration_rate: stats.health.regeneration_rate,
                last_damage_time: 0.0,
            },
            Combat {
                attack_damage: stats.combat.attack_damage,
                attack_range: stats.combat.attack_range,
                attack_cooldown: 1.0 / stats.combat.attack_speed,
                last_attack_time: 0.0,
                target: None,
                attack_type: stats.combat.attack_type,
                is_attacking: false,
                auto_attack: stats.combat.auto_attack,
                move_dest: None,
            },
        ));

        add_gatherer_if_needed(&mut entity, &unit_type);

        entity.id()
    }

    /// Spawn a building at the given position for the given player.
    pub fn spawn_building(
        commands: &mut Commands,
        asset_server: &AssetServer,
        building_type: BuildingType,
        position: Vec3,
        player_id: u8,
    ) -> Entity {
        let stats = building_stats(&building_type);
        let model_path = building_model_path(&building_type);

        let mut entity = commands.spawn((
            SceneRoot(asset_server.load(model_path)),
            Transform::from_translation(position).with_scale(Vec3::splat(stats.scale)),
            RTSUnit { player_id, unit_type: None },
            Position { translation: position },
            Building {
                player_id,
                building_type: building_type.clone(),
                construction_progress: 100.0,
                max_construction: 100.0,
                is_complete: true,
            },
            RTSHealth {
                current: stats.health,
                max: stats.health,
                armor: stats.armor,
                regeneration_rate: 0.0,
                last_damage_time: 0.0,
            },
            Selectable {
                is_selected: false,
                selection_radius: stats.selection_radius,
            },
            CollisionRadius { radius: stats.collision_radius },
            ProductionQueue {
                queued: Vec::new(),
                progress: 0.0,
                production_time: stats.production_time,
            },
        ));

        entity.insert(Position { translation: position });

        entity.id()
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn add_gatherer_if_needed(entity: &mut EntityCommands, unit_type: &UnitType) {
    let gatherer = match unit_type {
        UnitType::Fourmi => Some(ResourceGatherer {
            gather_rate: 10.0,
            capacity: 10.0,
            carried_amount: 0.0,
            resource_type: None,
            target_resource: None,
        }),
        UnitType::Bee => Some(ResourceGatherer {
            gather_rate: 15.0,
            capacity: 3.0,
            carried_amount: 0.0,
            resource_type: None,
            target_resource: None,
        }),
        _ => None,
    };
    if let Some(g) = gatherer {
        entity.insert(g);
    }
}

struct BuildingConfig {
    health: f32,
    armor: f32,
    selection_radius: f32,
    collision_radius: f32,
    production_time: f32,
    scale: f32,
}

fn building_stats(building_type: &BuildingType) -> BuildingConfig {
    use crate::core::constants::collision;
    match building_type {
        BuildingType::Queen => BuildingConfig {
            health: 600.0,
            armor: 5.0,
            selection_radius: 10.0,
            collision_radius: collision::QUEEN_COLLISION_RADIUS,
            production_time: 8.0,
            scale: 20.0,
        },
        BuildingType::Nursery => BuildingConfig {
            health: 75.0,
            armor: 0.0,
            selection_radius: 5.0,
            collision_radius: collision::NURSERY_COLLISION_RADIUS,
            production_time: 6.0,
            scale: 10.0,
        },
        BuildingType::WarriorChamber => BuildingConfig {
            health: 200.0,
            armor: 2.0,
            selection_radius: 8.0,
            collision_radius: collision::WARRIOR_CHAMBER_COLLISION_RADIUS,
            production_time: 10.0,
            scale: 15.0,
        },
    }
}

fn unit_model_path(unit_type: &UnitType) -> &'static str {
    match unit_type {
        UnitType::Fourmi => "models/insects/good/fourmi.glb#Scene0",
        UnitType::Bee => "models/insects/good/bee.glb#Scene0",
        UnitType::CairnsBirdwing => "models/insects/good/cairns_birdwing.glb#Scene0",
        UnitType::Dragonfly => "models/insects/good/dragonfly.glb#Scene0",
        UnitType::RolyPoly => "models/insects/good/roly_poly.glb#Scene0",
        UnitType::Scorpion => "models/insects/good/scorpion.glb#Scene0",
        UnitType::GoliathBirdeater => "models/insects/good/goliath_birdeater.glb#Scene0",
        UnitType::RhinoBeetle => "models/insects/good/rhino_beetle.glb#Scene0",
    }
}

fn building_model_path(building_type: &BuildingType) -> &'static str {
    match building_type {
        BuildingType::Queen => "models/objects/anthill.glb#Scene0",
        BuildingType::Nursery => "models/objects/anthill.glb#Scene0",
        BuildingType::WarriorChamber => "models/objects/anthill.glb#Scene0",
    }
}

fn unit_model_rotation(unit_type: &UnitType) -> Quat {
    match unit_type {
        UnitType::Dragonfly => Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        _ => Quat::IDENTITY,
    }
}

fn unit_model_scale(unit_type: &UnitType) -> f32 {
    match unit_type {
        UnitType::Fourmi | UnitType::Bee => 3.75,
        UnitType::CairnsBirdwing => 18.75,
        UnitType::Dragonfly => 100.0,
        UnitType::RolyPoly => 0.3,
        UnitType::Scorpion => 10.0,
        UnitType::GoliathBirdeater => 1.5,
        UnitType::RhinoBeetle => 10.0,
    }
}
