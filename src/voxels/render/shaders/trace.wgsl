#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_voxel_engine::common::{
    VoxelUniforms, TraceUniforms, Ray, VOXELS_PER_METER, hash, 
    clip_aabb, ray_plane, in_bounds, ray_box_dist, PORTAL_FLAG, cosine_hemisphere, skybox
}

@group(0) @binding(0)
var<uniform> voxel_uniforms: VoxelUniforms;
@group(0) @binding(1)
var<storage, read_write> chunks: array<u32>;
@group(0) @binding(2)
var<storage, read> chunks_offsets: array<u32>;

@group(1) @binding(0)
var<uniform> trace_uniforms: TraceUniforms;
@group(1) @binding(2)
var normal: texture_storage_2d<rgba16float, read_write>;
@group(1) @binding(3)
var position: texture_storage_2d<rgba32float, read_write>;

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
    let side = voxel_uniforms.chunk_size;
    let outer = voxel_uniforms.offsets_grid_size;
    let chunk_size = vec3f(f32(side));
    let split = vec3f(grid) / chunk_size;
    var voxel_pos = vec3i(split - trunc(split)) * vec3i(i32(side));
    var chunk_pos = vec3i(trunc(split));
    let voxel_i = u32(voxel_pos.x) * (side * side) + u32(voxel_pos.y) * side + u32(voxel_pos.z);
    let chunk_i = u32(chunk_pos.x) * (outer * outer) + u32(chunk_pos.y) * outer + u32(chunk_pos.z);
    let offset = chunks_offsets[chunk_i];
    if offset == EMPTY_CHUNK {
        return chunks[0];
    }
    return chunks[voxel_i + offset];
}

fn get_value(pos: vec3<f32>) -> Voxel {
    let grid = vec3<i32>(pos);
    let data = get_at(grid);
    let rounded_pos = floor(pos);
    return Voxel(data, rounded_pos, voxel_uniforms.chunk_size);
}

struct HitInfo {
    hit: bool,
    data: u32,
    material: vec4<f32>,
    pos: vec3<f32>,
    reprojection_pos: vec3<f32>,
    normal: vec3<f32>,
    uv: vec3<f32>,
    steps: u32,
};

const IDENTITY = mat4x4<f32>(
    vec4<f32>(1.0, 0.0, 0.0, 0.0), 
    vec4<f32>(0.0, 1.0, 0.0, 0.0), 
    vec4<f32>(0.0, 0.0, 1.0, 0.0),
    vec4<f32>(0.0, 0.0, 0.0, 1.0),
);

fn intersect_scene(r: Ray, steps: u32) -> HitInfo {
    let infinity = 1000000000.0 * r.dir;
    return HitInfo(false, 0u, vec4(0.0), infinity, infinity, vec3(0.0), vec3(0.0), steps);
}

const PI: f32 = 3.14159265358979323846264338327950288;

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
    let chunk_size = vec3f(f32(voxel_uniforms.chunk_size));
    let chunk_grid_size = vec3f(f32(voxel_uniforms.offsets_grid_size));

    //ray.pos /= chunk_size;
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

    // outer chunks raycasting
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
            // skip empty chunks
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
    hit_info.material = voxel_uniforms.materials[voxel.data & 0xFFu];
    hit_info.pos = end_ray_pos;
    hit_info.reprojection_pos = ray.pos;
    hit_info.normal = -ray_step * mask;
    hit_info.uv = uv;
    hit_info.steps = u32(iters);
    return hit_info;
}

fn shoot_ray_chunk(ray: Ray, flags: u32) -> HitInfo {
    let chunk_size = vec3f(f32(voxel_uniforms.chunk_size));
    var map_pos = floor(ray.pos);
    var delta_dist = abs(vec3f(length(ray.dir)) / ray.dir);
    var ray_step = sign(ray.dir);
    var side_dist = (sign(ray.dir) * (map_pos - ray.pos) + (sign(ray.dir) * 0.5) + 0.5) * delta_dist; 
    var mask = vec3f(0.0);
    var hit = false;
    var voxel: Voxel;
    var iters = 0;
    for (iters = 0; iters < MAX_RAY_ITERS; iters++) {
		mask = step(side_dist.xyz, side_dist.yzx) * step(side_dist.xyz, side_dist.zxy);
		side_dist += mask * delta_dist;
		map_pos += mask * ray_step;
        if (!in_chunk_bounds(map_pos, vec3f(0.0), chunk_size)) {
            break;
        }
        voxel = get_value(map_pos);
        if ((voxel.data & 0xFFu) != 0u && (((voxel.data >> 8u) & flags) > 0u || flags == 0u)) {
            hit = true;
            break;
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
    hit_info.material = voxel_uniforms.materials[voxel.data & 0xFFu];
    hit_info.pos = end_ray_pos;
    hit_info.reprojection_pos = ray.pos;
    hit_info.normal = -ray_step * mask;
    hit_info.uv = uv;
    hit_info.steps = u32(iters);
    return hit_info;

    /*
    rayCastResults res;
    res.hit = hit;
    res.uv = uv;
    res.mapPos = mapPos;
    res.normal = -rayStep * mask;
    res.tangent = tangent1;
    res.bitangent = tangent2;
    res.rayPos = endRayPos;
    res.dist = length(rayPos - endRayPos);
    return res;
    */
}

// static directional light
const light_dir = vec3<f32>(0.8, -1.0, 0.8);
const light_colour = vec3<f32>(1.0, 1.0, 1.0);

fn calculate_direct(material: vec4<f32>, pos: vec3<f32>, normal: vec3<f32>, mode: u32, seed: vec3<u32>, shadow_samples: u32) -> vec3<f32> {
    // diffuse
    let diffuse = max(dot(normal, -normalize(light_dir)), 0.0);

    // shadow
    var shadow = 1.0;
    if trace_uniforms.shadows != 0u {
        if mode == 2u {
            for (var i = 0u; i < shadow_samples; i += 1u) {
                let rand = hash(seed + i) * 2.0 - 1.0;
                let shadow_ray = Ray(pos, -light_dir + rand * 0.1);
                let shadow_hit = shoot_ray(shadow_ray, 0u);
                shadow -= f32(shadow_hit.hit) / f32(shadow_samples);
            }
        } else {
            let shadow_ray = Ray(pos, -light_dir);
            let shadow_hit = shoot_ray(shadow_ray, 0u);
            shadow = f32(!shadow_hit.hit);
        }
    }

    // emissive
    var emissive = vec3(0.0);
    if material.a != 0.0 {
        emissive = vec3(material.rgb);
    }

    return diffuse * shadow * light_colour + emissive;
}

fn get_voxel(pos: vec3<f32>) -> f32 {
    if any(pos < vec3(0.0)) || any(pos >= vec3(f32(voxel_uniforms.chunk_size))) {
        return 0.0;
    }

    let grid = vec3<i32>(pos).xyz;
    let data = get_at(grid);
    return min(f32(0u & 0xFFu), 1.0);
}

// https://www.shadertoy.com/view/ldl3DS
fn vertex_ao(side: vec2<f32>, corner: f32) -> f32 {
    return (side.x + side.y + max(corner, side.x * side.y)) / 3.1;
}
fn voxel_ao(pos: vec3<f32>, d1: vec3<f32>, d2: vec3<f32>) -> vec4<f32> {
    let side = vec4(get_voxel(pos + d1), get_voxel(pos + d2), get_voxel(pos - d1), get_voxel(pos - d2));
    let corner = vec4(get_voxel(pos + d1 + d2), get_voxel(pos - d1 + d2), get_voxel(pos - d1 - d2), get_voxel(pos + d1 - d2));

    var ao: vec4<f32>;
    ao.x = vertex_ao(side.xy, corner.x);
    ao.y = vertex_ao(side.yz, corner.y);
    ao.z = vertex_ao(side.zw, corner.z);
    ao.w = vertex_ao(side.wx, corner.w);

    return 1.0 - ao;
}
fn glmod(x: vec2<f32>, y: vec2<f32>) -> vec2<f32> {
    return x - y * floor(x / y);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let seed = vec3<u32>(in.position.xyz) * 100u + u32(trace_uniforms.time * 120.0) * 15236u;
    let resolution = vec2<f32>(textureDimensions(normal));
    var jitter = vec2(0.0);
    // if (trace_uniforms.indirect_lighting != 0u) {
    //     jitter = (hash(seed).xy - 0.5) / resolution;
    // }
    var clip_space = vec2(1.0, -1.0) * ((in.uv + jitter) * 2.0 - 1.0);
    var output_colour = vec3(0.0);

    let pos1 = trace_uniforms.camera_inverse * vec4(clip_space.x, clip_space.y, 1.0, 1.0);
    let dir1 = trace_uniforms.camera_inverse * vec4(clip_space.x, clip_space.y, 0.01, 1.0);
    let pos = pos1.xyz / pos1.w;
    let dir = normalize(dir1.xyz / dir1.w - pos);
    var ray = Ray(pos, dir);

    let hit = shoot_ray(ray, 0u);
    var steps = hit.steps;

    // force voxel ambient occlusion
    let mode = 0u;

    var samples = 0.0;
    if hit.hit {
        // direct lighting
        let direct_lighting = calculate_direct(hit.material, hit.pos, hit.normal, mode, seed + 1u, trace_uniforms.samples);

        // indirect lighting
        var indirect_lighting = vec3(0.0);
        if mode == 2u {
            // raytraced indirect lighting
            for (var i = 0u; i < trace_uniforms.samples; i += 1u) {
                let indirect_dir = cosine_hemisphere(hit.normal, seed + i);
                let indirect_hit = shoot_ray(Ray(hit.pos, indirect_dir),0u);
                var lighting = vec3(0.0);
                if indirect_hit.hit {
                    lighting = calculate_direct(indirect_hit.material, indirect_hit.pos, indirect_hit.normal, mode, seed + 3u, 1u);
                } else {
                    lighting = vec3(0.2);
                    // lighting = skybox(indirect_dir, 10.0);
                }
                indirect_lighting += lighting / f32(trace_uniforms.samples);
            }
        } else {
            // voxel ao
            let texture_coords = hit.pos * VOXELS_PER_METER + f32(voxel_uniforms.chunk_size) / 2.0;
            let ao = voxel_ao(texture_coords, hit.normal.zxy, hit.normal.yzx);
            let uv = glmod(vec2(dot(hit.normal * texture_coords.yzx, vec3(1.0)), dot(hit.normal * texture_coords.zxy, vec3(1.0))), vec2(1.0));

            let interpolated_ao_pweig = mix(mix(ao.z, ao.w, uv.x), mix(ao.y, ao.x, uv.x), uv.y);
            let voxel_ao = pow(interpolated_ao_pweig, 1.0 / 3.0);

            indirect_lighting = vec3(2.0 * voxel_ao);
        }

        // final blend
        output_colour = (indirect_lighting + direct_lighting) * hit.material.rgb;
        output_colour = (indirect_lighting + direct_lighting) * hit.material.rgb;
    } else {
        output_colour = skybox(ray.dir, 10.0);
    }

    if trace_uniforms.show_ray_steps != 0u {
        output_colour = vec3<f32>(f32(steps) / 100.0);
    }

    output_colour = max(output_colour, vec3(0.0));
    textureStore(normal, vec2<i32>(in.position.xy), vec4(hit.normal, 0.0));
    textureStore(position, vec2<i32>(in.position.xy), vec4(hit.reprojection_pos, 0.0));
    return vec4<f32>(output_colour, 1.0);
}
