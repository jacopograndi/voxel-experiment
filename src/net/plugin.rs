use super::{client::*, ChunkReplication};
use crate::{
    server::{server_sync_players, server_sync_universe, server_update_system},
    Lobby,
};
use bevy::prelude::*;
use bevy_renet::{
    client_connected,
    netcode::{NetcodeClientPlugin, NetcodeServerPlugin},
    renet::RenetServer,
    RenetClientPlugin, RenetServerPlugin,
};

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum FixedNetSet {
    Receive,
    Send,
}

pub struct NetServerPlugin;
pub struct NetClientPlugin;

impl Plugin for NetServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RenetServerPlugin, NetcodeServerPlugin));
        app.init_resource::<Lobby>();
        app.init_resource::<ChunkReplication>();
        app.add_systems(
            FixedUpdate,
            (
                server_update_system.in_set(FixedNetSet::Receive),
                (server_sync_players, server_sync_universe)
                    .chain()
                    .in_set(FixedNetSet::Send),
            )
                .run_if(resource_exists::<RenetServer>),
        );
    }
}

impl Plugin for NetClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RenetClientPlugin, NetcodeClientPlugin));
        app.init_resource::<Lobby>();
        app.add_systems(
            FixedUpdate,
            ((client_sync_players, client_sync_universe).in_set(FixedNetSet::Receive),)
                .run_if(client_connected),
        );
    }
}
