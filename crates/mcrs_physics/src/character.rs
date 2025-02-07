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
    .is_some_and(|hit| hit.distance <= MARGIN_EPSILON * 2.0)
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
                direction: vel_dir.normalize_or_zero(),
                reach: vel_magnitude + 1.0,
            },
            character.size,
            &universe,
        ) {
            if hit.distance > vel_magnitude {
                test_trace(format!(
                    "added just: m:{}, d:{}, to tr:{}",
                    vel_magnitude, vel_dir, tr.translation
                ));
                println!(
                    "added just: m:{}, d:{}, to tr:{}",
                    vel_magnitude, vel_dir, tr.translation
                );

                let vel_delta = hit.distance.clamp(0.0, vel_magnitude - MARGIN_EPSILON);
                tr.translation += vel_dir * vel_delta;

                // Bound checks
                for bound in [
                    IVec3::X,
                    IVec3::Y,
                    IVec3::Z,
                    IVec3::NEG_X,
                    IVec3::NEG_Y,
                    IVec3::NEG_Z,
                ] {
                    if let Some(perp_hit) = cast_cuboid(
                        RayFinite {
                            position: tr.translation,
                            direction: bound.as_vec3(),
                            reach: 1.0,
                        },
                        character.size,
                        &universe,
                    ) {
                        if perp_hit.distance <= MARGIN_EPSILON * 2.0 {
                            tr.translation *= (IVec3::ONE - perp_hit.mask).as_vec3();
                            tr.translation += perp_hit.mask.as_vec3()
                                * (perp_hit.grid_pos.as_vec3()
                                    + Vec3::ONE * 0.5
                                    + perp_hit.normal().as_vec3()
                                        * (0.5 + MARGIN_EPSILON + character.size * 0.5));
                            println!("move out of margin {}, bound {}", tr.translation, bound);
                        }
                    }
                }

                return;
            }

            let vel_delta = hit.distance.clamp(0.0, vel_magnitude);
            vel_magnitude -= vel_delta;

            println!(
                "added bonk: m:{}, d:{}, n:{}, to tr:{}",
                vel_delta,
                vel_dir,
                hit.normal(),
                tr.translation
            );

            tr.translation += vel_dir * vel_delta;

            let wall = (IVec3::ONE - hit.mask).as_vec3();

            test_trace(format!(
                "{}, m:{}, D:{}, d:{}, w:{}",
                i, vel_magnitude, vel_delta, vel_dir, wall
            ));

            // Bound checks
            for bound in [
                IVec3::X,
                IVec3::Y,
                IVec3::Z,
                IVec3::NEG_X,
                IVec3::NEG_Y,
                IVec3::NEG_Z,
            ] {
                if let Some(perp_hit) = cast_cuboid(
                    RayFinite {
                        position: tr.translation,
                        direction: bound.as_vec3(),
                        reach: 1.0,
                    },
                    character.size,
                    &universe,
                ) {
                    let perp_distance = perp_hit.distance;
                    if perp_distance <= MARGIN_EPSILON {
                        tr.translation *= (IVec3::ONE - perp_hit.mask).as_vec3();
                        tr.translation += perp_hit.mask.as_vec3()
                            * (perp_hit.grid_pos.as_vec3()
                                + Vec3::ONE * 0.5
                                + perp_hit.normal().as_vec3()
                                    * (0.5 + MARGIN_EPSILON + character.size * 0.5));
                        println!("move out of margin {}, bound {}", tr.translation, bound);
                    }
                }
            }

            // Remove the velocity component that has hit a wall
            let vel_dir_bonk = vel_dir * wall;
            vel_dir = vel_dir_bonk.normalize_or_zero();
            vel.vel = vel.vel * wall;

            let lost_bonking = vel_dir_bonk.length();
            vel_magnitude = if lost_bonking != 0.0 {
                vel_magnitude * lost_bonking
            } else {
                0.0
            };

            if vel_magnitude <= MARGIN_EPSILON * 2.0 {
                test_trace(format!("out of gas: m:{}", vel_magnitude));
                break;
            }
        } else {
            test_trace(format!(
                "added: m:{}, d:{}, to tr:{}",
                vel_magnitude, vel_dir, tr.translation
            ));
            println!(
                "added: m:{}, d:{}, to tr:{}",
                vel_magnitude, vel_dir, tr.translation
            );

            let vel_delta = vel_magnitude.clamp(0.0, vel_magnitude - MARGIN_EPSILON);
            tr.translation += vel_dir * vel_delta;

            // Bound checks
            for bound in [
                IVec3::X,
                IVec3::Y,
                IVec3::Z,
                IVec3::NEG_X,
                IVec3::NEG_Y,
                IVec3::NEG_Z,
            ] {
                if let Some(perp_hit) = cast_cuboid(
                    RayFinite {
                        position: tr.translation,
                        direction: bound.as_vec3(),
                        reach: 1.0,
                    },
                    character.size,
                    &universe,
                ) {
                    let perp_distance = perp_hit.distance;
                    if perp_distance <= MARGIN_EPSILON {
                        tr.translation *= (IVec3::ONE - perp_hit.mask).as_vec3();
                        tr.translation += perp_hit.mask.as_vec3()
                            * (perp_hit.grid_pos.as_vec3()
                                + Vec3::ONE * 0.5
                                + perp_hit.normal().as_vec3()
                                    * (0.5 + MARGIN_EPSILON + character.size * 0.5));
                        println!("move out of margin {}, bound {}", tr.translation, bound);
                    }
                }
            }

            break;
        }
    }
}
