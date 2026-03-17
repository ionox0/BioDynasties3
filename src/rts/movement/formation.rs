//! Formation system.
//!
//! Assigns grid-layout movement positions to a group of units.
//! All movement changes go through `MovementTargetEvent`.

use bevy::prelude::*;
use crate::core::components::RTSUnit;
use super::events::MovementTargetEvent;
use super::formation_events::FormationMoveEvent;

/// Tags a unit as part of a formation, recording its slot offset.
// Owned by: FormationPlugin (apply_formation_move)
#[derive(Component, Debug, Clone)]
pub struct Formation {
    #[allow(dead_code)]
    pub slot_offset: Vec3,
}

const SPACING: f32 = 35.0;

pub struct FormationPlugin;

impl Plugin for FormationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FormationMoveEvent>()
            .add_systems(Update, apply_formation_move);
    }
}

/// Assigns staggered movement destinations so units spread into a grid.
fn apply_formation_move(
    mut commands: Commands,
    mut events: EventReader<FormationMoveEvent>,
    unit_q: Query<&RTSUnit>,
    mut move_events: EventWriter<MovementTargetEvent>,
) {
    for ev in events.read() {
        let columns = default_columns(ev.units.len());
        for (i, &entity) in ev.units.iter().enumerate() {
            if unit_q.get(entity).is_err() {
                continue;
            }
            let offset = grid_offset(i, columns);
            commands.entity(entity).insert(Formation { slot_offset: offset });
            move_events.send(MovementTargetEvent {
                entity,
                target_position: ev.target + offset,
            });
        }
    }
}

fn default_columns(count: usize) -> u32 {
    match count {
        0..=4 => 2,
        5..=9 => 3,
        _ => 4,
    }
}

fn grid_offset(index: usize, columns: u32) -> Vec3 {
    let col = (index as u32 % columns) as f32;
    let row = (index as u32 / columns) as f32;
    // Center the grid on the target point.
    let center_offset = (columns - 1) as f32 * SPACING * 0.5;
    Vec3::new(col * SPACING - center_offset, 0.0, row * SPACING)
}
