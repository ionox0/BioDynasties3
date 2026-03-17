use crate::core::components::*;
use crate::entities::entity_factory::EntityFactory;
use bevy::prelude::*;
use tracing::instrument;

pub struct ConstructionPlugin;

impl Plugin for ConstructionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ConstructionProgressEvent>()
            .add_event::<ConstructionCompletedEvent>()
            .add_event::<SpawnBuildingEvent>()
            .add_systems(
                Update,
                (construction_system, apply_construction_progress, building_spawn_handler).chain(),
            );
    }
}

/// Fired by any system (AI executor, future player shortcut) to spawn a new building.
/// Consumed by `building_spawn_handler`.
#[derive(Event, Debug, Clone)]
pub struct SpawnBuildingEvent {
    pub building_type: BuildingType,
    pub position: Vec3,
    pub player_id: u8,
}

/// Spawns building entities in response to `SpawnBuildingEvent`.
/// Shared infrastructure — handles requests from both AI and player systems.
pub fn building_spawn_handler(
    mut events: EventReader<SpawnBuildingEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for ev in events.read() {
        EntityFactory::spawn_building(
            &mut commands,
            &asset_server,
            ev.building_type.clone(),
            ev.position,
            ev.player_id,
        );
    }
}

/// Fired when a constructor works on a building.
#[derive(Event, Debug, Clone)]
pub struct ConstructionProgressEvent {
    pub building: Entity,
    pub delta: f32,
}

/// Fired when a building reaches full construction.
#[derive(Event, Debug, Clone)]
pub struct ConstructionCompletedEvent {
    pub building: Entity,
}

/// Fires construction progress events — does NOT mutate Building directly.
#[instrument(skip_all)]
pub fn construction_system(
    mut constructors: Query<(&mut Constructor, &Position), With<RTSUnit>>,
    buildings: Query<(Entity, &Building)>,
    time: Res<Time>,
    mut progress_events: EventWriter<ConstructionProgressEvent>,
    mut completed_events: EventWriter<ConstructionCompletedEvent>,
) {
    for (mut constructor, _constructor_pos) in constructors.iter_mut() {
        fire_construction_events(
            &mut constructor,
            &buildings,
            time.delta_secs(),
            &mut progress_events,
            &mut completed_events,
        );
    }
}

fn fire_construction_events(
    constructor: &mut Constructor,
    buildings: &Query<(Entity, &Building)>,
    delta_time: f32,
    progress_events: &mut EventWriter<ConstructionProgressEvent>,
    completed_events: &mut EventWriter<ConstructionCompletedEvent>,
) {
    let Some(target_entity) = constructor.current_target else {
        return;
    };

    let Ok((building_entity, building)) = buildings.get(target_entity) else {
        constructor.current_target = None;
        return;
    };

    if building.is_complete {
        constructor.current_target = None;
        return;
    }

    let delta = constructor.build_speed * delta_time;
    progress_events.send(ConstructionProgressEvent {
        building: building_entity,
        delta,
    });

    if building.construction_progress + delta >= building.max_construction {
        completed_events.send(ConstructionCompletedEvent {
            building: building_entity,
        });
        constructor.current_target = None;
    }
}

/// Owning system for Building — applies construction events to mutate building state.
pub fn apply_construction_progress(
    mut buildings: Query<&mut Building>,
    mut progress_events: EventReader<ConstructionProgressEvent>,
    mut completed_events: EventReader<ConstructionCompletedEvent>,
) {
    for event in progress_events.read() {
        if let Ok(mut building) = buildings.get_mut(event.building) {
            building.construction_progress =
                (building.construction_progress + event.delta).min(building.max_construction);
        }
    }

    for event in completed_events.read() {
        if let Ok(mut building) = buildings.get_mut(event.building) {
            building.is_complete = true;
        }
    }
}
