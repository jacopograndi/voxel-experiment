use super::get_spawn_chunks;
use crate::{settings::McrsSettings, Player};
use bevy::prelude::*;
use mcrs_physics::intersect::get_chunks_in_sphere;

pub fn requested_chunks<'a>(
    players: impl Iterator<Item = (&'a Transform, &'a Player)>,
    settings: &'a McrsSettings,
) -> Vec<(IVec3, i32)> {
    let players_pos = players.map(|(tr, _)| tr.translation).collect::<Vec<Vec3>>();

    let mut requested = vec![];

    // Check the spawn chunks
    for chunk_pos in get_spawn_chunks() {
        requested.push((chunk_pos.clone(), 0));
    }

    // Check near every player
    for player_pos in players_pos.iter() {
        let chunks = get_chunks_in_sphere(*player_pos, settings.load_distance_blocks as f32);
        for chunk_pos in chunks.iter() {
            requested.push((
                chunk_pos.clone(),
                (player_pos - chunk_pos.as_vec3()).length() as i32,
            ));
        }
    }

    requested
}
