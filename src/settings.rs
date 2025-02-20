use bevy::prelude::*;
use clap::Parser;
use mcrs_net::{NetSettings, NetworkMode, DEFAULT_NETWORK_ADDRESS};
use mcrs_render::settings::{RenderMode, RenderSettings, DEFAULT_VIEW_DISTANCE};

pub const DEFAULT_TICKS_PER_SECOND: u32 = 64;
pub const DEFAULT_LOAD_DISTANCE: u32 = 300;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub network_mode: Option<String>,

    #[arg(short, long)]
    pub address_server: Option<String>,

    #[arg(short, long)]
    pub view_distance: Option<u32>,

    #[arg(short, long)]
    pub load_distance: Option<u32>,

    #[arg(short, long)]
    pub render_mode: Option<String>,

    #[arg(short, long)]
    pub open_level_name: Option<String>,
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct McrsSettings {
    pub ticks_per_second: u32,
    pub view_distance_blocks: u32,
    pub load_distance_blocks: u32,
    pub server_address: String,
    pub network_mode: NetworkMode,
    pub render_mode: RenderMode,
    pub open_level_name: String,
}

impl Default for McrsSettings {
    fn default() -> Self {
        Self {
            ticks_per_second: DEFAULT_TICKS_PER_SECOND,
            view_distance_blocks: DEFAULT_VIEW_DISTANCE,
            load_distance_blocks: DEFAULT_LOAD_DISTANCE,
            server_address: DEFAULT_NETWORK_ADDRESS.to_string(),
            network_mode: NetworkMode::ClientAndServer,
            render_mode: RenderMode::default(),
            open_level_name: format!("world"),
        }
    }
}

impl From<Args> for McrsSettings {
    fn from(args: Args) -> Self {
        Self {
            view_distance_blocks: args.view_distance.unwrap_or(DEFAULT_VIEW_DISTANCE),
            server_address: args
                .address_server
                .unwrap_or(DEFAULT_NETWORK_ADDRESS.to_string()),
            network_mode: args.network_mode.into(),
            render_mode: args.render_mode.into(),
            ..Default::default()
        }
    }
}

impl From<McrsSettings> for NetSettings {
    fn from(settings: McrsSettings) -> Self {
        Self {
            server_address: settings.server_address,
            network_mode: settings.network_mode,
            replication_distance: settings.view_distance_blocks,
        }
    }
}

impl From<McrsSettings> for RenderSettings {
    fn from(settings: McrsSettings) -> Self {
        Self {
            view_distance_blocks: settings.view_distance_blocks,
            render_mode: settings.render_mode,
        }
    }
}
