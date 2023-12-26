use bevy::prelude::*;
use std::sync::{Arc, RwLock};

use crate::{
    CHUNK_AREA, CHUNK_SIDE, CHUNK_VOLUME,
    block::Block,
    BlockFlag, BlockID
};

#[derive(Debug, Clone)]
pub struct Chunk {
    _blocks: Arc<RwLock<[Block; CHUNK_VOLUME]>>,
    pub version: u32,
}

impl Chunk {

    pub fn clone_blocks(&self) -> [Block; CHUNK_VOLUME] {
        self._blocks.read().unwrap().clone()
    }

    pub fn empty() -> Self {
        Self {
            _blocks: Arc::new(RwLock::new([Block::default(); CHUNK_VOLUME])),
            version: 0
        }
    }

    fn _xyz2idx(xyz: IVec3) -> usize {
        xyz.x as usize * CHUNK_AREA + xyz.y as usize * CHUNK_SIDE + xyz.z as usize
    }

    fn _idx2xyz(index: usize) -> IVec3 {
        let layer = index / CHUNK_SIDE;
        IVec3 {
            x: (layer / CHUNK_SIDE) as i32,
            y: (layer % CHUNK_SIDE) as i32,
            z: (index % CHUNK_SIDE) as i32,
        }
    }

    pub fn filled() -> Self {
        let block = Block::new(BlockID::STONE);
        Self {
            _blocks: Arc::new(RwLock::new([block; CHUNK_VOLUME])),
            version: 0
        }
    }

    pub fn flatland() -> Self {
        let chunk = Self::empty();
        for i in 0..CHUNK_VOLUME {
            let xyz = Self::_idx2xyz(i);
            if xyz.y > (CHUNK_SIDE / 2) as i32 {
                chunk.set_block(xyz, BlockID::AIR);
            } else {
                chunk.set_block(xyz, BlockID::STONE);
            }
        }
        chunk
    }

    pub fn set_block(&self, xyz: IVec3, id: BlockID) {
        self._blocks.write().unwrap()[Self::_xyz2idx(xyz)] = Block::new(id);
    }

    pub fn read_block(&self, xyz: IVec3) -> Block {
        self._blocks.read().unwrap()[Self::_xyz2idx(xyz)]
    }

    pub fn contains(xyz: &IVec3) -> bool {
        let range = 0..CHUNK_SIDE as i32;
        range.contains(&xyz.x) && range.contains(&xyz.y) && range.contains(&xyz.z)
    }
}

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Palette([Color; 256]);

/// Voxel model with variable size
#[derive(Debug, Clone)]
pub struct VoxGrid {
    pub voxels: Vec<Block>,
    pub size: IVec3,
    pub palette: Palette,
}

impl VoxGrid {
    pub fn new(size: IVec3) -> VoxGrid {
        let volume = size.x * size.y * size.z;
        Self {
            voxels: vec![Block::default(); volume as usize],
            size,
            palette: Palette([Color::WHITE; 256]),
        }
    }

    pub fn to_bytes_vec(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend(self.size.x.to_le_bytes());
        bytes.extend(self.size.y.to_le_bytes());
        bytes.extend(self.size.z.to_le_bytes());
        bytes.extend((0_u32).to_le_bytes());
        bytes.extend(
            self.palette
                .iter()
                .map(|col| col.as_rgba_u8())
                .flatten()
                .collect::<Vec<u8>>(),
        );
        bytes.extend(bytemuck::cast_slice(&self.voxels));
        bytes
    }

    pub fn from_vox(file: &[u8]) -> Result<VoxGrid, String> {
        let vox = dot_vox::load_bytes(file)?;

        let size = vox.models[0].size;
        let size = UVec3::new(size.y, size.z, size.x).as_ivec3();
        let mut grid = VoxGrid::new(size);

        println!("{:?}", vox.palette);

        if vox.palette.len() > 255 {
            panic!("The zeroeth color is used for transparency");
        }

        for i in 0..vox.palette.len() {
            let colour = vox.palette[i];
            let mut material = Vec4::new(
                colour.r as f32 / 255.0,
                colour.g as f32 / 255.0,
                colour.b as f32 / 255.0,
                0.0,
            );
            material = material.powf(2.2);
            if let Some(vox_material) = vox.materials.get(i) {
                let vox_material = vox_material.properties.clone();
                if vox_material["_type"] == "_emit" {
                    material *= 1.0 + vox_material["_emit"].parse::<f32>().unwrap();
                    if vox_material.contains_key("_flux") {
                        material = material.powf(vox_material["_flux"].parse::<f32>().unwrap());
                    }
                    material.w = 1.0;
                }
            }
            grid.palette[i + 1] = material.into();
        }

        for voxel in &vox.models[0].voxels {
            let pos = IVec3::new(
                size.x as i32 - 1 - voxel.y as i32,
                voxel.z as i32,
                voxel.x as i32,
            );
            let index = pos.x * grid.size.y * grid.size.z + pos.y * grid.size.z + pos.z;
            grid.voxels[index as usize].id = voxel.i + 1;
            grid.voxels[index as usize].set_flag(BlockFlag::SOLID); // set the collision flag
        }

        Ok(grid)
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
