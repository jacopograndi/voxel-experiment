# Bugs:

- The tracing distance is incorrect when tracing through subgrids

- Fences are considered for ambient occlusion

---

# Tasks

- Main grid voxel textures handling

    0. Create some basic voxel models
    
        - [ ] Dirt
        - [ ] Stone
        - [ ] Wood
        - [ ] Fence
        - [ ] Torch

    1. Load the .vox textures from multiple folders
    2. Load the BlockInfo data from multiple folders
    3. Assign to each BlockInfo a BlockId
    4. The voxel.id is the BlockId
    5. Construct the textures buffer

        - Contiguous
        - The first 256 (can be expanded if needed) are 16x16x16 voxels

    6. While tracing, use the voxel.id as the offset in the texture buffer

- Better naming 

    - The main grid (Current name: ChunkMap)
        - Is divided into chunks (32x32x32 blocks) (Current name: Chunk or GridPtr or Grid)
            - Which are composed of blocks (16x16x16 voxels) (Current name: TexturedBox)

                Which are composed of tiny colored voxels
    
    - The textured boxes (Current name TexturedBox)
        - Are composed of tiny colored voxels

    This naming is terrible. 
    Consider this structure:

    - Voxel: Tiny colored voxels s. t. a 16x16x16 cube of them has a side 1m long.
        - TexturedBox: 

            A cuboid that can be translated, rotated and scaled.
            Contains a voxel model made of tiny colored voxels.

        - Block: grid of 16x16x16 Voxels
            - ChunkGrid: grid of 32x32x32 Blocks
                - WorldGrid: grid of ChunkGrids


- Floodfill lighting

    Each voxel has a light level from 0 to 15.
    Light sources have a light value > 0.
    A block has the light level of the max(light source - manhattan distance from source).
    A solution to this constraints is a floodfill algorithm:

    1. Each tick, step through each voxel
    2. Look at the 26 neighboring blocks
    3. Block light is max(neighbor light) - 1

    In Minecraft the sun lights all the highest blocks with a value of 15.
    When the night comes, the value goes gradually to 0.
    This may be a performance problem: i have to modify every clear column once every ~15 seconds.

- Speed up voxel textures tracing

    Now i have to cycle each voxel texture box to check if it's a hit for each pixel.
    I should check only the ones that touch current voxel while tracing the main voxel grid.
    Doing so requires a HashMap<IVec3, Vec<TextureOffset>>, which is huge.
    If i restrict the key IVec3 to being only 0..32 values, it would be smaller.
    I could restrict it further if needed, it shouldn't need to be chunk-aligned.

    1. HashMap<IVec3ModuloChunkSide, Vec<TextureOffset>> shader support
        
        - texture_offsets: array<u32>
        - keys: array<TextureOffsets>
        - struct TextureOffset { offset: u32, size: u32 }
        - Index into keys with the current map_pos and cycle size times 

    2. Construct the texture_offsets and keys from the extracted TextureBoxes every frame
    3. Cache the hit for the current pixel if i have large TextureBoxes.
    4. Test with and without optimization

- Transparency support

    Allow rays to continue through voxels that have a color.alpha < 1.0.
    Consider the length of ray that passed through the cube to darken it subsequently.

- Voxel animation support

    The voxel data could be written every frame like i do with chunk streaming.
    I could lookup a table BlockId to frames for cycling animations like fire.
