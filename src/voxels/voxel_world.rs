use crate::voxels::grid::{Grid, Palette};
use crate::voxels::LoadVoxelWorld;
use crate::Handles;
use bevy::render::render_asset::RenderAssets;
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
use std::sync::{Arc, RwLock};

pub struct VoxelWorldPlugin;

impl Plugin for VoxelWorldPlugin {
    fn build(&self, _app: &mut App) {}
    fn finish(&self, app: &mut App) {
        let render_app = app.get_sub_app(RenderApp).unwrap();
        let render_device = render_app.world.resource::<RenderDevice>();
        let render_queue = render_app.world.resource::<RenderQueue>();

        let gh = Grid::flatland(32);

        let buffer_size = gh.get_buffer_u8_size();
        let chunk_size = gh.size;
        let chunks_grid = 24;
        let chunks_volume = chunks_grid * chunks_grid * chunks_grid;

        // uniforms
        let voxel_uniforms = VoxelUniforms {
            palette: gh.palette.into(),
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
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: chunks.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: offsets_grid.as_entire_binding(),
                },
            ],
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
            buffer_alloc: RenderChunkBufferAllocation {
                buffer_size: chunks_volume,
                ..default()
            },
            ..default()
        };

        app.insert_resource(LoadVoxelWorld::None)
            .insert_resource(ChunkMap::default())
            .insert_resource(ExtractedImage::default())
            .insert_resource(ExtractedCameraPosition::default())
            .insert_resource(voxel_uniforms)
            .add_plugins(ExtractResourcePlugin::<ChunkMap>::default())
            .add_plugins(ExtractResourcePlugin::<VoxelUniforms>::default())
            .add_plugins(ExtractResourcePlugin::<ExtractedImage>::default())
            .add_plugins(ExtractResourcePlugin::<ExtractedCameraPosition>::default())
            .add_systems(Update, extract_images)
            .add_systems(Update, extract_cam);

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
            .add_systems(Render, prepare_chunks.in_set(RenderSet::Prepare))
            .add_systems(
                Render,
                (prepare_uniforms, load_voxel_world_prepare).in_set(RenderSet::Queue),
            )
            .add_systems(Render, queue_bind_group.in_set(RenderSet::Prepare))
            .add_systems(Render, bind_images.in_set(RenderSet::Prepare));
    }
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

#[derive(Debug, Clone)]
pub struct GridPtr(pub Arc<RwLock<Grid>>);

#[derive(Debug, Clone)]
pub struct Chunk {
    pub grid: GridPtr,
    pub version: u32,
}

/// Game resource, it's mutations are propagated to `RenderChunkMap`
/// and written to the gpu buffer.
#[derive(Resource, ExtractResource, Debug, Clone, Default)]
pub struct ChunkMap {
    pub chunks: HashMap<IVec3, Chunk>,
}

impl ChunkMap {
    pub fn pos_to_chunk_and_inner(&self, pos: &IVec3) -> (IVec3, IVec3) {
        // hardcoded chunk size
        let chunk_size = IVec3::splat(32);
        let chunk_pos = (pos.div_euclid(chunk_size)) * chunk_size;
        let inner_pos = pos.rem_euclid(chunk_size);
        (chunk_pos, inner_pos)
    }

    pub fn get_at(&self, pos: &IVec3) -> Option<[u8; 4]> {
        let (chunk_pos, inner_pos) = self.pos_to_chunk_and_inner(pos);
        self.chunks
            .get(&chunk_pos)
            .map(|chunk| chunk.grid.0.read().unwrap().get_at(inner_pos))
    }

    pub fn set_at(&mut self, pos: &IVec3, data: [u8; 4]) {
        let (chunk_pos, inner_pos) = self.pos_to_chunk_and_inner(pos);
        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            chunk.grid.0.write().unwrap().set_at(inner_pos, data);
            chunk.version = chunk.version.wrapping_add(1);
        }
    }
}

#[derive(Resource, Clone, Default)]
pub struct RenderChunkMap {
    pub to_be_written: Vec<(u32, GridPtr)>,
    pub buffer_alloc: RenderChunkBufferAllocation,
    pub versions: HashMap<IVec3, u32>,
}

#[derive(Clone, Default)]
pub struct RenderChunkBufferAllocation {
    pub allocations: HashMap<u32, IVec3>,
    pub buffer_size: u32,
}

use std::{error::Error, fmt};

#[derive(Debug)]
enum AllocationError {
    OutOfSpace,
    AlreadyAllocated,
    NotAllocated,
}

impl Error for AllocationError {}

impl fmt::Display for AllocationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::OutOfSpace => "out of space",
                Self::AlreadyAllocated => "already allocated",
                Self::NotAllocated => "not allocated",
            }
        )
    }
}

impl RenderChunkBufferAllocation {
    fn get_offset(&self, chunk_id: IVec3) -> Option<u32> {
        self.allocations
            .iter()
            .find(|&kv| *kv.1 == chunk_id)
            .map(|kv| *kv.0)
    }

    fn allocate_chunk(&mut self, chunk_id: IVec3) -> Result<u32, AllocationError> {
        if self.get_offset(chunk_id).is_some() {
            return Err(AllocationError::AlreadyAllocated);
        }
        for i in 0..self.buffer_size {
            if let None = self.allocations.get(&i) {
                self.allocations.insert(i, chunk_id);
                return Ok(i);
            }
        }
        Err(AllocationError::OutOfSpace)
    }

    fn deallocate_chunk(&mut self, chunk_id: IVec3) -> Result<(), AllocationError> {
        if let Some(key) = self.get_offset(chunk_id) {
            self.allocations.remove(&key);
            Ok(())
        } else {
            Err(AllocationError::NotAllocated)
        }
    }
}

#[derive(Resource, ExtractResource, Clone, Default)]
pub struct ExtractedImage {
    handle: AssetId<Image>,
}

#[derive(Default, Debug, Clone, Copy, ShaderType)]
pub struct PaletteEntry {
    pub colour: Vec4,
}

impl Into<[PaletteEntry; 256]> for Palette {
    fn into(self) -> [PaletteEntry; 256] {
        let mut pallete = [PaletteEntry::default(); 256];
        for i in 0..256 {
            pallete[i].colour = self[i].into();
        }
        pallete
    }
}

#[derive(Resource, ExtractResource, Clone, ShaderType)]
pub struct VoxelUniforms {
    pub palette: [PaletteEntry; 256],
    pub offsets_grid_size: u32,
    pub chunk_size: u32,
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

fn extract_images(mut extr: ResMut<ExtractedImage>, handles: Res<Handles>) {
    extr.handle = handles.texture_blocks.id();
}

fn extract_cam(mut cam_pos: ResMut<ExtractedCameraPosition>, camera: Query<(&Camera, &Transform)>) {
    let camera_pos = if let Ok((_, tr)) = camera.get_single() {
        tr.translation
    } else {
        Vec3::ZERO
    };

    cam_pos.pos = camera_pos;
}

#[derive(Resource, ExtractResource, Clone, Default)]
pub struct ExtractedCameraPosition {
    pub pos: Vec3,
}

fn prepare_chunks(
    voxel_uniforms: Res<VoxelUniforms>,
    mut chunk_map: ResMut<ChunkMap>,
    mut render_chunk_map: ResMut<RenderChunkMap>,
    cam_pos: Res<ExtractedCameraPosition>,
) {
    let chunk_view_distance: u32 = 200;

    let chunk_side = voxel_uniforms.chunk_size;
    let camera_chunk_pos = (cam_pos.pos / chunk_side as f32) * chunk_side as f32;

    let visible_chunks: HashSet<IVec3> = chunk_map
        .chunks
        .iter()
        .filter_map(|(pos, _chunk)| {
            if (camera_chunk_pos - pos.as_vec3()).length_squared()
                < chunk_view_distance.pow(2) as f32
            {
                Some(*pos)
            } else {
                None
            }
        })
        .collect();

    let to_be_removed: HashSet<IVec3> = render_chunk_map
        .buffer_alloc
        .allocations
        .iter()
        .filter_map(|(_offset, pos)| (!visible_chunks.contains(pos)).then(|| *pos))
        .collect();
    for &pos in to_be_removed.iter() {
        render_chunk_map.buffer_alloc.deallocate_chunk(pos).unwrap();
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
                    if render_chunk_map.buffer_alloc.get_offset(*pos).is_none() {
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
        if let Some(offset) = render_chunk_map.buffer_alloc.get_offset(pos) {
            render_chunk_map.to_be_written.push((offset, grid));
        } else {
            if let Ok(offset) = render_chunk_map.buffer_alloc.allocate_chunk(pos) {
                render_chunk_map.to_be_written.push((offset, grid));
            } else {
                panic!();
            }
        }
    }
}

fn load_voxel_world_prepare(
    voxel_uniforms: Res<VoxelUniforms>,
    voxel_data: Res<VoxelData>,
    //render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut render_chunk_map: ResMut<RenderChunkMap>,
    cam_pos: Res<ExtractedCameraPosition>,
) {
    let chunk_side = voxel_uniforms.chunk_size;
    let chunk_volume = chunk_side * chunk_side * chunk_side;
    let outer = voxel_uniforms.offsets_grid_size;

    let camera_chunk_pos = (cam_pos.pos / chunk_side as f32).as_ivec3() * chunk_side as i32;
    let center = IVec3::splat(outer as i32) / 2 * chunk_side as i32;

    // voxel outer grid
    let mut chunks_pos: Vec<u32> = vec![];
    for x in 0..outer {
        for y in 0..outer {
            for z in 0..outer {
                let mut pos = IVec3::new(x as i32, y as i32, z as i32) * chunk_side as i32;
                pos -= center;
                pos += camera_chunk_pos;
                if let Some(offset) = render_chunk_map.buffer_alloc.get_offset(pos) {
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

    // push new/modified chunks to stream buffer
    if !render_chunk_map.to_be_written.is_empty() {
        let mut linear_chunks_offsets = Vec::<u8>::new();
        let mut linear_chunks = Vec::<u8>::new();
        for (offset, grid_ptr) in render_chunk_map.to_be_written.iter() {
            let grid = grid_ptr.0.read().unwrap();
            let offset = *offset as u32 * chunk_volume as u32;
            assert_eq!(grid.voxels.len() as u32, chunk_volume * 4);
            linear_chunks.extend(&grid.voxels);
            linear_chunks_offsets.extend(offset.to_le_bytes());
        }
        render_queue.write_buffer(&voxel_data.chunks_loading, 0, &linear_chunks);
        linear_chunks_offsets.extend(u32::MAX.to_le_bytes());
        render_queue.write_buffer(
            &voxel_data.chunks_loading_offsets,
            0,
            &linear_chunks_offsets,
        );
        //println!("written {} chunks", render_chunk_map.to_be_written.len());
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

fn queue_bind_group(render_device: Res<RenderDevice>, mut voxel_data: ResMut<VoxelData>) {
    let bind_group = render_device.create_bind_group(
        None,
        &voxel_data.bind_group_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: voxel_data.uniform_buffer.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 1,
                resource: voxel_data.chunks.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: voxel_data.chunks_pos.as_entire_binding(),
            },
        ],
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
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: image.texture_view.into_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: image.sampler.into_binding(),
                },
            ],
        );
        voxel_data.texture_bind_group = Some(bind_group);
    }
}
