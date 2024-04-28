use bevy::{log::LogPlugin, prelude::*, window::PresentMode};
use bevy_egui::EguiPlugin;
use camera::McrsCameraPlugin;
use mcrs_blueprints::plugin::McrsBlueprintsPlugin;
use mcrs_debug::plugin::McrsDebugPlugin;

mod camera;
mod hotbar;
mod player;
mod terrain;
mod ui;

use hotbar::hotbar;
use mcrs_input::plugin::{InputSet, McrsInputPlugin};
use mcrs_net::plugin::{McrsNetClientPlugin, McrsNetServerPlugin, NetSet};
use mcrs_physics::plugin::{McrsPhysicsPlugin, PhysicsSet};
use mcrs_render::plugin::McrsVoxelRenderPlugin;
use mcrs_settings::{plugin::McrsSettingsPlugin, NetworkMode};
use mcrs_storage::McrsVoxelStoragePlugin;
use player::spawn_player;
use terrain::{terrain_editing, terrain_generation};
use ui::ui;

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum CoreSet {
    Update,
}

fn main() {
    let mut app = App::new(); // Instantiate Bevy App

    app.configure_sets(
        FixedUpdate,
        (
            NetSet::Receive,
            PhysicsSet::Update,
            CoreSet::Update,
            NetSet::Send,
            InputSet::Consume,
        )
            .chain(),
    );

    app.add_plugins((
        McrsSettingsPlugin,
        McrsVoxelStoragePlugin,
        McrsBlueprintsPlugin,
        McrsInputPlugin,
    ));

    match app.world.get_resource::<NetworkMode>() {
        Some(NetworkMode::Client) => {
            add_client(&mut app);
        }
        Some(NetworkMode::Server) => {
            app.add_plugins((MinimalPlugins, TransformPlugin, LogPlugin::default()));
            add_server(&mut app);
        }
        Some(NetworkMode::ClientAndServer) => {
            add_client(&mut app);
            add_server(&mut app);
        }
        None => panic!(
            "You are not client nor server. Fix yourself. Be a functioning member of society."
        ),
    }
    app.add_systems(Update, spawn_player);

    app.run()
}

fn add_client(app: &mut App) {
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
        McrsVoxelRenderPlugin,
        EguiPlugin,
        McrsDebugPlugin,
        McrsNetClientPlugin,
        McrsCameraPlugin,
    ));
    app.add_systems(Startup, ui);
    app.add_systems(Update, hotbar);
}

fn add_server(app: &mut App) {
    app.add_plugins((McrsNetServerPlugin, McrsPhysicsPlugin));
    app.add_systems(
        FixedUpdate,
        (terrain_generation, terrain_editing)
            .chain()
            .in_set(CoreSet::Update),
    );
}
