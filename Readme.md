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

- [ ] Voxel raycast (gpu)
    - [x] Simple ambient occlusion
    - [x] Chunk streaming
    - [x] Textured boxes rendering
    - [x] Textured boxes housing smaller voxel grids
    - [ ] Optimize texture boxes lookup
    - [ ] Static octree for seeing further away
    - [ ] GridHierarchy optimization
    - [ ] Particles
- [ ] Physics (cpu)
    - [x] Raycast
    - [x] Box sweep
    - [x] Simple collision solver
    - [x] Character controller
    - [ ] Consider fixed point physics
    - [ ] AABB-ray intersection
- [ ] Cellular Automata (for water, fire, grass, etc.)
    - [ ] floodfill lighting
- [x] Procedural map generation
    [x] Infinite terrain in all directions (Cubic chunks)
- [ ] Mobs
    - [ ] Mob controllers
    - [ ] Spawning
- [ ] Combat system
- [ ] Multiplayer server-based

### Game

- [ ] Procedural map generation
    - [ ] Mountains
    - [ ] Caves
    - [ ] Ore generation
    - [ ] Trees and decoration
- [ ] Ui
- [ ] Dropped items
- [ ] Inventory
- [ ] Crafting
- [ ] Tools
- [ ] Block interaction
    - [ ] Table
    - [ ] Furnace
    - [ ] Chest
- [ ] Farming
- [ ] Automata
    - [ ] Water & Lava
    - [ ] Fire
- [ ] Day/Night cycle
- [ ] Mobs 
    - [ ] Neutral
        - [ ] Cow
        - [ ] Pig
        - [ ] Chicken
        - [ ] Sheep
    - [ ] Hostile
        - [ ] Skeleton
        - [ ] Spider
        - [ ] Creeper
        - [ ] Zombie
         
### Tentative Development Roadmap
[INSPIRATION](https://minecraft-timeline.github.io/) 

- 0.1.0 -> enhanced MC Pre-Classic feature match: basic physics, lighting, multiplayer, generation, ghosts, placing/removing blocks (dirt, grass, stone, cobblestone, wood), sapling to tree, respawn, save
- 0.2.0 -> Full CREATIVE Mode (flight, inventory...)
- 0.3.0 -> MC Classic 1.0 feature match SURVIVAL: mobs (CREEPERS, zombies, spiders) and survival, commands, health, water, lava, sand, gravel, glass, obsidian, TNT
- 0.4.0 -> Crafting, tools, ores (coal, iron, gold, diamond), delay in breaking blocks, particles...
- 0.5.0 -> Farming (wheat, melons, pumpkins) and Animals(sheep, pig, chicken), Armor, Dungeons
- 0.6.0 -> MC Alpha feature matched: Nether, caves, Biomes
- 0.7.0 -> Redstone!
- 0.8.0 -> Pistons, Maps, Compass, Clock, Trapdoors, Rails, boat, fishing
- 0.9.0 -> Feature match with MCbeta1.7.3 -> Weather, server, structures, BEDS
- 1.0.0+ -> Modding support and/or carefully chosen optimizations and enhancements (raytracing lighting? Non-competitive enchanting? Big Trees? HUGE trees? More Structures, even in Ocean? Anvils? Horses? Elytra? VILLAGES!)
