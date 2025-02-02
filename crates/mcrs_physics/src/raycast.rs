use bevy::math::{IVec3, Vec3};

use mcrs_universe::{block::BlockFlag, universe::Universe};

use crate::{test_trace, MARGIN_EPSILON};

const RAYCAST_MAX_ITERATIONS: u32 = 1000;

#[derive(Debug, Clone)]
/// Represents a line segment
pub struct RayFinite {
    /// Origin of the segment
    pub position: Vec3,
    /// Direction of the line parallel to the segment
    pub direction: Vec3,
    /// Lenght of the segment
    pub reach: f32,
}

#[derive(Debug, Clone)]
/// Checks intersections of rays through a grid.
pub struct Raycaster {
    /// Starting conditions
    ray: RayFinite,

    /// Precalculated values
    /// The direction to follow when checking a new block.
    grid_step: IVec3,
    /// For each direction holds the distance required to travel 1 unit.
    delta_dist: Vec3,

    /// Mutated at every step
    /// The current position in the grid that is being checked.
    pub grid_pos: IVec3,
    /// Is 1 in the direction of the last checked grid position, 0 otherwise.
    pub mask: IVec3,
    /// Accumulates the distance travelled for each direction.
    side_dist: Vec3,

    iters: u32,
}

/// Inspired by https://lodev.org/cgtutor/raycasting.html
impl Raycaster {
    pub fn new(ray: &RayFinite) -> Self {
        let grid_pos = ray.position.floor().as_ivec3();
        let delta_dist = (1. / ray.direction).abs();

        let mut grid_step = IVec3::ZERO;
        let mut side_dist = Vec3::ZERO;

        if ray.direction.x < 0.0 {
            grid_step.x = -1;
            side_dist.x = (ray.position.x - grid_pos.as_vec3().x) * delta_dist.x;
        } else {
            grid_step.x = 1;
            side_dist.x = (-ray.position.x + 1.0 + grid_pos.as_vec3().x) * delta_dist.x;
        }
        if ray.direction.y < 0.0 {
            grid_step.y = -1;
            side_dist.y = (ray.position.y - grid_pos.as_vec3().y) * delta_dist.y;
        } else {
            grid_step.y = 1;
            side_dist.y = (-ray.position.y + 1.0 + grid_pos.as_vec3().y) * delta_dist.y;
        }
        if ray.direction.z < 0.0 {
            grid_step.z = -1;
            side_dist.z = (ray.position.z - grid_pos.as_vec3().z) * delta_dist.z;
        } else {
            grid_step.z = 1;
            side_dist.z = (-ray.position.z + 1.0 + grid_pos.as_vec3().z) * delta_dist.z;
        }

        let mut raycaster = Self {
            ray: ray.clone(),
            grid_pos,
            grid_step,
            side_dist,
            mask: IVec3::ZERO,
            delta_dist,
            iters: 0,
        };
        raycaster.update_mask();
        raycaster
    }

    /// Steps through the grid until a collision is detected.
    pub fn cast(ray: RayFinite, collision_check: impl Fn(&Self) -> bool) -> Option<Raycaster> {
        if ray.direction.length_squared() <= MARGIN_EPSILON {
            return None;
        }

        // Inspired by https://lodev.org/cgtutor/raycasting.html
        let mut raycaster = Raycaster::new(&ray);
        test_trace(format!("ray started: {:?}", raycaster));

        // Check if it's already inside a block
        if collision_check(&raycaster) {
            test_trace(format!(
                "ray was inside a block: distance {}, {:?}",
                raycaster.final_distance(),
                raycaster
            ));
            return Some(raycaster);
        }

        for _i in 0..RAYCAST_MAX_ITERATIONS {
            raycaster.step();

            test_trace(format!(
                "_i:{_i}, stepping_dist:{}, final_dist:{}",
                raycaster.stepping_distance(),
                raycaster.final_distance()
            ));

            if raycaster.final_distance() > ray.reach {
                test_trace(format!("no hit"));
                return None;
            }

            if collision_check(&raycaster) {
                test_trace(format!(
                    "ray hit: distance {}, {:?}",
                    raycaster.final_distance(),
                    raycaster
                ));
                return Some(raycaster);
            }
        }
        test_trace(format!("out of ray iterations"));
        None
    }

    /// Advance the ray by one block
    fn step(&mut self) {
        self.update_mask();
        self.side_dist += mul_or_zero_vec(self.mask.as_vec3(), self.delta_dist);
        self.grid_pos += self.mask * self.grid_step;
        self.iters += 1;
        test_trace(format!("ray stepped: {:?}", self));
    }

    pub fn update_mask(&mut self) {
        let Vec3 { x, y, z } = self.side_dist;
        self.mask = match (x < y, x < z, y < z) {
            (true, true, _) => IVec3::X,
            (false, _, true) => IVec3::Y,
            (_, false, false) => IVec3::Z,
            _ => unreachable!(),
        };
    }

    pub fn stepping_position(&self) -> Vec3 {
        self.ray.position + self.ray.direction * self.stepping_distance()
    }

    pub fn final_position(&self) -> Vec3 {
        self.ray.position + self.ray.direction * self.final_distance()
    }

    pub fn stepping_distance(&self) -> f32 {
        mul_or_zero_vec(self.mask.as_vec3(), self.side_dist).length()
    }

    pub fn final_distance(&self) -> f32 {
        if self.iters == 0 {
            // The ray is inside a block
            return 0.0;
        }
        mul_or_zero_vec(self.mask.as_vec3(), self.side_dist - self.delta_dist).length()
    }

    pub fn normal(&self) -> IVec3 {
        -self.grid_step * self.mask
    }
}

/// Checks if the segment defined by ray intersects a collidable block in universe.
pub fn cast_ray(ray: RayFinite, universe: &Universe) -> Option<Raycaster> {
    Raycaster::cast(ray, |raycast: &Raycaster| {
        is_block_collidable(&raycast.grid_pos, universe)
    })
}

/// Checks if by sweeping a cuboid along a segment defined by a ray
/// the cuboid intersects a collidable block in universe.
pub fn cast_cuboid(ray: RayFinite, size: Vec3, universe: &Universe) -> Option<Raycaster> {
    let leading_vertex = get_leading_aabb_vertex(size, ray.direction);

    test_trace(format!(
        "cuboid start: {:?}, size {}, leading vertex: {}",
        ray, size, leading_vertex
    ));

    let start = leading_vertex + ray.position;
    let collision_check = |raycaster: &Raycaster| {
        let ray_pos = raycaster.stepping_position();
        let inv_mask = (IVec3::ONE - raycaster.mask).as_vec3();
        let face_size = (size * inv_mask).length();
        let face_diagonal = -(leading_vertex.signum() * inv_mask).normalize_or_zero();
        let face_pos = ray_pos + face_diagonal * face_size;

        let min = ray_pos.min(face_pos).floor().as_ivec3();
        let max = ray_pos.max(face_pos).floor().as_ivec3();

        test_trace(format!(
            "cuboid check: ray_pos:{}, face_pos:{}, normal:{}, min:{}, max:{}",
            ray_pos,
            face_pos,
            raycaster.normal(),
            min,
            max
        ));

        iter_cuboid(min, max).any(|sample| is_block_collidable(&sample, universe))
    };

    let ray = RayFinite {
        position: start,
        direction: ray.direction,
        reach: ray.reach,
    };
    Raycaster::cast(ray, collision_check)
}

pub fn is_block_collidable(pos: &IVec3, universe: &Universe) -> bool {
    match universe.read_chunk_block(pos) {
        Some(voxel) => voxel.properties.check(BlockFlag::Collidable),
        None => false,
    }
}

// Iterates through all integers inside a cuboid
pub fn iter_cuboid(min: IVec3, max: IVec3) -> impl Iterator<Item = IVec3> {
    (min.x..max.x + 1)
        .map(move |x| {
            (min.y..max.y + 1)
                .map(move |y| (min.z..max.z + 1).map(move |z| IVec3::new(x, y, z)))
                .flatten()
        })
        .flatten()
}

// Finds the corner which is furthest from the center of a cuboid given a direction.
pub fn get_leading_aabb_vertex(size: Vec3, direction: Vec3) -> Vec3 {
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
