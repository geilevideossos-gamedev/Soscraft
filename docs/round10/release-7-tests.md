# Round 10 release 7: pandas, game events, mob buckets, squids, lightning, phantoms

Release 7 is `minecraft-survival` at 6ad783e5, copied in as `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-survival.exe`. Release 6 is kept beside it as `iw4l-survival-789f6d2.exe`. `iw4l-master.exe` (the relay) was copied in again.

New since release 6:
- Track M1, segment m1b (merge 7d171cc0): pandas. They spawn in bamboo jungles with their genes and faces, breed with bamboo, sit to eat bamboo and cake, sneeze, lie on their backs, roll, bite back once and hide from thunder. Friends see them too.
- Track W, segment w3 (merge 14c6fa97; track W is done for round 10): sculk sensors hear lead knots and leads, a rocket's launch and where it strikes, the ominous item spawner, silverfish breaking out of infested stone, villagers and piglins at doors, and mobs putting equipment on or taking it off. A bell rung by a player's snowball counts as that player's vibration.
- Track M2, segment m2a (merge 24d774f1): water hooks and water paths for the water mobs to come; mob buckets, which pour their water and let their mob out (unseen until that mob is live), also from a dispenser; and squids and glow squids, which spawn, swim in pulses, squirt ink, flee, dry out on land, glow and sparkle.
- Track M3, segment m3a (merge 593b4ba4): lightning from `/summon minecraft:lightning_bolt`. It lights fires, hurts, burns dropped items, flashes and thunders for every player, turns pigs, villagers, creepers and mooshrooms, cleans copper and makes lightning rods spark, and charged creepers wear their swirl. Also phantoms: they circle, swoop and bite, burn by day, shy away from cats and come for players who have not slept for three days.
- 110f3817 (docs only): README, CLAUDE.md and the state docs now describe Margit.

**Multiplayer:** this build is protocol 12. A friend needs this exe (and the relay from the game folder); release 6 (protocol 11) is refused with "Incompatible client! Please use the host's build (Minecraft multiplayer protocol 12)".

Release 6's steps (`docs/round10/release-6-tests.md`) and the earlier releases' still stand; this file only covers what is new. Screenshots are numbered 1 to 96 in the order below. Each part has its own kit: put its line in the `.env` next to the exe as `IW4L_MINECRAFT_KIT=...` and start a new world. The full background for every step is in the track logs: `docs/round10/track-m1-log.md` (segment m1b: pandas-live, pandas-drawn, pandas-eating, pandas-moods, pandas-thunder and the m1b review fixes), `docs/round10/track-w-log.md` (events-items, events-mobs and the w3 review), `docs/round10/track-m2-log.md` (water-hooks, water-paths, water-buckets, squids-live, squids-drawn and the m2a review) and `docs/round10/track-m3-log.md` (lightning-strikes, lightning-turns, lightning-drawn, lightning-glow, phantoms-live, phantoms-drawn and phantoms-spawner).

Setup for every part: survival, Normal, by day on flat open ground, unless a step says otherwise. Read your position with `/tp @s ~ ~ ~` (the fork has no F3 screen). The class loadout (the M4A1 on slot 1, frags on G) comes with the first join; parts A, Q and H use it.

## Part A. Pandas

Kit (17 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:panda_spawn_egg*16,minecraft:bamboo*64,minecraft:bamboo*64,minecraft:bamboo*64,minecraft:bone_meal*16,minecraft:cake,minecraft:cake,minecraft:golden_dandelion*2,minecraft:iron_sword,minecraft:shield,minecraft:flint_and_steel,minecraft:zombie_spawn_egg*2,minecraft:spawner,minecraft:water_bucket,minecraft:water_bucket,minecraft:iron_shovel,minecraft:cooked_beef*32
```

Setup: survival, Normal, by day, on wide flat open grass (plains are best). Run `/gamerule keep_inventory true`, and leave `mob_griefing` and `mob_drops` on (the defaults). Move the eggs, one bamboo stack, the sword and the flint and steel to the hotbar, and keep one hotbar slot empty for punches. The class's M4A1 stays on slot 1. Eat cooked beef whenever you are low.

- A `/summon minecraft:panda` with no `{...}` gets random genes, like an egg. One with `{...}` but no genes is plain.
- A summoned weak or lazy panda keeps 20 health and the normal walk, as in vanilla. Only pandas from eggs, breeding and the wild get the weak panda's 10 health and the lazy panda's slow walk.
- `/difficulty peaceful` does not remove pandas. So each group of steps starts on fresh grass 30 blocks from the last group's pandas. An aggressive panda joins any fight between you and a panda within 16 blocks of it.

1. **Seven genes.** Stand still and run these seven commands. They put seven pandas in a row 10 blocks from you, 3 blocks apart:
   - `/summon minecraft:panda ~-9 ~ ~10 {MainGene:"normal",HiddenGene:"normal"}`
   - `/summon minecraft:panda ~-6 ~ ~10 {MainGene:"lazy",HiddenGene:"lazy"}`
   - `/summon minecraft:panda ~-3 ~ ~10 {MainGene:"worried",HiddenGene:"worried"}`
   - `/summon minecraft:panda ~ ~ ~10 {MainGene:"playful",HiddenGene:"playful"}`
   - `/summon minecraft:panda ~3 ~ ~10 {MainGene:"brown",HiddenGene:"brown"}`
   - `/summon minecraft:panda ~6 ~ ~10 {MainGene:"weak",HiddenGene:"weak"}`
   - `/summon minecraft:panda ~9 ~ ~10 {MainGene:"aggressive",HiddenGene:"aggressive"}`

   Turn until you see the row. Each is a black-and-white panda about 2.25 blocks from nose to rump and 1.3 tall. From one end of the row to the other, the faces are: plain, sleepy, frowning, tongue out, brown and white fur, runny nose, angry brows. The lazy one soon flops onto its back and the playful one somersaults now and then; steps 24 and 26 check that. **Screenshot 1** with all seven faces.
2. **Genes that don't show.** Run these three. They stand 4 blocks behind the middle of the row.
   - `/summon minecraft:panda ~-3 ~ ~14 {MainGene:"weak",HiddenGene:"normal"}`: a plain face, because weak shows only when both genes are weak.
   - `/summon minecraft:panda ~ ~ ~14 {MainGene:"lazy",HiddenGene:"normal"}`: the sleepy face, because lazy shows from the main gene alone.
   - `/summon minecraft:panda ~3 ~ ~14 {MainGene:"sleepy"}`: a plain face, because a gene name the game does not know reads as normal.

   **Screenshot 2.**
3. **Walking and looking.** Walk toward the frowning (worried) panda. Once you are within about 8 blocks, it runs off fast. No other panda ever runs from you. Stand 4 blocks from any other panda: its legs swing as it ambles, and now and then its head turns to look at you. Listen for a while: the angry-browed panda huffs and the frowning one whimpers, where the others make the plain panda call.
4. **Cubs.** Run `/summon minecraft:panda ~2 ~ ~4 {Age:-24000,MainGene:"brown",HiddenGene:"brown"}` and `/summon minecraft:panda ~4 ~ ~4 {Age:-24000,MainGene:"playful",HiddenGene:"playful"}`. You get two cubs about half a block tall and a block long, on their own rounder model: a big head and short stubby legs, not shrunken adults. One is brown and white, the other has its tongue out. **Screenshot 3.** They follow the nearest adult, and the playful cub somersaults often.
5. **Egg pandas.** Walk 30 blocks away onto fresh grass and run `/gamemode creative`, so the pandas you hit in step 8 can't bite back. Walk along and use the panda egg ten times on the grass, 3 blocks apart.
   - About two in three have the plain face. Now and then one is sleepy, frowning, tongue-out or angry.
   - About one in ten has the runny nose (weak). Brown is rare (one in 64).
   - None runs from you, because you are in creative.

   If none has the runny nose, use more eggs until one does (`/give @s minecraft:panda_spawn_egg 16` if you run out). **Screenshot 4** with the runny-nosed panda.
6. **No bamboo, no cub.** No bamboo grows here. Pick two egg pandas standing close together and right-click each with bamboo: hearts show over each. Then each shakes its head from side to side and paws the air for about a second and a half (it may also turn to look at you). A sad bleat plays twice, under a second apart. No cub comes. **Screenshot 5** during the head shake.
7. **Saved.** Save and quit, then load. Walk back to the first pandas. They have the same faces as in steps 1 and 2: the panda with only the lazy main gene is still sleepy-faced, the one with the unknown gene is still plain, and both cubs are still small. **Screenshot 6.** Walk back to the egg pandas.
8. **Hits and drops.** Still in creative, hit each egg panda with the iron sword, away from the other pandas. Don't jump, and let the sword recover fully between hits.
   - Each hit flashes it red, and a dying panda tips over sideways.
   - The runny-nosed (weak) panda dies after two hits; every other one takes four.
   - Each drops one bamboo and experience orbs.

   Then run `/summon minecraft:panda ~ ~ ~3 {Age:-24000}` and kill the cub: it drops nothing, not even orbs.
9. **In a spawner.** Place the spawner on the grass and use a panda egg on it. A small panda turns inside the cage, and now and then its rump pokes a little out of the cage, as in vanilla. **Screenshot 7.** Break the spawner right away; any pandas it made around it can stay. Run `/gamemode survival`.
10. **A cub from bamboo.** Walk 30 blocks on. Right-click the top of a grass block with bamboo to plant a shoot. Use bone meal on it until it is a stalk at least 2 blocks tall. Use two panda eggs on the grass next to each other, 4 blocks from the stalk. Right-click both with bamboo: hearts show, they walk together, a few experience orbs pop out and a cub appears beside them. **Screenshot 8.**
11. **The cub.** Right-click the cub with bamboo: one bamboo is used, and it grows up a little faster. Right-click it with the golden dandelion: the dandelion is used, with its sound, and the cub now stays a cub. Right-click it with bamboo again: nothing happens, and the bamboo stays in your hand.
12. **Eating from your hand.** Within 5 minutes of the cub's birth, right-click one parent with bamboo. No hearts show this time. Instead it sits up, holds one bamboo stalk in its paws and munches. Every quarter second you hear a crunch, and little bamboo crumbs fall. Walk around it holding bamboo: it stays sitting. Usually within 5 to 30 seconds the bamboo is gone and it stands up: about half the meals are done within 15 seconds, and now and then one takes a minute. Most meals pause once or more: it stops munching for about 4 seconds, still sitting with the bamboo, then carries on. **Screenshot 9** while it munches. If it finished too fast, feed the other parent.
13. **A swap.** Run `/summon minecraft:panda ~ ~ ~3 {NoAI:1b,InLove:600,equipment:{mainhand:{id:"minecraft:cake",count:1}}}`. It has no AI, so it stands still and does not sit down with its cake by itself, as any panda holding food soon does. Right-click it with bamboo at once. The cake pops out onto the grass, and the panda sits down with one bamboo and munches; no hearts show. Pick the cake up.
14. **Fetching a stack.** Run `/summon minecraft:panda ~ ~ ~3 {CanPickUpLoot:1b}` and step 6 blocks back. Select a full bamboo stack (not the one you feed with) and throw it all with Ctrl+Q, about 4 blocks from the panda.
    - It walks over and picks up the whole stack: the stack vanishes from the ground.
    - It sits down, and a few seconds later starts munching with crumbs.
    - When it stands up, the whole stack is gone, as in vanilla.
    - Now and then it gets up before it is done. Then the stack drops at its feet, it picks it up again at once, and it sits back down to finish it within about 3 minutes.

    **Screenshot 10** while it eats.
15. **A cake.** When it has finished, throw a cake (Q) 3 blocks from it. Within a minute it fetches the cake, sits and eats it the same way.
16. **No mob griefing.** Run `/gamerule mob_griefing false`. Throw one bamboo (Q) 3 blocks from it, then select the sword at once: a panda follows anyone holding bamboo within 10 blocks, and it neither fetches nor sits while it does. Within a minute it walks over and leaves the bamboo on the ground. Run `/gamerule mob_griefing true`: it picks the bamboo up and sits down with it in its paws.
17. **Tempted up.** While it sits with that bamboo, stand 5 blocks away holding bamboo. It gets up and the bamboo pops out at its feet. About half a second later it picks the bamboo up again and follows you holding it (a standing panda does not show what it holds). Select the sword: it stops following. Later, after 10 seconds to almost 3 minutes, it sits down with the bamboo again; you don't need to wait for that.
18. **Killed while it eats.** Run `/summon minecraft:panda ~ ~ ~3 {CanPickUpLoot:1b}` and throw the third bamboo stack (Ctrl+Q) at its feet. Once it has picked the stack up and sat down, kill it with the sword. The whole stack falls out, with one more bamboo of its own and experience orbs. Pick the bamboo up again.
19. **Worried pandas don't settle.** Run `/gamemode creative`; a worried panda does not run from a player in creative. Run `/summon minecraft:panda ~ ~ ~3 {MainGene:"worried",HiddenGene:"worried",CanPickUpLoot:1b}` and throw one bamboo (Q) at its feet. It picks the bamboo up, but in a minute of watching it never sits down to eat it. Run `/gamemode survival`.
20. **It bites back once.** Walk 30 blocks on and run `/summon minecraft:panda ~ ~ ~4 {MainGene:"normal",HiddenGene:"normal"}`. Select the empty hotbar slot and punch it once. It flashes red but does not run away. Instead it comes at you and bites once, with a bite sound, for about 3 hearts. Then it wanders off and leaves you alone.
21. **An aggressive panda joins in.** Eat cooked beef until your hearts are full. Put the shield in the offhand (F). Run `/summon minecraft:panda ~2 ~ ~6 {MainGene:"aggressive",HiddenGene:"aggressive"}` once. Punch the plain panda again: it bites you once and leaves you alone. The angry-browed panda comes at you too and keeps biting, about 3 hearts a bite. Select the sword and hold right-click to block its bites with the shield. **Screenshot 11** with it coming at you. Select the empty hotbar slot and punch the angry-browed panda once, then select the bamboo and right-click it: hearts show, and it stops attacking. Bamboo only calms a panda that you hit yourself; one that only joined in keeps biting even with hearts, as in vanilla. Then shoot it with the M4A1 until it dies, so it can't join the next fight.
22. **Shot.** Shoot the plain panda with the M4A1 until it dies. Each hit flashes it red, and it drops one bamboo and experience orbs.
23. **Fire and a hot floor.** Run `/summon minecraft:panda ~ ~ ~4 {MainGene:"normal",HiddenGene:"normal"}`. Right-click the top of the grass block right beside it with the flint and steel: when the fire touches it, it runs off fast. Then run `/setblock ~ ~-1 ~3 minecraft:magma_block` and `/summon minecraft:panda ~ ~ ~3 {MainGene:"normal",HiddenGene:"normal"}`. The new panda stands on the magma block, flashes red and runs off fast. A punch never makes a panda run (step 20).
24. **Lazy pandas.** Walk 30 blocks on and run `/summon minecraft:panda ~-2 ~ ~4 {MainGene:"lazy",HiddenGene:"lazy"}` and `/summon minecraft:panda ~2 ~ ~4 {MainGene:"lazy",HiddenGene:"lazy"}`. Within a minute each flops onto its back and kicks its legs in the air. A lazy panda stays on its back for a minute or two. Walk around them holding bamboo: they stay on their backs. **Screenshot 12.**
25. **Getting up.** Right-click one with an empty hand: it gets back onto its feet. Right-click the other with bamboo: it only gets up. The bamboo stays in your hand, and no hearts show. Neither lies down again for at least 10 seconds.
26. **Somersaults.** Run `/summon minecraft:panda ~ ~ ~5 {MainGene:"playful",HiddenGene:"playful"}`. Every few seconds it does a forward somersault: a full roll over about a second and a half that carries it about five blocks. It hops once at the start and bounces twice as it lands. **Screenshot 13** mid-roll.
27. **Off a ledge.** Walk 15 blocks from the other pandas onto open grass, so no panda stands where the platform goes. Run `/fill ~2 ~ ~2 ~6 ~1 ~6 minecraft:cobblestone`, a platform 5 by 5 and 2 high beside you. Then run `/summon minecraft:panda ~4 ~2 ~4 {MainGene:"playful",HiddenGene:"playful"}` to put a playful panda on top. Whenever it comes to an edge facing outward, it rolls straight off at once and tumbles down.
28. **Sneezes.** Walk 20 blocks from the other pandas. Run `/summon minecraft:panda ~ ~ ~3 {Age:-24000,MainGene:"weak",HiddenGene:"weak"}`, a weak cub. Then put two plain adults beside it: `/summon minecraft:panda ~3 ~ ~5 {MainGene:"normal",HiddenGene:"normal"}` and `/summon minecraft:panda ~-3 ~ ~5 {MainGene:"normal",HiddenGene:"normal"}`.
    - Within a minute or two the cub slowly tips its head back with a sniffling sound and holds it there for a moment.
    - Then it sneezes, with the sneeze sound. A small see-through green puff shows in front of its nose and fades within 1 to 3 seconds.
    - Both adults jump. An adult that is sitting, lying or rolling does not.

    **Screenshot 14** with the puff. It sneezes about every half minute, and it somersaults now and then too. Then run `/summon minecraft:panda ~ ~ ~3 {Age:-24000}`, a plain cub, and watch it for a few minutes. It somersaults about every half minute, and it sneezes too, but rarely (about once in five minutes).
29. **Worried pandas.** Walk 30 blocks on and run `/summon minecraft:panda ~ ~ ~6 {MainGene:"worried",HiddenGene:"worried"}` and `/summon minecraft:panda ~4 ~ ~8 {MainGene:"normal",HiddenGene:"normal"}`. Walk toward the worried one: within about 8 blocks it runs off fast. Run `/gamemode creative` and walk up to it; it no longer runs from you. Use a zombie egg 2 blocks from it: it runs from the zombie. Run `/gamemode survival`. The zombie burns in the sun, or kill it with the sword.
30. **Thunder.** Keep the worried panda in view, run `/weather thunder` and wait about 5 seconds. As the storm builds, the worried panda sits down, covers its face with its front paws and trembles. The plain panda carries on as before. **Screenshot 15.**
31. **Too scared for bamboo.** Walk right up to the worried panda: it stays where it sits. Right-click it with bamboo: nothing happens, and the bamboo stays in your hand. Right-click the plain panda with bamboo: hearts.
32. **Bamboo in a storm.** Stand still and run `/summon minecraft:panda ~ ~ ~2 {MainGene:"worried",HiddenGene:"worried",CanPickUpLoot:1b}`: it sits and trembles at once. Look at its feet and throw one bamboo (Q). It picks the bamboo up (the bamboo vanishes from the ground) but stays trembling, with no bamboo in its paws and no munching. If the bamboo lands out of its reach, throw another.
33. **In water.** With the shovel, dig a pit 2 by 2 and 1 deep. Pour the two water buckets into two opposite corners, and all four fill. Stand in the water and run `/summon minecraft:panda ~ ~ ~ {MainGene:"worried",HiddenGene:"worried"}`, then climb out. While it is in the water it does not sit: it bobs about. If it climbs out onto dry ground, it sits and trembles at once.
34. **The storm ends.** Run `/weather clear`. Within about half a second every worried panda stands up. Put your bamboo away: the first worried panda walks away from you again. Then hold bamboo and walk to within 10 blocks of it: it comes to you, because bamboo wins over its fear. Right-click it with the bamboo: hearts show.
35. **Wild pandas.** Run `/gamemode creative` and `/locate biome minecraft:bamboo_jungle`, then `/tp @s <x> 120 <z>` with the x and z it gives. Fly down, then fly on over the bamboo for a few hundred blocks into new land. Pandas stand among the bamboo in ones and twos, mostly plain, and now and then a cub stands with an adult. **Screenshot 16.** Run `/gamemode survival`.
36. **Optional, a slime ball (do it last).** About one sneeze in 700 drops a slime ball at the cub's feet. At a sneeze every half minute, that is about once in six hours. To check it:
    - On flat ground, run `/fill ~-1 ~-1 ~-1 ~1 ~2 ~2 minecraft:cobblestone hollow`. It shuts you in a dark room 1 wide, 2 long and 2 high.
    - Run `/summon minecraft:panda ~ ~ ~1 {Age:-24000,AgeLocked:1b,MainGene:"weak",HiddenGene:"weak"}`. This weak cub never grows up and stands in the room's other half.
    - Run `/difficulty peaceful`, so you don't get hungry, then `/tick sprint 14d`. The sprint takes about 15 to 40 minutes. The cub sneezes over and over, and you pick up anything it drops.
    - When the chat says the sprint is completed, open the inventory. About two runs in three give at least one slime ball; if none came, sprint again.

    Run `/difficulty normal` and break out through a wall. Do this step last: after 14 days without sleep, phantoms come at night and in thunderstorms.

## Part G1. Game events at a sculk sensor

Part G needs two kits, so it is split in two: G1 in the Overworld, and G2 with piglins in the Nether. In this build a sculk sensor also hears lead knots and leads, a rocket's launch and the spot where it hits, the ominous item spawner, silverfish breaking out of infested stone, villagers and piglins at doors, and mobs putting equipment on or taking it off. A bell rung by your snowball now counts as your vibration, so it sets off a shrieker.

Kit (22 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:white_carpet*64,minecraft:cobblestone*64,minecraft:oak_fence*8,minecraft:cow_spawn_egg*4,minecraft:lead*8,minecraft:shears,minecraft:firework_rocket*16,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*8,minecraft:bell*2,minecraft:sculk_shrieker*2,minecraft:snowball*16,minecraft:infested_stone*4,minecraft:silverfish_spawn_egg*2,minecraft:villager_spawn_egg*4,minecraft:oak_door*2,minecraft:blast_furnace*2,minecraft:emerald*16,minecraft:pig_spawn_egg*2,minecraft:saddle,minecraft:saddle,minecraft:iron_horse_armor,minecraft:cooked_beef*16
```

Setup: start a new world in survival on Normal, by day, on fairly flat open ground. Stand still and run these commands. The block you stand on is **the start**.

```
/time set day
/gamerule advance_time false
/fill ~-10 ~-1 ~-6 ~20 ~-1 ~22 minecraft:stone
/fill ~-10 ~ ~-6 ~20 ~4 ~22 minecraft:air
/setblock ~ ~ ~4 minecraft:sculk_sensor
/setblock ~ ~1 ~4 minecraft:cobblestone
/setblock ~1 ~ ~4 minecraft:redstone_lamp
/setblock ~ ~ ~5 minecraft:comparator[facing=north]
/fill ~ ~ ~6 ~ ~ ~20 minecraft:redstone_wire
/fill ~-8 ~ ~-4 ~8 ~ ~12 minecraft:white_carpet keep
```

The commands do these things:
- They keep it day.
- They lay a stone floor with a sculk sensor 4 blocks in front of the start. A cobblestone sits on top of the sensor, so no mob can stand on it, and a redstone lamp is on the sensor's left.
- They put a comparator on the sensor's far side, leading into a straight line of 15 redstone dust.
- They cover the floor with white carpet for 8 blocks around the sensor.

"Left" and "right" below mean as seen from the start, facing the sensor.

**Reading the line:**
- When the sensor hears something, the lamp lights for 1.5 s. The comparator lights as many dust as the frequency of what the sensor heard, counted from the comparator ("the line shows 10").
- Wait 2 s between actions, because the sensor hears nothing while it is lit or resting. It hears things within 8 blocks.
- Placing or breaking carpet and walking on it are not heard, so you can walk normally on the carpet.
- Most blocks below go on the stone, so break the carpet on that spot first.
- A mob that steps on bare floor near the sensor (the comparator, the dust or a doorway) lights 1. Wait until it walks off. If a 1 got in the way of an action, repeat that action.

37. **A knot.** Break the carpet 3 blocks to the left of the start and place an oak fence there: 13 (a block placed). Use a cow egg on the carpet beside the fence: 14. Right-click the cow with a lead: the lead goes on and the line stays dark. Stand up (not crouched) and right-click the fence. The lead ties to a knot on the post: 10. With the knot and the 10: **Screenshot 17.**
38. **Taking it back.** Select an empty hotbar slot, crouch and right-click the fence. The cow stays tied and the line stays dark, as in vanilla. Stand up and right-click the fence again: the cow is back on your lead, and the line shows 10.
39. **Punching and cutting the knot.**
    - Right-click the fence to tie the cow again: 10.
    - Stand 2 blocks from the fence. Aim at the knot from the side (the upper half of the post) and left-click once. The knot vanishes with the untie sound and the lead drops: 15.
    - Pick up the lead, put it on the cow and tie it to the fence again: 10.
    - Hold the shears and right-click the knot from the side. The snip sound plays, the knot goes and the lead drops: 6.
40. **A lead on a cow.** Pick up the lead and put it on the cow. Right-click the cow with an empty hand: you untie it and the lead drops: 6. Put the lead on again and right-click the cow with the shears. The lead is cut: 6.
41. **Cow to cow.**
    - Use a second cow egg on the carpet: 14.
    - Put the lead on the first cow and select an empty hotbar slot. Crouch and right-click the second cow. The first cow is now tied to the second: 4.
    - To keep both cows by the fence, put a lead on the second cow and right-click the fence: 10.
42. **A launch.** Stand on the start, not crouched, and place a plain rocket on the carpet 2 blocks in front of you. It flies up and the line shows 3 (its launch). The rocket ends high up, out of the sensor's reach.
43. **A rocket against a roof.** Stand on the start and run:

    ```
    /fill ~-3 ~ ~2 ~-3 ~2 ~2 minecraft:cobblestone
    /setblock ~-4 ~2 ~2 minecraft:cobblestone
    ```

    This builds a cobblestone pillar 3 blocks high, 3 blocks to the right and 2 ahead. One block sticks out from its top to the right: the overhang.
    - Stand 2 blocks to the right of the carpet under the overhang and crouch.
    - Place a plain rocket on the carpet under the overhang. It rises, presses against the overhang and puffs after about a second.
    - Its launch is not heard because you are crouched, but the rocket hitting the overhang is: 2.
44. **A burst against the roof.** Stay crouched in the same place and place the red rocket under the overhang. It bursts against the overhang in red sparks with a bang: 15. Your own rocket may take about 2 hearts. You stand farther from the sensor than the burst, so the line still shows the burst and not your hurt (7). With the 15 lit: **Screenshot 18.**
45. **The ominous item spawner.** Stand on the carpet right in front of the sensor, between it and the start. Run `/summon minecraft:ominous_item_spawner ~3 ~3 ~ {item:{id:"minecraft:wind_charge",count:1},spawn_item_after_ticks:60}`.
    - A wind charge floats 3 blocks above the carpet and 3 blocks to the left.
    - 3 s later the spawner drops it and is gone: 15 (the spawner's end). With the 15 lit: **Screenshot 19.**
    - A moment later the wind charge bursts on the floor. The sensor is busy, so it doesn't hear the burst.

    Wait 2 s and run the same command with `minecraft:diamond` in place of `minecraft:wind_charge`. A diamond floats there, drops after 3 s, and the line shows 15 again.
46. **A bell rung by your snowball.**
    - Break the carpet 4 blocks to the right of the sensor. Stand 2 blocks further right, look at that spot and place a bell there: 13.
    - Break the carpet 2 blocks to the left of the sensor, just past the lamp, and place a sculk shrieker there: 13. The shrieker may shriek once right away, because the sensor heard you place it. Wait until it is quiet (5 s).
    - Walk to 10 blocks to the right of the sensor, in line with the bell, and crouch.
    - Throw a snowball at the side of the bell that faces you, below the rim. The bell rings and swings, and the line shows 11. The shrieker shrieks: you hear its sound and see its rising rings. It shrieks because the ring is yours. Before this build, a bell rung by a thrown thing had no player behind it, and the shrieker stayed quiet. With the 11 and the rings: **Screenshot 20.**
47. **Clearing up.** Stand on the start and run:

    ```
    /setblock ~-4 ~ ~4 minecraft:air
    /setblock ~2 ~ ~4 minecraft:air
    /fill ~-4 ~ ~2 ~-3 ~2 ~2 minecraft:air
    /fill ~-8 ~ ~-4 ~8 ~ ~12 minecraft:white_carpet keep
    ```

    The bell, the shrieker and the pillar go, and the carpet is whole again. Commands make no vibrations.
48. **Silverfish break out.**
    - Break the carpet 5 blocks to the left of the sensor and place an infested stone there: 13.
    - Walk on along that line to 14 blocks left of the sensor (9 past the stone) and turn to face the sensor.
    - Use a silverfish egg on the floor 2 blocks in front of you. Out here the sensor hears neither the egg nor the fight.
    - When the silverfish comes at you, hit it once with an empty hand. It survives.
    - About a second later the infested stone breaks with the stone break sound and bits of stone. A second silverfish crawls out with a small white puff: 12. With the gap and the 12: **Screenshot 21.**

    Run `/difficulty peaceful` (both silverfish vanish), then `/difficulty normal`.
49. **Without mob griefing.** Run `/gamerule mob_griefing false`. Place another infested stone on the same spot (13), and do step 48 again with the second silverfish egg. This time nothing breaks and no silverfish comes out. The hurt silverfish quietly turns the stone back into plain stone, and the line stays dark. Run `/difficulty peaceful`, `/difficulty normal` and `/gamerule mob_griefing true`.
50. **A villager at a door.** Stand on the start and run:

    ```
    /fill ~-5 ~ ~ ~-5 ~1 ~3 minecraft:cobblestone
    /fill ~-5 ~ ~5 ~-5 ~1 ~8 minecraft:cobblestone
    ```

    This builds a cobblestone wall 2 high, 5 blocks to the right of the sensor, with a gap level with the sensor.
    - Break the carpet in the gap and place an oak door in it, standing on the sensor's side: 13.
    - Use a villager egg on the carpet 2 blocks to the right of the sensor, 3 blocks from the door: 14.
    - Walk round the end of the wall, behind the start, to its far side. Both ends of the wall are open, so the villager can wander round them too. If it has wandered round to the far side, wait until it wanders back to the sensor's side before you go on.
    - Place a blast furnace on the floor 4 blocks beyond the door, in line with it. It is 9 blocks from the sensor, so placing it is not heard. Step out of the line.
    - Within a minute or two the villager walks through the door to the blast furnace. The door opens with its sound and the line shows 10. With the open door and the 10: **Screenshot 22.**
    - If it shuts the door behind it, the line shows 9. Vanilla villagers also sometimes leave a door open when they get past it quickly.
    - If it walks round an end of the wall to the blast furnace instead of through the door, the line stays dark. Note it and go on.

    Once it is through, stand on the start and run `/fill ~-5 ~ ~4 ~-5 ~1 ~4 minecraft:cobblestone`. This closes the gap. The wall's ends are still open, so the villager may wander back round them later; its steps on the carpet are not heard.
51. **A villager shows its armour.**
    - Break the carpet 4 blocks to the left of the sensor, next to the stone from step 49, and place the second blast furnace there: 13.
    - Use a villager egg on the carpet right beside it: 14. Wait 10 s while it takes the armorer's job.
    - Select the emeralds, crouch and stand right next to the villager. When it looks at you, it holds up a piece of iron armour that it sells for emeralds: 5. If it sells two pieces, it swaps them every 2 s, and each swap lights 5 again. With the armour in its hand and the 5: **Screenshot 23.**
    - Select an empty hotbar slot. The villager lowers its hand, now empty: 4.
52. **Saddles, horse armour and carpet.** Don't crouch for these clicks unless the step says to.
    - **Pig.** Use a pig egg on the carpet 2 blocks to the right of the start: 14. Wait 2 s and right-click the pig with a saddle. It wears the saddle: 5.
    - **Horse.** Stand on the start and run `/summon minecraft:horse ~-3 ~ ~2 {Tame:1b}`. A tame horse appears 3 blocks to your right and 2 ahead. Wait 2 s.
      - Right-click it with the other saddle: 5.
      - Right-click it with the iron horse armour. It wears the armour: 5.
      - Crouch and right-click it to open its inventory. Take the armour out of its armour slot into your inventory: 4. Close the screen.
    - **Llama.** Stand on the start and run `/summon minecraft:llama ~-3 ~ ~-1 {Tame:1b}`. Wait 2 s and right-click the llama with a white carpet. It wears the carpet: 5.

    Vanilla shows 6 for each of these clicks (see the known limits).

## Part G2. Piglins at a sculk sensor

Kit (8 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*4,minecraft:gold_ingot*8,minecraft:golden_helmet,minecraft:golden_helmet,minecraft:oak_door*2,minecraft:cobblestone*64,minecraft:white_carpet*64,minecraft:cooked_beef*16
```

Setup: start a new world in survival on Normal. Open the inventory, put on one golden helmet and keep it on. Piglins then leave you alone; a piglin hitting you would be heard too. Run `/execute in minecraft:the_nether run tp @s 0 128 0`. This puts you on the flat bedrock roof of the Nether, where no mobs spawn. Stand still and run:

```
/setblock ~ ~ ~4 minecraft:sculk_sensor
/setblock ~ ~1 ~4 minecraft:cobblestone
/setblock ~1 ~ ~4 minecraft:redstone_lamp
/setblock ~ ~ ~5 minecraft:comparator[facing=north]
/fill ~ ~ ~6 ~ ~ ~20 minecraft:redstone_wire
/fill ~-8 ~ ~-4 ~8 ~ ~12 minecraft:white_carpet keep
```

This builds G1's rig on the roof, with the same start and carpet, and you read the line the same way. If an egg hatches a small baby piglin, run `/difficulty peaceful` and `/difficulty normal`, then use another egg.

53. **A piglin takes a helmet.**
    - Use a piglin egg on the carpet 2 blocks to the left of the start: 14. Wait 2 s.
    - Look at its head. About one piglin in ten already wears a golden helmet. Such a piglin would keep the dropped helmet in its inventory, and the line would show only 4. Then run `/difficulty peaceful` and `/difficulty normal` and use another egg (`/give @s minecraft:piglin_spawn_egg 4` if you run out).
    - Stand 2 blocks from the piglin, select the other golden helmet and press Q to drop it at its feet.
    - The piglin picks the helmet up and holds it in its other hand, looking at it: 5. With the helmet in its hand and the 5: **Screenshot 24.**
    - About 6 s later it puts the helmet on its head: 5 again. The helmet leaves its hand (4) at the same moment, and the sensor keeps the 5.
54. **Gold for a barter.** When the piglin wears the helmet, right-click it with the gold ingots. It takes one and holds it in its other hand, looking at it: 4. About 6 s later it throws you an item and its hand is empty again: 4.
55. **A piglin at a door.** Stand on the start and run:

    ```
    /fill ~-9 ~ ~2 ~-5 ~1 ~6 minecraft:cobblestone
    /fill ~-8 ~ ~3 ~-6 ~1 ~5 minecraft:air
    /fill ~-5 ~ ~4 ~-5 ~1 ~4 minecraft:air
    ```

    This builds a room 3 by 3 inside with walls 2 high, 5 blocks to the right of the sensor. Its wall facing the sensor has a gap.
    - Place an oak door in the gap from the sensor's side: 13.
    - Stand on the start and run `/fill ~-8 ~ ~-4 ~8 ~ ~12 minecraft:white_carpet keep` again, to carpet the room's floor.
    - Open the door (10), use a piglin egg on the carpet at the far side of the room (14) and close the door (9).
    - Walk back to the start and wait, up to two minutes. When the piglin strolls out, it opens the door: 10.
    - If the piglin is still within 3 blocks of the door the next time it looks back, it shuts the door: 9.

    The first piglin may also come through the door; it opens and shuts it the same way.

## Part B. Mob buckets

Kit (23 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:cod_bucket,minecraft:salmon_bucket,minecraft:tropical_fish_bucket,minecraft:pufferfish_bucket,minecraft:axolotl_bucket,minecraft:tadpole_bucket,minecraft:sulfur_cube_bucket,minecraft:sulfur_cube_bucket,minecraft:water_bucket,minecraft:lava_bucket,minecraft:bucket*16,minecraft:cauldron,minecraft:campfire,minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:rabbit_spawn_egg*4,minecraft:polar_bear_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:silverfish_spawn_egg*2,minecraft:glass*16,minecraft:stone*64,minecraft:iron_shovel,minecraft:cooked_beef*16
```

Start a new world with this kit: survival, Normal, by day, on flat open grass in the Overworld. Only squids and glow squids are live in this build. Fish, axolotls, tadpoles and sulfur cubes are not live yet, so a bucket lets its mob out unseen. The mob stays in the world and is saved with it, and it will show once a later build makes it live. A sculk sensor reacts when a mob comes out, so steps 56 to 62 use a sensor rig. Dispensers also empty fish buckets in this build, but nothing can be put into a dispenser yet (there is no dispenser screen), so no step tests that.

56. **The sensor rig.** Stand still near where you first appeared and run these six commands:

    ```
    /fill ~-2 ~-1 ~-2 ~2 ~-1 ~22 minecraft:stone
    /fill ~-2 ~ ~-2 ~2 ~3 ~22 minecraft:air
    /setblock ~ ~ ~4 minecraft:sculk_sensor
    /setblock ~ ~ ~5 minecraft:comparator[facing=north]
    /fill ~ ~ ~6 ~ ~ ~20 minecraft:redstone_wire
    /setblock ~ ~-1 ~2 minecraft:air
    ```

    They build:
    - a stone floor 5 blocks wide, cleared 4 blocks high;
    - a sculk sensor 4 blocks south of you;
    - a comparator on the sensor's far side, with 15 redstone dust in a straight line from it;
    - a 1 x 1 hole, one block deep, in the floor 2 blocks south of you, between you and the sensor.

    The block you stood on is the start. When the sensor hears something, its tendrils click. For 1.5 seconds the comparator then lights as many dust blocks as the frequency of the sound, counting from the comparator. Wait 2 seconds between actions, and stay crouched at the rig, because crouched steps make no vibration. The floor is bare stone, so a mob that walks within 8 blocks of the sensor lights 1 and keeps the sensor busy for 2 seconds. Wait until it has walked off, then repeat that action.
57. **Plain buckets as before.** Stand on the start facing the sensor and crouch. Pour the water bucket into the hole: 13 dust light (emptying a bucket). Take the water back with the empty bucket you now hold: 12 dust light (filling a bucket). Do the same with the lava bucket: 13, then 12.
58. **A cod comes out.** Use the cod bucket on the floor of the hole. The hole fills with water, you hear the fish bucket's own splash (not the plain pour), and you hold an empty bucket. No cod shows, but 14 dust light. A mob placed (14) beats the water (13), so the cod came out. **Screenshot 25** while the 14 are lit. Take the water back with the empty bucket (12).
59. **Axolotl and tadpole.** Wait 2 s and use the axolotl bucket on the floor of the hole. Take the water back, wait 2 s, then do the same with the tadpole bucket. Each one pours water, leaves an empty bucket in your hand and lights 14 dust. Neither mob shows. The axolotl bucket makes the same splash as the cod bucket, because 26.3 uses the fish splash for it. The tadpole bucket plays that splash higher and softer. Take the tadpole's water back too (12), so the hole is empty.
60. **A sulfur cube bucket pours nothing.** Wait 2 s and use a sulfur cube bucket on the floor of the hole. No water comes out. You hear the bucket's own sound, not a splash, and you hold an empty bucket. 14 dust light, so the cube came out unseen. **Screenshot 26.**
61. **A cauldron as before.** Stand up, walk 15 blocks from the rig onto open grass and place the cauldron. Use the water bucket on it: it fills to the brim with the plain pour sound, and you hold an empty bucket. Use that empty bucket on it: you get the water bucket back, and the cauldron is empty again.
62. **Creative keeps a mob bucket.** Run `/gamemode creative`. Go back to the start, crouch and wait 2 s. Use the salmon bucket on the floor of the hole: the hole fills with water with the fish splash, 14 dust light, and you still hold the salmon bucket. Take the water back with an empty bucket.
63. **Not into a cauldron.** Still in creative, go back to the empty cauldron and use the salmon bucket on it. The water lands on top of the cauldron or beside it, as on any block that holds no water, with the fish splash. The cauldron stays empty: only a plain water bucket fills a cauldron, as in vanilla. **Screenshot 27.** Take the water back with an empty bucket.
64. **The campfire.** Still in creative, place the campfire on dry grass 5 blocks from the cauldron: it burns. Use the salmon bucket on it: the flames go out with a hiss and a puff of smoke, then you hear the fish splash. You still hold the salmon bucket. **Screenshot 28.** Run `/gamemode survival`.
65. **The Nether.** At least 10 blocks from the rig, build a Nether portal frame from the obsidian (4 wide and 5 high, corners left out). Light it with the flint and steel and go through. Use the tropical fish bucket on the ground in front of you: you hear a short, high hiss and see a little puff of grey smoke, and no water appears. You hold an empty bucket, and no fish shows. **Screenshot 29.** Go back through the portal.
66. **Buckets on mobs as before.** Go onto open grass at least 15 blocks from everything you have built. Use a cow egg and a silverfish egg on the grass. The silverfish crawls at you with no footstep sounds, as before. It still hisses now and then. Use an empty bucket on the cow: you get a milk bucket. Use the water bucket on the silverfish: it is not picked up. The water pours out on the ground where you aimed, as from any water bucket. Take the water back with an empty bucket. Then run `/difficulty peaceful`, then `/difficulty normal`, to clear the silverfish.
67. **Steps and splashes as before.** Lay a 5 x 5 floor of stone on the grass and use a polar bear egg on it. As the bear walks on the stone, you hear its own heavy steps. **Screenshot 30.** Use two rabbit eggs on the grass beside the floor: their hops make grass step sounds. Pour the water bucket on the grass next to them: a rabbit that hops into the water splashes. Take the water back with an empty bucket.
68. **Rabbits drown as before.** Dig a 1 x 1 hole 3 blocks deep in the grass with the shovel. Use a rabbit egg on its bottom and pour the water bucket into the hole straight away. Look down into the hole and place glass against its wall on the second block down. The glass now sits right on top of the water, so the rabbit can't come up for air. If the glass won't go in, wait a moment and try again. About 16 s after you place the glass, the rabbit flashes red, and one second later it dies. **Screenshot 31** as it flashes or dies.

## Part Q. Squids and glow squids

Kit (9 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:squid_spawn_egg*16,minecraft:glow_squid_spawn_egg*16,minecraft:iron_sword,minecraft:lead*4,minecraft:golden_dandelion*4,minecraft:spawner,iw4l:m4,iw4l:rifle_ammo*64,minecraft:cooked_beef*32
```

Setup: start a new world. Play in survival on Normal, by day. Use the class's M4A1 on slot 1. The kit's own M4A1 is for the friends' steps. Move the squid eggs, the glow squid eggs, the iron sword, the leads and the golden dandelions to the hotbar, and keep one hotbar slot empty for punches. In survival a bullet also mines the block it hits, and two or three bullets break a glass block. So shoot only at squids in open water with no glass between you and them, never at one under the platform or in line with the walkway. Hatch no squids until the platform is built, because hatched squids count toward the limit of water animals around you, and fewer wild ones would then spawn. Squids are quiet, so turn the sound up. An egg reaches only about 4 blocks and a sword hit about 3, so hatch each squid close to the platform's edge and hit it before it swims off. Read your position with `/tp @s ~ ~ ~`.

69. **Squids in a river.** Run `/gamemode creative`, then `/locate biome minecraft:river`, then `/tp @s <x> 75 <z>` with the x and z it gives. Fly slowly along the river, about 8 blocks above the water, for two minutes. Dark blue squids appear in the river near the surface, one to four together. They appear at least 24 blocks from you, so look ahead along the river. **Screenshot 32** of one. Rivers are narrow and squids are their only water animal, so if none show in two minutes, note it and go on: step 70's ocean is the main check.
70. **Squids in the ocean.** Run `/locate biome minecraft:lukewarm_ocean`, then `/tp @s <x> 75 <z>` with its x and z. Fly round slowly 8 blocks above the sea for two minutes. Squids appear here too, near the surface. They spawn only in the top 13 blocks of water, from y 50 up to the surface.
71. **The platform.** Fly down until you hover a block or two above the water. Then run these two commands:

    ```
    /fill ~-2 62 ~-2 ~2 62 ~2 minecraft:glass
    /fill ~3 62 ~ ~32 62 ~ minecraft:glass
    ```

    The top layer of water under you becomes a 5 x 5 glass platform, level with the sea. A glass walkway 30 blocks long runs east from it. Run `/tp @s ~ 63 ~` to stand in the middle of the platform, then run `/gamemode survival`. You can watch the squids through the glass. When you face along the walkway (east), the platform's south edge is on your right. The steps below use that edge.
72. **Swimming in pulses.** Stand on the south edge, aim at the water 2 to 3 blocks out and use the squid egg three times. Watch the dark blue squids:
    - On each stroke, the eight tentacles spread out, then snap shut as the squid shoots forward. Then it glides, slows down and keeps slowly spinning round its long axis. A stroke takes 1.5 to 3 seconds.
    - Its body tips toward where it swims. It lies flat with its tentacles trailing behind when it swims sideways, and it stands upright when it rises.
    - It makes no swimming sound. Now and then, at least 6 seconds apart, you hear its quiet call.
    - Run `/summon minecraft:squid ~ ~5 ~3`. The squid drops into the water with a splash.

    **Screenshot 33**, a squid mid-stroke.
73. **Ink and flight.** Use a squid egg on the water just off the south edge. Before it swims off, hit it once with the iron sword:
    - It flashes red, and you hear its hurt sound and a squirt. A cloud of black ink bursts out from among its tentacles, spreads, fades and is gone within about a second and a half.
    - The squid darts away from you, a few blocks with each stroke. This lasts up to 5 seconds, or until it is about 10 blocks away. Then it swims as before.

    **Screenshot 34** with the ink cloud.
74. **Shot from near and far.** Select the M4A1. Shoot once at a squid about 8 blocks from you: it squirts ink and shoots off away from you. Then walk 12 blocks out along the walkway. Shoot once at a squid at least 12 blocks from you, off to the side of the walkway in open water, not one in line with the walkway or under the platform (the bullets would break the glass): it squirts ink too, but it does not flee and swims on as before. A squid flees only from an attacker within 10 blocks.
75. **Death.** Shoot a squid until it dies (two or three hits). With the killing hit you hear its death sound. It flashes red and keeps drifting the way it was going, without tipping over or sinking, and a second later it vanishes in a puff of smoke. It leaves 1 to 3 ink sacs and experience orbs.
76. **Drying out on land.** Go back to the middle of the platform. Use a squid egg on the glass 2 blocks from you, and count the seconds:
    - It stays on its spot with its tentacles beating, and it slowly tips over until it lies flat.
    - At about 16 s it flashes red with its hurt sound. It flashes again every second after that, and it dies at about 20 s.
    - It never squirts ink, because nobody hit it. It leaves 1 to 3 ink sacs but no orbs, because no player killed it.

    **Screenshot 35** while it lies flat.
77. **A fall, and ink in the air.** Stand in the middle of the platform and run `/summon minecraft:squid ~ ~8 ~1`. The squid drops 8 blocks onto the glass beside you with the heavy fall sound and flashes red, but it squirts no ink. Wait a second. Then select an empty hotbar slot and punch it once with your bare hand. A black ink cloud puffs out of its tentacle end, spreads, fades and slowly sinks through the air. The squid cannot flee on land. **Screenshot 36** with the ink in the air. One sword hit kills it: it leaves ink sacs and orbs.
78. **The glow squid.** Run `/time set midnight`. At night, drowned may come up out of the sea; shoot them with the M4A1. Use the glow squid egg on the water just off the south edge. A teal squid swims like the others, but it glows at full brightness against the dark water. Tiny teal and pale green sparkles keep drifting off it, and each one brightens before it goes out. **Screenshot 37.**
79. **Going dark.** Hit the glow squid once with the sword, or shoot it once with the M4A1 from about 8 blocks. It squirts a teal ink cloud with its own squirt sound and flees. At the same moment, it goes as dark as the water around it, and its sparkles keep coming. After 5 s it brightens to full again within half a second. **Screenshot 38** while it is dark.
80. **A glow squid on land.** Use a glow squid egg on the glass and count the seconds. Like the squid that dried out before, it lies there and tips flat. From about 16 s it flashes red every second, and it dies at about 20 s. It stays bright the whole time and never squirts ink, because only a squirt makes it go dark. It leaves 1 to 3 glow ink sacs. Then run `/time set day`.
81. **Babies.** Stand on the south edge, facing out over the water.
    - Run `/summon minecraft:squid ~ ~-1 ~2 {Age:-24000}`. A baby squid appears. It is about half as big as an adult, with a rounder body and short tentacles, and it swims in the same pulses. **Screenshot 39**, with an adult in view if one is near.
    - Run `/summon minecraft:squid ~-2 ~-1 ~2 {Age:-100}`. Another baby appears, and 5 s later it suddenly becomes a full-size squid.
    - Shoot the first baby with the M4A1 until it dies. It leaves no ink sacs and no orbs.
82. **The golden dandelion.** Still on the south edge, select the golden dandelion first, because this baby grows up 5 s after the command. Then run `/summon minecraft:squid ~ ~-1 ~1 {Age:-100}`. Right away, right-click the new baby, just below you, with the golden dandelion. One dandelion is used, and you hear its sound. Wait 10 s. This baby stays small, while the baby from the step before grew up after 5 s. If an adult swims within reach, right-click it with the golden dandelion: nothing happens, and the dandelion is not used.
83. **Saved.** Keep the locked baby in view. Save and quit, then load the world. You are standing on the platform, and the locked baby is still small and swimming. The squids near the platform are back and swimming. No squid puffs out ink as the world loads.
84. **A lead.** Walk 3 blocks out along the walkway. Use a squid egg on the water right beside the walkway. Right away, right-click the squid with a lead, and the lead ties to it. Walk slowly on along the walkway for about 10 blocks. Once you are about 6 blocks from the squid, the lead pulls it along after you through the water. **Screenshot 40.** Then run `/tp @s ~14 ~ ~`. The lead snaps with its breaking sound and drops into the water beside the squid.
85. **In a spawner.** Go back to the platform and run `/gamemode creative`. Place the spawner on the platform and use a squid egg on it. A small squid turns slowly inside the cage, with its tentacles hanging out below it. **Screenshot 41.** Break the spawner and run `/gamemode survival`.
86. **Glow squids in the deep.** Run `/gamemode creative`, then `/locate biome minecraft:deep_ocean`, then `/tp @s <x> 70 <z>` with its x and z. Fly straight down to the sea floor (hold C). The deep water is dark, but the glow squids glow on their own. Run `/tp @s ~ ~ ~` and read your height, which is the middle number. Glow squids spawn only in dark water at y 30 or lower. If the floor here is higher, fly on over it until it is at y 30 or lower; deep ocean floors dip that low in places, and underwater ravines go much deeper. Stay near the floor and look round for up to five minutes. Glow squids appear in the dark water, often several together, glowing teal with their sparkles. **Screenshot 42.** If none appear in five minutes, note it. Then run `/gamemode survival`.

## Part L. Lightning

Kit (18 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*16,minecraft:oak_planks*16,minecraft:netherite_ingot,minecraft:experience_bottle*16,minecraft:water_bucket,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:obsidian*10,minecraft:light_blue_stained_glass*16,minecraft:weathered_copper*16,minecraft:weathered_lightning_rod*2,minecraft:redstone_lamp*2,minecraft:composter*2,minecraft:villager_spawn_egg*4,minecraft:creeper_spawn_egg*2,minecraft:cooked_beef*32
```

Survival, Normal, by day, on flat open plains with few grass tufts (fire spreads through them). Play close to where you first appeared, so you respawn nearby after step 92. Run `/weather clear` and `/gamerule keep_inventory true`. Stand still and run `/setblock ~ ~-1 ~ minecraft:gold_block`, then `/setblock ~ ~-1 ~8 minecraft:gold_block`. The gold block under your feet is **your place**. The one 8 blocks south is **the spot**. Face the spot: you are looking south, so east is on your left and west on your right. Unless a step says otherwise, run every command standing on your gold block, so `~ ~ ~8` is always the spot. Until step 92, put out fires with the water bucket and pick the water up again. A bolt lasts less than a second, so if you miss a screenshot, summon another one. Lightning also goes through a raised shield, but you can't hold a shield up while you type a command, so this part does not test that.

87. **A bolt.** Run `/summon minecraft:cow ~ ~ ~8`, then `/summon minecraft:lightning_bolt ~ ~ ~8`. The chat says "Summoned new Lightning Bolt". A jagged blue-white bolt with two side forks stands on the spot and reaches up out of sight. You hear a long roll of thunder and a sharp crack, and the sky flashes a little paler. The bolt jumps to a new shape one to three times and is gone in under a second. Fire lights on the spot's gold block and on up to four blocks right around it. The cow flashes red and loses half its health. A bolt hits everything near it on each of its flashes, and a later flash that comes half a second or more after the first hurts again in full, so if one does, the cow dies at once. Otherwise it runs off burning, and the burns may kill it. With the bolt in view: **Screenshot 43.** Put the fires out.
88. **Dropped things burn.** Walk 6 blocks past the spot (14 blocks south of your place). Select the cobblestone and press Ctrl+Q to drop it. Walk back to the spot and stand on its gold block facing south. Drop the oak planks with Ctrl+Q and the netherite ingot with Q. Walk back to your place without crossing them. Throw three bottles o' enchanting at the spot, then at once run `/summon minecraft:lightning_bolt ~ ~ ~8`. The planks, the netherite ingot (which fire and lava can't burn) and the experience orbs around the spot all vanish. Orbs that have already rolled more than 3 blocks toward you are spared and come to you. The cobblestone 6 blocks further on stays. **Screenshot 44.** Put the fires out.
89. **Easy.** Run `/difficulty easy` and `/summon minecraft:cow ~ ~ ~8`, then summon the bolt at the spot. The cow is hurt (it dies if a later flash hits it again, as in step 87), but only the spot's gold block catches fire, never a block beside it. On Easy, only the bolt's later flashes light a fire, and only on its own block. Do it once more with a new cow. **Screenshot 45.** Run `/difficulty normal` and put the fire out.
90. **In armour.** Put on the iron armour. Run `/summon minecraft:lightning_bolt ~ ~ ~2`. The bolt strikes 2 blocks in front of you and still hits you. You lose a little over one heart, or two and a half when a later flash of the bolt hits you again, and you are not pushed back. Fire lights only in front of you, not under you. Put it out.
91. **A portal.** About 10 blocks to your left, build a Nether portal frame standing on the ground from the 10 obsidian: 4 wide and 5 high, with the corners left out. Step into the frame onto its bottom row and run `/summon minecraft:lightning_bolt ~ ~ ~`. Step out at once, before the portal takes you. The frame fills with the purple portal, and you lose a little over one heart through the armour, or two and a half when a later flash hits you again. **Screenshot 46.** Put out any fire on the grass.
92. **Struck dead.** Run `/gamerule fire_spread_radius_around_player 0`. From now on, bolts light no fires until the last step turns this back. In vanilla, fire spreads and bolts light fires only within that many blocks of a player. Eat cooked beef and wait until your hearts are full. Then run `/gamerule natural_health_regeneration false`, and take the armour off. On your gold block, run `/summon minecraft:lightning_bolt ~ ~ ~` again and again. Each hit takes two and a half hearts, twice what a hit took through the armour, and you never catch fire. When a later flash of the same bolt comes half a second or more after the first, it hits you again, so a bolt takes two and a half or five hearts, about half the time each. You die on the second, third or fourth bolt, and the chat says "Sos was struck by lightning". **Screenshot 47.** Respawn, run `/gamerule natural_health_regeneration true` and walk back to your gold block.
93. **Fire-proof mobs are hurt too.** Run `/summon minecraft:blaze ~-8 ~ ~8 {NoAI:1b}` and `/summon minecraft:polar_bear ~-8 ~ ~9 {NoAI:1b}`. They appear 8 blocks to the right of the spot. Then run `/summon minecraft:lightning_bolt ~-8 ~ ~8`. Both flash red with their hurt sounds, and both live. The bolt takes 5 or 10 of the blaze's 20 health and 5 or 10 of the bear's 30 (10 when a later flash hits again). Fire never hurts a blaze, but lightning does.
94. **The night lights up.** Run `/time set midnight` and summon the bolt at the spot. At each flash, the whole land lights up as if it were day, and the sky turns pale blue for a moment. During a flash: **Screenshot 48.**
95. **Far bolts.** Run `/summon minecraft:lightning_bolt ~ ~ ~70`. No bolt shows, because bolts are drawn only within 64 blocks, but you hear the thunder and the night lights up. Then run `/summon minecraft:lightning_bolt ~ ~ ~40`: this one shows, far off. **Screenshot 49.**
96. **Through glass and on water.** Build a wall of light blue stained glass 5 wide and 2 high on the ground 4 blocks in front of you, between you and the spot. Summon the bolt at the spot. It shows through the glass, brighter than the blue glass around it. **Screenshot 50.** Break the glass. Walk to the spot, pour the water bucket on its gold block, then walk back and summon the bolt again. It stands bright over the water. Pick the water up again.
97. **In the rain.** Run `/weather rain` and summon the bolt at the spot. The rain falls in front of the bolt. **Screenshot 51.** Run `/weather clear` and `/time set day`.
98. **Struck copper.** Place a weathered copper block on the spot's gold block. Run `/summon minecraft:lightning_bolt ~ ~1 ~8`, so the bolt lands on top of the block. The block turns orange at once, like new copper, and you may see a few tiny white-violet sparks on its faces. **Screenshot 52.** Leave the block there.
99. **A struck rod.** Place weathered copper on the ground right next to the orange block on three sides: the side facing you, the far side and your right. Place the redstone lamp against its left side. Put the weathered lightning rod on top of the orange block. Run `/summon minecraft:lightning_bolt ~ ~2 ~8`, so the bolt lands on the rod's tip:
    - the lamp lights for about half a second;
    - the green-grey rod turns into a shiny orange rod at once;
    - 10 to 19 tiny white-violet sparks dart up and down along the rod and are gone within a tenth of a second;
    - some of the weathered copper blocks turn mostly orange again, each with a few sparks on all its faces.

    **Screenshot 53.** The sparks are quick, so summon more bolts on the rod until you catch them. Each bolt lights the lamp again and cleans more copper.
100. **A lying rod.** Run `/setblock ~ ~ ~12 minecraft:stone_bricks` and `/setblock ~1 ~ ~12 minecraft:lightning_rod[facing=east]`. This puts a stone brick block 4 blocks past the spot, with a lightning rod lying flat against its east side and pointing east. Run `/summon minecraft:lightning_bolt ~1 ~1 ~12`. The sparks dart east and west along the lying rod. **Screenshot 54.**
101. **Pigs.** Run `/gamemode creative` and break the orange block, the rod, the lamp and the three copper blocks around it. Leave the gold block. Run `/gamemode survival`. Run `/summon minecraft:pig ~ ~ ~8 {equipment:{saddle:{id:"minecraft:saddle",count:1}}}` to get a saddled pig, then summon the bolt at the spot. The pig becomes a zombified piglin holding a golden sword (one time in twenty, a golden spear). The saddle is gone, and none drops. Then run `/summon minecraft:pig ~ ~ ~8 {Age:-24000}` and summon the bolt again: you get a baby zombified piglin. Each one flashes red a moment after it appears, because the rest of its bolt hits it, but neither gets angry. Walk up and stand beside them for half a minute: they leave you alone. Don't hit them. **Screenshot 55.**
102. **They stay.** Run `/summon minecraft:zombified_piglin ~3 ~ ~8`. This makes a third zombified piglin without lightning. Run `/gamemode creative`, double-tap jump to fly and rise a few blocks. Run `/tp @s ~100 ~ ~-100`, which puts you about 140 blocks from them. Wait 10 seconds, then run `/tp @s ~-100 ~ ~100` and land on your gold block. If you dropped to the ground while you were away, walk back to it. The third zombified piglin is gone, because a mob more than 128 blocks from every player despawns. The blaze from step 93 is gone too, for the same reason; the polar bear stays, because animals don't despawn. The two from the pigs are still there, because lightning made them persistent. **Screenshot 56.** Run `/difficulty peaceful`, then `/difficulty normal`: the zombified piglins vanish. Stay in creative until the last step.
103. **A mooshroom flips.** Run `/summon minecraft:mooshroom ~ ~ ~8` and summon the bolt at the spot. The red mooshroom turns brown with a short transform sound. It doesn't flash red, and it doesn't run. It flips only once per bolt, even though the bolt flashes several times. **Screenshot 57.** Summon another bolt at the spot: it turns red again.
104. **A charged creeper.** Run `/summon minecraft:creeper ~ ~ ~8` and summon the bolt at the spot. The creeper flashes red and becomes charged. A blue swirl a little larger than its body wraps it and slides slowly and steadily on a diagonal, with its green skin showing through the gaps. Walk up to it (in creative it leaves you alone) and watch it walk: the swirl turns and walks with it. **Screenshot 58.**
105. **In the dark.** Run `/time set midnight`. The creeper darkens with the night, but the swirl stays as bright as it was by day. **Screenshot 59.**
106. **Saved.** Save and quit, then load the world: the creeper still wears the swirl. Use a creeper egg beside it: the new creeper has no swirl. Run `/time set day`.
107. **Twice the blast.** Run `/summon minecraft:creeper ~10 ~ ~16 {ignited:1b}`: it swells and blows up within 2 seconds. Then run `/summon minecraft:creeper ~ ~ ~16 {ignited:1b,Fuse:400}`, and right after it `/summon minecraft:lightning_bolt ~ ~ ~16`. This creeper gets the swirl and swells slowly, flashing white, and the swirl swells and wobbles with it. About 20 seconds after you summoned it, it blows up. Its crater is about twice as wide as the first one. With both craters: **Screenshot 60.** If the creeper walked more than 3 blocks before the bolt came, it stays plain and leaves a small crater; then do it again. Run `/difficulty peaceful`, then `/difficulty normal`, to clear the other creepers.
108. **A villager becomes a witch.** Walk 12 blocks north, behind your place. Place a composter on the ground and use a villager egg 2 blocks from it. Wait until the villager wears a farmer's straw hat (up to a minute by day). Stand on the ground about 5 blocks from it and look straight at it with your view level. Run `/summon minecraft:lightning_bolt ^ ^ ^5`, which strikes 5 blocks in front of you. The villager becomes a witch. **Screenshot 61.**
109. **Its job is free again.** Use a second villager egg 4 blocks from the composter. Within about a minute, the new villager becomes a farmer at the same composter, because the first villager gave it up. **Screenshot 62.**
110. **Effects carry over.** Back on your gold block, run `/summon minecraft:villager ~ ~ ~8 {active_effects:[{id:"minecraft:glowing",duration:6000}]}`. The villager shows no outline, because older mobs don't get one yet. Summon the bolt at the spot. The witch it becomes shows the white outline, because it kept the villager's Glowing effect. **Screenshot 63.**
111. **Peaceful.** Run `/difficulty peaceful`: both witches vanish. Run `/summon minecraft:pig ~ ~ ~8` and summon the bolt at the spot. The pig flashes red, loses health and runs, but it stays a pig. If a later flash of the bolt hits it again, it dies, still a pig, and no zombified piglin appears; then summon another pig and try again. Stand 5 blocks from the farmer villager, look straight at it and run `/summon minecraft:lightning_bolt ^ ^ ^5`. It flashes red and stays a villager. **Screenshot 64.** Run `/difficulty normal`, `/gamemode survival` and `/gamerule fire_spread_radius_around_player 128`.

## Part H. Phantoms

Kit (11 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:phantom_spawn_egg*16,minecraft:spawner,minecraft:diamond_sword+looting=3,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:red_bed,minecraft:cobblestone*64,minecraft:golden_apple*8,minecraft:cooked_beef*32
```

Setup: start a new world. Play in survival on Normal, on flat open grass in plains, with no trees within 20 blocks. Your feet must be at y 63 or higher: run `/tp @s ~ ~ ~`, and the second number is y. Put on the diamond armour; the steps say when to take it off. Run `/gamerule keep_inventory true`, `/gamerule advance_time false`, `/weather clear` and `/time set midnight`. With `advance_time` off, the time stays wherever `/time set` puts it, so the night lasts as long as the steps need. Step 133 turns it back on. Other monsters come at night too: shoot them with the class's M4A1. At any time, `/difficulty peaceful` then `/difficulty normal` clears every phantom and monster.

112. **Circling.** Run `/gamemode creative`, so phantoms leave you alone. Use a phantom egg on the ground 5 blocks in front of you.
     - A dark blue-grey phantom with long thin wings and two small glowing green eyes flies round and round. It stays 5 to 15 blocks from a point about 5 blocks above where it hatched.
     - Its wings beat slowly up and down, and its tail curls with each beat.
     - It tips its body up when it climbs and down when it sinks.
     - Now and then you hear its screeching call.
     - While you are in creative, it never comes for you.

     **Screenshot 65.**
113. **Flaps and specks.** Stand still and listen. About every two and a half seconds, a soft wing flap comes from it as its wings sweep down. Small dark grey specks trail from both wingtips. They hang in the air for one to five seconds, then fade. With specks in view: **Screenshot 66.**
114. **Two sizes.** Look straight ahead and run `/summon minecraft:phantom ^3 ^3 ^6 {NoAI:1b,size:4}`, then `/summon minecraft:phantom ^-3 ^3 ^6 {NoAI:1b}`. Both hang in the air 6 blocks ahead and beat their wings in place, each flapping every two and a half seconds. The size 4 one is 1.6 times as large as the other, and its specks start further out to the sides. The specks of both rise and fall with the beat. **Screenshot 67.**
115. **Burning by day.** Run `/time set noon` and leave the three phantoms where they are.
     - Within a few seconds each one catches fire, with flames on it.
     - Its hurt sound comes about once a second.
     - After about 20 seconds each one dies with its death sound and tips over.
     - Nothing drops, no experience and no membrane, because no player killed them.

     While they burn: **Screenshot 68.**
116. **Not in rain.** When all three are gone, run `/weather rain` and wait until it rains around you. Run `/summon minecraft:phantom ^ ^3 ^5 {NoAI:1b}`. For 30 seconds it does not catch fire and makes no hurt sound. Run `/weather clear`: within about 10 seconds, as the rain stops, it catches fire and burns like the others.
117. **Swoop and bite.** Run `/time set midnight` and take the armour off. Use a phantom egg on the ground 5 blocks away, then run `/gamemode survival` and stand still in the open.
     - Within a few seconds a loud swoop screech sounds, and it dives at you head first, pointing down at you.
     - Its crunching bite takes 1 heart. Then it levels out and climbs away.
     - Every few seconds it comes again: a screech, a dive and a bite.

     After the second bite, with your hearts: **Screenshot 69.**
118. **Under a roof.** Run `/fill ~-2 ~2 ~-2 ~2 ~2 ~2 minecraft:cobblestone`. This puts a 5 by 5 roof 2 blocks above your feet, with you in the middle under it. Leave the roof there, because steps 122 to 125 use it. A dive that has already started may still reach you. After that, wait 30 seconds: no swoop screech and no bite. Walk 5 blocks out into the open: within about 10 seconds it dives and bites again. Run `/difficulty peaceful`: the phantom vanishes, and no screech comes again. Run `/difficulty normal`.
119. **Hits.** Put the armour back on. Look straight ahead and run `/summon minecraft:phantom ^ ^1 ^3 {NoAI:1b}`. Hit it with the diamond sword at full charge.
     - Each hit flashes its body red with its hurt sound, but its eyes stay green.
     - It dies within three or four hits and tips over onto its side.
     - It drops experience, often with a phantom membrane (the sword has Looting III).

     At a hit, with the red flash: **Screenshot 70.** Then run `/summon minecraft:phantom ^ ^1 ^10 {NoAI:1b}` and shoot it with the M4A1 from where you stand. Each hit flashes it red, and a few hits kill it.
120. **In the dark.** Walk 10 blocks away from the roof and stand still. Run `/fill ~-2 ~-1 ~-2 ~2 ~3 ~2 minecraft:cobblestone hollow`. This shuts you in a cobblestone room, 3 by 3 and 3 high, with no light inside. Look straight ahead and run `/summon minecraft:phantom ^ ^1 ^1 {NoAI:1b}`. Its body is so dark you can hardly see it, but its two eyes glow bright green. **Screenshot 71.** Kill it with the sword. Without moving, run `/fill ~-2 ~ ~-2 ~2 ~3 ~2 minecraft:air` to open the room. The floor stays.
121. **A spawner.** Stay at least 10 blocks from the roof. Run `/gamemode creative`. Place the spawner on the ground 4 blocks in front of you and use a phantom egg on it. A small phantom turns slowly inside the cage, its wingtips poking out through the cage's sides. **Screenshot 72.** Within 40 seconds, up to four phantoms appear around the spawner and circle there. In creative they leave you alone. Then break the spawner, so it makes no more. Run `/gamemode survival`: within a few seconds they come for you, swoop and bite. Run `/difficulty peaceful`, then `/difficulty normal`.
122. **A big one.** Go back to the roof. Take the armour off and eat cooked beef until your hearts are full. Don't eat a golden apple here, because its golden hearts would take the bite first. Stand in the open 2 blocks from the roof's edge and run `/summon minecraft:phantom ~ ~8 ~ {size:6}`. It is 1.9 times as large as a plain phantom. Within a few seconds it dives at you, and its bite takes 6 hearts. Right after the bite, walk under the middle of the roof. It can't see you there, so it doesn't dive. With your hearts after the bite: **Screenshot 73.**
123. **Saved, with a sleepless clock.** Still under the roof, Save and Quit.
     - In Explorer, open `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\saves\<world>\players\stats`. It holds one file, `<your UUID>.json`.
     - Open it in Notepad, find `"minecraft:time_since_rest":` and change the number after it to `1000000`. That counts as being awake for about 42 in-game days. Save the file.
     - Load the world. It is still midnight, and the big phantom circles outside the roof.

     From now on, a death sets this clock back to 0. If you die before step 129, Save and Quit and set the number again.
124. **The big one after loading.** Under the roof, eat cooked beef until your hearts are full. Walk 5 blocks out into the open. Within about 10 seconds it dives, and its bite still takes 6 hearts. Walk back under the roof. Run `/difficulty peaceful` (it vanishes), then `/difficulty hard`, and put the armour back on.

     Steps 125 to 130 are on Hard unless a step says otherwise. The game looks for sleepless players every one to two minutes. On Hard, three looks in four bring phantoms; on Normal, about half do.
125. **Not under a roof.** Stay in the middle under the roof for 5 minutes: no phantom comes.
126. **Phantoms for the sleepless.** Walk 20 blocks from the roof onto open ground and stand still. Usually within 3 minutes, and at most about 6, 1 to 4 phantoms appear together high above you: 20 to 34 blocks up and up to 10 blocks to the side. They screech and dive at you. With them above you: **Screenshot 74.** Fight them with the sword or the M4A1 if they wear you down.
127. **Peaceful.** Run `/difficulty peaceful`: every phantom vanishes. Stay in the open for 3 minutes: none come. Run `/difficulty hard`.
128. **The rule.** Run `/gamerule spawn_phantoms false` and stand in the open for 5 minutes: none come. Run `/gamerule spawn_phantoms true`: within about 6 minutes they come again.
129. **Sleep.** Run `/difficulty peaceful`, so no phantom or monster keeps you from sleeping. Place the bed on open ground and right-click it: you lie down. In bed: **Screenshot 75.** After about 5 seconds you get up again. It is still midnight, because the clock is stopped. Sleeping set your clock back to 0.
130. **Rested.** Run `/difficulty hard` and stand in the open beside the bed for 5 minutes: no phantom comes. Phantoms only come again after three in-game days awake (72000 ticks, an hour of play). Run `/difficulty normal`.
131. **An ocelot.** Use a phantom egg on the ground 5 blocks away and stand still in the open. Once it has bitten you, run `/summon minecraft:ocelot ~1 ~ ~ {NoAI:1b}`: an ocelot stands still right beside you. Stay next to it. The phantom keeps diving and biting, and no hiss comes, because ocelots don't scare phantoms.
132. **A cat far away.** Look straight ahead and run `/summon minecraft:cat ^ ^ ^40 {NoAI:1b}`. The cat stays where it appears, 40 blocks away. The phantom keeps biting you as before.
133. **A cat beside you.** Run `/summon minecraft:cat ~-1 ~ ~ {NoAI:1b}`: a cat stands still beside you, on the other side from the ocelot. Stay between them. The swoop screeches still come, but most dives now break off before the phantom reaches you. The cat hisses, and the phantom climbs away without biting. With the cat beside you and the phantom above: **Screenshot 76.** Run `/difficulty peaceful`, `/difficulty normal` and `/gamerule advance_time true`.

## Part F. Friends (two windows, both on this build)

Use `scripts\mp-local.ps1` or the relay. First put Part A's kit line in the game folder's `.env` (`C:\Games\Mashup\2010-Rust-Rewrite-Mashup\.env`) as `IW4L_MINECRAFT_KIT=...`: in a new world, both players get the kit in that file on their first join, with the class loadout. For two windows on this PC, run from PowerShell in `C:\Dev\Soscraft\mashup-crafting`:

```
& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r7friends -Fresh -HostSound
```

This opens the host window and a friend window. That PowerShell window keeps running the script until both game windows close, so run any other command from a second PowerShell window in `C:\Dev\Soscraft\mashup-crafting`. Leave out `-Fresh` to go on with a world. To start only the friend window again after the friend quit, run `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly` from that second window. `Sos` is the host's player name and `Friend` the friend's; use your names if they differ. In every new world the host types `/difficulty normal`, `/gamerule keep_inventory true` and `/op Friend` first.

Kits within a world: the host's `/kit` reads `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\mp-local\host\.env`, the host window's own copy, which the script writes from the game folder's `.env` only when it starts the host. So where a section below says "give the next kit", do this: the host runs `/clear @a`; you put that section's kit line in the host's `.env` above as `IW4L_MINECRAFT_KIT=...` and save it; the host runs `/kit @a`. `/kit` reads the file again every time. Clear first, because what doesn't fit in a full inventory drops at the player's feet.

Pandas, mob buckets and squids run in this first world. The game events, lightning and phantoms each get a fresh world of their own after that.

134. **Old build refused.** The friend quits to the main menu and closes the friend window; the host stays. In the second PowerShell window, start release 6 as the friend: `& scripts\mp-local.ps1 -Exe iw4l-survival-789f6d2.exe -FriendOnly`. It is refused with "Incompatible client! Please use the host's build (Minecraft multiplayer protocol 12)". **Screenshot 77.** Close it and start the friend on this build from the same window: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly`. The friend comes back with the kit from the first join.

### Pandas (Part A's kit)

Both players have Part A's kit from their first join. Play in survival, Normal, by day, on flat open grass, with the friend 5 blocks from the host. The host runs every `/summon` and `/weather` below.

135. **The same pandas.** The host runs the seven commands of step 1 and the two of step 4. Both windows show the same seven faces and the two cubs in the same places. The lazy panda flops onto its back and the playful one somersaults at the same moments in both windows. **Screenshot 78**, both windows.
136. **The friend breeds.** On fresh grass 30 blocks away, the host plants a bamboo shoot and bone-meals it into a stalk. The friend uses two panda eggs 4 blocks from the stalk and feeds both bamboo. Both windows show the hearts, the two pandas meeting, the orbs and the new cub. Then the friend right-clicks one parent with bamboo again. In both windows it sits with the bamboo in its paws, and the crumbs and the munching come at the same moments. **Screenshot 79**, both windows.
137. **Sulking for both.** 30 blocks from the stalk, the friend uses two more eggs side by side and feeds both bamboo. Both windows show the head shake and the pawing at the same moment, and both players hear the sad bleat twice.
138. **A thrown stack.** The host runs `/summon minecraft:panda ~ ~ ~4 {CanPickUpLoot:1b}`. The friend throws a whole bamboo stack (Ctrl+Q) 4 blocks from it. Neither player holds bamboo while it fetches and eats, because a panda follows anyone holding bamboo within 10 blocks instead. Both windows show it fetch the stack, sit and munch with crumbs.
139. **Moods.** On fresh grass, the host runs `/summon minecraft:panda ~-4 ~ ~4 {MainGene:"lazy",HiddenGene:"lazy"}` and `/summon minecraft:panda ~4 ~ ~8 {MainGene:"playful",HiddenGene:"playful"}`. Then the host summons the weak cub and the two adults of step 28. Both windows show the lazy panda on its back, the somersaults and the cub's green puff at the same moments, and both players hear the sneezes. **Screenshot 80**, both windows, with the lazy panda on its back. The friend right-clicks the lazy panda with an empty hand: it gets up in both windows.
140. **Thunder.** On fresh grass, the host runs `/summon minecraft:panda ~ ~ ~6 {MainGene:"worried",HiddenGene:"worried"}` and `/weather thunder`. About 5 seconds later, both windows show it sit down, cover its face and tremble at the same moment. The friend right-clicks it with bamboo: nothing happens, and the friend keeps the bamboo. **Screenshot 81**, both windows. The host runs `/weather clear`: it stands up in both windows at the same moment.
141. **The friend in a fight.** The host runs `/summon minecraft:panda ~ ~ ~8 {MainGene:"aggressive",HiddenGene:"aggressive"}`. The friend punches it once. In both windows it goes for the friend and keeps biting the friend, not the host. The friend right-clicks it with bamboo: it stops, with hearts in both windows. Then the friend shoots it with the M4A1 until it dies. Each hit flashes it red in both windows, and its bamboo and orbs drop for both.

### Mob buckets (Part B's kit)

Give the next kit, Part B's. By day (`/time set day`), on flat open grass at least 30 blocks from every panda, the host builds the sensor rig of step 56 with its six commands. The host then stands still 3 blocks to the side of the start, so that only the friend's actions light the line. The friend waits 2 seconds between actions at the rig. As in step 56, a mob that walks near the sensor lights 1 and keeps it busy for 2 seconds; then wait until it has walked off and repeat that action.

142. **The friend's cod bucket.** The friend stands 15 blocks from the rig and uses the cod bucket on the ground. Both windows show the water at once. The friend hears the fish bucket's splash and holds an empty bucket. No cod shows in either window. **Screenshot 82**, both windows. The friend takes the water back with an empty bucket, and it goes in both windows.
143. **Let out on the host.** The friend stands on the start of the rig, facing the sensor, then crouches and waits 2 s. The friend uses a sulfur cube bucket on the floor of the hole: no water comes out, the friend holds an empty bucket, and the host's line shows 14. After 2 s, the friend uses the axolotl bucket on the floor of the hole. The hole fills with water in both windows, the friend hears the fish splash, and the host's line shows 14 again. **Screenshot 83**, the host's window, while the 14 are lit. The friend takes the water back with an empty bucket.
144. **Creative for the friend.** The host runs `/gamemode creative Friend`. The friend uses the salmon bucket on the ground, 15 blocks from the rig. The water shows in both windows, and the friend still holds the salmon bucket. The friend takes the water back with an empty bucket, and the host runs `/gamemode survival Friend`.

### Squids and glow squids (Part Q's kit)

Give the next kit, Part Q's. `/clear @a` also takes the class's M4A1, and Part Q's kit has its own. The host runs `/gamemode creative @a`, then `/locate biome minecraft:lukewarm_ocean`, then `/tp @a <x> 75 <z>` with the x and z it gives. The host flies down until hovering a block or two above the water, then runs `/fill ~-2 62 ~-2 ~2 62 ~2 minecraft:glass` and `/fill ~3 62 ~ ~32 62 ~ minecraft:glass`. Then the host runs `/tp @s ~ 63 ~`, `/tp Friend @s` and `/gamemode survival @a`. Both players stand on the glass platform. When you face along the walkway (east), the platform's south edge is on your right. As in Part Q, both players shoot only at squids in open water with no glass between them and the squid, because two or three bullets break a glass block.

145. **The same strokes.** The host stands on the south edge and hatches two squids and a glow squid in the water 2 to 3 blocks off it. Then the host runs `/summon minecraft:squid ~ ~-1 ~2 {Age:-24000}`. Both windows show the same pulses, tilts, rolls and tentacle beats at the same moments. Both windows show the baby small. **Screenshot 84**, both windows.
146. **Ink for both.** The host hits a squid once with the sword. In the friend's window, the black ink cloud bursts out in the same place at the same moment, and the friend hears the squirt. In both windows, the squid darts away from the host.
147. **The friend makes the glow squid go dark.** The host runs `/time set midnight`. The friend hits the glow squid once with the sword, or shoots it once with the M4A1 from about 8 blocks. Both windows show the teal ink. The glow squid goes dark in both windows at the same moment and brightens again 5 s later. It flees from the friend, not from the host. Both windows show its sparkles the whole time. **Screenshot 85**, both windows, while it is dark.
148. **The friend's kill.** The friend shoots a squid with the M4A1 until it dies. Both windows show it flash red and vanish in a puff of smoke, and the ink sacs drop in both windows. The host runs `/time set day`.

### Game events at a sculk sensor (Part G1's kit)

A fresh world for this section:
- Both players quit to the main menu and close their windows. The script in the first PowerShell window ends when the host window closes.
- Put Part G1's kit line in the game folder's `.env` (`C:\Games\Mashup\2010-Rust-Rewrite-Mashup\.env`).
- From the first PowerShell window, start two windows on this build:

  ```
  & scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r7sculk -Fresh -HostSound
  ```

  Both players get G1's kit on their first join.
- The host types the new world's three commands. Then the host stands still and runs Part G1's setup commands. The block the host stands on is the start.

Both players stay on the carpet, and the host keeps the dust line in view. Wait 2 s between actions.

149. **The friend's knot.**
     - The host breaks the carpet 3 blocks to the left of the start and places an oak fence there. Wait 2 s.
     - The friend uses a cow egg on the carpet beside the fence and puts a lead on the cow. Standing up (not crouched), the friend right-clicks the fence. Both windows show the knot, and the host's line shows 10. Both windows: **Screenshot 86.**
     - After 2 s the friend punches the knot from the side. It vanishes in both windows, the lead drops and the host's line shows 15.
150. **The friend's snowball.**
     - The host places the bell and the shrieker as in step 46, then waits until the shrieker is quiet.
     - The friend crouches 10 blocks to the right of the sensor, in line with the bell, and throws a snowball at the side of the bell facing them.
     - Both windows show the bell swing and the shrieker's rising rings, and both players hear the ring and the shriek. The host's line shows 11. The shrieker shrieks because the ring is the friend's. Both windows: **Screenshot 87.**
151. **The friend's saddle.** The host runs the clearing commands of step 47. The friend uses a pig egg on the carpet 2 blocks to the right of the start and waits 2 s. Standing, the friend right-clicks the pig with a saddle. Both windows show the saddle, and the host's line shows 5.
152. **The friend's gold.**
     - The host runs `/give @a minecraft:golden_helmet`, `/give @a minecraft:gold_ingot 4` and `/give @s minecraft:piglin_spawn_egg 2`. Both players put on the golden helmet.
     - The host runs `/execute in minecraft:the_nether run tp @a 0 128 0`, then stands still and runs Part G2's setup commands.
     - The host uses a piglin egg on the carpet 2 blocks to the left of the start. If it hatches a baby, the host runs `/difficulty peaceful`, `/difficulty normal` and uses the other egg.
     - A piglin throws its barter toward the nearest player it can see, so the host steps 6 blocks back from the piglin, still on the carpet and with the dust line in view, and the friend stands 2 blocks from it.
     - After 2 s the friend right-clicks the piglin with a gold ingot. Both windows show the piglin holding the ingot, and the host's line shows 4.
     - About 6 s later the piglin throws the friend an item, and the line shows 4 again. Both windows: **Screenshot 88.**

### Lightning (Part L's kit)

A fresh world for this section. Both players quit to the main menu and close their windows; the script in the first PowerShell window ends when the host window closes. Put Part L's kit line in the game folder's `.env`, then run in the first PowerShell window:

```
& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r7lightning -Fresh -HostSound
```

Both players get Part L's kit on their first join. First, the host types `/difficulty normal`, `/gamerule keep_inventory true` and `/op Friend`. Play by day on flat open plains. The host lays the two gold blocks as in Part L's setup and runs every `/summon` standing on the first one, so `~ ~ ~8` is the spot.

153. **The friend is struck.** The friend stands on the spot's gold block, and the host runs `/summon minecraft:lightning_bolt ~ ~ ~8`. Both windows show the bolt, and both players hear the thunder. The friend loses two and a half hearts, or five when a later flash of the bolt hits again, and catches fire from the fire under their feet. The friend steps off, and the host puts the fire out with the water bucket. Both windows: **Screenshot 89.**
154. **One bolt for both.** The host runs `/time set midnight` and `/gamerule fire_spread_radius_around_player 0`. The friend stands beside the host, and the host summons the bolt at the spot. Both windows show the same bolt in the same shape. Both skies flash pale blue as the land lights up, and both players hear the thunder and the crack. Then the friend runs `/summon minecraft:lightning_bolt ~ ~ ~8`: the host sees and hears that bolt too. Both windows: **Screenshot 90.**
155. **Too far to see.** The host runs `/gamemode creative Friend`, then `/tp Friend ~ ~ ~300`. The host summons the bolt at the spot. The friend, 300 blocks away, sees no flash and hears no thunder. The host runs `/tp Friend @s` and `/gamemode survival Friend`.
156. **Turned for both.** The friend stands beside the host. The host runs `/summon minecraft:pig ~ ~ ~8` and summons the bolt at the spot. Then the host runs `/summon minecraft:mooshroom ~8 ~ ~8` and `/summon minecraft:lightning_bolt ~8 ~ ~8`. The friend sees the zombified piglin with its golden sword and the brown mooshroom. The friend's window: **Screenshot 91.**
157. **The swirl.** The host runs `/gamemode creative @a`, `/summon minecraft:creeper ~-8 ~ ~8` and `/summon minecraft:lightning_bolt ~-8 ~ ~8`. The friend walks up to the creeper and sees the same blue swirl wrap it and slide over it, still bright at night. The friend's window: **Screenshot 92.** The host runs `/difficulty peaceful`, then `/difficulty normal`.
158. **Sparks for both.** The host places a weathered copper block on the spot's gold block. The host puts weathered copper on the ground on three sides of it, the redstone lamp on the fourth side and the weathered lightning rod on top. The friend stands beside the host, and the host runs `/summon minecraft:lightning_bolt ~ ~2 ~8`. Both windows show the lamp light up and the rod turn orange, with the sparks along the rod and on the cleaned copper. Both windows: **Screenshot 93.** The host runs `/gamemode survival @a` and `/gamerule fire_spread_radius_around_player 128`.

### Phantoms (Part H's kit)

Use `scripts\mp-local.ps1`, with the host and the friend both on this build. Start a fresh world for this section:
- Both players quit to the main menu and close their windows.
- Put Part H's kit line in the game folder's `.env` (`C:\Games\Mashup\2010-Rust-Rewrite-Mashup\.env`) as `IW4L_MINECRAFT_KIT=...`.
- From PowerShell in `C:\Dev\Soscraft\mashup-crafting`, run:

```
& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r7phantoms -Fresh -HostSound
```

Both players get Part H's kit on their first join. The host types `/difficulty normal`, `/gamerule keep_inventory true` and `/op Friend`, then `/gamerule advance_time false`, `/weather clear` and `/time set midnight`. Play on flat open grass in plains, with your feet at y 63 or higher. Both put on the diamond armour.

159. **The same phantom.** The host runs `/gamemode creative @a` and uses a phantom egg on the ground 5 blocks from the friend.
     - Both windows show the same dark blue-grey phantom with glowing green eyes. It circles the same point, about 5 blocks above where it hatched, and tips up and down the same way as it climbs and sinks.
     - Each window hears its soft flap about every two and a half seconds and sees grey specks trail from its wingtips. The wings may be at a different point of their beat in each window.
     - The host looks straight ahead and runs `/summon minecraft:phantom ^ ^3 ^6 {NoAI:1b,size:4}`. In the friend's window it hangs there beating its wings, 1.6 times as large as the first.

     Both windows: **Screenshot 94.** The host runs `/difficulty peaceful`, then `/difficulty normal`.
160. **The highest player.** The host runs `/gamemode survival @a`. The friend takes the armour off, builds a cobblestone pillar 5 blocks high and stands on top. The host stays on the ground 3 blocks from the pillar and uses a phantom egg on the ground 5 blocks away.
     - Within a few seconds, both windows hear the swoop screech and show the phantom dive head first at the friend.
     - Its bite takes 1 heart from the friend.
     - It keeps diving at the friend and never bites the host while the friend is up there.

     Both windows, mid-dive: **Screenshot 95.** Then the friend comes down and stands beside the host: from now on it may go for either player. The host runs `/difficulty peaceful`, then `/difficulty normal`.
161. **Only the friend has not slept.**
     - The friend quits to the main menu and closes the window, then the host does the same. The script ends when the host window closes.
     - In Explorer, open `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\mp-local\host\iw4l-artifacts\saves\r7phantoms`. Open `ops.json` there in Notepad: the `uuid` next to `"name": "Friend"` is the friend's.
     - In the `players\stats` folder, open the friend's `<uuid>.json` in Notepad. Change the number after `"minecraft:time_since_rest":` to `1000000` and save.
     - In the other file there (the host's), set that number to `0` and save.
     - Start both windows again without `-Fresh`: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r7phantoms -HostSound`. It is still midnight.
     - The host runs `/difficulty hard`, so phantoms come sooner. Both put the armour on and stand in the open, 40 blocks apart.

     Usually within 3 minutes, and at most about 6, 1 to 4 phantoms appear high above the friend, not above the host, and dive at the friend. Both windows show them. Both windows: **Screenshot 96.** The host runs `/difficulty normal` and `/gamerule advance_time true`.

## Known limits

These are known gaps, not new bugs.

- **Pandas:**
  - Pandas from spawn eggs never pick up bamboo or cake from the ground, though vanilla egg pandas do; only a panda summoned with `{CanPickUpLoot:1b}` fetches thrown food.
  - When a panda picks up an item, the item just vanishes from the ground; vanilla shows it flying into the panda's paws.
  - A dispenser cannot put bamboo into a panda's paws.
  - A sneeze's green puff does not drift down toward a player standing within 2 blocks of the cub, as vanilla's does.
  - A panda that sits down by itself may slide on by up to a tenth of a block as it sits.
- **Game events at sculk sensors:**
  - Saddling a pig or horse, putting horse armour or a carpet on, and giving a piglin gold show 5 or 4 here, but vanilla shows 6 because it also hears the click on the mob.
  - Other items used on a mob, such as feeding, milking or naming it, are not heard yet.
  - A witch or a wandering trader finishing a drink shows 4 here, but vanilla shows 8.
  - Rockets from a dispenser can't be tested yet, because no dispenser can be loaded in this build.
  - Endermen moving blocks, sheep eating grass, farming villagers, snow golem trails, ravagers breaking leaves, withers breaking blocks and trampled farmland are not heard yet.
  - Zombies and skeletons picking up loot, raiders picking up banners, dispensers putting armour on mobs and shears taking a saddle off are not in this build, so none of them is heard.
  - Arrows, thrown items and explosions still don't break lead knots.
  - A bell rung by a ghast's fireball that a player knocked back counts as nobody's ring, so it sets off no shrieker.
- **Mob buckets:**
  - No fish, axolotl, tadpole or sulfur cube shows yet: a bucket lets its mob out unseen, and the mob stays in the world and is saved until a later build makes it live (only squids and glow squids are live now).
  - Because those mobs are not live, none of them can be scooped back into a bucket yet, and no mob sound plays when one comes out of its bucket.
  - A dispenser emptying a fish bucket can't be tested yet, because nothing can be put into a dispenser in this build: there is no dispenser screen.
  - A dispenser does not empty a sulfur cube bucket yet: the bucket stays inside, while 26.3 lets the cube out.
  - A dispenser that empties a fish bucket where water boils away, as in the Nether, makes no hiss, just as a plain water bucket from a dispenser makes none.
  - In creative a plain water bucket still turns into an empty bucket when you pour it, where vanilla keeps it full; only the mob buckets stay full in creative.
- **Squids and glow squids:**
  - A trident with Impaling does no extra damage to squids or glow squids yet, because the fork gives no mob the Impaling bonus.
  - The golden dandelion shows no particles when it locks or unlocks a baby squid, the same as with rabbits and polar bears.
  - In a friend's window the ink cloud starts in the same place as in the host's, but its specks spread a little differently, because each window draws its own.
- **Lightning, charged creepers and lightning rods:**
  - Thunderstorms don't strike lightning by themselves yet: `/weather thunder` brings the storm but no bolts, so in this build lightning comes only from `/summon minecraft:lightning_bolt`.
  - Tridents with Channeling don't call down lightning yet.
  - Lightning rods don't give off their small sparks during a thunderstorm; they spark only when a bolt hits them.
  - A bolt doesn't blow up an End crystal or hurt the ender dragon, as a vanilla bolt does.
  - Arrows, tridents and other thrown things in a bolt's reach don't catch fire.
  - A bundle or shulker box that a bolt burns vanishes with everything in it, instead of spilling what it holds.
  - If you are riding a saddled pig when a bolt turns it, you are dropped off instead of riding on as the zombified piglin.
  - `Silent:1b` on a summoned bolt doesn't quiet it.
  - There is no setting yet to hide the lightning flash or to change how far bolts are drawn: bolts always flash and are drawn up to 64 blocks away.
  - A bolt seen through water looks a little brighter than in vanilla, and far off in fog a bolt or a charged creeper's swirl fades out a little differently.
- **Phantoms:**
  - A phantom killed in flight drops to the ground while it tips over, where vanilla's keeps drifting in the air.
  - In vanilla, crouching, invisibility or a worn mob head lets a phantom spot you only from closer; here they make no difference.
  - A phantom summoned with `Silent:1b` still makes its bite sound.
  - Sculk sensors and wardens do not hear a phantom's wingbeats yet.
  - A phantom's flap sound stays where it flapped and does not move with the phantom.
  - Wing specks show however far away a phantom is, and no setting thins them; vanilla shows none beyond 32 blocks.
  - `/gamerule spawn_monsters false` does not stop phantoms from coming for a sleepless player, because that rule never reaches the host's world; `/gamerule spawn_mobs false` does stop them.
- **Not in this build:** the other M1 animals (foxes, goats, parrots, turtles, armadillos, sniffers and bees: their eggs still hatch nothing), the M2 water mobs other than squids and glow squids (fish, dolphins, axolotls, frogs, tadpoles, guardians, nautiluses and sulfur cubes), and the M3 mobs other than phantoms (zombie and skeleton horses, camels, allays, copper golems, creakings and happy ghasts).
- Release 6's and the earlier releases' other known limits still apply, except the lines this build fixes:
  - **Release 6:** from its "Not in this build" line, pandas, the game events W's first merge left out (W's segment w3), and squids, glow squids and phantoms among the M2 and M3 mobs.
  - **Release 4, "Vibrations not heard yet":** the whole "Other features" line (the ominous item spawner, lead knots tied, punched or cut, silverfish breaking infested blocks, and villagers and piglins at doors); "mob armour changes" (mobs putting equipment on or taking it off are heard now, with the gaps listed above); and the firework rockets in "Projectiles" (a rocket's launch and where it strikes are heard now; witch potions, fireballs, shulker bullets and llama spit still are not).
  - **Release 4, other lines:** "Lead knots still don't" in its list of exceptions; pandas in its "Other land animals" line and in its "Not in this build" list; and squids, glow squids and phantoms among "The M2 and M3 mobs are not in either".
  - **Release 3:** "Cutting and breaking knots send no game events (track W)", and pandas, squids, glow squids and phantoms among "The M1 to M3 mobs, phantoms among them" in its "Not in this build" list.
  - **Release 2:** pandas, squids, glow squids and phantoms among "the M1 to M3 mobs (phantoms among them)" in its "Not in this build" line.
  - **Release 1:** "Phantoms don't exist yet" in its "Cats" line.

## Screenshot numbering

- Name each screenshot by its number: `14.png`. When a step asks for both windows, name them `78-host.png` and `78-friend.png`.
- When a step fails, take a screenshot even if the step doesn't ask for one. Name it by its part letter and step number, e.g. `L-92.png` or `F-150-friend.png`. Say in one line what you expected and what you saw.
- Send `latest.log` after any crash. In two windows, send both windows' logs. Single player: `iw4l-artifacts\logs\latest.log`. With the script: `iw4l-artifacts\mp-local\host\iw4l-artifacts\logs\latest.log` and `...\mp-local\friend\...`.
