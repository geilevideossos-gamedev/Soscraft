# Multiplayer M2 tests: the friend plays survival

This collects the owner's two-window steps for milestone M2 of track E (`docs/MULTIPLAYER.md` section 19, E9-E15), one section per change, as each lands. The windows, the builds and the script are the same as in `docs/round8/e-m1-tests.md` Part 1 (build the relay once, build the game with `--profile play`, copy it as `iw4l-mp-m2.exe`, then run `mp-local.ps1`).

The friend window is called **Friend**. The host window uses your own player name, written `<you>` below.

## E9. The chunk map loads chunks around every player

Before E9, chunks loaded only around the host, and a friend more than about 8 chunks away walked into missing terrain. Now the host's world loads and keeps chunks around every player, while the host still draws and lights only its own surroundings.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m2 -Fresh`. Spawn in both.
2. Host window: run `/op Friend`, then `/gamemode creative`.
3. Host window: run `/tp <you> ~600 ~ ~`. Fly down (or up out of the ground) until you stand on the surface. The terrain appears around you as usual.
4. Host window: run `/tp Friend <you>`. The friend now stands next to you, 600 blocks from spawn.
5. Host window: run `/tp <you> ~-600 ~ ~`, back towards spawn. Land on the surface again.
6. Friend window: the friend is now about 600 blocks from the host, far outside the host's view.
   1. The ground under the friend stays. The friend does not fall.
   2. Walk about 100 blocks in one direction, then back. New terrain keeps appearing ahead of the friend, and the friend never walks into a gap or falls through the ground.
   3. Shoot a block next to the friend until it breaks. Remember the spot.

   Screenshot the friend window.
7. Host window: the host's own view is unchanged. Its frame rate is about what it was with the friend standing next to it.
8. Host window: run `/tp <you> Friend`. The host lands next to the friend. The terrain there shows within a second or two, with no wait for it to generate, and the block the friend broke is gone.

   Screenshot the host window from about where the friend stands.
9. Friend window: walk back towards spawn with the host for about 50 blocks. Both keep seeing the same terrain.
10. Close both windows (type `quit` in the script's console).

**Kit:** none needed.

## E10a. The per-player step runs for the host by client number

Nothing new to see: the host's own survival code moved into its own file so the same code can run for every player next. This checks that the host still plays exactly as before. Use the host window alone: start `iw4l-mp-m2.exe` on its own (no friend), with a new world (`IW4L_MINECRAFT_WORLD=mp-m2-solo` in its `.env`) and the kit below, by day. Type `/difficulty normal` first.

1. Walk on grass, then on stone: the footsteps change. Sprint for a while: the hunger bar drops. Eat the cooked beef (hold right click): the crumbs fly and the bar fills.
2. Jump down 6 blocks: the fall hurts, with the land sound. Swim under water until the bubbles run out: you drown slowly. Screenshot the bubbles. **S1**
3. Place cobblestone, then break it with the empty hand (select an empty hotbar slot, hold left click): the cracks grow and the block drops.
4. Place the crafting table and the chest and open each with right click. Put something in the chest, close it and open it again.
5. Hold the shield in the offhand (F swaps hands). Use the skeleton spawn egg on the ground a few blocks away, then hold right click towards it: its arrows clang off the shield.
6. Throw an ender pearl: you land where it lands and lose a little health.
7. Hold the totem in the offhand, then type `/kill`: the totem pops and you live. Screenshot. **S2**
8. Place the bed and sleep in it once (right click) at night (`/time set night`), then type `/kill`. You respawn at the bed, and your items drop where you died.
9. Build a nether portal with the obsidian and light it with the flint and steel. Walk in: the Nether loads. Walk back through: the Overworld loads where you left.
10. Wear the elytra, `/tp ~ ~40 ~`, jump while falling to glide, and use a rocket: it boosts you.

```
IW4L_MINECRAFT_KIT=minecraft:cooked_beef*8,minecraft:cobblestone*32,minecraft:crafting_table,minecraft:chest,minecraft:shield,minecraft:skeleton_spawn_egg,minecraft:ender_pearl*4,minecraft:totem_of_undying,minecraft:white_bed,minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:elytra,minecraft:firework_rocket*8
```

## E10b. Every player has their own hearts, hunger, XP and respawn

The host now runs the friend's survival too: the friend has Minecraft hearts, hunger, air, XP and fall damage of their own, MW2 damage takes Minecraft hearts, a death drops the friend's items where they died, and the friend respawns at their own respawn point. The friend's window shows its own HUD and hotbar, and the hotbar picks the friend's gun. The friend still cannot break, place or use blocks and items, open their inventory or swap hands: that comes with E13 and E14. A friend's own sounds (hurt, drowning, landing) are not played in the friend's window yet.

Put the kit below in the `.env` next to the exe before the first start: both players get it on their first join.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m2 -Fresh`. Spawn in both. Host window: `/time set day`, then `/op Friend`.
2. Friend window: the hotbar, hearts, hunger bar and XP bar show at the bottom, as in the host's window. The hotbar holds the kit and the friend's MW2 guns. Screenshot. **S3**
3. Friend window: scroll the mouse wheel and press the number keys: the selection moves. On a slot with a gun, that gun is raised. On a slot with a block, the gun goes away and the friend cannot shoot. On an empty slot, MW2's bare hands show.
4. Friend window: sprint around for about a minute without stopping. The friend's hunger bar goes down. Alt-Tab to the host window (the host stood still): the host's hunger bar is still full. Screenshot both. **S4**
5. Friend window: `/tp Friend ~ ~8 ~` and fall. The friend loses about 2.5 hearts. Host window: the host's hearts are full.
6. Friend window: swim under water until the bubbles run out. The bubbles show and go one by one, then the friend starts to lose hearts.
7. Host window: `/xp add Friend 10 levels`. Friend window: the XP bar shows level 10. Host window: the host's XP bar is unchanged.
8. Host window: shoot the friend once with the assault rifle. Friend window: the hearts drop.
9. Respawn points:
   1. Friend window: walk about 30 blocks away from the host, then type `/spawnpoint`. A chat line says the spawn point was set.
   2. Host window: place the bed and right click it by day: the chat says the respawn point was set.
   3. Friend window: `/kill`. The friend's items (the kit, not the guns) drop where the friend stood, and the death message shows in both windows' chat. Screenshot the host window. **S5**
   4. Friend window: press fire to respawn. The friend stands at their own spawn point, 30 blocks from the host, with full hearts and hunger. Their XP is gone.
   5. Host window: `/kill`, then respawn. The host stands beside its bed.
10. Host window: walk to the spot of step 9.3: the friend's items lie there. (The friend does not see items or mobs and cannot pick them up until E11 and E12.)
11. Close both windows (type `quit` in the script's console). Start again with `& scripts\mp-local.ps1 -World mp-m2` (no `-Fresh`): the friend's hearts, hunger and XP are as they were when the windows closed.

```
IW4L_MINECRAFT_KIT=minecraft:cooked_beef*8,minecraft:cobblestone*32,minecraft:white_bed,minecraft:torch*16
```

## Merge of track A

Track A's round 8 fixes (vanilla mining speed, the eye of ender, the server tick schedule and `/tick`) are now in the multiplayer build. The host's hand code had moved for E10a, so A's mining and eye of ender were carried into the moved code. This checks that they still work for the host, and that `/tick` works for the friend.

Put the kit below in the `.env` next to the exe before the first start: both players get it on their first join.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m2-a -Fresh`. Spawn in both. Host window: `/time set day`, then `/op Friend`.
2. Host window: place a cobblestone and select an empty hotbar slot. Hold left click on the cobblestone: the cracks grow slowly for about 10 seconds, the block breaks and nothing drops (a bare hand is the wrong tool for stone).
3. Host window: place another cobblestone, select the stone pickaxe and hold left click on it: it breaks in under a second, with the hit sounds and chips while it cracks, and the cobblestone drops. Screenshot while it cracks. **S6**
4. Host window: select the eyes of ender, look up and right click once: an eye flies off towards the stronghold, and the stack drops from 4 to 3. Keep right click held: no second eye is thrown until you let go and click again.
5. Friend window: `/tick freeze`. A chat line says the game is frozen. Host window: the mobs and dropped items stop and the sun stops moving, and the chat shows the same line in grey as Friend's command. Screenshot the host window. **S7**
6. Friend window: `/tick unfreeze`. Everything moves again.
7. Friend window: `/tick sprint 200`. After a moment a chat line reports the finished sprint (ticks per second and milliseconds per tick) in both windows. Screenshot the friend window. **S8**
8. Close both windows (type `quit` in the script's console).

```
IW4L_MINECRAFT_KIT=minecraft:stone_pickaxe,minecraft:ender_eye*4,minecraft:cobblestone*32
```
