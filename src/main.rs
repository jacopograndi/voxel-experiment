use bevy::{
    asset::LoadState,
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

use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_flycam::prelude::*;

use block_mesh::{
    greedy_quads,
    ndshape::{ConstShape, ConstShape3u32},
    visible_block_faces,
};

mod instanced_material;
use instanced_material::*;

mod raycast;
use raycast::*;

mod voxel_shapes;
use voxel_shapes::*;

fn main() {
    App::new()
        .add_plugins((
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
            InstancedMaterialPlugin,
            EguiPlugin,
            WireframePlugin,
        ))
        .insert_resource(ClearColor(Color::MIDNIGHT_BLUE))
        .insert_resource(Handles::default())
        .add_state::<FlowState>()
        .add_systems(Startup, load)
        .add_systems(Update, check_loading.run_if(in_state(FlowState::Loading)))
        .add_systems(OnEnter(FlowState::Base), (setup, start_benchmark))
        .add_systems(OnEnter(FlowState::Benchmark), setup_bench)
        .add_systems(OnExit(FlowState::Benchmark), teardown_bench)
        .add_systems(Update, print_mesh_count)
        .add_systems(Update, ui.run_if(in_state(FlowState::Benchmark)))
        .add_systems(Update, wireframe)
        .add_systems(OnEnter(FlowState::Transition), start_benchmark)
        .add_event::<ToggleWireframeEvent>()
        .run();
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum FlowState {
    #[default]
    Loading,
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut handles: ResMut<Handles>,
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
    handles.material = material_assets.add(StandardMaterial {
        unlit: true,
        //base_color: Color::rgba(0.5, 0.2, 0.0, 0.5),
        //alpha_mode: AlphaMode::Add,
        base_color_texture: Some(handles.texture_blocks.clone()),
        ..default()
    });
    let sp = shape::Box::new(1., 1., 1.);
    // suppose Y-up right hand, and camera look from +z to -z
    let vertices = &[
        // Front
        ([sp.min_x, sp.min_y, sp.max_z], [0., 0., 1.0], [0., 0.]),
        ([sp.max_x, sp.min_y, sp.max_z], [0., 0., 1.0], [1.0, 0.]),
        ([sp.max_x, sp.max_y, sp.max_z], [0., 0., 1.0], [1.0, 1.0]),
        ([sp.min_x, sp.max_y, sp.max_z], [0., 0., 1.0], [0., 1.0]),
        // Back
        ([sp.min_x, sp.max_y, sp.min_z], [0., 0., -1.0], [1.0, 0.]),
        ([sp.max_x, sp.max_y, sp.min_z], [0., 0., -1.0], [0., 0.]),
        ([sp.max_x, sp.min_y, sp.min_z], [0., 0., -1.0], [0., 1.0]),
        ([sp.min_x, sp.min_y, sp.min_z], [0., 0., -1.0], [1.0, 1.0]),
        // Right
        ([sp.max_x, sp.min_y, sp.min_z], [1.0, 0., 0.], [0., 0.]),
        ([sp.max_x, sp.max_y, sp.min_z], [1.0, 0., 0.], [1.0, 0.]),
        ([sp.max_x, sp.max_y, sp.max_z], [1.0, 0., 0.], [1.0, 1.0]),
        ([sp.max_x, sp.min_y, sp.max_z], [1.0, 0., 0.], [0., 1.0]),
        // Left
        ([sp.min_x, sp.min_y, sp.max_z], [-1.0, 0., 0.], [1.0, 0.]),
        ([sp.min_x, sp.max_y, sp.max_z], [-1.0, 0., 0.], [0., 0.]),
        ([sp.min_x, sp.max_y, sp.min_z], [-1.0, 0., 0.], [0., 1.0]),
        ([sp.min_x, sp.min_y, sp.min_z], [-1.0, 0., 0.], [1.0, 1.0]),
        // Top
        ([sp.max_x, sp.max_y, sp.min_z], [0., 1.0, 0.], [1.0, 0.]),
        ([sp.min_x, sp.max_y, sp.min_z], [0., 1.0, 0.], [0., 0.]),
        ([sp.min_x, sp.max_y, sp.max_z], [0., 1.0, 0.], [0., 1.0]),
        ([sp.max_x, sp.max_y, sp.max_z], [0., 1.0, 0.], [1.0, 1.0]),
        // Bottom
        ([sp.max_x, sp.min_y, sp.max_z], [0., -1.0, 0.], [0., 0.]),
        ([sp.min_x, sp.min_y, sp.max_z], [0., -1.0, 0.], [1.0, 0.]),
        ([sp.min_x, sp.min_y, sp.min_z], [0., -1.0, 0.], [1.0, 1.0]),
        ([sp.max_x, sp.min_y, sp.min_z], [0., -1.0, 0.], [0., 1.0]),
    ];

    let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
    let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
    let uvs: Vec<_> = vertices
        .iter()
        .map(|(_, _, uv)| [uv[0] / 16., uv[1] / 16.])
        .collect();

    let indices = Indices::U32(vec![
        0, 1, 2, 2, 3, 0, // front
        4, 5, 6, 6, 7, 4, // back
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // top
        20, 21, 22, 22, 23, 20, // bottom
    ]);

    let mesh = Mesh::new(PrimitiveTopology::TriangleList)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_indices(Some(indices));
    handles.cube = meshes.add(mesh);
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

fn chunked_block_mesh(
    commands: &mut Commands,
    handles: Res<Handles>,
    meshes: &mut ResMut<Assets<Mesh>>,
    shape: Res<VoxelShape>,
    greedy: bool,
) {
    let grid = Grid::from_vec(shape.iter().collect());

    type SampleShape = ConstShape3u32<CHUNK_SIDE_PADDED, CHUNK_SIDE_PADDED, CHUNK_SIDE_PADDED>;
    for (pos, chunk) in grid.chunks.iter() {
        let mut voxels = [EMPTY; SampleShape::SIZE as usize];
        for (j, voxel) in chunk.0.iter().enumerate() {
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
                VertexAttributeValues::Float32x2(vec![[0.0, 1.0]; num_vertices]),
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
