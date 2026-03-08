use bevy::prelude::*;
use crate::core::components::*;
use crate::core::constants::{camera, movement};
use crate::world::static_terrain::StaticTerrainHeights;

const INITIAL_CAMERA_HEIGHT: f32 = 400.0;
const INITIAL_CAMERA_LOOK_DISTANCE: f32 = 200.0;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_scene, spawn_test_entities))
            .add_systems(Update, handle_rts_camera_input);
    }
}

/// Spawns a worker ant, anthill base, and a nectar resource for basic system testing.
fn spawn_test_entities(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Worker ant — player unit with resource gathering capability
    commands.spawn((
        SceneRoot(asset_server.load("models/insects/fourmi.glb#Scene0")),
        Transform::from_xyz(80.0, 1.0, 0.0).with_scale(Vec3::splat(15.0)),
        RTSUnit { player_id: 1, unit_type: Some(UnitType::WorkerAnt) },
        Movement { max_speed: 80.0, current_velocity: Vec3::ZERO, target_position: None },
        PathfindingState::default(),
        Position { translation: Vec3::new(80.0, 1.0, 0.0) },
        CollisionRadius { radius: 6.0 },
        SpatialGridPosition::default(),
        Selectable::default(),
        RTSHealth { current: 100.0, max: 100.0 },
        ResourceGatherer {
            gather_rate: 5.0,
            capacity: 10.0,
            carried_amount: 0.0,
            resource_type: None,
            target_resource: None,
        },
    ));

    // Anthill — player base (complete, no construction needed)
    commands.spawn((
        SceneRoot(asset_server.load("models/objects/anthill.glb#Scene0")),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(20.0)),
        Building {
            player_id: 1,
            building_type: BuildingType::Queen,
            construction_progress: 100.0,
            max_construction: 100.0,
            is_complete: true,
        },
        Position { translation: Vec3::ZERO },
        CollisionRadius { radius: 20.0 },
    ));

    // Pine cone — nectar resource source (placed north, away from hills)
    commands.spawn((
        SceneRoot(asset_server.load("models/objects/pine_cone.glb#Scene0")),
        Transform::from_xyz(100.0, 0.0, -200.0).with_scale(Vec3::splat(10.0)),
        ResourceSource {
            resource_type: ResourceType::Nectar,
            amount: 300.0,
        },
        CollisionRadius { radius: 8.0 },
    ));
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
