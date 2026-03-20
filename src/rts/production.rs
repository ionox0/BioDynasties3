//! Unit production system — player and AI agnostic.
//!
//! Any system fires `QueueProductionEvent { building, unit_type }` to schedule
//! a unit. A per-building timer counts down and spawns the unit when ready.
//!
//! ## Component ownership
//!
//! | Field                     | Owner                          | Mechanism     |
//! |---------------------------|--------------------------------|---------------|
//! | `ProductionQueue.queued`  | `apply_production_queue_events`| Event         |
//! | `ProductionQueue.progress`| `production_queue_system`      | Direct write  |

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rand::Rng;
use crate::core::components::*;
use crate::entities::entity_factory::EntityFactory;
use crate::rts::movement::events::MovementTargetEvent;
use crate::world::building_grid::BuildingGrid;
use crate::world::static_terrain::StaticTerrainHeights;

const RALLY_RADIUS: f32 = 480.0;

#[derive(SystemParam)]
struct SpawnAccess<'w> {
    terrain: Res<'w, StaticTerrainHeights>,
    building_grid: Res<'w, BuildingGrid>,
    move_events: EventWriter<'w, MovementTargetEvent>,
}

pub struct ProductionPlugin;

impl Plugin for ProductionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<QueueProductionEvent>()
            .add_systems(
                Update,
                (apply_production_queue_events, production_queue_system).chain(),
            );
    }
}

/// Fired by any system (AI executor, player UI) to enqueue a unit for production.
#[derive(Event, Debug, Clone)]
pub struct QueueProductionEvent {
    pub building: Entity,
    pub unit_type: UnitType,
}

/// Sole writer of `ProductionQueue.queued` — applies `QueueProductionEvent` each frame.
fn apply_production_queue_events(
    mut events: EventReader<QueueProductionEvent>,
    mut queues: Query<&mut ProductionQueue>,
) {
    for ev in events.read() {
        let Ok(mut queue) = queues.get_mut(ev.building) else { continue };
        queue.queued.push(ev.unit_type.clone());
    }
}

/// Advances each building's production timer and spawns the next queued unit when ready.
fn production_queue_system(
    time: Res<Time>,
    mut buildings: Query<(&Building, &Transform, &mut ProductionQueue)>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut spawn: SpawnAccess,
) {
    for (building, tf, mut queue) in buildings.iter_mut() {
        let Some(unit_type) = queue.queued.first().cloned() else { continue };
        queue.progress += time.delta_secs();
        if queue.progress < queue.production_time { continue; }
        queue.progress -= queue.production_time;
        queue.queued.remove(0);
        let raw = tf.translation + Vec3::new(30.0, 0.0, 0.0);
        let pos = spawn.building_grid
            .find_clear_position(raw, &spawn.terrain)
            .unwrap_or_else(|| {
                let p = spawn.terrain.find_passable_near(raw.xz());
                Vec3::new(p.x, spawn.terrain.get_height(p.x, p.y), p.y)
            });
        let spawned = EntityFactory::spawn_unit(&mut commands, &asset_server, unit_type, pos, building.player_id);
        let mut rng = rand::thread_rng();
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let dist = rng.gen_range(0.0..RALLY_RADIUS);
        let rally_xz = spawn.terrain.find_passable_near((tf.translation + Vec3::new(angle.cos() * dist, 0.0, angle.sin() * dist)).xz());
        let rally = Vec3::new(rally_xz.x, spawn.terrain.get_height(rally_xz.x, rally_xz.y), rally_xz.y);
        spawn.move_events.send(MovementTargetEvent { entity: spawned, target_position: rally });
    }
}
