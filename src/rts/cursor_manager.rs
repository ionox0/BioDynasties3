//! Cursor manager — ray-cast hit-tests each frame and maintains `CursorState`.
//!
//! Owns `CursorState` resource. Other systems read it to determine what's under
//! the cursor without duplicating the ray-cast logic.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::core::components::{Building, ResourceSource, RTSUnit, Selectable};

type SelectableData = (Entity, &'static Transform, &'static Selectable);
/// Mobile units (buildings are also `RTSUnit`s).
type MobileUnit = (With<RTSUnit>, Without<Building>);
/// Hit radius for resource nodes, which have no `Selectable`.
const RESOURCE_HIT_RADIUS: f32 = 8.0;

/// Everything the cursor can hover, in priority order: units, then buildings, then resources.
#[derive(SystemParam)]
pub(crate) struct CursorTargets<'w, 's> {
    units: Query<'w, 's, SelectableData, MobileUnit>,
    buildings: Query<'w, 's, SelectableData, With<Building>>,
    resources: Query<'w, 's, (Entity, &'static Transform), With<ResourceSource>>,
}

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
    targets: CursorTargets,
) {
    let Ok(window) = windows.get_single() else { return };
    let Ok((camera, camera_tf)) = camera_q.get_single() else { return };
    let Some(ray) = window.cursor_position().and_then(|pos| camera.viewport_to_world(camera_tf, pos).ok()) else {
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

    let selectable_hit = |(entity, tf, sel): (Entity, &Transform, &Selectable)| (entity, tf.translation, sel.selection_radius);
    cursor_state.hovered = closest_hit(&ray, targets.units.iter().map(selectable_hit))
        .map(HoveredType::Unit)
        .or_else(|| closest_hit(&ray, targets.buildings.iter().map(selectable_hit)).map(HoveredType::Building))
        .or_else(|| {
            let resources = targets.resources.iter().map(|(e, tf)| (e, tf.translation, RESOURCE_HIT_RADIUS));
            closest_hit(&ray, resources).map(HoveredType::Resource)
        })
        .unwrap_or(HoveredType::Terrain(cursor_state.world_position));
}

/// Nearest entity along `ray` whose hit sphere (centre, radius) the ray passes through.
fn closest_hit(ray: &Ray3d, candidates: impl Iterator<Item = (Entity, Vec3, f32)>) -> Option<Entity> {
    candidates
        .filter_map(|(entity, center, radius)| Some((entity, ray_sphere_proj(ray, center, radius)?)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(entity, _)| entity)
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
