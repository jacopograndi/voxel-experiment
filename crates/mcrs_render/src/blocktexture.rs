use bevy::prelude::*;

use mcrs_flag_bank::BlockFlag;
use mcrs_storage::block::Block;

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Palette([Color; 256]);

/// Voxel model with variable size
#[derive(Debug, Clone)]
pub struct BlockTexture {
    pub voxels: Vec<Block>,
    pub size: IVec3,
    pub palette: Palette,
}

impl BlockTexture {
    pub fn new(size: IVec3) -> BlockTexture {
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

    pub fn from_vox(file: &[u8]) -> Result<BlockTexture, String> {
        let vox = dot_vox::load_bytes(file)?;

        let size = vox.models[0].size;
        let size = UVec3::new(size.y, size.z, size.x).as_ivec3();
        let mut grid = BlockTexture::new(size);

        for i in 0..vox.palette.len().min(255) {
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
            grid.voxels[index as usize].set_id(voxel.i + 1); // TODO: UNSAFE changing an id outside of BlockInfo
            grid.voxels[index as usize]
                .properties
                .set(BlockFlag::Collidable); // set the collision flag
        }

        Ok(grid)
    }
}
