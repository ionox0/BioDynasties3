//! Flow-field navigation for RTS terrain.
//!
//! The coarse passability grid is built once at startup by `NavigationPlugin`.
//! Each move order builds one shared flow field on a background thread (see `systems`).

pub mod flow_field;
pub mod nav_grid;
pub mod systems;

use bevy::prelude::*;
use std::sync::Arc;
use tracing::instrument;

use crate::world::static_terrain::StaticTerrainHeights;
use systems::NavGridResource;

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup_nav_grid.run_if(not(resource_exists::<NavGridResource>)));
    }
}

#[instrument(skip_all)]
fn setup_nav_grid(mut commands: Commands, terrain: Option<Res<StaticTerrainHeights>>) {
    let Some(terrain) = terrain else { return };
    let world_size = crate::core::constants::movement::MAP_BOUNDARY * 2.0;
    commands.insert_resource(NavGridResource(Arc::new(nav_grid::NavGrid::from_terrain(&terrain, world_size))));
}
