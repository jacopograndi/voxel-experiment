use bevy::{
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor,
            BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferInitDescriptor,
            BufferSize, BufferUsages, ShaderStages,
        },
        renderer::{RenderDevice, RenderQueue},
        view::ExtractedView,
        Extract, Render, RenderApp, RenderSet,
    },
    utils::EntityHashMap,
};

use crate::voxel_world::{VoxelUniforms, VIEW_DISTANCE};

const MAX_BOXES_BYTES: usize = 20000;

pub struct BoxesWorldPlugin;

impl Plugin for BoxesWorldPlugin {
    fn build(&self, _app: &mut App) {}
    fn finish(&self, app: &mut App) {
        let render_app = app.get_sub_app(RenderApp).unwrap();
        let render_device = render_app.world.resource::<RenderDevice>();

        let boxes_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            contents: &vec![0; std::mem::size_of::<PodTexturedBox>() * MAX_BOXES_BYTES],
            label: Some("chunk_storage"),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("boxes bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(0),
                    },
                    count: None,
                }],
            });

        let bind_group = render_device.create_bind_group(
            None,
            &bind_group_layout,
            &BindGroupEntries::sequential((boxes_buffer.as_entire_binding(),)),
        );

        app.sub_app_mut(RenderApp)
            .insert_resource(ExtractedTexturedBoxes::default())
            .insert_resource(BoxesData {
                bind_group_layout,
                bind_group,
                boxes_buffer,
            })
            .add_systems(ExtractSchedule, extract_boxes)
            .add_systems(
                Render,
                (write_boxes, bind_boxes_data).in_set(RenderSet::Prepare),
            );
    }
}

#[derive(Component, Default, Debug, Clone)]
pub struct TexturedBox {
    pub size: Vec3,
}

#[derive(Default, Debug, Clone)]
pub struct ExtractedTexturedBox {
    pub transform: GlobalTransform,
    pub size: Vec3,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PodTexturedBox {
    pub world_to_box: Mat4,
    pub box_to_world: Mat4,
    pub size: Vec4,
}

#[derive(Resource, Default)]
pub struct ExtractedTexturedBoxes {
    pub boxes: EntityHashMap<Entity, ExtractedTexturedBox>,
}

fn extract_boxes(
    box_query: Extract<Query<(Entity, &GlobalTransform, &TexturedBox, &ViewVisibility)>>,
    mut extracted_boxes: ResMut<ExtractedTexturedBoxes>,
) {
    extracted_boxes.boxes.clear();
    for (entity, transform, texbox, view_visibility) in box_query.iter() {
        if !view_visibility.get() {
            continue;
        }
        extracted_boxes.boxes.insert(
            entity,
            ExtractedTexturedBox {
                transform: *transform,
                size: texbox.size,
            },
        );
    }
}

#[derive(Resource)]
pub struct BoxesData {
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
    pub boxes_buffer: Buffer,
}

fn write_boxes(
    voxel_uniforms: Res<VoxelUniforms>,
    extracted_boxes: Res<ExtractedTexturedBoxes>,
    view_query: Query<(&ExtractedView, &ExtractedCamera)>,
    render_queue: Res<RenderQueue>,
    boxes_data: Res<BoxesData>,
) {
    let Ok((view, ..)) = view_query.get_single() else {
        return;
    };
    let cam_pos = view.transform.translation();

    let chunk_side = voxel_uniforms.chunk_size;
    let camera_chunk_pos = (cam_pos / chunk_side as f32) * chunk_side as f32;

    let visible_boxes: Vec<ExtractedTexturedBox> = extracted_boxes
        .boxes
        .iter()
        .filter_map(|(_ent, texbox)| {
            let pos = texbox.transform.translation();
            if (camera_chunk_pos - pos).length_squared() < VIEW_DISTANCE.pow(2) as f32 {
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
                size: texbox.size.extend(0.0),
            }
        })
        .collect();

    let slice: &[u8] = bytemuck::cast_slice(&boxes);
    let len = boxes.len() as u32;
    let bytes: Vec<u8> = [
        len.to_le_bytes().as_slice(),
        len.to_le_bytes().as_slice(),
        len.to_le_bytes().as_slice(),
        len.to_le_bytes().as_slice(),
        slice,
    ]
    .concat();

    render_queue.write_buffer(&boxes_data.boxes_buffer, 0, &bytes);
}

fn bind_boxes_data(render_device: Res<RenderDevice>, mut boxes_data: ResMut<BoxesData>) {
    let bind_group = render_device.create_bind_group(
        None,
        &boxes_data.bind_group_layout,
        &BindGroupEntries::sequential((boxes_data.boxes_buffer.as_entire_binding(),)),
    );
    boxes_data.bind_group = bind_group;
}
