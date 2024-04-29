use bevy::prelude::*;
use mcrs_blueprints::Blueprints;
use mcrs_net::{LocalPlayer, NewPlayerSpawned};
use mcrs_physics::character::{
    CameraController, Character, CharacterController, Friction, Velocity,
};
use mcrs_render::{
    boxes_world::{Ghost, LoadedVoxTextures},
    camera::VoxelCameraBundle,
};

use crate::hotbar::PlayerHand;

pub fn spawn_player(
    mut commands: Commands,
    query: Query<(Entity, &NewPlayerSpawned, Option<&LocalPlayer>)>,
    loaded_textures: Option<Res<LoadedVoxTextures>>,
    info: Res<Blueprints>,
) {
    for (entity, _, local_player) in query.iter() {
        let mut player = commands.entity(entity);
        player.remove::<NewPlayerSpawned>();
        player.insert((
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
        ));
        player.with_children(|parent| {
            let mut camera_pivot = parent.spawn(CameraController::default());
            if local_player.is_some() {
                camera_pivot.insert(VoxelCameraBundle {
                    transform: Transform::from_xyz(0.0, 0.5, 0.0),
                    projection: Projection::Perspective(PerspectiveProjection {
                        fov: 1.57,
                        ..default()
                    }),
                    ..default()
                });
            } else {
                camera_pivot.insert(SpatialBundle {
                    transform: Transform::from_xyz(0.0, 0.5, 0.0),
                    ..default()
                });
            }
        });
        if local_player.is_none() {
            if let Some(loaded_textures) = loaded_textures.as_ref() {
                player.with_children(|parent| {
                    parent.spawn((
                        SpatialBundle::from_transform(Transform {
                            scale: Vec3::new(16.0, 32.0, 8.0) / 16.0,
                            ..default()
                        }),
                        Ghost {
                            vox_texture_index: loaded_textures
                                .ghosts_id
                                .get(&info.ghosts.get_named("Steve").id)
                                .unwrap()
                                .clone(),
                        },
                    ));
                });
            }
        }
    }
}
