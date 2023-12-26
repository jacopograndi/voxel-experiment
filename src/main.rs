use std::f32::consts::PI;

use bevy::{
    core_pipeline::fxaa::Fxaa,
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics, DiagnosticsStore, RegisterDiagnostic},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

use bevy_egui::{egui, EguiContexts, EguiPlugin};

use voxel_physics::{
    character::{
        CameraController, Character, CharacterController, CharacterId, Friction, Velocity,
    },
    plugin::VoxelPhysicsPlugin,
    raycast,
};
use voxel_render::{
    boxes_world::{Ghost, VoxTextureIndex, VoxTextureLoadQueue},
    voxel_world::VIEW_DISTANCE,
    VoxelCameraBundle, VoxelRenderPlugin,
};
use voxel_storage::{
    BlockId,
    chunk::Chunk,
    universe::Universe,
    VoxelStoragePlugin, CHUNK_SIDE,
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
        VoxelRenderPlugin,
        VoxelPhysicsPlugin,
        VoxelStoragePlugin,
        EguiPlugin,
    ))
    .register_diagnostic(
        Diagnostic::new(DIAGNOSTIC_FRAME_TIME, "frame_time", 1000).with_suffix("ms"),
    )
    .register_diagnostic(Diagnostic::new(DIAGNOSTIC_FPS, "fps", 1000))
    .insert_resource(ClearColor(Color::MIDNIGHT_BLUE))
    .add_systems(Startup, setup)
    .add_systems(Update, ui)
    .add_systems(Update, load_and_gen_chunks)
    .add_systems(Update, control)
    .add_systems(Update, diagnostic_system)
    .add_systems(Update, spin);

    app.add_systems(Update, voxel_break);

    app.run();
}

// just for prototype
fn voxel_break(
    camera_query: Query<(&CameraController, &GlobalTransform)>,
    mut chunk_map: ResMut<Universe>,
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
            mouse.just_pressed(MouseButton::Left),
            mouse.just_pressed(MouseButton::Right),
            mouse.just_pressed(MouseButton::Middle),
        ) {
            (true, _, _) => Some(Act::RemoveBlock),
            (_, true, _) => Some(Act::PlaceBlock),
            (_, _, true) => Some(Act::Inspect),
            _ => None,
        };
        if let Some(act) = act {
            if let Some(hit) = raycast::raycast(tr.translation(), tr.forward(), 4.5, &chunk_map) {
                match act {
                    Act::Inspect => {
                        println!(
                            "pos:{}, {:?}, dist:{}",
                            hit.pos,
                            chunk_map.read_chunk(&hit.grid_pos),
                            hit.distance
                        );
                    }
                    Act::RemoveBlock => {
                        chunk_map.set_chunk(
                            &hit.grid_pos,
                            BlockId::AIR,
                        );
                    }
                    Act::PlaceBlock => {
                        let pos = hit.grid_pos + hit.normal;
                        chunk_map.set_chunk(
                            &pos,
                            BlockId::LOG,
                        );
                    }
                };
            } else {
                //dbg!("no hit");
            }
        }
    }
}

fn gen_chunk(pos: IVec3) -> Chunk {
    if pos.y < 0 {
        Chunk::filled()
    } else {
        Chunk::empty()
    }
}

fn load_and_gen_chunks(mut universe: ResMut<Universe>, camera: Query<(&Camera, &Transform)>) {
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
                    if let None = universe.chunks.get(&pos) {
                        // gen chunk
                        //println!("gen {:?}", pos);
                        let chunk: Chunk = gen_chunk(pos);
                        universe.chunks.insert(
                            pos,
                            chunk
                        );
                    }
                }
            }
        }
    }
}

fn setup(mut commands: Commands, mut queue: ResMut<VoxTextureLoadQueue>) {
    queue
        .to_load
        .push(("assets/voxels/stone.vox".to_string(), VoxTextureIndex(0)));
    queue
        .to_load
        .push(("assets/voxels/dirt.vox".to_string(), VoxTextureIndex(1)));
    queue
        .to_load
        .push(("assets/voxels/wood-oak.vox".to_string(), VoxTextureIndex(2)));

    // player character
    commands
        .spawn((
            SpatialBundle::from_transform(Transform::from_xyz(0.0, 5.0, 0.0)),
            Character {
                id: CharacterId(0),
                size: Vec3::new(0.5, 1.5, 0.5),
                air_speed: 0.001,
                ground_speed: 0.03,
                jump_strenght: 0.17,
            },
            CharacterController {
                acceleration: Vec3::splat(0.0),
                jumping: false,
            },
            Velocity::default(),
            Friction {
                air: Vec3::splat(0.99),
                ground: Vec3::splat(0.78),
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                VoxelCameraBundle {
                    transform: Transform::from_xyz(0.0, 0.5, 0.0),
                    projection: Projection::Perspective(PerspectiveProjection {
                        fov: 1.57,
                        ..default()
                    }),
                    ..default()
                },
                Fxaa::default(),
                CameraController::default(),
            ));
        });

    // center cursor
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

    commands.spawn((
        SpatialBundle::from_transform(Transform {
            translation: Vec3::new(0.0, 13.0 / 16.0 * 0.5, 0.0),
            ..default()
        }),
        Ghost {
            vox_texture_index: VoxTextureIndex(0),
        },
    ));

    commands.spawn((
        SpatialBundle::from_transform(Transform {
            translation: Vec3::new(3.0, 14.0 / 16.0 * 0.5, -2.0),
            rotation: Quat::from_rotation_y(PI / 2.0),
            ..default()
        }),
        Ghost {
            vox_texture_index: VoxTextureIndex(1),
        },
        Party::default(),
    ));
}

#[derive(Component, Clone, Default, Debug)]
struct Party {
    scale: Option<Vec3>,
}

fn spin(mut q: Query<(&mut Transform, &mut Party)>, time: Res<Time<Real>>) {
    for (mut tr, mut party) in q.iter_mut() {
        tr.rotate_y(0.1);
        if let None = party.scale {
            party.scale = Some(tr.scale)
        }
        tr.scale = party.scale.unwrap() * f32::cos(time.elapsed_seconds());
    }
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
        delta = delta.normalize_or_zero();
        controller.acceleration = delta;
        if keys.pressed(KeyCode::Space) {
            controller.jumping = true;
        } else {
            controller.jumping = false;
        }
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
