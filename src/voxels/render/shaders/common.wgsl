#define_import_path bevy_voxel_engine::common

const AUTOMATA_FLAG = 128u; // 0b10000000
const PORTAL_FLAG = 64u; // 0b01000000
const ANIMATION_FLAG = 32u; // 0b00100000
const COLLISION_FLAG = 16u; // 0b00010000
const SAND_FLAG = 8u; // 0b00001000

const VOXELS_PER_METER: f32 = 4.0;

struct Portal {
    transformation: mat4x4<f32>,
    position: vec3<f32>,
    normal: vec3<f32>,
}

struct VoxelUniforms {
    materials: array<vec4<f32>, 256>,
    offsets_grid_size: u32,
    chunk_size: u32,
};

struct TraceUniforms {
    camera: mat4x4<f32>,
    camera_inverse: mat4x4<f32>,
    last_camera: mat4x4<f32>,
    projection: mat4x4<f32>,
    time: f32,
    show_ray_steps: u32,
    indirect_lighting: u32,
    samples: u32,
    reprojection_factor: f32,
    shadows: u32,
    misc_bool: u32,
    misc_float: f32,
};


fn get_clip_space(frag_pos: vec4<f32>, dimensions: vec2<f32>) -> vec2<f32> {
    var clip_space = frag_pos.xy / dimensions * 2.0;
    clip_space = clip_space - 1.0;
    clip_space = clip_space * vec2<f32>(1.0, -1.0);
    return clip_space;
}

const k: u32 = 1103515245u;
const PRIME32_2: u32 = 2246822519u;
const PRIME32_3: u32 = 3266489917u;
const PRIME32_4: u32 = 668265263u;
const PRIME32_5: u32 = 374761393u;
fn xxhash32_base(p: vec3<u32>) -> u32 {
	var h32 =  p.z + PRIME32_5 + p.x * PRIME32_3;
	h32 = PRIME32_4 * ((h32 << 17u) | (h32 >> (32u - 17u)));
	h32 += p.y * PRIME32_3;
	h32 = PRIME32_4*((h32 << 17u) | (h32 >> (32u - 17u)));
    h32 = PRIME32_2*(h32^(h32 >> 15u));
    h32 = PRIME32_3*(h32^(h32 >> 13u));
    return h32 ^ (h32 >> 16u);
}
fn xxhash32(p: vec3<u32>) -> vec3<f32> {
    let n = xxhash32_base(p);
    let rz = vec3(n, n * 16807u, n * 48271u); //see: http://random.mat.sbg.ac.at/results/karl/server/node4.html
    return vec3<f32>((rz >> vec3(1u)) & vec3(0x7fffffffu)) / f32(0x7fffffff);
}
// pcg3d
// http://www.jcgt.org/published/0009/03/02/
fn hash(w: vec3<u32>) -> vec3<f32> {
    var v = w * 1664525u + 1013904223u;

    v.x += v.y*v.z;
    v.y += v.z*v.x;
    v.z += v.x*v.y;

    v ^= v >> vec3(16u);

    v.x += v.y*v.z;
    v.y += v.z*v.x;
    v.z += v.x*v.y;

    return vec3<f32>((v >> vec3(1u)) & vec3(0x7fffffffu)) / f32(0x7fffffff);
}
fn old_hash(x: vec3<u32>) -> vec3<f32> {
    let x1 = ((x >> vec3<u32>(8u)) ^ x.yzx) * k;
    let x2 = ((x >> vec3<u32>(8u)) ^ x.yzx) * k;
    let xx = ((x >> vec3<u32>(8u)) ^ x.yzx) * k;
    
    return vec3<f32>(xx) * (1.0 / f32(0xffffffffu));
}

const pi = 3.14159265359;

fn cosine_hemisphere(n: vec3<f32>, seed: vec3<u32>) -> vec3<f32> {
    let u = hash(seed);

    let r = sqrt(u.x);
    let theta = 2.0 * pi * u.y;
 
    let  b = normalize(cross(n, vec3<f32>(0.0, 1.0, 1.0)));
    let  t = cross(b, n);
    
    return normalize(r * sin(theta) * b + sqrt(1.0 - u.x) * n + r * cos(theta) * t);
}

struct Ray {
    pos: vec3<f32>,
    dir: vec3<f32>,
};

// returns the closest intersection and the furthest intersection
fn ray_box_dist(r: Ray, vmin: vec3<f32>, vmax: vec3<f32>) -> vec2<f32> {
    let v1 = (vmin.x - r.pos.x) / r.dir.x;
    let v2 = (vmax.x - r.pos.x) / r.dir.x;
    let v3 = (vmin.y - r.pos.y) / r.dir.y;
    let v4 = (vmax.y - r.pos.y) / r.dir.y;
    let v5 = (vmin.z - r.pos.z) / r.dir.z;
    let v6 = (vmax.z - r.pos.z) / r.dir.z;
    let v7 = max(max(min(v1, v2), min(v3, v4)), min(v5, v6));
    let v8 = min(min(max(v1, v2), max(v3, v4)), max(v5, v6));
    if (v8 < 0.0 || v7 > v8) {
        return vec2(0.0);
    }

    return vec2(v7, v8);
}

fn ray_plane(r: Ray, pos: vec3<f32>, normal: vec3<f32>) -> vec4<f32> {
    let denom = dot(normal, r.dir);
    if (denom < 0.00001) {
        let t = dot(normal, pos - r.pos) / denom;
        if (t >= 0.0) {
            let pos = r.pos + r.dir * t;
            return vec4(pos, t);
        }
    }
    return vec4(0.0);
}

fn in_bounds(v: vec3<f32>) -> bool {
    let s = step(vec3<f32>(-1.0), v) - step(vec3<f32>(1.0), v);
    return (s.x * s.y * s.z) > 0.5;
}

fn clip_aabb(hist: vec3<f32>, min_aabb: vec3<f32>, max_aabb: vec3<f32>) -> vec3<f32> {
    let p_clip = 0.5 * (max_aabb + min_aabb);
    let e_clip = 0.5 * (max_aabb - min_aabb);

    let v_clip = hist - p_clip;
    let v_unit = v_clip / e_clip;
    let a_unit = abs(v_unit);
    let max_unit = max(a_unit.x, max(a_unit.y, a_unit.z));

    if (max_unit > 1.0) {
        return p_clip + v_clip / max_unit;
    } else {
        return hist; // point inside aabb
    }
}

//j fun function
fn skybox(dir: vec3<f32>, time_of_day: f32) -> vec3<f32> {
    var dir1: vec3<f32>;
    var time_of_day1: f32;
    var t: f32;
    var sun_pos: vec3<f32>;
    var col: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    var p_sunset_dark: array<vec3<f32>,4u> = array<vec3<f32>,4u>(vec3<f32>(0.3720705509185791, 0.3037080764770508, 0.26548632979393005), vec3<f32>(0.4461638331413269, 0.3940589129924774, 0.42567673325538635), vec3<f32>(0.16514907777309418, 0.4046129286289215, 0.8799446225166321), vec3<f32>(-0.00000000000000007057075514128395, -0.08647964149713516, -0.26904296875));
    var p_sunset_bright: array<vec3<f32>,4u> = array<vec3<f32>,4u>(vec3<f32>(0.38976746797561646, 0.3156035840511322, 0.27932655811309814), vec3<f32>(1.2874523401260376, 1.0100154876708984, 0.8623254299163818), vec3<f32>(0.1260504275560379, 0.23134452104568481, 0.5261799693107605), vec3<f32>(-0.09298685193061829, -0.0733446329832077, -0.19287726283073425));
    var p_day: array<vec3<f32>,4u> = array<vec3<f32>,4u>(vec3<f32>(0.05101049691438675, 0.0975874736905098, 0.14233364164829254), vec3<f32>(0.721604585647583, 0.8130766749382019, 0.9907063245773315), vec3<f32>(0.23738746345043182, 0.6037047505378723, 1.279274582862854), vec3<f32>(-0.000000000000000483417267228435, 0.13545893132686615, -0.0000000000000014694301099188));
    var brightness_a: f32;
    var brightness_d: f32;
    var p_sunset: array<vec3<f32>,4u>;
    var sun_a: f32;
    var sun_d: f32;
    var a: vec3<f32>;
    var b: vec3<f32>;
    var c: vec3<f32>;
    var d: vec3<f32>;
    var sky_a: f32;
    var sky_d: f32;
    var sun_a1: f32;
    var sun_col: vec3<f32>;

    dir1 = dir;
    time_of_day1 = time_of_day;
    let e4: f32 = time_of_day1;
    t = ((e4 + 4.0) * ((360.0 / 24.0) * 0.017453292519943295));
    let e19: f32 = t;
    let e23: f32 = t;
    let e28: f32 = t;
    let e32: f32 = t;
    sun_pos = normalize(vec3<f32>(0.0, -(sin(e28)), cos(e32)));
    {
        let e104: vec3<f32> = dir1;
        let e105: vec3<f32> = sun_pos;
        let e109: vec3<f32> = dir1;
        let e110: vec3<f32> = sun_pos;
        brightness_a = acos(dot(e109, e110));
        let e132: f32 = brightness_a;
        brightness_d = ((1.5 * smoothstep((80.0 * 0.017453292519943295), (0.0 * 0.017453292519943295), e132)) - 0.5);
        let e139: array<vec3<f32>,4u> = p_sunset_dark;
        let e142: array<vec3<f32>,4u> = p_sunset_bright;
        let e146: array<vec3<f32>,4u> = p_sunset_dark;
        let e149: array<vec3<f32>,4u> = p_sunset_bright;
        let e151: f32 = brightness_d;
        let e155: array<vec3<f32>,4u> = p_sunset_dark;
        let e158: array<vec3<f32>,4u> = p_sunset_bright;
        let e162: array<vec3<f32>,4u> = p_sunset_dark;
        let e165: array<vec3<f32>,4u> = p_sunset_bright;
        let e167: f32 = brightness_d;
        let e171: array<vec3<f32>,4u> = p_sunset_dark;
        let e174: array<vec3<f32>,4u> = p_sunset_bright;
        let e178: array<vec3<f32>,4u> = p_sunset_dark;
        let e181: array<vec3<f32>,4u> = p_sunset_bright;
        let e183: f32 = brightness_d;
        let e187: array<vec3<f32>,4u> = p_sunset_dark;
        let e190: array<vec3<f32>,4u> = p_sunset_bright;
        let e194: array<vec3<f32>,4u> = p_sunset_dark;
        let e197: array<vec3<f32>,4u> = p_sunset_bright;
        let e199: f32 = brightness_d;
        p_sunset = array<vec3<f32>,4u>(mix(e146[0], e149[0], vec3<f32>(e151)), mix(e162[1], e165[1], vec3<f32>(e167)), mix(e178[2], e181[2], vec3<f32>(e183)), mix(e194[3], e197[3], vec3<f32>(e199)));
        let e209: vec3<f32> = sun_pos;
        let e220: vec3<f32> = sun_pos;
        sun_a = acos(dot(e220, vec3<f32>(0.0, 1.0, 0.0)));
        let e245: f32 = sun_a;
        sun_d = smoothstep((100.0 * 0.017453292519943295), (60.0 * 0.017453292519943295), e245);
        let e249: array<vec3<f32>,4u> = p_sunset;
        let e252: array<vec3<f32>,4u> = p_day;
        let e256: array<vec3<f32>,4u> = p_sunset;
        let e259: array<vec3<f32>,4u> = p_day;
        let e261: f32 = sun_d;
        a = mix(e256[0], e259[0], vec3<f32>(e261));
        let e266: array<vec3<f32>,4u> = p_sunset;
        let e269: array<vec3<f32>,4u> = p_day;
        let e273: array<vec3<f32>,4u> = p_sunset;
        let e276: array<vec3<f32>,4u> = p_day;
        let e278: f32 = sun_d;
        b = mix(e273[1], e276[1], vec3<f32>(e278));
        let e283: array<vec3<f32>,4u> = p_sunset;
        let e286: array<vec3<f32>,4u> = p_day;
        let e290: array<vec3<f32>,4u> = p_sunset;
        let e293: array<vec3<f32>,4u> = p_day;
        let e295: f32 = sun_d;
        c = mix(e290[2], e293[2], vec3<f32>(e295));
        let e300: array<vec3<f32>,4u> = p_sunset;
        let e303: array<vec3<f32>,4u> = p_day;
        let e307: array<vec3<f32>,4u> = p_sunset;
        let e310: array<vec3<f32>,4u> = p_day;
        let e312: f32 = sun_d;
        d = mix(e307[3], e310[3], vec3<f32>(e312));
        let e321: vec3<f32> = dir1;
        let e332: vec3<f32> = dir1;
        sky_a = acos(dot(e332, vec3<f32>(0.0, 1.0, 0.0)));
        let e357: f32 = sky_a;
        sky_d = smoothstep((90.0 * 0.017453292519943295), (60.0 * 0.017453292519943295), e357);
        let e360: vec3<f32> = col;
        let e361: vec3<f32> = b;
        let e362: vec3<f32> = d;
        let e365: f32 = sky_d;
        let e367: vec3<f32> = c;
        let e377: vec3<f32> = a;
        let e382: f32 = sky_d;
        let e384: vec3<f32> = c;
        let e394: vec3<f32> = a;
        let e400: vec3<f32> = d;
        col = (e360 + (((e361 - e362) * sin((vec3<f32>(1.0) / (((vec3<f32>(e382) / e384) + vec3<f32>((2.0 / (180.0 * 0.017453292519943295)))) - e394)))) + e400));
    }
    let e405: vec3<f32> = sun_pos;
    let e406: vec3<f32> = dir1;
    let e410: vec3<f32> = sun_pos;
    let e411: vec3<f32> = dir1;
    sun_a1 = acos(dot(e410, e411));
    let e421: f32 = sun_a1;
    sun_col = ((0.009999999776482582 * vec3<f32>(1.0, 0.949999988079071, 0.949999988079071)) / vec3<f32>(e421));
    let e425: vec3<f32> = col;
    let e427: vec3<f32> = sun_col;
    let e431: vec3<f32> = col;
    let e433: vec3<f32> = sun_col;
    let e436: vec3<f32> = sun_col;
    col = max((e431 + (0.5 * e433)), e436);
    let e438: vec3<f32> = col;
    return e438;
}
