pub mod block_texture;
pub mod boxes_world;
pub mod camera;
pub mod pipeline;
pub mod plugin;
pub mod settings;
pub mod voxel_world;

pub mod graph {
    use bevy::render::render_graph::{RenderLabel, RenderSubGraph};

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderSubGraph)]
    pub struct Voxel;

    pub mod input {
        pub const VIEW_ENTITY: &str = "view_entity";
    }

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum NodeVoxel {
        Trace,
        Tonemapping,
        Fxaa,
        Upscaling,
        Stream,
    }
}
