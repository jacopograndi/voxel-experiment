use bevy::{
    prelude::*,
    utils::HashSet,
    window::{PresentMode, WindowPlugin},
};

use bevy_renet::{
    client_connected,
    transport::{NetcodeClientPlugin, NetcodeServerPlugin},
    RenetClientPlugin, RenetServerPlugin,
};
use mcrs_automata::lighting::recalc_lights;
use mcrs_physics::plugin::VoxelPhysicsPlugin;
use mcrs_render::{
    boxes_world::{VoxTextureIndex, VoxTextureLoadQueue},
    voxel_world::VIEW_DISTANCE,
    VoxelRenderPlugin,
};
use mcrs_storage::{chunk::Chunk, universe::Universe, VoxelStoragePlugin, CHUNK_SIDE};
use renet::{transport::NetcodeClientTransport, ClientId, RenetServer};

mod camera;
mod diagnostics;
mod input;
mod net;
mod terrain_editing;

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
    app.insert_resource(Time::<Fixed>::from_seconds(0.01666));
    app.add_plugins((VoxelPhysicsPlugin, VoxelStoragePlugin));

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

fn setup(mut commands: Commands, mut queue: ResMut<VoxTextureLoadQueue>) {
    queue
        .to_load
        .push(("assets/voxels/stone.vox".to_string(), VoxTextureIndex(1)));
    queue
        .to_load
        .push(("assets/voxels/dirt.vox".to_string(), VoxTextureIndex(2)));
    queue
        .to_load
        .push(("assets/voxels/wood-oak.vox".to_string(), VoxTextureIndex(3)));
    queue.to_load.push((
        "assets/voxels/glowstone.vox".to_string(),
        VoxTextureIndex(4),
    ));
    queue
        .to_load
        .push(("assets/voxels/char.vox".to_string(), VoxTextureIndex(5)));

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

fn gen_chunk(pos: IVec3) -> Chunk {
    if pos.y < 0 {
        Chunk::filled()
    } else {
        Chunk::empty()
    }
}

fn get_chunks_in_sphere(pos: Vec3) -> HashSet<IVec3> {
    let load_view_distance: u32 = VIEW_DISTANCE;

    let camera_chunk_pos = (pos / CHUNK_SIDE as f32).as_ivec3() * CHUNK_SIDE as i32;
    let load_view_distance_chunk = load_view_distance as i32 / CHUNK_SIDE as i32;
    let lvdc = load_view_distance_chunk;

    // sphere centered on pos
    let mut chunks = HashSet::<IVec3>::new();
    for x in -lvdc..=lvdc {
        for y in -lvdc..=lvdc {
            for z in -lvdc..=lvdc {
                let rel = IVec3::new(x, y, z) * CHUNK_SIDE as i32;
                if rel.as_vec3().length_squared() < load_view_distance.pow(2) as f32 {
                    let pos = camera_chunk_pos + rel;
                    chunks.insert(pos);
                }
            }
        }
    }
    chunks
}

fn load_and_gen_chunks(
    mut universe: ResMut<Universe>,
    player_query: Query<(&NetPlayer, &Transform)>,
    network_mode: Res<NetworkMode>,
    transport: Option<Res<NetcodeClientTransport>>,
) {
    let client_id = if let Some(transport) = transport {
        Some(ClientId::from_raw(transport.client_id()))
    } else {
        None
    };

    let players_pos = match *network_mode {
        NetworkMode::Client => player_query
            .iter()
            .find(|(player, _)| client_id.map_or(false, |id| id == player.id))
            .map_or(vec![], |(_, tr)| vec![tr.translation]),
        _ => player_query
            .iter()
            .map(|(_, tr)| tr.translation)
            .collect::<Vec<Vec3>>(),
    };

    let mut added = HashSet::<IVec3>::new();
    for player_pos in players_pos.iter() {
        let chunks = get_chunks_in_sphere(*player_pos);
        for chunk_pos in chunks.iter() {
            if let None = universe.chunks.get(chunk_pos) {
                let chunk = gen_chunk(*chunk_pos);
                universe.chunks.insert(*chunk_pos, chunk);
                added.insert(*chunk_pos);
            }
        }
    }

    if !added.is_empty() {
        recalc_lights(&mut universe, added.into_iter().collect());
    }
}
