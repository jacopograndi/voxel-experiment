use std::sync::RwLockWriteGuard;

use crate::terrain_generation::generator::Generator;
use bevy::prelude::*;
use mcrs_universe::{block::Block, Blueprints, CHUNK_VOLUME};
use noise::{Exponent, HybridMulti, MultiFractal, NoiseFn, Perlin, Seedable};

pub struct GeneratorCrazyHill {
    terrain_noise: Exponent<f64, HybridMulti<Perlin>, 2>,
    sponge_noise: HybridMulti<Perlin>,
}

impl Default for GeneratorCrazyHill {
    fn default() -> Self {
        Self::new(0)
    }
}

impl GeneratorCrazyHill {
    fn new(seed: u32) -> Self {
        Self {
            terrain_noise: Exponent::new(
                HybridMulti::<Perlin>::default()
                    .set_frequency(0.001)
                    .set_octaves(4)
                    .set_seed(seed),
            ),
            sponge_noise: HybridMulti::<Perlin>::default()
                .set_frequency(0.003)
                .set_octaves(5)
                .set_persistence(0.5)
                .set_seed(seed),
        }
    }
}

impl Generator for GeneratorCrazyHill {
    fn gen_block(&self, pos: IVec3, bp: &Blueprints) -> Block {
        // create an envelope of 3d noise in -192..192
        // squish that envelope in the y direction using a 2d perlin noise

        let dpos = pos.as_dvec3();

        let block;
        let caves: f64 = -128.0;
        let sky: f64 = 128.0;
        let mid = (sky + caves) * 0.5;
        let amp = (sky - caves).abs() * 0.5;
        if dpos.y > sky {
            block = bp.blocks.get_named("Air");
        } else if dpos.y < caves {
            block = bp.blocks.get_named("Stone");
        } else {
            let flatness = self.terrain_noise.get(dpos.xz().to_array());
            let flatness_norm = flatness * 0.5 + 0.5;
            if dpos.y < mid - amp * flatness_norm {
                block = bp.blocks.get_named("Stone");
            } else if dpos.y > mid + amp * flatness_norm {
                block = bp.blocks.get_named("Air");
            } else {
                let sample = self.sponge_noise.get(dpos.to_array());
                if sample > 1.0 - flatness_norm {
                    block = bp.blocks.get_named("Stone");
                } else {
                    block = bp.blocks.get_named("Air");
                }
            }
        }

        Block::new(block)
    }

    fn gen_biome(
        &self,
        chunk_mut: RwLockWriteGuard<[Block; CHUNK_VOLUME]>,
        chunk_pos: IVec3,
        bp: &Blueprints,
    ) {
    }

    fn gen_structures(
        &self,
        chunk_mut: RwLockWriteGuard<[Block; CHUNK_VOLUME]>,
        chunk_pos: IVec3,
        bp: &Blueprints,
    ) {
    }

    fn lighting(
        &self,
        chunk_mut: RwLockWriteGuard<[Block; CHUNK_VOLUME]>,
        chunk_pos: IVec3,
        bp: &Blueprints,
    ) {
    }
}
