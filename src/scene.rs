use bevy::prelude::*;
use crate::core::components::*;
use crate::core::constants::{camera, movement};
use crate::core::resources::{Stockpile, Stockpiles};
use crate::entities::entity_factory::EntityFactory;
use crate::world::static_terrain::{StaticTerrainHeights, ROCKY_TERRAIN_HEIGHT_THRESHOLD};

const INITIAL_CAMERA_HEIGHT: f32 = 400.0;
const INITIAL_CAMERA_LOOK_DISTANCE: f32 = 200.0;

/// West edge: 85 % of MAP_BOUNDARY — player 1 spawn.
const PLAYER_SPAWN: Vec3 = Vec3::new(-2550.0, 0.0, 0.0);

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_scene, spawn_player_base, scatter_map_resources))
            .add_systems(Update, handle_rts_camera_input);
    }
}

/// Spawns player 1's Queen building and three worker ants at the west edge.
fn spawn_player_base(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut stockpiles: ResMut<Stockpiles>,
    terrain: Res<StaticTerrainHeights>,
) {
    *stockpiles.get_or_insert_mut(1) = Stockpile::starting();

    let passable = terrain.find_passable_near(Vec2::new(PLAYER_SPAWN.x, PLAYER_SPAWN.z));
    let ground_y = terrain.get_height(passable.x, passable.y);
    let base_pos = Vec3::new(passable.x, ground_y, passable.y);

    commands.spawn((
        SceneRoot(asset_server.load("models/objects/anthill.glb#Scene0")),
        Transform::from_translation(base_pos).with_scale(Vec3::splat(20.0)),
        Building {
            player_id: 1,
            building_type: BuildingType::Queen,
            construction_progress: 100.0,
            max_construction: 100.0,
            is_complete: true,
        },
        Position { translation: base_pos },
        CollisionRadius { radius: 20.0 },
        Selectable { is_selected: false, selection_radius: 10.0 },
        ProductionQueue::default(),
    ));

    let worker_offsets = [
        Vec3::new(60.0, 1.0, 0.0),
        Vec3::new(60.0, 1.0, 40.0),
        Vec3::new(100.0, 1.0, 0.0),
    ];
    for offset in worker_offsets {
        let wp = base_pos + offset;
        let wy = terrain.get_height(wp.x, wp.z) + 1.0;
        EntityFactory::spawn_unit(&mut commands, &asset_server, UnitType::WorkerAnt, Vec3::new(wp.x, wy, wp.z), 1);
    }
}

/// Scatters 50 resource sources across the map using a deterministic LCG.
fn scatter_map_resources(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    terrain: Res<StaticTerrainHeights>,
) {
    for (i, pos) in resource_scatter_positions(&terrain).into_iter().enumerate() {
        let rt = resource_type_for_index(i);
        let amount = resource_amount(&rt);
        commands.spawn((
            SceneRoot(asset_server.load("models/objects/pine_cone.glb#Scene0")),
            Transform::from_translation(pos).with_scale(Vec3::splat(10.0)),
            ResourceSource { resource_type: rt, amount },
            CollisionRadius { radius: 8.0 },
        ));
    }
}

// ─── Resource scatter helpers ────────────────────────────────────────────────

/// Generates 50 passable positions spread across the full map, avoiding spawn bases.
fn resource_scatter_positions(terrain: &StaticTerrainHeights) -> Vec<Vec3> {
    const TOTAL: usize = 50;
    const MIN_SPACING: f32 = 200.0;
    const MIN_FROM_BASE: f32 = 300.0;
    const EXTENT: f32 = 2550.0;

    let base_xz = [Vec2::new(-EXTENT, 0.0), Vec2::new(EXTENT, 0.0)];
    let mut positions: Vec<Vec3> = Vec::with_capacity(TOTAL);
    let mut seed = 0xDEAD_BEEFu64;

    while positions.len() < TOTAL {
        let x = (lcg_next(&mut seed) * 2.0 - 1.0) * EXTENT;
        let z = (lcg_next(&mut seed) * 2.0 - 1.0) * EXTENT;
        let y = terrain.get_height(x, z);

        if y > ROCKY_TERRAIN_HEIGHT_THRESHOLD { continue; }

        let flat = Vec2::new(x, z);
        if base_xz.iter().any(|b| b.distance(flat) < MIN_FROM_BASE) { continue; }
        if positions.iter().any(|p| Vec2::new(p.x, p.z).distance(flat) < MIN_SPACING) { continue; }

        positions.push(Vec3::new(x, y, z));
    }
    positions
}

/// LCG pseudo-random number in [0, 1]. Uses 31 high bits → divide by i32::MAX.
fn lcg_next(seed: &mut u64) -> f32 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*seed >> 33) as f32 / i32::MAX as f32
}

fn resource_type_for_index(i: usize) -> ResourceType {
    match i % 4 {
        0 => ResourceType::Nectar,
        1 => ResourceType::Chitin,
        2 => ResourceType::Minerals,
        _ => ResourceType::Pheromones,
    }
}

fn resource_amount(rt: &ResourceType) -> f32 {
    match rt {
        ResourceType::Nectar => 800.0,
        ResourceType::Chitin => 400.0,
        ResourceType::Minerals => 500.0,
        ResourceType::Pheromones => 350.0,
    }
}

// ─── Camera setup & input ────────────────────────────────────────────────────

fn setup_scene(mut commands: Commands) {
    // RTS camera starts above player 1's base (west edge)
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2550.0, INITIAL_CAMERA_HEIGHT, INITIAL_CAMERA_LOOK_DISTANCE)
            .looking_at(Vec3::new(-2550.0, 0.0, 0.0), Vec3::Y),
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
