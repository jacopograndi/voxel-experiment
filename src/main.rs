use std::sync::{Arc, RwLock};

use bevy::{
    asset::LoadState,
    core_pipeline::fxaa::Fxaa,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    utils::HashMap,
    window::{PresentMode, WindowPlugin},
};

use bevy_flycam::prelude::*;

mod voxels;
use voxels::{
    grid_hierarchy::Grid,
    voxel_world::{Chunk, ChunkMap, GridPtr},
    BevyVoxelEnginePlugin, LoadVoxelWorld, VoxelCameraBundle,
};

use crate::voxels::raycast;

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

    //bevy_mod_debugdump::print_render_graph(&mut app);

    app.run();
}

// just for prototype
fn voxel_break(
    camera_query: Query<(&Camera, &Transform)>,
    mut chunk_map: ResMut<ChunkMap>,
    mouse: Res<Input<MouseButton>>,
) {
    if let Ok((_cam, tr)) = camera_query.get_single() {
        for (pos, chunk) in chunk_map.chunks.iter_mut() {
            let mut gh = chunk.grid.0.write().unwrap();
            let s = gh.size as i32;
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
                    raycast::raycast(tr.translation - pos.as_vec3(), tr.forward(), &gh)
                {
                    if dist.is_finite() && gh.contains(&pos) {
                        match act {
                            Act::RemoveBlock => {
                                let i = (pos.z + pos.y * s + pos.x * s * s) * 4;
                                gh.voxels[i as usize] = 0;
                                gh.voxels[i as usize + 1] = 0;
                                chunk.version = chunk.version.wrapping_add(1);
                            }
                            Act::PlaceBlock => {
                                let pos = pos + norm;
                                if gh.contains(&pos) {
                                    let i = (pos.z + pos.y * s + pos.x * s * s) * 4;
                                    gh.voxels[i as usize] = 2;
                                    gh.voxels[i as usize + 1] = 16;
                                    chunk.version = chunk.version.wrapping_add(1);
                                }
                            }
                        };
                    }
                } else {
                    //dbg!("no hit");
                }
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

fn load(mut handles: ResMut<Handles>, asset_server: Res<AssetServer>) {
    handles.texture_blocks = asset_server.load("textures/blocks.png");
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
    let mut grid = if pos.y == 0 {
        Grid::flatland(32)
    } else if pos.y < 0 {
        Grid::filled(32)
    } else {
        Grid::empty(32)
    };
    GridPtr(Arc::new(RwLock::new(grid)))
}

fn load_and_gen_chunks(mut chunk_map: ResMut<ChunkMap>, camera: Query<(&Camera, &Transform)>) {
    let load_view_distance: u32 = 250;

    let camera_pos = if let Ok((_, tr)) = camera.get_single() {
        tr.translation
    } else {
        return;
    };

    let camera_chunk_pos = (camera_pos / 32.0).as_ivec3() * 32;

    // hardcoded chunk size
    let load_view_distance_chunk = load_view_distance as i32 / 32;
    let lvdc = load_view_distance_chunk;

    // sphere centered on the player
    for x in -lvdc..=lvdc {
        for y in -lvdc..=lvdc {
            for z in -lvdc..=lvdc {
                let rel = IVec3::new(x, y, z) * 32;
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

    /*
    println!(
        "{:?}",
        chunk_map.chunks.iter().map(|o| o.0).collect::<Vec<_>>()
    );
    */

    //dbg!(chunk_map.chunks.len());
}

fn setup(mut commands: Commands, mut chunk_map: ResMut<ChunkMap>) {
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

    // voxel world
    /*
    let size = 10;
    for x in -size..size {
        for y in -size..size {
            for z in -size..size {
                let pos = IVec3::new(x, y, z);
                chunk_map.chunks.insert(
                    // hardcoded chunk size
                    pos * 32,
                    Chunk {
                        grid: GridPtr(Arc::new(RwLock::new(Grid::filled(32, pos)))),
                        was_mutated: false,
                    },
                );
            }
        }
    }
    */

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
