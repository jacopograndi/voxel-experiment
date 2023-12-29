use bevy::prelude::*;
use mcrs_info::BlockInfo;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{
    block::{Block, LightType},
    CHUNK_AREA, CHUNK_SIDE, CHUNK_VOLUME,
};

#[derive(Debug, Clone)]
pub struct Chunk {
    _blocks: Arc<RwLock<[Block; CHUNK_VOLUME]>>,
    pub dirty_render: bool,
    pub dirty_replication: bool,
}

impl Chunk {
    pub fn get_ref(&self) -> RwLockReadGuard<[Block; CHUNK_VOLUME]> {
        self._blocks.read().unwrap()
    }

    pub fn get_mut(&self) -> RwLockWriteGuard<[Block; CHUNK_VOLUME]> {
        self._blocks.write().unwrap()
    }

    pub fn empty() -> Self {
        Self {
            _blocks: Arc::new(RwLock::new([Block::default(); CHUNK_VOLUME])),
            dirty_render: false,
            dirty_replication: false,
        }
    }

    pub fn filled(block_info: &BlockInfo) -> Self {
        let block = Block::new(block_info);
        Self {
            _blocks: Arc::new(RwLock::new([block; CHUNK_VOLUME])),
            dirty_render: false,
            dirty_replication: false,
        }
    }

    pub fn set_block(&self, xyz: IVec3, block: Block) {
        self._blocks.write().unwrap()[Self::_xyz2idx(xyz)] = block;
    }

    pub fn set_block_light(&self, xyz: IVec3, light_type: LightType, v: u8) {
        self._blocks.write().unwrap()[Self::_xyz2idx(xyz)].set_light(light_type, v);
    }

    pub fn read_block(&self, xyz: IVec3) -> Block {
        self._blocks.read().unwrap()[Self::_xyz2idx(xyz)]
    }

    pub fn _xyz2idx(xyz: IVec3) -> usize {
        xyz.x as usize * CHUNK_AREA + xyz.y as usize * CHUNK_SIDE + xyz.z as usize
    }

    pub fn _idx2xyz(index: usize) -> IVec3 {
        let layer = index / CHUNK_SIDE;
        IVec3 {
            x: (layer / CHUNK_SIDE) as i32,
            y: (layer % CHUNK_SIDE) as i32,
            z: (index % CHUNK_SIDE) as i32,
        }
    }
}

#[cfg(test)]
mod test {
    use bevy::math::IVec3;

    use crate::chunk::Chunk;
    #[test]
    fn xyz_to_index_to_xyz() {
        for x in 0..32 {
            for y in 0..32 {
                for z in 0..32 {
                    let xyz0 = IVec3 { x, y, z };
                    let index = Chunk::_xyz2idx(xyz0.clone());
                    let xyz1 = Chunk::_idx2xyz(index);
                    assert_eq!(xyz0, xyz1);
                }
            }
        }
    }
}
