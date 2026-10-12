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

## E11. The level thread serves every player, not player 0

The level thread now keeps pickups, XP orbs and the XP pickup delay, mob fights (who hurt whom, for wolves), the held item villagers see and trading for each player by number, and random ticks run around every player. The host's game still hands only the host to it; the friends follow with E12. So this checks that the host's pickups, orbs, tame wolf and crops work exactly as before, and that a friend leaving does not disturb the host.

Put the kit below in the `.env` next to the exe before the first start.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m2-e11 -Fresh`. Spawn in both. Host window: `/time set day`, then `/op Friend`.
2. Host window: place a cobblestone and break it with the stone pickaxe. The cobblestone drops and flies into the hotbar with the pickup pop.
3. Host window: look at the ground two blocks ahead and throw the 8 bottles o' enchanting one after the other. The green orbs fly to the host and are taken one at a time, and the XP bar fills. Screenshot while the orbs fly. **S9**
4. Host window: use the wolf spawn egg on the ground, then right click the wolf with bones until hearts show (it is tamed and sits). Right click it with an empty hand: it stands up. Type `/time set night`, then `/summon minecraft:zombie ~6 ~ ~`. Let the zombie hit the host once: the wolf runs at the zombie and bites it. Screenshot while it bites. **S10**
5. Host window: `/time set day`. Place the 4 dirt in a row, till them with the hoe and plant the wheat seeds. Type `/gamerule random_tick_speed 300`: within about 10 seconds the wheat grows to full height. Then `/gamerule random_tick_speed 3`.
6. Friend window: close the window (Escape, then quit). Host window: "Friend left the game" shows in yellow. Break another cobblestone: it is picked up as in step 2, and the wolf still follows the host.
7. Close the host window (type `quit` in the script's console).

```
IW4L_MINECRAFT_KIT=minecraft:stone_pickaxe,minecraft:cobblestone*32,minecraft:experience_bottle*8,minecraft:wolf_spawn_egg,minecraft:bone*8,minecraft:dirt*4,minecraft:wooden_hoe,minecraft:wheat_seeds*4
```

## E12. Mobs, items and XP treat every player alike

The friend now ticks in the level together with the host. Items and XP orbs the friend touches go into the friend's own hotbar and XP bar, mobs see and target the friend, and mob hits, arrows, ender pearls and splash potions land on whoever they hit. A grenade kill counts for the player who threw it. Bullets still count as the host's until E13a, but the loot and orbs go to whoever picks them up. The friend cannot break, place or use items yet (E13c).

Put the kit below in the `.env` next to the exe before the first start: both players get it on their first join.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m2-e12 -Fresh`. Spawn in both. Host window: `/time set day`, then `/op Friend`.
2. Host window: place a cobblestone on the ground and step back 4 blocks from it. Friend window: walk up right next to the cobblestone. Host window: break it with the stone pickaxe from where you stand. Friend window: the cobblestone flies to the friend and the friend's stack goes from 32 to 33. Host window: the host's stack stays at 31.
3. Host window: walk about 6 blocks away from the friend and throw the 8 bottles o' enchanting so they land at the friend's feet. Friend window: the orbs are taken one at a time and the friend's XP bar fills. Host window: the host's XP bar stays empty. Screenshot the friend window. **S11**
4. Friend window: `/summon minecraft:cow ~4 ~ ~`. Select a gun and throw a frag grenade at the cow. When it dies, walk over the spot: the beef or leather goes into the friend's hotbar and the orbs into the friend's XP bar.
5. Host window: `/time set night`, then walk about 15 blocks away from the friend. Friend window: `/summon minecraft:skeleton ~6 ~ ~`. The skeleton shoots at the friend, the nearer player: the friend's hearts drop with each arrow while the host's hearts stay full. Screenshot the friend window as an arrow hits. **S12** Then shoot the skeleton.
6. Friend window: `/summon minecraft:zombie ~4 ~ ~`. It walks up to the friend and hits them, and the friend's hearts drop. Shoot it.
7. Host window: break another cobblestone and walk over it: it goes into the host's hotbar with the pickup pop, as before.
8. Close both windows (type `quit` in the script's console).

```
IW4L_MINECRAFT_KIT=minecraft:stone_pickaxe,minecraft:cobblestone*32,minecraft:experience_bottle*8
```

## E13a. Voxel events say which level and which player they belong to

A bullet now names its shooter. A mob shot by the friend counts the friend as its attacker, and whether a bullet breaks blocks depends on the shooter's game mode, not the host's.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m2-e13a -Fresh`. Spawn in both. Host window: `/time set day`, then `/op Friend`.
2. Friend window: `/summon minecraft:wolf ~5 ~ ~`. Shoot the wolf once with a pistol. The wolf turns angry and runs at the friend, not at the host. Screenshot the friend window while the wolf bites. **S13** Then kill it.
3. Friend window: `/summon minecraft:cow ~4 ~ ~` and kill it with bullets. Orbs drop where it died; walk over them and the friend's XP bar fills.
4. Host window: `/gamemode adventure`. Shoot the ground in front of you: no crack shows and no block breaks.
5. Friend window: shoot the ground in front of the friend until a block breaks. Host window: the block is gone there too. Host window: `/gamemode survival`.
6. Close both windows (type `quit` in the script's console).

## E13b. The action rules live in minecraft_survival

The host's own hand runs through the moved rules. Nothing should look or sound different from before; this is a single-player check in the host window.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m2-e13b -Fresh -HostSound` (the host window keeps its sound). Spawn in both. Use only the host window.
2. Place cobblestone on the ground and on a wall: each one appears where you look, with the stone place sound. Hold the right button: blocks keep coming about five a second.
3. Break one with the stone pickaxe: the cracks grow, the hit sounds and particles show, and the block drops.
4. Place the oak door, then right click it: it opens with the wooden door sound; right click again: it closes.
5. Type `/effect give @s minecraft:hunger 10 10`, wait until the food bar is down by three drumsticks, then hold the right button with cooked beef: crumbs fly, the chewing sound plays and the bar refills.
6. Use the water bucket on the ground: water flows, the bucket is empty. Use it on the water source: it fills again.
7. Throw a few snowballs: the throw sound plays and they fly.
8. Place the white bed and right click it during the day: the message "You can sleep only at night or during thunderstorms" shows above the hotbar. Screenshot. **S14**
9. Close both windows (type `quit` in the script's console).

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*32,minecraft:oak_door*2,minecraft:cooked_beef*8,minecraft:water_bucket,minecraft:white_bed,minecraft:snowball*16,minecraft:stone_pickaxe
```

## E13c. Friends break, place and use blocks and items

The friend now has a Minecraft hand: placing, mining by hand or with bullets, using blocks and items, swapping hands and dropping items all run on the host for the friend. A placed or broken block shows up in the friend window after one round trip. Opening a crafting table or a chest is not for friends yet (E14).

Put the kit below in the `.env` next to the exe before the first start: both players get it on their first join.

1. Start both windows: `& scripts\mp-local.ps1 -World mp-m2-e13c -Fresh -HostSound` (the host window keeps its sound, so it can hear the friend). Spawn in both. Host window: `/time set day`, then `/op Friend`. Stand the two players about 6 blocks apart, facing each other.
2. Friend window: select the cobblestone and place three blocks on the ground in front of the friend, then one on top of another. Each appears in the friend window a moment after the click and in the host window too. Host window: the stone place sound comes from where the friend builds.
3. Friend window: select the stone pickaxe and hold the left button on one of the cobblestones. The crack grows in the friend window and in the host window. Screenshot both windows while it is half cracked. **S15** (host) **S16** (friend). The block breaks with stone particles in both windows and the cobblestone drop flies to the friend.
4. Friend window: let go of the left button halfway through breaking another block: the crack disappears in both windows.
5. Friend window: place the 4 coal ore, then shoot one with a pistol until it breaks. The bullet cracks show in both windows, the coal drops and the orbs go to the friend's XP bar.
6. Friend window: place the oak door and right click it: it opens in both windows, and the host window hears the door. Right click again: it closes.
7. Friend window: type `/effect give @s minecraft:hunger 10 10` and wait until the food bar is down by three drumsticks. Hold the right button with the cooked beef: after about 1.6 seconds the bar refills. Host window: the chewing is heard from the friend.
8. Friend window: use the water bucket on the ground: water flows in both windows and the friend's bucket is empty. Use it on the water source: it fills again.
9. Friend window: throw a few snowballs at the host: they fly in both windows.
10. Friend window: select the cobblestone, press F: it goes to the offhand. Press F again: it comes back. Press Q: one cobblestone drops in front of the friend, in both windows.
11. Friend window: hold the left button on a block, press T to open the chat while still holding: the digging stops. Close the chat.
12. Friend window: `/gamemode creative`. Left click blocks: each breaks at once with no drop. `/gamemode survival`.
13. Close both windows (type `quit` in the script's console).

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*32,minecraft:stone_pickaxe,minecraft:oak_door*2,minecraft:cooked_beef*8,minecraft:water_bucket,minecraft:snowball*16,minecraft:coal_ore*4
```
