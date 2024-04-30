use bevy::{prelude::*, utils::HashSet};
use mcrs_universe::CHUNK_SIDE;

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
