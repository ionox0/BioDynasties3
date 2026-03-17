use crate::core::components::*;
use bevy::prelude::*;

/// Marker for the green sphere shown while a gatherer is returning to base.
// Owned by: CargoIndicatorPlugin
#[derive(Component)]
struct CargoIndicator {
    target: Entity,
}

pub struct CargoIndicatorPlugin;

impl Plugin for CargoIndicatorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_cargo_indicators, despawn_cargo_indicators).chain());
    }
}

fn spawn_cargo_indicators(
    mut commands: Commands,
    returners: Query<(Entity, &UnitState), (With<ResourceGatherer>, Without<Dying>)>,
    existing: Query<&CargoIndicator>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, state) in returners.iter() {
        if *state != UnitState::ReturningToBase {
            continue;
        }
        let has_indicator = existing.iter().any(|ind| ind.target == entity);
        if has_indicator {
            continue;
        }
        let sphere = commands
            .spawn((
                Mesh3d(meshes.add(Sphere::new(0.67))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.22, 0.38, 0.08),
                    emissive: LinearRgba::new(0.05, 0.10, 0.02, 1.0),
                    metallic: 0.1,
                    perceptual_roughness: 0.8,
                    reflectance: 0.2,
                    ..default()
                })),
                Transform::from_translation(Vec3::new(0.0, 2.5, -1.0)),
                CargoIndicator { target: entity },
            ))
            .id();
        commands.entity(entity).add_child(sphere);
    }
}

fn despawn_cargo_indicators(
    mut commands: Commands,
    indicators: Query<(Entity, &CargoIndicator)>,
    units: Query<&UnitState>,
) {
    for (indicator_entity, indicator) in indicators.iter() {
        let should_despawn = units
            .get(indicator.target)
            .map_or(true, |state| *state != UnitState::ReturningToBase);
        if should_despawn {
            commands
                .entity(indicator.target)
                .remove_children(&[indicator_entity]);
            commands.entity(indicator_entity).despawn();
        }
    }
}
