use bevy::{
    ecs::entity::EntityHashMap,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntry, BindingType,
            Buffer, BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages,
            ShaderStages,
        },
        renderer::{RenderDevice, RenderQueue},
        view::ExtractedView,
        Extract, Render, RenderApp, RenderSet,
    },
    utils::HashMap,
};
use std::{
    fs,
    sync::{Arc, RwLock},
};
use mcrs_universe::{block::BlockId, ghost::GhostId, Blueprints};
use crate::{block_texture::BlockTexture, settings::RenderSettings, voxel_world::VoxelUniforms};

const MAX_BOXES: usize = 10000;
const MAX_VOX_TEXTURE_BYTES: usize = 100000000;

pub struct BoxesWorldPlugin;

impl Plugin for BoxesWorldPlugin {
    fn build(&self, _app: &mut App) {}
    fn finish(&self, app: &mut App) {
        let render_app = app.get_sub_app(RenderApp).unwrap();
        let render_device = render_app.world().resource::<RenderDevice>();

        let boxes_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: &vec![0; std::mem::size_of::<PodTexturedBox>() * MAX_BOXES],
            label: Some("boxes_storage"),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let vox_texture_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: &vec![0; MAX_VOX_TEXTURE_BYTES],
            label: Some("vox_texture_storage"),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let bind_group_layout = render_device.create_bind_group_layout(
            "boxes bind group layout",
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(0),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(0),
                    },
                    count: None,
                },
            ],
        );

        let bind_group = render_device.create_bind_group(
            None,
            &bind_group_layout,
            &BindGroupEntries::sequential((
                boxes_buffer.as_entire_binding(),
                vox_texture_buffer.as_entire_binding(),
            )),
        );

        app.insert_resource(LoadedVoxTextures::default())
            .insert_resource(VoxTextureLoadQueue::default())
            .add_plugins(ExtractResourcePlugin::<LoadedVoxTextures>::default())
            .add_systems(Startup, read_info_textures)
            .add_systems(Update, load_vox_textures);

        app.sub_app_mut(RenderApp)
            .insert_resource(ExtractedTexturedBoxes::default())
            .insert_resource(BoxesData {
                bind_group_layout,
                bind_group,
                boxes_buffer,
                vox_texture_buffer,
            })
            .add_systems(ExtractSchedule, extract_boxes)
            .add_systems(
                Render,
                (write_boxes, write_vox_textures, bind_boxes_data).in_set(RenderSet::Prepare),
            );
    }
}

fn read_info_textures(
    mut queue: ResMut<VoxTextureLoadQueue>,
    mut loaded: ResMut<LoadedVoxTextures>,
    info: Res<Blueprints>,
) {
    let mut max_id = 0;
    for block_info in info.blocks.iter() {
        let vox_texture_index = VoxTextureIndex(*block_info.id as u32);
        queue.to_load.push((
            block_info.voxel_texture_path.clone(),
            vox_texture_index.clone(),
        ));
        loaded.blocks_id.insert(block_info.id, vox_texture_index);
        max_id = max_id.max(*block_info.id as u32);
    }
    let mut serial_id = max_id + 1;
    for ghost_info in info.ghosts.iter() {
        let vox_texture_index = VoxTextureIndex(serial_id);
        queue.to_load.push((
            ghost_info.voxel_texture_path.clone(),
            vox_texture_index.clone(),
        ));
        loaded.ghosts_id.insert(ghost_info.id, vox_texture_index);
        serial_id += 1;
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct Ghost {
    pub vox_texture_index: VoxTextureIndex,
}

#[derive(Default, Debug, Clone)]
pub struct ExtractedTexturedBox {
    pub transform: GlobalTransform,
    pub size: Vec3,
    pub index: VoxTextureIndex,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PodTexturedBox {
    pub world_to_box: Mat4,
    pub box_to_world: Mat4,
    pub index: u32,
    pub _padding0: u32,
    pub _padding1: u32,
    pub _padding2: u32,
}

#[derive(Resource, Default)]
pub struct ExtractedTexturedBoxes {
    pub boxes: EntityHashMap<ExtractedTexturedBox>,
}

#[derive(Debug, Clone)]
pub struct BlockTexturePtr(pub Arc<RwLock<BlockTexture>>);

#[derive(Debug, Clone, Deref, DerefMut, Default, Hash, PartialEq, Eq)]
pub struct VoxTextureIndex(pub u32);

#[derive(Resource, Default, Clone, ExtractResource)]
pub struct LoadedVoxTextures {
    pub indices: HashMap<String, VoxTextureIndex>,
    pub textures: HashMap<VoxTextureIndex, BlockTexturePtr>,
    pub blocks_id: HashMap<BlockId, VoxTextureIndex>,
    pub ghosts_id: HashMap<GhostId, VoxTextureIndex>,
}

#[derive(Resource, Default)]
pub struct VoxTextureLoadQueue {
    pub to_load: Vec<(String, VoxTextureIndex)>,
}

fn load_vox_textures(
    mut loaded: ResMut<LoadedVoxTextures>,
    mut queue: ResMut<VoxTextureLoadQueue>,
) {
    for (path, id) in queue.to_load.iter() {
        let result = fs::read(path);
        if let Ok(slice) = result {
            let result = BlockTexture::from_vox(&slice);
            if let Ok(vox) = result {
                let grid = BlockTexturePtr(Arc::new(RwLock::new(vox)));
                loaded.indices.insert(path.clone(), id.clone());
                loaded.textures.insert(id.clone(), grid);
            } else {
                println!("{path} {:?}", result);
            }
        } else {
            println!("{path} {:?}", result);
        }
    }
    queue.to_load.clear()
}

fn extract_boxes(
    box_query: Extract<Query<(Entity, &GlobalTransform, &Ghost, &ViewVisibility)>>,
    mut extracted_boxes: ResMut<ExtractedTexturedBoxes>,
) {
    extracted_boxes.boxes.clear();
    for (entity, global_tr, ghost, view_visibility) in box_query.iter() {
        if !view_visibility.get() {
            continue;
        }
        extracted_boxes.boxes.insert(
            entity,
            ExtractedTexturedBox {
                transform: *global_tr,
                size: Vec3::ONE,
                index: ghost.vox_texture_index.clone(),
            },
        );
    }
}

#[derive(Resource)]
pub struct BoxesData {
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
    pub boxes_buffer: Buffer,
    pub vox_texture_buffer: Buffer,
}

fn write_boxes(
    voxel_uniforms: Res<VoxelUniforms>,
    extracted_boxes: Res<ExtractedTexturedBoxes>,
    view_query: Query<(&ExtractedView, &ExtractedCamera)>,
    render_queue: Res<RenderQueue>,
    boxes_data: Res<BoxesData>,
    settings: Res<RenderSettings>,
) {
    let Ok((view, ..)) = view_query.get_single() else {
        return;
    };
    let cam_pos = view.world_from_view.translation();

    let chunk_side = voxel_uniforms.chunk_size;
    let camera_chunk_pos = (cam_pos / chunk_side as f32) * chunk_side as f32;

    let visible_boxes: Vec<ExtractedTexturedBox> = extracted_boxes
        .boxes
        .iter()
        .filter_map(|(_ent, texbox)| {
            let pos = texbox.transform.translation();
            if (camera_chunk_pos - pos).length_squared()
                < settings.view_distance_blocks.pow(2) as f32
            {
                Some(texbox.clone())
            } else {
                None
            }
        })
        .collect();

    let boxes: Vec<PodTexturedBox> = visible_boxes
        .iter()
        .map(|texbox| {
            let box_to_world = texbox.transform.compute_matrix();
            let world_to_box = box_to_world.inverse();
            PodTexturedBox {
                world_to_box,
                box_to_world,
                index: texbox.index.0,
                _padding0: 0,
                _padding1: 0,
                _padding2: 0,
            }
        })
        .collect();

    let slice: &[u8] = bytemuck::cast_slice(&boxes);
    let len = boxes.len() as u32;
    let bytes: Vec<u8> = [
        len.to_le_bytes().as_slice(),
        // duplicated for alignment to 16 bytes
        len.to_le_bytes().as_slice(),
        len.to_le_bytes().as_slice(),
        len.to_le_bytes().as_slice(),
        slice,
    ]
    .concat();

    render_queue.write_buffer(&boxes_data.boxes_buffer, 0, &bytes);
}

const MAX_VOX_TEXTURE_STORAGE: u32 = 1024;

fn write_vox_textures(
    render_queue: Res<RenderQueue>,
    boxes_data: Res<BoxesData>,
    loaded: ResMut<LoadedVoxTextures>,
) {
    let mut bytes: Vec<u8> = vec![];
    let offsets = [0; (MAX_VOX_TEXTURE_STORAGE * 4) as usize];
    bytes.extend(offsets);

    let mut textures: Vec<(VoxTextureIndex, BlockTexturePtr)> = loaded
        .textures
        .iter()
        .map(|p| (p.0.clone(), p.1.clone()))
        .collect();
    textures.sort_by(|a, b| a.0.cmp(&b.0));

    let mut texture_offset: u32 = 0;
    for (i, (_id, vox_texture)) in textures.iter().enumerate() {
        let vox = vox_texture.0.read().unwrap().to_bytes_vec();
        let len = vox.len();
        bytes.extend(vox);
        let offset_bytes = texture_offset.to_le_bytes();
        for j in 0..4 {
            bytes[i * 4 + j] = offset_bytes[j];
        }
        texture_offset += len as u32 / 4;
    }
    render_queue.write_buffer(&boxes_data.vox_texture_buffer, 0, &bytes);
}

fn bind_boxes_data(render_device: Res<RenderDevice>, mut boxes_data: ResMut<BoxesData>) {
    let bind_group = render_device.create_bind_group(
        None,
        &boxes_data.bind_group_layout,
        &BindGroupEntries::sequential((
            boxes_data.boxes_buffer.as_entire_binding(),
            boxes_data.vox_texture_buffer.as_entire_binding(),
        )),
    );
    boxes_data.bind_group = bind_group;
}
