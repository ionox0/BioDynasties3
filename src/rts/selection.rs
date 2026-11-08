use crate::core::components::*;
use crate::world::static_terrain::StaticTerrainHeights;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(crate) struct PlayerSelectables<'w, 's> {
    query: Query<'w, 's, (Entity, &'static Selectable, &'static Transform, &'static RTSUnit)>,
}

/// Units whose selection state changed this frame.
type SelectionChanged = (With<RTSUnit>, Changed<Selectable>);

#[derive(SystemParam)]
pub(crate) struct ChangedSelectables<'w, 's> {
    query: Query<'w, 's, (Entity, &'static Selectable, &'static Transform), SelectionChanged>,
}

/// Mouse, keyboard, window and camera state shared by the selection input systems.
#[derive(SystemParam)]
pub(crate) struct SelectionInput<'w, 's> {
    mouse: Res<'w, ButtonInput<MouseButton>>,
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    windows: Query<'w, 's, &'static Window>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform)>,
}

impl SelectionInput<'_, '_> {
    fn is_shift_held(&self) -> bool {
        self.keyboard.pressed(KeyCode::ShiftLeft) || self.keyboard.pressed(KeyCode::ShiftRight)
    }
}

/// Selection change requests, applied by `apply_selection_changes`.
#[derive(SystemParam)]
pub(crate) struct SelectionEvents<'w> {
    changed: EventWriter<'w, SelectionChangedEvent>,
    cleared: EventWriter<'w, SelectionClearedEvent>,
}

/// Assets and terrain lookups used to draw selection rings.
#[derive(SystemParam)]
pub(crate) struct IndicatorAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    terrain: Res<'w, StaticTerrainHeights>,
}

#[derive(SystemParam)]
pub(crate) struct DeselectedUnits<'w, 's> {
    query: Query<'w, 's, &'static Selectable, (With<RTSUnit>, Without<SelectionIndicator>)>,
}

pub struct SelectionPlugin;

impl Plugin for SelectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SelectionChangedEvent>()
            .add_event::<SelectionClearedEvent>()
            .add_systems(
                Update,
                (
                    click_selection_system,
                    drag_selection_system,
                    apply_selection_changes,
                    create_selection_indicators,
                    selection_indicator_system,
                )
                    .chain(),
            );
    }
}

/// Event fired when a single entity's selection state should change.
// Owned by: SelectionPlugin (apply_selection_changes)
#[derive(Event, Debug, Clone)]
pub struct SelectionChangedEvent {
    pub entity: Entity,
    pub is_selected: bool,
}

/// Event fired when all selections should be cleared.
// Owned by: SelectionPlugin (apply_selection_changes)
#[derive(Event, Debug, Clone)]
pub struct SelectionClearedEvent;

/// Check if a click is in the main game area (not UI areas)
fn is_click_in_game_area(cursor_position: Vec2, window: &Window) -> bool {
    use crate::core::constants::ui::*;

    let window_size = Vec2::new(window.width(), window.height());

    cursor_position.x < (window_size.x - RIGHT_UI_WIDTH)
        && cursor_position.y > TOP_UI_HEIGHT
        && cursor_position.y < (window_size.y - BOTTOM_UI_HEIGHT)
}

/// System for raycast-based single-click selection.
/// Fires SelectionChangedEvent / SelectionClearedEvent instead of mutating Selectable directly.
pub fn click_selection_system(
    input: SelectionInput,
    selectables: PlayerSelectables,
    mut events: SelectionEvents,
) {
    if !input.mouse.just_released(MouseButton::Left) {
        return;
    }
    let window = input.windows.single();
    let Some(cursor_position) = window.cursor_position() else { return };
    if !is_click_in_game_area(cursor_position, window) {
        return;
    }
    let Ok((camera, camera_transform)) = input.cameras.get_single() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else { return };

    let shift_held = input.is_shift_held();
    if !shift_held {
        events.cleared.send(SelectionClearedEvent);
    }
    let Some(selected_entity) = find_closest_entity_to_ray(&selectables.query, ray) else { return };
    // Shift toggles the clicked unit; a plain click selects it.
    let is_selected = !shift_held
        || selectables.query.get(selected_entity).map_or(true, |(_, s, _, _)| !s.is_selected);
    events.changed.send(SelectionChangedEvent { entity: selected_entity, is_selected });
}

fn find_closest_entity_to_ray(
    selectables: &Query<(Entity, &Selectable, &Transform, &RTSUnit)>,
    ray: Ray3d,
) -> Option<Entity> {
    let mut closest_entity = None;
    let mut closest_distance = f32::INFINITY;

    for (entity, selectable, transform, unit) in selectables.iter() {
        if unit.player_id != 1 {
            continue;
        }

        let to_unit = transform.translation - ray.origin;
        let projected_distance = to_unit.dot(ray.direction.normalize());
        if projected_distance <= 0.0 {
            continue;
        }

        let closest_point = ray.origin + ray.direction.normalize() * projected_distance;
        let distance_to_ray = closest_point.distance(transform.translation);
        let camera_distance = ray.origin.distance(transform.translation);
        let effective_radius = scaled_selection_radius(selectable.selection_radius, camera_distance);

        if distance_to_ray < effective_radius && projected_distance < closest_distance {
            closest_distance = projected_distance;
            closest_entity = Some(entity);
        }
    }

    closest_entity
}

fn scaled_selection_radius(base_radius: f32, camera_distance: f32) -> f32 {
    let scale = if camera_distance <= 100.0 {
        1.0
    } else if camera_distance <= 500.0 {
        1.0 + (camera_distance - 100.0) / 200.0
    } else {
        3.0 + (camera_distance / 500.0).log2().max(0.0)
    };
    (base_radius * scale).max(10.0)
}

/// Drag-box state and the on-screen box it draws.
#[derive(SystemParam)]
pub(crate) struct DragSelectionMut<'w, 's> {
    commands: Commands<'w, 's>,
    drag_query: Query<'w, 's, &'static mut DragSelection>,
    box_query: Query<'w, 's, Entity, With<SelectionBox>>,
}

pub fn drag_selection_system(
    input: SelectionInput,
    selectables: PlayerSelectables,
    mut state: DragSelectionMut,
    mut events: SelectionEvents,
) {
    let window = input.windows.single();
    let Ok(view) = input.cameras.get_single() else { return };
    let Some(cursor_position) = window.cursor_position() else { return };

    if input.mouse.just_pressed(MouseButton::Left) && is_click_in_game_area(cursor_position, window) {
        start_drag_selection(&mut state.drag_query, cursor_position, &mut state.commands);
    }
    if input.mouse.pressed(MouseButton::Left) {
        update_drag_selection(&mut state.drag_query, cursor_position, &state.box_query, &mut state.commands);
    }
    if input.mouse.just_released(MouseButton::Left) {
        finalize_selection(&mut state, &selectables, input.is_shift_held(), view, &mut events);
    }
}

fn start_drag_selection(
    drag_selection_query: &mut Query<&mut DragSelection>,
    cursor_position: Vec2,
    commands: &mut Commands,
) {
    if drag_selection_query.is_empty() {
        commands.spawn(DragSelection {
            start_position: cursor_position,
            current_position: cursor_position,
            is_active: true,
        });
    } else if let Ok(mut drag_selection) = drag_selection_query.get_single_mut() {
        drag_selection.start_position = cursor_position;
        drag_selection.current_position = cursor_position;
        drag_selection.is_active = true;
    }
}

fn update_drag_selection(
    drag_selection_query: &mut Query<&mut DragSelection>,
    cursor_position: Vec2,
    selection_box_query: &Query<Entity, With<SelectionBox>>,
    commands: &mut Commands,
) {
    let Ok(mut drag_selection) = drag_selection_query.get_single_mut() else {
        return;
    };
    if !drag_selection.is_active {
        return;
    }

    drag_selection.current_position = cursor_position;
    let bounds = calculate_selection_bounds(&drag_selection);

    cleanup_old_selection_box(selection_box_query, commands);

    if is_significant_drag(&bounds) {
        create_visual_selection_box(&bounds, commands);
    }
}

struct SelectionBounds {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

fn calculate_selection_bounds(drag_selection: &DragSelection) -> SelectionBounds {
    SelectionBounds {
        min_x: drag_selection
            .start_position
            .x
            .min(drag_selection.current_position.x),
        max_x: drag_selection
            .start_position
            .x
            .max(drag_selection.current_position.x),
        min_y: drag_selection
            .start_position
            .y
            .min(drag_selection.current_position.y),
        max_y: drag_selection
            .start_position
            .y
            .max(drag_selection.current_position.y),
    }
}

fn is_significant_drag(bounds: &SelectionBounds) -> bool {
    (bounds.max_x - bounds.min_x > 5.0) && (bounds.max_y - bounds.min_y > 5.0)
}

fn cleanup_old_selection_box(
    selection_box_query: &Query<Entity, With<SelectionBox>>,
    commands: &mut Commands,
) {
    for entity in selection_box_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn create_visual_selection_box(bounds: &SelectionBounds, commands: &mut Commands) {
    let width = bounds.max_x - bounds.min_x;
    let height = bounds.max_y - bounds.min_y;

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(bounds.min_x),
            top: Val::Px(bounds.min_y),
            width: Val::Px(width),
            height: Val::Px(height),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BorderColor(Color::srgba(0.3, 0.8, 1.0, 0.8)),
        BackgroundColor(Color::srgba(0.3, 0.8, 1.0, 0.15)),
        SelectionBox,
    ));
}

/// Ends a drag: selects every player unit inside a significant box, then removes the box.
fn finalize_selection(
    state: &mut DragSelectionMut,
    selectables: &PlayerSelectables,
    shift_held: bool,
    view: (&Camera, &GlobalTransform),
    events: &mut SelectionEvents,
) {
    let Ok(mut drag_selection) = state.drag_query.get_single_mut() else { return };
    if !drag_selection.is_active {
        return;
    }
    let bounds = calculate_selection_bounds(&drag_selection);
    if is_significant_drag(&bounds) {
        if !shift_held {
            events.cleared.send(SelectionClearedEvent);
        }
        fire_box_selection_events(&bounds, &selectables.query, shift_held, view, &mut events.changed);
    }
    drag_selection.is_active = false;
    cleanup_old_selection_box(&state.box_query, &mut state.commands);
}

fn fire_box_selection_events(
    bounds: &SelectionBounds,
    selectables: &Query<(Entity, &Selectable, &Transform, &RTSUnit)>,
    shift_held: bool,
    (camera, camera_transform): (&Camera, &GlobalTransform),
    changed_events: &mut EventWriter<SelectionChangedEvent>,
) {
    for (entity, selectable, transform, unit) in selectables.iter() {
        if unit.player_id != 1 {
            continue;
        }
        let Ok(screen_pos) = camera.world_to_viewport(camera_transform, transform.translation) else {
            continue;
        };
        let is_inside = (bounds.min_x..=bounds.max_x).contains(&screen_pos.x)
            && (bounds.min_y..=bounds.max_y).contains(&screen_pos.y);
        if is_inside {
            let is_selected = !shift_held || !selectable.is_selected;
            changed_events.send(SelectionChangedEvent { entity, is_selected });
        }
    }
}

/// Owning system for Selectable — the only system that writes `is_selected`.
pub fn apply_selection_changes(
    mut selectables: Query<&mut Selectable>,
    mut changed_events: EventReader<SelectionChangedEvent>,
    mut cleared_events: EventReader<SelectionClearedEvent>,
) {
    for _ in cleared_events.read() {
        for mut selectable in selectables.iter_mut() {
            selectable.is_selected = false;
        }
    }

    for event in changed_events.read() {
        if let Ok(mut selectable) = selectables.get_mut(event.entity) {
            selectable.is_selected = event.is_selected;
        }
    }
}

fn spawn_selection_indicator(
    entity: Entity,
    transform: &Transform,
    selectable: &Selectable,
    commands: &mut Commands,
    assets: &mut IndicatorAssets,
) {
    let ring_radius = selectable.selection_radius;
    let ring_mesh = create_hollow_ring_mesh(ring_radius, 32);

    let terrain_height = assets.terrain.get_height(transform.translation.x, transform.translation.z);
    let unit_height = transform.translation.y;
    let relative_y_offset = (terrain_height + 0.5) - unit_height;

    let unit_scale = transform.scale.x;
    let indicator_scale = 1.0 / unit_scale;

    let indicator_entity = commands
        .spawn((
            Mesh3d(assets.meshes.add(ring_mesh)),
            MeshMaterial3d(assets.materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.6, 1.0),
                emissive: Color::srgb(0.2, 0.4, 0.8).into(),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                ..default()
            })),
            Transform::from_translation(Vec3::new(0.0, relative_y_offset, 0.0))
                .with_scale(Vec3::splat(indicator_scale)),
            SelectionIndicator { target: entity },
        ))
        .id();

    commands.entity(entity).add_child(indicator_entity);
}

fn create_hollow_ring_mesh(radius: f32, segments: usize) -> Mesh {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();

    let thickness = 0.3;
    let inner_radius = radius - thickness;
    let outer_radius = radius + thickness;

    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        positions.push([cos_angle * inner_radius, 0.0, sin_angle * inner_radius]);
        normals.push([0.0, 1.0, 0.0]);
        positions.push([cos_angle * outer_radius, 0.0, sin_angle * outer_radius]);
        normals.push([0.0, 1.0, 0.0]);
    }

    for i in 0..segments {
        let next = (i + 1) % segments;
        let inner_current = (i * 2) as u32;
        let outer_current = (i * 2 + 1) as u32;
        let inner_next = (next * 2) as u32;
        let outer_next = (next * 2 + 1) as u32;

        indices.push(inner_current);
        indices.push(outer_current);
        indices.push(inner_next);
        indices.push(inner_next);
        indices.push(outer_current);
        indices.push(outer_next);
    }

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    mesh
}

/// System to create selection indicators for newly selected units
pub fn create_selection_indicators(
    mut commands: Commands,
    selectables: ChangedSelectables,
    existing_indicators: Query<&SelectionIndicator>,
    mut assets: IndicatorAssets,
) {
    for (entity, selectable, transform) in selectables.query.iter() {
        let has_indicator = existing_indicators.iter().any(|ind| ind.target == entity);
        if selectable.is_selected && !has_indicator {
            spawn_selection_indicator(entity, transform, selectable, &mut commands, &mut assets);
        }
    }
}

/// System to remove indicators for deselected units
pub fn selection_indicator_system(
    selection_indicators: Query<(Entity, &SelectionIndicator)>,
    deselected: DeselectedUnits,
    mut commands: Commands,
) {
    for (indicator_entity, selection_indicator) in selection_indicators.iter() {
        if let Ok(selectable) = deselected.query.get(selection_indicator.target) {
            if !selectable.is_selected {
                commands
                    .entity(selection_indicator.target)
                    .remove_children(&[indicator_entity]);
                commands.entity(indicator_entity).despawn();
            }
        } else {
            commands.entity(indicator_entity).despawn();
        }
    }
}

