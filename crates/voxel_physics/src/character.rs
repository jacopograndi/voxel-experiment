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
    time: Res<Time>,
) {
    for (character, controller, mut tr, mut vel, friction) in character_query.iter_mut() {
        vel.vel += controller.acceleration * character.speed * time.delta_seconds();
        /*
        if let Some(hit) = sweep_aabb(tr.translation, character.size, vel.vel, &chunk_map) {
            vel.vel.x *= if hit.blocked.x { 0.0 } else { 1.0 };
            vel.vel.y *= if hit.blocked.y { 0.0 } else { 1.0 };
            vel.vel.z *= if hit.blocked.z { 0.0 } else { 1.0 };
            tr.translation += vel.vel * time.delta_seconds();
        } else {
            tr.translation += vel.vel * time.delta_seconds();
        }
        */
        if vel.vel.length_squared() == 0.0 {
            return;
        }
        let dist = (vel.vel * time.delta_seconds()).length();
        if let Some(hit) = raycast(tr.translation, vel.vel.normalize_or_zero(), &chunk_map) {
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
        if let Some(hit) = raycast(tr.translation, Vec3::NEG_Y, &chunk_map) {
            if hit.distance == 0.0 {
                vel.vel *= friction.ground;
            } else {
                vel.vel *= friction.air;
            }
        }
    }
}
