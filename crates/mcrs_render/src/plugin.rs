
use bevy::{
    core_pipeline::{
        fxaa::FxaaNode,
        tonemapping::TonemappingNode,
        upscaling::UpscalingNode,
    },
    prelude::*,
    render::{
        extract_resource::ExtractResourcePlugin,
        render_graph::{RenderGraph, RenderGraphApp, RunGraphOnViewNode, ViewNodeRunner},
        RenderApp,
    },
    ui::{draw_ui_graph, UiPassNode},
};

use crate::{settings::RenderGraphSettings, pipeline::{compute::ComputeResourcesPlugin, trace::{TracePlugin, TraceNode}, stream::StreamNode}, boxes_world::BoxesWorldPlugin, voxel_world::VoxelWorldPlugin, graph, VOXEL};

pub struct VoxelRenderPlugin;

impl Plugin for VoxelRenderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Off);
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