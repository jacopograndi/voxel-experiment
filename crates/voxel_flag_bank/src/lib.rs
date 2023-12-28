use serde::{Deserialize, Serialize};

pub mod flagbank;

// Enum containing the bit index of each block flag in human readable form
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum BlockFlag {
    Collidable,
    Opaque
}
impl Into<u8> for BlockFlag {fn into(self) -> u8 { self as u8 }}

pub enum ChunkFlag {
    Dirty,
}
impl Into<u8> for ChunkFlag {fn into(self) -> u8 { self as u8 }}