//! Unstuck system — detects and nudges stuck units.
//!
//! Uses `Local<u32>` for a frame counter so no unsafe static state is needed.
//! Sends `MovementTargetEvent` rather than mutating `Movement` directly.

use bevy::prelude::*;
use crate::core::components::{Movement, PathfindingState, Position, RTSUnit};
use super::events::MovementTargetEvent;

const CHECK_INTERVAL: u32 = 30;
const STUCK_DISTANCE_THRESHOLD: f32 = 1.0;
const NUDGE_DISTANCE: f32 = 20.0;

/// Per-entity stuck tracking stored as a resource keyed by entity.
#[derive(Component, Debug, Clone)]
pub struct StuckDetection {
    pub last_sampled_pos: Vec3,
    pub stuck_frames: u32,
}

impl Default for StuckDetection {
    fn default() -> Self {
        Self { last_sampled_pos: Vec3::ZERO, stuck_frames: 0 }
    }
}

/// Attaches `StuckDetection` to newly spawned moveable units.
pub fn add_stuck_detection(
    mut commands: Commands,
    new_units: Query<Entity, (Added<RTSUnit>, With<Movement>)>,
) {
    for entity in new_units.iter() {
        commands.entity(entity).insert(StuckDetection::default());
    }
}

/// Detects units stuck in place (have a target but aren't moving) and nudges them.
pub fn unstuck_system(
    mut frame: Local<u32>,
    mut units: Query<(Entity, &Position, &Movement, &PathfindingState, &mut StuckDetection)>,
    mut move_events: EventWriter<MovementTargetEvent>,
) {
    *frame = frame.wrapping_add(1);
    if !(*frame).is_multiple_of(CHECK_INTERVAL) {
        return;
    }
    for (entity, pos, movement, pf, mut stuck) in units.iter_mut() {
        let has_target = movement.target_position.is_some() || !pf.path.is_empty();
        if !has_target {
            stuck.stuck_frames = 0;
            stuck.last_sampled_pos = pos.translation;
            continue;
        }
        let moved = pos.translation.distance(stuck.last_sampled_pos);
        if moved < STUCK_DISTANCE_THRESHOLD {
            stuck.stuck_frames += 1;
        } else {
            stuck.stuck_frames = 0;
        }
        stuck.last_sampled_pos = pos.translation;

        if stuck.stuck_frames >= 3 {
            let nudge_dir = Vec3::new(
                (entity.index() as f32 * 0.31 + 0.7).sin(),
                0.0,
                (entity.index() as f32 * 0.71 + 1.3).cos(),
            )
            .normalize_or_zero();
            let nudge_target = pos.translation + nudge_dir * NUDGE_DISTANCE;
            move_events.send(MovementTargetEvent {
                entity,
                target_position: nudge_target,
            });
            stuck.stuck_frames = 0;
        }
    }
}
