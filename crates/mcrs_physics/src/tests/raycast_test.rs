#[cfg(test)]
mod test {
    use std::f32::consts::PI;

    use crate::raycast::{cast_cuboid, cast_ray, get_leading_aabb_vertex, RayFinite};
    use bevy::{prelude::*, utils::HashMap};
    use mcrs_blueprints::{
        blocks::{BlockBlueprint, BlockId},
        flagbank::BlockFlag,
    };
    use mcrs_storage::{block::Block, chunk::Chunk, universe::Universe};

    #[test]
    fn empty_out_of_range() {
        let universe = Universe {
            chunks: HashMap::new(),
            heightfield: HashMap::new(),
        };
        assert!(cast_ray(
            RayFinite {
                position: Vec3::ZERO,
                direction: Vec3::X,
                reach: 100.0
            },
            &universe
        )
        .is_none());
    }

    #[test]
    fn adjacent() {
        let universe = single_block_universe();
        let center = Vec3::ONE * 0.5;
        assert!(cast_ray(
            RayFinite {
                position: center - Vec3::X * 0.5,
                direction: Vec3::X,
                reach: 0.1
            },
            &universe
        )
        .is_some());
        assert!(cast_ray(
            RayFinite {
                position: center + Vec3::X * 0.5,
                direction: -Vec3::X,
                reach: 0.1
            },
            &universe
        )
        .is_some());
        assert!(cast_ray(
            RayFinite {
                position: center,
                direction: Vec3::new(-1.0, -1.0, 0.0).normalize(),
                reach: 0.1
            },
            &universe
        )
        .is_some());
    }

    #[test]
    fn giant_step() {
        let universe = single_block_universe();
        let center = Vec3::ONE * 0.5;
        assert!(cast_ray(
            RayFinite {
                position: center + Vec3::X * 0.7,
                direction: Vec3::new(-100.0, -1.0, 0.0).normalize(),
                reach: 0.1,
            },
            &universe
        )
        .is_none());
    }

    #[test]
    fn just_of_range() {
        let universe = single_block_universe();
        let center = Vec3::ONE * 0.5;
        assert!(cast_ray(
            RayFinite {
                position: center + Vec3::X * 1.5,
                direction: -Vec3::X,
                reach: 0.999999
            },
            &universe
        )
        .is_none());
        assert!(cast_ray(
            RayFinite {
                position: center + Vec3::X * 1.5,
                direction: -Vec3::X,
                reach: 1.000001
            },
            &universe
        )
        .is_some());
    }

    #[test]
    fn sweep_just_out_of_range() {
        let universe = single_block_universe();
        let center = Vec3::ONE * 0.5;
        assert!(cast_cuboid(
            RayFinite {
                position: center + Vec3::X * 2.0,
                direction: -Vec3::X,
                reach: 0.999999,
            },
            Vec3::ONE,
            &universe
        )
        .is_none());
        assert!(cast_cuboid(
            RayFinite {
                position: center + Vec3::X * 2.0,
                direction: -Vec3::X,
                reach: 1.000001,
            },
            Vec3::ONE,
            &universe
        )
        .is_some());
    }

    #[test]
    fn sweep_adjacent() {
        let universe = single_block_universe();
        let center = Vec3::ONE * 0.5;
        assert!(cast_cuboid(
            RayFinite {
                position: center + Vec3::X * 1.0,
                direction: -Vec3::X,
                reach: 1.0,
            },
            Vec3::ONE,
            &universe
        )
        .is_some());
        assert!(cast_cuboid(
            RayFinite {
                position: center + Vec3::X * 1.00001,
                direction: Vec3::new(-1.0, -1.0, 0.0).normalize(),
                reach: 1.0,
            },
            Vec3::ONE,
            &universe
        )
        .is_some());
        assert!(cast_cuboid(
            RayFinite {
                position: center + Vec3::X * 1.0,
                direction: Vec3::new(-1.0, -1.0, 0.0).normalize(),
                reach: 1.0,
            },
            Vec3::ONE,
            &universe
        )
        .is_some());
    }

    #[test]
    fn zero_length() {
        let universe = single_block_universe();
        let center = Vec3::ONE * 0.5;
        assert!(cast_ray(
            RayFinite {
                position: center + Vec3::X * 0.5,
                direction: -Vec3::X,
                reach: 0.000001
            },
            &universe
        )
        .is_some());
    }

    #[test]
    fn sweep_zero_length() {
        let universe = single_block_universe();
        let center = Vec3::ONE * 0.5;
        assert!(cast_cuboid(
            RayFinite {
                position: center + Vec3::X * 1.0,
                direction: -Vec3::X,
                reach: 0.000001,
            },
            Vec3::ONE,
            &universe
        )
        .is_some());
    }

    #[test]
    fn axis_aligned() {
        let universe = single_block_universe();
        let axis = vec![Vec3::X, Vec3::Y, Vec3::Z];
        let dirs = axis
            .iter()
            .map(|v| vec![*v, -*v])
            .flatten()
            .collect::<Vec<Vec3>>();
        let center = Vec3::ONE * 0.5;
        for direction in dirs {
            let position = center - direction;
            if let Some(hit) = cast_ray(
                RayFinite {
                    position,
                    direction,
                    reach: 2.0,
                },
                &universe,
            ) {
                assert_eq!(hit.distance(), 0.5);
            } else {
                panic!(
                    "No hit for ray ({}, {}) in single block map",
                    position, direction
                );
            }
        }
    }

    #[test]
    fn bombard_face() {
        let universe = single_block_universe();
        let center = Vec3::ONE * 0.5 + Vec3::Z * 0.5;
        for angle in 1..180 {
            let rot = Quat::from_rotation_y(angle as f32 / 360.0 * PI);
            let direction = rot * Vec3::X;
            let position = center - direction;
            if let Some(hit) = cast_ray(
                RayFinite {
                    position,
                    direction,
                    reach: 2.0,
                },
                &universe,
            ) {
                assert!(close_enough(hit.distance(), 1.0));
            } else {
                panic!(
                    "No hit for ray ({}, {}) in single block map",
                    position, direction
                );
            }
        }
    }

    #[test]
    fn corner_hit() {
        let universe = single_block_universe();
        let position = -Vec3::ONE;
        let direction = Vec3::ONE.normalize();
        if let Some(hit) = cast_ray(
            RayFinite {
                position,
                direction,
                reach: 2.0,
            },
            &universe,
        ) {
            assert!(close_enough(hit.distance(), Vec3::ONE.length()));
        } else {
            panic!(
                "No hit for ray ({}, {}) in single block map",
                position, direction
            );
        }
    }

    fn single_block_universe() -> Universe {
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
