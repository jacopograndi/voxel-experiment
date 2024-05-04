use std::fs::read_to_string;
use std::hash::Hash;

use bevy::{
    app::{App, Plugin},
    ecs::system::Resource,
    utils::HashMap,
};
use block::{BlockBlueprint, BlockId};
use ghost::{GhostBlueprint, GhostId};
use ron::from_str;
use serde::Deserialize;
use universe::Universe;

pub mod block;
pub mod chunk;
pub mod ghost;
pub mod plugin;
pub mod universe;

// TODO: To be passed from above
pub const BLOCK_BLUEPRINTS_PATH: &str = "assets/block_blueprints.ron";
pub const GHOST_BLUEPRINTS_PATH: &str = "assets/ghost_blueprints.ron";
pub const CHUNK_SIDE: usize = 32;
pub const CHUNK_AREA: usize = CHUNK_SIDE * CHUNK_SIDE;
pub const CHUNK_VOLUME: usize = CHUNK_AREA * CHUNK_SIDE;
pub const MAX_LIGHT: u8 = 15;

pub struct McrsUniversePlugin;

impl Plugin for McrsUniversePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Universe::default());
        app.insert_resource(Blueprints {
            blocks: BlueprintList::from_file(BLOCK_BLUEPRINTS_PATH),
            ghosts: BlueprintList::from_file(GHOST_BLUEPRINTS_PATH),
        });
    }
}

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
        ID: Clone + Copy + Default + Eq + PartialEq + Hash,
        BL: HasNameId<ID> + Clone + Default + for<'de> Deserialize<'de>,
    > BlueprintList<ID, BL>
{
    pub fn from_list(list: Vec<BL>) -> Self {
        let mut blueprints = Self::default();
        for blueprint in list {
            blueprints.list.insert(blueprint.id(), blueprint.clone());
            blueprints.name2id.insert(blueprint.name(), blueprint.id());
        }
        blueprints
    }

    pub fn from_file(path: &str) -> Self {
        let string = read_to_string(path).unwrap();
        let blueprints_vec: Vec<BL> = from_str(&string).unwrap();
        Self::from_list(blueprints_vec)
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
