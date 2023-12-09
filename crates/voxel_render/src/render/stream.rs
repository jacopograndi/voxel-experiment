use crate::voxel_world::{VoxelData, VoxelUniforms};
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
    stream_pipeline: CachedComputePipelineId,
    stream_bind_group_layout: BindGroupLayout,
}

impl FromWorld for Pipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let stream_bind_group_layout =
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
        let pipeline_cache = world.resource_mut::<PipelineCache>();

        let stream_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("stream pipeline")),
            layout: vec![stream_bind_group_layout.clone()],
            shader: STREAM_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: Cow::from("copy"),
            push_constant_ranges: vec![],
        });
        Pipeline {
            stream_pipeline,
            stream_bind_group_layout,
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

        let stream_bind_group = render_device.create_bind_group(
            None,
            &pipelines.stream_bind_group_layout,
            &BindGroupEntries::sequential((
                   voxel_data.uniform_buffer.binding().unwrap(),
                    voxel_data.chunks.as_entire_binding(),
                    voxel_data.chunks_loading.as_entire_binding(),
                    voxel_data.chunks_loading_offsets.as_entire_binding(),
            )),
        );

        let stream_pipeline = match pipeline_cache.get_compute_pipeline(pipelines.stream_pipeline) {
            Some(pipeline) => pipeline,
            None => return Ok(()),
        };

        {
            let mut pass = render_context
                .command_encoder()
                .begin_compute_pass(&ComputePassDescriptor::default());

            let dispatch_size = voxel_uniforms.chunk_size / 4;
            pass.set_bind_group(0, &stream_bind_group, &[]);

            pass.set_pipeline(stream_pipeline);
            pass.dispatch_workgroups(dispatch_size, dispatch_size, dispatch_size);
        }

        Ok(())
    }
}
