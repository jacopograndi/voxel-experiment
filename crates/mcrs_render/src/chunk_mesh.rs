use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
    utils::{HashMap, HashSet},
};
use block_mesh::{
    ndshape::{ConstShape, ConstShape3u32},
    visible_block_faces, MergeVoxel, UnitQuadBuffer, Voxel, VoxelVisibility,
    RIGHT_HANDED_Y_UP_CONFIG,
};
use mcrs_universe::{
    block::BlockFlag,
    chunk::{Chunk, ChunkVersion},
    universe::Universe,
    Blueprints, CHUNK_SIDE, MAX_LIGHT,
};

#[derive(Resource, Default)]
pub struct ChunkEntities {
    pub map: HashMap<IVec3, ChunkEntity>,
    pub to_update: HashSet<IVec3>,
}

#[derive(Component)]
pub struct ChunkEntity {
    pub entity: Entity,
    pub version: ChunkVersion,
}
impl ChunkEntity {
    fn new(entity: Entity, version: ChunkVersion) -> Self {
        Self { entity, version }
    }
}

const MAX_CHUNK_REMESH_PER_FRAME: u32 = 10;

#[derive(Resource, Default)]
pub struct TextureHandles {
    pub blocks: Handle<Image>,
}

pub fn load_texture(mut texture_handle: ResMut<TextureHandles>, asset_server: Res<AssetServer>) {
    texture_handle.blocks = asset_server.load("textures/blocks.png");
}

fn adjacent() -> impl Iterator<Item = IVec3> {
    [
        IVec3::new(1, 0, 0),
        IVec3::new(-1, 0, 0),
        IVec3::new(0, 1, 0),
        IVec3::new(0, -1, 0),
        IVec3::new(0, 0, 1),
        IVec3::new(0, 0, -1),
    ]
    .into_iter()
}

/// Creates a mesh for every chunk that is in universe
/// When a chunk is modified, a new mesh is created
pub fn sync_chunk_meshes(
    mut commands: Commands,
    mut chunk_entities: ResMut<ChunkEntities>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    universe: Res<Universe>,
    bp: Res<Blueprints>,
    handles: Res<TextureHandles>,
) {
    let mut remeshed_chunks = 0;

    // For each chunk that is in universe, check that it is instanced
    for (chunk_pos, chunk) in universe.chunks.iter() {
        if let Some(chunk_entity) = chunk_entities.map.get(chunk_pos) {
            if chunk_entity.version != chunk.version {
                info!(
                    "despawned chunk mesh at {}, obsolete (mesh: {:?}, chunk: {:?})",
                    chunk_pos, chunk_entity.version, chunk.version
                );
                commands.entity(chunk_entity.entity).despawn_recursive();
            } else {
                continue;
            }
        }

        if adjacent().any(|dir| {
            universe
                .chunks
                .get(&(chunk_pos + dir * CHUNK_SIDE as i32))
                .is_none()
        }) {
            continue;
        }

        let entity = create_chunk_entity(
            chunk_pos,
            &mut commands,
            &mut materials,
            &mut meshes,
            &universe,
            &bp,
            &handles,
        );

        chunk_entities.map.insert(
            chunk_pos.clone(),
            ChunkEntity::new(entity, chunk.version.clone()),
        );

        for dir in adjacent() {
            chunk_entities
                .to_update
                .insert(chunk_pos + dir * CHUNK_SIDE as i32);
        }

        remeshed_chunks += 1;
        if remeshed_chunks >= MAX_CHUNK_REMESH_PER_FRAME {
            break;
        }
    }

    // For each chunk that is instanced and not in universe, despawn
    for (chunk_pos, chunk_entity) in chunk_entities.map.iter() {
        if let None = universe.chunks.get(chunk_pos) {
            info!("despawned chunk mesh at {}", chunk_pos);
            commands.entity(chunk_entity.entity).despawn_recursive();
        }
    }
}

pub const UV_SCALE: f32 = 1.0 / 16.0;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum GridVoxel {
    Empty,
    Full,
}
impl Voxel for GridVoxel {
    fn get_visibility(&self) -> VoxelVisibility {
        match self {
            GridVoxel::Empty => VoxelVisibility::Empty,
            GridVoxel::Full => VoxelVisibility::Opaque,
        }
    }
}

impl MergeVoxel for GridVoxel {
    type MergeValue = Self;

    fn merge_value(&self) -> Self::MergeValue {
        *self
    }
}

pub struct ChunkMesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
}

pub fn create_chunk_entity(
    chunk_pos: &IVec3,
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    universe: &Universe,
    bp: &Blueprints,
    handles: &TextureHandles,
) -> Entity {
    let mut entity_commands = commands.spawn((Transform::from_translation(chunk_pos.as_vec3()),));
    if let Some(mut raw_mesh) = generate_chunk_mesh(chunk_pos, &bp, &universe) {
        info!("spawned chunk mesh at {}", chunk_pos);
        let mut indices = Vec::with_capacity(raw_mesh.indices.len() / 3);
        for i in 0..raw_mesh.indices.len() / 3 {
            indices.push([
                raw_mesh.indices[i * 3],
                raw_mesh.indices[i * 3 + 1],
                raw_mesh.indices[i * 3 + 2],
            ])
        }
        let mut render_mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        for uv in raw_mesh.uvs.iter_mut() {
            for c in uv.iter_mut() {
                *c *= UV_SCALE;
            }
        }
        render_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, raw_mesh.vertices);
        render_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, raw_mesh.normals);
        render_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, raw_mesh.uvs);
        render_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, raw_mesh.colors);
        render_mesh.insert_indices(Indices::U32(raw_mesh.indices));
        entity_commands.insert((
            Mesh3d(meshes.add(render_mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                perceptual_roughness: 0.9,
                base_color_texture: Some(handles.blocks.clone_weak()),
                ..default()
            })),
        ));
    } else {
        info!("spawned empty chunk at {}", chunk_pos);
    }
    entity_commands.id()
}

pub fn generate_chunk_mesh(
    chunk_pos: &IVec3,
    bp: &Blueprints,
    universe: &Universe,
) -> Option<ChunkMesh> {
    type SampleShape = ConstShape3u32<34, 34, 34>;

    let mut empty = true;

    let Some(chunk) = universe.chunks.get(chunk_pos) else {
        return None;
    };
    let chunk_ref = chunk.get_ref();
    let mut samples = [GridVoxel::Empty; SampleShape::SIZE as usize];
    for i in 0u32..(SampleShape::SIZE) {
        let [x, y, z] = SampleShape::delinearize(i);
        // apply a boundary of one block around the chunk
        let pos = IVec3::new(x as i32, y as i32, z as i32) - IVec3::splat(1);
        if Chunk::contains(pos) {
            let block = chunk_ref[Chunk::xyz2idx(pos) as usize];
            if block.properties.check(BlockFlag::Opaque) {
                empty = false;
                samples[i as usize] = GridVoxel::Full;
            }
        } else {
            // always generate the faces at the boundary
            samples[i as usize] = GridVoxel::Empty;
        };
    }
    if empty {
        return None;
    }

    let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

    let mut buffer = UnitQuadBuffer::new();
    visible_block_faces(
        &samples,
        &SampleShape {},
        [0; 3],
        [33; 3],
        &faces,
        &mut buffer,
    );
    let num_indices = buffer.num_quads() * 6;
    let num_vertices = buffer.num_quads() * 4;
    let mut indices = Vec::with_capacity(num_indices);
    let mut vertices = Vec::with_capacity(num_vertices);
    let mut normals = Vec::with_capacity(num_vertices);
    let mut uvs = Vec::with_capacity(num_vertices);
    let mut colors = Vec::with_capacity(num_vertices);
    for (group, face) in buffer.groups.into_iter().zip(faces.into_iter()) {
        for quad in group.into_iter() {
            let mut face_vertices = face.quad_mesh_positions(&quad.into(), 1.0);

            let mut face_xyz = Vec3::ZERO;
            // remove the boundary
            for vertex in face_vertices.iter_mut() {
                vertex[0] -= 1.0;
                vertex[1] -= 1.0;
                vertex[2] -= 1.0;
                face_xyz += Vec3::from_array(*vertex);
            }
            face_xyz /= 4.0;
            indices.extend_from_slice(&face.quad_mesh_indices(vertices.len() as u32));
            normals.extend_from_slice(&face.quad_mesh_normals());
            vertices.extend_from_slice(&face_vertices);

            // remove the boundary
            let block_xyz = IVec3::from_array([
                quad.minimum[0] as i32 - 1,
                quad.minimum[1] as i32 - 1,
                quad.minimum[2] as i32 - 1,
            ]);
            let block = chunk_ref[Chunk::xyz2idx(block_xyz)];
            let block_bp = bp.blocks.get(&block.id);
            let offset = [
                (block_bp.block_texture_offset[0]) as f32,
                (block_bp.block_texture_offset[1]) as f32,
            ];
            let uv_face = face.tex_coords(RIGHT_HANDED_Y_UP_CONFIG.u_flip_face, true, &quad.into());
            uvs.extend_from_slice(
                &uv_face
                    .iter()
                    .map(|uv| [uv[0] + offset[0], uv[1] + offset[1]])
                    .collect::<Vec<[f32; 2]>>(),
            );

            let block_normal = IVec3::from_array(face.signed_normal().to_array());
            let looking_at = block_xyz + block_normal;
            let light = if let Some(block) = universe.read_chunk_block(&(looking_at + chunk_pos)) {
                block.light0.max(block.light1) as f32 / MAX_LIGHT as f32
            } else {
                0.0
            };

            // vertex ambient occlusion
            let mut face_colors = vec![];
            for vertex in face_vertices {
                let xyz = Vec3::from_array(vertex);
                let normal = block_normal.as_vec3();
                let dir = (xyz - face_xyz).normalize();
                let ortho = dir.cross(normal);
                let mut ao = 0.0;

                let diagonal = (xyz + normal * 0.5 - ortho).floor().as_ivec3();
                if universe
                    .read_chunk_block(&(chunk_pos + diagonal))
                    .map_or(false, |b| b.properties.check(BlockFlag::Opaque))
                {
                    ao += 0.3;
                }

                let diagonal = (xyz + normal * 0.5 + ortho).floor().as_ivec3();
                if universe
                    .read_chunk_block(&(chunk_pos + diagonal))
                    .map_or(false, |b| b.properties.check(BlockFlag::Opaque))
                {
                    ao += 0.3;
                }

                let diagonal = (xyz + normal * 0.5 + dir).floor().as_ivec3();
                if universe
                    .read_chunk_block(&(chunk_pos + diagonal))
                    .map_or(false, |b| b.properties.check(BlockFlag::Opaque))
                {
                    if ao == 0.0 {
                        ao += 0.3;
                    }
                }

                let c = (light - ao).clamp(0.02, 1.0);
                face_colors.push([c, c, c, 1.0]);
            }
            colors.extend_from_slice(&face_colors.as_slice());
        }
    }

    assert_eq!(vertices.len(), colors.len());
    assert_eq!(vertices.len(), normals.len());
    assert_eq!(vertices.len(), uvs.len());

    if vertices.len() == 0 {
        return None;
    }

    Some(ChunkMesh {
        indices,
        vertices,
        normals,
        uvs,
        colors,
    })
}
