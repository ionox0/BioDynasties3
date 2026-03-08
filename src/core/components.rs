use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// Removed surrounding system components

// Removed unused old game components - using RTS components instead

// Owned by: ScenePlugin (setup_scene)
#[derive(Component)]
pub struct MainCamera;

// Owned by: ScenePlugin (handle_rts_camera_input)
#[derive(Component, Debug, Clone)]
pub struct RTSCamera {
    pub move_speed: f32,
}




// RTS-specific components

// Owned by: spawn sites (scene setup, AI spawner)
#[derive(Component, Debug, Clone)]
pub struct RTSUnit {
    pub player_id: u8,
    pub unit_type: Option<UnitType>, // None for buildings
}

// Owned by: MovementSystem (position sync)
#[derive(Component, Debug, Clone)]
pub struct Position {
    pub translation: Vec3,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
        }
    }
}

// Owned by: MovementSystem (collision, animation)
#[derive(Component, Debug, Clone)]
pub struct Movement {
    pub max_speed: f32,
    pub current_velocity: Vec3,
    pub target_position: Option<Vec3>,
}

// Owned by: PathfindingPlugin (pathfinding_system) and MovementPlugin (path_index advancement)
#[derive(Component, Debug, Clone)]
pub struct PathfindingState {
    /// World-space waypoints produced by A*.
    pub path: Vec<Vec3>,
    /// Index of the next waypoint to head toward.
    pub path_index: usize,
    /// Bevy elapsed time of the last pathfinding failure (NEG_INFINITY = no failure yet).
    pub last_pathfinding_failure: f32,
    /// Target that triggered the last failure — cleared when a new target arrives.
    pub last_failed_target: Option<Vec3>,
}

impl Default for PathfindingState {
    fn default() -> Self {
        Self {
            path: Vec::new(),
            path_index: 0,
            last_pathfinding_failure: f32::NEG_INFINITY,
            last_failed_target: None,
        }
    }
}

/// Component to track spatial grid position for incremental updates
// Owned by: CollisionPlugin (spatial_grid_update_system)
#[derive(Component, Debug, Clone)]
pub struct SpatialGridPosition {
    pub last_grid_coord: Option<crate::core::spatial_grid::GridCoord>,
    pub dirty: bool,
}


impl Default for SpatialGridPosition {
    fn default() -> Self {
        Self {
            last_grid_coord: None,
            dirty: true, // Start dirty to ensure initial grid insertion
        }
    }
}

impl Default for Movement {
    fn default() -> Self {
        Self {
            max_speed: 200.0,
            current_velocity: Vec3::ZERO,
            target_position: None,
        }
    }
}


// Owned by: CombatPlugin (apply_damage_system, health_regen_system)
#[derive(Component, Debug, Clone)]
pub struct RTSHealth {
    pub current: f32,
    pub max: f32,
}

impl Default for RTSHealth {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
        }
    }
}

// Owned by: CombatPlugin (combat_system, attack_system)
#[derive(Component, Debug, Clone)]
pub struct Combat {
    pub attack_range: f32,
    pub target: Option<Entity>,
    pub auto_attack: bool,
}

// Owned by: CombatStatePlugin (update_combat_states)
#[derive(Component, Debug, Clone, PartialEq)]
pub struct CombatState {
    pub state: CombatStateType,
    pub target_entity: Option<Entity>,
    pub target_position: Option<Vec3>,
    pub last_state_change: f32,
    pub engagement_start_time: f32,
    pub last_attack_attempt: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CombatStateType {
    /// Unit is not in combat, following normal movement/orders
    Idle,
    /// Unit is moving toward a combat engagement (initial movement to fight)
    MovingToCombat,
    /// Unit is moving toward an attack target but not yet in range
    MovingToAttack,
    /// Unit is actively engaged in combat, within attack range
    InCombat,
    /// Unit is in combat but temporarily moving (chasing fleeing enemy, repositioning)
    CombatMoving,
}

impl Default for CombatState {
    fn default() -> Self {
        Self {
            state: CombatStateType::Idle,
            target_entity: None,
            target_position: None,
            last_state_change: 0.0,
            engagement_start_time: 0.0,
            last_attack_attempt: 0.0,
        }
    }
}

// Owned by: ResourceStateSystem (resource_state_system)
#[derive(Component, Debug, Clone)]
pub struct ResourceGatherer {
    pub capacity: f32,
    pub carried_amount: f32,
    pub resource_type: Option<ResourceType>,
    pub target_resource: Option<Entity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Nectar,
    Chitin,
    Minerals,
    Pheromones,
}


// Owned by: ResourcePlugin (resource_collection_system)
#[derive(Component, Debug, Clone)]
pub struct ResourceSource;

// Owned by: ConstructionPlugin (apply_construction_progress)
#[derive(Component, Debug, Clone)]
pub struct Building {
    pub building_type: BuildingType,
    pub construction_progress: f32,
    pub max_construction: f32,
    pub is_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BuildingType {
    Queen,
    Nursery,
    WarriorChamber,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UnitType {
    WorkerAnt,
    BeetleKnight,
    SpearMantis,
    ScoutAnt,
    BatteringBeetle,

    // Additional unit types for new models
    DragonFly,   // Flying reconnaissance unit
    DefenderBug, // Defensive unit
    EliteSpider, // Elite predator unit

    // Units for previously unused models
    Scorpion,     // Heavy melee unit with armor
    WolfSpider,   // Heavy predator unit

    // Units for newly added models
    Housefly,       // Fast flying harassment unit
    TermiteWorker,  // Builder/gatherer specialist
    TermiteWarrior, // Heavy siege unit (giant_termite.glb)
    Stinkbug,       // Area denial/support unit

    // Expanded unit categories for multi-team system

    // Beetles family
    StagBeetle,        // Heavy melee beetle
    RhinoBeetle,       // Armored assault beetle
    JewelBug,          // Fast support beetle

    // Mantids family - keeping SpearMantis as primary
    OrchidMantis,      // Stealth/ambush mantis

    // Cephalopoda family (Isopods/Crustaceans)
    Woodlouse,         // Armored defensive unit
    SandFleas,         // Jumping swarm unit

    // Small creatures family
    Aphids,            // Tiny swarm units
    Mites,             // Microscopic fast units
    Ticks,             // Parasitic units
    Fleas,             // Small jumping units
    Lice,              // Tiny fast units

    // Butterflies family
    Moths,             // Night flying units
    Caterpillars,      // Ground larvae units
    PeacockMoth,       // Large beautiful flyer

    // Spiders family
    WidowSpider,       // Venomous predator
    Tarantula,         // Large ground predator

    // Flies family
    Firefly,           // Light/energy fly
    DragonFlies,       // Large aerial predator

    // Bees family
    Hornets,           // Aggressive flying unit
    Honeybees,         // Economic flying unit (consolidated from multiple bee types and AcidSpitter)

    // Termites family
    Earwigs,           // Pincer assault unit

    // Individual species
    StickBugs,         // Camouflaged units
    Cicadas,           // Sound/support units
}

// Owned by: SelectionPlugin (apply_selection_changes)
#[derive(Component, Debug, Clone)]
pub struct Selectable {
    pub is_selected: bool,
    pub selection_radius: f32,
}

impl Default for Selectable {
    fn default() -> Self {
        Self {
            is_selected: false,
            selection_radius: 8.0, // Increased from 5.0 for better clickability
        }
    }
}

// Owned by: ConstructionPlugin (construction_system)
#[derive(Component, Debug, Clone)]
pub struct Constructor {
    pub build_speed: f32,
    pub current_target: Option<Entity>,
}

// Owned by: SelectionPlugin (create_selection_indicators, selection_indicator_system)
#[derive(Component)]
pub struct SelectionIndicator {
    pub target: Entity,
}

// Owned by: SelectionPlugin (drag_selection_system)
#[derive(Component)]
pub struct DragSelection {
    pub start_position: Vec2,
    pub current_position: Vec2,
    pub is_active: bool,
}

// Owned by: SelectionPlugin (drag_selection_system visual cleanup)
#[derive(Component)]
pub struct SelectionBox;

/// Component to mark entities that are in the process of dying.
/// Prevents duplicate death processing and race conditions.
// Owned by: LifecyclePlugin (mark_dying_units)
#[derive(Component, Debug)]
pub struct Dying;

/// Resource gathering state component
/// Tracks units that are collecting or returning resources.
/// For the active resource target, use ResourceGatherer.target_resource.
// Owned by: ResourceStatePlugin (update_gathering_states)
#[derive(Component, Debug, Clone, PartialEq)]
pub struct GatheringState {
    pub state: GatheringStateType,
    pub return_building: Option<Entity>,
    pub gather_start_time: f32,
    pub last_state_change: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GatheringStateType {
    /// Unit is idle, waiting for work assignment
    Idle,
    /// Moving to a resource to start gathering
    MovingToResource,
    /// Actively gathering from a resource
    Gathering,
    /// Moving back to base with gathered resources
    ReturningToBase,
    /// Delivering resources to a building
    DeliveringResources,
}

impl Default for GatheringState {
    fn default() -> Self {
        Self {
            state: GatheringStateType::Idle,
            return_building: None,
            gather_start_time: 0.0,
            last_state_change: 0.0,
        }
    }
}

// Owned by: CollisionPlugin (spatial_grid_update_system, unit_collision_avoidance_system)
#[derive(Component, Debug, Clone)]
pub struct CollisionRadius {
    pub radius: f32,
}

impl Default for CollisionRadius {
    fn default() -> Self {
        Self {
            radius: 2.5, // Larger default radius for GLB models with scaling
        }
    }
}

/// Component for environment objects (non-interactive decorations).
// Owned by: ScenePlugin (world setup)
#[derive(Component, Debug, Clone)]
pub struct EnvironmentObject;

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
