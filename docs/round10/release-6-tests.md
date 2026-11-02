# Round 10 release 6: wardens, Margit, fireworks, maps, silverfish

Release 6 is `minecraft-survival` at 789f6d23, copied in as `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-survival.exe`. Release 5 is kept beside it as `iw4l-survival-e7c9a1c.exe`. `iw4l-master.exe` (the relay) was copied in again.

New since release 5:
- Track W, segment w2 (merge a56b6ef0): the warden. It climbs out when a shrieker summons it and digs back in, sniffs, hears vibrations, roars, hits hard enough to knock a shield down, fires sonic booms through walls and spreads darkness. It is drawn with its glowing spots, its heartbeat and the ground it throws up.
- Track F, segment f3 (merge 02ea99f2, with 07825f4a and 0ca69632): Margit is drawn and animated with his golden mist. His cane, golden weapons and daggers hurt players, MW2 guns and explosives go through his own resistances, and he drops gold with "GREAT ENEMY FELLED". He has a boss bar and music, friends see and fight him, the gold block altar summons him, his fog ring shows, and he keeps a player beside a bed awake. The two follow-ups let silverfish, endermites, rabbits, polar bears, breezes and Margit feed sculk catalysts like the other mobs, also with no player kill or with mob drops off.
- Track E1, the review of segment e1b with bells-outline (merge 0b24cb27): glowing mobs show a white outline, also through walls, in place of a filled white shape. The review restored one reformatted test line and fixed 13 clippy warnings.
- Track E2, segment e2b (merge 44d96765): firework stars burst in their own shapes, with fades, trails and twinkles. Stars, rockets with stars and banner shields are crafted. An elytra rocket goes off at the end of its boost. Thrown things and mobs' arrows, fireballs, spit and potions ring bells. Maps are held in first person, drawn in vanilla's colours, fill in as you go, are made from empty maps and reach friends. Snowballs, eggs and tridents break End crystals, and tridents and wind charges hurt the ender dragon.
- Track D0, segment d0b (merge c0366f3a, right after e2b): silverfish hide in stone, burst out of infested blocks and wake their friends, and ender pearls leave endermites. It also brings vanilla's flying navigation, which no mob uses yet.

**Multiplayer:** this build is protocol 11. A friend needs this exe (and the relay from the game folder); release 5 (protocol 10) is refused with "Incompatible client! Please use the host's build (Minecraft multiplayer protocol 11)".

Release 5's steps (`docs/round10/release-5-tests.md`) and the earlier releases' still stand; this file only covers what is new. Screenshots are numbered 1 to 94 in the order below. Each part has its own kit: put its line in the `.env` next to the exe as `IW4L_MINECRAFT_KIT=...` and start a new world. The full background for every step is in the track logs: `docs/round10/track-w-log.md` (groups warden-live, warden-fight, warden-render and the w2 review), `docs/round10/track-f-log.md` (er-look, er-fight, er-guns, er-bar-friend, er-altar and the f3 review), `docs/round10/track-e1-log.md` ("Glowing mobs show a white outline" and the review fixes), `docs/round10/track-e2-log.md` (fw-shapes, fw-craft, fw-elytra, bell-hits, maps-held, maps-view, maps-colours, maps-explore, ti4-end-targets and the e2b review fixes) and `docs/round10/track-d0-log.md` (silverfish-behaviour, flying-navigation and the d0b review).

Setup for every part: survival, Normal, by day on flat open ground, unless a step says otherwise. Read your position with `/tp @s ~ ~ ~` (the fork has no F3 screen). The class loadout (the M4A1 on slot 1, frags on G) comes with the first join; parts W, M and S use it.

## Part W. The warden

Kit (19 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:warden_spawn_egg*4,minecraft:spawner,minecraft:cow_spawn_egg*4,minecraft:snowball*16,minecraft:trident+loyalty=3,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying,minecraft:golden_apple*16,minecraft:ender_pearl*16,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cobblestone*64,minecraft:torch*4,minecraft:cooked_beef*32
```

Survival, Normal, by day, on flat open ground with no animals within 20 blocks. Run `/gamerule keep_inventory true` and `/gamerule fall_damage false`, so your ender pearls don't hurt you. Put on the diamond armour, put the shield in the offhand and keep the M4A1 on slot 1. To raise the shield, select the sword and hold right-click. Build a cobblestone wall 5 wide and 3 high, and do steps 1 to 3 about 15 blocks from it; you hide behind it in step 9. To clear every warden at any time, run `/difficulty peaceful`, then `/difficulty normal`.

1. **In a spawner.** Run `/gamemode creative`. Place the spawner and use a warden egg on it: a small warden turns inside the cage, with its glowing spots. **Screenshot 1.** Then break the spawner so it makes no wardens later.
2. **The warden.** Run `/time set night`. Still in creative, use a warden egg on the ground in front of you: a warden stands there at once. A warden from an egg does not climb out of the ground. Then look straight ahead and run `/summon minecraft:warden ^ ^ ^8 {PersistenceRequired:1b,Brain:{memories:{"minecraft:dig_cooldown":{value:{},ttl:1200L}}}}`. This makes a second warden that never digs in. The `Brain` part gives it the one-minute wait that an egg gives; without it, a warden summoned with a tag digs straight back in, as in vanilla. Step back 8 blocks and look at them. Each one is a dark blue-green giant, over 3 blocks tall, with two horn-like tendrils on its head. It has glowing cyan spots on its head, arms and legs, and two sets of patches on its body and legs that slowly pulse in turn. A heart in its chest glows each time you hear it beat, about every 2 s. Walk around them: in creative they take no notice of you, and you get no darkness. **Screenshot 2.**
3. **Digging in.** Run `/time set day`. Stay in creative and keep 5 to 8 blocks from the warden from the egg. Step away if it walks up to you: each touch makes it wait another minute before it digs in. So does an animal it hears walking by; kill any animal that comes near. About a minute after you hatched it, it sinks into the ground over 5 s with a digging sound and throws up bits of the ground for the first 4.5 s. Then it is gone and drops nothing. While it sinks, walk up and hit it with the sword: there is no red flash, because it cannot be hurt now. **Screenshot 3** while it sinks. The summoned warden stays. Wait two more minutes: it is still there.
4. **Darkness.** Move 12 blocks from the warden, crouch and run `/gamemode survival`. Within 6 s your view goes dark: the fog closes in and the light pulses. It stays dark as long as you are within 20 blocks of the warden. Crouch-walk 25 blocks away: the darkness ends within 13 s. Crouch-walk back to about 18 blocks from it, where it cannot hear you drink: within 6 s it is dark again. Drink a milk bucket: the darkness clears at once, then comes back at the warden's next pulse, within 6 s. **Screenshot 4.**
5. **Sniffing.** Stay crouched and still, 10 blocks from the warden. It wanders, and every 5 to 10 s it stops, raises its head and sniffs for about 4 s with a long sniffing sound. Stay more than 6 blocks away: if you are closer when a sniff ends, it growls, grows angrier at you and walks to where you are.
6. **A snowball.** Stay crouched, so it does not hear the throw. Throw one snowball so that it lands about 3 blocks beside the warden. Its tendrils click, twitch and glow for half a second. Then it walks to where the snowball landed and sniffs there. **Screenshot 5.**
7. **It hears you.** First crouch-walk around it, 8 blocks away: no clicks. Then stand up and walk around at the same distance. Each time it hears you (at most once every 2 s), its tendrils click, it growls as it listens, and it walks to where you were. After two or three clicks, it rears up and roars at you for about 4 s. Its heartbeat speeds up from one beat every 2 s to one every half second. **Screenshot 6** during the roar.
8. **Its blow and your shield.** Now it comes for you. Raise the shield as it swings, a fast two-armed slam with a heavy thud. The blow lands on the shield and you are pushed back, and the shield goes down for 5 s. It swings at most once a second. Without the shield, one blow takes about 6 of your 10 hearts in this armour. Eat golden apples as you need them. **Screenshot 7.**
9. **Sonic boom.** Throw ender pearls to get behind your cobblestone wall, about 10 blocks from the warden. At least 10 s after its roar, and at least 2 s after its last swing, it bends forward and its chest opens, with a rising charge sound for about 1.7 s. Then a line of grey shock-wave rings runs from its chest to your eyes, straight through the wall, with a boom. You lose 5 hearts through your armour and the raised shield, and you are thrown back. It does not swing while it charges. As long as you stay within 15 blocks of it, it booms about every 5 s. **Screenshot 8**, with the rings.
10. **Totem and death.** Hold the totem and press F: the totem goes to your offhand and the shield to your hand. Do not eat from now on. Stay 6 to 15 blocks from the warden, pearl away whenever it comes close, and let its booms hit you. A boom that would kill you pops the totem instead, with its sound and burst. Keep away from its blows: a later boom kills you, and the chat says "<your name> was obliterated by a sonically-charged shriek". **Screenshot 9.** Respawn and run `/difficulty peaceful`: the warden vanishes. Then run `/difficulty normal`.
11. **Hits from near and far.** Put the shield back in your offhand. Crouch, look straight ahead and run `/summon minecraft:warden ^ ^ ^10`. Throw the trident at it: it roars first, and only then comes for you. A hit from more than 5 blocks away makes it angry but does not start the fight at once. The Loyalty trident flies back to you. Run `/difficulty peaceful`, then `/difficulty normal`. Next, run `/summon minecraft:warden ^ ^ ^3` and hit it with the sword at once: it does not roar, but swings back straight away. Pearl away, then run `/difficulty peaceful` and `/difficulty normal`.
12. **Shot from far, and saved.** Crouch, run `/summon minecraft:warden ^ ^ ^20` and shoot it once with the M4A1: it roars, then comes for you. As soon as it starts to roar, Save and quit, then load: it is still there. Within a few seconds it roars at you again, because its anger was saved, and comes for you. Run `/difficulty peaceful`, then `/difficulty normal`.
13. **It fights mobs.** Look straight ahead and run `/summon minecraft:warden ^ ^ ^10`. Use two cow eggs on the ground in front of you, then crouch-walk 15 blocks back and stand still. When it hears the cows' steps, its tendrils click and it walks to them. A cow that bumps into it makes it growl and go to where the cow is. Once it is angry at a cow, it roars at it and kills it with one blow. **Screenshot 10.** Run `/difficulty peaceful`, then `/difficulty normal`.
14. **Climbing out.** Run `/time set midnight`, and shoot any night monster that comes near with the M4A1. Stand on open flat ground with no torches or lava within 16 blocks and run `/setblock ~ ~-1 ~3 minecraft:sculk_shrieker[can_summon=true]`. Warn yourself four times: step onto the shrieker and off again, waiting at least 10 s each time. The first three times, you get a shriek with rising rings, then a warden sound that is closer each time, and darkness. The fourth time, the ground cracks within 5 blocks of the shrieker after the shriek, and a warden climbs out over about 7 s, growling. It throws up bits of the ground for the first 4.5 s, and no reply sound plays. **Screenshot 11.**
15. **Bumping into it.** As soon as the ground cracks, walk into that spot. Keep walking into the warden while it climbs out: about once a second you hear its listening growl. Hit it with the sword: there is no red flash. When it is standing, it roars at you straight away.
16. **No warning near a warden.** While it roars, step onto the shrieker; by now more than 10 s have passed since the fourth warning. There is no shriek, no rising rings and no warden sound, because a warden is within 24 blocks. Your view is still dark from the fourth warning and from the warden itself. Pearl away, then run `/difficulty peaceful` and `/difficulty normal`.
17. **Too bright for a warden.** Place a torch on the ground right next to the shrieker. Wait 10 s and step onto the shrieker. You get the shriek, then a loud burst of warden growls around it, then the angriest warden sound and darkness, but no warden comes. Break the torch and run `/time set day`. Wait 10 s and step onto it again: the same happens, and again no warden comes. Run `/gamerule fall_damage true`.

## Part M. Margit, the Fell Omen

Kit (24 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:barrett,iw4l:sniper_rounds*20,iw4l:rpg,iw4l:rocket*8,minecraft:netherite_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:gold_block*8,minecraft:lantern*4,minecraft:cobblestone*64,minecraft:iron_pickaxe,minecraft:white_bed,minecraft:obsidian*10,minecraft:flint_and_steel
```

Setup: start a fresh world on flat open ground (plains are best). Play near where you first appeared, so you respawn close to him, with open flat ground for 70 blocks to the north. Run `/gamerule keep_inventory true` and turn the music volume up in the options. The class's M4A1 and frags come with the first join; the kit's own M4A1 is for Part F. Put on the diamond armour and put the shield in the offhand (F). Keep both on unless a step says otherwise, and eat a golden apple whenever you are low. Every `/summon` below puts him that many blocks south of you, and the spot where he appears is the middle of his arena. `/difficulty peaceful` then `/difficulty normal` clears him between steps.

18. **Asleep, then awake.** Stand still and run `/summon iw4l:margit ~ ~ ~30`. The chat says "Summoned new Margit, the Fell Omen". 30 blocks away, a hunched old man kneels on one knee with his head bowed, leaning on a cane. There is no sound and no bar. Run `/tp @s ~ ~ ~4` (26 blocks from him): he sleeps on. **Screenshot 12.** Run `/tp @s ~ ~ ~4` again (22 blocks): he rises from his knee over about a second and a half. Green-gold sparkles and gold motes swirl round him with a deep mirror sound. A full red bar "Margit, the Fell Omen" shows at the top of the screen, and the Ender Dragon fight music starts. While the mist swirls: **Screenshot 13.**
19. **His look and walk.** He is about 3 blocks tall in a dark hooded cloak with a ragged hem. He has a pale face with two gold eyes, a grey beard, two small pale horns through the hood and fur on the hood's rim and his shoulders. The cane is in his right hand, and his left arm is bare and pale. Walk round him at 6 to 10 blocks. He limps after you: his right leg swings further than his left, and his cane arm swings. His head follows you, and his steps are low, heavy iron golem steps. Every few seconds he mutters like a deep evoker, and between moves his body slowly rises and falls. From the front, about 6 blocks away: **Screenshot 14.**
20. **The cane combo.** Take the armour off and lower the shield. Stand 2 to 3 blocks in front of him and do not attack. He swings the cane from his right to his left, then back. Each swing has a low sweep sound and takes about 2 hearts. Then he raises the cane overhead and holds it a moment, his two gold eyes grow and glow, and it comes down for 3 hearts and pushes you back. Sometimes he stops after one or two swings. Run straight back as he winds up a swing: it misses. With the cane up and the eyes bright: **Screenshot 15.** Put the armour back on.
21. **The cane slam and the shield.** Raise the shield facing him: his cane swings are blocked with the shield sound and take nothing. Wait for the cane slam. He raises the cane overhead and holds it, his eyes flare, and it comes down 1 to 2 blocks in front of him. You hear the mace's smash and a low boom, and a ring of white puffs flies outward round a small grey explosion cloud. The shield blocks the slam too, but you are pushed back. At the slam: **Screenshot 16.**
22. **The golden sword.** Keep the shield up. A glowing gold sword grows in his left hand with a chime, and white sparks fly from the hand. He draws it back to his left and sweeps it across to his right. It goes through the shield and the armour for about 3.5 hearts and throws you back a block or so. Mid-sweep: **Screenshot 17.**
23. **Daggers.** Back off to 15 to 20 blocks and stand still. A small gold dagger grows in his left hand with the chime and sparks. He raises it behind his head, holds it for half a second to a second and a half, and throws it with the trident's throw sound. A small glowing gold blade flies at your chest and hits through the raised shield and the armour for 2 hearts, with the trident's hit sound. Next time, strafe sideways as he throws: it flies past. With a dagger in the air: **Screenshot 18.**
24. **Close up and far off.** Stand right next to him: he hops back about 4 blocks, a dagger forms, and he throws two (sometimes three). Then stand 10 to 14 blocks away in the open. He crouches, a ravager roars, and he leaps with his arms up. He lands crouched 1 to 2 blocks in front of you with the smash and the boom, and puffs and a cloud show where he lands. The landing takes about a heart through the armour and tosses you up and back. Mid-leap: **Screenshot 19.**
25. **Kneel and critical.** Hit him with the netherite sword at full charge. Each hit flashes him red and slides the bar down, and his deep hurt voice comes at most every half second. After five quick hits, a ravager's stunned sound plays and sparks burst from his chest. He drops to one knee, leaning on his cane, for 4 seconds, then rises. Break his stance again with five more hits, then hit him once from the front while he kneels: you hear the critical sound, another burst of sparks flies, and he gets up at once. At the second burst: **Screenshot 20.**
26. **Phase 2.** `/difficulty peaceful`, `/difficulty normal`, then `/summon iw4l:margit ~ ~ ~6 {Health:401f}` (just over half his health). Walk up and hit him once. He stops, at the latest when his current swing ends, and raises his left hand. A gold hammer grows in it with a chime and white sparks. Then green-gold sparkles and gold motes flash round him with the elder guardian's curse sound. From then on, gold motes keep dripping from him. **Screenshot 21.**
27. **The hammer.** Stand 2 to 3 blocks in front of him with the shield raised. Wait for the hammer slam. He holds the hammer high behind his head, his eyes flare, and he slams it down with a heavy smash, a boom and a ring of puffs 2 blocks in front of him. The shield blocks the hammer, but it drops with the shield's break sound and cannot be raised for 5 seconds, and you are pushed back. A shockwave runs out along the ground and takes 2 hearts through shield and armour if it catches you. Next time, jump as it passes under you: it does nothing. At 10 to 14 blocks, he does the slam from a leap with a roar first. Close up, two cane swings sometimes turn into the hammer slam. With the hammer raised: **Screenshot 22.**
28. **Killed by him.** Take the armour off, do not raise the shield, and let him kill you. A cane, leap or hammer death says "Sos was slain by Margit, the Fell Omen". A dagger, golden sword or shockwave death says "Sos was killed by Margit, the Fell Omen using magic". The bar goes on the death screen. **Screenshot 23.** Respawn at once: near him again, the bar is back while he is awake. Die again until you have seen both lines, then put the armour back on.
29. **GREAT ENEMY FELLED.** `/difficulty peaceful`, `/difficulty normal`, `/xp set @s 0 levels`, `/xp set @s 0 points`. Run `/summon iw4l:margit ~ ~ ~6 {Health:20f}`, walk up as he rises and hit him twice with the sword. He sinks to his knees in golden mist with a deep evoker death sound, slumps forward and vanishes in more mist. "GREAT ENEMY FELLED" shows in white above the hotbar with the challenge-complete fanfare, and the bar and the music stop. Where he fell lie an enchanted golden apple, 2 gold blocks, 3 gold ingots and experience orbs. With the line and the drops: **Screenshot 24.** Pick it all up: `/xp query @s levels` says 7.
30. **No mob drops.** Run `/gamerule mob_drops false`, then summon and kill another the same way. The line and the fanfare come, but nothing drops, not even orbs. Run `/gamerule mob_drops true`.
31. **M4A1 and Barrett.** Run `/summon iw4l:margit ~ ~ ~10 {Health:20f}` and shoot his chest with single M4A1 shots: he dies on the 7th body hit. Each hit shows the hit marker and his red flash, and no sword swoosh plays with the shots (release 5 played one). Do it again with head shots at the top of his hood: 5 hits. Again with the Barrett: two or three body hits.
32. **Frag and RPG.** Run `/summon iw4l:margit ~ ~ ~10 {Health:40f}` and cook a frag so it bursts at his feet. He lives, and the blast neither lifts nor pushes him. About four M4A1 body hits finish him. Do it once more with an RPG rocket at his feet instead: it takes at least as much as the frag, and the second rocket kills him at the latest.
33. **His stance against bullets.** Run `/summon iw4l:margit ~ ~ ~10` (full health). Hold the M4A1 on his body from about 10 blocks, reloading quickly. After about 135 body hits the bar is at half: he raises his left hand and the hammer forms. Keep shooting. After about 160 hits in all (five or six magazines), he drops to one knee in sparks with the stunned sound. If you stop shooting for 6 seconds, his stance refills and the count starts again. While he kneels, walk up and shoot him from the front: no critical sound plays, and he stays down the full 4 seconds. As he kneels: **Screenshot 25.**
34. **Bar range, fog ring and loading.** `/difficulty peaceful`, `/difficulty normal`. Stand still and run `/summon iw4l:margit ~ ~ ~10 {Health:600f}`: as he rises, the bar shows three quarters full. Turn round: about 14 blocks behind you, an arc of small gold sparkles lies on the ground. It curves round his spot and reaches about 20 blocks to each side of you. **Screenshot 26.** Without walking, run `/tp @s ~ ~ ~-60`: the bar is gone and the music stops at once. Within 5 seconds, run `/tp @s ~ ~ ~60`: the bar is back, still three quarters full, and the music starts again. Save and Quit, and load: he stands where he was and comes for you at once with a full bar, because a loaded Margit starts his fight over.
35. **The fight resets.** Without walking, run `/tp @s ~ ~ ~-38` (about 48 blocks from his spot). He stops where he is, because you are outside his arena. Wait 6 seconds: the bar and the music go, and he turns, walks back to his spot and kneels there. Walk back south toward his spot. No gold sparkles show on the ground until you are within 24 blocks of it. There he wakes again with a full bar and the music, and the arc shows round you.
36. **A wall, a pillar and a hole.** `/difficulty peaceful`, `/difficulty normal`. Stand still and run `/fill ~-2 ~ ~2 ~2 ~2 ~2 minecraft:cobblestone` (a wall 5 wide and 3 high, 2 blocks south of you), then `/summon iw4l:margit ~ ~ ~17`. Stay right behind the wall. A dagger thrown at you clinks into the wall with the trident's hit sound and does nothing. When he comes up to the wall, his swings do not reach you through it, and he walks round it to get at you. Then pillar up 5 blocks on a single column of cobblestone. He cannot reach you, and after about 5 seconds he only throws daggers, which hit you up there. He never appears beside you on the pillar. Come down, dig a 1 x 1 hole 2 blocks deep and stand in it: his cane still hits you.
37. **He fades in.** Run `/difficulty peaceful`. On flat ground, stand still and run `/fill ~-4 ~-1 ~-4 ~4 ~5 ~4 minecraft:cobblestone hollow`, then `/fill ~-3 ~5 ~-3 ~3 ~5 ~3 minecraft:air`. This builds a cobblestone ring 6 high, open at the top, 4 blocks out from you on every side. Run `/difficulty normal` and `/summon iw4l:margit ~ ~ ~10`: he rises outside the ring and cannot get in. After about 5 seconds, he vanishes in golden mist with the mirror sound. He appears beside you inside the ring in another burst of mist, and you never see him slide across. **Screenshot 27.** Run `/difficulty peaceful`, break out with the pickaxe, then run `/difficulty normal`.
38. **Not an altar.** Run `/difficulty peaceful`. Stack two gold blocks and put a lantern on top: it stays, and nothing happens. **Screenshot 28.** Break the lantern and run `/difficulty normal`. Build a column of 4 cobblestone right beside the gold blocks. Put one cobblestone against the side of its top block, so it sits over the gold blocks with one empty block above the upper gold block. Click its underside with a lantern: the lantern hangs over the gold blocks, and nothing happens. Break the lantern and the cobblestone. Break the upper gold block with the pickaxe and put a lantern on the one left: nothing happens. Break the lantern and put the gold block back.
39. **The altar.** Stand 2 to 3 blocks from the gold blocks and put a lantern on top. The lantern and both gold blocks break with their particles and sounds, and nothing drops. Margit kneels where they stood, facing you, and at once rises in golden mist, with his bar and the music. **Screenshot 29.**
40. **The ring near his spot.** This close to his spot, no gold sparkles show anywhere, because his arena's edge is 24 blocks out. Walk straight away from his spot, keeping clear of his swings. After about 10 blocks, an arc of sparkles shows on the ground about 14 blocks ahead of you, curving round his spot. Walk along it: the part you see moves with you, and the far side of the ring never shows. Run `/difficulty peaceful`: he and his ring go. Run `/difficulty normal`. Optional, if a hill is near: summon him at its foot and walk up. The arc lies on the hill's ground, not in the air or inside the hill.
41. **No sleep beside him.** Run `/time set night`. Place the bed on open ground and stand beside it. Run `/summon iw4l:margit ~ ~ ~4 {NoAI:1b}`: he kneels asleep 4 blocks away and never wakes. Right-click the bed: "You may not rest now; there are monsters nearby" shows above the hotbar, and you stay standing. With the line and him in view: **Screenshot 30.** Run `/difficulty peaceful`: he vanishes. Right-click the bed: you lie down, and the night skips to morning. Run `/difficulty normal`.
42. **The Nether.** Build a Nether portal frame from the obsidian (4 wide and 5 high, corners left out), light it and go through. On open ground there, run `/summon iw4l:margit ~ ~ ~6`: the same red bar and the same music come. **Screenshot 31.** Run `/difficulty peaceful`, then `/difficulty normal`, and go back through the portal. Optional: in the Overworld, run `/summon minecraft:wither ~ ~ ~10`. Its purple bar comes with no music. Run `/difficulty peaceful` to remove it, then `/difficulty normal`.

## Part O. Glowing outlines and sculk catalysts

Kit (20 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:bell*2,minecraft:pillager_spawn_egg*8,minecraft:witch_spawn_egg*2,minecraft:oak_planks*64,minecraft:cobblestone*64,minecraft:sculk_catalyst*4,minecraft:lava_bucket,minecraft:silverfish_spawn_egg*8,minecraft:endermite_spawn_egg*4,minecraft:rabbit_spawn_egg*4,minecraft:breeze_spawn_egg*4,minecraft:iron_golem_spawn_egg*2,minecraft:netherite_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

Put on the diamond armour first. Play in survival on Normal, by day, on flat open grass.

43. **Set up.** Place a cobblestone block in front of you. Look straight ahead and place a bell on it. About 20 blocks in front of the bell, build a plank wall 9 wide and 4 high. Type `/gamemode creative` so nothing attacks you. Then use three eggs:
    - a pillager egg in the open, about 15 blocks from the bell, where you can see it;
    - another pillager egg right behind the wall, where the wall hides it;
    - a witch egg in the open, about 10 blocks from the bell.
44. **Ring.** Go back to the bell and right-click the side facing you, below the rim. It rings and swings, and a moment later you hear it resonate. About 2 seconds after the ring, both pillagers and the witch glow for 3 seconds:
    - In the open, each one has a thin white rim around its body, with its own colours inside. It is not covered in white.
    - The hidden pillager shows as a white outline through the wall, with the planks visible inside it. It is not a filled white shape (release 5 showed a solid white shape there).
    - **Screenshot 32**, with all three in view.
45. **Two together.** Use two more pillager eggs side by side, so that one stands partly in front of the other as seen from the bell. Ring again right away: the two share one outline, with no white line between them. **Screenshot 33.**
46. **Newer mobs behind the wall.** Go behind the wall and stand at one end, two blocks back from it. Run each of these commands, taking two steps along the wall before the next one:
    - `/summon minecraft:silverfish ~ ~ ~ {NoAI:1b,active_effects:[{id:"minecraft:glowing",duration:6000}]}`
    - `/summon minecraft:rabbit ~ ~ ~ {NoAI:1b,active_effects:[{id:"minecraft:glowing",duration:6000}]}`
    - `/summon minecraft:polar_bear ~ ~ ~ {NoAI:1b,active_effects:[{id:"minecraft:glowing",duration:6000}]}`
    - `/summon minecraft:breeze ~ ~ ~ {NoAI:1b,active_effects:[{id:"minecraft:glowing",duration:6000}]}`

    Each one stands still and glows for 5 minutes. Go back to the bell: all four show white outlines through the wall, with the planks inside them. **Screenshot 34.** Walk 40 blocks back from the wall: they are still outlines, only thinner.
47. **In the open and partly hidden.** Walk round to the far side of the wall. Each of the four has a thin white rim with its own colours inside, and the breeze's rim follows its head, rods and wind. **Screenshot 35.** Place a cobblestone block on the ground right in front of the polar bear and step 5 blocks back. The block hides the bear's legs, its outline runs on over the block, and you can see the block inside the outline.
48. **Catalysts.** Walk 60 blocks away from the wall onto plain grass. Type `/gamemode survival`, then `/gamerule mob_drops false`. Place three sculk catalysts in a row, 20 blocks apart. Call them A, B and C.
49. **Your kills without mob drops (A).** Stand next to A. Use a silverfish egg 3 blocks from it and kill the silverfish with the sword when it comes at you:
    - A flashes, two soul wisps rise over it and you hear its bloom sound;
    - no experience orbs drop;
    - a little sculk spreads over the grass where the silverfish died, with soft sparks and a crackle.

    **Screenshot 36** of the new sculk. Before this build, these mobs gave a catalyst nothing to spread, so only the bloom came. Do the same with an endermite.
50. **Rabbits, a polar bear and a breeze (A).** Stay next to A. Run each command below and kill the mob that appears. Wait until the sparks stop before the next one.
    - `/summon minecraft:rabbit ~ ~ ~3 {NoAI:1b}`: A blooms and a block or two of sculk spreads. No raw rabbit and no hide drop.
    - `/summon minecraft:rabbit ~ ~ ~3 {NoAI:1b,Age:-24000}`, a baby: A blooms but no new sculk spreads. A baby animal is worth no experience, as in vanilla.
    - `/summon minecraft:polar_bear ~ ~ ~3 {NoAI:1b}`, three hits: A blooms and a little sculk spreads. No fish drop.
    - `/summon minecraft:breeze ~ ~ ~3 {NoAI:1b}`, three hits: A blooms, no breeze rods drop, and a bigger patch spreads (a breeze is worth 10).
51. **Margit (B).** Stand next to B and run `/summon iw4l:margit ~ ~ ~3 {Health:20f}`. He rises. Hit him twice with the sword:
    - He sinks to his knees, and "GREAT ENEMY FELLED" shows in white above the hotbar with the fanfare.
    - Nothing drops: no gold, no apple, no orbs.
    - B blooms. Over the next seconds his 100 points spread a wide patch of sculk around where he fell, with sparks and crackles. Now and then a sculk sensor or a shrieker grows 4 or more blocks from B.

    **Screenshot 37**, a few seconds after he falls. Before this build, only the bloom came.
52. **Killed by nobody (C).** Type `/gamerule mob_drops true`. Stand next to C, dig out one grass block 3 blocks from it, and pour the lava bucket into the hole. Use a silverfish egg on the bottom of the hole (aim at the lava). The silverfish burns and dies there within a second or two:
    - no orbs drop, because no player killed it;
    - C blooms;
    - sculk spreads on the grass around the hole, with sparks and a crackle.

    **Screenshot 38.** Before this build, only the bloom came. Do the same with a rabbit egg.
53. **Killed by a golem (C).** Type `/gamemode creative`.
    - Use an iron golem egg 2 blocks from C, on the side away from the lava. Then use a silverfish egg 3 blocks from the golem. The golem walks up and swats it. C blooms and sculk spreads where it died, with no orbs.
    - Then use a breeze egg 5 blocks from the golem. The breeze shoots wind charges at the golem, and the golem fights back. If the golem kills it within 8 blocks of C, C blooms and a bigger patch spreads. If they fight farther away, nothing blooms.

    Type `/gamemode survival` after.

## Part X1. Fireworks, shields and elytra

Part X needs more than 27 stacks, so it is split in two. X1 covers firework shapes, crafting, shields and the elytra rocket. X2 covers bells.

Kit (26 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:small_ball,colors:[I;4312372]}]}]*64,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*64,minecraft:firework_rocket[fireworks={flight_duration:2,explosions:[{shape:star,colors:[I;14602026],fade_colors:[I;2437522]},{shape:creeper,colors:[I;4312372],has_twinkle:1b}]}]*64,minecraft:firework_rocket[fireworks={flight_duration:0,explosions:[{shape:burst,colors:[I;15790320,11743532]},{shape:small_ball,colors:[I;2437522]},{shape:large_ball,colors:[I;14602026]}]}]*64,minecraft:firework_rocket*16,minecraft:crafting_table,minecraft:gunpowder*64,minecraft:paper*32,minecraft:red_dye*16,minecraft:lime_dye*16,minecraft:blue_dye*16,minecraft:fire_charge*8,minecraft:diamond*8,minecraft:glowstone_dust*16,minecraft:creeper_head,minecraft:black_banner*4,minecraft:white_banner[banner_patterns=[{pattern:"minecraft:creeper",color:"black"},{pattern:"minecraft:border",color:"red"}]]*4,minecraft:shield*2,minecraft:shield+unbreaking=3+damage=200,minecraft:shield[base_color=red,banner_patterns=[{pattern:"minecraft:cross",color:"yellow"}]],minecraft:stick*4,minecraft:elytra,minecraft:husk_spawn_egg*4,minecraft:iron_sword,minecraft:cooked_beef*32
```

Setup: survival, Normal, on open flat ground. Do steps 54 to 66 at night (`/time set night`). Move the first four rocket stacks to the hotbar: the lime one, the red one, the two-star one and the three-star one. If a burst is 16 or more blocks from you, you hear the distant version of its bang and crackle. The two-star rocket always bursts that high.

54. **A small ball.** Right-click the grass 3 blocks ahead with the lime rocket. It climbs with the launch whoosh and bursts into a small round ball of lime sparks, with a lime flash and a bang. The sparks drift down and go out after about 3 seconds. **Screenshot 39.**
55. **A large ball with trails.** Place the red rocket the same way. Its ball of red sparks is about twice as wide, and the bang is a deeper boom. As each spark flies out and falls, it leaves a short red streak of still sparks hanging where it passed. The streaks fade soon after. **Screenshot 40.**
56. **A star and a creeper face.** Place the two-star rocket. It climbs higher than the first two. Then it shows a yellow five-pointed star (three thin stars turned around the up axis), and a tenth of a second later a lime creeper face. The face points a random way, so if you see it edge-on, place another rocket. **Screenshot 41** just after the burst. Place one more and watch it to the end:
    - About a second after the bang, the creeper face starts to flicker on and off, with a crackling sound.
    - From about 1.2 s, the yellow star turns blue as its sparks fall and dim.

    **Screenshot 42** while the star turns blue.
57. **Three stars.** Place the three-star rocket. It stays low and bursts three times, a tenth of a second apart:
    - a white and red fountain,
    - a small blue ball,
    - a large yellow ball.

    Each has a flash in its first colour, and you hear the deep boom. All three keep their colours to the end, with no streaks and no flicker. **Screenshot 43.**
58. **Stars at the crafting table.** Place the crafting table.
    - Put one gunpowder, one red dye, one fire charge and one diamond in any four cells: the result is one firework star with a plain grey icon. Take it.
    - Put in one gunpowder, one lime dye, the creeper head and one glowstone dust: a second star. **Screenshot 44** with it in the result slot. Take it.
59. **Fade.** Put the lime star and one blue dye in: one star. Take it.
60. **Rockets with stars.** Put paper and 2 gunpowder in the bottom row, the red star in the top left cell and the lime star with the blue fade in the middle: 3 rockets. Take them and place one on the grass 3 blocks ahead. It climbs as high as the two-star rocket and bursts into a big red ball with trails and a deep boom. A tenth of a second later comes a lime creeper face that turns blue and flickers, with a crackle. **Screenshot 45.**
61. **Gunpowder.**
    - Put paper and 4 gunpowder in: no result.
    - Take one gunpowder out: 3 rockets. Leave them there.
    - Clear the grid and put in paper and one gunpowder. Take the 3 rockets onto the stack of 16 plain rockets: they join it and it shows 19.
62. **Recipe book.** Open the recipe book and search "firework". Only the plain rocket recipe (paper and gunpowder) shows, as in vanilla. Stars and rockets with stars are only made by hand.
63. **A black shield.** Put the black banner and a plain shield in any two cells: the result is a shield. Take it: the banner is used up and the shield is black. **Screenshot 46.**
64. **A patterned shield keeps its enchantment.** Put the white patterned banner and the worn, enchanted shield in. The result keeps the glint and the durability bar, and shows the black creeper face in a red border on white. Take it, put it in the offhand (F), select the iron sword and hold right-click: the shield rises and the pattern shows. **Screenshot 47.**
65. **What does not combine.** None of these gives a result:
    - the red shield with the yellow cross and a black banner;
    - a plain shield and two black banners;
    - a plain shield, a black banner and a stick.
66. **The small grid.** Open the inventory and put a black banner and the other plain shield in its 2x2 grid: the result is a black shield, as at the table.
67. **The elytra rocket.** Run `/time set day` and `/gamerule pvp true`. Put the elytra in your chest slot and eat until you are full. Run `/tp @s ~ ~60 ~` and press jump while you fall, so you glide. Use the red rocket:
    - You boost forward with the launch whoosh, and a trail of white sparks follows you. No rocket shows at your hand (press F5 to see).
    - After 1 to 1.6 s, the rocket goes off at your right side: a deep boom, a big red ball with trails around you, and you lose 3.5 hearts.

    **Screenshot 48** (the red ball around you; F5 helps). Wait until your hearts are full again. Don't eat a golden apple before a damage check, because its golden hearts take the hit first.
68. **A plain rocket.** Glide again and use a plain rocket. You get the same boost, then a few grey puffs at your side, no bang and no damage.
69. **Three stars.** Glide and use the three-star rocket. You get a shorter boost (half a second to a second), then three bursts one after another at your side, and you lose 5.5 hearts. Wait until your hearts are full.
70. **Without pvp.** Run `/gamerule pvp false`. Glide and use the red rocket: you get the boost and the red ball, but no damage. Then run `/gamerule pvp true`.
71. **Near husks.** On the ground, run `/gamemode creative` so the husks leave you alone, and hatch 3 husks close together. Husks don't burn by day. Walk about 30 blocks away from them. Run `/gamerule pvp false`, so your rocket can't hurt you (step 70), then `/gamemode survival` and `/tp @s ~ ~10 ~`. Glide toward them and use the red rocket at once. If it goes off within 5 blocks of a husk, that husk flashes red and is knocked away from the burst. This may take a few tries: for another try, run `/gamemode creative`, walk 30 blocks away from them again and repeat. Run `/gamerule pvp true` after, and kill the husks with the sword.

## Part X2. Bells rung by throws, rockets and mobs

Kit (26 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:bell*4,minecraft:snowball*32,minecraft:egg*16,minecraft:splash_potion@water,minecraft:trident+loyalty=3,minecraft:wind_charge*32,minecraft:dispenser*2,minecraft:lever*2,minecraft:firework_rocket*16,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*16,minecraft:villager_spawn_egg*2,minecraft:white_bed,minecraft:skeleton_spawn_egg*2,minecraft:blaze_spawn_egg*2,minecraft:llama_spawn_egg*2,minecraft:witch_spawn_egg*2,minecraft:iron_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*16,minecraft:water_bucket
```

Setup: survival, Normal, by day on flat open ground. Put on the diamond armour and put the shield in the offhand (F). Every ring plays the bell sound. The bell body swings away from the side that was hit and dies out over 2.5 s; the posts and bar stay still.

72. **Throws.** Look straight ahead and place a bell on the ground 2 blocks in front of you. Only the side facing you and the opposite side can ring. Step back until you are 5 blocks from it.
    - Throw a snowball at the side facing you, below the rim: it rings and swings away from you. **Screenshot 49** mid-swing.
    - Lob a snowball so it lands on the bell's top: no ring.
    - Throw an egg, the splash water potion and the trident at the side facing you: each one rings it. The trident flies back to you.
    - Walk round to one of the other two sides and throw a snowball at it: no ring.
    - Go on to the far side and throw one: it rings and swings away from you there.

    Then go back to where you placed the bell from.
73. **Villagers hide.** Place the bed 10 blocks from the bell and use a villager egg next to the bell. Throw a snowball at the side of the bell facing you. The villager runs to the bed and stays by it for a while, as when you ring the bell by hand.
74. **Wind charges.** Throw a wind charge at the side facing you. The hit and the burst ring the bell at the same moment, and the bell swings toward you: the burst's ring comes last and rings it from the way it faces. Throw one at the ground right beside the bell: one ring, and it swings toward you again. **Screenshot 50** mid-swing.
75. **Rockets from a dispenser.** Do this 8 blocks from the first bell.
    - Look straight ahead at the ground 3 blocks away and place a dispenser. Its front, with the hole, faces you. If it faces up, break it and place it again from further back.
    - Step back one block and place the second bell on the ground 2 blocks in front of the hole, with one empty block between them.
    - Walk round to the back of the dispenser, crouch and place a lever on its back.
    - Open the dispenser and put in one plain rocket.
    - Stand behind the dispenser, so its block keeps the bursts off you, and pull the lever. The rocket flies into the bell and keeps pressing on it. For about a second the bell rings again and again but hardly moves. Then the rocket puffs and the bell swings out from the last ring.
    - Flip the lever back, put in one red rocket and pull the lever again. The rocket bursts on the bell into the red ball with one ring.

    **Screenshot 51.**
76. **Skeleton.** Run `/time set night`. On open ground, look straight ahead and place a bell 2 blocks in front of you, then step back one block, so you are 3 blocks behind it. Walk straight on past the bell for 12 blocks, hatch a skeleton there and run back to your place behind the bell. Select the iron sword, crouch and hold right-click to raise the shield.
    - An arrow that hits the bell's front low down rings it and sticks in it.
    - An arrow stuck in the bell does not ring it again.

    **Screenshot 52** with an arrow in the bell. Kill the skeleton with the sword.
77. **Blaze.** Do the same with a blaze egg. A fireball that hits the bell's front rings it and sets fire beside it. Put the fires out with the water bucket and pick the water up again. Kill the blaze with the sword or snowballs (snowballs hurt blazes).
78. **Llama.** Hatch a llama 6 blocks past the bell, hit it once with the sword and run back behind the bell. When its spit hits the bell's front, the bell rings.
79. **Witch.** Hatch a witch 8 blocks past the bell and stand behind the bell. A potion that breaks on the bell's front rings it; one that breaks on its top does not. Eat a golden apple if her potions hurt you. Run `/time set day` after.

## Part P. Maps and End crystals

Kit (22 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:potion@long_night_vision,minecraft:iron_shovel,minecraft:spyglass,minecraft:torch*64,minecraft:map*8,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:diamond_sword,minecraft:shield,minecraft:trident+loyalty=3,minecraft:snowball*16,minecraft:egg*16,minecraft:wind_charge*16,minecraft:end_crystal*4,minecraft:obsidian*8,minecraft:splash_potion@harming,minecraft:potion@slow_falling,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

A new world, by day. Run `/gamemode creative` for steps 80 to 88, so you can fly over the sea and never drown. Step 89 goes back to survival. As in vanilla, the arrows and dots on a map move a few times a second, not every frame.

80. **A treasure map.** Run `/locate structure minecraft:shipwreck`, then `/tp @s <x> 80 <z>` with the x and z it gives. Drink the night vision potion and dive to the wreck. The map chest is usually in the cabin at the back, and it holds a **Buried Treasure Map**. Take it and move it to the hotbar. Half wrecks have no map chest; if yours has none, fly 300 blocks on and locate the next one.
81. **Two hands.** Fly up out of the water. Keep the offhand empty, select the map and look straight ahead: the map lies low at the bottom of the screen, leaning away from you, and no map item shows in your hand. Look down: the map rises to the middle of the screen and faces you. It has a parchment border and a faint beige checkerboard where nothing is mapped yet. The sea near the X is hatched in orange lines, with brown edges along the land. You see the red X and your white arrow pointing where you look. If the wreck lies outside the map, you get a small white dot on the map's edge instead of the arrow. **Screenshot 53.**
82. **One hand.** Select the torches and press F to put them in the offhand, then select the map again. The map moves to the right, held smaller in one hand, and the torch shows on the left. **Screenshot 54.** Press F with the map selected: now the map is on the left and the torch on the right. Press F again to swap them back.
83. **Swing.** Left-click the air: the map swings with its hand and settles back.
84. **Hidden.** Press F5: the map leaves the screen in third person. Press F5 until you are back in first person, and the map is there again. Put the spyglass in the offhand, select the map and hold right-click: the map is gone while you look through the spyglass. Let go and it comes back.
85. **Filling in.** Empty the offhand, select the map and fly toward the red X. As you get close, the land and sea within about 128 blocks of you fill in, in a circle around you, within about a second:
    - sand, grass, trees and stone in their own colours;
    - water in blue, darker where it is deeper;
    - slopes shaded lighter or darker.

    The real colours replace the orange lines wherever they are drawn. Fly on: the new ground ahead fills in as you go, and your arrow moves and turns with you. **Screenshot 55.**
86. **Offhand.** Press F to move the map to the offhand, then select the torches (any slot that holds no map). Fly toward a corner of the map that is still blank: the corner fills in as you get near.
87. **The edge.** Select the map with an empty offhand and fly straight away from the X, out of the map. When your arrow reaches the map's edge, it turns into a small white dot. The dot stays on the edge and slides along it as you fly. **Screenshot 56.**
88. **Treasure.** Fly back to the X, land on it and dig straight down with the shovel. Within a few blocks, a chest under the sand or gravel holds a Heart of the Sea. **Screenshot 57.**
89. **A new map.** Stand on the ground and run `/gamemode survival`. Move the 8 Empty Maps to the hotbar, select them and right-click the air once. One empty map is used up (7 left), you hear the cartography table's pencil scratch and your hand swings. A "Map" appears in the first free hotbar slot. **Screenshot 58.**
90. **It fills in.** Empty the offhand and select the new Map. Your white arrow is on it, and within about a second the ground around you fills in out to the map's edges, in its real colours. A new map covers vanilla's fixed 128 by 128 block square that you stand in, so your arrow is not always in its middle. Walk 30 blocks the way your arrow has the most room on the map: the arrow moves with you and stays on the map. **Screenshot 59.**
91. **The last one.** Open the inventory and put a single Empty Map on its own in an empty hotbar slot (pick up the stack, right-click once on the empty slot, then put the rest back). Select that single map and right-click the air: the new map takes its place in your hand at once.
92. **A map in each hand.** Select one of the two new maps and press F to move it to the offhand. Then select the other one: a small map shows in each hand, each with your arrow. **Screenshot 60.**
93. **Creative.** Run `/gamemode creative`. Select the Empty Maps and right-click the air three times: the count of empty maps stays the same, and a new map appears each time. Run `/gamemode survival`.
94. **Saved.** Save and quit, then load the world. Hold the treasure map, then a new map. Each shows the same picture as before, the treasure map still has its X, and your arrow is back where you stand. In Explorer, open `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\saves\<world>\data\minecraft\maps`. It holds `last_id.dat` and one numbered `.dat` file per map, starting at `0.dat`.
95. **The Nether.** Run `/execute in minecraft:the_nether run tp @s 0 128 0`, which puts you on the bedrock roof. Empty the offhand, right-click an Empty Map and hold the new map. It fills in with vanilla's brown and grey speckle in a round patch that leaves the map's corners empty, and your arrow spins. **Screenshot 61.** Then hold one of the Overworld maps: nothing fills in, and your arrow stays where you left the Overworld, as in vanilla.
96. **Into the End.** Put on the diamond armour. Run `/execute in minecraft:the_end run tp @s 0 70 0`: you land on the bedrock pillar in the middle of the main island, and the pink Ender Dragon bar shows. Jump down and walk 20 blocks away from the portal onto open end stone. Each crystal you break blows up like a charged creeper, so keep at least 8 blocks away from it. If the dragon throws you into the air, drink the slow falling potion.
97. **Snowball on a crystal.** Place obsidian on the ground and put an end crystal on it. Walk 10 blocks away and throw a snowball at the crystal: the snowball breaks on it with white crumbs, and the crystal blows up. **Screenshot 62.**
98. **Potion and wind charge.** Place a second crystal the same way.
    - From 10 blocks away, throw the harming splash potion at it: the bottle shatters on the crystal, and the crystal stays.
    - Throw a wind charge at it: the charge flies through the crystal and bursts behind it, and the crystal still stands. **Screenshot 63.**
99. **Egg.** Throw an egg at the same crystal: it blows up. Now and then a chick hatches first.
100. **Trident at the flying dragon.** Select the trident and wait for the dragon to fly low past you. Throw the trident at its head. The dragon flashes red and growls, and the boss bar drops by about a 25th (8 of its 200 health). The trident drops off and flies back to you. **Screenshot 64.**
     - A hit on its body, wings or tail drops the bar by less (3).
     - A snowball or an egg breaks on the dragon and does nothing.
     - A wind charge makes it flash red and growl, but the bar barely moves (1).
101. **Perched.** Wait until the dragon lands on the exit portal in the middle and sits there; it looks around, roars or breathes purple flames over the portal. From outside the flames, throw the trident at it: the trident drops off and comes back, the dragon does not flash or growl, and the bar stays the same. A wind charge or a snowball does nothing either. **Screenshot 65.**
102. **A pillar crystal.** Throw a snowball up at the crystal on top of the lowest pillar, one without an iron cage. If it is out of reach, build up with cobblestone first. The crystal blows up.
     - If the dragon was circling, it turns, flies at you and spits a purple fireball.
     - If it was healing from that crystal (a beam from the crystal to the dragon), the beam ends and the bar drops by a 20th.

## Part S. Silverfish and endermites

Kit (23 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:silverfish_spawn_egg*16,minecraft:infested_stone_bricks*64,minecraft:infested_stone*32,minecraft:infested_deepslate*4,minecraft:stone_bricks*64,minecraft:diamond_pickaxe,minecraft:diamond_pickaxe+silk_touch,minecraft:diamond_sword+sharpness=5,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:ender_pearl*16,minecraft:ender_pearl*16,minecraft:ender_pearl*16,minecraft:ender_pearl*16,minecraft:ender_pearl*16,minecraft:ender_pearl*16,minecraft:ender_pearl*16,minecraft:ender_pearl*16,minecraft:zombie_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:rabbit_spawn_egg*2,minecraft:oak_fence*16,minecraft:cooked_beef*32
```

Survival, Normal, by day on flat open grass, with `/gamerule mob_griefing true` (the default). A hurt silverfish wakes infested blocks up to 10 blocks away and 5 up or down. So build each step's blocks at least 15 blocks from the infested blocks of earlier steps. Kill every silverfish with one hit of the Sharpness sword: one that is hurt and survives wakes more blocks near it a second later, as in vanilla.

103. **Silverfish burst out of a wall.** Build a wall 5 wide and 3 high. Use infested stone bricks for the bottom and top rows and infested stone for the middle row. Use a silverfish egg on the ground at the foot of the wall and step 4 blocks back. When it comes at you, hit it once with an empty hand (it survives). About a second later, one or more of the wall's blocks break with the stone break sound and bits of stone. Nothing drops, and a silverfish crawls out of each broken block with a small white puff. Show the gap in the wall and the silverfish: **Screenshot 66.** Kill them all.
104. **Mining.** Mine one of the wall's infested stone blocks with the plain diamond pickaxe: it drops nothing, and a silverfish pops out of it with a puff: **Screenshot 67.** Kill it. Mine one of the wall's infested stone bricks with the Silk Touch pickaxe: no silverfish, and it drops a Stone Bricks block. Place an infested deepslate block on the ground and mine it with the plain pickaxe: a silverfish comes out. `/gamemode creative` and break an infested block: no silverfish. `/gamemode survival`.
105. **A command and block drops.** Place an infested stone block on the ground, jump onto it and run `/setblock ~ ~-1 ~ minecraft:air destroy`. The block breaks under you, and a silverfish comes out where it was with a puff. Kill it. `/gamerule block_drops false`, then place another infested stone block and mine it with the plain pickaxe: no silverfish. `/gamerule block_drops true`.
106. **TNT and a frag.** Build a row of three infested stone blocks with a TNT block at one end. Light the TNT and run 10 blocks away. The blast breaks the row, and one silverfish comes out of each block it broke, each with a puff. Kill them. 15 blocks further on, build a row of three infested stone bricks. Step back 12 blocks and throw a frag (G) so it lands at the row: each block its crater takes lets out one silverfish. Show the silverfish in the crater: **Screenshot 68.** Kill them.
107. **A creeper's blast.** Go back to what is left of the wall from step 103. `/gamemode creative`, stand with your back against its middle and run `/summon minecraft:creeper ~ ~ ~ {ignited:1b}`, then walk 6 blocks away. The creeper swells and blows up at the wall (in creative it can't hurt you), and a silverfish comes out of each infested block it blew up. In creative they leave you alone. `/gamemode survival`: they come for you. Kill them.
108. **Without mob griefing.** `/gamerule mob_griefing false`. Build a row of three infested stone bricks, hatch a silverfish beside it and hit it once with an empty hand. A second later nothing breaks and no silverfish come out. Kill it. Mine the three blocks with the plain pickaxe. At least one of them drops a Stone Bricks block and lets no silverfish out: the hurt silverfish turned it back into plain stone bricks. Any other block drops nothing and lets a silverfish out. Kill those.
109. **Diving into stone.** Leave `mob_griefing` off. Build a pen of plain stone bricks on the grass, 4 by 4 inside and 2 high. Hatch a silverfish inside, step 6 blocks back and run `/gamemode creative`. Watch it for a minute: it crawls about the pen and stays. Run `/gamerule mob_griefing true`. Within about a minute it dives into one of the pen's lowest blocks and is gone, with a small white puff where it stood: **Screenshot 69.** If you miss it, hatch another silverfish in the pen and watch again. `/gamemode survival`, then mine the pen's lowest row with the plain pickaxe. The plain blocks drop Stone Bricks. The block it dived into drops nothing and lets it out again with a puff. Kill it.
110. **An endermite from a pearl.** Run `/gamerule fall_damage false` so the pearls don't hurt you. Stand still and throw ender pearls one at a time at the ground about 10 blocks away. Turn round after each landing so you stay in the same area. On average one pearl in twenty leaves a small purple endermite with purple sparkles on the spot you threw from, with no puff. Keep throwing until one appears; if you run out, run `/give @s minecraft:ender_pearl 64`. It comes after you and bites for a heart. Show the endermite: **Screenshot 70.**
111. **It leaves after two minutes.** `/gamemode creative`: it stops chasing you. Keep it in sight. Two minutes after it appeared it vanishes at once, without falling over and without dropping experience. You can build the fence row of step 113 next to it while you wait. `/gamemode survival`.
112. **No endermites in peaceful or without mob spawning.** `/difficulty peaceful`: any silverfish or endermite still about vanishes. Throw 40 pearls: none leaves an endermite. `/difficulty normal`, then `/gamerule spawn_mobs false`, and throw 40 more: none leaves one. `/gamerule spawn_mobs true`, then `/gamerule fall_damage true`.
113. **Ground paths: round a fence.** No mob uses the new flying navigation yet, so steps 113 and 114 check that walking mobs still find their way. Build a straight row of 7 oak fences on the grass (if you didn't in step 111). `/gamemode creative`, then use a zombie egg on the ground 2 blocks in front of the row's middle. Walk round one end and stand 2 blocks behind the middle, facing the zombie. `/gamemode survival`. The zombie sees you over the fence, walks round one end and comes at you (it burns in the daylight, as always). Take it as it rounds the end: **Screenshot 71.** Kill it.
114. **Cows and rabbits.** Use two cow eggs and two rabbit eggs on open grass and watch for a minute. They wander about as before, and the rabbits hop.

## Part F. Friends (two windows, both on this build)

Use `scripts\mp-local.ps1` or the relay. First put Part W's kit line in the game folder's `.env` (`C:\Games\Mashup\2010-Rust-Rewrite-Mashup\.env`) as `IW4L_MINECRAFT_KIT=...`: in a new world, both players get the kit in that file on their first join, with the class loadout. For two windows on this PC, run from PowerShell in `C:\Dev\Soscraft\mashup-crafting`:

```
& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r6friends -Fresh -HostSound
```

This opens the host window and a friend window. That PowerShell window keeps running the script until both game windows close, so run any other command from a second PowerShell window in `C:\Dev\Soscraft\mashup-crafting`. Leave out `-Fresh` to go on with a world. To start only the friend window again after the friend quit, run `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly` from that second window. `Sos` is the host's player name and `Friend` the friend's; use your names if they differ. In every new world the host types `/difficulty normal`, `/gamerule keep_inventory true` and `/op Friend` first.

Kits within a world: the host's `/kit` reads `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\mp-local\host\.env`, the host window's own copy, which the script writes from the game folder's `.env` only when it starts the host. So where a section below says "give the next kit", do this: the host runs `/clear @a`; you put that section's kit line in the host's `.env` above as `IW4L_MINECRAFT_KIT=...` and save it; the host runs `/kit @a`. `/kit` reads the file again every time. Clear first, because what doesn't fit in a full inventory drops at the player's feet.

115. **Old build refused.** The friend quits to the main menu and closes the friend window; the host stays. In the second PowerShell window, start release 5 as the friend: `& scripts\mp-local.ps1 -Exe iw4l-survival-e7c9a1c.exe -FriendOnly`. It is refused with "Incompatible client! Please use the host's build (Minecraft multiplayer protocol 11)". **Screenshot 72.** Close it and start the friend on this build from the same window: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly`. The friend comes back with the kit from the first join.

### The warden (Part W's kit)

Both players have Part W's kit from their first join. Both put on the diamond armour and put the shield in the offhand; to raise it, select the sword and hold right-click. Survival, Normal, by day, on flat open ground. The host runs `/gamerule fall_damage false`, so the pearls don't hurt. The friend builds a cobblestone wall 5 wide and 3 high, and both stand about 15 blocks from it for step 116; the friend hides behind it in step 118. To clear every warden, run `/difficulty peaceful`, then `/difficulty normal`.

116. **The same warden.** The friend stands 5 blocks beside the host, and both crouch. The host looks straight ahead and runs `/summon minecraft:warden ^ ^ ^10 {PersistenceRequired:1b,Brain:{memories:{"minecraft:dig_cooldown":{value:{},ttl:1200L}}}}`, a warden that never digs in, as in step 2. Both windows show the warden in the same place, with the same glow. It walks, and stops to sniff with its head raised, at the same moments in both windows. Each window hears its heartbeat from the warden, about every 2 s. Both views go dark within 6 s. **Screenshot 73**, both windows. Then the friend crouch-walks 25 blocks away from the warden: the friend's darkness ends within 13 s, and the host's goes on. The friend crouch-walks back to about 10 blocks from the warden.
117. **The friend is heard.** The friend stands up and walks around, 8 blocks from the warden. In both windows its tendrils click and glow at the same moment, and it walks to where the friend was. After two or three clicks it roars at the friend. Both windows show the roar at the same time, and both hear the heartbeat speed up.
118. **The friend in the fight.** The friend raises the shield as the warden swings: both windows show the slam, and the friend's shield goes down for 5 s. The friend pearls 10 blocks away behind cobblestone. Both windows show the warden bend forward, then a line of rings from the warden to the friend. The friend loses 5 hearts and is thrown back. **Screenshot 74**, both windows, with the rings.
119. **Darkness for both.** While both players stay within 20 blocks of the warden, both views stay dark. The friend's darkness and its icon do not end early, because each pulse renews them. Run `/difficulty peaceful`, then `/difficulty normal`.
120. **Digging in.** The host uses a warden egg, then both players crouch-walk 25 blocks away and stand still. After about a minute, both windows show the warden sink into the ground at the same moment, throwing up the same bits of ground. **Screenshot 75**, both windows.
121. **Climbing out.** The host runs `/time set midnight`, then `/setblock ~ ~-1 ~3 minecraft:sculk_shrieker[can_summon=true]`, with the friend within 16 blocks. The host warns four times, stepping onto the shrieker at least 10 s apart. Both windows get the shrieks and the darkness each time. On the fourth, both windows show the warden climb out at the same spot, with the same bits of ground. **Screenshot 76**, both windows.

### Margit (Part M's kit)

The host runs `/gamerule fall_damage true`. Then give the next kit, Part M's: `/clear @a` also takes the class's M4A1, and Part M's kit has its own. Both players put on the diamond armour and put the shield in the offhand, on open flat ground.

122. **He rises for both.** The host and the friend stand 10 blocks apart. The host stands still and runs `/summon iw4l:margit ~ ~ ~10`. In both windows, he kneels, then rises in golden mist with green-gold sparkles and the mirror sound. Both players get the red bar "Margit, the Fell Omen" and the Ender Dragon fight music. Both windows: **Screenshot 77.**
123. **His moves.** The friend stands 2 to 3 blocks in front of him and does not attack. The friend sees the cane combo, with the raised cane and the flaring gold eyes before the third swing. The friend also sees the cane slam's ring of puffs and the golden sword with its sparks, and hears his heavy steps. The host sees the same moves from the side at the same moments.
124. **Daggers.** The friend backs off to 15 to 20 blocks and stands still. The friend sees a dagger form in his left hand, the throw, and the glowing gold blade flying at the friend; it hits for 2 hearts. When the friend strafes, the friend sees the dagger fly past. Then the friend stands right next to him: he hops back and throws two or three daggers. The host, standing to the side, sees the same daggers on the same paths. Both windows, with a dagger in the air: **Screenshot 78.**
125. **The fog ring.** The friend stands about 15 blocks from his spot, on the other side from the host. Each window shows the arc of gold sparkles nearest its own player.
126. **The friend fights.** The friend shoots him with the M4A1 and hits him with the sword: the bar drops in both windows. The friend keeps hurting him for 10 seconds while the host only watches: he turns on the friend and goes for the friend.
127. **The friend dies.** The friend takes the armour off and lets him kill the friend. Both chats read "Friend was slain by Margit, the Fell Omen", or "Friend was killed by Margit, the Fell Omen using magic" for a dagger, the golden sword or the shockwave. The friend respawns and walks back, and the fight goes on.
128. **Two fighters.** The host runs `/difficulty peaceful` and `/difficulty normal`. With both players within 24 blocks, the host runs `/summon iw4l:margit ~ ~ ~10 {Health:20f}`. The friend kills him with M4A1 body hits: it takes about 11 (alone it takes 7). Both windows show "GREAT ENEMY FELLED", and each player hears the fanfare once. He drops the enchanted golden apple, 2 gold blocks, 6 gold ingots and 150 points of orbs. Both windows: **Screenshot 79.**
129. **Only fighters are told.** The friend goes about 60 blocks away and stays there. The host summons another `{Health:20f}` Margit and kills him: only the host sees the line and hears the fanfare. The host then summons a full-health one and fights him. The friend, 60 blocks away, still sees him move and his daggers fly, but has no bar and no music. The host clears him with `/difficulty peaceful` and `/difficulty normal`.
130. **The friend's own kill.** The host goes at least 50 blocks north of the friend, so more than 40 blocks from where Margit will appear (the friend is op since the setup). The friend runs `/summon iw4l:margit ~ ~ ~10 {Health:20f}` and kills him with 7 M4A1 body hits. Only the friend sees "GREAT ENEMY FELLED" and hears the fanfare, and the friend picks up all 100 points of orbs.
131. **The friend's altar.** The host stands 30 blocks away while the friend stacks two gold blocks and puts a lantern on top. Margit rises facing the friend, and both windows show the three blocks breaking and the golden mist. Both windows: **Screenshot 80.**
132. **Reset for both.** Both players run 45 blocks from his spot and wait 6 seconds. The fight resets: both bars and the music go, and both players see him walk home and kneel.

### Glowing outlines and catalysts (Part O's kit)

Give the next kit, Part O's. Both put on the diamond armour. The host runs `/gamemode creative @a`, so nothing attacks you, and repeats step 43 in this world: a cobblestone block with a bell on it, a plank wall 9 wide and 4 high about 20 blocks in front of the bell, a pillager egg in the open about 15 blocks from the bell, another pillager egg right behind the wall where the wall hides it, and a witch egg in the open about 10 blocks from the bell.

133. **Bell outlines.** The friend stands about 10 blocks from the bell while the host rings it. In the host's and the friend's window the pillagers and the witch glow at the same moment: a thin rim in the open, and the hidden pillager's outline through the wall. **Screenshot 81**, both windows.
134. **A soldier in front.** The host rings again, and the friend stands between the host and a glowing pillager. In the host's window, the friend's soldier is drawn normally in front of the pillager, not white. **Screenshot 82**, the host's window.
135. **A friend's kill feeds a catalyst.** The host runs `/difficulty peaceful` and `/difficulty normal` (the pillagers and the witch go), then `/gamemode survival @a` and `/gamerule mob_drops false`. On plain grass, the friend places a sculk catalyst, uses a silverfish egg 3 blocks from it and kills the silverfish with one sword hit, while the host stands 10 blocks away. Both windows show the bloom and the two soul wisps. Nothing drops, and both windows show the new sculk spreading with its sparks. **Screenshot 83**, both windows.
136. **Margit with two fighters.** The host places a second catalyst on grass 20 blocks from the first, and both stand next to it. The host runs `/summon iw4l:margit ~ ~ ~3 {Health:20f}`, and the friend lands the last hit. Both windows show "GREAT ENEMY FELLED", and each player hears the fanfare. Nothing drops, and a wider patch of sculk spreads (150 points for two fighters). Type `/gamerule mob_drops true` after.

### Fireworks, shields, elytra and bells (Part X1's and X2's kits)

Give the next kit, Part X1's.

137. **Fireworks.** At night (`/time set night`), the host places the two-star rocket and then the three-star rocket 3 blocks from the friend. The friend sees the same yellow star and lime creeper face, the star turning blue, the face flickering and the three bursts. The friend hears the bangs and the crackle. The friend places the red rocket: the host sees the red ball with its trails. **Screenshot 84**, both windows.
138. **The friend's elytra rocket.** By day (`/time set day`), the friend puts on the elytra, runs `/tp @s ~ ~60 ~`, glides and uses the red rocket. The friend boosts. At the end of the boost, the red ball goes off at the friend's side and the friend loses 3.5 hearts. The host sees the spark trail behind the friend and the red ball at the friend. **Screenshot 85** (host's window). **Screenshot 86** (friend's window).
139. **The friend's shield.** The friend puts a black banner and a plain shield in the inventory's 2x2 grid: the result is a black shield. The friend puts it in the offhand, selects the iron sword and holds right-click: the host sees it black on the friend.
140. **Bells.** Give the next kit, Part X2's. The friend places a bell as in step 72 and throws a snowball at the side facing them while the host stands near. Both windows hear the ring and see the same swing. At night, the friend stands next to the host behind a bell, set up as in step 76, while a skeleton shoots at them. Both windows hear each ring and see the bell swing. **Screenshot 87**, both windows.

### Maps and End crystals (Part P's kit)

A fresh world for this part. Both players quit to the main menu and close their windows; the script in the first PowerShell window ends when the host window closes. Put Part P's kit line in the game folder's `.env`, then run in the first PowerShell window:

```
& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r6maps-mp -Fresh -HostSound
```

Both players get Part P's kit on their first join. After the new world's three commands, the host runs `/gamemode creative @a`.

141. **The friend gets the picture.** The host gets a Buried Treasure Map as in step 80. The host holds it near the X until land has filled in, then lands on a beach and runs `/tp Friend @s`. The host drops the map with Q next to the friend, and the friend picks it up and holds it with an empty offhand. The friend sees the same coast, X and filled-in land as the host did, with the friend's own white arrow. **Screenshot 88**, both windows.
142. **The friend fills it in.** The friend flies toward a blank corner of the map: the friend's map fills in around them. The friend drops the map back to the host with Q. The host picks it up and holds it: the land the friend filled in shows on the host's map too.
143. **A friend's own map.** The host runs `/gamemode survival @a`, and both stand on the ground side by side. The friend selects the Empty Maps and right-clicks the air. The friend gets a Map, and the host hears the cartography table's pencil scratch from where the friend stands. The friend holds the new map and walks 20 blocks: it fills in on the friend's screen. **Screenshot 89**, the friend's window.
144. **Rejoining.** The host does not carry the friend's new map. The friend quits to the main menu and closes the window. Then start the friend window again from the second PowerShell window with `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -FriendOnly`. The friend selects the same map: it shows the land filled in before, with the friend's arrow, not a blank checkerboard. **Screenshot 90**, the friend's window.
145. **Crystals and the dragon together.** Both put on the diamond armour. The host runs `/execute in minecraft:the_end run tp @a 0 70 0`. Both jump down and walk 20 blocks away from the portal.
     - The host places obsidian with an end crystal on it, 10 blocks from both. The friend throws a snowball at the crystal: it blows up in both windows.
     - The friend throws the trident at the flying dragon's head: the dragon flashes red in both windows, and the boss bar drops by about a 25th for both. **Screenshot 91**, both windows.

### Silverfish and endermites (Part S's kit)

Another fresh world, in the Overworld. Both players quit to the main menu and close their windows. Put Part S's kit line in the game folder's `.env`, then run in the first PowerShell window:

```
& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r6silverfish -Fresh -HostSound
```

Both players get Part S's kit on their first join. After the new world's three commands, play by day on flat open grass.

146. **Silverfish out of stone.** The friend stands 5 blocks from the host, places an infested stone block on the ground and mines it with the plain pickaxe: in both windows a silverfish comes out of it with a small white puff and bites the friend. Then the friend places an infested stone brick block and mines it with the Silk Touch pickaxe: no silverfish in either window. Both windows: **Screenshot 92.**
147. **A hurt silverfish wakes the stone.** The host builds a row of three infested stone bricks. The friend hatches a silverfish beside it and hits it once with an empty hand. About a second later, in both windows, one or more of the row's blocks break, and a silverfish crawls out of each with a puff. The friend kills them with one sword hit each.
148. **Diving.** The host builds the stone brick pen from step 109 (4 by 4 inside, 2 high, on grass) and hatches a silverfish inside. Both players stand 6 blocks away in creative (`/gamemode creative @a`). When it dives into one of the pen's lowest blocks, both windows show the puff where it stood. Both windows: **Screenshot 93.** `/gamemode survival @a`.
149. **Pearls.** The host runs `/gamerule fall_damage false` and `/gamemode creative`, so the endermite goes for the friend and not the host. The friend throws ender pearls one at a time until an endermite appears on the spot the friend threw from. The host sees it at the same spot with its purple sparkles, and both windows show it biting the friend. Both windows: **Screenshot 94.** The host runs `/gamerule fall_damage true` and `/gamemode survival`.

## Known limits

These are known gaps, not new bugs.

- **The warden:**
  - A warden's blow on a raised shield still pushes you back; in vanilla a blocked blow does not.
  - When a warden is hurt, only its body flashes red; its glowing spots, tendrils and heart do not.
  - A vibration's ripple flies to where the warden's head was when it heard the vibration, and does not follow the warden if it walks on.
  - A sonic boom on a mob is reduced by the mob's armour, and a witch takes all of it; in vanilla the boom ignores armour and a witch takes only 15 %.
  - The warden's darkness shows its effect icon; vanilla hides it.
- **Margit:**
  - Margit is our own stand-in: a cube model with a skin painted by code, our own moves and numbers, and vanilla sounds. Nothing comes from ELDEN RING yet, and the host does not load or send its assets.
  - He walks through water and lava with his dry-ground walk, though lava and fire still hurt him.
  - The fog ring is only gold sparkles on the ground: it blocks nothing, and a player more than 10 blocks above the ground there sees none of it.
  - Loading the world starts his fight over on purpose: full health, phase 1, and he walks back to his spot unless a player is within 24 blocks of it.
  - He takes no damage while he sleeps or walks home after a reset.
  - His poses switch from one clip to the next without a blend, so a Margit walking home drops straight onto his knee when he reaches his spot.
  - A friend sees only the three newest daggers in flight, so an older fourth dagger can still hit the friend unseen.
  - Only a plain lantern makes the altar; soul and copper lanterns do nothing.
  - He rises from the altar facing the nearest player, who is not always the one who placed the lantern.
  - Not built yet: a spawn egg, lock-on, a dodge roll, hit-stop, status build-up, an Elden Ring style bar, the gate arena, Margit's Shackle and the MW2 knife against him. Other mobs never fight him.
- **Glowing outlines:**
  - Mobs cannot be made to glow with /effect, because it takes players only: `/effect give @e[...] minecraft:glowing` answers "Selector not allowed". A mob glows only from a rung bell (raiders and witches) or from `/summon` with `active_effects`.
  - Spectral arrows make nothing glow: players have no bow in the fork, and no arrow gives a mob the Glowing effect.
  - Only pillagers, vindicators, evokers, ravagers, witches and the newer mobs (silverfish, endermites, rabbits, polar bears, breezes, wardens) get the outline. A glowing zombie, cow or other older mob gets none, and neither does a glowing player.
  - Margit never glows, because his effects are cleared every tick.
  - In a friend's window only raiders and witches get the outline. A silverfish, rabbit, polar bear or breeze summoned glowing shows its outline only in the host's window.
  - The outline is a rim a sixteenth of a block wide around the model, so it gets thinner on screen with distance. Vanilla's outline keeps the same width on screen.
  - Items a glowing mob holds, such as a pillager's crossbow, get no outline.
- **Fireworks, shields and elytra:**
  - A firework star's icon stays plain grey. Firework stars and rockets show no tooltip lines (shape, colours, flight), so you only see the colours when the rocket bursts.
  - If an elytra rocket hits a mob or a block early, it bursts there, but your boost still lasts its full time.
  - A second rocket used during a boost only stretches the boost to the longer of the two, where vanilla adds both together.
  - An elytra rocket stays at your hand's side until it goes off, even after you land, where vanilla moves it to your feet.
- **Bells:** MW2 bullets don't ring bells, because vanilla has no bullets. The player has no bow, so only mobs' arrows can ring a bell with an arrow.
- **Maps:**
  - A map in an item frame does not show its picture, and frames put no marker on maps.
  - Banners can't be marked on maps yet: using a map on a banner adds nothing.
  - The held map is always fully bright, while vanilla's darkens at night and in caves.
  - Your arms are not drawn holding the map, and the map does not dip when you switch to it.
- **The dragon and End crystals:**
  - Arrows hit neither the dragon nor the end crystals yet.
  - A splash potion of harming or healing does not reach the dragon.
  - A trident the sitting dragon shrugs off is not set on fire by its flames.
- **Silverfish and endermites:**
  - `/gamerule spawn_monsters false` does not stop endermites from pearls, or natural monster spawning, because the rule never reaches the host's world; `/gamerule spawn_mobs false` does stop them.
  - No mob uses the new flying navigation yet: parrots, bees, allays and ghastlings come with later tracks, and the wither still flies straight at its target instead of around walls.
  - The Infested effect still lets no silverfish out of a hurt mob or player.
- **Not in this build:** the other M1 animals (foxes, goats, parrots, pandas, turtles, armadillos, sniffers and bees: their eggs still hatch nothing), the M2 and M3 mobs, and the game events W's first merge left out (W's segment w3).
- Release 5's and the earlier releases' other known limits still apply, except the lines this build fixes:
  - **Release 5:** "Track E1's second segment was not reviewed" (its review is in), "Glowing mobs have no white outline yet", and "Margit, the warden, the new firework shapes and crafting, and held maps are not in this build".
  - **Release 4:** "The warden is not in this build" (a fourth warning now brings one), and from its "Not in this build" list the warden, Margit's look, hits and altar, segment e2b and silverfish hiding in stone. The other M1 animals and the M2 and M3 mobs are still missing.
  - **Release 3, Margit:** all of "What's missing" (he is drawn, with his mist, sparks, dust and fog ring; his strikes and daggers hurt; he has his bar, music, altar, drops and reward), the line on MW2 bullets and blasts, the bed beside him and his strikes through walls. His walk through water and lava, the "By design" lines and his estimated numbers still apply.
  - **Release 3, other lines:** snowballs and tridents now hit End crystals and the ender dragon (rockets still don't); holding a map shows it, and maps fill in as you walk; bursts take their stars' shapes, fades, trails and twinkles, and stars and rockets with stars are crafted; silverfish hide in stone and come out of infested blocks, and pearls leave endermites.
  - **Release 2:** the two silverfish and endermite lines (hiding, bursting out of infested blocks, waking, and endermites from pearls); firework stars are drawn in their bursts (their icon is still plain grey); and the warden and Margit in its "Not in this build" list.
  - **Release 1:** bells: arrows from mobs ring them now, and the glow is an outline, not a white hull.

## Screenshot numbering

- Name each screenshot by its number: `14.png`. When a step asks for both windows, name them `73-host.png` and `73-friend.png`.
- When a step fails, take a screenshot even if the step doesn't ask for one. Name it by its part letter and step number, e.g. `W-8.png` or `F-120-friend.png`. Say in one line what you expected and what you saw.
- Send `latest.log` after any crash. In two windows, send both windows' logs. Single player: `iw4l-artifacts\logs\latest.log`. With the script: `iw4l-artifacts\mp-local\host\iw4l-artifacts\logs\latest.log` and `...\mp-local\friend\...`.
