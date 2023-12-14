use std::sync::{Arc, RwLock};

use bevy::{
    asset::LoadState,
    core_pipeline::fxaa::Fxaa,
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics, DiagnosticsStore, RegisterDiagnostic},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_flycam::prelude::*;

use voxel_physics::{
    character::{Character, CharacterController, CharacterId, Friction, Velocity},
    plugin::VoxelPhysicsPlugin,
    raycast,
};
use voxel_render::{
    voxel_world::{RenderHandles, VIEW_DISTANCE},
    VoxelCameraBundle, VoxelRenderPlugin,
};
use voxel_storage::{
    chunk_map::{Chunk, ChunkMap, GridPtr},
    grid::{Grid, Voxel},
    CHUNK_SIDE,
};

pub const DIAGNOSTIC_FPS: DiagnosticId =
    DiagnosticId::from_u128(288146834822086093791974408528866909484);
pub const DIAGNOSTIC_FRAME_TIME: DiagnosticId =
    DiagnosticId::from_u128(54021991829115352065418785002088010278);

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
        NoCameraPlayerPlugin,
        VoxelRenderPlugin,
        VoxelPhysicsPlugin,
        EguiPlugin,
    ))
    .register_diagnostic(
        Diagnostic::new(DIAGNOSTIC_FRAME_TIME, "frame_time", 1000).with_suffix("ms"),
    )
    .register_diagnostic(Diagnostic::new(DIAGNOSTIC_FPS, "fps", 1000))
    .insert_resource(ClearColor(Color::MIDNIGHT_BLUE))
    .insert_resource(Handles::default())
    .add_state::<FlowState>()
    .add_systems(Startup, load)
    .add_systems(OnEnter(FlowState::Base), setup)
    .add_systems(Update, check_loading.run_if(in_state(FlowState::Loading)))
    .add_systems(Update, ui)
    .add_systems(Update, load_and_gen_chunks)
    .add_systems(Update, control)
    .add_systems(Update, diagnostic_system);

    app.add_systems(Update, voxel_break);

    app.run();
}

// just for prototype
fn voxel_break(
    camera_query: Query<(&Camera, &Transform)>,
    mut chunk_map: ResMut<ChunkMap>,
    mouse: Res<Input<MouseButton>>,
) {
    if let Ok((_cam, tr)) = camera_query.get_single() {
        #[derive(PartialEq)]
        enum Act {
            PlaceBlock,
            RemoveBlock,
            Inspect,
        }
        let act = match (
            mouse.pressed(MouseButton::Left),
            mouse.pressed(MouseButton::Right),
            mouse.pressed(MouseButton::Middle),
        ) {
            (true, _, _) => Some(Act::PlaceBlock),
            (_, true, _) => Some(Act::RemoveBlock),
            (_, _, true) => Some(Act::Inspect),
            _ => None,
        };
        if let Some(act) = act {
            if let Some(hit) = raycast::raycast(tr.translation, tr.forward(), 4.5, &chunk_map) {
                match act {
                    Act::Inspect => {
                        println!(
                            "pos:{}, {:?}, dist:{}",
                            hit.pos,
                            chunk_map.get_at(&hit.pos),
                            hit.distance
                        );
                    }
                    Act::RemoveBlock => {
                        chunk_map.set_at(
                            &hit.pos,
                            Voxel {
                                id: 0,
                                flags: 0,
                                ..default()
                            },
                        );
                    }
                    Act::PlaceBlock => {
                        let pos = hit.pos + hit.normal;
                        chunk_map.set_at(
                            &pos,
                            Voxel {
                                id: 2,
                                flags: 16,
                                ..default()
                            },
                        );
                    }
                };
            } else {
                //dbg!("no hit");
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum FlowState {
    #[default]
    Loading,
    Base,
}

#[derive(Resource, Default)]
struct Handles {
    texture_blocks: Handle<Image>,
}

fn load(
    mut handles: ResMut<Handles>,
    asset_server: Res<AssetServer>,
    mut render_handles: ResMut<RenderHandles>,
) {
    handles.texture_blocks = asset_server.load("textures/blocks.png");
    render_handles.texture_blocks = handles.texture_blocks.clone();
}

fn check_loading(
    handles: Res<Handles>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<FlowState>>,
) {
    match asset_server.get_load_state(handles.texture_blocks.clone()) {
        Some(LoadState::Loaded) => next_state.set(FlowState::Base),
        _ => (),
    }
}

fn gen_chunk(pos: IVec3) -> GridPtr {
    let grid = if pos.y < 0 {
        Grid::filled()
    } else {
        Grid::empty()
    };
    GridPtr(Arc::new(RwLock::new(grid)))
}

fn load_and_gen_chunks(mut chunk_map: ResMut<ChunkMap>, camera: Query<(&Camera, &Transform)>) {
    let load_view_distance: u32 = VIEW_DISTANCE + CHUNK_SIDE as u32 * 2;

    let camera_pos = if let Ok((_, tr)) = camera.get_single() {
        tr.translation
    } else {
        return;
    };

    let camera_chunk_pos = (camera_pos / CHUNK_SIDE as f32).as_ivec3() * CHUNK_SIDE as i32;

    // hardcoded chunk size
    let load_view_distance_chunk = load_view_distance as i32 / CHUNK_SIDE as i32;
    let lvdc = load_view_distance_chunk;

    // sphere centered on the player
    for x in -lvdc..=lvdc {
        for y in -lvdc..=lvdc {
            for z in -lvdc..=lvdc {
                let rel = IVec3::new(x, y, z) * CHUNK_SIDE as i32;
                if rel.as_vec3().length_squared() < load_view_distance.pow(2) as f32 {
                    let pos = camera_chunk_pos + rel;
                    if let None = chunk_map.chunks.get(&pos) {
                        // gen chunk
                        //println!("gen {:?}", pos);
                        let grid_ptr = gen_chunk(pos);
                        chunk_map.chunks.insert(
                            pos,
                            Chunk {
                                grid: grid_ptr,
                                version: 0,
                            },
                        );
                    }
                }
            }
        }
    }
}

fn setup(mut commands: Commands) {
    // bevy-fly-cam camera settings
    // bevy-fly-cam is prototype only
    commands.insert_resource(MovementSettings {
        sensitivity: 0.00015,
        speed: 30.0,
    });
    commands.insert_resource(KeyBindings {
        move_ascend: KeyCode::U,
        move_descend: KeyCode::O,
        move_forward: KeyCode::I,
        move_backward: KeyCode::K,
        move_left: KeyCode::J,
        move_right: KeyCode::L,
        ..Default::default()
    });

    // voxel camera
    commands.spawn((
        VoxelCameraBundle {
            transform: Transform::from_xyz(5.0, 5.0, -5.0).looking_at(Vec3::ZERO, Vec3::Y),
            projection: Projection::Perspective(PerspectiveProjection {
                fov: 1.57,
                ..default()
            }),
            ..default()
        },
        Fxaa::default(),
        FlyCam,
        Velocity::default(),
        Friction {
            air: Vec3::splat(0.9),
            ground: Vec3::splat(0.9),
        },
        Character {
            id: CharacterId(0),
            size: Vec3::new(0.5, 1.5, 0.5),
            speed: 0.04,
        },
        CharacterController {
            acceleration: Vec3::splat(0.0),
        },
    ));

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(5.0),
                    height: Val::Px(5.0),
                    ..default()
                },
                background_color: Color::rgba(0.1, 0.1, 0.1, 0.3).into(),
                ..default()
            });
        });
}

fn control(
    mut character_query: Query<(&mut CharacterController, &Transform)>,
    keys: Res<Input<KeyCode>>,
) {
    for (mut controller, tr) in character_query.iter_mut() {
        let mut delta = Vec3::ZERO;
        if keys.pressed(KeyCode::W) {
            delta += tr.forward();
        }
        if keys.pressed(KeyCode::S) {
            delta -= tr.forward();
        }
        if keys.pressed(KeyCode::A) {
            delta += tr.left();
        }
        if keys.pressed(KeyCode::D) {
            delta -= tr.left();
        }
        if keys.pressed(KeyCode::Q) {
            delta += Vec3::Y;
        }
        if keys.pressed(KeyCode::E) {
            delta -= Vec3::Y;
        }
        delta = delta.normalize_or_zero();
        controller.acceleration = delta;
    }
}

fn ui(mut contexts: EguiContexts, diagnostics: Res<DiagnosticsStore>) {
    egui::Window::new("Debug menu").show(contexts.ctx_mut(), |ui| {
        if let Some(value) = diagnostics
            .get(DIAGNOSTIC_FPS)
            .and_then(|fps| fps.smoothed())
        {
            ui.label(format!("fps: {value:>4.2}"));
            use egui_plot::{Line, PlotPoints};
            let n = 1000;
            let line_points: PlotPoints = if let Some(diag) = diagnostics.get(DIAGNOSTIC_FPS) {
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
            .get(DIAGNOSTIC_FRAME_TIME)
            .and_then(|ms| ms.value())
        {
            ui.label(format!("time: {value:>4.2} ms"));
            use egui_plot::{Line, PlotPoints};
            let n = 1000;
            let line_points: PlotPoints = if let Some(diag) = diagnostics.get(DIAGNOSTIC_FRAME_TIME)
            {
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

pub fn diagnostic_system(mut diagnostics: Diagnostics, time: Res<Time<Real>>) {
    let delta_seconds = time.delta_seconds_f64();
    if delta_seconds == 0.0 {
        return;
    }
    diagnostics.add_measurement(DIAGNOSTIC_FRAME_TIME, || delta_seconds * 1000.0);
    diagnostics.add_measurement(DIAGNOSTIC_FPS, || 1.0 / delta_seconds);
}
