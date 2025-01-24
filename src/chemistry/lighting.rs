use crate::LightSource;
use bevy::prelude::*;
use mcrs_universe::{
    block::{Block, BlockFlag, LightType},
    chunk::Chunk,
    universe::Universe,
    Blueprints, CHUNK_VOLUME,
};
use std::collections::VecDeque;
use std::sync::RwLockWriteGuard;

const MAX_LIGHTING_PROPAGATION: usize = 1000000;
const MAX_DARKNESS_PROPAGATION: usize = 100000;
pub const DIRS: [IVec3; 6] = [
    IVec3::X,
    IVec3::Y,
    IVec3::Z,
    IVec3::NEG_X,
    IVec3::NEG_Y,
    IVec3::NEG_Z,
];

/// Propagate the light from `sources` and return the light sources leaving the chunk.
/// Both `sources` positions and the return leaked sources positions are relative to this chunk
pub fn propagate_light_chunk(
    chunk_mut: &mut RwLockWriteGuard<[Block; CHUNK_VOLUME]>,
    sources: Vec<IVec3>,
    lt: LightType,
) -> Vec<LightSource> {
    debug!(target: "lighting_chunk", "{} sources of {lt} light", sources.len());

    let mut leaking = vec![];

    let mut frontier: VecDeque<IVec3> = sources.into();
    let mut iter = 0;
    while let Some(pos) = frontier.pop_front() {
        if iter >= MAX_LIGHTING_PROPAGATION {
            break;
        }
        let source = chunk_mut[Chunk::xyz2idx(pos)];
        let light = source.get_light(lt);
        for dir in DIRS.iter() {
            let target = pos + *dir;
            if Chunk::contains(&target) {
                let neighbor = &mut chunk_mut[Chunk::xyz2idx(target)];
                if !neighbor.properties.check(BlockFlag::Opaque)
                    && neighbor.get_light(lt) + 2 <= light
                {
                    neighbor.set_light(lt, light - 1);
                    frontier.push_back(target);
                }
            } else if light > 0 {
                leaking.push(LightSource {
                    pos: target,
                    brightness: light - 1,
                });
            }
        }
        iter += 1;
    }

    debug!(target: "lighting_chunk", "{} iters for {lt} light", iter);

    leaking
}

// Remove one light source and set to 0 brightness the volume that was lit by that light source.
// Then return the sources at the boundary and every other light source that was inside.
pub fn propagate_darkness(
    universe: &mut Universe,
    bp: &Blueprints,
    source: IVec3,
    lt: LightType,
) -> Vec<LightSource> {
    let val = if let Some(voxel) = universe.read_chunk_block(&source) {
        let val = voxel.get_light(lt);
        let mut dark = voxel.clone();
        dark.set_light(lt, 0);
        universe.set_chunk_block(&source, dark);
        val
    } else {
        0
    };

    debug!(target: "automata_lighting", "1 source of {lt} darkness val:{val}");

    let mut new_lights: Vec<LightSource> = vec![];
    let mut frontier: VecDeque<IVec3> = [source].into();
    for iter in 0..MAX_DARKNESS_PROPAGATION {
        if let Some(pos) = frontier.pop_front() {
            for dir in DIRS.iter() {
                let target = pos + *dir;
                let mut unlit: Option<Block> = None;
                if let Some(neighbor) = universe.read_chunk_block(&target) {
                    let target_light = neighbor.get_light(lt);
                    if target_light != 0 && target_light < val {
                        let mut l = neighbor;
                        l.set_light(lt, 0);
                        unlit = Some(l);
                        let target_bp = bp.blocks.get(&neighbor.id);
                        if target_bp.is_light_source() {
                            new_lights.push(LightSource {
                                pos: target,
                                brightness: target_bp.light_level,
                            });
                        }
                    } else if target_light >= val {
                        new_lights.push(LightSource {
                            pos: target,
                            brightness: target_light,
                        });
                    }
                }
                if let Some(voxel) = unlit {
                    universe.set_chunk_block(&target, voxel);
                    frontier.push_back(target);
                }
            }
        } else {
            debug!(target: "automata_lighting", "{} iters for {lt} darkness", iter);
            break;
        }
    }
    new_lights
}
