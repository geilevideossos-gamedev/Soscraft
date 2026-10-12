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
