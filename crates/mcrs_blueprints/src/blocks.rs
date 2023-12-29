use bevy::utils::HashMap;
use bytemuck::{Pod, Zeroable};
use ron::from_str;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;

use mcrs_flag_bank::BlockFlag;

use bevy::prelude::*;

#[derive(Debug, Default)]
pub struct BlockBlueprints {
    blocks: HashMap<BlockId, BlockBlueprint>,
    name_to_block: HashMap<String, BlockId>,
}

impl BlockBlueprints {
    pub fn from_file(path: &str) -> Self {
        let string = read_to_string(path).unwrap();
        let block_blueprints_vec: Vec<BlockBlueprint> = from_str(&string).unwrap();
        let mut blueprints = Self::default();
        for block_blueprints in block_blueprints_vec {
            let id = block_blueprints.id.clone();
            blueprints.blocks.insert(id.clone(), block_blueprints.clone());
            blueprints.name_to_block.insert(block_blueprints.name, id);
        }
        blueprints
    }

    pub fn iter(&self) -> impl Iterator<Item = &BlockBlueprint> {
        self.blocks.iter().map(|(_, b)| b)
    }

    pub fn get(&self, id: &BlockId) -> &BlockBlueprint {
        self.blocks.get(id).unwrap()
    }
    pub fn get_checked(&self, id: &BlockId) -> Option<&BlockBlueprint> {
        self.blocks.get(id)
    }

    pub fn id_from_name(&self, name: &str) -> BlockId {
        *self.name_to_block.get(name).unwrap()
    }
    pub fn id_from_name_checked(&self, name: &str) -> Option<&BlockId> {
        self.name_to_block.get(name)
    }

    pub fn from_name(&self, name: &str) -> &BlockBlueprint {
        self.blocks.get(&self.id_from_name(name)).unwrap()
    }
    pub fn from_name_checked(&self, name: &str) -> Option<&BlockBlueprint> {
        self.blocks.get(self.id_from_name_checked(name)?)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockBlueprint {
    pub name: String,
    pub id: BlockId,
    pub flags: Vec<BlockFlag>,
    pub light_level: u8,
    pub voxel_texture_path: String,
    pub drop_item_id: BlockId,
}

impl BlockBlueprint {
    pub fn is_light_source(&self) -> bool {
        self.light_level > 0
    }
}

/// This is the logical block id.
/// It also is the offset in the 3d texture buffer.
#[repr(C)]
#[derive(Debug, Default, PartialEq, Eq, Clone, Hash, Copy, Deref, DerefMut, Pod, Zeroable)]
pub struct BlockId(u8);

// tell serde to serialize only the number and not the type
impl Serialize for BlockId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for BlockId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|id| BlockId(id))
    }
}
