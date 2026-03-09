use bevy::prelude::*;

// Owned by: ScenePlugin (setup_scene)
#[derive(Component)]
pub struct MainCamera;

// Owned by: ScenePlugin (handle_rts_camera_input)
#[derive(Component, Debug, Clone)]
pub struct RTSCamera {
    pub move_speed: f32,
}
