/// Debug grid overlay
pub mod grid;
/// Static terrain system for RTS gameplay
pub mod static_terrain;
/// Triplanar mapping for terrain texturing
pub mod triplanar_mapping;

pub use grid::GridPlugin;
pub use static_terrain::StaticTerrainPlugin;
pub use triplanar_mapping::SimpleMaterialPlugin;
