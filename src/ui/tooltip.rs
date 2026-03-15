use crate::core::components::*;
use bevy::prelude::*;
use bevy::ecs::system::SystemParam;

/// Resource to track which unit is currently being hovered
#[derive(Resource, Default)]
pub struct HoveredUnit {
    pub entity: Option<Entity>,
    pub last_update: f32,
}

/// Resource to track which resource source is currently being hovered
#[derive(Resource, Default)]
pub struct HoveredResource {
    pub entity: Option<Entity>,
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

/// System to detect which resource source is under the cursor
pub fn resource_hover_detection_system(
    mut hovered_resource: ResMut<HoveredResource>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    resources: Query<(Entity, &Transform, &CollisionRadius), With<ResourceSource>>,
) {
    let Ok(window) = windows.get_single() else { return; };
    let Ok((camera, camera_transform)) = camera_q.get_single() else { return; };

    let Some(cursor_position) = window.cursor_position() else {
        hovered_resource.entity = None;
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        hovered_resource.entity = None;
        return;
    };

    let mut closest_distance = f32::INFINITY;
    let mut closest_entity = None;

    for (entity, transform, collision) in resources.iter() {
        let to_entity = transform.translation - ray.origin;
        let projected_distance = to_entity.dot(*ray.direction);
        if projected_distance <= 0.0 {
            continue;
        }
        let closest_point = ray.origin + *ray.direction * projected_distance;
        let distance_to_ray = closest_point.distance(transform.translation);
        if distance_to_ray < collision.radius && projected_distance < closest_distance {
            closest_distance = projected_distance;
            closest_entity = Some(entity);
        }
    }

    hovered_resource.entity = closest_entity;
}

/// Builds tooltip text for a hovered resource source.
fn build_resource_tooltip_content(
    entity: Entity,
    resource_source_query: &Query<(&ResourceSource, &CollisionRadius)>,
) -> Option<String> {
    let (source, _) = resource_source_query.get(entity).ok()?;
    Some(format!("{:?}\n{:.0} remaining", source.resource_type, source.amount))
}

/// Determine the current task of a unit
fn get_unit_task(
    entity: Entity,
    unit_data: &UnitDataQueries,
    unit_state: &UnitStateQueries,
) -> String {
    // Check if it's a building first
    if let Ok(building) = unit_data.building_query.get(entity) {
        let completion_percent =
            (building.construction_progress / building.max_construction * 100.0) as i32;
        return format!(
            "{:?} ({}% complete)",
            building.building_type, completion_percent
        );
    }

    // Check for death first
    if let Ok(health) = unit_state.health_query.get(entity) {
        if health.current <= 0.0 {
            return "Dead".to_string();
        }
    }

    if let Ok(us) = unit_state.unit_state_query.get(entity) {
        let gatherer = unit_data.gatherer_query.get(entity).ok();
        match us {
            UnitState::Idle => return "Idle".to_string(),
            UnitState::Moving => return "Moving".to_string(),
            UnitState::MovingToResource => return "Moving to Resource".to_string(),
            UnitState::Gathering => {
                return gatherer
                    .and_then(|g| g.resource_type.as_ref())
                    .map_or("Gathering Resources".to_string(), |rt| format!("Gathering {rt:?}"));
            }
            UnitState::ReturningToBase | UnitState::DeliveringResources => {
                return gatherer
                    .and_then(|g| g.resource_type.as_ref().map(|rt| (rt, g.carried_amount)))
                    .map_or("Returning to Base".to_string(), |(rt, amt)| {
                        format!("Returning {rt:?} ({amt:.0})")
                    });
            }
            UnitState::InCombat => return "In Combat".to_string(),
            UnitState::MovingToAttack => return "Moving to Attack".to_string(),
            UnitState::MovingToCombat => return "Engaging Enemy".to_string(),
        }
    }

    "Idle".to_string()
}

/// Parameter group for unit data queries to reduce parameter count
#[derive(SystemParam)]
pub struct UnitDataQueries<'w, 's> {
    pub units: Query<'w, 's, (&'static RTSUnit, &'static RTSHealth, &'static Transform)>,
    pub gatherer_query: Query<'w, 's, &'static ResourceGatherer>,
    pub movement_query: Query<'w, 's, &'static Movement>,
    pub building_query: Query<'w, 's, &'static Building>,
    pub resource_source_query: Query<'w, 's, (&'static ResourceSource, &'static CollisionRadius)>,
    pub pathfinding_query: Query<'w, 's, &'static PathfindingState>,
}


/// Parameter group for unit state queries to reduce parameter count
#[derive(SystemParam)]
pub struct UnitStateQueries<'w, 's> {
    pub unit_state_query: Query<'w, 's, &'static UnitState>,
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

    let mut text = format!(
        "{} ({})\nHealth: {:.0}/{:.0}\nTask: {}",
        entity_name, player_name, health.current, health.max, task
    );

    if let Ok(gatherer) = unit_data.gatherer_query.get(entity) {
        if let Some(target) = gatherer.target_resource {
            text.push_str(&format!("\nTarget resource: {target:?}"));
        }
    }

    if let Ok(movement) = unit_data.movement_query.get(entity) {
        match movement.target_position {
            Some(pos) => text.push_str(&format!("\nMove target: ({:.0}, {:.0}, {:.0})", pos.x, pos.y, pos.z)),
            None => text.push_str("\nMove target: none"),
        }
    }

    if let Ok(pf) = unit_data.pathfinding_query.get(entity) {
        let waypoints = pf.path.len().saturating_sub(pf.path_index);
        let failure = if pf.last_pathfinding_failure.is_finite() {
            format!("fail@{:.1}s", pf.last_pathfinding_failure)
        } else {
            "ok".to_string()
        };
        text.push_str(&format!("\nPath: {waypoints} waypoints ({failure})"));
    }

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
    hovered_resource: Res<HoveredResource>,
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

    if let Some(entity) = hovered_resource.entity {
        if let Some(text) = build_resource_tooltip_content(entity, &unit_data.resource_source_query) {
            show_tooltip(&mut tooltip_style, &mut tooltip_bg, &mut tooltip_text, &tooltip_ui.windows, text);
            *tooltip_bg = BackgroundColor(Color::srgba(0.10, 0.12, 0.05, 0.95));
            return;
        }
    }

    let Some((text, player_id)) = build_tooltip_content(hovered_unit.entity, &unit_data, &unit_state) else {
        tooltip_style.display = Display::None;
        return;
    };

    show_tooltip(&mut tooltip_style, &mut tooltip_bg, &mut tooltip_text, &tooltip_ui.windows, text);
    if player_id == 1 {
        *tooltip_bg = BackgroundColor(Color::srgba(0.05, 0.15, 0.15, 0.95));
    } else {
        *tooltip_bg = BackgroundColor(Color::srgba(0.15, 0.05, 0.05, 0.95));
    }
}

fn show_tooltip(
    style: &mut Node,
    _bg: &mut BackgroundColor,
    text: &mut Text,
    windows: &Query<&Window>,
    content: String,
) {
    **text = content;
    if let Ok(window) = windows.get_single() {
        if let Some(cursor_pos) = window.cursor_position() {
            style.left = Val::Px(cursor_pos.x + 15.0);
            style.top = Val::Px(cursor_pos.y + 15.0);
        }
    }
    style.display = Display::Flex;
}
