use bevy::prelude::*;
use crate::core::components::{BuildingType, ResourceType, UnitType};

/// A single AI goal with all parameters embedded.
#[derive(Debug, Clone)]
pub enum UnifiedGoal {
    /// Assign an idle worker to gather from a specific resource node.
    AssignWorkerToResource {
        worker: Entity,
        resource_entity: Entity,
        resource_type: ResourceType,
        resource_pos: Vec3,
    },
    /// Queue a unit for production. The strategy system resolves the building entity
    /// so the executor only needs to fire an event — no query required.
    BuildUnit {
        building: Entity,
        unit_type: UnitType,
        player_id: u8,
    },
    /// Order an AI unit to attack a specific target.
    AttackTarget {
        attacker: Entity,
        target: Entity,
    },
    /// Spawn a new building at the given position.
    BuildBuilding {
        building_type: BuildingType,
        position: Vec3,
        player_id: u8,
    },
}

/// A goal with an attached priority score (higher = executed first).
#[derive(Debug, Clone)]
pub struct PrioritizedGoal {
    pub priority: f32,
    pub goal: UnifiedGoal,
}

/// Per-frame goal queue. Strategy systems write goals here; `execute_ai_goals_system` drains it.
///
/// Owned by: AIPlugin (strategy systems write; execute_ai_goals_system drains)
#[derive(Resource, Default, Debug)]
pub struct GlobalGoalManager {
    pub goals: Vec<PrioritizedGoal>,
}

impl GlobalGoalManager {
    pub fn push(&mut self, priority: f32, goal: UnifiedGoal) {
        self.goals.push(PrioritizedGoal { priority, goal });
    }

    /// Returns true if any queued goal already references `entity` as its acting unit.
    pub fn has_goal_for(&self, entity: Entity) -> bool {
        self.goals.iter().any(|pg| match &pg.goal {
            UnifiedGoal::AssignWorkerToResource { worker, .. } => *worker == entity,
            UnifiedGoal::AttackTarget { attacker, .. } => *attacker == entity,
            UnifiedGoal::BuildUnit { .. } | UnifiedGoal::BuildBuilding { .. } => false,
        })
    }

    /// Drains all goals sorted by descending priority.
    pub fn drain_sorted(&mut self) -> Vec<PrioritizedGoal> {
        let mut goals = std::mem::take(&mut self.goals);
        goals.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        goals
    }
}
