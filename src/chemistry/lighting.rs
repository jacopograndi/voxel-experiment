use std::collections::VecDeque;

use bevy::{prelude::*, utils::HashSet};
use mcrs_universe::{
    block::BlockFlag,
    block::{Block, LightType},
    chunk::Chunk,
    universe::Universe,
    Blueprints, CHUNK_SIDE, MAX_LIGHT,
};

pub fn recalc_lights(universe: &mut Universe, chunks: Vec<IVec3>, info: &Blueprints) {
    debug!(target: "automata_lighting", chunks = ?chunks);

    // calculate sunlight beams
    let mut suns: Vec<IVec3> = vec![];
    let mut planars = HashSet::<IVec2>::new();
    let mut highest = i32::MIN;
    for pos in chunks.iter() {
        let chunk = universe.chunks.get_mut(pos).unwrap();
        chunk.dirty_render = true;
        chunk.dirty_replication = true;
        for x in 0..CHUNK_SIDE {
            for z in 0..CHUNK_SIDE {
                let mut sunlight = MAX_LIGHT;
                for y in (0..CHUNK_SIDE).rev() {
                    let xyz = IVec3::new(x as i32, y as i32, z as i32);
                    if chunk.read_block(xyz).properties.check(BlockFlag::Opaque) {
                        sunlight = 0;
                    }
                    if sunlight > 0 {
                        suns.push(*pos + xyz);
                    }
                    chunk.set_block_light(xyz, LightType::Sun, sunlight);
                    chunk.set_block_light(xyz, LightType::Torch, 0);
                    highest = highest.max(pos.y + y as i32);
                }
                let planar = IVec2::new(x as i32 + pos.x, z as i32 + pos.z);
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
        let chunk = universe.chunks.get(pos).unwrap();
        for xyz in Chunk::iter() {
            let id = chunk.read_block(xyz).id;
            if info.blocks.get(&id).is_light_source() {
                torches.push(*pos + xyz);
                chunk.set_block_light(xyz, LightType::Torch, 15);
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

pub const DIRS: [IVec3; 6] = [
    IVec3::X,
    IVec3::Y,
    IVec3::Z,
    IVec3::NEG_X,
    IVec3::NEG_Y,
    IVec3::NEG_Z,
];
const MAX_LIGHTITNG_PROPAGATION: usize = 100000000;

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
    for iter in 0..MAX_LIGHTITNG_PROPAGATION {
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

pub fn propagate_light(universe: &mut Universe, sources: Vec<IVec3>, lt: LightType) {
    const DIRS: [IVec3; 6] = [
        IVec3::X,
        IVec3::Y,
        IVec3::Z,
        IVec3::NEG_X,
        IVec3::NEG_Y,
        IVec3::NEG_Z,
    ];
    const MAX_LIGHTITNG_PROPAGATION: usize = 100000000;

    debug!(target: "automata_lighting", "{} sources of {lt} light", sources.len());

    let mut frontier: VecDeque<IVec3> = sources.clone().into();
    for iter in 0..MAX_LIGHTITNG_PROPAGATION {
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
