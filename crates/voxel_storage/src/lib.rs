use bevy::app::{App, Plugin};
use bevy::utils::HashMap;
use universe::Universe;
use lazy_static::lazy_static;

use voxel_info::{BlockInfo, get_block_info};

pub mod block;
pub mod chunk;
pub mod universe;

pub const CHUNK_SIDE: usize = 32;
pub const CHUNK_AREA: usize = CHUNK_SIDE * CHUNK_SIDE;
pub const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_SIDE;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BlockType {
    Air,
    Stone,
    Path,
    Dirt,
    Cobblestone,
    Wood
}

impl Default for BlockType {
    fn default() -> Self { BlockType::Air }
}

lazy_static! {
    #[derive(Debug)]
    static ref BLOCK_INFO: HashMap<u8, BlockInfo> = get_block_info();
}

pub struct VoxelStoragePlugin;

impl Plugin for VoxelStoragePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Universe::default());
    }
}
