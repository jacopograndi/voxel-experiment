use bevy::app::{App, Plugin};
use bevy::utils::HashMap;
use universe::Universe;
use lazy_static::lazy_static;

pub mod block;
pub mod grid;
pub mod universe;

pub const CHUNK_SIDE: usize = 32;
pub const CHUNK_AREA: usize = CHUNK_SIDE * CHUNK_SIDE;
pub const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_SIDE;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum BlockID {
    AIR,
    STONE,
    GRASS,
    LOG
}

// Enum containing the bit index of each block flag in human readable form
#[derive(Copy, Clone)]
pub enum BlockFlag {
    SOLID,

}

// HashMap containing a description for all default flags by block ID --> Is there a cleaner initialization method than this??
lazy_static! {
    static ref BLOCK_FLAGS: HashMap<BlockID, Vec<BlockFlag>> = {
        let mut map = HashMap::new();
        map.insert(BlockID::AIR, vec![]);
        map.insert(BlockID::STONE, vec![BlockFlag::SOLID]);
        map.insert(BlockID::GRASS, vec![]);
        map.insert(BlockID::LOG, vec![BlockFlag::SOLID]);
        map
    };
}


pub struct VoxelStoragePlugin;

impl Plugin for VoxelStoragePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Universe::default());
    }
}
