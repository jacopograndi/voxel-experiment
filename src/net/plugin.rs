use super::{
    client::*,
    server::{
        server_receive_client_messages, server_receive_player_state, server_send_player_replica, server_send_universe, setup_open_server
    },
    LocalPlayerId, NetPlayerSpawned, PlayersChunkReplication, PlayersReplica, PlayersState,
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

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        let settings = app.world().get_resource::<McrsSettings>().unwrap().clone();
        let local_id = if let Some(player_name) = settings.player_name {
            LocalPlayerId {
                id: Some(player_name.into()),
            }
        } else {
            LocalPlayerId::default()
        };

        app.add_plugins((RenetClientPlugin, NetcodeClientPlugin));
        app.add_plugins((RenetServerPlugin, NetcodeServerPlugin));

        app.init_resource::<Lobby>();
        app.init_resource::<PlayersReplica>();
        app.init_resource::<PlayersState>();
        app.init_resource::<PlayersChunkReplication>();
        app.insert_resource(local_id);

        app.add_event::<NetPlayerSpawned>();

        app.add_systems(Startup, setup_open_client);
        app.add_systems(Startup, setup_open_server);

        app.add_systems(
            FixedUpdate,
            (
                (
                    client_receive_server_messages,
                    client_receive_player_replica,
                    client_receive_universe,
                )
                    .in_set(FixedNetSet::Receive),
                client_send_player_state.chain().in_set(FixedNetSet::Send),
            )
                .run_if(client_connected),
        );
        app.add_systems(
            FixedUpdate,
            (
                (
                    server_update_system,
                    server_receive_client_messages,
                    server_receive_player_state,
                )
                    .in_set(FixedNetSet::Receive),
                (server_send_universe.chain(), server_send_player_replica)
                    .in_set(FixedNetSet::Send),
            )
                .run_if(resource_exists::<RenetServer>),
        );
    }
}
