use crate::voxel_pipeline::{
    compute::ComputeResourcesPlugin, denoise::DenoisePlugin, mip::MipNode, rebuild::RebuildNode,
};

use self::{
    attachments::AttachmentsPlugin,
    denoise::DenoiseNode,
    trace::{TraceNode, TracePlugin},
    voxel_world::VoxelWorldPlugin,
};
use bevy::{
    core_pipeline::{fxaa::FxaaNode, tonemapping::TonemappingNode, upscaling::UpscalingNode},
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_graph::{RenderGraphApp, ViewNodeRunner},
        RenderApp,
    },
};

pub mod attachments;
pub mod compute;
pub mod denoise;
pub mod mip;
pub mod rebuild;
pub mod trace;
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
        pub const MIP: &str = "mip";
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
            .add_render_graph_node::<ViewNodeRunner<MipNode>>(VOXEL, MIP)
            .add_render_graph_node::<ViewNodeRunner<RebuildNode>>(VOXEL, REBUILD)
            .add_render_graph_node::<ViewNodeRunner<TraceNode>>(VOXEL, TRACE)
            .add_render_graph_node::<ViewNodeRunner<DenoiseNode>>(VOXEL, DENOISE)
            .add_render_graph_node::<ViewNodeRunner<TonemappingNode>>(VOXEL, TONEMAPPING)
            .add_render_graph_node::<ViewNodeRunner<FxaaNode>>(VOXEL, FXAA)
            .add_render_graph_node::<ViewNodeRunner<UpscalingNode>>(VOXEL, UPSCALING)
            .add_render_graph_edges(
                VOXEL,
                &[MIP, REBUILD, TRACE, DENOISE, TONEMAPPING, FXAA, UPSCALING],
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
            rebuild: true,
            mip: true,
            physics: false,
            trace: true,
            denoise: false,
        }
    }
}
