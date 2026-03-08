//! RTS gameplay systems.

pub mod combat;
pub mod movement;
pub mod resource;
pub mod selection;

pub use combat::CombatStatePlugin;
pub use movement::MovementPlugin;
pub use movement::pathfinding::PathfindingPlugin;
pub use movement::unit_commands::UnitCommandsPlugin;
pub use resource::construction::ConstructionPlugin;
pub use resource::ResourceStatePlugin;
pub use selection::SelectionPlugin;
