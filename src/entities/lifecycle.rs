//! Entity lifecycle management.
//!
//! Handles death detection and cleanup for all game entities.

use crate::core::components::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

pub struct LifecyclePlugin;

impl Plugin for LifecyclePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, mark_dying_units);
    }
}

#[derive(SystemParam)]
struct LivingUnits<'w, 's> {
    query: Query<'w, 's, (Entity, &'static RTSHealth), Without<Dying>>,
}

/// Marks units with zero health as `Dying`, preventing duplicate death processing.
/// Sole writer of the `Dying` component.
fn mark_dying_units(mut commands: Commands, units: LivingUnits) {
    for (entity, health) in units.query.iter() {
        if health.current <= 0.0 {
            if let Some(mut ec) = commands.get_entity(entity) {
                ec.insert(Dying);
            }
        }
    }
}
