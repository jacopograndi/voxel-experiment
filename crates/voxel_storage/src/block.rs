use bevy::utils::HashMap;
use bytemuck::{Pod, Zeroable};
use lazy_static::lazy_static;

// HashMap containing a description for all default flags by block ID
lazy_static! {
    static ref LEGAL_BLOCKS: HashMap<u8, Vec<BlockFlag>> = {
        let mut map = HashMap::new();
        map.insert(0, vec![]);
        map.insert(1, vec![BlockFlag::SOLID]);
        map.insert(2, vec![]);
        map.insert(3, vec![BlockFlag::SOLID]);
        map
    };
}

// Enum containing the bit index of each block flag in human readable form
#[derive(Clone)]
pub enum BlockFlag {
    SOLID,

}

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

    // set_flag does not work so there could be an alignment problem in _write_flag
    pub fn new(id:u8, solid: bool) -> Self {
        let mut new_block: Block = Self {
            id: id,
            _properties: 0,
            light: 0,
        };
        if solid { new_block._properties = 1; }
        new_block
    }

    // TODO learn how to navigate a hashmap
    // pub fn new(id:u8, solid: bool) -> Self {
    //     // let mut p: u16 = 0;
    //     let mut new_block: Block = Self {
    //         id: id,
    //         _properties: 0,
    //         light: 0,
    //     };
    //     // for f in 0..16 {
    //     //     for flags in LEGAL_BLOCKS.get(&f) {
    //     //         for flag in flags {
    //     //             new_block.set_flag(*flag);
    //     //         }
    //     //     }
    //     // }
    //     if solid {
    //         new_block.set_flag(BlockFlag::SOLID);
    //     }
    //     new_block
    // }

    // Might not be working due to bytes being ordered big-endianly
    fn _write_flag(&mut self, flag: BlockFlag, value: bool) {
        if value {
            self._properties &= !(0b1 << flag as u8);
        } else {
            self._properties |= 0b1 << flag as u8;
        }
    }

    pub fn set_flag(&mut self, flag: BlockFlag) {
        self._write_flag(flag, true);
    }

    pub fn unset_flag(&mut self, flag: BlockFlag) {
        self._write_flag(flag, false);
    }

    pub fn check_flag(&self, flag: BlockFlag) -> bool {
        (self._properties >> flag as u8) & 0b1 == 1
    }

}
