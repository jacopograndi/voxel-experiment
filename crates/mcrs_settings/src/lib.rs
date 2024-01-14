use bevy::prelude::*;

mod args;
pub mod plugin;

const DEFAULT_TICKS_PER_SECOND: u32 = 64;
const DEFAULT_NETWORK_ADDRESS: &str = "127.0.0.1";
const DEFAULT_VIEW_DISTANCE: u32 = 64;

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct NetworkAddress {
    pub server: String,
}

impl Default for NetworkAddress {
    fn default() -> Self {
        Self {
            server: DEFAULT_NETWORK_ADDRESS.to_string(),
        }
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct TicksPerSecond(pub u32);

impl Default for TicksPerSecond {
    fn default() -> Self {
        Self(DEFAULT_TICKS_PER_SECOND)
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct ViewDistance(pub u32);

impl Default for ViewDistance {
    fn default() -> Self {
        Self(DEFAULT_VIEW_DISTANCE)
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub enum NetworkMode {
    Server,
    ClientAndServer,
    Client,
}

impl From<Option<&str>> for NetworkMode {
    fn from(netmode: Option<&str>) -> NetworkMode {
        match netmode {
            Some("client") => NetworkMode::Client,
            Some("server") => NetworkMode::Server,
            None => NetworkMode::ClientAndServer,
            Some(_) => panic!("Use \"client\" for client-only mode, \"server\" for server-only mode, leave blank for standard (client+server) mode."),
        }
    }
}

impl Default for NetworkMode {
    fn default() -> Self {
        Self::ClientAndServer
    }
}
