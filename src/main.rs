use std::{
    collections::VecDeque,
    f32::consts::PI,
    sync::{Arc, RwLock},
    time::Duration,
};

use bevy::{
    core_pipeline::fxaa::Fxaa,
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics, DiagnosticsStore, RegisterDiagnostic},
    input::mouse::MouseMotion,
    prelude::*,
    utils::HashSet,
    window::{CursorGrabMode, PresentMode, PrimaryWindow, WindowPlugin},
};

use bevy_egui::{egui, EguiContexts, EguiPlugin};

use voxel_physics::{
    character::{
        CameraController, Character, CharacterController, CharacterId, Friction, Velocity,
    },
    plugin::VoxelPhysicsPlugin,
    raycast,
};
use voxel_render::{
    boxes_world::{Ghost, VoxTextureIndex, VoxTextureLoadQueue},
    voxel_world::VIEW_DISTANCE,
    VoxelCameraBundle, VoxelRenderPlugin,
};
use voxel_storage::{
    grid::{Grid, LightType, Voxel, MAX_LIGHT},
    universe::{Chunk, GridPtr, Universe},
    VoxelStoragePlugin, CHUNK_SIDE, CHUNK_VOLUME,
};

pub const DIAGNOSTIC_FPS: DiagnosticId =
    DiagnosticId::from_u128(288146834822086093791974408528866909484);
pub const DIAGNOSTIC_FRAME_TIME: DiagnosticId =
    DiagnosticId::from_u128(54021991829115352065418785002088010278);

fn app_client(app: &mut App) {
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
    );
    app.add_plugins((
        VoxelRenderPlugin,
        VoxelPhysicsPlugin,
        VoxelStoragePlugin,
        EguiPlugin,
    ));
    app.register_diagnostic(
        Diagnostic::new(DIAGNOSTIC_FRAME_TIME, "frame_time", 1000).with_suffix("ms"),
    )
    .register_diagnostic(Diagnostic::new(DIAGNOSTIC_FPS, "fps", 1000))
    .add_systems(Startup, setup)
    .add_systems(Update, (ui, diagnostic_system, spin, cursor_grab));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let network_mode = if args.len() > 1 {
        match args[1].as_str() {
            "client" => NetworkMode::Client,
            "server" => NetworkMode::Server,
            "headless" => NetworkMode::HeadlessServer,
            _ => panic!("Invalid argument, must be \"client\", \"server\" or \"headless\"."),
        }
    } else {
        NetworkMode::Server
    };

    let mut app = App::new();
    app.init_resource::<Lobby>();
    app.insert_resource(network_mode.clone());
    app.insert_resource(Time::<Fixed>::from_seconds(0.01666));

    match network_mode {
        NetworkMode::HeadlessServer => {
            app.add_plugins((
                MinimalPlugins,
                TransformPlugin,
                RenetServerPlugin,
                NetcodeServerPlugin,
                VoxelPhysicsPlugin,
                VoxelStoragePlugin,
            ));
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
                    voxel_break,
                )
                    .run_if(resource_exists::<RenetServer>()),
            );
            app.add_systems(Update, load_and_gen_chunks);
        }
        NetworkMode::Server => {
            app.add_plugins((
                RenetServerPlugin,
                RenetClientPlugin,
                NetcodeClientPlugin,
                NetcodeServerPlugin,
            ));
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
                    voxel_break,
                )
                    .run_if(resource_exists::<RenetServer>()),
            );
            app.add_systems(Update, load_and_gen_chunks);

            app_client(&mut app);

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
        }
        NetworkMode::Client => {
            app.add_plugins(RenetClientPlugin);
            app.add_plugins(NetcodeClientPlugin);
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
            app_client(&mut app);
        }
    }

    app.run();
}

// net

use bevy_renet::{
    client_connected,
    renet::{
        transport::{ClientAuthentication, ServerAuthentication, ServerConfig},
        ConnectionConfig, DefaultChannel, RenetClient, RenetServer, ServerEvent,
    },
    transport::{NetcodeClientPlugin, NetcodeServerPlugin},
    RenetClientPlugin, RenetServerPlugin,
};
use renet::{
    transport::{NetcodeClientTransport, NetcodeServerTransport},
    ChannelConfig, ClientId, SendType,
};

use std::time::SystemTime;
use std::{collections::HashMap, net::UdpSocket};

use serde::{Deserialize, Serialize};

const PROTOCOL_ID: u64 = 7;

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
enum NetworkMode {
    /// Functions as a server, no local player
    HeadlessServer,
    /// Functions as a server, has a local player
    Server,
    Client,
}

#[derive(Debug, Default, Serialize, Deserialize, Component, Resource)]
struct PlayerInput {
    acceleration: Vec3,
    rotation_camera: f32,
    rotation_body: f32,
    jumping: bool,
    placing: bool,
    mining: bool,
    block_in_hand: u8,
}

#[derive(Debug, Component)]
pub struct Player {
    id: ClientId,
}

#[derive(Debug, Component)]
pub struct LocalPlayer;

#[derive(Debug, Default, Resource)]
pub struct Lobby {
    players: HashMap<ClientId, Entity>,
}

#[derive(Debug, Default, Resource)]
pub struct ChunkReplication {
    requested_chunks: HashMap<ClientId, HashSet<IVec3>>,
}

#[derive(Debug, Serialize, Deserialize, Component)]
enum ServerMessages {
    PlayerConnected { id: ClientId },
    PlayerDisconnected { id: ClientId },
}

pub enum ServerChannel {
    ServerMessages,
    NetworkedEntities,
    NetworkedUniverse,
}

impl From<ServerChannel> for u8 {
    fn from(channel_id: ServerChannel) -> Self {
        match channel_id {
            ServerChannel::ServerMessages => 0,
            ServerChannel::NetworkedEntities => 1,
            ServerChannel::NetworkedUniverse => 2,
        }
    }
}

impl ServerChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ChannelConfig {
                channel_id: Self::ServerMessages.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::NetworkedEntities.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::Unreliable,
            },
            ChannelConfig {
                channel_id: Self::NetworkedUniverse.into(),
                max_memory_usage_bytes: 100 * (CHUNK_VOLUME * 4),
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
        ]
    }
}

pub fn connection_config() -> ConnectionConfig {
    ConnectionConfig {
        available_bytes_per_tick: 1024 * 1024,
        server_channels_config: ServerChannel::channels_config(),
        ..default()
    }
}

fn new_renet_client() -> (RenetClient, NetcodeClientTransport) {
    let server_addr = "127.0.0.1:5000".parse().unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let client_id = current_time.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: PROTOCOL_ID,
        server_addr,
        user_data: None,
    };

    let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();
    let client = RenetClient::new(connection_config());

    (client, transport)
}

fn new_renet_server() -> (RenetServer, NetcodeServerTransport) {
    let public_addr = "127.0.0.1:5000".parse().unwrap();
    let socket = UdpSocket::bind(public_addr).unwrap();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let server_config = ServerConfig {
        current_time,
        max_clients: 64,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };

    let transport = NetcodeServerTransport::new(server_config, socket).unwrap();
    let server = RenetServer::new(connection_config());

    (server, transport)
}

fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    mut commands: Commands,
    mut lobby: ResMut<Lobby>,
    mut server: ResMut<RenetServer>,
    network_mode: Res<NetworkMode>,
    transport: Option<Res<NetcodeClientTransport>>,
    mut chunk_replication: ResMut<ChunkReplication>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("Player {} connected.", client_id);
                let is_local_player = if let Some(local_id) = transport
                    .as_ref()
                    .map(|t| ClientId::from_raw(t.client_id()))
                {
                    local_id == *client_id
                } else {
                    true
                };
                let spawn_point = Vec3::new(0.0, 0.0, 0.0);
                // player character
                let player_entity = commands
                    .spawn((
                        SpatialBundle::from_transform(Transform::from_translation(spawn_point)),
                        Character {
                            id: CharacterId(0),
                            size: Vec3::new(0.5, 1.99, 0.5),
                            air_speed: 0.001,
                            ground_speed: 0.03,
                            jump_strenght: 0.17,
                        },
                        CharacterController {
                            acceleration: Vec3::splat(0.0),
                            jumping: false,
                            ..default()
                        },
                        Velocity::default(),
                        Friction {
                            air: Vec3::splat(0.99),
                            ground: Vec3::splat(0.78),
                        },
                        Player { id: *client_id },
                        PlayerInput::default(),
                    ))
                    .with_children(|parent| {
                        let mut camera_pivot =
                            parent.spawn((Fxaa::default(), CameraController::default()));
                        if is_local_player && matches!(*network_mode, NetworkMode::Server) {
                            camera_pivot.insert(VoxelCameraBundle {
                                transform: Transform::from_xyz(0.0, 0.5, 0.0),
                                projection: Projection::Perspective(PerspectiveProjection {
                                    fov: 1.57,
                                    ..default()
                                }),
                                ..default()
                            });
                        } else {
                            camera_pivot.insert(SpatialBundle {
                                transform: Transform::from_xyz(0.0, 0.5, 0.0),
                                ..default()
                            });
                        }
                    })
                    .id();
                if is_local_player && matches!(*network_mode, NetworkMode::Server) {
                    commands.entity(player_entity).insert(LocalPlayer);
                }

                // We could send an InitState with all the players id and positions for the client
                // but this is easier to do.
                for &player_id in lobby.players.keys() {
                    let message =
                        bincode::serialize(&ServerMessages::PlayerConnected { id: player_id })
                            .unwrap();
                    server.send_message(*client_id, ServerChannel::ServerMessages, message);
                }

                if !(is_local_player && matches!(*network_mode, NetworkMode::Server)) {
                    chunk_replication
                        .requested_chunks
                        .insert(*client_id, get_chunks_in_sphere(spawn_point));
                }

                lobby.players.insert(*client_id, player_entity);

                let message =
                    bincode::serialize(&ServerMessages::PlayerConnected { id: *client_id })
                        .unwrap();
                server.broadcast_message(ServerChannel::ServerMessages, message);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("Player {} disconnected: {}", client_id, reason);
                if let Some(player_entity) = lobby.players.remove(client_id) {
                    commands.entity(player_entity).despawn_recursive();
                }

                chunk_replication.requested_chunks.remove(client_id);

                let message =
                    bincode::serialize(&ServerMessages::PlayerDisconnected { id: *client_id })
                        .unwrap();
                server.broadcast_message(ServerChannel::ServerMessages, message);
            }
        }
    }

    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            let player_input: PlayerInput = bincode::deserialize(&message).unwrap();
            if let Some(player_entity) = lobby.players.get(&client_id) {
                commands.entity(*player_entity).insert(player_input);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlayerState {
    position: Vec3,
    rotation_body: f32,
    rotation_camera: f32,
}

fn server_sync_players(
    mut server: ResMut<RenetServer>,
    transforms: Query<&Transform>,
    query: Query<(Entity, &Player, &Children)>,
) {
    let mut players: HashMap<ClientId, PlayerState> = HashMap::new();
    for (entity, player, children) in query.iter() {
        let tr = transforms.get(entity).unwrap();
        let camera_entity = children.iter().next().unwrap();
        let tr_camera = transforms.get(*camera_entity).unwrap();
        let playerstate = PlayerState {
            position: tr.translation,
            rotation_camera: tr_camera.rotation.to_euler(EulerRot::YXZ).1,
            rotation_body: tr.rotation.to_euler(EulerRot::YXZ).0,
        };
        players.insert(player.id, playerstate);
    }

    let sync_message = bincode::serialize(&players).unwrap();
    server.broadcast_message(ServerChannel::NetworkedEntities, sync_message);
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct SyncUniverse {
    chunks: Vec<(IVec3, Vec<u8>)>,
    heightfield: Vec<(IVec2, i32)>,
}

fn server_sync_universe(
    mut server: ResMut<RenetServer>,
    mut universe: ResMut<Universe>,
    mut chunk_replication: ResMut<ChunkReplication>,
) {
    let mut changed_chunks = HashSet::<IVec3>::new();
    for (pos, chunk) in universe.chunks.iter_mut() {
        if chunk.to_replicate {
            chunk.reset_to_replicate();
            changed_chunks.insert(*pos);
        }
    }

    for (_, chunks) in chunk_replication.requested_chunks.iter_mut() {
        chunks.extend(changed_chunks.clone());
    }

    for (client_id, chunks) in chunk_replication.requested_chunks.iter_mut() {
        let channel_size =
            server.channel_available_memory(*client_id, ServerChannel::NetworkedUniverse) as i32;
        let mut available_bytes = channel_size;

        let mut sync = SyncUniverse::default();
        let chunk_size = (CHUNK_VOLUME * 4 + 12) as i32;

        let mut sent_chunks = HashSet::<IVec3>::new();

        for chunk_pos in chunks.iter() {
            if let Some(chunk) = universe.chunks.get(chunk_pos) {
                if available_bytes > chunk_size {
                    available_bytes -= chunk_size;
                    let grid = chunk.grid.0.read().unwrap();
                    let data = grid.to_bytes().iter().cloned().collect();
                    sync.chunks.push((*chunk_pos, data));
                    sent_chunks.insert(*chunk_pos);
                }
            }
        }

        if !sent_chunks.is_empty() {
            let sync_message = bincode::serialize(&sync).unwrap();
            println!("sending dirty universe: {}", sync_message.len());
            server.send_message(*client_id, ServerChannel::NetworkedUniverse, sync_message);
            *chunks = chunks.difference(&sent_chunks).cloned().collect();
        }
    }
}

fn client_sync_players(
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<Lobby>,
    transport: Res<NetcodeClientTransport>,
    query: Query<(Entity, &Player, &Children)>,
    mut query_transform: Query<&mut Transform>,
    network_mode: Res<NetworkMode>,
) {
    while let Some(message) = client.receive_message(ServerChannel::ServerMessages) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerConnected { id } => {
                println!("Player {} connected. This client is ", id,);
                let spawn_point = Vec3::new(0.0, 0.0, 0.0);
                let is_local_player = id == ClientId::from_raw(transport.client_id());
                if !(is_local_player && matches!(*network_mode, NetworkMode::Server)) {
                    let player_entity = commands
                        .spawn((
                            SpatialBundle::from_transform(Transform::from_translation(spawn_point)),
                            Character {
                                id: CharacterId(0),
                                size: Vec3::new(0.5, 1.99, 0.5),
                                air_speed: 0.001,
                                ground_speed: 0.03,
                                jump_strenght: 0.17,
                            },
                            CharacterController {
                                acceleration: Vec3::splat(0.0),
                                jumping: false,
                                ..default()
                            },
                            Velocity::default(),
                            Friction {
                                air: Vec3::splat(0.99),
                                ground: Vec3::splat(0.78),
                            },
                            Player { id },
                        ))
                        .with_children(|parent| {
                            let mut camera_pivot =
                                parent.spawn((Fxaa::default(), CameraController::default()));
                            if is_local_player {
                                camera_pivot.insert(VoxelCameraBundle {
                                    transform: Transform::from_xyz(0.0, 0.5, 0.0),
                                    projection: Projection::Perspective(PerspectiveProjection {
                                        fov: 1.57,
                                        ..default()
                                    }),
                                    ..default()
                                });
                            } else {
                                camera_pivot.insert(SpatialBundle {
                                    transform: Transform::from_xyz(0.0, 0.5, 0.0),
                                    ..default()
                                });
                            }
                        })
                        .id();
                    if !is_local_player {
                        commands.entity(player_entity).with_children(|parent| {
                            parent.spawn((
                                SpatialBundle::from_transform(Transform {
                                    scale: Vec3::new(16.0, 32.0, 8.0) / 16.0,
                                    ..default()
                                }),
                                Ghost {
                                    vox_texture_index: VoxTextureIndex(5),
                                },
                            ));
                        });
                    } else {
                        commands.entity(player_entity).insert(LocalPlayer);
                    }

                    lobby.players.insert(id, player_entity);
                }
            }
            ServerMessages::PlayerDisconnected { id } => {
                println!("Player {} disconnected.", id);
                if let Some(player_entity) = lobby.players.remove(&id) {
                    commands.entity(player_entity).despawn_recursive();
                }
            }
        }
    }

    while let Some(message) = client.receive_message(ServerChannel::NetworkedEntities) {
        let players: HashMap<ClientId, PlayerState> = bincode::deserialize(&message).unwrap();
        for (player_id, playerstate) in players.iter() {
            let is_local_player = *player_id == ClientId::from_raw(transport.client_id());
            if let Some(player_entity) = lobby.players.get(player_id) {
                if let Ok((_, _, children)) = query.get(*player_entity) {
                    let camera_entity = children.iter().next().unwrap(); // todo find camera
                    let mut tr = query_transform.get_mut(*player_entity).unwrap();
                    if !is_local_player {
                        tr.translation = playerstate.position;
                        tr.rotation = Quat::from_axis_angle(Vec3::Y, playerstate.rotation_body);
                        let mut tr_camera = query_transform.get_mut(*camera_entity).unwrap();
                        tr_camera.rotation =
                            Quat::from_axis_angle(Vec3::X, playerstate.rotation_camera);
                    } else if matches!(*network_mode, NetworkMode::Client) {
                        tr.translation = playerstate.position;
                    }
                }
            }
        }
    }
}

fn client_sync_universe(mut client: ResMut<RenetClient>, mut universe: ResMut<Universe>) {
    while let Some(message) = client.receive_message(ServerChannel::NetworkedUniverse) {
        let server_message: SyncUniverse = bincode::deserialize(&message).unwrap();
        println!("{:?}", server_message.chunks.len());
        for (pos, chunk_bytes) in server_message.chunks.iter() {
            if let Some(chunk) = universe.chunks.get_mut(pos) {
                chunk.to_render = true;
                let mut grid = chunk.grid.0.write().unwrap();
                for i in 0..chunk_bytes.len() / 4 {
                    let voxel = Voxel {
                        id: chunk_bytes[i * 4],
                        flags: chunk_bytes[i * 4 + 1],
                        light0: chunk_bytes[i * 4 + 2],
                        light1: chunk_bytes[i * 4 + 3],
                    };
                    grid.set_at(Grid::index_to_xyz(i), voxel);
                }
            } else {
                let mut grid = Grid::empty();
                for i in 0..chunk_bytes.len() / 4 {
                    let voxel = Voxel {
                        id: chunk_bytes[i * 4],
                        flags: chunk_bytes[i * 4 + 1],
                        light0: chunk_bytes[i * 4 + 2],
                        light1: chunk_bytes[i * 4 + 3],
                    };
                    grid.set_at(Grid::index_to_xyz(i), voxel);
                }
                universe.chunks.insert(
                    *pos,
                    Chunk {
                        grid: GridPtr(Arc::new(RwLock::new(grid))),
                        to_render: true,
                        to_replicate: false,
                    },
                );
            }
        }
    }
}

fn player_input(
    mut player_input: ResMut<PlayerInput>,
    keys: Res<Input<KeyCode>>,
    query_transform: Query<&Transform>,
    query_camera: Query<(Entity, &Camera, &Parent)>,
    mouse: Res<Input<MouseButton>>,
    mut query_player: Query<&mut CharacterController, With<LocalPlayer>>,
    network_mode: Res<NetworkMode>,
) {
    if let Ok((entity, _, parent)) = query_camera.get_single() {
        let tr_camera = query_transform.get(entity).unwrap();
        let tr_body = query_transform.get(parent.get()).unwrap();
        player_input.rotation_camera = tr_camera.rotation.to_euler(EulerRot::YXZ).1;
        player_input.rotation_body = tr_body.rotation.to_euler(EulerRot::YXZ).0;
    }
    let mut delta = Vec3::ZERO;
    if keys.pressed(KeyCode::W) {
        delta += Vec3::X;
    }
    if keys.pressed(KeyCode::S) {
        delta -= Vec3::X;
    }
    if keys.pressed(KeyCode::A) {
        delta += Vec3::Z;
    }
    if keys.pressed(KeyCode::D) {
        delta -= Vec3::Z;
    }
    delta = delta.normalize_or_zero();
    player_input.acceleration = delta;
    if keys.pressed(KeyCode::Space) {
        player_input.jumping = true;
    } else {
        player_input.jumping = false;
    }
    if mouse.pressed(MouseButton::Right) {
        player_input.placing = true;
    } else {
        player_input.placing = false;
    }
    if mouse.pressed(MouseButton::Left) {
        player_input.mining = true;
    } else {
        player_input.mining = false;
    }
    if matches!(*network_mode, NetworkMode::Server) {
        if let Ok(mut controller) = query_player.get_single_mut() {
            controller.acceleration = player_input.acceleration;
            controller.jumping = player_input.jumping;
        }
    }
}

fn client_send_input(player_input: Res<PlayerInput>, mut client: ResMut<RenetClient>) {
    let input_message = bincode::serialize(&*player_input).unwrap();
    client.send_message(DefaultChannel::ReliableOrdered, input_message);
}

fn move_players_system(
    mut query_player: Query<
        (
            Entity,
            &mut CharacterController,
            &PlayerInput,
            &mut Transform,
            Option<&LocalPlayer>,
        ),
        Without<CameraController>,
    >,
    mut query_camera: Query<
        (&CameraController, &Parent, &mut Transform),
        Without<CharacterController>,
    >,
) {
    for (_, parent, mut tr_camera) in query_camera.iter_mut() {
        if let Ok((_, mut controller, input, mut tr, localplayer)) =
            query_player.get_mut(parent.get())
        {
            if localplayer.is_none() {
                controller.acceleration = input.acceleration;
                controller.jumping = input.jumping;
                tr_camera.rotation = Quat::from_axis_angle(Vec3::X, input.rotation_camera);
                tr.rotation = Quat::from_axis_angle(Vec3::Y, input.rotation_body);
            }
        }
    }
}

// net end

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

    // center cursor
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

    commands.spawn((
        SpatialBundle::from_transform(Transform {
            translation: Vec3::new(0.0, 13.0 / 16.0 * 0.5, 0.0),
            ..default()
        }),
        Ghost {
            vox_texture_index: VoxTextureIndex(1),
        },
    ));

    commands.spawn((
        SpatialBundle::from_transform(Transform {
            translation: Vec3::new(3.0, 14.0 / 16.0 * 0.5, -2.0),
            rotation: Quat::from_rotation_y(PI / 2.0),
            ..default()
        }),
        Ghost {
            vox_texture_index: VoxTextureIndex(2),
        },
        Party::default(),
    ));
}

// just for prototype
fn voxel_break(
    camera_query: Query<(&CameraController, &GlobalTransform, &Parent)>,
    player_query: Query<&PlayerInput>,
    mut universe: ResMut<Universe>,
) {
    for (_cam, tr, parent) in camera_query.iter() {
        let Ok(input) = player_query.get(parent.get()) else {
            continue;
        };
        #[derive(PartialEq)]
        enum Act {
            PlaceBlock,
            RemoveBlock,
            Inspect,
        }
        let act = match (input.placing, input.mining, false) {
            (true, _, _) => Some(Act::PlaceBlock),
            (_, true, _) => Some(Act::RemoveBlock),
            (_, _, true) => Some(Act::Inspect),
            _ => None,
        };
        if let Some(act) = act {
            if let Some(hit) = raycast::raycast(tr.translation(), tr.forward(), 4.5, &universe) {
                match act {
                    Act::Inspect => {
                        println!(
                            "hit(pos:{}, block:{:?}, dist:{}), head(block:{:?})",
                            hit.pos,
                            universe.get_at(&hit.grid_pos),
                            hit.distance,
                            universe.get_at(&tr.translation().floor().as_ivec3()),
                        );
                    }
                    Act::RemoveBlock => {
                        println!("removed block");

                        let pos = hit.grid_pos;

                        let mut light_suns = vec![];
                        let mut light_torches = vec![];

                        if let Some(voxel) = universe.get_at(&pos) {
                            // todo: use BlockInfo.is_light_source
                            if voxel.id == 3 {
                                let new = propagate_darkness(&mut universe, pos, LightType::Torch);
                                propagate_light(&mut universe, new, LightType::Torch)
                            }
                        }

                        universe.set_at(
                            &pos,
                            Voxel {
                                id: 0,
                                flags: 0,
                                ..default()
                            },
                        );

                        let planar = IVec2::new(pos.x, pos.z);
                        if let Some(height) = universe.heightfield.get(&planar) {
                            if pos.y == *height {
                                // recalculate the highest sunlit point
                                let mut beam = pos.y - 100;
                                for y in 0..=100 {
                                    let h = pos.y - y;
                                    let sample = IVec3::new(pos.x, h, pos.z);
                                    if let Some(voxel) = universe.get_at(&sample) {
                                        if voxel.is_opaque() {
                                            beam = h;
                                            break;
                                        } else {
                                            light_suns.push(sample);

                                            let mut lit = voxel.clone();
                                            lit.set_light(LightType::Sun, 15);
                                            universe.set_at(&sample, lit);
                                        }
                                    }
                                }
                                universe.heightfield.insert(planar, beam);
                            }
                        }

                        for dir in DIRS.iter() {
                            let sample = pos + *dir;
                            if let Some(voxel) = universe.get_at(&sample) {
                                if !voxel.is_opaque() {
                                    if voxel.get_light(LightType::Sun) > 1 {
                                        light_suns.push(sample);
                                    }
                                    if voxel.get_light(LightType::Torch) > 1 {
                                        light_torches.push(sample);
                                    }
                                }
                            }
                        }

                        propagate_light(&mut universe, light_suns, LightType::Sun);
                        propagate_light(&mut universe, light_torches, LightType::Torch);
                    }
                    Act::PlaceBlock => {
                        println!("placed block");

                        let pos = hit.grid_pos + hit.normal;

                        let mut dark_suns = vec![];

                        //if keys.pressed(KeyCode::Key3) {
                        if false {
                            // todo: use BlockInfo
                            universe.set_at(
                                &pos,
                                Voxel {
                                    id: 3,
                                    flags: 2,
                                    light0: 14,
                                    ..default()
                                },
                            );
                            propagate_light(&mut universe, vec![pos], LightType::Torch)
                        } else {
                            let new = propagate_darkness(&mut universe, pos, LightType::Torch);

                            universe.set_at(
                                &pos,
                                Voxel {
                                    id: 1,
                                    flags: 3,
                                    ..default()
                                },
                            );

                            propagate_light(&mut universe, new, LightType::Torch);
                        }

                        let planar = IVec2::new(pos.x, pos.z);
                        if let Some(height) = universe.heightfield.get(&planar) {
                            if pos.y > *height {
                                // recalculate the highest sunlit point
                                for y in (*height)..pos.y {
                                    let sample = IVec3::new(pos.x, y, pos.z);
                                    dark_suns.push(sample);
                                }
                                universe.heightfield.insert(planar, pos.y);
                            }
                        }

                        for sun in dark_suns {
                            let new = propagate_darkness(&mut universe, sun, LightType::Sun);
                            propagate_light(&mut universe, new, LightType::Sun)
                        }
                    }
                };
            } else {
                //dbg!("no hit");
            }
        }
    }
}

fn gen_chunk(pos: IVec3) -> GridPtr {
    let grid = if pos.y < 0 {
        Grid::filled()
    } else {
        Grid::empty()
    };
    GridPtr(Arc::new(RwLock::new(grid)))
}

fn recalc_lights(universe: &mut Universe, chunks: Vec<IVec3>) {
    println!("lighting {:?} chunks", chunks.len());

    // calculate sunlight beams
    let mut suns: Vec<IVec3> = vec![];
    let mut planars = HashSet::<IVec2>::new();
    let mut highest = i32::MIN;
    for pos in chunks.iter() {
        let chunk = universe.chunks.get_mut(pos).unwrap();
        chunk.set_dirty();
        let mut grid = chunk.grid.0.write().unwrap();
        for x in 0..CHUNK_SIDE {
            for z in 0..CHUNK_SIDE {
                let mut sunlight = MAX_LIGHT;
                for y in (0..CHUNK_SIDE).rev() {
                    let xyz = IVec3::new(x as i32, y as i32, z as i32);
                    let voxel = grid.get_at_mut(xyz);
                    if voxel.is_opaque() {
                        sunlight = 0;
                    }
                    if sunlight > 0 {
                        suns.push(*pos + xyz);
                    }
                    voxel.set_light(LightType::Sun, sunlight);
                    voxel.set_light(LightType::Torch, 0);
                    highest = highest.max(pos.y + y as i32);
                }
                let planar = IVec2::new(x as i32 + pos.x, z as i32 + pos.z);
                planars.insert(planar);
            }
        }
    }

    for planar in planars.iter() {
        let mut beam = 0;
        let mut block_found = false;
        for y in 0..1000 {
            let h = highest - y;
            let sample = IVec3::new(planar.x, h, planar.y);

            if let Some(voxel) = universe.get_at(&sample) {
                block_found = true;
                if voxel.is_opaque() {
                    beam = h;
                    break;
                }
            } else {
                if block_found {
                    break;
                }
            }
        }
        if let Some(height) = universe.heightfield.get_mut(planar) {
            *height = (*height).min(beam);
        } else {
            universe.heightfield.insert(*planar, beam);
        }
    }

    // find new light sources
    let mut torches: Vec<IVec3> = vec![];
    for pos in chunks.iter() {
        let chunk = universe.chunks.get(pos).unwrap();
        let mut grid = chunk.grid.0.write().unwrap();
        for i in 0..CHUNK_VOLUME {
            let xyz = Grid::index_to_xyz(i);
            let voxel = grid.get_at_mut(xyz);
            // todo: fetch from BlockInfo when implemented
            if voxel.id == 3 {
                torches.push(*pos + xyz);
                voxel.set_light(LightType::Torch, 15);
            }
        }
    }

    if !suns.is_empty() {
        propagate_light(universe, suns, LightType::Sun);
    }

    if !torches.is_empty() {
        propagate_light(universe, torches, LightType::Torch);
    }
}

const DIRS: [IVec3; 6] = [
    IVec3::X,
    IVec3::Y,
    IVec3::Z,
    IVec3::NEG_X,
    IVec3::NEG_Y,
    IVec3::NEG_Z,
];
const MAX_LIGHTITNG_PROPAGATION: usize = 100000000;

fn propagate_darkness(universe: &mut Universe, source: IVec3, lt: LightType) -> Vec<IVec3> {
    let voxel = universe.get_at(&source).unwrap();
    let val = voxel.get_light(lt);
    let mut dark = voxel.clone();
    dark.set_light(lt, 0);
    universe.set_at(&source, dark);

    println!("1 source of {lt} darkness val:{val}");

    let mut new_lights: Vec<IVec3> = vec![];
    let mut frontier: VecDeque<IVec3> = [source].into();
    for iter in 0..MAX_LIGHTITNG_PROPAGATION {
        if let Some(pos) = frontier.pop_front() {
            for dir in DIRS.iter() {
                let target = pos + *dir;
                let mut unlit: Option<Voxel> = None;
                if let Some(neighbor) = universe.get_at(&target) {
                    let target_light = neighbor.get_light(lt);
                    if target_light != 0 && target_light < val {
                        let mut l = neighbor;
                        l.set_light(lt, 0);
                        unlit = Some(l);
                    } else if target_light >= val {
                        new_lights.push(target);
                    }
                }
                if let Some(voxel) = unlit {
                    universe.set_at(&target, voxel);
                    frontier.push_back(target);
                    let (c, _) = universe.pos_to_chunk_and_inner(&target);
                    universe.chunks.get_mut(&c).unwrap().set_dirty();
                }
            }
        } else {
            println!("{} iters for {lt} darkness", iter);
            break;
        }
    }
    new_lights
}

fn propagate_light(universe: &mut Universe, sources: Vec<IVec3>, lt: LightType) {
    const DIRS: [IVec3; 6] = [
        IVec3::X,
        IVec3::Y,
        IVec3::Z,
        IVec3::NEG_X,
        IVec3::NEG_Y,
        IVec3::NEG_Z,
    ];
    const MAX_LIGHTITNG_PROPAGATION: usize = 100000000;

    println!("{} sources of {lt} light", sources.len());
    let mut frontier: VecDeque<IVec3> = sources.clone().into();
    for iter in 0..MAX_LIGHTITNG_PROPAGATION {
        if let Some(pos) = frontier.pop_front() {
            let voxel = universe.get_at(&pos).unwrap();
            let light = voxel.get_light(lt);
            for dir in DIRS.iter() {
                let target = pos + *dir;
                let mut lit: Option<Voxel> = None;
                if let Some(neighbor) = universe.get_at(&target) {
                    if !neighbor.is_opaque() && neighbor.get_light(lt) + 2 <= light {
                        let mut l = neighbor;
                        l.set_light(lt, light - 1);
                        lit = Some(l);
                    }
                }
                if let Some(voxel) = lit {
                    universe.set_at(&target, voxel);
                    frontier.push_back(target);
                    let (c, _) = universe.pos_to_chunk_and_inner(&target);
                    universe.chunks.get_mut(&c).unwrap().set_dirty();
                }
            }
        } else {
            println!("{} iters for {lt} light", iter);
            break;
        }
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
    player_query: Query<(&Player, &Transform)>,
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
                let grid_ptr = gen_chunk(*chunk_pos);
                universe.chunks.insert(
                    *chunk_pos,
                    Chunk {
                        grid: grid_ptr,
                        to_render: true,
                        to_replicate: true,
                    },
                );
                added.insert(*chunk_pos);
            }
        }
    }

    if !added.is_empty() {
        recalc_lights(&mut universe, added.into_iter().collect());
    }
}

#[derive(Component, Clone, Default, Debug)]
struct Party {
    scale: Option<Vec3>,
}

fn spin(mut q: Query<(&mut Transform, &mut Party)>, time: Res<Time<Real>>) {
    for (mut tr, mut party) in q.iter_mut() {
        tr.rotate_y(0.1);
        if let None = party.scale {
            party.scale = Some(tr.scale)
        }
        tr.scale = party.scale.unwrap() * f32::cos(time.elapsed_seconds());
    }
}

fn ui(mut contexts: EguiContexts, diagnostics: Res<DiagnosticsStore>) {
    egui::Window::new("Debug menu").show(contexts.ctx_mut(), |ui| {
        if let Some(value) = diagnostics
            .get(DIAGNOSTIC_FPS)
            .and_then(|fps| fps.smoothed())
        {
            ui.label(format!("fps: {value:>4.2}"));
            use egui_plot::{Line, PlotPoints};
            let n = 1000;
            let line_points: PlotPoints = if let Some(diag) = diagnostics.get(DIAGNOSTIC_FPS) {
                diag.values()
                    .take(n)
                    .enumerate()
                    .map(|(i, v)| [i as f64, *v])
                    .collect()
            } else {
                PlotPoints::default()
            };
            let line = Line::new(line_points).fill(0.0);
            egui_plot::Plot::new("fps")
                .include_y(0.0)
                .height(70.0)
                .show_axes([false, true])
                .show(ui, |plot_ui| plot_ui.line(line))
                .response;
        } else {
            ui.label("no fps data");
        }
        if let Some(value) = diagnostics
            .get(DIAGNOSTIC_FRAME_TIME)
            .and_then(|ms| ms.value())
        {
            ui.label(format!("time: {value:>4.2} ms"));
            use egui_plot::{Line, PlotPoints};
            let n = 1000;
            let line_points: PlotPoints = if let Some(diag) = diagnostics.get(DIAGNOSTIC_FRAME_TIME)
            {
                diag.values()
                    .take(n)
                    .enumerate()
                    .map(|(i, v)| [i as f64, *v])
                    .collect()
            } else {
                PlotPoints::default()
            };
            let line = Line::new(line_points).fill(0.0);
            egui_plot::Plot::new("frame time")
                .include_y(0.0)
                .height(70.0)
                .show_axes([false, true])
                .show(ui, |plot_ui| plot_ui.line(line))
                .response;
        } else {
            ui.label("no frame time data");
        }
        ui.separator()
    });
}

pub fn diagnostic_system(mut diagnostics: Diagnostics, time: Res<Time<Real>>) {
    let delta_seconds = time.delta_seconds_f64();
    if delta_seconds == 0.0 {
        return;
    }
    diagnostics.add_measurement(DIAGNOSTIC_FRAME_TIME, || delta_seconds * 1000.0);
    diagnostics.add_measurement(DIAGNOSTIC_FPS, || 1.0 / delta_seconds);
}

pub fn camera_controller_movement(
    mut camera_query: Query<(&CameraController, &mut Transform, &Parent)>,
    mut character_query: Query<
        (&Character, &mut Transform, &CharacterController),
        Without<CameraController>,
    >,
    mut mouse_motion: EventReader<MouseMotion>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    query_local: Query<&LocalPlayer>,
) {
    let Ok(window) = primary_window.get_single() else {
        return;
    };
    for (camera_controller, mut camera_tr, parent) in camera_query.iter_mut() {
        if query_local.get(parent.get()).is_err() {
            continue;
        }
        let Ok((_character, mut parent_tr, _character_controller)) =
            character_query.get_mut(parent.get())
        else {
            continue;
        };
        for ev in mouse_motion.read() {
            let (mut yaw, _, _) = parent_tr.rotation.to_euler(EulerRot::YXZ);
            let (_, mut pitch, _) = camera_tr.rotation.to_euler(EulerRot::YXZ);
            match window.cursor.grab_mode {
                CursorGrabMode::None => (),
                _ => {
                    // Using smallest of height or width ensures equal vertical and horizontal sensitivity
                    let window_scale = window.height().min(window.width());
                    pitch -=
                        (camera_controller.sensitivity.y * ev.delta.y * window_scale).to_radians();
                    yaw -=
                        (camera_controller.sensitivity.x * ev.delta.x * window_scale).to_radians();
                }
            }
            pitch = pitch.clamp(-1.54, 1.54);
            parent_tr.rotation = Quat::from_axis_angle(Vec3::Y, yaw);
            camera_tr.rotation = Quat::from_axis_angle(Vec3::X, pitch);
        }
    }
}

/// Grabs the cursor when game first starts
pub fn initial_grab_cursor(mut primary_window: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = primary_window.get_single_mut() {
        toggle_grab_cursor(&mut window);
    } else {
        warn!("Primary window not found for `initial_grab_cursor`!");
    }
}

/// Grabs/ungrabs mouse cursor
pub fn cursor_grab(
    keys: Res<Input<KeyCode>>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok(mut window) = primary_window.get_single_mut() {
        if keys.just_pressed(KeyCode::Escape) {
            toggle_grab_cursor(&mut window);
        }
    } else {
        warn!("Primary window not found for `cursor_grab`!");
    }
}

fn toggle_grab_cursor(window: &mut Window) {
    match window.cursor.grab_mode {
        CursorGrabMode::None => {
            window.cursor.grab_mode = CursorGrabMode::Confined;
            window.cursor.visible = false;
        }
        _ => {
            window.cursor.grab_mode = CursorGrabMode::None;
            window.cursor.visible = true;
        }
    }
}
