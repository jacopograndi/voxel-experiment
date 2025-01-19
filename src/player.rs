use std::time::Duration;

use bevy::prelude::*;
use mcrs_net::{LocalPlayer, NetworkMode};
use mcrs_physics::character::{
    CameraController, Character, CharacterController, Friction, Velocity,
};
use mcrs_render::{camera::VoxelCameraBundle, settings::RenderMode};

use crate::{
    get_player_from_save, get_single_event, hotbar::PlayerHand, settings::McrsSettings, Level,
    LevelOwned, LevelReadyEvent, PlayerInputBuffer, SerdePlayer,
};

pub fn spawn_camera(mut camera_pivot: EntityCommands, settings: &McrsSettings) {
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
            });
        }
    }
}

pub fn spawn_player(
    mut commands: Commands,
    settings: Res<McrsSettings>,
    level_ready_event: EventReader<LevelReadyEvent>,
    level: Option<Res<Level>>,
) {
    if !matches!(settings.network_mode, NetworkMode::ClientAndServer) {
        return;
    }

    let Some(level) = level.as_ref() else {
        return;
    };

    let Some(_) = get_single_event(level_ready_event) else {
        return;
    };

    info!("Spawning local player");

    // Todo: handle player names
    let serde_player = match get_player_from_save("Nameless", &level.name) {
        Some(p) => {
            info!("Found player in save.");
            p
        }
        None => SerdePlayer {
            name: "Nameless".to_string(),
            // Todo: find spawnpoint in spawn chunks
            translation: Vec3::ZERO,
            body_rotation: Quat::IDENTITY,
            camera_rotation: Quat::IDENTITY,
        },
    };

    commands
        .spawn((
            Transform {
                translation: serde_player.translation,
                rotation: serde_player.body_rotation,
                ..default()
            },
            LevelOwned,
            LocalPlayer,
            PlayerInputBuffer::default(),
            Character {
                size: Vec3::new(0.5, 1.8, 0.5),
                air_speed: 0.001,
                ground_speed: 0.03,
                jump_strenght: 0.2,
                jump_cooldown: Duration::from_millis(200),
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
            let camera_pivot = parent.spawn((
                CameraController::default(),
                Transform::from_xyz(0.0, 0.5, 0.0).with_rotation(serde_player.camera_rotation),
            ));
            spawn_camera(camera_pivot, &settings);
        });
}
