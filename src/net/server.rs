use std::{
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};

use bevy::{
    core_pipeline::fxaa::Fxaa,
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_renet::renet::{
    transport::{ServerAuthentication, ServerConfig},
    DefaultChannel, RenetServer, ServerEvent,
};
use mcrs_blueprints::Blueprints;
use mcrs_physics::character::{
    CameraController, Character, CharacterController, CharacterId, Friction, Velocity,
};
use mcrs_render::{
    boxes_world::{Ghost, LoadedVoxTextures},
    camera::VoxelCameraBundle,
};
use mcrs_storage::{universe::Universe, CHUNK_VOLUME};
use renet::{
    transport::{NetcodeClientTransport, NetcodeServerTransport},
    ClientId,
};

use crate::{
    input::PlayerInput,
    net::{LocalPlayer, NetPlayer, ServerChannel, ServerMessages},
    terrain_generation::get_chunks_in_sphere,
};

use super::{
    connection_config, ChunkReplication, Lobby, NetworkMode, PlayerState, SyncUniverse, PORT,
    PROTOCOL_ID,
};

const SERVER_TICKS_PER_SECOND: u32 = 60;

pub fn server_refresh_time() -> bevy::prelude::Time<bevy::prelude::Fixed> {
    Time::<Fixed>::from_seconds(1. / (SERVER_TICKS_PER_SECOND as f64))
}

pub fn new_renet_server(addr: &str) -> (RenetServer, NetcodeServerTransport) {
    let bind_addr: SocketAddr = ("0.0.0.0:".to_string() + &PORT.to_string())
        .parse()
        .unwrap();
    let public_addr = (addr.to_string() + ":" + &PORT.to_string())
        .parse()
        .unwrap();
    let socket = UdpSocket::bind(bind_addr).unwrap();
    let duration_since = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
    let current_time = duration_since.unwrap();
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

pub fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    mut commands: Commands,
    mut lobby: ResMut<Lobby>,
    mut server: ResMut<RenetServer>,
    network_mode: Res<NetworkMode>,
    transport: Option<Res<NetcodeClientTransport>>,
    mut chunk_replication: ResMut<ChunkReplication>,
    mut player_input_query: Query<&mut PlayerInput>,
    loaded_textures: Option<Res<LoadedVoxTextures>>,
    info: Res<Blueprints>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                let is_local_player = if let Some(local_id) = transport
                    .as_ref()
                    .map(|t| ClientId::from_raw(t.client_id()))
                {
                    local_id == *client_id
                } else {
                    true
                };

                if is_local_player {
                    debug!(target: "net_server", "Connected to the server (our id = {})", client_id);
                } else {
                    debug!(target: "net_server", "New player connected with id = {}", client_id);
                }

                let spawn_point = Vec3::new(0.0, 5.0, 0.0);
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
                        NetPlayer { id: *client_id },
                        PlayerInput::default(),
                    ))
                    .with_children(|parent| {
                        let mut camera_pivot =
                            parent.spawn((Fxaa::default(), CameraController::default()));
                        if is_local_player && matches!(*network_mode, NetworkMode::ClientAndServer)
                        {
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
                if !is_local_player && !matches!(*network_mode, NetworkMode::Server) {
                    if let Some(loaded_textures) = loaded_textures.as_ref() {
                        commands.entity(player_entity).with_children(|parent| {
                            parent.spawn((
                                SpatialBundle::from_transform(Transform {
                                    scale: Vec3::new(16.0, 32.0, 8.0) / 16.0,
                                    ..default()
                                }),
                                Ghost {
                                    vox_texture_index: loaded_textures
                                        .ghosts_id
                                        .get(&info.ghosts.get_named("Steve").id)
                                        .unwrap()
                                        .clone(),
                                },
                            ));
                        });
                    }
                } else if matches!(*network_mode, NetworkMode::ClientAndServer) {
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

                if !(is_local_player && matches!(*network_mode, NetworkMode::ClientAndServer)) {
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
                let is_local_player = if let Some(local_id) = transport
                    .as_ref()
                    .map(|t| ClientId::from_raw(t.client_id()))
                {
                    local_id == *client_id
                } else {
                    true
                };

                if is_local_player {
                    debug!(target: "net_server", "Disconnected from the server: {}", reason);
                } else {
                    debug!(target: "net_server", "Player {} disconnected: {}", client_id, reason);
                }
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
                if let Ok(mut current_player_input) = player_input_query.get_mut(*player_entity) {
                    current_player_input.update(player_input);
                }
            }
        }
    }
}

pub fn server_sync_players(
    mut server: ResMut<RenetServer>,
    transforms: Query<&Transform>,
    query: Query<(Entity, &NetPlayer, &Children)>,
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

pub fn server_sync_universe(
    mut server: ResMut<RenetServer>,
    mut universe: ResMut<Universe>,
    mut chunk_replication: ResMut<ChunkReplication>,
) {
    let mut changed_chunks = HashSet::<IVec3>::new();
    for (pos, chunk) in universe.chunks.iter_mut() {
        if chunk.dirty_replication {
            chunk.dirty_replication = false;
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
                    let read = chunk.get_ref();
                    let slice = bytemuck::cast_slice(read.as_ref());
                    sync.chunks
                        .push((*chunk_pos, slice.iter().cloned().collect()));
                    sent_chunks.insert(*chunk_pos);
                }
            }
        }

        if !sent_chunks.is_empty() {
            let sync_message = bincode::serialize(&sync).unwrap();
            debug!(target: "net_server", "sending dirty universe ({} bytes)", sync_message.len());
            server.send_message(*client_id, ServerChannel::NetworkedUniverse, sync_message);
            *chunks = chunks.difference(&sent_chunks).cloned().collect();
        }
    }
}

pub fn move_players_system(
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

pub fn consume_player_input(mut player_input_query: Query<&mut PlayerInput>) {
    for mut input in player_input_query.iter_mut() {
        input.consume();
    }
}
