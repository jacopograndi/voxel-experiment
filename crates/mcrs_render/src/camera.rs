use bevy::{
    core_pipeline::{core_3d::Camera3d, tonemapping::Tonemapping},
    ecs::bundle::Bundle,
    prelude::default,
    render::{
        camera::{Camera, CameraMainTextureUsages, CameraRenderGraph, Projection},
        primitives::Frustum,
        view::VisibleEntities,
    },
    transform::components::{GlobalTransform, Transform},
};

use crate::{graph::Voxel, pipeline::trace::TraceSettings};

// todo: maybe convert to required component like a `Camera3d`
#[derive(Bundle)]
pub struct VoxelCameraBundle {
    pub camera: Camera,
    pub camera_render_graph: CameraRenderGraph,
    pub projection: Projection,
    pub visible_entities: VisibleEntities,
    pub frustum: Frustum,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub camera_3d: Camera3d,
    pub tonemapping: Tonemapping,
    pub trace_settings: TraceSettings,
    pub main_texture_usages: CameraMainTextureUsages,
}
impl Default for VoxelCameraBundle {
    fn default() -> Self {
        Self {
            camera_render_graph: CameraRenderGraph::new(Voxel),
            tonemapping: Tonemapping::ReinhardLuminance,
            camera: Camera { ..default() },
            projection: default(),
            visible_entities: default(),
            frustum: default(),
            transform: default(),
            global_transform: default(),
            camera_3d: default(),
            trace_settings: default(),
            main_texture_usages: default(),
        }
    }
}
