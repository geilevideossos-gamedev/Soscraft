# Round 10, release 1: test steps

Build: `iw4l-survival.exe` from `minecraft-survival` at `6e0e0573`: round 9's release 2 plus track D's last 28 commits (segment d6 and the first part of d5). The release 2 exe is kept as `iw4l-survival-f743218.exe`. Release 2's steps ([../round9/release-2-tests.md](../round9/release-2-tests.md)) are not repeated here.

**Multiplayer is protocol 6** (release 2 was 5). A friend on the release 2 exe is refused at the join with the version message, so the host and every friend need this build. `mp-local.ps1` runs it in both windows. The relay `iw4l-master.exe` is unchanged and already next to the exe. A friend over the internet needs a new zip from `-FriendKit` (see Setup).

**TNT now hurts players and mobs everywhere, in single player too.** Until this build a TNT blast only broke blocks and hurt dropped items. Every Minecraft explosion (TNT, creepers, ghast fireballs, beds and respawn anchors, end crystals) now also pushes players.

If you still run release 2's steps on this exe, two of them change. In C3-10 the TNT now pushes H (H still loses no hearts). In C3-13 the death line names the player who lit the TNT, as in vanilla ("Sos was blown up by Friend", or "by Sos" for your own TNT); only TNT set off by redstone still reads "Sos blew up".

The whole test takes about 4 hours, plus 30 minutes for the skating part K. Steps marked **optional** can be skipped.

## What is in this build

Track D, segment d6, the friend fixes ([track-d-log.md](../round9/track-d-log.md), from "Group friend-hand"):

- A friend sees their own held item in first person: the sword, eating and drinking, the raised shield, the trident pull and the spyglass. A click swings at once.
- A friend sees the purple swirl while standing in a Nether portal, and hears the End, dragon and credits music.
- Minecraft melee hits other players like vanilla: full and half-charged hits, crits, sweeps, sprint and Knockback pushes, shields and axes, the `pvp` game rule, and death lines such as "Friend was slain by Sos using [Excalibur]".
- Friends can skate (both PCs need Skate 3), and everyone sees every skater on a board in the skate pose.
- A strider carrying a player keeps walking under `/tick freeze`.
- TNT hurts mobs and players like vanilla, and Minecraft explosions push players. TNT remembers who lit it: with `pvp` off it only pushes players, and a kill reads "was blown up by <player>".
- Smaller fixes: a spawner that refuses an egg says so in chat, the enchanting table's offers are saved with the player, an MW2 gun hit wakes a sleeping player, and a hoglin or piglin hurt by a ghast's fireball blast turns on the ghast.
- Review fixes: a client in an MW2 match can't put a board down, and a rocket with no shooter reads "went off with a bang".

Track D, the first part of segment d5:

- Keyframe animations, shared by every mob model. Nothing changes on screen yet: the bat moves as before.
- Withers: built from soul sand and three wither skeleton skulls, drawn like vanilla, charging and blowing up, flying after you, shooting black and blue skulls, the blue armour below half health, breaking blocks, Wither II, a nether star and a purple boss bar.
- Wind charges open wooden and copper doors, trapdoors and fence gates, press buttons, flip levers, ring bells and put out candles. Breezes are not in this build yet.

## Setup

- **The exe:** `iw4l-survival.exe` in `C:\Games\Mashup\2010-Rust-Rewrite-Mashup`. Leave the relay `iw4l-master.exe` next to it as it is.
- **Class:** keep release 2's class, an **M4A1 with Red Dot Sight, a USP .45, Frag and Flash**. The M4A1 is used against withers (W) and sleepers (F2).
- **Single player:** set `IW4L_MINECRAFT_WORLD=<world>` in the `.env` next to the exe, then start `iw4l-survival.exe map minecraft:overworld`.
- **Two windows on this PC:** run from PowerShell in `C:\Dev\mashup-crafting`:

  ```
  & scripts\mp-local.ps1 -Exe iw4l-survival.exe -World <world> -Fresh -HostSound
  ```

  - `-Fresh` starts the world new. Leave it out to go on with a world.
  - Leave out `-HostSound` to mute the host window (part F1 does this).
  - To start only the friend window again after the friend quit: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly`.
- **A friend over the internet** needs a new zip: `& scripts\mp-local.ps1 -FriendKit -Exe iw4l-survival.exe -PublicAddress <your public IP>`, then [../round8/friend-install.md](../round8/friend-install.md). The zip has no Skate 3, so that friend can't skate.
- **The `.env` next to the exe:** besides `IW4L_GAMES`, it needs these lines. Replace the world and kit lines for each part:

  ```
  IW4L_SKATE=off
  IW4L_MINECRAFT_WORLD=r10w
  IW4L_MINECRAFT_KIT=<the part's kit line>
  ```

  - Remove `IW4L_MINECRAFT_DIMENSION` and `IW4L_MINECRAFT_SEED`. Keep it free of `IW4L_MASTER_HOST_NAME` and `IW4L_MASTER_JOIN`.
  - `IW4L_SKATE=off` stays until you set up Skate 3 for part K. That setup replaces it with `IW4L_SKATE_ASSETS=...`.
  - In two windows the world comes from `-World`, and both windows read the kit line.
- **Kits:** each part has its own kit line in its own block. A kit fills only the backpack of a player new to the world.
  - **The first join also gives the class loadout,** to the host and to every friend: the M4A1 on hotbar slot 1, the USP .45 on slot 2, 32 frags and 32 flashes in the two grenade slots, and 90 rifle and 36 pistol rounds (three stacks). The rounds go into the backpack, or onto hotbar slots 3 to 5 when the kit fills the backpack.
  - In a world played before, `/clear` and `/kit` give the kit again, but no loadout.
- **Names:** `Sos` is the host's player name and `Friend` the friend's. Use your names if they differ.
- **In every new world**, the host types `/difficulty normal` and `/gamerule keep_inventory true` first. In two windows the host also types `/op Friend`.
- **Keys:** G throws the lethal grenade, Shift+G the tactical one, F swaps hands, E opens the inventory, Q drops, T opens the chat for commands, F5 changes the camera, J puts the skateboard down and picks it up (only with Skating on).
- **Logs:** single player, `iw4l-artifacts\logs\latest.log`. With the script, `iw4l-artifacts\mp-local\host\iw4l-artifacts\logs\latest.log` and `...\mp-local\friend\...`.
- Steps marked **(two windows)** need the friend. Only the window in front gets your keys.

## W. Withers

World `r10w`, single player, a fresh world. Survival, Normal, by day, on open flat ground in the Overworld (plains are best). Put on the diamond armour first. Keep the totem out of the offhand. Allow about 50 minutes.

Kit W (27 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:soul_sand*16,minecraft:soul_soil*4,minecraft:wither_skeleton_skull*12,minecraft:wither_spawn_egg*2,minecraft:spawner,minecraft:cow_spawn_egg*2,minecraft:zombie_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:chicken_spawn_egg*4,minecraft:netherite_sword,minecraft:bow,minecraft:arrow*64,minecraft:trident,minecraft:wind_charge*16,iw4l:rifle_ammo*64,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:milk_bucket*2,minecraft:obsidian*32,minecraft:bedrock*4,minecraft:cobblestone*64
```

The kit fills the backpack, so the loadout's rounds land on hotbar slots 3 to 5.

### Building and the charge

- **W-1. Build it.** Place one soul sand block, three soul sand blocks across on top of it (a T), and a wither skeleton skull on two of the top blocks: nothing happens. Place the third skull and run 15 blocks back at once.
  - The seven blocks break with their particles, and a small wither stands where the bottom block was.
  - It flashes between its blue and grey skin and is wrapped in moving blue armour. Blue sparks fly around it, and smoke rises from its three heads.
  - A purple "Wither" bar appears at the top of the screen, empty.

  **Screenshot 1.**
- **W-2. The charge.** Shoot it with the bow while it charges: the arrows do nothing (no red flash). Over 11 seconds it grows to full size and the bar fills. The flashing slows and stops, and the armour fades halfway through, as it heals past half health. **Screenshot 2** (halfway).
- **W-3.** At the end it blows up with a deep roar, leaving a crater a little bigger than a charged creeper's. The bar is full.

### Flight, heads and skulls

- **W-4.** It flies up and comes after you, staying about five blocks above you, its centre head on you. Hatch a cow and a zombie near it: the side heads turn to you and to the cow, never to the zombie (the zombie burns by day, so look at once). **Screenshot 3.** Use an iron golem egg: the golem goes for the wither.
- **W-5. Skulls.** It shoots black skulls from all three heads, each with a short shriek. A hit hurts and gives Wither II (black hearts) for 10 seconds. **Screenshot 4** (the black hearts). Drink a milk bucket to clear it.
- **W-6.** Stand behind a cobblestone wall: the black skulls blow small holes in it. Now and then (more often while you hide) a slower, light blue skull flies from a side head. Put up an obsidian wall with one bedrock block in it: a blue skull blows through the obsidian, and the bedrock stays. **Screenshot 5.**

### Armour, block breaking and the kill

- **W-7.** Shoot it with the bow while it is above half health: the arrows hurt (red flash) and the bar drops. Then the bar creeps back up by one health a second.
- **W-8. The shield.** Bring it below half health with the netherite sword and the M4A1. The moving blue armour comes back. Arrows, a thrown trident and wind charges now bounce off without hurting it. The sword and the M4A1 still hurt it. **Screenshot 6.**
- **W-9. Block breaking.** Lure it under a cobblestone roof and hit it once. A second later the blocks around it (3 wide, 4 high) break with a crack and drop as items, obsidian too. Bedrock never breaks. **Screenshot 7.**
  - `/gamerule mob_griefing false`: hitting it breaks nothing, and its skulls' blasts leave the blocks.
  - `/gamerule mob_griefing true`.
- **W-10.** Use the chicken eggs near it: its skulls kill them, and it heals 5 for each (a small jump in the bar). The iron golem gets Wither II from the skulls.
- **W-11. The kill.** Kill it with the sword and the M4A1. Its bar empties and goes as it dies. It drops a nether star and 50 experience. **Screenshot 8.**
  - **Optional:** leave the star on the ground for 6 minutes (`/time add` does not age items; just wait). It is still there; a plain item would vanish after 5.

### Two bars and the death line

- **W-12.** `/gamemode creative`. Use a wither egg: a full-size wither that does not charge and has no armour, with a full purple bar. Then `/summon minecraft:wither ~ ~3 ~10 {Health:10f}`: a second wither, already in its blue armour, and a second bar under the first, almost empty. **Screenshot 9** (two bars).
- **W-13.** `/gamemode survival` and kill the weak wither with the sword: its bar goes, and the full one stays.
- **W-14. Far away.** `/tp ~ ~ ~200`: the bar goes. `/tp ~ ~ ~-200`: it is back with the wither.
- **W-15. The death line.** Take the armour off and let the skulls hit you until one kills you. The chat reads "Sos was shot by a skull from Wither". **Screenshot 10.** If the Wither effect kills you between hits, the line is "Sos withered away"; try again. Put the armour back on.
- **W-16. Peaceful.** `/difficulty peaceful`: the wither vanishes with its bar. Build a soul sand T and place the three skulls: no wither is built. Break the blocks and skulls and pick them up. `/difficulty normal`.

### Other builds, saving and the spawner cage

- **W-17.** Build one from soul soil: it is built. Type `/difficulty peaceful` before it blows up, to remove it, then `/difficulty normal`.
- **W-18. Lying flat and saving.** Build one lying on the ground: the T and the skulls all on one layer, a skull last. It is built. Save and quit while it charges, then load: it is still charging, and it still blows up at the end. Then `/difficulty peaceful` and `/difficulty normal`.
- **W-19. The cage.** `/gamemode creative`. Place the spawner and use a wither egg on it: a small wither turns in the cage. **Screenshot 11.** The spawner also spawns full withers, so break it at once after the screenshot. `/difficulty peaceful` to remove any that came out, then `/difficulty normal` and `/gamemode survival`.

## E. TNT and wind charges

World `r10e`, single player, a fresh world. Survival, Normal, by day, on flat ground, no armour. Start each TNT step at full health (eat a golden apple or wait). Allow about 25 minutes.

Kit E (22 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:tnt*16,minecraft:flint_and_steel,minecraft:cow_spawn_egg*8,minecraft:creeper_spawn_egg*4,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:totem_of_undying,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:wind_charge*64,minecraft:oak_door*4,minecraft:copper_door*2,minecraft:iron_door*2,minecraft:oak_trapdoor*4,minecraft:iron_trapdoor*2,minecraft:oak_fence_gate*4,minecraft:stone_button*4,minecraft:oak_button*4,minecraft:lever*4,minecraft:bell,minecraft:candle*8,minecraft:redstone_lamp*2
```

### TNT

- **E-1.** Lay a 9 by 9 floor of cobblestone. Hatch a cow, place TNT one block from it, step 3 blocks away and light the TNT with flint and steel.
  - You lose hearts (about 7 on Normal without armour) and are thrown away from it, a little up.
  - The cow dies and drops beef and experience.

  **Screenshot 12.** If the blast kills you, the line reads "Sos was blown up by Sos", as vanilla says for your own TNT.
- **E-2.** Build a cobblestone wall 3 wide and 3 high. Place TNT 2 blocks in front of it, light it, run behind the wall and stand right against it. The wall breaks, but you lose few hearts or none. **Screenshot 13.**
- **E-3.** Place TNT, throw a few cobblestone right beside it with Q, and light it: the dropped items are blown away, as before. Put a cow beside another TNT and light it: afterwards the beef lies in the crater. **Screenshot 14.**
- **E-4.** Hatch a creeper and let it blow up beside you: you lose hearts and are pushed away too. **Screenshot 15.**
- **E-5.** `/gamemode creative` and fly (double-tap space) next to lit TNT: no push. Land beside another lit TNT: you are pushed and lose no hearts. `/gamemode survival`.
- **E-6. Optional, the totem's grace.** Put the totem in the offhand. Place two TNT with one block of air between them, stand right next to one and light it. Its blast pops the totem and sets off the other TNT, which goes off a moment later. That second blast takes no hearts but pushes you again. **Screenshot 16.**

### Wind charges

Walk 20 blocks away from the craters onto flat ground.

- **E-7.** Set the blocks up:
  - An oak door, a copper door and an iron door in a row on the ground.
  - Near them an oak trapdoor, an iron trapdoor, an oak fence gate and a candle. Light the candle with flint and steel.
  - A stone button and a lever, each on the side of its own cobblestone block, at least two blocks from any door (so their power does not open the doors).
- **E-8.** Stand about 5 blocks away and throw a wind charge (right-click) at the ground between them.
  - The oak and copper doors, the oak trapdoor and the gate swing open with their sounds.
  - The button clicks, the lever flips and the candle goes out.
  - The iron door and the iron trapdoor stay shut.

  **Screenshot 17.**
- **E-9.** Throw another: the doors, the trapdoor, the gate and the lever flip back. A button that is still pressed is left alone.
- **E-10.** Place the bell on a block and throw a wind charge at it: it rings.
- **E-11.** Place a redstone lamp, put an oak button on its side and throw a wind charge at it: the lamp lights for a second and a half. **Screenshot 18.**

## N. Enchanting, bats, striders and hoglins

World `r10n`, single player, a fresh world. Survival, Normal. Start by day in the Overworld. Allow about 35 minutes.

Kit N (24 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:enchanting_table,minecraft:lapis_lazuli*32,minecraft:experience_bottle*32,minecraft:iron_sword,minecraft:iron_sword,minecraft:strider_spawn_egg*4,minecraft:saddle,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:lava_bucket,minecraft:lava_bucket,minecraft:netherrack*64,minecraft:netherrack*64,minecraft:netherrack*64,minecraft:cow_spawn_egg*8,minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:hoglin_spawn_egg*4,minecraft:ghast_spawn_egg*2,minecraft:blaze_spawn_egg*2,minecraft:piglin_spawn_egg*2,minecraft:shield,minecraft:golden_apple*8,minecraft:cooked_beef*16
```

### The enchanting table

- **N-1.** Place the enchanting table and drink experience bottles to level 3 or more. Put an iron sword and lapis into the table and note the three offers. **Screenshot 19.**
- **N-2.** Take the sword out. Save and quit to the menu, load the world and put the sword back: the same three offers. **Screenshot 20.**
- **N-3.** Enchant the sword with any offer, then put the second iron sword in: the offers have changed. Save and quit, load, and put that sword in again: the same new offers. **Screenshot 21.**

### Bats

- **N-4. Optional, nothing should change.** Bats now run on the shared keyframe code and must look exactly as before.
  - Stand under a roof two blocks above your feet (dig into a hillside, or build one with netherrack) and type `/summon minecraft:bat ~1 ~1 ~ {BatFlags:1b}`: the bat hangs upside down from the roof with its wings folded.
  - Step into the open and type `/summon minecraft:bat ~ ~2 ~`: it flutters about, flapping as before.

  **Screenshot 22.**

### Striders under /tick freeze

- **N-5.** Lay netherrack 10 by 10 on flat ground and pour the two lava buckets onto the middle. Hatch two striders in the lava and saddle both. Hatch a cow 5 blocks away.
- **N-6.** Hold the warped fungus on a stick and right-click one strider to mount it. Type `/tick freeze`. Hold W: the strider walks smoothly where you look, while the cow and the other strider stay still. **Screenshot 23** at the start, **screenshot 24** after 5 seconds of walking.
- **N-7.** Crouch to get off: the strider stops at once. Mount it again: it walks. `/tick step 20`: the cow and the other strider move for one second, then stop. `/tick unfreeze`: everything runs again. **Screenshot 25.**

### Hoglins and piglins against fireballs

The hoglins and the piglin also go for you: keep the shield up while you wait.

- **N-8.** Build a Nether portal with the obsidian (4 wide, 5 high, without the corners) and light it. Go through to the Nether. Find an open spot with room above (a big cave, or the shore of a lava lake) and lay a 9 by 9 netherrack platform. `/gamemode creative`, so the mobs leave you alone.
- **N-9.** Use a hoglin egg in the middle of the platform, then `/summon minecraft:ghast ~12 ~5 ~` (a ghast 12 blocks away and a few blocks up). `/gamemode survival` and stand so the hoglin is between you and the ghast. When the ghast shoots, step aside so the fireball lands next to the hoglin. The hurt hoglin turns and charges the ghast, jumping at it from under it. **Screenshot 26.**
- **N-10.** `/gamemode creative`. Use a blaze egg 8 blocks from a new hoglin, then `/gamemode survival` with the hoglin between you and the blaze. When a blaze fireball hits the hoglin, it goes for the blaze. **Screenshot 27.**
- **N-11.** Do N-9 again with a piglin instead of the hoglin: the piglin shoots at the ghast or runs at it. **Screenshot 28.**

## P. Fights between players (two windows)

World: two windows, `-World r10p -Fresh -HostSound`. Host H (`Sos`), friend F. After `/op Friend`, the host types `/gamerule pvp true` (it is on by default). Stand on flat ground. Before each step both have full health: eat a golden apple or wait. A full hit means waiting at least a second since the last swing. Allow about 45 minutes.

Kit P, for both players (23 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword,minecraft:iron_sword,minecraft:iron_sword+knockback=2,minecraft:iron_axe,minecraft:shield,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:cow_spawn_egg*8,minecraft:zombie_spawn_egg*2,minecraft:anvil,minecraft:experience_bottle*32,minecraft:tnt*32,minecraft:flint_and_steel,minecraft:flint_and_steel,minecraft:redstone_block*8,minecraft:end_crystal*4,minecraft:obsidian*8,minecraft:lava_bucket,minecraft:cobblestone*64,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

"The plain iron sword" is the one without Knockback.

### Melee

- **P-1 (two windows).** Both put on the iron armour. F hits H three times with the diamond sword, a full hit each time. H loses about 2 hearts a hit, is pushed back a little and hears the strong-hit sound. **Screenshot 29** (H's window, the hearts).
- **P-2 (two windows). A crit.** F jumps and hits H while falling: the crit sound and a bigger hit. **Screenshot 30.**
- **P-3 (two windows). Knockback.** F sprints into H and hits: H flies further, with the knockback sound. F hits with the Knockback II iron sword: H flies further still. **Screenshot 31.**
- **P-4 (two windows). Shield and axe.** H holds the shield in the offhand and raises it (hold right-click) facing F. F hits with the sword: blocked, no hearts lost. F hits with the iron axe: H's shield drops and stays down for 5 seconds. **Screenshot 32.**
- **P-5 (two windows). The sweep.** H hatches two cows right beside H. F stands still and gives H a full hit with the diamond sword (not sprinting, not falling): the sweep sound plays and both cows are hurt too. **Screenshot 33.** F hits one of the cows the same way while H stands beside it: H is hurt by the sweep.
- **P-6 (two windows).** Swap roles: H hits F with full, falling and sprinting hits. F's window shows the same results. **Screenshot 34** (F's window).
- **P-7 (two windows). Pvp off.** `/gamerule pvp false`. Both hit each other: only the no-damage sound, no hearts lost, no push. **Screenshot 35.** `/gamerule pvp true`.
- **P-8 (two windows). Creative.** `/gamemode creative Friend`. H hits F: the no-damage sound, no hearts lost. `/gamemode survival Friend`.

### Death lines

Both take the armour off for these kills.

- **P-9.** H places the anvil, drinks experience bottles to level 5 or more and renames the plain iron sword "Excalibur" in the anvil.
- **P-10 (two windows).** H kills F with Excalibur. Both chats show "Friend was slain by Sos using [Excalibur]", once. F's kill card names Sos. **Screenshot 36**, both windows.
- **P-11 (two windows).** H kills F with the diamond sword: "Friend was slain by Sos". **Screenshot 37.**
- **P-12 (two windows). Doomed to fall.** H drops Excalibur with Q and F picks it up. Build a cobblestone tower at least 6 blocks high, with room for both on top. F hits H off it with Excalibur: "Sos was doomed to fall by Friend using [Excalibur]". **Screenshot 38.**
- **P-13 (two windows). Lava.** H pours the lava bucket into a hole. F hits H once with the sword, and within 5 seconds H walks into the lava and dies: "Sos tried to swim in lava to escape Friend". **Screenshot 39.**

### TNT and pvp

Both put the iron armour back on.

- **P-14 (two windows).** `/gamerule pvp false`. Both stand 2 blocks from a TNT with a cow beside it. F lights it with flint and steel. Neither loses hearts and both are pushed. The cow dies and drops beef and experience. **Screenshot 40** (H's window).
- **P-15 (two windows).** Lay 4 TNT in a row. Both stand 3 blocks from the row, and F lights the TNT at one end: the whole chain spares both. **Screenshot 41.**
- **P-16 (two windows).** Place a TNT, put a redstone block beside it (so nobody lit it) and step back about 4 blocks: both lose hearts. **Screenshot 42.**
- **P-17 (two windows).** `/gamerule pvp true`. F lights a TNT 2 blocks from both: both lose hearts, F too. **Screenshot 43.**
- **P-18 (two windows). An end crystal.** `/gamerule pvp false`. Place an obsidian block 3 blocks from both and an end crystal on it. F punches the crystal: both are pushed but lose no hearts. **Screenshot 44.** `/gamerule pvp true`.

### TNT death lines

Pvp stays on. Both take the armour off.

- **P-19 (two windows). "Blew up".** H puts a TNT and a redstone block on the hotbar. F brings H down to about 2 hearts with sword hits. Then F hits H once with a fist, and within 5 seconds H places the TNT and the redstone block beside it and stays. Both chats read "Sos blew up", not "Sos was blown up by Friend". **Screenshot 45**, both windows.
- **P-20 (two windows).** H hatches a zombie next to F. After the zombie hits F, H places a TNT and a redstone block beside F, and F stays: "Friend blew up". **Screenshot 46.**
- **P-21 (two windows). "Blown up by".** F lights a TNT next to H, and H stays. Both chats read "Sos was blown up by Friend", and H's kill card names Friend. **Screenshot 47.**
- **P-22 (two windows).** F drinks experience bottles and renames the second flint and steel "Boom" in the anvil. F lights a TNT next to H with it and keeps holding it until the blast: "Sos was blown up by Friend using [Boom]". **Screenshot 48.**
- Quit the friend first, then the host, through the menu.

## F. The friend's side (two windows)

### F1. Hand, swing, portal, striders and music

World: two windows, `-World r10f1 -Fresh`, **without `-HostSound`**, so only the friend window makes sound. Host H (`Sos`), friend F. Allow about 35 minutes.

Kit F1, for both players (22 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword,minecraft:iron_pickaxe,minecraft:shield,minecraft:cooked_beef*32,minecraft:dried_kelp*16,minecraft:trident,minecraft:spyglass,minecraft:potion@swiftness,minecraft:totem_of_undying,minecraft:white_bed,minecraft:cobblestone*64,minecraft:cow_spawn_egg*4,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:strider_spawn_egg*4,minecraft:saddle,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:lava_bucket,minecraft:lava_bucket,minecraft:netherrack*64,minecraft:netherrack*64
```

In the friend window, first move the USP .45 into the backpack. Then put the diamond sword, iron pickaxe, cooked beef, dried kelp, trident, spyglass and potion on the hotbar. One hotbar slot stays empty.

#### The held items

- **F-1.** F selects the diamond sword: it shows in the right hand, as on the host. **Screenshot 49** (F's window). F selects the empty slot: the MW2 hands show.
- **F-2.** In the inventory, F puts the shield into the offhand slot. F holds right-click with the sword selected: the shield rises. **Screenshot 50.** F lets go: it lowers.
- **F-3. Eating.** Host: `/effect give Friend minecraft:hunger 30 40`. When half of F's food bar is gone, host: `/effect clear Friend`.
  - F selects the cooked beef and holds right-click: the beef rises to the mouth and bobs, and after 1.6 seconds the hand comes back down. **Screenshot 51.**
  - F selects the dried kelp and presses the F key (the kelp goes to the offhand, the shield to that slot). F selects the empty slot and holds right-click: the same in half the time.
  - The swiftness potion: the same drinking pose.
- **F-4. The trident.** F holds right-click with the trident: it is raised and pulled back over half a second, and shakes. **Screenshot 52.** F lets go: it is thrown, and the hand is empty.
- **F-5.** F right-clicks with the spyglass: the scope shows and no item is in view. F lets go: the items come back.
- **F-6.** F presses F5: the first-person items are gone. F5 back: they show again. Host: `/time set night`. F places the bed and lies down: no items in view. F gets up again. Host: `/time set day`.
- **F-7.** In the host window, the host's own sword, shield, potion and trident look as before. **Screenshot 53** (host window).

#### The swing

- **F-8.** F selects the diamond sword and left-clicks into the air: the sword swings at once, with no wait for the host. **Screenshot 54.** F clicks fast: each click swings, but a click early in a swing does not restart it.
- **F-9.** F selects the empty slot and left-clicks: the MW2 hands swing. **Screenshot 55.**
- **F-10.** F hatches a cow and hits it with the sword: one swing, and the cow is hurt.
- **F-11.** F places cobblestone: the hand swings once. **Screenshot 56.** F holds left-click on the cobblestone with the iron pickaxe: the hand keeps swinging while it digs.
- **F-12.** Host: `/gamemode creative Friend`. F left-clicks into the air twice fast: both clicks swing. Host: `/gamemode survival Friend`.
- **F-13.** The host's own swings look as before. **Screenshot 57** (host window).

#### The portal

- **F-14.** F builds a 4 by 5 obsidian frame and lights it with flint and steel.
- **F-15.** F walks into the portal and presses E while inside: the inventory closes again at once. The purple swirl fades in with the trigger sound, and after 4 seconds F goes to the Nether. **Screenshot 58** (F's window, the swirl).
- **F-16.** In the Nether, F walks into the portal and steps out at once: the swirl fades out within a second. **Screenshot 59.**
- **F-17.** F goes back to the Overworld through the portal and steps out. F steps into the portal once more: the swirl starts again from nothing. F steps out.
- **F-18.** The host steps into the same portal: the host's swirl looks as before. **Screenshot 60** (host window). The host steps out before travelling.

#### A strider under /tick freeze

- **F-19.** The host lays netherrack 10 by 10 and pours the two lava buckets onto the middle. The host hatches two striders in the lava and saddles both, and hatches a cow 5 blocks away.
- **F-20 (two windows).** F holds its own warped fungus on a stick and mounts one strider. The host stays off the other one. Host: `/tick freeze`. F holds W: both windows show F's strider walking smoothly, while the empty strider and the cow stay frozen. **Screenshot 61**, both windows. Host: `/tick unfreeze`.

#### The music

Listen to the friend window only (the host window is muted).

- **F-21.** Host: `/execute in minecraft:the_end run tp @a 0 70 0`. When the dragon's bar shows at the top of F's screen, F hears the dragon music at once. **Screenshot 62** (F's window with the bar).
- **F-22.** Host: `/kill Friend`. F respawns in the Overworld: the music stops at once, and no new music starts there. **Screenshot 63.**
- **F-23.** Host: `/execute in minecraft:the_end run tp Friend 0 70 0`. Once the music plays, F quits to the main menu: the music stops with it. Host: `/kill` to get back to the Overworld, then quit through the menu.

### F2. Spawners, enchanting, sleep, withers and wind charges

World: two windows, `-World r10f2 -Fresh -HostSound`. Host H (`Sos`), friend F, both in survival. Allow about 40 minutes.

Kit F2, for both players (24 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:spawner*2,minecraft:pig_spawn_egg*4,minecraft:zombie_spawn_egg*4,minecraft:enchanting_table,minecraft:lapis_lazuli*32,minecraft:experience_bottle*32,minecraft:iron_sword,minecraft:white_bed,minecraft:soul_sand*8,minecraft:wither_skeleton_skull*6,minecraft:netherite_sword,minecraft:bow,minecraft:arrow*64,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:milk_bucket,minecraft:totem_of_undying,minecraft:wind_charge*16,minecraft:oak_door*2,minecraft:cobblestone*64
```

#### Spawners

- **F-24 (two windows).** Both stand at the same spot. Host: `/gamerule spawner_blocks_work false`. H places a spawner and right-clicks it with the pig egg. H's chat shows "Spawner blocks are not enabled" and the egg count stays the same. F's chat shows nothing. **Screenshot 64**, both windows.
- **F-25 (two windows).** F right-clicks the same spawner with the zombie egg: only F's chat shows the line, and F keeps the egg. **Screenshot 65**, both windows.
- **F-26.** Host: `/gamerule spawner_blocks_work true`. The pig egg on the spawner now puts a pig in the cage, the egg is spent and no line appears. **Screenshot 66.**

#### The enchanting table

- **F-27 (two windows).** H places the enchanting table. Both drink experience bottles to level 3 or more. H puts the iron sword and lapis in and notes the three offers, then takes them out. F does the same and notes F's own offers. **Screenshot 67**, both windows.
- **F-28 (two windows).** F quits to the main menu. Start the friend again: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly`. F puts the sword and lapis back: the same offers as before. H's offers have not changed either. **Screenshot 68**, both windows.

#### A gun hit wakes a sleeper

- **F-29 (two windows).** Host: `/time set night` and `/gamerule pvp true`. H and F each place a bed, and H lies down. F shoots H once with the M4A1: H gets up at once beside the bed, without the slow fade. **Screenshot 69**, both windows.
- **F-30 (two windows).** F lies down and H shoots F once: F gets up at once. **Screenshot 70**, both windows.
- **F-31 (two windows).** Both lie down again and nobody shoots: the night is skipped as before. **Screenshot 71.**

#### Withers

Both put on the diamond armour. Use open flat ground.

- **F-32 (two windows).** H builds a wither (four soul sand in a T, three skulls) about 20 blocks from F and runs back. F sees it grow, flash, wear the armour, give off sparks and smoke, and blow up. Both see the purple bar fill. After the blast, F shoots it with the M4A1: a red flash, and the bar drops in both windows. **Screenshot 72** (F's window).
- **F-33 (two windows).** F sees the side heads turn and the black and blue skulls fly and turn. A skull hit gives F Wither II (black hearts; drink the milk). H lures the wither under a cobblestone roof and hits it: F sees the blocks break. **Screenshot 73** (F's window).
- **F-34 (two windows).** Both bring it below half health with the netherite swords and the M4A1s. The blue armour comes on in both windows, and F's arrows bounce off. **Screenshot 74** (F's window).
- **F-35 (two windows). The bar's reach.**
  - Host: `/tp Friend ~ ~ ~200`: F's bar goes.
  - Host: `/execute in minecraft:the_nether run tp Friend 0 128 0` (the Nether roof): F has no bar there.
  - Host: `/execute in minecraft:overworld run tp Friend ~ ~ ~`: F lands at H and sees the bar again.

  **Screenshot 75** (F's window).
- **F-36 (two windows).** F builds a second wither. H sees it grow, flash and blow up, and both windows show two bars. **Screenshot 76** (H's window). Host: `/difficulty peaceful` (both withers vanish), then `/difficulty normal`.

#### Wind charges

- **F-37 (two windows).** H places an oak door and throws a wind charge at the ground in front of it: F sees the door open and hears it. F throws one: H sees and hears it close. **Screenshot 77.**
- Quit the friend first, then the host, through the menu.

## K. Skating with a friend (two windows, optional)

Only with an extracted Skate 3 (Xbox 360) `default.xex` with its `data` folder beside it; an ISO does not work. Skip K without it. Allow about 30 minutes.

Set Skate 3 up once:

1. Start the game in single player (any world). Options > Controls > Minecraft > Skating: Enabled. Quit.
2. Start it again: it asks "Do you have Skate 3?". Answer Yes, pick `default.xex` and wait for the converter. The `.env` now has `IW4L_SKATE_ASSETS=...` and no `IW4L_SKATE=off`.
3. `mp-local.ps1` copies the Skating switch and the Skate 3 path into both windows. In both windows check that Options > Controls > Minecraft shows Skating: Enabled.

World: two windows, `-World r10k -Fresh -HostSound`. Host H (`Sos`), friend F.

Kit K, for both players (7 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:oak_stairs*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:hay_block*16,minecraft:water_bucket,minecraft:diamond_sword,minecraft:cooked_beef*32
```

- **K-1 (two windows).** F stands on flat ground, holds the diamond sword and presses J: the board goes down and the skate camera takes over. In the host window F's soldier stands on a board in the skate pose (knees bent, arms out), with no sword in its hands. It leans and turns with the board without stutter. **Screenshot 78**, both windows (F in third person with F5, for comparison).
- **K-2 (two windows).** F does an ollie and a kickflip next to H: H sees the jump and the flip of the board. **Screenshot 79** (host window).
- **K-3 (two windows).** F builds a 5-step staircase of oak stairs on cobblestone columns and skates up and down it. H sees F go up and down smoothly, without jumps back. **Screenshot 80** (host window).
- **K-4.** F builds a cobblestone tower 4 blocks high with a 6-block runway on top and rolls off the edge without an ollie: half a heart is lost on landing. **Screenshot 81.** With a hay bale set into the ground at the landing spot: nothing is lost.
- **K-5.** F digs a pool 3 by 3 and 2 deep. Host: `/gamemode creative Friend`. F fills the pool with the water bucket. Host: `/gamemode survival Friend`. F rides in: the board is picked up when the eyes go under, and F swims. **Screenshot 82.**
- **K-6.** F, on the board, types `/tp ~ ~ ~20`: F stands at the new place with the board picked up. **Screenshot 83.** J puts it down again there.
- **K-7.** F is on the board. Host: `/kill Friend`. After the respawn, J puts the board down again.
- **K-8.** F, on the board, presses E: the inventory opens and the skater coasts. H still sees F moving.
- **K-9.** F presses J again: in the host window the board is gone at once, and F's soldier stands in its idle pose.
- **K-10 (two windows).** H presses J and rolls past F. In F's window H rides a board in the skate pose, with no item in its hands. **Screenshot 84** (F's window).
- **K-11 (two windows).** Both skate side by side: each window shows the other on a board. **Screenshot 85**, both windows.
- **K-12 (two windows).** H stays on the board in the Overworld. Host: `/execute in minecraft:the_nether run tp Friend 0 128 0` (the Nether roof): F sees no skater. Host: `/execute in minecraft:overworld run tp Friend ~ ~ ~`: F lands at H and sees H on the board again.
- **K-13. Optional, a friend without Skate 3.** A friend over the internet with the zip (no Skate 3) sees the skating host as a standing soldier moving along, with no board, and the game does not crash.
- **K-14. Optional, an MW2 match.** Quit both windows. Start an MW2 match in two windows: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -Map iw4:mp_rust`. In the friend window, press J. No board appears, the friend keeps walking normally, and the host sees the friend where the friend sees itself. **Screenshot 86**, both windows.

## Known limits

These are known gaps, not new bugs:

- **Explosions:** MW2 blasts (grenades, rockets, C4) still push no player. Blast Protection does not soften a push. TNT set off by a creeper's blast does not name the creeper. An end crystal blown up by a player's TNT does not name that player. A ghast fireball that a player sent back still hurts players on a direct hit with `pvp` off. Primed TNT is not saved.
- **Melee between players:** the attacker is not slowed after a sprint knockback hit. Fire Aspect and other after-hit enchantments don't work on players. A sweep round a mob also reaches the players beside it when the mob itself took no damage. Other players see no Minecraft arm swing on a body (the MW2 body keeps its own animations). A Minecraft melee or TNT kill gives no MW2 kill credit.
- **The host's hand:** the host's own first-person items still show with the UI off (`ui_draw 0`). A friend's hide, as in vanilla.
- **Skating:** a friend needs Skate 3 converted on their own PC. The friend zip has none, so a friend over the internet can't skate and sees skaters as standing soldiers without a board. Other players' boards keep their trucks and wheels stiff on the deck. After a stall of more than a second on the host, other skaters show as standing soldiers until the next update. Each skater costs every other player in the same dimension about 23 KB/s.
- **`/tick freeze`:** a player on foot does not push a ridden strider or horse. Respawn anchor and end crystal blasts wait for the next normal tick. The friend window doesn't know the host's tick state, so other things there can still move a little.
- **Withers:** the sky does not darken under a wither's bar. Skulls and fireballs leave no smoke trail. A skull loaded from a save has no owner and hurts as magic for 5. A skull that kills a player does not heal its wither. Withers fly straight at their goal and stop at a wall until they break it or give up. A dispenser does not build a wither. The bar is always named "Wither" (no name tags). The blue armour is drawn see-through instead of glowing, so it looks a little darker than vanilla's.
- **Wind charges:** a candle put out by a burst gives no smoke. A player flying in creative is still pushed by a burst. Sculk sensors don't hear doors and buttons yet. Breezes are not in this build.
- **Keyframes:** no mob reads its animations from the game's files yet. The bat runs on the new code with its old tables.
- **Guns:** MW2 drops no gun model on death; only the Minecraft items drop. Killstreak items vanish on death and can't go into chests or the offhand. Two copies of one gun switch faster than a reload (accepted). Guns don't wear armour, and gun damage to mobs is unchanged.
- **Crafting:** the attach, akimbo and split recipes are not in the recipe book (the tooltips say how). The grindstone loses the attachments and the rounds above the bare clip. The book has no furnace version, no keyboard navigation and no bounce for new recipes. A crafted suspicious stew carries its effect, but eating it applies none yet.
- **Grenades:** only the slots throw; a grenade held in the hand does nothing. Placed claymores and C4 vanish when the host quits. A world saved by vanilla loses the grenade slots.
- **Tactical insertion:** it leaves no marker on the ground, is not saved, and only MW2 blasts destroy it (TNT and creepers don't).
- **Particles:** taming hearts, bucking smoke, ocelot trust hearts, the villagers' sweat in raids and the llama spit puff are not drawn yet.
- **Trading:** number keys, Q, double-click and drags on the payment and result slots do nothing.
- **Effects:** the inventory screens don't list the effects beside them, so the HUD icons stay. `/effect give` of an omen plays no sound. The ominous bottle's tooltip has no "Bad Omen" line.
- **Horses:** the mount is not drawn in its inventory screen, only one player can open a mount's inventory at a time, the rider takes no fall damage, there is no gallop sound and no vehicle hearts. Leather horse armour is undyed.
- **Leads:** a knot can't be punched off; break the fence and the mob is free within 5 seconds.
- **Cats:** a cat on a chest does not keep it shut. Phantoms don't exist yet.
- **Drowned:** a raised trident uses the plain held model. Zombie nautilus jockeys and the ocean ruin drowned's trident roll are missing.
- **Golems:** a dispenser does not finish a golem with a pumpkin, and copper golems don't exist yet.
- **Chests:** abandoned camps, shipwreck map chests and ocean ruins still roll nothing (explorer maps are missing). Sculk shriekers, sensors and catalysts do nothing yet.
- **Illagers:** banners show plain white (no patterns), a held crossbow shows no arrow or pull, and Johnny comes only from `/summon` (no name tags). Evoker fangs give no knockback, and vexes ignore water and lava.
- **No door breaking:** vindicators in raids and zombies on Hard don't break or open doors.
- **Raids:** a ravager walks to the village itself even with a rider, raiders don't pick up banners, and there are no raid omen particles.
- **Bells** don't swing, and arrows don't ring them (wind charges do now). The glow is a white hull, not vanilla's outline.
- **Fireworks:** only celebrating villagers launch rockets. A rocket does not burst on a mob in its path, and mobs hurt by a burst are not knocked back.
- **Thrown things:** a player's tridents, snowballs, eggs and splash potions can't hit pillagers, vindicators, evokers, vexes, ravagers, hoglins, piglins, striders, ghasts or blazes. Arrows and guns can.
- **Vanilla quirks kept:** the wandering trader's egg brings no llamas, a llama can't be steered, and patrols drift towards the world's origin.
- **Friends:** no command suggestions, and no frost outline in powder snow. A friend's jump bar and effect blink lag the host by the round trip.
- The limits at the end of [../round9/release-1-tests.md](../round9/release-1-tests.md) still apply, except its first two lines: friends now skate (with Skate 3), see their own held item and its swing, hear music, trade, hit other players with a sword and see the portal swirl.

## Screenshot numbering

- Name each screenshot by its number: `14.png`. When a step asks for both windows, name them `36-host.png` and `36-friend.png`.
- When a step fails, take a screenshot even if the step doesn't ask for one. Name it by its step label, e.g. `W-8.png` or `F-20-friend.png`. Say in one line what you expected and what you saw.
- Send `latest.log` after any crash. In two windows, send both windows' logs (see Setup for where they are).
