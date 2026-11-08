/// Building occupancy grid for spawn/placement clearance
pub mod building_grid;
/// Debug grid overlay
pub mod grid_overlay;
/// Static terrain system for RTS gameplay
pub mod static_terrain;
/// Triplanar mapping for terrain texturing
pub mod triplanar_mapping;

pub use grid_overlay::GridOverlayPlugin;
pub use static_terrain::StaticTerrainPlugin;
pub use triplanar_mapping::SimpleMaterialPlugin;
