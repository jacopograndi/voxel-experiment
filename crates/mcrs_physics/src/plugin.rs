use bevy::prelude::*;

use crate::character::character_controller_movement;

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum FixedPhysicsSet {
    Tick,
}

pub struct McrsPhysicsPlugin;

impl Plugin for McrsPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            character_controller_movement.in_set(FixedPhysicsSet::Tick),
        );
    }
}
