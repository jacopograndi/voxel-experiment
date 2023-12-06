use bevy::math::{IVec3, Vec3};

use super::{grid_hierarchy::Grid, voxel_world::ChunkMap};

const RAYCAST_MAX_ITERATIONS: u32 = 1000;

// this code is pretty bad
// i think there is a bug based on direction
// needs to be fast and correct
// -> a lot of generated tests
// -> some benchmarks on different Grid structs
/// http://www.cs.yorku.ca/~amana/research/grid.pdf
pub fn raycast(start: Vec3, direction: Vec3, chunk_map: &ChunkMap) -> Option<(IVec3, IVec3, f32)> {
    if direction.length_squared() == 0. {
        return None;
    }
    assert!((0.999..1.001).contains(&direction.length_squared()));
    let mut grid_pos = IVec3::new(start.x as i32, start.y as i32, start.z as i32);
    let mut step = IVec3::ZERO;
    let mut sidedist = Vec3::ZERO;

    let deltadist = (1. / direction).abs();
    if direction.x < 0. {
        step.x = -1;
        sidedist.x = (start.x - grid_pos.x as f32) * deltadist.x;
    } else {
        step.x = 1;
        sidedist.x = ((grid_pos.x as f32 + 1.) - start.x) * deltadist.x;
    }
    if direction.y < 0. {
        step.y = -1;
        sidedist.y = (start.y - grid_pos.y as f32) * deltadist.y;
    } else {
        step.y = 1;
        sidedist.y = ((grid_pos.y as f32 + 1.) - start.y) * deltadist.y;
    }
    if direction.z < 0. {
        step.z = -1;
        sidedist.z = (start.z - grid_pos.z as f32) * deltadist.z;
    } else {
        step.z = 1;
        sidedist.z = ((grid_pos.z as f32 + 1.) - start.z) * deltadist.z;
    }
    enum Side {
        X,
        Y,
        Z,
    }
    let mut side;
    for _i in 0..RAYCAST_MAX_ITERATIONS {
        if sidedist.x < sidedist.y {
            if sidedist.x < sidedist.z {
                sidedist.x += deltadist.x;
                grid_pos.x += step.x;
                side = Side::X;
            } else {
                sidedist.z += deltadist.z;
                grid_pos.z += step.z;
                side = Side::Z;
            }
        } else {
            if sidedist.y < sidedist.z {
                sidedist.y += deltadist.y;
                grid_pos.y += step.y;
                side = Side::Y;
            } else {
                sidedist.z += deltadist.z;
                grid_pos.z += step.z;
                side = Side::Z;
            }
        }
        let chunk_pos = (grid_pos / 32) * 32;
        let voxel_pos = grid_pos % 32;
        if let Some(chunk) = chunk_map.chunks.get(&chunk_pos) {
            let grid = chunk.grid.0.read().unwrap();
            if grid.contains(&voxel_pos) {
                if grid.get_at(voxel_pos) & 16 == 16 {
                    let dist = match side {
                        Side::X => sidedist.x - deltadist.x,
                        Side::Y => sidedist.y - deltadist.y,
                        Side::Z => sidedist.z - deltadist.z,
                    };
                    let norm = match side {
                        Side::X => -IVec3::X * step.x,
                        Side::Y => -IVec3::Y * step.y,
                        Side::Z => -IVec3::Z * step.z,
                    };
                    return Some((grid_pos, norm, dist.abs()));
                }
            }
        }
    }
    //println!("out of raycast iterations");
    None
}
