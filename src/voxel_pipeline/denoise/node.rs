use super::{super::RenderGraphSettings, DenoisePassData, DenoisePipeline};
use crate::voxel_pipeline::{attachments::RenderAttachments, trace::TraceSettings};
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
pub struct DenoiseNode;

impl ViewNode for DenoiseNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static TraceSettings,
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
        let denoise_pipeline = world.resource::<DenoisePipeline>();
        let render_graph_settings = world.get_resource::<RenderGraphSettings>().unwrap();

        let (target, trace_uniforms, render_attachments) = view_query;

        if !render_graph_settings.denoise || !trace_uniforms.indirect_lighting {
            return Ok(());
        }

        let pipeline = match pipeline_cache.get_render_pipeline(denoise_pipeline.pipeline_id) {
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

        let bind_group = render_context.render_device().create_bind_group(
            None,
            &denoise_pipeline.bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: denoise_pipeline.uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&accumulation),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&normal),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&position),
                },
            ],
        );
        let source_bind_group = render_context.render_device().create_bind_group(
            None,
            &denoise_pipeline.pass_data_bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: denoise_pipeline.pass_data.binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(source),
                },
            ],
        );
        let destination_bind_group = render_context.render_device().create_bind_group(
            None,
            &denoise_pipeline.pass_data_bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: denoise_pipeline.pass_data.binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(destination),
                },
            ],
        );

        let source_descriptor = RenderPassDescriptor {
            label: Some("denoise pass"),
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
            label: Some("denoise pass"),
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

        let offset_size = u64::from(DenoisePassData::SHADER_SIZE) as u32;

        {
            let mut render_pass = render_context
                .command_encoder()
                .begin_render_pass(&destination_descriptor);

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_bind_group(1, &source_bind_group, &[0]);
            render_pass.draw(0..3, 0..1);
        }
        {
            let mut render_pass = render_context
                .command_encoder()
                .begin_render_pass(&source_descriptor);

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_bind_group(1, &destination_bind_group, &[offset_size]);
            render_pass.draw(0..3, 0..1);
        }
        {
            let mut render_pass = render_context
                .command_encoder()
                .begin_render_pass(&destination_descriptor);

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_bind_group(1, &source_bind_group, &[2 * offset_size]);
            render_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}
