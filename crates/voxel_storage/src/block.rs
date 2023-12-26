use crate::{BlockId, BLOCK_FLAGS};
use voxel_flag_bank::flagbank::FlagBank;
use bytemuck::{Pod, Zeroable};

// Struct representing 1 cubic meter cube inside the game
#[repr(C)]
#[derive(Debug, Clone, Pod, Zeroable, Copy, Default, PartialEq, Eq)]
pub struct Block {
    pub id: u8,
    pub light: u8,
    pub properties: FlagBank,
}

// Generation and flag checking/setting utilities
impl Block {

    // TODO learn how to navigate a hashmap
    pub fn new(id:BlockId) -> Self {
        let mut new_block: Block = Self {
            id: id as u8,
            light: 0,
            properties: FlagBank::empty(),
        };
        if let Some(flags) = BLOCK_FLAGS.get(&id) {
            for flag in flags {
                new_block.properties.set(*flag);
            }
        }
        new_block
    }
}