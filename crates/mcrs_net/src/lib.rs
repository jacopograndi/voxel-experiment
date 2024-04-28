pub mod client;
pub mod plugin;
pub mod server;

use std::{collections::HashMap, time::Duration};

use bevy::{prelude::*, utils::HashSet};
use mcrs_storage::CHUNK_VOLUME;
use renet::{ChannelConfig, ClientId, ConnectionConfig, SendType};
use serde::{Deserialize, Serialize};

const PROTOCOL_ID: u64 = 7;

const PORT: u32 = 54550;

#[derive(Debug, Component)]
pub struct NetPlayer {
    pub id: ClientId,
}

#[derive(Debug, Component, Clone)]
pub struct LocalPlayer;

#[derive(Debug, Default, Resource)]
pub struct Lobby {
    pub players: HashMap<ClientId, Entity>,
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

pub enum ClientChannel {
    PlayerInput,
    PlayerStates,
}

impl From<ClientChannel> for u8 {
    fn from(channel_id: ClientChannel) -> Self {
        match channel_id {
            ClientChannel::PlayerInput => 0,
            ClientChannel::PlayerStates => 1,
        }
    }
}

impl ClientChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ChannelConfig {
                channel_id: Self::PlayerInput.into(),
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

pub enum ServerChannel {
    ServerMessages,
    PlayerTransform,
    PlayerStates,
    Universe,
}

impl From<ServerChannel> for u8 {
    fn from(channel_id: ServerChannel) -> Self {
        match channel_id {
            ServerChannel::ServerMessages => 0,
            ServerChannel::PlayerTransform => 1,
            ServerChannel::PlayerStates => 2,
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
                channel_id: Self::PlayerTransform.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::Unreliable,
            },
            ChannelConfig {
                channel_id: Self::PlayerStates.into(),
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
struct PlayerState {
    position: Vec3,
    rotation_body: f32,
    rotation_camera: f32,
}

// todo: consider using an event instead of a marker component
#[derive(Debug, Clone, Component)]
pub struct NewPlayerSpawned;
