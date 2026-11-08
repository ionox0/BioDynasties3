use bevy::prelude::*;
use bevy::diagnostic::DiagnosticsStore;
use crate::core::components::UnitType;
use crate::entities::entity_factory::EntityFactory;
use crate::world::static_terrain::StaticTerrainHeights;

/// Units added per press of the stress-test key.
const STRESS_SPAWN_COUNT: usize = 500;
const STRESS_SPAWN_SPACING: f32 = 12.0;
/// Where stress-test units appear (east of the player base).
const STRESS_SPAWN_ORIGIN: Vec2 = Vec2::new(-2200.0, 0.0);

/// Debug plugin for memory and performance monitoring
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins((
                bevy::diagnostic::FrameTimeDiagnosticsPlugin,
                bevy::diagnostic::EntityCountDiagnosticsPlugin,
                bevy::diagnostic::SystemInformationDiagnosticsPlugin,
            ))
            .add_systems(Update, (
                debug_memory_system,
                debug_performance_system,
                stress_spawn_units,
            ));
    }
}

/// System to track entity growth and memory usage
fn debug_memory_system(
    entities: Query<Entity>,
    units: Query<Entity, With<crate::core::components::RTSUnit>>,
    movements: Query<Entity, With<crate::core::components::Movement>>,
    resources: Query<Entity, With<crate::core::components::ResourceSource>>,
    buildings: Query<Entity, With<crate::core::components::Building>>,
    time: Res<Time>,
    mut last_log: Local<f32>,
) {
    let current_time = time.elapsed_secs();
    if current_time - *last_log <= 10.0 {
        return;
    }

    let total_entities = entities.iter().count();
    let unit_count = units.iter().count();
    let movement_count = movements.iter().count();
    let resource_count = resources.iter().count();
    let building_count = buildings.iter().count();

    info!(
        "MEMORY DEBUG - Total: {}, Units: {}, Movement: {}, Resources: {}, Buildings: {}",
        total_entities, unit_count, movement_count, resource_count, building_count
    );

    *last_log = current_time;
}

/// System to track performance metrics
fn debug_performance_system(
    diagnostics: Res<DiagnosticsStore>,
    time: Res<Time>,
    mut last_perf_log: Local<f32>,
) {
    let current_time = time.elapsed_secs();
    if current_time - *last_perf_log <= 5.0 {
        return;
    }

    if let Some(fps) = diagnostics.get_measurement(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS) {
        if fps.value > 0.0 {
            info!("PERFORMANCE DEBUG - FPS: {:.2}", fps.value);
        }
    }

    if let Some(frame_time) = diagnostics.get_measurement(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FRAME_TIME) {
        info!("PERFORMANCE DEBUG - Frame Time: {:.2} ms", frame_time.value);
    }

    *last_perf_log = current_time;
}

/// F9: spawns a block of player units for movement stress testing.
fn stress_spawn_units(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    terrain: Res<StaticTerrainHeights>,
) {
    if !keys.just_pressed(KeyCode::F9) {
        return;
    }
    let origin = terrain.find_passable_near(STRESS_SPAWN_ORIGIN);
    let columns = (STRESS_SPAWN_COUNT as f32).sqrt().ceil() as usize;
    for i in 0..STRESS_SPAWN_COUNT {
        let x = origin.x + (i % columns) as f32 * STRESS_SPAWN_SPACING;
        let z = origin.y + (i / columns) as f32 * STRESS_SPAWN_SPACING;
        let position = Vec3::new(x, terrain.get_height(x, z) + 1.0, z);
        EntityFactory::spawn_unit(&mut commands, &asset_server, UnitType::RolyPoly, position, 1);
    }
    info!("Stress test: spawned {STRESS_SPAWN_COUNT} units");
}
