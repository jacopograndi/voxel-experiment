use bevy::prelude::*;
use mcrs_net::{ClientChannel, Lobby, LocalPlayer};
use mcrs_physics::character::{CameraController, CharacterController};
use renet::{RenetClient, RenetServer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Component, Resource, Clone)]
pub enum PlayerInput {
    Acceleration(Vec3),
    RotationCamera(f32),
    RotationBody(f32),
    Jumping(bool),
    Placing(bool),
    Mining(bool),
}

#[derive(Debug, Default, Serialize, Deserialize, Component, Resource, Clone)]
pub struct PlayerInputBuffer {
    pub buffer: Vec<PlayerInput>,
}

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum InputSet {
    Gather,
}

pub fn player_input(
    mut player_input_buffer: ResMut<PlayerInputBuffer>,
    keys: Res<ButtonInput<KeyCode>>,
    query_transform: Query<&Transform>,
    query_camera: Query<(Entity, &Camera, &Parent)>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let mut input = PlayerInputBuffer::default();
    if let Ok((entity, _, parent)) = query_camera.get_single() {
        let tr_camera = query_transform.get(entity).unwrap();
        let tr_body = query_transform.get(parent.get()).unwrap();
        input.buffer.push(PlayerInput::RotationCamera(
            tr_camera.rotation.to_euler(EulerRot::YXZ).1,
        ));
        input.buffer.push(PlayerInput::RotationBody(
            tr_body.rotation.to_euler(EulerRot::YXZ).0,
        ));
    }

    let mut delta = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        delta += Vec3::X;
    }
    if keys.pressed(KeyCode::KeyS) {
        delta -= Vec3::X;
    }
    if keys.pressed(KeyCode::KeyA) {
        delta += Vec3::Z;
    }
    if keys.pressed(KeyCode::KeyD) {
        delta -= Vec3::Z;
    }
    delta = delta.normalize_or_zero();
    input.buffer.push(PlayerInput::Acceleration(delta));
    input
        .buffer
        .push(PlayerInput::Jumping(keys.pressed(KeyCode::Space)));
    input
        .buffer
        .push(PlayerInput::Placing(mouse.just_pressed(MouseButton::Right)));
    input
        .buffer
        .push(PlayerInput::Mining(mouse.just_pressed(MouseButton::Left)));

    player_input_buffer.buffer.append(&mut input.buffer);
}

pub fn client_send_input(
    mut res_player_input: ResMut<PlayerInputBuffer>,
    mut client: ResMut<RenetClient>,
) {
    let input_message = bincode::serialize(&*res_player_input).unwrap();
    // maybe unreliable is better (faster and if a packet is lost, whatever)
    client.send_message(ClientChannel::PlayerInput, input_message);
    res_player_input.buffer.clear();
}

pub fn server_receive_input(
    lobby: Res<Lobby>,
    mut server: ResMut<RenetServer>,
    mut player_input_query: Query<&mut PlayerInputBuffer>,
) {
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, ClientChannel::PlayerInput) {
            let mut player_input: PlayerInputBuffer = bincode::deserialize(&message).unwrap();
            if let Some(player_entity) = lobby.players.get(&client_id) {
                if let Ok(mut current_player_input) = player_input_query.get_mut(*player_entity) {
                    current_player_input.buffer.append(&mut player_input.buffer);
                }
            }
        }
    }
}

pub fn server_move_players(
    mut query_player: Query<
        (
            Entity,
            &mut CharacterController,
            &mut PlayerInputBuffer,
            &mut Transform,
            Option<&LocalPlayer>,
        ),
        Without<CameraController>,
    >,
    mut query_camera: Query<
        (&CameraController, &Parent, &mut Transform),
        Without<CharacterController>,
    >,
) {
    for (_, parent, mut tr_camera) in query_camera.iter_mut() {
        if let Ok((_, mut controller, mut input_buffer, mut tr, local)) =
            query_player.get_mut(parent.get())
        {
            input_buffer.buffer.retain(|input| match input {
                PlayerInput::Acceleration(acc) => {
                    controller.acceleration = *acc;
                    false
                }
                PlayerInput::Jumping(jumping) => {
                    controller.jumping = *jumping;
                    false
                }
                PlayerInput::RotationCamera(rot) => {
                    if local.is_none() {
                        tr_camera.rotation = Quat::from_axis_angle(Vec3::X, *rot);
                    }
                    false
                }
                PlayerInput::RotationBody(rot) => {
                    if local.is_none() {
                        tr.rotation = Quat::from_axis_angle(Vec3::Y, *rot);
                    }
                    false
                }
                _ => true,
            });
        }
    }
}
