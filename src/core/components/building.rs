use bevy::prelude::*;
use super::types::{BuildingType, UnitType};

// Owned by: ConstructionPlugin (apply_construction_progress)
#[derive(Component, Debug, Clone)]
pub struct Building {
    pub player_id: u8,
    pub building_type: BuildingType,
    pub construction_progress: f32,
    pub max_construction: f32,
    pub is_complete: bool,
}

// Owned by: ConstructionPlugin (construction_system)
#[derive(Component, Debug, Clone)]
pub struct Constructor {
    pub build_speed: f32,
    pub current_target: Option<Entity>,
}

/// Pending unit production queue for a building.
///
/// Field ownership:
/// - `queued`    — `ProductionPlugin` (apply_production_queue_events, via events)
/// - `progress`  — `ProductionPlugin` (production_queue_system, direct write)
#[derive(Component, Debug, Clone)]
pub struct ProductionQueue {
    pub queued: Vec<UnitType>,
    pub progress: f32,
    pub production_time: f32,
}

/// Fraction of a building's production time that worker ants take.
const WORKER_TIME_SCALE: f32 = 0.1;

impl ProductionQueue {
    /// Seconds this building takes to produce `unit_type`.
    pub fn time_for(&self, unit_type: &UnitType) -> f32 {
        let scale = if matches!(unit_type, UnitType::Fourmi) { WORKER_TIME_SCALE } else { 1.0 };
        self.production_time * scale
    }
}

impl Default for ProductionQueue {
    fn default() -> Self {
        Self { queued: Vec::new(), progress: 0.0, production_time: 5.0 }
    }
}
