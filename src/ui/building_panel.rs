//! Building production panel UI.
//!
//! Shows when a building is selected; buttons fire `QueueProductionEvent`.
//! No direct `ProductionQueue` mutation — all changes go through events.

use bevy::prelude::*;
use crate::core::components::{Building, BuildingType, ProductionQueue, Selectable, UnitType};
use crate::rts::production::QueueProductionEvent;

type ProdButtonQuery<'w, 's> = Query<'w, 's, (&'static Interaction, &'static ProductionButton), (Changed<Interaction>, With<Button>)>;

/// Marker for the building panel root node.
#[derive(Component)]
pub struct BuildingPanel;

/// Button that queues a specific unit type.
#[derive(Component, Debug, Clone)]
pub struct ProductionButton {
    pub building: Entity,
    pub unit_type: UnitType,
}

pub struct BuildingPanelPlugin;

impl Plugin for BuildingPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_building_panel)
            .add_systems(
                Update,
                (update_building_panel, handle_production_buttons).chain(),
            );
    }
}

fn setup_building_panel(mut commands: Commands) {
    commands.spawn((
        BuildingPanel,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            left: Val::Px(8.0),
            width: Val::Px(360.0),
            min_height: Val::Px(80.0),
            padding: UiRect::all(Val::Px(8.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
        ZIndex(800),
    ));
}

/// Shows the panel with production buttons for whichever building is selected.
fn update_building_panel(
    mut commands: Commands,
    panel_q: Query<(Entity, &Children), With<BuildingPanel>>,
    mut panel_style_q: Query<&mut Node, With<BuildingPanel>>,
    selected_buildings: Query<(Entity, &Building, &Selectable), With<ProductionQueue>>,
) {
    let Ok((panel_entity, _children)) = panel_q.get_single() else { return };
    let Ok(mut panel_node) = panel_style_q.get_single_mut() else { return };

    let selected = selected_buildings
        .iter()
        .find(|(_, _, sel)| sel.is_selected);

    if let Some((building_entity, building, _)) = selected {
        panel_node.display = Display::Flex;
        // Rebuild buttons for this building
        commands.entity(panel_entity).despawn_descendants();
        let units = production_units(&building.building_type);
        commands.entity(panel_entity).with_children(|parent| {
            parent.spawn((
                Text::new(format!("{:?}", building.building_type)),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::srgb(0.8, 0.8, 0.5)),
            ));
            let mut row = parent.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            });
            row.with_children(|row| {
                for unit_type in units {
                    row.spawn((
                        ProductionButton { building: building_entity, unit_type: unit_type.clone() },
                        Button,
                        Node {
                            width: Val::Px(70.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.9)),
                        BorderColor(Color::srgb(0.4, 0.4, 0.4)),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(unit_label(&unit_type)),
                            TextFont { font_size: 9.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
                }
            });
        });
    } else {
        panel_node.display = Display::None;
    }
}

/// Fires `QueueProductionEvent` when a production button is clicked.
fn handle_production_buttons(
    interaction_q: ProdButtonQuery,
    mut queue_events: EventWriter<QueueProductionEvent>,
) {
    for (interaction, btn) in interaction_q.iter() {
        if *interaction == Interaction::Pressed {
            queue_events.send(QueueProductionEvent {
                building: btn.building,
                unit_type: btn.unit_type.clone(),
            });
        }
    }
}

fn production_units(building_type: &BuildingType) -> Vec<UnitType> {
    match building_type {
        BuildingType::Queen => vec![UnitType::WorkerAnt],
        BuildingType::Nursery => vec![
            UnitType::DragonFly,
            UnitType::Housefly,
            UnitType::ScoutAnt,
        ],
        BuildingType::WarriorChamber => vec![
            UnitType::BeetleKnight,
            UnitType::SpearMantis,
            UnitType::BatteringBeetle,
        ],
    }
}

fn unit_label(unit_type: &UnitType) -> &'static str {
    match unit_type {
        UnitType::WorkerAnt => "Worker\nAnt",
        UnitType::DragonFly => "Dragon\nFly",
        UnitType::Housefly => "House\nFly",
        UnitType::ScoutAnt => "Scout\nAnt",
        UnitType::BeetleKnight => "Beetle\nKnight",
        UnitType::SpearMantis => "Spear\nMantis",
        UnitType::BatteringBeetle => "Battering\nBeetle",
        _ => "Unit",
    }
}
