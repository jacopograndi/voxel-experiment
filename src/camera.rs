use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use mcrs_physics::character::{CameraController, Character, CharacterController};
use crate::LocalPlayer;

pub struct McrsCameraPlugin;

impl Plugin for McrsCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, initial_grab_cursor);
        app.add_systems(Update, (camera_controller_movement, cursor_grab));
        app.add_systems(PostUpdate, lock_cursor_position);
    }
}

/// Move the camera up and down and the player body left and right.
pub fn camera_controller_movement(
    mut camera_query: Query<(&CameraController, &mut Transform, &Parent)>,
    mut character_query: Query<
        (&Character, &mut Transform, &CharacterController),
        Without<CameraController>,
    >,
    mut mouse_motion: EventReader<MouseMotion>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    query_local: Query<&LocalPlayer>,
) {
    let Ok(window) = primary_window.get_single() else {
        return;
    };
    for (camera_controller, mut camera_tr, parent) in camera_query.iter_mut() {
        if query_local.get(parent.get()).is_err() {
            continue;
        }
        let Ok((_character, mut parent_tr, character_controller)) =
            character_query.get_mut(parent.get())
        else {
            continue;
        };
        if !character_controller.is_active {
            continue;
        }
        for ev in mouse_motion.read() {
            let (mut yaw, _, _) = parent_tr.rotation.to_euler(EulerRot::YXZ);
            let (_, mut pitch, _) = camera_tr.rotation.to_euler(EulerRot::YXZ);
            match window.cursor_options.grab_mode {
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
    keys: Res<ButtonInput<KeyCode>>,
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
    match window.cursor_options.grab_mode {
        CursorGrabMode::None => {
            window.cursor_options.grab_mode = CursorGrabMode::Confined;
            window.cursor_options.visible = false;
        }
        _ => {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
        }
    }
}

fn lock_cursor_position(mut primary_window: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = primary_window.get_single_mut() {
        let (w, h) = (window.width(), window.height());
        match window.cursor_options.grab_mode {
            CursorGrabMode::None => {}
            _ => {
                window.set_cursor_position(Some(Vec2::new(w / 2., h / 2.)));
            }
        }
    }
}
