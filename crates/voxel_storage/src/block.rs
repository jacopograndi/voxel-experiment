use crate::{BlockId, BLOCK_INFO};
use voxel_flag_bank::{flagbank::FlagBank, BlockFlag};
use bytemuck::{Pod, Zeroable};
use std::fmt::Display;

// Struct representing 1 cubic meter cube inside the game
#[repr(C)]
#[derive(Debug, Clone, Pod, Zeroable, Copy, Default, PartialEq, Eq)]
pub struct Block {
    pub id: u8,
    pub properties: FlagBank,
    // for now i'm using light0 as torchlight and light1 as sunlight
    // in the future they could be a u16 divided into 4 u4
    // that encode red, green and blue torchlight and sunlight.
    pub light0: u8,
    pub light1: u8,
}

// Generation and flag checking/setting utilities
impl Block {

    // TODO learn how to navigate a hashmap
    pub fn new(id:BlockId) -> Self {
        let mut new_block: Block = Self {
            id: id as u8,
            light0: 0,
            light1: 0,
            properties: FlagBank::empty(),
        };
        let blockinfo = BLOCK_INFO.get(&(id as u8)).unwrap();
        let flags: &Vec<BlockFlag> = &blockinfo.flags; 
        for flag in flags {
            new_block.properties.set(*flag);
        }
        new_block
    }

    pub fn get_light(&self, light_type: LightType) -> u8 {
        match light_type {
            LightType::Torch => self.light0,
            LightType::Sun => self.light1,
        }
    }
    pub fn set_light(&mut self, light_type: LightType, v: u8) {
        assert!((0..=MAX_LIGHT).contains(&v));
        match light_type {
            LightType::Torch => self.light0 = v,
            LightType::Sun => self.light1 = v,
        }
    }

}

pub const MAX_LIGHT: u8 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    Torch,
    Sun,
}
impl Display for LightType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LightType::Torch => "torch",
                LightType::Sun => "sun",
            }
        )
    }
}