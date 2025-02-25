use crate::{
    settings::McrsSettings,
    terrain::{get_spawn_chunks, UniverseChanges},
    FixedMainSet, LightSources, SunBeam, SunBeams,
};
use bevy::{prelude::*, utils::HashSet};
use bytemuck::{Pod, Zeroable};
use mcrs_physics::{
    character::{CameraController, Character},
    TickStep,
};
use mcrs_universe::{chunk::Chunk, universe::Universe, CHUNK_AREA, CHUNK_SIDE, CHUNK_VOLUME};
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec_with_limit};
use redb::{Database, Error, ReadTransaction, Table, TableDefinition, WriteTransaction};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

pub const TABLE_BLOCKS: TableDefinition<[i32; 3], &[u8]> = TableDefinition::new("blocks");
pub const TABLE_SUN_BEAMS: TableDefinition<[i32; 2], &[u8]> = TableDefinition::new("sun_beams");
pub const TABLE_PLAYERS: TableDefinition<&str, &[u8]> = TableDefinition::new("players");
pub const TABLE_LEVEL: TableDefinition<&str, &[u8]> = TableDefinition::new("level");

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

#[derive(Resource)]
pub struct Db {
    db: Database,
}

// Todo: make a new error that allows removal of .expect

impl Db {
    pub fn write<F>(&self, f: F) -> Result<(), Error>
    where
        F: FnOnce(&WriteTransaction) -> Result<(), Error>,
    {
        let write_txn = self.db.begin_write()?;
        {
            f(&write_txn)?
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&ReadTransaction) -> Option<R>,
    {
        let read_txn = self.db.begin_read().ok()?;
        let opt = f(&read_txn);
        drop(read_txn);
        opt
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
    mut tickstep: ResMut<TickStep>,
    existing_level: Option<Res<Level>>,
    existing_db: Option<Res<Db>>,
) {
    let Some(event) = get_single_event(event_reader) else {
        return;
    };

    if existing_level.is_some() {
        warn!("Another level is loaded");
        return;
    }

    if existing_db.is_some() {
        warn!("Another db is open");
        panic!();
    }

    if event.level_name.is_empty() {
        warn!("Level name is empty");
        return;
    }

    let path = get_save_path().map_or(String::new(), |p| p.to_str().unwrap_or("").to_string());

    let Ok(db) = Database::create(&format!("{}/{}.redb", path, event.level_name)) else {
        warn!("Failed to open level db");
        return;
    };

    // todo read `Level`

    commands.insert_resource(Db { db });

    commands.insert_resource(Level {
        name: event.level_name,
        seed: 0,
    });

    *tickstep = TickStep::Tick;

    // In offline net_mode the player is spawned when the spawn chunks are ready
    // The chunks and sun beams are loaded when they are needed
}

pub fn close_level(
    event_reader: EventReader<CloseLevelEvent>,
    mut commands: Commands,
    existing_level: Option<Res<Level>>,
    existing_db: Option<Res<Db>>,
    mut universe: ResMut<Universe>,
    mut universe_changes: ResMut<UniverseChanges>,
    mut tickstep: ResMut<TickStep>,
    level_owned_query: Query<(Entity, &LevelOwned)>,
    mut light_sources: ResMut<LightSources>,
    mut sun_beams: ResMut<SunBeams>,
) {
    let Some(_) = get_single_event(event_reader) else {
        return;
    };

    if existing_level.is_none() {
        warn!("No level is loaded");
        return;
    }

    if existing_db.is_none() {
        warn!("No db was opened");
        panic!();
    }

    commands.remove_resource::<Db>();
    commands.remove_resource::<Level>();
    commands.remove_resource::<LevelReady>();

    universe.chunks.clear();
    universe_changes.queue.clear();
    light_sources.leaked_sources.clear();
    light_sources.chunked_sources.clear();
    sun_beams.beams.clear();
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

#[derive(Pod, Zeroable, Copy, Debug, Clone, Default)]
#[repr(C)]
pub struct BeamPod {
    bottom: i32,
    top: i32,
}

pub fn save_level(
    event_reader: EventReader<SaveLevelEvent>,
    level: Option<Res<Level>>,
    universe: Res<Universe>,
    players_query: Query<&Transform, With<Character>>,
    camera_query: Query<(&Transform, &Parent), With<CameraController>>,
    sun_beams: Res<SunBeams>,
    existing_db: Option<ResMut<Db>>,
) {
    let Some(_) = get_single_event(event_reader) else {
        return;
    };

    let Some(level) = level.as_ref() else {
        warn!("There is no level to save");
        return;
    };

    let Some(db) = existing_db else {
        warn!("No db was opened");
        panic!();
    };

    info!("saving to: {:?}", get_save_path());

    let mut serde_players = vec![];
    for (camera_tr, parent) in camera_query.iter() {
        let body_tr = players_query
            .get(parent.get())
            .expect("Players should have a body");
        let player = SerdePlayer {
            // Todo: handle player names
            name: "Nameless".to_string(),
            translation: body_tr.translation,
            body_rotation: body_tr.rotation,
            camera_rotation: camera_tr.rotation,
        };
        serde_players.push(player);
    }

    db.write(|tx| {
        write_level(tx, level)?;
        write_sun_beams(tx, &sun_beams, &universe)?;
        write_chunks(tx, &universe)?;

        let mut table = tx.open_table(TABLE_PLAYERS)?;
        for serde_player in serde_players.iter() {
            write_player(tx, serde_player, Some(&mut table))?
        }

        Ok(())
    })
    .expect("db write failed");

    info!("save successful");
}

pub fn write_level<'txn>(write_txn: &'txn WriteTransaction, level: &Level) -> Result<(), Error> {
    let mut table = write_txn.open_table(TABLE_LEVEL)?;
    let bytes = bincode::serialize(level).expect("failed to serialize level");
    table.insert("info", &*bytes)?;
    Ok(())
}

pub fn write_player<'txn>(
    write_txn: &'txn WriteTransaction,
    player: &SerdePlayer,
    table: Option<&mut Table<'txn, &str, &[u8]>>,
) -> Result<(), Error> {
    let player_bytes = bincode::serialize(player).expect("failed player serialization");
    let table = if let Some(table) = table {
        table
    } else {
        &mut write_txn.open_table(TABLE_PLAYERS)?
    };
    table.insert(player.name.as_str(), &*player_bytes)?;
    Ok(())
}

pub fn write_sun_beams<'txn>(
    write_txn: &'txn WriteTransaction,
    sun_beams: &SunBeams,
    universe: &Universe,
) -> Result<(), Error> {
    let mut sun_beams_by_region = HashSet::<IVec2>::new();
    for (xz, _) in sun_beams.beams.iter() {
        let (region_pos, _) = universe.pos_to_region_and_inner(xz);
        sun_beams_by_region.insert(region_pos);
    }
    let mut table = write_txn.open_table(TABLE_SUN_BEAMS)?;
    for region_pos in sun_beams_by_region {
        write_sun_beams_region(write_txn, region_pos, sun_beams, Some(&mut table))?
    }
    Ok(())
}

pub fn write_sun_beams_region<'txn>(
    write_txn: &'txn WriteTransaction,
    region_pos: IVec2,
    sun_beams: &SunBeams,
    table: Option<&mut Table<'txn, [i32; 2], &[u8]>>,
) -> Result<(), Error> {
    let mut region = [BeamPod::default(); CHUNK_AREA];
    for (x, z) in (0..CHUNK_SIDE as i32)
        .map(|x| (0..CHUNK_SIDE as i32).map(move |z| (x, z)))
        .flatten()
    {
        let xz = IVec2::new(x, z) + region_pos;
        let beam = if let Some(beam) = sun_beams.beams.get(&xz) {
            beam
        } else {
            &SunBeam::new_top(&xz)
        };
        let region_index = (x + z * CHUNK_SIDE as i32) as usize;
        region[region_index].bottom = beam.bottom;
        region[region_index].top = beam.top;
    }
    let beams_bytes: &[u8] = bytemuck::cast_slice(&region);
    let table = if let Some(table) = table {
        table
    } else {
        &mut write_txn.open_table(TABLE_SUN_BEAMS)?
    };
    table.insert(region_pos.to_array(), &*beams_bytes)?;
    Ok(())
}

pub fn write_chunks<'txn>(
    write_txn: &'txn WriteTransaction,
    universe: &Universe,
) -> Result<(), Error> {
    let mut table = write_txn.open_table(TABLE_BLOCKS)?;
    for (chunk_pos, chunk) in universe.chunks.iter() {
        write_chunk(write_txn, chunk_pos, &chunk, Some(&mut table))?
    }
    Ok(())
}

pub fn write_chunk<'txn>(
    write_txn: &'txn WriteTransaction,
    chunk_pos: &IVec3,
    chunk: &Chunk,
    table: Option<&mut Table<'txn, [i32; 3], &[u8]>>,
) -> Result<(), Error> {
    let chunk_ref = chunk.get_ref();
    let block_bytes: &[u8] = bytemuck::cast_slice(chunk_ref.as_ref());
    let block_compressed = compress_to_vec(block_bytes, 6);
    let table = if let Some(table) = table {
        table
    } else {
        &mut write_txn.open_table(TABLE_BLOCKS)?
    };
    table.insert(&chunk_pos.to_array(), &*block_compressed)?;
    Ok(())
}

pub fn read_player<'txn>(
    read_txn: &'txn ReadTransaction,
    player_name: &str,
) -> Option<SerdePlayer> {
    let table = read_txn.open_table(TABLE_PLAYERS).ok()?;
    let option = table.get(player_name).ok()?;
    let value = option?;
    let player = bincode::deserialize(value.value()).expect("failed to deserialize player");
    Some(player)
}

pub fn read_chunk<'txn>(read_txn: &'txn ReadTransaction, chunk_pos: &IVec3) -> Option<Chunk> {
    let table = read_txn.open_table(TABLE_BLOCKS).ok()?;
    let option = table.get(chunk_pos.to_array()).ok()?;
    let value = option?;
    let block_decompressed = decompress_to_vec_with_limit(value.value(), CHUNK_VOLUME * 4)
        .expect("failed to decompress chunk");
    let chunk = Chunk::empty();
    {
        let mut write = chunk.get_mut();
        let bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut (*write));
        bytes.copy_from_slice(&*block_decompressed);
    }
    Some(chunk)
}

pub fn read_sun_beams<'txn>(
    read_txn: &'txn ReadTransaction,
    region_pos: &IVec2,
) -> Option<Vec<(IVec2, SunBeam)>> {
    let table = read_txn.open_table(TABLE_SUN_BEAMS).ok()?;
    let option = table.get(region_pos.to_array()).ok()?;
    let value = option?;
    let mut beams = vec![];
    let beams_pod: &[BeamPod] = bytemuck::cast_slice(value.value());
    for i in 0..CHUNK_AREA as i32 {
        let (x, z) = (i % CHUNK_SIDE as i32, i / CHUNK_SIDE as i32);
        let pos = IVec2::new(region_pos.x + x, region_pos.y + z);
        let beam = SunBeam {
            bottom: beams_pod[i as usize].bottom,
            top: beams_pod[i as usize].top,
        };
        beams.push((pos, beam));
    }
    Some(beams)
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
