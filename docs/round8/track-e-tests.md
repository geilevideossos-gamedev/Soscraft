# Multiplayer test: everything of track E in one run

This is the owner's test of round 8 track E (`docs/MULTIPLAYER.md`, milestones M1 to M6), in one ordered run with two game windows on one PC, then over the internet with a friend. It replaces the per-milestone tests `docs/round8/e-m1-tests.md` to `e-m6-tests.md` (they stay as the record of each milestone). The host's single-player regression steps are in `docs/round8/release-2-tests.md`.

The friend window plays a player called **Friend**. The host window uses your own player name from `iw4l-artifacts\settings.cfg`, written `<you>` below. "Host:" means the host window, "Friend:" the friend window. Screenshots are numbered **S1** to **S58**. If a step fails, screenshot it, note its number and go on.

## Before you start

1. **The game.** This test uses the release 2 build, `iw4l-survival.exe` in `C:\Games\Mashup\2010-Rust-Rewrite-Mashup`. For a `--profile play` build instead, copy it there as `iw4l-mp-r8.exe` and leave `-Exe iw4l-survival.exe` out of every command below: the script then takes the newest `iw4l-mp-*.exe`.
2. **The relay**, once (the copy in the game folder must be this round's build):

   ```
   & scripts\cargo-slot.ps1 build --release --locked -p iw4l-master
   ```

   Copy `D:\iw4l-target\release\iw4l-master.exe` to the game folder.
3. **The kit.** Put the line below in the game folder's `.env`. Both players get it on their first join to the fresh world (27 stacks). Items that do not fit in 27 stacks come from `/give` lines in the steps. Keep the `.env` free of `IW4L_MASTER_HOST_NAME` and `IW4L_MASTER_JOIN`.

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:iron_pickaxe,minecraft:iron_sword,minecraft:cooked_beef*32,minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:white_bed,minecraft:chest,minecraft:oak_log*4,minecraft:furnace,minecraft:raw_iron*3,minecraft:coal*6,minecraft:experience_bottle*8,minecraft:cow_spawn_egg*4,minecraft:snowball*16,minecraft:ender_pearl*8,minecraft:water_bucket,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:elytra,minecraft:firework_rocket*8,minecraft:totem_of_undying,minecraft:powder_snow_bucket,minecraft:ender_eye*12,minecraft:splash_potion@harming,minecraft:splash_potion@poison,minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}]
```

4. **No builds while testing.** The script waits for both cargo slots and holds them while the windows run, so no build starts beside two games and a relay.

### How two games run from one install

`scripts\mp-local.ps1` starts both games from the one game folder:
- It makes `iw4l-artifacts\mp-local\host\` and `iw4l-artifacts\mp-local\friend\`. Each holds a hard link to the same exe (no copy), links to the shared `iw4l-artifacts\cache` and `iw4l-artifacts\minecraft-26.3`, and its own `.env`, `settings.cfg`, logs and command history, all written from the game folder's own on every run.
- It makes the relay's certificates on its first run (with Git for Windows' OpenSSL) and starts the relay on `127.0.0.1:4433`.
- It starts the host window, waits for its room, then starts the friend window with that room's code. Both are windowed (1280x720 on a 2560-wide screen, else 960x540).
- The host's world is `iw4l-artifacts\mp-local\host\iw4l-artifacts\saves\<world>`. Your main world is never touched.
- Its PowerShell window is the script's console: `h <command>` runs an MW2 console command in the host window, `f <command>` in the friend window, and `quit` closes both. Minecraft chat and commands are typed in the game windows.

To do the same by hand, see `docs/MULTIPLAYER.md` section 17.3.

**Keys:** only the window in front gets your keys, so Alt-Tab between them. **T** opens the Minecraft chat, **/** opens it with a command. Chat lines fade after 10 seconds; press T to see them again.

## Start

In PowerShell in `C:\Dev\mashup-crafting`:

```
& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World mp-r8 -Fresh -HostSound
```

- `-Fresh` deletes the test world `mp-r8` first, so both players get the kit. `-HostSound` keeps the host window's sound on, so you can hear the friend from it.
- The script prints `room <code>`, starts the friend window, then prints `both players are in the room`.

## Part 1. Joining, the world and chat (M1, E23)

1. **The join code.** Host: as soon as the world shows, the chat reads "Friends join with the code [<code>]", the code in green. Open `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\mp-local\host\master.status` in Notepad: its `room=` value is the same code. Screenshot. **S1**
2. **Loading.** Friend: the loading screen stays until the host's chunks have arrived, then class select shows. The chat shows "Friend joined the game" in yellow (press T), and no join code line. Host: the same yellow line.
3. **The world.** Both: pick a class and spawn. The friend stands near the host on the same terrain under the same sky. Friend: walk, jump and drop off a block; the friend never falls through the ground or walks through blocks. Host: the friend's soldier moves. Friend: the host's soldier stands there. Screenshot both from about the same spot. **S2**
4. **Blocks.** Host: shoot a block until it breaks. Friend: it is gone. Friend: shoot another block until it breaks. Host: it is gone too.
5. **Chat.** Friend: T, `hello`, Enter. Host: `<Friend> hello`. Host: `hi back`. Friend: `<you> hi back`. Screenshot both chats. **S3**
6. **Permissions.** Friend: `/time set night`: a red "Unknown or incomplete command" line, nothing changes (a new friend is level 0). Host: `/op Friend`. Friend: the gray, italic "[<you>: Made Friend a server operator]". Friend: `/time set night`: both skies turn to night within a second, and the host sees "[Friend: Set ...]". Friend: `/time set day`.
7. **Whispers.** Host: `/tell Friend psst`. Friend: "<you> whispers to you: psst". Friend: `/msg <you> got it`. Host: "Friend whispers to you: got it".
8. **Teleports.** Friend: walk 10 blocks away, then `/tp Friend <you>`: the friend's soldier jumps to the host. Host: walk 10 blocks away, then `/tp Friend ~ ~ ~5`: the friend now stands 5 blocks from the host.
9. **Answers go to whoever asked.** Friend: `/summon minecraft:cow ~2 ~ ~`. Friend: "Summoned new Cow". Host: "[Friend: Summoned new Cow]". Both windows show the cow.
10. Host: `/difficulty normal`, `/time set day`.

## Part 2. The friend plays survival (M2)

11. **The friend's HUD.** Friend: the hotbar, hearts, hunger and XP bar show, as in the host window. The hotbar holds the friend's MW2 guns. Scroll and press number keys: on a gun the gun is raised, on an empty slot MW2's bare hands show. Press E: the inventory holds the kit. Screenshot. **S4**
12. **Their own hunger.** Friend: sprint for about a minute. The friend's hunger bar goes down; the host's (standing still) stays full.
13. **Their own falls and XP.** Friend: `/tp @s ~ ~8 ~`: about 2.5 hearts lost; the host's hearts stay full. Host: `/xp add Friend 10 levels`: the friend's bar shows level 10, the host's is unchanged.
14. **Placing and digging.** Stand the players 6 blocks apart. Friend: place three cobblestone; each shows in both windows a moment after the click, and the host hears the stone from where the friend builds. Friend: hold left click with the iron pickaxe on one: the crack grows in both windows. Screenshot both half cracked. **S5** (host) **S6** (friend). It breaks with stone particles in both, and the drop flies to the friend. Friend: let go halfway through another block: the crack goes away in both.
15. Host: `/give @a minecraft:oak_door`, `/give @a minecraft:coal_ore 4`, `/give @a minecraft:anvil`.
16. **Bullets for the friend.** Friend: place a coal ore and shoot it with a pistol until it breaks: the bullet cracks show in both windows, the coal drops, and the orbs fill the friend's XP bar.
17. **Using things.** Friend:
    1. place the oak door and right click it: it opens in both windows and the host hears it; again: it closes;
    2. `/effect give @s minecraft:hunger 10 10`, wait until three drumsticks are gone, hold right click with the beef: the bar refills, and the host hears the chewing from the friend;
    3. use the water bucket on the ground: water flows in both windows; use it on the source: the bucket fills again;
    4. throw two snowballs at the host: they fly in both windows;
    5. select the cobblestone, press F: it goes to the offhand; F again: back; Q: one drops in front of the friend, in both windows;
    6. hold left click on a block and press T while holding: the digging stops; close the chat;
    7. `/gamemode creative`, left click blocks: each breaks at once; `/gamemode survival`.
18. **Pickups and orbs.** Host: place a cobblestone, step back 4 blocks. Friend: stand next to it. Host: break it with the pickaxe: it flies to the friend, whose cobblestone stack grows by one, and the host's stays as it was. Host: walk 6 blocks away and throw the 8 bottles o' enchanting at the friend's feet: the friend takes the orbs one by one; the host's XP bar stays as it was. Screenshot the friend window. **S7**
19. **Mobs pick the right player.** Friend: `/summon minecraft:wolf ~5 ~ ~` and shoot it once with a pistol: the wolf runs at the friend, not at the host. Screenshot while it bites. **S8** Kill it. Host: `/time set night` and walk 15 blocks away. Friend: `/summon minecraft:skeleton ~6 ~ ~`: its arrows hurt the friend while the host's hearts stay full. Screenshot as an arrow hits. **S9** Shoot it. Host: `/time set day`.
20. **The friend's own screens.** Friend: E, put two oak logs in the 2x2 grid and shift-click the output: 8 planks. Put 4 planks in the grid and take the crafting table; close. The host's inventory did not change. Friend: place the crafting table, open it, make sticks from two planks. Put one plank in the grid and walk 8 blocks away with the screen open: it closes by itself and the plank is back in the friend's inventory. Screenshot the friend's 2x2 grid with the log in it. **S10**
21. **Furnace.** Friend: place the furnace, raw iron on top, one coal below: the flame and arrow move. While it smelts, shift-click the coal out and back in three times: each click moves it at once. Take the iron: the friend's XP bar rises.
22. **One chest for both.** Host: place the chest, open it and leave it open. Friend: open the same chest: the lid is open in both windows. Friend: shift-click the friend's cobblestone into it. Host: it shows in the host's chest at once; take it. Friend: the slot is empty. Screenshot both. **S11** (host) **S12** (friend) Friend: Escape: the lid stays open (the host still has it). Host: Escape: the lid closes with its sound.
23. **Anvil.** Friend: place the anvil, put the iron sword in, name it `Blade`, take it: the friend's level drops by 1 and the sword is called Blade.
24. **Gear.** Friend: wear the iron helmet and chestplate and hold the sword. Host: the friend's soldier wears them, holds the sword, and the MW2 gun is gone from its hands. Friend: select a gun: the gun is back. Screenshot the host window. **S13** Host: wear your own helmet and chestplate, hold your sword. Friend: the host's soldier shows them. Screenshot. **S14** Friend: F5 shows its own armour; F5 back to first person.
25. **Elytra.** Friend: take off the chestplate, put on the elytra, `/tp @s ~ ~40 ~`, press jump while falling and use a rocket. Host: the wings on the friend's back, spread in the glide pose, folded on landing. Screenshot. **S15** Host: do the same; Friend: sees the host's wings and glide. Both: put the chestplates back on.
26. **Respawn points and death drops.** Friend: walk 30 blocks away and run `/spawnpoint`. Host: place the bed, right click it by day: "Respawn point set". Friend: `/kill`: the friend's items (not the guns) drop where the friend stood, and the death line shows in both chats. Screenshot the host window. **S16** Friend: respawn at the friend's own spawn point, with full hearts and hunger. Host: `/kill`, respawn beside the bed. Both: walk back and pick up your items.
27. Host: `/gamerule keep_inventory true`, so later deaths keep the kit.
28. **Their bodies after death.** Friend: hold the sword. Host: `/kill Friend`: the friend's body lies without the MW2 gun. Friend: respawn, select a gun. Host: `/kill Friend` again: this time the body holds the gun. Screenshot. **S17**
29. **Chunks around every player.** Host: `/gamemode creative`, `/tp <you> ~600 ~ ~`, land on the surface, then `/tp Friend <you>`, then `/tp <you> ~-600 ~ ~` and land. Friend: the ground stays; walk 100 blocks one way and back: terrain keeps appearing ahead and the friend never falls; shoot a block to break it. Host: its frame rate is about as before. Host: `/tp <you> Friend`: the terrain there shows within a second or two, and the broken block is gone. Screenshot. **S18** Host: `/gamemode survival`, `/tp Friend <you>`.
30. **`/tick`.** Friend: `/tick freeze`: the host's mobs, items and sun stop, and the host sees the line in gray. Friend: `/tick unfreeze`. Friend: `/tick sprint 200`: after a moment both windows report the finished sprint. Screenshot the friend window. **S19**

## Part 3. Seeing and hearing the world (M3)

31. Host: `/give @a minecraft:pig_spawn_egg`, `/give @a minecraft:saddle 2`, `/give @a minecraft:tnt 4`, `/give @a minecraft:lava_bucket`, `/give @a minecraft:bone_meal 8`, `/give @a minecraft:item_frame 2`. Put away what you no longer need in a chest when the hotbar is full (a full inventory drops the rest at your feet).
32. **Mobs.** Friend: use the cow spawn egg three times 5 blocks ahead: the cows walk and turn their heads in the same places in both windows, and moo now and then in the friend window. Screenshot both. **S20** Friend: walk 30 blocks away and back: they are still there and moving.
33. **Equipment and professions.** Host: `/summon minecraft:zombie ~3 ~ ~ {equipment:{mainhand:{id:"minecraft:iron_sword",count:1},head:{id:"minecraft:iron_helmet",count:1}}}` and `/summon minecraft:villager ~ ~ ~3 {VillagerData:{type:"minecraft:desert",profession:"minecraft:farmer",level:3}}`. Friend: the zombie holds the sword and wears the helmet; the villager is a desert farmer. Screenshot. **S21** Friend: walk away: the zombie follows with raised arms (host sees it chase). Friend: shoot it: it turns red, falls and puffs away, its hurt and death are heard; its drops lie in both windows; walk over them: the pickup pops, the orbs chime, the friend's XP rises, and the host hears the pop too.
34. **Saddles and riding.** Friend: use the pig egg and right click the pig with a saddle: the saddle shows a moment later. Screenshot. **S22**
35. **Projectiles.** Friend: throw snowballs at the cows and an ender pearl: both windows show the same arcs, and the friend lands where the pearl hit, with its sound.
36. **Fireballs, bullets, potions and levitation.** Host: `/give @a minecraft:shulker_spawn_egg`. Friend: stand 10 blocks away. Host: `/summon minecraft:blaze ~ ~2 ~8`, `/summon minecraft:witch ~8 ~ ~`, and use the shulker egg 6 blocks from the friend. Friend: blaze fireballs fly at the friend, the witch's potions fly and break, and the shulker opens and fires spinning bullets that follow the friend. Screenshot one in the air. **S23** When a bullet hits the friend, the friend floats up for about 10 seconds and falls slowly. Host: screenshot the friend in the air. **S24** Kill the three.
37. **Creeper.** Host: `/summon minecraft:creeper ~6 ~ ~`. Friend: walk up to it: it swells and flashes white; stand back. Both: the crater is the same.
38. **Sounds of each other.** Friend: place and break cobblestone, eat beef, open and close the chest of step 22: each is heard in the friend window and the lid moves in both. Host: walk around the friend, place and break a block: the friend hears the footsteps and blocks. Host: walk 30 blocks away and place a block: the friend does not hear it. Come back.
39. **TNT.** Host: place TNT 6 blocks from the friend, light it, step back. Friend: the fuse hisses, the TNT blinks, the explosion is heard and seen, and the crater is the same as the host's. Screenshot the explosion from the friend window. **S25**
40. **Weather.** Host: `/weather rain`. Friend: the sky darkens, rain falls and is heard within a few seconds. Screenshot. **S26** Host: `/weather clear`.
41. **Level events.** Friend: empty the lava bucket, pour water next to it: it hisses and smokes and the stone appears. Use bone meal on grass: green sparkles with their sound.
42. **Item frames.** Friend: put both frames on a cobblestone wall, right click one with the sword: the sword shows in it in both windows; right click twice more: it turns. Screenshot the host window. **S27** Friend: punch it: the sword pops out; pick it up. Friend: drop a cobblestone with Q: it bobs on the ground in both windows; pick it up.
43. **Range.** Friend: `/tp Friend ~200 ~ ~`, then `/summon minecraft:cow ~3 ~ ~`: the friend sees the cow walk; the host draws nothing out there. Host: place blocks: the friend hears none of it. Friend: `/tp Friend <you>`.
44. Host: `/give @a minecraft:strider_spawn_egg`, `/give @a minecraft:warped_fungus_on_a_stick`, `/give @a minecraft:trapped_chest`, `/give @a minecraft:redstone_lamp`, `/give @a minecraft:gold_block`, `/give @a minecraft:golden_helmet`.
45. **Striders.** Host: use the strider egg 2 blocks in front of the friend. Friend: right click the strider with the second saddle, then with an empty hand: the friend sits on it. Host: screenshot the friend riding. **S28** Friend: hold the fungus on a stick and look around: the strider walks that way; right click: it speeds up. Crouch: the friend gets off.
46. **Mobs loaded with riders.** Host: `/summon minecraft:strider ~4 ~ ~ {Passengers:[{id:"minecraft:zombified_piglin"}]}`. Friend: the strider with the zombified piglin on it shows. Screenshot. **S29**
47. **Trapped chest.** Friend: place the trapped chest with the lamp next to it, open the chest: the lamp lights in both windows. Host: open it too; Friend: close: the lamp stays lit until the host closes as well.
48. **Piglins.** Host: `/summon minecraft:piglin ~4 ~ ~ {IsImmuneToZombification:1b}`. Both: put on the golden helmet: the piglin leaves both alone. Friend: place the gold block and mine it next to the piglin: the piglin attacks the friend, not the host. Screenshot. **S30** Kill it. Both: put the iron helmets back on.

## Part 4. Fighting (M4)

49. Host: `/give @a minecraft:creeper_spawn_egg`, `/give @a minecraft:trident`, `/give @a minecraft:wind_charge 8`. Stand 10 blocks apart.
50. **Gun kills.** Host: shoot the friend until they die. Both: the MW2 kill feed shows it and the chat shows "Friend was slain by <you> using [<gun>]" once. Screenshot both chats. **S31** Friend: respawn and kill the host the same way: "<you> was slain by Friend using [<gun>]" once. A knife kill gives the same line.
51. **pvp off.** Host: `/gamerule pvp false`.
    1. Host: empty a magazine at the friend and throw a grenade at their feet: the friend loses nothing. Friend: shoot the host: nothing either. Host: throw a grenade at your own feet: it still hurts you.
    2. Host: throw the trident at the friend: no damage. Throw a wind charge at the friend: it pushes them but does not hurt them.
    3. Friend: throw the splash potion of harming at the host's feet: the purple burst shows and neither player loses health. Screenshot the host's full hearts. **S32** Friend: throw the poison potion the same way: both are poisoned (green hearts), down to half a heart at most.
    4. Host: `/gamerule pvp true`. Host: throw the host's harming potion at the friend: the friend loses 3 hearts.
52. **The world stays open.** Host: kill the friend 30 times in a row (the friend respawns each time). No intermission and no end-of-round scoreboard ever comes.
53. **A Minecraft death.** Friend: use the creeper egg next to the friend and stand still. Both: "Friend was blown up by Creeper" once. Friend: the death card names the Creeper. Screenshot. **S33**
54. **Death messages off.** Host: `/gamerule show_death_messages false`, shoot the friend dead: the kill feed shows it, neither chat has a line. Host: `/gamerule show_death_messages true`.

## Part 5. Different dimensions (M5)

55. **A friend travels alone.** Friend: build a nether portal with the 10 obsidian (4 wide, 5 tall, no corners), light it, walk in. The purple loading screen shows, then the Nether: the friend stands on netherrack, walks, jumps and mines. Screenshot the loading screen and the Nether. **S34**
56. **Each dimension keeps running.** Host: walk, mine, place; mobs move and the day goes on. The friend is not drawn anywhere, not even a floating helmet or name; look down where the friend stood. Screenshot. **S35** Host: fire at where the friend stood: nothing reaches them.
57. **The Nether runs without the host.** Friend: use the last cow egg: the cow walks; kill it with the sword: its beef and XP go to the friend. Place and mine cobblestone. Nether mobs move. Screenshot. **S36**
58. **Rejoining in the Nether with a typed code.** Friend: close the window (Alt+F4). Host: "Friend left the game". In a **second** PowerShell window in the repo, start the friend with the code from the host's chat, brackets included:

    ```
    & scripts\mp-local.ps1 -Exe iw4l-survival.exe -World mp-r8 -FriendOnly -Join "[<code>]"
    ```

    The script says "starting the friend window (Friend) for room [<code>]". The friend comes back in the Nether where they left. Screenshot. **S37** From now on the friend window belongs to this second PowerShell window: `f <command>` and `quit` for it go there.
59. **Back and forth.** Friend: walk back through the portal: the loading screen, then the overworld next to the portal, not stuck in a block. Host: walk through the portal into the Nether: the friend window keeps running (mobs move, a block placed and mined still works). Host: walk back.
60. **Sleeping counts the level only.** Host: `/time set 13000`. Both in the overworld. Host: lie in the bed: "1/2 players sleeping", and the night does not pass. Screenshot. **S38** Host: leave the bed. Friend: go into the Nether. Host: lie down again: the night passes.
61. **Respawn and commands across dimensions.** Friend, in the Nether: `/kill`, respawn: the loading screen, then the overworld at the friend's spawn point from step 26. Friend: `/execute in minecraft:the_nether run tp @s ~ 100 ~`: the loading screen, then the Nether (mine out if inside netherrack). `/execute in minecraft:overworld run tp @s ~ 100 ~`: back.
62. **Bodies only in the same dimension.** Both wear the iron helmet and chestplate and hold the sword, a few blocks apart. Each sees the other with armour, sword and, when aiming at them, the name. Screenshot both. **S39** Friend: `/execute in minecraft:the_nether run tp @s ~ ~ ~`: neither window shows the other anywhere. Friend: `/execute in minecraft:overworld run tp @s ~ ~ ~`: both see each other again.
63. **A body in another dimension does not block a block.** Friend: `/gamemode spectator`. Host: stand still and run `/execute in minecraft:the_end run tp Friend ~ ~ ~`. Host: step back and place two cobblestones, one on the other, where you stood: both are placed. Screenshot. **S40** Friend: `/execute in minecraft:overworld run tp @s ~ ~3 ~`, `/gamemode survival`.
64. **The walls between dimensions.** Host: `/tp @s ~ -150 ~`: you fall, take void damage, and die at once with "<you> fell out of the world" when your feet reach y -192, told once, with no landing sound. Screenshot the death screen. **S41** Respawn. Host: `/execute in minecraft:the_end run tp @s 0 100 300`: you fall and die the same way at y -128; respawn in the overworld. Host: `/gamemode spectator`, `/tp @s ~ 700 ~`: you stop at about y 574, just under the top wall, and cannot fly higher; `/gamemode survival`, `/tp @s ~ 100 ~`.
65. **A friend's chest in a portal.** Host: go into the Nether through the portal and stay there. Friend: place the friend's chest right beside the portal, put a cobblestone stack in it, step into the portal, open the chest and move a few items out while the swirl builds, without closing it. In the Nether the screen is gone and nothing lies on the netherrack around the friend. Screenshot the friend's inventory. **S42** Friend: back through the portal: the chest is closed and holds what was left in it. Host: come back to the overworld.

## Part 6. The M6 leftovers, the End and the credits (M6)

66. Host: `/give @a minecraft:leather_boots`, `/give @a minecraft:spawner`, `/give @a minecraft:zombie_spawn_egg`, `/give @a minecraft:lectern`.
67. **Freezing.** Friend: dig a hole one block wide and deep, pour the powder snow into it, step in and stay: the friend sinks, the white frost outline grows over about 7 seconds, then the hearts turn light blue and one is lost every 2 seconds. Screenshot. **S43** Friend: jump out and walk: clearly slower for a few seconds while the outline fades, without stutter. Host: do the same in the hole: the same as before.
68. **Leather boots.** Friend: wear the leather boots and walk onto the powder snow: the friend walks on top; holding Shift sinks in. Host: screenshot the friend standing on the snow. **S44**
69. **Snowflakes.** Host: stand in the powder snow and walk back and forth. Friend: snowflakes rise around the host's legs while it moves. Screenshot. **S45** Friend: do the same (F5 helps): flakes around the friend's own body, and the host sees them too.
70. **Totems.** Both: put the totem in the offhand (F). Friend: `/tp @s ~ ~40 ~` and fall: the friend's screen shows the totem rise, spin and drop away, with its sound, and the friend lives. Screenshot during the spin. **S46** Host: the friend is wrapped in green and yellow particles, and no totem shows on the host's screen. Host: do the same fall: the host sees its own animation; the friend sees only the particles. **S47**
71. **Spawner cages.** Host: `/time set night`, place the spawner, `/gamemode creative`, right click it with the zombie egg, `/gamemode survival`. Friend: walk near it: the small zombie turns in the cage with flames and smoke. Screenshot. **S48** Friend: more than 16 blocks away it stops; back near, it turns again; when it spawns zombies the cage spins fast in both windows. Friend: `/gamemode creative`, break it: gone in both; `/gamemode survival`. Host: `/time set day`.
72. **Lecterns.** Friend: place the lectern, right click it with the written book: the book lies on it in both windows. Friend: right click: the book screen opens on "Page one text."; the right arrow turns to page two with its sound. Screenshot. **S49** Host: open the same lectern: page two; turn to page three: the friend's screen turns too. Both: Escape. Friend: open it again; Host: `/tp Friend ~10 ~ ~`: the friend's screen closes by itself. Friend: `/gamemode adventure`, open it: no Take Book button; `/gamemode survival`, open it: Take Book takes the book into the friend's hotbar and the lectern is empty in both windows.
73. **Beds.** Host: `/time set 13000`. Friend: place the friend's bed beside the host's and right click it: the view goes to the pillow, the screen darkens, the chat bar opens with "Leave Bed", both windows show "1/2 players sleeping", and the host sees the friend lying on the bed. Screenshot both. **S50** Friend: Leave Bed: the friend stands up in both windows. Both: lie down: after about 5 seconds the night skips, both wake beside their beds and the sky is bright. Screenshot the friend window. **S51** Host: `/time set 13000`. Friend: lie down alone: the night does not pass; Escape gets the friend up.
74. **An eye of ender breaks for both.** Stand together. Friend: throw eyes of ender one by one until one breaks in the air. Host: the break's crumbs and purple ring show there too. Screenshot. **S52** Throw from the host: the friend sees its breaks too.
75. Host: `/give @a minecraft:bow`, `/give @a minecraft:arrow 64`, `/give @a minecraft:diamond_helmet`, `/give @a minecraft:diamond_chestplate`, `/give @a minecraft:diamond_leggings`, `/give @a minecraft:diamond_boots`, `/give @a minecraft:golden_apple 8`. Both: put on the diamond armour.
76. **Into the End.** Host: `/execute in minecraft:the_end run tp @a 0 70 0`. Both land on the pillar and jump down. Both windows show the pink "Ender Dragon" bar within a second or two, and the End fog thickens. Screenshot both. **S53**
77. **The bar only near the fight.** Friend: `/gamemode spectator`, `/tp @s 300 100 0`: the friend's bar goes, the host's stays. Screenshot the friend window. **S54** Friend: `/tp @s 0 80 0`, `/gamemode survival`: the bar is back.
78. **Crystals, breath and the bar.** Shoot a crystal from one window: both show the explosion. Wait near the middle until the dragon perches and breathes: the friend sees the breath from its mouth, the cloud on the ground and the burst where a fireball lands. Screenshot the breath in the friend window. **S55** Shoot the dragon from both windows: both bars go down together.
79. **The kill.** Kill the dragon: both see its death (beams, explosions), the bars go, the exit portal opens. Screenshot the friend window during the death. **S56**
80. **The friend's credits.** Friend: walk into the exit portal: the end poem with the name "Friend" and the credits roll (no music on the friend's game). Host: keeps playing; the friend's soldier is gone from the End. Screenshot both. **S57** Friend: Escape: the friend stands at their spawn point in the overworld and plays again.
81. **The host's credits.** Host: walk into the exit portal: the credits roll for the host. Friend: keeps playing (walk, break a block). Host: Escape: at your bed.
82. **Only once.** Host: `/execute in minecraft:the_end run tp @a 0 70 0`. Each walks into the exit portal: both go straight home, no credits.
83. **Quitting during the credits.** Friend: close the window (Alt+F4). In the second PowerShell window:

    ```
    & scripts\mp-local.ps1 -Exe iw4l-survival.exe -World mp-r8 -FriendOnly -Friend Bob
    ```

    Host: `/op Bob`, `/execute in minecraft:the_end run tp Bob 0 70 0`. Bob: jump down, walk into the exit portal; during the credits close Bob's window (Alt+F4). Host: "Bob left the game", and the host keeps playing. Run the same command again: Bob comes back and can play at once (no endless loading, no credits again). Close Bob's window.

## Part 7. Leaving, coming back and saving

84. **Rejoin with a code typed in capitals.** In the second PowerShell window: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World mp-r8 -FriendOnly -Join "<CODE IN CAPITALS WITH A SPACE AFTER EVERY 8 CHARACTERS>"`. Friend joins and stands where the friend left, with the inventory and XP as before.
85. **A chest lid when a friend quits.** Friend: open the chest the host placed in step 22 and leave it open. Friend: close the window (Alt+F4). Host: "Friend left the game", and the lid closes with the chest sound. Screenshot. **S58**
86. **Closing.** First PowerShell window: type `quit`: the host window closes and the relay stops. Each script prints a `peak working sets: ...` line when its windows have closed (the second one already did in step 85); send both.
87. **Saved.** Optional check of the game type at the same time: add `IW4L_GAMETYPE=war` to the `.env`. Run `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World mp-r8 -HostSound` (no `-Fresh`). Both come back where they were, with their own inventories, XP and hearts; the blocks of this test are still there; Friend: `/time set noon` still works (the op is kept in `ops.json`). With the `war` line: no team colours over the names and shooting the friend hurts them. Close with `quit` and remove the `IW4L_GAMETYPE` line.

## Part 8. Over the internet with a friend

The friend's PC needs its own MW2. Every step for both of you is in `docs/round8/friend-install.md`: part A (you, once), part B (the friend, once), part C (joining).

1. **You:** part A of `friend-install.md` (relay copied, certificates, password, firewall rule, port forward, `-Bats` and `-FriendKit` with `-Exe iw4l-survival.exe -PublicAddress <your public IP>`). Close every `iw4l-master.exe` still running from the two-window test first.
2. **The friend:** part B (unpack the zip, check `.env`, start once to the menu until the Minecraft files are in, set a player name of their own).
3. **You:** run `Host Minecraft World.bat`. Read the join code from your chat and send it. Screenshot your chat. **I1**
4. **The friend:** run `Join Minecraft World.bat`, type the code. The friend joins your world. Screenshot both PCs. **I2**
5. Together, check in this order and screenshot what fails:
   1. both see each other move, and blocks one breaks are gone for the other;
   2. chat both ways, and after `/op <friend's name>` a command from the friend;
   3. the friend's hotbar, hearts and hunger; a chest you both open; mobs near the friend;
   4. a trip of the friend's own to the Nether and back while you stay;
   5. the friend quits and joins again with the same code: back in the same place with the same items;
   6. you quit through the menu: the friend's game goes back to the menu.
6. Send the `latest.log` of both PCs (`iw4l-artifacts\logs\latest.log`) if anything failed.

## Kit

The same line as at the top, for a fresh world (27 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:iron_pickaxe,minecraft:iron_sword,minecraft:cooked_beef*32,minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:white_bed,minecraft:chest,minecraft:oak_log*4,minecraft:furnace,minecraft:raw_iron*3,minecraft:coal*6,minecraft:experience_bottle*8,minecraft:cow_spawn_egg*4,minecraft:snowball*16,minecraft:ender_pearl*8,minecraft:water_bucket,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:elytra,minecraft:firework_rocket*8,minecraft:totem_of_undying,minecraft:powder_snow_bucket,minecraft:ender_eye*12,minecraft:splash_potion@harming,minecraft:splash_potion@poison,minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}]
```

The rest comes from `/give @a` in steps 15, 31, 36, 44, 49, 66 and 75.
