use crate::{
    block::Block,
    chunk::Chunk,
    CHUNK_SIDE,
};

use bevy::{prelude::*, render::extract_resource::ExtractResource, utils::HashMap};

/// Game resource, it's mutations are propagated to `RenderUniverse`
/// and written to the gpu buffer.
#[derive(Resource, ExtractResource, Debug, Clone, Default)]
pub struct Universe {
    pub chunks: HashMap<IVec3, Chunk>,
}

impl Universe {
    pub fn pos_to_chunk_and_inner(&self, pos: &IVec3) -> (IVec3, IVec3) {
        let chunk_size = IVec3::splat(CHUNK_SIDE as i32);
        let chunk_pos = (pos.div_euclid(chunk_size)) * chunk_size;
        let inner_pos = pos.rem_euclid(chunk_size);
        (chunk_pos, inner_pos)
    }

    pub fn get_at(&self, pos: &IVec3) -> Option<Block> {
        let (chunk_pos, inner_pos) = self.pos_to_chunk_and_inner(pos);
        self.chunks
            .get(&chunk_pos)
            .map(|chunk| chunk.grid.0.read().unwrap().get_at(inner_pos))
    }

    pub fn set_at(&mut self, pos: &IVec3, voxel: Block) {
        let (chunk_pos, inner_pos) = self.pos_to_chunk_and_inner(pos);
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.grid.0.write().unwrap().set_at(inner_pos, voxel);
            chunk.version = chunk.version.wrapping_add(1);
        }
    }
}
