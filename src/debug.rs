use crate::{
    client::open_client, server::open_server, Lobby, LocalPlayer, LocalPlayerId, NetSettings,
    PlayerId,
};
use bevy::{
    diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, DiagnosticsStore, RegisterDiagnostic},
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_egui::{egui, EguiContexts};
use bevy_renet::netcode::{NetcodeClientTransport, NetcodeServerTransport};
use mcrs_physics::{
    character::{CameraController, CharacterController, Rigidbody, Velocity},
    intersect::intersect_aabb_block,
    raycast::{cast_ray, RayFinite},
    TickStep,
};
use mcrs_universe::{universe::Universe, Blueprints, CHUNK_SIDE};
use renet::{RenetClient, RenetServer};

use crate::{
    player::spawn_camera, settings::McrsSettings, ChunkGenerationRequest, CloseLevelEvent,
    GenerationPass, Level, OpenLevelEvent, SaveLevelEvent,
};

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
        .add_event::<DebugCameraEvent>()
        .insert_resource(DebugOptions::default())
        .add_systems(
            Update,
            (
                debug_options_ui,
                (
                    debug_diagnostic_system,
                    debug_diagnostic_ui,
                    debug_saveload_ui,
                    debug_net_ui,
                    debug_show_hitboxes,
                    debug_camera_toggle,
                    debug_camera_movement,
                    debug_chunks,
                    debug_terrain_editing,
                )
                    .run_if(debug_active),
            ),
        );
    }
}

pub fn debug_active(debug_options: Res<DebugOptions>) -> bool {
    debug_options.active
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
    egui::Window::new("Debug Diagnostics")
        .anchor(egui::Align2::LEFT_BOTTOM, egui::Vec2::new(5.0, -5.0))
        .show(contexts.ctx_mut(), |ui| {
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

#[derive(Event)]
pub struct DebugCameraEvent {
    active: bool,
    has_character_control: bool,
}

#[derive(Resource)]
pub struct DebugOptions {
    pub active: bool,
    show_hitboxes: bool,
    debug_camera_active: bool,
    debug_camera_has_character_control: bool,
    show_chunks: bool,
}

impl Default for DebugOptions {
    fn default() -> Self {
        Self {
            active: false,
            show_hitboxes: false,
            debug_camera_active: false,
            debug_camera_has_character_control: false,
            show_chunks: false,
        }
    }
}

pub fn ui_toggle_shortcut(
    ui: &mut egui::Ui,
    keys: &ButtonInput<KeyCode>,
    value: &mut bool,
    text: &str,
    key: KeyCode,
) -> bool {
    if keys.just_pressed(key) {
        *value = !*value;
    }

    ui.checkbox(value, format!("{} [{:?}]", text, key))
        .clicked()
        || keys.just_pressed(key)
}

pub fn ui_button_shortcut(
    ui: &mut egui::Ui,
    keys: &ButtonInput<KeyCode>,
    text: &str,
    key: KeyCode,
    modifier: Option<Modifier>,
) -> bool {
    let (modifier_name, modifier_pressed) = if let Some(modifier) = modifier {
        (
            format!("{} ", modifier.get_name()),
            modifier.get_keys().iter().any(|m| keys.pressed(*m)),
        )
    } else {
        (String::new(), false)
    };
    ui.button(format!("{} [{}{:?}]", text, modifier_name, key))
        .clicked()
        || (keys.just_pressed(key) && modifier_pressed)
}

pub enum Modifier {
    Shift,
}

impl Modifier {
    fn get_name(&self) -> &'static str {
        match self {
            Modifier::Shift => "Shift",
        }
    }
    fn get_keys(&self) -> &[KeyCode] {
        match self {
            Modifier::Shift => &[KeyCode::ShiftLeft, KeyCode::ShiftRight],
        }
    }
}

pub fn debug_saveload_ui(
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    level: Option<Res<Level>>,
    mut open_event: EventWriter<OpenLevelEvent>,
    mut close_event: EventWriter<CloseLevelEvent>,
    mut save_event: EventWriter<SaveLevelEvent>,
    mut edit_level_name: Local<Option<String>>,
    settings: Res<McrsSettings>,
) {
    let Some(edit_level_name) = edit_level_name.as_mut() else {
        *edit_level_name = Some(settings.open_level_name.clone());
        return;
    };

    let ctx = contexts.ctx_mut();
    egui::Window::new("Debug Level")
        .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-5.0, 5.0))
        .show(ctx, |ui| {
            if let Some(level) = level {
                ui.label(format!("Loaded level: {}", level.name));
            } else {
                ui.label("No loaded level");
            }

            ui.horizontal(|ui| {
                ui.label("Level to Create/Load: ");
                ui.text_edit_singleline(edit_level_name);
            });

            if ui_button_shortcut(
                ui,
                &keys,
                "Open Level",
                KeyCode::KeyO,
                Some(Modifier::Shift),
            ) {
                open_event.send(OpenLevelEvent {
                    level_name: edit_level_name.clone(),
                });
            }
            if ui_button_shortcut(
                ui,
                &keys,
                "Save Level",
                KeyCode::KeyI,
                Some(Modifier::Shift),
            ) {
                save_event.send(SaveLevelEvent);
            }
            if ui_button_shortcut(
                ui,
                &keys,
                "Quit Level",
                KeyCode::KeyP,
                Some(Modifier::Shift),
            ) {
                close_event.send(CloseLevelEvent);
            }
        });
}

pub struct WidgetLobbyDebug<'a> {
    pub lobby: &'a Lobby,
}

impl<'a> egui::Widget for WidgetLobbyDebug<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.group(|ui| {
            for player in self.lobby.local_players.iter() {
                ui.label(format!("{} (Local)", player.name));
            }
            for player in self.lobby.remote_players.iter() {
                ui.label(format!("{} (Remote)", player.name));
            }
        })
        .response
    }
}

pub fn debug_net_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut renet_server: Option<ResMut<RenetServer>>,
    mut renet_client: Option<ResMut<RenetClient>>,
    net_settings: Option<Res<NetSettings>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    lobby: Res<Lobby>,
    mut local_player_id: ResMut<LocalPlayerId>,
    mut edit_name: Local<String>,
) {
    let ctx = contexts.ctx_mut();
    egui::Window::new("Debug Multiplayer")
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::Vec2::new(-5.0, -5.0))
        .show(ctx, |ui| {
            let Some(local_id) = local_player_id.id.as_ref() else {
                ui.label("Set your name:");
                if ui.text_edit_singleline(&mut *edit_name).has_focus() {
                    keys.reset_all();
                }
                if ui.button("Set").clicked() {
                    local_player_id.id = Some(PlayerId {
                        name: edit_name.clone(),
                    });
                }
                return;
            };

            ui.label(format!("Name: {}", local_id.name));

            if let Some(server) = &mut renet_server {
                ui.label("Server up");
                ui.add(WidgetLobbyDebug { lobby: &lobby });
                if ui.button("Close").clicked() {
                    server.disconnect_all();
                    commands.remove_resource::<RenetServer>();
                    commands.remove_resource::<NetcodeServerTransport>();
                }
            } else if let Some(client) = &mut renet_client {
                ui.label(format!(
                    "Client up, {}",
                    if client.is_connected() {
                        "connected"
                    } else {
                        "disconnected"
                    }
                ));
                ui.add(WidgetLobbyDebug { lobby: &lobby });
                if ui.button("Disconnect").clicked() {
                    client.disconnect();
                    commands.remove_resource::<RenetClient>();
                    commands.remove_resource::<NetcodeClientTransport>();
                }
            } else {
                ui.label("Disconnected");

                if ui_button_shortcut(
                    ui,
                    &keys,
                    "Open Client",
                    KeyCode::KeyC,
                    Some(Modifier::Shift),
                ) {
                    let address = net_settings
                        .as_ref()
                        .map(|settings| settings.server_address.clone())
                        .unwrap_or(format!("127.0.0.1"));
                    open_client(&mut commands, address);
                }

                if ui_button_shortcut(
                    ui,
                    &keys,
                    "Open Server",
                    KeyCode::KeyV,
                    Some(Modifier::Shift),
                ) {
                    let address = net_settings
                        .as_ref()
                        .map(|settings| settings.server_address.clone())
                        .unwrap_or(format!("127.0.0.1"));
                    open_server(&mut commands, address);
                }
            }
        });
}

pub fn debug_options_ui(
    mut contexts: EguiContexts,
    mut debug_options: ResMut<DebugOptions>,
    mut debug_camera_event: EventWriter<DebugCameraEvent>,
    keys: Res<ButtonInput<KeyCode>>,
    mut tickstep: ResMut<TickStep>,
) {
    let ctx = contexts.ctx_mut();
    if debug_options.active {
        egui::Window::new("Debug Options")
            .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(5.0, 5.0))
            .show(ctx, |ui| {
                ui_toggle_shortcut(
                    ui,
                    &keys,
                    &mut debug_options.active,
                    "Show debug windows",
                    KeyCode::F1,
                );

                if ui_button_shortcut(ui, &keys, "Physics Step 1 Tick", KeyCode::F2, None) {
                    *tickstep = TickStep::Step { step: true }
                }
                if ui_button_shortcut(ui, &keys, "Physics Resume Tick", KeyCode::F3, None) {
                    *tickstep = TickStep::Step { step: true }
                }

                ui_toggle_shortcut(
                    ui,
                    &keys,
                    &mut debug_options.show_hitboxes,
                    "Show Hitboxes",
                    KeyCode::F4,
                );

                if ui_toggle_shortcut(
                    ui,
                    &keys,
                    &mut debug_options.debug_camera_active,
                    "Debug Cam",
                    KeyCode::F5,
                ) {
                    debug_camera_event.send(DebugCameraEvent {
                        active: debug_options.debug_camera_active,
                        has_character_control: debug_options.debug_camera_has_character_control,
                    });
                }

                if ui_toggle_shortcut(
                    ui,
                    &keys,
                    &mut debug_options.debug_camera_has_character_control,
                    "Toggle Camera Control",
                    KeyCode::F6,
                ) {
                    debug_camera_event.send(DebugCameraEvent {
                        active: debug_options.debug_camera_active,
                        has_character_control: debug_options.debug_camera_has_character_control,
                    });
                }

                ui_toggle_shortcut(
                    ui,
                    &keys,
                    &mut debug_options.show_chunks,
                    "Toggle Chunk Borders",
                    KeyCode::F7,
                );
            });
    } else {
        if keys.just_pressed(KeyCode::F1) {
            debug_options.active = true;
        }
    }
}

#[derive(Component)]
pub struct DebugCamera {
    speed: f32,
    controller: CameraController,
}

pub fn set_active_controller_cameras(
    camera_pivot_query: &Query<(&GlobalTransform, &CameraController)>,
    all_cameras: &mut Query<(&mut Camera, &Parent)>,
    is_active: bool,
) {
    all_cameras.iter_mut().for_each(|(mut cam, parent)| {
        if camera_pivot_query.get(parent.get()).is_ok() {
            cam.is_active = is_active
        }
    });
}

pub fn set_active_character(
    local_character_controllers: &mut Query<(&LocalPlayer, &mut CharacterController)>,
    is_active: bool,
) {
    local_character_controllers
        .iter_mut()
        .for_each(|(_, mut contr)| contr.is_active = is_active);
}

pub fn debug_camera_toggle(
    mut commands: Commands,
    camera_pivot_query: Query<(&GlobalTransform, &CameraController)>,
    debug_camera_query: Query<(Entity, &DebugCamera)>,
    mut all_cameras: Query<(&mut Camera, &Parent)>,
    mut local_character_controllers: Query<(&LocalPlayer, &mut CharacterController)>,
    settings: Res<McrsSettings>,
    mut debug_camera_event: EventReader<DebugCameraEvent>,
) {
    for event in debug_camera_event.read() {
        if let Ok((debug_cam, _)) = debug_camera_query.get_single() {
            if !event.active {
                commands.entity(debug_cam).despawn_recursive();

                set_active_controller_cameras(&camera_pivot_query, &mut all_cameras, true);
            }
        } else {
            if event.active {
                let Ok((tr, controller)) = camera_pivot_query.get_single() else {
                    warn!("No player character");
                    return;
                };
                let camera_pivot = commands.spawn((
                    DebugCamera {
                        speed: 0.1,
                        controller: controller.clone(),
                    },
                    tr.compute_transform(),
                ));
                spawn_camera(camera_pivot, &settings);

                set_active_controller_cameras(&camera_pivot_query, &mut all_cameras, false);
            }
        }

        set_active_character(
            &mut local_character_controllers,
            event.has_character_control,
        );
    }
}

pub fn debug_show_hitboxes(
    local_characters: Query<
        (&LocalPlayer, &Transform, &Rigidbody, &Velocity),
        Without<DebugCamera>,
    >,
    mut gizmos: Gizmos,
    debug_options: Res<DebugOptions>,
) {
    if debug_options.show_hitboxes {
        for (_, character_tr, rigidbody, vel) in local_characters.iter() {
            gizmos.cuboid(
                character_tr
                    .with_rotation(Quat::IDENTITY)
                    .with_scale(rigidbody.size),
                Color::srgb(0.0, 0.8, 0.0),
            );
            gizmos.arrow(
                character_tr.translation,
                character_tr.translation + vel.vel,
                Color::srgb(0.0, 0.8, 0.0),
            );
        }
    }
}

pub fn debug_camera_movement(
    mut camera_query: Query<(&DebugCamera, &mut Transform)>,
    mut mouse_motion: EventReader<MouseMotion>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    debug_options: Res<DebugOptions>,
) {
    let Ok(window) = primary_window.get_single() else {
        return;
    };

    if debug_options.debug_camera_has_character_control {
        return;
    }

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

pub fn debug_chunks(
    universe: Res<Universe>,
    chunk_gen: Res<ChunkGenerationRequest>,
    mut gizmos: Gizmos,
    debug_options: Res<DebugOptions>,
) {
    if !debug_options.show_chunks {
        return;
    }

    for (chunk_pos, gen_request) in &chunk_gen.requested {
        let scale = Vec3::splat(CHUNK_SIDE as f32);
        let center = chunk_pos.as_vec3() + scale * 0.5;
        let color = match gen_request.pass {
            GenerationPass::Blocks => Color::srgb(1.0, 0.0, 0.0),
            GenerationPass::Lighting => Color::srgb(1.0, 0.2, 0.0),
            GenerationPass::WaitingForSunbeams => Color::srgb(1.0, 0.4, 0.0),
            GenerationPass::Sunbeams => Color::srgb(1.0, 0.6, 0.0),
            GenerationPass::Biome => Color::srgb(1.0, 0.8, 0.0),
            GenerationPass::Done => Color::srgb(1.0, 1.0, 0.0),
        };
        gizmos.cuboid(Transform::from_translation(center).with_scale(scale), color);
    }
    for (chunk_pos, _) in &universe.chunks {
        let scale = Vec3::splat(CHUNK_SIDE as f32);
        let center = chunk_pos.as_vec3() + scale * 0.5;
        gizmos.cuboid(
            Transform::from_translation(center).with_scale(scale),
            Color::srgb(0.0, 0.5, 0.0),
        );
    }
}

pub fn debug_terrain_editing(
    camera_query: Query<(&CameraController, &GlobalTransform, &Parent)>,
    mut player_query: Query<(&Transform, &Rigidbody)>,
    universe: Res<Universe>,
    bp: Res<Blueprints>,
    mut gizmos: Gizmos,
    mut contexts: EguiContexts,
    mut hide_red_cube: Local<bool>,
) {
    for (_cam, tr, parent) in camera_query.iter() {
        let Ok((tr_player, rigidbody)) = player_query.get_mut(parent.get()) else {
            continue;
        };

        let hit_option = cast_ray(
            RayFinite {
                position: tr.translation(),
                direction: tr.forward().as_vec3(),
                reach: 4.5,
            },
            &universe,
        );

        egui::Window::new("Debug Player Raycast Hit")
            .anchor(egui::Align2::LEFT_CENTER, egui::Vec2::new(5.0, 0.0))
            .show(contexts.ctx_mut(), |ui| {
                if let Some(hit) = &hit_option {
                    ui.add(WidgetBlockDebug::new(hit.grid_pos, &universe, &bp));
                    if !*hide_red_cube {
                        ui.add(WidgetBlockDebug::new(
                            hit.grid_pos + hit.normal(),
                            &universe,
                            &bp,
                        ));
                    }
                }
                ui.checkbox(&mut hide_red_cube, "Hide the facing cube in red");
            });

        if let Some(hit) = &hit_option {
            let intersection = hit.final_position();

            gizmos.cuboid(
                Transform::from_translation(intersection).with_scale(Vec3::splat(0.01)),
                Color::BLACK,
            );

            let center_pos = hit.grid_pos.as_vec3() + Vec3::splat(0.5);
            gizmos.cuboid(
                Transform::from_translation(center_pos).with_scale(Vec3::splat(1.001)),
                Color::BLACK,
            );

            if !*hide_red_cube {
                let block_pos = hit.grid_pos + hit.normal();
                if !intersect_aabb_block(tr_player.translation, rigidbody.size, block_pos) {
                    gizmos.cuboid(
                        Transform::from_translation(center_pos + hit.normal().as_vec3())
                            .with_scale(Vec3::splat(1.001)),
                        Color::srgb(1.0, 0.0, 0.0),
                    );
                } else {
                    gizmos.cuboid(
                        Transform::from_translation(center_pos + hit.normal().as_vec3())
                            .with_scale(Vec3::splat(1.001)),
                        Color::srgb(0.5, 0.0, 0.0),
                    );
                }
                gizmos.arrow(
                    intersection,
                    intersection + hit.normal().as_vec3() * 0.5,
                    Color::srgb(1.0, 0.0, 0.0),
                );
            }
        }
    }
}
