pub mod flagbank;

// Enum containing the bit index of each block flag in human readable form
#[derive(Copy, Clone)]
pub enum BlockFlag {
    SOLID,
    OPAQUE
}
impl Into<u8> for BlockFlag {fn into(self) -> u8 { self as u8 }}

pub enum ChunkFlag {
    DIRTY,
}
impl Into<u8> for ChunkFlag {fn into(self) -> u8 { self as u8 }}