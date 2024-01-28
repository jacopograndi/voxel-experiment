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
        tests::test::single_block_universe,
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
