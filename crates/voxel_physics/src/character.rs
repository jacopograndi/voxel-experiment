use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};

use voxel_storage::chunk_map::ChunkMap;

use crate::{raycast::*, MARGIN_EPSILON};

#[derive(Debug, Clone, Default)]
pub struct CharacterId(pub u32);

#[derive(Component, Debug, Clone, Default)]
pub struct Character {
    pub id: CharacterId,
    pub size: Vec3,
    pub air_speed: f32,
    pub ground_speed: f32,
    pub jump_strenght: f32,
}

#[derive(Component, Debug, Clone, Default)]
pub struct Velocity {
    pub vel: Vec3,
}

#[derive(Component, Debug, Clone, Default)]
pub struct Friction {
    pub air: Vec3,
    pub ground: Vec3,
}

#[derive(Component, Debug, Clone, Default)]
pub struct CharacterController {
    pub acceleration: Vec3,
    pub jumping: bool,
}

#[derive(Component, Debug, Clone)]
pub struct CameraController {
    pub sensitivity: Vec3,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            sensitivity: Vec3::splat(0.00012),
        }
    }
}

pub fn character_controller_movement(
    mut character_query: Query<(
        &Character,
        &CharacterController,
        &mut Transform,
        &mut Velocity,
        &mut Friction,
    )>,
    chunk_map: Res<ChunkMap>,
) {
    for (character, controller, mut tr, mut vel, friction) in character_query.iter_mut() {
        vel.vel -= Vec3::Y * 0.01;

        let mut grounded = false;
        if let Some(hit) = sweep_aabb(
            tr.translation,
            character.size,
            Vec3::NEG_Y,
            MARGIN_EPSILON * 2.,
            &chunk_map,
        ) {
            if hit.distance <= MARGIN_EPSILON * 2. {
                grounded = true;
                if controller.jumping {
                    vel.vel += Vec3::Y * character.jump_strenght;
                    grounded = false;
                }
            } else {
                grounded = false;
            }
        }
        if grounded {
            vel.vel += controller.acceleration * Vec3::new(1.0, 0.0, 1.0) * character.ground_speed;
            vel.vel *= friction.ground;
        } else {
            vel.vel += controller.acceleration * Vec3::new(1.0, 0.0, 1.0) * character.air_speed;
            vel.vel *= friction.air;
        }

        for _ in 0..4 {
            if let Some(hit) = sweep_aabb(
                tr.translation,
                character.size,
                vel.vel.normalize_or_zero(),
                vel.vel.length(),
                &chunk_map,
            ) {
                tr.translation += vel.vel.normalize_or_zero() * (hit.distance - MARGIN_EPSILON);
                vel.vel *= (IVec3::ONE - hit.blocked).as_vec3();
                if vel.vel.length() < MARGIN_EPSILON {
                    break;
                }
            }
        }
        tr.translation += vel.vel;
}
}

pub fn camera_controller_movement(
    mut camera_query: Query<(&CameraController, &mut Transform, &Parent)>,
    mut character_query: Query<
        (&Character, &mut Transform, &CharacterController),
        Without<CameraController>,
    >,
    mut mouse_motion: EventReader<MouseMotion>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = primary_window.get_single() else {
        return;
    };
    for (camera_controller, mut camera_tr, parent) in camera_query.iter_mut() {
        let Ok((_character, mut parent_tr, _character_controller)) =
            character_query.get_mut(parent.get())
        else {
            continue;
        };
        for ev in mouse_motion.read() {
            let (mut yaw, _, _) = parent_tr.rotation.to_euler(EulerRot::YXZ);
            let (_, mut pitch, _) = camera_tr.rotation.to_euler(EulerRot::YXZ);
            match window.cursor.grab_mode {
                CursorGrabMode::None => (),
                _ => {
                    // Using smallest of height or width ensures equal vertical and horizontal sensitivity
                    let window_scale = window.height().min(window.width());
                    pitch -=
                        (camera_controller.sensitivity.y * ev.delta.y * window_scale).to_radians();
                    yaw -=
                        (camera_controller.sensitivity.x * ev.delta.x * window_scale).to_radians();
                }
            }
            pitch = pitch.clamp(-1.54, 1.54);
            parent_tr.rotation = Quat::from_axis_angle(Vec3::Y, yaw);
            camera_tr.rotation = Quat::from_axis_angle(Vec3::X, pitch);
        }
    }
}

/// Grabs the cursor when game first starts
pub fn initial_grab_cursor(mut primary_window: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = primary_window.get_single_mut() {
        toggle_grab_cursor(&mut window);
    } else {
        warn!("Primary window not found for `initial_grab_cursor`!");
    }
}

/// Grabs/ungrabs mouse cursor
pub fn cursor_grab(
    keys: Res<Input<KeyCode>>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok(mut window) = primary_window.get_single_mut() {
        if keys.just_pressed(KeyCode::Escape) {
            toggle_grab_cursor(&mut window);
        }
    } else {
        warn!("Primary window not found for `cursor_grab`!");
    }
}

fn toggle_grab_cursor(window: &mut Window) {
    match window.cursor.grab_mode {
        CursorGrabMode::None => {
            window.cursor.grab_mode = CursorGrabMode::Confined;
            window.cursor.visible = false;
        }
        _ => {
            window.cursor.grab_mode = CursorGrabMode::None;
            window.cursor.visible = true;
        }
    }
}
