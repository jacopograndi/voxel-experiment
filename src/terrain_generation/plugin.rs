use super::{
    generator::{
        generate_chunks_system, setup_chunk_generation, ChunkGen, Generator, GeneratorLoaded,
    },
    generators::crazy_hills::GeneratorCrazyHill,
    sun_beams::SunBeams,
};
use crate::FixedMainSet;
use bevy::prelude::*;

pub struct TerrainGenerationPlugin;

impl Plugin for TerrainGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SunBeams>();
        app.insert_resource(ChunkGen::default());

        app.add_systems(Startup, setup_chunk_generation);

        // Change here the generator for testing
        add_generator::<GeneratorCrazyHill>(app);
    }
}

pub fn add_generator<Gen: Generator>(app: &mut App) {
    app.insert_resource(GeneratorLoaded::<Gen>::default());
    app.add_systems(
        FixedUpdate,
        generate_chunks_system::<Gen>.in_set(FixedMainSet::Terrain),
    );
}
