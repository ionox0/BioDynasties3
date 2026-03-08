//! Player unit command system.
//!
//! Translates right-click mouse input into movement and resource targeting events.
//! Does not mutate any component directly.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::core::components::*;
use crate::core::constants::resource_interaction::RESOURCE_CLICK_RADIUS;
use crate::core::constants::ui::*;
use super::events::MovementTargetEvent;

pub struct UnitCommandsPlugin;

impl Plugin for UnitCommandsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, right_click_command);
    }
}

#[derive(SystemParam)]
struct CommandTargets<'w, 's> {
    selected: Query<'w, 's, (Entity, &'static Selectable), With<RTSUnit>>,
    resources: Query<'w, 's, (Entity, &'static Transform), With<ResourceSource>>,
    move_events: EventWriter<'w, MovementTargetEvent>,
}

/// Issues move orders to all selected units on right-click.
fn right_click_command(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut targets: CommandTargets,
) {
    if !mouse_button.just_pressed(MouseButton::Right) {
        return;
    }

    let window = windows.single();
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    if !is_click_in_game_area(cursor_pos, window) {
        return;
    }

    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };
    let Some(ground_pos) = ray_ground_intersection(ray) else {
        return;
    };

    let destination = closest_resource_pos(&targets.resources, ground_pos)
        .unwrap_or(ground_pos);

    let selected: Vec<Entity> = targets
        .selected
        .iter()
        .filter(|(_, s)| s.is_selected)
        .map(|(e, _)| e)
        .collect();

    for entity in selected {
        targets.move_events.send(MovementTargetEvent { entity, target_position: destination });
    }
}

/// Returns the position of the nearest ResourceSource within click radius, if any.
fn closest_resource_pos(
    resources: &Query<(Entity, &Transform), With<ResourceSource>>,
    ground_pos: Vec3,
) -> Option<Vec3> {
    resources
        .iter()
        .filter_map(|(_, tf)| {
            let d = tf.translation.distance(ground_pos);
            (d < RESOURCE_CLICK_RADIUS).then_some((d, tf.translation))
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        .map(|(_, pos)| pos)
}

/// Intersects a ray with the y=0 ground plane. Returns None if ray points away from plane.
fn ray_ground_intersection(ray: Ray3d) -> Option<Vec3> {
    if ray.direction.y.abs() < 1e-6 {
        return None;
    }
    let t = -ray.origin.y / ray.direction.y;
    if t <= 0.0 {
        return None;
    }
    Some(ray.origin + *ray.direction * t)
}

fn is_click_in_game_area(cursor_pos: Vec2, window: &Window) -> bool {
    let w = window.width();
    let h = window.height();
    cursor_pos.x < (w - RIGHT_UI_WIDTH)
        && cursor_pos.y > TOP_UI_HEIGHT
        && cursor_pos.y < (h - BOTTOM_UI_HEIGHT)
}
