# Reference: Minecraft's data model

Or: structure of Minecraft's save file

```
/// root folder 
world/

    /// file that contains global information
    /// - time of day
    /// - difficulty (peaceful, easy, normal, hard)
    /// - game type (survival, creative)
    /// - is raining
    level.dat

    /// folder containing a file for each player
    playerdata/

        /// file named as the universally unique id of the player with
        /// - items in inventory
        /// - coordinates of the spawn anchor (bed)
        /// - air
        <uuid>.dat

    /// folder containing the block data
    region/
    
        /// file that holds a grid of 32x32 chunks worth of blocks and block entities
        /// block entities are like chests, signs, furnaces
        /// the chunks have variable size: a chest with 27 written books is heavy
        r.<x>.<z>.dat

    /// folder containing the entities data
    entities/

        /// file that holds all entities within a grid of 32x32 chunks (with 32*x, 32*z offset)
        /// entities are mobs, projectiles, items, dynamic tiles (falling sand), tnt
        r.<x>.<z>.dat
```

yaa man da isser


# Our data model

As we are reimplementing Minecraft we need to hold the same data in various ways.
This is a possible way to organize the data in our implementation.

We need to hold this data in four places:
- Disk: The disk (in a save file)
- Server Ram: The ram of the server's machine (in rust and bevy's abstractions)
- Client Ram: The ram of the client machine
- Graphics Ram: The gpu ram (in shader buffers, meshes and particles)

## Disk

Following quite closely the original save file structure:
- `level` and `players` are saved the same way, we could consider merging all players in a single file
- `regions` and `entities` are saved in 8x8x8 chunks

Minecraft uses a complex binary compressed format with variable width for it's data.
We could roll our own binary format mixed with a plaintext like `ron` for world and players.

## Ram (Server and Client)

- `level` is a global bevy resource
- `players` are bevy entities
    - They are linked to another server entity that has the actual Steve model
- `blocks` are in `struct Universe`
- `block entities` can't be in the current `struct Universe`
    - They can hold dynamically sized data (?Sized), so they need to be held somewhere else
    - Maybe as another field of the `struct Chunk`? Maybe they are just bevy entities?
- `entities` are bevy entities

## Graphics Ram

- `level` isn't needed
- `players` are rendered as `Ghost`s with the voxel renderer
- `blocks` are rendered with the voxel renderer
- `block entities` (which may be animated) are rendered either 
    - with `Ghost` and the voxel renderer
    - with the bevy 3d rasterize renderer
- `entities` are rendered either
    - with `Ghost` and the voxel renderer and a lot of optimizations
    - with the bevy 3d rasterize renderer, which would support them easily

It would be awesome if we could merge the bevy 3d rasterization pipeline and our voxel pipeline.


# Data lifetime

Analyzing when each bit is created, modified and destroyed is actually feasible.
Let's go:

## Disk

- `level`:
    1. Created at the initial world generation
    2. Updated when the world is saved
- `players` files:
    1. Created when a player joins
    2. Updated when the world is saved
- `blocks`, `block entities` and `entities` files:
    1. Created 
        - or at the initial world generation
        - or when the world is saved
    2. Updated when the world is saved

## Server Ram

- `level`:
    1. Created when the player generates a new world
    2. Updated every tick
- `players`:
    1. Created when a player joins
    2. Updated every tick
    3. Deleted when a player leaves
- `blocks`, `block entities`:
    1. Created
        - or by chunk generation
        - or by loading from disk
    2. Updated some ticks, depends when a player edits it, a cellular automata is run, ...
    3. Deleted when the chunk is out of view distance of a player
- `entities`
    1. Created
        - or by chunk generation
        - or by loading from disk
    2. Updated every tick
    3. Deleted when the entity is out of view distance of a player

## Client Ram

The client ram is ideally a replication of the server ram. 
But, doing so naively is out of question, chunks are huge.
Moreover, replication of players and entities must hide the latency.

- `level`:
    1. Created when the server sends it after a player joins a world
    2. Updated when the server sends it
- `players`: 
    1. Created when the server sends it after a player joins
    2. Updated when the server sends it
    3. Deleted when the server tells so

When the server sends it, it strips it of private player info like the inventory, 
leaving only external visible information.

- `blocks`, `block entities`:
    1. Created when the server sends it
    2. Updated when the server sends an update
        - or the server sends the whole chunk with all the data
        - or the server sends just the diff, but that implies that the server has to hold
        each client's chunks to calculate the diff.
    3. Deleted when the chunk is out of view distance of a player.
- `entities`
    1. Created when the server sends it
    2. Updated when the server sends it
    3. Deleted when the server tells so

## Graphics Ram

Sending the data from Ram to the Gpu.

<wip>

### Bevy 0.12 Render World

We introduce an abstraction with Bevy: between Graphics Ram and Normal Ram there is another layer.
This layer helps setting up the Gpu buffers (aka the Graphics Ram) and it copies data from
Normal Ram to Normal Ram. 
It does so by extracting data from the Bevy Main App World 
and copying it to the Bevy Render SubApp World.
