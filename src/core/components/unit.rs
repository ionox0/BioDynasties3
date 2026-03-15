use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use super::types::UnitType;

/// Authoritative top-level activity state. Each owning system writes only its variants.
// Owned by: UnitCommandsPlugin (Moving), ResourceStatePlugin (gathering variants),
//           CombatStatePlugin (combat variants). All may write Idle.
#[derive(Component, Debug, Clone, PartialEq, Copy, Default)]
pub enum UnitState {
    #[default]
    Idle,
    // Movement-owned
    Moving,
    // Resource-owned
    MovingToResource,
    Gathering,
    ReturningToBase,
    DeliveringResources,
    // Combat-owned
    InCombat,
    MovingToAttack,
    MovingToCombat,
}

/// Core identity component present on every RTS unit.
// Owned by: spawn sites (scene setup, AI spawner)
#[derive(Component, Debug, Clone)]
pub struct RTSUnit {
    pub player_id: u8,
    pub unit_type: Option<UnitType>,
}

/// Team identifier for AI-controlled players.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeamType {
    Ai,
}

/// Component to track which team a player belongs to.
// Owned by: spawn systems (AI setup)
#[derive(Component, Debug, Clone)]
pub struct PlayerTeam {
    pub team_type: TeamType,
    pub player_id: u8,
}

/// Marker for non-interactive decoration entities.
// Owned by: ScenePlugin (world setup)
#[derive(Component, Debug, Clone)]
pub struct EnvironmentObject;
