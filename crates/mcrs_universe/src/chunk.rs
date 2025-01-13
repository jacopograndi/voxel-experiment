use bevy::prelude::*;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{
    block::{Block, LightType},
    CHUNK_AREA, CHUNK_SIDE, CHUNK_VOLUME,
};

/// Cube of Blocks with side length of `CHUNK_SIDE`
#[derive(Debug, Clone)]
pub struct Chunk {
    pointer: ChunkPointer,
    pub version: ChunkVersion,
}

impl Chunk {
    pub fn iter() -> impl Iterator<Item = IVec3> {
        (0..CHUNK_VOLUME).map(Self::idx2xyz)
    }

    pub fn get_ref(&self) -> RwLockReadGuard<[Block; CHUNK_VOLUME]> {
        self.pointer.get_ref()
    }

    pub fn get_mut(&self) -> RwLockWriteGuard<[Block; CHUNK_VOLUME]> {
        // todo: maybe update version here too
        self.pointer.get_mut()
    }

    pub fn empty() -> Self {
        Self {
            pointer: ChunkPointer(Arc::new(RwLock::new([Block::default(); CHUNK_VOLUME]))),
            version: ChunkVersion::new(),
        }
    }

    pub fn set_block(&mut self, xyz: IVec3, block: Block) {
        self.pointer.get_mut()[Self::xyz2idx(xyz)] = block;
        self.version.update();
    }

    pub fn set_block_light(&mut self, xyz: IVec3, light_type: LightType, v: u8) {
        self.pointer.get_mut()[Self::xyz2idx(xyz)].set_light(light_type, v);
        self.version.update();
    }

    pub fn read_block(&self, xyz: IVec3) -> Block {
        self.pointer.get_ref()[Self::xyz2idx(xyz)]
    }

    pub fn xyz2idx(xyz: IVec3) -> usize {
        xyz.x as usize * CHUNK_AREA + xyz.y as usize * CHUNK_SIDE + xyz.z as usize
    }

    pub fn idx2xyz(index: usize) -> IVec3 {
        let layer = index / CHUNK_SIDE;
        IVec3 {
            x: (layer / CHUNK_SIDE) as i32,
            y: (layer % CHUNK_SIDE) as i32,
            z: (index % CHUNK_SIDE) as i32,
        }
    }

    pub fn contains(xyz: IVec3) -> bool {
        (0..CHUNK_SIDE as i32).contains(&xyz.x)
            && (0..CHUNK_SIDE as i32).contains(&xyz.y)
            && (0..CHUNK_SIDE as i32).contains(&xyz.z)
    }
}

/// Points to an array of Blocks in a thread-safe way
#[derive(Debug, Clone)]
pub struct ChunkPointer(Arc<RwLock<[Block; CHUNK_VOLUME]>>);
impl ChunkPointer {
    fn get_ref(&self) -> RwLockReadGuard<[Block; CHUNK_VOLUME]> {
        self.0.read().unwrap()
    }
    fn get_mut(&self) -> RwLockWriteGuard<[Block; CHUNK_VOLUME]> {
        self.0.write().unwrap()
    }
}

/// Used to tell apart a chunk from a chunk that has been modified
/// Every system that uses chunks keeps their version of the chunk and listens to chunk
/// version changes (renderer sends triangles/data to the gpu, replication sends data to clients)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChunkVersion(u64);
impl ChunkVersion {
    pub fn new() -> Self {
        Self(0)
    }
    pub fn update(&mut self) {
        self.0 += 1;
    }
}

/// Test if the index functions are correct
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
                    let index = Chunk::xyz2idx(xyz0.clone());
                    let xyz1 = Chunk::idx2xyz(index);
                    assert_eq!(xyz0, xyz1);
                }
            }
        }
    }
}
