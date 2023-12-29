use bevy::{ecs::bundle::Bundle, render::{camera::{Camera, CameraRenderGraph, Projection}, view::VisibleEntities, primitives::Frustum}, transform::components::{Transform, GlobalTransform}, core_pipeline::{core_3d::Camera3d, tonemapping::Tonemapping}, prelude::default};

use crate::{pipeline::trace::TraceSettings, VOXEL};


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
}
impl Default for VoxelCameraBundle {
    fn default() -> Self {
        Self {
            camera_render_graph: CameraRenderGraph::new(VOXEL),
            tonemapping: Tonemapping::ReinhardLuminance,
            camera: Camera {
                hdr: true,
                ..default()
            },
            projection: default(),
            visible_entities: default(),
            frustum: default(),
            transform: default(),
            global_transform: default(),
            camera_3d: default(),
            trace_settings: default(),
        }
    }
}