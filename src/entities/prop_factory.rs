//! Factory for spawning decorative environmental props.
//!
//! Call `PropFactory::spawn_prop` to place any `PropType` at a world position.

use bevy::prelude::*;

// ─── Prop configuration ───────────────────────────────────────────────────────

const MUSHROOMS_SCALE: f32 = 60.0;
const SMALL_ROCKS_SCALE: f32 = 37.5;
const TERMITE_MOUND_SMALL_SCALE: f32 = 0.75;
const WOOD_STICK_SCALE: f32 = 7.5;

/// Marker component for all environmental props.
#[derive(Component)]
pub struct EnvProp;

/// The four supported decorative prop types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropType {
    Mushrooms,
    SmallRocks,
    TermiteMoundSmall,
    WoodStick,
}

pub struct PropFactory;

impl PropFactory {
    pub fn spawn_prop(commands: &mut Commands, asset_server: &AssetServer, prop_type: PropType, position: Vec3) {
        let path = prop_model_path(prop_type);
        let scale = prop_model_scale(prop_type);
        commands.spawn((
            EnvProp,
            SceneRoot(asset_server.load(path)),
            Transform::from_translation(position).with_scale(Vec3::splat(scale)),
        ));
    }
}

fn prop_model_path(prop_type: PropType) -> &'static str {
    match prop_type {
        PropType::Mushrooms => "models/objects/good/mushrooms.glb#Scene0",
        PropType::SmallRocks => "models/objects/good/small_rocks.glb#Scene0",
        PropType::TermiteMoundSmall => "models/objects/good/termite_mound_small.glb#Scene0",
        PropType::WoodStick => "models/objects/good/wood_stick.glb#Scene0",
    }
}

fn prop_model_scale(prop_type: PropType) -> f32 {
    match prop_type {
        PropType::Mushrooms => MUSHROOMS_SCALE,
        PropType::SmallRocks => SMALL_ROCKS_SCALE,
        PropType::TermiteMoundSmall => TERMITE_MOUND_SMALL_SCALE,
        PropType::WoodStick => WOOD_STICK_SCALE,
    }
}
