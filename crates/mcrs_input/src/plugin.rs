use bevy::prelude::*;

use crate::{player_input, PlayerInput};

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum InputSet {
    Consume,
}

pub struct McrsInputPlugin;

impl Plugin for McrsInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInput>();
        app.add_systems(Update, player_input);
        app.add_systems(
            FixedUpdate,
            consume_player_input.in_set(InputSet::Consume),
        );
    }
}

pub fn consume_player_input(mut player_input_query: Query<&mut PlayerInput>) {
    for mut input in player_input_query.iter_mut() {
        input.consume();
    }
}
