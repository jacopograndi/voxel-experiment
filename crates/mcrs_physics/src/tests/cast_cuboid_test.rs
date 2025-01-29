use crate::{
    raycast::{cast_cuboid, get_leading_aabb_vertex, RayFinite},
    tests::{universe_single_block, EPS},
};
use bevy::prelude::*;

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
fn just_out_of_range() {
    for i in 0..1000 {
        let f = i as f32;
        cast_cube(Vec3::X * (1.0 + EPS * f), -Vec3::X, EPS * (f - 1.0), false);
        cast_cube(Vec3::X * (1.0 + EPS * f), -Vec3::X, EPS * (f + 1.0), true);
    }
}

#[test]
fn corner_hit() {
    // Corner head on check
    cast_cube(-Vec3::ONE, Vec3::ONE.normalize(), 2.0, true);

    // Manual checking
    {
        cast_cube(Vec3::new(2.0, 1.0, 1.0), -Vec3::X, 1.0 + EPS, false);
        cast_cube(Vec3::new(2.0, -1.0, 1.0), -Vec3::X, 1.0 + EPS, false);
        cast_cube(Vec3::new(2.0, 1.0, -1.0), -Vec3::X, 1.0 + EPS, false);

        // Same asymmetry as a ray along -x
        cast_cube(Vec3::new(2.0, -1.0, -1.0), -Vec3::X, 1.0 + EPS, true);
    }

    // There may be other asymmetries in choosing the leading vertex for casting a face head on
}
