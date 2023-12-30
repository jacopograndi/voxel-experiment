use std::fs::read_to_string;

use bevy::{ecs::system::Resource, utils::HashMap};
use std::hash::Hash;
use blocks::{BlockBlueprint, BlockId};
use ghosts::{GhostId, GhostBlueprint};
use ron::from_str;
use serde::Deserialize;

pub mod blocks;
pub mod ghosts;
pub mod plugin;

pub const BLOCK_BLUEPRINTS_PATH: &str = "assets/block_blueprints.ron";
pub const GHOST_BLUEPRINTS_PATH: &str = "assets/ghost_blueprints.ron";

#[derive(Resource, Debug)]
pub struct Blueprints {
    pub blocks: BlueprintList<BlockId, BlockBlueprint>,
    pub ghosts: BlueprintList<GhostId, GhostBlueprint>,
}

#[derive(Debug, Default)]
pub struct BlueprintList<ID, BL> {
    list: HashMap<ID, BL>,
    name2id: HashMap<String, ID>,
}

pub trait HasNameId<ID> {
    fn id(&self) -> ID;
    fn name(&self) -> String;
}

impl<
    ID: Clone + Copy + Default +  Eq + PartialEq + Hash, 
    BL: HasNameId<ID> + Clone + Default + for<'de> Deserialize<'de>
> BlueprintList<ID, BL> {
    fn from_file(path: &str) -> Self {
        let string = read_to_string(path).unwrap();
        let blueprints_vec: Vec<BL> = from_str(&string).unwrap();
        let mut blueprints = Self::default();
        for blueprint in blueprints_vec {
            blueprints.list.insert(blueprint.id(), blueprint.clone());
            blueprints.name2id.insert(blueprint.name(), blueprint.id());
        }
        blueprints
    }

    pub fn iter(&self) -> impl Iterator<Item = &BL> {
        self.list.iter().map(|(_, b)| b)
    }

    pub fn get(&self, id: &ID) -> &BL {
        self.list.get(id).unwrap()
    }
    pub fn get_checked(&self, id: &ID) -> Option<&BL> {
        self.list.get(id)
    }

    pub fn get_named(&self, name: &str) -> &BL {
        self.list.get(&self.id_named(name)).unwrap()
    }
    pub fn get_named_checked(&self, name: &str) -> Option<&BL> {
        self.list.get(self.id_named_checked(name)?)
    }

    pub fn id_named(&self, name: &str) -> ID {
        *self.name2id.get(name).unwrap()
    }
    pub fn id_named_checked(&self, name: &str) -> Option<&ID> {
        self.name2id.get(name)
    }
}
