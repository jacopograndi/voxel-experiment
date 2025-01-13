use bevy::prelude::*;
use mcrs_universe::universe::Universe;

use crate::{raycast::*, MARGIN_EPSILON};

#[derive(Component, Debug, Clone, Default)]
pub struct Character {
    pub size: Vec3,
    pub air_speed: f32,
    pub ground_speed: f32,
    pub jump_strenght: f32,
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

#[derive(Component, Debug, Clone)]
pub struct CharacterController {
    pub acceleration: Vec3,
    pub jumping: bool,
    pub is_active: bool,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            acceleration: Vec3::default(),
            jumping: false,
            is_active: true,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct CameraController {
    pub sensitivity: Vec3,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            sensitivity: Vec3::splat(0.00012),
        }
    }
}

pub fn is_grounded(character: &Character, tr: &Transform, universe: &Universe) -> bool {
    cast_cuboid(
        RayFinite {
            position: tr.translation,
            direction: Vec3::NEG_Y,
            reach: MARGIN_EPSILON * 2.,
        },
        character.size,
        &universe,
    )
    .is_some_and(|hit| hit.distance() <= MARGIN_EPSILON * 2.0)
}

pub fn character_controller_movement(
    mut character_query: Query<(
        &Character,
        &CharacterController,
        &mut Transform,
        &mut Velocity,
        &Friction,
    )>,
    universe: Res<Universe>,
) {
    for (character, controller, mut tr, mut vel, friction) in character_query.iter_mut() {
        if !controller.is_active {
            continue;
        }

        let (chunk_pos, _) = universe.pos_to_chunk_and_inner(&tr.translation.as_ivec3());
        let waiting_for_loading = universe.chunks.get(&chunk_pos).is_none();
        if waiting_for_loading {
            continue;
        }

        let acc = controller.acceleration.x * tr.forward() + controller.acceleration.z * tr.left();
        if is_grounded(character, &tr, &universe) {
            if controller.jumping {
                vel.vel.y = character.jump_strenght;
            }
            vel.vel += acc * Vec3::new(1.0, 0.0, 1.0) * character.ground_speed;
            vel.vel *= friction.ground;
        } else {
            vel.vel += acc * Vec3::new(1.0, 0.0, 1.0) * character.air_speed;
            vel.vel *= friction.air;
            vel.vel -= Vec3::Y * 0.01;
        }

        for _ in 0..3 {
            if let Some(hit) = cast_cuboid(
                RayFinite {
                    position: tr.translation,
                    direction: vel.vel.normalize_or_zero(),
                    reach: vel.vel.length(),
                },
                character.size,
                &universe,
            ) {
                tr.translation += vel.vel.normalize_or_zero() * (hit.distance() - MARGIN_EPSILON);
                vel.vel *= (IVec3::ONE - hit.mask).as_vec3();
                if vel.vel.length() < MARGIN_EPSILON {
                    break;
                }
            }
        }
        tr.translation += vel.vel;
    }
}
