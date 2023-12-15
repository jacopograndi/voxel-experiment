use bevy::prelude::*;

use crate::character::{
    camera_controller_movement, character_controller_movement, cursor_grab, initial_grab_cursor,
};

pub struct VoxelPhysicsPlugin;

impl Plugin for VoxelPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, initial_grab_cursor);
        app.add_systems(Update, cursor_grab);
        app.add_systems(Update, camera_controller_movement);
        app.add_systems(FixedUpdate, character_controller_movement);
    }
}
