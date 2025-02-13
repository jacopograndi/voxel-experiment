use bevy::math::{IVec3, Vec3};

use mcrs_universe::{block::BlockFlag, universe::Universe};

use crate::{test_print, MARGIN_EPSILON};

const RAYCAST_MAX_ITERATIONS: u32 = 1000;

/// Represents a line segment
#[derive(Debug, Clone)]
pub struct RayFinite {
    /// Origin of the segment
    pub position: Vec3,

    /// Direction of the line parallel to the segment
    pub direction: Vec3,

    /// Lenght of the segment
    pub reach: f32,
}

impl RayFinite {
    pub fn view(&self) -> String {
        format!(
            "pos:{}, dir:{}, reach:{}",
            self.position, self.direction, self.reach
        )
    }
}

/// Checks intersections of rays through a grid.
#[derive(Debug, Clone)]
pub struct Raycaster {
    /// Starting conditions
    ray: RayFinite,

    /// Precalculated value
    /// The direction to follow when checking a new block.
    pub grid_step: IVec3,

    /// Precalculated value
    /// For each direction holds the distance required to travel 1 unit.
    delta_dist: Vec3,

    /// Mutated at every step
    /// The current position in the grid that is being checked.
    pub grid_pos: IVec3,

    /// Mutated at every step
    /// Is 1 in the direction of the last checked grid position, 0 otherwise.
    pub mask: IVec3,

    /// Mutated at every step
    /// Accumulates the distance travelled for each direction.
    side_dist: Vec3,

    /// Mutated at every step
    /// Distance traveled by the ray
    pub distance: f32,
}

/// Inspired by https://lodev.org/cgtutor/raycasting.html
impl Raycaster {
    /// Initialize a Raycaster from a ray
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
            distance: 0.0,
        };
        raycaster.update_mask();
        raycaster
    }

    /// Steps through the grid until a collision is detected.
    /// Starts at ray.pos and ends when either the distance > ray.reach or there's a collision.
    pub fn cast(
        ray: RayFinite,
        collision_check: impl Fn(&RaycastCheck) -> bool,
    ) -> Option<Raycaster> {
        if ray.direction.length_squared() <= MARGIN_EPSILON || ray.reach <= 0.0 {
            return None;
        }

        let mut raycaster = Raycaster::new(&ray);

        test_print(format!(
            "# raycasting: start [{}, step:{}, delta:{}]",
            ray.view(),
            raycaster.grid_step,
            raycaster.delta_dist
        ));

        for _i in 0..RAYCAST_MAX_ITERATIONS {
            if raycaster.distance > ray.reach {
                test_print(format!("# raycasting: miss. dist:{}", raycaster.distance));
                return None;
            }

            raycaster.step();

            if collision_check(&(&raycaster).into()) {
                if raycaster.distance <= ray.reach {
                    test_print(format!("# raycasting: hit dist:{}", raycaster.distance));
                    return Some(raycaster);
                }
            }
        }
        test_print(format!("# raycasting: out of ray iterations"));
        None
    }

    /// Advance the ray by one block
    fn step(&mut self) {
        self.update_mask();
        self.distance = mul_or_zero_vec(self.mask.as_vec3(), self.side_dist).length();
        self.side_dist += mul_or_zero_vec(self.mask.as_vec3(), self.delta_dist);
        self.grid_pos += self.mask * self.grid_step;
        test_print(format!("# raycasting: [{}]", self.view_state()));
    }

    pub fn update_mask(&mut self) {
        let Vec3 { x, y, z } = self.side_dist;
        self.mask = match (x < y, x < z, y < z) {
            (true, true, _) => IVec3::X,
            (false, _, true) => IVec3::Y,
            (_, false, false) => IVec3::Z,
            _ => IVec3::ZERO,
        };
    }

    pub fn final_position(&self) -> Vec3 {
        self.ray.position + self.ray.direction * self.distance
    }

    pub fn normal(&self) -> IVec3 {
        -self.grid_step * self.mask
    }

    pub fn view_state(&self) -> String {
        format!(
            "grid:{}, mask:{}, side:{}, dist:{}",
            self.grid_pos, self.mask, self.side_dist, self.distance
        )
    }
}

#[derive(Clone, Debug)]
pub struct RaycastCheck {
    /// The position on the block's boundary
    position: Vec3,

    /// Axis aligned versor
    direction: Vec3,
}

impl RaycastCheck {
    /// Get the plane of the face that is being checked
    fn wall(&self) -> Vec3 {
        Vec3::ONE - self.direction.abs()
    }

    /// Get the block that needs to be checked
    fn grid_pos(&self) -> IVec3 {
        (self.position + self.direction * MARGIN_EPSILON)
            .floor()
            .as_ivec3()
    }
}

impl From<&Raycaster> for RaycastCheck {
    fn from(value: &Raycaster) -> Self {
        Self {
            position: value.final_position(),
            direction: (value.grid_step * value.mask).as_vec3(),
        }
    }
}

/// Checks if the segment defined by ray intersects a collidable block in universe.
pub fn cast_ray(ray: RayFinite, universe: &Universe) -> Option<Raycaster> {
    Raycaster::cast(ray, |c: &RaycastCheck| {
        is_block_collidable(&c.grid_pos(), universe)
    })
}

/// Checks if by sweeping a cuboid along a segment defined by a ray
/// the cuboid intersects a collidable block in universe.
pub fn cast_cuboid(ray: RayFinite, size: Vec3, universe: &Universe) -> Option<Raycaster> {
    let leading_vertex = get_leading_aabb_vertex(size, ray.direction);

    test_print(format!(
        "cuboid: start [{}, size:{}, leading vertex:{}]",
        ray.view(),
        size,
        leading_vertex
    ));

    let start = leading_vertex + ray.position;
    let collision_check = |c: &RaycastCheck| {
        let ray_pos = c.position + c.direction * MARGIN_EPSILON;

        let wall = c.wall();
        let face_size = (size * wall).length();
        let face_diagonal = -(leading_vertex.signum() * wall * size).normalize_or_zero();
        let face_pos = ray_pos + face_diagonal * face_size;

        let min = ray_pos.min(face_pos).floor().as_ivec3();
        let max = ray_pos.max(face_pos).floor().as_ivec3();

        test_print(format!(
            "cuboid: check [ray_pos:{}, face_pos:{}, normal:{}, min:{}, max:{}, lead:{}]",
            ray_pos, face_pos, c.direction, min, max, leading_vertex
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
