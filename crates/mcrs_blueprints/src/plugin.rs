use bevy::app::{Plugin, App};

use crate::{Blueprints, BLOCK_BLUEPRINTS_PATH, GHOST_BLUEPRINTS_PATH, BlueprintList};

pub struct BlueprintsPlugin;

impl Plugin for BlueprintsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(
            Blueprints {
                blocks: BlueprintList::from_file(BLOCK_BLUEPRINTS_PATH),
                ghosts: BlueprintList::from_file(GHOST_BLUEPRINTS_PATH),
            }
        );
    }
}