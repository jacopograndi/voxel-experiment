use std::time::Duration;

use bevy::prelude::*;
use mcrs_universe::universe::Universe;

use crate::{raycast::*, test_print, MARGIN_EPSILON};

/// An axis aligned cuboid that can be stepped forward in time.
/// Paired with `Velocity` and `Friction`
#[derive(Component, Debug, Clone, Default)]
pub struct Rigidbody {
    // The extent of the bounding box.
    pub size: Vec3,
}

/// Defines the properties of how a character is impacted by forces
/// Paired with `Rigidbody`, `Velocity` and `Friction`
#[derive(Component, Debug, Clone, Default)]
pub struct Character {
    // The speed multiplier when not standing on the ground.
    pub air_speed: f32,

    // The speed multiplier when standing on the ground.
    pub ground_speed: f32,

    // The force applied upwards when a jump is started.
    pub jump_strenght: f32,

    // The amount of time that must pass before jumping again.
    pub jump_cooldown: Duration,
}

/// The `Rigidbody` velocity component
#[derive(Component, Debug, Clone, Default)]
pub struct Velocity {
    pub vel: Vec3,
}

/// Removes some speed per tick to a `Rigidbody` based on the movement type of the body
#[derive(Component, Debug, Clone, Default)]
pub struct Friction {
    pub air: Vec3,
    pub ground: Vec3,
}

/// Tells the `Character` how to move
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

// Todo: Maybe move in src
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

/// Checks if a rigidbody is very close to a block below
pub fn is_grounded(rigidbody: &Rigidbody, tr: &Transform, universe: &Universe) -> bool {
    cast_cuboid(
        RayFinite {
            position: tr.translation,
            direction: Vec3::NEG_Y,
            reach: MARGIN_EPSILON * 2.,
        },
        rigidbody.size,
        &universe,
    )
    .is_some_and(|hit| hit.distance <= MARGIN_EPSILON * 2.0)
}

// Todo: add a system that moves `Rigidbody`s that are not `Character`s
// It would handle items. Projectiles and mobs would be handled separately.

/// System to step forward the `Character` using the values from the `CharacterController`
pub fn character_controller_movement(
    mut character_query: Query<(
        &mut CharacterController,
        &mut Transform,
        &mut Velocity,
        &Character,
        &Rigidbody,
        &Friction,
    )>,
    universe: Res<Universe>,
    time: Res<Time<Fixed>>,
) {
    for (mut controller, mut tr, mut vel, character, rigidbody, friction) in
        character_query.iter_mut()
    {
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
            &rigidbody,
            &friction,
            &universe,
            time.delta(),
        );
    }
}

/// Move a `Character` using the `CharacterController`, then step the `Rigidbody`
pub fn character_controller_step(
    controller: &mut CharacterController,
    tr: &mut Transform,
    vel: &mut Velocity,
    character: &Character,
    rigidbody: &Rigidbody,
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
    if is_grounded(rigidbody, &tr, &universe) {
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

    rigidbody_step(tr, vel, rigidbody, universe);
}

/// Move a `Rigidbody` towards a `Velocity` and solve collisions with the `Universe`.
pub fn rigidbody_step(
    tr: &mut Transform,
    vel: &mut Velocity,
    rigidbody: &Rigidbody,
    universe: &Universe,
) {
    if vel.vel.length_squared() < MARGIN_EPSILON {
        return;
    }

    let mut vel_magnitude = vel.vel.length();
    let mut vel_dir = vel.vel.normalize_or_zero();

    // Loop until vel_magnitude is <= 0.0, but implementation bugs may lead to an infinite loop.
    for _i in 0..10 {
        let hit = cast_cuboid(
            RayFinite {
                position: tr.translation,
                direction: vel_dir.normalize_or_zero(),
                reach: vel_magnitude + 1.0,
            },
            rigidbody.size,
            &universe,
        );

        // Check if the trajectory is clear
        if hit.is_none() || hit.as_ref().is_some_and(|hit| hit.distance > vel_magnitude) {
            test_print(format!(
                "rigidbody: added just: m:{}, d:{}, to tr:{}",
                vel_magnitude, vel_dir, tr.translation
            ));

            // Speed Bleed: This operation removes some speed.
            let vel_delta = vel_magnitude.clamp(0.0, vel_magnitude - MARGIN_EPSILON * 2.0);
            tr.translation += vel_dir * vel_delta;

            rigidbody_margin_check(tr, rigidbody, universe);
            return;
        }

        // Handle the collision
        let Some(hit) = hit else {
            return;
        };

        // Speed Bleed: This operation removes some speed.
        let vel_delta = hit
            .distance
            .clamp(0.0, vel_magnitude - MARGIN_EPSILON * 2.0);
        vel_magnitude -= vel_delta;

        test_print(format!(
            "rigidbody: added bonk: m:{}, d:{}, n:{}, to tr:{}",
            vel_delta,
            vel_dir,
            hit.normal(),
            tr.translation,
        ));

        // Apply the translation
        // Now the character is touching the block it hit
        tr.translation += vel_dir * vel_delta;

        // Add some margin back so that the character isn't touching the block anymore
        // Note: the margin is added to all directions because if the character hits a block
        // near a corner it's possible that it gets too close to the corner.
        // Speed Bleed: This operation removes some speed.
        tr.translation -= hit.grid_step.as_vec3() * (MARGIN_EPSILON * 0.5);

        let wall = (IVec3::ONE - hit.mask).as_vec3();

        rigidbody_margin_check(tr, rigidbody, universe);

        // Remove the velocity component that has hit a wall
        let vel_dir_bonk = vel_dir * wall;
        vel_dir = vel_dir_bonk.normalize_or_zero();
        vel.vel = vel.vel * wall;

        // Normalize vel_magnitude to account for having removed a component of vel_dir
        let lost_bonking = vel_dir_bonk.length();
        vel_magnitude = if lost_bonking != 0.0 {
            vel_magnitude * lost_bonking
        } else {
            0.0
        };

        if vel_magnitude <= MARGIN_EPSILON * 2.0 {
            // Finished moving the rigidbody along vel
            test_print(format!("rigidbody: out of gas: m:{}", vel_magnitude));
            return;
        }
    }
}

/// Checks if the rigidbody is too close to a block and moves it back by `MARGIN_EPSILON`
fn rigidbody_margin_check(tr: &mut Transform, rigidbody: &Rigidbody, universe: &Universe) {
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
            rigidbody.size,
            &universe,
        ) {
            if perp_hit.distance <= MARGIN_EPSILON * 2.0 {
                // Remove the component that is too close to the margin
                tr.translation *= (IVec3::ONE - perp_hit.mask).as_vec3();

                // Recalculate the position by placing the character touching the margin
                let center_of_the_bonked_block = perp_hit.grid_pos.as_vec3() + Vec3::ONE * 0.5;
                let touching_the_margin =
                    perp_hit.normal().as_vec3() * (0.5 + MARGIN_EPSILON + rigidbody.size * 0.5);
                tr.translation +=
                    perp_hit.mask.as_vec3() * (center_of_the_bonked_block + touching_the_margin);

                test_print(format!(
                    "rigidbody: move out of margin {}, bound {}",
                    tr.translation, bound
                ));
            }
        }
    }
}
