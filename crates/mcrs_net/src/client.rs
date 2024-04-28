use mcrs_input::PlayerInputBuffer;
use mcrs_settings::NetworkMode;
use std::{
    net::{ToSocketAddrs, UdpSocket},
    time::SystemTime,
};

use bevy::{prelude::*, utils::HashMap};
use mcrs_storage::{chunk::Chunk, universe::Universe};
use renet::{
    transport::{ClientAuthentication, NetcodeClientTransport},
    ClientId, RenetClient,
};

use crate::{ClientChannel, LocalPlayer, NewPlayerSpawned, SyncUniverse};

use super::{
    connection_config, Lobby, NetPlayer, PlayerState, ServerChannel, ServerMessages, PORT,
    PROTOCOL_ID,
};

pub fn new_renet_client(addr: &str) -> (RenetClient, NetcodeClientTransport) {
    let addr_port = addr.to_string() + ":" + &PORT.to_string();
    let Ok(mut resolved_addrs) = addr_port.to_socket_addrs() else {
        panic!("cannot resolve addr {}", addr_port);
    };
    let Some(resolved_addr) = resolved_addrs.next() else {
        panic!("cannot resolve addr {}", addr_port);
    };
    let server_addr = resolved_addr.to_socket_addrs().unwrap().next().unwrap();
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
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

pub fn client_sync_players(
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<Lobby>,
    transport: Res<NetcodeClientTransport>,
    query: Query<(Entity, &NetPlayer, &Children)>,
    mut query_transform: Query<&mut Transform>,
    network_mode: Res<NetworkMode>,
) {
    while let Some(message) = client.receive_message(ServerChannel::ServerMessages) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerConnected { id } => {
                let is_local_player = id == ClientId::from_raw(transport.client_id());

                if is_local_player {
                    debug!(target: "net_client", "Connected to the server");
                } else {
                    debug!(target: "net_client", "New player connected with id = {}", id);
                }

                let spawn_point = Vec3::new(0.0, 0.0, 0.0);
                if matches!(*network_mode, NetworkMode::Client) {
                    let player_entity = commands
                        .spawn((
                            SpatialBundle::from_transform(Transform::from_translation(spawn_point)),
                            NewPlayerSpawned,
                            NetPlayer { id },
                        ))
                        .id();
                    if is_local_player {
                        commands.entity(player_entity).insert(LocalPlayer);
                    }

                    lobby.players.insert(id, player_entity);
                }
            }
            ServerMessages::PlayerDisconnected { id } => {
                debug!(target: "net_client", "Player {} disconnected.", id);
                if let Some(player_entity) = lobby.players.remove(&id) {
                    commands.entity(player_entity).despawn_recursive();
                }
            }
        }
    }

    while let Some(message) = client.receive_message(ServerChannel::PlayerTransform) {
        let players: HashMap<ClientId, PlayerState> = bincode::deserialize(&message).unwrap();
        for (player_id, playerstate) in players.iter() {
            let is_local_player = *player_id == ClientId::from_raw(transport.client_id());
            if let Some(player_entity) = lobby.players.get(player_id) {
                if let Ok((_, _, children)) = query.get(*player_entity) {
                    let camera_entity = children.iter().next().unwrap(); // todo find camera
                    let mut tr = query_transform.get_mut(*player_entity).unwrap();
                    if !is_local_player && !matches!(*network_mode, NetworkMode::ClientAndServer) {
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

pub fn client_sync_universe(mut client: ResMut<RenetClient>, mut universe: ResMut<Universe>) {
    while let Some(message) = client.receive_message(ServerChannel::Universe) {
        let server_message: SyncUniverse = bincode::deserialize(&message).unwrap();
        debug!(target: "net_client", "{:?}", server_message.chunks.len());
        for (pos, chunk_bytes) in server_message.chunks.iter() {
            if let Some(chunk) = universe.chunks.get_mut(pos) {
                chunk.dirty_render = true;
                let mut write = chunk.get_mut();
                let bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut (*write));
                bytes.copy_from_slice(chunk_bytes.as_slice());
            } else {
                let chunk = Chunk::empty();
                {
                    let mut write = chunk.get_mut();
                    let bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut (*write));
                    bytes.copy_from_slice(chunk_bytes.as_slice());
                }
                universe.chunks.insert(*pos, chunk);
            }
        }
    }
}

pub fn client_send_input(
    mut res_player_input: ResMut<PlayerInputBuffer>,
    mut client: ResMut<RenetClient>,
) {
    let input_message = bincode::serialize(&*res_player_input).unwrap();
    // maybe unreliable is better (faster and if a packet is lost, whatever)
    client.send_message(ClientChannel::PlayerInput, input_message);
    res_player_input.buffer.clear();
}
