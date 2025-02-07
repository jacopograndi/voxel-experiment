use crate::{
    raycast::{cast_ray, RayFinite},
    tests::{universe_single_block, DIRS, EPS},
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
    cast(-Vec3::X * (0.5 + EPS), Vec3::X, 0.1, true);
    cast(Vec3::X * (0.5 + EPS), -Vec3::X, 0.1, true);
    cast(
        -Vec3::ONE * (0.5 + EPS),
        Vec3::new(1.0, 1.0, 1.0).normalize(),
        0.1,
        true,
    );
}

#[test]
fn adjacent_tangent() {
    cast(-Vec3::X * 0.5, Vec3::Y, 0.1, false);
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
        cast(Vec3::X * (0.5 + EPS * f), -Vec3::X, EPS * (f - 1.0), false);
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
    // Can the corner be hit head on?
    cast(-Vec3::ONE, Vec3::ONE.normalize(), 2.0, true);

    // Manual checking corners
    {
        cast(Vec3::new(1.5, 0.5, 0.5), -Vec3::X, 1.0 + EPS, false);
        cast(Vec3::new(1.5, -0.5, 0.5), -Vec3::X, 1.0 + EPS, false);
        cast(Vec3::new(1.5, 0.5, -0.5), -Vec3::X, 1.0 + EPS, false);

        // The corner that sits on a zero axis is different from the others
        // This is just a boundary condition, it shouldn't matter that it's different
        // Although, it's not pretty.
        cast(Vec3::new(1.5, -0.5, -0.5), -Vec3::X, 1.0 + EPS, true);
    }

    // Check the corner of each face with a ray parallel to the face's normal
    for n in DIRS {
        // Find an axis aligned vec perpendicular to n
        let Some(u) = DIRS.into_iter().find(|d| d.dot(n) == 0.0) else {
            panic!("ZF is incomplete");
        };

        // Get the other axis aligned vec perpendicular to n and u
        let v = n.cross(u);

        // Now the plane defined by u and v is tangential to the face in exam
        dbg!(n, u, v);

        // Cast rays to each face's corner
        for (s, t) in [(0.5, 0.5), (0.5, -0.5), (-0.5, -0.5), (-0.5, 0.5)].iter() {
            let offset = u * s + v * t;
            let pos = n + offset;

            let zero = Vec2::new(-0.5, -0.5);

            // The corner that sits on a zero axis is different from the others
            // This is just a boundary condition, it shouldn't matter that it's different
            // Although, it's not pretty.
            let outcome = if pos.xy() == zero || pos.xz() == zero || pos.yz() == zero {
                true
            } else {
                false
            };
            cast(pos, -n, 1.0 + EPS, outcome);
        }
    }
}
