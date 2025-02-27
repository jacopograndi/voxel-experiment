use super::{
    connection_config, Lobby, LocalPlayerId, NetPlayerSpawned, NetworkMode, PlayerId,
    PlayerReplica, PlayerState, PlayersReplica, ServerChannel, ServerMessages, PORT,
    PROTOCOL_ID,
};
use crate::net::SyncUniverse;
use crate::{ClientChannel, ClientMessages, LocalPlayer, NetSettings, PlayerUniverseChanges};
use bevy::{prelude::*, utils::HashMap};
use bevy_renet::{
    netcode::{ClientAuthentication, NetcodeClientTransport},
    renet::{ RenetClient},
};
use mcrs_universe::CHUNK_VOLUME;
use mcrs_universe::{chunk::Chunk, universe::Universe};
use miniz_oxide::inflate::decompress_to_vec_with_limit;
use std::{
    net::{ToSocketAddrs, UdpSocket},
    time::SystemTime,
};

/// System that automatically opens client connection if it's required at the start by network_mode
pub fn setup_open_client(mut commands: Commands, settings: Option<Res<NetSettings>>) {
    if let Some(settings) = settings {
        let open = match settings.network_mode {
            NetworkMode::Client => true,
            _ => false,
        };
        if open {
            open_client(&mut commands, settings.server_address.clone());
        }
    }
}

pub fn open_client(commands: &mut Commands, server_address: String) {
    info!("client opening");
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

pub fn client_receive_server_messages(
    mut lobby: ResMut<Lobby>,
    mut client: ResMut<RenetClient>,
    local_id: Res<LocalPlayerId>,
    mut events: EventWriter<NetPlayerSpawned>,
) {
    let Some(local_id) = local_id.id.as_ref() else {
        panic!("client is opened without a local id");
    };

    while let Some(message) = client.receive_message(ServerChannel::ServerMessages) {
        let server_message = bincode::deserialize(&message).unwrap();

        info!(
            target: "net_client",
            "received server message {:?}", server_message
        );

        match server_message {
            ServerMessages::PlayerConnected { mut ids } => {
                lobby.remote_players.append(&mut ids);
                lobby.remote_players.dedup();
            }
            ServerMessages::PlayerDisconnected { id } => {
                lobby.remote_players.retain(|p| *p != id);
            }
            ServerMessages::LoginRequest => {
                send_login_to_server(&mut client, local_id);
            }
            ServerMessages::PlayerSpawned { id, data } => {
                if local_id == &id && !lobby.local_players.contains(&id) {
                    lobby.local_players.push(id.clone());
                } else if !lobby.remote_players.contains(&id) {
                    lobby.remote_players.push(id.clone());
                }
                events.send(NetPlayerSpawned { id, data });
            }
        }
    }
}

fn send_login_to_server(client: &mut RenetClient, local_id: &PlayerId) {
    let message = bincode::serialize(&ClientMessages::Login {
        id: local_id.clone(),
    })
    .unwrap();
    client.send_message(ClientChannel::ClientMessages, message);
}

pub fn client_receive_universe(mut client: ResMut<RenetClient>, mut universe: ResMut<Universe>) {
    while let Some(message) = client.receive_message(ServerChannel::Universe) {
        let server_message: SyncUniverse = bincode::deserialize(&message).unwrap();
        debug!(target: "net_client", "{:?}", server_message.chunks.len());
        info!(target: "net_client", "{:?}", server_message.chunks.len());
        for (pos, chunk_bytes) in server_message.chunks.iter() {
            let block_decompressed =
                decompress_to_vec_with_limit(chunk_bytes, CHUNK_VOLUME * 4 + 12)
                    .expect("failed to decompress chunk");
            if let Some(chunk) = universe.chunks.get_mut(pos) {
                {
                    let mut write = chunk.get_mut();
                    let bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut (*write));
                    bytes.copy_from_slice(&block_decompressed);
                }
                chunk.version.update();
            } else {
                let chunk = Chunk::empty();
                {
                    let mut write = chunk.get_mut();
                    let bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut (*write));
                    bytes.copy_from_slice(&block_decompressed);
                }
                universe.chunks.insert(*pos, chunk);
            }
        }
    }
}

pub fn client_send_player_state(
    mut client: ResMut<RenetClient>,
    transforms: Query<&Transform>,
    query: Query<(Entity, &LocalPlayer, &Children)>,
    mut player_changes: ResMut<PlayerUniverseChanges>,
) {
    let mut players: HashMap<PlayerId, PlayerState> = HashMap::new();
    for (entity, player, children) in query.iter() {
        let tr = transforms.get(entity).unwrap();
        let camera_entity = children.iter().next().unwrap();
        let tr_camera = transforms.get(*camera_entity).unwrap();

        let mut universe_changes = vec![];
        universe_changes.append(&mut player_changes.queue);

        let playerstate = PlayerState {
            position: tr.translation,
            rotation_camera: tr_camera.rotation.to_euler(EulerRot::YXZ).1,
            rotation_body: tr.rotation.to_euler(EulerRot::YXZ).0,
            universe_changes,
        };

        players.insert(player.id.clone(), playerstate);
    }

    let message = bincode::serialize(&players).unwrap();
    client.send_message(ClientChannel::PlayerStates, message);
}

pub fn client_receive_player_replica(
    mut client: ResMut<RenetClient>,
    local_id: Res<LocalPlayerId>,
    mut players_replica: ResMut<PlayersReplica>,
) {
    let Some(local_id) = local_id.id.as_ref() else {
        panic!("client is opened without a local id");
    };

    while let Some(message) = client.receive_message(ServerChannel::PlayerReplica) {
        let players: HashMap<PlayerId, PlayerReplica> = bincode::deserialize(&message).unwrap();
        for (player_id, playerstate) in players.into_iter() {
            if &player_id == local_id {
                // ignore replicas of the local players
                continue;
            }

            players_replica.players.insert(player_id, playerstate);
        }
    }
}
