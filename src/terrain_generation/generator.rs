use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::{
    requested::requested_chunks,
    sun_beams::{RegionSunBeam, SunBeams},
};
use crate::{Db, Level, LightSources, McrsSettings, Player};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
    utils::{HashMap, HashSet},
};
use mcrs_universe::{block::Block, chunk::Chunk, universe::Universe, Blueprints, CHUNK_VOLUME};

/// Defines how the blocks in the universe are before any player edit
pub trait Generator: Sync + Send + Default + 'static {
    fn gen_block(&self, pos: IVec3, bp: &Blueprints) -> Block;
    fn gen_biome(
        &self,
        chunk_mut: RwLockWriteGuard<[Block; CHUNK_VOLUME]>,
        chunk_pos: IVec3,
        bp: &Blueprints,
    );
    fn gen_structures(
        &self,
        chunk_mut: RwLockWriteGuard<[Block; CHUNK_VOLUME]>,
        chunk_pos: IVec3,
        bp: &Blueprints,
    );
    fn lighting(
        &self,
        chunk_mut: RwLockWriteGuard<[Block; CHUNK_VOLUME]>,
        chunk_pos: IVec3,
        bp: &Blueprints,
    );
}

#[derive(Resource, Default)]
pub struct GeneratorLoaded<Gen: Generator + Default> {
    loaded: Arc<RwLock<Gen>>,
}

pub const CHUNK_GEN_QUEUE_SIZE: usize = 100;

#[derive(Resource, Default, Clone)]
pub struct ChunkGen {
    /// Pre-allocated to avoid runtime allocations
    pub queue: Vec<ChunkGenState>,

    /// Pointers to the queue
    pub requests: HashMap<IVec3, ChunkGenRequest>,
}

#[derive(Clone)]
pub struct ChunkGenState {
    pub chunk: Chunk,
    pub sun_beams: SunBeamsPointer,
    pub current_block: usize,
    pub pos: IVec3,
    pub pass: GenPass,
    pub requested_chunks: Vec<Chunk>,
}

/// Points to an array of SunBeams in a thread-safe way
#[derive(Debug, Clone)]
pub struct SunBeamsPointer(Arc<RwLock<RegionSunBeam>>);
impl SunBeamsPointer {
    fn get_ref(&self) -> RwLockReadGuard<RegionSunBeam> {
        self.0.read().unwrap()
    }
    fn get_mut(&self) -> RwLockWriteGuard<RegionSunBeam> {
        self.0.write().unwrap()
    }
}

#[derive(Default, Clone)]
pub struct ChunkGenRequest {
    pub pass: GenPass,
    pub depends_on: Vec<IVec3>,
}

#[derive(Default, PartialEq, Eq, Clone)]
pub enum GenPass {
    #[default]
    /// The chunk is requesting for blocks to be generated
    Blocks,

    /// The chunk is waiting on the chunk above to be done before reverting to `Lighting`
    WaitingForSunbeams,

    /// The chunk is requesting for sunbeams to be propagated through it
    Sunbeams,

    /// The chunk is requesting biome to be added
    Biome,

    /// The chunk is requesting structures to be added
    Structures,

    /// The chunk is requesting lighting to be recalculated
    Lighting,

    /// The chunk is ready to be added to the universe
    Done,
}

#[derive(Resource)]
struct ChunkGenTasks {
    generating_chunks: HashMap<IVec3, Task<ChunkGenState>>,
}

#[derive(Resource, Clone)]
struct GenSync {
    chunks: HashMap<IVec3, Arc<RwLock<ChunkGenRequest>>>,
    region_sun_beams: Arc<RwLock<HashMap<IVec2, RegionSunBeam>>>,
    bp: Arc<RwLock<Blueprints>>,
    bp: Arc<RwLock<Blueprints>>,
}

fn queue_generating_chunks(mut tasks: ResMut<ChunkGenTasks>, mut gen_sync: ResMut<GenSync>) {
    let task_pool = AsyncComputeTaskPool::get();

    for chunk_pos in [IVec3::new(1, 1, 1)] {
        // we might have already spawned a task for this `chunk_coord`
        if tasks.generating_chunks.contains_key(&chunk_pos) {
            continue;
        }

        // chunk pointer that it's working on
        // depends_on chunk pointer
        // bp arc ref
        // control block

        let chunk_state = ChunkGenState {
            chunk: todo!(),
            sun_beams: todo!(),
            current_block: todo!(),
            pos: todo!(),
            pass: todo!(),
            requested_chunks: todo!(),
        };

        let a_bp = gen_sync.bp.clone();

        let task = task_pool.spawn(async move {
            loop {
                let Ok(bp) = a_bp.read() else {
                    continue;
                };
                let Ok(generator) = generator.read() else {
                    continue;
                };

                match chunk_state.pass {
                    GenPass::Blocks => ,
                    GenPass::WaitingForSunbeams => todo!(),
                    GenPass::Sunbeams => todo!(),
                    GenPass::Biome => todo!(),
                    GenPass::Structures => todo!(),
                    GenPass::Lighting => todo!(),
                    GenPass::Done => todo!(),
                }

                &chunk_state;
            }
            todo!()
        });

        tasks.generating_chunks.insert(chunk_pos, task);
    }
}

pub fn setup_chunk_generation(
    mut commands: Commands,
    mut chunk_gen: ResMut<ChunkGen>,
    bp: Res<Blueprints>,
) {
    chunk_gen.queue.reserve(CHUNK_GEN_QUEUE_SIZE);

    commands.insert_resource(GenSync {
        chunks: HashMap::new(),
        region_sun_beams: Arc::new(RwLock::new(HashMap::new())),
        bp: Arc::new(RwLock::new((*bp).clone())),
    });
}

pub const CHUNK_GEN_MAX_PER_FRAME: u32 = 10;

pub fn generate_chunks_system<Gen: Generator>(
    mut universe: ResMut<Universe>,
    players: Query<(&Transform, &Player)>,
    bp: Res<Blueprints>,
    mut light_sources: ResMut<LightSources>,
    mut request: ResMut<ChunkGen>,
    generator: Res<GeneratorLoaded<Gen>>,
    mut sun_beams: ResMut<SunBeams>,
    settings: Res<McrsSettings>,
    level: Option<Res<Level>>,
    db: Option<Res<Db>>,
) {
    let Some(level) = level else {
        return;
    };
    let Some(db) = db else {
        return;
    };

    let base_chunks = requested_chunks(players.iter(), &settings);
    for (chunk_pos, priority) in base_chunks.iter() {
        if let None = universe.chunks.get(chunk_pos) {
            //request.insert_priority(*chunk_pos, *priority);
        }
    }

    let base_set: HashSet<&IVec3> = base_chunks.iter().map(|(pos, _)| pos).collect();
    /*
    let depended_on_set: HashSet<&IVec3> = request
        .requested
        .iter()
        .map(|(_, part)| part.depends_on.iter())
        .flatten()
        .collect();
    */

    /*
    // Chunks out of load distance and no longer needed are unloaded
    db.write(|tx| {
        let mut sun_table = tx.open_table(TABLE_SUN_BEAMS)?;
        let mut block_table = tx.open_table(TABLE_BLOCKS)?;
        let mut unload_chunks: Vec<IVec3> = vec![];
        for (chunk_pos, chunk) in universe.chunks.iter() {
            if !base_set.contains(&chunk_pos)
                && !request.requested.contains_key(chunk_pos)
                && !depended_on_set.contains(chunk_pos)
            {
                info!("unloaded chunk at {}", chunk_pos);
                write_chunk(tx, chunk_pos, chunk, Some(&mut block_table))?;
                write_sun_beams_region(tx, chunk_pos.xz(), &sun_beams, Some(&mut sun_table))?;
                unload_chunks.push(*chunk_pos);
            }
        }
        for chunk_pos in unload_chunks {
            universe.chunks.remove(&chunk_pos);
        }
        Ok(())
    })
    .expect("db write failed");

    drop(depended_on_set);
    */
}
