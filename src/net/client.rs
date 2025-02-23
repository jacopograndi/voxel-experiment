use super::{
    connection_config, Lobby, NetPlayer, NetworkMode, PlayerState, ServerChannel, ServerMessages,
    PORT, PROTOCOL_ID,
};
use crate::{LocalPlayer, NetSettings, NewPlayerSpawned};
use crate::net::SyncUniverse;
use bevy::{prelude::*, utils::HashMap};
use bevy_renet::{
    netcode::{ClientAuthentication, NetcodeClientTransport},
    renet::{ClientId, RenetClient},
};
use mcrs_universe::{chunk::Chunk, universe::Universe};
use std::{
    net::{ToSocketAddrs, UdpSocket},
    time::SystemTime,
};

pub fn open_client(mut commands: Commands, settings: Option<Res<NetSettings>>) {
    let server_address = settings.map_or("127.0.0.1".to_string(), |s| s.server_address.clone());
    let (client, transport) = new_renet_client(&server_address);
    commands.insert_resource(client);
    commands.insert_resource(transport);
}

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
    settings: Res<NetSettings>,
) {
    while let Some(message) = client.receive_message(ServerChannel::ServerMessages) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerConnected { id } => {
                let is_local_player = id == transport.client_id();

                if is_local_player {
                    debug!(target: "net_client", "Connected to the server");
                } else {
                    debug!(target: "net_client", "New player connected with id = {}", id);
                }

                let spawn_point = Vec3::new(0.0, 0.0, 0.0);
                if matches!(settings.network_mode, NetworkMode::Client) {
                    let player_entity = commands
                        .spawn((
                            Transform::from_translation(spawn_point),
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
            let is_local_player = *player_id == transport.client_id();
            if let Some(player_entity) = lobby.players.get(player_id) {
                if let Ok((_, _, children)) = query.get(*player_entity) {
                    let camera_entity = children.iter().next().unwrap(); // todo find camera
                    let mut tr = query_transform.get_mut(*player_entity).unwrap();
                    if !is_local_player
                        && !matches!(settings.network_mode, NetworkMode::ClientAndServer)
                    {
                        tr.translation = playerstate.position;
                        tr.rotation = Quat::from_axis_angle(Vec3::Y, playerstate.rotation_body);
                        let mut tr_camera = query_transform.get_mut(*camera_entity).unwrap();
                        tr_camera.rotation =
                            Quat::from_axis_angle(Vec3::X, playerstate.rotation_camera);
                    } else if matches!(settings.network_mode, NetworkMode::Client) {
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
                chunk.version.update();
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
