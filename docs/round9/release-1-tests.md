# Round 9, release 1: test steps

Build: `iw4l-survival.exe` from `minecraft-survival` at `57302e99`, with tracks A and B of round 9. The round 8 release 2 exe is kept as `iw4l-survival-03e0c9b.exe`. The last test was [../round8/release-2-tests.md](../round8/release-2-tests.md) on 03e0c9be.

## What is in this build

Part A, the fixes from your two-window test:

- **A1.** You see each other in the Nether and deep in caves: the body, armour, held items and the name. Before, the other player was invisible there.
- **A2.** Held items sit in the MW2 body's fists. The main-hand item is in the right fist and the offhand item in the left. They no longer float or overlap, on other players, in F5 and on the inventory's small character.
- **A3.** The ender dragon and its fireballs fly smoothly for the friend, and for the host too.
- **A4.** Mob heads and the carved pumpkin show on the head when worn. The dragon head opens its jaw and the piglin head flaps its ears while you walk. A worn carved pumpkin blurs the first-person view, as in vanilla.
- **A5.** The dragon respawn needed no fix. Your crystals in screenshot 9 were on valid spots. Each side of the exit portal's rim has three straight blocks, and a crystal on any of the three counts, because a crystal is two blocks wide. Only the four diagonal corner blocks don't count. Vanilla 26.3 does exactly the same. New tests pin this rule. Section A5 lets you see it in game.
- **A6.** A friend in creative mode gets the creative inventory, with tabs, search and saved hotbars. Creative drops are limited like vanilla, for the host and friends.
- **A7.** Players pass through each other instead of getting stuck, and they push each other apart like vanilla. Respawns without a bed and new friends land on random spots within 10 blocks of the spawn. Before, everyone landed on the same spot.

Part B, items and the screen:

- **B8.** Written books are read from the hand. A book and quill opens vanilla's edit screen and can be signed into a written book. Friends can do all of it. A written book is named by its title, and its tooltip shows the author and "Original".
- **B9.** Potions, splash and lingering potions and tipped arrows show their vanilla colour everywhere: inventory, hotbar, hands, on the ground, in item frames and on other players. Mobs and players with an effect give off coloured swirls.
- **B10.** A hit gives a faint red flash of half a second. The lasting red screen at low health and MW2's pain vision are gone on Minecraft maps. Show Blood still turns the flash off. MW2 maps are unchanged.

## Setup

- Use a class with an assault rifle, a pistol, Frag and Flash, as in round 8.
- **Single player:** set `IW4L_MINECRAFT_WORLD=<world>` in the `.env` next to the exe, then start `iw4l-survival.exe map minecraft:overworld`.
- **Two windows on this PC:** follow [../round8/multiplayer-quickstart.md](../round8/multiplayer-quickstart.md) by hand, or run the script from PowerShell in `C:\Dev\mashup-crafting`:

  ```
  & scripts\mp-local.ps1 -Exe iw4l-survival.exe -World <world> -Fresh -HostSound
  ```

  - `-Fresh` starts the world new. Leave it out to go on with a world.
  - Both windows use the same exe, so both run this build. A game from an older build is refused at the join with the version message. A friend over the internet needs this build too (a new zip from `-FriendKit`, as in round 8 part I).
  - To start only the friend window again: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World <world> -FriendOnly`. Add `-Friend Bob` to join as a new player.
  - By hand, use `IW4L_MINECRAFT_WORLD=<world>` in the host window, and delete `iw4l-artifacts\saves\<world>` first for a fresh world.
- **Names:** in the steps, `Sos` is the host's player name and `Friend` the friend's. Use your names if they differ.
- **Kits:** put the kit in the game folder's `.env` as one line, replacing the old line. Each part has one kit. A kit fills the backpack of a player new to a world, for the host and the friend alike. In a world you have played before, type `/clear`, then `/kit`.
- Remove `IW4L_MINECRAFT_DIMENSION` and `IW4L_MINECRAFT_SEED` from the `.env`. Keep it free of `IW4L_MASTER_HOST_NAME` and `IW4L_MASTER_JOIN`.
- **In every new world**, the host types `/difficulty normal` and `/gamerule keep_inventory true` first. In two windows the host also types `/op Friend`.
- **Logs:** single player and by hand, `iw4l-artifacts\logs\latest.log`. By hand, `latest.log` is the window started last (the friend); the host's log is the one named by its start time. With the script, each window has its own: `iw4l-artifacts\mp-local\host\iw4l-artifacts\logs\latest.log` and `...\mp-local\friend\...`.
- Only the window in front gets your keys. Alt-Tab between the windows.
- Steps marked **(two windows)** need the friend. The others can be done in the host window alone.

## A. The multiplayer fixes

World: two windows, `-World r9a -Fresh` (by hand: `IW4L_MINECRAFT_WORLD=r9a`, fresh). Start by day on open grass. Don't move the host before step A7-1.

Kit A, for both players and the whole part (27 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_sword+sharpness=5,minecraft:torch*64,minecraft:cobblestone*64,minecraft:totem_of_undying,minecraft:enchanted_book,minecraft:spyglass,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:cooked_beef*32,minecraft:end_crystal*16,minecraft:elytra,minecraft:firework_rocket*64,minecraft:dragon_head,minecraft:piglin_head,minecraft:skeleton_skull,minecraft:wither_skeleton_skull,minecraft:zombie_head,minecraft:creeper_head,minecraft:player_head,minecraft:carved_pumpkin,minecraft:white_bed,minecraft:bow,minecraft:arrow*64
```

The rest (ladders, vines, a saddle, a pig, a boat) comes from `/give` and `/summon` lines in the steps.

The sections go in the order the world needs: the Overworld first (A7, A2, A4), then the Nether and the End (A1, A3, A5), then the creative inventory (A6), because A6 empties the friend's inventory.

### A7. Players pass through each other, push apart and spawn apart

- **A7-1 (two windows).** On the friend's first join, before anyone moves: the friend appears within 10 blocks of the spawn, usually not inside the host. If they do overlap, both can walk away and slide apart. Press F3 in both windows and note both positions. **Screenshot 1** from the host window.
- **A7-2 (two windows).** Friend: `/tp Sos`. The friend lands exactly on the host. Both can walk away at once; nobody is frozen. While both stand still on exactly the same point, nobody is pushed (vanilla needs a tiny gap between the two centres too). As soon as either takes a step, both slide apart. Then host: `/tp Friend`: the same. **Screenshot 2** right after the teleport, both windows.
- **A7-3 (two windows).** Walk into each other. You slide into the other a little and get nudged sideways, as in vanilla: no wall and no stop. Stand still half inside each other: you slide apart within about a quarter of a second. Crouch and jump inside each other: nothing sticks. **Screenshot 3** while you overlap, both windows.
- **A7-4 (two windows).** One player crouches next to the other, then jumps onto the other's head. Standing on the head does not push.
- **A7-5 (two windows).** Build a short cobblestone wall. Push the other player against it: they stop at the wall and don't go into it.
- **A7-6 (two windows). Sneaking at an edge.** Build a 1-wide cobblestone bridge 4 blocks out over a drop of at least 4 blocks. The host sneaks to the end of the bridge and stays at its edge. The friend walks along the bridge into the host's back, sneaking or not. The host is nudged to the edge and stays on the bridge. Then the host stops sneaking: the friend's push slides the host off. **Screenshot 4** of the host at the edge with the friend behind.
- **A7-7 (two windows). Ladders and vines.** Host: `/give @a minecraft:ladder 16` and `/give @a minecraft:vine 16`. Place a column of ladders 6 high on a wall.
  - The host climbs halfway and stops. The friend climbs up behind and into the host: the host stays on the ladder.
  - The friend stands at the foot and the host climbs down into them: the host is not pushed off.
  - The same on a column of vines.

  **Screenshot 5** with both on the ladder.
- **A7-8 (two windows).** One player types `/gamemode spectator` and flies through the other: no pushing either way. Then `/gamemode survival`.
- **A7-9 (two windows). Riding.** Host: `/give @s minecraft:saddle` and `/summon minecraft:pig ~2 ~ ~`. Right-click the pig with the saddle, then right-click it again to ride. The friend walks into the pig and the rider: the rider is not pushed and doesn't push. Get off; keep the pig for A4-10.
- **A7-10 (two windows).** Shoot each other with an MW2 gun: hits still register (the `pvp` rule is on by default).
- **A7-11 (two windows). Respawns without beds.** Both `/kill` and respawn, 3 times. Each respawn lands on a different spot within 10 blocks of the spawn, usually apart from the other player. Note the F3 positions. **Screenshot 6** after one of them.
- **A7-12 (two windows).** Host: `/gamerule respawn_radius 0`. Both `/kill`. Both land on the spawn column, one inside the other. Neither is frozen, and a step by either slides them apart. Host: `/gamerule respawn_radius 10`.
- **A7-13 (two windows). A returning friend.** The friend walks 30 blocks away and closes the window (Alt+F4). Start the friend again (`-FriendOnly`, see Setup). The friend is back exactly where they left.
- **A7-14 (two windows). A new friend.** The host stands still at the spawn. Close the friend window and start a new player: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r9a -FriendOnly -Friend Bob`. Bob appears within 10 blocks of the spawn, usually not inside the host. Close Bob's window and start `Friend` again (`-FriendOnly`).
- **A7-15 (two windows).** Optional, before the beds: both walk 300 blocks away from the spawn, in the same direction, then both `/kill`. Note it if the respawn takes noticeably longer than near the spawn (the spawn's chunks may have to load).
- **A7-16 (two windows). Beds.** Host: `/time set night`. Both place the white bed side by side and right-click their own bed.
  - The host lies down. The friend walks into the host's bed: the host stays in bed, and the friend passes through and is not pushed. **Screenshot 7** from the host window.
  - When both are up, both `/kill`: each respawns at their own bed, as before.

Adventure respawns are at the end of part A (A7-17), because they need another world.

### A2. Held items in the fists

Both stand 4 blocks apart, facing each other. `/time set day`.

- **A2-1. Host, F5 front view.** Diamond sword in the main hand, a torch in the offhand (F), then F5 twice. The sword comes out of the right fist and the torch out of the left. Nothing floats, and they don't touch.
  - Select the pistol, wait, then select the sword again: still in the fists. **Screenshot 8.**
  - Do the same with the assault rifle. **Screenshot 9.**
  - If you have a class with an SMG, do it once more after the SMG.
- **A2-2. Inventory.** Press E. The small character holds the sword in its right fist and the torch in its left, with nothing above the shoulder (your screenshot 2 of round 8). Turn it: the items stay in the hands. **Screenshot 10.**
- **A2-3 (two windows).** In the friend window, look at the host: the same sword and torch in the fists. The host walks, crouches, goes prone and looks up and down: the items stay in the fists and tilt with the chest. **Screenshot 11** from the friend window.
- **A2-4 (two windows).** The friend empties the main hand and holds the totem in the offhand. In the host window, the totem is in the friend's left fist, not low on the right (your screenshot 3). **Screenshot 12** from the host window.
- **A2-5 (two windows).** The friend selects the enchanted book. The friend's inventory character and the host window show it in the right fist (your screenshot 4). **Screenshot 13**, both windows.
- **A2-6.** The host holds a gun with cobblestone in the offhand. The gun is held as before, and the cobblestone sits in the left fist.
- **A2-7.** The host's `latest.log` still has `Minecraft gear bones missing: []`. If an item sits a little inside or outside a fist, send a screenshot: the fist position is one tuning value.

### A4. Worn heads and the pumpkin blur

- **A4-1. Inventory.** E, drag the dragon head into the helmet slot. The small character wears it with the snout forward (your screenshot 7). **Screenshot 14.**
- **A4-2. F5.** The dragon head sits on the soldier's head (your screenshot 8). Walk forward: the lower jaw opens and closes about twice a second, faster when you sprint. Stop: it eases to a halt and stays where it is. **Screenshot 15** with the jaw open.
- **A4-3.** Walk, stop with the jaw open, and press E. The small character's jaw is open by the same amount as the F5 body's.
- **A4-4. The other heads.** In F5, wear the skeleton skull, wither skeleton skull, zombie head, creeper head, player head and piglin head in turn. Each sits on the head, a little bigger than it. With the piglin head, walk: both ears flap, a little out of step with each other. **Screenshot 16** of the piglin head. Take a screenshot of any head where the MW2 helmet or face pokes through.
- **A4-5. Carved pumpkin in F5.** The pumpkin sits on the head with the carved face forward, the size of the MW2 head. **Screenshot 17.** Swap it for the diamond helmet: the helmet is drawn as before, with no head or pumpkin under it.
- **A4-6. The pumpkin blur.** Put the carved pumpkin back on and go to first person. An orange blur covers the screen, dark at the edges, with the carved eye holes clear. It is smooth, not pixelated. **Screenshot 18.** F5: no blur. F5 again (front view): no blur either. Back in first person: the blur is back.
- **A4-7.** In first person with the pumpkin on, hold the spyglass and right-click: only the scope shows, no blur. Let go: the blur is back. **Screenshot 19** while you scope.
- **A4-8.** Move the pumpkin out of the helmet slot: the blur goes. Put on the diamond helmet: no blur.
- **A4-9. Death.** Wear the pumpkin and `/kill`: no blur on the death screen. Respawn, put on the dragon head and stand still in F5: the jaw is at rest, a small gap, like a placed dragon head.
- **A4-10. Riding.** `/give @s minecraft:oak_boat`. Place the boat on land, wear the dragon head, F5, and walk into the boat mid-stride and sit down. The jaw closes to its rest and stays there while you sit, rowing or not. **Screenshot 20** in the boat. Get out and walk: the jaw moves again. Ride the saddled pig from A7-9 (or `/summon minecraft:pig ~2 ~ ~` and `/give @s minecraft:saddle`): the jaw stays at rest there too.
- **A4-11 (two windows).** The host wears the dragon head and the friend the carved pumpkin.
  - Each sees the other's head item on the body, and their own in F5 and in the inventory.
  - The friend has the blur in first person; the host has none.
  - The host walks: the friend sees the jaw move.

  **Screenshot 21**, both windows. Then swap: the host wears the pumpkin and gets the blur, and the friend sees the pumpkin on the host.

### A1. Seeing each other in the Nether and deep in caves

Both put on the diamond helmet and chestplate and hold the sword.

- **A1-1 (two windows). Overworld.** Stand a few blocks apart. Each sees the other's soldier with armour and sword, and the name when aiming at them. **Screenshot 22**, both windows.
- **A1-2 (two windows). Deep in the Overworld.** Host:

  ```
  /fill ~-4 -40 ~-4 ~4 -36 ~4 minecraft:air
  /tp @s ~ -40 ~
  /tp Friend ~3 ~ ~
  ```

  Place torches. Both see each other with armour, sword and name. **Screenshot 23**, both windows. Then go back up (dig up, or both `/kill`: you respawn at your beds).
- **A1-3 (two windows). The Nether.** Build a 4 x 5 obsidian portal, light it and walk in together. Stand 3 to 4 blocks apart. Each sees the other's soldier, dimly lit like the terrain, with armour, sword and name (your screenshot 1). **Screenshot 24**, both windows.
- **A1-4 (two windows).** The friend walks 30 blocks away and back: the host sees them all the way, with the legs moving. The friend switches to a gun: the host sees it in their hands.
- **A1-5 (two windows).** The friend wears the piglin head and walks: in the host window both ears flap. **Screenshot 25** from the host window.
- **A1-6.** Both press F5: your own body shows in the Nether with its armour.
- **A1-7 (two windows).** The friend goes back through the portal. In the Nether the host sees no body, no name and no floating sword. The friend comes back: both see each other again.
- **A1-8 (two windows).** Both go through the portal together and back again. Nobody is frozen at either end, and if you come out inside each other you slide apart.
- **A1-9.** Optional: in the Nether, shoot a wall and throw a frag. If bullet impacts, blood or the explosion are missing there, take a screenshot. That part was not checked.
- **A1-10.** The host's `latest.log` has one line `minecraft scene ents: entity ... has no cell in the stand-in map, linked into every cell`, from when you entered the Nether.
- **A1-11.** Back at the Overworld spawn: no MW2 desert props or crates appear anywhere.

Seeing each other in the End is step A3-1.

### A3. A smooth dragon for the friend

- **A3-1 (two windows).** Host: `/execute in minecraft:the_end run tp @a 0 70 0`. Both put on all four diamond armour pieces. You see each other in the End too.
- **A3-2 (two windows).** Both stand still near the middle and watch the dragon circle the pillars for about 30 seconds, in each window. The body, neck and tail glide without twitching. The wings beat evenly, and the flap sound keeps time with them. The heal beam stays fixed to the dragon. **Screenshot 26** in each window while it circles close by (your screenshot 5).
- **A3-3 (two windows).** The friend watches the circling dragon through the spyglass for 10 seconds: smooth.
- **A3-4 (two windows). Freeze.** Host: `/tick freeze` while the dragon circles. It stops dead in both windows, without shaking. In the friend window the wings stop, no flap sound or growl plays, and the heal beam holds still. **Screenshot 27** from the friend window. Host: `/tick unfreeze`: it flies on smoothly.
- **A3-5 (two windows). Fireballs.** Shoot crystals with the bow and wait until the dragon flies at a player and shoots a fireball. Watch it from the other window too: the purple fireball flies in a smooth line and speeds up, with no back-and-forth. **Screenshot 28** of a fireball in flight, from the friend window.
  - Stand where one lands: the breath cloud appears where it hits, in both windows.
  - Host: `/tick freeze` while a fireball flies: in the friend window it stops dead. `/tick unfreeze`: it flies on.
- **A3-6 (two windows).** Shoot the crystals until the dragon perches. Watch the landing, the breath and the take-off from the friend window: no snapping. Shoot the dragon from both windows: both boss bars go down. Note it if shots near the drawn head miss while it flies fast. If you run out of arrows: `/give @a minecraft:arrow 64`.
- **A3-7 (two windows).** Kill the dragon. It rises smoothly, with the light rays turning evenly, in both windows. **Screenshot 29.**
- **A3-8.** If the dragon still stutters anywhere above, look for `Can't keep up!` in the host's `latest.log` and send the log.

Step A3-9 (the host alone) is at the end of part A.

### A5. The dragon respawn and the crystal spots

Why your spots were fine: from above, the exit portal's rim looks like this (`S` straight rim blocks, `C` corner blocks, `P` portal, `#` the pillar):

```
. . S S S . .
. C P P P C .
S P P P P P S
S P P # P P S
S P P P P P S
. C P P P C .
. . S S S . .
```

The respawn needs one crystal touching the middle of each side. A crystal is two blocks wide, so a crystal on any of the three `S` blocks of a side touches that side's middle. A crystal on a `C` block touches no side. Vanilla 26.3 uses exactly this rule. In your screenshot 9, the crystal that looked wrong was one block off the middle, still on an `S` block, so vanilla would have started the respawn too.

Needs the dead dragon from A3-7.

- **A5-1.** Host: `/gamemode creative`. Fly over the exit portal and look straight down. The rim has three straight blocks on each side and one diagonal block at each corner, as in the picture.
- **A5-2.** The host places one crystal on each of the four corner blocks (`C`). `/gamemode survival`, wait 10 seconds: no beams and no new dragon. **Screenshot 30** from above.
- **A5-3.** Break the four corner crystals from a distance with arrows (they explode). While a corner crystal stands, you can't place a crystal on the rim block next to it, as in vanilla.
- **A5-4 (two windows).** The friend places one crystal on an end block of each straight side (`S`, one block beside the middle), like your screenshot 9. Right after the fourth: the portal goes out, beams rise, the pillars are rebuilt, and after about 25 seconds the new dragon and its boss bar appear, in both windows. **Screenshot 31**, both windows.
- **A5-5.** Only if the corner crystals in A5-2 did start a respawn: press F3 and note the coordinates of each crystal's block and of the portal pillar. **Screenshot 32.**

Then both `/kill`: you respawn at your beds in the Overworld.

### A6. The creative inventory for a friend

Last in part A, because A6-5 clears the friend's inventory.

Host first: `/deop Friend` (the friend is not op for A6-1 to A6-9), then `/gamemode creative Friend`.

- **A6-1 (two windows).** Friend: press E. The creative screen opens with the tabs, and no Operator Utilities tab. **Screenshot 33.**
- **A6-2 (two windows).** Friend: the Search tab. Type `oak`, then `#logs`: only logs and stems show. **Screenshot 34.**
- **A6-3 (two windows).** Friend: take an oak log into hotbar slot 1, shift-click a diamond sword from a tab into the hotbar, close. The host sees the friend holding them. **Screenshot 35** from the host window.
- **A6-4 (two windows).** Friend: Q over cobblestone in a tab, then Ctrl+Q. One cobblestone, then a stack, fly out, in both windows. **Screenshot 36.**
- **A6-5 (two windows).** Friend: the Survival Inventory tab. Put a diamond helmet on the head: the host sees it on the friend's body. Drop a stack onto the trash slot, then shift-click the trash slot: the inventory is cleared. **Screenshot 37.**
- **A6-6 (two windows).** Friend: middle-click an item in a tab, then a stack in the inventory: a full stack is on the cursor each time.
- **A6-7 (two windows). Saved hotbars.** Friend, with the screen closed: fill the hotbar, then C+1 saves it ("Saved hotbar (restore with X+1)"). Clear it with the trash, then X+1 restores it. The Saved Hotbars tab shows the row. It is saved in the friend's own hotbar file, not the host's.
- **A6-8 (two windows).** Host: `/gamemode survival Friend` while the friend's creative screen is open: it turns into the survival inventory. `/gamemode creative Friend`: the next E opens the creative screen again.
- **A6-9 (two windows). Drop limit.** Friend: open the creative screen and press Q over cobblestone in a tab as fast as you can, about 5 times a second, for 20 to 25 seconds. Holding the key doesn't repeat: every drop needs its own press.
  - After about 90 drops (about 18 seconds), further presses drop only one stack about every second, in both windows.
  - The host's `latest.log` has `Player Friend was dropping items too fast in creative mode, ignoring.`

  **Screenshot 38** of the pile. Then the host: `/gamemode creative` and the same in the host window: the same limit, and the same log line with the host's name. A few normal Q presses are not affected.
- **A6-10 (two windows). Operator Utilities, optional.** Host: `/op Friend`. Set `minecraft_operator_items=true` in the game folder's `iw4l-artifacts\settings.cfg` (the script copies it into both windows at every start; by hand, in `iw4l-artifacts\mp-manual\friend.cfg`). Close the friend window and start it again (`-FriendOnly`). The friend's creative screen now has the Operator Utilities tab. Host: `/deop Friend`: the tab is gone on the next open. Host: `/op Friend` again.
- **A6-11.** The host's own creative screen works as before. **Screenshot 39.** Then `/gamemode survival` and `/gamemode survival Friend`.

### End of part A

- **A3-9.** Close the friend window. Host alone, in the Overworld: `/summon minecraft:ender_dragon ~ ~20 ~`. It flies as smoothly as in A3-2.
- Quit the friend first, then the host, through the menu.
- **A7-17. Adventure respawns, optional, alone.** This needs an adventure world. The fork has no `/defaultgamemode`. Create a world in vanilla 26.3 with game mode Adventure and copy its folder into `iw4l-artifacts\saves\`, or set `Data.GameType` to 2 in a copy's `level.dat` with an NBT editor. Start it single player and note the world spawn with F3 after the first join. `/kill` three times without a bed: every respawn is on exactly the same block, the spawn column. `/gamerule respawn_radius 5` changes nothing there. In a survival world the respawns still spread (A7-11).

## B. Books, potion colours and the hit flash

Do all the steps without a mark first, single player in world `IW4L_MINECRAFT_WORLD=r9b` (a new world, Overworld). Start by day on open grass. Then start two windows with `-World r9b-mp -Fresh` (by hand: `IW4L_MINECRAFT_WORLD=r9b-mp`, fresh) and do every step marked **(two windows)**.

Kit B, for both players and the whole part (27 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}],minecraft:writable_book[writable_book_content={pages:["Quill page one","Quill page two"]}],minecraft:writable_book,minecraft:lectern*2,minecraft:comparator,minecraft:redstone*16,minecraft:redstone_lamp,minecraft:book,minecraft:ink_sac,minecraft:feather,minecraft:crafting_table,minecraft:potion@healing,minecraft:potion@poison*2,minecraft:potion@fire_resistance,minecraft:potion@night_vision,minecraft:potion@water,minecraft:splash_potion@harming,minecraft:splash_potion@regeneration*2,minecraft:splash_potion@poison,minecraft:lingering_potion@poison,minecraft:tipped_arrow@poison*16,minecraft:item_frame*4,minecraft:cow_spawn_egg*4,minecraft:golden_apple*16,minecraft:zombie_spawn_egg*4,minecraft:magma_block*4,minecraft:totem_of_undying
```

The other potions come from the creative tabs (B9-2). In the kit there are two books and quills: the first has pages ("Quill page one"), the second is empty.

### B8. Books

- **B8-1. Reading from the hand.** Hold "Test" and right-click at the sky.
  - "Page 1 of 3" opens, with only the forward arrow and one wide Done button (no Take Book).
  - The arrows and PageUp/PageDown turn the pages. Clicking an arrow plays the page sound.
  - Done and Esc close it.
  - Press F to move it to the offhand, select an empty hotbar slot and right-click: it opens again.
  - Holding right-click doesn't reopen it after Done.

  **Screenshot 40.**
- **B8-2. Tooltip.** Open the inventory (E) and hover over "Test". The name is "Test" (not italic), then gray "by Owner" and gray "Original". Select it in the hotbar: the name above the hotbar is "Test". A book and quill shows only "Book and Quill". **Screenshot 41.**
- **B8-3. Writing.** Right-click the empty book and quill.
  - The book opens with a blinking `_` cursor, "Page 1 of 1", only the forward arrow, and the Sign and Done buttons.
  - Type two sentences. Enter (and keypad Enter) starts a new line. Text wraps at the right edge of the page.
  - E, Q and the number keys type letters and don't close the book.

  **Screenshot 42.**
- **B8-4.** Hold a letter key: typing stops when the page has 14 lines. Left/Right, Up/Down (by wrapped line), Home/End, Ctrl+A, Ctrl+C and Ctrl+V work. Clicking in the text moves the cursor.
- **B8-5.** Click the forward arrow: "Page 2 of 2", and the back arrow appears. Type on it, click Done, and right-click the quill again: both pages are kept. Type more and press Esc, then reopen: the new text is gone, as in vanilla.
- **B8-6.** Right-click the other book and quill ("Quill page one"): it opens on its own text, and PageDown/PageUp turn its pages.
- **B8-7. Signing.** In the quill you wrote in, click Sign.
  - "Enter Book Title:", "by <your name>" in dark gray and the warning text show, with "Sign and Close" and "Cancel".
  - "Sign and Close" is grey and does nothing until there is a title. The title stops at 15 characters.
  - Type "Diary" and click Cancel: the edit screen is back. Click Sign again: "Diary" is still there.

  **Screenshot 43.** Click "Sign and Close". The quill becomes a written book in the same slot, and right-clicking it reads both pages. Hover over it: the name is "Diary", then gray "by <your name>" and "Original".
- **B8-8. Crafting.** Craft a book and quill from the book, the ink sac and the feather at the crafting table. Write in it and sign it; Enter in the title box signs too.
- **B8-9. Lecterns.**
  - Place a lectern and right-click it with "Test": the book goes onto the lectern instead of opening in the hand.
  - Put a comparator behind the lectern, then redstone and the lamp after it. Page turns change the signal, and Take Book works.
  - Put the book and quill with "Quill page one" on the second lectern: it reads there.
- **B8-10. Saving.** Save, quit and start `r9b` again: the lectern books and the written books are kept.
- **B8-11. Game modes.** `/gamemode adventure`: right-clicking "Test" reads it. `/gamemode spectator`: right-clicking with a book does nothing. Then `/gamemode survival`.
- **B8-12 (two windows). The friend reads.** The friend repeats B8-1: "Test" opens from the main hand and from the offhand, with page turns.
- **B8-13 (two windows). The friend writes.** The friend writes in the empty book and quill: typing, Enter, the 14-line stop, a second page. Done keeps both pages after reopening, and Esc discards.
- **B8-14 (two windows). The friend signs.** The friend signs that quill: the sign screen shows "by Friend". After "Sign and Close" the quill turns into a written book in the friend's hotbar within a moment, and right-clicking it reads the pages. The host looks at the friend: the quill in the friend's hand turns into a written book. **Screenshot 44**, both windows.
- **B8-15 (two windows).**
  - The friend places a lectern with a book: both players read the same lectern.
  - The friend leaves and rejoins (`-FriendOnly`): the signed book and the other quill's pages are kept.
  - Host: `/gamemode spectator Friend`. The friend right-clicks a book: nothing happens. Host: `/gamemode survival Friend`.

### B9. Potion colours and effect swirls

- **B9-1. Inventory.** Each bottle has its own liquid colour: Healing red, Poison olive green, Fire Resistance orange, Night Vision lime, Water still the old blue. The splash and lingering bottles and the tipped arrows' heads are coloured the same way. Hovering still shows each potion's name. This was your screenshot 10. **Screenshot 45.**
- **B9-2. Creative.** `/gamemode creative`. In the Food & Drinks tab every potion has its own colour: Swiftness cyan, Strong Turtle Master lavender blue, Invisibility almost white, Leaping pale yellow, Weakness dark grey, Awkward the old blue. In the Combat tab every tipped arrow has its own colour. **Screenshot 46.** Then `/gamemode survival`.
- **B9-3.** Potions in the hotbar and one carried on the cursor keep their colours.
- **B9-4. Held.** A held Healing potion is red in first person, in the offhand (F) and in F5. Poison held in first person is olive green. Water stays the old blue. **Screenshot 47** in F5.
- **B9-5. Dropped and framed.** Press Q on a Poison and on Fire Resistance: they lie green and orange on the ground. Place an item frame and put a Poison tipped arrow in it: the head is green. Put the Night Vision bottle in a second frame: it is lime. **Screenshot 48.** Pick up the dropped potions again.
- **B9-6. Thrown.** Throw the Splash Harming bottle on the ground: the bottle and its burst are dusky red. Throw the Lingering Poison: it leaves a green cloud. This was already right; it checks that nothing broke.
- **B9-7. Swirls.**
  - Drink a Poison and press F5: olive green swirls rise around you for the 45 seconds of the effect, a few every second. In first person a few drift past the view too. **Screenshot 49.**
  - Spawn a cow and throw a Splash Regeneration at it: pink swirls rise around the cow. Throw the Splash Poison at a second cow: green swirls.
  - Eat a golden apple: pink (regeneration) and blue (absorption) swirls mix around you.
  - While an effect is on, `/gamemode spectator`: no swirls. `/gamemode survival`: they come back.
- **B9-8 (two windows).** The friend's potions are coloured in the friend's inventory and hotbar, and on the friend's own body in F5.
- **B9-9 (two windows).** The host holds Healing (red) in the main hand and Poison (green) in the offhand: the friend sees both in their colours. Then the other way round. **Screenshot 50** from the friend window.
- **B9-10 (two windows).** The host drops a Poison and a Fire Resistance and frames the Night Vision bottle and a Poison arrow. The friend sees all four in their colours.
- **B9-11 (two windows).** The friend drinks a Poison: the host sees green swirls around the friend, and the friend sees them in F5. Then the host drinks one: the friend sees swirls around the host. A cow splashed with Regeneration swirls pink in both windows. **Screenshot 51**, both windows.

### B10. The hit flash

`/time set midnight`, so the zombies don't burn. Kill them with your gun when a step is done.

- **B10-1.** Options > Advanced Video: Show Blood is on.
- **B10-2. One hit.** Spawn a zombie and let it hit you once: a faint red flash, gone in about half a second, with the view kick and the blinking hearts.
- **B10-3. Low hearts.** Go down to 1 or 2 hearts with the zombie and a magma block. Between hits the screen is completely clear (your screenshot 11 case). **Screenshot 52** at low hearts between hits.
- **B10-4.** Kill the zombie and stand still at 1 or 2 hearts for 10 seconds. The screen stays clear, with no red and no pain vision; only the hearts jitter.
- **B10-5.** Eat golden apples: nothing changes on screen.
- **B10-6.** Turn Show Blood off and step onto a magma block: no flash. Turn it back on.
- **B10-7. The totem.** Hold the totem and survive a lethal fall (`/tp @s ~ ~40 ~`): at most one short flash.
- **B10-8.** `/kill` and respawn: no flash is replayed.
- **B10-9 (two windows).**
  - The friend gets hit by a zombie: only the friend's window flashes.
  - The friend's screen is clear at 1 heart.
  - The friend turns Show Blood off in their window: the friend gets no flash, and the host still does.
- **B10-10. MW2.** Quit and start `iw4l-survival.exe map mp_rust`. Hurt yourself, for example with a frag a few metres away or a fall from a roof: MW2's low-health blood and pain vision are still there. **Screenshot 53.**

## Known limits

These are known gaps, not new bugs:

- **Friends** still can't skate, see their own held item and its swing in first person, hear music, trade, hit other players with Minecraft melee (a sword), see the purple overlay while standing in a portal, or get command suggestions. A friend's window may also miss the powder-snow outline.
- **Players and swords:** a Minecraft sword doesn't hurt other players yet. MW2 guns do.
- **The push** between players has no momentum. It is the same in the air and on ice as on the ground. A player in bed is not pushed and doesn't push others (vanilla's sleeper still pushes). Players take no cramming damage from each other. Riders on a mob don't push walkers.
- **Spawns:** the host's own first spawn in a new world is not spread. A respawn far from the spawn may hitch while the spawn's chunks load (A7-15). There is no world border, so the radius is never clamped.
- **Held items:** the fist position is a first guess and is tuned from your screenshots.
- **Worn heads:** a player head always shows the default skin. On a horse, pig, strider or camel, a worn dragon head's jaw and a piglin head's ears rest; vanilla moves them with the mount's gait. In a boat or minecart they rest, as in vanilla.
- **The dragon:** the drawn dragon trails its hit boxes by about 1.5 to 3 ticks, like every other mob, so a shot at the drawn head can miss while it flies fast. A fireball can fly on for a tick or two after it hits. Under the host's `/tick freeze`, other things in the friend window can still move a little, because the friend doesn't know the host's tick state.
- **MW2 effects in the Nether:** bullet impacts, blood and explosions there were not checked (A1-9).
- **Creative for friends:** the friend's creative screen shows each change one round trip later, as every friend menu does.
- **Books:** no book copying (the crafting recipe is missing). Pages are plain text, without colours, styles or click and hover text. A text selection is a plain blue box. Page and title limits count characters, so emoji count differently from vanilla. A friend carrying a full 100-page book receives 100 to 300 KB on every inventory change.
- **Swirls:** ghasts, blazes, magma cubes and endermites never swirl. Players only swirl for the effects the game simulates, so swiftness, strength, jump boost, invisibility, night vision and water breathing give none. Your own swirls are not hidden while you scope with the spyglass.
- **Colours:** dyed leather armour keeps its default colour.
- **The hit flash** is not vanilla; vanilla has no red screen on a hit. It is kept at your request, and its strength is one tuning value.
- The limits at the end of round 8's release-2-tests.md still apply, except "Friends can't use the creative inventory".

## Screenshot numbering

- Name each screenshot by its number: `14.png`. When a step asks for both windows, name them `3-host.png` and `3-friend.png`.
- When a step fails, take a screenshot even if the step doesn't ask for one. Name it by its step label, e.g. `A7-6.png` or `B8-13-friend.png`. Say in one line what you expected and what you saw.
- Send `latest.log` after any crash. In two windows, send both windows' logs (see Setup for where they are).
