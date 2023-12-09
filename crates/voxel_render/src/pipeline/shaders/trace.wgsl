#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View
#import bevy_render::globals::Globals
#import bevy_voxel_engine::common::{
    VoxelUniforms, TraceUniforms, Ray, VOXELS_PER_METER, hash, 
    clip_aabb, ray_plane, in_bounds, ray_box_dist, PORTAL_FLAG, cosine_hemisphere, skybox, PI
}

@group(0) @binding(0) var<uniform> voxel_uniforms: VoxelUniforms;
@group(0) @binding(1) var<storage, read_write> chunks: array<u32>;
@group(0) @binding(2) var<storage, read> chunks_offsets: array<u32>;

@group(1) @binding(0) var<uniform> trace_uniforms: TraceUniforms;
@group(1) @binding(1) var<uniform> view: View;
@group(1) @binding(2) var<uniform> globals: Globals;

@group(2) @binding(0) var texture_sheet: texture_2d<f32>;
@group(2) @binding(1) var texture_sheet_sampler: sampler;

const MAX_RAY_CHUNK_ITERS = 1000u;
const MAX_RAY_ITERS = 1000;

const EMPTY_CHUNK = 4294967295u;

/*
// possibly better structure
var<storage, read> chunks: array<Chunk>;
const CHUNK_SIZE = 32768u; // 32^3
struct Chunk {
    data: array<u32, CHUNK_SIZE>,
};
*/

struct Voxel {
    data: u32,
    pos: vec3<f32>,
    grid_size: u32,
};

fn get_at(grid: vec3<i32>) -> u32 {
    let outer = voxel_uniforms.offsets_grid_size;
    let side = voxel_uniforms.chunk_size;
    let chunk_size = vec3f(f32(side));
    let pos = vec3f(grid);
    let outer_pos = floor(pos / chunk_size);
    let chunk_i = u32(outer_pos.x) * (outer * outer) + u32(outer_pos.y) * outer + u32(outer_pos.z);
    let offset = chunks_offsets[chunk_i];
    if (offset != EMPTY_CHUNK) {
        let voxel_map = pos % chunk_size;
        let voxel_i = u32(voxel_map.x) * (side * side) + u32(voxel_map.y) * side + u32(voxel_map.z);
        return chunks[offset + voxel_i];
    }
    else {
        return 0u;
    }
}

struct HitInfo {
    hit: bool,
    data: u32,
    pos: vec3<f32>,
    reprojection_pos: vec3<f32>,
    normal: vec3<f32>,
    tangent1: vec3<f32>,
    tangent2: vec3<f32>,
    uv: vec3<f32>,
    steps: u32,
};

fn intersect_scene(r: Ray, steps: u32) -> HitInfo {
    let infinity = 1000000000.0 * r.dir;
    var hit_info : HitInfo;
    hit_info.hit = false;
    return hit_info;
}

fn in_chunk_bounds (v: vec3f, offset: vec3f, size: vec3f) -> bool {
    return 
        v.x >= offset.x && v.x < offset.x + size.x &&
        v.y >= offset.y && v.y < offset.y + size.y &&
        v.z >= offset.z && v.z < offset.z + size.z
    ;
}

fn shoot_ray(inray: Ray, flags: u32) -> HitInfo {
    var ray = inray;
    let epsilon = 0.00001;

    let outer = voxel_uniforms.offsets_grid_size;
    let side = voxel_uniforms.chunk_size;
    let chunk_size = vec3f(f32(side));
    let chunk_grid_size = vec3f(f32(outer));

    var map_pos = floor(ray.pos);

    // initial raycast against the outer chunks bounds
    // only done if the ray.pos is outside the chunks bounds
    if (!in_chunk_bounds(map_pos, vec3f(0.0), chunk_grid_size * chunk_size)) {
        let chunk_pos = vec3f(0.0);
        let dist = ray_box_dist(
            ray, 
            chunk_pos + vec3f(-epsilon), 
            chunk_pos + vec3f(chunk_grid_size * chunk_size + epsilon)
        ).x;
        if (dist == 0.0) {
            return intersect_scene(ray, 1u);
        }
        else {
            //return intersect_scene(ray, 99u);
            ray.pos = ray.pos + ray.dir * dist;
            map_pos = floor(ray.pos);
        }
    }

    // actual raycasting
    var scale = chunk_size.x;
    var delta_dist = abs(vec3f(length(ray.dir)) / ray.dir);
    var ray_step = sign(ray.dir);
    var side_dist = (sign(ray.dir) * (map_pos - ray.pos) + (sign(ray.dir) * 0.5) + 0.5) * delta_dist; 
    var mask = vec3f(0.0);
    var hit = false;
    var voxel: Voxel;
    var iters = 0u;
    for (iters = 0u; iters < MAX_RAY_CHUNK_ITERS; iters++) {
		mask = step(side_dist.xyz, side_dist.yzx) * step(side_dist.xyz, side_dist.zxy);
		side_dist += mask * delta_dist;
		map_pos += mask * ray_step;

        // out of bounds
        if (!in_chunk_bounds(map_pos, vec3f(0.0), chunk_grid_size * chunk_size)) {
            return intersect_scene(ray, 2u);
        }

        let outer_pos = floor(map_pos / chunk_size);
        let chunk_i = u32(outer_pos.x) * (outer * outer) + u32(outer_pos.y) * outer + u32(outer_pos.z);
        let offset = chunks_offsets[chunk_i];
        if (offset != EMPTY_CHUNK) {
            let voxel_map = map_pos % chunk_size;
            let voxel_i = u32(voxel_map.x) * (side * side) + u32(voxel_map.y) * side + u32(voxel_map.z);
            voxel = Voxel(chunks[offset + voxel_i], map_pos, side);
            if ((voxel.data & 0xFFu) != 0u && (((voxel.data >> 8u) & flags) > 0u || flags == 0u)) {
                hit = true;
                break;
            }
        }
        else {
            // todo skip empty chunks
        }
	}
    let end_ray_pos = ray.dir 
        / dot(mask * ray.dir, vec3f(1.0)) 
        * dot(mask * (map_pos + step(ray.dir, vec3f(0.0)) - ray.pos), vec3f(1.0)) 
        + ray.pos
    ;
   	var uv = vec3f(0.0);
    var tangent1 = vec3f(0.0);
    var tangent2 = vec3f(0.0);
    if (abs(mask.x) > 0.0) {
        uv = vec3f(end_ray_pos.yz, 0.0);
        tangent1 = vec3f(0.0, 1.0, 0.0);
        tangent2 = vec3f(0.0, 0.0, 1.0);
    }
    else if (abs(mask.y) > 0.) {
        uv = vec3f(end_ray_pos.xz, 0.0);
        tangent1 = vec3f(1.0, 0.0, 0.0);
        tangent2 = vec3f(0.0, 0.0, 1.0);
    }
    else {
        uv = vec3f(end_ray_pos.xy, 0.0);
        tangent1 = vec3f(1.0, 0.0, 0.0);
        tangent2 = vec3f(0.0, 1.0, 0.0);
    }
    uv = fract(uv);

    var hit_info : HitInfo;
    hit_info.hit = hit;
    hit_info.data = voxel.data;
    hit_info.pos = end_ray_pos;
    hit_info.reprojection_pos = ray.pos;
    hit_info.normal = -ray_step * mask;
    hit_info.tangent1 = tangent1;
    hit_info.tangent2 = tangent2;
    hit_info.uv = uv;
    hit_info.steps = u32(iters);
    return hit_info;
}

fn get_voxel(pos: vec3<f32>) -> f32 {
    let data = get_at(vec3i(pos));
    return min(f32(data & 0xFFu), 1.0);
}

// https://www.shadertoy.com/view/ldl3DS
fn vertex_ao(side: vec2<f32>, corner: f32) -> f32 {
    return 1.0 - (side.x + side.y + max(corner, side.x * side.y)) / 3.1;
}
fn voxel_ao(pos: vec3<f32>, s: vec3<f32>, t: vec3<f32>) -> vec4<f32> {
    let side = vec4(
        get_voxel(pos + t), 
        get_voxel(pos - s), 
        get_voxel(pos + s), 
        get_voxel(pos - t)
    );
    let corner = vec4(
        get_voxel(pos - s + t), 
        get_voxel(pos + s + t), 
        get_voxel(pos + s - t), 
        get_voxel(pos - s - t)
    );
    var ao: vec4<f32>;
    ao.x = vertex_ao(side.xy, corner.x);
    ao.y = vertex_ao(side.xz, corner.y);
    ao.z = vertex_ao(side.zw, corner.z);
    ao.w = vertex_ao(side.yw, corner.w);
    return ao;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let seed = vec3<u32>(in.position.xyz) * 100u + u32(globals.time * 120.0) * 15236u;
    let resolution = vec2<f32>(view.viewport.zw);
    var clip_space = vec2(1.0, -1.0) * (in.uv * 2.0 - 1.0);
    var output_colour = vec3(0.0);

    let chunk_size = vec3f(f32(voxel_uniforms.chunk_size));
    let chunk_grid_size = vec3f(f32(voxel_uniforms.offsets_grid_size));
    var center_in_grid = vec3f(
        vec3i(i32(voxel_uniforms.chunk_size * voxel_uniforms.offsets_grid_size))  
        / 2
    );
    if voxel_uniforms.offsets_grid_size % 2u == 1u {
        center_in_grid -= vec3f(f32(voxel_uniforms.chunk_size)) / 2.0;
    }

    let camera_inverse = view.inverse_view_proj;
    let pos1 = camera_inverse * vec4(clip_space.x, clip_space.y, 1.0, 1.0);
    let dir1 = camera_inverse * vec4(clip_space.x, clip_space.y, 0.01, 1.0);
    let pos = pos1.xyz / pos1.w;
    let dir = normalize(dir1.xyz / dir1.w - pos);
    var constrained_pos = view.world_position % chunk_size + center_in_grid;
    var ray = Ray(constrained_pos, dir);

    let hit = shoot_ray(ray, 0u);

    // lighting
    var indirect_lighting = vec3(0.0);

    var uv = hit.uv.xy;
    uv = vec2f(1.0) - uv;
    uv /= 16.0;
    uv.x += f32(hit.data& 0xFFu) / 16.0;
    let color = textureSample(texture_sheet, texture_sheet_sampler, uv);

    if hit.hit {
        // voxel ao
        let ao = voxel_ao(hit.pos + hit.normal / 2.0, hit.tangent1, hit.tangent2);
        let interpolated_ao_pweig = mix(mix(ao.w, ao.z, hit.uv.x), mix(ao.x, ao.y, hit.uv.x), hit.uv.y);
        let voxel_ao = pow(interpolated_ao_pweig, 1.0 / 2.0);
        indirect_lighting = vec3(2.0 * voxel_ao);
        output_colour = (indirect_lighting) * color.xyz;
    }
    else {
        output_colour = skybox(ray.dir, 10.0);
    }

    if trace_uniforms.show_ray_steps != 0u {
        output_colour = vec3<f32>(f32(hit.steps) / 100.0);
    }

    output_colour = max(output_colour, vec3(0.0));
    return vec4<f32>(output_colour, 1.0);
}
