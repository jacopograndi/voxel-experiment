use crate::{
    debug::{DebugOptions, WidgetBlockDebug},
    get_player_from_save, get_single_event,
    hotbar::PlayerHand,
    settings::McrsSettings,
    Level, LevelOwned, LevelReadyEvent, PlayerInput, PlayerInputBuffer, SerdePlayer,
    UniverseChange, UniverseChanges,
};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use mcrs_net::{LocalPlayer, NetworkMode};
use mcrs_physics::{
    character::{CameraController, Character, CharacterController, Friction, Velocity},
    raycast::{cast_ray, RayFinite},
};
use mcrs_render::{camera::VoxelCameraBundle, settings::RenderMode};
use mcrs_universe::{block::Block, universe::Universe, Blueprints};
use std::time::Duration;

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

pub fn terrain_editing(
    camera_query: Query<(&CameraController, &GlobalTransform, &Parent)>,
    mut player_query: Query<(&mut PlayerInputBuffer, &PlayerHand)>,
    universe: Res<Universe>,
    bp: Res<Blueprints>,
    mut gizmos: Gizmos,
    mut contexts: EguiContexts,
    mut hide_red_cube: Local<bool>,
    mut changes: ResMut<UniverseChanges>,
    debug_options: Res<DebugOptions>,
) {
    for (_cam, tr, parent) in camera_query.iter() {
        let Ok((mut input, hand)) = player_query.get_mut(parent.get()) else {
            continue;
        };

        let hit_option = cast_ray(
            RayFinite {
                position: tr.translation(),
                direction: tr.forward().as_vec3(),
                reach: 4.5,
            },
            &universe,
        );

        if let Some(hit) = &hit_option {
            for input in input.buffer.iter() {
                match input {
                    PlayerInput::Placing(true) => {
                        if let Some(block_id) = hand.block_id {
                            changes.queue.push(UniverseChange::Add {
                                pos: hit.grid_pos + hit.normal(),
                                block: Block::new(bp.blocks.get(&block_id)),
                            });
                        }
                    }
                    PlayerInput::Mining(true) => {
                        changes
                            .queue
                            .push(UniverseChange::Remove { pos: hit.grid_pos });
                    }
                    _ => {}
                };
            }
        }

        input.buffer.clear();

        if !debug_options.active {
            return;
        }

        egui::Window::new("Debug Player Raycast Hit")
            .anchor(egui::Align2::LEFT_CENTER, egui::Vec2::new(5.0, 0.0))
            .show(contexts.ctx_mut(), |ui| {
                if let Some(hit) = &hit_option {
                    ui.add(WidgetBlockDebug::new(hit.grid_pos, &universe, &bp));
                    if !*hide_red_cube {
                        ui.add(WidgetBlockDebug::new(
                            hit.grid_pos + hit.normal(),
                            &universe,
                            &bp,
                        ));
                    }
                }
                ui.checkbox(&mut hide_red_cube, "Hide the facing cube in red");
            });

        if let Some(hit) = &hit_option {
            let intersection = hit.final_position();

            gizmos.cuboid(
                Transform::from_translation(intersection).with_scale(Vec3::splat(0.01)),
                Color::BLACK,
            );

            let center_pos = hit.grid_pos.as_vec3() + Vec3::splat(0.5);
            gizmos.cuboid(
                Transform::from_translation(center_pos).with_scale(Vec3::splat(1.001)),
                Color::BLACK,
            );

            if !*hide_red_cube {
                gizmos.cuboid(
                    Transform::from_translation(center_pos + hit.normal().as_vec3())
                        .with_scale(Vec3::splat(1.001)),
                    Color::srgb(1.0, 0.0, 0.0),
                );
                gizmos.arrow(
                    intersection,
                    intersection + hit.normal().as_vec3() * 0.5,
                    Color::srgb(1.0, 0.0, 0.0),
                );
            }
        }
    }
}
