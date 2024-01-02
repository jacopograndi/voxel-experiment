use bevy::prelude::{Deref, DerefMut};
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::HasNameId;

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct GhostBlueprint {
    pub name: String,
    pub id: GhostId,
    pub voxel_texture_path: String,
}

impl HasNameId<GhostId> for GhostBlueprint {
    fn id(&self) -> GhostId {
        self.id
    }
    fn name(&self) -> String {
        self.name.clone()
    }
}

#[repr(C)]
#[derive(Debug, Default, PartialEq, Eq, Clone, Hash, Copy, Deref, DerefMut, Pod, Zeroable)]
pub struct GhostId(u32);

// tell serde to serialize only the number and not the type
impl Serialize for GhostId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for GhostId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|id| GhostId(id))
    }
}
