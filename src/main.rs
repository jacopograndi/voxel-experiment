use bevy::{log::LogPlugin, prelude::*, window::PresentMode};
use bevy_egui::EguiPlugin;
use bevy_renet::client_connected;
use camera::McrsCameraPlugin;
use mcrs_blueprints::plugin::McrsBlueprintsPlugin;
use mcrs_debug::plugin::McrsDebugPlugin;

mod camera;
mod hotbar;
mod player;
mod terrain;
mod ui;

use hotbar::{
    client_receive_replica, client_send_replica, hotbar, server_receive_replica,
    server_send_replica,
};
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
pub enum FixedCoreSet {
    Update,
}

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum CoreSet {
    Ui,
}

fn main() {
    let mut app = App::new(); // Instantiate Bevy App

    app.configure_sets(
        FixedUpdate,
        (
            NetSet::Receive,
            PhysicsSet::Update,
            FixedCoreSet::Update,
            NetSet::Send,
        )
            .chain(),
    );

    app.configure_sets(Update, (
                                CoreSet::Ui,
            InputSet::Gather, 
                                ).chain());

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
    app.add_systems(Update, hotbar.in_set(CoreSet::Ui));
    app.add_systems(
        FixedUpdate,
        (
            client_receive_replica.in_set(NetSet::Receive),
            client_send_replica.in_set(NetSet::Send),
        )
            .run_if(client_connected()),
    );
}

fn add_server(app: &mut App) {
    app.add_plugins((McrsNetServerPlugin, McrsPhysicsPlugin));
    app.add_systems(
        FixedUpdate,
        (terrain_generation, terrain_editing)
            .chain()
            .in_set(FixedCoreSet::Update),
    );
    app.add_systems(
        FixedUpdate,
        (
            server_receive_replica.in_set(NetSet::Receive),
            server_send_replica.in_set(NetSet::Send),
        ),
    );
}
