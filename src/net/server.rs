use super::{
    connection_config, ChunkReplication, ClientChannel, ClientMessages, Lobby, LocalPlayerId,
    Player, PlayerId, PlayerReplica, PlayerState, PlayersChunkReplication, PlayersState,
    SyncUniverse, PORT, PROTOCOL_ID,
};
use crate::{NetSettings, RemotePlayer, ServerChannel, ServerMessages};
use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_renet::{
    netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig},
    renet::{ClientId, RenetServer, ServerEvent},
};
use mcrs_physics::intersect::get_chunks_in_sphere;
use mcrs_universe::{chunk::ChunkVersion, universe::Universe, CHUNK_VOLUME};
use miniz_oxide::deflate::compress_to_vec;
use std::{
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};

/// System that automatically opens a server if it's required at the start by network_mode
pub fn setup_open_server(mut commands: Commands, settings: Option<Res<NetSettings>>) {
    if let Some(settings) = settings {
        let open = match settings.network_mode {
            super::NetworkMode::Server => true,
            super::NetworkMode::ClientAndServer => true,
            super::NetworkMode::Client => false,
            super::NetworkMode::Offline => false,
        };
        if open {
            open_server(&mut commands, settings.server_address.clone());
        }
    }
}

pub fn open_server(commands: &mut Commands, server_address: String) {
    info!("server opening");
    let (server, transport) = new_renet_server(&server_address);
    commands.insert_resource(server);
    commands.insert_resource(transport);
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
    mut lobby: ResMut<Lobby>,
    mut server: ResMut<RenetServer>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                info!(target: "net_server", "client {} connected", client_id);

                let message = bincode::serialize(&ServerMessages::LoginRequest).unwrap();
                server.send_message(*client_id, ServerChannel::ServerMessages, message);

                // send all other already connected clients
                let message = bincode::serialize(&ServerMessages::PlayerConnected {
                    ids: lobby
                        .remote_players
                        .iter()
                        .chain(lobby.local_players.iter())
                        .cloned()
                        .collect(),
                })
                .unwrap();
                server.send_message(*client_id, ServerChannel::ServerMessages, message);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!(
                    target: "net_server",
                    "client {} disconnected with reason {}", client_id, reason
                );

                if let Some(player_id) = lobby.connections.remove(client_id) {
                    lobby.remote_players.retain(|p| *p != player_id);
                    let message = bincode::serialize(&ServerMessages::PlayerDisconnected {
                        id: player_id.clone(),
                    })
                    .unwrap();
                    server.broadcast_message(ServerChannel::ServerMessages, message);
                }
            }
        }
    }
}

pub fn server_receive_client_messages(mut server: ResMut<RenetServer>, mut lobby: ResMut<Lobby>) {
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, ClientChannel::ClientMessages) {
            let message: ClientMessages = bincode::deserialize(&message).unwrap();
            match message {
                ClientMessages::Login { id } => {
                    lobby.remote_players.push(id.clone());
                    lobby.connections.insert(client_id, id.clone());
                    let broadcast_message =
                        bincode::serialize(&ServerMessages::PlayerConnected { ids: vec![id] })
                            .unwrap();
                    server.broadcast_message(ServerChannel::ServerMessages, broadcast_message);
                }
            }
        }
    }
}

pub fn server_send_universe(
    mut server: ResMut<RenetServer>,
    universe: Res<Universe>,
    mut chunk_replication: ResMut<PlayersChunkReplication>,
    lobby: Res<Lobby>,
    player_query: Query<(&RemotePlayer, &Transform)>,
    settings: Res<NetSettings>,
) {
    for id in lobby.remote_players.iter() {
        if let Some(player_tr) = player_query
            .iter()
            .find_map(|(rem, tr)| (&rem.id == id).then_some(tr))
        {
            let chunk_rep = chunk_replication.players.entry(id.clone()).or_default();
            let request =
                get_chunks_in_sphere(player_tr.translation, settings.replication_distance as f32);

            let request_versions: HashMap<IVec3, ChunkVersion> = request
                .iter()
                .filter_map(|chunk_pos| {
                    universe
                        .chunks
                        .get(chunk_pos)
                        .map(|c| (*chunk_pos, c.version.clone()))
                })
                .filter(|(pos, v)| match chunk_rep.sent.get(pos) {
                    Some(w) => v != w,
                    None => true,
                })
                .collect();

            chunk_rep
                .requested
                .extend(request_versions.clone().into_iter());

            if request_versions.len() > 0 {
                info!(target: "net_server", "player {} chunks: requested {}, sent {}", 
                id.name, chunk_rep.requested.len(), chunk_rep.sent.len());
            }
        }
    }

    for (player_id, chunk_rep) in chunk_replication.players.iter_mut() {
        let Some((client_id, _)) = lobby.connections.iter().find(|(_, v)| v == &player_id) else {
            continue;
        };

        let channel_size =
            server.channel_available_memory(*client_id, ServerChannel::Universe) as i32;
        let mut available_bytes = channel_size;

        let mut sync = SyncUniverse::default();

        let mut sent_chunks = HashMap::<IVec3, ChunkVersion>::new();

        for (chunk_pos, version) in chunk_rep.requested.iter() {
            if let Some(chunk) = universe.chunks.get(chunk_pos) {
                let read = chunk.get_ref();
                let block_bytes = bytemuck::cast_slice(read.as_ref());
                let block_compressed = compress_to_vec(block_bytes, 6);
                if available_bytes > (block_compressed.len() as i32) + 12 {
                    available_bytes -= block_compressed.len() as i32;
                    sync.chunks
                        .push((*chunk_pos, block_compressed.iter().cloned().collect()));
                    sent_chunks.insert(*chunk_pos, version.clone());
                }
            }
        }

        if !sent_chunks.is_empty() {
            let sync_message = bincode::serialize(&sync).unwrap();
            info!(target: "net_server", "sending to {} universe ({} bytes)", player_id.name, sync_message.len());
            server.send_message(*client_id, ServerChannel::Universe, sync_message);

            for (chunk_pos, _) in sent_chunks.iter() {
                chunk_rep.requested.remove(chunk_pos);
            }
            chunk_rep.sent.extend(sent_chunks.clone().into_iter());
        }
    }
}

pub fn server_receive_player_state(
    mut server: ResMut<RenetServer>,
    mut players_state: ResMut<PlayersState>,
) {
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, ClientChannel::PlayerStates) {
            let players: HashMap<PlayerId, PlayerState> = bincode::deserialize(&message).unwrap();
            for (player_id, playerstate) in players.into_iter() {
                players_state.players.insert(player_id, playerstate);
            }
        }
    }
}

pub fn server_send_player_replica(
    mut server: ResMut<RenetServer>,
    transforms: Query<&Transform>,
    query: Query<(Entity, &Player, &Children)>,
) {
    let mut players: HashMap<PlayerId, PlayerReplica> = HashMap::new();
    for (entity, player, children) in query.iter() {
        let tr = transforms.get(entity).unwrap();
        let camera_entity = children.iter().next().unwrap();
        let tr_camera = transforms.get(*camera_entity).unwrap();
        let playerstate = PlayerReplica {
            position: tr.translation,
            rotation_camera: tr_camera.rotation.to_euler(EulerRot::YXZ).1,
            rotation_body: tr.rotation.to_euler(EulerRot::YXZ).0,
        };
        players.insert(player.id.clone(), playerstate);
    }

    let sync_message = bincode::serialize(&players).unwrap();
    server.broadcast_message(ServerChannel::PlayerReplica, sync_message);
}
