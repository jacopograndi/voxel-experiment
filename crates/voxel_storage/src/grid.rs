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
