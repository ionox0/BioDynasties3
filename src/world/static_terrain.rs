//! Static Preloaded Terrain System
//! 
//! Simple, efficient terrain system designed specifically for RTS games.
//! Preloads the entire map at startup for instant camera response and zero lag.

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::random;
use super::triplanar_mapping::SimpleMaterial;
use tracing::instrument;
use crate::core::constants::movement::TERRAIN_SIZE;

// Enhanced terrain configuration for dramatic hills and varied landscapes with high granularity
pub const NOISE_HEIGHT: f32 = 120.0; // Dramatic terrain height variation - 8x increase!
pub const NOISE_SCALE: f32 = 400.0; // Reduced from 800.0 - creates more frequent terrain features
pub const RIDGES_SCALE: f32 = 600.0; // Reduced from 1200.0 - more frequent ridge/valley systems
pub const DETAIL_SCALE: f32 = 100.0; // Reduced from 200.0 - finer surface details
pub const MICRO_SCALE: f32 = 40.0; // Very fine micro-variations for maximum granularity

// Shared constants for texture and navigation consistency
// These heights determine both texture selection and terrain passability
pub const ROCKY_TERRAIN_HEIGHT_THRESHOLD: f32 = 36.0;      // Above this = rocky_terrain (IMPASSABLE)



const NORMAL_GRID_RESOLUTION: usize = 256;

/// Precomputed terrain normals on a 256×256 grid — O(1) lookup at runtime.
#[derive(Resource)]
pub struct TerrainNormals {
    normals: Vec<Vec3>,
    terrain_size: f32,
}

impl TerrainNormals {
    fn build(terrain: &StaticTerrainHeights) -> Self {
        let res = NORMAL_GRID_RESOLUTION;
        let size = TERRAIN_SIZE;
        let step = size * 2.0 / (res - 1) as f32;
        let mut normals = Vec::with_capacity(res * res);
        for z in 0..res {
            for x in 0..res {
                let wx = -size + x as f32 * step;
                let wz = -size + z as f32 * step;
                normals.push(Vec3::from(calculate_terrain_normal(wx, wz, step, terrain)));
            }
        }
        Self { normals, terrain_size: size }
    }

    pub fn get_normal(&self, x: f32, z: f32) -> Vec3 {
        let res = NORMAL_GRID_RESOLUTION;
        let t = self.terrain_size;
        let ix = ((x + t) / (t * 2.0) * (res - 1) as f32).clamp(0.0, (res - 1) as f32) as usize;
        let iz = ((z + t) / (t * 2.0) * (res - 1) as f32).clamp(0.0, (res - 1) as f32) as usize;
        self.normals[iz * res + ix]
    }
}

pub struct StaticTerrainPlugin;

impl Plugin for StaticTerrainPlugin {
    #[instrument(skip_all)]
    fn build(&self, app: &mut App) {
        let seed: u32 = random();
        info!("Map seed: {seed}");
        let terrain = StaticTerrainHeights::from_seed(seed);
        let normals = TerrainNormals::build(&terrain);
        app.insert_resource(terrain)
            .insert_resource(normals)
            .add_systems(Startup, spawn_terrain_mesh);
    }
}

/// Vertices per side of the terrain mesh and the baked height grid (they share one lattice).
pub const HEIGHT_GRID_RESOLUTION: usize = 256;

/// Terrain heights baked once from noise at the mesh vertices.
/// Lookups interpolate the same triangles the mesh renders, so units sit exactly on the visible surface.
#[derive(Resource, Debug)]
pub struct StaticTerrainHeights {
    heights: Vec<f32>,
}

impl StaticTerrainHeights {
    pub fn from_seed(seed: u32) -> Self {
        let noise = Perlin::new(seed);
        let res = HEIGHT_GRID_RESOLUTION;
        let step = height_grid_step();
        let heights = (0..res * res)
            .map(|i| {
                let (x, z) = ((i % res) as f32 * step - TERRAIN_SIZE, (i / res) as f32 * step - TERRAIN_SIZE);
                sample_terrain_height(&noise, x, z)
            })
            .collect();
        Self { heights }
    }

    fn vertex(&self, x: usize, z: usize) -> f32 {
        self.heights[z * HEIGHT_GRID_RESOLUTION + x]
    }

    /// Terrain height at any world position, interpolated across the mesh triangle containing it.
    pub fn get_height(&self, x: f32, z: f32) -> f32 {
        let last = (HEIGHT_GRID_RESOLUTION - 1) as f32;
        let gx = ((x + TERRAIN_SIZE) / height_grid_step()).clamp(0.0, last - 1e-3);
        let gz = ((z + TERRAIN_SIZE) / height_grid_step()).clamp(0.0, last - 1e-3);
        let (ix, iz) = (gx as usize, gz as usize);
        let (fx, fz) = (gx.fract(), gz.fract());
        // Mesh splits each quad along the bottom-right → top-left diagonal.
        let bottom_left = self.vertex(ix, iz);
        let bottom_right = self.vertex(ix + 1, iz);
        let top_left = self.vertex(ix, iz + 1);
        let top_right = self.vertex(ix + 1, iz + 1);
        if fx + fz <= 1.0 {
            bottom_left + (bottom_right - bottom_left) * fx + (top_left - bottom_left) * fz
        } else {
            top_right + (top_left - top_right) * (1.0 - fx) + (bottom_right - top_right) * (1.0 - fz)
        }
    }
    
    /// Check if a position is passable for unit movement
    pub fn is_passable(&self, x: f32, z: f32) -> bool {
        self.get_height(x, z) <= ROCKY_TERRAIN_HEIGHT_THRESHOLD
    }

    /// Searches outward from `origin` in 50-unit rings until a passable cell is found.
    /// Returns the original point if it is already passable.
    pub fn find_passable_near(&self, origin: Vec2) -> Vec2 {
        const STEP: f32 = 50.0;
        const MAX_RING: i32 = 40; // up to 2 000 units away
        if self.is_passable(origin.x, origin.y) {
            return origin;
        }
        for ring in 1..=MAX_RING {
            for dx in -ring..=ring {
                for dz in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring { continue; }
                    let x = origin.x + dx as f32 * STEP;
                    let z = origin.y + dz as f32 * STEP;
                    if self.is_passable(x, z) {
                        return Vec2::new(x, z);
                    }
                }
            }
        }
        origin
    }
}


/// Spawns the heightfield terrain mesh with its triplanar material.
#[instrument(skip_all)]
fn spawn_terrain_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut simple_materials: ResMut<Assets<SimpleMaterial>>,
    terrain_heights: Res<StaticTerrainHeights>,
    asset_server: Res<AssetServer>,
) {
    let mesh = generate_heightfield_mesh(TERRAIN_SIZE * 2.0, &terrain_heights);
    let material = SimpleMaterial {
        grass_texture: asset_server.load("textures/rocky_terrain.jpg"),
        dirt_texture: asset_server.load("textures/rocky_terrain_02.jpg"),
        light_direction: Vec3::new(0.5, -1.0, 0.3), // Sun angle: slightly angled from above
    };
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(simple_materials.add(material)),
        Transform::default(),
        Name::new("Terrain"),
    ));
}

/// Heightfield mesh on the shared height-grid lattice. Vertex colours mark passability
/// (green passable, red blocked) for the triplanar shader.
#[instrument(skip_all)]
fn generate_heightfield_mesh(terrain_size: f32, terrain_heights: &StaticTerrainHeights) -> Mesh {
    use bevy::render::mesh::{Indices, PrimitiveTopology};
    use bevy::render::render_asset::RenderAssetUsages;
    const TEXTURE_SCALE: f32 = 0.01; // UV units per world unit

    let resolution = HEIGHT_GRID_RESOLUTION;
    let step = terrain_size / (resolution - 1) as f32;
    let points: Vec<(f32, f32)> = (0..resolution * resolution)
        .map(|i| ((i % resolution) as f32 * step - terrain_size * 0.5, (i / resolution) as f32 * step - terrain_size * 0.5))
        .collect();
    let positions: Vec<[f32; 3]> = points.iter().map(|&(x, z)| [x, terrain_heights.get_height(x, z), z]).collect();
    let normals: Vec<[f32; 3]> = points.iter().map(|&(x, z)| calculate_terrain_normal(x, z, step, terrain_heights)).collect();
    let uvs: Vec<[f32; 2]> = points.iter().map(|&(x, z)| [x * TEXTURE_SCALE, z * TEXTURE_SCALE]).collect();
    let colors: Vec<[f32; 4]> = points
        .iter()
        .map(|&(x, z)| if terrain_heights.is_passable(x, z) { [0.2, 1.0, 0.2, 1.0] } else { [1.0, 0.2, 0.2, 1.0] })
        .collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(grid_triangle_indices(resolution as u32)));
    mesh
}

/// Two triangles per grid quad, split along the bottom-right → top-left diagonal
/// (`StaticTerrainHeights::get_height` interpolates the same triangles).
fn grid_triangle_indices(resolution: u32) -> Vec<u32> {
    (0..resolution - 1)
        .flat_map(|z| (0..resolution - 1).map(move |x| z * resolution + x))
        .flat_map(|bottom_left| {
            let (bottom_right, top_left) = (bottom_left + 1, bottom_left + resolution);
            [bottom_left, top_left, bottom_right, bottom_right, top_left, top_left + 1]
        })
        .collect()
}

/// Calculate smooth terrain normal using neighboring heights
fn calculate_terrain_normal(x: f32, z: f32, step: f32, terrain_heights: &StaticTerrainHeights) -> [f32; 3] {
    // Sample neighboring heights for normal calculation
    let h_left = terrain_heights.get_height(x - step, z);
    let h_right = terrain_heights.get_height(x + step, z);
    let h_down = terrain_heights.get_height(x, z - step);
    let h_up = terrain_heights.get_height(x, z + step);
    
    // Calculate gradients
    let dx = (h_right - h_left) / (2.0 * step);
    let dz = (h_up - h_down) / (2.0 * step);
    
    // Normal vector (cross product of tangent vectors)
    let normal = Vec3::new(-dx, 1.0, -dz).normalize();
    [normal.x, normal.y, normal.z]
}



fn height_grid_step() -> f32 {
    TERRAIN_SIZE * 2.0 / (HEIGHT_GRID_RESOLUTION - 1) as f32
}

/// Sample terrain height at world coordinates with enhanced features
fn sample_terrain_height(noise: &Perlin, x: f32, z: f32) -> f32 {
    // Primary landscape features (mountains, hills, valleys)
    let nx_primary = (x / NOISE_SCALE) as f64;
    let nz_primary = (z / NOISE_SCALE) as f64;
    let primary_height = noise.get([nx_primary, nz_primary]) as f32 * NOISE_HEIGHT;
    
    // Ridge and valley systems (different scale for more variety)
    let nx_ridges = (x / RIDGES_SCALE) as f64;
    let nz_ridges = (z / RIDGES_SCALE) as f64; 
    let ridge_height = noise.get([nx_ridges + 100.0, nz_ridges + 100.0]) as f32 * NOISE_HEIGHT * 0.7;
    
    // Medium-scale features (rolling hills)
    let nx_med = (x / (NOISE_SCALE * 0.5)) as f64;
    let nz_med = (z / (NOISE_SCALE * 0.5)) as f64;
    let medium_height = noise.get([nx_med + 200.0, nz_med + 200.0]) as f32 * NOISE_HEIGHT * 0.4;
    
    // Fine details (surface texture)
    let nx_detail = (x / DETAIL_SCALE) as f64;
    let nz_detail = (z / DETAIL_SCALE) as f64;
    let detail_height = noise.get([nx_detail + 300.0, nz_detail + 300.0]) as f32 * NOISE_HEIGHT * 0.15;
    
    // Micro-scale variations for maximum terrain granularity
    let nx_micro = (x / MICRO_SCALE) as f64;
    let nz_micro = (z / MICRO_SCALE) as f64;
    let micro_height = noise.get([nx_micro + 400.0, nz_micro + 400.0]) as f32 * NOISE_HEIGHT * 0.08;
    
    // Create terrain mask for different biomes
    let biome_noise = noise.get([(x / 2000.0) as f64, (z / 2000.0) as f64]) as f32;
    
    // Combine all height layers with biome-based variation including micro-details
    let base_height = primary_height + ridge_height + medium_height + detail_height + micro_height;
    
    // Apply biome modifications
    let final_height = if biome_noise > 0.3 {
        // Mountainous regions - amplify height
        base_height * 1.4 + 20.0
    } else if biome_noise < -0.3 {
        // Valley regions - reduce height and add gentle rolling
        (base_height * 0.6 - 15.0).max(-25.0)
    } else {
        // Plains regions - moderate height with gentle variation
        base_height * 0.8 + biome_noise * 10.0
    };
    
    // Ensure minimum height for gameplay (don't go too far below sea level)
    final_height.max(-30.0)
}
