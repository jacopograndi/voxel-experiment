use bevy::{prelude::*, utils::HashSet};
use mcrs_blueprints::Blueprints;
use mcrs_chemistry::lighting::recalc_lights;
use mcrs_physics::{character::CharacterController, intersect::get_chunks_in_sphere};
use mcrs_settings::ViewDistance;
use mcrs_storage::{chunk::Chunk, universe::Universe};

fn gen_chunk(pos: IVec3, info: &Blueprints) -> Chunk {
    if pos.y < 0 {
        Chunk::filled(info.blocks.get_named("Dirt"))
    } else {
        let chunk = Chunk::empty();
        // debug block to see which chunks are loaded
        chunk.set_block(
            IVec3::new(3, 3, 3),
            mcrs_storage::block::Block::new(info.blocks.get_named("Stone")),
        );
        chunk
    }
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
