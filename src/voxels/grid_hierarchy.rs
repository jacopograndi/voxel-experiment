use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct Grid {
    pub size: u32,
    pub voxels: Vec<u8>,
    pub palette: Palette,
}

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Palette([[f32; 4]; 256]);

impl Grid {
    pub fn empty(size: u32) -> Self {
        Self {
            size,
            voxels: vec![0; (size * size * size * 4) as usize],
            palette: Palette([[0.0; 4]; 256]),
        }
    }

    pub fn filled(size: u32) -> Grid {
        let mut gh = Grid::empty(size as u32);
        gh.palette[0] = [0.0, 0.0, 0.0, 0.0];
        gh.palette[1] = [1.0, 1.0, 1.0, 1.0];
        gh.palette[2] = [1.0, 0.0, 0.0, 1.0];
        for z in 0..size {
            for y in 0..size {
                for x in 0..size {
                    let index = (x * size * size + y * size + z) as usize;
                        gh.voxels[index * 4] = 1;
                        gh.voxels[index * 4 + 1] = 16;
                }
            }
        }
        gh
    }

    pub fn flatland(size: u32) -> Grid {
        let mut gh = Grid::empty(size as u32);
        gh.palette[0] = [0.0, 0.0, 0.0, 0.0];
        gh.palette[1] = [1.0, 1.0, 1.0, 1.0];
        gh.palette[2] = [1.0, 0.0, 0.0, 1.0];
        for z in 0..size {
            for y in 0..size {
                for x in 0..size {
                    let index = (x * size * size + y * size + z) as usize;
                    if y > size / 2 {
                        gh.voxels[index * 4] = 0;
                        gh.voxels[index * 4 + 1] = 0;
                    } else {
                        gh.voxels[index * 4] = 1;
                        gh.voxels[index * 4 + 1] = 16;
                    }
                }
            }
        }
        gh
    }

    pub fn get_at(&self, pos: IVec3) -> u32 {
        let size = self.size as i32;
        let index = (pos.x * size * size + pos.y * size + pos.z) as usize;
        let id = self.voxels[index * 4] as u32;
        let flags = self.voxels[index * 4 + 1] as u32;
        //j todo: this drops two bytes
        (id << 8) + flags
    }

    pub fn get_buffer_u8_size(&self) -> u32 {
        self.size * self.size * self.size * 4
    }

    pub fn contains(&self, pos: &IVec3) -> bool {
        let range = 0..self.size as i32;
        range.contains(&pos.x) && range.contains(&pos.y) && range.contains(&pos.z)
    }

    pub fn from_vox(file: &[u8]) -> Result<Grid, String> {
        let vox = dot_vox::load_bytes(file)?;
        let size = vox.models[0].size;
        if size.x != size.y || size.x != size.z || size.y != size.z {
            return Err("Voxel model is not a cube!".to_string());
        }

        let size = size.x as usize;

        let mut gh = Grid::empty(size as u32);
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
            gh.voxels[index as usize * 2] = voxel.i;
            gh.voxels[index as usize * 2 + 1] = 16; // set the collision flag
        }

        Ok(gh)
    }
}
