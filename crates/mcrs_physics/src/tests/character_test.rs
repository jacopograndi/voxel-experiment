use super::{universe_single_block, EPS};
use crate::{
    character::{
        character_controller_step, is_grounded, Character, CharacterController, Friction,
        Rigidbody, Velocity,
    },
    tests::{close_enough, stone},
    MARGIN_EPSILON,
};
use bevy::prelude::*;
use mcrs_universe::universe::Universe;
use std::{
    f32::consts::{FRAC_PI_4, PI},
    time::Duration,
};

#[derive(Clone, Debug, Default)]
struct Context {
    character: Character,
    rigidbody: Rigidbody,
    friction: Friction,
    universe: Universe,
    dt: Duration,
}

impl Context {
    fn new() -> Self {
        Self {
            rigidbody: Rigidbody {
                // Make the character slightly smaller to avoid edge issues in testing
                // See ray's corner_hit
                size: Vec3::splat(1.0 - EPS),
            },
            character: Character {
                air_speed: 0.001,
                ground_speed: 0.03,
                jump_strenght: 0.2,
                jump_cooldown: Duration::from_millis(200),
            },
            friction: Friction {
                air: Vec3::splat(0.99),
                ground: Vec3::splat(0.78),
            },
            universe: universe_single_block(),
            dt: Duration::from_millis(20),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct CharacterState {
    controller: CharacterController,
    tr: Transform,
    vel: Velocity,
}

impl ToString for CharacterState {
    fn to_string(&self) -> String {
        format!("tr:{}, vel:{}", self.tr.translation, self.vel.vel)
    }
}

fn step_cube_character(state: &CharacterState, opt: Option<Context>) -> CharacterState {
    let context = opt.unwrap_or(Context::new());

    // Return a mutated copy
    let mut state = state.clone();

    character_controller_step(
        &mut state.controller,
        &mut state.tr,
        &mut state.vel,
        &context.character,
        &context.rigidbody,
        &context.friction,
        &context.universe,
        context.dt,
    );

    state
}

#[test]
fn gravity() {
    let context = Context::new();
    let mut state = CharacterState::default();
    state.tr.translation = Vec3::splat(0.5) + Vec3::Y * 2.0;

    assert!(
        !is_grounded(&context.rigidbody, &state.tr, &context.universe),
        "grounded"
    );

    let stepped = step_cube_character(&state, Some(context.clone()));

    assert!(
        !is_grounded(&context.rigidbody, &state.tr, &context.universe),
        "grounded"
    );

    dbg!(&state, &stepped);
    assert!(
        state.vel.vel.y > stepped.vel.vel.y,
        "velocity did not become more negative"
    );
    assert!(
        state.tr.translation.y > stepped.tr.translation.y,
        "not moved down"
    );
}

#[test]
fn grounded() {
    let context = Context::new();
    let mut state = CharacterState::default();
    state.tr.translation = Vec3::splat(0.5) + Vec3::Y;

    assert!(
        is_grounded(&context.rigidbody, &state.tr, &context.universe),
        "not grounded"
    );

    let stepped = step_cube_character(&state, Some(context.clone()));

    dbg!(&state, &stepped);
    assert!(
        is_grounded(&context.rigidbody, &state.tr, &context.universe),
        "not grounded"
    );
    assert!(
        state.vel.vel.y == stepped.vel.vel.y,
        "velocity stayed the same"
    );
    assert!(
        state.tr.translation.y == stepped.tr.translation.y,
        "moved down"
    );
}

#[test]
fn jump_while_falling() {
    let mut state = CharacterState::default();
    state.tr.translation = Vec3::splat(0.5) + Vec3::Y * 2.0;
    state.controller.jumping = true;
    let stepped = step_cube_character(&state, None);
    dbg!(&state, &stepped);
    assert!(
        state.vel.vel.y > stepped.vel.vel.y,
        "velocity did not become more negative"
    );
    assert!(
        state.tr.translation.y > stepped.tr.translation.y,
        "not moved down"
    );
}

#[test]
fn jump_once() {
    let context = Context::new();
    let mut state = CharacterState::default();
    state.tr.translation = Vec3::splat(0.5) + Vec3::Y * 1.0;
    state.controller.jumping = true;

    let mut iterated = state.clone();
    let mut iter = 0;
    while iter < 100 {
        if iter > 0 && is_grounded(&context.rigidbody, &iterated.tr, &context.universe) {
            break;
        }
        if iter == 0 {
            state.controller.jumping = false;
        }

        let stepped = step_cube_character(&iterated, Some(context.clone()));
        iterated = stepped;
        iter += 1;
    }

    dbg!(iter);
    dbg!(&state, &iterated);

    assert!(iter < 100, "out of iterations");
    assert!(iter > 0, "stayed grounded after first iteration");
    assert!(iterated.vel.vel.length() < EPS, "character is moving");
    assert!(
        state.tr.translation.distance(iterated.tr.translation) < EPS * 100.0,
        "returned to another position (even with some leeway)"
    );
}

#[test]
fn grounded_then_fall() {
    let context = Context::new();
    let mut state = CharacterState::default();
    state.tr.translation = Vec3::splat(0.5) + Vec3::Y * 1.0;
    state.controller.acceleration.x = 1.0;

    let mut iterated = state.clone();
    let mut iter = 0;
    while iter < 100 {
        if (state.tr.translation - iterated.tr.translation).length() > 1.0 {
            assert!(
                !is_grounded(&context.rigidbody, &iterated.tr, &context.universe),
                "grounded while falling"
            );
            return;
        }

        if !is_grounded(&context.rigidbody, &iterated.tr, &context.universe) {
            panic!("not grounded while on the ground");
        }

        iterated = step_cube_character(&iterated, Some(context.clone()));
        iter += 1;
    }

    assert!(iter < 100, "out of iterations");
}

#[test]
fn bonk_into_wall() {
    for (dir, rot) in [
        (Vec3::X, -PI * 0.5),
        (Vec3::Z, PI),
        (-Vec3::X, PI * 0.5),
        (-Vec3::Z, 0.0),
    ] {
        let mut context = Context::new();
        let u = &mut context.universe;

        let idir = dir.as_ivec3();

        // The floor
        u.set_chunk_block(&(idir * 1), stone());
        u.set_chunk_block(&(idir * 2), stone());
        u.set_chunk_block(&(idir * 3), stone());

        // The wall
        u.set_chunk_block(&(idir * 3 + IVec3::Y), stone());
        u.set_chunk_block(&(idir * 3 + IVec3::Y * 2), stone());

        let mut state = CharacterState::default();
        state.tr.translation = Vec3::splat(0.5) + Vec3::Y;

        // tr.forward is -z
        state.tr.rotation = Quat::from_rotation_y(rot);

        state.controller.acceleration = Vec3::X;

        let mut bonked = false;

        let mut iterated = state.clone();
        let mut iter = 0;
        while iter < 1000 {
            if !is_grounded(&context.rigidbody, &iterated.tr, &context.universe) {
                dbg!(iter, dir, &state, &iterated);
                panic!("no longer grounded");
            }

            let stepped = step_cube_character(&iterated, Some(context.clone()));

            let traveled = (stepped.tr.translation - state.tr.translation).length();
            if iter > 1 && stepped.vel.vel.length_squared() < EPS {
                println!("BONK");
                bonked = true;
            }

            if bonked {
                println!(
                    "i:{}, dir:{}, state:({}), before:({}), after:({})",
                    iter,
                    dir,
                    state.to_string(),
                    iterated.to_string(),
                    stepped.to_string()
                );
                assert!(
                    stepped.vel.vel.length_squared() < EPS,
                    "character started moving after bonking"
                );
                assert!(
                    close_enough(traveled, 2.0, EPS * 1000.0),
                    "wall bonked after traveling {}, but not straight at {}",
                    traveled,
                    2.0
                );
            }

            iterated = stepped;
            iter += 1;
        }

        assert!(bonked);

        // If it reaches here it means that the wall successfully stopped the character
    }
}

#[test]
fn bonk_into_wall_and_jump() {
    for (dir, rot) in [
        (Vec3::X, -PI * 0.5),
        (Vec3::Z, PI),
        (-Vec3::X, PI * 0.5),
        (-Vec3::Z, 0.0),
    ] {
        let mut context = Context::new();
        let u = &mut context.universe;

        let idir = dir.as_ivec3();

        // The floor
        u.set_chunk_block(&(idir * 1), stone());
        u.set_chunk_block(&(idir * 2), stone());
        u.set_chunk_block(&(idir * 3), stone());

        // The wall
        u.set_chunk_block(&(idir * 3 + IVec3::Y), stone());
        u.set_chunk_block(&(idir * 3 + IVec3::Y * 2), stone());

        let mut state = CharacterState::default();
        state.tr.translation = Vec3::splat(0.5) + Vec3::Y * 1.0;

        // tr.forward is -z
        state.tr.rotation = Quat::from_rotation_y(rot);

        state.controller.acceleration = Vec3::X;

        let mut bonked = false;

        let mut iterated = state.clone();
        let mut iter = 0;
        while iter < 1000 {
            if !bonked && !is_grounded(&context.rigidbody, &iterated.tr, &context.universe) {
                dbg!(iter, dir, &state, &iterated);
                panic!("no longer grounded");
            }

            let mut stepped = step_cube_character(&iterated, Some(context.clone()));

            if iter > 1 && stepped.vel.vel.length_squared() < EPS {
                bonked = true;
                stepped.controller.jumping = true;
            }

            if bonked {
                println!(
                    "i:{}, dir:{}, state:({}), before:({}), after:({})",
                    iter,
                    dir,
                    state.to_string(),
                    iterated.to_string(),
                    stepped.to_string()
                );
                let plane = Vec3::new(1.0, 0.0, 1.0);
                let traveled =
                    (stepped.tr.translation * plane - state.tr.translation * plane).length();
                assert!(
                    close_enough(traveled, 2.0, EPS * 1000.0),
                    "wall bonked after traveling {}, but not straight at {}",
                    traveled,
                    2.0
                );
            }

            iterated = stepped;
            iter += 1;
        }

        assert!(bonked);

        // If it reaches here it means that the wall successfully stopped the character
    }
}

#[test]
fn bonk_into_corner_not_jumping() {
    bonk_into_corner(false);
}

#[test]
fn bonk_into_corner_jumping() {
    bonk_into_corner(true);
}

fn bonk_into_corner(start_jumping_after_bonk: bool) {
    for ((s, t), rot) in [
        //((-Vec3::X, -Vec3::Z), FRAC_PI_4 * 1.0),
        //((-Vec3::X, Vec3::Z), FRAC_PI_4 * 3.0),
        ((Vec3::X, Vec3::Z), FRAC_PI_4 * 5.0),
        ((Vec3::X, -Vec3::Z), FRAC_PI_4 * 7.0),
    ] {
        let mut context = Context::new();
        let u = &mut context.universe;

        let (is, it) = (s.as_ivec3(), t.as_ivec3());
        let dir = (s + t).normalize();

        // The floor
        u.set_chunk_block(&is, stone());
        u.set_chunk_block(&it, stone());
        u.set_chunk_block(&(is + it), stone());

        // The wall (knight moves, corner at 2,2 is missing)
        u.set_chunk_block(&((is * 2 + it) + IVec3::Y), stone());
        u.set_chunk_block(&((is * 2 + it) + IVec3::Y * 2), stone());
        u.set_chunk_block(&((it * 2 + is) + IVec3::Y), stone());
        u.set_chunk_block(&((it * 2 + is) + IVec3::Y * 2), stone());

        println!("{}", is);
        println!("{}", it);
        println!("{}", it + is);
        println!("{}", (is * 2 + it) + IVec3::Y);
        println!("{}", (is * 2 + it) + IVec3::Y * 2);
        println!("{}", (it * 2 + is) + IVec3::Y);
        println!("{}", (it * 2 + is) + IVec3::Y * 2);

        let mut state = CharacterState::default();
        state.tr.translation = Vec3::splat(0.5) + Vec3::Y * (1.0 + MARGIN_EPSILON);

        // tr.forward is -z
        state.tr.rotation = Quat::from_rotation_y(rot);

        state.controller.acceleration = Vec3::X;

        let mut bonked = false;
        let mut bonk_pos = Vec3::ZERO;

        dbg!(dir, &state);

        let mut iterated = state.clone();
        let mut iter = 0;
        while iter < 1000 {
            println!();

            if !bonked && !is_grounded(&context.rigidbody, &iterated.tr, &context.universe) {
                //dbg!(iter, dir, &state, &iterated);
                panic!("no longer grounded");
            }

            if iterated.tr.translation.y < 0.0 {
                //dbg!(iter, dir, &state, &iterated);
                panic!("falling to -Y");
            }

            let mut stepped = step_cube_character(&iterated, Some(context.clone()));

            if iter > 1 && stepped.vel.vel.length_squared() < EPS {
                println!("BONK");
                bonked = true;
                stepped.controller.jumping = start_jumping_after_bonk;
                bonk_pos = stepped.tr.translation;
            }

            if bonked {
                //dbg!(iter, dir, &state, &iterated, &stepped);
                println!(
                    "i:{}, dir:{}, state:({}), before:({}), after:({})",
                    iter,
                    dir,
                    state.to_string(),
                    iterated.to_string(),
                    stepped.to_string()
                );
                let plane = Vec3::new(1.0, 0.0, 1.0);
                let from_start_to_bonk = (bonk_pos * plane - state.tr.translation * plane).length();
                let from_start_to_traveled =
                    (stepped.tr.translation * plane - state.tr.translation * plane).length();
                assert!(
                    close_enough(from_start_to_traveled, from_start_to_bonk, 0.1),
                    "wall bonked after traveling {}, but not straight at {}",
                    from_start_to_traveled,
                    from_start_to_bonk
                );
            }

            iterated = stepped;
            iter += 1;
        }

        println!();
        dbg!(iter, dir, &state, &iterated);
        dbg!(&iterated);
        println!();

        let plane = Vec3::new(1.0, 0.0, 1.0);
        let from_start_to_bonk = (bonk_pos * plane - state.tr.translation * plane).length();
        let from_start_to_traveled =
            (iterated.tr.translation * plane - state.tr.translation * plane).length();
        assert!(
            close_enough(from_start_to_traveled, from_start_to_bonk, 0.1),
            "wall bonked after traveling {}, but not straight at {}",
            from_start_to_traveled,
            from_start_to_bonk
        );

        assert!(bonked);

        // If it reaches here it means that the wall successfully stopped the character
    }
}
