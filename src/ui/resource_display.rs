//! Resource display UI.
//!
//! Reads `Stockpiles` directly (no `PlayerResources` indirection).
//! Uses `Local<u32>` for throttled updates instead of an unsafe static frame counter.

use bevy::prelude::*;
use crate::core::resources::Stockpiles;

const UPDATE_INTERVAL: u32 = 10;

/// Marker for the resource display root panel.
#[derive(Component)]
pub struct ResourceDisplayPanel;

/// Marker for an individual resource text label.
#[derive(Component, Debug, Clone)]
pub struct ResourceLabel {
    pub resource_name: &'static str,
}

pub struct ResourceDisplayPlugin;

impl Plugin for ResourceDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_resource_display)
            .add_systems(Update, update_resource_display);
    }
}

fn setup_resource_display(mut commands: Commands) {
    commands
        .spawn((
            ResourceDisplayPanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(8.0),
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(4.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.85)),
            ZIndex(900),
        ))
        .with_children(|parent| {
            for name in ["Nectar", "Chitin", "Minerals", "Pheromones"] {
                parent.spawn((
                    ResourceLabel { resource_name: name },
                    Text::new(format!("{name}: 0")),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            }
        });
}

fn update_resource_display(
    mut frame: Local<u32>,
    stockpiles: Res<Stockpiles>,
    mut label_q: Query<(&ResourceLabel, &mut Text)>,
) {
    *frame = frame.wrapping_add(1);
    if !(*frame).is_multiple_of(UPDATE_INTERVAL) {
        return;
    }
    let player_stockpile = stockpiles.0.get(&1);
    for (label, mut text) in label_q.iter_mut() {
        let value = player_stockpile.map_or(0.0, |s| match label.resource_name {
            "Nectar" => s.nectar,
            "Chitin" => s.chitin,
            "Minerals" => s.minerals,
            "Pheromones" => s.pheromones,
            _ => 0.0,
        });
        **text = format!("{}: {:.0}", label.resource_name, value);
    }
}
