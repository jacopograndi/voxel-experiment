#[cfg(test)]
mod test {
    use std::f32::consts::PI;

    use crate::raycast::{get_leading_aabb_vertex, raycast, sweep_aabb};
    use bevy::{prelude::*, utils::HashMap};
    use voxel_storage::{
        BlockID,
        block::Block,
        chunk::Chunk,
        universe::Universe
    };

    #[test]
    fn empty_out_of_range() {
        let chunk_map = Universe {
            chunks: HashMap::new(),
        };
        assert_eq!(None, raycast(Vec3::ZERO, Vec3::X, 100.0, &chunk_map));
    }

    #[test]
    fn just_of_range() {
        let map = single_block_map();
        let center = Vec3::ONE * 0.5;
        assert_eq!(
            None,
            raycast(center + Vec3::X * 1.5, -Vec3::X, 0.999999, &map)
        );
        assert!(raycast(center + Vec3::X * 1.5, -Vec3::X, 1.000001, &map).is_some());
    }

    #[test]
    fn sweep_just_out_of_range() {
        let map = single_block_map();
        let center = Vec3::ONE * 0.5;
        assert_eq!(
            None,
            sweep_aabb(center + Vec3::X * 2.0, Vec3::ONE, -Vec3::X, 0.999999, &map)
        );
        assert!(sweep_aabb(center + Vec3::X * 2.0, Vec3::ONE, -Vec3::X, 1.000001, &map).is_some());
    }

    #[test]
    fn zero_length() {
        let map = single_block_map();
        let center = Vec3::ONE * 0.5;
        assert!(raycast(center + Vec3::X * 0.5, -Vec3::X, 0.000001, &map).is_some());
    }

    #[test]
    fn sweep_zero_length() {
        let map = single_block_map();
        let center = Vec3::ONE * 0.5;
        assert!(sweep_aabb(center + Vec3::X * 1.0, Vec3::ONE, -Vec3::X, 0.000001, &map).is_some());
    }

    #[test]
    fn axis_aligned() {
        let map = single_block_map();
        let axis = vec![Vec3::X, Vec3::Y, Vec3::Z];
        let dirs = axis
            .iter()
            .map(|v| vec![*v, -*v])
            .flatten()
            .collect::<Vec<Vec3>>();
        let center = Vec3::ONE * 0.5;
        for dir in dirs {
            let start = center - dir;
            if let Some(hit) = raycast(start, dir, 2.0, &map) {
                assert_eq!(hit.distance, 0.5);
            } else {
                println!("No hit for ray ({}, {}) in single block map", start, dir);
                panic!();
            }
        }
    }

    #[test]
    fn bombard_face() {
        let map = single_block_map();
        let center = Vec3::ONE * 0.5 + Vec3::Z * 0.5;
        for angle in 1..180 {
            let rot = Quat::from_rotation_y(angle as f32 / 360.0 * PI);
            let dir = rot * Vec3::X;
            let start = center - dir;
            if let Some(hit) = raycast(start, dir, 2.0, &map) {
                assert!(close_enough(hit.distance, 1.0));
            } else {
                println!("No hit for ray ({}, {}) in single block map", start, dir);
                panic!();
            }
        }
    }

    #[test]
    fn corner_hit() {
        let map = single_block_map();
        let start = -Vec3::ONE;
        let dir = Vec3::ONE.normalize();
        if let Some(hit) = raycast(start, dir, 2.0, &map) {
            assert!(close_enough(hit.distance, Vec3::ONE.length()));
        } else {
            println!("No hit for ray ({}, {}) in single block map", start, dir);
            panic!();
        }
    }

    fn single_block_map() -> Universe {
        let mut chunk_map = Universe {
            chunks: [(
                IVec3::ZERO,
                Chunk::empty(),
            )]
            .into_iter()
            .collect(),
        };
        chunk_map.set_chunk(
            &IVec3::ZERO,
            BlockID::STONE,
        );
        assert_eq!(
            Some(Block::new(BlockID::STONE)),
            chunk_map.read_chunk(&IVec3::ZERO)
        );
        chunk_map
    }

    fn single_chunk_map() -> Universe {
        let mut chunk_map = Universe {
            chunks: [(
                IVec3::ZERO,
                Chunk::filled(),
            )]
            .into_iter()
            .collect(),
        };
        chunk_map.set_chunk(
            &IVec3::ZERO,
            BlockID::STONE,
        );
        chunk_map
    }

    fn close_enough(a: f32, b: f32) -> bool {
        const EPS: f32 = 0.0001;
        (a - EPS..a + EPS).contains(&b)
    }

    #[test]
    fn leading_vertex() {
        let size = Vec3::new(1.0, 2.0, 3.0);
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if x == 0 || y == 0 || z == 0 {
                        continue;
                    }
                    let sample = Vec3::new(x as f32, y as f32, z as f32);
                    let direction = sample.normalize();
                    println!("{}", direction);
                    assert_eq!(
                        get_leading_aabb_vertex(size, direction),
                        size * sample * 0.5
                    );
                }
            }
        }
    }
}
