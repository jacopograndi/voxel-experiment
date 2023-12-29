use bevy::utils::HashMap;
use bytemuck::{Pod, Zeroable};
use ron::from_str;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;

use bevy::prelude::*;

#[derive(Debug, Default)]
pub struct GhostBlueprints {
    ghosts: HashMap<GhostId, GhostBlueprint>,
    name_to_ghost: HashMap<String, GhostId>,
}

impl GhostBlueprints {
    pub fn from_file(path: &str) -> Self {
        let string = read_to_string(path).unwrap();
        let block_blueprints_vec: Vec<GhostBlueprint> = from_str(&string).unwrap();
        let mut blueprints = Self::default();
        for block_blueprints in block_blueprints_vec {
            let id = block_blueprints.id.clone();
            blueprints.ghosts.insert(id.clone(), block_blueprints.clone());
            blueprints.name_to_ghost.insert(block_blueprints.name, id);
        }
        blueprints
    }

    pub fn iter(&self) -> impl Iterator<Item = &GhostBlueprint> {
        self.ghosts.iter().map(|(_, b)| b)
    }

    pub fn get(&self, id: &GhostId) -> &GhostBlueprint {
        self.ghosts.get(id).unwrap()
    }
    pub fn get_checked(&self, id: &GhostId) -> Option<&GhostBlueprint> {
        self.ghosts.get(id)
    }

    pub fn id_from_name(&self, name: &str) -> GhostId {
        *self.name_to_ghost.get(name).unwrap()
    }
    pub fn id_from_name_checked(&self, name: &str) -> Option<&GhostId> {
        self.name_to_ghost.get(name)
    }

    pub fn from_name(&self, name: &str) -> &GhostBlueprint {
        self.ghosts.get(&self.id_from_name(name)).unwrap()
    }
    pub fn from_name_checked(&self, name: &str) -> Option<&GhostBlueprint> {
        self.ghosts.get(self.id_from_name_checked(name)?)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GhostBlueprint {
    pub name: String,
    pub id: GhostId,
    pub voxel_texture_path: String,
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
