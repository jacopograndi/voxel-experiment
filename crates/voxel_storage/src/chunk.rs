use bevy::prelude::*;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use crate::{
    CHUNK_AREA, CHUNK_SIDE, CHUNK_VOLUME,
    block::{Block, LightType},
    BlockId
};

use voxel_flag_bank::flagbank::FlagBank;

#[derive(Debug, Clone)]
pub struct Chunk {
    _blocks: Arc<RwLock<[Block; CHUNK_VOLUME]>>,
    pub version: u32,
    pub properties: FlagBank,
}

impl Chunk {

    pub fn get_w_ref(&self) -> RwLockWriteGuard<[Block; CHUNK_VOLUME]> {
        self._blocks.write().unwrap()
    }

    pub fn empty() -> Self {
        Self {
            _blocks: Arc::new(RwLock::new([Block::default(); CHUNK_VOLUME])),
            version: 0,
            properties: FlagBank::empty()
        }
    }

    pub fn filled() -> Self {
        let block = Block::new(BlockId::STONE);
        Self {
            _blocks: Arc::new(RwLock::new([block; CHUNK_VOLUME])),
            version: 0,
            properties: FlagBank::empty()
        }
    }

    pub fn flatland() -> Self {
        let chunk = Self::empty();
        for i in 0..CHUNK_VOLUME {
            let xyz = Self::_idx2xyz(i);
            if xyz.y > (CHUNK_SIDE / 2) as i32 {
                chunk.set_block(xyz, BlockId::AIR);
            } else {
                chunk.set_block(xyz, BlockId::STONE);
            }
        }
        chunk
    }

    pub fn clone_blocks(&self) -> [Block; CHUNK_VOLUME] {
        self._blocks.read().unwrap().clone()
    }

    pub fn set_block(&self, xyz: IVec3, id: BlockId) {
        self._blocks.write().unwrap()[Self::_xyz2idx(xyz)] = Block::new(id);
    }

    pub fn set_entire_block(&self, xyz: IVec3, block: Block) {
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
