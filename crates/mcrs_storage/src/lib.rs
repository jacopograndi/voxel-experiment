use bevy::app::{App, Plugin};
use universe::Universe;

pub mod block;
pub mod chunk;
pub mod universe;

pub const CHUNK_SIDE: usize = 32;
pub const CHUNK_AREA: usize = CHUNK_SIDE * CHUNK_SIDE;
pub const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_SIDE;

pub struct McrsStoragePlugin;

impl Plugin for McrsStoragePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Universe::default());
    }
}
