use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// How a unit delivers damage.
// Owned by: entity spawn sites (entity_factory, inline spawns)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttackType {
    Melee,
    Siege,
}

/// Category of damage used for armor mitigation calculations.
#[derive(Debug, Clone, PartialEq)]
pub enum DamageType {
    Physical,
    Siege,
}

// Owned by: CombatPlugin (damage_resolution_system, health_regen_system)
#[derive(Component, Debug, Clone)]
pub struct RTSHealth {
    pub current: f32,
    pub max: f32,
    pub armor: f32,
    pub regeneration_rate: f32,
    pub last_damage_time: f32,
}

impl Default for RTSHealth {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
            armor: 0.0,
            regeneration_rate: 0.3,
            last_damage_time: 0.0,
        }
    }
}

// Owned by: CombatPlugin (combat_target_handler, target_management_system, combat_execution_system)
#[derive(Component, Debug, Clone)]
pub struct Combat {
    pub attack_damage: f32,
    pub attack_range: f32,
    pub attack_cooldown: f32,
    pub last_attack_time: f32,
    pub target: Option<Entity>,
    pub attack_type: AttackType,
    pub is_attacking: bool,
    pub auto_attack: bool,
    /// Last movement destination issued to the movement system.
    /// None when the unit is not actively chasing a target.
    pub move_dest: Option<Vec3>,
}

impl Default for Combat {
    fn default() -> Self {
        Self {
            attack_damage: 20.0,
            attack_range: 13.0,
            attack_cooldown: 1.0 / 1.5,
            last_attack_time: 0.0,
            target: None,
            attack_type: AttackType::Melee,
            is_attacking: false,
            auto_attack: false,
            move_dest: None,
        }
    }
}

// Owned by: CombatPlugin (update_combat_states)
#[derive(Component, Debug, Clone, PartialEq)]
pub struct CombatState {
    pub target_entity: Option<Entity>,
    pub target_position: Option<Vec3>,
    pub last_state_change: f32,
    pub engagement_start_time: f32,
    pub last_attack_attempt: f32,
}

impl Default for CombatState {
    fn default() -> Self {
        Self {
            target_entity: None,
            target_position: None,
            last_state_change: 0.0,
            engagement_start_time: 0.0,
            last_attack_attempt: 0.0,
        }
    }
}

/// Marker set when health reaches zero. Prevents duplicate death processing.
// Owned by: LifecyclePlugin (mark_dying_units)
#[derive(Component, Debug)]
pub struct Dying;
