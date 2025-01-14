use bevy::prelude::*;

use crate::{character::character_controller_movement, reset_tickstep, run_if_tickstep, TickStep};

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum FixedPhysicsSet {
    Tick,
}

pub struct McrsPhysicsPlugin;

impl Plugin for McrsPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            character_controller_movement
                .in_set(FixedPhysicsSet::Tick)
                .run_if(run_if_tickstep),
        )
        .add_systems(FixedUpdate, reset_tickstep.after(FixedPhysicsSet::Tick))
        .init_resource::<TickStep>();
    }
}
