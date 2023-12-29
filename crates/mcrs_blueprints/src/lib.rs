use bevy::prelude::*;

pub mod blocks;
pub mod ghosts;

pub use blocks::*;
pub use ghosts::*;

const BLOCK_BLUEPRINTS_PATH: &str = "assets/block_blueprints.ron";
const GHOST_BLUEPRINTS_PATH: &str = "assets/ghost_blueprints.ron";

pub struct BlueprintsPlugin;

impl Plugin for BlueprintsPlugin {
    fn build(&self, app: &mut App) {
        let blueprints = Blueprints {
            blocks: BlockBlueprints::from_file(BLOCK_BLUEPRINTS_PATH),
            ghosts: GhostBlueprints::from_file(GHOST_BLUEPRINTS_PATH),
        };
        app.insert_resource(blueprints);
    }
}

#[derive(Resource, Debug, Default)]
pub struct Blueprints {
    pub blocks: BlockBlueprints,
    pub ghosts: GhostBlueprints,
}
