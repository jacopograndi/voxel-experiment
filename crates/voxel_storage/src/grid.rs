use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};

use crate::{CHUNK_AREA, CHUNK_SIDE, CHUNK_VOLUME};

#[repr(C)]
#[derive(Debug, Clone, Pod, Zeroable, Copy, Default, PartialEq, Eq)]
pub struct Voxel {
    pub id: u8,
    pub flags: u8,
    pub light: u8,
    pub unused: u8,
}

/// Cubic section of the voxel world with the cube side = CHUNK_SIDE
#[derive(Debug, Clone)]
pub struct Grid {
    pub voxels: [Voxel; CHUNK_VOLUME],
}

impl Grid {
    pub fn to_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.voxels)
    }

    pub fn empty() -> Self {
        Self {
            voxels: [Voxel::default(); CHUNK_VOLUME],
        }
    }

    pub fn xyz_to_index(xyz: IVec3) -> usize {
        xyz.x as usize * CHUNK_AREA + xyz.y as usize * CHUNK_SIDE + xyz.z as usize
    }

    pub fn index_to_xyz(index: usize) -> IVec3 {
        let layer = index / CHUNK_SIDE;
        IVec3 {
            x: (layer / CHUNK_SIDE) as i32,
            y: (layer % CHUNK_SIDE) as i32,
            z: (index % CHUNK_SIDE) as i32,
        }
    }

    pub fn filled() -> Grid {
        let voxel = Voxel {
            id: 1,
            flags: 16,
            ..Default::default()
        };
        Self {
            voxels: [voxel; CHUNK_VOLUME],
        }
    }

    pub fn flatland() -> Grid {
        let mut grid = Grid::empty();
        for i in 0..CHUNK_VOLUME {
            let xyz = Self::index_to_xyz(i);
            if xyz.y > (CHUNK_SIDE / 2) as i32 {
                grid.voxels[i].id = 0;
                grid.voxels[i].flags = 0;
            } else {
                grid.voxels[i].id = 1;
                grid.voxels[i].flags = 16;
            }
        }
        grid
    }

    pub fn get_at(&self, xyz: IVec3) -> Voxel {
        self.voxels[Self::xyz_to_index(xyz)]
    }

    pub fn set_at(&mut self, xyz: IVec3, voxel: Voxel) {
        self.voxels[Self::xyz_to_index(xyz)] = voxel;
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
    pub voxels: Vec<Voxel>,
    pub size: IVec3,
    pub palette: Palette,
}

impl VoxGrid {
    pub fn new(size: IVec3) -> VoxGrid {
        let volume = size.x * size.y * size.z;
        Self {
            voxels: vec![Voxel::default(); volume as usize],
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
        let size = UVec3::new(size.x, size.y, size.z).as_ivec3();
        let mut grid = VoxGrid::new(size);

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
            grid.palette[i] = material.into();
        }

        for voxel in &vox.models[0].voxels {
            let pos = IVec3::new(
                size.x as i32 - 1 - voxel.x as i32,
                voxel.z as i32,
                voxel.y as i32,
            );
            let index = pos.x * grid.size.y * grid.size.z + pos.y * grid.size.z + pos.z;
            grid.voxels[index as usize].id = voxel.i;
            grid.voxels[index as usize].flags = 16; // set the collision flag
        }

        Ok(grid)
    }
}

#[cfg(test)]
mod test {
    use bevy::math::IVec3;

    use crate::grid::Grid;
    #[test]
    fn xyz_to_index_to_xyz() {
        for x in 0..32 {
            for y in 0..32 {
                for z in 0..32 {
                    let xyz0 = IVec3 { x, y, z };
                    let index = Grid::xyz_to_index(xyz0.clone());
                    let xyz1 = Grid::index_to_xyz(index);
                    assert_eq!(xyz0, xyz1);
                }
            }
        }
    }
}
