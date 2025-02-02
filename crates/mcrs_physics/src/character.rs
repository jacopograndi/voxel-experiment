use std::time::Duration;

use bevy::prelude::*;
use mcrs_universe::universe::Universe;

use crate::{raycast::*, test_trace, MARGIN_EPSILON};

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
    .is_some_and(|hit| hit.final_distance() <= MARGIN_EPSILON * 2.0)
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

    if vel.vel.length_squared() < MARGIN_EPSILON {
        return;
    }

    let mut vel_magnitude = vel.vel.length();
    let mut vel_dir = vel.vel.normalize_or_zero();

    // Loop until vel_magnitude is <= 0.0, but implementation bugs may lead to an infinite loop.
    for i in 0..10 {
        if let Some(hit) = cast_cuboid(
            RayFinite {
                position: tr.translation,
                direction: vel_dir,
                reach: vel_magnitude,
            },
            character.size,
            &universe,
        ) {
            // Project into normal for correct max bound
            let vel_delta = (hit.final_distance() - MARGIN_EPSILON).max(0.0);
            vel_magnitude -= vel_delta;

            tr.translation += vel_dir * vel_delta;

            let wall = (IVec3::ONE - hit.mask).as_vec3();

            test_trace(format!(
                "{}, m:{}, D:{}, d:{}, w:{}",
                i, vel_magnitude, vel_delta, vel_dir, wall
            ));

            // Get the distance from the character's boundary and the block it hit.
            let leading_vertex = get_leading_aabb_vertex(character.size, vel_dir);
            let perp_vec = leading_vertex.fract() * wall - hit.normal().as_vec3();
            let perp_distance = perp_vec.length();

            if perp_distance < MARGIN_EPSILON {
                tr.translation += hit.normal().as_vec3() * perp_distance;
            }

            // Remove the velocity component that has hit a wall
            vel_dir *= wall;
            vel.vel *= wall;

            if vel_magnitude < MARGIN_EPSILON {
                test_trace(format!("out of gas: m:{}", vel_magnitude));
                break;
            }
        } else {
            println!("added: m:{}, d:{}", vel_magnitude, vel_dir);
            tr.translation += vel_dir * vel_magnitude;
            break;
        }
    }
}
