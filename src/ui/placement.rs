//! Building placement UI.
//!
//! Enter placement mode via hotkeys (N/B) or `StartPlacementEvent`.
//! Preview mesh snaps to terrain height and turns red on impassable hills.
//! Left-click commits only on passable terrain; Escape/right-click cancels.

use bevy::prelude::*;
use crate::core::components::BuildingType;
use crate::core::resources::Stockpiles;
use crate::rts::cursor_manager::CursorState;
use crate::entities::entity_factory::EntityFactory;
use crate::world::static_terrain::StaticTerrainHeights;

// ─── Events ──────────────────────────────────────────────────────────────────

/// Fired by UI buttons to begin placing a specific building type.
/// Consumed by `handle_start_placement_events`.
#[derive(Event, Debug, Clone)]
pub struct StartPlacementEvent {
    pub building_type: BuildingType,
}

// ─── Resources & components ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum PlacementMode {
    None,
    Placing(BuildingType),
}

/// Owned by: PlacementPlugin
#[derive(Resource, Debug)]
pub struct BuildingPlacement {
    pub mode: PlacementMode,
    pub preview_entity: Option<Entity>,
    /// True on the frame placement starts — prevents committing on the same click
    /// that triggered the build button.
    skip_first_click: bool,
}

impl Default for BuildingPlacement {
    fn default() -> Self {
        Self { mode: PlacementMode::None, preview_entity: None, skip_first_click: false }
    }
}

/// Stores valid/invalid material handles on the preview entity for fast swaps.
#[derive(Component)]
struct PlacementPreview {
    valid: Handle<StandardMaterial>,
    invalid: Handle<StandardMaterial>,
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct PlacementPlugin;

impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildingPlacement>()
            .add_event::<StartPlacementEvent>()
            .add_systems(
                Update,
                (
                    handle_start_placement_events,
                    placement_hotkeys,
                    handle_placement_cancel,
                    update_preview_position,
                    update_preview_validity,
                    handle_placement_commit,
                )
                    .chain(),
            );
    }
}

// ─── Systems ─────────────────────────────────────────────────────────────────

/// Starts placement mode from UI button events.
fn handle_start_placement_events(
    mut events: EventReader<StartPlacementEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut placement: ResMut<BuildingPlacement>,
) {
    for ev in events.read() {
        begin_placement(&mut commands, &mut meshes, &mut materials, ev.building_type.clone(), &mut placement);
    }
}

/// N → Nursery, B → Warrior Chamber.
fn placement_hotkeys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut placement: ResMut<BuildingPlacement>,
) {
    let chosen = if keyboard.just_pressed(KeyCode::KeyN) { Some(BuildingType::Nursery) }
        else if keyboard.just_pressed(KeyCode::KeyB) { Some(BuildingType::WarriorChamber) }
        else { None };
    if let Some(bt) = chosen {
        begin_placement(&mut commands, &mut meshes, &mut materials, bt, &mut placement);
    }
}

/// Moves the preview mesh to cursor position, snapped to terrain height.
fn update_preview_position(
    placement: Res<BuildingPlacement>,
    cursor: Res<CursorState>,
    terrain: Res<StaticTerrainHeights>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(preview) = placement.preview_entity else { return };
    let Ok(mut tf) = transforms.get_mut(preview) else { return };
    let p = cursor.world_position;
    tf.translation = Vec3::new(p.x, terrain.get_height(p.x, p.z), p.z);
}

/// Colors the preview green (passable) or red (impassable hill).
fn update_preview_validity(
    placement: Res<BuildingPlacement>,
    cursor: Res<CursorState>,
    terrain: Res<StaticTerrainHeights>,
    mut preview_q: Query<(&PlacementPreview, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let Some(entity) = placement.preview_entity else { return };
    let Ok((preview, mut mat)) = preview_q.get_mut(entity) else { return };
    let valid = terrain.is_passable(cursor.world_position.x, cursor.world_position.z);
    mat.0 = if valid { preview.valid.clone() } else { preview.invalid.clone() };
}

/// Escape or right-click cancels the active placement.
fn handle_placement_cancel(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    mut placement: ResMut<BuildingPlacement>,
) {
    if keyboard.just_pressed(KeyCode::Escape) || mouse.just_pressed(MouseButton::Right) {
        cancel_placement(&mut commands, &mut placement);
    }
}

/// Left-click commits placement on passable terrain, deducting the cost.
fn handle_placement_commit(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Res<CursorState>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut placement: ResMut<BuildingPlacement>,
    mut stockpiles: ResMut<Stockpiles>,
    terrain: Res<StaticTerrainHeights>,
) {
    // Skip the same frame placement mode was activated (prevents double-fire from UI button click).
    if placement.skip_first_click {
        placement.skip_first_click = false;
        return;
    }

    let PlacementMode::Placing(ref building_type) = placement.mode.clone() else { return };

    if mouse.just_pressed(MouseButton::Left) {
        let pos = cursor.world_position;
        if !terrain.is_passable(pos.x, pos.z) { return; }
        let cost = building_type.build_cost_nectar();
        let stockpile = stockpiles.get_or_insert_mut(1);
        if stockpile.nectar < cost { return; }
        stockpile.nectar -= cost;
        let y = terrain.get_height(pos.x, pos.z);
        EntityFactory::spawn_building(&mut commands, &asset_server, building_type.clone(), Vec3::new(pos.x, y, pos.z), 1);
        cancel_placement(&mut commands, &mut placement);
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn begin_placement(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    building_type: BuildingType,
    placement: &mut BuildingPlacement,
) {
    if let Some(prev) = placement.preview_entity.take() {
        commands.entity(prev).despawn_recursive();
    }
    let valid = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 1.0, 0.2, 0.45),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let invalid = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.2, 0.2, 0.45),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let preview = commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(8.0, 6.0, 8.0))),
        MeshMaterial3d(valid.clone()),
        PlacementPreview { valid, invalid },
        Transform::default(),
    )).id();
    placement.mode = PlacementMode::Placing(building_type);
    placement.preview_entity = Some(preview);
    placement.skip_first_click = true;
}

fn cancel_placement(commands: &mut Commands, placement: &mut BuildingPlacement) {
    if let Some(prev) = placement.preview_entity.take() {
        commands.entity(prev).despawn_recursive();
    }
    placement.mode = PlacementMode::None;
}

