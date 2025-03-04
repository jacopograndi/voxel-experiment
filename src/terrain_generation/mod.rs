pub mod generator;
pub mod generators;
pub mod plugin;
pub mod requested;
pub mod sun_beams;

use bevy::prelude::*;
use mcrs_universe::CHUNK_SIDE;

pub fn get_spawn_chunks() -> impl Iterator<Item = IVec3> {
    (-1..=1)
        .map(|z| {
            (-1..=1)
                .map(move |y| (-1..=1).map(move |x| IVec3::new(x, y, z) * CHUNK_SIDE as i32))
                .flatten()
        })
        .flatten()
}

pub fn get_sun_heightfield(_xz: IVec2) -> i32 {
    256
}
