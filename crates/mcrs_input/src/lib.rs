pub mod plugin;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Component, Resource, Clone)]
pub struct PlayerInputBuffer {
    pub buffer: Vec<PlayerInput>,
}

#[derive(Debug, Serialize, Deserialize, Component, Resource, Clone)]
pub enum PlayerInput {
    Acceleration(Vec3),
    RotationCamera(f32),
    RotationBody(f32),
    Jumping(bool),
    Placing(bool),
    Mining(bool),
}

pub fn player_input(
    mut player_input_buffer: ResMut<PlayerInputBuffer>,
    keys: Res<Input<KeyCode>>,
    query_transform: Query<&Transform>,
    query_camera: Query<(Entity, &Camera, &Parent)>,
    mouse: Res<Input<MouseButton>>,
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
    if keys.pressed(KeyCode::W) {
        delta += Vec3::X;
    }
    if keys.pressed(KeyCode::S) {
        delta -= Vec3::X;
    }
    if keys.pressed(KeyCode::A) {
        delta += Vec3::Z;
    }
    if keys.pressed(KeyCode::D) {
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
