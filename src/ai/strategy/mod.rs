pub mod buildings;
pub mod combat;
pub mod production;
pub mod workers;

pub(crate) use buildings::{BuildingParams, generate_building_goals};
pub(crate) use combat::{CombatParams, generate_combat_goals};
pub(crate) use production::{ProductionParams, generate_production_goals};
pub(crate) use workers::{WorkerParams, generate_worker_goals};
