//! Health bar UI.
//!
//! Owns `HealthBarUI` and `HealthStatus` components.
//! Uses change-detection on `RTSHealth` to update bar width efficiently.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::core::components::{Dying, RTSHealth, RTSUnit, Selectable};

#[derive(SystemParam)]
pub(crate) struct NewUnits<'w, 's> {
    query: Query<'w, 's, (Entity, &'static RTSHealth), (Added<RTSUnit>, Without<HealthBarUI>)>,
}

#[derive(SystemParam)]
pub(crate) struct HealthBarViews<'w, 's> {
    bars: Query<'w, 's, (&'static HealthBarUI, &'static mut HealthStatus, &'static mut Node, &'static Children)>,
    fills: Query<'w, 's, (&'static mut Node, &'static mut BackgroundColor), (With<HealthFill>, Without<HealthBarUI>)>,
}

#[derive(SystemParam)]
pub(crate) struct UnitPresence<'w, 's> {
    dying: Query<'w, 's, Entity, With<Dying>>,
    all: Query<'w, 's, Entity, With<RTSUnit>>,
}

#[derive(SystemParam)]
pub(crate) struct ChangedHealth<'w, 's> {
    query: Query<'w, 's, &'static RTSHealth, Changed<RTSHealth>>,
}

#[derive(SystemParam)]
pub(crate) struct UnitTransforms<'w, 's> {
    query: Query<'w, 's, &'static Transform, With<RTSUnit>>,
}

/// Marker for the health bar root node tied to a specific entity.
// Owned by: HealthUIPlugin (setup_health_bars, cleanup_health_bars)
#[derive(Component, Debug, Clone)]
pub struct HealthBarUI {
    pub tracked: Entity,
}

/// Current health status snapshot — updated by change detection.
// Owned by: HealthUIPlugin (update_health_bars)
#[derive(Component, Debug, Clone)]
pub struct HealthStatus {
    pub fraction: f32,
}

/// Marker for the inner fill bar child node.
#[derive(Component)]
pub struct HealthFill;

pub struct HealthUIPlugin;

impl Plugin for HealthUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_health_bars,
                cleanup_health_bars,
                update_health_bars,
            )
                .chain(),
        );
    }
}

/// Spawns a health-bar UI element for each newly added `RTSUnit` that has `RTSHealth`.
fn spawn_health_bars(mut commands: Commands, new_units: NewUnits) {
    for (entity, health) in new_units.query.iter() {
        let fraction = health.current / health.max;
        commands.spawn((
            HealthBarUI { tracked: entity },
            HealthStatus { fraction },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(40.0),
                height: Val::Px(5.0),
                left: Val::Px(-9999.0), // Hidden until positioned
                top: Val::Px(-9999.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
            ZIndex(500),
        ))
        .with_children(|parent| {
            parent.spawn((
                HealthFill,
                Node {
                    width: Val::Percent(fraction * 100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(health_color(fraction)),
            ));
        });
    }
}

/// Removes health-bar UI for entities that have been despawned or are dying.
fn cleanup_health_bars(
    mut commands: Commands,
    bars: Query<(Entity, &HealthBarUI)>,
    units: UnitPresence,
) {
    for (bar_entity, bar) in bars.iter() {
        let dead = units.dying.contains(bar.tracked) || units.all.get(bar.tracked).is_err();
        if dead {
            commands.entity(bar_entity).despawn_recursive();
        }
    }
}

/// Updates health bar width and colour whenever `RTSHealth` changes.
fn update_health_bars(
    mut bar_views: HealthBarViews,
    health: ChangedHealth,
    selected_q: Query<&Selectable>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    unit_transforms: UnitTransforms,
) {
    let Ok((camera, camera_tf)) = camera_q.get_single() else { return };

    for (bar, mut status, mut bar_node, children) in bar_views.bars.iter_mut() {
        let Ok(health_data) = health.query.get(bar.tracked) else { continue };
        let fraction = (health_data.current / health_data.max).clamp(0.0, 1.0);
        status.fraction = fraction;

        // Position bar above unit in screen space
        if let Ok(unit_tf) = unit_transforms.query.get(bar.tracked) {
            let above = unit_tf.translation + Vec3::Y * 12.0;
            if let Some(ndc) = camera.world_to_ndc(camera_tf, above) {
                let viewport = camera.logical_viewport_size().unwrap_or(Vec2::new(1280.0, 720.0));
                let screen_x = (ndc.x + 1.0) * 0.5 * viewport.x - 20.0;
                let screen_y = (1.0 - (ndc.y + 1.0) * 0.5) * viewport.y - 5.0;

                let is_selected = selected_q
                    .get(bar.tracked)
                    .is_ok_and(|s| s.is_selected);
                let show = is_selected || fraction < 1.0;

                if show && ndc.z > 0.0 && ndc.z < 1.0 {
                    bar_node.left = Val::Px(screen_x);
                    bar_node.top = Val::Px(screen_y);
                } else {
                    bar_node.left = Val::Px(-9999.0);
                }
            }
        }

        for &child in children.iter() {
            let Ok((mut fill_node, mut fill_color)) = bar_views.fills.get_mut(child) else { continue };
            fill_node.width = Val::Percent(fraction * 100.0);
            *fill_color = BackgroundColor(health_color(fraction));
        }
    }
}

fn health_color(fraction: f32) -> Color {
    if fraction > 0.6 {
        Color::srgb(0.1, 0.8, 0.1)
    } else if fraction > 0.3 {
        Color::srgb(0.9, 0.7, 0.0)
    } else {
        Color::srgb(0.9, 0.1, 0.1)
    }
}
