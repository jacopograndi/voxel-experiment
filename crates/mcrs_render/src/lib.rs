use crate::{
    boxes_world::BoxesWorldPlugin,
    pipeline::{
        compute::ComputeResourcesPlugin,
        stream::StreamNode,
        trace::{TraceNode, TracePlugin, TraceSettings},
    },
    voxel_world::VoxelWorldPlugin,
};

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
        render_graph::{RenderGraph, RenderGraphApp, RunGraphOnViewNode, ViewNodeRunner},
        view::VisibleEntities,
        RenderApp,
    },
    ui::{draw_ui_graph, UiPassNode},
};

pub mod block_texture;
pub mod boxes_world;
pub mod pipeline;
pub mod voxel_world;

pub struct RenderPlugin;

pub mod graph {
    pub const NAME: &'static str = "voxel";
    pub mod node {
        pub const TRACE: &str = "trace";
        pub const TONEMAPPING: &str = "tonemapping";
        pub const FXAA: &str = "fxaa";
        pub const UPSCALING: &str = "upscaling";
        pub const STREAM: &str = "stream";
    }
}
pub const VOXEL: &str = graph::NAME;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RenderGraphSettings::default())
            .add_plugins(ExtractResourcePlugin::<RenderGraphSettings>::default())
            .add_plugins(VoxelWorldPlugin)
            .add_plugins(BoxesWorldPlugin)
            .add_plugins(TracePlugin)
            .add_plugins(ComputeResourcesPlugin);

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        use graph::node::*;
        render_app
            .add_render_sub_graph(VOXEL)
            .add_render_graph_node::<ViewNodeRunner<StreamNode>>(VOXEL, STREAM)
            .add_render_graph_node::<ViewNodeRunner<TraceNode>>(VOXEL, TRACE)
            .add_render_graph_node::<ViewNodeRunner<TonemappingNode>>(VOXEL, TONEMAPPING)
            .add_render_graph_node::<ViewNodeRunner<FxaaNode>>(VOXEL, FXAA)
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(VOXEL, UPSCALING)
            .add_render_graph_edges(VOXEL, &[STREAM, TRACE, TONEMAPPING, FXAA, UPSCALING]);

        let ui_pass_node = UiPassNode::new(&mut render_app.world);
        let mut ui_graph = RenderGraph::default();
        ui_graph.add_node(draw_ui_graph::node::UI_PASS, ui_pass_node);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();
        if let Some(graph_voxel) = graph.get_sub_graph_mut(VOXEL) {
            graph_voxel.add_sub_graph(draw_ui_graph::NAME, ui_graph);
            graph_voxel.add_node(
                draw_ui_graph::node::UI_PASS,
                RunGraphOnViewNode::new(draw_ui_graph::NAME),
            );
            graph_voxel.add_node_edge(FXAA, draw_ui_graph::node::UI_PASS);
            graph_voxel.add_node_edge(draw_ui_graph::node::UI_PASS, UPSCALING);
        }
    }
}

#[derive(Resource, Clone, ExtractResource)]
pub struct RenderGraphSettings {
    pub trace: bool,
    pub denoise: bool,
}

impl Default for RenderGraphSettings {
    fn default() -> Self {
        Self {
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

pub struct VoxelRenderPlugin;
impl Plugin for VoxelRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Off).add_plugins(RenderPlugin);
    }
}
