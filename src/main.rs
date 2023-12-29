use bevy::{
    prelude::*,
    window::{PresentMode, WindowPlugin},
};
use bevy_renet::{
    client_connected,
    transport::{NetcodeClientPlugin, NetcodeServerPlugin},
    RenetClientPlugin, RenetServerPlugin,
};
use mcrs_info::InfoPlugin;
use mcrs_physics::plugin::VoxelPhysicsPlugin;
use mcrs_render::{
    boxes_world::{VoxTextureIndex, VoxTextureLoadQueue},
    VoxelRenderPlugin,
};
use mcrs_storage::VoxelStoragePlugin;
use renet::RenetServer;

mod camera;
mod diagnostics;
mod input;
mod net;
mod terrain_editing;
mod terrain_generation;

use camera::*;
use diagnostics::*;
use input::*;
use net::{
    client::{client_send_input, client_sync_players, client_sync_universe, new_renet_client},
    server::{
        move_players_system, new_renet_server, server_sync_players, server_sync_universe,
        server_update_system,
    },
    *,
};
use terrain_editing::*;
use terrain_generation::*;

const SERVER_TICKS_PER_SECOND: u32 = 60;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let network_mode = if args.len() > 1 {
        match args[1].as_str() {
            "client" => NetworkMode::Client,
            "server" => NetworkMode::ClientAndServer,
            "headless" => NetworkMode::Server,
            _ => panic!("Invalid argument, must be \"client\", \"server\" or \"headless\"."),
        }
    } else {
        NetworkMode::ClientAndServer
    };

    let mut app = App::new();
    app.init_resource::<Lobby>();
    app.insert_resource(network_mode.clone());
    app.insert_resource(Time::<Fixed>::from_seconds(
        1. / (SERVER_TICKS_PER_SECOND as f64),
    ));
    app.add_plugins((VoxelPhysicsPlugin, VoxelStoragePlugin, InfoPlugin));

    match network_mode {
        NetworkMode::Server => {
            app.add_plugins((MinimalPlugins, TransformPlugin));
            app_server(&mut app);
        }
        NetworkMode::ClientAndServer => {
            app_server(&mut app);
            app_client(&mut app);
        }
        NetworkMode::Client => {
            app_client(&mut app);
        }
    }

    app.run();
}

fn app_server(app: &mut App) {
    app.add_plugins((RenetServerPlugin, NetcodeServerPlugin));
    let (server, transport) = new_renet_server();
    app.insert_resource(server);
    app.insert_resource(transport);
    app.init_resource::<ChunkReplication>();
    app.add_systems(
        FixedUpdate,
        (
            server_update_system,
            server_sync_players,
            server_sync_universe,
            move_players_system,
            player_edit_terrain,
        )
            .run_if(resource_exists::<RenetServer>()),
    );
    app.add_systems(Update, load_and_gen_chunks);
}

fn app_client(app: &mut App) {
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
    let (client, transport) = new_renet_client();
    app.insert_resource(client);
    app.insert_resource(transport);
    app.add_systems(Update, camera_controller_movement);
    app.add_systems(
        PreUpdate,
        (
            player_input,
            client_send_input,
            client_sync_players,
            client_sync_universe,
        )
            .run_if(client_connected()),
    );
    app.add_plugins(DebugDiagnosticPlugin);
    app.add_systems(Startup, setup)
        .add_systems(Update, cursor_grab);
}

fn setup(mut commands: Commands) {
    // ui center cursor
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(5.0),
                    height: Val::Px(5.0),
                    ..default()
                },
                background_color: Color::rgba(0.1, 0.1, 0.1, 0.3).into(),
                ..default()
            });
        });
}
