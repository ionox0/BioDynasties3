//! Combat systems.
//!
//! ## Component ownership
//!
//! | Component            | Owner                       |
//! |----------------------|-----------------------------|
//! | `Combat.target`      | `combat_target_handler`     |
//! | `Combat.is_attacking`| `combat_execution_system`   |
//! | `RTSHealth`          | `damage_resolution_system`  |
//! | `Dying`              | `damage_resolution_system`  |
//! | `CombatState`        | `update_combat_states`      |
//!
//! ## Unit combat state machine
//!
//! ```text
//!            ┌──────── target dies / CombatStopEvent ──────────────────────┐
//!            │                                                             │
//!            ▼                                                             │
//!       ╔════════╗   enemy enters   ┌────────────────────┐  in range  ┌──────────┐
//!       ║  Idle  ║   VISION_RANGE   │  MovingToAttack /  │──────────▶ │ InCombat │──▶ DamageEvent
//!       ╚════════╝─────────────────▶│  MovingToCombat    │            │          │    per cooldown
//!                                   └────────────────────┘            └──────────┘
//!                                            ▲                              │
//!                                            └──────── target leaves range ─┘
//! ```
//!
//! ## System order each Update frame
//!
//! ```text
//! combat_target_handler    (CombatTargetEvent → Combat.target)
//! combat_stop_handler      (CombatStopEvent   → clear Combat)
//! target_management_system (validate + auto-acquire targets)
//! combat_execution_system  (attack + MovementTargetEvent / StopMovementEvent)
//! damage_resolution_system (DamageEvent → RTSHealth, DeathEvent)
//! health_regen_system
//! death_system             (DeathEvent → despawn, clear targets)
//! ```

pub mod events;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crate::core::components::*;
use crate::core::GameSet;
use crate::core::spatial_grid::IncrementalSpatialGrid;
use crate::rts::movement::events::{MovementTargetEvent, StopMovementEvent};
use self::events::{CombatStopEvent, CombatTargetEvent, DamageEvent, DeathEvent};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct CombatSet;

#[derive(SystemParam)]
pub(crate) struct AliveUnits<'w, 's> {
    query: Query<'w, 's, (Entity, &'static Transform, &'static RTSUnit), (With<RTSHealth>, Without<Dying>)>,
}

#[derive(SystemParam)]
pub(crate) struct CombatTargets<'w, 's> {
    query: Query<'w, 's, (&'static Transform, &'static CollisionRadius), (With<RTSHealth>, Without<Dying>)>,
}

#[derive(SystemParam)]
pub(crate) struct CombatUnits<'w, 's> {
    query: Query<'w, 's, (Entity, &'static mut Combat, &'static Transform, &'static RTSUnit, Option<&'static UnitState>)>,
}

#[derive(SystemParam)]
pub(crate) struct AttackingUnits<'w, 's> {
    query: Query<'w, 's, (Entity, &'static mut Combat, &'static Transform, &'static RTSUnit, &'static CollisionRadius, Option<&'static Movement>)>,
}

#[derive(SystemParam)]
pub(crate) struct HealthyUnits<'w, 's> {
    query: Query<'w, 's, (Entity, &'static mut RTSHealth), Without<Dying>>,
}

#[derive(SystemParam)]
pub(crate) struct CombatStateUnits<'w, 's> {
    query: Query<'w, 's, (Entity, &'static mut CombatState, &'static Combat, &'static Transform, Option<&'static Movement>, &'static mut UnitState), Without<Dying>>,
}

#[derive(SystemParam)]
pub(crate) struct AliveUnitTransforms<'w, 's> {
    query: Query<'w, 's, &'static Transform, (With<RTSHealth>, Without<Dying>)>,
}

// ─── Constants ───────────────────────────────────────────────────────────────

mod cc {
    pub const REGEN_DELAY: f32 = 5.0;
    pub const ATTACK_RANGE_MARGIN: f32 = 0.8;
    pub const TARGET_POSITION_RATIO: f32 = 0.7;
    pub const GRID_CELL_SIZE: f32 = 150.0;
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
    pub fn query_nearby(&self, position: Vec3, range: f32) -> Vec<(Entity, Vec3, u8)> {
        self.0
            .query_nearby_with_keys(position)
            .into_iter()
            .filter(|(_, (pos, _))| position.distance(*pos) <= range)
            .map(|(entity, (pos, pid))| (entity, *pos, *pid))
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
                    retaliate_on_attack,
                    health_regen_system,
                    death_system,
                )
                    .chain()
                    .in_set(CombatSet)
                    .in_set(GameSet::RtsUpdate),
            )
            .add_systems(
                Update,
                (add_combat_state_to_fighters, update_combat_states)
                    .chain()
                    .after(CombatSet)
                    .in_set(GameSet::RtsUpdate),
            );
    }
}

// ─── Event handlers ──────────────────────────────────────────────────────────

/// Handles `CombatTargetEvent` — assigns `Combat.target`, sets `UnitState::MovingToAttack`,
/// and optionally triggers movement.
fn combat_target_handler(
    mut commands: Commands,
    mut target_events: EventReader<CombatTargetEvent>,
    mut move_events: EventWriter<MovementTargetEvent>,
    mut combat_q: Query<&mut Combat>,
    transform_q: Query<&Transform>,
) {
    for ev in target_events.read() {
        let Ok(mut combat) = combat_q.get_mut(ev.attacker) else { continue };
        combat.target = Some(ev.target);
        combat.auto_attack = true;
        commands.entity(ev.attacker).insert(UnitState::MovingToAttack);
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

/// Handles `CombatStopEvent` — clears combat state only.
/// Does NOT send StopMovementEvent: every caller also issues a MovementTargetEvent,
/// so stopping movement here would race with that command and leave the unit stuck.
fn combat_stop_handler(
    mut stop_events: EventReader<CombatStopEvent>,
    mut combat_q: Query<&mut Combat>,
) {
    for ev in stop_events.read() {
        let Ok(mut combat) = combat_q.get_mut(ev.entity) else { continue };
        combat.target = None;
        combat.is_attacking = false;
        combat.auto_attack = false;
    }
}

// ─── Target management ───────────────────────────────────────────────────────

/// Refreshes the spatial grid, validates existing targets, and auto-acquires new ones.
fn target_management_system(
    mut combatants: CombatUnits,
    alive: AliveUnits,
    mut target_grid: ResMut<TargetGrid>,
    mut removed: RemovedComponents<RTSUnit>,
) {
    for entity in removed.read() {
        target_grid.0.remove_item(entity);
    }
    for (entity, transform, unit) in alive.query.iter() {
        target_grid.0.update_item(entity, transform.translation, (transform.translation, unit.player_id));
    }

    for (_, mut combat, transform, unit, activity) in combatants.query.iter_mut() {
        if let Some(target) = combat.target {
            if alive.query.get(target).is_err() {
                combat.target = None;
            }
        }
        if combat.target.is_some() && !combat.auto_attack {
            continue;
        }
        // Only auto-acquire from idle — respect active player move orders.
        let is_idle = activity.is_none_or(|a| *a == UnitState::Idle);
        if !is_idle {
            continue;
        }
        let nearby = target_grid.query_nearby(transform.translation, cc::VISION_RANGE);
        combat.target = nearby
            .into_iter()
            .filter(|(_, _, pid)| *pid != unit.player_id)
            .min_by(|(_, pa, _), (_, pb, _)| {
                transform.translation.distance(*pa)
                    .partial_cmp(&transform.translation.distance(*pb))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(e, _, _)| e);
    }
}

// ─── Execution & damage ──────────────────────────────────────────────────────

/// Moves units toward their target and fires `DamageEvent` when in range.
fn combat_execution_system(
    mut attackers: AttackingUnits,
    targets: CombatTargets,
    mut damage_events: EventWriter<DamageEvent>,
    mut move_events: EventWriter<MovementTargetEvent>,
    mut stop_events: EventWriter<StopMovementEvent>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for (attacker, mut combat, atf, _unit, acol, movement) in attackers.query.iter_mut() {
        let Some(target) = combat.target else {
            combat.is_attacking = false;
            continue;
        };
        let Ok((ttf, tcol)) = targets.query.get(target) else {
            combat.target = None;
            combat.is_attacking = false;
            continue;
        };

        let edge_dist = atf.translation.distance(ttf.translation) - acol.radius - tcol.radius;

        if edge_dist > combat.attack_range * cc::ATTACK_RANGE_MARGIN {
            let dir = (ttf.translation - atf.translation).normalize();
            let dest = ttf.translation - dir * (combat.attack_range * cc::TARGET_POSITION_RATIO);
            let needs_move = movement.map_or(true, |m| m.target_position.is_none());
            if needs_move {
                move_events.send(MovementTargetEvent { entity: attacker, target_position: dest });
            }
            combat.is_attacking = false;
        } else {
            // In effective range: stop and attack.
            stop_events.send(StopMovementEvent { entity: attacker });
            if now - combat.last_attack_time >= combat.attack_cooldown {
                combat.last_attack_time = now;
                combat.is_attacking = true;
                damage_events.send(DamageEvent {
                    attacker,
                    target,
                    damage: combat.attack_damage,
                    damage_type: damage_type_for(&combat.attack_type),
                });
            }
        }
    }
}

/// Applies `DamageEvent` to `RTSHealth`, then emits `DeathEvent` if health reaches zero.
fn damage_resolution_system(
    mut commands: Commands,
    mut damage_events: EventReader<DamageEvent>,
    mut living: HealthyUnits,
    mut death_events: EventWriter<DeathEvent>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for ev in damage_events.read() {
        let Ok((target_entity, mut health)) = living.query.get_mut(ev.target) else { continue };
        let actual = apply_armor(ev.damage, health.armor, &ev.damage_type);
        health.current = (health.current - actual).max(0.0);
        health.last_damage_time = now;
        if health.current <= 0.0 {
            commands.entity(target_entity).insert(Dying);
            death_events.send(DeathEvent { entity: target_entity });
        }
    }
}

// ─── Regeneration & death ────────────────────────────────────────────────────

/// Regenerates health for units not in active combat after a delay.
fn health_regen_system(
    mut health_q: Query<(&mut RTSHealth, Option<&UnitState>)>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for (mut health, unit_state) in health_q.iter_mut() {
        let in_active_combat = unit_state.is_some_and(|us| matches!(
            us,
            UnitState::InCombat | UnitState::MovingToAttack | UnitState::MovingToCombat
        ));
        if in_active_combat { continue; }
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

// ─── Damage math ─────────────────────────────────────────────────────────────

fn damage_type_for(attack_type: &AttackType) -> DamageType {
    match attack_type {
        AttackType::Melee => DamageType::Physical,
        AttackType::Siege => DamageType::Siege,
    }
}

fn apply_armor(base: f32, armor: f32, damage_type: &DamageType) -> f32 {
    let reduction = match damage_type {
        DamageType::Physical => armor / (armor + 100.0),
        DamageType::Siege => (armor * 0.1) / (armor * 0.1 + 100.0),
    };
    base * (1.0 - reduction)
}


/// When a military unit (no `ResourceGatherer`) takes damage, immediately retarget the attacker.
fn retaliate_on_attack(
    mut damage_events: EventReader<DamageEvent>,
    military: Query<Entity, (With<Combat>, Without<ResourceGatherer>, Without<Dying>)>,
    mut target_events: EventWriter<CombatTargetEvent>,
) {
    for ev in damage_events.read() {
        if military.get(ev.target).is_err() { continue; }
        target_events.send(CombatTargetEvent { attacker: ev.target, target: ev.attacker, move_to_range: true });
    }
}

fn add_combat_state_to_fighters(
    mut commands: Commands,
    new_fighters: Query<(Entity, &Combat), Added<Combat>>,
) {
    for (entity, combat) in new_fighters.iter() {
        if combat.auto_attack {
            commands.entity(entity).insert((CombatState::default(), UnitState::Idle));
        }
    }
}

fn update_combat_states(
    mut units: CombatStateUnits,
    alive_transforms: AliveUnitTransforms,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for (_, mut state, combat, transform, movement, mut unit_state) in units.query.iter_mut() {
        if is_gathering_or_moving_state(&unit_state) {
            continue;
        }
        let new_state = derive_combat_state(combat, transform, movement, &alive_transforms.query);
        if new_state != *unit_state {
            state.last_state_change = now;
            *unit_state = new_state;
        }
        update_target_refs(&mut state, combat, &alive_transforms.query);
    }
}

fn is_gathering_or_moving_state(state: &UnitState) -> bool {
    matches!(
        state,
        UnitState::Moving
            | UnitState::MovingToResource
            | UnitState::Gathering
            | UnitState::ReturningToBase
            | UnitState::DeliveringResources
    )
}

fn derive_combat_state(
    combat: &Combat,
    transform: &Transform,
    movement: Option<&Movement>,
    targets: &Query<&Transform, (With<RTSHealth>, Without<Dying>)>,
) -> UnitState {
    let Some(target) = combat.target else { return UnitState::Idle };
    let Ok(target_tf) = targets.get(target) else { return UnitState::Idle };
    if transform.translation.distance(target_tf.translation) <= combat.attack_range {
        return UnitState::InCombat;
    }
    if movement.is_some_and(|m| m.target_position.is_some()) {
        UnitState::MovingToAttack
    } else {
        UnitState::MovingToCombat
    }
}

fn update_target_refs(
    state: &mut CombatState,
    combat: &Combat,
    targets: &Query<&Transform, (With<RTSHealth>, Without<Dying>)>,
) {
    let resolved = combat.target
        .and_then(|t| targets.get(t).ok().map(|tf| (t, tf.translation)));
    state.target_entity = resolved.map(|(e, _)| e);
    state.target_position = resolved.map(|(_, p)| p);
}
