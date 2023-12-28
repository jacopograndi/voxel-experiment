use serde::{Deserialize, Serialize};

pub mod flagbank;

pub trait IsFlagBank { fn to_u8(self) -> u8; }

// Enum containing the bit index of each block flag in human readable form
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum BlockFlag {
    Collidable,
    Opaque
}
impl IsFlagBank for BlockFlag {
    fn to_u8(self) -> u8 { self as u8 }
}