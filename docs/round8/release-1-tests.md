# Round 8, release 1: test steps

Build: `iw4l-survival.exe` from `minecraft-survival` at 4d45e4fe, with tracks A to D of round 8. Multiplayer (track E) is not in this build. The last test was [../round7/release-2-tests.md](../round7/release-2-tests.md) on e231362.

## What is new

- End gateways send players and ender pearls to the outer islands, build return gateways, and shine their beam.
- Eyes of ender fly toward the stronghold again, one per click.
- Mining speed is vanilla's: Efficiency, Haste, Mining Fatigue, the water and off-ground penalties, instant breaks, cracks, chips and hit sounds. Blocks mined with the wrong tool drop nothing, and tools wear.
- Spawners show their mob turning in the cage, with flames and smoke. Spawn eggs change a spawner's mob.
- The server survives overload like vanilla: it catches up, logs `Can't keep up!` and skips ticks. `/tick` works.
- Haste, Mining Fatigue, Conduit Power, Levitation and Slow Falling are real effects.
- Right-click on mobs: saddles, feeding and breeding, milking, shearing and taming.
- Striders walk on lava, shiver on land, breed, carry jockeys and can be ridden and steered.
- Hoglins and zoglins charge and throw, avoid their repellents, breed and turn into zoglins outside the Nether.
- Piglins and piglin brutes: gold armour, bartering, guarding gold, hunting hoglins, turning into zombified piglins, and live bastions.
- Sleeping follows the vanilla bed rules, with the sleep screen and the night skip.
- Shulkers, shulker bullets, levitation and slow falling.
- End cities with their ships and enchanted chest loot, item frames, and the elytra in the ship's frame.
- The End sky flashes and rumbles. The dragon has breath and death particles and knockback, and the End, boss and credits music plays.
- The elytra and the glide pose are drawn on the MW2 body.
- Powder snow and freezing, lectern books with the book screen, the trapped chest signal, the totem animation, the ladder ground jump, and rain and snow streaks.
- Skating in Minecraft worlds: an on/off switch, a rebindable J key, ramps, grinds on block edges, fences and rails, and keyboard and mouse. The board is refused or picked up where vanilla refuses a mount, and guns are put away while skating.

## Setup

- Use a class with an assault rifle and Frag.
- Each section names its world and its kit. Put the kit in the `.env` next to the exe as one line, `IW4L_MINECRAFT_KIT=...`, replacing the old line.
- A kit only fills the backpack of a player in a new world. In a world you have played before, type `/clear`, then `/kit`. `/kit` reads the `.env` again, so you don't have to restart.
- Only set `IW4L_MINECRAFT_DIMENSION` and `IW4L_MINECRAFT_SEED` when a section asks for them, and remove them when it ends.
- In every new world, type `/difficulty normal` and `/gamerule keep_inventory true` first.
- Useful commands: `/gamemode`, `/tp`, `/locate structure|biome`, `/execute in minecraft:<dimension> run tp ...`, `/time set`, `/weather`, `/summon`, `/setblock`, `/fill` and `/effect`.
- `/give @s minecraft:<id> <count>` gives any item, with enchantments as `+efficiency=5`. It can't give potions; those only come from the kit.
- `/tp ~ ~ ~` prints your position.
- The log is `iw4l-artifacts\logs\latest.log`.

## A. The five failures of e231362, and /tick

Kit A, for the whole section (27 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:ender_pearl*16,minecraft:elytra,minecraft:firework_rocket*64,minecraft:end_stone*64,minecraft:golden_apple*16,minecraft:ender_eye*48,minecraft:end_portal_frame*2,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_pickaxe,minecraft:netherrack*64,minecraft:stone*64,minecraft:obsidian*16,minecraft:milk_bucket,minecraft:spyglass,minecraft:spawner*16,minecraft:zombie_spawn_egg*4,minecraft:cave_spider_spawn_egg*4,minecraft:cobblestone*64,minecraft:oak_sapling*16,minecraft:cow_spawn_egg*4,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:sand*16,minecraft:water_bucket,minecraft:furnace,minecraft:coal*16,minecraft:raw_iron*16
```

### Failure 1: the end gateway

World: `IW4L_MINECRAFT_WORLD=endtest`, the round 7 End world where the dragon is dead. Type `/clear`, then `/kit`. If you don't start in the End, type `/gamemode creative`, then `/execute in minecraft:the_end run tp @s 0 100 40`.

If `endtest` is gone, do A6 in any End world, and A1 to A5 after the dragon kill in section C.

- **A1.** `/gamemode creative` and fly to the gateway from the kill. It is a starfield cube in a bedrock frame at height 75, about 96 blocks from the centre. Purple particles drift from its open sides. Land a few blocks away and type `/gamemode survival`.
- **A2.** Throw a pearl into its side gap. The gateway flashes a short purple beam. You land on end stone on an outer island more than 900 blocks out (a loading screen may show first). A new gateway in a bedrock frame floats about 10 blocks above the ground. **Screenshot 1.**
- **A3.** Pillar up with end stone to the new gateway, go prone and crawl into its side gap. You come back next to the first gateway, on the highest end stone nearby. **Screenshot 2.**
- **A4.** Straight away, while the beam still shows, throw a pearl through the gateway you came out of. The pearl flies through and lands behind it like a normal pearl.
- **A5.** Put on the elytra, glide off the island's edge with a rocket and fly into the first gateway's side gap. You come out at the same outer island, still gliding at the same speed and facing the same way. **Screenshot 3.**
- **A6.** Go back through the return gateway. About 20 blocks east of 0, 0, type `/setblock ~2 ~ ~ minecraft:end_gateway`. A tall magenta beam grows up to the sky and shrinks again over 10 seconds. **Screenshot 4.** Walk in: you land on the outer islands east of the centre, under a new return gateway.
- **A7.** Wait 2 minutes near a gateway: it flashes its beam by itself.
- **A8.** Quit and start again, then use the first gateway again. You arrive under the same return gateway, and no second one is built. Quit and keep this `latest.log`: it has a `Minecraft end gateway at ... leads to ...` line.

### Failure 2: the eye of ender

World: `IW4L_MINECRAFT_WORLD=r8a` (a new world) and `IW4L_MINECRAFT_SEED=4986579828511054607`. Kit A arrives with the new world. Stay in this world without restarting until A46.

- **A9.** Start by day. Type `/gamemode creative`, then `/tp @s -300 120 -1700`, land, and type `/gamemode survival`. `/locate structure minecraft:stronghold` answers about [-496, ~, -1936].
- **A10.** Face north-west, look level and right-click once with an eye. You hear the launch sound, your hand swings and the stack loses one eye. The eye rises from your chest and floats about 12 blocks toward the stronghold with a purple trail. **Screenshot 5** while it flies, with the `/locate` answer still in the chat.
- **A11.** After about 4 seconds the eye drops as an item, or breaks into crumbs and a purple ring. Out of 10 throws, about 8 drop and 2 break.
- **A12.** Turn your back to the stronghold and throw: the eye flies behind you, toward the stronghold.
- **A13.** Hold right-click for 3 seconds: only one eye flies. Let go and click again: another one flies.
- **A14.** `/gamemode creative`, `/tp @s -496 120 -1936`, then throw. The eye sinks straight down into the ground; vanilla does this above a stronghold. **Screenshot 6.**
- **A15.** Place an end portal frame. In `/gamemode adventure` you can still throw, but right-clicking the frame with an eye does nothing. In `/gamemode survival` the eye goes into the frame, and right-clicking the filled frame does nothing.
- **A16.** `/execute in minecraft:the_nether run tp ~ 128 ~` puts you on the Nether roof. Right-click with an eye, then hold the button: no sound, no swing, and the stack stays. **Screenshot 7.**

### Failures 4 and 3: the blaze spawner and netherrack

- **A17.** Still in the Nether, type `/locate structure minecraft:fortress`, then `/gamemode creative` and `/tp <x> 75 <z>`. Find the blaze spawner: a cage on a small platform of nether bricks with stairs.
- **A18.** A small blaze sits in the cage, leaning forward and turning fast, with flames and dark smoke rising around it. **Screenshot 8** (compare with screenshot 4 of e231362).
- **A19.** Walk more than 16 blocks away and look with the spyglass: the blaze and the flames stop. Walk back: they start again.
- **A20.** `/gamemode survival`. Click netherrack with the Efficiency V pickaxe: it breaks the moment the button goes down, with no crack. Hold the button and sweep along a wall: about 20 blocks a second, with no pause. **Screenshot 9** of the tunnel.
- **A21.** With the plain diamond pickaxe, netherrack shows a short crack flash. While you hold the button, there is a short beat before each next block.
- **A22.** With a bare hand (an empty slot), netherrack takes about 2 seconds and drops nothing.

### Failure 3: mining in the Overworld

- **A23.** `/gamemode creative`, `/execute in minecraft:overworld run tp @s -300 120 -1700`, land, and type `/gamemode survival`.
- **A24.** Place stone and mine it by hand. It takes 7.5 seconds: the crack climbs through 10 stages, the hit sound ticks and chips fly. Nothing drops. Let go halfway and click again: the crack starts over. **Screenshot 10** of a half-cracked block.
- **A25.** Stone with the Efficiency V pickaxe breaks in 2 ticks. After `/effect give @s minecraft:haste 60 1` (the chat answers `Applied effect Haste ...`) it breaks at once. After `/effect give @s minecraft:mining_fatigue 60 2` it takes about 2.5 seconds. Drink the milk: 2 ticks again.
- **A26.** Obsidian takes about 2.3 seconds with the Efficiency V pickaxe and drops. With the plain diamond pickaxe it takes about 9.4 seconds.
- **A27.** Type `/fill ~2 ~-2 ~-1 ~4 ~-1 ~1 minecraft:water`, place a stone on the pool's floor and dive in.
  - Standing on the bottom, the Efficiency V pickaxe mines it in about 0.35 seconds.
  - Swimming without touching the bottom, it takes about 1.7 seconds.
  - Wearing `/give @s minecraft:diamond_helmet+aqua_affinity`, swimming is back to about 0.35 seconds.
- **A28.** Mine 10 stone with the plain diamond pickaxe and hover over it: the durability bar shows a little wear.
- **A29.** In `/gamemode creative`, a click breaks any block at once, and holding the button breaks one every 6 ticks. Go back with `/gamemode survival`.

### Failure 4: placed spawners

- **A30.** Place a spawner: an empty cage without flames. Use the zombie egg on it. A small zombie leans forward and turns fast inside, flames and smoke rise, and the egg count drops by one. **Screenshot 11.**
- **A31.** Use the cave spider egg on a second spawner: a smaller blue-grey spider turns inside.
- **A32.** Optional: `/give` yourself skeleton, spider, enderman, ghast and pig spawn eggs and fill a row of spawners. Each mob fits its cage: the enderman and spider are shrunk and the ghast is tiny.
- **A33.** `/gamerule spawner_blocks_work false` and use an egg on an empty spawner: nothing changes and you keep the egg. Set the rule back with `/gamerule spawner_blocks_work true`.
- **A34.** Break a spawner with the pickaxe: the mob disappears with the cage.

### Failure 5: overload, and /tick

- **A35.** On grass by day, plant 4 saplings, use 2 cow eggs and drop a few cobblestone (Q). The cobblestone lands and can be picked up.
- **A36.** `/gamerule random_tick_speed 20000`. The Minecraft world slows down: mobs and items move in jerky steps and the saplings grow almost at once. MW2 movement and aiming stay smooth. **Screenshot 12.**
- **A37.** After 5 seconds, type `/tick query`. It shows `The game is running, but can't keep up with the target tick rate`, then `Target tick rate: 20.0 per second.` with an average above 50 ms, then a line ending in `Sample: 100`. **Screenshot 13.**
- **A38.** `/gamerule random_tick_speed 3`. Things may race for a moment, then they move normally within a second or two. A dropped item falls and can be picked up; in e231362 everything froze here. About 10 seconds later, `/tick query` says `The game is running normally`. **Screenshot 14.**
- **A39.** `/tick freeze` answers `The game is frozen`.
  - Dropped cobblestone hangs in the air and the cows stand still.
  - Lit TNT flashes but doesn't explode, sand placed in the air stays there, and poured water doesn't spread.
  - The sun stops.
  - You can still walk, jump and shoot, and walking over a cobblestone on the ground picks it up.

  **Screenshot 15.**
- **A40.** `/tick step` answers `Stepping 1 tick(s)`. `/tick step 5s` answers `Stepping 100 tick(s)`: the TNT explodes, the items fall and the water spreads, then everything stops again. `/tick step stop` answers in red `No tick step in progress`. **Screenshot 16.**
- **A41.** `/tick unfreeze` answers `The game is running normally`. Then `/tick step` answers in red `Unable to step the game - the game must be frozen first`.
- **A42.** Put coal and raw iron in the furnace. `/tick rate 5` answers `Set the target tick rate to 5.0 per second`. Mobs, items and the furnace arrow run at a quarter speed, and MW2 at normal speed. `/tick query` shows `(Target: 200.0ms)`. **Screenshot 17.** Then type `/tick rate 20`.
- **A43.** `/tick rate 0` answers in red `Float must not be less than 1.0: found 0.0`, followed by the grey `tick rate 0<--[HERE]` line.
- **A44.** `/tick sprint 1d` answers `The game is sprinting`. A day and a night pass in one to three minutes, and MW2 stays smooth. Then a grey italic `[Server: Sprint completed with ... ticks per second, or ... ms per tick]` appears. **Screenshot 18.**
- **A45.** Type `/tick sprint 3d`, and a second later `/tick sprint stop`: the grey report, then `Interrupted the current tick sprint`. A second `/tick sprint stop` answers in red `No tick sprint in progress`.
- **A46.** Press Tab after `/tick `, after `/tick sprint ` and after `/tick step `. The lists are `freeze query rate sprint step unfreeze`, `1d 3d 60s stop` and `1s 1t stop`. **Screenshot 19.**
- **A47.** `/gamerule random_tick_speed 20000`, wait 30 seconds, and quit through the menu: the game closes within a few seconds. Before you start the game again, copy `latest.log` and send it. It should have:
  - a `Can't keep up! Is the server overloaded?` line;
  - two `Sprint completed with` lines;
  - a `Minecraft stronghold rings: 128 worked out in` line;
  - a `Minecraft mining: 470 blocks need the right tool to drop` line;
  - a `Minecraft eye of ender:` line for each throw, with "no structure" only once for the held button in the Nether.
- **A48.** Start `r8a` again and type `/gamerule random_tick_speed 3`. The spawners still show their mobs. Then remove the seed line from the `.env`.

## B. The Nether and sleeping

World: `IW4L_MINECRAFT_WORLD=r8b` (a new world) and `IW4L_MINECRAFT_DIMENSION=the_nether`. You start in a portal built for you.

Kit B (27 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*8,minecraft:saddle,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:warped_fungus*32,minecraft:hoglin_spawn_egg*16,minecraft:zoglin_spawn_egg*4,minecraft:crimson_fungus*64,minecraft:piglin_spawn_egg*16,minecraft:piglin_brute_spawn_egg*4,minecraft:zombified_piglin_spawn_egg*4,minecraft:gold_ingot*64,minecraft:gold_nugget*32,minecraft:golden_boots,minecraft:gold_block*8,minecraft:chest*2,minecraft:diamond_sword,minecraft:shield,minecraft:diamond_chestplate,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:lava_bucket,minecraft:red_bed,minecraft:straw_bed,minecraft:splash_potion@slow_falling,minecraft:cooked_beef*32,minecraft:netherrack*64
```

### Striders and riding

- **B1.** `/gamemode creative`, fly down to the lava sea (height 31) and build a netherrack pier one block above it. Then type `/gamemode survival`.
- **B2.** Use a strider egg on the lava. A red strider with six swaying bristles stands on the surface, and its legs swing as it walks. **Screenshot 20.**
- **B3.** Use an egg on netherrack 5 blocks from the lava. It turns pale purple and shivers, then walks into the lava within about 20 seconds and turns red. **Screenshot 21.**
- **B4.** Hold warped fungus: striders come to you. Feed two adults: hearts, and a baby with three bristles appears. A saddle goes onto an adult with its sound and shows on its back. A baby takes no saddle.
- **B5.** Right-click the saddled strider with an empty hand. You sit about 2.7 blocks above its feet, and the view bobs as it wanders. You can look around and shoot.
- **B6.** While riding, look straight down and hit or shoot your strider: it isn't hurt. Shoot another strider: that one is.
- **B7.** Select the warped fungus on a stick: the strider walks where you look, at about 4 blocks a second. Right-click: it speeds up to about twice that, and the stick loses one durability. A second right-click during the boost does nothing. **Screenshot 22.**
- **B8.** Select another slot: the strider wanders on its own again. Steer it onto netherrack: it slows down, turns pale and shivers.
- **B9.** Crouch next to land: you step off forward onto the block you look at. Crouch over open lava: you land on top of the strider, never in the lava.
- **B10.** Copy this command into the chat:

  ```
  /summon minecraft:strider ~ ~1 ~ {Passengers:[{id:"minecraft:zombified_piglin",equipment:{mainhand:{id:"minecraft:warped_fungus_on_a_stick",count:1}}}],equipment:{saddle:{id:"minecraft:saddle",count:1}}}
  ```

  A zombified piglin sits on the saddled strider with its legs forward and steers it. **Screenshot 23.**
- **B11.** `/summon minecraft:strider ~ ~1 ~ {Passengers:[{id:"minecraft:strider",Age:-24000}]}`: a baby strider rides an adult.
- **B12.** Kill a saddled strider with the sword: 2 to 5 string and the saddle drop. Kill one that carries a rider: the rider drops down where it sat.
- **B13.** Ride a strider, then save and quit. Start again: you sit on the same strider, and there is no second one under you.

### Hoglins and zoglins

- **B14.** Away from the lava, make a floor with `/fill ~-10 ~ ~-10 ~10 ~5 ~10 minecraft:air`, then `/fill ~-10 ~-1 ~-10 ~10 ~-1 ~10 minecraft:netherrack`. Put on the diamond chestplate.
- **B15.** Use a hoglin egg and back off 10 blocks. A big pink tusked boar grunts, charges and headbutts you. Each hit takes 1.5 to 4 hearts and throws you up and away. **Screenshot 24** in mid-air. With the shield raised you take no damage, but you are still thrown.
- **B16.** `/give @s minecraft:respawn_anchor 1` and place it next to the angry hoglins. Within a second they snort and walk away. Break it: they stay calm for about 10 seconds, then charge again. **Screenshot 25.**
- **B17.** In `/gamemode creative`, feed two adults crimson fungus: hearts, and a piglet appears within about 5 seconds. Back in `/gamemode survival`, the piglet nibbles you for half a heart but never throws you.
- **B18.** Kill an adult: 2 to 4 raw porkchops, and sometimes leather. Light the netherrack under another one with flint and steel and kill it while it burns: the porkchops come out cooked.
- **B19.** Use a zoglin egg next to a hoglin. The grey, rotting zoglin attacks the hoglin, and the hoglin fights back. Pour lava under the zoglin: it doesn't burn.
- **B20.** `/locate biome minecraft:crimson_forest` and go there in creative: hoglins stand in groups of up to four on the red nylium. **Screenshot 26.**

### Piglins, brutes and bartering

Go back to the netherrack floor in survival.

- **B21.** Take off the golden boots. Use 2 piglin eggs and back off 10 blocks. Sword piglins run in and hit. Crossbow piglins step back, load with a sound and shoot. **Screenshot 27.**
- **B22.** Kill them and put on the golden boots. Use 4 piglin eggs: they wander and ignore you. Hold a gold ingot near them: they stare at you and grunt jealously.
- **B23.** Drop one gold ingot (Q). A piglin picks it up, admires it in its left hand for about 6 seconds, then throws an item at you. **Screenshot 28.**
- **B24.** Right-click piglins with gold ingots about 20 times. Each trade uses one ingot and gives items from the barter list: obsidian, crying obsidian, quartz, string, gravel, blackstone, soul sand, spectral arrows, ender pearls, fire charges, potions, and rarely a soul speed book or soul speed boots. **Screenshot 29** of the inventory.
- **B25.** Drop the gold nuggets: one piglin takes the whole stack. Kill it: the nuggets drop.
- **B26.** Hit a piglin while it admires: it stops at once and throws nothing. It and the adults near it attack you. Back off for 30 seconds and they calm down.
- **B27.** Near calm piglins, place a gold block and mine it: every piglin within about 16 blocks attacks you, even the ones behind you.
- **B28.** Behind a 3-high wall where they can't see you, open a chest: nothing happens. Open it where they can see you: they attack.
- **B29.** `/give @s minecraft:soul_torch 4` and place one next to calm piglins: they back away about 8 blocks. Use a zombified piglin egg near them: they run from it for a few seconds.
- **B30.** Use 2 brute eggs. Brutes hold golden axes and attack you even though you wear gold. Raise the shield as one swings: it is knocked down with a break sound and can't be raised for 5 seconds (a cooldown on the slot). **Screenshot 30.**
- **B31.** Use 3 piglin eggs, then a hoglin egg 8 blocks away. The piglins kill the hoglin and sometimes dance afterwards. **Screenshot 31.**
- **B32.** `/locate structure minecraft:bastion_remnant` and `/tp` there. Its piglins and brutes walk about, and its hoglin stable has hoglins. **Screenshot 32.**

### Spawners and slow falling

- **B33.** `/give @s minecraft:spawner 5`. Place them and use a strider, hoglin, zoglin, piglin and piglin brute egg, one on each. Each mob turns in its cage, and the piglins hold nothing. **Screenshot 33.**
- **B34.** Build a 2 x 2 netherrack platform 12 blocks up. Use a hoglin and a piglin egg on it, throw the splash potion of slow falling on them, and punch them off. They drift down and land unhurt. A hoglin without the potion falls fast and is hurt.

### Into the Overworld

- **B35.** Build a 4 x 5 obsidian portal, light it and go through. In the Overworld, use a piglin, a brute and a hoglin egg. They shake for 15 seconds (**Screenshot 34**). Then the piglin and the brute become zombified piglins that still hold their gold weapons and armour, and the hoglin becomes a zoglin (**Screenshot 35**).
- **B36.** `/summon minecraft:hoglin ~ ~ ~ {IsImmuneToZombification:1b}`: it never shakes.
- **B37.** `/give @s minecraft:cow_spawn_egg 2`, `/give @s minecraft:wheat 8` and `/give @s minecraft:bucket 1`. Use both eggs and feed the cows wheat: hearts, and a calf appears. Right-click an adult with the bucket: you get milk and hear the milking sound.

### Sleeping

On open ground in the Overworld, survival.

- **B38.** By day, place the red bed and right-click it. "You can sleep only at night or during thunderstorms" shows above the hotbar, and "Respawn point set" shows in the chat.
- **B39.** `/time set 12400`, wait about 7 seconds, and right-click the bed while holding the sword.
  - The view lies on the pillow and looks down the bed, with no gun or sword in view.
  - The screen darkens to dark blue over 5 seconds.
  - The chat bar and a "Leave Bed" button show.

  **Screenshot 36.**
- **B40.** Press F5: your MW2 body lies on its back on the bed, head on the pillow. **Screenshot 37.** Press F5 until you are back in first person.
- **B41.** Stay in bed. About 5 seconds later the sky jumps to sunrise, the screen fades in, and you stand beside the bed. `/time query daytime` gives a few ticks past 0.
- **B42.** `/time set night`, lie down, and click "Leave Bed" before the screen is dark: you stand up, and it is still night. Lie down again and press Esc: the same happens.
- **B43.** `/give @s minecraft:zombie_spawn_egg 2`. Use one 6 blocks from the bed, behind a netherrack wall, and right-click the bed: "You may not rest now; there are monsters nearby". **Screenshot 38.** Kill the zombie.
- **B44.** Right-click the bed from 4 blocks away: "You may not rest now; the bed is too far away". Put a block on top of the pillow end: "This bed is obstructed". Remove the block.
- **B45.** `/gamerule players_sleeping_percentage 101`, `/time set night`, and lie down: "No amount of rest can pass this night", and nothing happens. Type `/tp ~ ~ ~10` in the bed chat: the dark screen goes at once and you stand 10 blocks away. Set the rule back with `/gamerule players_sleeping_percentage 100`.
- **B46.** `/time set night`, lie down, and type `/effect give @s minecraft:poison 5` in the bed chat: you wake at the first poison hit.
- **B47.** `/time set noon`, `/weather thunder`, wait 5 seconds and lie down: you can sleep, and the storm clears at the skip.
- **B48.** `/time set night`, place the straw bed and sleep in it: the view is lower than on the red bed. In the morning the straw bed breaks, and it never said "Respawn point set".
- **B49.** Remove the dimension line from the `.env`.

## C. The End, and the small things

### The End

World: `IW4L_MINECRAFT_WORLD=r8end` (a new world) and `IW4L_MINECRAFT_DIMENSION=the_end`. Start the game with an internet connection, so it can fetch the End music.

Kit C1 (26 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword+sharpness=5,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:end_crystal*4,minecraft:lingering_potion@regeneration,minecraft:shulker_spawn_egg*16,minecraft:zombie_spawn_egg*4,minecraft:end_stone*64,minecraft:purpur_block*64,minecraft:shield,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:potion@slow_falling,minecraft:splash_potion@slow_falling,minecraft:elytra,minecraft:elytra+unbreaking=3,minecraft:firework_rocket*64,minecraft:diamond_pickaxe+efficiency=5,minecraft:cobblestone*64,minecraft:torch*64,minecraft:bogged_spawn_egg*2,minecraft:stray_spawn_egg*2,minecraft:spawner*2
```

**The dragon fight**

- **C1.** You start on the obsidian platform. When the pink "Ender Dragon" bar shows, the boss music starts with it. Put on the diamond armour.
- **C2.** Stand still for a minute and a half; a new world's first 30 seconds never flash. About every 30 seconds the end stone turns faintly purple for 5 to 19 seconds. About 1.5 seconds after each glow starts, a deep rumble comes from one side, and it stays on that side when you turn. **Screenshot 39** at the brightest moment.
- **C3.** On the island, let the dragon spit at you. Its fireball leaves a trail of purple puffs, and the burst sprays purple puffs low over the ground. The cloud it leaves is made of drifting purple puffs, without round sparkles. **Screenshot 40.** Throw the lingering potion: its cloud still has round pink sparkles.
- **C4.** Shoot the flying dragon from the side with the rifle: it lurches away from you as the bursts land. Hit its head with the sword when it swoops low: it jolts away.
- **C5.** Optional: build a wall for it to break.
  - Type `/gamemode creative` and `/tp 0 100 40`, and land.
  - Type `/fill ~-3 ~ ~2 ~2 ~19 ~2 minecraft:cobblestone` (a wall 6 wide and 20 high), then `/gamemode survival`.
  - Where the dragon tears through the wall, small explosion flashes pop.
- **C6.** Shoot all ten crystals; shoot the bars of the caged ones first. When the dragon comes in to land on the portal, purple breath streams from its mouth down to the ground. **Screenshot 41.** When it flames, four quick purple bursts pour from its mouth first. **Screenshot 42.** Shooting it while it sits doesn't move it.
- **C7.** Kill it. While it rises under the white rays, explosion flashes flicker around its body, and in its last second big clusters burst. **Screenshot 43.** The bar goes and the boss music stops. Within 2.5 minutes the calmer End music starts.
- **C8.** Stay on the island and look around the sky. About every 30 seconds, a soft purple glow swells and fades somewhere between straight up and the horizon. The island glows with it, and the rumble comes from that side. **Screenshot 44** with the glow in view.

**Shulkers and levitation**

On the island's end stone, survival.

- **C9.** `/setblock ~ ~ ~2 minecraft:spawner`, then use a shulker egg on it: a small, closed purple shulker turns in the cage, tilted forward. **Screenshot 45.**
- **C10.** Use a shulker egg 10 blocks away and walk 20 blocks off. A purple shulker sits closed on the block and lifts its lid a little every few seconds, with its sounds. Then type `/summon minecraft:shulker ~3 ~ ~ {Color:14b}` and `/summon minecraft:shulker ~5 ~ ~ {Color:0b}`: a red one and a white one appear.
- **C11.** Build a purpur wall and a ceiling. Use eggs on the wall's face and under the ceiling: the shulkers cling and hang there, with the lid away from the block. **Screenshot 46.**
- **C12.** Take your armour off. Stand 10 blocks from a shulker where it can see you. It opens, turns its head to you, and every 1 to 5.5 seconds fires a glowing, spinning bullet. The bullet moves in right-angled steps and leaves a white sparkle trail. **Screenshot 47.**
- **C13.** Let a bullet hit you: 2 hearts, and you float up for 10 seconds, about 9 blocks, and can still strafe. Then you fall and lose about 3 hearts. **Screenshot 48** in mid-air. Get hit again and drink milk while you float: you fall at once.
- **C14.** Raise the shield toward the shulker: bullets thud on it and you don't float. Shoot or punch a bullet: it pops with red and white sparks. A bullet that hits the floor puffs and pops.
- **C15.** Shoot a shulker while it is closed (one that has opened and closed once): little damage and a hollow sound. Shoot it while it is open: full damage. Below half health it now and then teleports, sliding to its new block with portal particles. Mine the block under one: it teleports.
- **C16.** Kill 8 shulkers with the sword: about half drop a shulker shell, and all drop experience.
- **C17.** Use a zombie egg between you and a shulker: a bullet that hits the zombie lifts it. Throw the splash slow falling potion at zombies at the edge of a 6-block tower and push one off: it sinks gently and lands unhurt.
- **C18.** Pillar 20 blocks up with purpur, drink the slow falling potion and jump off: you sink gently and take no damage. While you are slow falling, type `/effect give @s minecraft:levitation 3 0`: you rise.
- **C19.** `/effect give @s minecraft:levitation 5 4`: you rise fast, about 4.5 blocks a second. Then `/effect clear @s`.
- **C20.** Use a bogged egg 12 blocks away and let it shoot you: your hearts turn green for a few seconds. A stray's arrow slows you.
- **C21.** `/difficulty peaceful`: the shulkers close and stop firing. Then `/difficulty normal`.
- **C22.** Quit with shulkers of three colours on a floor, a wall and a ceiling. Start again: they keep their colour and their side.

**End cities and the elytra frame**

- **C23.** `/locate structure minecraft:end_city`, `/gamemode creative`, and `/tp <x> 120 <z>`. A purple city of towers and bridges stands on an outer island, with shulkers in its rooms and on its towers. Its banners are plain magenta (a known gap). **Screenshot 49.**
- **C24.** Find the city's ship: a long purple boat with a black dragon head. If there is none, fly 300 blocks further and type `/locate` again. **Screenshot 50.** In the lower deck, an item frame between two chests holds an elytra. **Screenshot 51.**
- **C25.** `/gamemode survival`. Right-click the frame: the elytra turns 45 degrees with a click. Left-click it: the elytra pops out, and the empty frame stays. Pick the elytra up.
- **C26.** Open the chests in the ship and the towers. They hold diamonds, iron or gold, often an enchanted diamond or iron tool or armour piece, and sometimes a spire armour trim. Hover over the gear to see its enchantments. No two chests hold the same items. **Screenshot 52** with a tooltip.

**The elytra on the body**

Use the city's towers for height.

- **C27.** Put the plain elytra in the chest slot and press F5: two grey wings fold on the back, from the shoulders to the knees. **Screenshot 53.** Press F5 again: the wing edges show behind the arms. In the inventory, the character wears the wings.
- **C28.** Swap in the Unbreaking III elytra: the wings shimmer. Put the plain one back. Crouch: the wings open a little and tilt back. Stand up: they fold back in about a quarter second.
- **C29.** In the back view, jump off a tower and press jump. Over half a second the body tips forward along your look and the wings spread. Look level: the body lies flat (**Screenshot 54**). Dive: it points head first down, with narrower wings (**Screenshot 55**). Look up: it angles up.
- **C30.** Flick the mouse sharply left, then right: the body rolls into each turn, then levels out. **Screenshot 56** mid-roll.
- **C31.** In the front view while gliding, the head looks ahead and the legs trail straight behind. **Screenshot 57.**
- **C32.** Hold the sword with a torch in the offhand: both tip with the body. An MW2 gun still fires while you glide. A rocket keeps the body lying along the faster flight.
- **C33.** Type `/effect give @s minecraft:levitation 3 0` while gliding: the glide stops and you rise.
- **C34.** Glide again and quit through the menu while in the air. Start again: you are still gliding at about the same speed.
- **C35.** Land: the body is upright at once, and the wings fold. If anything floats off the back, sinks into it or turns the wrong way, say which and by about how much.

**The credits**

- **C36.** `/gamemode creative`, `/tp 0 80 30`, and walk into the exit portal: the credits play their own music. Press Esc: the music stops, and no music plays in the Overworld.
- **C37.** Optional: `/execute in minecraft:the_end run tp @s 0 80 30` and put the 4 crystals on the exit portal's rim, one on each side. When the new dragon's bar shows, the boss music cuts in at once.
- **C38.** In `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3\resourcepacks\local\minecraft-26.3\assets\minecraft\sounds\music\game\end\` there are `alpha.ogg`, `boss.ogg` and `the_end.ogg`, about 7, 4 and 11 MB. `latest.log` has `Minecraft End music ready`. Remove the dimension line from the `.env`.

### The small things

World: `IW4L_MINECRAFT_WORLD=r8misc` (a new world, Overworld), on open grass by day.

Kit C2 (27 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:item_frame*16,minecraft:glow_item_frame*8,minecraft:compass,minecraft:carrot*16,minecraft:oak_planks*64,minecraft:comparator*4,minecraft:redstone*64,minecraft:redstone_lamp*8,minecraft:snowball*16,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:bucket,minecraft:leather_boots,minecraft:leather_helmet,minecraft:netherrack*16,minecraft:cobblestone*64,minecraft:trapped_chest*4,minecraft:lectern*2,minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}],minecraft:writable_book,minecraft:totem_of_undying,minecraft:totem_of_undying,minecraft:ladder*32,minecraft:torch*16
```

**Item frames**

- **C39.** Place an oak planks block. Right-click its four sides, its top and its bottom with item frames. Each frame lies flat on its face with the place sound, one per click, and a side that has a frame takes no second one. **Screenshot 58.**
- **C40.** Right-click the ground with a glow item frame: it lies flat, with a higher chime.
- **C41.** Put the compass in a frame on a wall and right-click it 8 times: it turns 45 degrees each time and comes back round. Left-click: the compass pops out and the frame stays. Left-click again: the frame breaks and drops.
- **C42.** Put a carrot in a frame and shoot it once with the MW2 gun: the carrot pops out. Shoot again: the frame breaks.
- **C43.** Throw a snowball at a filled frame: the item pops out. A second snowball breaks the empty frame.
- **C44.** Place a row from west to east: planks, a comparator facing away from them, and 8 redstone dust. Hang a frame on the west side of the planks and put a carrot in: one dust lights. Each turn lights one more, up to 8. **Screenshot 59.** Take the carrot out: all the dust goes dark.
- **C45.** Under a planks roof, type `/time set midnight`: a carrot in a glow frame stays bright, and one in a normal frame goes dark. Then `/time set day`.
- **C46.** Hang three filled frames around a TNT block and light it: all three break and drop the frame and the item.
- **C47.** Mine the planks block from C39: within 5 seconds all six frames drop. In creative, left-click a filled frame: the item vanishes and nothing drops. Go back to survival.

**Powder snow and freezing**

- **C48.** Dig a pit 1 x 1 and 3 deep, and fill it with the three powder snow buckets. Scoop the top block back up with an empty bucket, and pour it back.
- **C49.** Walk in. You sink slowly, white flakes rise as you move, and the screen turns into pale blue fog. **Screenshot 60** looking out.
- **C50.** Stay with your head in the snow. Over 7 seconds a frosty border fills the screen, the hearts turn blue-white, and you take half a heart every 2 seconds with a crackle. **Screenshot 61.** Press F5: the border stays.
- **C51.** Jumping doesn't get you out of 3 deep. Scoop the blocks up, pillar out with cobblestone, and pour them back. Outside, the frost fades over 3.5 seconds, and you walk slower until it has gone.
- **C52.** Stay in until the frost kills you: the chat says "<name> froze to death". Type `/gamerule freeze_damage false` and go back in: no damage. Then `/gamerule freeze_damage true`.
- **C53.** Put on the leather boots: you walk on the powder snow. **Screenshot 62.** Crouch: you sink in. Hold jump: you climb out.
- **C54.** Wear only the leather helmet (no boots) and stay inside: no frost border and no damage.
- **C55.** With no boots on, `/tp ~ ~12 ~` above the pit: you land on the snow with a fall sound and no damage, then sink in.
- **C56.** Light netherrack, stand in the fire, then jump into the snow: the fire goes out with a hiss, and that snow block melts. MW2 bullets pass through powder snow.

**Ladder jump**

- **C57.** Put a ladder on a wall 6 high, stand at its foot and tap jump. You hop up about half a block and slide back down; before, a tap lifted you only a hand's width. **Screenshot 63** mid-hop. Hold jump: a quick hop, then the steady climb.

**Trapped chest**

- **C58.** Place a trapped chest, a redstone lamp beside it, and 15 redstone dust from its other side. Open it: the lamp lights, and exactly one dust is lit. **Screenshot 64.** Close it: both go off.
- **C59.** Put a second trapped chest on a cobblestone block, with a lamp under the cobblestone. Opening the chest lights that lamp. A comparator facing out of a trapped chest with one stack inside lights 1 dust.

**Books and lecterns**

- **C60.** Type `/give @s minecraft:written_book[written_book_content={author:"Owner"}]`. A red line starts with "Malformed 'minecraft:written_book_content' component" and ends with "No key title", and nothing is given.
- **C61.** Place a lectern, a comparator facing out of its side into 15 dust, and a lamp beside the lectern. Right-click the lectern with the written book: it lies open with the book sound, and 1 dust lights.
- **C62.** Right-click the lectern. The book screen shows "Page one text.", "Page 1 of 3", a forward arrow, and the Done and Take Book buttons. **Screenshot 65.**
- **C63.** Click the arrow: a page sound, page 2, the lamp flashes, and 8 dust light. **Screenshot 66.** Click it again: page 3, the arrow is gone, and 15 dust light. Page Up goes back. E leaves the screen open, and Esc closes it.
- **C64.** Open the book again and click Take Book: the book is back in your inventory, the lectern is empty, and the dust is dark.
- **C65.** Put the book and quill on the second lectern and open it: "Page 1 of 1" and an empty page, and its comparator lights 15 dust. In `/gamemode adventure` there is only one wide Done button. Walk about 8 blocks away: the screen closes. Go back to survival.
- **C66.** Put the written book back, then quit and start again: it is still on the lectern. Break that lectern: the book pops out on top.

**Totem**

- **C67.** Hold a totem in the offhand with an MW2 gun selected, and type `/tp ~ ~40 ~`. On landing, a big totem flies out from the screen centre, spins and drops back over 2 seconds, and green and yellow glitter bursts around you. **Screenshot 67** mid-spin.
- **C68.** Put the second totem in the offhand, press F5 and repeat: the totem shows, and the glitter surrounds the body. Then `/give @s minecraft:totem_of_undying 1`, press F1 to hide the HUD, and repeat: only the glitter shows.

**Rain and snow**

- **C69.** On plains, type `/weather rain`. Streaks fall all around you up to about 10 blocks, fainter at the edge, and they lean in when you look up. **Screenshot 68.**
- **C70.** Under a 5 x 5 roof 4 blocks up, no streaks fall inside. After `/time set midnight` the streaks are dark, and brighter near a torch.
- **C71.** `/locate biome minecraft:snowy_plains`, then `/tp` there: slow white flakes drift down instead. **Screenshot 69.** After `/weather clear` they fade out over a few seconds.

## D. Skating

World: `IW4L_MINECRAFT_WORLD=r8skate` (a new world, Overworld), on flat ground by day.

You need the extracted Skate 3 `default.xex` with its `data` folder. Use a controller except in D27 and D28, and a class with a gun.

Kit D (20 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:oak_stairs*64,minecraft:stone_slab*64,minecraft:cobblestone_slab*64,minecraft:white_carpet*64,minecraft:snow*64,minecraft:oak_fence*64,minecraft:cobblestone_wall*64,minecraft:iron_bars*64,minecraft:rail*64,minecraft:powered_rail*16,minecraft:hay_block*16,minecraft:iron_helmet,minecraft:ender_pearl*16,minecraft:strider_spawn_egg*2,minecraft:saddle,minecraft:red_bed,minecraft:elytra,minecraft:firework_rocket*16
```

**The switch and the key**

- **D1.** Leave `IW4L_SKATE=off` in the `.env` if it is there. Start the game. Options > Controls > Minecraft shows "Skating: Disabled" and "Skateboard Down/Up: J" under the six keys. **Screenshot 70.**
- **D2.** In the world, press J: only "Skate unavailable: Skating is off (Options > Controls > Minecraft)" shows, for about 3 seconds.
- **D3.** Set Skating to Enabled and quit. `iw4l-artifacts\settings.cfg` starts with `// IW4L user settings v5` and has `skating=true` and `bind j mc_skate`.
- **D4.** Start again. Even with `IW4L_SKATE=off`, the game asks "Do you have Skate 3?", with a line about the Skating switch. Answer Yes, pick `default.xex` and let it convert. The `.env` now has `IW4L_SKATE_ASSETS=...` and no `IW4L_SKATE=off`. (If you answer No, J says "Skate 3 data is not set up: restart to select your default.xex", and the question comes back on every start until Skating is Disabled.)
- **D5.** Put on the iron helmet and select a cobblestone. Press J. The board goes down and the overlay ends in "J: pick up". The helmet shows, and nothing is in the skater's hands. **Screenshot 71.** Press J again: you stand where the skater was, on the same slot. With a gun selected, the same gun comes back.
- **D6.** On the board, pull both triggers (or click both mouse buttons) at a block: nothing is mined, placed or shot. E opens the inventory while the skater coasts. Q drops the selected item. Rolling makes no footstep sounds.

**Ramps**

- **D7.** Build a 5-high staircase of oak stairs, each stair on a cobblestone column one block higher than the last, all facing the same way. Push into it: the board rolls up without a bump and rides down again. **Screenshot 72** halfway up.
- **D8.** Ride fast over a single oak stair on flat ground: you get air off it, like a kicker.
- **D9.** Ride up a stone slab on the ground. Then ride up a staircase that rises half a block per block: a cobblestone slab, a cobblestone, a cobblestone with a slab on top, and so on.
- **D10.** Ride over a row of white carpet, and over snow 3 layers deep (right-click snow onto one spot 3 times): no bumps.

**Grinds**

- **D11.** A one-block cobblestone ledge stops the board. Ollie up and grind along its edge. **Screenshot 73.** A slab on the ground doesn't grind, not even at its corners.
- **D12.** On a cobblestone wall one block wide and two high, you can grind either edge, but not the middle.
- **D13.** Build a 10-block oak fence with a stair in front of it as a kicker. Land on the fence and grind along its middle. **Screenshot 74.** Do the same on a line of cobblestone wall and a line of iron bars.
- **D14.** Ride the side edge of the D7 staircase: it grinds like a hubba on both sides. Riding up the middle never snaps you into a grind.
- **D15.** Lay 10 straight rails, an ascending rail onto a cobblestone, 2 rails on top and a curve. Ollie on and grind up the slope and through the curve. **Screenshot 75.** Do the same with powered rails.

**Falls and hunger**

- **D16.** Type `/fill ~3 ~ ~-1 ~10 ~7 ~1 minecraft:stone` to make a tower 8 blocks high. Get on top in creative, type `/gamemode survival`, and roll off the far end without an ollie: you lose 2.5 hearts.
- **D17.** Dig out the ground where you landed, set hay bales in flush with the ground, and roll off again: you lose half a heart. Then roll off badly so the skater bails in the air over plain ground: 2.5 hearts again, not less.
- **D18.** Ride for a minute: the hunger bar doesn't drop. Walk and sprint for a minute: it does.

**The board refused or picked up**

- **D19.** Type `/fill ~3 ~-2 ~-1 ~5 ~-1 ~1 minecraft:water` and ride into the pool. The board is picked up as your eyes go under, and you swim.
- **D20.** On the board, type `/tp ~ ~ ~20`: you arrive standing. Throw an ender pearl high, then press J while it flies: you land standing where it hits.
- **D21.** Fly in creative and press J: nothing happens. In `/gamemode spectator`, J does nothing either. Back in `/gamemode survival`, put on the elytra, type `/tp ~ ~30 ~`, glide and press J: nothing happens.
- **D22.** In clear weather, use a strider egg on the ground, saddle the strider and ride it. Press J: nothing happens. **Screenshot 76.** Crouch to get off and press J: the board goes down. From the board, right-click the strider: nothing happens.
- **D23.** `/time set night`, lie in the red bed, and press J: nothing happens.
- **D24.** Type `/fill ~3 ~-2 ~-1 ~5 ~-1 ~1 minecraft:powder_snow` and skate into it. The board is picked up as you sink in; you are held and start to freeze. **Screenshot 77.** In the snow J does nothing. Climb out and J works again.
- **D25.** Type `/effect give @s minecraft:levitation 5 0`, then press J: the board goes down, and you keep rolling while the effect counts down.

**Rebinding, keyboard and mouse, switching off**

- **D26.** In Options > Controls > Minecraft, rebind "Skateboard Down/Up" to K and restart. K puts the board down, J does nothing, and the overlay names K. Set it back to J.
- **D27.** Unplug the controller and press J. The overlay reads "SKATE | Keyboard: WASD steer, mouse flick, Space push | J: pick up". **Screenshot 78.**
- **D28.** Try the keyboard and mouse controls:
  - Space pushes, Left Ctrl brakes, and A and D steer.
  - While rolling, flick the mouse down and straight back up: the skater ollies.
  - In the air, hold a mouse button: the skater grabs the board.
  - Tab steps off the board, and Tab again gets back on.
  - Alt-Tab out while rolling: the skater coasts and doesn't keep turning.
  - Plug the controller back in: the overlay says "Original controller controls".
- **D29.** While riding, set Skating to Disabled in the Esc menu. The board is picked up, and J shows "Skating is off" again. `latest.log` has "Skating turned off: map session dropped".
- **D30.** Optional: add `IW4L_LOG_LEVEL=debug` to the `.env`. Ride straight at full speed for a minute into land you have never visited. If the world can't keep up, the board stops at an invisible wall at a chunk edge and rolls on once the land appears; `latest.log` then has "Skate chunk walls". Remove the line afterwards.

## Known limits

These are known gaps, not new bugs:

- **Spawners**
  - Trial spawners show no mob and refuse spawn eggs.
  - Silverfish spawners (strongholds) and breeze spawners show flames but no mob, and a jockey in a spawner shows nothing.
  - An egg refused because `spawner_blocks_work` is off gives no chat message; it is only logged.
- **`/tick`**
  - Under `/tick freeze`, a strider you ride stands still. In vanilla it keeps walking.
  - Your own timers (hunger, effects, fire, mining, elytra) run at 20 a second at any tick rate and while frozen.
  - MW2, the sky and the block animations never freeze.
  - One huge tick can't be cut short.
- **End gateways**
  - Only players and pearls go through.
  - Every pose touches the gateway with the standing box.
  - The first trip may hitch while far chunks generate.
  - There is no advancement.
- **Riding**
  - Only striders can be ridden.
  - The MW2 body stands on the strider, and you see the strider one server tick behind.
  - Riders don't travel through portals.
  - Pigs take a saddle but can't be ridden.
- **Hoglins and piglins**
  - Hoglins take no leads, and Fire Aspect isn't modelled.
  - A golden spear fights like a sword. A piglin's crossbow is drawn uncharged and never wears out.
  - Bastion brutes saved before round 8 have no golden axe.
- **Sleeping**
  - There are no phantoms.
  - The MW2 body lies in its standing idle pose.
  - A player saved while asleep stands up on joining.
  - The multiplayer sleep messages wait for multiplayer.
- **Shulkers**
  - They aren't solid to the MW2 player.
  - Looting doesn't raise the shell chance.
  - A new shulker has no shell armour until it first closes; vanilla does the same.
  - There are no effect icons, so levitation and slow falling only show by what they do.
- **End cities and frames**
  - Banners have no patterns, and end rods give off no particles.
  - There are no maps, so no map in a frame.
  - TNT that explodes in water still breaks frames.
- **The dragon:** its dissolve while it dies is not drawn.
- **The glide pose**
  - MW2 bullets hit the upright pose of a gliding body.
  - A lying body can pop at the screen edge.
  - There are no rocket or wind particles, and in F5 the body sits lower than in vanilla.
- **Music:** none plays in the Overworld or the Nether.
- **Powder snow:** it catches falls by speed, not by fall distance, and mobs don't freeze.
- **Books:** they can't be written in game, and their pages show as plain text.
- **Weather:** rain and snow streaks follow the eyes, not the F5 camera, within a fixed radius of 10.
- **Skating**
  - The keyboard keys are physical positions: on a German keyboard, "Z" is the key labelled Y.
  - Only the J key can be rebound.
  - The mouse flick ignores Invert mouse.
  - A bail that starts on the ground and rolls over an edge takes no fall damage.
- **Creative:** buckets still empty, unlike vanilla.
- **Multiplayer:** not in this build.

## Screenshot numbering

- Name each screenshot by its number, e.g. `14.png`. The numbers run through the whole document, so they never repeat between sections.
- When a step fails, take a screenshot even if the step doesn't ask for one. Name it by its step label, e.g. `B24.png`, and say in one line what you expected and what you saw.
- Send `latest.log` from A8, A47 and D29. Send it after any crash too.
