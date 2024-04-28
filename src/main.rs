use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use mcrs_core::plugin::{CoreSet, McrsCorePlugin};
use mcrs_debug::plugin::McrsDebugPlugin;

mod hotbar;
mod spawn_player;
mod terrain_editing;
mod terrain_generation;
mod ui;

use hotbar::hotbar;
use mcrs_settings::NetworkMode;
use spawn_player::spawn_player;
use terrain_editing::terrain_editing;
use terrain_generation::terrain_generation;
use ui::ui;

fn main() {
    let mut app = App::new(); // Bevy App
    app.add_plugins(McrsCorePlugin);
    match app.world.get_resource::<NetworkMode>() {
        Some(NetworkMode::Client) => add_ui(&mut app),
        Some(NetworkMode::Server) => add_terrain(&mut app),
        Some(NetworkMode::ClientAndServer) => {
            add_ui(&mut app);
            add_terrain(&mut app);
        }
        None => panic!("You are not client nor server. Fix yourself. Be a functioning member of society."),
    }
    app.add_systems(Update, spawn_player);
    app.run()
}

fn add_ui(app: &mut App) {
    app.add_plugins(EguiPlugin);
    app.add_plugins(McrsDebugPlugin);
    app.add_systems(Startup, ui);
    app.add_systems(Update, hotbar);
}

fn add_terrain(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (terrain_generation, terrain_editing)
            .chain()
            .in_set(CoreSet::Update)
    );
}