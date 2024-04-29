use bevy::{log::LogPlugin, prelude::*, window::PresentMode};
use bevy_egui::EguiPlugin;
use bevy_renet::client_connected;
use camera::McrsCameraPlugin;
use clap::Parser;
use mcrs_blueprints::plugin::McrsBlueprintsPlugin;
use mcrs_debug::plugin::McrsDebugPlugin;

mod camera;
mod hotbar;
mod player;
mod settings;
mod terrain;
mod ui;

use hotbar::{
    client_receive_replica, client_send_replica, hotbar, server_receive_replica,
    server_send_replica,
};
use mcrs_input::plugin::{InputSet, McrsInputPlugin};
use mcrs_net::{
    plugin::{FixedNetSet, McrsNetClientPlugin, McrsNetServerPlugin},
    NetSettings, NetworkMode,
};
use mcrs_physics::plugin::{FixedPhysicsSet, McrsPhysicsPlugin};
use mcrs_render::plugin::{McrsVoxelRenderPlugin, RenderSettings};
use mcrs_storage::McrsStoragePlugin;
use player::spawn_player;
use settings::{Args, McrsSettings};
use terrain::{terrain_editing, terrain_generation};
use ui::ui;

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum FixedMainSet {
    Terrain,
}

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum UiSet {
    Overlay,
}

fn main() {
    let mut app = App::new(); // Instantiate Bevy App

    app.configure_sets(
        FixedUpdate,
        (
            FixedNetSet::Receive,
            FixedPhysicsSet::Tick,
            FixedMainSet::Terrain,
            FixedNetSet::Send,
        )
            .chain(),
    );

    app.configure_sets(Update, (UiSet::Overlay, InputSet::Gather).chain());

    app.add_plugins((McrsStoragePlugin, McrsBlueprintsPlugin, McrsInputPlugin));

    let settings: McrsSettings = Args::parse().into();
    app.insert_resource(Time::<Fixed>::from_seconds(
        1f64 / settings.ticks_per_second as f64,
    ));
    app.insert_resource::<NetSettings>(settings.clone().into());
    app.insert_resource::<RenderSettings>(settings.clone().into());
    app.insert_resource(settings.clone());

    match settings.network_mode {
        NetworkMode::Client => {
            add_client(&mut app);
        }
        NetworkMode::Server => {
            app.add_plugins((MinimalPlugins, TransformPlugin, LogPlugin::default()));
            add_server(&mut app);
        }
        NetworkMode::ClientAndServer => {
            add_client(&mut app);
            add_server(&mut app);
        }
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
    app.add_systems(Update, hotbar.in_set(UiSet::Overlay));
    app.add_systems(
        FixedUpdate,
        (
            client_receive_replica.in_set(FixedNetSet::Receive),
            client_send_replica.in_set(FixedNetSet::Send),
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
            .in_set(FixedMainSet::Terrain),
    );
    app.add_systems(
        FixedUpdate,
        (
            server_receive_replica.in_set(FixedNetSet::Receive),
            server_send_replica.in_set(FixedNetSet::Send),
        ),
    );
}
