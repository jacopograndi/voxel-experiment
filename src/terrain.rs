use crate::{chemistry::lighting::*, debug::WidgetBlockDebug, get_chunk_from_save, Level};
use bevy::{prelude::*, utils::HashSet};
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
    Blueprints, CHUNK_SIDE, CHUNK_VOLUME,
};

use crate::{PlayerInput, PlayerInputBuffer};
use noise::{NoiseFn, OpenSimplex, RidgedMulti, Seedable};

use crate::{hotbar::PlayerHand, settings::McrsSettings};

const BLOCK_GENERATION_LOW_MULTIPLIER: usize = 2;
const BLOCK_GENERATION_LOW_THRESHOLD: usize = 400;
const MAX_BLOCK_GENERATION_PER_FRAME: usize = 20000;

#[derive(Debug, Clone)]
pub enum UniverseChange {
    Add { pos: IVec3, block: Block },
    Remove { pos: IVec3 },
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
                let mut dark_suns = vec![];
                universe.set_chunk_block(&pos, *block);
                propagate_light(&mut universe, vec![*pos], LightType::Torch);

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
            }
        }
    }
    changes.queue.clear()
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

#[derive(Default)]
pub struct PartialGenerationState {
    chunk: Option<Chunk>,
    pos: IVec3,
    current_block: usize,
    queued: HashSet<IVec3>,
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

pub fn terrain_generation(
    mut universe: ResMut<Universe>,
    bp: Res<Blueprints>,
    players: Query<(&Transform, &LocalPlayer)>,
    mut part: Local<PartialGenerationState>,
    mut generator: Local<Option<GeneratorSponge>>,
    settings: Res<McrsSettings>,
    level: Option<Res<Level>>,
) {
    let Some(level) = level else {
        *part = PartialGenerationState::default();
        return;
    };

    let players_pos = players
        .iter()
        .map(|(tr, _)| tr.translation)
        .collect::<Vec<Vec3>>();

    // Queue new chunks to be generated
    if part.queued.is_empty() {
        // Check the spawn chunks
        for chunk_pos in get_spawn_chunks() {
            if let None = universe.chunks.get(&chunk_pos) {
                part.queued.insert(chunk_pos.clone());
            }
        }

        // Check near every player
        for player_pos in players_pos.iter() {
            let chunks = get_chunks_in_sphere(*player_pos, settings.view_distance_blocks as f32);
            for chunk_pos in chunks.iter() {
                if let None = universe.chunks.get(chunk_pos) {
                    part.queued.insert(chunk_pos.clone());
                }
            }
        }
    }

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

    let mut added_chunks = HashSet::new();

    // Try to load chunks before generating them
    // Todo: limit the number of loaded chunk per frame
    let mut loaded_chunks = vec![];
    for chunk_pos in part.queued.iter() {
        if let Some(chunk) = get_chunk_from_save(chunk_pos, &level.name) {
            universe.chunks.insert(*chunk_pos, chunk);
            loaded_chunks.push(*chunk_pos);
            added_chunks.insert(*chunk_pos);
            info!("loaded chunk at {}", chunk_pos);
        }
    }
    for loaded_chunk in loaded_chunks {
        part.queued.remove(&loaded_chunk);
    }

    let mut generated_blocks = 0;

    for _ in 0..10 {
        // Get a chunk from the queue and start generating it
        if part.chunk.is_none() {
            let Some(selected) = part.queued.iter().next() else {
                return;
            };
            let selected = selected.clone();
            part.queued.remove(&selected);
            part.chunk = Some(Chunk::empty());
            part.pos = selected;
            part.current_block = 0;
            info!("generating chunk at {}", selected);
        }

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

        // Add the chunk to universe if finished
        if part.current_block == CHUNK_VOLUME {
            if let Some(chunk) = part.chunk.take() {
                part.chunk = None;
                part.current_block = 0;
                universe.chunks.insert(part.pos, chunk);
                added_chunks.insert(part.pos);
            }
        }

        if generated_blocks >= max_block_generation {
            break;
        }
    }

    if generated_blocks > 0 {
        info!("generated {} blocks this pass", generated_blocks);
        recalc_lights(&mut universe, added_chunks.into_iter().collect(), &bp);
    }
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
