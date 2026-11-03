# Round 10 release 8: riders share their mount's fall

Release 8 is `minecraft-survival` at cc26ea7f, copied in as `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-survival.exe`. Release 7 is kept beside it as `iw4l-survival-6ad783e.exe`. `iw4l-master.exe` (the relay) was copied in again.

New since release 7:
- Track E3, segment e3a (merge aa54bd0c): mobs and players riding a horse or a strider now take their share of its fall, and a rider thrown up by a wind charge counts only the fall below the burst. The same segment fixed every older clippy warning in the fork's Minecraft crates and the four old warnings of the launcher build, with no change in behaviour; Part S checks that nothing looks different.
- Nothing else in the game changed. The other tracks were stopped for the week before their next segments were done (`docs/PROGRESS.md`).

**Multiplayer:** this build is still protocol 12, like release 7, because e3a changed no network message. Use this build on both PCs anyway, so host and friend run the same rules.

Release 7's steps (`docs/round10/release-7-tests.md`) and the earlier releases' still stand; this file only covers what is new. Screenshots are numbered 1 to 12 in the order below. Each part has its own kit: put its line in the `.env` next to the exe as `IW4L_MINECRAFT_KIT=...` and start a new world. The background for every step is in `docs/round10/track-e3-log.md` (segment e3a: groups tool-lints-terrain, tool-lints-fork and rider-falls).

Setup for every part: survival, Normal, by day on flat open ground, unless a step says otherwise. Read your position with `/tp @s ~ ~ ~` (the fork has no F3 screen).

## Part S. Nothing looks different

Kit (17 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:water_bucket,minecraft:lava_bucket,minecraft:cow_spawn_egg*2,minecraft:wheat*16,minecraft:villager_spawn_egg,minecraft:composter,minecraft:iron_sword,minecraft:grass_block*16,minecraft:oak_leaves*16,minecraft:oak_fence*16,minecraft:cobblestone*64,minecraft:cooked_beef*16,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:chest,minecraft:crafting_table,minecraft:writable_book*2
```

A new world in a plains with water nearby, by day. These steps touch every place the lint fixes edited; everything should look and behave as in release 7.

1. **Blocks and fluids.** Look round: grass and leaves are green, water and lava animate, fences connect, and block corners are shaded. Place the water bucket on a slope and the lava bucket in a stone pit: the flowing water runs down its slopes and both surfaces animate as before. **Screenshot 1.**
2. **Item icons.** Open the inventory: the cobblestone, grass block, fence and leaves icons are 3D, and the grass and leaves icons are green. Hover over them: the names show. **Screenshot 2.**
3. **Cows and a bogged.** Use both cow eggs, then `/summon minecraft:cow ~ ~ ~4 {variant:"minecraft:warm"}`: the warm cow has its horns and its orange coat. Hold wheat: the cows follow you and their legs swing as they walk. Then `/summon minecraft:bogged ~ ~ ~6 {NoAI:1b}`: it has red and brown mushrooms on its head. **Screenshot 3.**
4. **Drops.** Kill a cow with the sword: it drops beef, maybe leather, and experience orbs. Walk over them: they go into your inventory.
5. **A farmer.** Place the composter, use the villager egg beside it and wait until the villager becomes a farmer (green sparkles, a straw hat). Right-click it: the trading screen lists its offers and closes as before. You hear its voice. **Screenshot 4.**
6. **Save and load.** Save and Quit, then load the world: the cows, the bogged and the farmer are where they were, and the farmer offers the same trades.
7. **Armour on the inventory figure.** Put on the iron helmet and chestplate and open the inventory: the character in the inventory wears them and turns towards the mouse. In third person (F5) the MW2 body wears them too. **Screenshot 5.**
8. **Drag split.** Place the chest and open it (you hear it open). With the left button held, drag the stack of 64 cobblestone over four empty slots: while you drag, each slot shows 16, and the counts stay when you let go. Hover over the cobblestone: its name shows. Place the crafting table and open it: it works as before. **Screenshot 6.**
9. **A book.** Write two pages in a book and quill, press Done and open it again: the pages are kept. **Screenshot 7.**
10. **Skating.** Press **J**: you skate as before. Bail off a ledge: you get back on the board. **Screenshot 8.**

## Part R. Riders share their mount's fall

Kit (10 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:strider_spawn_egg*2,minecraft:warped_fungus_on_a_stick,minecraft:wind_charge*16,minecraft:cobblestone*64,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:iron_sword
```

A new world, survival, by day on flat grass. Be at full health before each drop (eat).

11. **A pillar.** Build a 3-high pillar and stand on it: `/fill ~2 ~ ~ ~2 ~2 ~ cobblestone`, then `/tp @s ~2 ~3 ~`.
12. **A husk on a horse, 15 blocks.** Run `/summon minecraft:horse ~4 ~12 ~ {Passengers:[{id:"minecraft:husk"}]}`: the horse falls 15 blocks with the husk on it. On landing you hear the horse's thud and a heavy fall crunch from the husk, and both flash red. The husk rides on (it lost 12 of its 20). **Screenshot 9**, just after the landing.
13. **25 blocks.** Run `/summon minecraft:horse ~4 ~22 ~ {Passengers:[{id:"minecraft:husk"}]}`: the horse flashes red and lives, and the husk dies on landing. **Screenshot 10.** In release 7 both husks rode on unhurt. Kill the survivors with the sword.
14. **A strider, 6 blocks.** At a fresh spot, run `/fill ~3 ~ ~-3 ~9 ~5 ~3 netherrack` (6 high), then `/tp @s ~6 ~6 ~`. Use a strider egg on the top, saddle the strider and get on. Hold the warped fungus on a stick and ride off the edge. You lose 1.5 hearts and hear your fall; the strider loses 1.5 of its 10 mount hearts. **Screenshot 11**, right after landing.
15. **4 and 3 blocks.** Repeat from 4 high (`/fill ~3 ~ ~-3 ~9 ~3 ~3 netherrack`, `/tp @s ~6 ~4 ~`): you lose half a heart. From 3 high you lose nothing.
16. **A wind charge (optional).** Run `/fill ~3 ~ ~-3 ~9 ~11 ~3 stone` (12 high), `/tp @s ~6 ~12 ~`, then `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`. Saddle the horse and ride it off the edge: you lose 4.5 hearts. Get back up (`/gamemode creative`, fly up, `/gamemode survival`). Summon and saddle a new tame horse and ride it. Look straight down, throw a wind charge at its feet, and ride off the edge within two seconds. You again lose 4.5 hearts, not more.

## Part F. Friends (two windows, both on this build)

Put Part R's kit line in the game folder's `.env` (`C:\Games\Mashup\2010-Rust-Rewrite-Mashup\.env`): in a new world both players get it on their first join. Then open the two windows from PowerShell in `C:\Dev\Soscraft\mashup-crafting`, as in release 7's Part F:

```
& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r8friends -Fresh -HostSound
```

H is the host window, F the friend's.

17. **F's own strider.** F does step 14 on F's own strider at a fresh spot. F loses 1.5 hearts, and H hears F land. **Screenshot 12**, F's window.
18. **F's book.** F writes two pages in a book and quill (give F one with `/give @s minecraft:writable_book`), presses Done and opens it again: the pages are kept.

## Known limits

- **Falls shared with riders:** only horses (with donkeys, mules and llamas) and striders hand their fall to their riders. A mob riding a ravager or a spider, or a registered mob, takes no share. A horse or strider that is already dying when it lands hands nothing on, and a dying horse's landing has no thud of its own yet (E3's e3b, stopped).
- **Fall-damage grace:** every push (a hit, an arrow, an explosion, the dragon's wings, a ravager's roar, a shield block, a breeze's wind charge) starts the grace that vanilla starts only for your own wind charge and the mace. So a player, or a rider's share, thrown up by TNT or a creeper takes no fall damage when landing at the height of the hit. E3's e3g fixes this.
- **Snow golems and blazes** are still hurt by their own landings, and a **creeper** does not swell when it lands (E3's e3z).
- **Not in this build:** the mount screens shared by several players and the mount drawn in its screen (E3's e3b, stopped), and the dispenser, dropper, hopper, beacon, loom and cartography table screens (track C, planned).
- Release 7's and the earlier releases' other known limits still apply, except this line, which this build fixes: release 7 and earlier had mob riders landing unhurt when their horse fell (`docs/round10/track-e3.md`, gaps horses-4 and horses-7).

## Screenshot numbering

Name the screenshots `1.png` to `12.png` in the order above. Part S is 1 to 8, Part R 9 to 11 and Part F 12. If a step shows something else, add a short note with the step number.
