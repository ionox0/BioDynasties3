use bevy::prelude::*;

// Owned by: SelectionPlugin (apply_selection_changes)
#[derive(Component, Debug, Clone)]
pub struct Selectable {
    pub is_selected: bool,
    pub selection_radius: f32,
}

impl Default for Selectable {
    fn default() -> Self {
        Self { is_selected: false, selection_radius: 8.0 }
    }
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
