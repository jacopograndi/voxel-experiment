use bevy::{prelude::*, utils::HashSet};
use mcrs_chemistry::lighting::*;
use mcrs_physics::{
    character::{CameraController, CharacterController},
    intersect::get_chunks_in_sphere,
    raycast::{cast_ray, RayFinite},
};
use mcrs_universe::{
    block::{Block, LightType},
    chunk::Chunk,
    flagbank::BlockFlag,
    universe::Universe,
    Blueprints,
};

use crate::{PlayerInput, PlayerInputBuffer};
use noise::{NoiseFn, OpenSimplex, RidgedMulti, Seedable};

use crate::{hotbar::PlayerHand, settings::McrsSettings};

pub fn terrain_editing(
    camera_query: Query<(&CameraController, &GlobalTransform, &Parent)>,
    mut player_query: Query<(&mut PlayerInputBuffer, &PlayerHand)>,
    mut universe: ResMut<Universe>,
    blueprints: Res<Blueprints>,
) {
    for (_cam, tr, parent) in camera_query.iter() {
        let Ok((mut input, hand)) = player_query.get_mut(parent.get()) else {
            continue;
        };

        #[derive(PartialEq)]
        enum Act {
            PlaceBlock,
            RemoveBlock,
        }
        for input in input.buffer.iter() {
            let act = match input {
                PlayerInput::Placing(true) => Some(Act::PlaceBlock),
                PlayerInput::Mining(true) => Some(Act::RemoveBlock),
                _ => None,
            };
            if let Some(act) = act {
                if let Some(hit) = cast_ray(
                    RayFinite {
                        position: tr.translation(),
                        direction: tr.forward(),
                        reach: 4.5,
                    },
                    &universe,
                ) {
                    match act {
                        Act::RemoveBlock => {
                            let pos = hit.grid_pos;

                            debug!(target: "terrain_editing", "removed block at {}", pos);

                            let mut light_suns = vec![];
                            let mut light_torches = vec![];

                            if let Some(block) = universe.read_chunk_block(&pos) {
                                if blueprints.blocks.get(&block.id).is_light_source() {
                                    let new =
                                        propagate_darkness(&mut universe, pos, LightType::Torch);
                                    propagate_light(&mut universe, new, LightType::Torch)
                                }
                            }

                            universe.set_chunk_block(
                                &pos,
                                Block::new(blueprints.blocks.get_named("Air")),
                            );

                            let planar = IVec2::new(pos.x, pos.z);
                            if let Some(height) = universe.heightfield.get(&planar) {
                                if pos.y == *height {
                                    // recalculate the highest sunlit point
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
                        Act::PlaceBlock => {
                            let pos = hit.grid_pos + hit.normal();

                            debug!(target: "terrain_editing", "placed block at {}", pos);

                            let mut dark_suns = vec![];

                            let Some(block_id) = hand.block_id else {
                                continue;
                            };

                            let blueprint = blueprints.blocks.get(&block_id);
                            universe.set_chunk_block(&pos, Block::new(blueprint));

                            propagate_light(&mut universe, vec![pos], LightType::Torch);

                            let planar = IVec2::new(pos.x, pos.z);
                            if let Some(height) = universe.heightfield.get(&planar) {
                                if pos.y > *height {
                                    // recalculate the highest sunlit point
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
                    };
                }
            }
        }
        input.buffer.clear();
    }
}

fn gen_chunk(pos: IVec3, info: &Blueprints) -> Chunk {
    let noise = RidgedMulti::<OpenSimplex>::default().set_seed(23);

    let air = Block::new(info.blocks.get_named("Air"));
    let stone = Block::new(info.blocks.get_named("Stone"));
    let dirt = Block::new(info.blocks.get_named("Dirt"));

    let chunk = Chunk::empty();
    {
        let mut chunk_mut = chunk.get_mut();
        for (i, xyz) in Chunk::iter().enumerate() {
            let mut sample = noise.get(((pos + xyz).as_dvec3() * 0.01).to_array());
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
            let block = if sample > 0.9 {
                dirt
            } else if sample > 0.5 {
                air
            } else {
                stone
            };
            chunk_mut[i] = block;
        }
    }
    chunk
}

pub fn terrain_generation(
    mut universe: ResMut<Universe>,
    player_query: Query<(&CharacterController, &Transform)>,
    info: Res<Blueprints>,
    settings: Res<McrsSettings>,
) {
    let players_pos = player_query
        .iter()
        .map(|(_, tr)| tr.translation)
        .collect::<Vec<Vec3>>();

    let mut added = HashSet::<IVec3>::new();
    for player_pos in players_pos.iter() {
        let chunks = get_chunks_in_sphere(*player_pos, settings.view_distance_blocks as f32);
        for chunk_pos in chunks.iter() {
            if let None = universe.chunks.get(chunk_pos) {
                let chunk = gen_chunk(*chunk_pos, &*info);
                universe.chunks.insert(*chunk_pos, chunk);
                added.insert(*chunk_pos);
            }
        }
    }

    if !added.is_empty() {
        recalc_lights(&mut universe, added.into_iter().collect(), &*info);
    }
}
