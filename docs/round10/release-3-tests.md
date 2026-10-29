# Round 10, release 3: test steps

Build: `iw4l-survival.exe` from `minecraft-survival` at `3fefa3f0`. It is release 2 plus track F's segments f1 and f2 (Margit), track E2's segment e2a, the trial spawner fix `674afcc0` and the protocol bump. The release 2 exe (`605ad21e`) is kept as `iw4l-survival-605ad21.exe`. The steps of [release-2-tests.md](release-2-tests.md) and [release-1-tests.md](release-1-tests.md) are not repeated here.

**Multiplayer is protocol 8** (release 2 was 7), because E2 changed the firework burst message (a burst now carries its stars). A friend on the release 2 exe is refused at the join with the version message, so the host and every friend need this build. `mp-local.ps1` runs it in both windows. The relay `iw4l-master.exe` was copied in again with this build. Its own protocol is unchanged. A friend over the internet needs a new zip from `-FriendKit` (see Setup).

**Send numbered screenshots.** Every **Screenshot N** below has its own number, from 1 to 59 through the whole document. Name each file by its number (`14.png`), or `51-host.png` and `51-friend.png` when a step asks for both windows. See "Screenshot numbering" at the end.

The whole test takes about 6 hours. The single-player parts M, S, T, P, D, X, L and K take about 5 hours 10 minutes and can be done in any order. The two-window part F takes about 50 minutes. Spread it over a few sittings. Steps marked **optional** can be skipped.

## What is in this build

Track F, segments f1 and f2: Margit, the Fell Omen, the first Elden Ring boss ([track-f-log.md](track-f-log.md), design in [../ELDEN-RING-BOSSES.md](../ELDEN-RING-BOSSES.md)):

- `/summon iw4l:margit` (Tab offers it) puts him asleep at that spot, which is his anchor. The chat says "Summoned new Margit, the Fell Omen". Peaceful refuses the summon and removes him.
- **He is not drawn yet.** You see only his round shadow, about 2 blocks wide, and you hear him. His model, his particles, the boss bar and the altar come with segment f3.
- **His strikes and daggers hurt nobody yet** (f3, group er-fight). You hear his moves and see his shadow move, but you lose no health and nothing flies.
- **Waking and walking:**
  - He wakes when a player comes within 24 blocks of his anchor (a golden mist sound), walks to you and circles you, and leaps at you from farther away.
  - He never goes more than 24 blocks from his anchor.
  - When nobody is within 40 blocks for 5 seconds, the fight resets: he walks home and sleeps.
  - If he can't reach you for 5 seconds, he fades in beside you. A player on a pillar gets daggers instead.
- **Hits:**
  - Swords, arrows, thrown tridents, TNT and fire hurt him. Player hits have no hurt cooldown.
  - Five full netherite sword hits in a row break his stance. He kneels for 4 seconds, and the next melee hit from the front is a critical.
  - Falls and potions don't hurt him, and blasts and pushes never move him.
  - MW2 bullets and blasts hit him too. For now they count as melee hits and level blasts; f3's group er-guns gives them their own rules.
- Iron golems, wolves and other mobs never pick him as a target.
- **Saving and death:**
  - He is saved in the world as a vanilla `minecraft:marker`. Loading resets his fight: he is back at full health and walks back to his anchor.
  - His death plays a deep evoker death sound. He drops nothing yet.

Track E2, segment e2a ([track-e2-log.md](track-e2-log.md)):

- **Thrown things:**
  - Snowballs, eggs, pearls, tridents, splash potions and wind charges hit every mob. The mobs that thrown things used to fly through are pillagers, vindicators, evokers, vexes, ravagers, withers, ghasts, blazes, striders, hoglins, zoglins, piglins and brutes.
  - A hit knocks the mob back along the throw like vanilla and makes it angry at the thrower.
  - Snowballs hurt blazes.
  - Wind charges leave spiders, zombies, skeletons and slimes calm, as vanilla does.
  - Farm animals panic when a thrown thing or an arrow hits them.
- When E2 was merged, silverfish, endermites and Margit were added to the same hit list ([decisions.md](decisions.md), last bullet), so thrown things hit them too.
- **Potions:**
  - Splash and lingering potions work on ghasts and blazes.
  - Harming heals every undead mob (zoglins and withers too), and healing hurts them.
  - Water bottles hurt blazes, striders and snow golems.
- Thrown things fly through the mount their thrower rides.
- **Doors:**
  - Zombies with `CanBreakDoors` break wooden and copper doors on Hard. They knock, cracks grow on the door, and then it breaks. Friends see the cracks too.
  - Vindicators in a raid open doors and shut them behind them.
- **Maps:**
  - Maps are kept in vanilla's map files.
  - Shipwreck, ocean ruin and camp chests roll explorer maps. The coastline is drawn into each map's data, and a claim keeps two camp maps from pointing at the same place.
  - **Holding a map shows nothing yet** (E2's next segment).
- **Fences, leads and cats:**
  - The hand aims at a fence by its post and arms, not as a full block.
  - Lead knots can be punched off or cut with shears.
  - A sitting cat keeps a chest shut, and cats won't sit on a chest that someone has open.
- **Fireworks:**
  - Rockets carry their stars. A starless rocket pops in grey puffs and hurts nobody.
  - A starred rocket bursts on the first mob or player in its path, hurts more for each star, knocks mobs back and makes animals panic.
  - Rockets launch from a clicked block, and dispensers shoot them level.
  - A player's rocket blames that player: it follows the pvp rule, and its death line is "went off with a bang".

Also in the build:

- `674afcc0`: trial spawners spawn the shared registry's mobs, so **silverfish trial spawners run** (release 2's T-3 listed them as dark).
- `3fefa3f0`: multiplayer protocol 8.

**Not in this build:**

- Margit's look, his hits on players, the MW2 gun rules, the boss bar and the altar (track F, segment f3).
- Firework star shapes, firework and shield crafting, bells rung by thrown things, held and filled-in maps, and thrown things hitting End crystals and the dragon (E2's segment e2b).
- Dyed horse armour on the horse, drawn banners and crossbows, swinging bells and llama spit (E1's segment e1b).
- Breezes (track D, segment d2).
- Darkness, sculk and the warden (track W).
- Silverfish hiding in stone (track D0, segment d0b).
- The M1 to M3 mobs, phantoms among them.

## Setup

- **The exe:** `iw4l-survival.exe` in `C:\Games\Mashup\2010-Rust-Rewrite-Mashup`, with the relay `iw4l-master.exe` next to it. Part F starts the release 2 exe `iw4l-survival-605ad21.exe` once, for the version check.
- **Class:** keep release 1's class, an **M4A1 with Red Dot Sight, a USP .45, Frag and Flash**. The M4A1 is used against Margit (M) and to kill mobs between steps.
- **Single player:** set `IW4L_MINECRAFT_WORLD=<world>` in the `.env` next to the exe, then start `iw4l-survival.exe map minecraft:overworld`. Single-player worlds are in `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\saves\<world>` (part X looks inside).
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
  IW4L_MINECRAFT_WORLD=r10margit
  IW4L_MINECRAFT_KIT=<the part's kit line>
  ```

  - An `IW4L_SKATE_ASSETS=...` line from release 1 can stay instead of `IW4L_SKATE=off`. Skating is not tested here.
  - Remove `IW4L_MINECRAFT_DIMENSION` and `IW4L_MINECRAFT_SEED`. Keep it free of `IW4L_MASTER_HOST_NAME` and `IW4L_MASTER_JOIN`.
  - In two windows the world comes from `-World`, and both windows read the kit line.
- **Kits:** each part has its own kit line in its own block. A kit fills only the backpack of a player new to the world. Every part starts a new world, so the kit comes with the first join.
  - **The first join also gives the class loadout,** to the host and to every friend: the M4A1 on hotbar slot 1, the USP .45 on slot 2, 32 frags and 32 flashes in the two grenade slots, and 90 rifle and 36 pistol rounds (three stacks).
  - The rounds go into the backpack, or onto hotbar slots 3 to 5 when the kit fills the backpack (only kit T does). So in every part except T, hotbar slots 3 to 9 and the offhand start empty.
  - To move a backpack item to the hotbar, open the inventory (E), hover the item and press the hotbar slot's number key.
  - In a world played before, `/clear` and `/kit` give the kit again, but no loadout.
- **Names:** `Sos` is the host's player name and `Friend` the friend's. Use your names if they differ.
- **In every new world**, the host types `/difficulty normal` and `/gamerule keep_inventory true` first. In two windows the host also types `/op Friend`.
- **Commands:** `/kill` only takes players, so the steps clear monsters with `/difficulty peaceful` and then `/difficulty normal`. Peaceful removes Margit too. Only the host gets Tab suggestions.
- **Keys:** E opens the inventory, Q drops one item and Ctrl+Q a whole stack, F swaps hands, T opens the chat for commands, F5 changes the camera.
- **Your position:** the game has no F3 screen. `/tp @s ~ ~ ~` leaves you where you stand and prints your position in the chat, for example "Teleported Sos to 12.912000, 64.000000, -40.087000" (X, Y, Z).
- **Aiming at Margit:** he is invisible, and his shadow marks his feet. His body is 1.4 blocks wide and 3 blocks tall, so aim 1 to 2 blocks above the middle of the shadow.
- **Logs:** single player, `iw4l-artifacts\logs\latest.log`. With the script, `iw4l-artifacts\mp-local\host\iw4l-artifacts\logs\latest.log` and `...\mp-local\friend\...`.
- Steps marked **(two windows)** need the friend. Only the window in front gets your keys.

## M. Margit, the Fell Omen

World `r10margit`, single player, a fresh world. Survival, Normal, by day, on flat open ground in the Overworld (plains are best), with at least 50 blocks of flat ground in one direction. Read your position with `/tp @s ~ ~ ~` (see Setup). Allow about 50 minutes.

Kit M (16 stacks; `cobblestone*128` gives two):

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:trident+loyalty=3,minecraft:snowball*16,minecraft:splash_potion@harming,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:shield,minecraft:iron_golem_spawn_egg*2,minecraft:wolf_spawn_egg*2,minecraft:bone*16,minecraft:cobblestone*128,minecraft:iron_pickaxe,minecraft:cooked_beef*32
```

Put the netherite sword, the bow, the trident, the snowballs, the TNT, the flint and steel and the cobblestone on hotbar slots 3 to 9. Swap the rest in as the steps need them.

Margit's strikes and daggers hurt nobody in this build, so you can stand next to him. Most of these steps are sounds: write one line per step with what you heard.

### Asleep, awake, the leash and home

- **M-1.** Type `/summon iw4l:` and press Tab: `iw4l:margit` is offered.
- **M-2. Asleep.**
  - Stand near the corner of a block, not its middle: run `/tp @s ~ ~ ~`, and the X and Z it prints end in about .1 or .9. Note your X and Z.
  - Run `/summon iw4l:margit ~ ~ ~30`. The chat says "Summoned new Margit, the Fell Omen".
  - A round shadow about 2 blocks wide lies on the ground 30 blocks away, in the direction where Z grows. That spot is his anchor: your X, and your Z plus 30.
  - He sleeps, so you hear nothing from him.

  **Screenshot 1** (the chat line and the shadow).
- **M-3. Awake.** Walk toward the shadow.
  - Once you are within about 24 blocks of it, a deep illusioner mirror sound plays as he rises (golden mist). From then on you hear a deep evoker mutter every few seconds.
  - About a second and a half after the rise, the shadow comes toward you at walking pace.
  - Walk around it at 6 to 10 blocks: it follows you and circles you, 2 to 5 blocks away.

  **Screenshot 2** (the shadow near you).
- **M-4. The leap.** Step back to about 12 blocks from the shadow, staying within 24 blocks of his anchor, and stand still.
  - Within a few seconds a ravager roar plays and the shadow jumps.
  - It lands 1 to 2 blocks in front of you with a mace smash and a low boom.
  - You lose no health.

  Watch for a minute: his other moves play their sounds too, and none of them hurts you.
- **M-5. The leash.** Go back to where you stood for the summon, 30 blocks from his anchor: run `/tp @s <X> ~ <Z>` with the X and Z you noted in M-2, and face the anchor. The shadow stops about 24 blocks from the anchor, about 6 blocks from you, and stays there while you stand. **Screenshot 3** (the shadow, with the chat line of the `/tp`).
- **M-6. The reset and the walk home.**
  - Run `/tp @s ~ ~ ~-15`: you are now 45 blocks from the anchor, on the far side from it. Wait 6 seconds.
  - Run `/tp @s ~ ~ ~15` to stand 30 blocks from it again: the shadow walks back to its anchor (or is already there) and goes quiet (asleep). There is no mirror sound and no jump of the shadow at the end.
  - **Screenshot 4** (the shadow on its anchor).
  - Once it has gone quiet, walk toward it: within 24 blocks the mirror sound plays and he rises again. (If you come within 24 blocks while he is still walking home, he turns on you at once, with no mirror sound.)
- **M-7. Optional, another corner.** `/difficulty peaceful` (the shadow goes), then `/difficulty normal`. Stand near another block corner (check with `/tp @s ~ ~ ~`), note your X and Z, and run `/summon iw4l:margit ~ ~ ~`. Walk 10 blocks away so the shadow follows. Then go 45 blocks from the spot where you summoned him (check with `/tp @s ~ ~ ~`), wait 6 seconds and come back to 30 blocks. He walks all the way home again, with no mirror sound at the end.
- **M-8. Peaceful.**
  - `/difficulty peaceful`: the shadow goes.
  - `/summon iw4l:margit ~ ~ ~5`: the chat says "Monsters cannot be summoned in Peaceful difficulty", and no shadow appears.
  - `/difficulty normal`.

### The fade and the pillar

- **M-9. The fade.** On flat ground with nothing nearby, stand still in the middle of a block and run these four commands. They build a cobblestone ring 6 blocks high, 4 blocks out from your block on every side:

  ```
  /fill ~-4 ~ ~-4 ~4 ~5 ~-4 cobblestone
  /fill ~-4 ~ ~4 ~4 ~5 ~4 cobblestone
  /fill ~-4 ~ ~-3 ~-4 ~5 ~3 cobblestone
  /fill ~4 ~ ~-3 ~4 ~5 ~3 cobblestone
  ```

  - Stay on your block in the middle, and run `/summon iw4l:margit ~ ~ ~8`. He appears outside the ring.
  - The mirror sound plays at once (he wakes, because you are within 24 blocks), but he cannot get in.
  - About 5 seconds later the mirror sound plays again, and the shadow appears inside the ring beside you.

  **Screenshot 5** (inside the ring, with the shadow).
- **M-10. The pillar.**
  - Break two blocks of the wall, one above the other, with the pickaxe and walk 6 blocks out of the ring. He is too wide for that gap, so if you take longer than about 5 seconds he fades out beside you with the mirror sound. That is fine.
  - Pillar straight up 5 blocks with the cobblestone: look down, jump and place a block under you, five times.
  - The shadow stays below and does not fade up to you.
  - Every few seconds an amethyst chime and a beacon sound play together: a golden dagger forms in his hand. On a pillar he only throws daggers, and they don't fly or hurt yet.
  - Break your way down. `/difficulty peaceful` (the shadow goes), then `/difficulty normal`.

### Hits, stance and the critical

- **M-11. The sword.** On open ground, `/summon iw4l:margit ~ ~ ~6`: you hear him rise.
  - Stand 2 to 3 blocks from the shadow and hit the space above it with the netherite sword at full charge (let the attack bar fill between hits).
  - Each hit lands, and a deep illusioner hurt sound plays, at most twice a second.

  **Screenshot 6** (while hitting).
- **M-12. Stance and the critical.**
  - Keep hitting at full charge without a pause of more than a few seconds.
  - Within about five hits a ravager's stunned sound plays: his stance broke, and the shadow stays still for 4 seconds while he kneels.
  - At once hit him once more from where you stand, in front of him: the critical sound plays.
- **M-13. Ranged hits.** Step back 10 blocks.
  - Shoot him with the bow: the hurt sound plays.
  - Throw the trident at him: the hurt sound plays and the trident flies back.
  - Throw a snowball at the space above the shadow: it breaks in the air on his invisible body (white crumbs) instead of flying on.
  - Throw the harming splash at him: no hurt sound, because potions don't hurt him.
  - Shoot him with the M4A1: the hurt sound plays on the hits.
- **M-14. TNT.**
  - Put a TNT on the ground right beside the shadow, light it with the flint and steel, and step back 8 blocks. He follows you, so he is still a few blocks from the TNT when it blows.
  - When it blows: the hurt sound plays, and the shadow stays as dark as before. It doesn't fade for a moment, as it would if the blast threw him up.

  **Screenshot 7** (right after the blast).
- **M-15. Optional, fire.** When he kneels (break his stance as in M-12), use the flint and steel on the grass under the shadow. While he stands in the fire, the hurt sound repeats about twice a second.
- **M-16. Mobs leave him alone.**
  - Use an iron golem egg 5 blocks from the shadow. For 30 seconds the golem never walks at the shadow and never swings at it.
  - Use a wolf egg and tame the wolf with bones (it sits with a red collar), then right-click it so it stands.
  - Hit the shadow with the sword a few times: the wolf does not attack him.
  - `/difficulty peaceful` (the shadow goes, the golem and the wolf stay), then `/difficulty normal`.

### Saving and death

- **M-17. Saving.**
  - `/summon iw4l:margit ~ ~ ~6` and wait until the shadow walks at you.
  - Save and quit through the menu, then load the world.
  - The shadow is there again, where it was. Loading resets him to full health. Because you are within 24 blocks of his anchor, he takes up the fight again at once, with no rise sound, and walks at you.
  - Hit him with the sword: the hurt sound plays.

  **Screenshot 8** (after the load, with the shadow). Then `/difficulty peaceful` and `/difficulty normal`.
- **M-18. Death.**
  - `/summon iw4l:margit ~ ~ ~10 {Health:20f}`: you hear him rise.
  - Step back to about 8 blocks, so the shadow walks toward you. Hit it twice with the netherite sword as it reaches you.
  - After the first hit you may hear an elder guardian's curse with a beacon sound: below half health he changes phase.
  - After the second hit a deep evoker death sound plays. The shadow stops at once where it was hit, does not glide on toward you, and is gone about a second later.
  - Nothing drops, and no experience comes.

  **Screenshot 9** (right after the death sound).

## S. Silverfish trial spawners

World `r10silvertrial`, single player, a fresh world. Survival, Normal, by day, on flat open ground in the Overworld. Put on the diamond armour first. Allow about 30 minutes.

Kit S (13 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:trial_spawner*2,minecraft:silverfish_spawn_egg*4,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:cobblestone*64,minecraft:ladder*32
```

- **S-1. A placed spawner.** Place a trial spawner on the ground, 3 blocks from you: it stays dark. Use a silverfish egg on it.
  - The egg is used up. A small grey silverfish turns slowly inside, with small orange flames now and then. Release 2 left it dark.
  - You stand within 14 blocks in its sight, so it wakes up at once: with the detect sound a column of orange-gold sparks rises from it.
  - Silverfish come out one every 2 seconds, each in a puff of smoke and flames. There are 6 in all, never more than 2 at once.

  **Screenshot 10** (the silverfish in the cage and one outside).
- **S-2.** Kill them with the diamond sword: each dies to one full hit. About 2 seconds after the last one dies, the shutter opens and one reward pops out on top, with a sound and a little burst of flames and smoke. Then the spawner goes dark and smokes. **Screenshot 11** (the reward on top).
- **S-3. In a trial chamber.**
  - `/locate structure minecraft:trial_chambers`. `/gamemode creative`, then `/tp <x> 100 <z>` with the x and z it gives.
  - Dig straight down until you reach the copper and tuff rooms, about y −40 to −20.
  - Walk through the rooms and look for a spawner with a small grey silverfish turning inside. The spawners for small mobs each hold one of slimes, cave spiders, baby zombies or silverfish. The silverfish one shows small orange flames and plays a low hum now and then. Release 2 left silverfish trial spawners dark.
  - **Screenshot 12** (the silverfish spawner).
  - If no spawner in this chamber shows a silverfish, try one more chamber (`/locate` from 300 blocks away), or skip S-3 and S-4.
- **S-4.** Step out of sight of that spawner and run `/gamemode survival`, then walk into its sight within 14 blocks.
  - It wakes up with the sparks and sends silverfish out one at a time. They bite for half a heart.
  - Kill them all: it ejects a reward.

  **Screenshot 13** (silverfish coming out of the chamber's spawner).
- **S-5.** Breeze trial spawners (in the rooms for ranged mobs) still stay dark, because breezes come with track D's next segment.

## T. Thrown things

World `r10throw`, single player, a fresh world. Survival, Normal, by day, on flat open ground in a plains biome. Allow about 50 minutes.

Kit T (27 stacks; `snowball*32` and `cobblestone*128` give two each):

```
IW4L_MINECRAFT_KIT=minecraft:trident+loyalty=3,minecraft:snowball*32,minecraft:egg*16,minecraft:wind_charge*16,minecraft:llama_spawn_egg,minecraft:zombie_spawn_egg*2,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg,minecraft:evoker_spawn_egg,minecraft:ravager_spawn_egg,minecraft:strider_spawn_egg*3,minecraft:ghast_spawn_egg,minecraft:spider_spawn_egg,minecraft:golden_helmet,minecraft:saddle,minecraft:cow_spawn_egg,minecraft:sheep_spawn_egg,minecraft:pig_spawn_egg,minecraft:bow,minecraft:arrow*16,minecraft:silverfish_spawn_egg*2,minecraft:endermite_spawn_egg*2,minecraft:cobblestone*128,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

The kit fills the backpack, so the loadout's rounds land on hotbar slots 3 to 5. Swap the throwables and eggs onto hotbar slots 6 to 9 as the steps need them.

- Hatch each mob 6 to 8 blocks away, unless the step says otherwise.
- When a step is done, kill its mob with the M4A1, or clear the monsters with `/difficulty peaceful` and then `/difficulty normal`.
- Hoglins and piglins are summoned with `IsImmuneToZombification`, because in the Overworld they would turn into a zoglin or a zombified piglin after 15 seconds.

### Every mob

- **T-1. A llama.** Use the llama egg and throw a snowball at it. The llama is knocked back along the throw, turns to you and spits at you. **Screenshot 14.**
- **T-2. A zombie.** Use a zombie egg and throw the trident at it. The zombie flashes red, is knocked back along the throw and comes at you. The trident flies back to you.
- **T-3. A pillager.** Use a pillager egg and throw a snowball at it. The snowball breaks on the pillager (white crumbs) and pushes it back. It flashes red but loses no health, as in vanilla, and the pillager comes at you. **Screenshot 15.**
- **T-4. A vindicator, an evoker and a vex.**
  - Use the vindicator egg and the evoker egg and throw an egg at each. Each egg breaks on them and pushes them back (sometimes a chick hatches). The vindicator's axe hits hard, so shoot it with the M4A1 once its egg has landed.
  - When the evoker summons vexes, throw a snowball at a vex in the air: it stops on the vex.

  **Screenshot 16.** Then `/difficulty peaceful` and `/difficulty normal`.
- **T-5. A ravager.** Use the ravager egg and throw the trident at it. It flashes red and hardly moves back. The trident flies back to you.
- **T-6. A hoglin and a piglin.**
  - `/summon minecraft:hoglin ~ ~ ~6 {IsImmuneToZombification:1b}` and throw the trident at it: it flashes red and charges you. Kill it.
  - Take any gold off. `/summon minecraft:piglin ~ ~ ~6 {IsImmuneToZombification:1b}` and throw a snowball at it: it gets angry and attacks you.

  **Screenshot 17.**
- **T-7. A wind charge.** Use the three other pillager eggs close together and throw a wind charge into the group: all three are flung away.
- **T-8. A strider and a ghast.**
  - Use a strider egg (on land it shivers but works) and throw a snowball at it: the snowball stops on the strider instead of flying through. Keep the strider for T-11.
  - Use the ghast egg under open sky and throw a snowball at the ghast: it stops on the ghast.
- **T-9. A calm spider.**
  - Use the spider egg. Throw a wind charge at it: it is flung back and flashes red, but it keeps wandering and does not come at you.
  - Then throw a snowball at it: now it comes at you.

  **Screenshot 18** (the spider wandering after the wind charge).
- **T-10. An angry piglin.** Put on the golden helmet. `/summon minecraft:piglin ~ ~ ~6 {IsImmuneToZombification:1b}` and throw a wind charge at it. Piglins are not on the calm list, so it gets angry and attacks you although you wear gold.

### From a strider's saddle

- **T-11.** Build a cobblestone ring 2 blocks high around the strider from T-8 (or a new one), 3 by 3 inside, so it can't wander. Put the saddle on it, then right-click it again with the hotbar slot the saddle was on, which is empty now, to get on.
  - Look straight down and throw a snowball. It flies through your strider and breaks on the ground at its feet (white crumbs under it).
  - Your strider does not flash and is not pushed.

  **Screenshot 19.**
- **T-12.** Use the last strider egg on the block right in front of your strider, inside the ring.
  - Still riding, look down at the front of the second strider and throw a snowball. It breaks on the second strider, which is pushed away from you.
  - Yours stays put.

  **Screenshot 20.**
- **T-13.** Look straight down and throw the trident. It passes your strider, sticks in the ground under it and flies back to you. Your strider does not flash red. Get off (crouch).

### Animals, silverfish and endermites

- **T-14. Panic.**
  - Use the cow egg and the sheep egg and throw a snowball at each. Each is knocked back and runs off at a gallop for a few seconds, as when you punch it.
  - Use the pig egg and shoot it once with a half-drawn bow: it squeals and runs off.

  **Screenshot 21** (an animal running).
- **T-15. The registry mobs.**
  - Use a silverfish egg and throw a snowball at the silverfish: it breaks on it and knocks it back. Release 2's snowballs flew through silverfish.
  - Use an endermite egg and throw the trident at it: it dies at once and leaves experience, and the trident flies back.
  - Use the second silverfish egg and the second endermite egg. Throw an egg at the silverfish and a snowball at the endermite: each breaks on them and pushes them back.

  **Screenshot 22** (a snowball breaking on a silverfish, or right after). Kill them.

## P. Potions, blazes and snow golems

World `r10potion`, single player, a fresh world. Survival, Normal, by day, on flat open ground in a plains biome (not a desert, savanna or badlands, where snow golems melt). Drink a fire resistance potion before the blaze steps, because blazes shoot fireballs. Pen each mob in with a ring of cobblestone 2 high. Kill each mob with the M4A1 or the iron sword when its step is done. Allow about 30 minutes.

Kit P (22 stacks; each potion is its own stack, and `snowball*32` and `cobblestone*128` give two each):

```
IW4L_MINECRAFT_KIT=minecraft:splash_potion@poison,minecraft:splash_potion@harming,minecraft:splash_potion@harming,minecraft:splash_potion@harming,minecraft:splash_potion@healing,minecraft:splash_potion@water,minecraft:splash_potion@water,minecraft:snowball*32,minecraft:blaze_spawn_egg*4,minecraft:ghast_spawn_egg,minecraft:zoglin_spawn_egg,minecraft:strider_spawn_egg,minecraft:snow_golem_spawn_egg*2,minecraft:pig_spawn_egg,minecraft:cobblestone*128,minecraft:potion@fire_resistance,minecraft:potion@fire_resistance,minecraft:iron_sword,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

- **P-1. Snowballs on a blaze.** Use a blaze egg inside a pen. Throw snowballs at it one by one, about a second apart. Every hit flashes the blaze red, and about 7 hits kill it. **Screenshot 23.**
- **P-2. A snow golem's snowballs.** Use a snow golem egg 6 blocks from a new blaze. The golem's snowballs hit the blaze and flash it red. **Screenshot 24.** Kill the blaze if the golem doesn't.
- **P-3. Poison.** Use a blaze egg and throw the poison splash at it. Green swirls rise from it, and it flashes red every second or so until it has 1 heart left: poison never kills. **Screenshot 25.**
- **P-4. Harming on a ghast.** Use the ghast egg under open sky and throw a harming splash at the ghast. It flashes red and starts shooting fireballs at you. **Screenshot 26.**
- **P-5. Harming on a hoglin.** `/summon minecraft:hoglin ~ ~ ~6 {IsImmuneToZombification:1b}` and throw a harming splash at it: it flashes red and charges you.
- **P-6. A zoglin.**
  - Use the zoglin egg in a pen and throw the third harming splash at it. It shows no red flash and loses no health, because harming heals it.
  - **Screenshot 27.**
  - Throw the healing splash at it: now it flashes red and charges you.
- **P-7. Water bottles.**
  - Use a blaze egg, the strider egg and a snow golem egg next to each other in one pen.
  - Throw a water splash between them. All three flash red once and are pushed a little away from the splash, and the blaze then comes at you.
  - **Screenshot 28.**
  - Use the pig egg 3 blocks away and throw the second water splash next to it: the pig does not flash.

## D. Doors

World `r10doors`, single player, a fresh world, on flat grass in a plains biome. The zombie steps can be anywhere; D-9 travels to a village. Allow about 55 minutes.

Kit D (18 stacks; `oak_planks*128` gives two):

```
IW4L_MINECRAFT_KIT=minecraft:villager_spawn_egg*4,minecraft:oak_door*8,minecraft:copper_door*4,minecraft:iron_door*2,minecraft:oak_planks*128,minecraft:oak_fence*16,minecraft:glass*16,minecraft:torch*16,minecraft:diamond_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:ominous_bottle*2,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

### Zombies (in creative)

In creative the zombie leaves you alone and goes for the villager.

**Setup:**

- `/gamemode creative`, `/difficulty hard`, `/time set night`.
- Build a closed room of oak planks, 5 by 5 inside and 3 high, with a roof. Put one oak door in the middle of one wall.
- Inside, put 8 oak fences in a ring round the middle block and use a villager egg on that block (the fences keep the villager in).
- Open the door. Stand just outside it with your back to it, looking straight away from the house.

If the villager dies or turns into a zombie villager during these steps, kill what is left of it and use a new villager egg.

- **D-1.** Run `/summon minecraft:zombie ^ ^ ^12 {CanBreakDoors:1b,PersistenceRequired:1b}`, which puts a zombie 12 blocks ahead of you.
  - The zombie sees the villager through the doorway and walks toward the house.
  - Turn round and close the door before the zombie gets there. If it walks in first, kill it with the sword, open the door and try again.
- **D-2. The break.**
  - The zombie walks into the closed door and presses against it. Now and then a loud wooden knock plays and it swings its arms.
  - Cracks spread over the upper half of the door and grow. **Screenshot 29** (the cracks, about half way).
  - About 12 seconds after the first knock, the door breaks with a loud crunch, wood bits fly, and an oak door item drops.
  - The zombie walks in toward the villager. **Screenshot 30** (the doorway with the dropped door).
  - Kill the zombie with the sword.
- **D-3. Copper and iron.**
  - Put a copper door in and repeat D-1 and D-2: it breaks the same way and drops a copper door.
  - An iron door can't be opened by hand, so leave the doorway empty and run the summon of D-1. While the zombie walks toward the house, place an iron door in the doorway (it is placed shut). The zombie comes up to the door. There are no knocks and no cracks, and it stays outside. **Screenshot 31.** Kill it and break the iron door.
- **D-4. Normal.** Put an oak door in and repeat D-1.
  - Once the cracks show, run `/difficulty normal`: the knocking stops, the cracks vanish at once, and the zombie only presses against the door.
  - Run `/difficulty hard`: after a moment it knocks again, and the door breaks about 12 seconds later. Kill the zombie.
- **D-5. Killed mid-break.** Put an oak door in and repeat D-1. You are outside with the zombie. Once the cracks show, hit the zombie with the sword until it dies: the cracks vanish with it.
- **D-6. Mob griefing.** `/gamerule mob_griefing false` and repeat D-1 with an oak door in place: the zombie presses against the door and never knocks. Kill it. `/gamerule mob_griefing true`.
- **D-7. A plain zombie.** Repeat D-1 with `/summon minecraft:zombie ^ ^ ^12` (no tag): it stays outside the shut door and never knocks. Most zombies from eggs and from the night are like this. Kill it.
- **D-8. Saving.**
  - Summon a door breaker as in D-1 and close the door, then save and quit before it breaks the door, and load the world again.
  - Open the door for a moment so the zombie sees the villager, then close it: it knocks and breaks the door again. The tag is saved, the target is not, as in vanilla.
  - Kill the zombie.

### Vindicators in a raid (survival)

- **D-9. A raid.**
  - Put on the diamond armour. `/gamemode survival`, `/difficulty hard`, `/time set day`.
  - Run `/locate structure minecraft:village_plains`, then `/gamemode creative` and `/tp <x> 100 <z>` with the x and z it gives. Land in the village and `/gamemode survival`.
  - Pick a house with a wooden door and a window.
  - Walk at least 70 blocks out of the village, drink an ominous bottle (Bad Omen) and walk back in. The Bad Omen turns into Raid Omen, and about 30 seconds later the raid starts and its bar shows at the top.
- **D-10. Opened, not broken.**
  - Go into the house, shut its door and stand 3 blocks inside, where raiders can see you through the window. Fight the first wave from inside or outside as you like.
  - From wave 2 on, vindicators come with axes. A vindicator that runs at you swings the door open with the door's sound, comes in, and the door swings shut behind it with the shut sound. **Screenshot 32** (a vindicator in the open doorway).
  - Watch the door when a vindicator reaches it: now and then a crack flashes on its upper half for a moment just before it opens. The door never breaks while it can be opened.
- **D-11. No raid.**
  - Win the raid, or leave it running, and go at least 110 blocks from the village.
  - `/gamemode creative` and build a 5 by 5 plank room with a roof, one oak door, and a glass window next to the door. `/gamemode survival`, go in, shut the door and stand 3 blocks from it.
  - From in there a spawn egg doesn't reach outside, so summon the vindicator: face the door, look level and run `/summon minecraft:vindicator ^ ^ ^9`. A vindicator with an axe appears 6 blocks outside the door. It sees you through the window, walks round the house looking for a way in, and never opens the door. **Screenshot 33.**

## X. Explorer maps

World `r10maps`, single player, a fresh world. Creative is fine for this whole part: it makes the travel and the digging quick, and chests roll their loot the same way. Allow about 40 minutes.

Kit X (12 stacks; each potion is its own stack):

```
IW4L_MINECRAFT_KIT=minecraft:potion@long_water_breathing,minecraft:potion@long_water_breathing,minecraft:potion@long_water_breathing,minecraft:potion@long_night_vision,minecraft:potion@long_night_vision,minecraft:turtle_helmet,minecraft:oak_boat,minecraft:diamond_pickaxe,minecraft:iron_shovel,minecraft:torch*64,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

Holding a map shows nothing yet (E2's next segment), so these steps check the chests and the map files.

- **X-1. A shipwreck.**
  - `/gamemode creative`, `/locate structure minecraft:shipwreck`, then `/tp <x> 80 <z>` with the x and z it gives. Drink a night vision potion and dive to the wreck.
  - Open its chests. The map chest is usually in the cabin at the back. It holds paper, feathers, a compass, a map or a clock, and now also a **Buried Treasure Map**. Hover the map to see its name.
  - **Screenshot 34** (the chest screen with the map's name).
  - If the wreck has no map chest (half wrecks do not), fly 300 blocks away and locate the next one.
- **X-2. An ocean ruin.**
  - `/locate structure minecraft:ocean_ruin_warm` (or `minecraft:ocean_ruin_cold`), `/tp` there and open the ruin chests.
  - They roll coal, wheat, gold nuggets and the like again, instead of nothing. About 4 in 10 also hold a Buried Treasure Map.
  - **Screenshot 35** (a ruin chest; one with a map if you find it).
- **X-3. A camp.**
  - `/locate structure #minecraft:abandoned_camp` and `/tp` there.
  - Open the camp's chests. A common chest now rolls its loot (arrows, bones, candles and the like), and often a camp map named after another biome's camp, such as a **Swamp Camp Map**. **Screenshot 36.**
- **X-4. The secret chest.**
  - Dig around the camp for its secret chest. It can hold an explorer map such as a **Woodland Mansion Map**, a **Buried Trial Chambers Map** or a **Buried Ancient City Map**.
  - Opening it may pause the game for up to 2 seconds while the map searches.
  - **Screenshot 37.** Optional: try a second camp if the first secret chest has no map.
- **X-5. The files.** Save and quit. Open `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\saves\r10maps` in Explorer.
  - `data\minecraft\maps` holds `last_id.dat` and one numbered file per map found, starting at `0.dat`. **Screenshot 38** (that folder).
  - `dimensions\minecraft\overworld\data\iw4l\map_claims.dat` is there, because a camp map was found. Shipwreck and ruin treasure maps alone do not make this file. **Screenshot 39** (that folder).

## L. Fences, lead knots and cats on chests

World `r10leads`, single player, a fresh world. Survival, Normal, by day, on flat grass, with no beds or furnaces nearby (cats sit on those too). Allow about 30 minutes.

Kit L (17 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:lead*16,minecraft:cow_spawn_egg*4,minecraft:oak_fence*16,minecraft:nether_brick_fence*8,minecraft:stone*16,minecraft:iron_pickaxe,minecraft:iron_sword,minecraft:shears,minecraft:shears,minecraft:wheat*16,minecraft:cat_spawn_egg*4,minecraft:cod*64,minecraft:chest*4,minecraft:trapped_chest*2,minecraft:copper_chest*2,minecraft:ender_chest,minecraft:cooked_beef*16
```

### Fences and knots

- **L-1. Aiming past a post.** Place a single oak fence post with a stone block right behind it. Aim just past the side of the post at the stone: the stone gets the outline and is mined with the pickaxe, not the fence. **Screenshot 40** (the outline on the stone beside the post).
- **L-2.** Place a row of three oak fences. The middle one's outline is the post with an arm to each side, not a full block.
- **L-3. Punching a knot.** Use a cow egg twice. Put a lead on each cow and right-click an oak fence post: a knot appears on the post with two ropes.
  - Stand 2 blocks from the fence. Aim at the knot from the side (the upper half of the post) and left-click once with the sword.
  - The knot vanishes with the untie sound. A lead drops by each cow, and both cows are free.
  - The fence is still there, and the hunger bar has not moved.

  **Screenshot 41** (the fence without a knot and the leads on the ground).
- **L-4.** Tie a cow to the fence again. Hold left-click on the bottom third of the post, below the knot: the fence is dug as before, and the cow goes free within 5 seconds.
- **L-5. Creative.** `/gamemode creative`. Tie a cow to a fence and left-click the knot: the knot goes, the fence stays and a lead drops. `/gamemode survival`.
- **L-6. Saving.** Tie a cow, punch the knot off, then save and quit and load: the cow is loose and there is no knot.

### Shears

- **L-7.** Tie two cows to a nether brick fence. Hold fresh shears and right-click the knot from the side.
  - The snip sound plays, two leads drop and the knot is gone. There is no untie sound.
  - The shears now show a durability bar: one point of 238 is used, so the bar itself still looks full.

  **Screenshot 42.**
- **L-8.** Right-click a fence with no knot with the shears: nothing is cut and the shears don't wear. Then lead a cow and right-click the fence with an empty hand: it ties as before.
- **L-9.** Lead one cow yourself and tie another cow to a fence. Hold the shears and right-click the knot: the knot's cow is cut loose, and your own cow stays on your lead.

### Cats on chests

- **L-10.** Use a cat egg and tame the cat with cod until it sits with a collar. Right-click it with an empty hand so it stands.
- **L-11.** Place two chests side by side (a double chest) 2 blocks from the cat. Within about 10 seconds it walks onto one half and sits.
  - Right-click the other half: your arm swings, the chest stays shut and makes no sound.
  - Right-click the half under the cat: the same.

  **Screenshot 43.**
- **L-12.** Right-click the cat twice (it sits, then stands) and walk 15 blocks away so it follows you off the chest. Go back and open the chest: it opens. Break both chests and pick them up.
- **L-13. Other chests.** Cats only sit on plain chests by themselves, so put one on the others by command.
  - Place a trapped chest, jump onto it, run `/summon minecraft:cat ~ ~ ~ {Sitting:1b}` and step off. The trapped chest stays shut.
  - Do the same on a copper chest: it stays shut.
  - Do the same on an ender chest: it still opens, because vanilla checks only the block above an ender chest.
- **L-14. An open chest.** Walk 20 blocks away from those chests with your tame cat following, and make it stand.
  - Place a chest 2 blocks from the cat, open it and stay in its screen: for 20 seconds the cat does not climb onto it.
  - **Screenshot 44** (the cat beside the open chest).
  - Close it: within about 10 seconds the cat walks onto it and sits.

## K. Fireworks

World `r10rocket`, single player, a fresh world. Survival, Normal, by day, on open flat ground. Allow about 25 minutes.

Kit K (13 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:firework_rocket*64,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*64,minecraft:firework_rocket[fireworks={flight_duration:2,explosions:[{shape:star,colors:[I;14602026],fade_colors:[I;2437522]},{shape:creeper,colors:[I;4312372],has_twinkle:1b}]}]*64,minecraft:firework_rocket[fireworks={flight_duration:0,explosions:[{shape:burst,colors:[I;15790320,11743532]},{shape:small_ball,colors:[I;2437522]},{shape:large_ball,colors:[I;14602026]}]}]*64,minecraft:zombie_spawn_egg*16,minecraft:cow_spawn_egg*8,minecraft:iron_golem_spawn_egg*2,minecraft:elytra,minecraft:dispenser*4,minecraft:lever*4,minecraft:cobblestone*64,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

The four rocket stacks fill the first four backpack slots in the kit's order. Move them to the hotbar in that order:

- slot 3: plain rockets;
- slot 4: red rockets, one star;
- slot 5: two-star rockets, yellow and green, flight 2;
- slot 6: three-star rockets, white, red, blue and yellow, flight 0.

Until E2's next segment, every starred burst draws the same spark burst in all its stars' colours, with the bang.

- **K-1. A plain rocket.** Right-click the grass 3 blocks ahead with a plain rocket.
  - A rocket rises from just above the click, with its launch whoosh and a trail.
  - After 1 to 1.5 seconds it pops in 2 to 4 grey puffs, with no bang.
  - The stack shows 63.

  **Screenshot 45.**
- **K-2. Starred rockets.**
  - Place a red rocket the same way: it rises and bursts into red sparks with a bang. **Screenshot 46.**
  - Place a two-star rocket: it climbs higher before it bursts.
  - Place a three-star rocket: it bursts lower.
- **K-3. Creative keeps it.** `/gamemode creative` and place a plain rocket: it flies, and the stack keeps its count. `/gamemode survival`.
- **K-4. Gliding still boosts.** Put on the elytra. `/tp @s ~ ~30 ~` and press Space while falling to glide. Right-click with a plain rocket while you look at the ground below: you boost, and no rocket rises from the ground. Land and take the elytra off.
- **K-5. A burst near a zombie.** Build a 3 by 3 cobblestone roof 3 blocks above the ground on four pillars.
  - Hatch a zombie under it. When it is 2 blocks from you, right-click the ground under the roof next to it with a red rocket.
  - The rocket meets the roof at once and bursts. The zombie flashes red and hops away from the burst.
  - You lose about two hearts, because you are within 5 blocks and your own rocket hurts you.

  **Screenshot 47.** Kill the zombie.
- **K-6. Animals.** Hatch 3 cows under the roof and burst a red rocket among them the same way: all three are hurt, are pushed away from the burst and run in panic.
- **K-7. An iron golem.** Hatch an iron golem under the roof and burst a red rocket next to it.
  - It flashes red and does not move.
  - It turns on you, because a player's rocket blames the player.

  **Screenshot 48.** Run, then kill it with the M4A1 or `/gamemode creative` until it walks off, then `/gamemode survival`.
- **K-8. A dispenser.**
  - A placed dispenser faces the player who places it. Stand at one end of a flat 10-block lane with your back to the lane and look straight ahead.
  - Look down only a little, at the ground 2 blocks in front of you (if you look down steeply, it faces up), and place the dispenser there: its hole faces you and the lane behind you.
  - Put a lever on its side and 4 red rockets in it. Hatch a zombie 7 blocks down the lane, and stand beside the dispenser, not in front of it.
  - Flip the lever on. A rocket shoots out level, with no arc, and a sharp shoot sound instead of the click.
  - It bursts on the zombie, which flashes red and hops back. The dispenser holds 3 rockets.

  **Screenshot 49.**
- **K-9.** Take the red rockets out and put plain rockets in. Flip the lever off and on: the rocket flies level 10 to 15 blocks and puffs in grey smoke, with no bang.

## F. Friends (two windows)

World: two windows, `-World r10mp3 -Fresh -HostSound`. Host H (`Sos`), friend F, both in survival, on flat open ground in the Overworld by day. Allow about 50 minutes.

Kit F, for both players (18 stacks; `oak_planks*128` gives two):

```
IW4L_MINECRAFT_KIT=minecraft:villager_spawn_egg*2,minecraft:oak_door*4,minecraft:oak_planks*128,minecraft:oak_fence*16,minecraft:lead*8,minecraft:cow_spawn_egg*4,minecraft:shears,minecraft:cat_spawn_egg*2,minecraft:cod*32,minecraft:chest*4,minecraft:firework_rocket*16,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*16,minecraft:firework_rocket[fireworks={flight_duration:0,explosions:[{shape:burst,colors:[I;15790320,11743532]},{shape:small_ball,colors:[I;2437522]},{shape:large_ball,colors:[I;14602026]}]}]*16,minecraft:cobblestone*64,minecraft:diamond_sword,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

The rockets are in the backpack in the kit's order: plain, red (one star), then three-star.

### The version check

- **F-1.** Once both have joined, F quits to the main menu and closes the friend window. Start the release 2 exe as the friend: `& scripts\mp-local.ps1 -Exe iw4l-survival-605ad21.exe -FriendOnly`.
  - It is refused at the join with "Incompatible client! Please use the host's build (Minecraft multiplayer protocol 8)". **Screenshot 50** (the friend window).
  - Close it and start this build's friend again: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly`. It joins.

### Door cracks

- **F-2 (two windows).** Host: `/gamemode creative`, `/gamemode creative Friend`, `/difficulty hard`, `/time set night`.
  - H builds the plank room with the penned villager and the oak door, as in part D's setup, and stands outside the open door as there.
  - F stands outside the house within 30 blocks of the door, where F can see it.
  - H runs `/summon minecraft:zombie ^ ^ ^12 {CanBreakDoors:1b,PersistenceRequired:1b}` and closes the door before the zombie arrives.
  - F sees the zombie swing, sees the cracks grow on the door and the wood bits when it breaks, and hears the knocks and the break.

  **Screenshot 51**, both windows, while the cracks show. F picks up the dropped door, and the zombie can be killed with the sword. Then the host runs `/gamemode survival`, `/gamemode survival Friend`, `/difficulty normal` and `/time set day`.

### Knots and cats

- **F-3 (two windows).** H ties a cow to an oak fence with a lead. F punches the knot from the side with the sword. Both windows show the knot vanish within half a second and a lead drop. **Screenshot 52**, F's window.
- **F-4 (two windows).** F ties F's own cow to a fence and cuts the knot with F's shears. H sees the knot go and the lead drop, and F's fresh shears show a durability bar. **Screenshot 53**, F's window.
- **F-5 (two windows).**
  - H uses a cat egg, tames the cat with cod and makes it stand.
  - H places a double chest 2 blocks from the cat. Within about 10 seconds the cat sits on one half.
  - F right-clicks the chest: it stays shut for F too, and F's arm swings.

  **Screenshot 54**, F's window. H breaks both chests.
- **F-6 (two windows).**
  - 20 blocks from there, with H's cat standing nearby, F places a chest and keeps its screen open while H watches the cat: the cat does not sit on it.
  - F closes the chest: within about 10 seconds the cat sits on it.

  **Screenshot 55**, H's window, while F has the chest open.

### Fireworks

- **F-7 (two windows). Bursts for the friend.** F stands 6 blocks from H and watches H.
  - H places a plain rocket on the ground: F sees it rise and pop in grey puffs, with no bang.
  - H places a red rocket: F sees red sparks and hears the bang.
  - H places a three-star rocket: F sees sparks in white, red, blue and yellow.

  **Screenshot 56**, F's window, during a starred burst.
- **F-8 (two windows). Without pvp.**
  - H builds a 3 by 3 cobblestone roof 3 blocks above the ground on four pillars, and uses a cow egg under it.
  - Host: `/gamerule pvp false`. F stands under the roof next to the cow.
  - H places a three-star rocket on the ground next to F. F takes no damage and is not pushed, and neither is H. The cow is hurt, pushed away and runs in panic.

  **Screenshot 57**, both windows.
- **F-9 (two windows). With pvp.** Host: `/gamerule pvp true`. H places three-star rockets next to F under the roof until F dies.
  - Each burst deals F about 7 (three and a half hearts), so three rockets on full health.
  - F loses hearts and is pushed away from each burst.
  - The death reads "Friend went off with a bang", and the kill feed names the host.

  **Screenshot 58**, both windows. F respawns (the inventory is kept).

### Margit for the friend

- **F-10 (two windows, optional).** The track only built a friend's copy of Margit and did not try it in two windows, so note what F sees and hears.
  - On open ground, with both players 10 blocks apart, the host runs `/summon iw4l:margit ~ ~ ~10`.
  - Both hear him rise. F sees the round shadow walk and circle one of the players.
  - F hits the space above the shadow with the diamond sword: both hear the hurt sound.
  - **Screenshot 59**, both windows.
  - Host: `/difficulty peaceful`. The shadow goes in both windows. Then `/difficulty normal`.
- Quit the friend first, then the host, through the menu.

## Known limits

These are known gaps, not new bugs:

- **Margit** (most of this comes with track F's segment f3):
  - **What's missing:**
    - He is not drawn, only his shadow. He has no particles (golden mist, sparks, dust) and no golden ring at the leash.
    - His strikes and daggers hurt nobody, and no dagger flies. So a death by him and its line ("Sos was slain by Margit, the Fell Omen") can't be tested yet.
    - There is no boss bar, no music and no altar (only `/summon`), and he gives no drops, experience or reward.
  - **MW2 bullets and blasts** count as melee hits and level blasts until group er-guns. So a bullet breaks his stance at 1.5 times, and a close shot from the front at a kneeling Margit is a critical.
  - **Not handled yet:**
    - A player can sleep in a bed beside him: the bed check does not list him, and vanilla keeps you awake near any monster.
    - His strikes ignore walls. Once they hurt, an arc or a ring that reaches through a block will hit a player behind it. This is also why the fade only works with a wall at least 4 blocks out (M-9): at 3 blocks his sweep reaches through the wall and counts as a hit.
    - He walks through water and lava as on dry ground. Lava and fire do hurt him.
  - **By design:**
    - Loading the world resets his fight: full health, phase 1, and he walks back to his anchor.
    - He takes no damage while asleep or while he walks home after a reset.
    - Falls and potions never hurt him.
    - Only the data the game writes is kept in his marker.
  - His numbers are estimates, because Elden Ring is not installed on this PC. In scripted fights a steady sword broke his stance about every 7 seconds. Say so if he kneels too often.
- **Thrown things:**
  - Snowballs, tridents and rockets don't hit End crystals or the ender dragon yet (e2b).
  - Arrows, tridents, thrown items and explosions don't break lead knots, and MW2 bullets pass through knots.
  - Silverfish, endermites and Margit take a thrown hit as a player's attack in its damage type. Only the breeze (track D's d2) needs the real type.
- **Doors:**
  - Drowned don't break doors.
  - A raid door seldom breaks, because a vindicator opens it in the same tick as it chops.
  - Only villagers plan their paths again when a mob changes a door.
  - A mob's crack vanishes as soon as the mob dies or stops. Vanilla keeps an orphaned crack for up to 20 seconds.
- **Maps:**
  - Holding a map shows nothing, and maps don't fill in as you walk (e2b).
  - Cartographers don't sell explorer maps yet.
  - A camp's secret chest can pause the game for up to 2 seconds while its map searches, and a search that runs out of time gives no map.
- **Fences, leads and cats:**
  - Walls, panes and iron bars are still aimed at as full blocks.
  - Knots ignore the `entity_drops` and `mob_griefing` rules.
  - Cutting and breaking knots send no game events (track W).
  - Cats keep only chest screens shut: hoppers and comparators ignore them.
  - Copper chests don't tell cats when they are open (cats only sit on plain chests by themselves anyway).
- **Fireworks:**
  - Every starred burst draws the same spark burst in its stars' colours: shapes, fades, trails and twinkles come with e2b, and a star with no colours draws black.
  - Firework stars and rockets can't be crafted yet.
  - A dispensed rocket is drawn flat, and dispensers play no click or smoke for other items.
  - A rocket from a player who has left still hurts other players with pvp off.
- **Trial spawners:** breeze trial spawners stay dark until breezes land (track D's d2). Release 2's trial spawner limits still apply.
- **Silverfish and endermites:** silverfish don't hide in stone or come out of infested blocks, and an ender pearl never leaves an endermite (D0's d0b).
- **Not in this build:** see the list at the top of this file.
- The limits at the end of [release-2-tests.md](release-2-tests.md) still apply, except:
  - silverfish trial spawners, which run now;
  - the items of its "Not in this build" list that this build brings: thrown things hitting the new mobs, doors broken by zombies and vindicators, fireworks bursting on mobs, explorer maps, and shipwreck and ocean ruin loot. Margit is here, with the limits above.

## Screenshot numbering

- Name each screenshot by its number: `14.png`. When a step asks for both windows, name them `51-host.png` and `51-friend.png`.
- When a step fails, take a screenshot even if the step doesn't ask for one. Name it by its step label, e.g. `M-9.png` or `F-8-friend.png`. Say in one line what you expected and what you saw.
- For the sound steps (M-2 to M-18 and D-2), write one line per step with what you heard.
- Send `latest.log` after any crash. In two windows, send both windows' logs (see Setup for where they are).
