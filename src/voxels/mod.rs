use crate::voxels::render::{
    compute::ComputeResourcesPlugin,
    denoise::{DenoiseNode, DenoisePlugin},
    stream::StreamNode,
    trace::{TraceNode, TracePlugin, TraceSettings},
};

use self::{render::attachments::AttachmentsPlugin, voxel_world::VoxelWorldPlugin};
use bevy::{
    core_pipeline::{
        fxaa::FxaaNode,
        tonemapping::{Tonemapping, TonemappingNode},
        upscaling::UpscalingNode,
    },
    prelude::*,
    render::{
        camera::CameraRenderGraph,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        primitives::Frustum,
        render_graph::{RenderGraphApp, ViewNodeRunner},
        view::VisibleEntities,
        RenderApp,
    },
};

pub mod grid;
pub mod raycast;
pub mod render;
pub mod voxel_world;

pub struct RenderPlugin;

pub mod graph {
    pub const NAME: &'static str = "voxel";
    pub mod node {
        pub const TRACE: &str = "trace";
        pub const DENOISE: &str = "denoise";
        pub const TONEMAPPING: &str = "tonemapping";
        pub const FXAA: &str = "fxaa";
        pub const UPSCALING: &str = "upscaling";
        pub const STREAM: &str = "stream";
        pub const REBUILD: &str = "rebuild";
    }
}
pub const VOXEL: &str = graph::NAME;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RenderGraphSettings::default())
            .add_plugins(ExtractResourcePlugin::<RenderGraphSettings>::default())
            .add_plugins(AttachmentsPlugin)
            .add_plugins(VoxelWorldPlugin)
            .add_plugins(TracePlugin)
            .add_plugins(DenoisePlugin)
            .add_plugins(ComputeResourcesPlugin);

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        use graph::node::*;
        render_app
            .add_render_sub_graph(VOXEL)
            .add_render_graph_node::<ViewNodeRunner<StreamNode>>(VOXEL, STREAM)
            //.add_render_graph_node::<ViewNodeRunner<RebuildNode>>(VOXEL, REBUILD)
            .add_render_graph_node::<ViewNodeRunner<TraceNode>>(VOXEL, TRACE)
            .add_render_graph_node::<ViewNodeRunner<DenoiseNode>>(VOXEL, DENOISE)
            .add_render_graph_node::<ViewNodeRunner<TonemappingNode>>(VOXEL, TONEMAPPING)
            .add_render_graph_node::<ViewNodeRunner<FxaaNode>>(VOXEL, FXAA)
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(VOXEL, UPSCALING)
            .add_render_graph_edges(
                VOXEL,
                &[
                    //MIP, REBUILD,
                    STREAM,
                    TRACE,
                    DENOISE,
                    TONEMAPPING,
                    FXAA,
                    UPSCALING,
                ],
            );
    }
}

#[derive(Resource, Clone, ExtractResource)]
pub struct RenderGraphSettings {
    pub clear: bool,
    pub automata: bool,
    pub animation: bool,
    pub voxelization: bool,
    pub rebuild: bool,
    pub mip: bool,
    pub physics: bool,
    pub trace: bool,
    pub denoise: bool,
}

impl Default for RenderGraphSettings {
    fn default() -> Self {
        Self {
            clear: false,
            automata: false,
            animation: false,
            voxelization: false,
            rebuild: false,
            mip: false,
            physics: false,
            trace: true,
            denoise: true,
        }
    }
}

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
            camera_render_graph: CameraRenderGraph::new("voxel"),
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

pub struct BevyVoxelEnginePlugin;
impl Plugin for BevyVoxelEnginePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Off).add_plugins(RenderPlugin);
    }
}

//j this will be handled by an AssetLoader
#[allow(dead_code)]
#[derive(Resource)]
pub enum LoadVoxelWorld {
    Empty(u32),
    File(String),
    Flatland(u32),
    None,
}
