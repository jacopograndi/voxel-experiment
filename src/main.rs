use bevy::{
    app::AppExit,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

use bevy_flycam::prelude::*;

mod instanced_material;
use instanced_material::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
            NoCameraPlayerPlugin,
            InstancedMaterialPlugin,
        ))
        .insert_resource(ClearColor(Color::MIDNIGHT_BLUE))
        .add_state::<FlowState>()
        .add_systems(Startup, setup)
        .add_systems(Startup, start_benchmark)
        .add_systems(OnEnter(FlowState::Benchmark), setup_bench)
        .add_systems(OnExit(FlowState::Benchmark), teardown_bench)
        .add_systems(Update, exit)
        .add_systems(Update, print_mesh_count)
        .add_systems(Update, (method_selector, voxels_selector))
        .add_systems(OnEnter(FlowState::Transition), start_benchmark)
        .run();
}

fn exit(keys: Res<Input<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if keys.pressed(KeyCode::Escape) {
        exit.send(AppExit);
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum FlowState {
    #[default]
    Base,
    Benchmark,
    Transition,
}

fn setup(mut commands: Commands) {
    commands.insert_resource(VoxelShape::default());
    commands.insert_resource(RenderMethod::default());
    commands.spawn(DirectionalLightBundle { ..default() });
    commands.spawn((
        Camera3dBundle {
            transform: Transform {
                translation: Vec3::new(60.0, 16.0, 312.0),
                ..Default::default()
            },
            ..default()
        },
        FlyCam,
    ));
    commands.insert_resource(MovementSettings {
        sensitivity: 0.00015,
        speed: 30.0,
    });
    commands.insert_resource(KeyBindings {
        move_ascend: KeyCode::E,
        move_descend: KeyCode::Q,
        ..Default::default()
    });
}

#[derive(Resource)]
enum RenderMethod {
    Naive,
    Instanced,
}
impl Default for RenderMethod {
    fn default() -> Self {
        Self::Naive
    }
}
impl RenderMethod {
    fn opts() -> impl IntoIterator<Item = Self> {
        [Self::Naive, Self::Instanced].into_iter()
    }
}

#[derive(Resource)]
enum VoxelShape {
    FilledCuboid(IVec3),
}
impl Default for VoxelShape {
    fn default() -> Self {
        Self::opts().into_iter().next().unwrap()
    }
}
impl VoxelShape {
    fn opts() -> impl IntoIterator<Item = Self> {
        [
            Self::FilledCuboid(IVec3::splat(16)),
            Self::FilledCuboid(IVec3::splat(32)),
            Self::FilledCuboid(IVec3::splat(64)),
            Self::FilledCuboid(IVec3::splat(128)),
            Self::FilledCuboid(IVec3::splat(256)),
        ]
        .into_iter()
    }
    fn iter(&self) -> impl Iterator<Item = IVec3> {
        let mut vec: Vec<IVec3> = vec![];
        match self {
            Self::FilledCuboid(size) => {
                for x in 0..size.x {
                    for y in 0..size.y {
                        for z in 0..size.z {
                            vec.push(IVec3::new(x, y, z));
                        }
                    }
                }
            }
        }
        vec.into_iter()
    }
}

const SELECTOR_KEYS: [KeyCode; 9] = [
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::Key4,
    KeyCode::Key5,
    KeyCode::Key6,
    KeyCode::Key7,
    KeyCode::Key8,
    KeyCode::Key9,
];
fn method_selector(
    keys: Res<Input<KeyCode>>,
    mut method: ResMut<RenderMethod>,
    mut next_state: ResMut<NextState<FlowState>>,
) {
    if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
        return;
    }
    for (key, m) in SELECTOR_KEYS.iter().zip(RenderMethod::opts()) {
        if keys.just_pressed(*key) {
            *method = m;
            next_state.set(FlowState::Transition);
            return;
        }
    }
}

fn voxels_selector(
    keys: Res<Input<KeyCode>>,
    mut voxels: ResMut<VoxelShape>,
    mut next_state: ResMut<NextState<FlowState>>,
) {
    if !(keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)) {
        return;
    }

    for (key, v) in SELECTOR_KEYS.iter().zip(VoxelShape::opts()) {
        if keys.just_pressed(*key) {
            *voxels = v;
            next_state.set(FlowState::Transition);
            return;
        }
    }
}

fn start_benchmark(mut next_state: ResMut<NextState<FlowState>>) {
    next_state.set(FlowState::Benchmark);
}

#[derive(Component)]
struct BenchedMesh;

fn setup_bench(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    material_assets: ResMut<Assets<StandardMaterial>>,
    shape: Res<VoxelShape>,
    method: Res<RenderMethod>,
) {
    match *method {
        RenderMethod::Naive => naive(&mut commands, &mut meshes, material_assets, shape),
        RenderMethod::Instanced => instanced(&mut commands, &mut meshes, shape),
    }
}

fn naive(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material_assets: ResMut<Assets<StandardMaterial>>,
    shape: Res<VoxelShape>,
) {
    let mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let material_assets = material_assets.into_inner();
    let material = material_assets.add(StandardMaterial {
        base_color: Color::WHITE,
        ..default()
    });
    for pos in shape.iter() {
        commands.spawn((
            PbrBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform: Transform::from_translation(pos.as_vec3()),
                ..default()
            },
            BenchedMesh,
        ));
    }
}

fn instanced(commands: &mut Commands, meshes: &mut ResMut<Assets<Mesh>>, shape: Res<VoxelShape>) {
    let vec: Vec<InstanceData> = shape
        .iter()
        .map(|pos| InstanceData {
            position: pos.as_vec3() * Vec3::new(1., 1., 1.),
            scale: 1.0,
            color: Color::hsla(1.0, 0.0, 0.0, 1.0).as_rgba_f32(),
        })
        .collect();
    let instances = InstanceMaterialData(vec);
    commands.spawn((
        meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        SpatialBundle::INHERITED_IDENTITY,
        instances,
        // If the camera doesn't see (0, 0, 0) all instances would be called.
        bevy::render::view::NoFrustumCulling,
        BenchedMesh,
    ));
}

fn teardown_bench(mut commands: Commands, query: Query<(Entity, &BenchedMesh)>) {
    for (ent, _) in query.iter() {
        commands.entity(ent).despawn_recursive()
    }
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
