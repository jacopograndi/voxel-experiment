use bevy::prelude::*;
use voxel_flag_bank::BlockFlag;
use voxel_physics::{character::CameraController, raycast};
use voxel_storage::{
    block::{Block, LightType},
    universe::Universe,
    BlockType,
};

use crate::{lighting::*, PlayerInput};

pub fn voxel_break(
    camera_query: Query<(&CameraController, &GlobalTransform, &Parent)>,
    player_query: Query<&PlayerInput>,
    mut universe: ResMut<Universe>,
) {
    for (_cam, tr, parent) in camera_query.iter() {
        let Ok(input) = player_query.get(parent.get()) else {
            continue;
        };
        #[derive(PartialEq)]
        enum Act {
            PlaceBlock,
            RemoveBlock,
            Inspect,
        }
        let act = match (input.placing, input.mining, false) {
            (true, _, _) => Some(Act::PlaceBlock),
            (_, true, _) => Some(Act::RemoveBlock),
            (_, _, true) => Some(Act::Inspect),
            _ => None,
        };
        if let Some(act) = act {
            if let Some(hit) = raycast::raycast(tr.translation(), tr.forward(), 4.5, &universe) {
                match act {
                    Act::Inspect => {
                        println!(
                            "hit(pos:{}, block:{:?}, dist:{}), head(block:{:?})",
                            hit.pos,
                            universe.read_chunk_block(&hit.grid_pos),
                            hit.distance,
                            universe.read_chunk_block(&tr.translation().floor().as_ivec3()),
                        );
                    }
                    Act::RemoveBlock => {
                        println!("removed block");

                        let pos = hit.grid_pos;

                        let mut light_suns = vec![];
                        let mut light_torches = vec![];

                        if let Some(voxel) = universe.read_chunk_block(&pos) {
                            // todo: use BlockInfo.is_light_source
                            if voxel.is(BlockType::Dirt) {
                                let new = propagate_darkness(&mut universe, pos, LightType::Torch);
                                propagate_light(&mut universe, new, LightType::Torch)
                            }
                        }

                        universe.set_chunk_block(&pos, Block::new(BlockType::Air));

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
                        println!("placed block");

                        let pos = hit.grid_pos + hit.normal;

                        let mut dark_suns = vec![];

                        //if keys.pressed(KeyCode::Key3) {
                        if false {
                            // todo: use BlockInfo
                            universe.set_chunk_block(&pos, Block::new(BlockType::Wood));
                            universe
                                .read_chunk_block(&pos)
                                .unwrap()
                                .set_light(LightType::Torch, 14);
                            propagate_light(&mut universe, vec![pos], LightType::Torch)
                        } else {
                            let new = propagate_darkness(&mut universe, pos, LightType::Torch);

                            universe.set_chunk_block(&pos, Block::new(BlockType::Wood));

                            propagate_light(&mut universe, new, LightType::Torch);
                        }

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
            } else {
                //dbg!("no hit");
            }
        }
    }
}
