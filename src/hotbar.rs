use bevy::{prelude::*, utils::HashMap};
use bevy_egui::{egui, EguiContexts};
use mcrs_blueprints::{
    blocks::{BlockBlueprint, BlockId},
    Blueprints,
};
use mcrs_input::{PlayerInput, PlayerInputBuffer};
use mcrs_net::{ClientChannel, Lobby, LocalPlayer, NetPlayer, ServerChannel};
use renet::{transport::NetcodeClientTransport, ClientId, RenetClient, RenetServer};
use serde::{Deserialize, Serialize};

// this is very much a hack

#[derive(Debug, Default, Serialize, Deserialize, Component, Resource, Clone)]
pub struct PlayerHand {
    pub block_id: Option<BlockId>,
}

pub fn client_send_replica(
    hand_query: Query<(&PlayerHand, &LocalPlayer)>,
    mut client: ResMut<RenetClient>,
) {
    if let Ok((hand, _)) = hand_query.get_single() {
        let msg = bincode::serialize(&*hand).unwrap();
        client.send_message(ClientChannel::PlayerStates, msg);
    }
}

pub fn server_receive_replica(
    lobby: Res<Lobby>,
    mut server: ResMut<RenetServer>,
    transport: Res<NetcodeClientTransport>,
    mut query: Query<(Entity, &mut PlayerHand)>,
) {
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, ClientChannel::PlayerStates) {
            let replicated_hand: PlayerHand = bincode::deserialize(&message).unwrap();
            let is_local_player = client_id == ClientId::from_raw(transport.client_id());
            if is_local_player {
                continue;
            }
            if let Some(player_entity) = lobby.players.get(&client_id) {
                if let Ok((_, mut hand)) = query.get_mut(*player_entity) {
                    *hand = replicated_hand;
                }
            }
        }
    }
}

pub fn server_send_replica(
    mut server: ResMut<RenetServer>,
    query: Query<(&NetPlayer, &PlayerHand)>,
) {
    let players: HashMap<ClientId, PlayerHand> = query
        .iter()
        .map(|(player, hand)| (player.id, hand.clone()))
        .collect();
    let sync_message = bincode::serialize(&players).unwrap();
    server.broadcast_message(ServerChannel::PlayerStates, sync_message);
}

pub fn client_receive_replica(
    mut client: ResMut<RenetClient>,
    lobby: Res<Lobby>,
    transport: Res<NetcodeClientTransport>,
    mut query_hand: Query<&mut PlayerHand>,
) {
    while let Some(message) = client.receive_message(ServerChannel::PlayerStates) {
        let players: HashMap<ClientId, PlayerHand> = bincode::deserialize(&message).unwrap();
        for (player_id, replicated_hand) in players.into_iter() {
            let is_local_player = player_id == ClientId::from_raw(transport.client_id());
            if let Some(player_entity) = lobby.players.get(&player_id) {
                if !is_local_player {
                    if let Ok(mut hand) = query_hand.get_mut(*player_entity) {
                        *hand = replicated_hand;
                    }
                }
            }
        }
    }
}

pub fn hotbar(
    mut input: ResMut<PlayerInputBuffer>,
    mut contexts: EguiContexts,
    mut hand_query: Query<(&mut PlayerHand, &LocalPlayer)>,
    blueprints: Res<Blueprints>,
    mut mouse: ResMut<Input<MouseButton>>,
) {
    let Ok((mut hand, _)) = hand_query.get_single_mut() else {
        egui::Window::new("Hotbar")
            .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::ZERO)
            .title_bar(false)
            .resizable(false)
            .show(contexts.ctx_mut(), |ui| {
                ui.label("no hand");
            });
        return;
    };
    egui::Window::new("Hotbar")
        .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::ZERO)
        .title_bar(false)
        .resizable(false)
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                let mut blueprints: Vec<&BlockBlueprint> = blueprints.blocks.iter().collect();
                blueprints.sort_by(|a, b| a.id.cmp(&b.id));
                for blueprint in blueprints.iter() {
                    if blueprint.name == "Air" {
                        continue;
                    }
                    let selected = hand.block_id == Some(blueprint.id);
                    let button =
                        egui::Button::new(format!("{}", blueprint.name)).selected(selected);
                    let response = ui.add(button);
                    if response.hovered() && mouse.just_pressed(MouseButton::Left) {
                        mouse.clear_just_pressed(MouseButton::Left);
                        mouse.clear_just_pressed(MouseButton::Right);
                        hand.block_id = Some(blueprint.id);
                    }
                    if response.hovered() && mouse.just_pressed(MouseButton::Right) {
                        mouse.clear_just_pressed(MouseButton::Left);
                        mouse.clear_just_pressed(MouseButton::Right);
                        hand.block_id = None;
                    }
                }
            });
        });
}
