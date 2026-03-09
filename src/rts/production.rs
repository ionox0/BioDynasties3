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

use bevy::prelude::*;
use crate::core::components::*;
use crate::entities::entity_factory::EntityFactory;

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
) {
    for (building, tf, mut queue) in buildings.iter_mut() {
        let Some(unit_type) = queue.queued.first().cloned() else { continue };
        queue.progress += time.delta_secs();
        if queue.progress < queue.production_time {
            continue;
        }
        queue.progress -= queue.production_time;
        queue.queued.remove(0);
        spawn_unit(&mut commands, &asset_server, building.player_id, unit_type, tf.translation);
    }
}

fn spawn_unit(
    commands: &mut Commands,
    asset_server: &AssetServer,
    player_id: u8,
    unit_type: UnitType,
    base_pos: Vec3,
) {
    let pos = base_pos + Vec3::new(30.0, 1.0, 0.0);
    EntityFactory::spawn_unit(commands, asset_server, unit_type, pos, player_id);
}
