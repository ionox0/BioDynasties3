//! Formation system.
//!
//! Assigns grid-layout movement positions to a group of units.
//! All movement changes go through `MovementTargetEvent`.

use bevy::prelude::*;
use crate::core::components::RTSUnit;
use super::events::MovementTargetEvent;
use super::formation_events::FormationMoveEvent;
use super::pathfinding::grid::NavGrid;
use super::pathfinding::systems::NavGridResource;

/// Tags a unit as part of a formation, recording its slot offset.
// Owned by: FormationPlugin (apply_formation_move)
#[derive(Component, Debug, Clone)]
pub struct Formation {
    #[allow(dead_code)]
    pub slot_offset: Vec3,
}

const SPACING: f32 = 35.0;
/// How far (in nav cells) a slot may be nudged off blocked terrain.
const SLOT_SNAP_RING: i32 = 8;

pub struct FormationPlugin;

impl Plugin for FormationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FormationMoveEvent>()
            .add_systems(Update, apply_formation_move);
    }
}

/// Assigns each unit a slot in a square grid; the whole group shares one flow field to the centre.
fn apply_formation_move(
    mut commands: Commands,
    mut events: EventReader<FormationMoveEvent>,
    unit_q: Query<&RTSUnit>,
    mut move_events: EventWriter<MovementTargetEvent>,
    grid: Option<Res<NavGridResource>>,
) {
    for ev in events.read() {
        let columns = default_columns(ev.units.len());
        for (i, &entity) in ev.units.iter().enumerate() {
            if unit_q.get(entity).is_err() {
                continue;
            }
            let offset = grid_offset(i, columns);
            commands.entity(entity).insert(Formation { slot_offset: offset });
            let slot = grid.as_ref().map_or(ev.target + offset, |g| passable_slot(&g.0, ev.target + offset));
            move_events.send(MovementTargetEvent { entity, target_position: slot, group_goal: ev.target });
        }
    }
}

/// Roughly square formation: √n columns.
fn default_columns(count: usize) -> u32 {
    ((count as f32).sqrt().ceil() as u32).max(2)
}

/// Moves a slot that lands on blocked terrain to the centre of the nearest passable cell.
fn passable_slot(grid: &NavGrid, slot: Vec3) -> Vec3 {
    let Some(cell) = grid.cell_of(slot) else { return slot };
    match grid.nearest_passable(cell, SLOT_SNAP_RING) {
        Some(open) if open != cell => {
            let center = grid.cell_center(open);
            Vec3::new(center.x, slot.y, center.y)
        }
        _ => slot,
    }
}

fn grid_offset(index: usize, columns: u32) -> Vec3 {
    let col = (index as u32 % columns) as f32;
    let row = (index as u32 / columns) as f32;
    // Center the grid on the target point.
    let center_offset = (columns - 1) as f32 * SPACING * 0.5;
    Vec3::new(col * SPACING - center_offset, 0.0, row * SPACING)
}
