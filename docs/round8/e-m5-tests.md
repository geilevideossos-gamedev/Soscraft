# Multiplayer M5 tests: players in different dimensions

This collects the owner's two-window steps for milestone M5 of track E (`docs/MULTIPLAYER.md` section 19, E19-E21), one section per change, as each lands. The windows, the builds and the script are the same as in `docs/round8/e-m1-tests.md` Part 1 (build the relay once, build the game with `--profile play`, copy it as `iw4l-mp-m5.exe`, then run `mp-local.ps1`).

The friend window is called **Friend**. The host window uses your own player name, written `<you>` below.

## E19. Each dimension has its own collision

Every dimension now has its own height band in the MW2 world: the overworld around map height 0, the Nether far below it and the End far above it. Each band keeps its own blocks, so a dimension's blocks stay solid while you are in another one and are still there when you come back, and nothing (a bullet, a grenade, a player) can cross into another band: a solid wall closes each band 128 blocks below the dimension's bottom and 256 blocks above its top. A player who falls onto the bottom wall falls out of the world, as vanilla's void would have done a little lower.

Until E20 travel is still refused while a friend is online, and a friend who joins is placed in the host's dimension, so the travel steps below are done with the friend window closed.

Put the kit below in the `.env` next to the exe before the first start: both players get it on their first join.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m5 -Fresh`. Spawn in both. Host window: `/time set day`, `/op Friend`, `/gamemode survival`. Stand next to each other on flat ground.
2. **Nothing changed in one dimension.** Both windows: walk, jump, mine a few blocks with the pickaxe, place cobblestone and stand on it. Host window: use a cow spawn egg and shoot the cow with your MW2 gun: it takes the hits and dies. Friend window: shoot a block with the MW2 gun: it cracks and breaks. Everything behaves as in M4.
3. **The Nether keeps its own blocks.** Friend window: close it (Alt+F4). Host window: build a nether portal with the 10 obsidian (a frame 4 wide and 5 tall without the corners) and light it with the flint and steel. Before you step in, mine a 1-block hole in the ground next to the portal and remember it.
   1. Walk into the portal. In the Nether: you stand on the netherrack at once, walls stop you, your bullets hit the netherrack and the ground holds a placed cobblestone. Screenshot. **S1**
   2. Walk back through the portal. In the overworld you land on solid ground straight away (no fall through the world), and the 1-block hole you mined is still there. Screenshot the hole. **S2**
   3. Go to the Nether and back a second time: no fall, no stuck in a block, and the hole is still there.
4. **A friend joining in the Nether.** Host window: walk into the portal and stay in the Nether. Restart the friend window: `& scripts\mp-local.ps1 -World mp-m5 -FriendOnly`. Friend window: the friend arrives next to the host in the Nether (the interim rule), stands on the netherrack, can walk and mine it, and is not stuck in a block. Screenshot both. **S3** Close the friend window again (Alt+F4) and go back to the overworld through the portal.
5. **The bottom wall.** Host window: `/tp @s ~ -150 ~`. You fall through the void, take void damage below y -128, and when your feet reach the wall at y -192 you die at once with "<you> fell out of the world" in the chat. There is no landing or fall-damage sound, and the death is told once. Screenshot the death screen. **S4** Respawn.
6. **The End's bottom wall.** Host window: `/execute in minecraft:the_end run tp @s 0 100 300` (over the void, away from the main island). You fall, take void damage below y -64, and die with "<you> fell out of the world" when you reach y -128. Respawn: you come back in the overworld, standing on the ground.
7. **A spectator stays in the band.** Host window: `/gamemode spectator`, then `/tp @s ~ 700 ~`. You end up just under the top wall (y about 574), not above it, and holding jump does not take you higher. Fly down a little and back up: the same invisible ceiling. `/gamemode survival` and `/tp @s ~ 100 ~` afterwards.
8. Close the windows (type `quit` in the script's console).

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:iron_pickaxe,minecraft:cobblestone*32,minecraft:cow_spawn_egg*2,minecraft:cooked_beef*8
```

## E20. Every dimension with a player in it keeps running

Travel is no longer refused while a friend is online, and a friend who joins comes back in the dimension they left. Every dimension that has a player in it now loads its chunks around that player and ticks its mobs, containers and sleepers on the one server clock, whether or not the host is in it. A friend who goes through a portal sees the vanilla loading screen (the purple swirl for the Nether, the dark one for the End) and then the new dimension.

Until E21, a friend in another dimension may still be drawn in the host's window somewhere far off, and the host may be drawn in the friend's window. Ignore that here; E21 hides them.

Put the kit below in the `.env` next to the exe before the first start: both players get it on their first join.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m5 -Fresh`. Spawn in both. Host window: `/time set day`, `/op Friend`, `/gamerule players_sleeping_percentage 100`. Stand next to each other on flat ground.
2. **A friend travels while the host stays.** Friend window: build a nether portal with the 10 obsidian (4 wide, 5 tall, no corners) and light it with the flint and steel. Walk in and wait. The purple loading screen shows, then the Nether: the friend stands on netherrack and can walk, jump and mine it with the pickaxe. Screenshot the loading screen and the Nether. **S5**
3. **The host's dimension keeps running.** Alt-Tab to the host window: the host walks, mines and places blocks as usual, mobs move and the time of day goes on. Alt-Tab back: nothing in the Nether froze while you were away.
4. **The friend's dimension keeps running without the host.** Friend window (in the Nether): use a cow spawn egg next to you. The cow walks around. Kill it with the sword: it drops beef and XP, and the friend gets them. Place cobblestone and mine it again: both work. Screenshot. **S6**
5. **Leaving and joining in another dimension.** Friend window: close it (Alt+F4) while in the Nether. Start it again with `& scripts\mp-local.ps1 -World mp-m5 -FriendOnly`. The friend comes back in the Nether where they left (not next to the host in the overworld), standing on netherrack. Screenshot. **S7**
6. **Back through the portal.** Friend window: walk back into the nether portal. The loading screen shows, then the overworld next to the portal the friend built. The friend is not stuck in a block and does not fall through the ground.
7. **The host travels while the friend stays.** Host window: walk through the same nether portal. The host gets the purple loading screen and the Nether. Friend window (still in the overworld): mobs keep moving, a block placed and mined still works, the time of day goes on. Host window: walk back through the portal.
8. **Sleeping counts the players in the host's level only.** Host window: `/time set 13000` (dusk). Both players stand in the overworld. Host window: place the bed and sleep in it. The message "1/2 players sleeping" shows and the night does not pass while the friend stays awake. Screenshot. **S8** Leave the bed. Friend window: go through the portal into the Nether. Host window: sleep in the bed again: the night now passes, because the friend is in another dimension.
9. **A respawn in another dimension.** Friend window, in the Nether: `/kill`. Respawn: the friend gets the loading screen and comes back at the world spawn in the overworld.
10. **A command into another dimension.** Friend window: `/execute in minecraft:the_nether run tp @s ~ 100 ~`. The friend goes to the Nether with the loading screen. `/execute in minecraft:overworld run tp @s ~ 100 ~` brings them back.
11. Close the windows (type `quit` in the script's console).

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:iron_pickaxe,minecraft:iron_sword,minecraft:cobblestone*32,minecraft:cow_spawn_egg*2,minecraft:white_bed,minecraft:cooked_beef*8
```

## E21. Players in another dimension are not drawn

A player in another dimension is no longer drawn anywhere: not their MW2 body or corpse, not their armour, held item or elytra, and not their name over the head. A friend in another dimension also no longer counts as standing in the way when you place a block, which before this change could refuse a block at the spot whose coordinates matched where the friend stood in their own dimension.

Put the kit below in the `.env` next to the exe before the first start: both players get it on their first join.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m5 -Fresh`. Spawn in both. Host window: `/time set day`, `/op Friend`. Both windows: put on the iron helmet and chestplate and hold a torch. Stand a few blocks apart on flat ground.
2. **Seen in the same dimension.** Host window: the friend's body wears the helmet and chestplate and holds the torch; aim at the friend and their name shows over the head. Friend window: the same for the host. Screenshot both. **S9**
3. **Not seen from another dimension.** Friend window: `/execute in minecraft:the_nether run tp @s ~ ~ ~`. The Nether loads (if the friend lands inside netherrack, mine out with the pickaxe). Host window: look all around, straight down and straight up, and walk to where the friend stood: no body, no floating helmet, chestplate or torch, and no name anywhere. Screenshot looking down where the friend stood. **S10** Friend window: look around the same way: the host is not drawn anywhere either.
4. **Back in the same dimension.** Friend window: `/execute in minecraft:overworld run tp @s ~ ~ ~`, which brings the friend back to where they stood in the overworld (vanilla divides x and z by 8 into the Nether and multiplies them back). Both windows: each sees the other again with the armour, the torch and the name.
5. **A friend in another dimension does not block a block.** Friend window: `/gamemode spectator`. Host window: stand still on flat ground and type `/execute in minecraft:the_end run tp Friend ~ ~ ~`, which puts the friend in the End at your exact coordinates (the End, because the Nether would divide x and z by 8); as a spectator the friend floats there. Step one block back and place cobblestone on the ground where you stood, then a second one on top of it: both are placed. Screenshot. **S11** Friend window: `/execute in minecraft:overworld run tp @s ~ ~3 ~`, which puts the friend just above the two cobblestones, then `/gamemode survival`.
6. **Owner test M5 of the design** (a fresh world: close the windows by typing `quit` in the script's console, then `& scripts\mp-local.ps1 -World mp-m5b -Fresh`; spawn in both):
   1. Host window: `/op Friend`. Friend window: `/execute in minecraft:the_nether run tp @s 0 70 0`.
   2. Friend window: the Nether loads and the friend walks (mine out first if they landed in netherrack). Alt-Tab to the host: the host keeps walking in the overworld, and the friend is not seen anywhere. **S12**
   3. Friend window: mobs move in the Nether (zombified piglins or other Nether mobs nearby; walk a little if none are in sight). Host window: mobs move in the overworld.
   4. Host window: fire your MW2 gun at where the friend was: nothing reaches them. Friend window: the friend's health is untouched.
   5. Friend window: build a nether portal with the 10 obsidian (4 wide, 5 tall, no corners) and light it with the flint and steel, and walk through. The loading screen shows, then the overworld. Host window: `/tp @s Friend` if the friend came out far away. Both windows: the friend is back and seen again, with the armour and the name. **S13**
7. Close the windows (type `quit` in the script's console).

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:iron_pickaxe,minecraft:cobblestone*32,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:torch*16,minecraft:cooked_beef*8
```

## Review fixes

The adversarial review of M5 fixed the defects below. Use the E20 kit (repeated at the end of this section) and a fresh world.

R1. **A friend arriving alone in a dimension gets its chunks.** Start both windows: `& scripts\mp-local.ps1 -World mp-m5r -Fresh`. Spawn in both. Host window: `/op Friend`, `/time set day`. The host stays in the overworld for the whole step.
   1. Friend window: `/execute in minecraft:the_nether run tp @s ~ 100 ~`. The loading screen shows and then the Nether: the friend stands (or falls a little onto netherrack, or mines out if they land inside it) and can walk and mine. Before this fix the friend stayed on the loading screen for ever. Screenshot. **R1a**
   2. Friend window: `/execute in minecraft:overworld run tp @s ~ 100 ~`, which brings the friend back next to the host. Then build a nether portal with the 10 obsidian, light it and walk in, while the host stays away from the portal. The purple loading screen shows, then the Nether, and the friend can walk. Screenshot. **R1b**
   3. Friend window: `/kill` in the Nether. Host window: walk through the friend's portal into the Nether before the friend respawns, so the overworld is left to the friend. Friend window: respawn. The friend gets the loading screen and then the overworld at the world spawn, standing on the ground.

Close the windows (type `quit` in the script's console).

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:iron_pickaxe,minecraft:iron_sword,minecraft:cobblestone*32,minecraft:cow_spawn_egg*2,minecraft:white_bed,minecraft:cooked_beef*8
```
