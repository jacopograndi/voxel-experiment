use bevy::{
    diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, DiagnosticsStore, RegisterDiagnostic},
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_egui::{egui, EguiContexts};
use mcrs_net::LocalPlayer;
use mcrs_physics::character::{CameraController, Character, CharacterController, Velocity};
use mcrs_universe::{universe::Universe, Blueprints};

use crate::{player::spawn_camera, settings::McrsSettings};

pub const DIAGNOSTIC_FPS: DiagnosticPath = DiagnosticPath::const_new("game/fps");
pub const DIAGNOSTIC_FRAME_TIME: DiagnosticPath = DiagnosticPath::const_new("game/frame_time");

pub struct DebugDiagnosticPlugin;

impl Plugin for DebugDiagnosticPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.register_diagnostic(
            Diagnostic::new(DIAGNOSTIC_FRAME_TIME)
                .with_max_history_length(1000)
                .with_suffix("ms"),
        )
        .register_diagnostic(Diagnostic::new(DIAGNOSTIC_FPS).with_max_history_length(1000))
        .add_systems(
            Update,
            (
                debug_diagnostic_system,
                debug_diagnostic_ui,
                debug_camera_toggle,
                debug_camera_movement,
            ),
        );
    }
}

pub fn debug_diagnostic_system(mut diagnostics: Diagnostics, time: Res<Time<Real>>) {
    let delta_seconds = time.delta_secs_f64();
    if delta_seconds == 0.0 {
        return;
    }
    diagnostics.add_measurement(&DIAGNOSTIC_FRAME_TIME, || delta_seconds * 1000.0);
    diagnostics.add_measurement(&DIAGNOSTIC_FPS, || 1.0 / delta_seconds);
}

pub fn debug_diagnostic_ui(mut contexts: EguiContexts, diagnostics: Res<DiagnosticsStore>) {
    egui::Window::new("Debug menu").show(contexts.ctx_mut(), |ui| {
        if let Some(value) = diagnostics
            .get(&DIAGNOSTIC_FPS)
            .and_then(|fps| fps.smoothed())
        {
            ui.label(format!("fps: {value:>4.2}"));
            use egui_plot::{Line, PlotPoints};
            let n = 1000;
            let line_points: PlotPoints = if let Some(diag) = diagnostics.get(&DIAGNOSTIC_FPS) {
                diag.values()
                    .take(n)
                    .enumerate()
                    .map(|(i, v)| [i as f64, *v])
                    .collect()
            } else {
                PlotPoints::default()
            };
            let line = Line::new(line_points).fill(0.0);
            egui_plot::Plot::new("fps")
                .include_y(0.0)
                .height(70.0)
                .show_axes([false, true])
                .show(ui, |plot_ui| plot_ui.line(line))
                .response;
        } else {
            ui.label("no fps data");
        }
        if let Some(value) = diagnostics
            .get(&DIAGNOSTIC_FRAME_TIME)
            .and_then(|ms| ms.value())
        {
            ui.label(format!("time: {value:>4.2} ms"));
            use egui_plot::{Line, PlotPoints};
            let n = 1000;
            let line_points: PlotPoints =
                if let Some(diag) = diagnostics.get(&DIAGNOSTIC_FRAME_TIME) {
                    diag.values()
                        .take(n)
                        .enumerate()
                        .map(|(i, v)| [i as f64, *v])
                        .collect()
                } else {
                    PlotPoints::default()
                };
            let line = Line::new(line_points).fill(0.0);
            egui_plot::Plot::new("frame time")
                .include_y(0.0)
                .height(70.0)
                .show_axes([false, true])
                .show(ui, |plot_ui| plot_ui.line(line))
                .response;
        } else {
            ui.label("no frame time data");
        }
        ui.separator()
    });
}

pub struct WidgetBlockDebug<'a> {
    pub pos: IVec3,
    pub universe: &'a Universe,
    pub bp: &'a Blueprints,
}

impl<'a> WidgetBlockDebug<'a> {
    pub fn new(pos: IVec3, universe: &'a Universe, bp: &'a Blueprints) -> Self {
        Self { pos, universe, bp }
    }
}

impl<'a> egui::Widget for WidgetBlockDebug<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.group(|ui| {
            if let Some(block) = self.universe.read_chunk_block(&self.pos) {
                egui::Grid::new("Block").striped(true).show(ui, |ui| {
                    let block_bp = self.bp.blocks.get(&block.id);
                    ui.label("Position");
                    ui.add(egui::Label::new(format!("{}", self.pos)));
                    ui.end_row();

                    ui.label("Type");
                    ui.label(format!("{}", block_bp.name));
                    ui.end_row();

                    ui.label("Id");
                    ui.label(format!("{:?}", block_bp.id));
                    ui.end_row();

                    ui.label("Brightness");
                    ui.label(format!("{}", block_bp.light_level));
                    ui.end_row();

                    ui.label("Lighting (torch)");
                    ui.label(format!("{}", block.light0));
                    ui.end_row();

                    ui.label("Lighting (sun)");
                    ui.label(format!("{}", block.light1));
                    ui.end_row();
                });
            }
        })
        .response
    }
}

#[derive(Component)]
pub struct DebugCamera {
    speed: f32,
    controller: CameraController,
}

pub fn debug_camera_toggle(
    mut commands: Commands,
    camera_pivot_query: Query<(&GlobalTransform, &CameraController)>,
    debug_camera_query: Query<(Entity, &DebugCamera)>,
    mut all_cameras: Query<&mut Camera>,
    mut local_character_controllers: Query<(&LocalPlayer, &mut CharacterController)>,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<McrsSettings>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        if let Ok((debug_cam, _)) = debug_camera_query.get_single() {
            commands.entity(debug_cam).despawn_recursive();
            all_cameras
                .iter_mut()
                .for_each(|mut cam| cam.is_active = true);
            local_character_controllers
                .iter_mut()
                .for_each(|(_, mut contr)| contr.is_active = true);
        } else {
            let Ok((tr, controller)) = camera_pivot_query.get_single() else {
                warn!("No player character");
                return;
            };
            all_cameras
                .iter_mut()
                .for_each(|mut cam| cam.is_active = false);
            local_character_controllers
                .iter_mut()
                .for_each(|(_, mut contr)| contr.is_active = false);
            let camera_pivot = commands.spawn((
                DebugCamera {
                    speed: 0.01,
                    controller: controller.clone(),
                },
                tr.compute_transform(),
            ));
            spawn_camera(camera_pivot, &settings);
        }
    }
}

pub fn debug_camera_movement(
    mut camera_query: Query<(&DebugCamera, &mut Transform)>,
    mut mouse_motion: EventReader<MouseMotion>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    local_characters: Query<
        (&LocalPlayer, &Transform, &Character, &Velocity),
        Without<DebugCamera>,
    >,
    mut gizmos: Gizmos,
) {
    if !camera_query.is_empty() {
        for (_, character_tr, character, vel) in local_characters.iter() {
            gizmos.cuboid(
                character_tr.with_scale(character.size),
                Color::srgb(0.0, 0.8, 0.0),
            );
            gizmos.arrow(
                character_tr.translation,
                character_tr.translation + vel.vel,
                Color::srgb(0.0, 0.8, 0.0),
            );
        }
    }

    let Ok(window) = primary_window.get_single() else {
        return;
    };

    for (debug_cam, mut tr) in camera_query.iter_mut() {
        for ev in mouse_motion.read() {
            let (mut yaw, mut pitch, _) = tr.rotation.to_euler(EulerRot::YXZ);
            match window.cursor_options.grab_mode {
                CursorGrabMode::None => (),
                _ => {
                    let window_scale = window.height().min(window.width());
                    pitch -= (debug_cam.controller.sensitivity.y * ev.delta.y * window_scale)
                        .to_radians();
                    yaw -= (debug_cam.controller.sensitivity.x * ev.delta.x * window_scale)
                        .to_radians();
                }
            }
            pitch = pitch.clamp(-1.54, 1.54);
            tr.rotation =
                Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);
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
        if keys.pressed(KeyCode::KeyE) {
            delta += Vec3::Y;
        }
        if keys.pressed(KeyCode::KeyQ) {
            delta -= Vec3::Y;
        }
        delta = delta.normalize_or_zero() * debug_cam.speed;
        let forward = tr.forward();
        let left = tr.left();
        tr.translation += forward * delta.x;
        tr.translation += Vec3::Y * delta.y;
        tr.translation += left * delta.z;
    }
}
