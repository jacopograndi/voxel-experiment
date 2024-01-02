#[cfg(test)]
mod test {
    use bevy::math::IVec3;
    use mcrs_blueprints::{
        blocks::{BlockBlueprint, BlockId},
        BlueprintList, Blueprints,
    };
    use mcrs_flag_bank::BlockFlag;
    use mcrs_storage::{
        block::{Block, LightType::*},
        chunk::Chunk,
        universe::Universe,
    };

    use crate::lighting::{recalc_lights, DIRS};

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
                assert_eq!(universe.read_chunk_block(&xyz).unwrap().get_light(Torch), 0);
            }
        }
    }

    #[test]
    fn simple_torch_occlusion() {
        let blueprints = debug_blueprints();

        let mut universe = Universe::default();
        universe.chunks.insert(IVec3::new(0, 0, 0), Chunk::empty());
        universe.set_chunk_block(
            &IVec3::new(2, 2, 2),
            Block::new(blueprints.blocks.get_named("Glowstone")),
        );
        universe.set_chunk_block(
            &IVec3::new(2, 2, 3),
            Block::new(blueprints.blocks.get_named("Stone")),
        );

        assert_eq!(
            universe
                .read_chunk_block(&IVec3::new(2, 2, 3))
                .unwrap()
                .get_light(Torch),
            0,
        );

        for i in 0..15 {
            assert_eq!(
                universe
                    .read_chunk_block(&IVec3::new(2 + i, 2, 2))
                    .unwrap()
                    .get_light(Torch),
                0,
            );
        }

        recalc_lights(&mut universe, vec![IVec3::new(0, 0, 0)], &blueprints);

        assert_eq!(
            universe
                .read_chunk_block(&IVec3::new(2, 2, 3))
                .unwrap()
                .get_light(Torch),
            0,
        );

        for i in 0..13 {
            assert_eq!(
                universe
                    .read_chunk_block(&IVec3::new(4 + i, 2, 2))
                    .unwrap()
                    .get_light(Torch),
                13 - i as u8,
            );
        }
    }

    #[test]
    fn simple_torch() {
        let blueprints = debug_blueprints();

        let mut universe = Universe::default();
        universe.chunks.insert(IVec3::new(0, 0, 0), Chunk::empty());
        universe.set_chunk_block(
            &IVec3::new(2, 2, 2),
            Block::new(blueprints.blocks.get_named("Glowstone")),
        );

        for i in 0..15 {
            assert_eq!(
                universe
                    .read_chunk_block(&IVec3::new(2 + i, 2, 2))
                    .unwrap()
                    .get_light(Torch),
                0,
            );
        }

        recalc_lights(&mut universe, vec![IVec3::new(0, 0, 0)], &blueprints);

        for i in 0..15 {
            assert_eq!(
                universe
                    .read_chunk_block(&IVec3::new(2 + i, 2, 2))
                    .unwrap()
                    .get_light(Torch),
                15 - i as u8,
            );
        }
    }

    fn debug_blueprints() -> Blueprints {
        Blueprints {
            blocks: BlueprintList::from_list(vec![
                BlockBlueprint {
                    name: "Air".to_string(),
                    id: BlockId::from_u8(0),
                    flags: vec![],
                    light_level: 0,
                    ..Default::default()
                },
                BlockBlueprint {
                    name: "Stone".to_string(),
                    id: BlockId::from_u8(1),
                    flags: vec![BlockFlag::Collidable, BlockFlag::Opaque],
                    light_level: 0,
                    ..Default::default()
                },
                BlockBlueprint {
                    name: "Glowstone".to_string(),
                    id: BlockId::from_u8(2),
                    flags: vec![BlockFlag::Collidable],
                    light_level: 15,
                    ..Default::default()
                },
            ]),
            ghosts: BlueprintList::from_list(vec![]),
        }
    }
}
