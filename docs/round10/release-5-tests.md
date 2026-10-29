# Round 10 release 5: breezes, banners, crossbows, bells, dyed horse armour

Release 5 is `minecraft-survival` at e7c9a1c8, copied in as `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-survival.exe`. Release 4 is kept beside it as `iw4l-survival-c41ac6a.exe`. `iw4l-master.exe` (the relay) was copied in again; it has not changed.

New since release 4:
- Track D, segment d2 (merge fd93ce96): breezes.
- Track E1, segment e1b (merge 23b449ef): dyed leather horse armour, crossbow and bow states, banner and shield patterns everywhere, bells swinging, the llama spit puff, love hearts on fed farm animals. This segment's review and clippy run did not happen before the week stopped; every group was tested by its own agent and the full suite passed on the merge.

**Multiplayer:** this build is protocol 10. A friend needs this exe (and the relay from the game folder); release 4 (protocol 9) is refused with "Incompatible client! Please use the host's build (Minecraft multiplayer protocol 10)".

Release 4's steps (`docs/round10/release-4-tests.md`) and release 3's still stand; this file only covers what is new. Screenshots are numbered 1 to 34 in the order below. Each part has its own kit: put its line in the `.env` next to the exe as `IW4L_MINECRAFT_KIT=...` and start a new world. The full background for every step is in the track logs: `docs/round10/track-d-log.md` (groups breezes, breeze-render, breeze-review) and `docs/round10/track-e1-log.md` (E-horses, E-banners, E-bells, E-particles).

Setup for every part: survival, Normal, by day on flat open ground, unless a step says otherwise. Read your position with `/tp @s ~ ~ ~` (the fork has no F3 screen).

## Part B. Breezes

Kit (20 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:breeze_spawn_egg*16,minecraft:iron_golem_spawn_egg*2,minecraft:skeleton_spawn_egg*2,minecraft:blaze_spawn_egg*2,minecraft:trial_spawner,minecraft:spawner,minecraft:trident*2,minecraft:trident+loyalty=3,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:diamond_pickaxe+efficiency=5,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:oak_door*2,minecraft:torch*32
```

Put on the diamond armour first. `/gamerule mob_griefing true`.

1. **The breeze.** Use a breeze egg on the ground 10 blocks away. A pale breeze stands on a whirling, see-through column of wind; its head bobs, three rods turn around it and its eyes glow. It kicks up a little dust of the block at its feet. **Screenshot 1.**
2. **It notices you.** Use another egg 20 blocks away and stand still in its sight: it comes for you, leaping or sliding closer, then shoots. You hear a sharp inhale, then a whoosh, and a wind charge flies at you, knocking you back with a gust (half a heart at most through the armour). **Screenshot 2** with a charge in the air.
3. **Jumps and slides.** Watch it fight: before a leap it gathers wind (a rising sound), leaps with a whoosh and lands beside or behind you with a thud, unhurt. A slide is a swish with a cloud of ground dust; a leap leaves a short dust trail. **Screenshot 3** mid-leap.
4. **Out of sight.** Build a 3-high cobblestone wall without gaps and step behind it while it shoots: after about 5 seconds it stops and idles. Step back into its sight: it comes again. Walk 30 blocks away: it stops shooting.
5. **Low roof.** Under a 3-high cobblestone roof, with the breeze under the same roof: it only slides and shoots, it never leaps.
6. **Doors.** Place an oak door on its own (no wall). Stand on a cobblestone block 4 blocks behind it so your head shows over it, with the breeze about 6 blocks in front of the door. A charge bursting on the door opens it. **Screenshot 4.** `/gamerule mob_griefing false`, close the door: the bursts leave it shut. `/gamerule mob_griefing true`.
7. **Breezes and golems.** Use a second breeze egg next to the first: their charges never hurt each other. Use an iron golem egg 6 blocks from a breeze: the breeze shoots at the golem too. `/gamemode creative`: the breezes stop shooting at you. `/gamemode survival`.
8. **Arrows and fireballs turned back.** At night (`/time set night`) use a skeleton egg 15 blocks away with a breeze between you and the skeleton: arrows that hit the breeze fly back with a whoosh and do not hurt it. Do the same with a blaze egg: a fireball that hits the breeze drifts back toward the blaze at half its speed. **Screenshot 5.** `/time set day`.
9. **Your own throws.** Throw a trident at a breeze: it bounces back toward you with the deflect whoosh and the breeze is unhurt. The Loyalty trident comes back only after it has landed. **Screenshot 6.** (Your own wind charge hits it for half a heart, as in vanilla: only a breeze's own charges leave it unhurt.)
10. **Dark room.** In a closed cobblestone room without torches: its body and wind go dark, its eyes still glow. **Screenshot 7.**
11. **Hurt and death.** Hit it with the sword: the head and rods flash red, the wind and eyes do not. Kill it: it tips over sideways and drops 1-2 breeze rods and experience.
12. **Saving a charge.** With a charge in the air, save and quit, load: the charge flies on and bursts.
13. **Cages.** Place the trial spawner and use a breeze egg on it: a small breeze turns inside the cage. **Screenshot 8.** The mob spawner with a breeze egg shows one too. The trial spawner then runs vanilla's default trial when you come near: 6 breezes, 2 at once.
14. **Trial chamber.** `/locate structure minecraft:trial_chambers`, go there and find the breeze room: its spawner shows its breeze while waiting, wakes, sends one breeze at a time (two for one player), and gives its reward when both are gone. **Screenshot 9.**

## Part H. Dyed leather horse armour

Kit (8 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:leather_horse_armor@red,minecraft:leather_horse_armor@blue,minecraft:leather_horse_armor@lime,minecraft:leather_horse_armor,minecraft:iron_horse_armor,minecraft:saddle,minecraft:iron_sword,minecraft:cooked_beef*16
```

15. **Red on a horse.** Run `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`. Move the red leather horse armour to the hotbar, hold it and right-click the horse: you hear the armour sound, and the horse wears red armour with the plain brown buckles, straps and trim on top. Walk round it: nothing flickers. **Screenshot 10.**
16. **Plain leather and iron.** Summon two more tame horses beside it. Put the plain leather armour on one (brown with the same trim) and the iron armour on the other (iron, as before). **Screenshot 11**, all three.
17. **The screen.** Crouch and right-click the red horse: the armour slot shows the red icon. Take it out: it is red on the cursor and in your inventory. Shift-click the lime armour in: the horse wears lime with the trim. **Screenshot 12.**
18. **Save and load.** Save and quit, load: the horse still wears lime and its screen shows the lime icon.
19. **The drop.** Kill the lime horse with the iron sword: the armour lies on the ground lime and is lime when picked up. **Screenshot 13.**
20. **Riding.** Saddle the plain-leather horse and ride it: armour, trim, saddle and reins move with it.

## Part C. Crossbows and bows

Kit (9 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:crossbow[charged_projectiles=[{id:"minecraft:arrow"}]],minecraft:crossbow[charged_projectiles=[{id:"minecraft:firework_rocket"}]],minecraft:crossbow,minecraft:item_frame*8,minecraft:oak_planks*32,minecraft:pillager_spawn_egg*16,minecraft:shield,minecraft:iron_sword,minecraft:cooked_beef*16
```

21. **Icons and hand.** Open the inventory: the first crossbow has an arrow lying in it, the second a firework rocket, the third is plain; all are named "Crossbow". Move them to the hotbar and hold each in first person: the arrow, the rocket and the plain string show. Press Q with each: on the ground they look the same. **Screenshot 14.**
22. **Third person.** Hold the arrow crossbow and press F5: the arrow lies in the crossbow in your hand; the firework one shows the rocket, the plain one nothing. **Screenshot 15.**
23. **Item frames.** Build a plank wall, hang frames and put in the arrow and firework crossbows: each shows its arrow or rocket, and keeps it when turned.
24. **A pillager loads.** Use a pillager egg about 12 blocks away and stand still with the shield raised (shield in the offhand, iron sword selected). When it loads, the string pulls back in three steps (about a second); when loaded an arrow lies in its crossbow while it aims, and after the shot it is empty again. **Screenshot 16** while it pulls, **screenshot 17** with the arrow. About one pillager in twenty is left-handed and does the same in its left hand.
25. **A piglin.** Run `/summon minecraft:piglin ~ ~ ~10 {IsImmuneToZombification:1b,equipment:{mainhand:{id:"minecraft:crossbow",count:1}}}` (summon another if it holds no crossbow). Without gold armour it attacks: it steps back, the string pulls back in three steps and the arrow shows until it shoots. **Screenshot 18.**

## Part N. Banners and shields

Kit (17 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:white_banner[banner_patterns=[{pattern:"minecraft:creeper",color:"black"},{pattern:"minecraft:border",color:"red"}]]*8,minecraft:blue_banner[banner_patterns=[{pattern:"minecraft:gradient",color:"white"},{pattern:"minecraft:globe",color:"lime"},{pattern:"minecraft:stripe_bottom",color:"yellow"}]]*4,minecraft:black_banner*2,minecraft:shield[base_color=red,banner_patterns=[{pattern:"minecraft:cross",color:"yellow"}]],minecraft:shield[base_color=black],minecraft:shield,minecraft:pillager_spawn_egg*16,minecraft:oak_planks*32,minecraft:diamond_axe,minecraft:diamond_sword,minecraft:tnt*2,minecraft:flint_and_steel,minecraft:shulker_box,minecraft:stone*4,minecraft:item_frame*4,minecraft:torch*8,minecraft:cooked_beef*16
```

26. **Icons, hand and ground.** Open the inventory: the white banner shows a black creeper face in a red border; the blue banner a white fade from the top, a lime globe and a yellow bottom stripe; the black banner is plain. The first shield is red with a yellow cross, the second plain black, the third plain wood. Hold each banner in first person: the pattern is on the flag (the blue banner's fade has a hard edge near the bottom: patterns are drawn cut-out). Put the red shield in the offhand (F) and raise it: the cross shows. Press Q with each: the same on the ground. **Screenshot 19.**
27. **Placed banners.** Build a plank wall three wide and two high. Place the white banner on the ground two blocks in front of it and the blue one on the wall: both show their patterns from front and back; a black banner beside them is plain. Place three more white banners facing different ways: each pattern turns with its flag. At night (`/time set night`) with a torch beside them, the patterns are lit like the black banner. **Screenshot 20.** Save and quit, load: the patterns show at once.
28. **Drops.** Break the white, blue and black banners with the axe: each drops with its pattern (the blue one as a blue banner, not a wall banner). **Screenshot 21**, the drops on the ground. Hang a blue banner on the wall and break the plank behind it: the banner falls and drops with its pattern. Blow a patterned banner up with TNT: when it drops, it has its pattern. In creative, a broken banner drops nothing.
29. **Shulker box.** Place the empty shulker box and break it: it drops. Put a stone in it and break it again: it drops with the stone inside.
30. **The captain.** Use pillager eggs until one carries a banner. Its banner shows the ominous pattern: a cyan diamond with grey and black stripes on white, in a black border. Kill it: the dropped "Ominous Banner" shows the pattern on the ground, as an icon and in your hand; placed, and in an item frame, it shows the same. **Screenshot 22.**
31. **Third person and frames.** In F5 hold the white banner, then the blue one, with the red shield in the offhand: the patterns and the cross show on your body. Put the white banner and the red shield in item frames: the frames show them. **Screenshot 23.**

## Part L. Bells

Kit (14 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:bell*4,minecraft:lever*2,minecraft:cobblestone*64,minecraft:oak_planks*64,minecraft:villager_spawn_egg*4,minecraft:ominous_bottle*2,minecraft:white_bed*2,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*16
```

32. **Floor bell.** Look straight ahead and place a bell on cobblestone in front of you. Right-click the side facing you, below the rim: it rings, and the bell body swings away from you, dying out over 2.5 seconds; the posts and bar stay still. **Screenshot 24** mid-swing. Ring again mid-swing: the swing starts over. Ring the opposite side: it swings the other way. The two other sides and the top do nothing.
33. **Wall, ceiling and lever.** Place a bell on a plank wall and one under a plank ceiling: each swings away from the side you hit. Put a lever next to one and pull it: it rings and swings. **Screenshot 25.**
34. **Raid.** Place a bell and two beds in the open, hatch four villagers there and wait until they have claimed the beds. Drink an ominous bottle: when the raid starts, a villager rings the bell now and then and you see it swing.

## Part S. Llama spit and love hearts

Kit (17 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:llama_spawn_egg*4,minecraft:iron_sword,minecraft:shield,minecraft:oak_fence*64,minecraft:cow_spawn_egg*2,minecraft:sheep_spawn_egg*2,minecraft:pig_spawn_egg*2,minecraft:chicken_spawn_egg*2,minecraft:mooshroom_spawn_egg*2,minecraft:strider_spawn_egg*2,minecraft:hoglin_spawn_egg*2,minecraft:wheat*64,minecraft:carrot*64,minecraft:wheat_seeds*64,minecraft:warped_fungus*16,minecraft:crimson_fungus*16,minecraft:cooked_beef*16
```

35. **Llama spit.** Use a llama egg and hit the llama once with the iron sword: it spits at you, and a small white puff of 7 specks flies from its mouth toward you and drops within a second while the spit flies on. Each spit puffs once, where it leaves the mouth. **Screenshot 26.**
36. **Love hearts.** Put up a fence pen. Use the cow egg twice inside and feed each cow wheat: 7 red hearts rise around each, then a calf appears. **Screenshot 27.** Feeding the calf shows no hearts. Do the same with sheep (wheat), pigs (carrots), chickens (seeds), mooshrooms (wheat) and striders (warped fungus): hearts on each. For hoglins `/gamemode creative` first, feed each crimson fungus within 15 seconds (before they turn into zoglins), then `/gamemode survival`. **Screenshot 28**, the mooshrooms.

## Part F. Friends (two windows, both on this build)

Use `scripts\mp-local.ps1` or the relay. The friend gets each part's kit with `/kit @a` from the host.

37. **Old build refused.** Start release 4 (`iw4l-survival-c41ac6a.exe`) as the friend: it is refused with the protocol 10 message. **Screenshot 29.**
38. **Breezes.** The friend stands near a breeze: they see the same breeze, wind, eyes, bobbing, slides, shots and leaps at the same moments, and its dust; its charges fly at the friend and push them back, and they hear the inhale, the whoosh and its whirl. **Screenshot 30**, both windows.
39. **Horse armour.** The host puts the blue armour on a horse and the friend the red one on another: each window shows both colours with the trim. The host swaps the blue for the lime in its screen: in the friend's window it turns lime within a second. **Screenshot 31**, both windows.
40. **Crossbows, banners and shields.** The host holds the arrow crossbow, then the white banner, with the red shield in the offhand: the friend sees the arrow, then the pattern, and the cross on the host's body. The friend does the same and the host sees it. The friend sees the host's item frames and placed banners with the same looks, and a pillager's crossbow pulling at the same moments. **Screenshot 32**, both windows.
41. **Bells.** The host rings a bell while the friend stands near: the friend sees the same swing. The friend rings it from the other side and pulls the lever: both windows swing. **Screenshot 33**, both windows.
42. **Spit and hearts.** The friend hits a llama and feeds a cow while the host stands 10 blocks away: both windows show the puff and the hearts. **Screenshot 34**, both windows.

## Known limits

- **Track E1's second segment was not reviewed.** The week stopped before its review and clippy run. Every group's agent tested its own commits and the full suite passed on the merge.
- **Glowing mobs have no white outline yet** (E1's optional group, left as unfinished work on `track-e1`).
- No hearts show on a parent when a baby is born from breeding (only when fed).
- The particles setting does not thin the breeze's dust.
- Llama spit never reaches registered mobs (breezes, silverfish, Margit).
- Comparators and calibrated sculk sensors always face north when placed by hand (from release 4).
- Margit, the warden, the new firework shapes and crafting, and held maps are not in this build: their tracks stopped mid-segment (see `docs/PROGRESS.md`, "Where it stands").
- Release 4's and release 3's other known limits still apply.
