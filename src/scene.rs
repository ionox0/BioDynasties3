use bevy::prelude::*;
use crate::core::components::{MainCamera, RTSCamera};
use crate::core::constants::{camera, movement};
use crate::world::static_terrain::StaticTerrainHeights;

const INITIAL_CAMERA_HEIGHT: f32 = 400.0;
const INITIAL_CAMERA_LOOK_DISTANCE: f32 = 200.0;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_scene)
            .add_systems(Update, handle_rts_camera_input);
    }
}

fn setup_scene(mut commands: Commands) {
    // RTS camera looking down at origin
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, INITIAL_CAMERA_HEIGHT, INITIAL_CAMERA_LOOK_DISTANCE)
            .looking_at(Vec3::ZERO, Vec3::Y),
        MainCamera,
        RTSCamera {
            move_speed: camera::CAMERA_MOVE_SPEED,
        },
    ));

    // Sun
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.95, 0.8),
            illuminance: 32000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.3, 0.0)),
    ));

    // Ambient fill
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.5, 0.5, 0.7),
        brightness: 500.0,
    });
}

pub fn handle_rts_camera_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: EventReader<bevy::input::mouse::MouseWheel>,
    mut camera_query: Query<(&mut Transform, &RTSCamera), With<MainCamera>>,
    terrain_heights: Res<StaticTerrainHeights>,
    time: Res<Time>,
) {
    let Ok((mut transform, rts_camera)) = camera_query.get_single_mut() else {
        return;
    };

    let dt = time.delta_secs();
    let mut movement_delta = Vec3::ZERO;

    if keyboard.pressed(KeyCode::KeyW) { movement_delta += Vec3::NEG_Z; }
    if keyboard.pressed(KeyCode::KeyS) { movement_delta += Vec3::Z; }
    if keyboard.pressed(KeyCode::KeyA) { movement_delta += Vec3::NEG_X; }
    if keyboard.pressed(KeyCode::KeyD) { movement_delta += Vec3::X; }

    if movement_delta.length_squared() > 0.0 {
        let speed_mult = if keyboard.pressed(KeyCode::ShiftLeft) { 10.0 }
            else if keyboard.pressed(KeyCode::AltLeft) { 50.0 }
            else { 1.0 };
        transform.translation += movement_delta.normalize() * rts_camera.move_speed * speed_mult * dt;
        transform.translation.x = transform.translation.x.clamp(-movement::CAMERA_BOUNDARY, movement::CAMERA_BOUNDARY);
        transform.translation.z = transform.translation.z.clamp(-movement::CAMERA_BOUNDARY, movement::CAMERA_BOUNDARY);
    }

    // Terrain-aware height floor
    let terrain_h = terrain_heights.get_height(transform.translation.x, transform.translation.z);
    if transform.translation.y < terrain_h + camera::MIN_HEIGHT_ABOVE_TERRAIN {
        transform.translation.y = terrain_h + camera::MIN_HEIGHT_ABOVE_TERRAIN;
    }
    transform.translation.y = transform.translation.y
        .clamp(camera::MIN_HEIGHT_ABOVE_TERRAIN, camera::MAX_HEIGHT_ABOVE_TERRAIN);

    // Scroll wheel zoom
    for ev in mouse_wheel.read() {
        let speed_mult = if keyboard.pressed(KeyCode::ShiftLeft) { camera::ZOOM_SPEED_FAST_MULTIPLIER }
            else if keyboard.pressed(KeyCode::AltLeft) { camera::ZOOM_SPEED_HYPER_MULTIPLIER }
            else { 1.0 };
        let delta = -ev.y * camera::SCROLL_ZOOM_SENSITIVITY * speed_mult;
        let new_y = transform.translation.y + delta;
        let min = terrain_h + camera::MIN_HEIGHT_ABOVE_TERRAIN;
        let max = terrain_h + camera::MAX_HEIGHT_ABOVE_TERRAIN;
        transform.translation.y = new_y.clamp(min, max);
    }
}
