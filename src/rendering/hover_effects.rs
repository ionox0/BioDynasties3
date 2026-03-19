use crate::core::components::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[derive(SystemParam)]
pub(crate) struct HoverableEntities<'w, 's> {
    query: Query<'w, 's, (Entity, &'static Transform, &'static Selectable), Or<(With<RTSUnit>, With<Building>, With<ResourceSource>)>>,
}

#[derive(SystemParam)]
pub(crate) struct CurrentlyHoveredEntities<'w, 's> {
    query: Query<'w, 's, Entity, With<HoveredEntity>>,
}

#[derive(SystemParam)]
pub(crate) struct NewlyHoveredEntities<'w, 's> {
    query: Query<'w, 's, Entity, (With<HoveredEntity>, Without<HoverEffectApplied>)>,
}

#[derive(SystemParam)]
pub(crate) struct ExHoveredEntities<'w, 's> {
    query: Query<'w, 's, Entity, (With<HoverEffectApplied>, Without<HoveredEntity>)>,
}

/// Plugin for handling model hover effects
pub struct HoverEffectsPlugin;

impl Plugin for HoverEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                hover_detection_system,
                apply_hover_effects.after(hover_detection_system),
                remove_hover_effects.after(hover_detection_system),
            ),
        );
    }
}

/// Component to mark entities that are currently being hovered
#[derive(Component, Debug)]
pub struct HoveredEntity;

/// Component to track original material before hover effect
#[derive(Component, Debug)]
pub struct OriginalMaterial {
    pub handle: Handle<StandardMaterial>,
}

/// Component to mark entities with active hover effects
#[derive(Component, Debug)]
pub struct HoverEffectApplied;

/// System to detect which entities should be highlighted based on mouse position
pub fn hover_detection_system(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    hoverable_entities: HoverableEntities,
    currently_hovered: CurrentlyHoveredEntities,
    mut commands: Commands,
) {
    let window = windows.single();
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return; // No camera found, skip hover detection
    };

    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    // Find entity closest to the cursor within hover radius
    let mut closest_entity = None;
    let mut closest_distance = f32::INFINITY;

    for (entity, transform, selectable) in hoverable_entities.query.iter() {
        if let Some(projected_distance) = calculate_projected_distance(ray, transform.translation) {
            let distance_to_ray =
                calculate_distance_to_ray(ray, transform.translation, projected_distance);

            // Use each entity's individual selection radius (same as click selection)
            if distance_to_ray < selectable.selection_radius
                && projected_distance < closest_distance
            {
                closest_distance = projected_distance;
                closest_entity = Some(entity);
            }
        }
    }

    // Remove hover from all currently hovered entities (use try_remove to avoid panics)
    for entity in currently_hovered.query.iter() {
        commands.entity(entity).remove::<HoveredEntity>();
    }

    // Add hover to the closest entity if found (check if it still exists)
    let Some(entity) = closest_entity else {
        return;
    };
    // Use try_insert to avoid panics if entity was despawned between query and command application
    commands.entity(entity).try_insert(HoveredEntity);
}

/// System to apply green hover effects to newly hovered entities
pub fn apply_hover_effects(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    newly_hovered: NewlyHoveredEntities,
    material_query: Query<&MeshMaterial3d<StandardMaterial>>,
    children_query: Query<&Children>,
) {
    for entity in newly_hovered.query.iter() {
        // Find all materials in this entity's hierarchy
        let mut material_updates = Vec::new();
        collect_material_updates_for_hover(
            entity,
            &material_query,
            &children_query,
            &mut material_updates,
            0,
        );

        let material_count = material_updates.len();
        if material_updates.is_empty() {
            continue;
        }

        // Store original materials and create hover versions
        for (entity_to_update, original_handle) in material_updates {
            let Some(original_material) = materials.get(&original_handle) else {
                continue;
            };

            let mut hover_material = original_material.clone();

            // Apply green tint (mix with existing color)
            let original_linear = LinearRgba::from(hover_material.base_color);
            let green_tint = LinearRgba::rgb(0.0, 1.0, 0.0);
            hover_material.base_color = Color::from(LinearRgba::rgb(
                original_linear.red * 0.7 + green_tint.red * 0.3,
                original_linear.green * 0.7 + green_tint.green * 0.3,
                original_linear.blue * 0.7 + green_tint.blue * 0.3,
            ));
            hover_material.emissive = LinearRgba::rgb(0.0, 0.2, 0.0);

            let hover_handle = materials.add(hover_material);
            commands.entity(entity_to_update)
                .try_insert(OriginalMaterial { handle: original_handle.clone() })
                .try_insert(MeshMaterial3d(hover_handle));
        }

        commands.entity(entity).try_insert(HoverEffectApplied);

        debug!(
            "Applied hover effect to entity {:?} with {} materials",
            entity, material_count
        );
    }
}

/// System to remove hover effects from entities no longer hovered
pub fn remove_hover_effects(
    mut commands: Commands,
    no_longer_hovered: ExHoveredEntities,
    original_materials: Query<&OriginalMaterial>,
    children_query: Query<&Children>,
) {
    for entity in no_longer_hovered.query.iter() {
        // Restore original materials in entity hierarchy
        restore_original_materials_recursive(
            entity,
            &original_materials,
            &children_query,
            &mut commands,
            0,
        );

        // Remove hover components (check if entity still exists)
        let Some(mut entity_commands) = commands.get_entity(entity) else {
            continue;
        };
        entity_commands.remove::<HoverEffectApplied>();
    }
}

/// Helper function to collect materials that need hover effects
fn collect_material_updates_for_hover(
    entity: Entity,
    material_query: &Query<&MeshMaterial3d<StandardMaterial>>,
    children_query: &Query<&Children>,
    material_updates: &mut Vec<(Entity, Handle<StandardMaterial>)>,
    depth: usize,
) {
    if depth > 10 {
        return;
    } // Prevent infinite recursion

    // Check if this entity has a material
    if let Ok(material) = material_query.get(entity) {
        material_updates.push((entity, material.0.clone()));
    }

    // Check children
    if let Ok(children) = children_query.get(entity) {
        for &child in children.iter() {
            collect_material_updates_for_hover(
                child,
                material_query,
                children_query,
                material_updates,
                depth + 1,
            );
        }
    }
}

/// Helper function to restore original materials
fn restore_original_materials_recursive(
    entity: Entity,
    original_materials: &Query<&OriginalMaterial>,
    children_query: &Query<&Children>,
    commands: &mut Commands,
    depth: usize,
) {
    if depth > 10 {
        return;
    } // Prevent infinite recursion

    // Restore material for this entity if it has one
    if let Ok(original) = original_materials.get(entity) {
        let Some(mut entity_commands) = commands.get_entity(entity) else {
            return;
        };
        entity_commands
            .insert(MeshMaterial3d(original.handle.clone()))
            .remove::<OriginalMaterial>();
    }

    // Check children
    if let Ok(children) = children_query.get(entity) {
        for &child in children.iter() {
            restore_original_materials_recursive(
                child,
                original_materials,
                children_query,
                commands,
                depth + 1,
            );
        }
    }
}

/// Helper function to calculate projected distance (reused from unit_commands)
fn calculate_projected_distance(ray: Ray3d, target_position: Vec3) -> Option<f32> {
    let to_target = target_position - ray.origin;
    let projected_distance = to_target.dot(ray.direction.normalize());

    if projected_distance > 0.0 {
        Some(projected_distance)
    } else {
        None
    }
}

/// Helper function to calculate distance to ray (reused from unit_commands)
fn calculate_distance_to_ray(ray: Ray3d, target_position: Vec3, projected_distance: f32) -> f32 {
    let closest_point = ray.origin + ray.direction.normalize() * projected_distance;
    closest_point.distance(target_position)
}
