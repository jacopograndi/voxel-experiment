use bevy::render::camera::ExtractedCamera;
use bevy::render::render_asset::RenderAssets;
use bevy::render::view::ExtractedView;
use bevy::utils::{HashMap, HashSet};
use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        Render, RenderApp, RenderSet,
    },
};
use voxel_storage::chunk_map::{ChunkMap, GridPtr};
use voxel_storage::{CHUNK_SIDE, CHUNK_VOLUME};

pub const VIEW_DISTANCE: u32 = 220;

pub struct VoxelWorldPlugin;

impl Plugin for VoxelWorldPlugin {
    fn build(&self, _app: &mut App) {}
    fn finish(&self, app: &mut App) {
        let render_app = app.get_sub_app(RenderApp).unwrap();
        let render_device = render_app.world.resource::<RenderDevice>();
        let render_queue = render_app.world.resource::<RenderQueue>();

        let buffer_size = CHUNK_VOLUME * 4;
        let chunk_size = CHUNK_SIDE as u32;
        let chunks_grid = (VIEW_DISTANCE + CHUNK_SIDE as u32) * 2 / CHUNK_SIDE as u32;
        let chunks_volume = chunks_grid * chunks_grid * chunks_grid;

        // uniforms
        let voxel_uniforms = VoxelUniforms {
            chunk_size,
            offsets_grid_size: chunks_grid,
        };
        let mut uniform_buffer = UniformBuffer::from(voxel_uniforms.clone());
        uniform_buffer.write_buffer(render_device, render_queue);

        // storage
        let chunks = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: &vec![0; buffer_size as usize * chunks_volume as usize],
            label: Some("chunk_storage"),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let offsets_grid = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: &vec![0; chunks_volume as usize * 4],
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let chunks_loading = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: &vec![0; buffer_size as usize * chunks_volume as usize],
            label: Some("chunk_loading"),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let chunks_loading_offsets = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: &vec![0; (chunks_volume + 1) as usize * 4],
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("voxelization bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(VoxelUniforms::SHADER_SIZE.into()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(0),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(0),
                        },
                        count: None,
                    },
                ],
            });

        let bind_group = render_device.create_bind_group(
            None,
            &bind_group_layout,
            &BindGroupEntries::sequential((
                uniform_buffer.binding().unwrap(),
                chunks.as_entire_binding(),
                offsets_grid.as_entire_binding(),
            )),
        );

        let texture_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
                label: Some("tilemap_material_layout"),
            });

        let render_chunk_map = RenderChunkMap {
            buffer_alloc: ChunkAllocator::new(chunks_volume as usize),
            ..default()
        };

        app.insert_resource(ChunkMap::default())
            .insert_resource(ExtractedImage::default())
            .insert_resource(RenderHandles::default())
            .insert_resource(voxel_uniforms)
            .add_plugins(ExtractResourcePlugin::<ChunkMap>::default())
            .add_plugins(ExtractResourcePlugin::<VoxelUniforms>::default())
            .add_plugins(ExtractResourcePlugin::<ExtractedImage>::default())
            .add_systems(Update, extract_images);

        app.sub_app_mut(RenderApp)
            .insert_resource(VoxelData {
                uniform_buffer,
                chunks,
                chunks_pos: offsets_grid,
                chunks_loading,
                chunks_loading_offsets,
                bind_group_layout,
                bind_group,
                texture_bind_group_layout,
                texture_bind_group: None,
            })
            .insert_resource(render_chunk_map)
            .add_systems(
                Render,
                (
                    prepare_uniforms,
                    prepare_chunks,
                    write_chunks,
                    bind_voxel_data,
                    bind_images,
                )
                    .in_set(RenderSet::Prepare),
            );
    }
}

#[derive(Resource, ExtractResource, Clone, Default)]
pub struct ExtractedImage {
    handle: AssetId<Image>,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct RenderHandles {
    pub texture_blocks: Handle<Image>,
}

fn extract_images(mut extr: ResMut<ExtractedImage>, handles: Res<RenderHandles>) {
    extr.handle = handles.texture_blocks.id();
}

#[derive(Resource, ExtractResource, Clone, ShaderType)]
pub struct VoxelUniforms {
    pub offsets_grid_size: u32,
    pub chunk_size: u32,
}

#[derive(Resource)]
pub struct VoxelData {
    pub uniform_buffer: UniformBuffer<VoxelUniforms>,
    pub chunks: Buffer,
    pub chunks_pos: Buffer,
    pub chunks_loading: Buffer,
    pub chunks_loading_offsets: Buffer,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
    pub texture_bind_group_layout: BindGroupLayout,
    pub texture_bind_group: Option<BindGroup>,
}

#[derive(Resource, Clone, Default)]
pub struct RenderChunkMap {
    pub to_be_written: Vec<(u32, GridPtr)>,
    pub buffer_alloc: ChunkAllocator,
    pub versions: HashMap<IVec3, u32>,
}

#[derive(Clone, Default, Debug, Hash, PartialEq, Eq)]
struct BufferOffset(u32);

#[derive(Clone, Default)]
pub struct ChunkAllocator {
    allocated: HashMap<IVec3, BufferOffset>,
    free: Vec<BufferOffset>,
}

impl ChunkAllocator {
    fn new(size: usize) -> Self {
        Self {
            allocated: HashMap::new(),
            free: (0..size).map(|i| BufferOffset(i as u32)).collect(),
        }
    }

    fn iter(&self) -> impl Iterator<Item = &IVec3> {
        self.allocated.iter().map(|kv| kv.0)
    }

    fn get(&mut self, pos: &IVec3) -> Option<BufferOffset> {
        self.allocated.get(pos).cloned()
    }

    fn is_allocated(&self, pos: &IVec3) -> bool {
        self.allocated.get(pos).is_some()
    }

    fn allocate(&mut self, pos: IVec3) -> Option<BufferOffset> {
        if let Some(slot) = self.free.pop() {
            self.allocated.insert(pos, slot.clone());
            Some(slot)
        } else {
            None
        }
    }

    fn deallocate(&mut self, pos: IVec3) {
        if let Some(slot) = self.allocated.remove(&pos) {
            self.free.push(slot);
        }
    }
}

fn prepare_uniforms(
    voxel_uniforms: Res<VoxelUniforms>,
    mut voxel_data: ResMut<VoxelData>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    voxel_data.uniform_buffer.set(voxel_uniforms.clone());
    voxel_data
        .uniform_buffer
        .write_buffer(&render_device, &render_queue);
}

fn prepare_chunks(
    voxel_uniforms: Res<VoxelUniforms>,
    chunk_map: Res<ChunkMap>,
    mut render_chunk_map: ResMut<RenderChunkMap>,
    view_query: Query<(&ExtractedView, &ExtractedCamera)>,
) {
    let Ok((view, ..)) = view_query.get_single() else {
        return;
    };
    let cam_pos = view.transform.translation();

    let chunk_side = voxel_uniforms.chunk_size;
    let camera_chunk_pos = (cam_pos / chunk_side as f32) * chunk_side as f32;

    let visible_chunks: HashSet<IVec3> = chunk_map
        .chunks
        .iter()
        .filter_map(|(pos, _chunk)| {
            if (camera_chunk_pos - pos.as_vec3()).length_squared() < VIEW_DISTANCE.pow(2) as f32 {
                Some(*pos)
            } else {
                None
            }
        })
        .collect();

    let to_be_removed: HashSet<IVec3> = render_chunk_map
        .buffer_alloc
        .iter()
        .filter_map(|pos| (!visible_chunks.contains(pos)).then(|| *pos))
        .collect();

    for &pos in to_be_removed.iter() {
        render_chunk_map.buffer_alloc.deallocate(pos);
        render_chunk_map.versions.remove(&pos);
    }

    let to_be_rendered: HashSet<IVec3> = chunk_map
        .chunks
        .iter()
        .filter_map(|(pos, chunk)| {
            if visible_chunks.contains(pos) {
                if let Some(version) = render_chunk_map.versions.get(pos) {
                    if version != &chunk.version {
                        Some(*pos)
                    } else {
                        None
                    }
                } else {
                    if !render_chunk_map.buffer_alloc.is_allocated(pos) {
                        Some(*pos)
                    } else {
                        None
                    }
                }
            } else {
                None
            }
        })
        .collect();

    for &pos in to_be_rendered.iter() {
        let chunk = chunk_map.chunks.get(&pos).unwrap();
        let grid = chunk.grid.clone();
        render_chunk_map.versions.insert(pos, chunk.version);
        if let Some(BufferOffset(offset)) = render_chunk_map.buffer_alloc.get(&pos) {
            render_chunk_map.to_be_written.push((offset, grid));
        } else {
            if let Some(BufferOffset(offset)) = render_chunk_map.buffer_alloc.allocate(pos) {
                render_chunk_map.to_be_written.push((offset, grid));
            } else {
                panic!();
            }
        }
    }
}

fn write_chunks(
    voxel_uniforms: Res<VoxelUniforms>,
    voxel_data: Res<VoxelData>,
    render_queue: Res<RenderQueue>,
    mut render_chunk_map: ResMut<RenderChunkMap>,
    view_query: Query<(&ExtractedView, &ExtractedCamera)>,
) {
    let Ok((view, ..)) = view_query.get_single() else {
        return;
    };
    let cam_pos = view.transform.translation();

    let chunk_side = voxel_uniforms.chunk_size;
    let chunk_volume = chunk_side * chunk_side * chunk_side;
    let outer = voxel_uniforms.offsets_grid_size;

    let camera_chunk_pos = (cam_pos / chunk_side as f32).as_ivec3() * chunk_side as i32;
    let center = IVec3::splat(outer as i32) / 2 * chunk_side as i32;

    let mut chunks_pos: Vec<u32> = vec![];
    for x in 0..outer {
        for y in 0..outer {
            for z in 0..outer {
                let mut pos = IVec3::new(x as i32, y as i32, z as i32) * chunk_side as i32;
                pos -= center;
                pos += camera_chunk_pos;
                if let Some(BufferOffset(offset)) = render_chunk_map.buffer_alloc.get(&pos) {
                    chunks_pos.push(offset * chunk_volume);
                } else {
                    chunks_pos.push(u32::MAX);
                }
            }
        }
    }
    let chunks_pos: Vec<u8> = chunks_pos
        .iter()
        // https://www.w3.org/TR/WGSL/#internal-value-layout
        .map(|off| off.to_le_bytes())
        .flatten()
        .collect();
    render_queue.write_buffer(&voxel_data.chunks_pos, 0, &chunks_pos);

    if !render_chunk_map.to_be_written.is_empty() {
        // push new/modified chunks to stream buffer
        let mut linear_chunks_offsets = Vec::<u8>::new();
        let mut linear_chunks = Vec::<u8>::new();
        for (offset, grid_ptr) in render_chunk_map.to_be_written.iter() {
            let grid = grid_ptr.0.read().unwrap();
            let offset = *offset as u32 * chunk_volume as u32;
            assert_eq!(grid.voxels.len() as u32, chunk_volume);
            linear_chunks.extend(grid.to_bytes());
            linear_chunks_offsets.extend(offset.to_le_bytes());
        }
        render_queue.write_buffer(&voxel_data.chunks_loading, 0, &linear_chunks);
        linear_chunks_offsets.extend(u32::MAX.to_le_bytes());
        render_queue.write_buffer(
            &voxel_data.chunks_loading_offsets,
            0,
            &linear_chunks_offsets,
        );
        render_chunk_map.to_be_written.clear();
    } else {
        // reset
        render_queue.write_buffer(&voxel_data.chunks_loading_offsets, 0, &[]);
        render_queue.write_buffer(
            &voxel_data.chunks_loading_offsets,
            0,
            &u32::MAX.to_le_bytes(),
        );
    }
}

fn bind_voxel_data(render_device: Res<RenderDevice>, mut voxel_data: ResMut<VoxelData>) {
    let bind_group = render_device.create_bind_group(
        None,
        &voxel_data.bind_group_layout,
        &BindGroupEntries::sequential((
            voxel_data.uniform_buffer.binding().unwrap(),
            voxel_data.chunks.as_entire_binding(),
            voxel_data.chunks_pos.as_entire_binding(),
        )),
    );
    voxel_data.bind_group = bind_group;
}

fn bind_images(
    mut voxel_data: ResMut<VoxelData>,
    extr: Res<ExtractedImage>,
    render_device: Res<RenderDevice>,
    render_images: Res<RenderAssets<Image>>,
) {
    if let Some(image) = render_images.get(extr.handle) {
        let bind_group = render_device.create_bind_group(
            None,
            &voxel_data.texture_bind_group_layout,
            &BindGroupEntries::sequential((
                image.texture_view.into_binding(),
                image.sampler.into_binding(),
            )),
        );
        voxel_data.texture_bind_group = Some(bind_group);
    }
}
