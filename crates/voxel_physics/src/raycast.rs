use bevy::math::{BVec3, IVec3, Vec3, Vec3Swizzles};

use voxel_storage::chunk_map::ChunkMap;

const RAYCAST_MAX_ITERATIONS: u32 = 1000;

pub struct RaycastHit {
    pub pos: IVec3,
    pub normal: IVec3,
    pub distance: f32,
}

fn step(edge: f32, x: f32) -> i32 {
    if edge <= x {
        1
    } else {
        0
    }
}

fn step_vec(edge: Vec3, x: Vec3) -> IVec3 {
    IVec3::new(step(edge.x, x.x), step(edge.y, x.y), step(edge.z, x.z))
}

/// http://www.cs.yorku.ca/~amana/research/grid.pdf
pub fn raycast(start: Vec3, direction: Vec3, chunk_map: &ChunkMap) -> Option<RaycastHit> {
    if direction.length_squared() == 0. {
        return None;
    }

    let mut grid_pos = start.as_ivec3();

    let deltadist = (1. / direction).abs();
    let dir_sign = direction.signum();
    let ray_step = dir_sign.as_ivec3();
    let mut sidedist =
        (dir_sign * (grid_pos.as_vec3() - start) + (dir_sign * 0.5) + 0.5) * deltadist;
    let mut hit = false;
    let mut mask = IVec3::ZERO;
    for _i in 0..RAYCAST_MAX_ITERATIONS {
        mask = step_vec(sidedist.xyz(), sidedist.yzx()) * step_vec(sidedist.xyz(), sidedist.zxy());
        sidedist += mask.as_vec3() * deltadist;
        grid_pos += mask * ray_step;
        if let Some(voxel) = chunk_map.get_at(&grid_pos) {
            // hardcoded flag 16 to be collision detection
            if voxel.flags & 16 == 16 {
                hit = true;
                break;
            }
        }
    }
    if hit {
        let mask_f = mask.as_vec3();
        let final_pos = direction / (mask_f + direction).dot(Vec3::splat(1.))
            * (mask_f * (grid_pos.as_vec3() + step_vec(direction, Vec3::ZERO).as_vec3() - start))
                .dot(Vec3::splat(1.))
            + start;
        return Some(RaycastHit {
            pos: grid_pos,
            normal: -ray_step * mask,
            distance: (start - final_pos).length(),
        });
    }
    None
}

pub struct SweepHit {
    pub blocked: BVec3,
    pub distance: f32,
}

/// https://github.com/fenomas/voxel-aabb-sweep
pub fn sweep_aabb(pos: Vec3, size: Vec3, velocity: Vec3, chunk_map: &ChunkMap) -> Option<SweepHit> {
    let direction = velocity.normalize_or_zero();
    let distance = velocity.length();
    if direction.length_squared() <= 0.01 {
        return None;
    }
    if distance == 0.0 {
        return None;
    }

    let leading_vertex = get_leading_aabb_vertex(size, direction);
    let start = leading_vertex + pos;

    let mut grid_pos = IVec3::new(start.x as i32, start.y as i32, start.z as i32);
    let mut step = IVec3::ZERO;
    let mut sidedist = Vec3::ZERO;

    let deltadist = (1. / direction).abs();
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

        let dist = match side {
            Side::X => sidedist.x - deltadist.x,
            Side::Y => sidedist.y - deltadist.y,
            Side::Z => sidedist.z - deltadist.z,
        };

        if dist >= distance {
            return None;
        }

        let vert_pos = start + direction * dist;
        let center_pos = vert_pos - leading_vertex;

        let min = (center_pos - size).as_ivec3();
        let max = (center_pos + size).as_ivec3();

        let hit = SweepHit {
            blocked: match side {
                Side::X => BVec3::new(true, false, false),
                Side::Y => BVec3::new(false, true, false),
                Side::Z => BVec3::new(false, false, true),
            },
            distance: dist.abs(),
        };

        match side {
            Side::X => {
                for y in min.y..max.y {
                    for z in min.z..max.z {
                        let sample_pos = grid_pos + IVec3::new(0, y * step.y, z * step.z);
                        if let Some(voxel) = chunk_map.get_at(&sample_pos) {
                            if voxel.flags & 16 == 16 {
                                return Some(hit);
                            }
                        }
                    }
                }
            }
            Side::Y => {
                for x in min.x..max.x {
                    for z in min.z..max.z {
                        let sample_pos = grid_pos + IVec3::new(x * step.x, 0, z * step.z);
                        if let Some(voxel) = chunk_map.get_at(&sample_pos) {
                            if voxel.flags & 16 == 16 {
                                return Some(hit);
                            }
                        }
                    }
                }
            }
            Side::Z => {
                for y in min.y..max.y {
                    for x in min.x..max.x {
                        let sample_pos = grid_pos + IVec3::new(x * step.x, y * step.y, 0);
                        if let Some(voxel) = chunk_map.get_at(&sample_pos) {
                            if voxel.flags & 16 == 16 {
                                return Some(hit);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_leading_aabb_vertex(size: Vec3, direction: Vec3) -> Vec3 {
    // find leading corner
    let vertices = [
        size * Vec3::new(1., 1., 1.),
        size * Vec3::new(1., 1., -1.),
        size * Vec3::new(1., -1., 1.),
        size * Vec3::new(1., -1., -1.),
        size * Vec3::new(-1., 1., 1.),
        size * Vec3::new(-1., 1., -1.),
        size * Vec3::new(-1., -1., 1.),
        size * Vec3::new(-1., -1., -1.),
    ];
    let leading_vertex = vertices
        .iter()
        .map(|vert| (vert.normalize().dot(direction), vert))
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        .unwrap()
        .1
        .clone();
    leading_vertex * 0.5
}

#[cfg(test)]
mod test {
    use super::get_leading_aabb_vertex;
    use bevy::math::Vec3;

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
