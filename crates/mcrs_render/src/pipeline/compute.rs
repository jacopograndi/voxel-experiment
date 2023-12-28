use bevy::{asset::load_internal_asset, prelude::*, render::RenderApp};

use crate::pipeline::stream;

pub const STREAM_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(6189143918759879663);
pub const REBUILD_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(18135969847573717619);

pub struct ComputeResourcesPlugin;

impl Plugin for ComputeResourcesPlugin {
    fn build(&self, _app: &mut App) {}
    fn finish(&self, app: &mut App) {
        load_internal_asset!(
            app,
            STREAM_SHADER_HANDLE,
            "shaders/stream.wgsl",
            Shader::from_wgsl
        );

        app.sub_app_mut(RenderApp)
            .init_resource::<stream::Pipeline>();
    }
}
