pub mod chunk_map;
pub mod grid;

pub const CHUNK_SIDE: usize = 32;
pub const CHUNK_AREA: usize = CHUNK_SIDE * CHUNK_SIDE;
pub const CHUNK_VOLUME: usize = CHUNK_SIDE * CHUNK_SIDE * CHUNK_SIDE;
