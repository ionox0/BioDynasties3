use bevy::ecs::system::SystemParam;
use crate::core::components::*;
use crate::core::unit_stats::get_unit_stats;
use bevy::prelude::*;
use bevy_animation::graph::AnimationNodeIndex;
use bevy_animation::prelude::{AnimationGraph, AnimationGraphHandle};

/// Units that haven't been given an animation controller yet.
type Uncontrolled = (Without<UnitAnimationController>, With<RTSUnit>);

#[derive(SystemParam)]
pub(crate) struct UncontrolledUnits<'w, 's> {
    query: Query<'w, 's, (Entity, &'static RTSUnit), Uncontrolled>,
}

#[derive(SystemParam)]
pub(crate) struct AnimationPlayerAssignment<'w, 's> {
    controllers: Query<'w, 's, (Entity, &'static mut UnitAnimationController), Without<AnimationPlayer>>,
    players: Query<'w, 's, Entity, With<AnimationPlayer>>,
}

#[derive(SystemParam)]
pub(crate) struct PendingGlbModels<'w, 's> {
    query: Query<'w, 's, (Entity, &'static SceneRoot, &'static mut UnitAnimationController, &'static RTSUnit), Without<AnimationPlayerSearched>>,
}


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
}

/// Grouped read-only queries for animation state derivation.
#[derive(SystemParam)]
pub(crate) struct AnimStateParams<'w, 's> {
    unit_state: Query<'w, 's, &'static UnitState>,
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
            events.send(AnimationStateChangeEvent { entity, new_state });
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

    if let Ok(us) = state.unit_state.get(entity) {
        match us {
            UnitState::InCombat => return AnimationState::Attacking,
            UnitState::Gathering => return AnimationState::Special,
            UnitState::MovingToResource | UnitState::ReturningToBase => {
                return AnimationState::Walking;
            }
            UnitState::DeliveringResources => return AnimationState::Idle,
            _ => {}
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
        UnitType::Dragonfly | UnitType::CairnsBirdwing | UnitType::Bee
    )
}

fn play_with_speed(player: &mut AnimationPlayer, index: AnimationNodeIndex, unit_type: Option<&UnitType>) {
    let speed = unit_type.map_or(1.0, |ut| get_unit_stats(ut).animation_speed);
    if !player.is_playing_animation(index) {
        player.stop_all();
    }
    player.play(index).repeat().set_speed(speed).resume();
}

/// glTF clip index for each animation role in a unit's model (`None` = model has no such clip).
struct ClipSet {
    walk: usize,
    idle: Option<usize>,
    attack: Option<usize>,
    death: Option<usize>,
}

fn clip_set(unit_type: &UnitType) -> ClipSet {
    let set = |walk, idle, attack, death| ClipSet { walk, idle, attack, death };
    match unit_type {
        // Meshy models with generated clips: 0 Walk, 1 Idle, 2 Attack, 3 Death.
        UnitType::Fourmi | UnitType::EmperorScorpion | UnitType::MajestyBeetle => set(0, Some(1), Some(2), Some(3)),
        UnitType::Bee => set(1, Some(0), None, None), // 0 idle, 1 hover
        UnitType::Scorpion => set(4, Some(3), Some(0), Some(1)), // Area Attack, Death, Defend, Idle, Walk
        UnitType::RhinoBeetle => set(3, Some(2), Some(0), Some(1)), // attack, death, idle, walk
        // Single-clip flyers keep flapping while idle.
        UnitType::Dragonfly | UnitType::CairnsBirdwing => set(0, Some(0), None, None),
        UnitType::RolyPoly | UnitType::GoliathBirdeater => set(0, None, None, None),
    }
}

/// Clip to play for a unit type in an animation state.
fn get_animation_name(unit_type: &UnitType, animation_state: &AnimationState) -> String {
    let clips = clip_set(unit_type);
    let index = match animation_state {
        AnimationState::Idle => clips.idle.unwrap_or(clips.walk),
        // Special = flying for flyers, gathering for ground units.
        AnimationState::Special if !is_flying_unit(unit_type) => clips.idle.unwrap_or(clips.walk),
        AnimationState::Attacking => clips.attack.unwrap_or(clips.walk),
        AnimationState::Death => clips.death.unwrap_or(clips.walk),
        AnimationState::Walking | AnimationState::Running | AnimationState::Special => clips.walk,
    };
    format!("Animation{index}")
}

/// Units whose model has a dedicated idle clip (others pause their walk clip when idle).
fn has_idle_clip(unit_type: Option<&UnitType>) -> bool {
    unit_type.is_some_and(|ut| clip_set(ut).idle.is_some())
}

/// Every clip a unit type uses, walk first (the first loaded clip is the default).
fn get_animations_for_unit(unit_type: &Option<UnitType>) -> Vec<String> {
    let Some(unit_type) = unit_type else { return vec!["Animation0".to_string()] };
    let clips = clip_set(unit_type);
    let mut indices = vec![clips.walk];
    for index in [clips.idle, clips.attack, clips.death].into_iter().flatten() {
        if !indices.contains(&index) {
            indices.push(index);
        }
    }
    indices.into_iter().map(|i| format!("Animation{i}")).collect()
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

                    if event.new_state == AnimationState::Idle && !has_idle_clip(rts_unit.unit_type.as_ref()) {
                        player.pause_all();
                    } else if let Some(index) = node_index {
                        play_with_speed(&mut player, index, rts_unit.unit_type.as_ref());
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
    mut assignment: AnimationPlayerAssignment,
    children: Query<&Children>,
    parents: Query<&Parent>,
    scene_roots: Query<&SceneRoot>,
) {
    for (controller_entity, mut controller) in assignment.controllers.iter_mut() {
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
                search_recursive_for_player(controller_entity, &children, &assignment.players, 0)
            {
                controller.animation_player = Some(player);
            }
        } else {
            // Non-GLB entity, use simpler search
            if let Some(player) = search_simple_for_player(
                controller_entity,
                &children,
                &parents,
                &assignment.players,
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


// System to retroactively add animation controllers to units that don't have them
pub fn add_missing_animation_controllers(
    mut commands: Commands,
    units: UncontrolledUnits,
) {
    for (entity, unit) in units.query.iter() {
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

/// Scene-hierarchy lookups used to find and drive a model's `AnimationPlayer`.
#[derive(SystemParam)]
pub(crate) struct SceneAnimationPlayers<'w, 's> {
    players: Query<'w, 's, &'static mut AnimationPlayer>,
    children: Query<'w, 's, &'static Children>,
}

/// Finds each loaded model's `AnimationPlayer` (often on a child entity), builds its clip graph,
/// and starts the default clip.
pub fn setup_glb_animations(
    mut glb_models: PendingGlbModels,
    mut scene: SceneAnimationPlayers,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for (entity, scene_root, mut controller, rts_unit) in glb_models.query.iter_mut() {
        if scene.children.get(entity).is_err() {
            continue; // Scene not spawned yet.
        }
        let Some(mut entity_commands) = commands.get_entity(entity) else { continue };
        entity_commands.insert(AnimationPlayerSearched);
        let Some(player_entity) = search_for_animation_player_entity(entity, &scene.children, &scene.players) else {
            continue;
        };
        controller.animation_player = Some(player_entity);
        let Some(scene_path) = asset_server.get_path(scene_root.0.id()) else { continue };
        let scene_path = scene_path.path().display().to_string();
        let Some(graph) = build_clip_graph(&scene_path, &rts_unit.unit_type, &mut controller, &asset_server) else {
            continue;
        };
        if let Some(mut player_commands) = commands.get_entity(player_entity) {
            player_commands.insert(AnimationGraphHandle(animation_graphs.add(graph)));
        }
        if let (Ok(mut player), Some(node)) = (scene.players.get_mut(player_entity), controller.animation_node_index) {
            play_with_speed(&mut player, node, rts_unit.unit_type.as_ref());
        }
    }
}

/// Graph of every clip this unit type uses, recording each clip's node in `controller`
/// (the first becomes the default). `None` if the unit type uses no clips.
fn build_clip_graph(
    scene_path: &str,
    unit_type: &Option<UnitType>,
    controller: &mut UnitAnimationController,
    asset_server: &AssetServer,
) -> Option<AnimationGraph> {
    let mut graph = AnimationGraph::new();
    for name in get_animations_for_unit(unit_type) {
        let clip = asset_server.load(format!("{scene_path}#{name}"));
        let node = graph.add_clip(clip, 1.0, graph.root);
        controller.animation_node_index.get_or_insert(node);
        controller.animation_clips.insert(name, node);
    }
    (!controller.animation_clips.is_empty()).then_some(graph)
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
