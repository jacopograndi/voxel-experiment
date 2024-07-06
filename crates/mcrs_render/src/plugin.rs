use crate::{
    boxes_world::BoxesWorldPlugin,
    graph::{NodeVoxel, Voxel},
    pipeline::{
        compute::ComputeResourcesPlugin,
        stream::StreamNode,
        trace::{TraceNode, TracePlugin},
    },
    settings::RenderGraphSettings,
    voxel_world::VoxelWorldPlugin,
};
use bevy::{
    core_pipeline::{fxaa::FxaaNode, tonemapping::TonemappingNode, upscaling::UpscalingNode},
    prelude::*,
    render::{
        extract_resource::ExtractResourcePlugin,
        render_graph::{RenderGraph, RenderGraphApp, RunGraphOnViewNode, ViewNodeRunner},
        Extract, RenderApp,
    },
    ui::{
        graph::{NodeUi, SubGraphUi},
        UiPassNode,
    },
};

pub struct McrsVoxelRenderPlugin;

impl Plugin for McrsVoxelRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Off);
        app.insert_resource(RenderGraphSettings::default())
            .add_plugins(ExtractResourcePlugin::<RenderGraphSettings>::default())
            .add_plugins(VoxelWorldPlugin)
            .add_plugins(BoxesWorldPlugin)
            .add_plugins(TracePlugin)
            .add_plugins(ComputeResourcesPlugin);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            error!("The render subapp doesn't exist.");
            return;
        };

        // Voxel render graph, substituting the core3d graph
        render_app
            .add_render_sub_graph(Voxel)
            .add_render_graph_node::<ViewNodeRunner<StreamNode>>(Voxel, NodeVoxel::Stream)
            .add_render_graph_node::<ViewNodeRunner<TraceNode>>(Voxel, NodeVoxel::Trace)
            .add_render_graph_node::<ViewNodeRunner<TonemappingNode>>(Voxel, NodeVoxel::Tonemapping)
            .add_render_graph_node::<ViewNodeRunner<FxaaNode>>(Voxel, NodeVoxel::Fxaa)
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(Voxel, NodeVoxel::Upscaling)
            .add_render_graph_edges(
                Voxel,
                (
                    NodeVoxel::Stream,
                    NodeVoxel::Trace,
                    NodeVoxel::Tonemapping,
                    NodeVoxel::Fxaa,
                    NodeVoxel::Upscaling,
                ),
            );

        let ui_pass_node = UiPassNode::new(render_app.world_mut());
        let mut ui_graph = RenderGraph::default();
        ui_graph.add_node(NodeUi::UiPass, ui_pass_node);
        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();

        if let Some(graph_voxel) = graph.get_sub_graph_mut(Voxel) {
            graph_voxel.add_sub_graph(SubGraphUi, ui_graph);
            graph_voxel.add_node(NodeUi::UiPass, RunGraphOnViewNode::new(SubGraphUi));
            graph_voxel.add_node_edge(NodeVoxel::Fxaa, NodeUi::UiPass);
            graph_voxel.add_node_edge(NodeUi::UiPass, NodeVoxel::Upscaling);
        }

        render_app
            .insert_resource(RenderSettings::default())
            .add_systems(ExtractSchedule, extract_render_settings);
    }
}

pub fn extract_render_settings(
    settings: Extract<Res<RenderSettings>>,
    mut render_settings: ResMut<RenderSettings>,
) {
    *render_settings = settings.clone();
}

pub const DEFAULT_VIEW_DISTANCE: u32 = 64;

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct RenderSettings {
    pub view_distance_blocks: u32,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            view_distance_blocks: DEFAULT_VIEW_DISTANCE,
        }
    }
}
