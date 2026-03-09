//! Resource display UI.
//!
//! Player 1 resources: top-left bar (always visible).
//! AI player resources: top-right panel, one row per AI player.
//!
//! Both panels read `Stockpiles` directly.
//! `manage_ai_resource_panels` adds/removes per-player rows as `Stockpiles` keys change,
//! supporting any number of AI players.

use std::collections::HashSet;
use bevy::prelude::*;
use crate::core::resources::Stockpiles;

const UPDATE_INTERVAL: u32 = 10;

// ─── Components ──────────────────────────────────────────────────────────────

/// Marker for the player-1 resource display root panel.
#[derive(Component)]
pub struct ResourceDisplayPanel;

/// Marker for an individual player-1 resource text label.
#[derive(Component, Debug, Clone)]
pub struct ResourceLabel {
    pub resource_name: &'static str,
}

/// Marker for the AI resource display container (top-right).
#[derive(Component)]
pub struct AIResourceDisplayPanel;

/// Container for a single AI player's resource row.
#[derive(Component, Debug, Clone)]
pub struct AIPlayerPanel {
    pub player_id: u8,
}

/// Resource label within an AI player's panel.
#[derive(Component, Debug, Clone)]
pub struct AIResourceLabel {
    pub player_id: u8,
    pub resource_name: &'static str,
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct ResourceDisplayPlugin;

impl Plugin for ResourceDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_resource_display, setup_ai_resource_panel))
            .add_systems(
                Update,
                (manage_ai_resource_panels, update_resource_display, update_ai_resource_display),
            );
    }
}

// ─── Setup ───────────────────────────────────────────────────────────────────

fn setup_resource_display(mut commands: Commands) {
    commands
        .spawn((
            ResourceDisplayPanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(8.0),
                padding: UiRect::all(Val::Px(8.0)),
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

fn setup_ai_resource_panel(mut commands: Commands) {
    commands
        .spawn((
            AIResourceDisplayPanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                right: Val::Px(8.0),
                min_width: Val::Px(180.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.85)),
            ZIndex(900),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("AI Players"),
                TextFont { font_size: 12.0, ..default() },
                TextColor(Color::srgb(0.8, 0.8, 0.5)),
            ));
        });
}

// ─── Dynamic AI panel management ─────────────────────────────────────────────

/// Adds a row for each new AI player (stockpile key ≥ 2); removes rows for gone players.
fn manage_ai_resource_panels(
    mut commands: Commands,
    stockpiles: Res<Stockpiles>,
    container_q: Query<Entity, With<AIResourceDisplayPanel>>,
    existing_q: Query<(Entity, &AIPlayerPanel)>,
    mut known: Local<HashSet<u8>>,
) {
    let Ok(container) = container_q.get_single() else { return };

    for &player_id in stockpiles.0.keys().filter(|&&id| id >= 2) {
        if known.insert(player_id) {
            commands.entity(container).with_children(|p| {
                spawn_ai_player_panel(p, player_id);
            });
        }
    }

    for (entity, panel) in existing_q.iter() {
        if !stockpiles.0.contains_key(&panel.player_id) {
            known.remove(&panel.player_id);
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn spawn_ai_player_panel(parent: &mut ChildBuilder, player_id: u8) {
    parent
        .spawn((
            AIPlayerPanel { player_id },
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                padding: UiRect::all(Val::Px(4.0)),
                border: UiRect::top(Val::Px(1.0)),
                ..default()
            },
            BorderColor(Color::srgb(0.35, 0.35, 0.35)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(format!("AI Player {player_id}")),
                TextFont { font_size: 10.0, ..default() },
                TextColor(player_color(player_id)),
            ));
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|row| {
                for name in ["Nectar", "Chitin", "Minerals", "Pheromones"] {
                    row.spawn((
                        AIResourceLabel { player_id, resource_name: name },
                        Text::new(format!("{}:0", &name[..1])),
                        TextFont { font_size: 9.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                }
            });
        });
}

fn player_color(player_id: u8) -> Color {
    use crate::core::constants::team_colors as tc;
    match player_id {
        2 => tc::PLAYER_2_PRIMITIVE,
        3 => tc::PLAYER_3_PRIMITIVE,
        4 => tc::PLAYER_4_PRIMITIVE,
        5 => tc::PLAYER_5_PRIMITIVE,
        6 => tc::PLAYER_6_PRIMITIVE,
        7 => tc::PLAYER_7_PRIMITIVE,
        _ => tc::UNKNOWN_PLAYER_PRIMITIVE,
    }
}

// ─── Update systems ──────────────────────────────────────────────────────────

fn update_resource_display(
    mut frame: Local<u32>,
    stockpiles: Res<Stockpiles>,
    mut label_q: Query<(&ResourceLabel, &mut Text)>,
) {
    *frame = frame.wrapping_add(1);
    if !(*frame).is_multiple_of(UPDATE_INTERVAL) {
        return;
    }
    let stockpile = stockpiles.0.get(&1);
    for (label, mut text) in label_q.iter_mut() {
        let value = stockpile.map_or(0.0, |s| match label.resource_name {
            "Nectar" => s.nectar,
            "Chitin" => s.chitin,
            "Minerals" => s.minerals,
            "Pheromones" => s.pheromones,
            _ => 0.0,
        });
        **text = format!("{}: {:.0}", label.resource_name, value);
    }
}

fn update_ai_resource_display(
    mut frame: Local<u32>,
    stockpiles: Res<Stockpiles>,
    mut label_q: Query<(&AIResourceLabel, &mut Text)>,
) {
    *frame = frame.wrapping_add(1);
    if !(*frame).is_multiple_of(UPDATE_INTERVAL) {
        return;
    }
    for (label, mut text) in label_q.iter_mut() {
        let value = stockpiles.0.get(&label.player_id).map_or(0.0, |s| {
            match label.resource_name {
                "Nectar" => s.nectar,
                "Chitin" => s.chitin,
                "Minerals" => s.minerals,
                "Pheromones" => s.pheromones,
                _ => 0.0,
            }
        });
        **text = format!("{}:{:.0}", &label.resource_name[..1], value);
    }
}
