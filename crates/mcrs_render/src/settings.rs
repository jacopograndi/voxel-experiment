use bevy::{prelude::*, render::extract_resource::ExtractResource};

pub const DEFAULT_VIEW_DISTANCE: u32 = 64;

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct RenderSettings {
    pub view_distance_blocks: u32,
    pub render_mode: RenderMode,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            view_distance_blocks: DEFAULT_VIEW_DISTANCE,
            render_mode: RenderMode::default(),
        }
    }
}

#[derive(Resource, Clone, ExtractResource, PartialEq, Eq, Debug)]
pub enum RenderMode {
    RasterizeOnly,
    RaytraceOnly,
    RaytraceThenRasterize,
}

impl RenderMode {
    pub fn is_raytrace_active(&self) -> bool {
        match self {
            RenderMode::RasterizeOnly => false,
            _ => true,
        }
    }
    pub fn is_rasterize_active(&self) -> bool {
        match self {
            RenderMode::RaytraceOnly => false,
            _ => true,
        }
    }
}

impl Default for RenderMode {
    fn default() -> Self {
        Self::RaytraceOnly
    }
}

impl From<Option<String>> for RenderMode {
    fn from(render_mode: Option<String>) -> RenderMode {
        match render_mode {
            None => Self::default(),
            Some(s) => {
                match s.as_str() {
                    "rasterize" => RenderMode::RasterizeOnly,
                    "raytrace" => RenderMode::RaytraceOnly,
                    "mixed" => RenderMode::RaytraceThenRasterize,
                    _ => panic!("Use \"rasterize\" for rasterization only, \"raytrace\" for raytrace only, \"mixed\" for raytrace then rasterization, leave blank for default"),
                }
            },
        }
    }
}

#[derive(Resource, Clone, ExtractResource)]
pub struct RenderGraphSettings {
    pub trace: bool,
}

impl Default for RenderGraphSettings {
    fn default() -> Self {
        Self { trace: true }
    }
}
