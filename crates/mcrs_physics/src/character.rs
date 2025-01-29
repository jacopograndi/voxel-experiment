use std::time::Duration;

use bevy::prelude::*;
use mcrs_universe::universe::Universe;

use crate::{raycast::*, MARGIN_EPSILON};

#[derive(Component, Debug, Clone, Default)]
pub struct Character {
    // The extent of the bounding box.
    pub size: Vec3,

    // The speed multiplier when not standing on the ground.
    pub air_speed: f32,

    // The speed multiplier when standing on the ground.
    pub ground_speed: f32,

    // The force applied upwards when a jump is started.
    pub jump_strenght: f32,

    // The amount of time that must pass before jumping again.
    pub jump_cooldown: Duration,
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
    // The acceleration applied to Self.
    pub acceleration: Vec3,

    // The jump command. If this is true a jump will be attempted.
    pub jumping: bool,

    // Tracks the amount of time passed from the start of the last jump.
    pub jump_timer: Timer,

    // If Self isn't active any input will be dropped, gravity and other forces will be applied.
    pub is_active: bool,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            acceleration: Vec3::default(),
            jumping: false,
            jump_timer: Timer::default(),
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

/// Step forward the `Character` using the values from the `CharacterController`
pub fn character_controller_movement(
    mut character_query: Query<(
        &mut CharacterController,
        &mut Transform,
        &mut Velocity,
        &Character,
        &Friction,
    )>,
    universe: Res<Universe>,
    time: Res<Time<Fixed>>,
) {
    for (mut controller, mut tr, mut vel, character, friction) in character_query.iter_mut() {
        let (chunk_pos, _) = universe.pos_to_chunk_and_inner(&tr.translation.as_ivec3());
        let waiting_for_loading = universe.chunks.get(&chunk_pos).is_none();
        if waiting_for_loading {
            continue;
        }

        character_controller_step(
            &mut controller,
            &mut tr,
            &mut vel,
            &character,
            &friction,
            &universe,
            time.delta(),
        );
    }
}

/// Move a Character using the CharacterController and solve collisions with the Universe.
pub fn character_controller_step(
    controller: &mut CharacterController,
    tr: &mut Transform,
    vel: &mut Velocity,
    character: &Character,
    friction: &Friction,
    universe: &Universe,
    dt: Duration,
) {
    if !controller.is_active {
        controller.acceleration = Vec3::ZERO;
        controller.jumping = false;
    }

    controller.jump_timer.tick(dt);

    let acc = controller.acceleration.x * tr.forward() + controller.acceleration.z * tr.left();
    if is_grounded(character, &tr, &universe) {
        if controller.jumping && controller.jump_timer.finished() {
            vel.vel.y = character.jump_strenght;
            controller.jump_timer.set_duration(character.jump_cooldown);
            controller.jump_timer.reset();
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
