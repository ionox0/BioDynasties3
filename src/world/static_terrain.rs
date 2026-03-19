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

// Shared constants for texture and pathfinding consistency
// These heights determine both texture selection and terrain passability
pub const ROCKY_TERRAIN_HEIGHT_THRESHOLD: f32 = 36.0;      // Above this = rocky_terrain (IMPASSABLE)


/// Stores the seed used for this session's terrain so other systems can reproduce it.
#[derive(Resource)]
pub struct MapSeed(pub u32);

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
        app.insert_resource(MapSeed(seed))
            .insert_resource(terrain)
            .insert_resource(normals)
            .insert_resource(DebugCounters::default())
            .add_systems(Update, (debug_system_update, check_textures_and_generate_terrain, debug_terrain_entities, immediate_entity_verification));
    }
}


#[derive(Resource, Default)]
pub struct DebugCounters {
    debug_counter: u32,
    entity_debug_counter: u32,
    verify_counter: u32,
}

/// Simple terrain height provider for static terrain
#[derive(Resource, Debug)]
pub struct StaticTerrainHeights {
    noise: Perlin,
}

impl StaticTerrainHeights {
    pub fn from_seed(seed: u32) -> Self {
        Self { noise: Perlin::new(seed) }
    }
}

impl StaticTerrainHeights {
    /// Get terrain height at any world position
    pub fn get_height(&self, x: f32, z: f32) -> f32 {
        sample_terrain_height(&self.noise, x, z)
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

    /// Get detailed terrain information including passability
    pub fn get_terrain_info(&self, x: f32, z: f32) -> TerrainInfo {
        let height = self.get_height(x, z);
        TerrainInfo {
            is_passable: height <= ROCKY_TERRAIN_HEIGHT_THRESHOLD,
        }
    }
}


#[derive(Component)]
pub struct TerrainChunk;

/// Terrain information for a given position
#[derive(Debug, Clone)]
pub struct TerrainInfo {
    pub is_passable: bool,
}




#[instrument(skip_all)]
fn debug_system_update(mut debug_counters: ResMut<DebugCounters>) {
    debug_counters.debug_counter += 1;
}

/// Check if textures are loaded and generate terrain with proper sampler configuration
#[instrument(skip_all)]
pub fn check_textures_and_generate_terrain(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    simple_materials: ResMut<Assets<SimpleMaterial>>,
    terrain_heights: Res<StaticTerrainHeights>,
    asset_server: Res<AssetServer>,
    mut terrain_generated: Local<bool>,
) {
    // Only run once
    if *terrain_generated {
        return;
    }
    
    // Generate terrain immediately with texture
    
    // Generate terrain with texture material
    generate_static_terrain_with_simple(
        commands,
        meshes,
        simple_materials,
        terrain_heights,
        asset_server,
    );
    
    *terrain_generated = true;
}

/// Generate 3D heightfield terrain with simple texture material
#[instrument(skip_all)]
fn generate_static_terrain_with_simple(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut simple_materials: ResMut<Assets<SimpleMaterial>>,
    terrain_heights: Res<StaticTerrainHeights>,
    asset_server: Res<AssetServer>,
) {
    use crate::core::constants::movement::TERRAIN_SIZE;
    let terrain_diameter = TERRAIN_SIZE * 2.0;
    
    
    // Generate heightfield mesh with actual 3D geometry
    let mesh = generate_heightfield_mesh(terrain_diameter, &terrain_heights);
    let mesh_handle = meshes.add(mesh);
    
    // Simple working material with reliable textures and configurable lighting
    let terrain_material = SimpleMaterial {
        grass_texture: asset_server.load("textures/rocky_terrain.jpg"),
        dirt_texture: asset_server.load("textures/rocky_terrain_02.jpg"),
        light_direction: Vec3::new(0.5, -1.0, 0.3), // Sun angle: slightly angled from above
    };
    
    let simple_material_handle = simple_materials.add(terrain_material);
    
    // Spawn terrain entity with enhanced triplanar material
    
    
    // ONLY USE TRIPLANAR MATERIAL - no fallback
    commands
        .spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(simple_material_handle.clone()),
            Transform::from_translation(Vec3::ZERO), // At ground level
            Visibility::Visible,
            InheritedVisibility::VISIBLE,
            ViewVisibility::HIDDEN,
            TerrainChunk,
            Name::new("ONLY Triplanar Terrain"),
        ));
        
    // IMMEDIATE POST-SPAWN VERIFICATION (entities will be available next frame)
}


/// Generate a 3D heightfield mesh based on noise terrain with biome-based vertex colors
#[instrument(skip_all)]
fn generate_heightfield_mesh(terrain_size: f32, terrain_heights: &StaticTerrainHeights) -> Mesh {
    use bevy::render::mesh::PrimitiveTopology;
    use bevy::render::render_asset::RenderAssetUsages;
    
    // Mesh resolution - balance between quality and performance
    let resolution = 256; // 256x256 vertices = 65,536 vertices (good balance)
    let step = terrain_size / (resolution - 1) as f32;
    let half_size = terrain_size * 0.5;
    
    
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    
    // Track height range and passability for debugging
    let mut min_height = f32::MAX;
    let mut max_height = f32::MIN;
    let mut _passable_vertices = 0;
    let mut _blocked_vertices = 0;
    
    // Generate vertices with height variation and biome-based coloring
    for z_index in 0..resolution {
        for x_index in 0..resolution {
            let x = x_index as f32 * step - half_size;
            let z = z_index as f32 * step - half_size;
            let height = terrain_heights.get_height(x, z);
            
            // Track height range
            min_height = min_height.min(height);
            max_height = max_height.max(height);
            
            
            // Position
            positions.push([x, height, z]);
            
            // UV coordinates - use world space coordinates with proper scaling
            let texture_scale = 0.01; // 1 world unit = 0.01 texture units
            uvs.push([
                x * texture_scale,
                z * texture_scale,
            ]);
            
            // Calculate normal using neighboring heights
            let normal = calculate_terrain_normal(x, z, step, terrain_heights);
            normals.push(normal);
            
            // Color vertices based on terrain passability for visual debugging
            let terrain_info = terrain_heights.get_terrain_info(x, z);
            let color = if terrain_info.is_passable {
                _passable_vertices += 1;
                // Green for passable terrain
                [0.2, 1.0, 0.2, 1.0]
            } else {
                _blocked_vertices += 1;
                // Red for blocked terrain
                [1.0, 0.2, 0.2, 1.0]
            };
            colors.push(color);
        }
    }
    
    // Generate triangle indices (two triangles per quad)
    for z in 0..(resolution - 1) {
        for x in 0..(resolution - 1) {
            let bottom_left = z * resolution + x;
            let bottom_right = bottom_left + 1;
            let top_left = bottom_left + resolution;
            let top_right = top_left + 1;
            
            // First triangle
            indices.push(bottom_left as u32);
            indices.push(top_left as u32);
            indices.push(bottom_right as u32);
            
            // Second triangle  
            indices.push(bottom_right as u32);
            indices.push(top_left as u32);
            indices.push(top_right as u32);
        }
    }
    
    
    
    // Create mesh with enhanced attributes
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors); // Add vertex colors for biome variation
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    
    mesh
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



/// Sample terrain height at world coordinates with enhanced features
pub fn sample_terrain_height(noise: &Perlin, x: f32, z: f32) -> f32 {
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

/// Debug system to track terrain entities
fn debug_terrain_entities(
    terrain_query: Query<(Entity, &Transform, &Name), With<TerrainChunk>>,
    mut debug_counters: ResMut<DebugCounters>,
) {
    debug_counters.entity_debug_counter += 1;
    if debug_counters.entity_debug_counter.is_multiple_of(300) { // Every 5 seconds at 60fps
        let terrain_count = terrain_query.iter().count();
        if terrain_count == 0 {
        }
    }
}

type TerrainVerifyQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Transform, &'static Name, &'static Mesh3d, &'static MeshMaterial3d<SimpleMaterial>),
    With<TerrainChunk>,
>;

/// Immediate verification system that runs right after terrain generation
fn immediate_entity_verification(
    terrain_query: TerrainVerifyQuery,
    all_entities_query: Query<(Entity, Option<&Name>)>,
    mut verified: Local<bool>,
    mut debug_counters: ResMut<DebugCounters>,
) {
    // Only run once after terrain generation
    if *verified {
        return;
    }
    
    debug_counters.verify_counter += 1;
    if debug_counters.verify_counter.is_multiple_of(120) { // Every 2 seconds
        // Check all entities in world
        let _total_entities = all_entities_query.iter().count();
        
        // Check specifically for terrain entities
        let terrain_count = terrain_query.iter().count();
        if terrain_count > 0 {
            *verified = true; // Stop verifying once we find entities
        } else {
            // Still checking for terrain entities...
            
            // Look for any entities with terrain-related names
            for (_entity, name_opt) in all_entities_query.iter() {
                if let Some(name) = name_opt {
                    let name_str = name.to_string();
                    if name_str.contains("Terrain") {
                    }
                }
            }
        }
    }
}

