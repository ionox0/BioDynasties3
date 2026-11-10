//! Biome texture picker: press T to toggle; click a thumbnail to retexture a terrain layer.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::world::static_terrain::{TerrainLayer, TerrainTextureSelected};

const BIOME_TEXTURES: [&str; 21] = [
    "brown_mud_leaves_01.jpg",
    "damp_beach_sand.jpg",
    "damp_beach_sand_02.jpg",
    "dense_sand.jpg",
    "dry_decay_leaves.jpg",
    "farm_soil.jpg",
    "forest_ground_05.jpg",
    "forest_ground_06.jpg",
    "forest_leaves_02.jpg",
    "forrest_ground_01.jpg",
    "grass_ground.jpg",
    "gravel_floor_04.jpg",
    "gravelly_sand.jpg",
    "leaf_scattered_gravel.jpg",
    "moon_01.jpg",
    "mud_cracked_dry_03.webp",
    "rocky_terrain.jpg",
    "rocky_terrain_02.jpg",
    "rocky_terrain_03.webp",
    "rocky_trail.jpg",
    "sandy_gravel_02.jpg",
];
const THUMBNAIL_SIZE: f32 = 40.0;

#[derive(Component)]
struct BiomePickerPanel;

#[derive(Component)]
struct BiomeButton {
    layer: TerrainLayer,
    path: String,
}

/// Biome buttons whose interaction changed this frame.
#[derive(SystemParam)]
struct ChangedBiomeButtons<'w, 's> {
    query: Query<'w, 's, (&'static Interaction, &'static BiomeButton), Changed<Interaction>>,
}

/// The picker root panel's visibility.
#[derive(SystemParam)]
struct BiomePickerVisibility<'w, 's> {
    query: Query<'w, 's, &'static mut Visibility, With<BiomePickerPanel>>,
}

pub struct BiomePickerPlugin;

impl Plugin for BiomePickerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_biome_picker)
            .add_systems(Update, (toggle_biome_picker, send_biome_selection));
    }
}

fn setup_biome_picker(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            BiomePickerPanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(48.0),
                left: Val::Px(8.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.85)),
            Visibility::Hidden,
            ZIndex(900),
        ))
        .with_children(|panel| {
            for (label, layer) in [("Lowland", TerrainLayer::Low), ("Highland", TerrainLayer::High)] {
                panel.spawn((Text::new(label), TextFont { font_size: 12.0, ..default() }));
                panel
                    .spawn(Node { flex_wrap: FlexWrap::Wrap, max_width: Val::Px(480.0), row_gap: Val::Px(4.0), column_gap: Val::Px(4.0), ..default() })
                    .with_children(|row| spawn_biome_buttons(row, layer, &asset_server));
            }
        });
}

fn spawn_biome_buttons(row: &mut ChildBuilder, layer: TerrainLayer, asset_server: &AssetServer) {
    for name in BIOME_TEXTURES {
        let path = format!("textures/biomes/{name}");
        row.spawn((
            Button,
            ImageNode::new(asset_server.load(&path)),
            Node { width: Val::Px(THUMBNAIL_SIZE), height: Val::Px(THUMBNAIL_SIZE), ..default() },
            BiomeButton { layer, path },
        ));
    }
}

fn toggle_biome_picker(keys: Res<ButtonInput<KeyCode>>, mut panels: BiomePickerVisibility) {
    if !keys.just_pressed(KeyCode::KeyT) {
        return;
    }
    for mut visibility in &mut panels.query {
        visibility.toggle_visible_hidden();
    }
}

fn send_biome_selection(buttons: ChangedBiomeButtons, mut events: EventWriter<TerrainTextureSelected>) {
    for (interaction, button) in &buttons.query {
        if *interaction == Interaction::Pressed {
            events.send(TerrainTextureSelected { layer: button.layer, path: button.path.clone() });
        }
    }
}
