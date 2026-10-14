# Round 9, release 2: test steps

Build: `iw4l-survival.exe` from `minecraft-survival` at `f7432188`: release 1 plus track C and track D's segments d1 and d2. The release 1 exe is kept as `iw4l-survival-57302e9.exe`. Tracks A and B were tested in [release-1-tests.md](release-1-tests.md) and are not repeated here.

**Multiplayer is protocol 5** (release 1 was 4). A friend on the release 1 exe is refused at the join with the version message, so both windows must run this exe (`mp-local.ps1` does that). The relay `iw4l-master.exe` was rebuilt with it and copied next to the exe. A friend over the internet needs a new zip from `-FriendKit`.

The whole test takes about 2.5 to 3 hours. Steps marked **optional** can be skipped.

## What is in this build

Track C, guns and grenades as real items ([track-c-log.md](track-c-log.md), [../WEAPONS.md](../WEAPONS.md)):

- Guns, ammo, attachments, gun parts and grenades are Minecraft items (`iw4l:` ids) with MW2 icons, names and tooltips. They are saved in a vanilla form, drop on death, go into chests and lie on the ground as their icon.
- MW2 holds only the guns on the hotbar. The gun copied by a chest (your round 8 screenshot 12) is gone. The free Intervention, SPAS-12 and UMP45 are gone too.
- The loaded rounds live on the gun (the item bar under its icon). A reload takes ammo items from the inventory.
- On the first join each player gets the class guns, three magazines each and 32 + 32 grenades, once.
- Every gun, ammo type, grenade and attachment has a recipe. The inventory and the crafting table have vanilla's recipe book. All fork items are at the end of the creative Combat tab.
- Attachments go onto a gun in a crafting grid, two identical guns make an akimbo pair, and a grindstone strips a gun.
- Two grenade slots, lethal and tactical, left of the hotbar. G and Shift+G throw from them.
- The tactical insertion is a one-use respawn point. It sits in the lethal slot, as MW2 keeps it.
- In Minecraft worlds the class menu changes perks only.
- Guns do half damage to players, and armour takes twice vanilla's share. LMGs, snipers and FMJ pierce part of it. A popped totem turns bullets and explosions away for 4 seconds.

Track D, segments d1 and d2 ([track-d-log.md](track-d-log.md), [mobs.md](mobs.md)):

- d1: drowned (spawning, tridents), the vanilla trading screen for the host and friends, zombie villagers and their cure, omens, ominous bottles and effect icons, cats and ocelots, mules, llamas and trader llamas, riding and the horse inventory, leads, wandering traders, built iron and snow golems, loot functions (worn gear, potions and more in structure chests) and ancient city chests.
- d2: pillagers and outposts, patrols with a banner captain, vindicators, evokers and vexes, ravagers, raids with the bar and the horn, witches healing raiders, bells, villagers hiding, hero gifts, celebration fireworks and glowing raiders.

## Setup

- **Class:** on the main menu pick a class with an **M4A1 with Red Dot Sight, a USP .45, Frag and Flash**. The steps count on it. In a Minecraft world the class menu no longer changes guns or grenades.
- **Single player:** set `IW4L_MINECRAFT_WORLD=<world>` in the `.env` next to the exe, then start `iw4l-survival.exe map minecraft:overworld`.
- **Two windows on this PC:** run from PowerShell in `C:\Dev\mashup-crafting`:

  ```
  & scripts\mp-local.ps1 -Exe iw4l-survival.exe -World <world> -Fresh -HostSound
  ```

  - `-Fresh` starts the world new. Leave it out to go on with a world.
  - To start only the friend window again: add `-FriendOnly` instead of `-Fresh`. Add `-Friend Bob` to join as a new player.
  - By hand, use `IW4L_MINECRAFT_WORLD=<world>` in the host window, and delete `iw4l-artifacts\saves\<world>` first for a fresh world.
- **Names:** `Sos` is the host's player name and `Friend` the friend's. Use your names if they differ.
- **Kits:** put the part's kit in the game folder's `.env` as one line, replacing the old line. Both windows read it. A kit fills only the backpack of a player new to the world.
  - **The first join also gives the class loadout**, to the host and to every friend: the M4A1 (red dot) on hotbar slot 1 and the USP .45 on slot 2, three magazines each in the backpack (90 rifle and 36 pistol rounds) and 32 frags and 32 flashes in the two grenade slots. So every kit here is at most 24 stacks: the ammo needs 3 backpack slots.
  - The loadout comes once per player and world (the tag `iw4l_loadout_given`). In a world played before, `/clear` and `/kit` give the kit again, but no loadout.
- Remove `IW4L_MINECRAFT_DIMENSION` and `IW4L_MINECRAFT_SEED` from the `.env`. Keep it free of `IW4L_MASTER_HOST_NAME` and `IW4L_MASTER_JOIN`.
- **In every new world**, the host types `/difficulty normal` and `/gamerule keep_inventory true` first. In two windows the host also types `/op Friend`.
- **Keys:** G throws the lethal grenade, Shift+G the tactical one, F swaps hands, E opens the inventory, Q drops, T opens the chat for commands.
- **Logs:** single player and by hand, `iw4l-artifacts\logs\latest.log`. With the script, `iw4l-artifacts\mp-local\host\iw4l-artifacts\logs\latest.log` and `...\mp-local\friend\...`.
- Steps marked **(two windows)** need the friend. Only the window in front gets your keys.

## C1. Guns as items

World `r10c1`, single player, a fresh world. Start by day on open grass.

Kit C1 (24 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:m4,iw4l:usp,iw4l:usp,iw4l:barrett,iw4l:fmj,iw4l:acog_scope,iw4l:extended_mags,iw4l:red_dot_sight,minecraft:chest,minecraft:crafting_table,minecraft:grindstone,minecraft:item_frame*2,minecraft:lava_bucket,minecraft:cobblestone*64,minecraft:iron_ingot*32,minecraft:iron_block*2,minecraft:copper_ingot*16,minecraft:iron_nugget*16,minecraft:gunpowder*32,minecraft:tripwire_hook*2,minecraft:redstone*8,minecraft:repeater*2,minecraft:oak_log*4
```

The kit's two M4A1s and two USP .45s are bare, with full clips.

### First join, tooltips and reloads

- **C1-1. First join.** Hotbar slot 1 holds the M4A1 and slot 2 the USP .45. Left of the hotbar are two frames: 32 frags (nearer the hotbar) and 32 flashes. The backpack holds the kit, 90 rifle ammo (64 + 26) and 36 pistol ammo. The mouse wheel raises only those two guns: no Intervention, SPAS-12 or UMP45. **Screenshot 1** (inventory open). `/tag @s list` lists `iw4l_loadout_given`.
- **C1-2. Icons and tooltips.** Guns show their MW2 kill icon, the attachments their MW2 picture, rifle ammo a copper nugget and pistol ammo a brass one. Hover the slot 1 M4A1: "M4A1", "Red Dot Sight", "Clip 30/30", "Ammo: Rifle Ammo", then MW2's stat lines. A full green bar sits under its icon. Hover the Red Dot Sight: "Craft together with a gun to attach it". **Screenshot 2** (the M4A1 tooltip).
- **C1-3. A reload takes items.** Press 1, fire 12 rounds and press R. MW2's counter shows 30 and, on the right, the rifle rounds you carry (or MW2's maximum, if smaller). Open the inventory: the rifle ammo went from 90 to 78 in all. **Screenshot 3.** Press 2 and fire the USP until it reloads by itself: the pistol ammo drops, the rifle ammo stays.

### Chests, copies and empty guns

- **C1-4. No copy from a chest.** Place the chest and shift-click the slot 1 M4A1 into it. Close it. You hold no MW2 gun, pressing 1 raises nothing, and the only M4A1 with a red dot is the one in the chest. **Screenshot 4** (chest and inventory open). Take it back to slot 1: it comes up with the same clip.
- **C1-5. Two copies.** Move the two bare M4A1s to slots 4 and 5. Press 4 and fire 10 rounds. Press 5: a full 30. Press 4: 20 again.
- **C1-6. No ammo.** Put all rifle ammo into the chest. Press 1 and fire the M4A1 empty. The bar under its icon turns yellow at half and shrinks to one red pixel. It clicks, the counter shows 0 and 0, and R does nothing. Take 5 rifle ammo back: the right number shows 5 at once, R loads 5, and those 5 items are gone. **Screenshot 5** (the hotbar with the bars).
- **C1-7. A thrown copy keeps its own clip.** Leave the rest of the rifle ammo in the chest. Press 4 and fire that M4A1 empty. Press 5, hold fire and press Q after a few rounds, while still firing. Slot 4 still shows the empty bar. **Screenshot 6** (hotbar). Pick the thrown M4A1 up and select it: its clip is 30 less what you fired, and slot 4 still clicks empty. Take all rifle ammo back and reload both.
- **C1-8. No guns in the offhand.** Select the USP .45 and press F: nothing swaps. In the inventory, pick up the Barrett and click the offhand slot: it stays on the cursor.

### On the ground and in frames

- **C1-9.** Press Q on the Barrett, the Red Dot Sight and one rifle ammo. Each lies on the ground as its inventory picture, a few pixels thick and spinning like a vanilla item: the Barrett as its kill icon, the sight as its picture, the ammo as a copper nugget. **Screenshot 7.**
- **C1-10.** Place the two item frames on a cobblestone wall and put the Barrett and the Red Dot Sight in them: flat pictures on the wall. **Screenshot 8.** Hold rifle ammo in the main hand: it is drawn like a vanilla item. Hold the M4A1: MW2 draws the gun. Hit the frames and pick everything up.
- If the log has "Minecraft fork items with a stand-in look on the ground: ...", send that line.

### Recipes and the recipe book

- **C1-11. Crafting unlocks the recipe (do this first).** Open the inventory. Put an oak log into the 2x2 grid and click the planks result: 4 planks on the cursor. Right-click the four grid cells so each holds one plank, without putting planks into the inventory. Click the crafting table result.
- **C1-12. The book.** Click the green book button below the 2x2 grid: the book opens on the left and the inventory moves right. The crafting table from C1-11 is in it, highlighted. **Screenshot 9.**
  - The iron axe tab shows vanilla's equipment recipes first, then the fork's: gun parts, pistols as one button, and so on. Red buttons are the ones you can't make now.
  - Click Rifle Ammo: a copper ingot, an iron nugget and gunpowder jump into the 2x2 grid, and the result shows 16. Take it.
  - Type `m4` in the search box: the M4A1. While typing, E and the number keys type. Clear the box.
- **C1-13. A crafted gun.** Place the crafting table, open it and open the book. Click Gun Barrel (three iron ingots in the middle row) and take the 2 barrels. Click Firing Mechanism and take it. Click the M4A1: its ingredients jump into the grid. Take it and hover it: "Clip 0/30", with one red pixel under the icon. **Screenshot 10.**
- **C1-14. Ghosts.** Search `rpd` and click the RPD (no blaze rod in the kit): the grid empties, red ghost items show every ingredient, and a red RPD sits over the result. Click it again: nothing changes. Put any item into the grid: the ghosts go. **Screenshot 11.**

### Attachments, akimbo and the grindstone

- **C1-15. Attachments.** Put the crafted M4A1, the FMJ and the ACOG Scope into the 2x2 grid: the result is the M4A1 with "ACOG Scope" and "FMJ". Take it. Put it in the grid with the Red Dot Sight: no result (a third attachment). **Screenshot 12** (the tooltip with both attachments).
- **C1-16. Akimbo.** Put two USP .45s into the 2x2 grid: one USP .45 with "Akimbo" and "Clip 12/12 + 12/12". Take it. Two M4A1s give no result. Put the akimbo USP alone into the grid: one USP .45 comes out and the other stays in its cell. Make the pair again.
- **C1-17. An akimbo reload loads only what you have.** `/clear @s iw4l:pistol_ammo`. Put the akimbo pair on the hotbar, select it and fire both pistols empty. `/give @s iw4l:pistol_ammo 12` and press R. MW2's counter shows 12 rounds in all (one pistol full, the other empty), and no pistol ammo is left. **Screenshot 13.** Fire until both click: exactly 12 shots.
- **C1-18. The grindstone.** Put a bare M4A1 and the Extended Mags into the 2x2 grid: the result lists "Extended Mags" and "Clip n/45". Put it on the hotbar, select it and press R: "Clip 45/45". Place the grindstone and put that M4A1 in: the result has no Extended Mags and "Clip 30/30". Take it: a few XP points come (3 to 5). **Screenshot 14.** A bare gun in the grindstone gives no result.

### Fire, saving, death, creative and the class menu

- **C1-19. Netherite survives lava.** Dig a hole two blocks wide and pour the lava in. Look into the lava and press Q on the Barrett, then on the stripped M4A1. The M4A1 burns up, and the Barrett floats on the lava. **Screenshot 15.**
- **C1-20. Saving.** Put the akimbo USP into the chest. Save and quit, then load the world. The chest holds the akimbo USP with its clips. The hotbar M4A1's tooltip and bar are as before, the grenade frames show 32 and 32, and no second loadout came. **Screenshot 16.**
- **C1-21. Death.** `/gamerule keep_inventory false`. On flat ground, `/kill`. Guns, ammo, attachments and both grenade stacks drop as their pictures, and no MW2 gun model lies next to them. **Screenshot 17** (the drops). Respawn: you hold no gun and get nothing new. Pick everything up: the frags and flashes go back into their empty grenade slots. `/gamerule keep_inventory true`.
- **C1-22. Creative.** `/gamemode creative`, the Combat tab, scroll to the end. After vanilla's items come pistols, machine pistols, SMGs, rifles, shotguns, the riot shield, crossbow and ballistic knife, snipers, LMGs, launchers, the gun parts, attachments, the seven ammo types, the lethal grenades (the Tactical Insertion after C4) and the tactical grenades. **Screenshot 18.**
  - Hover the M4A1: "Clip 30/30". The Riot Shield has no clip line. Shift-click Rockets: a stack of 16.
  - Search tab: `M4A1`, `intervention` and `frag` each find their item. `/gamemode survival`.
- **C1-23. The class menu.** Open the class menu in the world and edit a custom class. The status line reads "Guns and grenades are items in Minecraft worlds". Primary, Secondary, Lethal and Tactical open nothing, and the perks open as before. **Screenshot 19.** After you quit to the main menu, the weapon rows open again.

## C2. Grenades and the tactical insertion

World `r10c2`, single player, a fresh world. Start by day on open grass.

Kit C2 (16 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:frag_grenade*2,iw4l:frag_grenade*16,iw4l:semtex*16,iw4l:c4*4,iw4l:throwing_knife*8,iw4l:flash_grenade*8,iw4l:concussion_grenade*8,iw4l:smoke_grenade*8,iw4l:tactical_insertion*8,minecraft:white_bed,minecraft:cobblestone*64,minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:shield,minecraft:torch*16,minecraft:cooked_beef*16
```

### The slots

- **C2-1. First join.** The lethal frame shows 32 frags and the tactical frame 32 flashes, and MW2's grenade icons at the bottom right show the same. **Screenshot 20.**
- **C2-2.** Press G: a frag is thrown, 31. Press Shift+G: a flash, 31. Holding cobblestone changes nothing: G still throws from the slot.
- **C2-3. Empty the slots.** `/gamemode creative`, open the creative inventory tab. The two grenade slots sit right of the leggings and boots slots. Click each stack onto the trash slot. `/gamemode survival`. The two frames left of the hotbar stay, empty, in the same place. Open the inventory: two slot frames right of the book button, level with the offhand. Hover them: "Lethal" and "Tactical". **Screenshot 21.**
- **C2-4.** Shift-click the stack of 2 frags: it goes into the lethal slot. Shift-click the flashes: into the tactical slot. Cobblestone is refused by both slots. Close: the frames and MW2's icons show 2 and 8.

### Throwing

- **C2-5.** Press G twice. The second throw empties the slot, and it refills at once with the backpack's 16 frags. Hold G until the frag blows up in your hand: 15. `/clear @s iw4l:frag_grenade 0` finds 15. **Screenshot 22.**
- **C2-6.** Shift-click the semtex: it goes to the hotbar, because the lethal slot holds frags. `/clear @s iw4l:frag_grenade`, then shift-click the semtex: into the lethal slot. G throws semtex.
- **C2-7. C4.** `/clear @s iw4l:semtex`, shift-click the C4. G places C4 four times. With the slot empty the C4 icon stays with 0, and G detonates the placed C4. **Screenshot 23.**
- **C2-8. Knives.** Shift-click the throwing knives into the lethal slot. Throw one at a wall (7) and walk over it: 8 again.
- **C2-9. Tacticals.** `/clear @s iw4l:flash_grenade`, shift-click the stun grenades: Shift+G throws a stun grenade. `/clear @s iw4l:concussion_grenade`, shift-click the smoke grenades: Shift+G throws smoke.
- **C2-10.** Save and quit, load: the counts in both frames are kept. `/gamemode creative`: G throws and the count stays. `/gamemode survival`.

### The tactical insertion

- **C2-11.** `/clear @s iw4l:throwing_knife`. Place the bed and right-click it ("Respawn point set"). Shift-click the tactical insertions: they go into the lethal slot (the left frame). Pick them up and click the tactical slot: refused. Put them back in the lethal slot. **Screenshot 24.**
- **C2-12.** Walk 30 blocks away, face north (F3), press G: the flare is thrown and the frame shows 7. Walk 20 blocks on and `/kill`. You respawn where you threw it, facing north. **Screenshot 25.** `/kill` again: you respawn at the bed.
- **C2-13.** Plant one at spot A, walk 20 blocks to spot B and plant another. Walk away and `/kill`: you respawn at B.
- **C2-14. A blast destroys it.** `/give @s iw4l:frag_grenade 4`. Plant an insertion. Shift-click the insertions out of the lethal slot and the frags in. Step back 6 blocks, throw a frag onto the spot and wait for the blast. Swap back: the frags out, the insertions in. `/kill`: you respawn at the bed.
- **C2-15. A blocked spot.** Plant one, put two cobblestone blocks on the spot, one above the other. `/kill`: you respawn at the bed.
- **C2-16. Another dimension.** Build a Nether portal with the obsidian and light it. Plant an insertion next to it, go to the Nether and `/kill` there: you respawn at the flare in the Overworld. **Screenshot 26.**
- **C2-17. Not saved.** Plant one, save and quit, load, `/kill`: you respawn at the bed.
- **C2-18. Optional: a class with the tactical insertion.** On the main menu set your class's Tactical to Tactical Insertion and Lethal to Frag. Start a new world `r10c2b` (the same kit is fine). Open the inventory: 32 frags in the lethal slot, 32 flashes in the tactical slot and 32 tactical insertions in the backpack. **Screenshot 27.** Set the class back to Flash afterwards.

## C3. Gun damage and the totem (two windows)

World: two windows, `-World r10c3 -Fresh`. Host H (`Sos`), friend F. Stand on flat ground about 5 blocks apart. Before each step both have full health: eat a golden apple or wait.

Kit C3, for both players (23 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64,iw4l:fmj*2,iw4l:barrett,iw4l:sniper_rounds*16,iw4l:rpd,iw4l:lmg_ammo*64,minecraft:crafting_table,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying*4,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:golden_apple*16,minecraft:zombie_spawn_egg*2,minecraft:cooked_beef*32
```

"The bare M4A1" is the kit's, without the red dot.

### Half damage and armour

- **C3-1 (two windows).** H unarmoured. F shoots H once in the chest with the bare M4A1: H loses 1.5 hearts (3 before this build). **Screenshot 28** (H's window).
- **C3-2 (two windows).** H puts on the iron set: an M4A1 chest hit takes about a third of a heart. F puts the M4A1 and one FMJ into the 2x2 grid and takes the M4A1 with FMJ: about 0.6 heart a hit. **Screenshot 29** (H's window).
- **C3-3 (two windows).** F fires the RPD at H in iron: about 0.8 heart a hit.
- **C3-4 (two windows). The Barrett.** Unarmoured, H dies on the second chest hit. In iron, about 3 hearts a hit and the fourth kills. **Screenshot 30** (H's hearts after one hit in iron). In the diamond set it is the same: the bare M4A1 about a third of a heart, the Barrett 3 hearts.
- **C3-5 (two windows).** F puts the second FMJ on the Barrett. Against H in iron: about 4 hearts a hit, and the third kills.
- **C3-6 (two windows). The knife pierces nothing.** F holds the FMJ Barrett and knifes H in iron with MW2's melee key: H loses about 2.3 hearts, the same as a knife with the M4A1 held.
- **C3-7. Own frag.** H in the diamond set throws a frag (G) at H's own feet: 1.5 hearts at most. Unarmoured: about 7.5 hearts, and it no longer kills from full health. Swap the plain diamond chestplate for the Protection IV one: the frag takes a little less. **Screenshot 31.**
- **C3-8.** `/summon minecraft:zombie ~ ~ ~5`: the M4A1 kills it in about 4 body hits, as before.

### The totem's 4 seconds

- **C3-9 (two windows).** H unarmoured with a totem in the offhand. F shoots H down to 2 or 3 hearts, then holds the trigger on H. The totem pops (it rises, glitters and sounds). For about 4 seconds H's hearts don't drop and F sees no hit markers; after that the absorption hearts go first. **Screenshot 32**, both windows, during the 4 seconds with F firing.
- **C3-10 (two windows).** H takes a new totem into the offhand and stands next to TNT. F lights the TNT and shoots H down just before it explodes. H loses no hearts and is not pushed (explosions push no player in the fork). **Screenshot 33.**
- **C3-11 (two windows).** Right after a pop, F knifes H: H takes damage. Right after another pop, F throws a frag at H's feet: no damage.
- **C3-12 (two windows).** Swap roles: F's totem protects F the same way. **Screenshot 34** (F's window).
- **C3-13.** Without a totem, H stands next to TNT when it explodes: the death message is "Sos blew up".

## D1. Villagers and trading

World `r10d1`, single player, a fresh world. Start by day on open grass, away from any village.

Kit D1 (22 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:emerald*64,minecraft:emerald*64,minecraft:wheat*64,minecraft:paper*64,minecraft:villager_spawn_egg*4,minecraft:composter,minecraft:oak_fence*32,minecraft:zombie_spawn_egg*4,minecraft:splash_potion@weakness,minecraft:splash_potion@weakness,minecraft:golden_apple*16,minecraft:iron_bars*8,minecraft:red_bed,minecraft:oak_planks*64,minecraft:oak_planks*64,minecraft:ominous_bottle*2,minecraft:ominous_bottle[ominous_bottle_amplifier=4],minecraft:milk_bucket,minecraft:milk_bucket,minecraft:diamond_sword,minecraft:torch*32,minecraft:cooked_beef*16
```

### The trading screen

- **D1-1.** Place the composter, fence a pen of about 5 x 5 around it and hatch a villager egg inside, next to the composter. Wait until it is a farmer (the straw hat; up to a minute or two by day).
- **D1-2.** Right-click it. The wide vanilla trading screen opens. On the left, "Trades" sits over the offer buttons: payment, arrow and result. On the right, "Farmer - Novice" sits over an empty experience bar, with the two payment slots and the result. The inventory is below. **Screenshot 35.** Hover an offer's payment, second payment and result: each shows its tooltip.
- **D1-3.** Click an offer: it stays highlighted, the payments move into the slots, and a light green part of the bar shows the experience the trade gives. Take the result, then shift-click the result to trade as often as your items last. The bar fills and small orbs drop. After a few trades the title is "Farmer - Apprentice" with more offers. **Screenshot 36.**
- **D1-4.** Buy one cheap offer until it is sold out: its arrow gets a red cross. While it is picked, a big red cross sits between the payments and the result. Hover it: "Villagers restock up to two times per day." **Screenshot 37.**
- **D1-5.** Put payments in the slots and walk 8 blocks away with the screen open: it closes and the payments are back in your inventory. Escape does the same.
- **D1-6.** Hit the villager once with your fist and right-click it again: each first payment shows the old price struck through and the higher one next to it. **Screenshot 38.** `/summon minecraft:villager ~ ~ ~2 {Age:-24000}` and right-click the baby: it shakes its head and no screen opens.
- **D1-7. Nine offers.** `/summon minecraft:villager ~ ~ ~2 {CustomName:"Bob",Xp:249,VillagerData:{type:"minecraft:plains",profession:"minecraft:librarian",level:1}}` and fence it in. Make four trades (paper, or emeralds for bookshelves), closing and reopening the screen if it does not level up at once. It reaches "Bob - Master", the bar is gone, and it has 9 offers. The scroller right of the buttons is lit: the mouse wheel and dragging both scroll two steps. **Screenshot 39.**

### Hero of the Village, effect icons and ominous bottles

- **D1-8.** `/effect give @s minecraft:hero_of_the_village 600 0` and right-click the farmer: every price is lower, the old one struck through. `/effect give @s minecraft:hero_of_the_village 600 4`: lower still. **Screenshot 40.** Drink a milk bucket: the next screen has the normal prices.
- **D1-9. Effect icons.** Eat a golden apple: Regeneration and Absorption icons appear at the top right in one row, each in a grey frame. Regeneration blinks and fades in its last seconds. `/effect give @s minecraft:darkness 30`, `/effect give @s minecraft:trial_omen 30` and `/effect give @s minecraft:raid_omen 30`: each shows its own icon in a second row. **Screenshot 41.** Open the inventory: the icons stay at the top right.
- **D1-10. Ominous bottles.** First walk at least 60 blocks away from the pen, the composter and any village: Bad Omen inside a village starts a raid.
  - Hold the plain ominous bottle and keep right-click pressed. It drinks for 1.6 seconds, then an eerie omen sound and a bottle-break sound play, and the bottle is gone with no glass bottle back. The Bad Omen icon joins the second row. **Screenshot 42.**
  - `/effect clear @s minecraft:bad_omen`: removed. Drink the level V bottle (`amplifier=4`), then `/effect give @s minecraft:bad_omen 10 0`: it can't be applied, because a stronger one is running.
  - Drink the second milk bucket: every icon is gone.

### Zombie villagers and the cure

- **D1-11. Infection.** Build a closed box of planks with a roof, about 5 x 5 inside, and go in. `/difficulty hard`, `/time set night`, `/gamemode creative` (so the zombie ignores you).
  - `/summon minecraft:villager ~ ~ ~2 {VillagerData:{type:"minecraft:plains",profession:"minecraft:farmer",level:2},Xp:12}`, then hatch a zombie egg next to it.
  - The zombie kills the villager and a loud groan plays. A zombie villager in the farmer's straw hat and clothes stands where it died, and nothing drops. **Screenshot 43.**
  - `/gamemode survival`, kill the zombie (not the zombie villager) with the sword, then `/difficulty normal` and `/time set day`.
- **D1-12. The cure.** Right-click the zombie villager with a golden apple: nothing happens and the apple stays. Throw a splash potion of weakness at it, then right-click it with a golden apple: one apple is used, a cure sound plays and it shakes from side to side. **Screenshot 44.**
- **D1-13.** Place the iron bars and the red bed within 4 blocks of it. Save and quit, load: it still shakes. Wait 3 to 5 minutes, or `/tick sprint 6000`. It turns back into a plains farmer with a loud sound. **Screenshot 45.**
- **D1-14.** Right-click the farmer: its prices are much lower than a normal farmer's, with the old ones struck through. **Screenshot 46.**

## D2. Mobs and riding

World `r10d2`, single player, a fresh world.

Kit D2 (23 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:drowned_spawn_egg*8,minecraft:cat_spawn_egg*4,minecraft:ocelot_spawn_egg*2,minecraft:creeper_spawn_egg*2,minecraft:cod*64,minecraft:blue_dye*2,minecraft:chest*4,minecraft:red_bed,minecraft:horse_spawn_egg*4,minecraft:donkey_spawn_egg*2,minecraft:mule_spawn_egg*2,minecraft:llama_spawn_egg*4,minecraft:trader_llama_spawn_egg*2,minecraft:saddle*4,minecraft:iron_horse_armor,minecraft:golden_carrot*16,minecraft:wheat*32,minecraft:white_carpet*2,minecraft:oak_fence*16,minecraft:shield,minecraft:iron_sword,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

### Drowned

- **D2-1.** `/locate biome minecraft:river` and `/tp` to it. `/time set night`. Stand on the bank and summon a trident drowned in the water, about 10 blocks out (change the offset to hit water): `/summon minecraft:drowned ~ ~ ~10 {equipment:{mainhand:{id:"minecraft:trident",count:1}}}`. It raises the trident over its head and throws it about every 2 seconds. **Screenshot 47.**
- **D2-2.** Raise the shield: the tridents bounce off. Without it a hit takes about 4 hearts. Thrown tridents stick in the ground, can't be picked up and vanish after a minute.
- **D2-3.** `/summon minecraft:drowned ~ ~ ~2 {equipment:{offhand:{id:"minecraft:nautilus_shell",count:1}},drop_chances:{offhand:2.0f}}`: it holds the shell in its left hand. Kill it: it always drops the shell.
- **D2-4.** `/time set day` and hatch a few drowned eggs on the beach, two blocks from the water: within a few seconds they walk into the water.

### Cats and ocelots

On grass, by day.

- **D2-5.** Use the cat egg three times: cats in different coats, which run away when you come close. Hold raw cod and stand still about 8 blocks from one: it creeps towards you, crouched. **Screenshot 48.**
- **D2-6.** Right-click it with cod until it takes (some tries fail): it sits and wears a red collar. **Screenshot 49.**
  - Right-click it with an empty hand: it stands up and follows you. Walk 15 blocks away fast: it appears next to you.
  - Right-click it with blue dye: the collar turns blue.
  - Use a creeper egg 3 blocks from the cat: the creeper runs away from it.
  - Place a chest next to the standing cat: it walks onto the chest and sits.
- **D2-7.** Use the ocelot egg: a spotted ocelot, which runs from you. Hold cod and stand still; when it creeps within 3 blocks, right-click it with cod until it stops running from you.
- **D2-8. Optional, the morning gift.** At night place the bed and sleep with the tame cat standing near: it lies at the foot of the bed, and in the morning it may have left a gift on the floor.

### Mules and llamas

- **D2-9.** `/summon minecraft:horse ~ ~ ~ {Tame:1b}` and `/summon minecraft:donkey ~2 ~ ~ {Tame:1b}`. Feed each a golden carrot: soon a mule foal stands between them. **Screenshot 50.**
- **D2-10.** `/summon minecraft:mule ~ ~ ~ {Tame:1b}` and right-click it with a chest: the chest sound plays and two chests hang on its sides.
- **D2-11.** Use the llama egg a few times: creamy, white, brown and gray coats. `/summon minecraft:llama ~ ~ ~ {Tame:1b}`, right-click it with a chest, then with a white carpet: a white blanket covers its back. **Screenshot 51.** Hit a wild llama with the sword: it spits once at you (about half a heart).
- **D2-12.** Use the trader llama egg: it wears the blue trader's blanket.

### Riding and the horse inventory

- **D2-13.** Use the horse egg and right-click the horse with an empty hand: you sit on it while it runs about, then it rears and throws you off. Feed it wheat and try again until it stays calm with you on it: it is tamed. (The taming hearts are not drawn yet.)
- **D2-14.** Right-click it with a saddle: saddle and bridle appear with the saddle sound. Ride it: the reins show. Steer with W, A, S, D and the mouse. Hold space: the jump bar fills over the experience bar; let go: it jumps, higher for a fuller bar. Jump a fence with the bar full. **Screenshot 52.** Crouch to get off.
- **D2-15.** Right-click it with the iron horse armour: the armour shows on it. **Screenshot 53.**
- **D2-16.** `/summon minecraft:donkey ~2 ~ ~ {Tame:1b}`. Crouch and right-click it: its screen opens with the saddle slot and no armour slot. Shift-click a saddle: it goes on. Close, right-click it with a chest, crouch and right-click again: 15 chest slots. Shift-click cobblestone and wheat in. **Screenshot 54.**
- **D2-17.** Ride the donkey and press E: its inventory opens instead of yours. Get off, open its screen and walk 8 blocks away: it closes.
- **D2-18.** Crouch and right-click your tamed horse: saddle and armour slots. Take the armour out: it is gone from the horse. Put it back.
- **D2-19.** `/summon minecraft:llama ~ ~ ~-2 {Tame:1b,Strength:3}`, give it a chest, crouch and right-click: 9 chest slots, a carpet slot with a llama icon and no saddle slot.
- **D2-20. Saving.** Ride the horse, save and quit while riding, load: you are still on it, saddled and armoured. The donkey's chest items are in the same slots, and the cats keep their coats and collars.

## D3. Golems, leads, wandering traders and chests

World `r10d3`, single player, a fresh world. Start by day on flat grass in a temperate biome, in clear weather.

Kit D3 (18 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:iron_block*8,minecraft:carved_pumpkin*4,minecraft:jack_o_lantern*2,minecraft:snow_block*4,minecraft:shears,minecraft:zombie_spawn_egg*4,minecraft:lead*8,minecraft:cow_spawn_egg*3,minecraft:llama_spawn_egg*2,minecraft:oak_fence*8,minecraft:wandering_trader_spawn_egg*2,minecraft:emerald*64,minecraft:iron_sword,minecraft:diamond_pickaxe+efficiency=5,minecraft:torch*64,minecraft:potion@night_vision,minecraft:potion@night_vision,minecraft:cooked_beef*16
```

### Golems

- **D3-1.** Place one iron block, three across on top of it (a T) and a carved pumpkin on the middle top block. The five blocks break with their particles and an iron golem stands there. **Screenshot 55.** Hit it: it does not fight back. Use a zombie egg near it: the golem goes for the zombie.
- **D3-2.** Build a T lying on the ground (all on one layer, the jack o'lantern last): a golem stands up from it.
- **D3-3.** Stack two snow blocks and put a carved pumpkin on top: a snow golem. Watch it walk: it leaves thin snow layers behind. **Screenshot 56.**
- **D3-4.** Use a zombie egg about 6 blocks from it: it throws snowballs, the zombie is knocked back without losing health and comes after the golem. Kill the zombie.
- **D3-5.** Right-click the snow golem with shears: a carved pumpkin pops out at its head height and its face shows. **Screenshot 57.**

### Leads

- **D3-6.** Use the cow egg twice. Right-click a cow with a lead: a rope runs from its head to your right hand. Walk away: it follows about 2 blocks behind. **Screenshot 58.**
- **D3-7.** Place an oak fence and right-click it while you hold the cow: a knot appears on the post. Walk 15 blocks away: the cow stays. Save and quit, load: still tied, with knot and rope. **Screenshot 59.** Right-click the fence with an empty hand: the cow is back on your lead.
- **D3-8.** With the cow on the lead, `/tp ~ ~ ~14`: the lead snaps with its sound and a lead lies by the cow. Lead it again and use shears on it: snip, and the lead drops. A lead on a zombie does nothing.
- **D3-9.** Lead a llama and walk: a second llama from the egg falls in behind it in a caravan. **Screenshot 60.**

### Wandering traders

- **D3-10.** Use the wandering trader egg by day: a trader in a blue robe, with no llamas (vanilla's egg brings none). Right-click it: the "Wandering Trader" screen with 9 offers and no experience bar. Buy something: a small orb drops at its feet. **Screenshot 61.**
- **D3-11.** `/time set night`: within a few seconds it drinks a potion and vanishes; only the bottle shows while it drinks. **Screenshot 62.** `/time set day`: it drinks milk and reappears.
- **D3-12. Optional, a natural visit.** `/tick sprint 30d`. A trader arrives within 48 blocks with two trader llamas on leads to it. Hit the trader: the llamas spit at you. It is random; sprint again if none came.

### Chests

- **D3-13.** Drink a night vision potion. `/locate structure minecraft:ancient_city` and `/tp` there. Open chests in different rooms: worn diamond hoes or leggings (their bars part used), enchanted books, a strong regeneration potion. **Screenshot 63.** Leave the sculk shriekers alone; they do nothing yet.

## D4. Pillagers, illagers and raids

World `r10d4`, single player, a fresh world. Put on the diamond armour first. Your MW2 guns kill all of these mobs too.

Kit D4 (24 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:pillager_spawn_egg*16,minecraft:vindicator_spawn_egg*8,minecraft:evoker_spawn_egg*8,minecraft:ravager_spawn_egg*4,minecraft:villager_spawn_egg*8,minecraft:iron_golem_spawn_egg*2,minecraft:sheep_spawn_egg*4,minecraft:cow_spawn_egg*4,minecraft:ominous_bottle*4,minecraft:milk_bucket,minecraft:bell,minecraft:lever*2,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:oak_leaves*64
```

### Pillagers

- **D4-1.** `/locate structure minecraft:pillager_outpost` and `/tp` to about 40 blocks from the tower. Within a minute pillagers stand about, each holding a crossbow up in both hands. **Screenshot 64.**
- **D4-2.** Walk to about 15 blocks: a pillager turns, pulls the string back with two clicks, waits a second or two and shoots. The shield blocks the arrows. **Screenshot 65.** Back away beyond 8 blocks: they follow and stop again within 8 to shoot.
- **D4-3. Captains.** In creative, on open ground away from the outpost, use pillager eggs one after another: about one in 17 is a captain with a banner standing up from its head (plain white: banner patterns are not drawn yet). **Screenshot 66.** `/gamemode survival` and kill it: it drops the "Ominous Banner" and an ominous bottle. If none of the 16 was a captain, `/give @s minecraft:pillager_spawn_egg 16`.

### Vindicators and evokers

- **D4-4.** Use a vindicator egg: it stands with crossed arms and no axe in sight. Stay close: it runs at you, raises the iron axe and strikes hard. A blocked hit knocks the shield out of use for 5 seconds. **Screenshot 67.**
- **D4-5. Johnny.** Put down three cow eggs and a pillager egg, then `/summon minecraft:vindicator ~ ~ ~4 {CustomName:"Johnny",equipment:{mainhand:{id:"minecraft:iron_axe",count:1}}}`. Johnny attacks the cows one after another and leaves the pillager alone. **Screenshot 68.**
- **D4-6.** In creative, use an evoker egg, `/gamemode survival` and walk to about 10 blocks. It backs away, raises its arms with grey-blue swirls and three vexes with swords appear above it. **Screenshot 69.** A few seconds later it raises them with dark swirls: a line of fangs bursts out of the ground towards you, snaps shut and sinks back. **Screenshot 70.**
  - A bite takes 3 hearts through diamond armour, and the shield does not stop it.
  - Within 3 blocks of it the fangs come as a ring around it.
  - Build a wall of leaves and stand behind it: the vexes fly through it.
- **D4-7.** Kill the evoker: it always drops a totem of undying. Its vexes fight on, and after 30 seconds to 2 minutes they lose half a heart a second until they die.
- **D4-8. Wololo.** In creative (so it has no target), use an evoker egg, then `/summon minecraft:sheep ~5 ~ ~ {Color:11b}` and a white sheep from the egg. The evoker raises its arms with orange swirls and the blue sheep turns red; the white one stays white. **Screenshot 71.** `/gamemode survival`.

### Ravagers

- **D4-9.** In creative, use a ravager egg: a big grey horned beast. **Screenshot 72.** `/gamemode survival` and walk to about 10 blocks. It runs at you, the neck lunges and the jaw snaps. A bite takes about 2 hearts through diamond armour and throws you forward, then it stands still for half a second.
- **D4-10.** Put a cow egg beside it, raise the shield and block its bites. A blocked bite either throws you far away, or stuns it: the stunned sound, its head hangs and sways under grey swirls for 2 seconds, then it roars with the jaw wide open and a ring of puffs. Stand within 4 blocks for the roar: you take a hit with the usual small knockback, not a long throw. The cow is hurt and thrown away. **Screenshot 73.**
- **D4-11.** Build a wall of oak leaves three high between you and it: it walks through, breaking the leaves.
- **D4-12.** `/summon minecraft:ravager ~ ~ ~6 {Passengers:[{id:"minecraft:pillager",equipment:{mainhand:{id:"minecraft:crossbow",count:1}}}]}`: the pillager sits on its back and shoots while the ravager runs at you. **Screenshot 74.** Kill the ravager: it drops a saddle, and the pillager fights on.

### Bells

By day in a plains village with villagers.

- **D4-13.** `/locate structure minecraft:village_plains` and `/tp` there. Right-click the village bell on a side, below its rim (or place the kit's bell). It rings, and every villager within 32 blocks runs into a house with a bed and stays there. About 15 seconds after the last ring they come out again. **Screenshot 75.** Right-clicking the top of the bell does not ring it.
- **D4-14.** Place a lever next to the bell and switch it on: the bell rings.
- **D4-15.** In creative, hatch two pillagers about 20 blocks from the bell, one behind a house. Ring the bell: a humming sound follows, and two seconds later both glow for 3 seconds: the one in the open with a white rim, the hidden one as a white shape seen through the house. **Screenshot 76.** `/gamemode survival` and kill them.

### The raid

On Normal: five waves. Allow about 20 minutes.

- **D4-16.** Walk at least 60 blocks out of the village and drink an ominous bottle: the Bad Omen icon. Walk back among the houses: Bad Omen turns into Raid Omen with its sound. **Screenshot 77.**
- **D4-17.** Wait 30 seconds. Raid Omen ends and a red bar notched in ten, "Raid", appears at the top and fills over 15 seconds. Villagers gather at the bell and now and then ring it. The raid horn sounds from the side the raiders come from and the first wave arrives: 4 or 5 pillagers, one wearing the banner. The villagers run into the houses. **Screenshot 78.**
- **D4-18.** The raiders go from house to house and shoot villagers, golems and you. The bar empties as you hurt them, and with two left it reads "Raid - Raiders Remaining: 2". **Screenshot 79.** The captain drops the banner and an ominous bottle.
- **D4-19.** Each later wave comes 15 seconds after the last raider falls, with the horn and a refilled bar. Wave 2 brings vindicators, wave 3 a ravager, wave 4 witches, wave 5 an evoker and a ravager with a pillager on its back. In wave 4, hurt a raider near a witch and step back: the witch throws a pink potion at it. **Screenshot 80.**
- **D4-20.** Between two waves, save and quit, then load: the bar comes back and the raid goes on.
- **D4-21. Victory.** Two seconds after the last raider falls the bar reads "Raid - Victory" for 30 seconds, and the Hero of the Village icon shows. **Screenshot 81.**
- **D4-22.** The villagers step out under the open sky, cheer and launch firework rockets: a whoosh, a white trail, then a burst of sparks of one colour with a flash and a bang. **Screenshot 82.**
- **D4-23.** Stay near a farmer: within about 30 seconds it walks up and throws you bread, pumpkin pie or a cookie. **Screenshot 83.**
- **D4-24. Optional, a lost raid.** In another village start a raid and `/kill @e[type=minecraft:villager]` after the first wave arrives: the bar reads "Raid - Defeat" and the raiders celebrate, arms up, cheering and jumping.
- **D4-25. Optional, patrols.** `/time add 120000`. Stand at least 60 blocks from any village, `/time set day`, save and quit, load, then `/tick sprint 1d` a few times. A patrol of two to four pillagers appears 24 to 47 blocks away, the captain in front.

## MP. The friend's side (two windows)

World: two windows, `-World r10mp -Fresh`. Both windows use the same class (the script copies the settings).

Kit MP, for both players (24 stacks):

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:usp,iw4l:fmj,iw4l:frag_grenade*16,iw4l:semtex*16,minecraft:chest*3,minecraft:crafting_table,minecraft:emerald*64,minecraft:emerald*64,minecraft:wheat*64,minecraft:villager_spawn_egg*4,minecraft:composter,minecraft:oak_fence*32,minecraft:diamond*64,minecraft:saddle*2,minecraft:horse_spawn_egg*2,minecraft:golden_carrot*16,minecraft:ominous_bottle*2,minecraft:milk_bucket,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_chestplate,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

### Guns and grenades for the friend

- **MP-1 (two windows).** The friend's first join gives the friend's own loadout: the M4A1 and USP .45 on slots 1 and 2, the ammo in the backpack, 32 frags and 32 flashes in the friend's frames. The host's inventory is unchanged. **Screenshot 84** (friend window, inventory open).
- **MP-2 (two windows).** In the friend's inventory the guns show their icons, tooltips ("Red Dot Sight", "Clip 30/30") and clip bars, and the two grenade slots sit right of the book button.
- **MP-3 (two windows).** The friend fires 12 rounds and reloads: the friend's rifle ammo drops, the host's does not. **Screenshot 85**, both windows.
- **MP-4 (two windows).** The friend presses G and Shift+G: the friend's frames show 31 and 31, the host's still 32. The host sees the friend's grenades fly.
- **MP-5 (two windows).** The friend places a chest and puts the kit M4A1 in. The host takes it out and fires it; the friend can't raise it any more. The friend drops the kit USP with Q: the host sees its kill icon on the ground and picks it up. **Screenshot 86** (host window).
- **MP-6 (two windows). Creative guns.** Host: `/gamemode creative Friend`. The friend opens the creative screen: the fork items are at the end of the Combat tab, and the M4A1 shows "Clip 30/30". The friend takes it to the hotbar, closes and fires it: the clip empties and reloads with no ammo used.
  - The friend's inventory tab has the two grenade slots right of the leggings and boots. The friend takes the flashes out and puts them back.

  **Screenshot 87** (friend window). Host: `/gamemode survival Friend`.

### Trading and horses for the friend

- **MP-7 (two windows).** The host places the composter in a fenced pen and hatches a villager egg next to it; wait until it is a farmer. The friend right-clicks it: the same vanilla screen, "Farmer - Novice". The friend picks an offer, takes the result and shift-clicks it: the friend's items change and the bar fills. **Screenshot 88** (friend window).
- **MP-8 (two windows).** While the friend's screen is open, the host right-clicks the same villager: nothing opens. The friend presses Escape, and now the host's screen opens and shows the uses the friend used up.
- **MP-9 (two windows).** The friend hits the villager once and opens it again: the friend's prices are higher, struck through. The host's prices stay as they were. **Screenshot 89**, both windows.
- **MP-10 (two windows).** The friend: `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`, right-clicks it with a saddle and rides it. Holding space fills the friend's own jump bar, and the horse jumps. The host sees the friend on the horse with the reins. **Screenshot 90**, both windows. The friend presses E while riding: the horse's inventory opens.
- **MP-11 (two windows). One player per horse inventory.** Host: `/summon minecraft:donkey ~ ~ ~2 {Tame:1b,ChestedHorse:1b}`, crouch and right-click it, put the 64 diamonds into its chest and leave the screen open. **Screenshot 91** (host window). The friend crouches and right-clicks the same donkey: no screen opens. The host closes the screen. The friend opens it now and takes the diamonds. The host opens it after the friend closed it: the chest is empty, and the diamonds exist only once, with the friend. **Screenshot 92**, both windows.

### A raid with the friend

- **MP-12 (two windows).** Both `/tp` to a plains village (`/locate structure minecraft:village_plains`). Host: `/difficulty easy` (three waves). The host walks 60 blocks out, drinks an ominous bottle and walks back into the village; the friend waits in the village. 30 seconds after the Raid Omen the red "Raid" bar shows in both windows, and both hear the horn. **Screenshot 93**, both windows.
- **MP-13 (two windows).** Both kill raiders with guns. The friend walks more than 100 blocks away: the friend's bar goes. The friend comes back: the bar is back.
- **MP-14 (two windows).** After the victory, each player who killed a raider has the Hero of the Village icon. **Screenshot 94** (friend window). Host: `/difficulty normal`.
- Quit the friend first, then the host, through the menu.

## Known limits

These are known gaps, not new bugs:

- **Explosions push no player**, MW2 or Minecraft, totem or not. Only wind charges and the dragon push players.
- **Guns:** MW2 drops no gun model on death; only the Minecraft items drop. Killstreak items vanish on death and can't go into chests or the offhand. Two copies of one gun switch faster than a reload (accepted). Guns don't wear armour, and gun damage to mobs is unchanged.
- **Crafting:** the attach, akimbo and split recipes are not in the recipe book (the tooltips say how). The grindstone loses the attachments and the rounds above the bare clip. The book has no furnace version, no keyboard navigation and no bounce for new recipes. A crafted suspicious stew carries its effect, but eating it applies none yet.
- **Grenades:** only the slots throw; a grenade held in the hand does nothing. Placed claymores and C4 vanish when the host quits. A world saved by vanilla loses the grenade slots.
- **Tactical insertion:** it leaves no marker on the ground, is not saved, and only MW2 blasts destroy it (TNT and creepers don't).
- **Particles:** taming hearts, bucking smoke, ocelot trust hearts, the villagers' sweat in raids and the llama spit puff are not drawn yet.
- **Trading:** number keys, Q, double-click and drags on the payment and result slots do nothing.
- **Effects:** the inventory screens don't list the effects beside them, so the HUD icons stay. `/effect give` of an omen plays no sound. The ominous bottle's tooltip has no "Bad Omen" line.
- **Horses:** the mount is not drawn in its inventory screen, only one player can open a mount's inventory at a time, the rider takes no fall damage, there is no gallop sound and no vehicle hearts. Leather horse armour is undyed.
- **Leads:** a knot can't be punched off; break the fence and the mob is free within 5 seconds.
- **Cats:** a cat on a chest does not keep it shut. Phantoms don't exist yet.
- **Drowned:** a raised trident uses the plain held model. Zombie nautilus jockeys and the ocean ruin drowned's trident roll are missing.
- **Golems:** a dispenser does not finish a golem with a pumpkin, and copper golems don't exist yet.
- **Chests:** abandoned camps, shipwreck map chests and ocean ruins still roll nothing (explorer maps are missing). Sculk shriekers, sensors and catalysts do nothing yet.
- **Illagers:** banners show plain white (no patterns), a held crossbow shows no arrow or pull, and Johnny comes only from `/summon` (no name tags). Evoker fangs give no knockback, and vexes ignore water and lava.
- **No door breaking:** vindicators in raids and zombies on Hard don't break or open doors.
- **Raids:** a ravager walks to the village itself even with a rider, raiders don't pick up banners, and there are no raid omen particles.
- **Bells** don't swing, and arrows and wind charges don't ring them. The glow is a white hull, not vanilla's outline.
- **Fireworks:** only celebrating villagers launch rockets. A rocket does not burst on a mob in its path, and mobs hurt by a burst are not knocked back.
- **Thrown things:** a player's tridents, snowballs, eggs and splash potions can't hit pillagers, vindicators, evokers, vexes, ravagers, hoglins, piglins, striders, ghasts or blazes. Arrows and guns can.
- **Vanilla quirks kept:** the wandering trader's egg brings no llamas, a llama can't be steered, and patrols drift towards the world's origin.
- **Friends:** a friend's jump bar and effect blink lag the host by the round trip.
- The limits at the end of [release-1-tests.md](release-1-tests.md) still apply, except "trade": friends trade now.

## Screenshot numbering

- Name each screenshot by its number: `14.png`. When a step asks for both windows, name them `3-host.png` and `3-friend.png`.
- When a step fails, take a screenshot even if the step doesn't ask for one. Name it by its step label, e.g. `C1-7.png` or `MP-11-friend.png`. Say in one line what you expected and what you saw.
- Send `latest.log` after any crash. In two windows, send both windows' logs (see Setup for where they are).
