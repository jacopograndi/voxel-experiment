use bevy::app::{Plugin, App};

use crate::{Blueprints, blocks::BlockBlueprints, ghosts::GhostBlueprints, BLOCK_BLUEPRINTS_PATH, GHOST_BLUEPRINTS_PATH};

pub struct BlueprintsPlugin;

impl Plugin for BlueprintsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(
            Blueprints {
                blocks: BlockBlueprints::from_file(BLOCK_BLUEPRINTS_PATH),
                ghosts: GhostBlueprints::from_file(GHOST_BLUEPRINTS_PATH),
            }
        );
    }
}