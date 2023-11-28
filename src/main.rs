use bevy::{
    asset::LoadState,
    core_pipeline::fxaa::Fxaa,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

use bevy_flycam::prelude::*;

mod voxels;
use voxels::{
    voxel_world::ArcGridHierarchy, BevyVoxelEnginePlugin, LoadVoxelWorld, VoxelCameraBundle,
};

mod voxel_shapes;
use voxel_shapes::*;

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
    .add_systems(Update, voxel_break);

    bevy_mod_debugdump::print_render_graph(&mut app);

    app.run();
}

// just for prototype
fn voxel_break(
    camera_query: Query<(&Camera, &Transform)>,
    mut new_gh: ResMut<ArcGridHierarchy>,
    mouse: Res<Input<MouseButton>>,
) {
    if let Ok((_cam, tr)) = camera_query.get_single() {
        if let ArcGridHierarchy::Some(newgh) = new_gh.as_mut().clone() {
            let mut gh = newgh.lock().unwrap();
            //j todo bug
            let s = gh.texture_size as i32;
            let grid = Grid::from_vec(
                gh.raw
                    .iter()
                    .map(|pos| *pos - IVec3::splat(s / 2))
                    .collect(),
            );
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
                    raycast::raycast(tr.translation * 4., tr.forward(), &grid)
                {
                    let pos = pos + IVec3::splat(s / 2);
                    if dist.is_finite() && gh.raw.contains(&pos) {
                        match act {
                            Act::RemoveBlock => {
                                let i = (pos.z + pos.y * s + pos.x * s * s) * 2;
                                gh.texture_data[i as usize] = 0;
                                gh.raw.retain(|p| p != &pos);
                            }
                            Act::PlaceBlock => {
                                let pos = pos - norm;
                                let i = (pos.z + pos.y * s + pos.x * s * s) * 2;
                                gh.texture_data[i as usize] = 1;
                                gh.raw.push(pos);
                            }
                        };
                    }
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

fn setup(mut commands: Commands, mut load_voxel_world: ResMut<LoadVoxelWorld>) {
    commands.insert_resource(VoxelShape::default());

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
    *load_voxel_world = LoadVoxelWorld::File("assets/monu9.vox".to_string());

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
