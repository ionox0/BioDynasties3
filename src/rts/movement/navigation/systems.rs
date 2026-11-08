//! Bevy systems that build flow fields off the main thread and hand them to units.
//!
//! Ownership: `Route` (written by MovementPlugin through these helpers).

use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{block_on, AsyncComputeTaskPool, Task};
use std::sync::Arc;
use tracing::instrument;

use crate::core::components::{Movement, Route};
use super::flow_field::FlowField;
use super::nav_grid::NavGrid;

/// Coarse passability grid shared with background flow-field builds.
#[derive(Resource)]
pub struct NavGridResource(pub Arc<NavGrid>);

/// A unit waiting on a flow field, with the target it was ordered to.
pub struct RouteRequest {
    pub entity: Entity,
    pub start: Vec3,
    pub target: Vec3,
}

struct PendingField {
    task: Task<FlowField>,
    units: Vec<(Entity, Vec3)>,
}

/// Flow fields still being built.
// Owned by: MovementPlugin (apply_movement_targets, poll_flow_fields)
#[derive(Resource, Default)]
pub struct PendingFlowFields(Vec<PendingField>);

impl PendingFlowFields {
    /// Starts one background build toward `goal` shared by every unit in `requests`.
    pub fn spawn(&mut self, grid: &Arc<NavGrid>, goal: Vec3, requests: Vec<RouteRequest>) {
        let grid = Arc::clone(grid);
        let starts: Vec<Vec3> = requests.iter().map(|r| r.start).collect();
        let task = AsyncComputeTaskPool::get().spawn(async move { build_field(grid, goal, &starts) });
        let units = requests.into_iter().map(|r| (r.entity, r.target)).collect();
        self.0.push(PendingField { task, units });
    }
}

#[instrument(skip_all, fields(units = starts.len()))]
fn build_field(grid: Arc<NavGrid>, goal: Vec3, starts: &[Vec3]) -> FlowField {
    FlowField::build(grid, goal, starts)
}

/// Gives each finished flow field to the units still heading for the target it was built for.
pub fn poll_flow_fields(
    mut pending: ResMut<PendingFlowFields>,
    mut units: Query<(&Movement, &mut Route)>,
) {
    pending.0.retain_mut(|field| {
        let Some(built) = block_on(future::poll_once(&mut field.task)) else { return true };
        let built = Arc::new(built);
        for &(entity, target) in &field.units {
            let Ok((movement, mut route)) = units.get_mut(entity) else { continue };
            if movement.target_position == Some(target) && matches!(*route, Route::Pending) {
                *route = Route::Field(Arc::clone(&built));
            }
        }
        false
    });
}
