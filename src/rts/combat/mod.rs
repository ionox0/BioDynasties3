//! Combat systems.
//!
//! Owns: `Combat` (target, is_attacking, last_attack_time),
//!       `RTSHealth` (current, last_damage_time), `Dying` (on death).
//!
//! ## System order each Update frame
//!
//! ```text
//! combat_target_handler    (CombatTargetEvent → Combat.target)
//! combat_stop_handler      (CombatStopEvent → clear Combat)
//! target_management_system (validate + auto-acquire targets)
//! combat_execution_system  (attack + MovementTargetEvent / StopMovementEvent)
//! damage_resolution_system (DamageEvent → RTSHealth, DeathEvent)
//! health_regen_system
//! death_system             (DeathEvent → despawn, clear targets)
//! ```

pub mod events;

use bevy::prelude::*;
use crate::core::components::*;
use crate::core::spatial_grid::IncrementalSpatialGrid;
use crate::rts::movement::events::{MovementTargetEvent, StopMovementEvent};
use self::events::{CombatStopEvent, CombatTargetEvent, DamageEvent, DeathEvent};

type AliveQuery<'w, 's> = Query<'w, 's, (Entity, &'static Transform, &'static RTSUnit), (With<RTSHealth>, Without<Dying>)>;
type TargetQuery<'w, 's> = Query<'w, 's, (&'static Transform, &'static CollisionRadius), (With<RTSHealth>, Without<Dying>)>;

// ─── Constants ───────────────────────────────────────────────────────────────

mod cc {
    pub const REGEN_DELAY: f32 = 5.0;
    pub const ATTACK_RANGE_MARGIN: f32 = 0.8;
    pub const TARGET_POSITION_RATIO: f32 = 0.7;
    pub const GRID_CELL_SIZE: f32 = 100.0;
    pub const VISION_RANGE: f32 = 120.0;
}

// ─── Spatial target grid ─────────────────────────────────────────────────────

/// Spatial grid mapping Entity → (world position, player_id) for fast target queries.
pub type CombatTargetGrid = IncrementalSpatialGrid<Entity, (Vec3, u8)>;

#[derive(Resource)]
pub struct TargetGrid(pub CombatTargetGrid);

impl Default for TargetGrid {
    fn default() -> Self {
        Self(CombatTargetGrid::new(cc::GRID_CELL_SIZE))
    }
}

impl TargetGrid {
    fn query_nearby(&self, position: Vec3, range: f32) -> Vec<(Entity, Vec3, u8)> {
        self.0
            .query_nearby_with_keys(position)
            .into_iter()
            .filter_map(|(entity, (entity_pos, player_id))| {
                if position.distance(*entity_pos) <= range {
                    Some((entity, *entity_pos, *player_id))
                } else {
                    None
                }
            })
            .collect()
    }
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TargetGrid>()
            .add_event::<DamageEvent>()
            .add_event::<DeathEvent>()
            .add_event::<CombatTargetEvent>()
            .add_event::<CombatStopEvent>()
            .add_systems(
                Update,
                (
                    combat_target_handler,
                    combat_stop_handler,
                    target_management_system,
                    combat_execution_system,
                    damage_resolution_system,
                    health_regen_system,
                    death_system,
                )
                    .chain(),
            );
    }
}

// ─── Systems ─────────────────────────────────────────────────────────────────

/// Handles `CombatTargetEvent` — assigns `Combat.target` and optionally triggers movement.
fn combat_target_handler(
    mut target_events: EventReader<CombatTargetEvent>,
    mut move_events: EventWriter<MovementTargetEvent>,
    mut combat_q: Query<&mut Combat>,
    transform_q: Query<&Transform>,
) {
    for ev in target_events.read() {
        let Ok(mut combat) = combat_q.get_mut(ev.attacker) else { continue };
        combat.target = Some(ev.target);
        combat.auto_attack = true;
        if ev.move_to_range {
            if let Ok(target_tf) = transform_q.get(ev.target) {
                move_events.send(MovementTargetEvent {
                    entity: ev.attacker,
                    target_position: target_tf.translation,
                });
            }
        }
    }
}

/// Handles `CombatStopEvent` — clears target, stops attacking, and halts movement.
fn combat_stop_handler(
    mut stop_events: EventReader<CombatStopEvent>,
    mut combat_q: Query<&mut Combat>,
    mut stop_move: EventWriter<StopMovementEvent>,
) {
    for ev in stop_events.read() {
        let Ok(mut combat) = combat_q.get_mut(ev.entity) else { continue };
        combat.target = None;
        combat.is_attacking = false;
        combat.auto_attack = false;
        stop_move.send(StopMovementEvent { entity: ev.entity });
    }
}

/// Validates existing targets and auto-acquires new ones from the spatial grid.
fn target_management_system(
    mut combat_q: Query<(Entity, &mut Combat, &Transform, &RTSUnit)>,
    alive_q: AliveQuery,
    mut target_grid: ResMut<TargetGrid>,
    mut removed: RemovedComponents<RTSUnit>,
) {
    for entity in removed.read() {
        target_grid.0.remove_item(entity);
    }
    for (entity, transform, unit) in alive_q.iter() {
        target_grid.0.update_item(entity, transform.translation, (transform.translation, unit.player_id));
    }

    for (_entity, mut combat, transform, unit) in combat_q.iter_mut() {
        if let Some(target) = combat.target {
            if alive_q.get(target).is_err() {
                combat.target = None;
            }
        }
        if combat.target.is_some() && !combat.auto_attack {
            continue;
        }
        let nearby = target_grid.query_nearby(transform.translation, cc::VISION_RANGE);
        let closest = nearby
            .into_iter()
            .filter(|(_, _, pid)| *pid != unit.player_id)
            .min_by(|(_, pa, _), (_, pb, _)| {
                transform.translation.distance(*pa)
                    .partial_cmp(&transform.translation.distance(*pb))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        combat.target = closest.map(|(e, _, _)| e);
    }
}

/// Moves units toward their target and fires `DamageEvent` when in range.
fn combat_execution_system(
    mut unit_q: Query<(
        Entity,
        &mut Combat,
        &Transform,
        &RTSUnit,
        &CollisionRadius,
    )>,
    target_q: TargetQuery,
    mut damage_events: EventWriter<DamageEvent>,
    mut move_events: EventWriter<MovementTargetEvent>,
    mut stop_events: EventWriter<StopMovementEvent>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for (attacker, mut combat, atf, unit, acol) in unit_q.iter_mut() {
        let Some(target) = combat.target else {
            combat.is_attacking = false;
            continue;
        };
        let Ok((ttf, tcol)) = target_q.get(target) else {
            combat.target = None;
            combat.is_attacking = false;
            continue;
        };

        let center_dist = atf.translation.distance(ttf.translation);
        let edge_dist = center_dist - acol.radius - tcol.radius;
        let effective_range = combat.attack_range * cc::ATTACK_RANGE_MARGIN;

        if edge_dist > effective_range {
            let dir = (ttf.translation - atf.translation).normalize();
            let dest = ttf.translation - dir * (combat.attack_range * cc::TARGET_POSITION_RATIO);
            let should_move = unit.player_id != 1 || edge_dist < combat.attack_range * 2.0;
            if should_move {
                move_events.send(MovementTargetEvent { entity: attacker, target_position: dest });
            }
            combat.is_attacking = false;
        } else {
            if edge_dist <= combat.attack_range {
                stop_events.send(StopMovementEvent { entity: attacker });
            }
            if edge_dist <= combat.attack_range && now - combat.last_attack_time >= combat.attack_cooldown {
                combat.last_attack_time = now;
                combat.is_attacking = true;
                let damage_type = match combat.attack_type {
                    AttackType::Melee => DamageType::Physical,
                    AttackType::Siege => DamageType::Siege,
                };
                damage_events.send(DamageEvent {
                    attacker,
                    target,
                    damage: combat.attack_damage,
                    damage_type,
                });
            }
        }
    }
}

/// Applies `DamageEvent` to `RTSHealth`, then emits `DeathEvent` if health reaches zero.
fn damage_resolution_system(
    mut commands: Commands,
    mut damage_events: EventReader<DamageEvent>,
    mut health_q: Query<(Entity, &mut RTSHealth), Without<Dying>>,
    mut death_events: EventWriter<DeathEvent>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for ev in damage_events.read() {
        let Ok((target_entity, mut health)) = health_q.get_mut(ev.target) else { continue };
        let actual = apply_armor(ev.damage, health.armor, &ev.damage_type);
        health.current = (health.current - actual).max(0.0);
        health.last_damage_time = now;
        if health.current <= 0.0 {
            commands.entity(target_entity).insert(Dying);
            death_events.send(DeathEvent { entity: target_entity });
        }
    }
}

/// Regenerates health for units not in active combat after a delay.
fn health_regen_system(
    mut health_q: Query<(&mut RTSHealth, Option<&CombatState>)>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for (mut health, combat_state) in health_q.iter_mut() {
        if let Some(cs) = combat_state {
            if matches!(
                cs.state,
                CombatStateType::InCombat
                    | CombatStateType::MovingToAttack
                    | CombatStateType::MovingToCombat
            ) {
                continue;
            }
        }
        if health.current < health.max
            && now - health.last_damage_time >= cc::REGEN_DELAY
            && health.regeneration_rate > 0.0
        {
            health.current = (health.current + health.regeneration_rate * time.delta_secs()).min(health.max);
        }
    }
}

/// Clears dead entities as combat targets and despawns them.
fn death_system(
    mut commands: Commands,
    mut death_events: EventReader<DeathEvent>,
    mut combat_q: Query<&mut Combat>,
) {
    for ev in death_events.read() {
        for mut combat in combat_q.iter_mut() {
            if combat.target == Some(ev.entity) {
                combat.target = None;
                combat.is_attacking = false;
            }
        }
        if let Some(ec) = commands.get_entity(ev.entity) {
            ec.despawn_recursive();
        }
    }
}

fn apply_armor(base: f32, armor: f32, damage_type: &DamageType) -> f32 {
    let reduction = match damage_type {
        DamageType::Physical => armor / (armor + 100.0),
        DamageType::Siege => (armor * 0.1) / (armor * 0.1 + 100.0),
    };
    base * (1.0 - reduction)
}

// Re-export for CombatStatePlugin usage in mod.rs
pub struct CombatStatePlugin;

impl Plugin for CombatStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (add_combat_state_to_fighters, update_combat_states).chain(),
        );
    }
}

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
    let Some(target) = combat.target else { return CombatStateType::Idle };
    let Ok(target_tf) = targets.get(target) else { return CombatStateType::Idle };
    let dist = transform.translation.distance(target_tf.translation);
    if dist <= combat.attack_range {
        return CombatStateType::InCombat;
    }
    if movement.is_some_and(|m| m.target_position.is_some()) {
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
