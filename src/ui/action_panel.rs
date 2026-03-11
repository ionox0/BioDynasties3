//! Bottom action bar — always-visible panel combining:
//!   • BUILD section (left): placement buttons for Nursery and Warrior Chamber.
//!   • PRODUCE section (right): unit buttons for the currently selected building.
//!
//! Fires `StartPlacementEvent` and `QueueProductionEvent` via events only.
//! No direct mutation of `BuildingPlacement` or `ProductionQueue`.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::core::components::{Building, BuildingType, ProductionQueue, Selectable, UnitType};
use crate::rts::production::QueueProductionEvent;
use crate::ui::placement::{building_cost, StartPlacementEvent};

// ─── SystemParam query structs ────────────────────────────────────────────────

#[derive(SystemParam)]
pub(crate) struct BuildButtons<'w, 's> {
    query: Query<'w, 's, (&'static Interaction, &'static BuildButton), (Changed<Interaction>, With<Button>)>,
}

#[derive(SystemParam)]
pub(crate) struct UnitButtons<'w, 's> {
    query: Query<'w, 's, (&'static Interaction, &'static UnitProductionButton), (Changed<Interaction>, With<Button>)>,
}

#[derive(SystemParam)]
pub(crate) struct AllBuildings<'w, 's> {
    query: Query<'w, 's, (Entity, &'static Building, &'static Selectable), With<ProductionQueue>>,
}

#[derive(SystemParam)]
pub(crate) struct ChangedSelectionBuildings<'w, 's> {
    query: Query<'w, 's, Entity, (With<Building>, Changed<Selectable>)>,
}

#[derive(SystemParam)]
pub(crate) struct ActionPanelSections<'w, 's> {
    query: Query<'w, 's, Entity, With<ActionPanelUnitsSection>>,
}

// ─── Components ───────────────────────────────────────────────────────────────

/// Marker for the root action bar node.
#[derive(Component)]
pub struct ActionPanel;

/// Marker for the right (PRODUCE) section — rebuilt on building selection changes.
#[derive(Component)]
struct ActionPanelUnitsSection;

/// Button that starts building placement mode.
#[derive(Component, Clone)]
struct BuildButton {
    building_type: BuildingType,
}

/// Button that queues a unit for production.
#[derive(Component, Clone)]
pub struct UnitProductionButton {
    pub building: Entity,
    pub unit_type: UnitType,
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct ActionPanelPlugin;

impl Plugin for ActionPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_action_panel)
            .add_systems(
                Update,
                (handle_build_buttons, update_units_section, handle_unit_buttons).chain(),
            );
    }
}

// ─── Setup ───────────────────────────────────────────────────────────────────

fn setup_action_panel(mut commands: Commands) {
    commands
        .spawn((
            ActionPanel,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                height: Val::Px(88.0),
                flex_direction: FlexDirection::Row,
                padding: UiRect::all(Val::Px(8.0)),
                column_gap: Val::Px(12.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.08, 0.08, 0.92)),
            ZIndex(800),
        ))
        .with_children(|parent| {
            spawn_build_section(parent);
            // Vertical divider
            parent.spawn((
                Node { width: Val::Px(1.0), height: Val::Percent(80.0), ..default() },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
            ));
            spawn_produce_section(parent);
        });
}

fn spawn_build_section(parent: &mut ChildBuilder) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            width: Val::Px(220.0),
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new("BUILD"),
                TextFont { font_size: 10.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.5)),
            ));
            col.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(6.0), ..default() })
                .with_children(|row| {
                    spawn_build_button(row, BuildingType::Nursery, "Nursery", building_cost(&BuildingType::Nursery));
                    spawn_build_button(row, BuildingType::WarriorChamber, "Warrior", building_cost(&BuildingType::WarriorChamber));
                });
        });
}

fn spawn_produce_section(parent: &mut ChildBuilder) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new("PRODUCE  (select a building)"),
                TextFont { font_size: 10.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.5)),
            ));
            col.spawn((
                ActionPanelUnitsSection,
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
            ));
        });
}

fn spawn_build_button(parent: &mut ChildBuilder, bt: BuildingType, name: &str, cost: f32) {
    parent
        .spawn((
            BuildButton { building_type: bt },
            Button,
            Node {
                width: Val::Px(80.0),
                height: Val::Px(54.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.35, 0.9)),
            BorderColor(Color::srgb(0.4, 0.4, 0.6)),
        ))
        .with_children(|btn| {
            btn.spawn((Text::new(name), TextFont { font_size: 10.0, ..default() }, TextColor(Color::WHITE)));
            btn.spawn((Text::new(format!("{cost:.0}N")), TextFont { font_size: 9.0, ..default() }, TextColor(Color::srgb(1.0, 0.85, 0.3))));
        });
}

fn spawn_unit_button(parent: &mut ChildBuilder, building: Entity, unit_type: UnitType) {
    let label = unit_label(&unit_type);
    parent
        .spawn((
            UnitProductionButton { building, unit_type },
            Button,
            Node {
                width: Val::Px(64.0),
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
            btn.spawn((Text::new(label), TextFont { font_size: 9.0, ..default() }, TextColor(Color::WHITE)));
        });
}

// ─── Update systems ───────────────────────────────────────────────────────────

fn handle_build_buttons(
    btn_q: BuildButtons,
    mut placement_events: EventWriter<StartPlacementEvent>,
) {
    for (interaction, btn) in btn_q.query.iter() {
        if *interaction == Interaction::Pressed {
            placement_events.send(StartPlacementEvent { building_type: btn.building_type.clone() });
        }
    }
}

/// Rebuilds unit buttons whenever any building's selection state changes.
fn update_units_section(
    mut commands: Commands,
    section: ActionPanelSections,
    changed: ChangedSelectionBuildings,
    all_buildings: AllBuildings,
) {
    if changed.query.is_empty() { return; }
    let Ok(section_entity) = section.query.get_single() else { return };
    commands.entity(section_entity).despawn_descendants();
    let Some((building_entity, building, _)) = all_buildings.query.iter().find(|(_, _, sel)| sel.is_selected) else {
        return;
    };
    commands.entity(section_entity).with_children(|parent| {
        for unit_type in production_units(&building.building_type) {
            spawn_unit_button(parent, building_entity, unit_type);
        }
    });
}

fn handle_unit_buttons(
    btn_q: UnitButtons,
    mut queue_events: EventWriter<QueueProductionEvent>,
) {
    for (interaction, btn) in btn_q.query.iter() {
        if *interaction == Interaction::Pressed {
            queue_events.send(QueueProductionEvent { building: btn.building, unit_type: btn.unit_type.clone() });
        }
    }
}

// ─── Data helpers ─────────────────────────────────────────────────────────────

fn production_units(building_type: &BuildingType) -> Vec<UnitType> {
    match building_type {
        BuildingType::Queen => vec![UnitType::WorkerAnt],
        BuildingType::Nursery => vec![UnitType::DragonFly, UnitType::Housefly, UnitType::ScoutAnt],
        BuildingType::WarriorChamber => vec![UnitType::BeetleKnight, UnitType::SpearMantis, UnitType::BatteringBeetle],
    }
}

fn unit_label(unit_type: &UnitType) -> &'static str {
    match unit_type {
        UnitType::WorkerAnt => "Worker Ant",
        UnitType::DragonFly => "DragonFly",
        UnitType::Housefly => "Housefly",
        UnitType::ScoutAnt => "Scout Ant",
        UnitType::BeetleKnight => "Beetle Knight",
        UnitType::SpearMantis => "Spear Mantis",
        UnitType::BatteringBeetle => "Battering Beetle",
        _ => "Unit",
    }
}
