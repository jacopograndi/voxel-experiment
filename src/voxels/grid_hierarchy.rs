use bevy::{prelude::*, utils::HashSet};

#[derive(Clone)]
pub struct GridHierarchy {
    pub levels: [u32; 8],
    pub texture_size: u32,
    pub texture_data: Vec<u8>,
    pub palette: Palette,
}

#[derive(Clone, Deref, DerefMut)]
pub struct Palette([[f32; 4]; 256]);

impl GridHierarchy {
    pub fn empty(texture_size: u32) -> Self {
        let mut levels = [0; 8];
        let i = texture_size.trailing_zeros() - 3;
        for i in 0..i {
            levels[i as usize] = 1 << (i + 3);
        }

        Self {
            levels,
            texture_size,
            texture_data: vec![0; (texture_size * texture_size * texture_size * 2) as usize],
            palette: Palette([[0.0; 4]; 256]),
        }
    }

    pub fn get_offsets(&self) -> [u32; 8] {
        let mut offsets = [0; 8];
        let mut last = 0;
        for i in 0..8 {
            offsets[i] = last;
            last = last + self.levels[i] as u32 * self.levels[i] as u32 * self.levels[i] as u32;
        }
        offsets
    }

    pub fn get_buffer_size_from_levels(levels: &[u32; 8]) -> usize {
        let mut length = 0;
        for i in 0..8 {
            length += levels[i] * levels[i] * levels[i];
        }
        length as usize / 8
    }

    pub fn get_buffer_size(&self) -> usize {
        Self::get_buffer_size_from_levels(&self.levels)
    }

    pub fn flatland(size: u32) -> GridHierarchy {
        let mut gh = GridHierarchy::empty(size as u32);
        gh.palette[0] = [0.0, 0.0, 0.0, 0.0];
        gh.palette[1] = [1.0, 1.0, 1.0, 1.0];
        gh.palette[2] = [1.0, 0.0, 0.0, 1.0];
        for z in 0..size {
            for y in 0..size {
                for x in 0..size {
                    let index = (x * size * size + y * size + z) as usize;
                    if y > size / 2 {
                        gh.texture_data[index * 2] = 0;
                        gh.texture_data[index * 2 + 1] = 0;
                    } else {
                        gh.texture_data[index * 2] = 1;
                        gh.texture_data[index * 2 + 1] = 16;
                    }
                }
            }
        }
        gh
    }

    pub fn get_at(&self, pos: IVec3) -> u16 {
        let size = self.texture_size as i32;
        let index = (pos.x * size * size + pos.y * size + pos.z) as usize;
        let id = self.texture_data[index * 2] as u16;
        let flags = self.texture_data[index * 2 + 1] as u16;
        (id << 8) + flags
    }

    pub fn contains(&self, pos: &IVec3) -> bool {
        let range = 0..self.texture_size as i32;
        range.contains(&pos.x) && range.contains(&pos.y) && range.contains(&pos.z)
    }

    pub fn from_vox(file: &[u8]) -> Result<GridHierarchy, String> {
        let vox = dot_vox::load_bytes(file)?;
        let size = vox.models[0].size;
        if size.x != size.y || size.x != size.z || size.y != size.z {
            return Err("Voxel model is not a cube!".to_string());
        }

        let size = size.x as usize;

        let mut gh = GridHierarchy::empty(size as u32);
        for i in 0..256 {
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

            gh.palette[i] = material.to_array();
        }

        for voxel in &vox.models[0].voxels {
            let pos = IVec3::new(
                size as i32 - 1 - voxel.x as i32,
                voxel.z as i32,
                voxel.y as i32,
            );

            let index = pos.x as usize * size * size + pos.y as usize * size + pos.z as usize;
            gh.texture_data[index as usize * 2] = voxel.i;
            gh.texture_data[index as usize * 2 + 1] = 16; // set the collision flag
        }

        Ok(gh)
    }
}
