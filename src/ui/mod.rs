//! UI components and systems.

pub mod action_panel;
pub mod ai_goals_debug;
pub mod biome_picker;
pub mod icons;
pub mod placement;
pub mod resource_display;
pub mod tooltip;

use bevy::prelude::*;
use action_panel::ActionPanelPlugin;
use ai_goals_debug::AIGoalsDebugPlugin;
use biome_picker::BiomePickerPlugin;
use placement::PlacementPlugin;
use resource_display::ResourceDisplayPlugin;
use tooltip::{
    setup_tooltip, unit_hover_detection_system, resource_hover_detection_system,
    update_tooltip_system, HoveredUnit, HoveredResource,
};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoveredUnit>()
            .init_resource::<HoveredResource>()
            .init_resource::<icons::UIIcons>()
            .add_plugins((
                ResourceDisplayPlugin,
                ActionPanelPlugin,
                PlacementPlugin,
                AIGoalsDebugPlugin,
                BiomePickerPlugin,
            ))
            .add_systems(Startup, (setup_tooltip, icons::load_ui_icons))
            .add_systems(
                Update,
                (unit_hover_detection_system, resource_hover_detection_system, update_tooltip_system).chain(),
            );
    }
}
