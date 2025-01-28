mod character_test;
mod raycast_test;

#[cfg(test)]
mod test {
    use bevy::{
        app::App,
        math::{IVec3, Vec3},
        prelude::default,
    };
    use mcrs_universe::{
        block::{Block, BlockBlueprint, BlockFlag, FlagBank},
        chunk::Chunk,
        universe::Universe,
    };

    pub fn universe_single_block() -> Universe {
        let mut universe = Universe {
            chunks: [(IVec3::ZERO, Chunk::empty())].into_iter().collect(),
        };
        universe.set_chunk_block(&IVec3::ZERO, stone());
        universe
    }

    pub fn stone() -> Block {
        Block::new(&BlockBlueprint {
            name: "Stone".to_string(),
            id: 1.into(),
            flags: FlagBank::from(vec![BlockFlag::Collidable]),
            ..default()
        })
    }

    pub fn add_block(app: &mut App, pos: IVec3) {
        let mut universe = app.world_mut().get_resource_mut::<Universe>().unwrap();
        universe.set_chunk_block(&pos, stone());
    }

    // floats are no fun
    pub const EPS: f32 = 0.00001;

    pub fn close_enough_vec(a: Vec3, b: Vec3) -> bool {
        (-EPS..EPS).contains(&(a - b).length())
    }

    pub const DIRS: [Vec3; 6] = [
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
        Vec3::NEG_X,
        Vec3::NEG_Y,
        Vec3::NEG_Z,
    ];
}
