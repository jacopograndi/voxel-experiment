use crate::{
    debug::{DebugOptions, WidgetBlockDebug},
    get_single_event, player, read_player,
    settings::McrsSettings,
    Db, LevelOwned, LevelReady, LevelReadyEvent, Lobby, LocalPlayer, LocalPlayerId,
    NetPlayerSpawned, NetworkMode, Player, PlayerHand, PlayerId, PlayerInput, PlayerInputBuffer,
    PlayersReplica, PlayersState, RemotePlayer, SerdePlayer, ServerChannel, ServerMessages,
    UniverseChange, UniverseChanges,
};
use bevy::{prelude::*, utils::HashMap};
use bevy_egui::{egui, EguiContexts};
use mcrs_physics::{
    character::{CameraController, Character, CharacterController, Friction, Rigidbody, Velocity},
    intersect::intersect_aabb_block,
    raycast::{cast_ray, RayFinite},
};
use mcrs_render::{camera::VoxelCameraBundle, settings::RenderMode};
use mcrs_universe::{
    block::{Block, BlockId},
    universe::Universe,
    Blueprints,
};
use renet::RenetServer;
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

#[derive(Default, Debug, Clone, Resource)]
pub struct LobbySpawnedPlayers {
    pub local_players: HashMap<PlayerId, Entity>,
    pub remote_players: HashMap<PlayerId, Entity>,
}

pub fn apply_players_replica(
    mut commands: Commands,
    query: Query<(Entity, &RemotePlayer, &Children)>,
    mut query_transform: Query<&mut Transform>,
    players_replica: Res<PlayersReplica>,
    mut spawned: ResMut<LobbySpawnedPlayers>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (player_id, replica) in players_replica.players.iter() {
        let rotation_body = Quat::from_axis_angle(Vec3::Y, replica.rotation_body);
        let rotation_head = Quat::from_axis_angle(Vec3::X, replica.rotation_camera);
        if let Some(player_entity) = spawned.remote_players.get(player_id) {
            if let Ok((_, _, children)) = query.get(*player_entity) {
                // update existing replica
                let camera_entity = children.iter().next().unwrap();

                let mut tr = query_transform.get_mut(*player_entity).unwrap();
                tr.translation = replica.position;
                tr.rotation = Quat::from_axis_angle(Vec3::Y, replica.rotation_body);

                let mut tr_camera = query_transform.get_mut(*camera_entity).unwrap();
                tr_camera.rotation = Quat::from_axis_angle(Vec3::X, replica.rotation_camera);
            }
        } else {
            // spawn a new remote player replica
            let serde_player = SerdePlayer {
                name: player_id.name.clone(),
                translation: replica.position,
                body_rotation: rotation_body,
                camera_rotation: rotation_head,
            };
            let entity = spawn_remote_player(
                &mut commands,
                serde_player,
                player_id.clone(),
                &mut meshes,
                &mut materials,
            );
            spawned.remote_players.insert(player_id.clone(), entity);
        }
    }
}

pub fn apply_players_state(
    mut commands: Commands,
    query: Query<(Entity, &RemotePlayer, &Children)>,
    mut query_transform: Query<&mut Transform>,
    mut players_state: ResMut<PlayersState>,
    mut spawned: ResMut<LobbySpawnedPlayers>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut universe_changes: ResMut<UniverseChanges>,
) {
    for (player_id, state) in players_state.players.iter_mut() {
        universe_changes.queue.append(&mut state.universe_changes);

        let rotation_body = Quat::from_axis_angle(Vec3::Y, state.rotation_body);
        let rotation_head = Quat::from_axis_angle(Vec3::X, state.rotation_camera);
        if let Some(player_entity) = spawned.remote_players.get(player_id) {
            if let Ok((_, _, children)) = query.get(*player_entity) {
                // update existing replica
                let camera_entity = children.iter().next().unwrap();

                let mut tr = query_transform.get_mut(*player_entity).unwrap();
                tr.translation = state.position;
                tr.rotation = Quat::from_axis_angle(Vec3::Y, state.rotation_body);

                let mut tr_camera = query_transform.get_mut(*camera_entity).unwrap();
                tr_camera.rotation = Quat::from_axis_angle(Vec3::X, state.rotation_camera);
            } else {
                // spawn a new remote player replica
                let serde_player = SerdePlayer {
                    name: player_id.name.clone(),
                    translation: state.position,
                    body_rotation: rotation_body,
                    camera_rotation: rotation_head,
                };
                let entity = spawn_remote_player(
                    &mut commands,
                    serde_player,
                    player_id.clone(),
                    &mut meshes,
                    &mut materials,
                );
                spawned.remote_players.insert(player_id.clone(), entity);
            }
        }
    }
}

pub fn spawn_players_client(
    mut commands: Commands,
    mut events: EventReader<NetPlayerSpawned>,
    mut spawned: ResMut<LobbySpawnedPlayers>,
    settings: Res<McrsSettings>,
    lobby: Res<Lobby>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for NetPlayerSpawned { id, data } in events.read() {
        if lobby.local_players.contains(id) && !spawned.local_players.contains_key(id) {
            let entity = spawn_local_player(&mut commands, &settings, data.clone(), id.clone());
            spawned.local_players.insert(id.clone(), entity);
        }
        if lobby.remote_players.contains(id) && !spawned.remote_players.contains_key(id) {
            let entity = spawn_remote_player(
                &mut commands,
                data.clone(),
                id.clone(),
                &mut meshes,
                &mut materials,
            );
            spawned.remote_players.insert(id.clone(), entity);
        }
    }

    // Todo: despawn far away players
    // Todo: despawn disconnected players
}

pub fn spawn_players_server(
    mut commands: Commands,
    mut spawned: ResMut<LobbySpawnedPlayers>,
    settings: Res<McrsSettings>,
    lobby: Res<Lobby>,
    level_ready: Option<Res<LevelReady>>,
    db: Option<Res<Db>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut server: ResMut<RenetServer>,
    mut players_state: ResMut<PlayersState>,
    mut players_replica: ResMut<PlayersReplica>,
) {
    let (Some(db), Some(_)) = (db, level_ready.as_ref()) else {
        return;
    };

    for id in lobby.local_players.iter() {
        if !spawned.local_players.contains_key(id) {
            let serde_player = get_or_spawn_player(&db, &id.name);
            let entity = spawn_local_player(&mut commands, &settings, serde_player, id.clone());
            spawned.local_players.insert(id.clone(), entity);
        }
    }

    for id in lobby.remote_players.iter() {
        if !spawned.remote_players.contains_key(id) {
            let serde_player = get_or_spawn_player(&db, &id.name);
            let entity = spawn_remote_player(
                &mut commands,
                serde_player.clone(),
                id.clone(),
                &mut meshes,
                &mut materials,
            );
            spawned.remote_players.insert(id.clone(), entity);

            // Broadcast a player spawned event to every client
            // It is used by the clients to spawn their local player
            let message = bincode::serialize(&ServerMessages::PlayerSpawned {
                id: id.clone(),
                data: serde_player,
            })
            .unwrap();
            server.broadcast_message(ServerChannel::ServerMessages, message);
        }
    }

    let mut to_remove = vec![];
    for (id, entity) in spawned.local_players.iter() {
        if !lobby.local_players.contains(id) {
            commands.entity(*entity).despawn_recursive();
            to_remove.push(id.clone());
        }
    }
    for id in to_remove {
        spawned.local_players.remove(&id);
    }

    let mut to_remove = vec![];
    for (id, entity) in spawned.remote_players.iter() {
        if !lobby.remote_players.contains(id) {
            commands.entity(*entity).despawn_recursive();
            to_remove.push(id.clone());
        }
    }
    for id in to_remove {
        spawned.remote_players.remove(&id);
        players_state.players.remove(&id);
        players_replica.players.remove(&id);
    }
}

pub fn spawn_local_players_on_level_loaded(
    settings: Res<McrsSettings>,
    level_ready_event: EventReader<LevelReadyEvent>,
    local_player_id: Res<LocalPlayerId>,
    mut commands: Commands,
    mut spawned: ResMut<LobbySpawnedPlayers>,
    lobby: Option<ResMut<Lobby>>,
    db: Option<Res<Db>>,
) {
    let Some(_) = get_single_event(level_ready_event) else {
        return;
    };
    let Some(db) = db else {
        return;
    };
    let Some(mut lobby) = lobby else {
        return;
    };

    match settings.network_mode {
        NetworkMode::ClientAndServer => {
            let Some(id) = local_player_id.id.clone() else {
                panic!("No local player name set");
            };
            let serde_player = get_or_spawn_player(&db, &id.name);
            let entity = spawn_local_player(&mut commands, &settings, serde_player, id.clone());
            spawned.local_players.insert(id.clone(), entity);
            lobby.local_players.push(id);
        }
        NetworkMode::Offline => {
            let id = local_player_id
                .id
                .iter()
                .cloned()
                .next()
                .unwrap_or(PlayerId {
                    name: format!("Nameless"),
                });
            let serde_player = get_or_spawn_player(&db, &id.name);
            let entity = spawn_local_player(&mut commands, &settings, serde_player, id.clone());
            spawned.local_players.insert(id.clone(), entity);
        }
        _ => {}
    }
}

pub fn get_or_spawn_player(db: &Db, player_name: &str) -> SerdePlayer {
    match db.get(|tx| read_player(tx, player_name)) {
        Some(p) => {
            info!("Found player in save.");
            p
        }
        None => SerdePlayer {
            name: player_name.to_string(),
            // Todo: find spawnpoint in spawn chunks
            translation: Vec3::ZERO,
            body_rotation: Quat::IDENTITY,
            camera_rotation: Quat::IDENTITY,
        },
    }
}

pub fn spawn_remote_player(
    commands: &mut Commands,
    serde_player: SerdePlayer,
    id: PlayerId,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Entity {
    info!(
        "Spawning remote player for {:?} named {}",
        id, serde_player.name
    );

    commands
        .spawn((
            Transform {
                translation: serde_player.translation,
                rotation: serde_player.body_rotation,
                ..default()
            },
            LevelOwned,
            RemotePlayer { id: id.clone() },
            Player { id: id.clone() },
            Rigidbody {
                size: Vec3::new(0.5, 1.8, 0.5),
            },
            Character {
                air_speed: 0.001,
                ground_speed: 0.03,
                jump_strenght: 0.2,
                jump_cooldown: Duration::from_millis(200),
            },
            Velocity::default(),
            Friction {
                air: Vec3::splat(0.99),
                ground: Vec3::splat(0.78),
            },
            PlayerHand {
                block_id: None,
                hotbar_index: 0,
            },
            Mesh3d(meshes.add(Cuboid::new(0.5, 1.8, 0.5))),
            MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        ))
        .with_children(|parent| {
            parent.spawn((
                Transform::from_xyz(0.0, 0.5, 0.0).with_rotation(serde_player.camera_rotation),
                Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
                MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
            ));
        })
        .id()
}

pub fn spawn_local_player(
    commands: &mut Commands,
    settings: &McrsSettings,
    serde_player: SerdePlayer,
    id: PlayerId,
) -> Entity {
    info!("Spawning local player named {}", serde_player.name);

    commands
        .spawn((
            Transform {
                translation: serde_player.translation,
                rotation: serde_player.body_rotation,
                ..default()
            },
            LevelOwned,
            LocalPlayer { id: id.clone() },
            Player { id: id.clone() },
            PlayerInputBuffer::default(),
            Rigidbody {
                size: Vec3::new(0.5, 1.8, 0.5),
            },
            Character {
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
            PlayerHand {
                block_id: Some(BlockId::from(1)),
                hotbar_index: 0,
            },
        ))
        .with_children(|parent| {
            let camera_pivot = parent.spawn((
                CameraController::default(),
                Transform::from_xyz(0.0, 0.5, 0.0).with_rotation(serde_player.camera_rotation),
            ));
            spawn_camera(camera_pivot, &settings);
        })
        .id()
}

#[derive(Resource, Default, Clone, Debug)]
pub struct PlayerUniverseChanges {
    pub queue: Vec<UniverseChange>,
}

pub fn terrain_editing(
    camera_query: Query<(&CameraController, &GlobalTransform, &Parent)>,
    mut player_query: Query<(&mut PlayerInputBuffer, &PlayerHand, &Transform, &Rigidbody)>,
    universe: Res<Universe>,
    bp: Res<Blueprints>,
    mut universe_changes: ResMut<UniverseChanges>,
    mut player_changes: ResMut<PlayerUniverseChanges>,
) {
    let mut changes = vec![];
    for (_cam, tr, parent) in camera_query.iter() {
        let Ok((mut input, hand, tr_player, rigidbody)) = player_query.get_mut(parent.get()) else {
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
                            let block_pos = hit.grid_pos + hit.normal();
                            if !intersect_aabb_block(
                                tr_player.translation,
                                rigidbody.size,
                                block_pos,
                            ) {
                                changes.push(UniverseChange::Add {
                                    pos: hit.grid_pos + hit.normal(),
                                    block: Block::new(bp.blocks.get(&block_id)),
                                });
                            }
                        }
                    }
                    PlayerInput::Mining(true) => {
                        changes.push(UniverseChange::Remove { pos: hit.grid_pos });
                    }
                    _ => {}
                };
            }
        }

        input.buffer.clear();
    }

    universe_changes.queue.extend(changes.iter().cloned());
    player_changes.queue.extend(changes.iter().cloned());
}
