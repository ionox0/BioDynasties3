use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
    pbr::Material,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SimpleMaterial {
    // Simple working textures - JPG format only for stability
    #[texture(0)]
    #[sampler(1, sampler_type = "filtering")]
    pub grass_texture: Handle<Image>,
    #[texture(2)]
    #[sampler(3, sampler_type = "filtering")]
    pub dirt_texture: Handle<Image>,
    // Light direction for elevation-based shadow shading
    #[uniform(4)]
    pub light_direction: Vec3,
}

impl Material for SimpleMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/triplanar.wgsl".into()
    }
}

/// Plugin to register simple material
pub struct SimpleMaterialPlugin;

impl Plugin for SimpleMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<SimpleMaterial>::default());
    }
}