//! RTS gameplay systems.

pub mod combat;
pub mod construction;
pub mod cursor_manager;
pub mod movement;
pub mod production;
pub mod resource;
pub mod selection;

pub use combat::{CombatPlugin, CombatStatePlugin};
pub use construction::ConstructionPlugin;
pub use cursor_manager::CursorManagerPlugin;
pub use movement::formation::FormationPlugin;
pub use movement::MovementPlugin;
pub use movement::pathfinding::PathfindingPlugin;
pub use movement::unit_commands::UnitCommandsPlugin;
pub use production::ProductionPlugin;
pub use resource::ResourceStatePlugin;
pub use selection::SelectionPlugin;
