//! Game configuration constants: the magic numbers and tuning values used throughout the game.

// === WINDOW AND DISPLAY ===
pub const WINDOW_TITLE: &str = "BioDynasties3";
pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 720.0;

// === MOVEMENT SYSTEM ===
pub mod movement {
    /// Map extends from -MAP_SIZE to +MAP_SIZE on both axes. Single source of truth for all boundaries.
    pub const MAP_SIZE: f32 = 3000.0;
    pub const MAP_BOUNDARY: f32 = MAP_SIZE;
    /// Camera may travel 200 units beyond the map edge.
    pub const CAMERA_BOUNDARY: f32 = MAP_SIZE + 200.0;
    pub const TERRAIN_SIZE: f32 = MAP_SIZE;

    /// A unit within this distance of its target has arrived.
    pub const ARRIVAL_THRESHOLD: f32 = 8.0;
    /// Height units stand above the terrain surface.
    pub const DEFAULT_SPAWN_HEIGHT: f32 = 0.5;
    /// Maximum turning speed per frame.
    pub const MAX_TURN_SPEED: f32 = 0.8;
}

// === CAMERA SYSTEM ===
pub mod camera {
    pub const MIN_HEIGHT_ABOVE_TERRAIN: f32 = 50.0;
    /// High enough to view the entire map.
    pub const MAX_HEIGHT_ABOVE_TERRAIN: f32 = 20000.0;
    pub const CAMERA_MOVE_SPEED: f32 = 250.0;

    /// World units moved per scroll-wheel click.
    pub const SCROLL_ZOOM_SENSITIVITY: f32 = 2.0;
    /// Zoom multiplier while Shift is held.
    pub const ZOOM_SPEED_FAST_MULTIPLIER: f32 = 5.0;
    /// Zoom multiplier while Alt is held.
    pub const ZOOM_SPEED_HYPER_MULTIPLIER: f32 = 15.0;
}

// === COLLISION SYSTEM ===
pub mod collision {
    pub const WORKER_ANT_COLLISION_RADIUS: f32 = 6.0;
    pub const DEFAULT_UNIT_COLLISION_RADIUS: f32 = 10.0;
    pub const BEETLE_KNIGHT_COLLISION_RADIUS: f32 = 8.0;

    pub const NURSERY_COLLISION_RADIUS: f32 = 8.0;
    pub const WARRIOR_CHAMBER_COLLISION_RADIUS: f32 = 10.0;
    pub const QUEEN_COLLISION_RADIUS: f32 = 12.0;
}

// === UI CONSTANTS ===
pub mod ui {
    /// Screen regions covered by UI panels (clicks there don't reach the world).
    pub const BOTTOM_UI_HEIGHT: f32 = 180.0;
    pub const RIGHT_UI_WIDTH: f32 = 250.0;
    pub const TOP_UI_HEIGHT: f32 = 60.0;
}

// === TEAM COLORS ===
pub mod team_colors {
    use bevy::prelude::*;

    pub const PLAYER_2_PRIMITIVE: Color = Color::srgb(1.0, 0.1, 0.1); // Bright red
    pub const PLAYER_3_PRIMITIVE: Color = Color::srgb(0.1, 0.8, 0.1); // Green
    pub const PLAYER_4_PRIMITIVE: Color = Color::srgb(1.0, 0.8, 0.0); // Yellow
    pub const PLAYER_5_PRIMITIVE: Color = Color::srgb(0.8, 0.1, 0.8); // Purple
    pub const PLAYER_6_PRIMITIVE: Color = Color::srgb(0.0, 0.8, 0.8); // Teal
    pub const PLAYER_7_PRIMITIVE: Color = Color::srgb(1.0, 0.5, 0.0); // Orange
    pub const UNKNOWN_PLAYER_PRIMITIVE: Color = Color::srgb(0.6, 0.6, 0.6); // Light gray
}

// === RESOURCE SYSTEM ===
pub mod resources {
    pub const STARTING_NECTAR: f32 = 120.0;
    pub const STARTING_CHITIN: f32 = 120.0;
    pub const STARTING_MINERALS: f32 = 120.0;
    pub const STARTING_PHEROMONES: f32 = 120.0;
}

// === RESOURCE INTERACTION ===
pub mod resource_interaction {
    /// Right-clicks within this distance of a resource target it.
    pub const RESOURCE_CLICK_RADIUS: f32 = 150.0;
    /// Gathering happens within this distance of a resource.
    pub const GATHERING_DISTANCE: f32 = 40.0;
    /// Resources are delivered within this distance of a drop-off building.
    pub const DROPOFF_TRAVEL_DISTANCE: f32 = 45.0;
}

// === GRID SYSTEM ===
pub mod grid_overlay {
    use bevy::prelude::*;

    pub const GRID_SPACING: f32 = 50.0;
    /// Grid extends from -GRID_SIZE / 2 to +GRID_SIZE / 2.
    pub const GRID_SIZE: f32 = 2000.0;
    pub const GRID_LINE_WIDTH: f32 = 0.5;
    /// Height of grid lines above the terrain.
    pub const GRID_HEIGHT: f32 = 1.0;

    pub const GRID_COLOR: Color = Color::srgba(0.5, 0.5, 0.5, 0.3);
    pub const GRID_MAJOR_COLOR: Color = Color::srgba(0.7, 0.7, 0.7, 0.5);
}

// === INPUT HOTKEYS ===
pub mod hotkeys {
    use bevy::prelude::KeyCode;

    pub const TOGGLE_GRID: KeyCode = KeyCode::KeyL;
}
