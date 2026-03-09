use crate::core::components::*;
use crate::rts::resource::{GatheringState, GatheringStateType};
use bevy::prelude::*;
use bevy::ecs::system::SystemParam;

/// Resource to track which unit is currently being hovered
#[derive(Resource, Default)]
pub struct HoveredUnit {
    pub entity: Option<Entity>,
    pub last_update: f32,
}

/// Component marking the tooltip UI element
#[derive(Component)]
pub struct UnitTooltip;

/// Component for the tooltip text
#[derive(Component)]
pub struct TooltipText;

/// Setup the tooltip UI (invisible by default)
pub fn setup_tooltip(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                display: Display::None, // Hidden by default
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.95)),
            BorderColor(Color::srgb(0.6, 0.6, 0.6)),
            UnitTooltip,
            ZIndex(1000),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TooltipText,
            ));
        });
}

/// System to detect which unit is under the cursor
pub fn unit_hover_detection_system(
    mut hovered_unit: ResMut<HoveredUnit>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    units: Query<(Entity, &Transform, &RTSUnit, &Selectable)>,
    time: Res<Time>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        hovered_unit.entity = None;
        return;
    };

    // Convert cursor position to world ray
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        hovered_unit.entity = None;
        return;
    };

    // Find the closest unit to the cursor ray
    let mut closest_distance = f32::INFINITY;
    let mut closest_entity = None;

    for (entity, transform, _unit, selectable) in units.iter() {
        // Show tooltips for all units (both player and AI)

        // Calculate distance from ray to unit
        let to_entity = transform.translation - ray.origin;
        let projected_distance = to_entity.dot(*ray.direction);

        if projected_distance <= 0.0 {
            continue;
        }

        let closest_point = ray.origin + *ray.direction * projected_distance;
        let distance_to_ray = closest_point.distance(transform.translation);

        // Check if cursor is within selection radius
        if distance_to_ray < selectable.selection_radius && projected_distance < closest_distance {
            closest_distance = projected_distance;
            closest_entity = Some(entity);
        }
    }

    // Update hovered unit
    if hovered_unit.entity != closest_entity {
        hovered_unit.entity = closest_entity;
        hovered_unit.last_update = time.elapsed_secs();
    }
}

/// Determine the current task of a unit
fn get_unit_task(
    entity: Entity,
    unit_data: &UnitDataQueries,
    unit_state: &UnitStateQueries,
) -> String {
    let building_query = &unit_data.building_query;
    let gatherer_query = &unit_data.gatherer_query;
    let combat_query = &unit_data.combat_query;
    let movement_query = &unit_data.movement_query;
    let gathering_state_query = &unit_state.gathering_state_query;
    let combat_state_query = &unit_state.combat_state_query;
    let health_query = &unit_state.health_query;

    // Check if it's a building first
    if let Ok(building) = building_query.get(entity) {
        let completion_percent =
            (building.construction_progress / building.max_construction * 100.0) as i32;
        return format!(
            "{:?} ({}% complete)",
            building.building_type, completion_percent
        );
    }


    // Check for death first
    if let Ok(health) = health_query.get(entity) {
        if health.current <= 0.0 {
            return "Dead".to_string();
        }
    }

    // Check gathering state — authoritative label; augmented with cargo info from ResourceGatherer
    if let Ok(gathering_state) = gathering_state_query.get(entity) {
        let gatherer = gatherer_query.get(entity).ok();
        match gathering_state.state {
            GatheringStateType::Idle => return "Idle".to_string(),
            GatheringStateType::MovingToResource => return "Moving to Resource".to_string(),
            GatheringStateType::Gathering => {
                let label = gatherer
                    .and_then(|g| g.resource_type.as_ref())
                    .map_or("Gathering Resources".to_string(), |rt| format!("Gathering {rt:?}"));
                return label;
            }
            GatheringStateType::ReturningToBase | GatheringStateType::DeliveringResources => {
                let label = gatherer
                    .and_then(|g| g.resource_type.as_ref().map(|rt| (rt, g.carried_amount)))
                    .map_or("Returning to Base".to_string(), |(rt, amt)| {
                        format!("Returning {rt:?} ({amt:.0})")
                    });
                return label;
            }
        }
    }

    // Check combat state
    if let Ok(combat_state) = combat_state_query.get(entity) {
        match combat_state.state {
            CombatStateType::InCombat => return "In Combat".to_string(),
            CombatStateType::MovingToAttack => return "Moving to Attack".to_string(),
            CombatStateType::MovingToCombat => return "Engaging Enemy".to_string(),
            _ => {} // Continue to other checks
        }
    }

    // Check if in combat (only for units that actually auto-attack AND have a target)
    if let Ok(combat) = combat_query.get(entity) {
        if combat.auto_attack && combat.target.is_some() {
            if let Ok(movement) = movement_query.get(entity) {
                if movement.target_position.is_some() {
                    return "Moving to attack".to_string();
                }
            }
            return "In combat".to_string();
        }
    }

    // Check if just moving
    if let Ok(movement) = movement_query.get(entity) {
        if movement.target_position.is_some() {
            return "Moving".to_string();
        }
    }

    // Default to idle
    "Idle".to_string()
}

/// Parameter group for unit data queries to reduce parameter count
#[derive(SystemParam)]
pub struct UnitDataQueries<'w, 's> {
    pub units: Query<'w, 's, (&'static RTSUnit, &'static RTSHealth, &'static Transform)>,
    pub gatherer_query: Query<'w, 's, &'static ResourceGatherer>,
    pub combat_query: Query<'w, 's, &'static Combat>,
    pub movement_query: Query<'w, 's, &'static Movement>,
    pub building_query: Query<'w, 's, &'static Building>,
}

/// Parameter group for unit state queries to reduce parameter count
#[derive(SystemParam)]
pub struct UnitStateQueries<'w, 's> {
    pub gathering_state_query: Query<'w, 's, &'static GatheringState>,
    pub combat_state_query: Query<'w, 's, &'static CombatState>,
    pub health_query: Query<'w, 's, &'static RTSHealth>,
    pub player_teams: Query<'w, 's, &'static PlayerTeam>,
}

/// Parameter group for UI queries to reduce parameter count
#[derive(SystemParam)]
pub struct TooltipUI<'w, 's> {
    pub tooltip_query: Query<'w, 's, (&'static mut Node, &'static mut BackgroundColor), With<UnitTooltip>>,
    pub text_query: Query<'w, 's, &'static mut Text, With<TooltipText>>,
    pub windows: Query<'w, 's, &'static Window>,
}

/// Builds the tooltip text and returns `(formatted_text, player_id)`, or `None` if no unit is hovered.
fn build_tooltip_content(
    hovered_entity: Option<Entity>,
    unit_data: &UnitDataQueries,
    unit_state: &UnitStateQueries,
) -> Option<(String, u8)> {
    let entity = hovered_entity?;
    let (unit, health, _transform) = unit_data.units.get(entity).ok()?;

    let entity_name = unit_display_name(entity, unit, unit_data);
    let player_name = player_display_name(unit, unit_state);
    let task = get_unit_task(entity, unit_data, unit_state);

    let text = format!(
        "{} ({})\nHealth: {:.0}/{:.0}\nTask: {}",
        entity_name, player_name, health.current, health.max, task
    );
    Some((text, unit.player_id))
}

fn unit_display_name(entity: Entity, unit: &RTSUnit, unit_data: &UnitDataQueries) -> &'static str {
    if let Ok(building) = unit_data.building_query.get(entity) {
        return building.building_type.display_name();
    }
    unit.unit_type.as_ref().map_or("Unit", UnitType::display_name)
}

fn player_display_name(unit: &RTSUnit, unit_state: &UnitStateQueries) -> String {
    if unit.player_id == 1 {
        return "Player".to_string();
    }
    unit_state.player_teams.iter()
        .find(|team| team.player_id == unit.player_id)
        .map_or_else(|| format!("AI Player {}", unit.player_id), |team| format!("{:?}", team.team_type))
}

/// System to update tooltip content and position
pub fn update_tooltip_system(
    hovered_unit: Res<HoveredUnit>,
    unit_data: UnitDataQueries,
    unit_state: UnitStateQueries,
    mut tooltip_ui: TooltipUI,
) {
    let Ok((mut tooltip_style, mut tooltip_bg)) = tooltip_ui.tooltip_query.get_single_mut() else {
        return;
    };
    let Ok(mut tooltip_text) = tooltip_ui.text_query.get_single_mut() else {
        return;
    };

    let Some((text, player_id)) = build_tooltip_content(hovered_unit.entity, &unit_data, &unit_state) else {
        tooltip_style.display = Display::None;
        return;
    };

    **tooltip_text = text;

    if let Ok(window) = tooltip_ui.windows.get_single() {
        if let Some(cursor_pos) = window.cursor_position() {
            tooltip_style.left = Val::Px(cursor_pos.x + 15.0);
            tooltip_style.top = Val::Px(cursor_pos.y + 15.0);
        }
    }

    tooltip_style.display = Display::Flex;
    if player_id == 1 {
        *tooltip_bg = BackgroundColor(Color::srgba(0.05, 0.15, 0.15, 0.95));
    } else {
        *tooltip_bg = BackgroundColor(Color::srgba(0.15, 0.05, 0.05, 0.95));
    }
}
