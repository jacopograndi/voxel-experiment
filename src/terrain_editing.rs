use bevy::prelude::*;
use mcrs_blueprints::{blocks::BlockId, flagbank::BlockFlag, Blueprints};
use mcrs_chemistry::lighting::*;
use mcrs_physics::{
    character::CameraController,
    raycast::{cast_ray, RayFinite},
};
use mcrs_storage::{
    block::{Block, LightType},
    universe::Universe,
};

use mcrs_input::PlayerInputBuffer;

pub fn terrain_editing(
    camera_query: Query<(&CameraController, &GlobalTransform, &Parent)>,
    mut player_query: Query<&mut PlayerInputBuffer>,
    mut universe: ResMut<Universe>,
    blueprints: Res<Blueprints>,
) {
    for (_cam, tr, parent) in camera_query.iter() {
        let Ok(mut input) = player_query.get_mut(parent.get()) else {
            continue;
        };

        #[derive(PartialEq)]
        enum Act {
            PlaceBlock,
            RemoveBlock,
        }
        for input in input.buffer.iter() {
            let act = match (input.placing, input.mining) {
                (true, _) => Some(Act::PlaceBlock),
                (_, true) => Some(Act::RemoveBlock),
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

                            let blueprint = blueprints
                                .blocks
                                .get(&BlockId::from_u8(input.block_in_hand));
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
