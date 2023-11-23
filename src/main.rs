use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    pbr::wireframe::{Wireframe, WireframePlugin},
    prelude::*,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    utils::HashMap,
    window::{PresentMode, WindowPlugin},
};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_flycam::prelude::*;

mod instanced_material;
use block_mesh::{
    greedy_quads,
    ndshape::{ConstShape, ConstShape3u32},
    visible_block_faces,
};
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
            EguiPlugin,
            WireframePlugin,
        ))
        .insert_resource(ClearColor(Color::MIDNIGHT_BLUE))
        .add_state::<FlowState>()
        .add_systems(Startup, setup)
        .add_systems(Startup, start_benchmark)
        .add_systems(OnEnter(FlowState::Benchmark), setup_bench)
        .add_systems(OnExit(FlowState::Benchmark), teardown_bench)
        .add_systems(Update, print_mesh_count)
        .add_systems(Update, ui)
        .add_systems(Update, wireframe)
        .add_systems(OnEnter(FlowState::Transition), start_benchmark)
        .add_event::<ToggleWireframeEvent>()
        .run();
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum FlowState {
    #[default]
    Base,
    Benchmark,
    Transition,
}

fn ui(
    mut contexts: EguiContexts,
    mut method: ResMut<RenderMethod>,
    mut shape: ResMut<VoxelShape>,
    mut next_state: ResMut<NextState<FlowState>>,
    mut settings: ResMut<Settings>,
    mut toggle_wireframe: EventWriter<ToggleWireframeEvent>,
) {
    egui::SidePanel::new(egui::panel::Side::Right, "Benchmark").show(contexts.ctx_mut(), |ui| {
        ui.separator();
        ui.label("CHOOSE RENDER METHOD");
        for m in RenderMethod::opts() {
            let mut sel = m == *method;
            if ui.toggle_value(&mut sel, format!("{:?}", m)).clicked() {
                *method = m.clone();
                next_state.set(FlowState::Transition);
            }
        }
        ui.separator();
        ui.label("CHOOSE VOXEL SHAPE");
        for s in VoxelShape::opts() {
            let mut sel = s == *shape;
            if ui.toggle_value(&mut sel, format!("{:?}", s)).clicked() {
                *shape = s.clone();
                next_state.set(FlowState::Transition);
            }
        }
        ui.separator();
        ui.label("RENDER SETTINGS");
        if ui
            .toggle_value(&mut settings.wireframe, "wireframe")
            .changed()
        {
            toggle_wireframe.send(ToggleWireframeEvent {
                active: settings.wireframe,
            })
        }
        ui.separator();
        ui.label("COMMANDS");
        ui.label("Press esc to use the mouse");
        ui.label("WASD to move in xz plane");
        ui.label("EQ to move along y axis");
    });
}

#[derive(Resource)]
struct Settings {
    wireframe: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self { wireframe: false }
    }
}

#[derive(Event)]
struct ToggleWireframeEvent {
    active: bool,
}

fn wireframe(
    mut commands: Commands,
    query: Query<(Entity, &BenchedMesh)>,
    mut event: EventReader<ToggleWireframeEvent>,
) {
    for event in event.read() {
        if event.active {
            for (ent, _) in query.iter() {
                commands.entity(ent).insert(Wireframe);
            }
        } else {
            for (ent, _) in query.iter() {
                commands.entity(ent).remove::<Wireframe>();
            }
        }
    }
}

#[derive(Resource, Default)]
struct Handles {
    material: Handle<StandardMaterial>,
    cube: Handle<Mesh>,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(Settings::default());
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
    let mut handles = Handles::default();
    handles.material = material_assets.add(StandardMaterial {
        base_color: Color::rgba(0.5, 0.2, 0.0, 0.5),
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });
    handles.cube = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    commands.insert_resource(handles);
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
enum RenderMethod {
    Naive,
    Instanced,
    ChunkedBlockMesh { greedy: bool },
}
impl Default for RenderMethod {
    fn default() -> Self {
        Self::Naive
    }
}
impl RenderMethod {
    fn opts() -> impl IntoIterator<Item = Self> {
        [
            Self::Naive,
            Self::Instanced,
            Self::ChunkedBlockMesh { greedy: false },
            Self::ChunkedBlockMesh { greedy: true },
        ]
        .into_iter()
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
enum VoxelShape {
    FilledCuboid(IVec3),
    Sphere(u32),
    Random { size: IVec3, seed: u64 },
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
            Self::Sphere(8),
            Self::Sphere(32),
            Self::Sphere(128),
            Self::Random {
                size: IVec3::splat(256),
                seed: 69,
            },
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
            Self::Sphere(radius) => {
                let size = IVec3::splat(*radius as i32 * 2);
                let center = Vec3::splat(*radius as f32);
                let rad2 = *radius as f32 * *radius as f32;
                for x in 0..size.x {
                    for y in 0..size.y {
                        for z in 0..size.z {
                            if (Vec3::new(x as f32, y as f32, z as f32) - center).length_squared()
                                < rad2
                            {
                                vec.push(IVec3::new(x, y, z));
                            }
                        }
                    }
                }
            }
            Self::Random { size, seed } => {
                let mut rng = ChaCha8Rng::seed_from_u64(*seed);
                for x in 0..size.x {
                    for y in 0..size.y {
                        for z in 0..size.z {
                            if rng.gen_bool(0.5) {
                                vec.push(IVec3::new(x, y, z))
                            }
                        }
                    }
                }
            }
        }
        vec.into_iter()
    }
}

fn start_benchmark(mut next_state: ResMut<NextState<FlowState>>) {
    next_state.set(FlowState::Benchmark);
}

#[derive(Component)]
struct BenchedMesh;

fn setup_bench(
    mut commands: Commands,
    handles: Res<Handles>,
    mut meshes: ResMut<Assets<Mesh>>,
    shape: Res<VoxelShape>,
    method: Res<RenderMethod>,
    mut toggle_wireframe: EventWriter<ToggleWireframeEvent>,
    settings: Res<Settings>,
) {
    match *method {
        RenderMethod::Naive => naive(&mut commands, handles, shape),
        RenderMethod::Instanced => instanced(&mut commands, handles, shape),
        RenderMethod::ChunkedBlockMesh { greedy } => {
            chunked_block_mesh(&mut commands, handles, &mut meshes, shape, greedy)
        }
    }
    toggle_wireframe.send(ToggleWireframeEvent {
        active: settings.wireframe,
    });
}

fn naive(commands: &mut Commands, handles: Res<Handles>, shape: Res<VoxelShape>) {
    for pos in shape.iter() {
        commands.spawn((
            PbrBundle {
                mesh: handles.cube.clone(),
                material: handles.material.clone(),
                transform: Transform::from_translation(pos.as_vec3()),
                ..default()
            },
            BenchedMesh,
        ));
    }
}

fn instanced(commands: &mut Commands, handles: Res<Handles>, shape: Res<VoxelShape>) {
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
        handles.cube.clone(),
        SpatialBundle::INHERITED_IDENTITY,
        instances,
        // If the camera doesn't see (0, 0, 0) all instances would be called.
        bevy::render::view::NoFrustumCulling,
        BenchedMesh,
    ));
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct BoolVoxel(bool);

impl Default for BoolVoxel {
    fn default() -> Self {
        EMPTY
    }
}

const EMPTY: BoolVoxel = BoolVoxel(false);
const FILLED: BoolVoxel = BoolVoxel(true);

impl block_mesh::Voxel for BoolVoxel {
    fn get_visibility(&self) -> block_mesh::VoxelVisibility {
        if *self == EMPTY {
            block_mesh::VoxelVisibility::Empty
        } else {
            block_mesh::VoxelVisibility::Opaque
        }
    }
}

impl block_mesh::MergeVoxel for BoolVoxel {
    type MergeValue = Self;

    fn merge_value(&self) -> Self::MergeValue {
        *self
    }
}

fn chunked_block_mesh(
    commands: &mut Commands,
    handles: Res<Handles>,
    meshes: &mut ResMut<Assets<Mesh>>,
    shape: Res<VoxelShape>,
    greedy: bool,
) {
    const CHUNK_SIDE: u32 = 16;
    const CHUNK_SIDE_PADDED: u32 = CHUNK_SIDE + 2;
    const CHUNK_AREA: u32 = CHUNK_SIDE * CHUNK_SIDE;
    const CHUNK_VOLUME: u32 = CHUNK_SIDE * CHUNK_SIDE * CHUNK_SIDE;

    let mut chunks = HashMap::<IVec3, [BoolVoxel; CHUNK_VOLUME as usize]>::new();
    for pos in shape.iter() {
        let chunk_pos = pos / 16;
        let chunk = chunks
            .entry(chunk_pos)
            .or_insert([EMPTY; CHUNK_VOLUME as usize]);
        let chunk_offset = pos % 16;
        let i = chunk_offset.x
            + chunk_offset.y * CHUNK_SIDE as i32
            + chunk_offset.z * CHUNK_AREA as i32;
        chunk[i as usize] = FILLED;
    }

    type SampleShape = ConstShape3u32<CHUNK_SIDE_PADDED, CHUNK_SIDE_PADDED, CHUNK_SIDE_PADDED>;
    for (pos, chunk) in chunks.iter() {
        let mut voxels = [EMPTY; SampleShape::SIZE as usize];
        for (j, voxel) in chunk.iter().enumerate() {
            let mut j = j as u32;
            let z = j / CHUNK_AREA;
            j -= z * CHUNK_AREA;
            let y = j / CHUNK_SIDE;
            let x = j % CHUNK_SIDE;
            let voxel_pos = UVec3::new(x, y, z) + UVec3::splat(1);
            let i = SampleShape::linearize(voxel_pos.to_array());
            voxels[i as usize] = voxel.clone();
        }

        let faces = block_mesh::RIGHT_HANDED_Y_UP_CONFIG.faces;

        let render_mesh = if greedy {
            let mut buffer = block_mesh::GreedyQuadsBuffer::new(voxels.len());
            greedy_quads(
                &voxels,
                &SampleShape {},
                [0; 3],
                [CHUNK_SIDE_PADDED - 1; 3],
                &faces,
                &mut buffer,
            );
            let num_indices = buffer.quads.num_quads() * 6;
            let num_vertices = buffer.quads.num_quads() * 4;
            let mut indices = Vec::with_capacity(num_indices);
            let mut positions = Vec::with_capacity(num_vertices);
            let mut normals = Vec::with_capacity(num_vertices);
            for (group, face) in buffer.quads.groups.into_iter().zip(faces.into_iter()) {
                for quad in group.into_iter() {
                    indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                    positions.extend_from_slice(&face.quad_mesh_positions(&quad.into(), 1.0));
                    normals.extend_from_slice(&face.quad_mesh_normals());
                }
            }
            let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);
            render_mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                VertexAttributeValues::Float32x3(positions),
            );
            render_mesh.insert_attribute(
                Mesh::ATTRIBUTE_NORMAL,
                VertexAttributeValues::Float32x3(normals),
            );
            render_mesh.insert_attribute(
                Mesh::ATTRIBUTE_UV_0,
                VertexAttributeValues::Float32x2(vec![[0.0; 2]; num_vertices]),
            );
            render_mesh.set_indices(Some(Indices::U32(indices.clone())));
            render_mesh
        } else {
            let mut buffer = block_mesh::UnitQuadBuffer::new();
            visible_block_faces(
                &voxels,
                &SampleShape {},
                [0; 3],
                [CHUNK_SIDE_PADDED - 1; 3],
                &faces,
                &mut buffer,
            );
            let num_indices = buffer.num_quads() * 6;
            let num_vertices = buffer.num_quads() * 4;
            let mut indices = Vec::with_capacity(num_indices);
            let mut positions = Vec::with_capacity(num_vertices);
            let mut normals = Vec::with_capacity(num_vertices);
            for (group, face) in buffer.groups.into_iter().zip(faces.into_iter()) {
                for quad in group.into_iter() {
                    indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                    positions.extend_from_slice(&face.quad_mesh_positions(&quad.into(), 1.0));
                    normals.extend_from_slice(&face.quad_mesh_normals());
                }
            }
            let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);
            render_mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                VertexAttributeValues::Float32x3(positions),
            );
            render_mesh.insert_attribute(
                Mesh::ATTRIBUTE_NORMAL,
                VertexAttributeValues::Float32x3(normals),
            );
            render_mesh.insert_attribute(
                Mesh::ATTRIBUTE_UV_0,
                VertexAttributeValues::Float32x2(vec![[0.0; 2]; num_vertices]),
            );
            render_mesh.set_indices(Some(Indices::U32(indices.clone())));
            render_mesh
        };

        // i do not understand why Vec3::splat(1.5) is needed
        let world_pos = pos.as_vec3() * Vec3::splat(CHUNK_SIDE as f32) - Vec3::splat(1.5);
        let mesh_handle = meshes.add(render_mesh);
        commands.spawn((
            PbrBundle {
                mesh: mesh_handle,
                material: handles.material.clone(),
                transform: Transform::from_translation(world_pos),
                ..default()
            },
            BenchedMesh,
        ));
    }
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
