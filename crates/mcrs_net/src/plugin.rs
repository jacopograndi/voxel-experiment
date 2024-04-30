use bevy::prelude::*;
use bevy_renet::client_connected;
use bevy_renet::transport::NetcodeClientPlugin;
use bevy_renet::transport::NetcodeServerPlugin;
use bevy_renet::RenetClientPlugin;
use bevy_renet::RenetServerPlugin;
use renet::RenetServer;

use crate::server::new_renet_server;
use crate::server::server_sync_players;
use crate::server::server_sync_universe;
use crate::server::server_update_system;
use crate::Lobby;
use crate::NetSettings;

use super::client::*;
use super::ChunkReplication;

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum FixedNetSet {
    Receive,
    Send,
}

pub struct McrsNetServerPlugin;
pub struct McrsNetClientPlugin;

impl Plugin for McrsNetServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RenetServerPlugin, NetcodeServerPlugin));
        let (server, transport) = new_renet_server(get_server_address(app));
        app.init_resource::<Lobby>();
        app.insert_resource(server);
        app.insert_resource(transport);
        app.init_resource::<ChunkReplication>();
        app.add_systems(
            FixedUpdate,
            (
                server_update_system.in_set(FixedNetSet::Receive),
                (server_sync_players, server_sync_universe)
                    .chain()
                    .in_set(FixedNetSet::Send),
            )
                .run_if(resource_exists::<RenetServer>()),
        );
    }
}

impl Plugin for McrsNetClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RenetClientPlugin, NetcodeClientPlugin));
        let (client, transport) = new_renet_client(get_server_address(app));
        app.init_resource::<Lobby>();
        app.insert_resource(client);
        app.insert_resource(transport);
        app.add_systems(
            FixedUpdate,
            ((client_sync_players, client_sync_universe).in_set(FixedNetSet::Receive),)
                .run_if(client_connected()),
        );
    }
}

fn get_server_address(app: &App) -> &str {
    if let Some(settings) = app.world.get_resource::<NetSettings>() {
        &settings.server_address
    } else {
        "127.0.0.1"
    }
}
