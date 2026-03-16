//! Player unit command system.
//!
//! Translates right-click mouse input into movement and resource targeting events.
//! Does not mutate any component directly.
//!
//! For AI units: send the same events (SetTargetResourceEvent + MovementTargetEvent)
//! from the AI goal system — the gathering loop is identical.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::core::components::*;
use crate::core::constants::resource_interaction::RESOURCE_CLICK_RADIUS;
use crate::core::constants::ui::*;
use crate::rts::combat::events::CombatStopEvent;
use crate::rts::resource::events::SetTargetResourceEvent;
use super::events::{MovementTargetEvent, UnitArrivedEvent};
use super::formation_events::FormationMoveEvent;

pub struct UnitCommandsPlugin;

impl Plugin for UnitCommandsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (right_click_command, clear_move_activity_on_arrival));
    }
}

#[derive(SystemParam)]
struct CommandTargets<'w, 's> {
    selectables: Query<'w, 's, (Entity, &'static Selectable, Option<&'static ResourceGatherer>, Option<&'static Combat>), With<RTSUnit>>,
    resources: Query<'w, 's, (Entity, &'static Transform, &'static ResourceSource)>,
    move_events: EventWriter<'w, MovementTargetEvent>,
    formation_events: EventWriter<'w, FormationMoveEvent>,
    set_target_events: EventWriter<'w, SetTargetResourceEvent>,
    combat_stop_events: EventWriter<'w, CombatStopEvent>,
    commands: Commands<'w, 's>,
}

/// Issues move orders (and optional gather orders) to all selected units on right-click.
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

    let resource_click = closest_resource(&targets.resources, ground_pos);
    let destination = resource_click
        .as_ref()
        .map(|(_, pos, _)| *pos)
        .unwrap_or(ground_pos);

    let selected: Vec<(Entity, bool, bool)> = targets.selectables.iter()
        .filter(|(_, sel, _, _)| sel.is_selected)
        .map(|(entity, _, gatherer, combat)| (entity, gatherer.is_some(), combat.is_some()))
        .collect();

    // Per-entity: state, resource target, combat stop.
    for &(entity, has_gatherer, has_combat) in &selected {
        if has_gatherer {
            if let Some((resource_entity, _, resource_type)) = &resource_click {
                targets.set_target_events.send(SetTargetResourceEvent {
                    gatherer: entity,
                    target_resource: *resource_entity,
                    resource_type: resource_type.clone(),
                });
            }
        }
        targets.commands.entity(entity).insert(UnitState::Moving);
        if has_combat {
            targets.combat_stop_events.send(CombatStopEvent { entity });
        }
    }

    // Movement: spread multiple units into a formation; send single unit directly.
    let entities: Vec<Entity> = selected.iter().map(|&(e, _, _)| e).collect();
    if entities.len() > 1 {
        targets.formation_events.send(FormationMoveEvent { units: entities, target: destination });
    } else if let Some(entity) = entities.into_iter().next() {
        targets.move_events.send(MovementTargetEvent { entity, target_position: destination });
    }
}

/// Returns the nearest ResourceSource within click radius, if any.
fn closest_resource(
    resources: &Query<(Entity, &Transform, &ResourceSource)>,
    ground_pos: Vec3,
) -> Option<(Entity, Vec3, ResourceType)> {
    resources
        .iter()
        .filter_map(|(entity, tf, source)| {
            let d = tf.translation.distance(ground_pos);
            (d < RESOURCE_CLICK_RADIUS).then_some((d, entity, tf.translation, source.resource_type.clone()))
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        .map(|(_, entity, pos, resource_type)| (entity, pos, resource_type))
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

/// Clears `UnitState::Moving` when a unit reaches its destination.
fn clear_move_activity_on_arrival(
    mut commands: Commands,
    mut arrived: EventReader<UnitArrivedEvent>,
    activity_q: Query<&UnitState>,
) {
    for ev in arrived.read() {
        if activity_q.get(ev.entity).is_ok_and(|a| *a == UnitState::Moving) {
            commands.entity(ev.entity).insert(UnitState::Idle);
        }
    }
}

fn is_click_in_game_area(cursor_pos: Vec2, window: &Window) -> bool {
    let w = window.width();
    let h = window.height();
    cursor_pos.x < (w - RIGHT_UI_WIDTH)
        && cursor_pos.y > TOP_UI_HEIGHT
        && cursor_pos.y < (h - BOTTOM_UI_HEIGHT)
}
