use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::{camera::CameraRenderGraph, primitives::Frustum, view::VisibleEntities},
};

use crate::voxel_pipeline::{trace::TraceSettings, RenderPlugin};

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
            camera_render_graph: CameraRenderGraph::new("voxel"),
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

pub struct BevyVoxelEnginePlugin;
impl Plugin for BevyVoxelEnginePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa::Off).add_plugins(RenderPlugin);
    }
}

#[derive(Resource)]
pub enum LoadVoxelWorld {
    Empty(u32),
    File(String),
    None,
}

// Grid Hierarchy
#[derive(Clone)]
pub struct GH {
    pub levels: [u32; 8],
    pub texture_size: u32,
    pub texture_data: Vec<u8>,
    pub pallete: Pallete,
}

#[derive(Clone, Deref, DerefMut)]
pub struct Pallete([[f32; 4]; 256]);

impl GH {
    pub fn empty(texture_size: u32) -> Self {
        let mut levels = [0; 8];
        let i = texture_size.trailing_zeros() - 3;
        for i in 0..i {
            levels[i as usize] = 1 << (i + 3);
        }

        Self {
            levels,
            texture_size,
            texture_data: vec![0; (texture_size * texture_size * texture_size * 2) as usize],
            pallete: Pallete([[0.0; 4]; 256]),
        }
    }

    pub fn get_offsets(&self) -> [u32; 8] {
        let mut offsets = [0; 8];
        let mut last = 0;
        for i in 0..8 {
            offsets[i] = last;
            last = last + self.levels[i] as u32 * self.levels[i] as u32 * self.levels[i] as u32;
        }
        offsets
        //[src/voxel_engine.rs:91] offsets = [
        //   0,
        //   512,
        //   4608,
        //   37376,
        //   299520,
        //   299520,
        //   299520,
        //   299520,
        // ]
    }

    pub fn get_buffer_size_from_levels(levels: &[u32; 8]) -> usize {
        let mut length = 0;
        for i in 0..8 {
            length += levels[i] * levels[i] * levels[i];
        }
        length as usize / 8
    }

    pub fn get_buffer_size(&self) -> usize {
        Self::get_buffer_size_from_levels(&self.levels)
    }

    pub fn from_vox(file: &[u8]) -> Result<GH, String> {
        let vox = dot_vox::load_bytes(file)?;
        let size = vox.models[0].size;
        if size.x != size.y || size.x != size.z || size.y != size.z {
            return Err("Voxel model is not a cube!".to_string());
        }

        let size = size.x as usize;

        let mut gh = GH::empty(size as u32);
        for i in 0..256 {
            let colour = vox.palette[i];
            let mut material = Vec4::new(
                colour.r as f32 / 255.0,
                colour.g as f32 / 255.0,
                colour.b as f32 / 255.0,
                0.0,
            );
            material = material.powf(2.2);

            if let Some(vox_material) = vox.materials.get(i) {
                let vox_material = vox_material.properties.clone();
                if vox_material["_type"] == "_emit" {
                    material *= 1.0 + vox_material["_emit"].parse::<f32>().unwrap();
                    if vox_material.contains_key("_flux") {
                        material = material.powf(vox_material["_flux"].parse::<f32>().unwrap());
                    }
                    material.w = 1.0;
                }
            }

            gh.pallete[i] = material.to_array();
        }

        for voxel in &vox.models[0].voxels {
            let pos = IVec3::new(
                size as i32 - 1 - voxel.x as i32,
                voxel.z as i32,
                voxel.y as i32,
            );

            let index = pos.x as usize * size * size + pos.y as usize * size + pos.z as usize;
            gh.texture_data[index as usize * 2] = voxel.i;
            gh.texture_data[index as usize * 2 + 1] = 16; // set the collision flag
        }

        dbg!(gh.levels);
        //[src/voxel_engine.rs:76] levels = [
        //   8,
        //   16,
        //   32,
        //   64,
        //   128,
        //   0,
        //   0,
        //   0,
        // ]

        Ok(gh)
    }
}
