# From game start to player spawning

A `Level` is everything inside a save:
- time of day
- blocks
- players
- entities (mobs, items, dynamic tiles like sand and other stuff)

The game only has one `Level` opened at a time.

## Level opening and closing

Think of `Level` as a file handle.
When **opened**, the game can read a write block data, player data and so on.
The game is continuously saved and loaded as the simulation requires.
Examples:
- The player walks a while into unexplored territory: 
    - Out of view-distance chunks are saved and unloaded
    - New chunks are created
- An automatic save is detected (like every 5 minutes):
    - All chunks are saved

When **closed**, the game is reverted to the loading phase.
Everything that was in the level is despawned and the game state is cleared.
The simulation stops and waits for a new level to be loaded.

## Loading phase

The game starts without a `Level` being loaded.
This is useful to represent the loading phase (and a potential load menu).
The main advantage to having it is that the systems that expect to have a level loaded will have it.
(for example: the lighting expect blocks to exist, running the system when there is no blocks does
not make sense)
This phase should be very brief and will be made automatic at the start.
I expect this flow as the normal case:
1. Start game
2. Level loading (or creation)
4. Level ready 
3. Spawn player

### Level ready

The level is considered ready if the spawn chunks have been generated or loaded.
The spawn chunks are needed for spawning the player.

## Network interaction

This section answers the question: when is the local player spawned?

I would like to support 3 different modes to launch the game:
- Client (like a terminal connected to the mainframe)
- Server (like the mainframe)
- ClientAndServer (like a mainframe with a screen and keyboard)

### NetworkMode Client

The simulation happens on the server. 
This is only a local reconstruction of the `Level` in the server. 
This means that the client views it's actions after they are sent to the server and sent back by
the server.

There are ways to mitigate this latency by applying the client's actions on the client as well
and checking for incongruences.
This is hard to do correctly in our case because the blocks can change on the server or on the
client at any time.

There is one local player.

### NetworkMode Server

The Server:
1. Gathers the inputs of every client and applies them
2. Runs the simulation for one tick
3. Sends the clients the `Level` changes from the previous tick

There is no local player.

### NetworkMode ClientAndServer

This is a normal server with a local player. 

This is an optimization and an architectural choice.
By having a Client and a Server there technically is no need for this mode:
1. Run the Server as a process (or thread)
2. Run the Client in a different process (or thread) and connect to the Server.
This Client is no different from the other Clients that can connect to the Server.
But there are a network connection, a process (or thread) and lag that are not needed.

Making a special Client that can be run in the same process of the Server adds complexity.
I think this added complexity is fine.

### Spawning the local player 

The spawn is controlled by the network system.

Depending on the NetworkMode:
- Server: no local player
- Client: the local player is spawned when the Server sends the command.
- ClientAndServer: the local player is spawned when the level is ready.
