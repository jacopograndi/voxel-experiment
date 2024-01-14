use bevy::{
    log::LogPlugin,
    prelude::*,
    window::{PresentMode, WindowPlugin},
};
use bevy_renet::{
    client_connected,
    transport::{NetcodeClientPlugin, NetcodeServerPlugin},
    RenetClientPlugin, RenetServerPlugin,
};
use clap::Parser;
use mcrs_blueprints::plugin::BlueprintsPlugin;
use mcrs_physics::plugin::VoxelPhysicsPlugin;
use mcrs_render::plugin::VoxelRenderPlugin;
use mcrs_storage::VoxelStoragePlugin;
use renet::RenetServer;

mod camera;
mod diagnostics;
mod input;
mod net;
mod terrain_editing;
mod terrain_generation;
mod ui;

use camera::*;
use diagnostics::*;
use input::*;
use net::{
    client::{client_send_input, client_sync_players, client_sync_universe, new_renet_client},
    server::{
        consume_player_input, move_players_system, new_renet_server, server_refresh_time,
        server_sync_players, server_sync_universe, server_update_system,
    },
    *,
};
use terrain_editing::*;
use terrain_generation::*;
use ui::client_ui;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    network_mode: Option<String>,

    #[arg(short, long)]
    address_server: Option<String>,
}

fn main() {
    let args = Args::parse();
    let network_mode = NetworkMode::from(args.network_mode.as_deref());
    let addr = if let Some(addr) = args.address_server {
        addr
    } else {
        "127.0.0.1".to_string()
    };

    let mut app = App::new();
    app.init_resource::<Lobby>();
    app.insert_resource(network_mode.clone());
    app.insert_resource(server_refresh_time());
    app.add_plugins((VoxelStoragePlugin, BlueprintsPlugin));

    match network_mode {
        NetworkMode::Server => {
            app.add_plugins((MinimalPlugins, TransformPlugin, LogPlugin::default()));
            app_server(&mut app, &addr);
        }
        NetworkMode::ClientAndServer => {
            app_server(&mut app, &addr);
            app_client(&mut app, "127.0.0.1");
        }
        NetworkMode::Client => {
            app_client(&mut app, &addr);
        }
    }

    app.run();
}

fn app_server(app: &mut App, addr: &str) {
    app.add_plugins((RenetServerPlugin, NetcodeServerPlugin, VoxelPhysicsPlugin));
    let (server, transport) = new_renet_server(addr);
    app.insert_resource(server);
    app.insert_resource(transport);
    app.init_resource::<ChunkReplication>();
    app.add_systems(
        FixedUpdate,
        (
            server_update_system,
            server_sync_players,
            server_sync_universe,
            player_edit_terrain,
            move_players_system,
            generate_chunks,
            consume_player_input,
        )
            .chain()
            .run_if(resource_exists::<RenetServer>()),
    );
}

fn app_client(app: &mut App, addr: &str) {
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
        RenetClientPlugin,
        NetcodeClientPlugin,
        VoxelRenderPlugin,
    ));
    app.init_resource::<PlayerInput>();
    let (client, transport) = new_renet_client(addr);
    app.insert_resource(client);
    app.insert_resource(transport);
    app.add_systems(Update, camera_controller_movement);
    app.add_systems(Update, player_input);
    app.add_systems(
        FixedUpdate,
        (client_send_input, client_sync_players, client_sync_universe).run_if(client_connected()),
    );
    app.add_plugins(DebugDiagnosticPlugin);
    app.add_systems(Startup, client_ui)
        .add_systems(Update, cursor_grab);
}
