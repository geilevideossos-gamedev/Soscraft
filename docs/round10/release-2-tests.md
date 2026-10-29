# Round 10, release 2: test steps

Build: `iw4l-survival.exe` from `minecraft-survival` at `605ad21e`: release 1 plus track D0's segment d0a, track E1's segment e1a and track D's segment d1, then a test fix and the protocol bump. The release 1 exe (`6e0e0573`) is kept as `iw4l-survival-6e0e057.exe`. Release 1's steps ([release-1-tests.md](release-1-tests.md)) are not repeated here.

**Multiplayer is protocol 7** (release 1 was 6). A friend on the release 1 exe is refused at the join with the version message, so the host and every friend need this build. `mp-local.ps1` runs it in both windows. The relay `iw4l-master.exe` was copied in again with this build (its own protocol is unchanged). A friend over the internet needs a new zip from `-FriendKit` (see Setup).

**Send numbered screenshots.** Every **Screenshot N** below has its own number, from 1 to 78 through the whole document. Name each file by its number (`14.png`), or `61-host.png` and `61-friend.png` when a step asks for both windows. See "Screenshot numbering" at the end.

The whole test takes about 6.5 hours: about 4 hours 45 minutes for the single-player parts S, T, V, W, H, I and R (any order), and about 1 hour 40 minutes for the two-window parts F1 and F2. Spread it over a few sittings. Steps marked **optional** can be skipped.

## What is in this build

Track D0, segment d0a, the shared mob registry ([track-d0-log.md](track-d0-log.md)):

- Silverfish and endermites live. They hunt you and bite (half a heart and a full heart). They come from eggs, `/summon` (with Tab completion), mob spawners and stronghold spawners, and they are saved with the world.
- Both are drawn like vanilla: the side-to-side wiggle, the red flash, the roll onto the back when they die, the endermite's purple sparkles, the Glowing outline and tiny copies in spawner cages.
- MW2 guns hit them. Iron golems hunt both, endermen hunt endermites, and an endermite leaves after 2 minutes. Friends see them and can shoot them.
- A silverfish spawner over stone, cobblestone, stone bricks or deepslate keeps spawning in daylight and torchlight, as in a stronghold's portal room.
- Players can't sleep with a pillager, vindicator, evoker, vex, ravager, wither, silverfish or endermite near the bed.
- Behind this is one shared mob registry, which the round's new mobs will plug into.

Track E1, segment e1a, screens, visuals and sounds ([track-e1-log.md](track-e1-log.md)):

- `/give`, `/kit` and the `.env` kit take banner patterns, shield colours, loaded crossbows, rocket flight times and firework stars. Only the rocket's flight time shows yet; the rest is stored and not drawn.
- The starting kit keeps the quotes inside components (book pages, banner patterns), as `/kit` already did.
- Particles: taming hearts, smoke for a failed try or a bucking horse, the ocelot's trust hearts and love hearts. Villagers show hearts, anger, green sparkles after a trade and water drops in a raid. Friends see them too.
- Trading screen: number keys and F on the payment and result slots, Q and Ctrl+Q, drags over the payments, and no gathering on a double click (as vanilla).
- The active effects are listed beside the inventory and the creative screen.
- Omens given with `/effect give` play their sound, and the ominous bottle's tooltip shows its Bad Omen line.
- Horses take vanilla fall damage (safe up to 6 blocks, half damage, hay and beds soften it), and their riders share it. Ridden horses gallop and snort. Riders see their mount's hearts where the food bar was, friends too.
- Dyed leather items show their colour in icons, in the hand and on the ground.

Track D, segment d1, trial chambers, vaults and lodestones ([track-d-log.md](track-d-log.md), the round 10 entries):

- Fireballs and wither skulls keep their shooter through a save.
- A player flying in creative is not pushed by a wind charge. The code was already right and a test now pins it, so release 1's known limit was wrong.
- Trial spawners: the turning mob, flames and hum, spawn eggs, and the trial itself (detection through glass and bars, mobs in turn, one reward per player, the cooldown, saving), with vanilla's particles and sounds. Ominous trials with Trial Omen, armed mobs and items dropped from above.
- Mob spawners and trial spawners spawn husks, strays and slimes without open sky or a slime chunk, and trial spawners ignore light.
- Vaults take trial keys once per player, show their reward item turning and have vanilla's effects.
- Lodestone compasses, and friends' compasses and clocks turn like the host's.

Also in the build: `274c6dd9` makes a flaky test wait by time (nothing in game), and `605ad21e` moves multiplayer to protocol 7.

**Not in this build:** track E2 (thrown items hitting the new mobs, doors broken by zombies and vindicators, fireworks bursting on mobs, explorer maps and shipwreck loot), track W (darkness, sculk, the warden), track F (Margit), the M1 to M3 mobs, banner and crossbow drawing, swinging bells, llama spit and dyed horse armour on the horse.

## Setup

- **The exe:** `iw4l-survival.exe` in `C:\Games\Mashup\2010-Rust-Rewrite-Mashup`, with the relay `iw4l-master.exe` next to it. Part F1 starts the release 1 exe `iw4l-survival-6e0e057.exe` once, for the version check.
- **Class:** keep release 1's class, an **M4A1 with Red Dot Sight, a USP .45, Frag and Flash**. The M4A1 is used against silverfish and endermites (S, F1).
- **Single player:** set `IW4L_MINECRAFT_WORLD=<world>` in the `.env` next to the exe, then start `iw4l-survival.exe map minecraft:overworld`.
- **Two windows on this PC:** run from PowerShell in `C:\Dev\mashup-crafting`:

  ```
  & scripts\mp-local.ps1 -Exe iw4l-survival.exe -World <world> -Fresh -HostSound
  ```

  - `-Fresh` starts the world new. Leave it out to go on with a world.
  - To start only the friend window again after the friend quit: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly`.
- **A friend over the internet** needs a new zip: `& scripts\mp-local.ps1 -FriendKit -Exe iw4l-survival.exe -PublicAddress <your public IP>`, then [../round8/friend-install.md](../round8/friend-install.md).
- **The `.env` next to the exe:** besides `IW4L_GAMES`, it needs these lines. Replace the world and kit lines for each part:

  ```
  IW4L_SKATE=off
  IW4L_MINECRAFT_WORLD=r10silver
  IW4L_MINECRAFT_KIT=<the part's kit line>
  ```

  - If you set up Skate 3 for release 1's part K, its `IW4L_SKATE_ASSETS=...` line can stay instead of `IW4L_SKATE=off`. Skating is not tested here.
  - Remove `IW4L_MINECRAFT_DIMENSION` and `IW4L_MINECRAFT_SEED`. Keep it free of `IW4L_MASTER_HOST_NAME` and `IW4L_MASTER_JOIN`.
  - In two windows the world comes from `-World`, and both windows read the kit line.
- **Kits:** each part has its own kit line in its own block. A kit fills only the backpack of a player new to the world. Every part starts a new world, so the kit comes with the first join.
  - **The first join also gives the class loadout,** to the host and to every friend: the M4A1 on hotbar slot 1, the USP .45 on slot 2, 32 frags and 32 flashes in the two grenade slots, and 90 rifle and 36 pistol rounds (three stacks). The rounds go into the backpack, or onto hotbar slots 3 to 5 when the kit fills the backpack (only kit T does).
  - So in every part except T, hotbar slots 3 to 9 and the offhand start empty. "An empty hand" below means one of those empty hotbar slots (the MW2 hands show).
  - **New in this build:** the starting kit reads its line from the `.env` file itself, so quotes inside components stay (part I checks this with a book).
  - In a world played before, `/clear` and `/kit` give the kit again, but no loadout.
- **Names:** `Sos` is the host's player name and `Friend` the friend's. Use your names if they differ.
- **In every new world**, the host types `/difficulty normal` and `/gamerule keep_inventory true` first. In two windows the host also types `/op Friend`.
- **Commands:** `/kill` only takes players, so the steps clear monsters with `/difficulty peaceful` and then `/difficulty normal`. Only the host gets Tab suggestions.
- **Keys:** E opens the inventory, Q drops one item and Ctrl+Q a whole stack, F swaps hands, T opens the chat for commands, F5 changes the camera, G throws the lethal grenade. In an open screen, a number key or F over a slot swaps that slot with the hotbar slot or the offhand.
- **Logs:** single player, `iw4l-artifacts\logs\latest.log`. With the script, `iw4l-artifacts\mp-local\host\iw4l-artifacts\logs\latest.log` and `...\mp-local\friend\...`.
- Steps marked **(two windows)** need the friend. Only the window in front gets your keys.

## S. Silverfish, endermites and beds

World `r10silver`, single player, a fresh world. Survival, Normal, by day, on flat open ground in the Overworld (plains are best). Allow about 30 minutes.

Kit S (17 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:silverfish_spawn_egg*16,minecraft:endermite_spawn_egg*8,minecraft:spawner*3,minecraft:stone_bricks*64,minecraft:stone_bricks*64,minecraft:andesite*64,minecraft:andesite*64,minecraft:torch*32,minecraft:enderman_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:bow,minecraft:spectral_arrow*16,minecraft:iron_sword,minecraft:lead*2,minecraft:white_bed,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

Put the iron sword, the bow, the lead and the silverfish and endermite eggs on hotbar slots 3 to 7. The spectral arrows can stay in the backpack (the bow takes them from there). Swap the other items onto slots 8 and 9 as the steps need them.

### Summons and models

- **S-1.** Type `/summon minecraft:s` and press Tab: `minecraft:silverfish` is among the suggestions. Run `/summon minecraft:silverfish ~ ~ ~3`. The chat says "Summoned new Silverfish" (release 1 said "Unable to summon entity"), and a small grey silverfish crawls at you. **Screenshot 1** (the chat line and the silverfish).
- **S-2. The sword.** Hit the silverfish once with the iron sword: it flashes red and lives. Hit it again: it dies, rolls over onto its back, lies there for a second and puffs away, leaving experience.
- **S-3.** Use a silverfish egg three times on the ground 6 blocks away. Three small grey silverfish crawl towards you, wiggling from side to side. Each is a row of seven segments with three fur ridges on its back, and each bites for half a heart about once a second. **Screenshot 2.** Shoot them with the M4A1: each flashes red and dies after a hit or two and leaves experience.
- **S-4.** Use an endermite egg 4 blocks away. A small dark purple mite of four segments comes at you, purple portal sparkles drift off it all the time, and its bites take a full heart. **Screenshot 3** (with the sparkles). Shoot it with the M4A1: it dies and leaves experience.
- **S-5.** Use a silverfish egg and shoot the silverfish with a spectral arrow from the bow: a white outline traces its whole body. **Screenshot 4.**
- **S-6.** Hold the lead and right-click the silverfish: nothing happens and the lead stays in your hand. Kill it.
- **S-7. Its hunters.** Don't look at an enderman's head in this step.
  - Use an endermite egg and an enderman egg 6 blocks apart: the enderman goes for the endermite and hits it.
  - Use a silverfish egg and an iron golem egg 6 blocks apart: the golem walks to the silverfish and hits it.

  **Screenshot 5** (either hunt). Then `/difficulty peaceful` (the enderman goes) and `/difficulty normal`.
- **S-8. The endermite leaves.** Build a cobblestone pillar 2 blocks high. Use an endermite egg 3 blocks from it, climb the pillar and watch the endermite without hitting it. After about 2 minutes it vanishes at once, with no roll onto its back and no experience.
- **S-9. Saving.** Use a silverfish egg and an endermite egg next to you, then save and quit through the menu at once. Load the world: both are there and come at you. **Screenshot 6.** Kill them.

### Spawners in daylight

- **S-10.** Lay a 9 by 9 floor of stone bricks in full daylight, place a spawner in the middle and use a silverfish egg on it: a tiny silverfish turns in the cage. Stand 6 blocks away. Within half a minute silverfish crawl out onto the stone bricks in broad daylight. **Screenshot 7** (the cage and the silverfish).
- **S-11.** Put torches all around the spawner on the stone bricks: silverfish keep coming. Break the spawner and kill the silverfish.
- **S-12.** 15 blocks away, lay a 9 by 9 floor of andesite in daylight, place the second spawner in the middle and use a silverfish egg on it. Wait a minute 6 blocks away: no silverfish come. Break it.
- **S-13.** Place the third spawner on the stone bricks and use an endermite egg on it: a tiny endermite turns in its cage. **Screenshot 8.** Wait a minute: no endermites come in daylight. Break it.

### Beds

- **S-14.** `/time set night`. Place the bed on open ground and stand beside it. If a zombie or another monster comes near during these steps, kill it before you try the bed. `/summon minecraft:pillager ~3 ~ ~ {NoAI:1b}` and right-click the bed: "You may not rest now; there are monsters nearby" shows and you stay up. **Screenshot 9.** `/difficulty peaceful` (the pillager vanishes), then `/difficulty normal`.
- **S-15.** Do the same with `minecraft:vex`, `minecraft:ravager` and `minecraft:silverfish`, one at a time, each with `{NoAI:1b}`: each one keeps you awake. Clear each with `/difficulty peaceful` and `/difficulty normal` before the next.
- **S-16.** `/summon minecraft:wither ~6 ~ ~ {NoAI:1b}` and right-click the bed: you stay up. `/difficulty peaceful` (the wither vanishes with its bar), then `/difficulty normal`.
- **S-17.** With no monster near, right-click the bed: you lie down and the night passes. **Screenshot 10** (lying down).

## T. Trial spawners

World `r10trial`, single player, a fresh world. Survival, Normal. Put on the diamond armour first and keep the totem out of the offhand. Allow about 75 minutes.

Kit T (27 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:totem_of_undying,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:cobblestone*64,minecraft:glass*32,minecraft:iron_bars*16,minecraft:ladder*32,minecraft:water_bucket,minecraft:trial_spawner*3,minecraft:spawner*2,minecraft:zombie_spawn_egg*4,minecraft:skeleton_spawn_egg*2,minecraft:husk_spawn_egg*4,minecraft:slime_spawn_egg*4,minecraft:ominous_bottle*4,minecraft:ominous_bottle[ominous_bottle_amplifier=1]*2,minecraft:milk_bucket
```

The kit fills the backpack, so the loadout's rounds land on hotbar slots 3 to 5.

"A fresh spawner" is one that has not run a trial yet. A trial chamber has plenty, and a used one is fresh again after its 30-minute cooldown. Trial spawners never see a player in creative, so explore in creative and switch to survival only when a step says so.

### The chamber

- **T-1.** `/locate structure minecraft:trial_chambers`. `/gamemode creative`, then `/tp <x> 100 <z>` with the x and z it gives. Dig straight down (in creative a click breaks a block at once) until you reach the copper and tuff rooms, about y −40 to −20. Light the way with torches.
- **T-2. Waiting spawners.** Walk through the rooms in creative. The zombie, husk, spider, skeleton, stray, bogged, cave spider, baby zombie and slime trial spawners each show a small mob turning slowly inside (about one turn every 2 seconds), with a small orange flame now and then and a low hum now and then. None of them wakes up for you in creative. **Screenshot 11.**
- **T-3.** Breeze and silverfish trial spawners stay dark, with no mob and no flames (see Known limits).

### A trial

- **T-4.** Step out of sight of a fresh zombie trial spawner, `/gamemode survival`, then walk into its sight within 14 blocks.
  - With the detect sound, a column of orange-gold sparks rises from it (about 35 of them).
  - It lights up (a brighter block) and the mob inside spins faster.
  - About 2 seconds later zombies come out one at a time, one a second, 6 in all and never more than 3 at once.
  - Each zombie appears in a puff of smoke and flames with a low spawn sound, and the spawner puffs too.

  **Screenshot 12.**
- **T-5. Optional.** During the trial, lure one zombie 50 blocks away (run so it follows you, and come back without it): the spawner forgets it and sends a new one about 2 seconds later.
- **T-6.** Kill them all. About 2 seconds after the last one dies, the shutter opens with a click. One reward (a trial key, or food and potions) pops out on top with a sound and a little burst of small flames and smoke. Then the spawner goes dark and smokes. **Screenshot 13.** Walk up to it: nothing more happens (it cools down for 30 minutes).
- **T-7. Optional.** The husk, spider and plain skeleton spawners run the same way. A "slow ranged" skeleton spawner (in the rooms with raised platforms) sends one every 8 seconds, up to 4 at once.

### Sight, peaceful and saving

- **T-8. Glass.** `/gamemode creative` and go to the doorway of another fresh spawner's room (or dig a hole in the room's wall), within 14 blocks of the spawner.
  - Standing beside the doorway, out of the spawner's sight, fill the doorway with cobblestone and `/gamemode survival`. Step in front of it: nothing happens for 10 seconds.
  - Step back beside the doorway, replace the cobblestone with glass (or iron bars) and step in front again: the spawner detects you through the glass and the trial starts. Break the glass to fight.
- **T-9. Peaceful.** While its zombies are out, `/difficulty peaceful`: the zombies vanish and the spawner goes back to waiting (dim, with its mob turning slowly). `/difficulty normal`: being in its sight starts a new trial. Win it.
- **T-10. Saving.** Start a trial at a third fresh spawner and kill 2 of its zombies. Save and quit, then load. The same spawner goes on where it was: it keeps its count and does not start over. Once all 6 have come and died, it ejects the reward.

### Placed trial spawners and spawn eggs

- **T-11.** Back on the surface, at least 30 blocks from the chamber, place a trial spawner on the ground: it stays dark (no mob). Use a zombie egg on it: the egg is used up and a small zombie turns inside with small orange flames. Because you stand within 14 blocks in its sight, it wakes up at once: zombies come one every 2 seconds, 6 in all, never more than 2 at once (vanilla's default numbers). **Screenshot 14.** Kill them and take the reward.
- **T-12.** `/gamemode creative`. Use a skeleton egg on the same spawner: a small skeleton now turns inside, and the spawner just waits (you are in creative).
- **T-13. Refusals.**
  - `/difficulty peaceful`, then `/gamemode survival` (in this order, so the spawner can't start a trial in between). Use a zombie egg on the spawner: nothing changes (the skeleton stays) and the egg stays in your hand.
  - `/gamerule spawner_blocks_work false`, then `/difficulty normal`. Use a zombie egg on it: the chat says "Spawner blocks are not enabled" and the egg stays.
  - `/gamemode creative`, then `/gamerule spawner_blocks_work true`.

### Underground: husks and slimes

- **T-14.** Stay in creative. At least 30 blocks to the side of the chamber, dig a room about 7 by 7 and 4 high at about y −30, with no sky above and no torches. `/gamemode survival`. Place a mob spawner (the plain spawner from the kit), use a husk egg on it and stand 5 blocks away: husks come out in the dark (until this build a husk spawner needed open sky). **Screenshot 15.**
- **T-15.** Light the room with torches all around the husk spawner: no more husks come (a mob spawner still needs the dark for monsters). Kill the husks and break the spawner.
- **T-16.** Leave the torches. Place the second mob spawner in the room and use a slime egg on it: slimes come out anyway (a spawner's slimes need no slime chunk and no dark). Kill them and break it.
- **T-17.** Place the other two trial spawners in the lit room, at opposite sides of it (about 5 blocks apart). Use a husk egg on one and a slime egg on the other: husks and slimes come out of both in the torchlight (trial spawners ignore light). **Screenshot 16.** Win both trials.

### Ominous trials

Go back to the chamber. Use fresh zombie spawners.

- **T-18. Trial Omen.** Drink a plain ominous bottle: the Bad Omen icon shows at the top right. Walk into sight of a fresh zombie spawner. A deep chime plays, blue (soul) flames swirl at your head and at the spawner, and the Bad Omen icon becomes Trial Omen (15 minutes). **Screenshot 17.**
- **T-19. Armed mobs.** The spawner burns with blue flames. Its zombies come with iron or diamond swords and often with iron, chainmail or diamond helmets and chestplates. Kill them: no sword or armour drops.
- **T-20. Items from above.** About 8 seconds after it turned, a small item appears 2 to 5 blocks above you or above one of its zombies. It grows for 2.5 seconds, spins, is lit, and has blue sparks flying into it, with a soft chime. **Screenshot 18.**
- **T-21. The drop.** About 1.5 seconds before it drops, a rising sound plays. Then it drops with a puff of blue flames:
  - a lingering potion breaks into a cloud;
  - an arrow flies down (a plain, poison or slowness arrow);
  - a wind charge flies down, with the whoosh of a thrown wind charge;
  - or a fire charge falls burning, with a blaze's shot.

  A new item follows every 8 seconds or more while the trial lasts. Each spawner only ever drops two kinds of item: one kind of lingering potion, and one of the arrow, wind charges or fire charges (as vanilla). Spawners in the same area of about 30 by 30 blocks drop the same two kinds, so a spawner farther away may show other kinds.
- **T-22. A low roof.** Stand under a low roof (place cobblestone so you have two blocks of headroom): the next item above you appears just under the roof.
- **T-23. Saving.** While an item floats above you, save and quit, then load. The item is still there and drops 3 to 6 seconds later. The spawner is still blue and goes on.
- **T-24. The reward.** Win the trial: no more items fall. It ejects an ominous trial key or ominous consumables. **Screenshot 19.**
- **T-25.** While Trial Omen lasts, walk into sight of a second fresh spawner. It turns blue at once: the blue flames swirl at your head and at the spawner again, with the spawner's activation sound, but the Trial Omen's own chime does not play again and the Trial Omen time does not change. Win it, or leave it.
- **T-26. Turning mid-trial.** `/effect clear @s`. Start a plain trial at a third fresh spawner. While its zombies are out, drink a level II bottle (the kit's ominous bottle with amplifier 1, which gives Bad Omen II) and stay in its sight. The zombies vanish in puffs of flame and the trial starts over, blue. The Trial Omen icon shows 30 minutes.
- **T-27.** Save and quit, then load: the spawner is still blue and goes on. Win it.
- **T-28. Optional, an arrow on Hard.** `/difficulty hard`. Take the armour off. At an ominous spawner (Trial Omen still turns a fresh one blue), build a cobblestone pillar 2 blocks high and stand on it, out of the zombies' reach. Wait for an arrow to fall on you from above (the floating item is an arrow). If this spawner never drops arrows (T-21), use one at least 30 blocks away. It takes about 1.5 hearts, as on Normal, not 2¼. **Screenshot 20.** If it kills you, the chat says "Sos was shot by Arrow". `/difficulty normal` and put the armour back on.

## V. Vaults and lodestones

World `r10vault`, single player, a fresh world. Survival, Normal, by day, on open ground in the Overworld. Allow about 40 minutes.

Kit V (19 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:trial_key*6,minecraft:ominous_trial_key*3,minecraft:vault*2,minecraft:lodestone*4,minecraft:compass*8,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:cobblestone*64,minecraft:ladder*32
```

### A placed vault

- **V-1.** Place a vault, step 6 blocks back and wait 2 seconds: it is dark (if it lit up while you stood next to it, it goes dark now). Walk to 3 blocks of it.
  - It lights up with a puff of smoke and small orange flames inside and a rising chime.
  - An item from its reward table turns inside, a full turn about every 1.8 seconds, changing to another item every second.
  - Small flames and smoke drift inside, and now and then it hums.

  **Screenshot 21.**
- **V-2.** Stand still next to it: about once a second a few faint glowing sparks fly from you into the top of the vault and sink into it. **Screenshot 22.**
- **V-3.** Step 5 blocks away: it goes dark with a puff of small flames from its middle and a falling sound, and the item is gone. Come back to 3 blocks: it lights up again.
- **V-4. A wrong key.** Right-click the lit vault with an ominous trial key: a dull failing sound, and the key stays. Hold right-click: the sound repeats at most about once every three quarters of a second.
- **V-5. A trial key.** Right-click it with a trial key: a click, and one key is gone. About a second later the shutter opens with a sound. Then items pop out on top one by one, one a second, each a little higher in pitch and each with a little burst of flames and smoke. While it ejects, the item inside shows the next reward to come out. **Screenshot 23.**
- **V-6.** After the last item the shutter closes with a sound and the vault goes dark. Right-click it with a second key: nothing happens and the key stays. Step 6 blocks away and come back: it stays dark for you.
- **V-7.** `/gamemode creative`. Place the second vault 10 blocks away and open it with a trial key: it opens and the key stays in your hand. `/gamemode survival`.
- **V-8.** Save and quit, then load: the first vault stays dark for you.

### The chamber's vaults

- **V-9.** Find a trial chamber as in T-1 (in creative: vaults still open for you, the key stays in your hand, and the trial spawners leave you alone). Its vaults are in the corridors near the chamber's middle; an ominous vault has a darker frame.
  - A plain vault opens with a trial key.
  - An ominous vault burns with blue (soul) flames. It refuses the trial key with the failing sound, takes an ominous trial key and gives other rewards (often a block of emerald, iron or diamond, sometimes a heavy core or an enchanted golden apple).

  **Screenshot 24** (the ominous vault's rewards).

### Lodestones

Go back to the surface and `/gamemode survival`.

- **V-10. The lock.** Place a lodestone. Put the stack of 8 compasses on the hotbar, hold it and right-click the lodestone. A metallic click plays, one glinting compass named "Lodestone Compass" appears in the inventory, and 7 plain compasses stay in your hand. **Screenshot 25.**
- **V-11. The needle.** Put the lodestone compass on the hotbar beside the plain ones. Walk 30 blocks away and turn round: the lodestone compass's needle points back at the lodestone, in the hotbar and in your hand, while the plain compass points to the world spawn. **Screenshot 26.**
- **V-12. Relinking.** Place a second lodestone 20 blocks to the side. Hold the lodestone compass and right-click the second lodestone: the click again, and no new compass appears (it changed in place). Its needle swings round to the new lodestone.
- **V-13. Creative.** `/gamemode creative`. Hold a plain compass and right-click the first lodestone: you keep the compass and get one more lodestone compass. `/gamemode survival`.
- **V-14. Optional, a full inventory.** Hold a stack of exactly 2 plain compasses. `/give @s minecraft:cobblestone 2304` fills every other slot (the rest drops at your feet). Right-click a lodestone: the lodestone compass drops on the ground in front of you. `/clear @s minecraft:cobblestone`, pick up the lodestone compass, and `/clear @s minecraft:cobblestone` again for the cobblestone you picked up with it.
- **V-15. A broken lodestone.** Hold the lodestone compass locked to the second lodestone and break that lodestone with the pickaxe: the needle starts spinning, and the name and the glint stay.
- **V-16. The Nether.** Build a Nether portal with the obsidian (4 wide, 5 high, without the corners), light it and go through. Place a lodestone there, lock a plain compass to it and walk away: it points at it. The Overworld lodestone compass from V-13 spins at random. **Screenshot 27.**
- **V-17.** Go back to the Overworld. Save and quit, then load: the lodestone compasses keep their names, the one from V-13 still points at the first lodestone, and the one from V-15 still spins.

## W. Wind charges and saved wither skulls

World `r10skull`, single player, a fresh world. Survival, Normal, by day, on open flat ground (plains are best). Allow about 15 minutes.

Kit W (14 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:wind_charge*16,minecraft:cobblestone*64,minecraft:soul_sand*8,minecraft:wither_skeleton_skull*6,minecraft:netherite_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:milk_bucket,minecraft:totem_of_undying,minecraft:cooked_beef*32
```

- **W-1. Flying in creative.** Build a cobblestone pillar 3 blocks high. `/gamemode creative`, double-tap jump to fly, and hover right beside its top. Throw a wind charge (right-click) at the top block next to you: the burst does not move you. **Screenshot 28** (still hovering beside the pillar).
- **W-2.** Double-tap jump to stop flying and land next to the pillar. Throw a wind charge at its foot, right next to you: it launches you, as it does on foot in survival. `/gamemode survival`.
- **W-3. Optional, fiddly: a skull after a reload.** Take the armour off (diamond armour cuts a skull's hit to under a heart, so 4 and 2.5 hearts could not be told apart).
  - Build a wither (four soul sand in a T, three wither skeleton skulls on top), run 20 blocks away and wait for the blast (11 seconds).
  - `/tick rate 2` (the world runs ten times slower). When a black skull is in the air towards you, `/tick freeze`.
  - Save and quit, then load the world. `/tick rate 20`, then `/tick unfreeze` (if the world is already running after the load, these change nothing).
  - The skull hits you for about 4 hearts (not 2.5) and gives Wither II (black hearts). **Screenshot 29** (the hearts after the hit). If it kills you, the chat says "Sos was shot by a skull from Wither".
  - `/difficulty peaceful` (the wither vanishes), `/difficulty normal`, drink the milk and put the armour back on.

## H. Horses and taming

World `r10horse`, single player, a fresh world. Survival, Normal, by day, on flat grass (plains are best). Eat golden carrots when you get hungry. Allow about 50 minutes.

Kit H (23 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:horse_spawn_egg*4,minecraft:llama_spawn_egg*2,minecraft:wolf_spawn_egg*2,minecraft:cat_spawn_egg*4,minecraft:ocelot_spawn_egg*2,minecraft:strider_spawn_egg*2,minecraft:wheat*64,minecraft:golden_carrot*32,minecraft:bone*32,minecraft:cod*64,minecraft:hay_block*16,minecraft:oak_planks*64,minecraft:warped_fungus_on_a_stick,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:saddle
```

Hearts are small red hearts that rise slowly for under a second. Smoke is a few grey puffs that drift up. "Saddle it" means right-click the horse with a saddle; then right-click it with an empty hand to ride. Crouch to get off.

### Taming

- **H-1. A horse.** Use a horse egg and right-click the horse with an empty hand: you ride it while it runs about. When it rears and throws you off, 7 grey smoke puffs rise around it. **Screenshot 30.** Feed it wheat a few times and try again until it stays calm: 7 red hearts rise around it, about half a block above its body. **Screenshot 31.**
- **H-2. A llama.** Use a llama egg and tame it the same way: smoke when it throws you, hearts when it is tamed.
- **H-3. A wolf.** Use a wolf egg and right-click the wolf with a bone. Each failure shows smoke. The success shows hearts, and the wolf sits with a red collar. **Screenshot 32.**
- **H-4. A cat.** Use a cat egg. Hold cod and stand still until it creeps close, then right-click it with cod. A failure shows smoke and a success shows hearts. **Screenshot 33.** **Optional:** tame a second cat the same way, then feed both cod again: hearts on each.
- **H-5. An ocelot.** Use an ocelot egg, hold cod and stand still. When it is within 3 blocks, right-click it with cod. A failure shows smoke. The success shows hearts, and it stops running from you. **Screenshot 34.**
- **H-6. Love hearts.** Feed the tamed horse from H-1 a golden carrot: hearts.
- **H-7.** Saddle the horse from H-1 and ride it: the food bar is gone, and between 7 and 15 mount hearts show on the right, depending on the horse.

### Falls

Before each drop have full health. Each drop uses a new tame horse and a saddle.

- **H-8. Ten blocks.** `/fill ~3 ~ ~-3 ~9 ~9 ~3 stone` (a stone block 10 high), then `/tp @s ~6 ~10 ~` and `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`. Saddle it, ride it and ride straight off the edge.
  - On landing you hear the horse's heavy hoof thud with the stone's fall sound, then your own big fall sound and hurt.
  - You lose 3.5 hearts (7 damage).
  - The horse flashes red with its hurt sound. It makes no generic fall sound of its own.

  **Screenshot 35** (your hearts just after landing).
- **H-9. Seven blocks.** At a fresh spot, `/fill ~3 ~ ~-3 ~9 ~6 ~3 stone` (7 high), `/tp @s ~6 ~7 ~`, then summon and saddle a new tame horse the same way. Ride off the edge: the thud plays, but the horse does not flash red and you lose no hearts. **Screenshot 36.**
- **H-10. Hay.** Stand at the foot of the 10-high block, where you landed in H-8, and run `/fill ~-1 ~-1 ~-1 ~1 ~-1 ~1 hay_block` (hay set into the ground). `/gamemode creative`, fly up onto the 10-high block and `/gamemode survival`. Summon and saddle a new tame horse there and ride off at the same place onto the hay: the thud plays, the horse does not flash red and you lose no hearts. Keep this horse for the gallop below.
  - **Jumps.** Ride this horse on flat grass. Hold Space until the jump bar is full and let go, ten times: the horse never flashes red and you never lose a heart (until this build a strong horse hurt itself on its own jumps).
- **H-11. Optional, creative.** `/gamemode creative` and fly up onto the 10-high block again. Summon and saddle a new tame horse and ride off on a side without hay: you lose nothing (the horse still flashes red). `/gamemode survival`.
- **H-12. Dropped from the air.** On flat grass:
  - `/summon minecraft:horse ~3 ~10 ~`: it falls 10 blocks, thuds and flashes red.
  - `/summon minecraft:llama ~-3 ~10 ~`: it falls 10 blocks and flashes red, with no hoof thud.
  - `/summon minecraft:cow ~ ~10 ~3`: the cow still lands with the generic big "oof" fall sound.

### Mount hearts

- **H-13. Thirty health, two rows.** `/summon minecraft:horse ~ ~ ~2 {Tame:1b,attributes:[{id:"minecraft:max_health",base:30.0d}]}`. Saddle it and ride it. The food bar is gone and 15 full mount hearts show on the right: a row of 10 at the hotbar's level and a row of 5 above it, both lined up at the right end. Your own hearts stay on the left. **Screenshot 37.**
- **H-14. A hurt horse.** Get off and punch the horse once with an empty hand (1 damage), then ride it again: the leftmost heart of the top row is now a half heart. **Screenshot 38.** Get off, right-click the horse with a golden carrot (it heals 4) and ride it again: all 15 hearts are full. Get off: the food bar comes back.
- **H-15. Twenty-one and twenty-two.** Summon a tame horse the same way with `base:21.0d`, saddle and ride it: one full row of 10 hearts. Then do the same with `base:22.0d`: 10 hearts and one more above the right end.
- **H-16. Creative.** Ride the 30-health horse and `/gamemode creative`: your own hearts go, and the mount hearts stay. `/gamemode survival`.
- **H-17. A strider.** Use a strider egg, saddle the strider and ride it holding the warped fungus on a stick: 10 mount hearts. **Screenshot 39.**
- **H-18. Optional, the bubbles.** Ride the 30-health horse to water at least 2 deep (a lake or a river). Get off, dive until most bubbles are gone, then climb out and ride the horse at once. While the bubbles refill, they show just above the top row of mount hearts.

### The gallop

These are sounds, so there are no screenshots: note what you hear for each step.

- **H-19.** Ride the horse from H-10 and hold W across the grass. The first five hoofbeats are hollow wooden clops. Then you hear a gallop every third step with quiet steps between, and now and then a snort.
- **H-20. Stopping.** Let go of W until the horse stands still, then hold W again: the five wooden clops come back before the gallop starts again.
- **H-21. Backing up.** Hold S: the horse backs up slowly with a wooden clop on every step and never gallops.
- **H-22. Planks.** Lay the oak planks as a strip 3 wide and 20 long (by hand, or ride facing south and run `/fill ~ ~-1 ~ ~2 ~-1 ~30 oak_planks`). Ride along it holding W: the same pattern, five clops, then a gallop every third step.
- **H-23. On its own.** Get off and step back a few blocks. When the horse wanders by itself you hear its ordinary hoof steps on grass and wooden clops on the planks, never a gallop.
- **H-24. A donkey.** `/summon minecraft:donkey ~ ~ ~2 {Tame:1b}`, saddle it and ride it holding W: ordinary hoof steps only (wooden clops on the planks), no gallop and no snort.

## I. Items, effects and omens

World `r10items`, single player, a fresh world. Survival, Normal, by day, on open grass at least 60 blocks from any village (an omen near a village can start a raid). Default GUI scale, and keep the recipe book closed unless a step says otherwise. Allow about 35 minutes.

Kit I (21 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:written_book[written_book_content={title:"Kit",author:"Owner",pages:["One, two"]}],minecraft:firework_rocket[fireworks={flight_duration:3}]*16,minecraft:firework_rocket*16,minecraft:elytra,minecraft:leather_horse_armor@red,minecraft:leather_horse_armor@blue,minecraft:leather_horse_armor@lime,minecraft:leather_horse_armor,minecraft:leather_chestplate@red,minecraft:leather_boots@blue,minecraft:compass,minecraft:clock,minecraft:ominous_bottle*2,minecraft:ominous_bottle[ominous_bottle_amplifier=4],minecraft:golden_apple*8,minecraft:potion@long_night_vision,minecraft:potion@long_night_vision,minecraft:chest,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cooked_beef*16
```

### The starting kit and dyed items

- **I-1. The book.** Open the written book "Kit" from the backpack: it has one page reading "One, two". Release 1 gave it two pages, "One" and "two". **Screenshot 40.**
- **I-2. Icons.** Open the inventory. The red, blue and lime leather horse armour icons are red, blue and lime, with their untinted buckles and trim on top. The plain one is brown. The red leather chestplate and the blue boots are red and blue. Hover each horse armour: all four are named "Leather Horse Armor". **Screenshot 41.**
- **I-3. In the hand.** Move the red horse armour and the blue boots to the hotbar. Hold the horse armour: it is red in your hand. Hold the boots: blue. **Screenshot 42.** Press F5 while holding the red horse armour: it is red in your character's hand. Press F5 twice more to go back.
- **I-4. On the ground.** Drop the lime horse armour and the red chestplate with Q: both lie on the ground in their colour. Pick them up: still lime and red. **Screenshot 43.**
- **I-5.** Hold the compass and turn round: the needle still swings, in your hand and in the hotbar. Hold the clock: it still shows the sun or the moon.

### Rockets and /give

- **I-6. Flight times.** Put on the elytra and move both rocket stacks to the hotbar. `/tp @s ~ ~30 ~` and press Space while falling to glide. Use a rocket from the plain stack (flight 1): it pushes for about 1 second. Use one from the other stack (flight 3): it pushes for about 2 seconds, clearly longer.
- **I-7. `/give` takes the five components.** Type each line. Each gives the item with no red error:
  - `/give @s minecraft:white_banner[banner_patterns=[{pattern:"creeper",color:"black"}]]`
  - `/give @s minecraft:shield[base_color=blue]`
  - `/give @s minecraft:crossbow[charged_projectiles=[{id:"minecraft:arrow"}]]`
  - `/give @s minecraft:firework_rocket[fireworks={flight_duration:2,explosions:[{shape:star,colors:[I;14602026]}]}] 8`
  - `/give @s minecraft:firework_star[firework_explosion={shape:creeper,colors:[I;4312372],has_twinkle:1b}]`

  The banner still looks plain white, the shield plain and the crossbow unloaded: nothing draws these yet.
- **I-8. Bad values.** Each line gives a red "Malformed '...' component: '...'" error that names the component and the reason, and gives nothing:
  - `/give @s minecraft:shield[base_color=purpl]` ("Unknown element name:purpl")
  - `/give @s minecraft:firework_star[firework_explosion={shape:square}]` ("Unknown element name:square")
  - `/give @s minecraft:white_banner[banner_patterns={pattern:"creeper",color:"black"}]` ("Not a list")

  **Screenshot 44** (the chat with the three errors).

### The ominous bottle

- **I-9.** Open the inventory and hover the stack of two ominous bottles: "Ominous Bottle", and under it a blue "Bad Omen (01:40:00)". **Screenshot 45.**
- **I-10.** Hover the single bottle: a blue "Bad Omen V (01:40:00)". **Screenshot 46.**
- **I-11.** `/gamemode creative`, open the inventory, search "ominous" and hover the bottle: "Bad Omen (01:40:00)". `/gamemode survival`.

### The effect list

- **I-12. The list.** Drink a night vision potion and eat a golden apple. `/effect give @s minecraft:hero_of_the_village 600 0`. The icons show at the top right. Open the inventory (E). To the right of the inventory is a column of grey boxes, each with an icon, a white name and a grey time. From the top: "Regeneration II" (a few seconds, while it lasts), "Absorption 01:5x", "Night Vision 07:5x", "Hero of the Village 09:5x". The icons at the top right are gone. **Screenshot 47.**
- **I-13. The recipe book.** Click the recipe book button. The inventory moves right and the boxes shrink to icons only. Hover one: a tooltip shows its name and time. **Screenshot 48.** Close the book: the full boxes come back.
- **I-14. Many effects.** Close the inventory. Run `/effect give @s minecraft:speed 60`, `/effect give @s minecraft:haste 60 1`, `/effect give @s minecraft:strength 60` and `/effect give @s minecraft:luck infinite`. Open the inventory. With 7 or more effects the boxes overlap, packed into the same height. Luck shows "∞" at the bottom, and Haste shows "Haste II". **Screenshot 49.**
- **I-15. Other screens.** Place the chest and open it: the icons stay at the top right and there is no list (as vanilla). `/gamemode creative` and press E: the list sits beside the creative screen, on every tab. **Screenshot 50.** `/gamemode survival`.
- **I-16. Clearing.** Drink a milk bucket and open the inventory: no boxes. Close it: no icons either.

### Omen sounds

- **I-17. Bad Omen.** `/effect clear @s`, then `/effect give @s minecraft:bad_omen 60`: an eerie omen sound plays and the Bad Omen icon shows. Run the same command again: no sound, because the omen is already there. `/effect give @s minecraft:bad_omen 60 2`: the omen gets stronger, still with no sound.
- **I-18. Trial and Raid Omen.** `/effect give @s minecraft:trial_omen 60`: the trial omen sound. `/effect give @s minecraft:raid_omen 5`: the raid omen sound. `/effect clear @s` at once.
- **I-19.** `/effect give @s minecraft:speed 60`: no sound.
- **I-20. The bottle.** Drink an ominous bottle: the omen sound plays once (not twice), then the bottle's dispose sound. Drink the second milk bucket.

### Saving and /kit

- **I-21.** Save and quit, then load. Glide again (as in I-6) and use a rocket from the flight 3 stack: it still pushes for about 2 seconds.
- **I-22.** `/clear`, then `/kit`: the kit comes again (without the class loadout). Open the new book: it also has the one page "One, two".

## R. Trading and villagers

World `r10trade`, single player, a fresh world. Survival, Normal, by day, on open grass at least 60 blocks from any village. Allow about 40 minutes.

Kit R (23 stacks; each `*192` gives three stacks):

```
IW4L_MINECRAFT_KIT=minecraft:wheat*192,minecraft:potato*192,minecraft:carrot*192,minecraft:beetroot*192,minecraft:emerald*64,minecraft:villager_spawn_egg*4,minecraft:composter,minecraft:oak_fence*32,minecraft:bread*32,minecraft:white_bed,minecraft:white_bed,minecraft:white_bed,minecraft:ominous_bottle*2,minecraft:iron_chestplate,minecraft:cooked_beef*32
```

The hotbar starts with only the M4A1 (slot 1) and the USP .45 (slot 2), so the steps use slots 4 to 7. Use slot 3 for the composter, the fences and the eggs, and before R-3 put whatever is on slots 3 to 9 and in the offhand back into the backpack, so they are empty.

### The farmer and an angry villager

- **R-1.** Place the composter, fence a 5 by 5 pen around it and use a villager egg inside. Wait until it wears a straw hat (a farmer). This can take a minute or two.
- **R-2. Anger.** Use a second villager egg 10 blocks from the pen. Punch that villager once with an empty hand: a few dark grey clouds puff over its head. **Screenshot 51.**

### Number keys and F

- **R-3.** Right-click the farmer and pick its crop offer (wheat, potatoes, carrots or beetroot: a 26.3 farmer picks 2 of its 5 novice trades at random, so it always wants at least one crop). The crop moves into the first payment slot and an emerald shows as the result. "The crop" below means that one.
- **R-4. A number key on a payment.** Hover the first payment slot and press 4: the crop goes to hotbar slot 4 and the emerald disappears. **Screenshot 52.** Press 4 again: the crop goes back and the emerald returns.
- **R-5. F on a payment.** Hover the payment and press F: the crop goes to the offhand. Press F again to bring it back.
- **R-6. A number key on the result.** Hover the result and press 5. One trade is made: the emerald lands in hotbar slot 5, the payment shrinks by the price, an experience orb drops, and green sparkles rise over the farmer's head. Press 5 again over the result: nothing happens, because a filled hotbar slot doesn't take the result. **Screenshot 53.**
- **R-7. Swapping two stacks.** Close the screen and put a stack of another crop in hotbar slot 6. Open the farmer again and pick the crop offer. Hover the payment and press 6: the two stacks change places, and the result shows only if the farmer also buys the other crop.

### Q and Ctrl+Q

Pick the crop offer again first.

- **R-8. Q on the result.** Hover the result and press Q: one trade is made, and the emerald is thrown in front of you with an experience orb. **Screenshot 54.**
- **R-9. Ctrl+Q on the result.** Press Ctrl+Q over the result: it keeps trading and throwing emeralds until the crop in the payment runs short or the offer is sold out (a red cross). An orb drops for each trade.
- **R-10. Q on a payment.** Pick the crop offer again. Hover the payment and press Q: one crop is thrown. Press Ctrl+Q: the rest of that stack is thrown and the result disappears. **Screenshot 55.** Close the screen and pick everything up.
- **R-11. Carrying a stack.** Open the farmer. Pick up a stack from the inventory, hover the result and press Q: nothing happens. Put the stack back.

### Drags and the double click

Close the screen and open the farmer again with empty payment slots, without picking an offer.

- **R-12. Left drag.** Pick up a full stack (64) of the crop. Hold the left button, drag across the first payment slot and one empty inventory slot, then let go: 32 land in each, the cursor is empty, and the emerald shows as the result. **Screenshot 56.**
- **R-13. Right drag.** Pick up another full stack of the crop and right-drag over both payment slots and the same inventory slot: one more lands in each (33, 1 and 33). The emerald disappears, because a trade that costs one item wants the second payment slot empty (as vanilla). **Screenshot 57.**
- **R-14. Four slots.** Close the screen (the payments go back to the inventory) and open it again. Pick up a full stack and left-drag over both payment slots and two empty inventory slots: 16 land in each of the four, and the cursor is empty.
- **R-15. The double click.** Close and open the farmer again. Put two small stacks of the same crop in the inventory (split one with a right click). Click one of them to pick it up, then click the same slot again quickly: the stack is put back, and the other stack is **not** gathered onto the cursor. **Screenshot 58.** Shift-double-click a crop stack: only that stack moves between the inventory and the hotbar.
- **R-16.** Close the screen and press E. Do the same double click: here the other stack is gathered onto the cursor (as before).

### Villager hearts and a raid

- **R-17. Optional, villager hearts.** Put the three beds down inside the farmer's pen and use a third villager egg there. Throw each of the two villagers in the pen 6 bread. After a while hearts rise over both, and a baby villager appears with hearts of its own.
- **R-18. Raid sweat.** Put on the iron chestplate. `/locate structure #minecraft:village`, then `/gamemode creative` and `/tp <x> 100 <z>` with the x and z it gives. Land in the village and `/gamemode survival`.
  - Walk 60 blocks out of the village, drink an ominous bottle (the Bad Omen icon shows) and walk back in. The Bad Omen turns into Raid Omen, and about 30 seconds later the raid starts.
  - While the villagers hide in the houses, each one now and then (about every 5 seconds) shows a few light blue water drops around its head, which fall and vanish.

  **Screenshot 59** (through a door or a window). Fight the raiders with the M4A1, or `/difficulty peaceful` to clear them and then `/difficulty normal`.

## F1. Friends: mobs, horses, items and trading (two windows)

World: two windows, `-World r10mp1 -Fresh -HostSound`. Host H (`Sos`), friend F, both in survival, on flat open ground in the Overworld by day. Allow about 50 minutes.

Kit F1, for both players (24 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:silverfish_spawn_egg*8,minecraft:endermite_spawn_egg*4,minecraft:wolf_spawn_egg*2,minecraft:horse_spawn_egg*4,minecraft:cat_spawn_egg*2,minecraft:villager_spawn_egg*4,minecraft:strider_spawn_egg*2,minecraft:bone*32,minecraft:cod*32,minecraft:golden_carrot*16,minecraft:composter,minecraft:oak_fence*32,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:potion@long_night_vision,minecraft:ominous_bottle*2,minecraft:ominous_bottle[ominous_bottle_amplifier=4],minecraft:leather_horse_armor@red,minecraft:leather_horse_armor@lime,minecraft:leather_chestplate@red,minecraft:cooked_beef*16
```

### The version check

- **F-1.** Once both have joined, F quits to the main menu and closes the friend window. Start the release 1 exe as the friend: `& scripts\mp-local.ps1 -Exe iw4l-survival-6e0e057.exe -FriendOnly`. It is refused at the join with "Incompatible client! Please use the host's build (Minecraft multiplayer protocol 7)". **Screenshot 60** (the friend window). Close it and start this build's friend again: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly`. It joins.

### Silverfish and endermites

- **F-2 (two windows).** H uses a silverfish egg and an endermite egg next to F. Both windows show the same models and the same wiggle, and the endermite's sparkles. F's game keeps running smoothly, with no error line in the chat, while they bite F (half a heart from the silverfish, a full heart from the endermite). **Screenshot 61**, both windows.
- **F-3 (two windows).** F shoots the silverfish with the M4A1: it flashes red and rolls over in both windows, and experience orbs drop there. F shoots the endermite the same way.

### Taming

- **F-4 (two windows).** F tames a wolf with bones while H stands 10 blocks away. The smoke of each failure and the hearts of the success show in **both** windows. **Screenshot 62**, both windows.
- **F-5 (two windows).** H uses a horse egg and rides the horse until it bucks, while F watches from 10 blocks: F sees the smoke. H keeps trying until it is tamed: F sees the hearts.
- **F-6 (two windows, optional).** F walks 200 blocks away while H tames a cat with cod. F comes back, and H feeds the tame cat cod again: F sees the hearts.

### Mount hearts, falls and the gallop

- **F-7.** F runs `/summon minecraft:horse ~ ~ ~2 {Tame:1b,attributes:[{id:"minecraft:max_health",base:30.0d}]}`, saddles it and rides it. F's food bar is replaced by 15 mount hearts in two rows on the right. **Screenshot 63**, F's window.
- **F-8 (two windows).** H punches F's horse once with an empty hand: in F's window the leftmost heart of the top row turns into a half heart within a moment. F gets off: F's food bar comes back. F rides again.
- **F-9 (two windows). Both at once.** H saddles and rides the horse H tamed in F-5, while F rides the 30-health horse. Each window shows its own mount's hearts, not the other's. **Screenshot 64**, both windows.
- **F-10 (two windows). The gallop.** F rides the 30-health horse past H, holding W. H hears F's horse clop five times and then gallop.
- **F-11.** F uses a strider egg, saddles the strider and rides it holding F's warped fungus on a stick: 10 mount hearts in F's window.
- **F-12 (two windows). A fall.** F runs `/fill ~3 ~ ~-3 ~9 ~9 ~3 stone`, then `/tp @s ~6 ~10 ~` and `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`, saddles the horse and rides off the edge while H watches from the ground. F loses 3.5 hearts and hears the fall, and H hears F's horse thud. **Screenshot 65**, both windows.

### Items, effects and omens

- **F-13 (two windows). Dyed items.** In F's window the red and lime horse armour and the red chestplate icons are coloured. F holds the red horse armour: it is red in F's hand, and H sees it red in F's hand. F drops the lime horse armour with Q: it lies lime on the ground in both windows. **Screenshot 66**, both windows.
- **F-14.** F hovers both ominous bottle stacks in F's inventory: a blue "Bad Omen (01:40:00)" and a blue "Bad Omen V (01:40:00)".
- **F-15.** F drinks the night vision potion and opens the inventory: the effect list shows beside it, and F's top-right icons hide. **Screenshot 67**, F's window. H's own screen is unchanged.
- **F-16 (two windows). The omen sound.** Do this before the farmer's pen is built, with both standing next to each other. H runs `/effect give Friend minecraft:bad_omen 30`: F hears the omen sound, and H hears it too. H runs `/effect clear Friend`.

### Trading and villagers

- **F-17.** H places the composter, fences a 5 by 5 pen around it and uses a villager egg inside. When it wears a straw hat, H right-clicks it, notes which crop its offer wants and closes the screen. F first empties hotbar slots 3 to 9 and the offhand (the horse armour, the fungus on a stick, the bottle and anything else F put there go into the backpack, or are dropped with Q). H gives F three stacks of that crop, for example `/give Friend minecraft:wheat 192` (or `potato`, `carrot`, `beetroot`). They land on F's hotbar slots 3 to 5, so slots 6 to 9 stay empty.
- **F-18 (two windows). Number keys.** F opens the farmer and picks the crop offer. F hovers the payment and presses 6: the crop goes to F's hotbar slot 6 and the emerald disappears. F presses 6 again: it comes back. F hovers the result and presses 7: one trade is made and the emerald lands in slot 7. H sees F's experience orb drop, and both windows show the green sparkles over the farmer. **Screenshot 68**, both windows.
- **F-19 (two windows). Q.** F hovers the result and presses Q: one trade is made and the emerald flies from F's eye in F's direction. H sees it land in front of F. **Screenshot 69**, both windows. F presses Ctrl+Q over the result: it keeps trading and throwing until the crop runs short.
- **F-20. A drag.** F closes and opens the farmer again with empty payment slots, picks up a full stack of the crop and left-drags over the first payment slot and one empty inventory slot: 32 land in each and the emerald shows.
- **F-21. The double click.** F splits a crop stack in two, picks one up and clicks the same slot again quickly: the stack is put back and the other is not gathered.
- **F-22 (two windows).** H uses a villager egg next to F, outside the pen, and punches the villager once: F sees the grey clouds.
- **F-23 (two windows, optional, long).** Raid as in R-18 with both players in the village: F also sees the villagers' water drops.
- Quit the friend first, then the host, through the menu.

## F2. Friends: compasses, vaults and trial chambers (two windows)

World: two windows, `-World r10mp2 -Fresh -HostSound`. Host H (`Sos`), friend F, both in survival, starting by day at the world spawn. Both put on the diamond armour. Allow about 50 minutes.

Kit F2, for both players (23 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:cobblestone*64,minecraft:ladder*32,minecraft:trial_spawner*2,minecraft:slime_spawn_egg*2,minecraft:ominous_bottle*2,minecraft:trial_key*4,minecraft:vault*2,minecraft:lodestone*2,minecraft:compass*4,minecraft:clock,minecraft:recovery_compass,minecraft:soul_sand*4,minecraft:wither_skeleton_skull*3
```

### Compasses and clocks

- **F-24.** F holds a plain compass and walks 20 blocks away from the spawn: its needle points back at the world spawn (it never moved for a friend before this build). F holds the clock: it shows the sun and turns with the day. **Screenshot 70**, F's window.
- **F-25 (two windows). The lock.** F places a lodestone and right-clicks it holding a plain compass: both players hear the click, and F gets a glinting "Lodestone Compass". **Screenshot 71**, both windows.
- **F-26.** F walks 20 blocks away from the lodestone: the lodestone compass points at it, in the hotbar and in F's hand. **Screenshot 72**, F's window.
- **F-27 (two windows).** F walks 30 blocks from the spawn. Host: `/kill Friend`. F respawns at the spawn and holds the recovery compass: it points to where F died. Host: `/execute in minecraft:the_nether run tp Friend 0 128 0` (the Nether roof): F's clock and plain compass spin. Host: `/execute in minecraft:overworld run tp Friend ~ ~ ~`: F lands at H.

### Vaults

- **F-28 (two windows).** F stands 10 blocks away. H places a vault, walks up to it and opens it with a trial key. After the last item it goes dark for H. Now F walks up to it: it lights up because F has not opened it. Both windows show the turning item and the small flames and smoke, sparks fly from F into the vault (not from H, whom it has rewarded), and F hears the hum. **Screenshot 73**, both windows.
- **F-29 (two windows).** H right-clicks the lit vault with a trial key: a refusing sound, and H's key stays. F opens it with F's own trial key and gets F's rewards. Then it goes dark for both. **Screenshot 74**, both windows.

### Trial spawners

- **F-30 (two windows).** Host: `/gamemode creative` and `/gamemode creative Friend` (trial spawners never see creative players). H places a trial spawner and F uses a slime egg on it: both windows show a slime turning inside, with small orange flames. **Screenshot 75**, both windows. H breaks it.
- **F-31.** Host: `/locate structure minecraft:trial_chambers`, then `/tp @a <x> 100 <z>` with the x and z it gives. Both dig down to the chamber in creative.
- **F-32 (two windows).** In the chamber, F sees the waiting spawners' mobs turning and their small flames, and hears the hum, as H does.
- **F-33 (two windows). A trial for two.** Host: `/gamemode survival` and `/gamemode survival Friend`, out of sight of the spawners. Both walk into sight of a fresh zombie spawner together.
  - The detect sound plays and a column of sparks rises (more than for one player).
  - It spawns 8 zombies for two players, at most 3 at once, each in a puff of smoke and flames with the spawn sound.
  - Both kill them. It ejects two rewards, one per player, 1.5 seconds apart, each with the eject sound and burst.
  - F sees the light, the faster spin, the zombies and the rewards.

  **Screenshot 76**, both windows.
- **F-34 (two windows). Ominous.** F drinks a plain ominous bottle and walks into sight of another fresh spawner. F's Bad Omen becomes Trial Omen with the chime, and both windows see the blue flames. **Screenshot 77**, both windows.
- **F-35 (two windows). Items from above.** While that trial runs, F sees the floating items above F or the zombies, with their blue sparks, and sees them drop. F hears the rising warning sound, the blaze's shot when a fire charge drops and the whoosh when a wind charge drops (each spawner drops only its own two kinds, see T-21). H, next to F, hears the same. **Screenshot 78**, F's window. Win the trial.
- **F-36 (two windows, optional).** Back on the surface on open flat ground: H builds a wither 20 blocks from both and both run back. After its blast, H saves and quits (F's session ends) and loads the world again. F joins again with `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly`, sees the wither's next skulls fly and hears them hit. Host: `/difficulty peaceful`, then `/difficulty normal`.
- Quit the friend first, then the host, through the menu.

## Known limits

These are known gaps, not new bugs:

- **Silverfish and endermites:**
  - Silverfish don't hide in stone or burst out of infested blocks, and a hurt silverfish wakes no hidden ones. Breaking an infested block (strongholds, mountains) spawns nothing.
  - A thrown ender pearl never leaves an endermite (vanilla: 1 in 20).
  - **Silverfish trial spawners stay dark** even though silverfish live now: the trial spawner still checks the old list of mobs. In vanilla they run. Breeze trial spawners stay dark too (no breezes yet).
- **Trial spawners:**
  - Arrows from above poison or slow at level I and can't be picked up.
  - A wind charge, snowball or egg from above that hits a mob directly doesn't hurt it; its burst still pushes.
  - A mob that a spawner sends away when it turns ominous drops nothing it picked up.
  - A spawner's 30-minute cooldown pauses while nobody is in the Overworld.
  - The detection sparks always face the camera, so they tilt when seen from far above or below.
  - The ominous lingering potions' Wind Charged, Weaving, Oozing and Infested effects do nothing yet.
- **Vaults:** after a save in the middle of an ejection, the pitch keeps rising to the top (vanilla starts lower). Your luck is always 0. The extra copies of a stacked reward item are spread like dropped items.
- **Lodestones and compasses:**
  - A compass held by another player, in an item frame or on the ground keeps the plain compass's look.
  - A lodestone in a chunk that is not loaded still counts as there.
  - The game has no `/setworldspawn`, so a friend's compass can't be seen following a moved world spawn.
- **Saved fireballs and skulls** find a mob shooter only in their own dimension, and a player shooter only once that player has been in that dimension this session.
- **Items and kits:**
  - Banner patterns, shield colours, loaded crossbows and firework stars are stored but not drawn: banners show plain white, and a loaded crossbow looks empty (in illagers' hands too).
  - An unknown banner pattern id is kept (vanilla refuses it), and a `flight_duration` above 255 is refused (vanilla wraps it).
  - Leather horse armour on a horse is still brown, dyed wolf armour shows undyed, and item frames show dyed items undyed.
  - Tooltips have no rarity colours, so the ominous bottle's name is white (vanilla: yellow).
- **Effects:** the effect list and the bottle's tooltip count 20 ticks a second, so under `/tick rate` they show the time at the normal rate.
- **Particles:** the llama spit puff is still not drawn. Host and friend see the same puffs with slightly different specks.
- **Horses:** the mount is not drawn in its inventory screen, and only one player can open a mount's inventory at a time. A rider hears no block fall sound, and a mob riding a horse takes no share of its fall. Mount hearts show for horses, donkeys, mules, llamas and striders, the only mounts so far.
- **Not in this build:** thrown items hitting the new mobs, doors broken by zombies and vindicators, fireworks bursting on mobs, explorer maps and shipwreck and ocean ruin loot (track E2), darkness, sculk and the warden (track W), Margit (track F), the M1 to M3 mobs (phantoms among them), banner and crossbow drawing, swinging bells, llama spit and dyed horse armour on the horse.
- The limits at the end of [release-1-tests.md](release-1-tests.md) still apply, except the lines this build fixes:
  - Particles: all of them except the llama spit.
  - Trading: number keys, Q, double-click and drags.
  - Effects: the list beside the inventory, omen sounds and the bottle's Bad Omen line.
  - Horses: the rider's fall damage, the gallop, vehicle hearts and the dyed icons.
  - Withers: a skull loaded from a save keeps its owner.
  - Wind charges: a player flying in creative was never pushed.

## Screenshot numbering

- Name each screenshot by its number: `14.png`. When a step asks for both windows, name them `61-host.png` and `61-friend.png`.
- When a step fails, take a screenshot even if the step doesn't ask for one. Name it by its step label, e.g. `T-20.png` or `F-33-friend.png`. Say in one line what you expected and what you saw.
- For the sound steps (H-19 to H-24, F-10), write one line per step with what you heard.
- Send `latest.log` after any crash. In two windows, send both windows' logs (see Setup for where they are).
