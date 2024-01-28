#[cfg(test)]
mod character {
    use std::time::Duration;

    use bevy::{
        prelude::*,
        time::{TimePlugin, TimeUpdateStrategy},
    };
    use mcrs_storage::universe::Universe;

    use crate::{
        character::*,
        plugin::{McrsPhysicsPlugin, PhysicsSet},
        tests::test::{add_block, close_enough_vec, single_block_universe},
    };

    #[test]
    fn floating() {
        let (mut app, entity) = test_app(Vec3::new(0.5, 3.0, 0.5));
        assert!(!is_character_grounded(&app, entity));
        app.update();
        assert!(!is_character_grounded(&app, entity));
    }

    #[test]
    fn grounded_middle() {
        let (mut app, entity) = test_app(Vec3::new(0.5, 2.0, 0.5));
        assert!(is_character_grounded(&app, entity));
        app.update();
        assert!(is_character_grounded(&app, entity));
    }

    #[test]
    fn jumping() {
        let (mut app, entity) = test_app(Vec3::new(0.5, 2.0, 0.5));
        {
            let mut controller = app.world.get_mut::<CharacterController>(entity).unwrap();
            controller.jumping = true;
        }
        assert!(is_character_grounded(&app, entity));
        app.update();
        assert!(!is_character_grounded(&app, entity));
    }

    #[test]
    fn jumping_hug_corner() {
        for corner in [
            IVec3::new(1, 0, 1),
            IVec3::new(-1, 0, 1),
            IVec3::new(-1, 0, -1),
            IVec3::new(1, 0, -1),
        ] {
            let (mut app, entity) = test_app(Vec3::new(0.5, 2.0, 0.5));
            add_block(&mut app, IVec3::new(1, 1, 0));
            add_block(&mut app, IVec3::new(0, 1, 1));
            add_block(&mut app, IVec3::new(-1, 1, 0));
            add_block(&mut app, IVec3::new(0, 1, -1));
            let mut last_position = Vec3::ZERO;
            let mut i = 0;
            while i < 10 {
                // move against the corner
                app.update();
                assert!(is_character_grounded(&app, entity));
                let current_position = {
                    let mut controller = app.world.get_mut::<CharacterController>(entity).unwrap();
                    // negative sign is because acceleration is scaled by tr.forward
                    // z and x are swapped because x * tr.forward and z * tr.left
                    controller.acceleration = -corner.zyx().as_vec3().normalize();
                    let tr = app.world.get::<Transform>(entity).unwrap();
                    tr.translation
                };
                if close_enough_vec(current_position, last_position) {
                    break;
                } else {
                    last_position = current_position;
                }
                i += 1;
            }
            assert!(i < 10, "player couldn't reach the corner");
            {
                // jump
                let mut controller = app.world.get_mut::<CharacterController>(entity).unwrap();
                controller.acceleration = -corner.as_vec3().normalize();
                controller.jumping = true;
            }
            app.update();
            assert!(!is_character_grounded(&app, entity));
            let velocity_after_jump = {
                let velocity = app.world.get_mut::<Velocity>(entity).unwrap();
                velocity.vel
            };
            for _ in 0..10 {
                app.update();
                assert!(!is_character_grounded(&app, entity));
                {
                    let velocity = app.world.get_mut::<Velocity>(entity).unwrap();
                    if velocity.vel.y < 0.0 {
                        break;
                    }
                    assert!(
                        velocity.vel.y < velocity_after_jump.y,
                        "player is accelerating upwards"
                    );
                    println!("velocity: {}", velocity.vel);
                };
            }
        }
    }

    fn test_app(character_translation: Vec3) -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(single_block_universe());
        app.add_plugins(McrsPhysicsPlugin);
        app.add_plugins(TimePlugin);
        app.configure_sets(FixedUpdate, PhysicsSet::Update);
        // Run the FixedUpdate every app.update()
        app.world
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                1. / 64.,
            )));
        let entity = app
            .world
            .spawn((
                SpatialBundle {
                    transform: Transform {
                        translation: character_translation,
                        ..default()
                    },
                    ..default()
                },
                Character {
                    size: Vec3::new(0.5, 2.0, 0.5),
                    air_speed: 0.001,
                    ground_speed: 0.03,
                    jump_strenght: 0.2,
                },
                CharacterController {
                    acceleration: Vec3::splat(0.0),
                    jumping: false,
                    ..default()
                },
                Velocity::default(),
                Friction {
                    air: Vec3::splat(0.99),
                    ground: Vec3::splat(0.78),
                },
            ))
            .id();
        // The first update ignores FixedUpdate
        app.update();
        (app, entity)
    }

    fn is_character_grounded(app: &App, entity: Entity) -> bool {
        let character = app.world.get::<Character>(entity).unwrap();
        let tr = app.world.get::<Transform>(entity).unwrap();
        let universe = app.world.resource::<Universe>();
        println!("character at {}", tr.translation);
        is_grounded(character, tr, universe)
    }
}
