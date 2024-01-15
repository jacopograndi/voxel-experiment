use bevy::{prelude::*, utils::HashSet};
use mcrs_blueprints::Blueprints;
use mcrs_chemistry::lighting::recalc_lights;
use mcrs_physics::{character::CharacterController, intersect::get_chunks_in_sphere};
use mcrs_settings::ViewDistance;
use mcrs_storage::{block::Block, chunk::Chunk, universe::Universe};
use noise::{NoiseFn, OpenSimplex, RidgedMulti, Seedable};

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
    view_distance: Res<ViewDistance>,
) {
    let players_pos = player_query
        .iter()
        .map(|(_, tr)| tr.translation)
        .collect::<Vec<Vec3>>();

    let mut added = HashSet::<IVec3>::new();
    for player_pos in players_pos.iter() {
        let chunks = get_chunks_in_sphere(*player_pos, view_distance.0 as f32);
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
