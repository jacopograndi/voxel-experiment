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

Reference:
- Minecraft Pre-Classic (May 13-16 '09) [HERE](https://minecraft.fandom.com/wiki/Java_Edition_pre-Classic_rd-131648)
- Minecraft Classic (May.17-Nov.10'09) [HERE](https://minecraft.fandom.com/wiki/Java_Edition_Classic_0.0.2a)
    - Early (May.17-29'09) [HERE](https://minecraft.fandom.com/wiki/Java_Edition_Classic_0.0.11a)
    - Multiplayer (May.31-Jul.11'09) [HERE](https://minecraft.fandom.com/wiki/Java_Edition_Classic_0.0.15a_(Multiplayer_Test_1))
    - Survival (Sep.1-Oct.24'09) [HERE](https://minecraft.fandom.com/wiki/Java_Edition_Classic_0.24_SURVIVAL_TEST)
    - Late (Oct.27-Nov.10'09) [HERE](https://minecraft.fandom.com/wiki/Java_Edition_Classic_0.28)
- Minecraft Indev (Dec.23'09-Feb.23'10) [HERE](https://minecraft.fandom.com/wiki/Java_Edition_Indev_0.31_20091223-1)
- Minecraft Infdev (Feb.27-Jun30'10) [HERE](https://minecraft.fandom.com/wiki/Java_Edition_Infdev_20100227-1)
- Minecraft Alpha (Jun.30-Dec.3'10) [HERE](https://minecraft.fandom.com/wiki/Java_Edition_Alpha_v1.0.0)
- Minecraft Beta (Dec.10'10-Sep.15'11) [HERE](https://minecraft.fandom.com/wiki/Java_Edition_Beta_1.0)
    - 1.7.3 (Jul.8'11) [HERE](https://minecraft.fandom.com/wiki/Java_Edition_Beta_1.7.3) 
- Minecraft (Sep.22'11-Present) [HERE](https://minecraft.fandom.com/wiki/Java_Edition_1.0.0)

Roadmap:
- 0.1.0 -> ~MC Pre-Classic feature match with enhancements:
    - Basic Blocks&Textures: Air, Stone, Grass, Dirt, Cobblestone, Wood 
        - Removing with Left Click -> No Animations!
        - Placing with Right Click 
    - Basic physics/player movement + lighting
    - Basic multiplayer
    - Basic generation (flatland and at most hills)
    - Basic ghosts (granular block placement/rotation)
    - Respawn at origin
    - Save Universe
- 0.2.0 -> CREATIVE!
    - New Blocks: Bedrock, Sand, Gravel  
    - Add Water and Lava+Obsidian
    - Generation of lakes
    - Basic flight mechanics
    - Inventory with all blocks + Dock
- 0.3.0 -> SURVIVAL!
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
- 0.7.0 -> ALPHA!
    - Nether/portals + Netherrack/Glowstone...
    - Caves/Dungeons
    - Basic Biomes (Woods, Plains, Beach, Ocean, Mountains)
- 0.8.0 -> EXPLORATION!
    - Maps, Compass, Clock,
    - Doors, Trapdoors
    - Minecart, Rails
    - Boat, fishing
    - BEDS for resetting respawn!
- 0.9.0 -> MCbeta1.7.3
    - Weather and Day/Nighy
    - Villages and NPCs
    - NO HUNGER
- 1.0.0+ -> Modding support and/or carefully chosen optimizations and enhancements (raytracing lighting? Non-competitive enchanting? Non-Competitive End? Big Trees? HUGE trees? More Structures, even in Ocean? Anvils? Horses? Elytra? Compilation of client-only for WebAssembly)
