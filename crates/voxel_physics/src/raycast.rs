use bevy::math::{BVec3, IVec3, Vec3, Vec3Swizzles};

use voxel_storage::chunk_map::ChunkMap;

use crate::MARGIN_EPSILON;

const RAYCAST_MAX_ITERATIONS: u32 = 10;

#[derive(Debug, Clone, PartialEq)]
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

fn signum_or_zero(x: f32) -> f32 {
    if x == 0.0 {
        x
    } else {
        x.signum()
    }
}

fn signum_or_zero_vec(v: Vec3) -> Vec3 {
    Vec3::new(
        signum_or_zero(v.x),
        signum_or_zero(v.y),
        signum_or_zero(v.z),
    )
}

fn mul_or_zero(x: f32, y: f32) -> f32 {
    if y.is_finite() {
        x * y
    } else {
        0.0
    }
}

fn mul_or_zero_vec(v: Vec3, w: Vec3) -> Vec3 {
    Vec3::new(
        mul_or_zero(v.x, w.x),
        mul_or_zero(v.y, w.y),
        mul_or_zero(v.z, w.z),
    )
}

#[derive(Debug, Clone)]
struct Ray {
    start_pos: Vec3,
    direction: Vec3,
    grid_pos: IVec3,
    grid_step: IVec3,
    side_dist: Vec3,
    mask: IVec3,
    delta_dist: Vec3,
}

impl Ray {
    fn new(start_pos: Vec3, direction: Vec3) -> Self {
        let dir_sign = signum_or_zero_vec(direction);
        let grid_pos = start_pos.floor().as_ivec3();
        let delta_dist = (direction.length() / direction).abs();
        Self {
            start_pos,
            direction,
            grid_pos,
            grid_step: dir_sign.floor().as_ivec3(),
            side_dist: (dir_sign * (grid_pos.as_vec3() - start_pos) + (dir_sign * 0.5) + 0.5)
                * delta_dist,
            mask: IVec3::ZERO,
            delta_dist,
        }
    }

    fn step(&mut self) {
        self.mask = step_vec(self.side_dist.xyz(), self.side_dist.yzx())
            * step_vec(self.side_dist.xyz(), self.side_dist.zxy());
        self.side_dist += mul_or_zero_vec(self.mask.as_vec3(), self.delta_dist);
        self.grid_pos += self.mask * self.grid_step;
    }

    fn final_pos(&self) -> Vec3 {
        let mask_f = self.mask.as_vec3();
        self.direction / (mask_f * self.direction).dot(Vec3::splat(1.))
            * (mask_f
                * (self.grid_pos.as_vec3() + step_vec(self.direction, Vec3::ZERO).as_vec3()
                    - self.start_pos))
                .dot(Vec3::splat(1.))
            + self.start_pos
    }

    fn distance(&self) -> f32 {
        // todo: can be faster without lenght sqrt, like `match self.mask`
        //((self.side_dist - self.delta_dist) * self.mask.as_vec3()).length()
        (self.start_pos - self.final_pos()).length()
    }

    fn normal(&self) -> IVec3 {
        -self.grid_step * self.mask
    }

    fn raycast_hit(&self) -> RaycastHit {
        RaycastHit {
            pos: self.grid_pos,
            normal: self.normal(),
            distance: (self.start_pos - self.final_pos()).length(),
        }
    }
}

/// http://www.cs.yorku.ca/~amana/research/grid.pdf
pub fn raycast(
    start: Vec3,
    direction: Vec3,
    max_distance: f32,
    chunk_map: &ChunkMap,
) -> Option<RaycastHit> {
    if direction.length_squared() <= MARGIN_EPSILON {
        return None;
    }
    let mut ray = Ray::new(start, direction);
    for _i in 0..RAYCAST_MAX_ITERATIONS {
        ray.step();
        if ray.distance() > max_distance {
            return None;
        }
        if let Some(voxel) = chunk_map.get_at(&ray.grid_pos) {
            // hardcoded flag 16 to be collision detection
            if voxel.flags & 16 == 16 {
                return Some(ray.raycast_hit());
            }
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq)]
pub struct SweepHit {
    pub blocked: IVec3,
    pub distance: f32,
    pub normal: IVec3,
}

/// https://github.com/fenomas/voxel-aabb-sweep
pub fn sweep_aabb(
    pos: Vec3,
    size: Vec3,
    direction: Vec3,
    max_distance: f32,
    chunk_map: &ChunkMap,
) -> Option<SweepHit> {
    if direction.length_squared() < MARGIN_EPSILON {
        return None;
    }

    let leading_vertex = get_leading_aabb_vertex(size, direction);
    let start = leading_vertex + pos;

    let mut ray = Ray::new(start, direction);
    for _i in 0..RAYCAST_MAX_ITERATIONS {
        ray.step();
        if ray.distance() > max_distance {
            return None;
        }

        let vert_pos = ray.final_pos();
        let inv_mask = IVec3::ONE - ray.mask;
        let center_pos = vert_pos - leading_vertex * inv_mask.as_vec3()
            + (ray.mask * ray.grid_step).as_vec3() * size * 0.5;
        let min = (center_pos - size * 0.5 * inv_mask.as_vec3())
            .floor()
            .as_ivec3();
        let max = (center_pos + size * 0.5 * inv_mask.as_vec3())
            .floor()
            .as_ivec3();
        for x in min.x..max.x + 1 {
            for y in min.y..max.y + 1 {
                for z in min.z..max.z + 1 {
                    let sample_pos = IVec3::new(x, y, z);
                    if let Some(voxel) = chunk_map.get_at(&sample_pos) {
                        if voxel.flags & 16 == 16 {
                            let hit = SweepHit {
                                blocked: ray.mask,
                                distance: ray.distance(),
                                normal: ray.normal(),
                            };
                            return Some(hit);
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn get_leading_aabb_vertex(size: Vec3, direction: Vec3) -> Vec3 {
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
