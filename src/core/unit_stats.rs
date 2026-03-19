//! Unit statistics configuration.
//!
//! Pure data module — role-based stat generation.
//! Call `get_unit_stats` to retrieve the complete `UnitStatsConfig` for any unit type.

use crate::core::components::{AttackType, UnitType};
use std::collections::HashMap;

// Base unit statistics
const BASE_HEALTH: f32 = 100.0;
const BASE_DAMAGE: f32 = 20.0;
const BASE_SPEED: f32 = 80.0;
const BASE_MELEE_RANGE: f32 = 3.0;
const BASE_SIEGE_RANGE: f32 = 8.0;

/// Logical role of a unit, used for stat generation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnitRole {
    Economic,
    Tank,
    Dps,
    Scout,
    Siege,
    Elite,
}

#[derive(Debug, Clone)]
struct BaseStats {
    pub health_multiplier: f32,
    pub damage_multiplier: f32,
    pub speed_multiplier: f32,
    pub armor_base: f32,
    pub attack_type: AttackType,
    pub animation_speed: f32,
}

/// Health statistics for a unit type.
#[derive(Debug, Clone, Copy)]
pub struct HealthStats {
    pub current: f32,
    pub max: f32,
    pub armor: f32,
    pub regeneration_rate: f32,
}

/// Movement statistics for a unit type.
#[derive(Debug, Clone, Copy)]
pub struct MovementStats {
    pub max_speed: f32,
}

/// Combat statistics for a unit type.
#[derive(Debug, Clone)]
pub struct CombatStats {
    pub attack_damage: f32,
    pub attack_range: f32,
    pub attack_speed: f32,
    pub attack_type: AttackType,
    pub auto_attack: bool,
}

/// Complete unit statistics configuration.
#[derive(Debug, Clone)]
pub struct UnitStatsConfig {
    pub health: HealthStats,
    pub combat: CombatStats,
    pub movement: MovementStats,
    pub collision_radius: f32,
    pub animation_speed: f32,
}

// ─── Dynamic generation ───────────────────────────────────────────────────────

fn generate_unit_stats(unit_type: &UnitType) -> UnitStatsConfig {
    let role = get_unit_role(unit_type);
    let role_stats = get_role_base_stats();
    let base = role_stats.get(&role).unwrap();

    let collision_radius = match unit_type {
        UnitType::Fourmi => crate::core::constants::collision::WORKER_ANT_COLLISION_RADIUS,
        UnitType::RolyPoly => crate::core::constants::collision::BEETLE_KNIGHT_COLLISION_RADIUS,
        _ => crate::core::constants::collision::DEFAULT_UNIT_COLLISION_RADIUS,
    };

    let base_range = match base.attack_type {
        AttackType::Melee => BASE_MELEE_RANGE,
        AttackType::Siege => BASE_SIEGE_RANGE,
    };
    let attack_range = collision_radius + base_range;
    let health = BASE_HEALTH * base.health_multiplier;
    let damage = BASE_DAMAGE * base.damage_multiplier;
    let speed = BASE_SPEED * base.speed_multiplier * match unit_type {
        UnitType::Bee => 2.0,
        _ => 1.0,
    };

    UnitStatsConfig {
        health: HealthStats {
            current: health,
            max: health,
            armor: base.armor_base,
            regeneration_rate: if role == UnitRole::Economic { 0.1 } else { 0.3 },
        },
        combat: CombatStats {
            attack_damage: damage,
            attack_range,
            attack_speed: match base.attack_type {
                AttackType::Melee => 1.5,
                AttackType::Siege => 0.9,
            },
            attack_type: base.attack_type.clone(),
            auto_attack: role != UnitRole::Economic,
        },
        movement: MovementStats { max_speed: speed },
        collision_radius,
        animation_speed: match unit_type {
            UnitType::GoliathBirdeater => base.animation_speed * 3.0,
            _ => base.animation_speed,
        },
    }
}

fn get_unit_role(unit_type: &UnitType) -> UnitRole {
    match unit_type {
        UnitType::Fourmi | UnitType::Bee => UnitRole::Economic,
        UnitType::CairnsBirdwing => UnitRole::Scout,
        UnitType::Dragonfly | UnitType::GoliathBirdeater => UnitRole::Elite,
        UnitType::RolyPoly | UnitType::RhinoBeetle => UnitRole::Tank,
        UnitType::Scorpion => UnitRole::Dps,
    }
}

fn get_role_base_stats() -> HashMap<UnitRole, BaseStats> {
    let mut stats = HashMap::new();

    stats.insert(UnitRole::Economic, BaseStats {
        health_multiplier: 0.8,
        damage_multiplier: 0.4,
        speed_multiplier: 1.0,
        armor_base: 0.0,
        attack_type: AttackType::Melee,
        animation_speed: 1.5,
    });
    stats.insert(UnitRole::Tank, BaseStats {
        health_multiplier: 1.8,
        damage_multiplier: 0.8,
        speed_multiplier: 0.7,
        armor_base: 3.0,
        attack_type: AttackType::Melee,
        animation_speed: 0.7,
    });
    stats.insert(UnitRole::Dps, BaseStats {
        health_multiplier: 0.9,
        damage_multiplier: 1.4,
        speed_multiplier: 0.55,
        armor_base: 0.5,
        attack_type: AttackType::Melee,
        animation_speed: 1.2,
    });
    stats.insert(UnitRole::Scout, BaseStats {
        health_multiplier: 0.7,
        damage_multiplier: 0.6,
        speed_multiplier: 1.6,
        armor_base: 0.0,
        attack_type: AttackType::Melee,
        animation_speed: 1.4,
    });
    stats.insert(UnitRole::Siege, BaseStats {
        health_multiplier: 1.5,
        damage_multiplier: 2.0,
        speed_multiplier: 0.6,
        armor_base: 2.0,
        attack_type: AttackType::Siege,
        animation_speed: 0.8,
    });
    stats.insert(UnitRole::Elite, BaseStats {
        health_multiplier: 2.5,
        damage_multiplier: 2.2,
        speed_multiplier: 2.0,
        armor_base: 3.0,
        attack_type: AttackType::Melee,
        animation_speed: 1.0,
    });

    stats
}

/// Returns the complete stats configuration for a given unit type.
pub fn get_unit_stats(unit_type: &UnitType) -> UnitStatsConfig {
    generate_unit_stats(unit_type)
}
