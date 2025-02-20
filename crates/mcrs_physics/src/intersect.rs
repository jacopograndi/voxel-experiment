use bevy::{prelude::*, utils::HashSet};
use mcrs_universe::{universe::Universe, CHUNK_SIDE};

use crate::raycast::{is_block_collidable, iter_cuboid};

pub fn get_chunks_in_sphere(pos: Vec3, radius: f32) -> HashSet<IVec3> {
    let load_view_distance: u32 = radius as u32;

    let camera_chunk_pos = (pos / CHUNK_SIDE as f32).as_ivec3() * CHUNK_SIDE as i32;
    let load_view_distance_chunk = load_view_distance as i32 / CHUNK_SIDE as i32;
    let lvdc = load_view_distance_chunk;

    // sphere centered on pos
    let mut chunks = HashSet::<IVec3>::new();
    for x in -lvdc..=lvdc {
        for y in -lvdc..=lvdc {
            for z in -lvdc..=lvdc {
                let rel_map = IVec3::new(x, y, z) * CHUNK_SIDE as i32;
                let rel_center = IVec3::new(x, y, z).as_vec3() * CHUNK_SIDE as f32
                    + Vec3::ONE * CHUNK_SIDE as f32 * 0.5;
                if rel_center.length_squared() < load_view_distance.pow(2) as f32 {
                    let pos = camera_chunk_pos + rel_map;
                    chunks.insert(pos);
                }
            }
        }
    }
    chunks
}

/// True if an aabb(axis aligned bounding box) is intersecting any collidable block in the `Universe`
pub fn intersect_aabb_universe(pos: Vec3, size: Vec3, universe: &Universe) -> bool {
    let min = (pos - size * 0.5).floor().as_ivec3();
    let max = (pos + size * 0.5).floor().as_ivec3();
    iter_cuboid(min, max).any(|sample| {
        is_block_collidable(&sample, universe) && intersect_aabb_block(pos, size, sample)
    })
}

/// True if an aabb(axis aligned bounding box) is intersecting with a collidable block
pub fn intersect_aabb_block(a: Vec3, full_b: Vec3, block: IVec3) -> bool {
    intersect_aabb_aabb(a, full_b, block.as_vec3() + Vec3::splat(0.5), Vec3::ONE)
}

/// True if an aabb(axis aligned bounding box) is intersecting another aabb.
/// Each aabb is defined by a position and a full size.
pub fn intersect_aabb_aabb(a: Vec3, full_b: Vec3, e: Vec3, full_f: Vec3) -> bool {
    let b = full_b * 0.5;
    let f = full_f * 0.5;
    let check_x = (a.x - e.x).abs() < (b.x + f.x);
    let check_y = (a.y - e.y).abs() < (b.y + f.y);
    let check_z = (a.z - e.z).abs() < (b.z + f.z);
    return check_x && check_y && check_z;
}
