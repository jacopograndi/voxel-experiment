use bevy::prelude::*;

pub mod blocks;
pub mod ghosts;

pub use blocks::*;
pub use ghosts::*;

pub struct InfoPlugin;

impl Plugin for InfoPlugin {
    fn build(&self, app: &mut App) {
        let info = Info {
            blocks: BlocksInfo::from_file(),
            ghosts: GhostsInfo::from_file(),
        };
        app.insert_resource(info);
    }
}

#[derive(Resource, Debug, Default)]
pub struct Info {
    pub blocks: BlocksInfo,
    pub ghosts: GhostsInfo,
}
