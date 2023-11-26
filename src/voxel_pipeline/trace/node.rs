use super::{TracePipelineData, ViewTraceUniformBuffer};
use crate::voxel_pipeline::{
    attachments::RenderAttachments, voxel_world::VoxelData, RenderGraphSettings,
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self, ViewNode},
        render_resource::*,
        view::ViewTarget,
    },
};

#[derive(Default)]
pub struct TraceNode;

impl ViewNode for TraceNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ViewTraceUniformBuffer,
        &'static RenderAttachments,
    );
    fn run(
        &self,
        graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        view_query: bevy::ecs::query::QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let voxel_data = world.get_resource::<VoxelData>().unwrap();
        let trace_pipeline_data = world.get_resource::<TracePipelineData>().unwrap();
        let render_graph_settings = world.get_resource::<RenderGraphSettings>().unwrap();

        if !render_graph_settings.trace {
            return Ok(());
        }

        let (target, trace_uniform_buffer, render_attachments) = view_query;

        let trace_pipeline =
            match pipeline_cache.get_render_pipeline(trace_pipeline_data.trace_pipeline_id) {
                Some(pipeline) => pipeline,
                None => return Ok(()),
            };
        let reprojection_pipeline = match pipeline_cache
            .get_render_pipeline(trace_pipeline_data.reprojection_pipeline_id)
        {
            Some(pipeline) => pipeline,
            None => return Ok(()),
        };
        let accumulation_pipeline = match pipeline_cache
            .get_render_pipeline(trace_pipeline_data.accumulation_pipeline_id)
        {
            Some(pipeline) => pipeline,
            None => return Ok(()),
        };

        let post_process = target.post_process_write();
        let source = post_process.source;
        let destination = post_process.destination;

        let gpu_images = world.get_resource::<RenderAssets<Image>>().unwrap();

        let accumulation = &gpu_images
            .get(&render_attachments.accumulation)
            .unwrap()
            .texture_view;
        let normal = &gpu_images
            .get(&render_attachments.normal)
            .unwrap()
            .texture_view;
        let position = &gpu_images
            .get(&render_attachments.position)
            .unwrap()
            .texture_view;

        let trace_bind_group = render_context.render_device().create_bind_group(
            None,
            &trace_pipeline_data.trace_bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: trace_uniform_buffer.binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(accumulation),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(normal),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(position),
                },
            ],
        );
        let source_bind_group = render_context.render_device().create_bind_group(
            None,
            &trace_pipeline_data.reprojection_bind_group_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&source),
            }],
        );
        let destination_bind_group = render_context.render_device().create_bind_group(
            None,
            &trace_pipeline_data.reprojection_bind_group_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&destination),
            }],
        );

        let source_descriptor = RenderPassDescriptor {
            label: Some("trace pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: source,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        };
        let destination_descriptor = RenderPassDescriptor {
            label: Some("trace pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: destination,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        };

        {
            let mut render_pass = render_context
                .command_encoder()
                .begin_render_pass(&destination_descriptor);

            render_pass.set_bind_group(0, &voxel_data.bind_group, &[]);
            render_pass.set_bind_group(1, &trace_bind_group, &[]);

            render_pass.set_pipeline(trace_pipeline);
            render_pass.draw(0..3, 0..1);
        }
        {
            let mut render_pass = render_context
                .command_encoder()
                .begin_render_pass(&source_descriptor);

            render_pass.set_bind_group(0, &trace_bind_group, &[]);
            render_pass.set_bind_group(1, &destination_bind_group, &[]);

            render_pass.set_pipeline(reprojection_pipeline);
            render_pass.draw(0..3, 0..1);
        }
        {
            let mut render_pass = render_context
                .command_encoder()
                .begin_render_pass(&destination_descriptor);

            render_pass.set_bind_group(0, &trace_bind_group, &[]);
            render_pass.set_bind_group(1, &source_bind_group, &[]);

            render_pass.set_pipeline(accumulation_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}
