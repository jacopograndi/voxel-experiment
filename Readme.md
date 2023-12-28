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

- 0.1.0 -> MC Pre-Classic feature match with enhancements:
    - Basic Blocks&Textures: Air, Stone, Grass, Dirt, Cobblestone, Wood 
        - Removing with Left Click -> No Animations!
        - Placing with Right Click 
    - Basic physics/player movement + lighting
    - Basic multiplayer
    - Basic generation (flatland and at most hills)
    - Basic ghosts (granular block placement/rotation)
    - Respawn at origin
    - Save Universe
- 0.2.0 -> Full CREATIVE Mode
    - New Blocks: Bedrock, Sand, Gravel  
    - Add Water and Lava+Obsidian
    - Generation of lakes
    - Basic flight mechanics
    - Inventory with all blocks + Dock
- 0.3.0 -> MC Classic 1.0 feature match SURVIVAL
    - Mobs (CREEPERS, zombies, spiders, skeletons)
    - Basic commands
    - Health Bar
    - Basic Crafting (Saplings>Log>Planks, Sand>Glass)
    - Chests
- 0.4.0 -> MINING!
    - Ores (coal, iron, gold, diamond)
    - Smelting
    - Tools (sword, pickaxe, axe, hoe, bow)
    - Armor
    - Delay in breaking blocks, animations/particles...
- 0.5.0 -> FARMING!
    - Wheat, melons, pumpkins
    - Sheep, pig, chicken
- 0.6.0 -> REDSTONE!
    - Ore, dust, repeater, torch, switches, pistons
- 0.7.0 -> MC Alpha feature matched
    - Nether/portals + Netherrack/Glowstone...
    - Caves/Dungeons
    - Basic Biomes (Woods, Plains, Beach, Ocean, Mountains)
- 0.8.0 -> EXPLORATION!
    - Maps, Compass, Clock,
    - Doors, Trapdoors
    - Minecart, Rails
    - Boat, fishing
    - BEDS for resetting respawn!
- 0.9.0 -> Feature match with MCbeta1.7.3
    - Weather and Day/Nighy
    - Villages and NPCs
    - NO HUNGER
- 1.0.0+ -> Modding support and/or carefully chosen optimizations and enhancements (raytracing lighting? Non-competitive enchanting? Non-Competitive End? Big Trees? HUGE trees? More Structures, even in Ocean? Anvils? Horses? Elytra? Compilation of client-only for WebAssembly)
