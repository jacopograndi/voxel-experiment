use crate::voxels::voxel_world::{VoxelData, VoxelUniforms};
use crate::voxels::RenderGraphSettings;
use bevy::render::render_graph::ViewNode;
use bevy::{
    prelude::*,
    render::{
        render_graph::{NodeRunError, RenderGraphContext},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
    },
};
use std::borrow::Cow;

use super::compute::STREAM_SHADER_HANDLE;

#[derive(Default)]
pub struct StreamNode;

#[derive(Resource)]
pub struct Pipeline {
    copy_pipeline: CachedComputePipelineId,
    copy_bind_group_layout: BindGroupLayout,
}

impl FromWorld for Pipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let copy_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(VoxelUniforms::SHADER_SIZE.into()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(0),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(0),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(0),
                        },
                        count: None,
                    },
                ],
            });
        // let voxel_bind_group_layout = world.resource::<VoxelData>().bind_group_layout.clone();
        let pipeline_cache = world.resource_mut::<PipelineCache>();

        let copy_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("copy pipeline")),
            layout: vec![copy_bind_group_layout.clone()],
            shader: STREAM_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: Cow::from("copy"),
            push_constant_ranges: vec![],
        });
        Pipeline {
            copy_pipeline,
            copy_bind_group_layout,
        }
    }
}

impl ViewNode for StreamNode {
    type ViewQuery = ();
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        _view_query: bevy::ecs::query::QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let render_device = world.resource::<RenderDevice>();
        let voxel_data = world.resource::<VoxelData>();
        let voxel_uniforms = world.resource::<VoxelUniforms>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<Pipeline>();

        let copy_bind_group = render_device.create_bind_group(
            None,
            &pipelines.copy_bind_group_layout,
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
                    resource: voxel_data.chunks_loading.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: voxel_data.chunks_loading_offsets.as_entire_binding(),
                },
            ],
        );

        // stream pipeline
        let copy_pipeline = match pipeline_cache.get_compute_pipeline(pipelines.copy_pipeline) {
            Some(pipeline) => pipeline,
            None => return Ok(()),
        };

        {
            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor::default());

            let dispatch_size = voxel_uniforms.chunk_size / 4;
            pass.set_bind_group(0, &copy_bind_group, &[]);

            pass.set_pipeline(copy_pipeline);
            pass.dispatch_workgroups(dispatch_size, dispatch_size, dispatch_size);
        }

        Ok(())
    }
}
