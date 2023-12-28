#import bevy_voxel_engine::common::VoxelUniforms

@group(0) @binding(0)
var<uniform> voxel_uniforms: VoxelUniforms;
@group(0) @binding(1)
var<storage, read_write> chunks: array<u32>;
@group(0) @binding(2)
var<storage, read> chunks_loading: array<u32>;
@group(0) @binding(3)
var<storage, read> chunks_loading_offsets: array<u32>;

const EMPTY_CHUNK = 4294967295u;
const MAX_COPY_ITERS = 100000u;

@compute @workgroup_size(4, 4, 4)
fn copy(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let chunk_size = voxel_uniforms.chunk_size;
    let chunk_volume = chunk_size * chunk_size * chunk_size;
    let id = invocation_id.x * chunk_size * chunk_size 
        + invocation_id.y * chunk_size 
        + invocation_id.z
    ;
    for (var i = 0u; i < MAX_COPY_ITERS; i++) {
        let offset = chunks_loading_offsets[i];
        if (offset == EMPTY_CHUNK) {
            return;
        }
        let from_linear = chunk_volume * i + id;
        let to_offset = offset + id;
        chunks[to_offset] = chunks_loading[from_linear];
    }
}

