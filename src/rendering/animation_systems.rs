use bevy::ecs::system::SystemParam;
use crate::core::components::*;
use bevy::prelude::*;
use bevy_animation::graph::AnimationNodeIndex;
use bevy_animation::prelude::{AnimationGraph, AnimationGraphHandle};

type UnitsWithoutControllerQuery<'w, 's> = Query<'w, 's, (Entity, &'static RTSUnit), (Without<UnitAnimationController>, With<RTSUnit>)>;

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                add_missing_animation_controllers,
                setup_glb_animations,
                animation_state_manager,
                update_animations,
                find_animation_players,
                start_idle_animations,
            )
                .chain(),
        )
        .add_event::<AnimationStateChangeEvent>();
    }
}

#[derive(Component, Debug, Clone)]
pub struct UnitAnimationController {
    pub current_state: AnimationState,
    pub previous_state: AnimationState,
    pub animation_player: Option<Entity>,
    pub animation_node_index: Option<AnimationNodeIndex>,
    /// Stores named animation clips mapped to their node indices
    pub animation_clips: std::collections::HashMap<String, AnimationNodeIndex>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnimationState {
    Idle,
    Walking,
    Running,
    Attacking,
    Death,
    Special, // For unit-specific animations like flying, eating, etc.
}

#[derive(Event, Debug)]
pub struct AnimationStateChangeEvent {
    pub entity: Entity,
    pub new_state: AnimationState,
    #[allow(dead_code)]
    pub force: bool, // Force immediate transition without blending
}

/// Grouped read-only queries for animation state derivation.
#[derive(SystemParam)]
pub(crate) struct AnimStateParams<'w, 's> {
    gathering: Query<'w, 's, &'static GatheringState>,
    combat_state: Query<'w, 's, &'static CombatState>,
    health: Query<'w, 's, &'static RTSHealth>,
}

/// Derives animation state from specialized state components each frame.
/// Priority: Death > InCombat > Gathering > Velocity-based.
pub fn animation_state_manager(
    units: Query<(Entity, &UnitAnimationController, &Movement, &RTSUnit)>,
    state: AnimStateParams,
    mut events: EventWriter<AnimationStateChangeEvent>,
) {
    for (entity, controller, movement, rts_unit) in units.iter() {
        let new_state = derive_animation_state(entity, movement, rts_unit, &state);
        if controller.current_state != new_state {
            let force = matches!(new_state, AnimationState::Death);
            events.send(AnimationStateChangeEvent { entity, new_state, force });
        }
    }
}

fn derive_animation_state(
    entity: Entity,
    movement: &Movement,
    rts_unit: &RTSUnit,
    state: &AnimStateParams,
) -> AnimationState {
    if state.health.get(entity).is_ok_and(|h| h.current <= 0.0) {
        return AnimationState::Death;
    }

    if state.combat_state.get(entity).is_ok_and(|c| c.state == CombatStateType::InCombat) {
        return AnimationState::Attacking;
    }

    if let Ok(gathering) = state.gathering.get(entity) {
        match gathering.state {
            GatheringStateType::Gathering => return AnimationState::Special,
            GatheringStateType::MovingToResource | GatheringStateType::ReturningToBase => {
                return AnimationState::Walking;
            }
            GatheringStateType::DeliveringResources | GatheringStateType::Idle => {
                return AnimationState::Idle;
            }
        }
    }

    let velocity = movement.current_velocity.length();
    if velocity > 0.1 {
        if rts_unit.unit_type.as_ref().is_some_and(is_flying_unit) {
            return AnimationState::Special;
        }
        if velocity > movement.max_speed * 0.7 {
            AnimationState::Running
        } else {
            AnimationState::Walking
        }
    } else {
        AnimationState::Idle
    }
}

fn is_flying_unit(unit_type: &UnitType) -> bool {
    matches!(
        unit_type,
        UnitType::DragonFly
            | UnitType::Housefly
            | UnitType::Moths
            | UnitType::Hornets
            | UnitType::Honeybees
            | UnitType::Firefly
            | UnitType::DragonFlies
            | UnitType::ScoutAnt
            | UnitType::PeacockMoth
    )
}

/// Get the specific animation name for a unit type and animation state
fn get_animation_name(unit_type: &crate::core::components::UnitType, animation_state: &AnimationState) -> String {
    use crate::core::components::UnitType;
    match (unit_type, animation_state) {
        // Bees use bee_hover when moving (Special state for flying units)
        (UnitType::Honeybees, AnimationState::Special) => "bee_hover".to_string(),
        // Default to Animation0 for most cases
        _ => "Animation0".to_string(),
    }
}

/// Get list of animations to try loading for a specific unit type
fn get_animations_for_unit(unit_type: &Option<crate::core::components::UnitType>) -> Vec<String> {
    use crate::core::components::UnitType;

    if let Some(unit_type) = unit_type {
        match unit_type {
            UnitType::Honeybees => vec![
                "bee_hover".to_string(),  // Specific hover animation for bees
                "Animation0".to_string(), // Fallback default animation
            ],
            _ => vec![
                "Animation0".to_string(), // Default animation for most units
            ],
        }
    } else {
        vec!["Animation0".to_string()]
    }
}

// System to handle animation updates
pub fn update_animations(
    mut animation_events: EventReader<AnimationStateChangeEvent>,
    mut controllers: Query<(&mut UnitAnimationController, &RTSUnit)>,
    mut animation_players: Query<&mut AnimationPlayer>,
) {
    for event in animation_events.read() {
        if let Ok((mut controller, rts_unit)) = controllers.get_mut(event.entity) {
            // Update animation state
            controller.previous_state = controller.current_state.clone();
            controller.current_state = event.new_state.clone();

            // Try to play the animation if we have a player
            if let Some(player_entity) = controller.animation_player {
                if let Ok(mut player) = animation_players.get_mut(player_entity) {
                    // Try to use specific animation for this unit type and state
                    let mut node_index = controller.animation_node_index;

                    if let Some(unit_type) = &rts_unit.unit_type {
                        let animation_name = get_animation_name(unit_type, &event.new_state);

                        // Try to find the specific animation first
                        if let Some(specific_index) = controller.animation_clips.get(&animation_name) {
                            node_index = Some(*specific_index);
                            debug!("Using specific animation '{}' for {:?} in state {:?}",
                                   animation_name, unit_type, event.new_state);
                        }
                    }

                    if let Some(index) = node_index {
                        player.play(index).repeat();
                    } else {
                        debug!(
                            "No animation node index stored for entity {:?}",
                            event.entity
                        );
                    }
                } else {
                    warn!(
                        "AnimationPlayer entity {:?} not found for controller on entity {:?}",
                        player_entity, event.entity
                    );
                }
            } else {
                debug!(
                    "No AnimationPlayer assigned to controller on entity {:?}",
                    event.entity
                );
            }
        }
    }
}

// System to find animation players for controllers
// This waits for GLB scene instantiation to complete before searching
pub fn find_animation_players(
    mut controllers: Query<(Entity, &mut UnitAnimationController), Without<AnimationPlayer>>,
    animation_players: Query<Entity, With<AnimationPlayer>>,
    children: Query<&Children>,
    parents: Query<&Parent>,
    scene_roots: Query<&SceneRoot>,
) {
    for (controller_entity, mut controller) in controllers.iter_mut() {
        let None = controller.animation_player else { continue; };
        // Check if this entity has a SceneRoot (GLB model)
        if let Ok(_scene_root) = scene_roots.get(controller_entity) {
            // For GLB scenes, wait until the entity has children before searching
            // This indicates the scene has been instantiated
            if children.get(controller_entity).is_err() {
                // No children yet, scene still loading
                continue;
            }

            // Scene is ready (has children), now search for animation players
            if let Some(player) =
                search_recursive_for_player(controller_entity, &children, &animation_players, 0)
            {
                controller.animation_player = Some(player);
            }
        } else {
            // Non-GLB entity, use simpler search
            if let Some(player) = search_simple_for_player(
                controller_entity,
                &children,
                &parents,
                &animation_players,
            ) {
                controller.animation_player = Some(player);
            }
        }
    }
}

// Recursive search for animation players in GLB scene hierarchies
fn search_recursive_for_player(
    entity: Entity,
    children: &Query<&Children>,
    animation_players: &Query<Entity, With<AnimationPlayer>>,
    depth: usize,
) -> Option<Entity> {
    if depth > 8 {
        return None;
    } // Prevent infinite recursion, deeper limit for GLB scenes

    // Check if this entity is an animation player
    if animation_players.get(entity).is_ok() {
        return Some(entity);
    }

    // Search children
    if let Ok(children_list) = children.get(entity) {
        for &child in children_list.iter() {
            if let Some(player) =
                search_recursive_for_player(child, children, animation_players, depth + 1)
            {
                return Some(player);
            }
        }
    }

    None
}

// Simple search for animation players in non-GLB entities
fn search_simple_for_player(
    entity: Entity,
    children: &Query<&Children>,
    parents: &Query<&Parent>,
    animation_players: &Query<Entity, With<AnimationPlayer>>,
) -> Option<Entity> {
    // Check direct children
    if let Ok(children_list) = children.get(entity) {
        for &child in children_list.iter() {
            if animation_players.get(child).is_ok() {
                return Some(child);
            }
        }
    }

    // Check siblings
    if let Ok(parent) = parents.get(entity) {
        if let Ok(siblings) = children.get(parent.get()) {
            for &sibling in siblings.iter() {
                if animation_players.get(sibling).is_ok() {
                    return Some(sibling);
                }
            }
        }
    }

    None
}

// System to start idle animations for units that just got their animation player assigned
pub fn start_idle_animations(
    mut controllers: Query<
        (Entity, &mut UnitAnimationController),
        Changed<UnitAnimationController>,
    >,
    mut animation_players: Query<&mut AnimationPlayer>,
) {
    for (_entity, controller) in controllers.iter_mut() {
        // If we just got an animation player assigned, start the idle animation
        if let Some(player_entity) = controller.animation_player {
            if let Ok(mut player) = animation_players.get_mut(player_entity) {
                // Use the stored node index if available, otherwise fall back to index 0
                let node_index = controller
                    .animation_node_index
                    .unwrap_or(AnimationNodeIndex::new(0));
                player.play(node_index).repeat();
            }
        }
    }
}

// System to retroactively add animation controllers to units that don't have them
pub fn add_missing_animation_controllers(
    mut commands: Commands,
    units_without_controllers: UnitsWithoutControllerQuery,
) {
    for (entity, unit) in units_without_controllers.iter() {
        // Only add animation controller to units with a specific type
        let Some(_unit_type) = &unit.unit_type else {
            continue;
        };

        let animation_controller = UnitAnimationController {
            current_state: AnimationState::Idle,
            previous_state: AnimationState::Idle,
            animation_player: None, // Will be populated by find_animation_players system
            animation_node_index: None, // Will be populated by setup_glb_animations system
            animation_clips: std::collections::HashMap::new(), // Will be populated by setup_glb_animations system
        };

        // Check if entity still exists before trying to add components
        if let Some(mut entity_commands) = commands.get_entity(entity) {
            // Add the animation controller to the entity
            entity_commands.insert(animation_controller);
        }
    }
}

// System to set up animations for GLB models
// In Bevy 0.15, GLB animations are loaded automatically, but AnimationPlayer might be on child entities
pub fn setup_glb_animations(
    mut glb_models: Query<
        (Entity, &SceneRoot, &mut UnitAnimationController, &RTSUnit),
        Without<AnimationPlayerSearched>,
    >,
    mut animation_players: Query<&mut AnimationPlayer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut commands: Commands,
    children: Query<&Children>,
    asset_server: Res<AssetServer>,
) {
    for (entity, scene_root, mut controller, rts_unit) in glb_models.iter_mut() {
        // Check if scene has children (indicating it's loaded)
        if children.get(entity).is_err() {
            continue;
        }

        // Check if entity still exists before trying to add components
        let Some(mut entity_commands) = commands.get_entity(entity) else {
            // Entity has been despawned, skip processing
            continue;
        };
        // Mark this entity as searched so we don't search again
        entity_commands.insert(AnimationPlayerSearched);

        // Search for AnimationPlayer in the entity hierarchy
        if let Some(player_entity) =
            search_for_animation_player_entity(entity, &children, &animation_players)
        {
            // Store the AnimationPlayer entity in the controller
            controller.animation_player = Some(player_entity);

            // Load multiple animations based on unit type
            if let Some(scene_path) = asset_server.get_path(scene_root.0.id()) {
                let scene_path_str = scene_path.path().display().to_string();

                // Determine which animations to try loading based on unit type
                let animations_to_load = get_animations_for_unit(&rts_unit.unit_type);

                let mut graph = AnimationGraph::new();
                let mut loaded_any = false;

                for animation_name in &animations_to_load {
                    let animation_path = format!("{}#{}", scene_path_str, animation_name);
                    let animation_clip: Handle<bevy::animation::AnimationClip> =
                        asset_server.load(&animation_path);

                    // Try to add this animation to the graph
                    if let Ok(node_index) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        graph.add_clip(animation_clip.clone(), 1.0, graph.root)
                    })) {
                        controller.animation_clips.insert(animation_name.clone(), node_index);

                        // Set the first successful animation as default
                        if controller.animation_node_index.is_none() {
                            controller.animation_node_index = Some(node_index);
                        }

                        loaded_any = true;
                    }
                }

                if loaded_any {
                    let graph_handle = animation_graphs.add(graph);

                    // Insert the graph handle on the AnimationPlayer entity
                    if let Some(mut entity_commands) = commands.get_entity(player_entity) {
                        entity_commands.insert(AnimationGraphHandle(graph_handle));
                    }

                    // Start playing default animation immediately
                    if let Ok(mut player) = animation_players.get_mut(player_entity) {
                        if let Some(node_index) = controller.animation_node_index {
                            player.play(node_index).repeat();
                        }
                    }
                }
            }
        }
    }
}

// Mark component to track that we've searched for an animation player
#[derive(Component)]
pub(crate) struct AnimationPlayerSearched;

// Helper function to recursively search for AnimationPlayer entity
fn search_for_animation_player_entity(
    entity: Entity,
    children: &Query<&Children>,
    animation_players: &Query<&mut AnimationPlayer>,
) -> Option<Entity> {
    search_for_animation_player_recursive(entity, children, animation_players, 0)
}

fn search_for_animation_player_recursive(
    entity: Entity,
    children: &Query<&Children>,
    animation_players: &Query<&mut AnimationPlayer>,
    depth: usize,
) -> Option<Entity> {
    if depth > 10 {
        return None;
    } // Prevent infinite recursion

    // Check if this entity has AnimationPlayer
    if animation_players.get(entity).is_ok() {
        return Some(entity);
    }

    // Search children
    if let Ok(children_list) = children.get(entity) {
        for &child in children_list.iter() {
            if let Some(player) =
                search_for_animation_player_recursive(child, children, animation_players, depth + 1)
            {
                return Some(player);
            }
        }
    }

    None
}
