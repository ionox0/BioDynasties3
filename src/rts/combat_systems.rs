//! Combat state management.
//!
//! Owns `CombatState` component.
//! Derives it each frame from `Combat` and spatial data.

use crate::core::components::*;
use bevy::prelude::*;

pub struct CombatStatePlugin;

impl Plugin for CombatStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (add_combat_state_to_fighters, update_combat_states).chain(),
        );
    }
}

/// Inserts `CombatState` on newly spawned auto-attacking units.
fn add_combat_state_to_fighters(
    mut commands: Commands,
    new_fighters: Query<(Entity, &Combat), Added<Combat>>,
) {
    for (entity, combat) in new_fighters.iter() {
        if combat.auto_attack {
            commands.entity(entity).insert(CombatState::default());
        }
    }
}

/// Sole writer of `CombatState` — derives it each frame from `Combat` and transforms.
fn update_combat_states(
    mut query: Query<(&mut CombatState, &Combat, &Transform, Option<&Movement>)>,
    targets: Query<&Transform, (With<RTSHealth>, Without<Dying>)>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for (mut state, combat, transform, movement) in query.iter_mut() {
        let new_state = derive_combat_state(combat, transform, movement, &targets);
        if new_state != state.state {
            state.state = new_state;
            state.last_state_change = now;
        }
        update_target_refs(&mut state, combat, &targets);
    }
}

fn derive_combat_state(
    combat: &Combat,
    transform: &Transform,
    movement: Option<&Movement>,
    targets: &Query<&Transform, (With<RTSHealth>, Without<Dying>)>,
) -> CombatStateType {
    let Some(target) = combat.target else {
        return CombatStateType::Idle;
    };
    let Ok(target_tf) = targets.get(target) else {
        return CombatStateType::Idle;
    };

    let dist = transform.translation.distance(target_tf.translation);
    if dist <= combat.attack_range {
        return CombatStateType::InCombat;
    }

    let is_moving = movement.is_some_and(|m| m.target_position.is_some());
    if is_moving {
        CombatStateType::MovingToAttack
    } else {
        CombatStateType::MovingToCombat
    }
}

fn update_target_refs(
    state: &mut CombatState,
    combat: &Combat,
    targets: &Query<&Transform, (With<RTSHealth>, Without<Dying>)>,
) {
    match combat.target {
        Some(target) => {
            if let Ok(tf) = targets.get(target) {
                state.target_entity = Some(target);
                state.target_position = Some(tf.translation);
            } else {
                state.target_entity = None;
                state.target_position = None;
            }
        }
        None => {
            state.target_entity = None;
            state.target_position = None;
        }
    }
}
