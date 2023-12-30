use bevy::{prelude::*, utils::HashSet};
use mcrs_automata::lighting::recalc_lights;
use mcrs_blueprints::Blueprints;
use mcrs_render::voxel_world::VIEW_DISTANCE;
use mcrs_storage::{chunk::Chunk, universe::Universe, CHUNK_SIDE};
use renet::{transport::NetcodeClientTransport, ClientId};

use crate::net::{NetPlayer, NetworkMode};

fn gen_chunk(pos: IVec3, info: &Blueprints) -> Chunk {
    if pos.y < 0 {
        Chunk::filled(info.blocks.get_named("Dirt"))
    } else {
        Chunk::empty()
    }
}

pub fn get_chunks_in_sphere(pos: Vec3) -> HashSet<IVec3> {
    let load_view_distance: u32 = VIEW_DISTANCE;

    let camera_chunk_pos = (pos / CHUNK_SIDE as f32).as_ivec3() * CHUNK_SIDE as i32;
    let load_view_distance_chunk = load_view_distance as i32 / CHUNK_SIDE as i32;
    let lvdc = load_view_distance_chunk;

    // sphere centered on pos
    let mut chunks = HashSet::<IVec3>::new();
    for x in -lvdc..=lvdc {
        for y in -lvdc..=lvdc {
            for z in -lvdc..=lvdc {
                let rel = IVec3::new(x, y, z) * CHUNK_SIDE as i32;
                if rel.as_vec3().length_squared() < load_view_distance.pow(2) as f32 {
                    let pos = camera_chunk_pos + rel;
                    chunks.insert(pos);
                }
            }
        }
    }
    chunks
}

pub fn load_and_gen_chunks(
    mut universe: ResMut<Universe>,
    player_query: Query<(&NetPlayer, &Transform)>,
    network_mode: Res<NetworkMode>,
    transport: Option<Res<NetcodeClientTransport>>,
    info: Res<Blueprints>,
) {
    let client_id = if let Some(transport) = transport {
        Some(ClientId::from_raw(transport.client_id()))
    } else {
        None
    };

    let players_pos = match *network_mode {
        NetworkMode::Client => player_query
            .iter()
            .find(|(player, _)| client_id.map_or(false, |id| id == player.id))
            .map_or(vec![], |(_, tr)| vec![tr.translation]),
        _ => player_query
            .iter()
            .map(|(_, tr)| tr.translation)
            .collect::<Vec<Vec3>>(),
    };

    let mut added = HashSet::<IVec3>::new();
    for player_pos in players_pos.iter() {
        let chunks = get_chunks_in_sphere(*player_pos);
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
