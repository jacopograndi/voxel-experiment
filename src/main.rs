use std::sync::{Arc, RwLock};

use bevy::{
    asset::LoadState,
    core_pipeline::fxaa::Fxaa,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

use bevy_flycam::prelude::*;

use voxel_physics::raycast;
use voxel_render::{
    voxel_world::{RenderHandles, VIEW_DISTANCE},
    BevyVoxelEnginePlugin, VoxelCameraBundle,
};
use voxel_storage::{
    chunk_map::{Chunk, ChunkMap, GridPtr},
    grid::{Grid, Voxel},
    CHUNK_SIDE,
};

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
        FrameTimeDiagnosticsPlugin,
        LogDiagnosticsPlugin::default(),
        NoCameraPlayerPlugin,
        BevyVoxelEnginePlugin,
    ))
    .insert_resource(ClearColor(Color::MIDNIGHT_BLUE))
    .insert_resource(Handles::default())
    .add_state::<FlowState>()
    .add_systems(Startup, load)
    .add_systems(OnEnter(FlowState::Base), setup)
    .add_systems(Update, check_loading.run_if(in_state(FlowState::Loading)))
    .add_systems(Update, print_mesh_count)
    .add_systems(Update, load_and_gen_chunks);

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
        }
        let act = match (
            mouse.pressed(MouseButton::Left),
            mouse.pressed(MouseButton::Right),
        ) {
            (true, false) => Some(Act::PlaceBlock),
            (false, true) => Some(Act::RemoveBlock),
            _ => None,
        };
        if let Some(act) = act {
            if let Some((pos, norm, dist)) =
                raycast::raycast(tr.translation, tr.forward(), &chunk_map)
            {
                if dist.is_finite() {
                    match act {
                        Act::RemoveBlock => {
                            chunk_map.set_at(
                                &pos,
                                Voxel {
                                    id: 0,
                                    flags: 0,
                                    ..default()
                                },
                            );
                        }
                        Act::PlaceBlock => {
                            let pos = pos + norm;
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
                }
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
        move_ascend: KeyCode::E,
        move_descend: KeyCode::Q,
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
    ));
}

// System for printing the number of meshes on every tick of the timer
fn print_mesh_count(
    time: Res<Time>,
    mut timer: Local<PrintingTimer>,
    sprites: Query<(&Handle<Mesh>, &ViewVisibility)>,
) {
    timer.tick(time.delta());

    if timer.just_finished() {
        info!(
            "Meshes: {} - Visible Meshes {}",
            sprites.iter().len(),
            sprites.iter().filter(|(_, vis)| vis.get()).count(),
        );
    }
}

#[derive(Deref, DerefMut)]
struct PrintingTimer(Timer);

impl Default for PrintingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}
