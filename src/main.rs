use bevy::{log::LogPlugin, prelude::*, window::PresentMode};
use bevy_egui::EguiPlugin;
use bevy_renet::client_connected;
use camera::McrsCameraPlugin;
use clap::Parser;

use mcrs_net::{
    plugin::{FixedNetSet, McrsNetClientPlugin, McrsNetServerPlugin},
    NetSettings, NetworkMode,
};
use mcrs_physics::plugin::{FixedPhysicsSet, McrsPhysicsPlugin};
use mcrs_render::{plugin::McrsVoxelRenderPlugin, settings::RenderSettings};
use mcrs_universe::McrsUniversePlugin;

mod camera;
mod chemistry;
mod debug;
mod hotbar;
mod input;
mod player;
mod saveload;
mod settings;
mod terrain;
mod ui;

use debug::DebugDiagnosticPlugin;
use hotbar::{
    client_receive_replica, client_send_replica, hotbar, server_receive_replica,
    server_send_replica,
};
use input::*;
use player::{spawn_player, terrain_editing};
use renet::RenetServer;
use saveload::*;
use settings::{Args, McrsSettings};
use terrain::*;
use ui::ui;

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum FixedMainSet {
    Terrain,
    SaveLoad,
}

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum UiSet {
    Overlay,
}

fn main() -> AppExit {
    let mut app = App::new();

    app.configure_sets(
        FixedUpdate,
        (
            FixedNetSet::Receive,
            FixedPhysicsSet::Tick,
            FixedMainSet::Terrain,
            FixedMainSet::SaveLoad,
            FixedNetSet::Send,
        )
            .chain(),
    );
    app.configure_sets(Update, (UiSet::Overlay, InputSet::Gather).chain());

    // todo: encapsulate in a settings plugin?
    let settings: McrsSettings = Args::parse().into();
    app.insert_resource(Time::<Fixed>::from_seconds(
        1f64 / settings.ticks_per_second as f64,
    ));
    app.insert_resource::<NetSettings>(settings.clone().into());
    app.insert_resource::<RenderSettings>(settings.clone().into());
    app.insert_resource(settings.clone());

    app.add_plugins(McrsUniversePlugin);
    app.init_resource::<UniverseChanges>();
    app.init_resource::<LightSources>();
    app.init_resource::<ChunkGenerationRequest>();
    app.init_resource::<SunBeams>();

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
        DebugDiagnosticPlugin,
        McrsNetClientPlugin,
        McrsCameraPlugin,
    ));
    app.add_systems(Startup, ui);
    app.add_systems(Update, hotbar.in_set(UiSet::Overlay));
    app.add_systems(
        Update,
        (player_input, move_local_players)
            .chain()
            .in_set(InputSet::Gather),
    );
    app.add_systems(Update, terrain_editing.after(InputSet::Gather));
    app.add_systems(
        FixedUpdate,
        (
            client_receive_replica.in_set(FixedNetSet::Receive),
            client_send_replica.chain().in_set(FixedNetSet::Send),
        )
            .run_if(client_connected),
    );
}

fn add_server(app: &mut App) {
    app.add_plugins((McrsNetServerPlugin, McrsPhysicsPlugin, SaveLoadPlugin));
    app.add_systems(
        FixedUpdate,
        (
            server_receive_replica
                .run_if(resource_exists::<RenetServer>)
                .chain()
                .in_set(FixedNetSet::Receive),
            (
                request_base_chunks,
                chunk_generation,
                apply_terrain_changes,
                apply_lighting_sources,
            )
                .chain()
                .in_set(FixedMainSet::Terrain),
            server_send_replica
                .in_set(FixedNetSet::Send)
                .run_if(resource_exists::<RenetServer>),
        ),
    );
}
