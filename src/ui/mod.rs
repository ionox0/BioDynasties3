//! UI components and systems.

pub mod icons;
pub mod tooltip;

use bevy::prelude::*;
use tooltip::{setup_tooltip, unit_hover_detection_system, update_tooltip_system, HoveredUnit};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoveredUnit>()
            .init_resource::<icons::UIIcons>()
            .add_systems(Startup, (setup_tooltip, icons::load_ui_icons))
            .add_systems(
                Update,
                (unit_hover_detection_system, update_tooltip_system).chain(),
            );
    }
}
