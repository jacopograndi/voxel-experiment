use bevy::prelude::*;

use voxel_storage::chunk_map::ChunkMap;

use crate::{
    raycast::{raycast, sweep_aabb},
    MARGIN_EPSILON,
};

#[derive(Debug, Clone, Default)]
pub struct CharacterId(pub u32);

#[derive(Component, Debug, Clone, Default)]
pub struct Character {
    pub id: CharacterId,
    pub size: Vec3,
    pub speed: f32,
}

#[derive(Component, Debug, Clone, Default)]
pub struct Velocity {
    pub vel: Vec3,
}

#[derive(Component, Debug, Clone, Default)]
pub struct Friction {
    pub air: Vec3,
    pub ground: Vec3,
}

#[derive(Component, Debug, Clone, Default)]
pub struct CharacterController {
    pub acceleration: Vec3,
}

pub fn movement(
    mut character_query: Query<(
        &Character,
        &CharacterController,
        &mut Transform,
        &mut Velocity,
        &mut Friction,
    )>,
    chunk_map: Res<ChunkMap>,
) {
    for (character, controller, mut tr, mut vel, friction) in character_query.iter_mut() {
        vel.vel += controller.acceleration * character.speed;
        vel.vel -= Vec3::Y * 0.02;

        for _ in 0..4 {
            if let Some(hit) = sweep_aabb(
                tr.translation,
                character.size,
                vel.vel.normalize_or_zero(),
                vel.vel.length(),
                &chunk_map,
            ) {
                tr.translation += vel.vel.normalize_or_zero() * (hit.distance - MARGIN_EPSILON);
                vel.vel *= (IVec3::ONE - hit.blocked).as_vec3();
                if vel.vel.length() < MARGIN_EPSILON {
                    break;
                }
            }
        }
        tr.translation += vel.vel;
        /*
        if let Some(hit) = raycast(
            tr.translation,
            vel.vel.normalize_or_zero(),
            20.0,
            &chunk_map,
        ) {
            if hit.distance <= dist {
                tr.translation += vel.vel.normalize_or_zero() * (hit.distance - MARGIN_EPSILON);
                vel.vel = Vec3::ZERO;
                //tr.translation += vel.vel * time.delta_seconds();
            } else {
                tr.translation += vel.vel * time.delta_seconds();
            }
        } else {
            tr.translation += vel.vel * time.delta_seconds();
        }
        */
        vel.vel *= friction.air;
        /*
        if let Some(hit) = raycast(tr.translation, Vec3::NEG_Y, &chunk_map) {
            if hit.distance <= MARGIN_EPSILON {
                vel.vel *= friction.ground;
            } else {
                vel.vel *= friction.air;
            }
        }
        */
    }
}
