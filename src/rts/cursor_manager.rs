//! Cursor manager — ray-cast hit-tests each frame and maintains `CursorState`.
//!
//! Owns `CursorState` resource. Other systems read it to determine what's under
//! the cursor without duplicating the ray-cast logic.

use bevy::prelude::*;
use crate::core::components::{Building, ResourceSource, RTSUnit, Selectable};

type UnitQuery<'w, 's> = Query<'w, 's, (Entity, &'static Transform, &'static Selectable), (With<RTSUnit>, Without<Building>)>;
type BuildingQuery<'w, 's> = Query<'w, 's, (Entity, &'static Transform, &'static Selectable), With<Building>>;

/// What category of object is currently under the cursor.
#[derive(Debug, Clone, PartialEq)]
pub enum HoveredType {
    Unit(Entity),
    Building(Entity),
    Resource(Entity),
    Terrain(Vec3),
    None,
}

/// Current cursor state — world-space position and what's under it.
// Owned by: CursorManagerPlugin (update_cursor_state)
#[derive(Resource, Debug)]
pub struct CursorState {
    pub hovered: HoveredType,
    pub world_position: Vec3,
}

impl Default for CursorState {
    fn default() -> Self {
        Self { hovered: HoveredType::None, world_position: Vec3::ZERO }
    }
}

pub struct CursorManagerPlugin;

impl Plugin for CursorManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorState>()
            .add_systems(Update, update_cursor_state);
    }
}

fn update_cursor_state(
    mut cursor_state: ResMut<CursorState>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    units: UnitQuery,
    buildings: BuildingQuery,
    resources: Query<(Entity, &Transform), With<ResourceSource>>,
) {
    let Ok(window) = windows.get_single() else { return };
    let Ok((camera, camera_tf)) = camera_q.get_single() else { return };
    let Some(cursor_pos) = window.cursor_position() else {
        cursor_state.hovered = HoveredType::None;
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_tf, cursor_pos) else {
        cursor_state.hovered = HoveredType::None;
        return;
    };

    // Approximate ground plane intersection (y ≈ 0)
    let ground_t = if ray.direction.y.abs() > 0.001 {
        (-ray.origin.y / ray.direction.y).max(0.0)
    } else {
        1000.0
    };
    cursor_state.world_position = ray.origin + *ray.direction * ground_t;

    // Check units
    let mut best_unit: Option<(Entity, f32)> = None;
    for (entity, transform, sel) in units.iter() {
        if let Some(proj) = ray_sphere_proj(&ray, transform.translation, sel.selection_radius) {
            if best_unit.is_none_or(|(_, d)| proj < d) {
                best_unit = Some((entity, proj));
            }
        }
    }
    if let Some((entity, _)) = best_unit {
        cursor_state.hovered = HoveredType::Unit(entity);
        return;
    }

    // Check buildings
    let mut best_building: Option<(Entity, f32)> = None;
    for (entity, transform, sel) in buildings.iter() {
        if let Some(proj) = ray_sphere_proj(&ray, transform.translation, sel.selection_radius) {
            if best_building.is_none_or(|(_, d)| proj < d) {
                best_building = Some((entity, proj));
            }
        }
    }
    if let Some((entity, _)) = best_building {
        cursor_state.hovered = HoveredType::Building(entity);
        return;
    }

    // Check resources
    let mut best_resource: Option<(Entity, f32)> = None;
    for (entity, transform) in resources.iter() {
        if let Some(proj) = ray_sphere_proj(&ray, transform.translation, 8.0) {
            if best_resource.is_none_or(|(_, d)| proj < d) {
                best_resource = Some((entity, proj));
            }
        }
    }
    if let Some((entity, _)) = best_resource {
        cursor_state.hovered = HoveredType::Resource(entity);
        return;
    }

    cursor_state.hovered = HoveredType::Terrain(cursor_state.world_position);
}

/// Returns the projection distance along `ray` to the closest approach of the
/// sphere at `center` with `radius`, or `None` if the ray misses.
fn ray_sphere_proj(ray: &Ray3d, center: Vec3, radius: f32) -> Option<f32> {
    let to = center - ray.origin;
    let proj = to.dot(*ray.direction);
    if proj <= 0.0 {
        return None;
    }
    let closest = ray.origin + *ray.direction * proj;
    if closest.distance(center) < radius { Some(proj) } else { None }
}
