use bevy::prelude::*;
use mcrs_core::plugin::{CoreSet, McrsCorePlugin};
use mcrs_debug::plugin::McrsDebugPlugin;

mod hotbar;
mod spawn_player;
mod terrain_editing;
mod terrain_generation;
mod ui;

use hotbar::hotbar;
use mcrs_net::{IsClient, IsServer};
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

    app.add_systems(Startup, ui.run_if(resource_exists::<IsClient>()));

    // already added by McrsDebugPlugin
    //app.add_plugins(EguiPlugin);
    app.add_systems(Update, hotbar.run_if(resource_exists::<IsClient>()));
    app.run()
}
