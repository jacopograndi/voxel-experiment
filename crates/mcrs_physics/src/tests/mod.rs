mod character_test;
mod raycast_test;

#[cfg(test)]
mod test {
    use bevy::{
        app::App,
        math::{IVec3, Vec3},
        prelude::default,
        utils::HashMap,
    };
    use mcrs_universe::{
        block::BlockFlag,
        block::{Block, BlockBlueprint},
        chunk::Chunk,
        universe::Universe,
    };

    pub fn single_block_universe() -> Universe {
        let mut universe = Universe {
            chunks: [(IVec3::ZERO, Chunk::empty())].into_iter().collect(),
            heightfield: HashMap::new(),
        };
        let stone = Block::new(&BlockBlueprint {
            name: "Stone".to_string(),
            id: 1.into(),
            flags: vec![BlockFlag::Collidable],
            ..default()
        });
        universe.set_chunk_block(&IVec3::ZERO, stone);
        assert_eq!(Some(stone), universe.read_chunk_block(&IVec3::ZERO));
        universe
    }

    pub fn add_block(app: &mut App, pos: IVec3) {
        let mut universe = app.world.get_resource_mut::<Universe>().unwrap();
        let stone = Block::new(&BlockBlueprint {
            name: "Stone".to_string(),
            id: 1.into(),
            flags: vec![BlockFlag::Collidable],
            ..default()
        });
        universe.set_chunk_block(&pos, stone);
    }

    // floats are no fun
    const EPS: f32 = 0.0001;
    pub fn close_enough(a: f32, b: f32) -> bool {
        (a - EPS..a + EPS).contains(&b)
    }

    pub fn close_enough_vec(a: Vec3, b: Vec3) -> bool {
        (-EPS..EPS).contains(&(a - b).length())
    }
}
