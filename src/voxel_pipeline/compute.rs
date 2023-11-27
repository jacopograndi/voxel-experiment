use bevy::{
    asset::load_internal_asset,
    prelude::*,
    render::{
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        Render, RenderApp, RenderSet,
    },
};

use crate::voxel_pipeline::{mip, rebuild};

const MAX_TYPE_BUFFER_DATA: usize = 1000000; // 4mb

pub const MIP_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(6189143918759879663);
pub const REBUILD_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(18135969847573717619);

pub struct ComputeResourcesPlugin;

impl Plugin for ComputeResourcesPlugin {
    fn build(&self, app: &mut App) {}
    fn finish(&self, app: &mut App) {
        load_internal_asset!(
            app,
            MIP_SHADER_HANDLE,
            "shaders/mip.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            REBUILD_SHADER_HANDLE,
            "shaders/rebuild.wgsl",
            Shader::from_wgsl
        );

        let render_device = app.world.resource::<RenderDevice>();
        let render_queue = app.world.resource::<RenderQueue>();

        let mut uniform_buffer = UniformBuffer::from(ComputeUniforms {
            time: 0.0,
            delta_time: 0.0,
        });
        uniform_buffer.write_buffer(render_device, render_queue);

        let physics_buffer_gpu = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&vec![0u32; MAX_TYPE_BUFFER_DATA]),
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let physics_buffer_cpu = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&vec![0u32; MAX_TYPE_BUFFER_DATA]),
            label: None,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        });
        let animation_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: bytemuck::cast_slice(&vec![0u32; MAX_TYPE_BUFFER_DATA]),
            label: None,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("compute bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(ComputeUniforms::SHADER_SIZE.into()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(4),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(4),
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
                    resource: physics_buffer_gpu.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: animation_buffer.as_entire_binding(),
                },
            ],
        );

        app.sub_app_mut(RenderApp)
            .insert_resource(ComputeData {
                bind_group_layout,
                bind_group,
                uniform_buffer,
            })
            .init_resource::<rebuild::Pipeline>()
            .init_resource::<mip::Pipeline>()
            .add_systems(Render, prepare_uniforms.in_set(RenderSet::Queue));
    }
}

fn prepare_uniforms(
    time: Res<Time>,
    mut compute_data: ResMut<ComputeData>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let uniforms = ComputeUniforms {
        time: time.elapsed_seconds_f64() as f32,
        delta_time: time.delta_seconds() as f32,
    };
    compute_data.uniform_buffer.set(uniforms);
    compute_data
        .uniform_buffer
        .write_buffer(&render_device, &render_queue);
}

#[derive(Resource, ShaderType)]
struct ComputeUniforms {
    time: f32,
    delta_time: f32,
}

#[derive(Resource)]
pub struct ComputeData {
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
    uniform_buffer: UniformBuffer<ComputeUniforms>,
}
