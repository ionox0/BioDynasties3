use bevy::prelude::*;

mod ai;
mod core;
mod debug;
mod entities;
mod rendering;
mod rts;
mod scene;
mod ui;
mod world;

use ai::AIPlugin;
use core::constants;
use core::time_controls::TimeControlPlugin;
use core::CollisionPlugin;
use debug::DebugPlugin;
use entities::LifecyclePlugin;
use rendering::{AnimationPlugin, HoverEffectsPlugin};
use rts::{CombatStatePlugin, ConstructionPlugin, MovementPlugin, PathfindingPlugin, ProductionPlugin, ResourceStatePlugin, SelectionPlugin, UnitCommandsPlugin};
use scene::ScenePlugin;
use ui::UIPlugin;
use world::{GridPlugin, SimpleMaterialPlugin, StaticTerrainPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: constants::WINDOW_TITLE.into(),
                resolution: (constants::WINDOW_WIDTH, constants::WINDOW_HEIGHT).into(),
                mode: bevy::window::WindowMode::Windowed,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            SimpleMaterialPlugin,
            StaticTerrainPlugin,
            GridPlugin,
        ))
        .add_plugins((
            ScenePlugin,
            TimeControlPlugin,
            PathfindingPlugin,
            DebugPlugin,
            CollisionPlugin,
            AnimationPlugin,
            HoverEffectsPlugin,
            LifecyclePlugin,
            CombatStatePlugin,
            ResourceStatePlugin,
            SelectionPlugin,
            ConstructionPlugin,
            MovementPlugin,
            UnitCommandsPlugin,
            UIPlugin,
        ))
        .add_plugins((ProductionPlugin, AIPlugin))
        .run();
}
