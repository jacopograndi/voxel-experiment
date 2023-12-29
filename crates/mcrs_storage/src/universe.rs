use std::sync::RwLockWriteGuard;

use crate::{block::Block, chunk::Chunk, CHUNK_SIDE, CHUNK_VOLUME};

use bevy::{prelude::*, utils::HashMap};

/// Game resource, it's mutations are propagated to `RenderUniverse`
/// and written to the gpu buffer.
#[derive(Resource, Debug, Clone, Default)]
pub struct Universe {
    pub chunks: HashMap<IVec3, Chunk>,
    /// Stores the highest block which still receives sunlight
    pub heightfield: HashMap<IVec2, i32>,
}

impl Universe {
    // maybe useful to lock everything when operating on all blocks
    pub fn lock_write(
        &mut self,
    ) -> impl Iterator<Item = (IVec3, RwLockWriteGuard<[Block; CHUNK_VOLUME]>)> {
        self.chunks
            .iter()
            .map(|(pos, chunk)| (pos.clone(), chunk.get_mut()))
    }

    pub fn pos_to_chunk_and_inner(&self, pos: &IVec3) -> (IVec3, IVec3) {
        let chunk_size = IVec3::splat(CHUNK_SIDE as i32);
        let chunk_pos = (pos.div_euclid(chunk_size)) * chunk_size;
        let inner_pos = pos.rem_euclid(chunk_size);
        (chunk_pos, inner_pos)
    }

    pub fn read_chunk_block(&self, pos: &IVec3) -> Option<Block> {
        let (chunk_pos, inner_pos) = self.pos_to_chunk_and_inner(pos);
        self.chunks
            .get(&chunk_pos)
            .map(|chunk| chunk.read_block(inner_pos))
    }

    pub fn set_chunk_block(&mut self, pos: &IVec3, block: Block) {
        let (chunk_pos, inner_pos) = self.pos_to_chunk_and_inner(pos);
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.set_block(inner_pos, block);
            chunk.dirty_render = true;
            chunk.dirty_replication = true;
        }
    }
}
