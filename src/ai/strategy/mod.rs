pub mod combat;
pub mod production;
pub mod workers;

pub use combat::combat_goal_system;
pub use production::production_goal_system;
pub use workers::worker_goal_system;
