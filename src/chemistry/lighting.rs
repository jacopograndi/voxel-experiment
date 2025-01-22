use bevy::prelude::*;
use mcrs_universe::{
    block::{Block, BlockFlag, LightType},
    chunk::Chunk,
    universe::Universe,
    CHUNK_VOLUME,
};
use std::collections::VecDeque;
use std::sync::RwLockWriteGuard;

use crate::LightSource;

/*
pub fn recalc_lights(
    universe: &mut Universe,
    request: &mut ChunkGenerationRequest,
    chunks: Vec<IVec3>,
    bp: &Blueprints,
) {
    debug!(target: "automata_lighting", chunks = ?chunks);

    // calculate sunlight beams
    let mut suns: Vec<IVec3> = vec![];
    let mut planars = HashSet::<IVec2>::new();
    let mut highest = i32::MIN;
    for chunk_pos in chunks.iter() {
        let Some(chunk) = get_chunk_mut_at(universe, request, chunk_pos) else {
            error!("lighting: the chunk at {} was missing", chunk_pos);
            continue;
        };

        chunk.version.update();
        for x in 0..CHUNK_SIDE {
            for z in 0..CHUNK_SIDE {
                let mut sunlight = MAX_LIGHT;
                for y in (0..CHUNK_SIDE).rev() {
                    let xyz = IVec3::new(x as i32, y as i32, z as i32);
                    if chunk.read_block(xyz).properties.check(BlockFlag::Opaque) {
                        sunlight = 0;
                    }
                    if sunlight > 0 {
                        suns.push(*chunk_pos + xyz);
                    }
                    chunk.set_block_light(xyz, LightType::Sun, sunlight);
                    chunk.set_block_light(xyz, LightType::Torch, 0);
                    highest = highest.max(chunk_pos.y + y as i32);
                }
                let planar = IVec2::new(x as i32 + chunk_pos.x, z as i32 + chunk_pos.z);
                planars.insert(planar);
            }
        }
    }

    for planar in planars.iter() {
        let mut beam = 0;
        let mut block_found = false;
        for y in 0..1000 {
            let h = highest - y;
            let sample = IVec3::new(planar.x, h, planar.y);

            if let Some(voxel) = universe.read_chunk_block(&sample) {
                block_found = true;
                if voxel.properties.check(BlockFlag::Opaque) {
                    beam = h;
                    break;
                }
            } else {
                if block_found {
                    break;
                }
            }
        }
        if let Some(height) = universe.heightfield.get_mut(planar) {
            *height = (*height).min(beam);
        } else {
            universe.heightfield.insert(*planar, beam);
        }
    }

    // find new light sources
    let mut torches: Vec<IVec3> = vec![];
    for pos in chunks.iter() {
        let chunk = universe.chunks.get_mut(pos).unwrap();
        for xyz in Chunk::iter() {
            let id = chunk.read_block(xyz).id;
            let block_bp = bp.blocks.get(&id);
            if block_bp.is_light_source() {
                torches.push(*pos + xyz);
                chunk.set_block_light(xyz, LightType::Torch, block_bp.light_level);
            }
        }
    }

    if !suns.is_empty() {
        propagate_light(universe, suns, LightType::Sun);
    }

    if !torches.is_empty() {
        propagate_light(universe, torches, LightType::Torch);
    }
}
*/

pub const DIRS: [IVec3; 6] = [
    IVec3::X,
    IVec3::Y,
    IVec3::Z,
    IVec3::NEG_X,
    IVec3::NEG_Y,
    IVec3::NEG_Z,
];
const MAX_LIGHTING_PROPAGATION: usize = 100000;

pub fn propagate_darkness(universe: &mut Universe, source: IVec3, lt: LightType) -> Vec<IVec3> {
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

    let mut new_lights: Vec<IVec3> = vec![];
    let mut frontier: VecDeque<IVec3> = [source].into();
    for iter in 0..MAX_LIGHTING_PROPAGATION {
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
                    } else if target_light >= val {
                        new_lights.push(target);
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

// Todo: Modify so that the light that it operates only on one chunk.
// Return the light sources leaving the chunk.
// `sources` positions are relative to this chunk
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

pub fn propagate_light(universe: &mut Universe, sources: Vec<IVec3>, lt: LightType) {
    debug!(target: "automata_lighting", "{} sources of {lt} light", sources.len());

    let mut frontier: VecDeque<IVec3> = sources.clone().into();
    for iter in 0..MAX_LIGHTING_PROPAGATION {
        if let Some(pos) = frontier.pop_front() {
            let Some(voxel) = universe.read_chunk_block(&pos) else {
                continue;
            };
            let light = voxel.get_light(lt);
            for dir in DIRS.iter() {
                let target = pos + *dir;
                let mut lit: Option<Block> = None;
                if let Some(neighbor) = universe.read_chunk_block(&target) {
                    if !neighbor.properties.check(BlockFlag::Opaque)
                        && neighbor.get_light(lt) + 2 <= light
                    {
                        let mut l = neighbor;
                        l.set_light(lt, light - 1);
                        lit = Some(l);
                    }
                }
                if let Some(voxel) = lit {
                    universe.set_chunk_block(&target, voxel);
                    frontier.push_back(target);
                }
            }
        } else {
            debug!(target: "automata_lighting", "{} iters for {lt} light", iter);
            break;
        }
    }
}
