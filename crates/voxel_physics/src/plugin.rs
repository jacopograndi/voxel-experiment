use bevy::prelude::*;

use crate::character::character_controller_movement;

pub struct VoxelPhysicsPlugin;

impl Plugin for VoxelPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, character_controller_movement);
    }
}
