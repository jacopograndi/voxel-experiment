use bevy::prelude::*;
use mcrs_physics::character::CharacterController;
use serde::{Deserialize, Serialize};

use crate::net::{LocalPlayer, NetworkMode};

#[derive(Debug, Default, Serialize, Deserialize, Component, Resource, Clone)]
pub struct PlayerInput {
    pub acceleration: Vec3,
    pub rotation_camera: f32,
    pub rotation_body: f32,
    pub jumping: bool,
    pub placing: bool,
    pub mining: bool,
    pub block_in_hand: u8,
}

impl PlayerInput {
    pub fn update(&mut self, next: Self) {
        let old = self.clone();
        *self = next.clone();
        self.placing |= old.placing;
        self.mining |= old.mining;
    }

    pub fn consume(&mut self) {
        let old = self.clone();
        *self = Self::default();
        self.rotation_body = old.rotation_body;
        self.rotation_camera = old.rotation_camera;
    }
}

pub fn player_input(
    mut player_input: ResMut<PlayerInput>,
    keys: Res<Input<KeyCode>>,
    query_transform: Query<&Transform>,
    query_camera: Query<(Entity, &Camera, &Parent)>,
    mouse: Res<Input<MouseButton>>,
    mut query_player: Query<&mut CharacterController, With<LocalPlayer>>,
    network_mode: Res<NetworkMode>,
) {
    let mut input = PlayerInput::default();
    if let Ok((entity, _, parent)) = query_camera.get_single() {
        let tr_camera = query_transform.get(entity).unwrap();
        let tr_body = query_transform.get(parent.get()).unwrap();
        input.rotation_camera = tr_camera.rotation.to_euler(EulerRot::YXZ).1;
        input.rotation_body = tr_body.rotation.to_euler(EulerRot::YXZ).0;
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
    input.acceleration = delta;
    input.jumping = keys.pressed(KeyCode::Space);
    input.placing = mouse.just_pressed(MouseButton::Right);
    input.mining = mouse.just_pressed(MouseButton::Left);

    player_input.update(input);

    if matches!(*network_mode, NetworkMode::ClientAndServer) {
        if let Ok(mut controller) = query_player.get_single_mut() {
            controller.acceleration = player_input.acceleration;
            controller.jumping = player_input.jumping;
        }
    }
}
