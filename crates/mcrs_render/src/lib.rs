pub mod block_texture;
pub mod boxes_world;
pub mod camera;
pub mod pipeline;
pub mod plugin;
pub mod settings;
pub mod voxel_world;
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
