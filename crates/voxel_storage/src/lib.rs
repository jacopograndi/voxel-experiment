use bevy::app::{App, Plugin};
use bevy::utils::HashMap;
use universe::Universe;
use lazy_static::lazy_static;

use::voxel_flag_bank::BlockFlag;

pub mod block;
pub mod chunk;
pub mod universe;

pub const CHUNK_SIDE: usize = 32;
pub const CHUNK_AREA: usize = CHUNK_SIDE * CHUNK_SIDE;
pub const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_SIDE;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BlockId {
    AIR,
    STONE,
    GRASS,
    LOG
}

impl Default for BlockId {
    fn default() -> Self { BlockId::AIR }
}

// HashMap containing a description for all default flags by block ID --> Is there a cleaner initialization method than this??
lazy_static! {
    static ref BLOCK_FLAGS: HashMap<BlockId, Vec<BlockFlag>> = {
        let mut map = HashMap::new();
        map.insert(BlockId::AIR, vec![]);
        map.insert(BlockId::STONE, vec![BlockFlag::SOLID, BlockFlag::OPAQUE]);
        map.insert(BlockId::GRASS, vec![]);
        map.insert(BlockId::LOG, vec![BlockFlag::SOLID, BlockFlag::OPAQUE]);
        map
    };
}


pub struct VoxelStoragePlugin;

impl Plugin for VoxelStoragePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Universe::default());
    }
}
