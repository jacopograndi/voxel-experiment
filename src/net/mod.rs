pub mod client;
pub mod plugin;
pub mod server;

use bevy::{prelude::*, utils::HashSet};
use bevy_renet::renet::{ChannelConfig, ClientId, ConnectionConfig, SendType};
use mcrs_universe::{chunk::ChunkVersion, CHUNK_VOLUME};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

use crate::SerdePlayer;

const PROTOCOL_ID: u64 = 7;
pub const DEFAULT_NETWORK_ADDRESS: &str = "127.0.0.1";
const PORT: u32 = 54550;
pub const DEFAULT_REPLICATION_DISTANCE: u32 = 64;

#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub enum NetworkMode {
    Server,
    ClientAndServer,
    Client,

    #[default]
    Offline,
}

impl From<Option<String>> for NetworkMode {
    fn from(netmode: Option<String>) -> NetworkMode {
        match netmode {
            None => NetworkMode::Offline,
            Some(s) => {
                match s.as_str() {
                    "client" => NetworkMode::Client,
                    "server" => NetworkMode::Server,
                    "offline" => NetworkMode::Offline,
                    "clientserver" => NetworkMode::ClientAndServer,
                    _ => panic!("Use \"client\" for client-only mode, \"server\" for server-only mode, leave blank for standard (client+server) mode."),
                }
            },
        }
    }
}

impl From<Option<&str>> for NetworkMode {
    fn from(netmode: Option<&str>) -> NetworkMode {
        netmode.map(|s| s.to_string()).into()
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct NetSettings {
    pub server_address: String,
    pub network_mode: NetworkMode,
    pub replication_distance: u32,
}

impl Default for NetSettings {
    fn default() -> Self {
        Self {
            server_address: DEFAULT_NETWORK_ADDRESS.to_string(),
            network_mode: NetworkMode::Offline,
            replication_distance: DEFAULT_REPLICATION_DISTANCE,
        }
    }
}

#[derive(Event, Debug, Clone)]
pub struct NetPlayerSpawned {
    pub id: PlayerId,
    pub data: SerdePlayer,
}

/// Marker component that identifies the replicated entity of a remotely connected player
#[derive(Debug, Component)]
pub struct RemotePlayer {
    pub id: PlayerId,
}

/// Marker component that identifies the local player entity
#[derive(Debug, Component, Clone)]
pub struct LocalPlayer {
    pub id: PlayerId,
}

/// Marker component that identifies an entity of a player
#[derive(Debug, Component)]
pub struct Player {
    pub id: PlayerId,
}

/// The id of the local player
#[derive(Default, Debug, Resource, Clone)]
pub struct LocalPlayerId {
    pub id: Option<PlayerId>,
}

/// Identifier of a player in a remote connection
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId {
    pub name: String,
}

impl From<String> for PlayerId {
    fn from(value: String) -> Self {
        PlayerId { name: value }
    }
}

/// List of connected players
#[derive(Debug, Default, Resource)]
pub struct Lobby {
    connections: HashMap<ClientId, PlayerId>,
    pub local_players: Vec<PlayerId>,
    pub remote_players: Vec<PlayerId>,
}

#[derive(Debug, Default, Resource)]
pub struct PlayersChunkReplication {
    players: HashMap<PlayerId, ChunkReplication>,
}

#[derive(Debug, Default)]
pub struct ChunkReplication {
    requested: HashMap<IVec3, ChunkVersion>,
    sent: HashMap<IVec3, ChunkVersion>,
}

/// Messages sent by the server to the clients
#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerMessages {
    PlayerConnected { ids: Vec<PlayerId> },
    LoginRequest,
    PlayerSpawned { id: PlayerId, data: SerdePlayer },
    PlayerDisconnected { id: PlayerId },
}

/// Messages sent by the client to the server
#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ClientMessages {
    Login { id: PlayerId },
}

/// Defines the different channels to which data is sent from the clients to the server
pub enum ClientChannel {
    ClientMessages,
    PlayerStates,
}

impl From<ClientChannel> for u8 {
    fn from(channel_id: ClientChannel) -> Self {
        match channel_id {
            ClientChannel::ClientMessages => 0,
            ClientChannel::PlayerStates => 1,
        }
    }
}

impl ClientChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ChannelConfig {
                channel_id: Self::ClientMessages.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::PlayerStates.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::Unreliable,
            },
        ]
    }
}

/// Defines the different channels to which data is sent from the server to the clients
pub enum ServerChannel {
    ServerMessages,
    ClientMessages,
    PlayerReplica,
    Universe,
}

impl From<ServerChannel> for u8 {
    fn from(channel_id: ServerChannel) -> Self {
        match channel_id {
            ServerChannel::ServerMessages => 0,
            ServerChannel::ClientMessages => 1,
            ServerChannel::PlayerReplica => 2,
            ServerChannel::Universe => 3,
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
                channel_id: Self::ClientMessages.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
            ChannelConfig {
                channel_id: Self::PlayerReplica.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::Unreliable,
            },
            ChannelConfig {
                channel_id: Self::Universe.into(),
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
        client_channels_config: ClientChannel::channels_config(),
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct SyncUniverse {
    chunks: Vec<(IVec3, Vec<u8>)>,
    heightfield: Vec<(IVec2, i32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerReplica {
    pub position: Vec3,
    pub rotation_body: f32,
    pub rotation_camera: f32,
    // Todo: hand
}

#[derive(Debug, Clone, Resource, Default)]
pub struct PlayersReplica {
    pub players: HashMap<PlayerId, PlayerReplica>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub position: Vec3,
    pub rotation_body: f32,
    pub rotation_camera: f32,
    // also includes the broken blocks, inventory and hand content
}

#[derive(Debug, Clone, Resource, Default)]
pub struct PlayersState {
    pub players: HashMap<PlayerId, PlayerState>,
}
