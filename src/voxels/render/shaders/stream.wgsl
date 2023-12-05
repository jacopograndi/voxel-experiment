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
const CHUNK_SIZE = 32768u; // 32^3

// increase parallelism using workgroup_size
@compute @workgroup_size(1)
fn copy(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let invocation = u32(invocation_id.x);
    for (var i = 0u; i < 16u; i++) {
        let offset = chunks_loading_offsets[i];
        if (offset == EMPTY_CHUNK) {
            return;
        }
        for (var j = 0u; j < CHUNK_SIZE / 1u; j++) {
            let from_linear = CHUNK_SIZE * i + j;
            let to_offset = offset + j;
            chunks[to_offset] = chunks_loading[from_linear];
            //chunks[to_offset] = 1u;
        }
    }
}

