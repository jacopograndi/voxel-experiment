use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};
use mcrs_flag_bank::BlockFlag;
use serde::{Deserialize, Serialize};

use crate::HasNameId;

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct BlockBlueprint {
    pub name: String,
    pub id: BlockId,
    pub flags: Vec<BlockFlag>,
    pub light_level: u8,
    pub voxel_texture_path: String,
    pub drop_item_id: BlockId,
}

impl HasNameId<BlockId> for BlockBlueprint {
    fn id(&self) -> BlockId {
        self.id
    }
    fn name(&self) -> String {
        self.name.clone()
    }
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
