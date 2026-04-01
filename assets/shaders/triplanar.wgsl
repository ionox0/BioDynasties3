#import bevy_pbr::forward_io::VertexOutput

@group(2) @binding(0) var rocky_texture: texture_2d<f32>;
@group(2) @binding(1) var rocky_sampler: sampler;
@group(2) @binding(2) var rocky_texture_02: texture_2d<f32>;
@group(2) @binding(3) var rocky_sampler_02: sampler;
@group(2) @binding(4) var<uniform> light_direction: vec3<f32>;

fn hash2(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(127.1, 311.7));
    return fract(dot(q, q + 19.19) * 43758.5453);
}

fn rotate_uv(uv: vec2<f32>, tile: vec2<f32>) -> vec2<f32> {
    let r = floor(hash2(tile) * 4.0);
    let c = uv - 0.5;
    if r < 1.0       { return c + 0.5; }
    else if r < 2.0  { return vec2<f32>(-c.y, c.x) + 0.5; }
    else if r < 3.0  { return vec2<f32>(-c.x, -c.y) + 0.5; }
    else             { return vec2<f32>(c.y, -c.x) + 0.5; }
}

fn sample_tiled(texture: texture_2d<f32>, tex_sampler: sampler, uv: vec2<f32>) -> vec4<f32> {
    let tile = floor(uv);
    let local = rotate_uv(fract(uv), tile);
    return textureSample(texture, tex_sampler, local);
}

// Sample triplanar textures for a given texture and return blended result
fn sample_triplanar_texture(
    texture: texture_2d<f32>,
    tex_sampler: sampler,
    uv_xy: vec2<f32>,
    uv_xz: vec2<f32>,
    uv_yz: vec2<f32>,
    triplanar_weights: vec3<f32>
) -> vec4<f32> {
    let sample_xy = sample_tiled(texture, tex_sampler, uv_xy);
    let sample_xz = sample_tiled(texture, tex_sampler, uv_xz);
    let sample_yz = sample_tiled(texture, tex_sampler, uv_yz);

    return sample_yz * triplanar_weights.x +    // X-facing surfaces use YZ projection
           sample_xz * triplanar_weights.y +    // Y-facing surfaces use XZ projection
           sample_xy * triplanar_weights.z;     // Z-facing surfaces use XY projection
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // TRIPLANAR TERRAIN TEXTURE MAPPING WITH ELEVATION-BASED SHADOW SHADING
    // Use actual terrain textures with triplanar mapping for realistic terrain rendering
    
    let world_pos = in.world_position.xyz;
    let world_normal = normalize(in.world_normal);
    
    // Directional light for elevation-based shadow shading
    // Light direction is now configurable via uniform
    let normalized_light = normalize(light_direction);
    
    // Calculate triplanar texture coordinates with proper scaling
    let texture_scale = 0.01; // Scale factor for texture tiling
    let uv_xy = world_pos.xy * texture_scale;
    let uv_xz = world_pos.xz * texture_scale; 
    let uv_yz = world_pos.yz * texture_scale;
    
    // Calculate triplanar blending weights based on normal
    let triplanar_weights = abs(world_normal);
    let triplanar_weights_normalized = triplanar_weights / (triplanar_weights.x + triplanar_weights.y + triplanar_weights.z);
    
    // Height-based texture blending
    let height = world_pos.y;
    
    // Sample both terrain textures with triplanar mapping
    let rocky_color = sample_triplanar_texture(
        rocky_texture, rocky_sampler, 
        uv_xy, uv_xz, uv_yz, 
        triplanar_weights_normalized
    );
    
    let rocky_02_color = sample_triplanar_texture(
        rocky_texture_02, rocky_sampler_02, 
        uv_xy, uv_xz, uv_yz, 
        triplanar_weights_normalized
    );
    
    // Calculate elevation-based shadow shading using surface normal
    let light_factor = max(0.0, dot(world_normal, -normalized_light));
    
    // Create shadow factor: areas facing away from light are darker
    // Base shadow level (ambient) + diffuse contribution
    let ambient_level = 0.2; // Reduced minimum light level for deeper shadows
    let diffuse_strength = 0.8; // Increased strength of directional lighting
    let shadow_factor = ambient_level + (diffuse_strength * light_factor);
    
    // Blend textures based on height (similar to terrain generation logic)
    // rocky_terrain_02 for lower areas, rocky_terrain for higher areas
    let height_blend_threshold = 36.0; // Match terrain generation threshold
    let blend_factor = smoothstep(20.0, height_blend_threshold, height);
    
    let base_color = mix(rocky_02_color, rocky_color, blend_factor);
    
    // Apply elevation-based shadows to final color
    return vec4<f32>(base_color.rgb * shadow_factor, base_color.a);
}