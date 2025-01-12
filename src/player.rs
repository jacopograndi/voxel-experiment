use bevy::prelude::*;
use mcrs_net::{LocalPlayer, NewPlayerSpawned};
use mcrs_physics::character::{
    CameraController, Character, CharacterController, Friction, Velocity,
};
use mcrs_render::{
    boxes_world::{Ghost, LoadedVoxTextures},
    camera::VoxelCameraBundle,
    settings::RenderMode,
};
use mcrs_universe::Blueprints;

use crate::{hotbar::PlayerHand, settings::McrsSettings, PlayerInputBuffer};

pub fn spawn_player(
    mut commands: Commands,
    query: Query<(Entity, &NewPlayerSpawned, Option<&LocalPlayer>)>,
    loaded_textures: Option<Res<LoadedVoxTextures>>,
    info: Res<Blueprints>,
    settings: Res<McrsSettings>,
) {
    for (player_entity, _, local_player) in query.iter() {
        commands
            .entity(player_entity)
            .remove::<NewPlayerSpawned>()
            .insert((
                PlayerInputBuffer::default(),
                Character {
                    size: Vec3::new(0.5, 1.8, 0.5),
                    air_speed: 0.001,
                    ground_speed: 0.03,
                    jump_strenght: 0.2,
                },
                CharacterController {
                    acceleration: Vec3::splat(0.0),
                    jumping: false,
                    ..default()
                },
                Velocity::default(),
                Friction {
                    air: Vec3::splat(0.99),
                    ground: Vec3::splat(0.78),
                },
                PlayerHand { block_id: None },
            ))
            .with_children(|parent| {
                let mut camera_pivot = parent.spawn((
                    CameraController::default(),
                    Transform::from_xyz(0.0, 0.5, 0.0),
                ));
                if local_player.is_some() {
                    let projection = Projection::Perspective(PerspectiveProjection {
                        fov: 1.57,
                        ..default()
                    });
                    match settings.render_mode {
                        RenderMode::RasterizeOnly => {
                            camera_pivot.with_children(|pivot| {
                                pivot.spawn((projection, Camera3d::default(), Msaa::Off));
                            });
                        }
                        RenderMode::RaytraceOnly => {
                            camera_pivot.with_children(|pivot| {
                                pivot.spawn((
                                    VoxelCameraBundle {
                                        transform: Transform::from_xyz(0.0, 0.5, 0.0),
                                        projection,
                                        ..default()
                                    },
                                    Msaa::Off,
                                ));
                            });
                        }
                        RenderMode::RaytraceThenRasterize => {
                            camera_pivot.with_children(|pivot| {
                                pivot.spawn((
                                    VoxelCameraBundle {
                                        transform: Transform::from_xyz(0.0, 0.5, 0.0),
                                        projection: projection.clone(),
                                        camera: Camera {
                                            order: 0,
                                            ..default()
                                        },
                                        ..default()
                                    },
                                    Msaa::Off,
                                ));
                                pivot.spawn((
                                    projection,
                                    Camera {
                                        order: 1,
                                        clear_color: ClearColorConfig::None,
                                        ..default()
                                    },
                                    Camera3d {
                                        // todo: enable this to load the voxel's camera depth buffer
                                        //depth_load_op: bevy::core_pipeline::core_3d::Camera3dDepthLoadOp::Load,
                                        ..default()
                                    },
                                    Msaa::Off,
                                ));
                                /*
                                todo!(
                                    "very low fps, probabily due to mcrs_render that is not made for two cameras/view targets"
                                );
                                */
                            });
                        }
                    }
                } else {
                    if let Some(loaded_textures) = loaded_textures.as_ref() {
                        parent.spawn((
                            Transform {
                                scale: Vec3::new(16.0, 32.0, 8.0) / 16.0,
                                ..default()
                            },
                            Ghost {
                                vox_texture_index: loaded_textures
                                    .ghosts_id
                                    .get(&info.ghosts.get_named("Steve").id)
                                    .unwrap()
                                    .clone(),
                            },
                        ));
                    }
                }
            });
    }
}
