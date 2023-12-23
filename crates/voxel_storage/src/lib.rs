use bevy::app::{App, Plugin};
use universe::Universe;

pub mod grid;
pub mod universe;

pub const CHUNK_SIDE: usize = 32;
pub const CHUNK_AREA: usize = CHUNK_SIDE * CHUNK_SIDE;
pub const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_SIDE;

pub struct VoxelStoragePlugin;

impl Plugin for VoxelStoragePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Universe::default());
    }
}
