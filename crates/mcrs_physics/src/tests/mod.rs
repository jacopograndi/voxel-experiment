mod character_test;
mod raycast_test;

#[cfg(test)]
mod test {
    use bevy::{math::IVec3, prelude::default, utils::HashMap};
    use mcrs_blueprints::{
        blocks::{BlockBlueprint, BlockId},
        flagbank::BlockFlag,
    };
    use mcrs_storage::{block::Block, chunk::Chunk, universe::Universe};

    pub fn single_block_universe() -> Universe {
        let mut universe = Universe {
            chunks: [(IVec3::ZERO, Chunk::empty())].into_iter().collect(),
            heightfield: HashMap::new(),
        };
        let stone = Block::new(&BlockBlueprint {
            name: "Stone".to_string(),
            id: BlockId::from_u8(1),
            flags: vec![BlockFlag::Collidable],
            ..default()
        });
        universe.set_chunk_block(&IVec3::ZERO, stone);
        assert_eq!(Some(stone), universe.read_chunk_block(&IVec3::ZERO));
        universe
    }

    pub fn close_enough(a: f32, b: f32) -> bool {
        const EPS: f32 = 0.0001;
        (a - EPS..a + EPS).contains(&b)
    }
}
