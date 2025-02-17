use crate::{
    boxes_world::BoxesWorldPlugin,
    chunk_mesh::{load_texture, sync_chunk_meshes, ChunkEntities, TextureHandles},
    graph::{NodeVoxel, Voxel},
    pipeline::{
        compute::ComputeResourcesPlugin,
        stream::StreamNode,
        trace::{TraceNode, TracePlugin},
    },
    settings::{RenderGraphSettings, RenderSettings},
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
        let settings = app
            .world()
            .get_resource::<RenderSettings>()
            .unwrap()
            .clone();

        if settings.render_mode.is_raytrace_active() {
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
                .add_render_graph_node::<ViewNodeRunner<TonemappingNode>>(
                    Voxel,
                    NodeVoxel::Tonemapping,
                )
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
        }

        if settings.render_mode.is_rasterize_active() {
            app.insert_resource(TextureHandles::default());
            app.insert_resource(ChunkEntities::default());
            app.insert_resource(AmbientLight {
                brightness: 4000.0,
                ..default()
            });
            app.add_systems(Startup, load_texture);
            app.add_systems(Update, sync_chunk_meshes);
        }

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            error!("The render subapp doesn't exist.");
            return;
        };
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
