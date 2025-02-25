use super::{
    client::*,
    server::{server_receive_client_messages, setup_open_server},
    ChunkReplication, LocalPlayerId,
};
use crate::{server::server_update_system, settings::McrsSettings, Lobby};
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
        app.add_systems(Startup, setup_open_server);
        app.add_systems(
            FixedUpdate,
            (
                (server_update_system, server_receive_client_messages).in_set(FixedNetSet::Receive),
                /*
                (server_sync_players, server_sync_universe)
                    .chain()
                    .in_set(FixedNetSet::Send),
                    */
            )
                .run_if(resource_exists::<RenetServer>),
        );
    }
}

impl Plugin for NetClientPlugin {
    fn build(&self, app: &mut App) {
        let settings = app.world().get_resource::<McrsSettings>().unwrap().clone();
        let local_id = if let Some(player_name) = settings.player_name {
            LocalPlayerId {
                id: Some(player_name.into()),
            }
        } else {
            LocalPlayerId::default()
        };

        // Todo: here, read net settings and act accordingly

        app.add_plugins((RenetClientPlugin, NetcodeClientPlugin));
        app.init_resource::<Lobby>();
        app.insert_resource(local_id);
        app.add_systems(Startup, setup_open_client);
        app.add_systems(
            FixedUpdate,
            ((client_receive_server_messages).in_set(FixedNetSet::Receive),)
                .run_if(client_connected),
        );
        /*
        app.add_systems(
            FixedUpdate,
            ((client_sync_players, client_sync_universe).in_set(FixedNetSet::Receive),)
                .run_if(client_connected),
        );
        */
    }
}
