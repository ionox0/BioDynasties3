//! Building placement UI.
//!
//! `BuildingPlacement` resource tracks whether the player is currently placing
//! a building.  Press N/W/B to enter placement mode.  A preview mesh follows the
//! cursor; left-click commits the build and deducts the resource cost from `Stockpiles`.

use bevy::prelude::*;
use crate::core::components::BuildingType;
use crate::core::resources::Stockpiles;
use crate::rts::cursor_manager::CursorState;
use crate::entities::entity_factory::EntityFactory;

/// Placement mode: None means idle, Placing(type) means actively placing.
#[derive(Debug, Clone, PartialEq)]
pub enum PlacementMode {
    None,
    Placing(BuildingType),
}

/// Resource that controls building placement state.
// Owned by: PlacementPlugin (placement_input_system, commit_placement)
#[derive(Resource, Debug)]
pub struct BuildingPlacement {
    pub mode: PlacementMode,
    pub preview_entity: Option<Entity>,
}

impl Default for BuildingPlacement {
    fn default() -> Self {
        Self { mode: PlacementMode::None, preview_entity: None }
    }
}

pub struct PlacementPlugin;

impl Plugin for PlacementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildingPlacement>()
            .add_systems(
                Update,
                (placement_hotkeys, update_preview_position, handle_placement_input).chain(),
            );
    }
}

/// N → Nursery, B → Warrior Chamber. Escape cancels.
fn placement_hotkeys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut placement: ResMut<BuildingPlacement>,
) {
    let chosen = if keyboard.just_pressed(KeyCode::KeyN) {
        Some(BuildingType::Nursery)
    } else if keyboard.just_pressed(KeyCode::KeyB) {
        Some(BuildingType::WarriorChamber)
    } else {
        None
    };

    if let Some(building_type) = chosen {
        if let Some(prev) = placement.preview_entity.take() {
            commands.entity(prev).despawn_recursive();
        }
        let preview = commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(8.0, 6.0, 8.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.3, 0.3, 1.0, 0.45),
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::default(),
        ))
        .id();
        placement.mode = PlacementMode::Placing(building_type);
        placement.preview_entity = Some(preview);
    }
}

/// Moves the preview mesh to follow the cursor world position.
fn update_preview_position(
    placement: Res<BuildingPlacement>,
    cursor: Res<CursorState>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(preview) = placement.preview_entity else { return };
    let Ok(mut tf) = transforms.get_mut(preview) else { return };
    tf.translation = cursor.world_position;
}

/// Handles right-click / Escape cancel and left-click commit.
fn handle_placement_input(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    cursor: Res<CursorState>,
    mut placement: ResMut<BuildingPlacement>,
    mut stockpiles: ResMut<Stockpiles>,
) {
    if keyboard.just_pressed(KeyCode::Escape) || mouse.just_pressed(MouseButton::Right) {
        cancel_placement(&mut commands, &mut placement);
        return;
    }

    let PlacementMode::Placing(ref building_type) = placement.mode.clone() else { return };

    if mouse.just_pressed(MouseButton::Left) {
        let cost = building_cost(building_type);
        let stockpile = stockpiles.get_or_insert_mut(1);
        if stockpile.nectar < cost {
            return;
        }
        stockpile.nectar -= cost;
        let pos = cursor.world_position;
        EntityFactory::spawn_building(&mut commands, &asset_server, building_type.clone(), pos, 1);
        cancel_placement(&mut commands, &mut placement);
    }
}

fn cancel_placement(commands: &mut Commands, placement: &mut BuildingPlacement) {
    if let Some(prev) = placement.preview_entity.take() {
        commands.entity(prev).despawn_recursive();
    }
    placement.mode = PlacementMode::None;
}

fn building_cost(building_type: &BuildingType) -> f32 {
    match building_type {
        BuildingType::Queen => 200.0,
        BuildingType::Nursery => 75.0,
        BuildingType::WarriorChamber => 120.0,
    }
}
