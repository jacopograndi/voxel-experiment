# Voxel Experiment

Recreation of Minecraft using rust and bevy.
Features a voxel raycaster, cubic chunks and an infinite (i32::MAX) map.

## Roadmap

Work on essential engine features first.
When any becomes a bottleneck consider implementing an optimization.
The game features will be implemented as standalone systems.

1. Essential engine features:
    - Voxel rendering
    - Complex voxels rendering (rail, fence, torch, glass, etc.)
    - Mob rendering
    - Voxel cellular automata support
    - Physics
    - Terrain generation support

## Todo list

### Engine

[ ] Voxel raycast (gpu)
    [x] Simple ambient occlusion
    [x] Chunk streaming
    [x] Textured boxes rendering
    [ ] Textured boxes housing smaller voxel grids
    [ ] Optimize texture boxes lookup
    [ ] Static octree for seeing further away
    [ ] GridHierarchy optimization
    [ ] Particles
[ ] Physics (cpu)
    [x] Raycast
    [x] Box sweep
    [x] Simple collision solver
    [x] Character controller
    [ ] Consider fixed point physics
    [ ] AABB-ray intersection
[ ] Cellular Automata (for water, fire, grass, etc.)
    [ ] floodfill lighting
[x] Procedural map generation
    [x] Infinite terrain in all directions (Cubic chunks)
[ ] Mobs
    [ ] Mob controllers
    [ ] Spawning
[ ] Combat system
[ ] Multiplayer server-based

### Game

[ ] Procedural map generation
    [ ] Mountains
    [ ] Caves
    [ ] Ore generation
    [ ] Trees and decoration
[ ] Ui
[ ] Dropped items
[ ] Inventory
[ ] Crafting
[ ] Tools
[ ] Block interaction
    [ ] Table
    [ ] Furnace
    [ ] Chest
[ ] Farming
[ ] Automata
    [ ] Water & Lava
    [ ] Fire
[ ] Day/Night cycle
[ ] Mobs 
    [ ] Neutral
        [ ] Cow
        [ ] Pig
        [ ] Chicken
        [ ] Sheep
    [ ] Hostile
        [ ] Skeleton
        [ ] Spider
        [ ] Creeper
        [ ] Zombie
