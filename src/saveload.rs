use std::{fs, path::PathBuf};

use bevy::prelude::*;
use mcrs_physics::{
    character::{CameraController, Character},
    TickStep,
};
use mcrs_universe::{chunk::Chunk, universe::Universe};
use ron::{de::SpannedError, ser::PrettyConfig};
use serde::{Deserialize, Serialize};

use crate::{
    settings::McrsSettings,
    terrain::{get_spawn_chunks, UniverseChanges},
    FixedMainSet,
};

pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<OpenLevelEvent>()
            .add_event::<CloseLevelEvent>()
            .add_event::<SaveLevelEvent>()
            .add_event::<LevelReadyEvent>()
            .add_systems(
                FixedUpdate,
                (open_level, save_level, close_level, is_level_ready)
                    .chain()
                    .in_set(FixedMainSet::SaveLoad),
            )
            .add_systems(Startup, auto_open_level);
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    pub name: String,
    pub seed: u32,
}

#[derive(Resource, Debug, Clone)]
pub struct LevelReady;

/// Every entity owned by the level must be marked with this component.
/// They will be destroyed when the level is closed.
#[derive(Component, Debug, Clone)]
pub struct LevelOwned;

#[derive(Event, Debug, Clone)]
pub struct OpenLevelEvent {
    pub level_name: String,
}

#[derive(Event, Debug, Clone)]
pub struct CloseLevelEvent;

#[derive(Event, Debug, Clone)]
pub struct SaveLevelEvent;

#[derive(Event, Debug, Clone)]
pub struct LevelReadyEvent;

pub fn auto_open_level(mut event_writer: EventWriter<OpenLevelEvent>, settings: Res<McrsSettings>) {
    event_writer.send(OpenLevelEvent {
        level_name: settings.open_level_name.clone(),
    });
}

pub fn open_level(
    event_reader: EventReader<OpenLevelEvent>,
    mut commands: Commands,
    existing_level: Option<Res<Level>>,
    mut tickstep: ResMut<TickStep>,
) {
    let Some(event) = get_single_event(event_reader) else {
        return;
    };

    if existing_level.is_some() {
        warn!("Another level is loaded");
        return;
    }

    if event.level_name.is_empty() {
        warn!("Level name is empty");
        return;
    }

    commands.insert_resource(Level {
        name: event.level_name,
        seed: 23,
    });

    *tickstep = TickStep::Tick;
}

pub fn close_level(
    event_reader: EventReader<CloseLevelEvent>,
    mut commands: Commands,
    existing_level: Option<Res<Level>>,
    mut universe: ResMut<Universe>,
    mut universe_changes: ResMut<UniverseChanges>,
    mut tickstep: ResMut<TickStep>,
    level_owned_query: Query<(Entity, &LevelOwned)>,
) {
    let Some(_) = get_single_event(event_reader) else {
        return;
    };

    if existing_level.is_none() {
        warn!("No level is loaded");
        return;
    }

    commands.remove_resource::<Level>();
    commands.remove_resource::<LevelReady>();

    universe.chunks.clear();
    universe.heightfield.clear();
    universe_changes.queue.clear();
    *tickstep = TickStep::STOP;

    for (entity, _) in level_owned_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn get_save_path() -> Option<PathBuf> {
    const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

    let mut path = dirs::data_dir()?;
    path.push(CRATE_NAME);
    let _ = fs::create_dir_all(path.clone());
    Some(path)
}

pub fn write_to_file<T: Serialize>(
    path: PathBuf,
    value: T,
    ron_config: PrettyConfig,
) -> Result<(), ()> {
    let level_ron = ron::ser::to_string_pretty(&value, ron_config);
    let Ok(level_ron) = level_ron else {
        error!("Failed to serialize level:\n {:?}", level_ron);
        return Err(());
    };
    if let Err(err) = fs::write(path.clone(), level_ron) {
        error!("Failed to write to {:?}:\n {:?}", path, err);
        return Err(());
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerdePlayer {
    pub name: String,
    pub translation: Vec3,
    pub body_rotation: Quat,
    pub camera_rotation: Quat,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerdePlayers {
    pub players: Vec<SerdePlayer>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerdeChunk {
    chunk: Vec<u8>,
}

pub fn save_level(
    event_reader: EventReader<SaveLevelEvent>,
    level: Option<Res<Level>>,
    universe: Res<Universe>,
    players_query: Query<&Transform, With<Character>>,
    camera_query: Query<(&Transform, &Parent), With<CameraController>>,
) {
    let Some(_) = get_single_event(event_reader) else {
        return;
    };

    let Some(level) = level.as_ref() else {
        warn!("There is no level to save");
        return;
    };

    let Some(mut path) = get_save_path() else {
        warn!("Couldn't find the path to save the level");
        return;
    };

    let ron_config = PrettyConfig::default().compact_arrays(true).depth_limit(2);

    path.push(level.name.as_str());
    let _ = fs::create_dir_all(path.clone());

    info!("saving to: {:?}", path);

    let Ok(_) = write_to_file(path.join("level.ron"), &**level, ron_config.clone()) else {
        return;
    };

    let mut players = SerdePlayers { players: vec![] };
    for (camera_tr, parent) in camera_query.iter() {
        let body_tr = players_query
            .get(parent.get())
            .expect("Players should have a body");
        players.players.push(SerdePlayer {
            // Todo: handle player names
            name: "Nameless".to_string(),
            translation: body_tr.translation,
            body_rotation: body_tr.rotation,
            camera_rotation: camera_tr.rotation,
        });
    }

    let Ok(_) = write_to_file(path.join("players.ron"), &players, ron_config.clone()) else {
        return;
    };

    let blocks_path = path.join("blocks");
    let _ = fs::create_dir_all(blocks_path.clone());

    for (chunk_pos, chunk) in universe.chunks.iter() {
        let file_name = format!("chunk_{}_{}_{}.bin", chunk_pos.x, chunk_pos.y, chunk_pos.z);
        let file_path = blocks_path.join(file_name);
        let chunk_ref = chunk.get_ref();
        let block_bytes: &[u8] = bytemuck::cast_slice(chunk_ref.as_ref());
        if let Err(err) = fs::write(file_path.clone(), block_bytes) {
            error!("Failed to write to {:?}:\n {:?}", file_path, err);
            continue;
        }
    }

    let entities_path = path.join("entities");
    let _ = fs::create_dir_all(entities_path.clone());

    info!("save successful");
}

pub fn get_player_from_save(player_name: &str, level_name: &str) -> Option<SerdePlayer> {
    let Some(mut path) = get_save_path() else {
        warn!("Couldn't find the path of the save directory");
        return None;
    };
    path.push(level_name);
    path.push("players.ron");
    let Ok(players_ron) = fs::read_to_string(path) else {
        warn!("Couldn't read players.ron");
        return None;
    };

    let players: Result<SerdePlayers, SpannedError> = ron::from_str(&players_ron);
    let Ok(players) = players else {
        warn!("Parse error of players.ron: \n {:?}", players);
        return None;
    };

    players
        .players
        .iter()
        .find(|p| p.name == player_name)
        .cloned()
}

pub fn get_chunk_from_save(chunk_pos: &IVec3, level_name: &str) -> Option<Chunk> {
    let Some(mut path) = get_save_path() else {
        warn!("Couldn't find the path of the save directory");
        return None;
    };
    path.push(level_name);
    path.push("blocks");
    let file_name = format!("chunk_{}_{}_{}.bin", chunk_pos.x, chunk_pos.y, chunk_pos.z);
    path.push(file_name);
    let Ok(block_bytes) = fs::read(path) else {
        return None;
    };

    let chunk = Chunk::empty();
    {
        let mut write = chunk.get_mut();
        let bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut (*write));
        bytes.copy_from_slice(block_bytes.as_slice());
    }
    Some(chunk)
}

pub fn is_level_ready(
    mut commands: Commands,
    mut event: EventWriter<LevelReadyEvent>,
    universe: Res<Universe>,
    level: Option<ResMut<Level>>,
    level_ready: Option<ResMut<LevelReady>>,
) {
    if level.is_some()
        && level_ready.is_none()
        && get_spawn_chunks().all(|pos| universe.chunks.contains_key(&pos))
    {
        commands.insert_resource(LevelReady);
        event.send(LevelReadyEvent);
    }
}

/// Helper function to run only one level operation per type per tick.
pub fn get_single_event<T: Event + Clone + std::fmt::Debug>(
    mut event_reader: EventReader<T>,
) -> Option<T> {
    let Some(event) = event_reader.read().next() else {
        return None;
    };
    let event = event.clone();
    if !event_reader.is_empty() {
        info!(
            "Too many events of type {:?}, only one will be executed per tick (remaining: {}).",
            event,
            event_reader.len()
        );
        event_reader.clear();
    }
    return Some(event.clone());
}
