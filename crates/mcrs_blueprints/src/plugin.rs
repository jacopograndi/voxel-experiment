use bevy::app::{App, Plugin};

use crate::{BlueprintList, Blueprints, BLOCK_BLUEPRINTS_PATH, GHOST_BLUEPRINTS_PATH};

pub struct BlueprintsPlugin;

impl Plugin for BlueprintsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Blueprints {
            blocks: BlueprintList::from_file(BLOCK_BLUEPRINTS_PATH),
            ghosts: BlueprintList::from_file(GHOST_BLUEPRINTS_PATH),
        });
    }
}
