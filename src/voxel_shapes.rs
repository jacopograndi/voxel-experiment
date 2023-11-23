use bevy::{prelude::*, utils::HashMap};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub enum VoxelShape {
    FilledCuboid(IVec3),
    Sphere(u32),
    Random { size: IVec3, seed: u64 },
}
impl Default for VoxelShape {
    fn default() -> Self {
        Self::opts().into_iter().next().unwrap()
    }
}
impl VoxelShape {
    pub fn opts() -> impl IntoIterator<Item = Self> {
        [
            Self::FilledCuboid(IVec3::splat(16)),
            Self::FilledCuboid(IVec3::splat(32)),
            Self::FilledCuboid(IVec3::splat(64)),
            Self::FilledCuboid(IVec3::splat(128)),
            Self::FilledCuboid(IVec3::splat(256)),
            Self::Sphere(8),
            Self::Sphere(32),
            Self::Sphere(128),
            Self::Random {
                size: IVec3::splat(256),
                seed: 69,
            },
        ]
        .into_iter()
    }
    pub fn iter(&self) -> impl Iterator<Item = IVec3> {
        let mut vec: Vec<IVec3> = vec![];
        match self {
            Self::FilledCuboid(size) => {
                for x in 0..size.x {
                    for y in 0..size.y {
                        for z in 0..size.z {
                            vec.push(IVec3::new(x, y, z));
                        }
                    }
                }
            }
            Self::Sphere(radius) => {
                let size = IVec3::splat(*radius as i32 * 2);
                let center = Vec3::splat(*radius as f32);
                let rad2 = *radius as f32 * *radius as f32;
                for x in 0..size.x {
                    for y in 0..size.y {
                        for z in 0..size.z {
                            if (Vec3::new(x as f32, y as f32, z as f32) - center).length_squared()
                                < rad2
                            {
                                vec.push(IVec3::new(x, y, z));
                            }
                        }
                    }
                }
            }
            Self::Random { size, seed } => {
                let mut rng = ChaCha8Rng::seed_from_u64(*seed);
                for x in 0..size.x {
                    for y in 0..size.y {
                        for z in 0..size.z {
                            if rng.gen_bool(0.5) {
                                vec.push(IVec3::new(x, y, z))
                            }
                        }
                    }
                }
            }
        }
        vec.into_iter()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BoolVoxel(bool);

impl Default for BoolVoxel {
    fn default() -> Self {
        EMPTY
    }
}

pub const EMPTY: BoolVoxel = BoolVoxel(false);
pub const FILLED: BoolVoxel = BoolVoxel(true);

impl block_mesh::Voxel for BoolVoxel {
    fn get_visibility(&self) -> block_mesh::VoxelVisibility {
        if *self == EMPTY {
            block_mesh::VoxelVisibility::Empty
        } else {
            block_mesh::VoxelVisibility::Opaque
        }
    }
}

impl block_mesh::MergeVoxel for BoolVoxel {
    type MergeValue = Self;

    fn merge_value(&self) -> Self::MergeValue {
        *self
    }
}

pub const CHUNK_SIDE: u32 = 16;
pub const CHUNK_SIDE_PADDED: u32 = CHUNK_SIDE + 2;
pub const CHUNK_AREA: u32 = CHUNK_SIDE * CHUNK_SIDE;
pub const CHUNK_VOLUME: u32 = CHUNK_SIDE * CHUNK_SIDE * CHUNK_SIDE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk(pub [BoolVoxel; CHUNK_VOLUME as usize]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    pub chunks: HashMap<IVec3, Chunk>,
}

impl Grid {
    pub fn from_vec(vec: Vec<IVec3>) -> Self {
        let mut grid = Self {
            chunks: HashMap::new(),
        };
        for pos in vec.iter() {
            let chunk_pos = *pos / 16;
            let chunk = grid
                .chunks
                .entry(chunk_pos)
                .or_insert(Chunk([EMPTY; CHUNK_VOLUME as usize]));
            let chunk_offset = pos.rem_euclid(IVec3::splat(16));
            let i = chunk_offset.x
                + chunk_offset.y * CHUNK_SIDE as i32
                + chunk_offset.z * CHUNK_AREA as i32;
            chunk.0[i as usize] = FILLED;
        }
        grid
    }
    pub fn get_at(&self, pos: IVec3) -> Option<BoolVoxel> {
        let chunk_pos = pos / 16;
        if let Some(chunk) = self.chunks.get(&chunk_pos) {
            let chunk_offset = pos.rem_euclid(IVec3::splat(16));
            let i = chunk_offset.x
                + chunk_offset.y * CHUNK_SIDE as i32
                + chunk_offset.z * CHUNK_AREA as i32;
            Some(chunk.0[i as usize])
        } else {
            None
        }
    }
}
