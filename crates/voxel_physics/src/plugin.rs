use bevy::prelude::*;

use crate::character;

pub struct VoxelPhysicsPlugin;

impl Plugin for VoxelPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, character::movement);
    }
}
