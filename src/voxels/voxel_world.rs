use crate::voxels::grid_hierarchy::{Grid, Palette};
use crate::voxels::LoadVoxelWorld;
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
use std::num::NonZeroU64;
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
        let chunks_grid = 3;
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
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let offsets_grid = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: &vec![0; chunks_volume as usize * 4],
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let chunks_loading = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: &vec![0; buffer_size as usize * chunks_volume as usize],
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let chunks_loading_offsets = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: &vec![0; chunks_volume as usize * 4],
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

        app.insert_resource(LoadVoxelWorld::None)
            .insert_resource(ArcGridHierarchy::None)
            .insert_resource(ChunkMap::default())
            .insert_resource(RenderChunkMap::default())
            .insert_resource(voxel_uniforms)
            .add_plugins(ExtractResourcePlugin::<RenderChunkMap>::default())
            .add_plugins(ExtractResourcePlugin::<ArcGridHierarchy>::default())
            .add_plugins(ExtractResourcePlugin::<VoxelUniforms>::default())
            .add_systems(Update, extract_chunks);

        app.sub_app_mut(RenderApp)
            .insert_resource(VoxelData {
                uniform_buffer,
                chunks,
                chunks_pos: offsets_grid,
                chunks_loading,
                chunks_loading_offsets,
                bind_group_layout,
                bind_group,
            })
            .add_systems(Render, prepare_uniforms.in_set(RenderSet::Queue))
            .add_systems(Render, load_voxel_world_prepare.in_set(RenderSet::Queue))
            .add_systems(Render, queue_bind_group.in_set(RenderSet::Prepare));
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
}

#[derive(Debug, Clone)]
pub struct GridPtr(pub Arc<RwLock<Grid>>);

#[derive(Debug, Clone)]
pub struct Chunk {
    pub grid: GridPtr,
    pub was_mutated: bool,
}

/// Game resource, it's mutations are propagated to `RenderChunkMap`
/// and written to the gpu buffer.
#[derive(Resource, Debug, Clone, Default)]
pub struct ChunkMap {
    pub chunks: HashMap<IVec3, Chunk>,
}

#[derive(Resource, ExtractResource, Clone, Default)]
pub struct RenderChunkMap {
    pub chunks: HashMap<IVec3, u32>,
    pub to_be_written: Vec<(IVec3, GridPtr)>,
}

pub struct RenderChunkBuffer {
    pub buffer_size: u32,
    pub allocation_table: [u32; 512],
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

#[derive(Resource, ExtractResource, Clone)]
pub enum ArcGridHierarchy {
    Some(Arc<RwLock<Grid>>),
    None,
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

fn extract_chunks(
    voxel_uniforms: Res<VoxelUniforms>,
    mut chunk_map: ResMut<ChunkMap>,
    mut render_chunk_map: ResMut<RenderChunkMap>,
) {
    let game_chunks: HashSet<IVec3> = chunk_map.chunks.iter().map(|c| *c.0).collect();
    let render_chunks: HashSet<IVec3> = render_chunk_map.chunks.iter().map(|rc| *rc.0).collect();
    let modified_chunks: HashSet<IVec3> = chunk_map
        .chunks
        .iter()
        .filter(|c| c.1.was_mutated)
        .map(|c| *c.0)
        .collect();

    let new_chunks: HashSet<IVec3> = game_chunks.difference(&render_chunks).cloned().collect();
    let _deleted_chunks: HashSet<IVec3> = render_chunks.difference(&game_chunks).cloned().collect();
    let new_chunks: HashSet<IVec3> = modified_chunks.union(&new_chunks).cloned().collect();

    render_chunk_map.to_be_written.clear();
    for (i, &pos) in new_chunks.iter().enumerate() {
        if let None = render_chunk_map.chunks.get(&pos) {
            let offset = i as u32;
            render_chunk_map.chunks.insert(pos, offset);
        }
        let chunk = chunk_map.chunks.get(&pos).unwrap();
        //println!("pos {pos}, {:p}", chunk.grid.0);
        render_chunk_map
            .to_be_written
            .push((pos, chunk.grid.clone()));
    }
    if !render_chunk_map.to_be_written.is_empty() && false {
        println!(
            "{:?}, {:?}, {:?}, {:?}",
            render_chunk_map.to_be_written.len(),
            new_chunks,
            game_chunks,
            render_chunks
        );
    }

    for (_, chunk) in chunk_map.chunks.iter_mut() {
        chunk.was_mutated = false;
    }
}

fn load_voxel_world_prepare(
    voxel_uniforms: Res<VoxelUniforms>,
    voxel_data: Res<VoxelData>,
    //render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    render_chunk_map: Res<RenderChunkMap>,
) {
    let chunk_side = voxel_uniforms.chunk_size;
    let chunk_volume = chunk_side * chunk_side * chunk_side;
    let chunk_bytes = chunk_volume * 4;

    let mut linear_chunks_offsets = Vec::<u8>::new();
    let mut linear_chunks = Vec::<u8>::new();
    //linear_chunks.reserve_exact(chunk_bytes as usize * render_chunk_map.to_be_written.len());
    for (pos, grid_ptr) in render_chunk_map.to_be_written.iter() {
        if let Some(offset) = render_chunk_map.chunks.get(pos) {
            let grid = grid_ptr.0.read().unwrap();
            let offset = *offset as u32 * chunk_bytes as u32;
            assert_eq!(grid.voxels.len() as u32, chunk_bytes);
            linear_chunks.extend(&grid.voxels);
            linear_chunks_offsets.extend(offset.to_le_bytes());
        } else {
            linear_chunks_offsets.extend(u32::MAX.to_le_bytes());
        }
    }
    linear_chunks_offsets.extend(u32::MAX.to_le_bytes());
    if linear_chunks_offsets.len() > 4 {
        println!("{:?}", linear_chunks_offsets);
    }
    // write_buffer calls overwrite each other
    render_queue.write_buffer(&voxel_data.chunks_loading, 0, &linear_chunks);
    render_queue.write_buffer(
        &voxel_data.chunks_loading_offsets,
        0,
        &linear_chunks_offsets,
    );

    /*
    for (pos, grid_ptr) in render_chunk_map.to_be_written.iter() {
        if let Some(offset) = render_chunk_map.chunks.get(pos) {
            let grid = grid_ptr.0.read().unwrap();
            let offset = *offset as u64 * grid.get_buffer_u8_size() as u64;
            dbg!(offset, grid.voxels.len());
            println!("podds{pos} {:p}", grid_ptr.0);
            if let Some(mut buffer) = render_queue.write_buffer_with(
                &voxel_data.chunks,
                offset,
                NonZeroU64::new(grid.voxels.len() as u64).unwrap(),
            ) {
                buffer.as_mut().copy_from_slice(&grid.voxels);
            }
        }
    }
    */

    let chunk_side = voxel_uniforms.chunk_size;
    let chunk_volume = chunk_side * chunk_side * chunk_side;
    let side = voxel_uniforms.offsets_grid_size;
    let mut chunks_pos: Vec<u32> = vec![];
    for z in 0..side {
        for y in 0..side {
            for x in 0..side {
                let mut pos = IVec3::new(x as i32, y as i32, z as i32);
                pos *= voxel_uniforms.chunk_size as i32;
                // translate the (0,0,0) chunk to the center of the chunk grid
                //pos = pos - IVec3::splat(side as i32) / 2;
                if let Some(offset) = render_chunk_map.chunks.get(&pos) {
                    chunks_pos.push(*offset * chunk_volume * 4);
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
