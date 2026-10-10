# Round 7, release 2: test steps

What's new since build 1: the dimension switch, the Nether, the End, elytra and phase 4. Build 1's steps are in [release-1-tests.md](release-1-tests.md).

**Setup:**
- Use a class with an assault rifle and Frag.
- In the `.env`, set `IW4L_MINECRAFT_WORLD=r7final` (a new world). Remove any `IW4L_MINECRAFT_DIMENSION`, `IW4L_MINECRAFT_WEATHER` and `IW4L_MINECRAFT_TIME` lines.
- Start by day, then type `/difficulty normal` and `/gamerule keep_inventory true`.
- Everything runs in `r7final` unless a step says otherwise. Get anything else with `/give @s <id> <count>`.

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*64,minecraft:flint_and_steel,minecraft:ender_eye*16,minecraft:end_portal_frame*12,minecraft:end_crystal*4,minecraft:elytra,minecraft:firework_rocket*64,minecraft:diamond_sword+sharpness=5,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+fire_protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:diamond_boots+frost_walker=2,minecraft:totem_of_undying,minecraft:golden_apple*16,minecraft:cobblestone*64,minecraft:water_bucket,minecraft:lava_bucket,minecraft:glass_bottle*16,minecraft:red_bed,minecraft:respawn_anchor*4,minecraft:glowstone*32,minecraft:compass,minecraft:clock,minecraft:campfire*4,minecraft:bone_meal*64,minecraft:iron_shovel
```

## Dimensions and commands

1. `/gamemode creative`, walk 100 blocks, type `/execute in `: the three dimensions are suggested. **S1**
2. `/execute in minecraft:the_nether run tp ~ 128 ~`: x and z are divided by 8, the purple loading screen shows, and you land on the bedrock roof. **S2**
3. `/time set day`: a red "There is no default clock in dimension minecraft:the_nether".
4. `/execute in minecraft:the_nether run locate structure minecraft:fortress`: you get Nether coordinates. Note them.
5. In the console, `give killstreak/predator`, then select it: "Air support is unavailable under this dimension's ceiling". **S3** `/kill`: the loading screen, then the overworld spawn, where the predator flies.
6. `/execute in minecraft:the_end run tp 100 50 0`: you're on the platform. `/time pause` answers "Paused clock minecraft:the_end", then `/time resume`. **S4**
7. Quit. Set `IW4L_MINECRAFT_WORLD=r7nether` (a new world) and `IW4L_MINECRAFT_DIMENSION=the_nether`: you start in a portal built for you. **S5** Then set the world back to `r7final` and remove the dimension line.

## Nether travel

1. You start in the End, where you quit. `/execute in minecraft:overworld run tp 800 120 -1600`, land, `/gamemode survival`, then build and light a 4x5 obsidian frame.
2. Step in: the purple swirl and its sound start, and pressing E closes the inventory at once. **S6** Step out: the swirl fades.
3. Stay in for 4 s: the loading screen, then the Nether and a whoosh. `/tp ~ ~ ~` shows about 100, y, -200. **S7**
4. Staying in the arrival portal never sends you back. Step out and back in: you return to your own portal, and the next trip uses the same Nether portal.
5. Build a second portal 40+ blocks away in the Nether: it leads to a new overworld portal 320+ blocks from the first. **S8**
6. In creative you travel at once. `/gamerule allow_entering_nether_using_portals false` blocks portals, then set it back to `true`.

## Nether look and rules

1. Back in the Nether: there's no sun, stars or clouds, and overhangs are as bright as floors. **S9**
2. Lava spreads 7 blocks in about 4 s. Water only hisses and steams, and mined ice (`/give`) leaves nothing.
3. The compass and the clock spin. **S10**
4. With the totem in your offhand and the chestplate on, use the bed: it explodes with fire. **S11**
5. Charge an anchor 4 times with glowstone, then use it empty-handed: "Respawn point set". `/kill`: you respawn beside it, now on 3 charges. **S12**
6. `/kill` four more times: three times beside the anchor, then "You have no home bed or charged respawn anchor..." and the overworld spawn.
7. In the overworld, use a `/give`n bed: "Respawn point set". Then `/kill` in the Nether: you're back at the bed. **S13**
8. In the overworld, an anchor with one charge explodes (`/give` a totem first). Walk 30 blocks from the bed and `/kill`: a `/give`n recovery compass points to where you died.

## Nether mobs

1. `/tp` to the fortress in creative, then switch to survival. Wither skeletons, zombified piglins and magma cubes spawn, and blazes appear at the spawner cage. **S14**
2. Zombified piglins (`/give @s minecraft:zombified_piglin_spawn_egg 4`) ignore you until you hit one, then the whole group chases you. Fire doesn't hurt them.
3. Magma cubes come in several sizes, glow and split. A wither skeleton's hit turns your hearts black. **S15**
4. A ghast (`/give` its spawn egg, in the open): its fireball flies back when you hit it with the sword or the rifle, and kills the ghast. **S16** A grenade only pushes it.
5. A blaze flares, fires three fireballs, sets you alight and leaves fire. **S17**
6. At an overworld portal with `/difficulty hard` and `/gamerule random_tick_speed 1000`, zombified piglins come out. Reset both afterwards.

## Eyes, frames and the way into the End

1. `/locate structure minecraft:stronghold`, then throw an eye: it flies about 12 blocks that way with a purple trail, then drops or shatters. **S18**
2. Stand in the middle of a cleared 3x3 and place 12 frames around it, so they face in. Fill them with eyes from outside: the twelfth opens the starfield. **S19**
3. Step in: you land on the platform facing west, hear a whoosh, and the pink "Ender Dragon" bar shows. **S20**
4. Mine one platform block, `/kill`, and go back through the end portal: the floor is whole again.

## The dragon fight

First `/gamemode creative`, fly to the island centre and build a cobblestone pillar about 30 blocks high near the middle. On top, `/xp set @s 0 levels` and `/gamemode survival`.

1. The dragon circles the pillars, and crystals bob on top of them. **S21**
2. Rifle hits make it flash red and lower the bar, head hits the most. Frags hurt it.
3. A crystal heals it with a beam. Shoot that crystal: the beam breaks and the dragon takes damage. **S22** Then shoot every crystal (shoot the iron bars of the caged ones first).
4. It swoops and spits purple fireballs. A glass bottle at the edge of a cloud fills with dragon's breath, and the cloud shrinks. **S23**
5. Its wings fling you. It breaks through cobblestone, but not obsidian or end stone.
6. On the podium rim: it perches, roars and breathes flame. **S24** Shooting its head makes it take off.
7. Kill it: white rays and a roar, then its XP takes you from 0 to level 68. **S25**

## After the dragon

1. The exit portal opens, the egg sits on the pillar, and a gateway appears about 96 blocks out. **S26**
2. Click or shoot the egg: it teleports up to 15 blocks with particles.
3. Exit portal: the credits roll with your name. **S27** Space speeds them up (Ctrl with it more), Up scrolls back, and Esc puts you at your bed.
4. Come back and leave again: no credits. After a restart, the portal, egg and gateway are still there.
5. Optional: put 4 crystals on the rim. The pillars are rebuilt and a new dragon comes. **S28**

## Elytra

1. Wear the elytra, `/tp ~ ~40 ~`, then press jump while falling: you glide. **S29**
2. A rocket boosts you. With rockets in the offhand and a gun selected, the swap-hands key (F) boosts you and the gun still fires. **S30**
3. Hitting a wall fast hurts, a shallow landing doesn't, and the durability bar goes down.

## Sounds, particles and small fixes

1. Lava pops and throws embers, and burning netherrack crackles and smokes. **S31**
2. Campfire smoke rises three times higher over a hay bale (`/give`). A shovel puts it out with a hiss and a smoke burst. **S32**
3. TNT (`/give`) smokes while it burns, then flashes. Water on lava hisses. Bone meal sparkles.
4. Hoes, shovels and axes (`/give`) till, make paths and strip logs. Pointed dripstone hangs from ceilings.
5. Without the totem, walk into lava: an orange fog, then the death sound. **S33** `/weather rain`: splashes and patter.
6. Feather Falling IV boots: a 10-block fall costs under 2 hearts. Frost Walker boots freeze water. **S34**

## Retest from build 1

- Go through [release-1-tests.md](release-1-tests.md) quickly: grenade keys, chat paste, one throwable, lids, F5 gear, tab completion, creative tabs, and a save and restart.
- Quit in the Nether: you start there again.
- `/kit` now gives the `@` entries too.

## Known gaps

- N6: no piglins, hoglins, striders or bartering, and no sleeping.
- End gateways don't teleport, there are no shulkers, item frames or levitation, and the sky doesn't flash.
- The dragon isn't knocked back and has no breath or death particles. There's no boss music.
- The elytra and its glide pose aren't drawn on the MW2 body.
- Single player only: friends can't be in other dimensions yet.
- Still open: powder snow, lectern books, trapped chest signal, totem animation, ladder ground jump, falling rain streaks.
