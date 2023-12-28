use bevy::prelude::*;

use mcrs_storage::universe::Universe;

use crate::{raycast::*, MARGIN_EPSILON};

#[derive(Debug, Clone, Default)]
pub struct CharacterId(pub u32);

#[derive(Component, Debug, Clone, Default)]
pub struct Character {
    pub id: CharacterId,
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

#[derive(Component, Debug, Clone, Default)]
pub struct CharacterController {
    pub acceleration: Vec3,
    pub jumping: bool,
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

pub fn character_controller_movement(
    mut character_query: Query<
        (
            &Character,
            &CharacterController,
            &mut Transform,
            &mut Velocity,
            &Friction,
        ),
        Without<CameraController>,
    >,
    universe: Res<Universe>,
) {
    for (character, controller, mut tr, mut vel, friction) in character_query.iter_mut() {
        let acc = controller.acceleration.x * tr.forward() + controller.acceleration.z * tr.left();
        vel.vel -= Vec3::Y * 0.01;

        let mut grounded = false;
        if let Some(hit) = sweep_aabb(
            tr.translation,
            character.size,
            Vec3::NEG_Y,
            MARGIN_EPSILON * 2.,
            &universe,
        ) {
            if hit.distance <= MARGIN_EPSILON * 2. {
                grounded = true;
                if controller.jumping {
                    vel.vel += Vec3::Y * character.jump_strenght;
                    grounded = false;
                }
            } else {
                grounded = false;
            }
        }
        if grounded {
            vel.vel += acc * Vec3::new(1.0, 0.0, 1.0) * character.ground_speed;
            vel.vel *= friction.ground;
        } else {
            vel.vel += acc * Vec3::new(1.0, 0.0, 1.0) * character.air_speed;
            vel.vel *= friction.air;
        }

        for _ in 0..4 {
            if let Some(hit) = sweep_aabb(
                tr.translation,
                character.size,
                vel.vel.normalize_or_zero(),
                vel.vel.length(),
                &universe,
            ) {
                tr.translation += vel.vel.normalize_or_zero() * (hit.distance - MARGIN_EPSILON);
                vel.vel *= (IVec3::ONE - hit.blocked).as_vec3();
                if vel.vel.length() < MARGIN_EPSILON {
                    break;
                }
            }
        }
        tr.translation += vel.vel;
    }
}
