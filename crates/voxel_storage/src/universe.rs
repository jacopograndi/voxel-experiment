use crate::{
    grid::{Grid, Voxel},
    CHUNK_AREA, CHUNK_SIDE,
};
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use bevy::{prelude::*, utils::HashMap};

#[derive(Debug, Clone)]
pub struct GridPtr(pub Arc<RwLock<Grid>>);

#[derive(Debug, Clone)]
pub struct Chunk {
    pub grid: GridPtr,
    pub updated: bool,
}

impl Chunk {
    pub fn set_dirty(&mut self) {
        self.updated = true;
    }
    pub fn reset_dirty(&mut self) {
        self.updated = false;
    }
}

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
    pub fn lock_write(&mut self) -> impl Iterator<Item = (IVec3, RwLockWriteGuard<Grid>)> {
        self.chunks
            .iter()
            .map(|(pos, chunk)| (pos.clone(), chunk.grid.0.write().unwrap()))
    }

    pub fn pos_to_chunk_and_inner(&self, pos: &IVec3) -> (IVec3, IVec3) {
        let chunk_size = IVec3::splat(CHUNK_SIDE as i32);
        let chunk_pos = (pos.div_euclid(chunk_size)) * chunk_size;
        let inner_pos = pos.rem_euclid(chunk_size);
        (chunk_pos, inner_pos)
    }

    pub fn get_at(&self, pos: &IVec3) -> Option<Voxel> {
        let (chunk_pos, inner_pos) = self.pos_to_chunk_and_inner(pos);
        self.chunks
            .get(&chunk_pos)
            .map(|chunk| chunk.grid.0.read().unwrap().get_at(inner_pos))
    }

    pub fn set_at(&mut self, pos: &IVec3, voxel: Voxel) {
        let (chunk_pos, inner_pos) = self.pos_to_chunk_and_inner(pos);
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.grid.0.write().unwrap().set_at(inner_pos, voxel);
            chunk.set_dirty();
        }
    }
}
