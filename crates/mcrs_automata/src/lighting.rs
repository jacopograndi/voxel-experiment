use std::collections::VecDeque;

use bevy::{prelude::*, utils::HashSet};
use mcrs_blueprints::Blueprints;
use mcrs_flag_bank::BlockFlag;
use mcrs_storage::{
    block::{Block, LightType, MAX_LIGHT},
    chunk::Chunk,
    universe::Universe,
    CHUNK_SIDE, CHUNK_VOLUME,
};

pub fn recalc_lights(universe: &mut Universe, chunks: Vec<IVec3>, info: &Blueprints) {
    println!("lighting {:?} chunks", chunks.len());

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
        for i in 0..CHUNK_VOLUME {
            let xyz = Chunk::_idx2xyz(i);
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
    let voxel = universe.read_chunk_block(&source).unwrap();
    let val = voxel.get_light(lt);
    let mut dark = voxel.clone();
    dark.set_light(lt, 0);
    universe.set_chunk_block(&source, dark);

    println!("1 source of {lt} darkness val:{val}");

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
                    let (c, _) = universe.pos_to_chunk_and_inner(&target);
                    universe.chunks.get_mut(&c).unwrap().dirty_render = true;
                    universe.chunks.get_mut(&c).unwrap().dirty_replication = true;
                }
            }
        } else {
            println!("{} iters for {lt} darkness", iter);
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

    println!("{} sources of {lt} light", sources.len());
    let mut frontier: VecDeque<IVec3> = sources.clone().into();
    for iter in 0..MAX_LIGHTITNG_PROPAGATION {
        if let Some(pos) = frontier.pop_front() {
            let voxel = universe.read_chunk_block(&pos).unwrap();
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
                    let (c, _) = universe.pos_to_chunk_and_inner(&target);
                    universe.chunks.get_mut(&c).unwrap().dirty_render = true;
                    universe.chunks.get_mut(&c).unwrap().dirty_replication = true;
                }
            }
        } else {
            println!("{} iters for {lt} light", iter);
            break;
        }
    }
}
