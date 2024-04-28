use bevy::prelude::*;

use crate::{player_input, PlayerInputBuffer};

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum InputSet {
    Consume,
}

pub struct McrsInputPlugin;

impl Plugin for McrsInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInputBuffer>();
        app.add_systems(Update, player_input);
    }
}
