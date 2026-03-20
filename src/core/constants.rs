#![allow(dead_code)] // Allow unused constants for future features
                     // Game configuration constants
                     // This module contains all the magic numbers and configuration values used throughout the game

// === WINDOW AND DISPLAY ===
pub const WINDOW_TITLE: &str = "BioDynasties3";
pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 720.0;

// === MOVEMENT SYSTEM ===
pub mod movement {
    // Unified map size - single source of truth for all boundaries  
    pub const MAP_SIZE: f32 = 3000.0; // Map extends from -3000 to +3000 (total 6000 units) - larger for full gameplay
    
    // Derived boundaries based on MAP_SIZE
    pub const MAP_BOUNDARY: f32 = MAP_SIZE; // Playable area boundary
    pub const CAMERA_BOUNDARY: f32 = MAP_SIZE + 200.0; // Camera can go 200 units beyond map
    pub const TERRAIN_SIZE: f32 = MAP_SIZE; // Terrain matches map size exactly
    
    // Movement behavior constants
    pub const BOUNDARY_PUSH_FORCE: f32 = 200.0; // Force applied when near boundary
    pub const BOUNDARY_BUFFER: f32 = 50.0; // Distance from boundary where push force starts

    // Safety limits to prevent units from going to astronomical positions
    pub const MAX_POSITION: f32 = 100_000.0; // Extreme position safety net (beyond map boundary)
    pub const MAX_VELOCITY: f32 = 500.0;
    pub const MAX_DISTANCE: f32 = 50000.0;

    // Movement physics
    pub const UNIT_SPEED: f32 = 80.0; // 2x speed increase
    pub const ARRIVAL_THRESHOLD: f32 = 8.0; // Much larger threshold to prevent slowdown between close waypoints
    pub const DECELERATION_FACTOR: f32 = 2.5; // Slightly higher deceleration

    // Collision and separation
    pub const SEPARATION_MULTIPLIER: f32 = 2.0; // Units separate at 2x their radius for better flow
    pub const SEPARATION_FORCE_STRENGTH: f32 = 2.0; // Further reduced force for smoother group movement

    // Default spawn height
    pub const DEFAULT_SPAWN_HEIGHT: f32 = 0.5;
    pub const TERRAIN_SAMPLE_LIMIT: f32 = 10000.0;
    
    // Movement processing constants
    pub const TERRAIN_COLLISION_SEARCH_RADIUS: f32 = 40.0; // Search radius for passable terrain (increased for better path finding)
    pub const VELOCITY_STOP_THRESHOLD: f32 = 0.1; // Velocity below which unit is considered stopped
    pub const HEIGHT_SAFETY_MARGIN: f32 = 0.5; // Minimal safety margin above terrain
    pub const DISTANT_TARGET_THRESHOLD: f32 = 5.0; // Distance threshold for applying boundary forces
    // Collision avoidance constants
    pub const MIN_COLLISION_DISTANCE: f32 = 0.1; // Minimum distance to prevent collision overlap
    pub const HARD_COLLISION_DISTANCE: f32 = 0.02; // Very close collision detection
    
    // Force calculations
    pub const LOOSE_SEPARATION_FORCE: f32 = 0.2; // Force for loose unit spacing
    pub const MEDIUM_SEPARATION_FORCE: f32 = 0.5; // Force for medium unit spacing
    pub const DIRECTION_THRESHOLD: f32 = 0.1; // Minimum direction length for rotation
    pub const MAX_TURN_SPEED: f32 = 0.4; // Maximum turning speed per frame
    
    // Pathfinding constants  
    pub const PATH_CLEAR_THRESHOLD: f32 = 0.1; // Minimum distance for path clearance
    pub const RESOURCE_APPROACH_DISTANCE: f32 = 5.0; // Distance to approach resources
    pub const GATHERER_APPROACH_DISTANCE: f32 = 15.0; // Close approach distance for gatherers
    pub const BUILDING_SAFETY_DISTANCE: f32 = 5.0; // Safety distance from buildings
    
    // Destination spreading
    pub const DESTINATION_RADIUS: f32 = 25.0; // Radius for destination spreading
    pub const SPREAD_DISTANCE: f32 = 12.0; // Distance to spread units apart
    pub const SPREAD_THRESHOLD: f32 = 0.1; // Minimum spread direction length
    pub const PATH_VARIATION_STRENGTH: f32 = 0.15; // Strength of path variation offset
    
}

// === CAMERA SYSTEM ===
pub mod camera {
    // Camera height constraints relative to terrain
    pub const MIN_HEIGHT_ABOVE_TERRAIN: f32 = 50.0; // Never go below 5 units above terrain
    pub const MAX_HEIGHT_ABOVE_TERRAIN: f32 = 20000.0; // Never go above 20000 units above terrain - allows viewing entire 5x expanded map
    pub const ZOOM_SPEED_BASE: f32 = 15.0; // Base zoom speed (unused for scroll wheel)
    pub const CAMERA_MOVE_SPEED: f32 = 250.0; // Base camera movement speed

    // Scroll wheel zoom settings
    pub const SCROLL_ZOOM_SENSITIVITY: f32 = 2.0; // Units of movement per scroll wheel click (reduced for better control)
    pub const ZOOM_SPEED_FAST_MULTIPLIER: f32 = 5.0; // Shift key multiplier (reduced)
    pub const ZOOM_SPEED_HYPER_MULTIPLIER: f32 = 15.0; // Alt key multiplier (reduced)

    // Camera look sensitivity
    pub const LOOK_SENSITIVITY: f32 = 0.02;
    pub const PITCH_LIMIT: f32 = 1.5; // Maximum pitch angle in radians
}

// === COLLISION SYSTEM ===
pub mod collision {
    // Unit collision radii
    pub const WORKER_ANT_COLLISION_RADIUS: f32 = 6.0;
    pub const DEFAULT_UNIT_COLLISION_RADIUS: f32 = 10.0;

    // Building collision radii
    pub const NURSERY_COLLISION_RADIUS: f32 = 8.0;
    pub const WARRIOR_CHAMBER_COLLISION_RADIUS: f32 = 10.0;
    pub const QUEEN_COLLISION_RADIUS: f32 = 12.0;
    pub const BEETLE_KNIGHT_COLLISION_RADIUS: f32 = 8.0;
    pub const DEFAULT_BUILDING_COLLISION_RADIUS: f32 = 8.0;
}

// === UI CONSTANTS ===
pub mod ui {
    use bevy::prelude::*;

    // UI panel dimensions for click area detection
    pub const BOTTOM_UI_HEIGHT: f32 = 180.0;    // Bottom panel (building/unit UI)
    pub const RIGHT_UI_WIDTH: f32 = 250.0;      // Right panel (resource display, minimap)
    pub const TOP_UI_HEIGHT: f32 = 60.0;        // Top panel (if any)

    // Top resource bar
    pub const RESOURCE_BAR_HEIGHT: f32 = 60.0;
    pub const RESOURCE_BAR_PADDING: f32 = 10.0;
    pub const RESOURCE_ICON_SIZE: f32 = 20.0;
    pub const RESOURCE_TEXT_SIZE: f32 = 16.0;

    // Bottom building panel
    pub const BUILDING_PANEL_HEIGHT: f32 = 300.0;
    pub const BUILDING_PANEL_PADDING: f32 = 10.0;
    pub const BUILDING_PANEL_BUILDINGS_WIDTH: f32 = 70.0;
    pub const BUILDING_PANEL_UNITS_WIDTH: f32 = 30.0;

    // Building buttons
    pub const BUILDING_BUTTON_SIZE: f32 = 80.0;
    pub const BUILDING_BUTTON_BORDER: f32 = 2.0;
    pub const BUILDING_BUTTON_ICON_SIZE: f32 = 24.0;
    pub const BUILDING_BUTTON_TEXT_SIZE: f32 = 10.0;

    // Unit buttons
    pub const UNIT_BUTTON_HEIGHT: f32 = 10.0;
    pub const UNIT_BUTTON_BORDER: f32 = 1.0;
    pub const UNIT_BUTTON_PADDING: f32 = 5.0;
    pub const UNIT_BUTTON_TEXT_SIZE: f32 = 8.0;

    // Panel titles
    pub const PANEL_TITLE_SIZE: f32 = 18.0;

    // Layout spacing
    pub const BUTTON_GAP: f32 = 5.0;

    // Colors
    pub const BACKGROUND_COLOR: Color = Color::srgba(0.1, 0.1, 0.1, 0.8);
    pub const PANEL_COLOR: Color = Color::srgba(0.15, 0.15, 0.15, 0.9);
    pub const BUTTON_COLOR: Color = Color::srgba(0.2, 0.2, 0.2, 0.8);
    pub const BUTTON_HOVER_COLOR: Color = Color::srgba(0.3, 0.3, 0.3, 0.8);
    pub const UNIT_BUTTON_COLOR: Color = Color::srgba(0.3, 0.3, 0.3, 0.8);
    pub const UNIT_BUTTON_HOVER_COLOR: Color = Color::srgba(0.4, 0.4, 0.4, 0.8);
    pub const BORDER_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);
    pub const BUTTON_BORDER_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);
    pub const PANEL_BORDER_COLOR: Color = Color::srgb(0.4, 0.4, 0.4);
    pub const UNIT_BORDER_COLOR: Color = Color::srgb(0.6, 0.6, 0.6);
    pub const TEXT_COLOR: Color = Color::WHITE;
}

// === TEAM COLORS ===
pub mod team_colors {
    use bevy::prelude::*;

    // Team color tints for GLB models and materials
    pub const PLAYER_1_TINT: Color = Color::srgba(0.1, 0.3, 1.0, 1.0); // Bright Cyan-Blue
    pub const PLAYER_2_TINT: Color = Color::srgba(1.0, 0.1, 0.1, 1.0); // Bright Red
    pub const PLAYER_3_TINT: Color = Color::srgba(0.1, 0.8, 0.1, 1.0); // Green
    pub const PLAYER_4_TINT: Color = Color::srgba(1.0, 0.8, 0.0, 1.0); // Yellow
    pub const PLAYER_5_TINT: Color = Color::srgba(0.8, 0.1, 0.8, 1.0); // Purple
    pub const PLAYER_6_TINT: Color = Color::srgba(0.0, 0.8, 0.8, 1.0); // Teal
    pub const PLAYER_7_TINT: Color = Color::srgba(1.0, 0.5, 0.0, 1.0); // Orange
    pub const PLAYER_8_TINT: Color = Color::srgba(0.8, 0.8, 0.8, 1.0); // White/Silver
    pub const UNKNOWN_PLAYER_TINT: Color = Color::srgba(0.5, 0.5, 0.5, 1.0); // Dark Gray

    // Team colors for primitive models
    pub const PLAYER_1_PRIMITIVE: Color = Color::srgb(0.1, 0.4, 1.0); // Bright cyan-blue
    pub const PLAYER_2_PRIMITIVE: Color = Color::srgb(1.0, 0.1, 0.1); // Bright red
    pub const PLAYER_3_PRIMITIVE: Color = Color::srgb(0.1, 0.8, 0.1); // Green
    pub const PLAYER_4_PRIMITIVE: Color = Color::srgb(1.0, 0.8, 0.0); // Yellow
    pub const PLAYER_5_PRIMITIVE: Color = Color::srgb(0.8, 0.1, 0.8); // Purple
    pub const PLAYER_6_PRIMITIVE: Color = Color::srgb(0.0, 0.8, 0.8); // Teal
    pub const PLAYER_7_PRIMITIVE: Color = Color::srgb(1.0, 0.5, 0.0); // Orange
    pub const PLAYER_8_PRIMITIVE: Color = Color::srgb(0.8, 0.8, 0.8); // White/Silver
    pub const UNKNOWN_PLAYER_PRIMITIVE: Color = Color::srgb(0.6, 0.6, 0.6); // Light gray
}

// === BUILDING SYSTEM ===
pub mod buildings {
    use bevy::prelude::*;

    // Building dimensions (width, height, depth)
    pub const NURSERY_SIZE: Vec3 = Vec3::new(4.0, 3.0, 4.0);
    pub const WARRIOR_CHAMBER_SIZE: Vec3 = Vec3::new(6.0, 4.0, 6.0);
    pub const HUNTER_CHAMBER_SIZE: Vec3 = Vec3::new(5.0, 4.0, 5.0);
    pub const FUNGAL_GARDEN_SIZE: Vec3 = Vec3::new(8.0, 1.0, 8.0);
    pub const DEFAULT_BUILDING_SIZE: Vec3 = Vec3::new(4.0, 3.0, 4.0);

    // Building colors
    pub const NURSERY_COLOR: Color = Color::srgb(0.8, 0.6, 0.4);
    pub const WARRIOR_CHAMBER_COLOR: Color = Color::srgb(0.6, 0.6, 0.6);
    pub const HUNTER_CHAMBER_COLOR: Color = Color::srgb(0.4, 0.6, 0.4);
    pub const FUNGAL_GARDEN_COLOR: Color = Color::srgb(0.3, 0.8, 0.3);
    pub const DEFAULT_BUILDING_COLOR: Color = Color::srgb(0.7, 0.7, 0.7);
    pub const PREVIEW_COLOR: Color = Color::srgba(0.5, 0.5, 1.0, 0.5);

    // Building stats
    pub const DEFAULT_BUILDING_HEALTH: f32 = 500.0;
    pub const DEFAULT_BUILDING_ARMOR: f32 = 5.0;
    pub const CONSTRUCTION_PROGRESS_MAX: f32 = 100.0;
}


// === RESOURCE SYSTEM ===
pub mod resources {
    // Starting resources (balanced to match AI starting resources)
    pub const STARTING_NECTAR: f32 = 120.0;
    pub const STARTING_CHITIN: f32 = 120.0;
    pub const STARTING_MINERALS: f32 = 120.0;
    pub const STARTING_PHEROMONES: f32 = 120.0;
    pub const STARTING_POPULATION_LIMIT: u32 = 200;

    // Resource costs for buildings (using new theme)
    pub const NURSERY_CHITIN_COST: f32 = 1.0;
    pub const WARRIOR_CHAMBER_CHITIN_COST: f32 = 1.0;
    pub const WARRIOR_CHAMBER_MINERALS_COST: f32 = 1.0;
    pub const HUNTER_CHAMBER_CHITIN_COST: f32 = 1.0;
    pub const FUNGAL_GARDEN_CHITIN_COST: f32 = 1.0;
    pub const WOOD_PROCESSOR_CHITIN_COST: f32 = 1.0;
    pub const MINERAL_PROCESSOR_CHITIN_COST: f32 = 1.0;
    pub const DEFAULT_BUILDING_CHITIN_COST: f32 = 1.0;

    // Resource costs for units (using new theme)
    pub const WORKER_ANT_NECTAR_COST: f32 = 1.0;
    pub const SOLDIER_ANT_NECTAR_COST: f32 = 1.0;
    pub const SOLDIER_ANT_PHEROMONES_COST: f32 = 1.0;
    pub const HUNTER_WASP_CHITIN_COST: f32 = 1.0;
    pub const HUNTER_WASP_PHEROMONES_COST: f32 = 1.0;

    // Housing values
    pub const NURSERY_POPULATION_CAPACITY: u32 = 5;
    pub const QUEEN_POPULATION_CAPACITY: u32 = 5;
}

// === AI SYSTEM ===
pub mod ai {
    // AI decision timing
    pub const AI_DECISION_INTERVAL_SECS: f32 = 5.0;

    // Resource generation rates for AI (per second)
    pub const AI_FOOD_RATE: f32 = 2.0;
    pub const AI_WOOD_RATE: f32 = 1.5;
    pub const AI_STONE_RATE: f32 = 0.5;
    pub const AI_GOLD_RATE: f32 = 0.8;

    // AI building thresholds
    pub const AI_MIN_WORKERS_FOR_WARRIOR_CHAMBER: i32 = 5;
    pub const AI_MAX_MILITARY_UNITS_EARLY: i32 = 3;
    pub const AI_MAX_MILITARY_UNITS_MID: i32 = 5;
    pub const AI_MIN_MILITARY_FOR_ATTACK: i32 = 3;
    pub const AI_MIN_MILITARY_FOR_DEFEND: i32 = 2;

    // AI spawn positions
    pub const AI_SPAWN_RANGE: f32 = 50.0;
    pub const AI_SPAWN_HEIGHT: f32 = 5.0;
}

// === COMBAT SYSTEM ===
pub mod combat {
    // Unit spawn positions
    pub const UNIT_SPAWN_RANGE: f32 = 10.0;
    pub const UNIT_SPAWN_OFFSET: f32 = 5.0;
    
    // Initial unit layout spacing at game start
    pub const INITIAL_UNIT_SPACING: f32 = 40.0; // Distance between units when first spawned
}

// === POPULATION MANAGEMENT ===
pub mod population {
    use std::time::Duration;

    pub const UPDATE_INTERVAL: Duration = Duration::from_secs(1);
}

// === RESOURCE INTERACTION ===
pub mod resource_interaction {
    // Resource selection and gathering distances (increased for better usability)
    pub const RESOURCE_CLICK_RADIUS: f32 = 150.0; // Distance for right-clicking to target resources (reduced by half)
    pub const GATHERING_DISTANCE: f32 = 40.0; // Distance within which gathering occurs (doubled for easier collection)
    pub const DROPOFF_TRAVEL_DISTANCE: f32 = 45.0; // Distance threshold for delivering resources

    // Resource source collision - scales with model size
    pub const RESOURCE_COLLISION_BASE_RADIUS: f32 = 0.5; // Base collision radius that gets multiplied by model scale (reduced to fix collection)
    pub const RESOURCE_SELECTION_MULTIPLIER: f32 = 2.0; // Selection radius = model_scale * this multiplier
    pub const MIN_SELECTION_RADIUS: f32 = 15.0; // Minimum selection radius for easy clicking
    
    // Legacy constant for compatibility - use RESOURCE_COLLISION_BASE_RADIUS instead
    pub const RESOURCE_COLLISION_RADIUS: f32 = RESOURCE_COLLISION_BASE_RADIUS;
}

// === TERRAIN SYSTEM ===
pub mod terrain {
    // Terrain generation limits
    pub const MAX_TERRAIN_COORDINATE: f32 = 10000.0;

    // Chunk management
    pub const CHUNK_SIZE: f32 = 64.0;
    pub const VIEW_DISTANCE: f32 = 200.0;
}

// === GRID SYSTEM ===
pub mod grid {
    use bevy::prelude::*;

    // Grid spacing and rendering
    pub const GRID_SPACING: f32 = 50.0; // Grid lines every 50 units
    pub const GRID_SIZE: f32 = 2000.0; // Total grid size (extends from -1000 to +1000)
    pub const GRID_LINE_WIDTH: f32 = 0.5; // Width of grid lines
    pub const GRID_HEIGHT: f32 = 1.0; // Height above terrain

    // Grid colors
    pub const GRID_COLOR: Color = Color::srgba(0.5, 0.5, 0.5, 0.3); // Semi-transparent gray
    pub const GRID_MAJOR_COLOR: Color = Color::srgba(0.7, 0.7, 0.7, 0.5); // Slightly brighter for major lines
}

// === INPUT HOTKEYS ===
pub mod hotkeys {
    use bevy::prelude::KeyCode;

    pub const BUILD_WARRIOR_CHAMBER: KeyCode = KeyCode::KeyB;
    pub const BUILD_NURSERY: KeyCode = KeyCode::KeyH;
    pub const BUILD_FUNGAL_GARDEN: KeyCode = KeyCode::KeyF;
    pub const CANCEL_BUILD: KeyCode = KeyCode::Escape;

    // Time control hotkeys
    pub const SPEED_UP: KeyCode = KeyCode::Equal; // + key
    pub const SLOW_DOWN: KeyCode = KeyCode::Minus; // - key
    pub const RESET_SPEED: KeyCode = KeyCode::Backspace; // Reset to normal speed

    // Debug/utility hotkeys
    pub const TOGGLE_GRID: KeyCode = KeyCode::KeyL; // Toggle grid lines
}

// === BUILDING PLACEMENT ===
pub mod building_placement {
    // Minimum spacing between buildings to prevent overlap
    pub const MIN_SPACING_BETWEEN_BUILDINGS: f32 = 2.0;

    // Minimum spacing from units when placing buildings
    pub const MIN_SPACING_FROM_UNITS: f32 = 5.0;

    // Minimum spacing from environment objects (rocks, trees, etc.)
    pub const MIN_SPACING_FROM_ENVIRONMENT: f32 = 3.0;

    // Visual feedback colors for placement preview
    pub const VALID_PLACEMENT_COLOR: bevy::prelude::Color =
        bevy::prelude::Color::srgba(0.5, 1.0, 0.5, 0.5);
    pub const INVALID_PLACEMENT_COLOR: bevy::prelude::Color =
        bevy::prelude::Color::srgba(1.0, 0.5, 0.5, 0.5);
}

// === UI STYLING ===
pub mod ui_styling {
    use bevy::prelude::Color;

    // Health bar dimensions
    pub const HEALTH_BAR_WIDTH: f32 = 3.5;
    pub const HEALTH_BAR_HEIGHT: f32 = 0.6;
    pub const HEALTH_BAR_Y_OFFSET: f32 = 3.0;
    pub const HEALTH_BAR_FOREGROUND_OFFSET: f32 = 0.1;

    // Health bar colors
    pub const HEALTH_BAR_BACKGROUND_COLOR: Color = Color::srgb(0.8, 0.2, 0.2);
    pub const HEALTH_BAR_HEALTHY_COLOR: Color = Color::srgb(0.2, 0.8, 0.2);
    pub const HEALTH_BAR_WOUNDED_COLOR: Color = Color::srgb(1.0, 0.8, 0.0);
    pub const HEALTH_BAR_CRITICAL_COLOR: Color = Color::srgb(1.0, 0.2, 0.2);

    // Health status thresholds
    pub const HEALTH_THRESHOLD_HEALTHY: f32 = 0.8; // 80%+
    pub const HEALTH_THRESHOLD_WOUNDED: f32 = 0.4; // 40-79%
                                                   // Below HEALTH_THRESHOLD_WOUNDED is critical (0-39%)
}
