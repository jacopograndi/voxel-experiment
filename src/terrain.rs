use crate::{
    chemistry::lighting::*, debug::WidgetBlockDebug, get_chunk_from_save, hotbar::PlayerHand,
    settings::McrsSettings, Level, PlayerInput, PlayerInputBuffer,
};
use bevy::{prelude::*, utils::HashMap};
use bevy_egui::{egui, EguiContexts};
use mcrs_net::LocalPlayer;
use mcrs_physics::{
    character::CameraController,
    intersect::get_chunks_in_sphere,
    raycast::{cast_ray, RayFinite},
};
use mcrs_universe::{
    block::{Block, BlockFlag, LightType},
    chunk::Chunk,
    universe::Universe,
    Blueprints, CHUNK_AREA, CHUNK_SIDE, CHUNK_VOLUME, MAX_LIGHT,
};
use noise::{NoiseFn, OpenSimplex, RidgedMulti, Seedable};

const BLOCK_GENERATION_LOW_MULTIPLIER: usize = 2;
const BLOCK_GENERATION_LOW_THRESHOLD: usize = 400;
const MAX_BLOCK_GENERATION_PER_FRAME: usize = 20000;

#[derive(Debug, Clone)]
pub enum UniverseChange {
    Add { pos: IVec3, block: Block },
    Remove { pos: IVec3 },
}

#[derive(Default, Clone, Debug)]
pub struct LightSource {
    pub pos: IVec3,
    pub brightness: u8,
}

#[derive(Default, Clone, Debug)]
pub struct LightSourcesChunked {
    pub sources: Vec<LightSource>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct LightSources {
    pub chunked_sources: HashMap<LightType, HashMap<IVec3, LightSourcesChunked>>,
    pub leaked_sources: HashMap<LightType, Vec<LightSource>>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct UniverseChanges {
    pub queue: Vec<UniverseChange>,
}

pub fn apply_terrain_changes(
    mut universe: ResMut<Universe>,
    mut changes: ResMut<UniverseChanges>,
    bp: Res<Blueprints>,
) {
    for change in changes.queue.iter() {
        match change {
            UniverseChange::Remove { pos } => {
                debug!(target: "terrain_editing", "removed block at {}", pos);

                let mut light_suns = vec![];
                let mut light_torches = vec![];

                if let Some(block) = universe.read_chunk_block(&pos) {
                    if bp.blocks.get(&block.id).is_light_source() {
                        let new = propagate_darkness(&mut universe, *pos, LightType::Torch);
                        propagate_light(&mut universe, new, LightType::Torch)
                    }
                }

                universe.set_chunk_block(&pos, Block::new(bp.blocks.get_named("Air")));

                // Todo: sun beams
                /*
                let planar = IVec2::new(pos.x, pos.z);
                if let Some(height) = universe.heightfield.get(&planar) {
                    if pos.y == *height {
                        // Recalculate the highest sunlit point
                        let mut beam = pos.y - 100;
                        for y in 0..=100 {
                            let h = pos.y - y;
                            let sample = IVec3::new(pos.x, h, pos.z);
                            if let Some(voxel) = universe.read_chunk_block(&sample) {
                                if voxel.properties.check(BlockFlag::Opaque) {
                                    beam = h;
                                    break;
                                } else {
                                    light_suns.push(sample);

                                    let mut lit = voxel.clone();
                                    lit.set_light(LightType::Sun, 15);
                                    universe.set_chunk_block(&sample, lit);
                                }
                            }
                        }
                        universe.heightfield.insert(planar, beam);
                    }
                }
                */

                for dir in DIRS.iter() {
                    let sample = pos + *dir;
                    if let Some(voxel) = universe.read_chunk_block(&sample) {
                        if !voxel.properties.check(BlockFlag::Opaque) {
                            if voxel.get_light(LightType::Sun) > 1 {
                                light_suns.push(sample);
                            }
                            if voxel.get_light(LightType::Torch) > 1 {
                                light_torches.push(sample);
                            }
                        }
                    }
                }

                propagate_light(&mut universe, light_suns, LightType::Sun);
                propagate_light(&mut universe, light_torches, LightType::Torch);
            }
            UniverseChange::Add { pos, block } => {
                debug!(target: "terrain_editing", "placed block at {}", pos);
                universe.set_chunk_block(&pos, *block);
                propagate_light(&mut universe, vec![*pos], LightType::Torch);

                // Todo: sun beams
                /*
                let mut dark_suns = vec![];
                let planar = IVec2::new(pos.x, pos.z);
                if let Some(height) = universe.heightfield.get(&planar) {
                    if pos.y > *height {
                        // Recalculate the highest sunlit point
                        for y in (*height)..pos.y {
                            let sample = IVec3::new(pos.x, y, pos.z);
                            dark_suns.push(sample);
                        }
                        universe.heightfield.insert(planar, pos.y);
                    }
                }

                for sun in dark_suns {
                    let new = propagate_darkness(&mut universe, sun, LightType::Sun);
                    propagate_light(&mut universe, new, LightType::Sun)
                }
                */
            }
        }
    }
    changes.queue.clear()
}

pub fn apply_lighting_sources(
    mut universe: ResMut<Universe>,
    mut light_sources: ResMut<LightSources>,
) {
    for (light_type, sources) in light_sources.leaked_sources.iter_mut() {
        debug!(target: "lighting_leaked", "leaked {}: {}",
            light_type,
            sources.len()
        );
    }
    for (light_type, chunked_sources) in light_sources.chunked_sources.iter_mut() {
        debug!(target: "lighting_source", "chunks {}: {}",
            light_type,
            chunked_sources.len()
        );
    }

    let mut leaked = HashMap::<LightType, Vec<LightSource>>::new();

    // Propagate the light in loaded chunks and collect the leaked light
    let mut processed_chunks = vec![];
    for (light_type, chunked_sources_list) in light_sources.chunked_sources.iter() {
        for (chunk_pos, chunked_sources) in chunked_sources_list {
            if let Some(chunk) = universe.chunks.get_mut(chunk_pos) {
                processed_chunks.push(*chunk_pos);

                let mut chunk_mut = chunk.get_mut();
                let mut sources = vec![];
                for source in chunked_sources.sources.iter() {
                    let block = &mut chunk_mut[Chunk::xyz2idx(source.pos)];
                    if block.get_light(*light_type) < source.brightness
                        && !block.properties.check(BlockFlag::Opaque)
                    {
                        block.set_light(*light_type, source.brightness);
                        sources.push(source.pos);
                    }
                }
                if !sources.is_empty() {
                    let mut leaked_from_chunk =
                        propagate_light_chunk(&mut chunk_mut, sources, *light_type);
                    for source in leaked_from_chunk.iter_mut() {
                        source.pos += chunk_pos;
                    }
                    leaked
                        .entry(*light_type)
                        .or_default()
                        .append(&mut leaked_from_chunk);
                }
            }
        }
    }

    // Remove chunks that have been lit
    for (_, chunked_sources_list) in light_sources.chunked_sources.iter_mut() {
        for chunk_pos in processed_chunks.iter() {
            chunked_sources_list.remove(chunk_pos);
        }
    }

    // Collect leaked light that has leaked outside of this function
    for (light_type, mut sources) in light_sources.leaked_sources.iter_mut() {
        leaked.entry(*light_type).or_default().append(&mut sources);
    }

    // Move the leaked light into the chunked sources
    for (light_type, sources) in leaked {
        for source in sources {
            let (chunk_pos_leaked, inner) = universe.pos_to_chunk_and_inner(&source.pos);
            let chunked_sources_list_leaked =
                light_sources.chunked_sources.entry(light_type).or_default();
            let chunked_sources_leaked = chunked_sources_list_leaked
                .entry(chunk_pos_leaked)
                .or_default();
            chunked_sources_leaked.sources.push(LightSource {
                pos: inner,
                brightness: source.brightness,
            });
        }
    }
}

pub fn terrain_editing(
    camera_query: Query<(&CameraController, &GlobalTransform, &Parent)>,
    mut player_query: Query<(&mut PlayerInputBuffer, &PlayerHand)>,
    universe: Res<Universe>,
    bp: Res<Blueprints>,
    mut gizmos: Gizmos,
    mut contexts: EguiContexts,
    mut show_red_cube: Local<bool>,
    mut changes: ResMut<UniverseChanges>,
) {
    for (_cam, tr, parent) in camera_query.iter() {
        let Ok((mut input, hand)) = player_query.get_mut(parent.get()) else {
            continue;
        };

        // Show debug info
        if let Some(hit) = cast_ray(
            RayFinite {
                position: tr.translation(),
                direction: tr.forward().as_vec3(),
                reach: 4.5,
            },
            &universe,
        ) {
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

            let intersection = hit.final_position();

            egui::Window::new("Player Raycast Hit")
                .anchor(egui::Align2::LEFT_CENTER, egui::Vec2::new(5.0, 0.0))
                .show(contexts.ctx_mut(), |ui| {
                    ui.add(WidgetBlockDebug::new(hit.grid_pos, &universe, &bp));
                    if *show_red_cube {
                        ui.add(WidgetBlockDebug::new(
                            hit.grid_pos + hit.normal(),
                            &universe,
                            &bp,
                        ));
                    }
                    ui.checkbox(&mut show_red_cube, "Show the facing cube in red");
                });

            gizmos.cuboid(
                Transform::from_translation(intersection).with_scale(Vec3::splat(0.01)),
                Color::BLACK,
            );

            let center_pos = hit.grid_pos.as_vec3() + Vec3::splat(0.5);
            gizmos.cuboid(
                Transform::from_translation(center_pos).with_scale(Vec3::splat(1.001)),
                Color::BLACK,
            );

            if *show_red_cube {
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
        input.buffer.clear();
    }
}

#[derive(Resource, Default)]
pub struct ChunkGenerationRequest {
    pub requested: HashMap<IVec3, ChunkGenerationState>,
}

impl ChunkGenerationRequest {
    fn insert(&mut self, chunk_pos: IVec3) {
        self.requested
            .entry(chunk_pos)
            .or_insert(ChunkGenerationState {
                pos: chunk_pos,
                ..Default::default()
            });
    }

    fn get_for_pass<'a>(
        &'a self,
        pass: &'a GenerationPass,
    ) -> impl Iterator<Item = (&'a IVec3, &'a ChunkGenerationState)> {
        self.requested
            .iter()
            .filter(|(_, state)| state.pass == *pass)
    }

    fn get_for_pass_mut<'a>(
        &'a mut self,
        pass: &'a GenerationPass,
    ) -> impl Iterator<Item = (&'a IVec3, &'a mut ChunkGenerationState)> {
        self.requested
            .iter_mut()
            .filter(|(_, state)| state.pass == *pass)
    }
}

#[derive(Default, PartialEq, Eq)]
pub enum GenerationPass {
    #[default]
    /// The chunk is requesting for blocks to be generated
    Blocks,
    /// The chunk is requesting lighting to be recalculated
    Lighting,
    /// The chunk is waiting on the chunk above to be done before reverting to `Lighting`
    WaitingForSunlight,
    /// The chunk is ready to be added to the universe
    Done,
}

pub struct ChunkGenerationState {
    pub chunk: Option<Chunk>,
    pub pos: IVec3,
    pub current_block: usize,
    pub pass: GenerationPass,
    pub blocks_lowest: [i32; CHUNK_AREA],
    pub blocks_beams: [i32; CHUNK_AREA],
}

impl Default for ChunkGenerationState {
    fn default() -> Self {
        Self {
            chunk: None,
            pos: IVec3::default(),
            current_block: 0,
            pass: GenerationPass::default(),
            blocks_lowest: [0; CHUNK_AREA],
            blocks_beams: [CHUNK_SIDE as i32 - 1; CHUNK_AREA],
        }
    }
}

pub fn get_spawn_chunks() -> impl Iterator<Item = IVec3> {
    (-1..=1)
        .map(|z| {
            (-1..=1)
                .map(move |y| (-1..=1).map(move |x| IVec3::new(x, y, z) * CHUNK_SIDE as i32))
                .flatten()
        })
        .flatten()
}

pub fn get_sun_heightfield(_xz: IVec2) -> i32 {
    128
}

pub fn request_base_chunks(
    universe: Res<Universe>,
    players: Query<(&Transform, &LocalPlayer)>,
    mut request: ResMut<ChunkGenerationRequest>,
    settings: Res<McrsSettings>,
) {
    let players_pos = players
        .iter()
        .map(|(tr, _)| tr.translation)
        .collect::<Vec<Vec3>>();

    // Check the spawn chunks
    for chunk_pos in get_spawn_chunks() {
        if let None = universe.chunks.get(&chunk_pos) {
            request.insert(chunk_pos.clone());
        }
    }

    // Check near every player
    for player_pos in players_pos.iter() {
        let chunks = get_chunks_in_sphere(*player_pos, settings.view_distance_blocks as f32);
        for chunk_pos in chunks.iter() {
            if let None = universe.chunks.get(chunk_pos) {
                request.insert(chunk_pos.clone());
            }
        }
    }
}

pub fn chunk_generation(
    mut universe: ResMut<Universe>,
    bp: Res<Blueprints>,
    mut light_sources: ResMut<LightSources>,
    mut request: ResMut<ChunkGenerationRequest>,
    mut generator: Local<Option<GeneratorSponge>>,
    level: Option<Res<Level>>,
) {
    let Some(level) = level else {
        return;
    };

    // Initialize the block generator
    if generator.is_none() {
        *generator = Some(GeneratorSponge::new(level.seed));
    }
    let Some(generator) = generator.as_ref() else {
        return;
    };

    // This is a rough estimate for when the world around the player is empty
    // and is used to speed up the generation of the initial chunks
    // Todo: use a better estimator after having implemented chunk unloading
    //       or use a better system
    let is_low = universe.chunks.len() < BLOCK_GENERATION_LOW_THRESHOLD;
    let max_block_generation = if is_low {
        MAX_BLOCK_GENERATION_PER_FRAME * BLOCK_GENERATION_LOW_MULTIPLIER
    } else {
        MAX_BLOCK_GENERATION_PER_FRAME
    };

    // Try to load chunks before generating them
    // Todo: limit the number of loaded chunk per frame
    let mut loaded_chunks = vec![];
    for (chunk_pos, _) in request.get_for_pass(&GenerationPass::Blocks) {
        if let Some(chunk) = get_chunk_from_save(chunk_pos, &level.name) {
            universe.chunks.insert(*chunk_pos, chunk);
            loaded_chunks.push(*chunk_pos);
            info!("loaded chunk at {}", chunk_pos);
        }
    }
    for loaded_chunk in loaded_chunks {
        request.requested.remove(&loaded_chunk);
    }

    let mut generated_blocks = 0;

    for _ in 0..10 {
        let Some((chunk_pos, part)) = request.get_for_pass_mut(&GenerationPass::Blocks).next()
        else {
            break;
        };

        if part.chunk.is_none() {
            part.chunk = Some(Chunk::empty());
        }

        info!("generating chunk at {}", chunk_pos);

        let mut generated_blocks_chunk = 0;

        // Generate up to a chunk
        if let Some(chunk) = &part.chunk {
            let mut chunk_mut = chunk.get_mut();
            let bound =
                (part.current_block + (max_block_generation - generated_blocks)).min(CHUNK_VOLUME);
            for i in part.current_block..bound {
                chunk_mut[i] = generator.gen_block(part.pos + Chunk::idx2xyz(i), &bp);
            }
            let count = bound - part.current_block;
            generated_blocks_chunk = count;
            generated_blocks += count;
            info!(
                "generated {} blocks for the chunk at {}, {} left",
                count, part.pos, part.current_block
            );
        }
        if generated_blocks_chunk > 0 {
            part.current_block += generated_blocks_chunk;
        }

        if part.current_block == CHUNK_VOLUME {
            part.pass = GenerationPass::Lighting;
        }

        if generated_blocks >= max_block_generation {
            break;
        }
    }

    let mut request_for_sunlight = vec![];

    for _ in 0..10 {
        let Some((chunk_pos, part)) = request.get_for_pass_mut(&GenerationPass::Lighting).next()
        else {
            break;
        };

        info!("trying to light chunk at {}", chunk_pos);

        let Some(chunk) = &part.chunk else {
            error!("the chunk has no blocks in it");
            continue;
        };

        let is_sun =
            |IVec3 { x, y, z }: IVec3| chunk_pos.y + y >= get_sun_heightfield(IVec2::new(x, z));

        // Calculate sunbeams by raycasting up from the lowest blocks of the chunk.
        // If any beam escapes the chunk, request the chunk above.
        let mut chunk_mut = chunk.get_mut();
        let mut any_beam_escaped = false;
        for (x, z) in (0..CHUNK_SIDE as i32)
            .map(|x| (0..CHUNK_SIDE as i32).map(move |z| (x, z)))
            .flatten()
        {
            let plane_index = (x + z * CHUNK_SIDE as i32) as usize;
            for y in (0..CHUNK_SIDE as i32).rev() {
                let xyz = IVec3::new(x, y, z);
                let is_opaque = chunk_mut[Chunk::xyz2idx(xyz)]
                    .properties
                    .check(BlockFlag::Opaque);
                if is_opaque {
                    part.blocks_lowest[plane_index] = y;
                    break;
                }
            }

            for y in part.blocks_lowest[plane_index]..CHUNK_SIDE as i32 {
                let xyz = IVec3::new(x, y, z);
                let is_opaque = chunk_mut[Chunk::xyz2idx(xyz)]
                    .properties
                    .check(BlockFlag::Opaque)
                    && y > part.blocks_lowest[plane_index];
                if is_sun(xyz) || is_opaque {
                    part.blocks_beams[plane_index] = y;
                    break;
                }
            }

            let is_escaped = part.blocks_beams[plane_index] == CHUNK_SIDE as i32 - 1;
            if is_escaped && !is_sun(IVec3::new(x, CHUNK_SIDE as i32 - 1, z)) {
                any_beam_escaped = true;
            }
        }

        let above = chunk_pos + IVec3::Y * CHUNK_SIDE as i32;
        let chunk_above = universe.chunks.get(&above);

        if any_beam_escaped && chunk_above.is_none() {
            request_for_sunlight.push(above);
            part.pass = GenerationPass::WaitingForSunlight;
            info!(
                target: "terrain_generation",
                "lighting for chunk {} is waiting for the chunk above", chunk_pos,
            );
            continue;
        }

        // Todo: Adjust sun beam cache

        // Calculate the light for the first time
        let mut new_sources = HashMap::<LightType, Vec<IVec3>>::new();

        info!(
            target: "terrain_generation",
            "lighting chunk at {}", chunk_pos
        );

        for (x, z) in (0..CHUNK_SIDE as i32)
            .map(|x| (0..CHUNK_SIDE as i32).map(move |z| (x, z)))
            .flatten()
        {
            let plane_index = (x + z * CHUNK_SIDE as i32) as usize;
            let sun_beam = chunk_pos.y + part.blocks_beams[plane_index]
                == get_sun_heightfield(IVec2::new(x, z));
            let escaped_beam = part.blocks_beams[plane_index] == CHUNK_SIDE as i32 - 1;
            let sun_light = if sun_beam {
                MAX_LIGHT
            } else if escaped_beam {
                if let Some(block) =
                    universe.read_chunk_block(&(chunk_pos + IVec3::new(x, CHUNK_SIDE as i32, z)))
                {
                    block.get_light(LightType::Sun)
                } else {
                    0
                }
            } else {
                0
            };
            for y in part.blocks_lowest[plane_index]
                ..=(part.blocks_beams[plane_index].min(CHUNK_SIDE as i32 - 1)) as i32
            {
                let xyz = IVec3::new(x, y, z);
                chunk_mut[Chunk::xyz2idx(xyz)].set_light(LightType::Sun, sun_light);
                new_sources.entry(LightType::Sun).or_default().push(xyz);
            }
        }

        for xyz in Chunk::iter() {
            let id = chunk_mut[Chunk::xyz2idx(xyz)].id;
            let block_bp = bp.blocks.get(&id);
            if block_bp.is_light_source() {
                chunk_mut[Chunk::xyz2idx(xyz)].set_light(LightType::Torch, block_bp.light_level);
                new_sources.entry(LightType::Torch).or_default().push(xyz);
            }
        }

        for (light_type, sources) in new_sources {
            if !sources.is_empty() {
                let mut leaked_from_chunk =
                    propagate_light_chunk(&mut chunk_mut, sources, light_type);
                for source in leaked_from_chunk.iter_mut() {
                    source.pos += chunk_pos;
                }
                light_sources
                    .leaked_sources
                    .entry(light_type)
                    .or_default()
                    .append(&mut leaked_from_chunk);
            }
        }

        part.pass = GenerationPass::Done;
    }

    for chunk_pos in request_for_sunlight {
        request.insert(chunk_pos);
    }

    for (chunk_pos, part) in request.get_for_pass_mut(&GenerationPass::WaitingForSunlight) {
        let above = chunk_pos + IVec3::Y * CHUNK_SIDE as i32;
        if universe.chunks.get(&above).is_some() {
            info!(
                target: "terrain_generation",
                "lighting for chunk {} resumed", chunk_pos,
            );
            part.pass = GenerationPass::Lighting;
        }
    }

    request
        .requested
        .retain(|chunk_pos, state| match state.pass {
            GenerationPass::Done => {
                universe
                    .chunks
                    .insert(*chunk_pos, state.chunk.take().unwrap());
                info!(
                    target: "terrain_generation",
                    "chunk generated at {}", chunk_pos
                );
                false
            }
            _ => true,
        });
}

pub struct GeneratorSponge {
    terrain_noise: RidgedMulti<OpenSimplex>,
}

impl GeneratorSponge {
    fn new(seed: u32) -> Self {
        Self {
            terrain_noise: RidgedMulti::<OpenSimplex>::default().set_seed(seed),
        }
    }

    fn sample_terrain(&self, pos: IVec3) -> f64 {
        let mut sample = self.terrain_noise.get((pos.as_dvec3() * 0.01).to_array());
        sample = (sample + 1.0) * 0.5;
        sample = sample.clamp(0.0, 1.0);
        if sample >= 1.0 {
            sample = 0.999999;
        }
        assert!(
            (0.0..1.0).contains(&sample),
            "sample {} not in 0.0..1.0",
            sample
        );
        sample
    }

    fn gen_block(&self, pos: IVec3, bp: &Blueprints) -> Block {
        let sample = self.sample_terrain(pos);

        let weigth: f64 = if pos.y > 0 {
            let amt = (pos.y as f64 / 64.0).min(1.0);
            0.5 - 0.5 * amt
        } else if pos.y > -64 {
            let y = pos.y + 64;
            let amt = (y as f64 / 64.0).min(1.0);
            0.8 - 0.3 * amt
        } else {
            0.8
        };

        let block_bp = if sample >= weigth {
            bp.blocks.get_named("Air")
        } else if pos.y > -32 {
            bp.blocks.get_named("Dirt")
        } else {
            bp.blocks.get_named("Stone")
        };

        Block::new(block_bp)
    }
}
