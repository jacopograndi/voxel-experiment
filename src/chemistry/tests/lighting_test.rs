#[cfg(test)]
mod test {
    use bevy::math::IVec3;
    use mcrs_blueprints::{blocks::BlockBlueprint, flagbank::BlockFlag, BlueprintList, Blueprints};
    use mcrs_storage::{
        block::{
            Block,
            LightType::{self, *},
        },
        chunk::Chunk,
        universe::Universe,
    };

    use crate::lighting::{propagate_darkness, propagate_light, recalc_lights, DIRS};

    #[test]
    fn two_torches_remove_one() {
        let blueprints = debug_blueprints();

        let light_pos0 = IVec3::new(2, 2, 2);
        let light_pos1 = IVec3::new(2, 4, 2);

        let mut universe_single = Universe::default();
        universe_single
            .chunks
            .insert(IVec3::new(0, 0, 0), Chunk::empty());
        universe_single.set_chunk_block(
            &light_pos0,
            Block::new(blueprints.blocks.get_named("Glowstone")),
        );
        recalc_lights(&mut universe_single, vec![IVec3::new(0, 0, 0)], &blueprints);

        let mut universe_double = Universe::default();
        universe_double
            .chunks
            .insert(IVec3::new(0, 0, 0), Chunk::empty());
        universe_double.set_chunk_block(
            &light_pos0,
            Block::new(blueprints.blocks.get_named("Glowstone")),
        );
        universe_double.set_chunk_block(
            &light_pos1,
            Block::new(blueprints.blocks.get_named("Glowstone")),
        );
        recalc_lights(&mut universe_double, vec![IVec3::new(0, 0, 0)], &blueprints);
        let new_lights = propagate_darkness(&mut universe_double, light_pos1, Torch);
        universe_double
            .set_chunk_block(&light_pos1, Block::new(blueprints.blocks.get_named("Air")));
        propagate_light(&mut universe_double, new_lights, Torch);

        for xyz in Chunk::iter() {
            let light0 = universe_single
                .read_chunk_block(&xyz)
                .unwrap()
                .get_light(Torch);
            let light1 = universe_double
                .read_chunk_block(&xyz)
                .unwrap()
                .get_light(Torch);
            assert_eq!(light0, light1, "at {}", xyz,);
        }
    }

    #[test]
    fn torch_place_remove() {
        let blueprints = debug_blueprints();

        let light_pos = IVec3::new(2, 2, 2);
        let mut universe = Universe::default();
        universe.chunks.insert(IVec3::new(0, 0, 0), Chunk::empty());
        universe.set_chunk_block(
            &light_pos,
            Block::new(blueprints.blocks.get_named("Glowstone")),
        );

        recalc_lights(&mut universe, vec![IVec3::new(0, 0, 0)], &blueprints);
        propagate_darkness(&mut universe, light_pos, LightType::Torch);
        universe.set_chunk_block(&light_pos, Block::new(blueprints.blocks.get_named("Air")));

        for xyz in Chunk::iter() {
            let light = universe.read_chunk_block(&xyz).unwrap().get_light(Torch);
            assert_eq!(light, 0, "at {}", xyz);
        }
    }

    #[test]
    fn torch_fully_occluded() {
        let blueprints = debug_blueprints();

        let mut universe = Universe::default();
        universe.chunks.insert(IVec3::new(0, 0, 0), Chunk::empty());
        universe.set_chunk_block(
            &IVec3::new(2, 2, 2),
            Block::new(blueprints.blocks.get_named("Glowstone")),
        );
        for dir in DIRS.iter() {
            universe.set_chunk_block(
                &(IVec3::new(2, 2, 2) + *dir),
                Block::new(blueprints.blocks.get_named("Stone")),
            );
        }

        recalc_lights(&mut universe, vec![IVec3::new(0, 0, 0)], &blueprints);

        for xyz in Chunk::iter() {
            if xyz != IVec3::new(2, 2, 2) {
                let light = universe.read_chunk_block(&xyz).unwrap().get_light(Torch);
                assert_eq!(light, 0, "at {}", xyz);
            }
        }
    }

    #[test]
    fn simple_torch_occlusion() {
        let blueprints = debug_blueprints();
        let light_pos = IVec3::new(2, 2, 2);
        let stone_pos = IVec3::new(2, 2, 3);

        let mut universe = Universe::default();
        universe.chunks.insert(IVec3::new(0, 0, 0), Chunk::empty());
        universe.set_chunk_block(
            &light_pos,
            Block::new(blueprints.blocks.get_named("Glowstone")),
        );
        universe.set_chunk_block(&stone_pos, Block::new(blueprints.blocks.get_named("Stone")));

        let light = universe
            .read_chunk_block(&stone_pos)
            .unwrap()
            .get_light(Torch);
        assert_eq!(light, 0, "at {}", stone_pos);

        for i in 1..15 {
            let xyz = light_pos + IVec3::Z * i;
            let light = universe.read_chunk_block(&xyz).unwrap().get_light(Torch);
            assert_eq!(light, 0, "at {}", xyz);
        }

        recalc_lights(&mut universe, vec![IVec3::new(0, 0, 0)], &blueprints);

        let light = universe
            .read_chunk_block(&stone_pos)
            .unwrap()
            .get_light(Torch);
        assert_eq!(
            light, 0,
            "at {}, base light is {}, should be 0",
            stone_pos, light
        );

        for i in 1..13 {
            let xyz = stone_pos + IVec3::Z * i;
            let light = universe.read_chunk_block(&xyz).unwrap().get_light(Torch);
            // 14 13 12
            // 15 ## 11
            // 14 13 12
            assert_eq!(light, 12 - i as u8, "at {}", xyz);
        }
    }

    #[test]
    fn simple_torch() {
        let blueprints = debug_blueprints();
        let light_pos = IVec3::new(2, 2, 2);

        let mut universe = Universe::default();
        universe.chunks.insert(IVec3::new(0, 0, 0), Chunk::empty());

        universe.set_chunk_block(
            &IVec3::new(2, 2, 2),
            Block::new(blueprints.blocks.get_named("Glowstone")),
        );

        for i in 1..15 {
            let xyz = light_pos + IVec3::Z * i;
            let light = universe.read_chunk_block(&xyz).unwrap().get_light(Torch);
            assert_eq!(light, 0, "at {}", xyz);
        }

        recalc_lights(&mut universe, vec![IVec3::new(0, 0, 0)], &blueprints);

        for i in 0..15 {
            let xyz = light_pos + IVec3::Z * i;
            let light = universe.read_chunk_block(&xyz).unwrap().get_light(Torch);
            assert_eq!(light, 15 - i as u8, "at {}", xyz);
        }
    }

    fn debug_blueprints() -> Blueprints {
        Blueprints {
            blocks: BlueprintList::from_list(vec![
                BlockBlueprint {
                    name: "Air".to_string(),
                    id: 0.into(),
                    flags: vec![],
                    light_level: 0,
                    ..Default::default()
                },
                BlockBlueprint {
                    name: "Stone".to_string(),
                    id: 1.into(),
                    flags: vec![BlockFlag::Collidable, BlockFlag::Opaque],
                    light_level: 0,
                    ..Default::default()
                },
                BlockBlueprint {
                    name: "Glowstone".to_string(),
                    id: 2.into(),
                    flags: vec![BlockFlag::Collidable],
                    light_level: 15,
                    ..Default::default()
                },
            ]),
            ghosts: BlueprintList::from_list(vec![]),
        }
    }
}
