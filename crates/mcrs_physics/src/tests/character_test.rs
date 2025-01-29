use super::{universe_single_block, EPS};
use crate::{
    character::{
        character_controller_step, is_grounded, Character, CharacterController, Friction, Velocity,
    },
    tests::stone,
};
use bevy::{math::NormedVectorSpace, prelude::*};
use mcrs_universe::universe::Universe;
use std::time::Duration;

#[derive(Clone, Debug, Default)]
struct Context {
    character: Character,
    friction: Friction,
    universe: Universe,
    dt: Duration,
}

impl Context {
    fn new() -> Self {
        Self {
            character: Character {
                // Make the character slightly smaller to avoid edge issues in testing
                // See ray's corner_hit
                size: Vec3::splat(1.0 - EPS),
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

fn step_cube_character(state: &CharacterState, opt: Option<Context>) -> CharacterState {
    let context = opt.unwrap_or(Context::new());

    // Return a mutated copy
    let mut state = state.clone();

    character_controller_step(
        &mut state.controller,
        &mut state.tr,
        &mut state.vel,
        &context.character,
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
        !is_grounded(&context.character, &state.tr, &context.universe),
        "grounded"
    );

    let stepped = step_cube_character(&state, Some(context.clone()));

    assert!(
        !is_grounded(&context.character, &state.tr, &context.universe),
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
        is_grounded(&context.character, &state.tr, &context.universe),
        "not grounded"
    );

    let stepped = step_cube_character(&state, Some(context.clone()));

    dbg!(&state, &stepped);
    assert!(
        is_grounded(&context.character, &state.tr, &context.universe),
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
        if iter > 0 && is_grounded(&context.character, &iterated.tr, &context.universe) {
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
        state.tr.translation.distance(iterated.tr.translation) < EPS * 10.0,
        "returned to another position (even with some leeway)"
    );
}

#[test]
fn run_into_wall() {
    let mut context = Context::new();
    let u = &mut context.universe;

    // The floor
    u.set_chunk_block(&IVec3::new(0, 0, 0), stone());
    u.set_chunk_block(&IVec3::new(0, 0, -1), stone());
    u.set_chunk_block(&IVec3::new(0, 0, -2), stone());

    // The wall
    u.set_chunk_block(&IVec3::new(0, 1, -3), stone());
    u.set_chunk_block(&IVec3::new(0, 2, -3), stone());

    let mut state = CharacterState::default();
    state.tr.translation = Vec3::splat(0.5) + Vec3::Y * 1.0;
    state.controller.acceleration = Vec3::X;

    let mut iterated = state.clone();
    let mut iter = 0;
    while iter < 1000 {
        if !is_grounded(&context.character, &iterated.tr, &context.universe) {
            dbg!(iter, &state, &iterated);
            panic!("no longer grounded");
        }

        let stepped = step_cube_character(&iterated, Some(context.clone()));
        if stepped.tr.translation.x > 1.0
            && stepped.tr.translation.x.distance(iterated.tr.translation.x) < EPS
        {
            dbg!(iter, &state, &stepped, &iterated);
            assert!(
                iterated.vel.vel.length() < EPS,
                "character is moving after smashing into the wall"
            );
        }
        iterated = stepped;
        iter += 1;
    }
}
