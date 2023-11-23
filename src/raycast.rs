use bevy::math::{IVec3, Vec3};

use crate::voxel_shapes::*;

const RAYCAST_MAX_ITERATIONS: u32 = 1000;

/// http://www.cs.yorku.ca/~amana/research/grid.pdf
fn raycast(start: Vec3, direction: Vec3, grid: &Grid) -> f32 {
    if direction.length_squared() == 0. {
        return 0.;
    }
    assert!((0.999..1.001).contains(&direction.length_squared()));
    let mut grid_pos = IVec3::new(start.x as i32, start.y as i32, start.z as i32);
    let mut step = IVec3::ZERO;
    let mut sidedist = Vec3::ZERO;

    //olc::vf2d vRayUnitStepSize =
    //{ sqrt(1 + (vRayDir.y / vRayDir.x) * (vRayDir.y / vRayDir.x))
    //, sqrt(1 + (vRayDir.x / vRayDir.y) * (vRayDir.x / vRayDir.y)) };
    let deltadist = (1. / direction).abs();
    //let deltadist = Vec3::new(f32::sqrt(1 + (direction.y / direction.x) * ()));
    if direction.x < 0. {
        step.x = -1;
        sidedist.x = (start.x - grid_pos.x as f32) * deltadist.x;
    } else {
        step.x = 1;
        sidedist.x = ((grid_pos.x as f32 + 1.) - start.x) * deltadist.x;
    }
    if direction.y < 0. {
        step.y = -1;
        sidedist.y = (start.y - grid_pos.y as f32) * deltadist.y;
    } else {
        step.y = 1;
        sidedist.y = ((grid_pos.y as f32 + 1.) - start.y) * deltadist.y;
    }
    if direction.z < 0. {
        step.z = -1;
        sidedist.z = (start.z - grid_pos.z as f32) * deltadist.z;
    } else {
        step.z = 1;
        sidedist.z = ((grid_pos.z as f32 + 1.) - start.z) * deltadist.z;
    }
    enum Side {
        X,
        Y,
        Z,
    }
    let mut side;
    println!(
        "{:?}, {:?}, {:?}, {:?}",
        grid_pos, sidedist, deltadist, direction
    );
    for _i in 0..RAYCAST_MAX_ITERATIONS {
        if sidedist.x < sidedist.y {
            if sidedist.x < sidedist.z {
                sidedist.x += deltadist.x;
                grid_pos.x += step.x;
                side = Side::X;
            } else {
                sidedist.z += deltadist.z;
                grid_pos.z += step.z;
                side = Side::Z;
            }
        } else {
            if sidedist.y < sidedist.z {
                sidedist.y += deltadist.y;
                grid_pos.y += step.y;
                side = Side::Y;
            } else {
                sidedist.z += deltadist.z;
                grid_pos.z += step.z;
                side = Side::Z;
            }
        }
        if let Some(FILLED) = grid.get_at(grid_pos) {
            println!(
                "{:?}, {:?}, {:?}, {:?}",
                grid_pos, sidedist, deltadist, direction
            );
            let dist = match side {
                Side::X => sidedist.x - deltadist.x,
                Side::Y => sidedist.y - deltadist.y,
                Side::Z => sidedist.z - deltadist.z,
            };
            return dist.abs();
        }
    }
    println!("out of raycast iterations");
    f32::INFINITY
}

#[cfg(test)]
mod test {
    use crate::{raycast::raycast, voxel_shapes::*};
    use bevy::math::{ivec3, Vec3};

    #[test]
    fn zero() {
        let grid = Grid::from_vec(vec![ivec3(0, 0, 0)]);
        let dist = raycast(Vec3::new(3., 0.5, 0.), Vec3::ZERO, &grid);
        assert_eq!(dist, 0.0);
    }

    #[test]
    fn simple_right() {
        let grid = Grid::from_vec(vec![ivec3(0, 0, 0)]);
        let dist = raycast(Vec3::new(-1., 0.5, 0.), Vec3::X, &grid);
        assert_eq!(dist, 1.0);
    }

    #[test]
    fn simple_left() {
        let grid = Grid::from_vec(vec![ivec3(0, 0, 0)]);
        let dist = raycast(Vec3::new(3., 0.5, 0.), Vec3::NEG_X, &grid);
        assert_eq!(dist, 2.0);
    }

    #[test]
    fn simple_down() {
        let grid = Grid::from_vec(vec![ivec3(0, 0, 0)]);
        let dist = raycast(Vec3::new(0.8, 1.5, 0.), Vec3::NEG_Y, &grid);
        assert_eq!(dist, 0.5);
    }

    #[test]
    fn simple_forward() {
        let grid = Grid::from_vec(vec![ivec3(0, 0, 0)]);
        let dist = raycast(Vec3::new(0.1, 0.5, -40.), Vec3::Z, &grid);
        assert_eq!(dist, 40.0);
    }

    fn eq_approx(a: f32, b: f32, eps: f32) -> bool {
        ((a - eps)..(a + eps)).contains(&b)
    }

    #[test]
    fn diag_to_corner() {
        let grid = Grid::from_vec(vec![ivec3(0, 0, 0)]);
        let dist = raycast(
            Vec3::new(1.5, 1.5, 0.0),
            Vec3::new(-1., -1., 0.).normalize(),
            &grid,
        );
        let t = Vec3::new(0.5, 0.5, 0.).length();
        assert!(eq_approx(t, dist, 0.01));
    }

    #[test]
    fn diag_to_center() {
        let grid = Grid::from_vec(vec![ivec3(0, 0, 0)]);
        let dist = raycast(
            Vec3::new(1.5, 1.5, 0.0),
            Vec3::new(-1., -1., 0.).normalize(),
            &grid,
        );
        let t = Vec3::new(0.5, 0.5, 0.).length();
        assert!(eq_approx(t, dist, 0.01));
    }
}
