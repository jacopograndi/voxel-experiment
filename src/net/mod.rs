pub mod client;
pub mod server;

use std::{collections::HashMap, time::Duration};

use bevy::{prelude::*, utils::HashSet};
use mcrs_storage::CHUNK_VOLUME;
use renet::{ChannelConfig, ClientId, ConnectionConfig, SendType};
use serde::{Deserialize, Serialize};

const PROTOCOL_ID: u64 = 7;

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub enum NetworkMode {
    /// A server without a local player (headless hosting)
    Server,
    /// A server with a local player (singleplayer and hosting)
    ClientAndServer,
    /// A player connected to a server.
    Client,
}

#[derive(Debug, Component)]
pub struct NetPlayer {
    pub id: ClientId,
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
pub enum ServerMessages {
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

#[derive(Clone, Serialize, Deserialize, Default)]
struct SyncUniverse {
    chunks: Vec<(IVec3, Vec<u8>)>,
    heightfield: Vec<(IVec2, i32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlayerState {
    position: Vec3,
    rotation_body: f32,
    rotation_camera: f32,
}
