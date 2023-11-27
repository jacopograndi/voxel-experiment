#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_voxel_engine::common::{
    VoxelUniforms, TraceUniforms, Ray, VOXELS_PER_METER, hash, 
    clip_aabb, ray_plane, in_bounds, ray_box_dist, PORTAL_FLAG, cosine_hemisphere, skybox
}

@group(0) @binding(0)
var<uniform> voxel_uniforms: VoxelUniforms;
@group(0) @binding(1)
var voxel_world: texture_storage_3d<r16uint, read_write>;
@group(0) @binding(2)
var<storage, read_write> gh: array<u32>;
@group(0) @binding(3)
var mip: texture_3d<f32>;
@group(0) @binding(4)
var texture_sampler: sampler;

@group(1) @binding(0)
var<uniform> trace_uniforms: TraceUniforms;
@group(1) @binding(2)
var normal: texture_storage_2d<rgba16float, read_write>;
@group(1) @binding(3)
var position: texture_storage_2d<rgba32float, read_write>;

// note: raytracing.wgsl requires common.wgsl and for you to define u, voxel_world and gh before you import it
// i copy pasted raytracing.wsgl
fn get_value_index(index: u32) -> bool {
    return ((gh[index / 32u] >> (index % 32u)) & 1u) != 0u;
    //return true;
}

struct Voxel {
    data: u32,
    pos: vec3<f32>,
    grid_size: u32,
};

fn get_value(pos: vec3<f32>) -> Voxel {
    let scaled = pos * 0.5 + 0.5;

    let size0 = voxel_uniforms.levels[0].x;
    let size1 = voxel_uniforms.levels[1].x;
    let size2 = voxel_uniforms.levels[2].x;
    let size3 = voxel_uniforms.levels[3].x;
    let size4 = voxel_uniforms.levels[4].x;
    let size5 = voxel_uniforms.levels[5].x;
    let size6 = voxel_uniforms.levels[6].x;
    let size7 = voxel_uniforms.levels[7].x;

    let scaled0 = vec3<u32>(scaled * f32(size0));
    let scaled1 = vec3<u32>(scaled * f32(size1));
    let scaled2 = vec3<u32>(scaled * f32(size2));
    let scaled3 = vec3<u32>(scaled * f32(size3));
    let scaled4 = vec3<u32>(scaled * f32(size4));
    let scaled5 = vec3<u32>(scaled * f32(size5));
    let scaled6 = vec3<u32>(scaled * f32(size6));
    let scaled7 = vec3<u32>(scaled * f32(size7));

    let state0 = get_value_index(voxel_uniforms.offsets[0].x + scaled0.x * size0 * size0 + scaled0.y * size0 + scaled0.z);
    let state1 = get_value_index(voxel_uniforms.offsets[1].x + scaled1.x * size1 * size1 + scaled1.y * size1 + scaled1.z);
    let state2 = get_value_index(voxel_uniforms.offsets[2].x + scaled2.x * size2 * size2 + scaled2.y * size2 + scaled2.z);
    let state3 = get_value_index(voxel_uniforms.offsets[3].x + scaled3.x * size3 * size3 + scaled3.y * size3 + scaled3.z);
    let state4 = get_value_index(voxel_uniforms.offsets[4].x + scaled4.x * size4 * size4 + scaled4.y * size4 + scaled4.z);
    let state5 = get_value_index(voxel_uniforms.offsets[5].x + scaled5.x * size5 * size5 + scaled5.y * size5 + scaled5.z);
    let state6 = get_value_index(voxel_uniforms.offsets[6].x + scaled6.x * size6 * size6 + scaled6.y * size6 + scaled6.z);
    let state7 = get_value_index(voxel_uniforms.offsets[7].x + scaled7.x * size7 * size7 + scaled7.y * size7 + scaled7.z);

    if (!state0 && size0 != 0u) {
        let rounded_pos = ((vec3<f32>(scaled0) + 0.5) / f32(size0)) * 2.0 - 1.0;
        return Voxel(0u, rounded_pos, size0);
    }
    if (!state1 && size1 != 0u) {
        let rounded_pos = ((vec3<f32>(scaled1) + 0.5) / f32(size1)) * 2.0 - 1.0;
        return Voxel(0u, rounded_pos, size1);
    }
    if (!state2 && size2 != 0u) {
        let rounded_pos = ((vec3<f32>(scaled2) + 0.5) / f32(size2)) * 2.0 - 1.0;
        return Voxel(0u, rounded_pos, size2);
    }
    if (!state3 && size3 != 0u) {
        let rounded_pos = ((vec3<f32>(scaled3) + 0.5) / f32(size3)) * 2.0 - 1.0;
        return Voxel(0u, rounded_pos, size3);
    }
    if (!state4 && size4 != 0u) {
        let rounded_pos = ((vec3<f32>(scaled4) + 0.5) / f32(size4)) * 2.0 - 1.0;
        return Voxel(0u, rounded_pos, size4);
    }
    if (!state5 && size5 != 0u) {
        let rounded_pos = ((vec3<f32>(scaled5) + 0.5) / f32(size5)) * 2.0 - 1.0;
        return Voxel(0u, rounded_pos, size5);
    }
    if (!state6 && size6 != 0u) {
        let rounded_pos = ((vec3<f32>(scaled6) + 0.5) / f32(size6)) * 2.0 - 1.0;
        return Voxel(0u, rounded_pos, size6);
    }
    if (!state7 && size7 != 0u) {
        let rounded_pos = ((vec3<f32>(scaled7) + 0.5) / f32(size7)) * 2.0 - 1.0;
        return Voxel(0u, rounded_pos, size7);
    }

    let rounded_pos = (floor(pos * f32(voxel_uniforms.texture_size) * 0.5) + 0.5) / (f32(voxel_uniforms.texture_size) * 0.5);
    let data = textureLoad(voxel_world, vec3<i32>(scaled * f32(voxel_uniforms.texture_size)).zyx).r;
    return Voxel(data, rounded_pos, voxel_uniforms.texture_size);
}

struct HitInfo {
    hit: bool,
    data: u32,
    material: vec4<f32>,
    pos: vec3<f32>,
    reprojection_pos: vec3<f32>,
    normal: vec3<f32>,
    portals: mat4x4<f32>,
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
    return HitInfo(false, 0u, vec4(0.0), infinity, infinity, vec3(0.0), IDENTITY, steps);
}

const PI: f32 = 3.14159265358979323846264338327950288;

/// physics_distance is in terms of t so make sure to normalize your 
/// ray direction if you want it to be in world cordinates.
/// only hits voxels that have any of the flags set or hits everything if flags is 0
fn shoot_ray(r: Ray, _physics_distance: f32, flags: u32) -> HitInfo {
    let wtr = VOXELS_PER_METER * 2.0 / f32(voxel_uniforms.texture_size); // world to render ratio
    let rtw = f32(voxel_uniforms.texture_size) / (VOXELS_PER_METER * 2.0); // render to world ratio

    let physics_distance = _physics_distance * wtr;
    var pos = r.pos * wtr;
    let dir_mask = vec3<f32>(r.dir == vec3(0.0));
    var dir = r.dir + dir_mask * 0.000001;

    var distance = 0.0;
    if (!in_bounds(pos)) {
        // Get position on surface of the octree
        let dist = ray_box_dist(Ray(pos, dir), vec3(-1.0), vec3(1.0)).x;
        if (dist == 0.0) {
            if (physics_distance > 0.0) {
                return HitInfo(false, 0u, vec4(0.0), (pos + dir * physics_distance) * rtw, vec3(0.0), vec3(0.0), IDENTITY, 1u);
            }
            return intersect_scene(Ray(pos, dir), 1u);
        }

        pos = pos + dir * dist;
        distance += dist;
    }

    var r_sign = sign(dir);
    var tcpotr = pos; // the current position of the ray
    var steps = 0u;
    var normal = trunc(pos * 1.00001);
    var voxel = Voxel(0u, vec3(0.0), 0u);
    var portal_mat = IDENTITY;
    var reprojection_pos = pos;
    while (steps < 1000u) {
        voxel = get_value(tcpotr);

        let should_portal_skip = ((voxel.data >> 8u) & PORTAL_FLAG) > 0u;
        if ((voxel.data & 0xFFu) != 0u && !should_portal_skip && (((voxel.data >> 8u) & flags) > 0u || flags == 0u)) {
            break;
        }

        let voxel_size = 2.0 / f32(voxel.grid_size);
        let t_max = (voxel.pos - pos + r_sign * voxel_size / 2.0) / dir;

        // https://www.shadertoy.com/view/4dX3zl (good old shader toy)
        let mask = vec3<f32>(t_max.xyz <= min(t_max.yzx, t_max.zxy));
        normal = mask * -r_sign;

        let t_current = min(min(t_max.x, t_max.y), t_max.z);
        tcpotr = pos + dir * t_current - normal * 0.000002;
        reprojection_pos = r.pos + (t_current + distance) * r.dir * rtw;

        if (t_current + distance > physics_distance && physics_distance > 0.0) {
            return HitInfo(false, 0u, vec4(0.0), (pos + dir * (physics_distance - distance)) * rtw, vec3(0.0), vec3(0.0), portal_mat, steps);
        }

        if (!in_bounds(tcpotr)) {
            if (physics_distance > 0.0) {
                return HitInfo(false, 0u, vec4(0.0), (pos + dir * (physics_distance - distance)) * rtw, vec3(0.0), vec3(0.0), portal_mat, steps);
            }
            return intersect_scene(Ray(pos, dir), steps);
        }

        steps = steps + 1u;
    }

    return HitInfo(true, voxel.data, voxel_uniforms.materials[voxel.data & 0xFFu], tcpotr * rtw + normal * 0.0001, reprojection_pos, normal, portal_mat, steps);
}
// end i copy pasted raytracing.wsgl

// static directional light
const light_dir = vec3<f32>(0.8, -1.0, 0.8);
const light_colour = vec3<f32>(1.0, 1.0, 1.0);

fn calculate_direct(material: vec4<f32>, pos: vec3<f32>, normal: vec3<f32>, mode: u32, seed: vec3<u32>, shadow_samples: u32) -> vec3<f32> {
    // diffuse
    let diffuse = max(dot(normal, -normalize(light_dir)), 0.0);

    // shadow
    var shadow = 1.0;
    if trace_uniforms.shadows != 0u {
        if mode == 1u {
            let shadow_ray = Ray(pos, -light_dir);
            let col = voxel_cone_raytracing(shadow_ray, 0.1);
            shadow = 1.0 - col.a;
        } else if mode == 2u {
            for (var i = 0u; i < shadow_samples; i += 1u) {
                let rand = hash(seed + i) * 2.0 - 1.0;
                let shadow_ray = Ray(pos, -light_dir + rand * 0.1);
                let shadow_hit = shoot_ray(shadow_ray, 0.0, 0u);
                shadow -= f32(shadow_hit.hit) / f32(shadow_samples);
            }
        } else {
            let shadow_ray = Ray(pos, -light_dir);
            let shadow_hit = shoot_ray(shadow_ray, 0.0, 0u);
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
    if any(pos < vec3(0.0)) || any(pos >= vec3(f32(voxel_uniforms.texture_size))) {
        return 0.0;
    }

    let voxel = textureLoad(voxel_world, vec3<i32>(pos.zyx));
    return min(f32(voxel.r & 0xFFu), 1.0);
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

fn voxel_cone_raytracing(ray: Ray, angle: f32) -> vec4<f32> {
    var color = vec4(0.0);
    var steps = 0u;
    // var distance = 0.025;
    var distance = 0.3 * angle;
    var tcpotr = ray.pos + ray.dir * distance;
    tcpotr = tcpotr * VOXELS_PER_METER / f32(voxel_uniforms.texture_size) + 0.5;
    loop {
        let size = distance * tan(angle);
        let mip_level = log2(size * f32(voxel_uniforms.texture_size));

        let col = textureSampleLevel(mip, texture_sampler, tcpotr.zyx, mip_level);
        color += col;
        if color.a > 1.0 {
            color.a = 1.0;
            break;
        }

        tcpotr += ray.dir * size;
        distance += size;
        if any(tcpotr < vec3(0.0)) || any(tcpotr >= vec3(1.0)) {
            break;
        }

        steps += 1u;
        if steps > 200u {
            break;
        }
    }

    return color;
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

    let hit = shoot_ray(ray, 0.0, 0u);
    var steps = hit.steps;

    // force voxel ambient occlusion
    let mode = 0u;

    var samples = 0.0;
    if hit.hit {
        // direct lighting
        let direct_lighting = calculate_direct(hit.material, hit.pos, hit.normal, mode, seed + 1u, trace_uniforms.samples);

        // indirect lighting
        var indirect_lighting = vec3(0.0);
        if mode == 1u {
            // voxel ao using voxel_cone_raytracing
            let texture_coords = hit.pos * VOXELS_PER_METER + f32(voxel_uniforms.texture_size) / 2.0;
            let ao = voxel_ao(texture_coords, hit.normal.zxy, hit.normal.yzx);
            let uv = glmod(vec2(dot(hit.normal * texture_coords.yzx, vec3(1.0)), dot(hit.normal * texture_coords.zxy, vec3(1.0))), vec2(1.0));

            let interpolated_ao_pweig = mix(mix(ao.z, ao.w, uv.x), mix(ao.y, ao.x, uv.x), uv.y);
            let voxel_ao = pow(interpolated_ao_pweig, 1.0 / 3.0);

            // voxel cone tracing
            let up = hit.normal;
            var right = cross(up, vec3(0.0, 0.0, 1.0));
            if all(right == vec3(0.0)) {
                right = cross(up, vec3(0.0, 1.0, 0.0));
            }
            let forward = normalize(cross(right, up));

            var color = voxel_cone_raytracing(Ray(hit.pos, up), 0.5);
            color += voxel_cone_raytracing(Ray(hit.pos, cos(0.5) * cos(1.257 * 0.0) * right + sin(0.5) * up + cos(0.5) * sin(1.257 * 0.0) * forward), 0.5);
            color += voxel_cone_raytracing(Ray(hit.pos, cos(0.5) * cos(1.257 * 1.0) * right + sin(0.5) * up + cos(0.5) * sin(1.257 * 1.0) * forward), 0.5);
            color += voxel_cone_raytracing(Ray(hit.pos, cos(0.5) * cos(1.257 * 2.0) * right + sin(0.5) * up + cos(0.5) * sin(1.257 * 2.0) * forward), 0.5);
            color += voxel_cone_raytracing(Ray(hit.pos, cos(0.5) * cos(1.257 * 3.0) * right + sin(0.5) * up + cos(0.5) * sin(1.257 * 3.0) * forward), 0.5);
            color += voxel_cone_raytracing(Ray(hit.pos, cos(0.5) * cos(1.257 * 4.0) * right + sin(0.5) * up + cos(0.5) * sin(1.257 * 4.0) * forward), 0.5);
            color /= 6.0;
            let sky = (1.0 - color.a);
            let indirect = color.rgb * color.a * 0.1;

            indirect_lighting = vec3(0.3 * voxel_ao * sky);
        } else if mode == 2u {
            // raytraced indirect lighting
            for (var i = 0u; i < trace_uniforms.samples; i += 1u) {
                let indirect_dir = cosine_hemisphere(hit.normal, seed + i);
                let indirect_hit = shoot_ray(Ray(hit.pos, indirect_dir), 0.0, 0u);
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
            let texture_coords = hit.pos * VOXELS_PER_METER + f32(voxel_uniforms.texture_size) / 2.0;
            let ao = voxel_ao(texture_coords, hit.normal.zxy, hit.normal.yzx);
            let uv = glmod(vec2(dot(hit.normal * texture_coords.yzx, vec3(1.0)), dot(hit.normal * texture_coords.zxy, vec3(1.0))), vec2(1.0));

            let interpolated_ao_pweig = mix(mix(ao.z, ao.w, uv.x), mix(ao.y, ao.x, uv.x), uv.y);
            let voxel_ao = pow(interpolated_ao_pweig, 1.0 / 3.0);

            indirect_lighting = vec3(0.3 * voxel_ao);
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
