use bevy::{ecs::system::Resource, render::extract_resource::ExtractResource};

#[derive(Resource, Clone, ExtractResource)]
pub struct RenderGraphSettings {
    pub trace: bool,
}

impl Default for RenderGraphSettings {
    fn default() -> Self {
        Self { trace: true }
    }
}
