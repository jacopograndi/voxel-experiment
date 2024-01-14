use bevy::prelude::*;
use mcrs_core::plugin::{CoreSet, McrsCorePlugin};
use mcrs_debug::plugin::McrsDebugPlugin;

mod spawn_player;
mod terrain_editing;
mod terrain_generation;
mod ui;

use mcrs_net::IsServer;
use spawn_player::spawn_player;
use terrain_editing::terrain_editing;
use terrain_generation::terrain_generation;
use ui::ui;

fn main() {
    let mut app = App::new();
    app.add_plugins((McrsCorePlugin, McrsDebugPlugin));

    app.add_systems(
        FixedUpdate,
        (terrain_generation, terrain_editing)
            .in_set(CoreSet::Update)
            .run_if(resource_exists::<IsServer>()),
    );
    app.add_systems(Update, spawn_player);

    // already added by McrsDebugPlugin
    //app.add_plugins(EguiPlugin);
    app.add_systems(Update, ui);
    app.run()
}
