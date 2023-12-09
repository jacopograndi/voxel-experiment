use crate::{
    voxel_world::{VoxelData},
    RenderGraphSettings,
};
use bevy::{
    asset::load_internal_asset,
    core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        globals::{GlobalsBuffer, GlobalsUniform},
        render_graph::{self, ViewNode},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        Render, RenderApp, RenderSet,
    },
};

const TRACE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(3541867952248261868);
const COMMON_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1874948457211004189);

pub struct TracePlugin;

impl Plugin for TracePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            COMMON_SHADER_HANDLE,
            "shaders/common.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            TRACE_SHADER_HANDLE,
            "shaders/trace.wgsl",
            Shader::from_wgsl
        );
    }
    fn finish(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<TraceSettings>::default());

        // setup custom render pipeline
        app.sub_app_mut(RenderApp)
            .init_resource::<TracePipelineData>()
            .add_systems(Render, prepare_uniforms.in_set(RenderSet::Prepare));
    }
}

#[derive(Resource)]
struct TracePipelineData {
    trace_pipeline_id: CachedRenderPipelineId,
    trace_bind_group_layout: BindGroupLayout,
}

#[derive(Component, Clone, ExtractComponent)]
pub struct TraceSettings {
    pub show_ray_steps: bool,
    pub indirect_lighting: bool,
    pub samples: u32,
    pub reprojection_factor: f32,
    pub shadows: bool,
}

impl Default for TraceSettings {
    fn default() -> Self {
        Self {
            show_ray_steps: false,
            indirect_lighting: false,
            samples: 1,
            reprojection_factor: 0.0,
            shadows: true,
        }
    }
}

#[derive(Clone, ShaderType)]
pub struct TraceUniforms {
    pub show_ray_steps: u32,
    pub indirect_lighting: u32,
    pub samples: u32,
    pub reprojection_factor: f32,
    pub shadows: u32,
}

#[derive(Component)]
pub struct TraceUniformBindGroup {
    bind_group: BindGroup,
}

fn prepare_uniforms(
    mut commands: Commands,
    query: Query<(Entity, &TraceSettings)>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    trace_pipeline_data: Res<TracePipelineData>,
    view_uniforms: Res<ViewUniforms>,
    global_buffer: Res<GlobalsBuffer>,
) {
    for (entity, settings) in query.iter() {
        let trace_uniforms = TraceUniforms {
            show_ray_steps: settings.show_ray_steps as u32,
            indirect_lighting: settings.indirect_lighting as u32,
            samples: settings.samples,
            reprojection_factor: settings.reprojection_factor,
            shadows: settings.shadows as u32,
        };

        let mut trace_uniform_buffer = UniformBuffer::from(trace_uniforms);
        trace_uniform_buffer.write_buffer(&render_device, &render_queue);

        let trace_bind_group = render_device.create_bind_group(
            None,
            &trace_pipeline_data.trace_bind_group_layout,
            &BindGroupEntries::sequential((
                trace_uniform_buffer.binding().unwrap(),
                view_uniforms.uniforms.binding().unwrap(),
                global_buffer.buffer.binding().unwrap(),
            )),
        );

        commands.entity(entity).insert(TraceUniformBindGroup {
            bind_group: trace_bind_group,
        });
    }
}

impl FromWorld for TracePipelineData {
    fn from_world(render_world: &mut World) -> Self {
        let voxel_data = render_world.get_resource::<VoxelData>().unwrap();

        let voxel_bind_group_layout = voxel_data.bind_group_layout.clone();
        let trace_bind_group_layout = render_world
            .resource::<RenderDevice>()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("trace bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(TraceUniforms::SHADER_SIZE.into()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: BufferSize::new(ViewUniform::SHADER_SIZE.into()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(GlobalsUniform::SHADER_SIZE.into()),
                        },
                        count: None,
                    },
                ],
            });
        let texture_bind_group_layout = render_world
            .resource::<RenderDevice>()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("texture bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
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
            });

        let trace_pipeline_descriptor = RenderPipelineDescriptor {
            label: Some("trace pipeline".into()),
            layout: vec![
                voxel_bind_group_layout.clone(),
                trace_bind_group_layout.clone(),
                texture_bind_group_layout.clone(),
            ],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: TRACE_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: ViewTarget::TEXTURE_FORMAT_HDR,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
        };

        let cache = render_world.resource::<PipelineCache>();
        let trace_pipeline_id = cache.queue_render_pipeline(trace_pipeline_descriptor);

        TracePipelineData {
            trace_pipeline_id,
            trace_bind_group_layout,
        }
    }
}

#[derive(Default)]
pub struct TraceNode;

impl ViewNode for TraceNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static TraceUniformBindGroup,
        &'static ViewUniformOffset,
    );
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        view_query: bevy::ecs::query::QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let voxel_data = world.get_resource::<VoxelData>().unwrap();
        let trace_pipeline_data = world.get_resource::<TracePipelineData>().unwrap();
        let render_graph_settings = world.get_resource::<RenderGraphSettings>().unwrap();

        let Some(texture_bind_group) = voxel_data.texture_bind_group.as_ref() else {
            println!("No texture");
            return Ok(());
        };

        if !render_graph_settings.trace {
            return Ok(());
        }

        let (target, trace_bind_group, view_uniform_offset) = view_query;

        let trace_pipeline =
            match pipeline_cache.get_render_pipeline(trace_pipeline_data.trace_pipeline_id) {
                Some(pipeline) => pipeline,
                None => return Ok(()),
            };

        let post_process = target.post_process_write();
        let destination = post_process.destination;

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
            render_pass.set_bind_group(
                1,
                &trace_bind_group.bind_group,
                &[view_uniform_offset.offset],
            );
            render_pass.set_bind_group(2, &texture_bind_group, &[]);

            render_pass.set_pipeline(trace_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}
