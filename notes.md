# Notes

## Existing work

[Veloren](https://github.com/veloren/veloren)
- amazing rendering/generation
- block mutation is implemented, but barely used.

[dust](https://github.com/dust-engine/dust)
- very good looking
- on top of bevy
- latest castle example not working on my stupid graphics card
- works on windows
- some viewing artifacts and a lot of motion blur
- no physics or mutation shown

[minecraft_bevy](https://github.com/Adamkob12/minecraft_bevy)
- example for bevy_meshem

[bevy_meshem](https://github.com/Adamkob12/bevy_meshem)
- voxel mesh generation library
- bevy integration

[vx-bevy](https://github.com/Game4all/vx_bevy)
- uses block-mesh-rs
- on top of bevy
- just rendering, no physics or gameplay

[kubi](https://github.com/griffi-gh/kubi)
- voxel engine
- cubic chunks

[Recursia](https://github.com/jim-works/Recursia)
- bevy game
- terrain generation
- block destruction
- physics
- laggy

[Rezcraft](https://github.com/Shapur1234/Rezcraft)
- pretty good
- voxel game
- kinda physics (flycam)
- block destruction/construction
- good performance

[infinigen](https://github.com/jameshiew/infinigen)
- terrain generation
- voxel mesh rendering
- cubic chunks
- levels of detail

[wolkenwelten-rs](https://github.com/wolkenwelten/wolkenwelten-rs)
- sandbox voxel engine
- latest not compiling

[block-mesh-rs](https://github.com/bonsairobo/block-mesh-rs)
- voxel mesh generation library
- i don't know how to map faces to block defined uv

[voxel-rs](https://github.com/voxel-rs/voxel-rs)
- extremely old, maybe useful for reference

[all-is-cubes](https://github.com/kpreid/all-is-cubes)
- latest not compiling

[3d_celluar_automata](https://github.com/TanTanDev/3d_celluar_automata)
- voxel rendering
- cool

[first_voxel_engine](https://github.com/TanTanDev/first_voxel_engine)
- latest not compiling


## Prototype

I want to know how the many methods of rendering a voxel grid compare.
The crate binary will implement each method and render the same voxels.

I think the target would be have a render distance of a sphere of 128 blocks of radius.
This is 8784522 ~= 9 million blocks.

For a broad comparison, Minecraft beta at 8 chunks render distance would be ~= 8 million.
``` python
view_distance_chunks = 8
side_chunks = view_distance_chunks * 2
visible_area_chunks = side_chunks*side_chunks
blocks_in_a_chunk = 16*16*128 # = 32768
visible_blocks = blocks_in_a_chunk * visible_area_chunks # = 8388608 ~= 8 million
```
At view distance of 4 chunks there would be 2097152 ~= 2 million blocks.
And Minecraft presumably does it every tick: 2 million / 50ms = 40 million/s.

### Naive method

Render each cube as its own mesh.

### Instanced method

Taken from the bevy shader_instanced example.
Render a material which passes to the gpu the positions of the voxels.

### Benchmark results

The voxels to be rendered are:
1. filled cube 16x16x16 => 4096 blocks
2. filled cube 32x32x32 => 32768 blocks
3. filled cube 64x64x64 => 262144 blocks
4. filled cube 128x128x128 => 2097152 blocks ~= 2 million
5. filled cube 256x256x256 => 16777216 blocks ~= 16 million

Current results:
- naive: 
    1. 9.555991ms
    2. 15.248357ms
    3. 49.935586ms
    4. 501.284233ms
    5. gpu i32 overflow

- instanced:
    1. 2.437907ms
    2. 2.974637ms
    3. 5.860903ms
    4. 45.032350ms
    5. 359.028827ms

Naive and instanced methods are way too slow.
