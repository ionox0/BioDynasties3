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
}

// ─── Named constants for specific units ─────────────────────────────────────

/// Spear Mantis — elite DPS (150 cost).
#[allow(dead_code)]
pub const SPEAR_MANTIS_STATS: UnitStatsConfig = UnitStatsConfig {
    health: HealthStats { current: 110.0, max: 110.0, armor: 1.0, regeneration_rate: 0.5 },
    combat: CombatStats {
        attack_damage: 40.0,
        attack_range: 13.0,
        attack_speed: 1.6,
        attack_type: AttackType::Melee,
        auto_attack: false,
    },
    movement: MovementStats { max_speed: 88.0 },
    collision_radius: crate::core::constants::collision::DEFAULT_UNIT_COLLISION_RADIUS,
};

/// Scout Ant — fast reconnaissance (80 cost).
#[allow(dead_code)]
pub const SCOUT_ANT_STATS: UnitStatsConfig = UnitStatsConfig {
    health: HealthStats { current: 65.0, max: 65.0, armor: 0.0, regeneration_rate: 0.3 },
    combat: CombatStats {
        attack_damage: 15.0,
        attack_range: 13.0,
        attack_speed: 2.5,
        attack_type: AttackType::Melee,
        auto_attack: false,
    },
    movement: MovementStats { max_speed: 128.0 },
    collision_radius: crate::core::constants::collision::DEFAULT_UNIT_COLLISION_RADIUS,
};

/// Beetle Knight — heavy tank (180 cost).
#[allow(dead_code)]
pub const BEETLE_KNIGHT_STATS: UnitStatsConfig = UnitStatsConfig {
    health: HealthStats { current: 280.0, max: 280.0, armor: 4.0, regeneration_rate: 0.2 },
    combat: CombatStats {
        attack_damage: 25.0,
        attack_range: 11.0,
        attack_speed: 1.2,
        attack_type: AttackType::Melee,
        auto_attack: false,
    },
    movement: MovementStats { max_speed: 56.0 },
    collision_radius: crate::core::constants::collision::BEETLE_KNIGHT_COLLISION_RADIUS,
};

/// DragonFly — ultimate elite unit (450 cost).
pub const DRAGONFLY_STATS: UnitStatsConfig = UnitStatsConfig {
    health: HealthStats { current: 300.0, max: 300.0, armor: 3.0, regeneration_rate: 0.8 },
    combat: CombatStats {
        attack_damage: 35.0,
        attack_range: 22.0,
        attack_speed: 1.5,
        attack_type: AttackType::Melee,
        auto_attack: false,
    },
    movement: MovementStats { max_speed: 160.0 },
    collision_radius: 3.0,
};

/// Mites — ultra-cheap swarm unit (8 cost).
pub const MITES_STATS: UnitStatsConfig = UnitStatsConfig {
    health: HealthStats { current: 15.0, max: 15.0, armor: 0.0, regeneration_rate: 0.0 },
    combat: CombatStats {
        attack_damage: 3.0,
        attack_range: 8.0,
        attack_speed: 1.0,
        attack_type: AttackType::Melee,
        auto_attack: false,
    },
    movement: MovementStats { max_speed: 80.0 },
    collision_radius: crate::core::constants::collision::DEFAULT_UNIT_COLLISION_RADIUS,
};

/// Ticks — ultra-cheap siege unit (12 cost).
pub const TICKS_STATS: UnitStatsConfig = UnitStatsConfig {
    health: HealthStats { current: 25.0, max: 25.0, armor: 1.0, regeneration_rate: 0.0 },
    combat: CombatStats {
        attack_damage: 8.0,
        attack_range: 10.0,
        attack_speed: 0.8,
        attack_type: AttackType::Siege,
        auto_attack: false,
    },
    movement: MovementStats { max_speed: 48.0 },
    collision_radius: crate::core::constants::collision::DEFAULT_UNIT_COLLISION_RADIUS,
};

/// Housefly — enhanced fast DPS unit.
pub const HOUSEFLY_STATS: UnitStatsConfig = UnitStatsConfig {
    health: HealthStats { current: 90.0, max: 90.0, armor: 0.5, regeneration_rate: 0.3 },
    combat: CombatStats {
        attack_damage: 28.0,
        attack_range: 13.0,
        attack_speed: 1.0,
        attack_type: AttackType::Melee,
        auto_attack: false,
    },
    movement: MovementStats { max_speed: 96.0 },
    collision_radius: crate::core::constants::collision::DEFAULT_UNIT_COLLISION_RADIUS,
};

// ─── Dynamic generation ───────────────────────────────────────────────────────

fn generate_unit_stats(unit_type: &UnitType) -> UnitStatsConfig {
    let role = get_unit_role(unit_type);
    let role_stats = get_role_base_stats();
    let base = role_stats.get(&role).unwrap();

    let collision_radius = match unit_type {
        UnitType::WorkerAnt => crate::core::constants::collision::WORKER_ANT_COLLISION_RADIUS,
        UnitType::BeetleKnight => crate::core::constants::collision::BEETLE_KNIGHT_COLLISION_RADIUS,
        UnitType::DragonFly => 3.0,
        _ => crate::core::constants::collision::DEFAULT_UNIT_COLLISION_RADIUS,
    };

    let base_range = match base.attack_type {
        AttackType::Melee => BASE_MELEE_RANGE,
        AttackType::Siege => BASE_SIEGE_RANGE,
    };
    let attack_range = collision_radius + base_range;
    let health = BASE_HEALTH * base.health_multiplier;
    let damage = BASE_DAMAGE * base.damage_multiplier;
    let speed = BASE_SPEED * base.speed_multiplier;

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
    }
}

fn get_unit_role(unit_type: &UnitType) -> UnitRole {
    match unit_type {
        UnitType::WorkerAnt | UnitType::TermiteWorker | UnitType::Honeybees => UnitRole::Economic,

        UnitType::BeetleKnight
        | UnitType::WolfSpider
        | UnitType::Scorpion
        | UnitType::TermiteWarrior
        | UnitType::DefenderBug
        | UnitType::StagBeetle
        | UnitType::RhinoBeetle
        | UnitType::Woodlouse
        | UnitType::Tarantula
        | UnitType::EliteSpider
        | UnitType::SpearMantis
        | UnitType::Housefly
        | UnitType::OrchidMantis
        | UnitType::WidowSpider
        | UnitType::Hornets
        | UnitType::Earwigs
        | UnitType::StickBugs
        | UnitType::JewelBug => UnitRole::Dps,

        UnitType::ScoutAnt
        | UnitType::Aphids
        | UnitType::Mites
        | UnitType::Firefly
        | UnitType::DragonFlies => UnitRole::Scout,

        UnitType::BatteringBeetle
        | UnitType::Stinkbug
        | UnitType::SandFleas
        | UnitType::Ticks
        | UnitType::Fleas
        | UnitType::Lice => UnitRole::Siege,

        UnitType::DragonFly
        | UnitType::Moths
        | UnitType::Caterpillars
        | UnitType::PeacockMoth
        | UnitType::Cicadas => UnitRole::Elite,
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
    });
    stats.insert(UnitRole::Tank, BaseStats {
        health_multiplier: 1.8,
        damage_multiplier: 0.8,
        speed_multiplier: 0.7,
        armor_base: 3.0,
        attack_type: AttackType::Melee,
    });
    stats.insert(UnitRole::Dps, BaseStats {
        health_multiplier: 0.9,
        damage_multiplier: 1.4,
        speed_multiplier: 1.1,
        armor_base: 0.5,
        attack_type: AttackType::Melee,
    });
    stats.insert(UnitRole::Scout, BaseStats {
        health_multiplier: 0.7,
        damage_multiplier: 0.6,
        speed_multiplier: 1.6,
        armor_base: 0.0,
        attack_type: AttackType::Melee,
    });
    stats.insert(UnitRole::Siege, BaseStats {
        health_multiplier: 1.5,
        damage_multiplier: 2.0,
        speed_multiplier: 0.6,
        armor_base: 2.0,
        attack_type: AttackType::Siege,
    });
    stats.insert(UnitRole::Elite, BaseStats {
        health_multiplier: 2.5,
        damage_multiplier: 2.2,
        speed_multiplier: 2.0,
        armor_base: 3.0,
        attack_type: AttackType::Melee,
    });

    stats
}

/// Returns the complete stats configuration for a given unit type.
pub fn get_unit_stats(unit_type: &UnitType) -> UnitStatsConfig {
    match unit_type {
        UnitType::DragonFly => DRAGONFLY_STATS,
        UnitType::Mites => MITES_STATS,
        UnitType::Ticks => TICKS_STATS,
        UnitType::Housefly => HOUSEFLY_STATS,
        _ => generate_unit_stats(unit_type),
    }
}
