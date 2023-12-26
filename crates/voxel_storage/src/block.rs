use crate::{BlockID, BlockFlag, BLOCK_FLAGS};
use bytemuck::{Pod, Zeroable};

// Struct representing 1 cubic meter cube inside the game
#[repr(C)]
#[derive(Debug, Clone, Pod, Zeroable, Copy, Default, PartialEq, Eq)]
pub struct Block {
    pub id: u8,
    pub light: u8,
    _properties: u16,
}

// Generation and flag checking/setting utilities
impl Block {

    // TODO learn how to navigate a hashmap
    pub fn new(id:BlockID) -> Self {
        let mut new_block: Block = Self {
            id: id as u8,
            light: 0,
            _properties: 0,
        };
        if let Some(flags) = BLOCK_FLAGS.get(&id) {
            for flag in flags {
                new_block.set_flag(*flag);
            }
        }
        new_block
    }

    pub fn set_flag(&mut self, flag: BlockFlag) {
        self._properties |= 0b1 << flag as u8;
    }

    pub fn unset_flag(&mut self, flag: BlockFlag) {
        self._properties &= !(0b1 << flag as u8);
    }

    pub fn check_flag(&self, flag: BlockFlag) -> bool {
        (self._properties >> flag as u8) & 0b1 == 1
    }

}
