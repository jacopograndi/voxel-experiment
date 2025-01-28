#[cfg(test)]
mod ray {
    use crate::{
        raycast::{cast_ray, RayFinite},
        tests::test::{universe_single_block, DIRS, EPS},
    };
    use bevy::prelude::*;
    use std::f32::consts::PI;

    /// Cast a ray from (0.5, 0.5, 0.5) into a universe with a block in 0, 0, 0
    /// The block starts at (0.0, 0.0, 0.0) and ends at (1.0, 1.0, 1.0)
    fn cast(pos: Vec3, dir: Vec3, reach: f32, result: bool) {
        let u = universe_single_block();
        let center = Vec3::ONE * 0.5;
        let hit = cast_ray(
            RayFinite {
                position: center + pos,
                direction: dir,
                reach,
            },
            &u,
        );
        assert_eq!(
            hit.is_some(),
            result,
            "ray in {pos} going {dir} for {reach}. \n{hit:?}",
        );
    }

    #[test]
    fn out_of_range() {
        cast(Vec3::Y, -Vec3::X, 100.0, false);
    }

    #[test]
    fn adjacent() {
        cast(-Vec3::X * 0.5, Vec3::X, 0.1, true);
        cast(Vec3::X * 0.5, -Vec3::X, 0.1, true);
        cast(
            Vec3::ZERO,
            Vec3::new(-1.0, -1.0, 0.0).normalize(),
            0.1,
            true,
        );
    }

    #[test]
    fn adjacent_tangent() {
        cast(-Vec3::X * 0.5, Vec3::Y, 0.1, true);
        cast(Vec3::X * (0.5 + EPS), Vec3::Y, 0.1, false);
    }

    #[test]
    fn small_angle() {
        cast(
            Vec3::X * 0.7,
            Vec3::new(-100.0, -1.0, 0.0).normalize(),
            0.1,
            false,
        );
    }

    #[test]
    fn just_out_of_range() {
        for i in 0..1000 {
            let f = i as f32;
            cast(Vec3::X * (0.5 + EPS * f), -Vec3::X, EPS * f, false);
            cast(Vec3::X * (0.5 + EPS * f), -Vec3::X, EPS * (f + 1.0), true);
        }
    }

    #[test]
    fn zero_length() {
        cast(Vec3::X * 0.5, -Vec3::X, 0.0, false);
    }

    #[test]
    fn eps_length() {
        cast(Vec3::X * 0.5, -Vec3::X, EPS, true);
    }

    #[test]
    fn axis_aligned() {
        for direction in DIRS {
            cast(-direction, direction, 2.0, true);
        }
    }

    #[test]
    fn bombard_face() {
        for angle in 1..180 {
            let rot = Quat::from_rotation_y(angle as f32 / 360.0 * PI);
            let direction = rot * Vec3::X;
            cast(-direction + Vec3::Z * 0.5, direction, 2.0, true);
        }
    }

    #[test]
    fn corner_hit() {
        cast(-Vec3::ONE, Vec3::ONE.normalize(), 2.0, true);
    }
}

#[cfg(test)]
mod cuboid {
    use crate::{
        raycast::{cast_cuboid, get_leading_aabb_vertex, RayFinite},
        tests::test::{universe_single_block, EPS},
    };
    use bevy::prelude::*;

    fn cast_cube(pos: Vec3, dir: Vec3, reach: f32, result: bool) {
        let u = universe_single_block();
        let center = Vec3::ONE * 0.5;
        let hit = cast_cuboid(
            RayFinite {
                position: center + pos,
                direction: -Vec3::X,
                reach,
            },
            Vec3::ONE,
            &u,
        );
        assert_eq!(
            hit.is_some(),
            result,
            "cube of side lenght 1 in {pos} going {dir} for {reach}. \n{hit:?}",
        );
    }

    #[test]
    fn single_dimension_continuous() {}

    #[test]
    fn just_out_of_range() {
        for i in 0..1000 {
            let f = i as f32;
            cast_cube(Vec3::X * (1.0 + EPS * f), -Vec3::X, EPS * f, false);
            cast_cube(Vec3::X * (1.0 + EPS * f), -Vec3::X, EPS * (f + 1.0), true);
        }
    }

    #[test]
    fn adjacent() {
        let universe = universe_single_block();
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
