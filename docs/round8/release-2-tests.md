# Round 8, release 2: test steps

Build: `iw4l-survival.exe` from `minecraft-survival` at `03e0c9be`, with tracks E and F of round 8. The release 1 exe is kept as `iw4l-survival-4d45e4f.exe`. The last test was [release-1-tests.md](release-1-tests.md) on 4d45e4fe.

## What is new

- The MW2 body, hands and guns are lit again in Minecraft worlds. They were black everywhere, by day too. On MW2 maps, models get their light grid and reflections back, and the MW2 sky shows instead of black.
- The spyglass and the spears show their flat inventory sprite as their icon.
- The spyglass zooms like vanilla: hold right-click for the zoom, the round overlay and its sounds.
- Cave spiders are real mobs, with their poison. Every spawn egg of a mob the game has hatches that mob (36 eggs), the ender dragon egg too. Cave spider spawners follow the monster light rules.
- G throws the lethal grenade and Shift+G the tactical, whatever you hold. B is free again. Any key can take a Shift chord in the controls menu. `settings.cfg` is now v6.
- Multiplayer: 2 to 4 friends join the host's world through the relay `iw4l-master.exe`, with a join code. Each player has their own inventory, XP, hunger and dimension.

## Setup

- Use a class with an assault rifle, a pistol, Frag and Flash.
- Each part names its world and its kit. Put the kit in the `.env` next to the exe as one line, `IW4L_MINECRAFT_KIT=...`, replacing the old line.
- A kit only fills the backpack of a player in a new world. In a world you have played before, type `/clear`, then `/kit`. `/kit` reads the `.env` again, so you don't have to restart.
- Remove `IW4L_MINECRAFT_DIMENSION` and `IW4L_MINECRAFT_SEED` from the `.env` unless a section asks for them.
- In every new world, type `/difficulty normal` and `/gamerule keep_inventory true` first.
- The log is `iw4l-artifacts\logs\latest.log`.

## F. The fixes from your test of section A

World: `IW4L_MINECRAFT_WORLD=r8f` (a new world, Overworld), single player. Start by day on open grass.

Kit F, for the whole part (27 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:torch*64,minecraft:elytra,minecraft:iron_chestplate,minecraft:spyglass,minecraft:spyglass,minecraft:iron_spear,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:golden_apple*16,minecraft:spawner,minecraft:milk_bucket,minecraft:iron_sword,minecraft:cave_spider_spawn_egg*16,minecraft:spider_spawn_egg*4,minecraft:zombie_spawn_egg*8,minecraft:ender_dragon_spawn_egg,minecraft:fox_spawn_egg,minecraft:wither_spawn_egg,minecraft:horse_spawn_egg,minecraft:donkey_spawn_egg,minecraft:iron_golem_spawn_egg,minecraft:villager_spawn_egg,minecraft:ghast_spawn_egg,minecraft:shulker_spawn_egg,minecraft:strider_spawn_egg
```

Seven eggs for F18 don't fit. Step F18 gives them with `/give`.

### The MW2 body and the guns (fix F1)

- **F1.** By day, hold the assault rifle. The gun and the gloves show their textures and shading. They were black. **Screenshot 1.**
- **F2.** Put on the iron chestplate and press F5. The MW2 body is lit, with the chestplate on it. Swap the chestplate for the elytra: the body is still lit, with the wings on its back. Press F5 again for the front view. **Screenshot 2.**
- **F3.** Build a cobblestone roof over you and type `/time set midnight`. Away from torches, the body and the gun are dark, as Minecraft mobs are there. Place a torch next to you: they brighten. Type `/time set day`.
- **F4.** Build a 4 x 5 obsidian portal, light it and go through. In third person, the body in the Nether is dimly lit like the terrain, warmer near lava, and never a black silhouette. **Screenshot 3.**
- **F5.** Go back through the portal to the Overworld.

### The spyglass and spear icons (fix F2a)

- **F6.** Open the inventory. The spyglass is the brass-and-glass sprite lying diagonally, as in vanilla. It is not a thin vertical strip. **Screenshot 4.**
- **F7.** Put it on the hotbar: the same sprite shows there. Hold it: in your hand it is still the small 3D spyglass.
- **F8.** The iron spear in the inventory is the diagonal spear sprite. **Screenshot 5.**

### The spyglass zoom (fix F2b)

In first person.

- **F9.** Select the spyglass and hold right-click.
  - The view zooms in quickly to about ten times.
  - The round spyglass overlay grows in, with black bars left and right.
  - The spyglass use sound plays.
  - The hands and the spyglass are not drawn, and you walk slowly.

  **Screenshot 6.**
- **F10.** Move the mouse while zoomed: the view turns much slower than normal.
- **F11.** Let go. The view zooms back out over a few frames, the overlay goes, and the stop sound plays.
- **F12.** Hold right-click again and press a number key for another slot. The zoom and the overlay end at once, with no stop sound.
- **F13.** Hold right-click and press F5: third person, with no zoom and no overlay. Press F5 twice to get back to first person while you still hold the button: the zoom and the overlay come back. **Screenshot 7** in third person.
- **F14.** Select the spyglass and press F: it goes to the offhand. Select an empty hotbar slot and hold right-click: it zooms from the offhand too.
- **F15.** Keep the spyglass in the offhand, select the gun and hold F. The gun is lowered and doesn't fire or aim while you are zoomed. Let go of F: the gun comes back.

### Spawn eggs and cave spiders (fix F3)

Normal difficulty, survival.

- **F16.** Type `/time set midnight`. Use a cave spider egg on the ground and a spider egg next to it. The cave spider is a small dark-blue spider, about two thirds of the spider's size, with red eyes. Each egg is used once per click. **Screenshot 8.**
- **F17.** Let a cave spider bite you. The hearts turn green and you lose health for about 7 seconds, but never below half a heart. **Screenshot 9** with the green hearts. Drink the milk to clear it.
  - Type `/difficulty hard` and let it bite again: the poison lasts about 15 seconds. Then type `/difficulty normal`.
  - Kill a cave spider with the sword: it drops string and sometimes a spider eye. In F5 it is drawn small too.
  - Type `/time set day`.
- **F18.** Use the horse, donkey, iron golem, villager, ghast, shulker and strider eggs from the kit. Then type these and use each egg:

  ```
  /give @s minecraft:wolf_spawn_egg
  /give @s minecraft:witch_spawn_egg
  /give @s minecraft:hoglin_spawn_egg
  /give @s minecraft:piglin_spawn_egg
  /give @s minecraft:mooshroom_spawn_egg
  /give @s minecraft:bat_spawn_egg
  /give @s minecraft:magma_cube_spawn_egg
  ```

  Each egg hatches its own mob. The hoglin and the piglin shake and then turn into a zoglin and a zombified piglin, as in release 1 step B35.
- **F19.** Use a zombie egg several times: a zombie comes out each time. Now and then a baby zombie comes with a chicken next to it.
- **F20.** Use the fox egg and the wither egg: nothing happens, and they stay in the hotbar. Those mobs are not in the game yet.
- **F21.** Optional: type `/gamemode creative`, open the spawn eggs tab and try any egg from this list. Each one hatches its mob: bat, blaze, bogged, cave spider, chicken, cow, creeper, donkey, ender dragon, enderman, ghast, hoglin, horse, husk, iron golem, magma cube, mooshroom, parched, pig, piglin, piglin brute, sheep, shulker, skeleton, slime, spider, stray, strider, villager, witch, wither skeleton, wolf, zoglin, zombie, zombie villager, zombified piglin. Then type `/gamemode survival`.

### Cave spider spawners (fix F3 and review R1)

Normal difficulty, survival, by day.

- **F22.** Place the spawner on grass in daylight and use a cave spider egg on it. The small cave spider turns in the cage. Wait a minute nearby: no cave spider comes out. **Screenshot 10.**
- **F23.** Build a dark room around it with cobblestone walls and a roof, and no torches. Wait inside, a few blocks away: cave spiders come out of the spawner. **Screenshot 11.**
- **F24.** Place torches on the inside walls all around the spawner, so the room is lit: no more cave spiders come out.

### The grenade keys (fix F4)

- **F25.** Quit the game. `iw4l-artifacts\settings.cfg` starts with `// IW4L user settings v6`. It has `bind g +frag`, `bind SHIFT+G +smoke` and `bind f mc_swaphands`, and no `bind b +smoke`. Start `r8f` again.
- **F26.** Options > Controls > Actions: "Throw Frag/Use Equipment" shows `g | R-BUMPER`, and "Throw Special Grenade" shows `SHIFT+G | L-BUMPER`. **Screenshot 12.**
- **F27.** Select the gun.
  - Press G: a frag flies.
  - Hold Shift and press G: a flash flies, and no frag.
  - Hold G to cook a frag, press Shift while you still hold G, then let go: the frag is thrown, not a flash.
- **F28.** Select cobblestone and press Shift+G. MW2's hand comes up with the flash and throws it, and the cobblestone comes back. **Screenshot 13** mid-throw. Select an empty slot: G throws a frag and Shift+G a flash.
- **F29.** Sprint with Shift+W and, still holding Shift, press G. The sprint ends and a flash flies. Let go of G and walk with Shift+W: you sprint as before.
- **F30.** Select cobblestone and press F: it goes to the offhand and nothing is thrown. Put the golden apples in the offhand, select the gun and hold F: you eat, as before.
- **F31.** Options > Controls > Actions: click "Throw Special Grenade", hold Shift and press H. The row shows `SHIFT+H`.
  - In game, Shift+H throws a flash, and Shift+G now throws a frag, because G has no chord any more.
  - Restart: the row still shows `SHIFT+H`.
  - Click the row again, hold Shift and press G to set it back.
- **F32.** Optional, with a controller: RB throws a frag and LB a flash, as before.

### The dragon egg

Last, because the dragon stays in this world.

- **F33.** Walk 200 blocks away from everything you built. Pillar 10 blocks up with cobblestone and use the ender dragon egg on a block, in open air. An ender dragon appears there, as with `/summon minecraft:ender_dragon`. **Screenshot 14.**

### On an MW2 map

- **F34.** Quit and start `iw4l-survival.exe map mp_rust`. The sky shows clouds instead of black, and the guns show reflections on their metal. **Screenshot 15.**
- **F35.** On the same map, G throws a frag and Shift+G a flash. **Screenshot 16.**

## B, C and D of release 1

You tested section A of [release-1-tests.md](release-1-tests.md) and kit A. Sections B, C and D of that document still apply to this build, unchanged, with the same worlds and kits. Keep their screenshot numbers (20 to 78). They don't clash with this document's numbers. Two things differ in this build:

- In D3, `settings.cfg` now starts with `// IW4L user settings v6`, not v5, and it has `bind SHIFT+G +smoke`.
- The MW2 body is lit now, for example in B40 (in bed) and C27 to C35 (the elytra). If it shows black, that is a failure of fix F1.

Send what you find there the same way as before; it goes into track F.

## M. Multiplayer on one PC

The full test is [track-e-tests.md](track-e-tests.md). Follow it from "Before you start" to the end of part 7. It is not copied here, so the two can't drift apart. In short:

1. **The relay.** `iw4l-master.exe` was built with this release and is already in the game folder, next to `iw4l-survival.exe`, so skip the relay build in step 1 of "Before you start" in track-e-tests.md. Run the script below from PowerShell in `C:\Dev\mashup-crafting`.
2. **The kit.** Put kit M in the game folder's `.env`, replacing kit F. Both players get it on their first join. Remove `IW4L_MINECRAFT_DIMENSION`, and keep the `.env` free of `IW4L_MASTER_HOST_NAME` and `IW4L_MASTER_JOIN`.
3. **Start both windows:**

   ```
   & scripts\mp-local.ps1 -Exe iw4l-survival.exe -World mp-r8 -Fresh -HostSound
   ```

   The script builds the relay's certificates the first time, starts the relay, then the host window, then the friend window with the room's code. It prints `room <code>`, then `both players are in the room`. The test world lives in `iw4l-artifacts\mp-local\host\`, so your own worlds are never touched.
4. **Keys.** Only the window in front gets your keys, so Alt-Tab between the windows. In the script's PowerShell window, `h <command>` runs an MW2 console command in the host window, `f <command>` in the friend window, and `quit` closes both.
5. **No builds while you test.** The script holds both cargo slots while the windows run.

Kit M (27 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:iron_pickaxe,minecraft:iron_sword,minecraft:cooked_beef*32,minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:white_bed,minecraft:chest,minecraft:oak_log*4,minecraft:furnace,minecraft:raw_iron*3,minecraft:coal*6,minecraft:experience_bottle*8,minecraft:cow_spawn_egg*4,minecraft:snowball*16,minecraft:ender_pearl*8,minecraft:water_bucket,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:elytra,minecraft:firework_rocket*8,minecraft:totem_of_undying,minecraft:powder_snow_bucket,minecraft:ender_eye*12,minecraft:splash_potion@harming,minecraft:splash_potion@poison,minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}]
```

The rest comes from `/give @a` lines inside the steps.

The parts of track-e-tests.md, with their screenshots. They are numbered with an S, so they never clash with the plain numbers of parts F and B to D:

| Part | Steps | Screenshots |
|---|---|---|
| 1. Joining, the world and chat | 1 to 10 | S1 to S3 |
| 2. The friend plays survival | 11 to 30 | S4 to S19 |
| 3. Seeing and hearing the world | 31 to 48 | S20 to S30 |
| 4. Fighting | 49 to 54 | S31 to S33 |
| 5. Different dimensions | 55 to 65 | S34 to S42 |
| 6. The M6 leftovers, the End and the credits | 66 to 83 | S43 to S57 |
| 7. Leaving, coming back and saving | 84 to 87 | S58 |

At the end of part 7, send the `peak working sets: ...` line from each PowerShell window.

## I. Over the internet with a friend

Part 8 of track-e-tests.md, with every detail in [friend-install.md](friend-install.md). Your friend needs their own MW2 and does part B of friend-install.md once. They unpack the zip, check the `.env`, start the game to the menu until the Minecraft files are in, and set their own player name.

Your side:

1. **Close the old relay.** Close every `iw4l-master.exe` left from part M in Task Manager. The one from `mp-local.ps1` only listens on your own PC.
2. **The relay and the certificates.** The relay is already in the game folder (part M step 1). Part M also made the certificates. If `iw4l-artifacts\mp-local\certs\iw4l-ca.pem` is missing, run `& scripts\mp-local.ps1 -CertsOnly`.
3. **The room password.** Add `IW4L_MASTER_PASSWORD=<room password>` to the game folder's `.env`.
4. **Open the port.**
   - In an administrator PowerShell:

     ```
     New-NetFirewallRule -DisplayName "iw4l-master UDP 4433" -Direction Inbound -Protocol UDP -LocalPort 4433 -Action Allow -Profile Private,Public
     ```

   - On the router, forward UDP port 4433 to this PC's IPv4 address (`ipconfig` shows it).
   - Compare the router's internet address with a "what is my IP" site. If they differ, your provider uses carrier-grade NAT and the port can't be reached; see friend-install.md part A step 5.
5. **The start files and the friend's zip.** Run both:

   ```
   & scripts\mp-local.ps1 -Bats -Exe iw4l-survival.exe
   ```

   ```
   & scripts\mp-local.ps1 -FriendKit -Exe iw4l-survival.exe -PublicAddress <your public IP>
   ```

   Send `iw4l-artifacts\mp-local\friend-kit-iw4l-survival.zip` to your friend.
6. **Host.** Set `IW4L_MINECRAFT_WORLD` in the `.env` to the world you want to share; a test world is safer the first time. Double-click `Host Minecraft World.bat`. Send your friend the code from "Friends join with the code [<code>]" in your chat. **Screenshot I1** of your chat.
7. **The friend joins** with `Join Minecraft World.bat` and the code. **Screenshot I2** from both PCs.
8. **Check together**, in this order, and screenshot what fails:
   1. you see each other move, and a block one of you breaks is gone for the other;
   2. chat works both ways, and after `/op <friend's name>` a command from the friend works;
   3. the friend's hotbar, hearts and hunger; a chest you both open; mobs near the friend;
   4. the friend goes to the Nether and back on their own while you stay;
   5. the friend quits and joins again with the same code: same place, same items;
   6. you quit through the menu: the friend's game goes back to the menu.

   If anything fails, send the `latest.log` of both PCs. The troubleshooting table at the end of friend-install.md lists the usual causes.

## Known limits

These are known gaps, not new bugs:

- **The spyglass:** in F5 the MW2 body doesn't raise it to its eye. There are no spyglass statistics or advancements.
- **Spawn eggs:** the eggs of the 52 mobs the game doesn't have yet do nothing and are kept. A zombie horse egg makes nothing, rider included.
- **Grenade keys:** only Shift chords exist. A `settings.cfg` saved by upstream's `iw4l.exe` drops the `SHIFT+G` line.
- **Friends** can't skate, see their held item and its swing in first person, hear music, trade, hit other players with Minecraft melee, use the creative inventory, or see the purple overlay while standing in a portal.
- The limits listed at the end of release-1-tests.md still apply, except "Multiplayer: not in this build".

## Screenshot numbering

- Name each screenshot by its number: `7.png` for part F and sections B to D, `S12.png` for part M, `I2.png` for part I.
- When a step fails, take a screenshot even if the step doesn't ask for one. Name it by its step label, e.g. `F27.png`, or `M58.png` for step 58 of track-e-tests.md. Say in one line what you expected and what you saw.
- Send `latest.log` after any crash, and from both PCs if part I fails.
