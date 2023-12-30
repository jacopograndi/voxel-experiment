use bevy::ecs::system::Resource;
use blocks::BlockBlueprints;
use ghosts::GhostBlueprints;

pub mod blocks;
pub mod ghosts;
pub mod plugin;

pub const BLOCK_BLUEPRINTS_PATH: &str = "assets/block_blueprints.ron";
pub const GHOST_BLUEPRINTS_PATH: &str = "assets/ghost_blueprints.ron";

#[derive(Resource, Debug, Default)]
pub struct Blueprints {
    pub blocks: BlockBlueprints,
    pub ghosts: GhostBlueprints,
}