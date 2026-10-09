# Round 7, release 1: test steps

Phase 1 and tracks A-D. Nether travel (screenshot 8) is not in this build.

**Setup:** a class with Frag and Flash. In the `.env`, set a new world name such as `IW4L_MINECRAFT_WORLD=r7test`, remove any `IW4L_MINECRAFT_WEATHER` and `IW4L_MINECRAFT_TIME` lines, and add this kit. `/kit` gives it again, except the three potions and the red leggings: its parser skips `@` entries, so only the start kit gives those.

```
IW4L_MINECRAFT_KIT=minecraft:experience_bottle*64,minecraft:snowball*16,minecraft:egg*16,minecraft:ender_pearl*16,minecraft:wind_charge*64,minecraft:splash_potion@poison,minecraft:splash_potion@harming,minecraft:lingering_potion@regeneration,minecraft:trident,minecraft:trident+loyalty=3,minecraft:red_shulker_box,minecraft:chest*4,minecraft:ender_chest,minecraft:barrel,minecraft:diamond_helmet+protection=4,minecraft:iron_chestplate+protection=4,minecraft:leather_leggings@red,minecraft:iron_boots,minecraft:diamond_sword+sharpness=5,minecraft:torch*64,minecraft:cobblestone*64,minecraft:golden_apple*16,minecraft:furnace,minecraft:raw_iron*16,minecraft:coal*16,minecraft:campfire,minecraft:beef*4
```

## Death and void

1. Type `/gamerule keep_inventory true`. In 26.3 the rule is `keep_inventory`, not `keepInventory`.
2. Near spawn: `/gamemode creative`, `/fill ~-1 ~-1 ~-1 ~1 ~-40 ~1 air`, drop to the bottom of the shaft, `/gamemode survival`, wait 10 s. You stay alive (no "Killed By FNG"). **S1**
3. `/tp ~ -150 ~`: hurt every half second, then `<name> fell out of the world` in chat, kill feed empty. Creative dies too. **S2**
4. Without armour, jump off a 25-block pillar (20 blocks only take 17 of your 20 health): `<name> fell from a high place` in chat, still shown while dead.
5. `/summon minecraft:skeleton ~ ~ ~6` and let it kill you: `<name> was shot by Skeleton`. **S3**
6. Kill yourself with a frag: the MW2 kill feed shows its normal line.

## Grenade keys

1. Options > Controls: lethal is on G, tactical on B, and F swaps hands. **S4**
2. With cobblestone or an empty slot selected, G throws a frag in MW2's hand and B a flash. **S5**
3. With cobblestone selected and looking at the ground, cook a frag (hold G) and right-click: no block is placed.
4. With the golden apples in the offhand (select them and press F) and a gun selected, hold F: you eat. Press B: a flash flies.

## Chat

1. Copy `/gamerule keep_inventory true` in Notepad. Press T, then Ctrl+V: you get exactly that line, with no `v`. **S6**
2. Type `hello`, Ctrl+A (blue box), Ctrl+C, type `x`, Ctrl+V: `xhello`. Ctrl+Shift+Left and Ctrl+X cut a word that pastes in Notepad.

## Throwables

1. Throw a bottle o' enchanting: you hear glass, a blue swirl rises and XP comes out. **S7**
2. Throw the splash Poison straight up so it lands on you: your hearts turn green.
3. The lingering Regeneration leaves a pink disc that shrinks over about 25 s.
4. Hold right-click with the trident: it pulls back. Release it: a 3D trident flies and sticks in the ground. **S8**
5. The Loyalty trident flies back to you.
6. A wind charge at your feet launches you 5-6 blocks with no fall damage.

Known gaps: no Riptide spin attack, and Speed or Night Vision effects do nothing.

## Lids

1. Open the shulker box: the lid rises and turns, with a sound, then closes back down. **S9**
2. On a wall or a ceiling, the lid slides out sideways or downward.
3. Single and double chests and the ender chest swing open; a double chest plays one sound. **S10**
4. A barrel shows open while its screen is up.

## Body gear in F5

1. Wear the four armour pieces, then press F5 and look from the back and the front. The armour bends with walking and prone, and the helmet and chestplate shimmer. **S11**
2. Select cobblestone with a torch in the offhand: they are in the right and left hands, and there's no gun. **S12**
3. The sword has the finer glint, the trident shows its 3D model, and the gun slot brings the gun back.
4. Press E: the character in the box wears and holds the same items. **S13**
5. Report anything floating or wrongly sized, with the `Minecraft gear bones missing` lines from `iw4l-artifacts\logs\latest.log`.

## Commands and tab completion

1. Type `/ga`: you see `gamemode` in yellow and `gamerule` in grey, with the rest of the word greyed in after the cursor. Tab and Shift+Tab cycle. **S14**
2. `/give ` then Esc shows the grey usage line. `/gamemode god` turns `god` red and shows the error. **S15**
3. `/xp set @s 0 levels`, then `/xp add @s 5 levels`: the bar shows 5 and the level-up chime plays (it only chimes on landing on a multiple of 5, and the bottles already gave you levels). Holding the sword, `/enchant @s unbreaking 3` works.
4. `/setblock ~ ~-1 ~ gold_block` answers "Changed the block at x, y, z". `/fill ~1 ~ ~1 ~5 ~4 ~5 glass hollow` answers "Successfully filled 98 block(s)".
5. Run `/spawnpoint`, walk 50 blocks away and `/kill`: you respawn on that block.
6. `/clear` then `/kit`: you get a "Gave" line for each of the 23 entries without `@` (no potions and no red leggings). `/execute in minecraft:the_nether run seed` says "Unknown dimension 'minecraft:the_nether'", which is expected.

## Creative inventory

1. `/gamemode creative`, then E: you see the vanilla tabs above and below. **S16**
2. Click takes 1, shift-click 64, a number key fills that slot. Shift-click on the trash empties everything (then `/kit`, which again leaves out the potions and the red leggings). Potions all share one icon colour for now.
3. In Search, `wool` shows 16 wools. `#logs` shows a purple tag line in the tooltip. **S17**
4. Hold C and press 1 to save the hotbar. Hold X and press 1 to load it back. Then `/gamemode survival`. **S18**

## Saving and restart

This needs `IW4L_MINECRAFT_WORLD` set (a `temp` world isn't saved) and a full restart.

1. Fill a chest and the shulker box, then put an item in the ender chest. Put iron and coal in the furnace, and beef on the campfire.
2. Put on the armour you have (after `/clear` and `/kit` that is the helmet, chestplate and boots), note your hearts, hunger and level, and drop (Q) the sword.
3. `/weather thunder`, `/time set night`, `/time pause`. **S19**
4. Quit through the menu and start again. The log should say `Minecraft kit skipped`.
5. You should be on the same spot, facing the same way, at night with thunder. Armour, hearts, hunger and level are the same. **S20**
6. The containers hold the same items, the furnace carries on and the sword is still on the ground. **S21**
7. T then Up: your earlier commands come back. Then `/time resume`, `/weather clear`.
