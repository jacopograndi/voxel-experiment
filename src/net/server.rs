use super::{
    connection_config, ChunkReplication, ClientChannel, ClientMessages, Lobby, LocalPlayerId,
    PlayerState, SyncUniverse, PORT, PROTOCOL_ID,
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
use mcrs_universe::{chunk::ChunkVersion, universe::Universe, CHUNK_VOLUME};
use std::{
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};

/// System that automatically opens a server if it's required at the start by network_mode
pub fn setup_open_server(
    mut commands: Commands,
    settings: Option<Res<NetSettings>>,
    local_id: Option<Res<LocalPlayerId>>,
    mut lobby: ResMut<Lobby>,
) {
    println!("server opening");
    if let Some(settings) = settings {
        let open = match settings.network_mode {
            super::NetworkMode::Server => true,
            super::NetworkMode::ClientAndServer => true,
            super::NetworkMode::Client => false,
            super::NetworkMode::Offline => false,
        };
        let default_id = LocalPlayerId::default();
        let local_id = local_id.as_deref().unwrap_or(&default_id);
        if open {
            open_server(
                &mut commands,
                settings.server_address.clone(),
                local_id,
                &mut lobby,
            );
        }
    }
}

pub fn open_server(
    commands: &mut Commands,
    server_address: String,
    local_id: &LocalPlayerId,
    lobby: &mut Lobby,
) {
    let (server, transport) = new_renet_server(&server_address);
    commands.insert_resource(server);
    commands.insert_resource(transport);
    if let Some(ref local_id) = local_id.id {
        lobby.players.push(local_id.clone());
    }
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
                    ids: lobby.players.clone(),
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
                    lobby.players.retain(|p| *p != player_id);
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
                    lobby.players.push(id.clone());
                    lobby.connections.insert(client_id, id.clone());
                    let broadcast_message =
                        bincode::serialize(&ServerMessages::PlayerConnected { ids: vec![id] })
                            .unwrap();
                    server.broadcast_message(ServerChannel::ServerMessages, broadcast_message);
                    // spawn too?
                }
            }
        }
    }
}

/*
pub fn server_sync_players(
    mut server: ResMut<RenetServer>,
    transforms: Query<&Transform>,
    query: Query<(Entity, &RemotePlayer, &Children)>,
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
    server.broadcast_message(ServerChannel::PlayerTransform, sync_message);
}

pub fn server_sync_universe(
    mut server: ResMut<RenetServer>,
    universe: Res<Universe>,
    mut chunk_replication: ResMut<ChunkReplication>,
    mut replicated_chunks: Local<HashMap<IVec3, ChunkVersion>>,
) {
    // todo: maybe make this observer pattern more general, it's the same in render and net
    let mut changed_chunks = HashSet::<IVec3>::new();
    for (chunk_pos, chunk) in universe.chunks.iter() {
        if let Some(version) = replicated_chunks.get(chunk_pos) {
            if version == &chunk.version {
                continue;
            }
        }
        replicated_chunks.insert(*chunk_pos, chunk.version.clone());
        changed_chunks.insert(*chunk_pos);
    }

    for (_, chunks) in chunk_replication.requested_chunks.iter_mut() {
        chunks.extend(changed_chunks.clone());
    }

    for (client_id, chunks) in chunk_replication.requested_chunks.iter_mut() {
        let channel_size =
            server.channel_available_memory(*client_id, ServerChannel::Universe) as i32;
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
            server.send_message(*client_id, ServerChannel::Universe, sync_message);
            *chunks = chunks.difference(&sent_chunks).cloned().collect();
        }
    }
}
*/
