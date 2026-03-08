//! Grid overlay system for development and debugging
//!
//! Provides a toggleable grid overlay that shows 50x50 unit squares
//! to help with unit placement, distance measurement, and map navigation.

use crate::core::constants::{grid::*, hotkeys::TOGGLE_GRID};
use bevy::prelude::*;

/// Resource to track grid visibility state
#[derive(Resource, Default)]
pub struct GridSettings {
    pub visible: bool,
}

/// Marker component for grid line entities
#[derive(Component)]
pub struct GridLine;

/// Plugin for the grid overlay system
pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GridSettings>()
            .add_systems(Startup, setup_grid_system)
            .add_systems(Update, (handle_grid_toggle, update_grid_visibility));
    }
}

/// Handle keyboard input to toggle grid visibility
fn handle_grid_toggle(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut grid_settings: ResMut<GridSettings>,
) {
    if keyboard.just_pressed(TOGGLE_GRID) {
        grid_settings.visible = !grid_settings.visible;
        info!(
            "Grid overlay: {}",
            if grid_settings.visible { "ON" } else { "OFF" }
        );
    }
}

/// Create grid lines when first enabled
pub fn spawn_grid(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let line_material = materials.add(StandardMaterial {
        base_color: GRID_COLOR,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    let major_line_material = materials.add(StandardMaterial {
        base_color: GRID_MAJOR_COLOR,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    // Create vertical lines (parallel to Z-axis)
    let half_size = GRID_SIZE / 2.0;
    let line_count = (GRID_SIZE / GRID_SPACING) as i32 + 1;

    for i in 0..=line_count {
        let x = -half_size + (i as f32 * GRID_SPACING);

        let is_major_line = i % 4 == 0;
        let material = if is_major_line {
            major_line_material.clone()
        } else {
            line_material.clone()
        };

        let line_mesh = create_line_mesh(
            Vec3::new(x, GRID_HEIGHT, -half_size),
            Vec3::new(x, GRID_HEIGHT, half_size),
        );

        commands.spawn((
            Mesh3d(meshes.add(line_mesh)),
            MeshMaterial3d(material.clone()),
            Transform::IDENTITY,
            Visibility::Hidden,
            GridLine,
        ));
    }

    // Create horizontal lines (parallel to X-axis)
    for i in 0..=line_count {
        let z = -half_size + (i as f32 * GRID_SPACING);

        let is_major_line = i % 4 == 0;
        let material = if is_major_line {
            major_line_material.clone()
        } else {
            line_material.clone()
        };

        let line_mesh = create_line_mesh(
            Vec3::new(-half_size, GRID_HEIGHT, z),
            Vec3::new(half_size, GRID_HEIGHT, z),
        );

        commands.spawn((
            Mesh3d(meshes.add(line_mesh)),
            MeshMaterial3d(material),
            Transform::IDENTITY,
            Visibility::Hidden,
            GridLine,
        ));
    }

    // Add map boundary lines (thick red lines at map edges)
    let boundary_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.0, 0.0, 0.8),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    use crate::core::constants::movement::MAP_BOUNDARY;
    let boundary = MAP_BOUNDARY;

    // Top boundary line
    let top_line = create_thick_line_mesh(
        Vec3::new(-boundary, GRID_HEIGHT + 0.1, boundary),
        Vec3::new(boundary, GRID_HEIGHT + 0.1, boundary),
        2.0,
    );
    commands.spawn((
        Mesh3d(meshes.add(top_line)),
        MeshMaterial3d(boundary_material.clone()),
        Transform::IDENTITY,
        Visibility::Hidden,
        GridLine,
    ));

    // Bottom boundary line
    let bottom_line = create_thick_line_mesh(
        Vec3::new(-boundary, GRID_HEIGHT + 0.1, -boundary),
        Vec3::new(boundary, GRID_HEIGHT + 0.1, -boundary),
        2.0,
    );
    commands.spawn((
        Mesh3d(meshes.add(bottom_line)),
        MeshMaterial3d(boundary_material.clone()),
        Transform::IDENTITY,
        Visibility::Hidden,
        GridLine,
    ));

    // Left boundary line
    let left_line = create_thick_line_mesh(
        Vec3::new(-boundary, GRID_HEIGHT + 0.1, -boundary),
        Vec3::new(-boundary, GRID_HEIGHT + 0.1, boundary),
        2.0,
    );
    commands.spawn((
        Mesh3d(meshes.add(left_line)),
        MeshMaterial3d(boundary_material.clone()),
        Transform::IDENTITY,
        Visibility::Hidden,
        GridLine,
    ));

    // Right boundary line
    let right_line = create_thick_line_mesh(
        Vec3::new(boundary, GRID_HEIGHT + 0.1, -boundary),
        Vec3::new(boundary, GRID_HEIGHT + 0.1, boundary),
        2.0,
    );
    commands.spawn((
        Mesh3d(meshes.add(right_line)),
        MeshMaterial3d(boundary_material),
        Transform::IDENTITY,
        Visibility::Hidden,
        GridLine,
    ));
}

/// Create a simple line mesh between two points
fn create_line_mesh(start: Vec3, end: Vec3) -> Mesh {
    let direction = (end - start).normalize();
    let _length = start.distance(end);
    let perpendicular = Vec3::new(-direction.z, 0.0, direction.x) * GRID_LINE_WIDTH * 0.5;

    let vertices = vec![
        start - perpendicular,
        start + perpendicular,
        end + perpendicular,
        end - perpendicular,
    ];

    let indices = vec![0u32, 1, 2, 0, 2, 3];
    let normals = vec![Vec3::Y; 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    mesh
}

/// Create a thick line mesh between two points (for boundary lines)
fn create_thick_line_mesh(start: Vec3, end: Vec3, width: f32) -> Mesh {
    let direction = (end - start).normalize();
    let _length = start.distance(end);
    let perpendicular = Vec3::new(-direction.z, 0.0, direction.x) * width * 0.5;

    let vertices = vec![
        start - perpendicular,
        start + perpendicular,
        end + perpendicular,
        end - perpendicular,
    ];

    let indices = vec![0u32, 1, 2, 0, 2, 3];
    let normals = vec![Vec3::Y; 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    mesh
}

/// Update visibility of all grid lines based on settings
fn update_grid_visibility(
    grid_settings: Res<GridSettings>,
    mut grid_query: Query<&mut Visibility, With<GridLine>>,
) {
    if grid_settings.is_changed() {
        let visibility = if grid_settings.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        for mut grid_visibility in grid_query.iter_mut() {
            *grid_visibility = visibility;
        }
    }
}

/// System to spawn grid on first run
pub fn setup_grid_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    grid_query: Query<&GridLine>,
) {
    if grid_query.is_empty() {
        spawn_grid(&mut commands, &mut meshes, &mut materials);
    }
}
