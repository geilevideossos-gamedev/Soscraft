# Round 10, release 4: test steps

Build: `iw4l-survival.exe` from `minecraft-survival` at `c41ac6af`. It is release 3 plus track W's segment w1 (darkness, vibrations and sculk), track M1's segment m1a (rabbits and polar bears) and the protocol bump. The release 3 exe (`3fefa3f0`) is kept as `iw4l-survival-3fefa3f.exe`. The steps of [release-3-tests.md](release-3-tests.md), [release-2-tests.md](release-2-tests.md) and [release-1-tests.md](release-1-tests.md) are not repeated here. Release 3's steps still hold for this build, so any you have not done yet can be done with it. Only its F-1 version check changes: with this build the refusal names protocol 9.

**Multiplayer is protocol 9** (release 3 was 8), because W's vibration ripples and sculk souls now reach friends inside the entity snapshot. A friend on the release 3 exe is refused at the join with the version message, so the host and every friend need this build. `mp-local.ps1` runs it in both windows. The relay `iw4l-master.exe` is copied in again with this build. Its own protocol is unchanged. A friend over the internet needs a new zip from `-FriendKit` (see Setup).

**Send numbered screenshots.** Every **Screenshot N** below has its own number, from 1 to 53 through the whole document. Name each file by its number (`14.png`), or `47-host.png` and `47-friend.png` when a step asks for both windows. See "Screenshot numbering" at the end.

The whole test takes about 6 hours 25 minutes. The single-player parts K, V, G, B, A, H, C, R and P take about 5 hours 25 minutes and can be done in any order, except that G follows V in V's world. The two-window part F takes about 60 minutes. Spread it over a few sittings. Steps marked **optional** can be skipped.

## What is in this build

Track W, segment w1: darkness, vibrations and sculk ([track-w-log.md](track-w-log.md)):

- **Darkness:**
  - `/effect give` with `minecraft:darkness` closes the fog in to about 15 blocks and makes the light pulse every 4 seconds, as in vanilla.
  - Milk ends it at once. Lava and powder snow keep their own fog.
  - Each window draws its own player's darkness.
  - An effect given again with a longer time reaches a friend with its new length.
- **Game events and vibrations:** players, mobs, blocks, explosions and MW2 guns make vanilla's game events.
  - **Who makes them:**
    - Players and mobs: steps, landings, eating, armour, riding, shearing, hurts and deaths.
    - Blocks: doors, levers, buttons, plates, containers, buckets, bells, note blocks, TNT, candles and pistons.
    - Thrown things: the throw and the landing.
  - **Sculk sensors:** they hear them within 8 blocks, as a ripple that flies to the sensor. They then power redstone for 1.5 seconds, and a comparator reads the frequency.
  - **What stops them:** crouched steps are quiet, and wool and carpet dampen. A shell of wool blocks the way.
- **Calibrated sensors:** they hear 16 blocks. When a redstone signal comes into their back, they take only the frequency equal to its strength. An amethyst block beside a sensor passes its frequency on (resonance).
- **MW2 guns:**
  - A shot is heard where it is fired, unless you are crouched or prone.
  - Where a bullet lands is always heard. A grenade is an explosion.
- **Shriekers:**
  - A player stepping on a shrieker, or a sensor near one clicking for a player, makes it shriek.
  - A shrieker that can summon warns. Each player's warning level is saved in the player file and shared with the players within 16 blocks, with a 10 second cooldown.
  - After the shriek a warden sound answers by level, and the players within 40 blocks get darkness. **The warden itself is not in this build** (W's segment w2): at warning level 4 the shrieker only plays the angry listening sound.
- **Catalysts:**
  - A catalyst takes the death of any mob or player within 8 blocks. It blooms with soul wisps, and no orbs drop.
  - The death's experience becomes charge, which spreads sculk over the ground. Bigger charges grow sensors and shriekers, which cannot summon.
- A friend who leaves with a chest open is heard closing it at once (review fix `f509b23f`).
- At W's merge ([decisions.md](decisions.md), last bullet), more features make vibrations:
  - a mob coming out of a trial spawner;
  - a firework burst;
  - a spawn egg used on a trial spawner;
  - raiders opening and closing doors, and zombies breaking doors.

Track M1, segment m1a: rabbits and polar bears ([track-m1-log.md](track-m1-log.md), rows 15 and 22 of [mobs.md](mobs.md)):

- **Rabbits:**
  - **Behaviour:**
    - They hop with vanilla's jump control, flee players, wolves and monsters, and panic when hurt.
    - Carrots, golden carrots and dandelions tempt them.
    - They breed. A baby takes a parent's colour, or one time in twenty the biome's. Dandelions make babies grow, and a golden dandelion locks their age.
    - With `mob_griefing` they raid full-grown carrots, one stage a bite.
  - **Looks:** they are drawn with vanilla's adult and baby models and 16 textures (Toast's among them), and hop and tilt their heads with vanilla's animations. Friends see them at the same moments.
  - **The killer bunny** (`RabbitType` 99): armour 8, never flees, leaps at players and wolves within 4 blocks and bites for 8.
  - **Hunters:** wild cats and wolves hunt rabbits.
  - They are saved, hatch from eggs and spawners, and spawn in their biomes in their colours.
- **Polar bears:**
  - **Fights:**
    - They rear up and growl before each swipe (6 before difficulty).
    - They defend their cubs: a hit cub calls the adults within 20 blocks, and adults attack players within 10 blocks of a cub.
    - They stay angry 20 to 39 seconds, and the anger is saved.
    - Adults panic only from fire, cubs from any hit. Cubs follow their mothers.
  - **Spawns:** on grass in snowy biomes and on sea ice in frozen oceans, an adult with cubs.
  - **Looks:** drawn with vanilla's adult model (1.2 times its size) and the cub's own model, standing up for each swipe. Friends see the same pose.

Also in the build: `c41ac6af`, multiplayer protocol 9.

**Not in this build:**

- The warden and its brain, sniffing, digging and sonic boom (track W, segment w2).
- Breezes (track D, segment d2).
- Margit's look, his hits on players and his altar (track F, segment f3).
- E2's segment e2b (firework shapes, held maps and the rest) and E1's segment e1b.
- Silverfish hiding in stone (track D0, segment d0b).
- The other M1 animals (foxes, goats, parrots, pandas, turtles, armadillos, sniffers and bees): their eggs still hatch nothing. The M2 and M3 mobs are not in either.

## Setup

- **The exe:** `iw4l-survival.exe` in `C:\Games\Mashup\2010-Rust-Rewrite-Mashup`, with the relay `iw4l-master.exe` next to it. Part F starts the release 3 exe `iw4l-survival-3fefa3f.exe` once, for the version check.
- **Class:** keep release 1's class, an **M4A1 with Red Dot Sight, a USP .45, Frag and Flash**. Part G shoots and throws a frag near a sensor. The M4A1 also kills mobs between steps.
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
  IW4L_MINECRAFT_WORLD=r10dark
  IW4L_MINECRAFT_KIT=<the part's kit line>
  ```

  - An `IW4L_SKATE_ASSETS=...` line from release 1 can stay instead of `IW4L_SKATE=off`. Skating is not tested here.
  - Remove `IW4L_MINECRAFT_DIMENSION` and `IW4L_MINECRAFT_SEED`. Keep it free of `IW4L_MASTER_HOST_NAME` and `IW4L_MASTER_JOIN`.
  - In two windows the world comes from `-World`, and both windows read the kit line.
- **Kits:** each part has its own kit line in its own block. A kit fills only the backpack of a player new to the world. Every part except G starts a new world, so the kit comes with the first join.
  - **The first join also gives the class loadout,** to the host and to every friend: the M4A1 on hotbar slot 1, the USP .45 on slot 2, 32 frags and 32 flashes in the two grenade slots, and 90 rifle and 36 pistol rounds (three stacks).
  - Every kit here leaves room for the rounds in the backpack, so hotbar slots 3 to 9 and the offhand start empty.
  - To move a backpack item to the hotbar, open the inventory (E), hover the item and press the hotbar slot's number key.
  - In a world played before, `/clear` and `/kit` give the kit again, but no loadout.
- **Names:** `Sos` is the host's player name and `Friend` the friend's. Use your names if they differ.
- **In every new world**, the host types `/difficulty normal` and `/gamerule keep_inventory true` first. In two windows the host also types `/op Friend`.
- **Commands:** `/kill` only takes players, so the steps clear monsters with `/difficulty peaceful` and then `/difficulty normal`. Rabbits and polar bears are not monsters, so peaceful does not remove them: kill them with the sword or the M4A1. Only the host gets Tab suggestions.
- **Keys:**
  - E opens the inventory, Q drops one item and Ctrl+Q a whole stack, F swaps hands, T opens the chat for commands, F5 changes the camera.
  - "Crouch" means your MW2 crouch key. Prone counts as crouched for vibrations too.
- **Eating:** as in vanilla, an apple can only be eaten when the hunger bar is not full, and a new world starts with it full. So before a step eats an apple, run `/effect give @s minecraft:hunger 5 99` (for a friend: `/effect give Friend minecraft:hunger 5 99`). For 5 seconds the bar turns green, and it ends three or four drumsticks lower, room for two apples. Milk can always be drunk.
- **Your position:** the game has no F3 screen. `/tp @s ~ ~ ~` leaves you where you stand and prints your position in the chat, for example "Teleported Sos to 12.912000, 64.000000, -40.087000" (X, Y, Z).
- **Commands with `~`** count from the block you stand on, not from where you look. `~ ~ ~4` is 4 blocks from you toward growing Z (south). The rigs below are built that way, so stand still while you type them, then turn to look at what they built.
- **Comparators, repeaters, pistons and calibrated sensors keep facing north when placed by hand.** In this build such a block, placed from the hotbar, keeps its block's default facing (north), whichever way you look. Doors, trapdoors, gates, furnaces and a few other blocks do turn. So the steps place these blocks with `/setblock` and a given facing.
- **Logs:** single player, `iw4l-artifacts\logs\latest.log`. With the script, `iw4l-artifacts\mp-local\host\iw4l-artifacts\logs\latest.log` and `...\mp-local\friend\...`.
- Steps marked **(two windows)** need the friend. Only the window in front gets your keys.

### The sensor rig

Parts V, G, B and F use this rig.

1. Stand on fairly flat open ground. Build the rig near where you first appeared in the world, so you respawn close to it.
2. Stand still and run these six commands:

   ```
   /fill ~-2 ~-1 ~-2 ~2 ~-1 ~22 minecraft:stone
   /fill ~-2 ~ ~-2 ~2 ~3 ~22 minecraft:air
   /setblock ~ ~ ~4 minecraft:sculk_sensor
   /setblock ~1 ~ ~4 minecraft:redstone_lamp
   /setblock ~ ~ ~5 minecraft:comparator[facing=north]
   /fill ~ ~ ~6 ~ ~ ~20 minecraft:redstone_wire
   ```

3. The commands build this:
   - a stone floor 5 blocks wide, cleared 4 blocks high;
   - a sculk sensor 4 blocks from you, with a redstone lamp against one side of it;
   - a comparator on the sensor's far side;
   - 15 redstone dust in a straight line from the comparator.
4. **The start** is the block you stood on. It is in line with the dust, 4 blocks from the sensor, on the side away from the comparator.
5. To build the rig again after a blast, stand on the start and run the six commands again.

**Reading the rig:** when the sensor hears something, its tendrils click and the lamp lights for 1.5 seconds. For those 1.5 seconds the comparator lights as many dust blocks as the frequency of what it heard, counting from the comparator. Then the sensor rests for half a second. **Wait 2 seconds between actions,** because the sensor hears nothing while it is lit or resting. **Move crouched between actions,** because crouched steps make no vibration.

| Lit dust | What made it |
|---|---|
| 1 | a step, swimming, a chicken's flap |
| 2 | a landing; a thrown thing or a bullet landing |
| 3 | a throw; a gun shot |
| 4 | taking armour off |
| 5 | putting armour on; getting off a horse |
| 6 | shearing; getting on a horse |
| 7 | getting hurt |
| 8 | eating; drinking |
| 9 | closing a door, gate, trapdoor or container; a lever off; a plate or piston going back |
| 10 | opening one of them; a lever on; a button; a plate pressed; a piston pushing; lighting TNT; a note block |
| 11 | a bell; lighting a candle; a spawn egg used on a trial spawner |
| 12 | breaking a block; filling a bucket |
| 13 | placing a block; emptying a bucket |
| 14 | a mob from a spawn egg or out of a trial spawner |
| 15 | an explosion; a firework burst; a death |

## K. Darkness

World `r10dark`, single player, a fresh world. Survival, by day, on open ground. `/difficulty peaceful` is fine for this part, so no monsters come at night. Allow about 15 minutes.

Kit K (6 stacks; each milk bucket is its own stack):

```
IW4L_MINECRAFT_KIT=minecraft:milk_bucket,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:torch*64,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

- **K-1. The fog and the pulse.** Look at land that reaches far, and run `/effect give @s minecraft:darkness 30`.
  - Within about a second, everything farther than about 15 blocks fades into black fog. The darkness icon shows.
  - The light around you dims and comes back in a slow pulse: about 2 seconds dimmer, 2 seconds normal.

  **Screenshot 1** (in a dim moment).
- **K-2. Milk.** Drink a milk bucket (hold right-click). The view snaps back at once, and the icon is gone.
- **K-3. The fade.** `/effect give @s minecraft:darkness 3`. The darkness fades in over about a second, and in its last second it fades out smoothly instead of snapping back.
- **K-4. Torches at night.**
  - `/time set midnight`. Place 4 torches on the ground around you, 3 blocks from you.
  - `/effect give @s minecraft:darkness 30`.
  - In the dim half of each pulse, the torch-lit ground goes almost black, and it comes back in the other half.

  **Screenshot 2** (in the dim half). Drink a milk bucket, then `/time set day`.
- **K-5. Lava and powder snow.**
  - Run these four commands. They give you fire resistance, then dig a lava pool 3 by 3 and 2 deep on one side of you and a powder snow pit of the same size on the other side, then give you darkness:

    ```
    /effect give @s minecraft:fire_resistance 120
    /fill ~2 ~-2 ~2 ~4 ~-1 ~4 minecraft:lava
    /fill ~-4 ~-2 ~2 ~-2 ~-1 ~4 minecraft:powder_snow
    /effect give @s minecraft:darkness 120
    ```

  - Walk into the lava pool and crouch at its bottom, so that your view is under the lava. You see the thick orange lava fog as before, not the black darkness fog.
  - **Screenshot 3** (the view under the lava).
  - Get out with `/tp @s ~ ~3 ~` if jumping does not bring you out.
  - Walk into the powder snow pit and crouch in it, so that your view is in the snow: you see the white powder snow fog, not the black one. Get out the same way.
  - `/effect clear @s`.

## V. Sculk sensors and vibrations

World `r10sculk`, single player, a fresh world. Survival, Normal, by day. Build the sensor rig (Setup) first and read "Reading the rig". Part G is done in this world after V. Allow about 45 minutes.

Kit V (17 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:sculk_sensor*8,minecraft:redstone_lamp*4,minecraft:white_wool*16,minecraft:white_carpet*16,minecraft:snowball*16,minecraft:trident+loyalty=3,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:cow_spawn_egg*4,minecraft:cobblestone*64,minecraft:iron_pickaxe,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*16,minecraft:trial_spawner,minecraft:silverfish_spawn_egg*4,minecraft:diamond_sword,minecraft:golden_apple*8,minecraft:cooked_beef*16
```

Put the iron pickaxe, the snowballs, the trident, the cobblestone and the white wool on hotbar slots 3 to 7.

### Steps, crouching and blocks

- **V-1. A step.** Stand on the start and wait until the lamp is dark. Take a few steps sideways, staying about 4 blocks from the sensor.
  - With a step, a small pale ripple flies from your feet to the sensor, and its tendrils click.
  - The lamp lights for 1.5 seconds, and the dust line lights only the dust next to the comparator (frequency 1).
  - The next step half a second after the lamp goes dark is heard again.

  **Screenshot 4** (the lamp lit, with the ripple if you catch it).
- **V-2. Crouching.**
  - Crouch (or go prone) and walk around near the sensor: no ripple, and the lamp stays dark.
  - Still crouched, place a cobblestone on the floor 3 blocks from the sensor: it is heard, and 13 dust light (placing is heard even crouched).
  - Wait 2 seconds and break that cobblestone with the pickaxe: 12.

  **Screenshot 5** (the 13 or the 12).
- **V-3. Standing on it.** Step up onto the sensor and stand there. It fires every 2 seconds, also while you crouch, and lights 1 each time. Step off.
- **V-4. A landing.** Stand still on the start for 2 seconds, then jump in place: 2 (the landing).

### Wool and carpet

- **V-5. Wool.**
  - Place one white wool block on top of the sensor, and one on the floor right next to the sensor on the start's side, between the sensor and the start.
  - Walk back and forth on the start's side, 3 to 5 blocks from the sensor and no more than 2 blocks to either side of the line through the dust. Nothing is heard: the wool between you and the sensor blocks the vibrations.
  - Break the wool on the start's side, then walk there again: the ripples come again.
  - **Screenshot 6** (walking with the wool in place, lamp dark).
- **V-6. Carpet.**
  - Crouch and lay white carpet on the floor on the start's side: 12 carpets, covering the blocks 2 to 5 from the sensor and one block to each side of the line. Laying it is not heard, because carpet dampens.
  - Stand up and walk on the carpet: nothing. Step off onto the stone: the ripples come again.
  - Place a white wool block on the floor 3 blocks from the sensor, wait 2 seconds and break it again: neither is heard.
  - Break the wool on top of the sensor and pick up the carpets.

### Throws, TNT and a cow

- **V-7. Thrown things.**
  - Walk crouched to 12 blocks from the sensor on the start's side and stay crouched. Throw a snowball so it lands on the stone about 5 blocks from the sensor. A ripple flies from where it landed, and 2 dust light (the crouched throw itself is not heard).
  - **Screenshot 7** (the 2).
  - Walk crouched to the start, stand up, stand still for 2 seconds and throw a snowball far to the side. 3 light: the throw is heard before the landing.
  - Crouch on the start, wait 2 seconds and throw the trident into the floor 4 blocks from the sensor, not onto the dust: 2. The trident flies back to you.
- **V-8. TNT.**
  - Go 3 blocks past the start, away from the sensor, so you stand 7 blocks from the sensor.
  - Place a TNT on the ground there. Build a cobblestone wall 3 wide and 2 high between the TNT and the sensor. Stone does not stop vibrations; only wool does.
  - Wait until the lamp is dark, then light the TNT with flint and steel: 10 (lighting it). Then crouch and walk 8 blocks to the side, and stand still.
  - About 4 seconds after lighting, it blows: 15.

  **Screenshot 8** (the 15). If the blast broke the sensor, the comparator or the dust, build the rig again (Setup).
- **V-9. A cow.**
  - Crouch and use a cow egg on the floor 5 blocks from the sensor: 14 (a mob from an egg).
  - As the cow wanders near the sensor, its steps light 1.
  - Optional: if it walks onto the sensor, it fires every 2 seconds while the cow stands there.
  - Kill the cow with the M4A1.
- **V-10. Optional, saving a ripple.** Stand 8 blocks from the sensor, take one step, and at once press Esc and save and quit. Load the world. If the ripple was still in flight when the game saved, the sensor clicks and the lamp lights once within a second of the load: a ripple saved in flight still arrives.

### Fireworks and trial spawners

These make vibrations since W's merge.

- **V-11. A firework burst.**
  - Build a 3 by 3 cobblestone roof 3 blocks above the floor on four cobblestone pillars, 5 blocks from the sensor on the start's side.
  - Stand 3 blocks from the middle of the roof, outside it, and wait 2 seconds.
  - Right-click the floor under the middle of the roof with a rocket. It meets the roof at once and bursts in red sparks with a bang: 15.
  - You may lose about two hearts, because your own rocket hurts you within 5 blocks.

  **Screenshot 9** (the 15).
- **V-12. A trial spawner.**
  - Place the trial spawner on the floor 5 blocks from the sensor on the start's side, beside the roof of V-11: 13. Wait 2 seconds.
  - Use a silverfish egg on it: 11 (a spawn egg on a trial spawner).
  - It wakes up because you are within 14 blocks, and silverfish come out one every 2 seconds. When one comes out while the sensor is ready (its lamp dark for more than half a second), 14 light. Their crawling lights 1.
  - **Screenshot 10** (the 14).
  - Kill the silverfish with the diamond sword: each death lights 15 when the sensor is free. The spawner then ejects its reward.

## G. MW2 guns

In the world `r10sculk` of part V, with the same rig. No new kit. Use the M4A1. Allow about 15 minutes.

- **G-1. A landing from far away.**
  - Walk crouched to 20 blocks from the sensor on the start's side, then stand up and stand still for 2 seconds.
  - Shoot the stone floor right beside the sensor once. Hitting the sensor itself counts too.
  - A ripple flies from where the bullet hit, and 2 dust light. The shot itself is 20 blocks away, too far to be heard.

  **Screenshot 11** (the 2).
- **G-2. The shot.**
  - Stand on the start, face away from the sensor and stand still for 2 seconds.
  - Shoot something at least 20 blocks away: a tree, a hill, or a 3-high cobblestone pillar you build 25 blocks away first.
  - 3 light: the shot was heard where you fired it. Its landing is too far away.
- **G-3. Crouched.**
  - Crouch (or go prone) on the start and shoot the same far target: nothing.
  - Still crouched, shoot the floor beside the sensor: 2. The landing is heard even when the shot is not.
- **G-4. Into the sky.** Crouched on the start, shoot straight up into the open sky: nothing. A bullet that hits nothing lands nowhere.
- **G-5. A grenade.**
  - Walk crouched to 20 blocks from the sensor on the start's side. A frag's blast reaches several blocks, and one that lands near the start would kill you there.
  - Stand up and throw a frag so that it lands about 6 blocks from the sensor on your side, about 14 blocks in front of you. Then crouch at once and stand still.
  - When it blows: 15, and the lamp lights. The throw is 20 blocks from the sensor, too far to be heard.

  **Screenshot 12** (the 15). If the blast broke the rig, build it again.

## B. Blocks and bodies

World `r10blocks`, single player, a fresh world. Survival, Normal, by day. Build the sensor rig (Setup) near where you first appeared. Do everything **5 blocks from the sensor** on the start's side or beside the start, never on the dust, unless a step says otherwise. Wait 2 seconds between actions, and move crouched. Allow about 45 minutes.

Kit B (22 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:sculk_sensor*4,minecraft:oak_door*2,minecraft:oak_trapdoor*2,minecraft:oak_fence_gate*2,minecraft:lever*4,minecraft:oak_button*4,minecraft:oak_pressure_plate*2,minecraft:chest*2,minecraft:barrel*2,minecraft:bucket,minecraft:bell,minecraft:note_block*2,minecraft:candle*4,minecraft:flint_and_steel,minecraft:apple*16,minecraft:milk_bucket,minecraft:diamond_helmet,minecraft:sheep_spawn_egg*2,minecraft:shears,minecraft:horse_spawn_egg*2,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

Every block you place lights 13 when it is heard, so wait 2 seconds after placing before you test the block.

### Blocks you use

- **B-1. A door, a trapdoor and a gate.**
  - Place an oak door on the floor. Open it: 10. Close it: 9.
  - **Screenshot 13** (the 10).
  - Do the same with an oak trapdoor on the floor and an oak fence gate: 10 when opened, 9 when closed.
- **B-2. A lever and a button.**
  - Place a lever on the floor. Flip it on: 10. Flip it off: 9.
  - Put a cobblestone on the floor, an oak button on its side, and press the button: 10.
  - The button springs back 1.5 seconds later, while the sensor is still lit from the press, so its release shows nothing. Vanilla is the same.
- **B-3. Containers.** Place a chest. Open it: 10. Close it (E): 9. Do the same with a barrel: 10 and 9.
- **B-4. A bucket.**
  - Stand on the start and run `/setblock ~2 ~-1 ~ minecraft:water`. This puts a water source in the floor 2 blocks beside you; it stays in its hole.
  - Fill the empty bucket from it: 12.
  - Empty the bucket back into the same hole: 13. Don't empty it on the floor, where the water would run over the dust and break it.
- **B-5. A bell and a note block.**
  - Place the bell on the floor, stay where you placed it from, and right-click the side that faces you, on the bell's body below its top: 11. As in vanilla, the top and the two other sides do not ring a bell standing on the floor.
  - Place a note block on the floor and right-click it: it plays a note, 10.
- **B-6. A pressure plate.**
  - Place an oak pressure plate on the floor. Crouch beside it, look at it and drop one cooked beef onto it (Q). The plate goes down: 10.
  - Wait 2 seconds, then crouch-walk over the beef to pick it up. The plate comes back up: 9. You yourself do not press plates in the fork; only dropped items do.
- **B-7. A candle.** Place a candle on the floor, then light it with flint and steel: 11.
- **B-8. A piston.** Stand on the start and run this command. It sets a piston 2 blocks beside you, on the other side from B-4's water, pointing up:

  ```
  /setblock ~-2 ~ ~ minecraft:piston[facing=up]
  ```

  - Wait 2 seconds and run `/setblock ~-2 ~ ~1 minecraft:redstone_block`. The redstone block next to the piston powers it, and the piston pushes its head up: 10.
  - **Screenshot 14** (the head up and the 10).
  - Wait 2 seconds and run `/setblock ~-2 ~ ~1 minecraft:air`. The piston pulls back: 9. The commands themselves make no vibration.

### Your body and animals

- **B-9. Eating and drinking.**
  - Run `/effect give @s minecraft:hunger 5 99` (see "Eating" in Setup) and wait until the bar is no longer green.
  - Stand on the start and eat an apple: 8.
  - Crouch and eat another: 8 again. Eating is heard even crouched.
  - Drink the milk bucket: 8.

  **Screenshot 15** (an 8).
- **B-10. Armour.** Open the inventory (E) and put the diamond helmet into the helmet slot: 5. Wait 2 seconds, then take it off again: 4.
- **B-11. A chicken.**
  - Stand on the start and wait until the lamp is dark.
  - Run `/summon minecraft:chicken ~2 ~6 ~2`. A chicken appears 6 blocks up, about 3 blocks to the side of the sensor, and falls slowly, flapping. The summon itself makes no vibration.
  - Its flaps light 1 as it falls.

  **Screenshot 16** (the 1, while it falls or just after). Kill the chicken with the M4A1, so its steps don't keep the sensor busy.
- **Mobs' steps:** the sheep and the horse below light 1 with their own steps, and a sensor lit by a step misses what comes in the next 2 seconds. When a number is missed that way, wait until the mob stands still and try again, or use the second egg.
- **B-12. A sheep.** Use a sheep egg (14), wait 2 seconds and shear the sheep: 6.
- **B-13. A horse.**
  - Use a horse egg (14), wait 2 seconds and right-click the horse with an empty hand to get on: 6.
  - Wait 2 seconds, then crouch to get off, or wait until it throws you off: 5.
- **B-14. A fall.** Stand 3 blocks from the sensor and pillar straight up 5 blocks with cobblestone: look down, jump and place a block under you, five times. Wait 2 seconds on top and jump off. You take fall damage, and 7 light (the hurt).
- **B-15. A death.** Stand 3 blocks from the sensor and run `/kill @s`: 15 light as you die. You respawn near the rig with everything you had.

## A. Amethyst and calibrated sensors

World `r10amethyst`, single player, a fresh world. Survival, Normal, by day, on fairly flat open ground. Allow about 25 minutes.

Kit A (6 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:sculk_sensor*2,minecraft:amethyst_block*2,minecraft:lever*2,minecraft:apple*16,minecraft:iron_pickaxe,minecraft:cooked_beef*16
```

- **A-1. The rig.** Stand still and run these eight commands:

  ```
  /fill ~-3 ~-1 ~-3 ~3 ~-1 ~30 minecraft:stone
  /fill ~-3 ~ ~-3 ~3 ~3 ~30 minecraft:air
  /setblock ~ ~ ~4 minecraft:sculk_sensor
  /setblock ~1 ~ ~4 minecraft:amethyst_block
  /setblock ~-1 ~ ~4 minecraft:redstone_lamp
  /setblock ~ ~ ~18 minecraft:calibrated_sculk_sensor[facing=north]
  /setblock ~1 ~ ~18 minecraft:redstone_lamp
  /fill ~ ~ ~19 ~ ~ ~26 minecraft:redstone_wire
  ```

  - **What they build:**
    - A plain sensor 4 blocks from you, with an amethyst block on one side and a lamp on the other.
    - A calibrated sensor 18 blocks from you, with a lamp beside it.
    - 8 redstone dust in a line behind the calibrated sensor, touching its back.
  - The block you stand on is **the near end**. The calibrated sensor is 18 blocks from it, out of its 16-block reach.
  - In this part, walk one block beside the line of the sensors and the dust, never over them: a sensor you step on fires.
  - Walk crouched to the end of the dust. Place a lever on the floor right after the last dust, in line with it, and flip it on.
  - The 8 dust light, brightest at the lever. The dust that touches the calibrated sensor carries 8, so it now takes only frequency 8 (eating and drinking).
- **A-2. Resonance.**
  - Walk crouched back to the near end. Run `/effect give @s minecraft:hunger 5 99` (see "Eating" in Setup) and wait until the bar is no longer green.
  - Stand still for 2 seconds. Eat an apple.
  - The plain sensor clicks and its lamp lights, and the amethyst chimes.
  - About a second later, a ripple flies from the amethyst block to the calibrated sensor. It clicks, and its lamp lights. A calibrated sensor stays on for only half a second, so watch the far lamp closely.

  **Screenshot 17** (the far lamp lit).
- **A-3. Another frequency.** Wait 2 seconds and take a few steps at the near end, standing up. The plain sensor triggers and the amethyst chimes at a lower note, but the calibrated sensor stays dark: it waits for 8, and steps are 1.
- **A-4. Heard directly.**
  - Walk crouched to 10 blocks from the calibrated sensor, which is 4 blocks past the plain sensor. Stand still for 2 seconds and eat an apple: the calibrated sensor lights. It heard the eating itself, 10 blocks away.
  - Wait 2 seconds and mine one stone of the floor beside you with the pickaxe. The plain sensor and the amethyst react (12), and the calibrated sensor stays dark.
- **A-5. The calibrated reach.**
  - Mine the amethyst block with the pickaxe. Otherwise the plain sensor, which hears your steps near the near end, passes them on to the calibrated sensor.
  - Walk crouched to the lever and flip it off. Now the calibrated sensor takes every frequency.
  - Walk back toward the near end, standing up. The calibrated sensor's lamp lights with your steps until you are about 16 blocks from it (about 2 blocks before the near end). A plain sensor hears only 8 blocks.

  **Screenshot 18** (at 12 blocks from the calibrated sensor, its lamp lit).

## H. Shriekers

World `r10shriek`, single player, a fresh world. Survival, Normal, by day, on flat open ground. A shrieker's darkness is the darkness of part K. For the sound steps, write one line per step with what you heard. Allow about 30 minutes.

Kit H (7 stacks; each milk bucket is its own stack):

```
IW4L_MINECRAFT_KIT=minecraft:sculk_shrieker*4,minecraft:sculk_sensor*4,minecraft:snowball*16,minecraft:cobblestone*64,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cooked_beef*32
```

### The warnings

- **H-1. The first warning.**
  - Run `/setblock ~ ~-1 ~3 minecraft:sculk_shrieker[can_summon=true]`. It sets a shrieker into the ground 3 blocks from you, toward growing Z; its top is half a block below the ground. A shrieker placed by hand never summons, as in vanilla, so this one is made by command.
  - Walk onto it and step off again at once.
  - It shrieks: pale blue rings rise from it one after another for about 4.5 seconds, with a scream. **Screenshot 19** (the rings).
  - When the shriek ends, a warden sound plays somewhere around the shrieker. Your view goes dark with the darkness fog and pulse for about 13 seconds, and the darkness icon shows. Vanilla hides that icon.
  - **Screenshot 20** (the darkness).
- **H-2. The cooldown and the second warning.**
  - Step on it again before 10 seconds have passed since the first shriek: it stays silent.
  - Wait until at least 10 seconds after the first shriek and step on it again. The second warning: a shriek, then a closer, louder warden sound, and darkness again.
- **H-3. Saved.** Save and quit, then load the world. Wait 10 seconds and step on it: the third warning, with the closest warden sound. Your warning level was kept in your player file.
- **H-4. The fourth warning.** Wait 10 seconds and step on it: the shriek, then the angriest warden sound, and darkness. No warden comes in this build.
- **H-5. Milk.** Drink a milk bucket while it is dark: the darkness ends at once.

### Other shriekers and rules

- **H-6. A shrieker that cannot summon.**
  - Walk 20 blocks away from the summoning shrieker. Place a sculk sensor, and a sculk shrieker from the kit 2 blocks from it.
  - Walk around near the sensor. When the sensor clicks, the shrieker shrieks with its rings and scream. No warden sound or darkness follows. A shriek lasts 4.5 seconds, and the clicks during it are not answered.
  - **Screenshot 21** (the rings over the shrieker by the sensor).
  - Wait until it is quiet, then walk crouched 12 blocks away. Throw a snowball so it lands next to the sensor: the sensor clicks and the shrieker shrieks for you, the thrower.
- **H-7. The game rule.**
  - `/gamerule spawn_wardens false`. Go back to the summoning shrieker and wait until 10 seconds have passed since its last warning.
  - Step on it: it shrieks, but no warden sound and no darkness follow.
  - `/gamerule spawn_wardens true`.
- **H-8. Peaceful.** `/difficulty peaceful`, wait 10 seconds and step on it: it shrieks, and nothing answers. `/difficulty normal`.

## C. Catalysts and spreading sculk

World `r10catalyst`, single player, a fresh world. Survival, Normal, by day, on flat grass within 20 blocks of where you first appeared, so you respawn close by. Allow about 30 minutes.

Kit C (4 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:sculk_catalyst*4,minecraft:cow_spawn_egg*16,minecraft:diamond_sword+sharpness=5,minecraft:cooked_beef*32
```

The sword kills a cow with one hit.

- **C-1. A bloom.**
  - Place a sculk catalyst on the grass. Use a cow egg 3 blocks from it and kill the cow with the sword.
  - The catalyst's top flashes, two soul wisps rise over it, and you hear its bloom sound. No experience orbs drop.
  - A few blocks of sculk and thin sculk veins spread over the ground near the cow's death spot, with soft blue sparks and a quiet crackle. A cow's experience is too little to grow anything.

  **Screenshot 22** (the wisps or the new sculk).
- **C-2. Out of reach.** Use a cow egg 10 blocks from the catalyst and kill the cow there. Orbs drop as usual, and the catalyst does nothing.
- **C-3. Two catalysts.** Place a second catalyst 8 blocks from the first. Use a cow egg between them, 2 blocks from the second, and kill the cow: only the second catalyst blooms. **Screenshot 23.**
- **C-4. No mob drops.** `/gamerule mob_drops false`. Kill a cow 3 blocks from a catalyst: it still blooms. `/gamerule mob_drops true`.
- **C-5. Your death, kept inventory.** Stand 3 blocks from the first catalyst and run `/kill @s`. It blooms. With kept inventory a death carries no experience, so nothing new spreads. Respawn and walk back.
- **C-6. Your death with experience.**
  - Run `/gamerule keep_inventory false`, then `/xp add @s 15 levels`.
  - Stand 3 blocks from the first catalyst and `/kill @s`. It blooms. No experience orbs drop where you died; your items do.
  - Respawn and walk back. Over the next minute, sculk spreads well past the first patches, with sparks and crackles. It may grow a sculk sensor or a sculk shrieker 4 or more blocks from the catalyst.
  - **Screenshot 24** (the spread).
  - Pick up your items, then `/gamerule keep_inventory true`.
- **C-7. Saving the spread.** While the sparks of C-6 still show, save and quit, then load: the spreading carries on. If the sparks have already stopped, kill three cows 3 blocks from a catalyst first.
- **C-8. A grown shrieker.** If a shrieker grew, step on it: it shrieks, and no warden sound or darkness follows. Shriekers grown by a catalyst cannot summon.

## R. Rabbits

World `r10rabbit`, single player, a fresh world. Survival, Normal, by day, on flat grass in a plains biome. Allow about 75 minutes.

Kit R (24 stacks; `bone_meal*128` gives two, because 25 carrots take about 60 to grow):

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*16,minecraft:carrot*64,minecraft:dandelion*16,minecraft:golden_dandelion*4,minecraft:iron_hoe,minecraft:bone_meal*128,minecraft:oak_fence*64,minecraft:wolf_spawn_egg*2,minecraft:cat_spawn_egg*2,minecraft:zombie_spawn_egg*2,minecraft:lead*4,minecraft:iron_sword,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:spawner,minecraft:bow,minecraft:arrow*64,minecraft:flint_and_steel,minecraft:cod*16,minecraft:golden_apple*8,minecraft:cooked_beef*32
```

### Hopping, carrots and babies

- **R-1. Hatching.** Use the rabbit egg three times on grass 8 blocks away.
  - Three small rabbits sit up, with long ears and a round tail. Their front paws are on the ground and their bodies slope up to the head.
  - They move only in hops, each with a soft jump sound, and never slide along the ground.
  - Each hop tucks the back legs and stretches the body forward with the ears swept back. The rabbit sits up again as it lands.

  **Screenshot 25** (a rabbit mid-hop if you catch one).
- **R-2. Fleeing and carrots.**
  - Walk at them: within about 8 blocks they hop away fast.
  - Hold a carrot and stand still: they hop up to about 2 blocks from you and stay there.
- **R-3. The head tilt.**
  - Hold a carrot and stand still until a rabbit sits about 2 blocks from you. Without a carrot, a rabbit flees a player within 8 blocks.
  - Step slowly sideways around it, staying about 2 blocks from it: its head turns to follow you.
  - Stand still again. Once it has not hopped for about 9 to 11 seconds, it tilts its head for 4 seconds.
  - After that, its head no longer turns to you until its next hop: step sideways again to see it.

  **Screenshot 26** (the tilt).
- **R-4. Breeding.**
  - Aim at an adult from close and right-click it with a carrot: one carrot is used. Do the same to a second adult.
  - Within a few seconds the two meet. A baby appears beside them, with an experience orb.
  - The baby is much smaller, with a big head, short ears and a parent's colour.

  **Screenshot 27** (the baby beside a parent).
- **R-5. The baby.**
  - Watch the baby hop and tilt its head: it has its own, smaller animations.
  - Right-click it with dandelions a few times: each one is used. Each takes a tenth off the time it needs to grow up, so it looks the same until it is grown.
  - Right-click it with the golden dandelion: it is used, with its sound, and the baby's age is locked. Right-click it again: used again with the other sound, and the lock is off.
- **R-6. Colours.** Hold a carrot, so the new rabbits stay near you instead of fleeing. Stand still and run these, one after the other:

  ```
  /summon minecraft:rabbit ~ ~ ~3 {RabbitType:1}
  /summon minecraft:rabbit ~1 ~ ~3 {RabbitType:2}
  /summon minecraft:rabbit ~2 ~ ~3 {RabbitType:3}
  /summon minecraft:rabbit ~3 ~ ~3 {RabbitType:4}
  /summon minecraft:rabbit ~4 ~ ~3 {RabbitType:5}
  /summon minecraft:rabbit ~5 ~ ~3 {CustomName:"Toast"}
  ```

  They are, in order: white, black, white with black patches, gold, salt-and-pepper, and Toast's black and white coat. **Screenshot 28.**
- **R-7. A lead.**
  - Put a carrot in your offhand (select the carrots and press F), so the rabbits come to you, and select the leads.
  - Right-click a rabbit with a lead. The rope ties at its neck, low at the front.
  - Walk away: it hops after you. **Screenshot 29.**
  - Place an oak fence and right-click it while you lead the rabbit, to tie it there. For 15 seconds, it never tilts its head.
  - Then punch the knot off: stand 2 blocks from the fence and left-click the knot from the side, on the upper half of the post. The rabbit is free, and it tilts its head at once.
  - Take the carrot out of your offhand: select an empty hotbar slot, press F, then select another slot.

### Carrot raids

- **R-8. The garden.**
  - On flat grass, run `/setblock ~ ~-1 ~2 minecraft:water`, which puts a water source in the ground 2 blocks from you.
  - Hoe a 5 by 5 square of grass that touches the water. Plant a carrot on every block and bone-meal them until every carrot is full grown, with orange tops showing.
  - Fence the patch in, leaving one block of grass inside the fence on every side. Leave one fence out.
  - Use the rabbit egg three times inside the fence. Then walk out through the gap and place the last fence from outside.
  - Stand more than 8 blocks away, because rabbits flee a nearer player, and hold no carrot, because rabbits come to a carrot within 10 blocks instead of raiding.
  - Within one to three minutes, a rabbit hops onto a full-grown carrot and looks down at it. That carrot drops back one stage, and its orange top disappears. **Screenshot 30.**
  - Keep watching: the rabbits bite only full-grown carrots. A bitten carrot is bitten again only after it has grown back (bone-meal it).
- **R-9. Optional, no griefing.**
  - `/gamerule mob_griefing false`. Bone-meal every carrot to full grown, and wait three minutes: no carrot is bitten.
  - `/gamerule mob_griefing true`: the bites start again.

### The killer bunny

- **R-10. Its bite.**
  - Take off any armour. Stand still and run `/summon minecraft:rabbit ~ ~ ~8 {RabbitType:99}`.
  - A white rabbit with red eyes hops straight at you, never away. From about 4 blocks it leaps at you, with two jump sounds.
  - Its bite takes exactly 4 hearts on Normal. **Screenshot 31** (the bunny close, with your hearts).
  - After one bite, kill it with the iron sword (one full hit).
- **R-11. Its armour.**
  - Put on the four iron armour pieces and `/summon minecraft:rabbit ~ ~ ~8 {RabbitType:99}` again.
  - Select an empty hotbar slot and punch it without jumping, about a second apart. It needs five punches; a plain rabbit dies on the third.
  - After each punch it runs off for a moment, then comes back. In the armour its bites take about 2 hearts. Eat a golden apple if your health runs low.
- **R-12. Against a wolf.**
  - Run `/summon minecraft:rabbit ~ ~ ~20 {RabbitType:99}`, then `/summon minecraft:wolf ~ ~ ~23`.
  - The bunny is more than 16 blocks from you, so it doesn't pick you. It goes for the wolf, and the wolf goes for it.
  - They fight until one dies. One bunny bite (8) kills the wild wolf at once, but the wolf's bite can kill the bunny first.

  **Screenshot 32** (the fight). Kill the bunny that is left.
- **R-13. Shot from afar.**
  - Run `/summon minecraft:rabbit ~ ~ ~20 {RabbitType:99}` and `/summon minecraft:rabbit ~1 ~ ~20`. They are a killer bunny and a plain rabbit side by side.
  - Shoot the killer bunny with the bow: it comes for you. The plain rabbit never attacks.
  - Kill the bunny.

### Hunters, drops and deaths

- **R-14. Wolves and cats.**
  - Use the rabbit egg twice, then the wolf egg 5 blocks from the rabbits: the wolf kills one.
  - Use the rabbit egg twice somewhere else, then the cat egg 6 blocks from them. Walk 20 blocks away, because a wild cat runs from a player within 16.
  - Within a minute the cat creeps up on a rabbit, pounces and kills it with a bite. **Screenshot 33.**
  - Use the second cat egg and tame the cat with cod until it sits with a collar. Right-click it with an empty hand so that it stands.
  - Use the rabbit egg twice next to it and watch for a minute: the tame cat never goes for the rabbits.
- **R-15. A zombie.** Use a zombie egg 3 blocks from a rabbit: the rabbit hops away. Kill the zombie with the M4A1.
- **R-16. Drops and deaths.**
  - Kill rabbits with the sword. Each flashes red and tips over sideways as it dies. They drop raw rabbit, often a rabbit hide, and now and then a rabbit's foot.
  - Set the grass next to a rabbit on fire with flint and steel. If it dies burning, it drops cooked rabbit.
  - Walk at a rabbit so it flees in quick hops, and shoot it with the bow while it is in the air. As it flashes red and tips over, its legs finish the hop and then stay still. They do not keep kicking for the whole second it lies dying.
  - **Screenshot 34** (the dying rabbit, legs still).
  - Shoot a rabbit with the M4A1: it flashes red and dies.
- **R-17. Saving.**
  - Keep your armour on and an adult rabbit alive nearby. The killer bunnies are dead by now and the baby of R-4 may have grown, so run `/summon minecraft:rabbit ~2 ~ ~4 {Age:-24000}` for a baby and `/summon minecraft:rabbit ~ ~ ~8 {RabbitType:99}` for a killer bunny.
  - Save and quit at once, then load. The same three are there: the baby still small, the bunny still white with red eyes and still attacking.
  - Kill the bunny.

### Spawns and spawners

- **R-18. Snowy plains and deserts.**
  - `/gamemode creative` and `/time set day`, so the rabbits are easy to see. `/locate biome minecraft:snowy_plains`, then `/tp <x> 120 <z>` with the x and z it gives.
  - Fly over land you have not seen before. Groups of white rabbits, or white ones with black patches, appear: an adult with babies of its colour. **Screenshot 35.**
  - `/locate biome minecraft:desert` and go there the same way: the rabbits there are gold.
- **R-19. A spawner.**
  - Still in creative, place the spawner on grass and use a rabbit egg on it. A small rabbit turns slowly inside the cage, and rabbits spawn around it.
  - **Screenshot 36.** `/gamemode survival`.
- **R-20. Optional, an older world.** Load a world from release 3 or older that had rabbits, which stood still there. They hop now.

## P. Polar bears

World `r10polar`, single player, a fresh world. Survival, **Normal**, by day, on flat grass in a plains biome. Allow about 45 minutes.

Kit P (10 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:polar_bear_spawn_egg*8,minecraft:iron_sword,minecraft:shield,minecraft:bow,minecraft:arrow*32,minecraft:flint_and_steel,minecraft:golden_dandelion*2,minecraft:spawner,minecraft:golden_apple*8,minecraft:cooked_beef*32
```

Put the shield in your offhand (select it and press F).

- **P-1. The bear.** Use the polar bear egg 8 blocks away.
  - A big white bear stands on all fours, about 2.6 blocks from snout to rump and 1.4 tall, with a snout and two small ears and a wide round shadow.
  - Its legs swing as it walks, and its head turns to look at you. It leaves you alone.

  **Screenshot 37.**
- **P-2. A cub.**
  - Walk 20 blocks away from that bear. Stand still and run `/summon minecraft:polar_bear ~ ~ ~14`, then `/summon minecraft:polar_bear ~2 ~ ~14 {Age:-24000}`.
  - The cub has its own, rounder model: a big head, a short snout, stubby legs, about half a block tall. It has its own skin, so it is not a shrunken adult.
  - The cub stays near the adult. Its growls are higher than the adult's.

  **Screenshot 38** (the cub, from where you stand).
- **P-3. Rearing up.**
  - Walk toward the pair. Once you are within 10 blocks of the adult, it comes at you without being hit, because its cub is near.
  - Just before each swipe it rears onto its hind legs within a third of a second: body tilted up, front paws reaching forward, head high, about 3 blocks tall. It drops back onto all fours as it swipes.
  - You hear a growl about every 2 seconds. Each swipe takes 3 hearts, about once a second.
  - **Screenshot 39** (the bear standing).
  - Select the iron sword and raise the shield (hold right-click): the swipes land on the shield. With the M4A1 selected, right-click aims instead.
  - Kill the adult with the iron sword. It flashes red on each hit and tips over sideways as it dies. It drops 0 to 2 raw cod or salmon, and experience.
- **P-4. A lone bear fights.** Punch the bear from P-1 once: it comes at you. Punching never makes an adult run; it fights. Kill it.
- **P-5. A hit cub.**
  - Walk 20 blocks from the cub of P-2 and any other bear. Run these three commands, which summon an adult, a cub and a second adult 20 blocks away:

    ```
    /summon minecraft:polar_bear ~ ~ ~20
    /summon minecraft:polar_bear ~2 ~ ~20 {Age:-24000}
    /summon minecraft:polar_bear ~-4 ~ ~20
    ```

  - Shoot only the new cub with an arrow. It runs off fast, and both adults come for you.
  - Kill them.
- **P-6. Anger fades.**
  - Use a polar bear egg 8 blocks away and punch the bear once.
  - Sprint 30 blocks away and wait 40 seconds, then walk back to 15 blocks from it: it stays calm. Kill it.
- **P-7. Anger is saved.**
  - Use a polar bear egg 8 blocks away, punch the bear once, and at once save and quit. Load the world and walk to within 15 blocks of it: it comes at you again.
  - Switch to creative (`/gamemode creative`): it stops. `/gamemode survival`, then kill it.
- **P-8. Fire.** Use a polar bear egg and set the grass under the bear on fire with flint and steel: it runs while it burns. Kill it.
- **P-9. The golden dandelion.** `/summon minecraft:polar_bear ~ ~ ~4 {Age:-24000}` and right-click the cub with the golden dandelion: it is used, with its sound. Save, quit and load: it is still a cub.
- **P-10. A spawner.**
  - `/gamemode creative`. Place the spawner on grass and use a polar bear egg on it.
  - A small bear turns slowly inside the cage. At times its rump pokes a little out of the cage, as in vanilla. Bears spawn around the spawner.

  **Screenshot 40.**
- **P-11. Natural spawns.**
  - Still in creative, `/time set day`, so the bears are easy to see. Then `/locate biome minecraft:frozen_ocean` and `/tp <x> 100 <z>` with the x and z it gives.
  - Fly over sea ice you have not seen before. Here and there a big bear stands on the ice, sometimes with one or two cubs. **Screenshot 41.**
  - `/locate biome minecraft:snowy_plains` and fly over new land there: now and then a bear, alone or with cubs, among the rabbits.
  - `/gamemode survival`.

## F. Friends (two windows)

World: two windows, `-World r10mp4 -Fresh -HostSound`. Host H (`Sos`), friend F, both in survival, on flat open ground in the Overworld by day. Allow about 60 minutes.

Kit F, for both players (14 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*8,minecraft:carrot*16,minecraft:polar_bear_spawn_egg*2,minecraft:sculk_sensor*4,minecraft:sculk_catalyst*2,minecraft:cow_spawn_egg*8,minecraft:chest*2,minecraft:oak_door*2,minecraft:apple*16,minecraft:milk_bucket,minecraft:diamond_sword+sharpness=5,minecraft:cobblestone*64,minecraft:golden_apple*8,minecraft:cooked_beef*32
```

### The version check

- **F-1.** Once both have joined, F quits to the main menu and closes the friend window. Start the release 3 exe as the friend: `& scripts\mp-local.ps1 -Exe iw4l-survival-3fefa3f.exe -FriendOnly`.
  - It is refused at the join with "Incompatible client! Please use the host's build (Minecraft multiplayer protocol 9)". **Screenshot 42** (the friend window).
  - Close it and start this build's friend again: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly`. It joins.

### Darkness

- **F-2 (two windows). Each window its own.**
  - Host: `/effect give Friend minecraft:darkness 30`. F's view goes dark and pulses, and H's view does not change.
  - **Screenshot 43**, both windows.
  - Host: `/effect give @s minecraft:darkness 30`: now H's view goes dark too. The two pulses need not line up.
  - Both drink the milk bucket.
- **F-3 (two windows). Given again.**
  - Host: `/effect give Friend minecraft:darkness 13`. Count about 4 seconds, then run the same command again.
  - F's view stays dark until about 13 seconds after the second command, about 17 seconds in all, and F's darkness icon stays as long.
  - With release 3, F's darkness ended about 13 seconds after the first command.
  - Write one line with how long it lasted.

### Vibrations

- **F-4 (two windows). The friend's steps.**
  - H builds the sensor rig (Setup), then stays crouched or stands still on the start for the vibration steps, so that only F's actions light the line.
  - F walks about 4 blocks from the sensor. H's lamp lights, and both windows show the same ripple from F's feet.
  - F crouches and walks: no ripple.

  **Screenshot 44**, both windows, with the ripple.
- **F-5 (two windows). The friend's door, apple and gun.**
  - F places an oak door on the floor 5 blocks from the sensor, waits 2 seconds and opens it: H's line shows 10.
  - Host: `/effect give Friend minecraft:hunger 5 99`, so that F can eat (see "Eating" in Setup). F waits until the bar is no longer green, then 2 seconds more, and eats an apple next to the door: 8.
  - F walks crouched to 20 blocks from the sensor, stands up, waits 2 seconds and shoots the floor beside the sensor with the M4A1: 2.

  **Screenshot 45**, H's window, with any of these lit.
- **F-6 (two windows). A shared chest.**
  - H places a chest 5 blocks from the sensor and steps back, crouched.
  - F opens it: 10.
  - Two seconds later H opens it too, so both screens are open: nothing is heard. H closes it: nothing, because F still has it open.
  - F closes it: 9.
- **F-7 (two windows). Leaving with the chest open.**
  - F opens the chest again (10) and waits 2 seconds.
  - With the chest screen still open, F closes the friend window with its X button (or Alt+F4). Don't use Esc, which would close the chest first.
  - Within about 8 seconds H's chat shows "Friend left the game", and at that moment H's line shows 9.
  - **Screenshot 46**, H's window.
  - Start the friend again: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly`. F crouches as soon as the world shows: H's line stays dark. With release 3 the 9 came only now, when the friend came back.

### Shriekers and catalysts

- **F-8 (two windows). A shared warning.**
  - H walks crouched 20 blocks from the rig, so the sensor is out of the shrieker's reach.
  - H runs `/setblock ~ ~-1 ~5 minecraft:sculk_shrieker[can_summon=true]`, then `/tp Friend ~ ~ ~5`. F lands on the shrieker, 5 blocks from H.
  - F sees and hears it shriek, with the rings and the scream. About 4.5 seconds later a warden sound plays, and both views go dark.
  - **Screenshot 47**, both windows, while dark.
  - F steps off. H waits until 10 seconds have passed since the shriek, then walks onto the shrieker. It shrieks, and the warden sound is the closer one of a second warning: H's level rose with F's.
- **F-9 (two windows). A catalyst.**
  - H places a catalyst 30 blocks from the shrieker.
  - F uses a cow egg 3 blocks from it and kills the cow with the sword.
  - Both windows show the bloom, the soul wisps and the sculk spreading with its sparks.

  **Screenshot 48**, F's window.

### Rabbits and polar bears

- **F-10 (two windows). Rabbits.** H and F stand side by side.
  - H uses the rabbit egg three times 6 blocks away and runs `/summon minecraft:rabbit ~ ~ ~6 {CustomName:"Toast"}`.
  - F sees the same rabbits in the same colours, Toast among them.
  - F holds a carrot and stands still: the rabbits come to F and sit near F. Without the carrot they flee both players and keep hopping.
  - Put the two windows side by side: the rabbits hop and tilt their heads at the same moments in both. A rabbit tilts its head after about 10 seconds without a hop. **Screenshot 49**, both windows.
  - F puts the carrot away, walks at a rabbit so it flees, and shoots it with the M4A1. It dies in both windows, and its legs stop as it lies dying.
  - **Screenshot 50**, F's window.
- **F-11 (two windows). The killer bunny.**
  - H runs `/tp Friend ~ ~ ~5`, then `/summon minecraft:rabbit ~ ~ ~9 {RabbitType:99}`. F is the nearer player.
  - The bunny goes for F, and F hears its hops and bites.
  - **Screenshot 51**, F's window.
  - F kills it with the sword.
- **F-12 (two windows). A polar bear.**
  - H runs `/tp Friend ~ ~ ~5`, then `/summon minecraft:polar_bear ~ ~ ~10`. F sees the same bear.
  - F punches the bear: it goes for F.
  - Both windows show it rear up and drop down at the same moments. **Screenshot 52**, both windows, while it stands.
  - F kills it with the sword.
- **F-13 (two windows). A cub.**
  - H runs `/summon minecraft:polar_bear ~ ~ ~12 {Age:-24000}`. F sees the same cub, with its own model and skin.
  - **Screenshot 53**, F's window.
- Quit the friend first, then the host, through the menu.

## Known limits

These are known gaps, not new bugs.

- **The warden** (track W, segment w2) is not in this build: a fourth warning plays the angry warden sound, and nothing digs out.
- **Shriekers:**
  - A shrieker's darkness shows its icon. Vanilla hides it.
  - A dropped item never makes a shrieker shriek, because the fork's dropped items keep no thrower.
  - The rings rise through a ceiling, where vanilla's stop under it.
  - A shrieker broken or replaced during a command answers one tick later.
- **Vibrations not heard yet:**
  - **Blocks:** composters, cakes, candles put out by hand, decorated pots, chiseled bookshelves, jukeboxes, tripwire hooks and beehives. Players and mobs on pressure plates make none, because only dropped items press plates in the fork.
  - **Bodies and items:** chorus fruit (the fork does not teleport the eater), the flaps of bats, parrots and bees, bounces on slime blocks and beds, hurt item entities, armour stands, and mob armour changes.
  - **Projectiles:** witch potions, fireballs, shulker bullets, llama spit, and firework rockets in flight. A firework's burst is heard.
  - **Other features** (still left after W's merge): the ominous item spawner, lead knots (tied, punched or cut), silverfish breaking infested blocks, and villagers and piglins at doors.
  - **Rabbits:** a rabbit biting a carrot makes no vibration.
  - **Not stepped here:** raiders opening and closing doors and zombies breaking doors do make vibrations since W's merge. These steps leave them out, because a mob's own steps keep a nearby sensor busy.
- **Vibrations that differ from vanilla:**
  - **Saving:** a vibration saved in flight arrives up to 0.8 seconds later after the load than in vanilla, and its ripple is drawn once. A shrieker that gets such a vibration after a load stays quiet.
  - **Missing sources:** a wind charge's burst carries no thrower. A container closed by the game's own recheck (no player) makes no close.
  - **Different sources:** a bucket used on a cauldron is heard from the player; vanilla gives it no source.
  - **Hurts:** a hurt that absorption took entirely still counts when it came from MW2 damage. A mob's killing blow can lose its hurt (7), but its death (15) wins anyway.
  - **Riding:** only players getting on are heard. A rider dropped because its mount died makes none.
  - **Missed landings:** an arrow turned away by a creative player makes no landing. A bullet that hits a player makes no landing; the hurt player's own 7 is heard instead.
  - **Leaving:** when the friend who leaves with a chest open was the last player in that level, the close is heard when someone comes back, because the level stops.
- **Darkness:**
  - The fork has no gamma or darkness options, so both stay at vanilla's defaults.
  - Each window pulses on its own clock, so the host's and a friend's pulses don't line up. Vanilla is the same.
- **Catalysts and sculk:**
  - The ender dragon's experience is never taken by a catalyst.
  - Sculk sparks are drawn upright, without vanilla's turn on the top and bottom faces.
- **Redstone placement** (read from the code while writing these steps): a comparator, repeater, piston or calibrated sensor placed by hand keeps facing north. Say if one turns away from you after all.
- **Rabbits:**
  - **Effects not shown:** the hop raises no dust, and a carrot bite has no crumbs or break sound.
  - **Names:**
    - The fork has no name tags, so Toast comes only from `/summon` with `CustomName`.
    - Mob names are not drawn. The killer bunny's name is saved for vanilla but not shown.
  - **Other limits:**
    - A spawn egg can't carry a rabbit's colour.
    - A baby rabbit floats in water like an adult.
    - The killer bunny's sounds can't be turned down on their own, because the fork has no sound category sliders.
- **Polar bears:**
  - **Targets and movement:**
    - A bear may pick a target it cannot reach.
    - Water does not slow it.
  - **Body:**
    - A standing bear keeps its low box.
    - It is not immune to freezing.
  - **Saving:** the old `AngerTime` key is not read.
- **Other land animals:** fox, goat, parrot, panda, turtle, armadillo, sniffer and bee eggs still hatch nothing (M1's later segments).
- **Not in this build:** see the list at the top of this file.
- **Release 3's limits:** the limits at the end of [release-3-tests.md](release-3-tests.md), and through it those of releases 2 and 1, still apply, except:
  - darkness and sculk are in now (the warden is not);
  - rabbits and polar bears, the first M1 mobs, are in now;
  - trial spawner mobs, firework bursts, spawn eggs on trial spawners, raiders' doors and zombies breaking doors now make vibrations. Lead knots still don't.

## Screenshot numbering

- Name each screenshot by its number: `14.png`. When a step asks for both windows, name them `43-host.png` and `43-friend.png`.
- When a step fails, take a screenshot even if the step doesn't ask for one. Name it by its step label, e.g. `V-8.png` or `F-7-host.png`. Say in one line what you expected and what you saw.
- For the sensor steps, write the number of lit dust you saw when it differs from the step.
- For the sound steps (H-1 to H-8, F-3 and F-8), write one line per step with what you heard or how long it lasted.
- Send `latest.log` after any crash. In two windows, send both windows' logs (see Setup for where they are).
