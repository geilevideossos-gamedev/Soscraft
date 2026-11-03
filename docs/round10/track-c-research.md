# Round 10 track C research (unfinished)

**Unfinished.** This is the research of the planning run for track C (the missing container screens: dispensers, droppers, hoppers, beacons, looms, cartography tables and anything else the inventory finds), stopped for the week on 2026-11-03 at about 05:10 while its writer was starting. The brief `docs/round10/track-c.md` and the run specs `docs/round10/runs/c*.json` are not written yet. The four reports below were written by the run's research agents at commit 20f1ce4e and are kept as they returned them; their anchors need a re-check against the current `minecraft-survival`. How to resume: `docs/PROGRESS.md`, "Where it stands".

## 1. Inventory: the vanilla 26.3 menus and how far the fork has each

## Track C inventory: vanilla 26.3 container screens and how far the fork has each

All anchors were read at commit **20f1ce4e** (HEAD of `minecraft-survival`). The working tree changes only docs. `docs/round10/release-7-tests.md` is untracked, so its anchors come from the working tree. Nothing was built, run or committed.

**Sources**
- The jar is `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3\client.jar`, the folder `scripts/cargo-slot.ps1:89-90` uses for `MINECRAFTOSS_ROOT`.
- javap is `C:\Program Files\Eclipse Adoptium\jdk-21.0.8.9-hotspot\bin\javap.exe`.
- I scanned every class in the jar for references to each menu class and each `ChestMenu` row factory. The results are in `C:\Users\tsupp\AppData\Local\Temp\claude\C--Dev-Soscraft-mashup-crafting\0db10490-5b69-4a4e-ab4e-d60da7dcb4d8\scratchpad\track-c-brief\refs.txt` and `...\track-c-brief\refs2.txt`.

### 0. Where the pieces live today

**Opening a screen** (`crates/minecraft_survival/src/actions.rs`):
- The crafting table is at :709-718.
- Containers come from `Opened::new`, through `Station::of`, at :719-731.
- Benches come from `Workshop::of` at :732-744.
- Campfires are at :745-752, lecterns at :753-770 and vaults at :771-779.
- Lid and cat checks, then `open_block`, are at :880-923.
- Anything else falls through to `entities.use_block` at :841-864, which reaches `server.rs:3060` and then the level's `use_block_facing`.
- The menus are opened in `menus::open_station` (`crates/minecraft_survival/src/menus.rs:281-365`). The host calls it at `render_anim/src/minecraft_world.rs:1373` and the friends' path at `render_anim/src/minecraft_players.rs:1847`.

**The menu model** (`menus.rs`):
- `enum Open` :45-52 and `still_valid` :394-409.
- `station_title` :247-259 and `bench_title` :261-269.
- `contents` :1410-1440, `kind_of` :1467-1492, `mirror_open` :1938-1983 and `mirror` :2089-2111.
- The friend snapshot and sync functions are `snapshot` :1629, `sync` :1695 and `changes` :1745.
- `Station` (Chest, Barrel, Oven, Brewer, Ender, Shulker) is at `crates/minecraft_terrain/src/containers.rs:46-74`.
- `Workshop` (Stonecutter, Smithing, Grindstone, Anvil, Enchanting) is at `crates/minecraft_terrain/src/stations.rs:23-44`.

**The wire** (`crates/minecraft_net/src/messages.rs`):
- `PROTOCOL = 12` is at :7. The codec is postcard (`docs/MULTIPLAYER.md:263-264`), so any new enum variant needs a protocol bump.
- `ToServer` messages: `Click` :45, `RenameItem` :46, `CloseMenu` :47-49, `Lectern` :52-55, `EditBook` :57-61 and `OpenMountInventory` :71.
- `MenuAction` is at :180-210 (`Choose(u16)` :205, `PlaceRecipe` :206).
- `ToClient` messages: `MenuOpen` :271, `MenuSlots` :272-278, `MenuClose` :279-281, `Lectern` :318 and `OpenBook` :324-326.
- `MenuKind` is at :567-590. `Loom`, `Cartography`, `Hopper`, `Dispenser`, `Dropper` and `Beacon` already exist there, but there is **no `Crafter`**.
- `MenuExtras` is at :592-621 and `MenuView` at :638-647. `MenuView.data` is a `Vec<(u8,i32)>`.

**The HUD:**
- `McScreen` is at `crates/frame/src/minecraft_ui.rs:69-95`, and `MinecraftUi.data` is `[i32; 4]` at :206.
- In `crates/hud/src/minecraft_screens.rs` the slot layouts are `layout()` :122-286, the textures come from `sheet()` :313-333, and the drawing is `draw_background` :493-535 and `draw_labels` :846-867. These are called from `hud/src/minecraft_inventory.rs:656-675`.
- The bridge from `Open` to `McScreen` is in `render_anim/src/minecraft_inventory.rs`: `screen_for` :785-794, `bench_screen` :1008-1016 and `screen_of` :1018-1027.
- The jar textures that get loaded are listed in `render_anim/src/minecraft_gui.rs`: `CONTAINER_SHEETS` :19-38 and `SPRITE_FOLDERS` :40-59. The dispenser, hopper, crafter, loom, cartography table and beacon textures are not in either list.

**Two places own items:**
1. **`Containers`**, on the main thread, owns chests, trapped chests, copper chests, barrels, shulker boxes, ovens, brewing stands and campfires (`containers.rs:40`, `:135-137`, `:603-613`). These are saved by `level.rs:233-238`.
2. **The level server thread** owns hoppers, dispensers, droppers and lecterns, inside the block entity tag (`third_party/minecraftoss/world/src/level/container.rs:1-10` and `:20-38`). These are handed over at `server.rs:4141-4161` and saved by `level.rs:240-253`.

The level's copy of a chunk has the `Items` of every container `Containers` owns stripped out (`server.rs:915-919`, `containers.rs:139-146`). So a hopper or dropper sees a player's chest as empty, and whatever it pushes into one goes into a copy that is never handed back. The test at `server.rs:10949` checks exactly this ("the chest's items stay the containers'"), and `docs/MINECRAFT-SURVIVAL.md:372` lists it as a limitation.

### 1. Every 26.3 MenuType and who opens it

`MenuType` has 25 fields (javap). Two more container screens are not MenuTypes: the player inventory (`InventoryMenu`, id 0) and the mount screens (`HorseInventoryMenu`, `NautilusInventoryMenu`, opened by `ClientboundMountScreenOpenPacket`).

| MenuType (class) | Opened by (from the jar scan) | Open / MenuKind | HUD screen (McScreen, sheet) | Contents live in | Friend path | Saved | Status |
|---|---|---|---|---|---|---|---|
| CRAFTING (`CraftingMenu`) | crafting_table | `Open::Table` / `CraftingTable` (menus.rs:1470, :1961) | `CraftingTable`, crafting_table (screens.rs:164-175) | `inventory.workbench` (menus.rs:1417) | mirrored; Click and PlaceRecipe | nothing (the grid goes back on close) | complete, with recipe book |
| GENERIC_9x3 (`ChestMenu.threeRows`) | chest, trapped_chest, 8 copper chests (4 stages, plain and waxed), barrel, ender_chest; also 11 chest boats/rafts and chest_minecart | `Open::Block` with Chest/Barrel/Ender / `Chest{rows:3}`, `EnderChest` (menus.rs:1476-1480, :1962-1963) | `Chest{rows}`, generic_54 (:176-183, :506-510) | `Containers.chests` (containers.rs:1060-1090); ender items in `Player.ender` | mirrored; shared viewers (test menus.rs:2221) | `block_entity_tag` (containers.rs:797-818); `EnderItems` (player_nbt.rs:158, :248, :345) | complete for blocks; the boat and minecart **entities don't exist** (MINECRAFT-SURVIVAL.md:480) |
| GENERIC_9x6 (`sixRows`) | double chest, trapped chest or copper chest | `Opened` with 2 cells (containers.rs:300-316) / `Chest{rows:6}` | as above | as above | as above | as above | complete |
| GENERIC_9x1, 9x2, 9x4, 9x5 | **no vanilla user**: only `ChestMenu` and `MenuType` reference the factories | `Chest{rows}` can carry any row count; `mirror_open` rounds to whole 27-slot cells (menus.rs:1962) | layout draws any row count | none | none | none | nothing needed |
| SHULKER_BOX (`ShulkerBoxMenu`) | 17 shulker boxes | `Station::Shulker` / `ShulkerBox` | `ShulkerBox`, shulker_box (:184-190) | `Containers`, no shulker in a shulker (containers.rs:1451); lid check (actions.rs:900-915) | mirrored | block entity (:801), item `container` component (`take_shulker` :930-944, `place_shulker` :913-928) | complete |
| FURNACE, SMOKER, BLAST_FURNACE | furnace, smoker, blast_furnace | `Station::Oven` / `Furnace`, `Smoker`, `BlastFurnace` | `Furnace(McOven)` (:191-194, :552) | `Containers.ovens`, ticked at :1019-1029 | data slots `oven_data` (menus.rs:1403) | `Oven` entries, `RecipesUsed` | complete; a hopper can't feed them (furnaces are not a level store, container.rs:9) |
| BREWING_STAND | brewing_stand | `Station::Brewer` / `BrewingStand` | `Brewing` (:195-208, :588) | `Containers.brewers` | mirrored | yes | complete (same hopper note) |
| ENCHANTMENT | enchanting_table | `Bench(Enchanting)`; shelves counted at menus.rs:340-355 | `Enchanting` (:209-216, :639) | `Bench` | `MenuExtras::Enchanting{seed,shelves}` | the seed is not saved (MINECRAFT-SURVIVAL.md:399) | complete; no 3D book (:417), the slot takes tools, armour and books only (:464) |
| ANVIL | anvil, chipped_anvil, damaged_anvil | `Bench(Anvil)` | `Anvil` (:217-220, :731) | `Bench` | `RenameItem`, `MenuExtras::Anvil` | nothing | complete; the name field edits only at the end (:464) |
| GRINDSTONE, SMITHING, STONECUTTER | grindstone, smithing_table, stonecutter | `Bench(...)` | :221-237, :770, :826 | `Bench` | mirrored; `Choose(u16)` for the stonecutter | nothing | complete; no smithing armour stand preview (:417) |
| MERCHANT | villager, wandering_trader | `Open::Merchant` / `Merchant` | `Merchant`, villager sheet (hud/minecraft_merchant.rs) | server thread: `merchant_view` server.rs:696-729, `Command::Merchant` :4385 | `MenuExtras::Merchant` with offers; `mirror_merchant` menus.rs:1583 | offers saved with the villager | complete (MINECRAFT-SURVIVAL.md:392 still says trading isn't wired, which is out of date) |
| LECTERN (`LecternMenu`) | lectern holding a book | not a menu here: `LecternAction` (actions.rs:924-953), server.rs:2161-2246 | `McScreen::Lectern`, book sheet (screens.rs:239, :331) | the level's lectern | `ToClient::Lectern(Option<BookView>)` and `ToServer::Lectern{pos,button}` (MULTIPLAYER.md:251) | `Book` and `Page` saved by the level (MINECRAFT-SURVIVAL.md:364) | complete (pages show as plain text, :456) |
| GENERIC_3x3 (`DispenserMenu`) | dispenser, dropper | **partly done**, see 2A | | | | | partly done |
| HOPPER (`HopperMenu`) | hopper; hopper_minecart (entity missing) | **partly done**, see 2B | | | | | partly done |
| CRAFTER_3x3 (`CrafterMenu`) | crafter | see 2C | | | | | missing |
| LOOM | loom | see 2D | | | | | missing |
| CARTOGRAPHY_TABLE | cartography_table | see 2E | | | | | missing |
| BEACON | beacon | see 2F | | | | | missing |
| (`InventoryMenu`) | the E key | `Open::Inventory` | `Inventory` | the player | mirrored | the player file | complete |
| (`HorseInventoryMenu`) | horse, donkey, mule, llama, trader_llama (live, `HorseKind` in third_party/minecraftoss/entities/src/horse.rs:11-16); camel, camel_husk, skeleton_horse, zombie_horse | `Open::Mount` / `Mount`, `MenuExtras::Mount` | `Mount{..}`, horse sheet (:240-271, :537) | the entity world (`mount_inventory.rs:31-59`), written back with `Command::MountInventory` (server.rs:4405) | `OpenMountInventory`, `mirror_mount` menus.rs:1985 | saved with the entity | complete for the 5 live kinds; camels, husks and skeleton and zombie horses are **not live mobs** (dormant.rs:4-12, MINECRAFT-SURVIVAL.md:471) |
| (`NautilusInventoryMenu`) | nautilus, zombie_nautilus | none | none (the jar has container/nautilus.png) | the entities are not live | none | none | missing; the entity has to come first |

### 2. The missing and partial screens in detail

These are the slot and data facts from `javap -constants`, used below:

| Menu | Slots | Data |
|---|---|---|
| Dispenser | 9; the inventory starts at 9 | none |
| Hopper | `CONTAINER_SIZE` = 5 | none |
| Crafter | 9 slots plus a result | `CrafterBlockEntity.NUM_DATA` = 10: nine slot states and `DATA_TRIGGERED` = 9 |
| Loom | banner, dye, pattern and result; the inventory starts at 4 | `PATTERN_NOT_SET` = -1 |
| Cartography table | `MAP_SLOT` 0, `ADDITIONAL_SLOT` 1, `RESULT_SLOT` 2 | none |
| Beacon | `PAYMENT_SLOT` 0 | `DATA_COUNT` = 3 |

The jar has the sheets `container/{dispenser,hopper,crafter,loom,cartography_table,beacon}.png` and the sprite folders `sprites/container/{crafter,loom,cartography_table,beacon}/`. The title keys in `en_us.json` are `container.dispenser`, `container.dropper`, `container.hopper` ("Item Hopper"), `container.crafter`, `container.loom`, `container.cartography_table` and `container.beacon`.

**A. GENERIC_3x3: dispenser and dropper. Partly done.**
- **Open and kind:**
  - `MenuKind::Dispenser` and `Dropper` exist (messages.rs:585-586), but there is no `Station` or `Open` arm (containers.rs:56-74, menus.rs:1467-1492), and `mirror_open` returns `None` (menus.rs:1979-1980).
  - Right-clicking one opens nothing. The level's `handles_use` does not cover it (`world/src/level/redstone.rs:999-1006`), so the click carries on down the chain in actions.rs. With a block in hand, that block is placed against the dispenser (actions.rs:1436ff), where vanilla would open the screen.
- **HUD:** there is no `McScreen` variant and no sheet for it.
- **Contents:**
  - The level owns them (`Store::Dispenser`, 9 slots, container.rs:22-37).
  - Dispensing works (`dispenser.rs:73-139`): plain items, water and lava buckets, mob buckets, empty buckets, honeycomb, rockets and bone meal. The other behaviours are only recorded as unsupported (`dispenser.rs:17-27`).
  - Redstone triggers it (redstone.rs:112-113, :678, :839).
  - Nothing in game can load one. `replace_block_item` (container.rs:181-188) is only called from tests (server.rs:10943, :17045), and `/setblock` takes no NBT (server.rs:2932-2954).
  - Jungle temple dispensers carry a `LootTable` (generator `jungle_temple.rs:206`, `:220`), which is never unpacked (container.rs:10).
  - A dropper that pushes into a chest, barrel or shulker box writes into the level's stripped copy, so those items are lost.
- **Friend path:** the `MenuKind` is there, but the friend's game has no mirror arm for it.
- **Saving:** done. The level hands the changed tag to the main thread (server.rs:4141-4161, then level.rs:240-253). A broken dropper hands over `None` (test server.rs:10968-10988), and the contents drop on break (`drops.rs:131-146`).
- **What's missing:** the opening arm, the screen, the mirror arm, and a way for the menu to read and write items the level owns. The mount (`Command::MountInventory`) and the merchant (`Command::Merchant` and its replies) already do this for things the server thread owns, so either can be the model.
- **Owner impact:** release 7's known limits cite this screen at `release-7-tests.md:619`, `:626` and `:634`, and `docs/PROGRESS.md:101` says the dispenser steps were dropped.

**B. HOPPER: hopper. Partly done.**
- **Open and kind:** `MenuKind::Hopper` exists (:584); the mirror returns `None` (menus.rs:1978); there is no `Station` arm, no `McScreen` and no sheet.
- **Contents:**
  - The level owns them (`Store::Hopper`, 5 slots). Ticking, ejecting, pulling and the `enabled` state work (container.rs:320-412, :470-477), and `TransferCooldown` is saved.
  - Hoppers don't pick up item entities (container.rs:9-10, no code for it).
  - They can't feed furnaces or brewing stands, which are not a level store (container.rs:79-95).
  - With a player's chest, barrel or shulker box they read empty and swallow what they push into it (section 0).
- **Friend path:** none.
- **Saving:** done.
- **Warning:** a hopper screen makes that item loss easy to reach. It needs the "one container truth" fix (MINECRAFT-SURVIVAL.md:372) in the same track or before it.

**C. CRAFTER_3x3: crafter. Missing.**
- **Open and kind:** there is no `MenuKind` (messages.rs:567-590). Adding one changes the protocol (`PROTOCOL` at :7). There is no `Station` or `Open` arm.
- **Slot toggles:** vanilla sends `ServerboundContainerSlotStateChangedPacket(slotId, containerId, newState)`. The fork has no such message (`ToServer` :33-73, `MenuAction` :180-210).
- **Data:** ten values (nine slot states and `triggered`) don't fit `Screen.data` `[i32;4]` (menus.rs:224) or `MinecraftUi.data` `[i32;4]` (frame/minecraft_ui.rs:206). `MenuView.data` can carry them.
- **Server:** nothing runs.
  - The block entity is only normalised on load and save (`third_party/minecraftoss/core/src/block_entity.rs:119-143`: `Items` of 9, `disabled_slots`, `triggered`, `crafting_ticks_remaining`).
  - It is not a level store (container.rs:9), and the redstone kind table has no `CrafterBlock` (redstone.rs:108-113). So `triggered` and `crafting` never change, a pulse crafts nothing and comparators read nothing.
  - Breaking one loses its items: drops.rs:138-142 records it as unsupported and drops nothing.
- **HUD:** none. The jar has `crafter.png` and the sprites `disabled_slot`, `powered_redstone` and `unpowered_redstone`.
- **Saving:** the tag passes through unchanged.

**D. LOOM. Missing.**
- **Open and kind:** `MenuKind::Loom` exists, but there is no `Workshop` arm (stations.rs:32-44) and the mirror returns `None` (menus.rs:1976).
- **HUD:** none. The jar has `loom.png` and the sprites `pattern`, `pattern_highlighted`, `pattern_selected`, `scroller`, `scroller_disabled` and `error`.
- **Logic:** none.
- **What already helps:**
  - Choosing a pattern can reuse `MenuAction::Choose(u16)` (vanilla uses a button click), so no new message is needed.
  - The selected index fits the 4 data slots.
  - Banners and shields already carry and draw `minecraft:banner_patterns` (kit tests at `crafting.rs:789-818`).
- **Saving:** none needed. The inputs go back to the player on close, as at the other benches (MINECRAFT-SURVIVAL.md:443).

**E. CARTOGRAPHY_TABLE. Missing.**
- **Open and kind:** `MenuKind::Cartography` exists, but there is no `Workshop` arm and the mirror returns `None` (menus.rs:1977).
- **HUD:** none. The jar has `cartography_table.png` and the sprites `map`, `scaled_map`, `duplicated_map`, `locked` and `error`.
- **Logic:** maps already store `scale` and `locked` (`maps.rs:85-122`, `:243-302`). But nothing anywhere reads `map_post_processing` and there is no map cloning (rg finds no matches), so zooming, locking and copying can't happen yet.
- **Saving:** new maps would go through the existing `data/minecraft/maps` storage.

**F. BEACON. Missing.**
- **Open and kind:** `MenuKind::Beacon` exists, but the mirror returns `None` (menus.rs:1981).
- **Simulation:** none. The only beacon code in minecraft_terrain is face culling (`face_culling.rs:417`): no pyramid levels, no beam and no effects (MINECRAFT-SURVIVAL.md:369, :441).
- **Wire:** choosing powers needs a set-beacon message (vanilla `ServerboundSetBeaconPacket(primary, secondary)`), which means a new variant and a protocol bump. The 3 data values fit.
- **HUD:** none. The jar has `beacon.png` and the sprites `button*`, `confirm` and `cancel`. Effect icons are already loaded (minecraft_gui.rs:17).
- **Saving:** the tag passes through as generated.

**G. Container entities. Missing; the entities have to come first.**
- The 11 chest boats and rafts and the chest_minecart (GENERIC_9x3), and the hopper_minecart (HOPPER), don't exist as entities in the fork (MINECRAFT-SURVIVAL.md:480).

**H. Mounts that aren't live mobs. Missing; the entities have to come first.**
- camel, camel_husk, skeleton_horse and zombie_horse would use `HorseInventoryMenu`.
- nautilus and zombie_nautilus would use `NautilusInventoryMenu`.

**Other screens that are not menus:**
- Book and quill, signing and reading written books exist (`McScreen::BookEdit`, `BookSign`; messages.rs:57-61, :324-326). MINECRAFT-SURVIVAL.md:456, which says books can't be written, is out of date.
- Sign editing (`openTextEdit`) is missing; sign text is not simulated (MINECRAFT-SURVIVAL.md:369).
- The operator screens (command block, structure, jigsaw, test blocks) are missing.

**What every new screen has to touch:**
- The exhaustive matches on `Station`: containers.rs:76-88 and :96-109, `openers.rs:124-158`, menus.rs:247-259 and :1471-1481, and render_anim minecraft_inventory.rs:1018-1027.
- The exhaustive matches on `Workshop`: stations.rs:32-44, menus.rs:261-269 and :1482-1488, and render_anim :1008-1016.
- `McScreen` and the hud `layout`, `sheet` and `draw_labels` functions.
- `CONTAINER_SHEETS` and `SPRITE_FOLDERS`.
- If a dispenser or hopper becomes a `Station`, the level already drops its contents when it breaks (drops.rs:131-146). The `Containers::remove` and `minecraft_world.rs:1245` paths must not drop them a second time.

### 3. Blocks whose use already works without a screen

| Group | Blocks | Where |
|---|---|---|
| Level `use_block_facing` | Wooden and copper doors and trapdoors (not iron ones), fence gates, note blocks (a click tunes, an attack plays), levers, buttons, repeaters (delay), comparators (mode), daylight detectors (invert) | `world/src/level/redstone.rs:999-1072`; reached through actions.rs:841-864 |
| Harvesting | Sweet berry bushes, cave vines with glow berries | `world/src/level/plants.rs:336-370` |
| Fork's own rules | Bells, rung by the face that is hit | actions.rs:827-840 |
| | Leads tied to fences | actions.rs:816-826 |
| | Campfires: raw food goes on, flint and steel lights, a shovel douses | actions.rs:971-985; MINECRAFT-SURVIVAL.md:126, :130 |
| | Putting a book on a lectern | actions.rs:931-952 |
| | Vault keys | actions.rs:954-970 |
| | Beds and respawn anchors: sleep, set spawn, explode | `use_respawn_block` actions.rs:986-1023, :1896 |
| | Dragon egg teleports | server.rs:3100 |
| | Cauldrons with buckets and bottles | MINECRAFT-SURVIVAL.md:125 |
| | Flint and steel on candles, candle cakes and TNT | MINECRAFT-SURVIVAL.md:126 |
| | Axe, shovel and hoe changes from `block_transformer` | MINECRAFT-SURVIVAL.md:127 |
| | Bone meal | |
| | Eyes of ender in end portal frames | MINECRAFT-SURVIVAL.md:246 |
| | Lodestone compasses | bucket.rs:144-156, `Change::Lodestone` |
| | Item frames (an entity) | |
| Not working yet (no screen needed) | Composter, eating cake, flower pots, jukebox, chiseled bookshelf, decorated pot and shelves, beehives, brushing, carving pumpkins | MINECRAFT-SURVIVAL.md:369 |

### 4. Missing or partial screens, in the order a player would miss them most

1. **Dispenser and dropper.** One 3x3 screen serves both. The blocks already fire, take redstone and save, and the `MenuKind` exists. Release 7's tests are blocked on it.
   - Watch that a dropper feeding a player's chest swallows the items.
   - Jungle temple dispensers with a `LootTable` stay empty until loot tables are unpacked.
2. **Hopper.** The most used survival block of these, but a screen alone does not give a working hopper. It doesn't pick up dropped items, can't feed furnaces and swallows items pushed into player chests, so it needs the "one container truth" fix with it.
3. **Loom.** Banners and their patterns already render, and `Choose` and the `MenuKind` exist. It needs the pattern list and the screen.
4. **Cartography table.** Maps already exist. It needs `map_post_processing` for zooming and locking, plus copying, and the screen.
5. **Crafter.** Everything is missing: the screen, a new `MenuKind` (protocol bump), a slot-toggle message, ten data slots, redstone crafting, comparator output, and dropping its items when broken.
6. **Beacon.** Everything is missing: the block simulation (levels, beam, effects), a set-beacon message (protocol bump) and the screen.
7. **Chest boats, chest minecarts and hopper minecarts.** The entities have to exist first.
8. **Camel, camel husk, skeleton horse and zombie horse inventories, and the nautilus and zombie nautilus inventories.** These mobs have to become live first.

GENERIC_9x1, 9x2, 9x4 and 9x5 need nothing, because no vanilla 26.3 block or entity opens them.

## 2. The fork's recipe for a container screen, end to end

## Track C planning: the fork's recipe for a container screen, end to end

**What this covers.** All anchors are read at commit **20f1ce4e** on `minecraft-survival`, as `path:line` under `C:\Dev\Soscraft\mashup-crafting`.
- **Repo state.** No code files differ from 20f1ce4e. The working tree only changes docs.
  - `docs/round10/decisions.md:51` holds the track C decision and is not committed yet.
  - `docs/round10/release-7-tests.md` is untracked.
- **Vanilla facts.** They come from JDK 21 javap (`C:\Program Files\Eclipse Adoptium\jdk-21.0.8.9-hotspot\bin\javap.exe`) on `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3\client.jar`.
  - That jar is the `MINECRAFTOSS_ROOT` default in `scripts/cargo-slot.ps1:89-91`.
  - Textures come from `...\minecraft-26.3\resourcepacks\local\minecraft-26.3\assets\minecraft\textures\gui\`.
- **What I did not do.** Nothing was built or run, no file was written, and no scratch file was made.

---

## 0. Five facts that decide the design

1. **There are two sources of truth for container items.** (`docs/MINECRAFT-SURVIVAL.md:364`, `:372`.)
   - **The fork's `Containers` store** (`crates/minecraft_terrain/src/containers.rs:602-613`) owns chests, barrels, shulker boxes, furnaces, brewing stands and campfires.
     - Ownership is decided by `owns_block_entity`, `containers.rs:135-137`.
     - The fork saves them itself through `block_entity_tag` (`:797`), `take_block_entities` (`:875`) and `level.rs:233-238`.
     - The server level gets these chunks with their `Items` removed (`leave_to_client`, `containers.rs:139-146`, called at `server.rs:915-919`).
   - **The server level thread** owns hoppers, dispensers and droppers.
     - Their `Items` live in the level's block entity tag (`third_party/minecraftoss/world/src/level/container.rs:22-38` `Store`, `:162-177`).
     - The level simulates them: hoppers tick (`:320`), dispensers fire (`level/dispenser.rs:73`), and comparators read them (`container_signal` `:295-307`, called from `level/redstone.rs:483-499`).
     - The level saves them (`server.rs:4141-4156`, `level.rs:240-253`), and the host reads the latest copy through `terrain.stream.stored_block_entity(pos)` (`terrain.rs:1296`).
   - **So hopper, dispenser and dropper screens must not use `Containers`.** Clicks have to change the level's copy, or hoppers, comparators and dispensers never see the items.
2. **The fork already has four screen models.** Each missing screen fits one of them (section 2).
   - (a) Fork-owned block containers: `Open::Block(Opened)` with `Station`.
   - (b) Per-player workbenches: `Open::Bench(pos, Bench)` with `Workshop`. Their slots are temporary and go back to the player on close.
   - (c) Server-owned with asynchronous operations: `Open::Merchant`, with `MerchantOp` and `MerchantUpdate`.
   - (d) Server-owned snapshot pushed back whole: `Open::Mount`. E3's `mount-viewers` group moves it into a shared store in `Containers`.
   - The lectern adds a server-side list of viewers that the server pushes views to (`server.rs:2161-2246`).
3. **Opening the six needs no wire change.** `MenuKind::{Loom, Cartography, Hopper, Dispenser, Dropper, Beacon}` already exist (`crates/minecraft_net/src/messages.rs:580-587`). Only `mirror_open` maps them to `None` (`crates/minecraft_survival/src/menus.rs:1976-1981`).
4. **The loom and cartography table can be new `Workshop` variants with no change to the open path.**
   - `Workshop::of` is already consulted at `actions.rs:740-743`.
   - Benches already flow through `Acted.open_bench`, `Opens`, `open_station` and both callers unchanged.
5. **E3 and M2 plan edits in the same files** (section 5). Keep track C's lines away from those spots.

---

## 1. The pipeline end to end

### 1.1 How a right-click on a block becomes an open menu

**Host and friend share one code path.**
- `crates/minecraft_survival/src/actions.rs:212` `act()` checks, in this order:
  1. Crafting table (`:709-718`).
  2. Block station: `Opened::new(pos, block, partner)` with the chest partner (`:719-731`).
  3. Workshop: `Workshop::of` (`:732-744`).
  4. Then campfire, lectern and vault (`:745-779`).
  - Each check is gated by `minecraft_terrain::crafting::opens_table(crouching, has_item)` (`crafting.rs:394`), which is vanilla's "sneak with an item does not open".
- The dispatch is at `:865-923`.
  - On `use_pressed` it sets `open_table`, `open_bench` or `open_block`, and returns `HandUse::Consumed`.
  - For stations it first checks what blocks the lid (`:892-917`): a conductor or cat above a chest, a conductor above an ender chest, and the shulker lid room (`containers.rs:227`).
- The result is `Acted { open_table, open_block, open_bench, .. }` (`actions.rs:117-128`).
- **Hoppers, dispensers, droppers, looms, cartography tables and beacons match nothing here, so nothing opens.** The click goes on to `entities.use_block` (`:841-864`) and the code below it.

**Host.**
- `render_anim/src/minecraft_world.rs:974` destructures `Acted`.
- `:1372-1377` calls `minecraft_survival::menus::open_station(...)`, then `inventory_ui.show_menu` (`render_anim/src/minecraft_inventory.rs:162-169`), which sets `ui.screen`, `screen_title`, `inventory_open` and `screen_opened`.

**Friend (on the host).**
- `render_anim/src/minecraft_players.rs:1961` `friend_hand` calls the same `act` (`:2017`) and returns the `Opens` tuple (`:2065-2070`; type alias at `:2097-2101`).
- `step_friend` calls `open_station` (`:1845-1857`).

**`open_station`** (`menus.rs:281-365`):
- **Tables:** `Open::Table`.
- **Blocks:** rolls structure loot for a new chest or barrel (`:299-330`), takes the title from the custom name in `Containers` or from `station_title` (`:247-259`, `:331-334`), then `Open::Block`.
- **Benches:** `Open::Bench(pos, Bench::new(kind))` with `bench_title` (`:261-269`); the enchanting table also counts its shelves (`:340-358`).
- **Every case ends in** `menus::open` (`:271-278`). It refuses if a menu is already open, and assigns the next id (`Menus::next_id` `:160-163`, ids 1 to 100).

**Is the menu still valid?**
- `still_valid` (`menus.rs:394-409`) uses `crafting::within_table_reach` (`crafting.rs:398`) and checks the block kind.
- It is checked each frame on the host (`minecraft_world.rs:1305-1307`) and for friends (`minecraft_players.rs:2201-2210`).

### 1.2 Menu state

- `Open` (`menus.rs:45-52`): `Inventory | Table | Block(Opened) | Bench(BlockPos, Bench) | Merchant(MerchantScreen) | Mount(MountInventory)`.
- `Menu` (`:77-112`) holds `id`, `state` (vanilla's state id), `dimension`, `title: Text`, `shelves` and the last `sent` snapshot.
- `Menus` (`:115-124`): `open`, `inventory`, `merchant_ops`, `merchant_waiting`, `mount_updates`, `mount_key`.
- `Menu::block()` (`:102-107`) returns only `Open::Block`, and the chest lids are keyed on it (1.7).

### 1.3 The server-side container and its saved data, by model

| Model | Where the slots live | Saved by | Anchors |
|---|---|---|---|
| Block (chest, barrel, ender, shulker, furnace, brewer) | `Containers.chests/ovens/brewers/ender` | `Containers::block_entity_tag` → `level::save_block_entities` | `Station` `containers.rs:45-89`, `Opened` `:293-336` (`rows` `:318`, `size` `:325`), `cells` `:1060-1090`, `store` `:1092-1121` (marks dirty), `import`/`load_chunk`/`forget_chunk` `:820-873` |
| Bench (stonecutter, smithing, grindstone, anvil, enchanting) | `Bench.cells` in the player's `Open` | not saved: `close` returns them | `stations.rs`: `Workshop` `:22-63`, `Bench` `:474-934` (`offer` `:491`, `choose` `:684`, `take` `:753`, `consume` `:804`, `Taken` `:466`), `cell_limit` `:936`, `accepts` `:944` |
| Merchant | server thread; host holds a view copy | server | `MerchantOp` `server.rs:280-292`, `MerchantUpdate` `:298-306`, `ServerSim::merchant` `:736`, `Command::Merchant` `:4385`, host side `menus.rs:1233-1371`, `entities.rs:1210-1221` |
| Mount | server mob inventory; the copy lives in `Open::Mount` | server | `MountInventory::of` at open `menus.rs:2052-2054`, whole copy pushed `:2071-2073` → `entities.rs:1178` → `Command::MountInventory` `server.rs:4405`, `:4869`, `:5155`; one viewer lock `menus.rs:2044-2050` |
| Level-owned (hopper, dispenser, dropper) | server level block entity `Items` | server, when changed | `container.rs:128` `container_at`, `:191` `block_container_items`, `:197` `container_item`, `:234` `container_set_changed`, `:241-251` `container_set_item` |

**The lectern** is the precedent for screens over a level block entity.
- The server keeps `lectern_viewers`, answers `Open`, pushes a new view to every viewer on a change, and closes viewers whose block went away (`server.rs:2161-2246`).
- The host takes the views with `entities.take_lecterns` (`entities.rs:1206`; host at `minecraft_world.rs:1406-1423`).

### 1.4 Click handling

**HUD to action.**
- `hud/src/minecraft_clicks.rs` turns raw input into `McClick`, through the generic `Menu` trait (`:7-16`):
  - press and pick-up on press with an empty cursor (`Clicks::press` `:115`);
  - place on release, with drag split (`drag_over` `:162`, `release` `:177`);
  - double-click gather (the `Gather` case in `release`);
  - number keys, F and Q (`key` `:261`).
- The HUD's `Menu` is `MenuView` over the layout and `ui.droppable` (`hud/src/minecraft_inventory.rs:107-132`; `picks_all` `:129` is false only on the trading screen).
- `McClick` (`frame/src/minecraft_ui.rs:163-183`) becomes `MenuAction` in `render_anim/src/minecraft_inventory.rs:809-821` `action_of`, through `menu_slot` (`:796`).
- On the wire: `MenuSlot` (`messages.rs:169-177`), `MenuAction` (`:180-210`).

**Host.**
- `InventoryUi::apply_input` (`minecraft_inventory.rs:324-385`) calls `menus::apply`; `McClick::Close` calls `menus::close`, and `Rename` calls `menus::rename`.

**Friend.**
1. The friend sends `ToServer::Click { menu, state, action }` (`minecraft_remote.rs:1485-1508`).
2. The host turns it into `MenuInput::Click` (`minecraft_host.rs:154-171`) and pushes it with `Session::push_menu` (`session.rs:366`, capped by `MENU_INPUTS`).
3. `friend_menus` passes it to `menus::take_inputs` (`minecraft_players.rs:2171-2179`), then `menus::click` (`menus.rs:1080-1117`).
   - `click` ignores the dead, spectators, a wrong menu id and a wrong dimension; checks `valid` (`:610-627`); and asks for a full resend when the state is stale.

**`menus::apply`** (`:974-1078`):
1. Guards run first: `keeps_gun` (`:493-543`; the variant list at `:528`), `misfits` (`:545`) and `fitted_spread` (`:566`).
2. Then it routes by model:
   - merchant (`:1005-1021`, queued while busy);
   - mount (`:1022-1033`);
   - bench (`:1038-1065`, the `Taken` levels, sounds, enchant seed and anvil);
   - block (`:1066-1070`);
   - finally `grid_action` (`:758-846`) for the player inventory and grids.

**`block_action`** (`:711-756`) maps each action to `Containers`:

| Action | `Containers` method |
|---|---|
| shift on a player slot | `quick_move_from_player` (`containers.rs:1184-1256`) |
| slot click or shift click | `click` (`:1133-1182`, through `pick` `:1463-1519`, `accepts` `:1439`, `limit` `:1431`, `is_output` `:1427`) |
| double click | `gather` (`:1383-1424`) |
| number key or F | `swap` (`:1258-1313`) |
| Q, with or without Ctrl | `throw` (`:1352-1381`) |
| drag | `spread` (`:1315-1337`, through `Inventory::distribute_limited`) |

- `bench_action` (`:629-709`) and `mount_action` (`:918-950`) are the same mapping onto `Bench` and `MountInventory`. `Choose(u16)` is the button action (`bench.choose`).
- Clicks outside the window drop through `grid_action`'s `Outside` case.

### 1.5 What a friend sees

**Host to friend.**
- `menus::changes` (`:1745-1786`) is called per friend from `Server::send_menus` (`session.rs:1423-1439`, called at `minecraft_host.rs:670`).
- It runs `sync` (`:1695-1743`) over:
  - `snapshot` (`:1629-1680`): `contents` (`:1410-1440`) gives cells and data, `data` is only filled for ovens, brewers and the stonecutter's choice (`:1631-1637`), and `extras` come from `:1638-1665`;
  - `view_of` (`:1682`), with `kind_of` (`:1467-1492`).
- It sends one of three messages (`messages.rs:271-281`):
  - `ToClient::MenuOpen(MenuView)` on the first send, a resend, or a change in the extras or the slot count;
  - otherwise `MenuSlots` with only the changed slots and data, and the carried stack;
  - `MenuClose` when the host closed the menu (`menus.told`).
- `MenuView` is defined at `messages.rs:638-647`.

**On the friend.**
- `remote.rs:467-494` stores and patches the view through `menus::apply_slots` (`menus.rs:2113-2138`).
- `minecraft_remote.rs:1449-1510` `show_menus` turns it into a screen: `mirror` (`menus.rs:2089-2111`) → `mirror_open` (`:1938-1983`, made-up cell positions `(0, -1000-i, 0)`) → `minecraft_inventory::screen_for`.
- `:1300-1358` publishes it with a stand-in `Containers::default()` that only carries the stations book (`:1344-1345`).
- **Bench results are computed on the friend's side** (`render_anim/src/minecraft_inventory.rs:522-551`, `bench.offer(&ctx)`), not sent. A result that needs data only the host has (cartography: the map's scale and lock) must be sent as a cell or in the extras.

**Protocol.**
- `PROTOCOL = 12` (`messages.rs:7`), and release 7 shipped it.
- A track never bumps it. A commit that changes `MenuExtras`, `MenuAction` or any `ToServer`/`ToClient` adds a line under "## Protocol" in the track log (`track-e3.md:74`).
- The release bumps it, along with the four `PROTOCOL` asserts at `codec.rs:923`, `:1003`, `:1021` and `:1095`.
- The codec round-trip coverage needs updating for any wire change:
  - the menu samples in `to_client()` (`codec.rs:567-616`) and the click samples (`:318-382`);
  - the variant counters `seen[18]` and `seen[37]` (`:784`, `:803`) and `client_index` (`:226`; `MenuOpen` 15, `MenuSlots` 16, `MenuClose` 17);
  - a round-trip test per new extras variant, like `:820` (trading) and `:863` (horse).
- The friend's menu rules are in `docs/MULTIPLAYER.md:205-206`, `:654-658` and `:822`.

### 1.6 HUD layout, textures and widgets

**Textures.**
- The container sheets are read from the runtime pack (the 26.3 jar) into one atlas: `render_anim/src/minecraft_gui.rs:231` `build_atlas` loops over `CONTAINER_SHEETS` (`:19-38`) as `minecraft:gui/container/{sheet}` (`:236-242`).
- Sprites come from `SPRITE_FOLDERS` (`:40-59`) and `LOOSE_SPRITES` (`:61-76`).
- Text is filtered by `TEXT_PREFIXES` (`:81-98`). `container.` is already in it; `block.minecraft.beacon.` is not.
- The HUD draws with `Painter::blit(sheet, ..)`, `sprite`, `sprite_part` and `item` (`hud/src/minecraft_gui.rs:305`, `:317`, `:358`, `:673`).
- The sheets for the six exist as 256x256 PNGs: `hopper.png`, `dispenser.png`, `loom.png`, `cartography_table.png`, `beacon.png`.
- Sprite folders exist for `container/loom/`, `container/cartography_table/` and `container/beacon/`. Hoppers and dispensers have none.
- The slot sprites `container/slot/banner`, `dye` and `banner_pattern` are 16x16.

**Layout and drawing** (`hud/src/minecraft_screens.rs`):
- `layout(McScreen)` (`:122-286`) gives the size and the `SlotSpec`s (`McSlot`, x, y, `Group`, empty-slot sprite). `player_inventory(out, top)` is at `:101-120`.
- `Layout::sheet` (`:313-333`) is an exhaustive match.
- `draw_background` (`:493-535`) blits the whole sheet by default, then draws per-screen widgets (furnace `:552`, brewing `:588`, enchanting `:639`, anvil `:731`, stonecutter `:770`, smithing `:826`, mount `:537`).
- `draw_labels` (`:846-867`) places the title per screen; the "Inventory" label sits at `h - 94`.
- `draw_slots`, `draw_highlight`, `draw_carried` and `station_tooltip` are at `:869-1063`.
- **The puppet box.** `PUPPET_BOX` (`:7`) is used only for `Inventory` (and the creative inventory tab) in `hud/src/minecraft_inventory.rs:643-650`. It cuts the hole in the background gradient and sets `ui.character_box` for the 3D player figure. E3's `mount-figure` group adds a `MOUNT_BOX` there.

**Buttons, lists and scrolling.**
- `station_input` (`hud/src/minecraft_inventory.rs:744-880`) handles the enchanting rows, the trade list and the stonecutter grid with its scroller.
- It pushes `McClick::Choose(index)`; the gamepad focus is `focused_choice` (`:612`).
- The choices come from `ui.choices`/`chosen`/`cost`/`name` (`frame/src/minecraft_ui.rs:209-212`), filled by `render_anim/src/minecraft_inventory.rs:522-590` from `Offer.choices`.
- Drag capacities come from `ui.droppable`, computed by `droppable()` (`:833-891`): `containers.accepts` for blocks, `bench.accepts_stack` for benches, and per screen for the mount and merchant screens.
- The screen enum is `frame::McScreen` (`frame/src/minecraft_ui.rs:68-95`).
- `screen_for`, `bench_screen` and `screen_of` (`render_anim/src/minecraft_inventory.rs:785-794`, `:1008-1027`) map `Open` to `McScreen`.

### 1.7 Sounds

- **Opening and closing chests, barrels, ender chests and shulker boxes.**
  - Host: `menus::lids` (`menus.rs:1858-1888`, at `minecraft_world.rs:1391-1400`). Friends: `minecraft_players.rs:2233-2248`.
  - It goes through `level::toggle_openers` (`level.rs:350`) → `Openers` (`openers.rs:44-114`, counted only for those four kinds, `:49-54`) → `lid_effects` (`menus.rs:1903-1936`).
  - That gives the block event, the game event, the trapped chest count, the sound (`openers::sound` `:116-164`, an exhaustive match on `Station`) and the barrel's `open` state (`:170`).
- **Workbench results:** `Taken.sound` → `Clicked.used` → `menus::finish` (`:1788-1849`) → `Effect::Sound`. The anvil is special-cased (`Clicked.anvil`).
- **Sounds played only on the player's own game:** `ui.ui_sounds` (`frame/src/minecraft_ui.rs:280`), played at `minecraft_world.rs:1366` and `minecraft_remote.rs:630`. Only the books use it now; `StonecutterScreen`'s `ui.stonecutter.select_recipe` is not played yet.
- **Vanilla for the six (javap).** None plays an open or close sound.
  - Loom: `ui.loom.select_pattern` on the player's side (`LoomScreen`) and `ui.loom.take_result` on taking (`LoomMenu$6`).
  - Cartography table: `ui.cartography_table.take_result` (`CartographyTableMenu$5`; already `maps.rs:41` `TAKE_RESULT_SOUND`).
  - Beacon: `block.beacon.ambient`, `activate` and `deactivate` from `BeaconBlockEntity`.

### 1.8 Game events, piglins and statistics

- `lid_effects` posts `OpenerEvent::game_event()` (`openers.rs:28-33`: `CONTAINER_OPEN`/`CLOSE`, `game_events.rs:21-22`) through `entities.game_event` (`entities.rs:1095`).
- It angers piglins only for the counted kinds (`menus.rs:1871-1877`, `entities.rs:1542`).
- **Vanilla (javap on each `useWithoutItem`).** Hopper, dispenser, dropper, loom, cartography table and beacon post no game event and anger no piglins. Each only awards a statistic: `INSPECT_HOPPER`, `INSPECT_DISPENSER`, `INSPECT_DROPPER`, `INTERACT_WITH_LOOM`, `INTERACT_WITH_CARTOGRAPHY_TABLE`, `INTERACT_WITH_BEACON`.
- The fork does not simulate statistics (`MINECRAFT-SURVIVAL.md:369`).
- So a new `Open` variant that is not `Open::Block` correctly gets no lid, sound or game event.

### 1.9 Comparators and hoppers reading the container

- Comparators read `analog_output` → `container_signal` over the level's copy (`redstone.rs:483-499`, `container.rs:295-307`).
- Hoppers move items between level copies (`server.rs:10929` test: "the chest's items stay the containers'").
- **Chests and furnaces seen by hoppers and comparators are therefore empty copies** (`MINECRAFT-SURVIVAL.md:372`, E3 `doors-8` left out). That is out of track C's scope unless the brief takes on "one container truth".
- **For hoppers, dispensers and droppers it works as soon as clicks write the level copy.**
  - `container_set_item` calls `block_entity_changed` (comparator updates), but not for `Store::Hopper` (`container.rs:247-250`).
  - A click on the menu must therefore also call `container_set_changed` (`:234`), as vanilla's `Slot.setChanged` does.

---

## 2. The six missing screens: vanilla facts (javap) and the model each fits

| Screen | Vanilla menu, slot positions and screen size | Sheet, sprites and title key | Fits |
|---|---|---|---|
| Hopper | `HopperMenu`: 5 slots at (44+18i, 20); inventory at (8, 51); `HopperScreen` 176x133, `inventoryLabelY` = 133-94 | `hopper`; `container.hopper` ("Item Hopper") | Level container: server-applied operations, merchant style, plus a lectern-style viewer list pushing live cells |
| Dispenser, dropper | `DispenserMenu` (the dropper uses it too): 3x3 through `add3x3GridSlots`, 18 apart (vanilla's 62,17); inventory (8, 84); 176x166, title centred (`DispenserScreen.init`) | `dispenser`; `container.dispenser`, `container.dropper` | Same as the hopper |
| Loom | `LoomMenu`: banner (13,26), dye (33,26), pattern (23,45), result (143,57), inventory (8,84); `clickMenuButton` = `Choose`; screen 176x166, title y −2, patterns 4x4 at (60,13), 14 px cells, scroller 12x15 over 56 px | `loom`; `container/loom/*` (pattern, pattern_highlighted, pattern_selected, scroller, scroller_disabled, error); empty-slot sprites banner, dye, banner_pattern; `container.loom` | Workshop/Bench, with the chosen pattern in `data[0]` like the stonecutter (`menus.rs:1635`, `:1951-1953`). Patterns come from the jar's `banner_pattern` data and pattern tags. Item icons already draw patterns (`item_look.rs`) |
| Cartography table | `CartographyTableMenu`: map (15,15), extra (15,52), result (145,39), inventory (8,84); result gets `map_post_processing` SCALE or LOCK | `cartography_table`; `container/cartography_table/{map,scaled_map,duplicated_map,locked,error}`; `container.cartography_table` | Workshop/Bench. A new map id on scale or lock needs `globals.maps` (the pattern of `menus.rs:367-392`); `finish` (`:1788`) gets no globals today. Friends need the result sent (1.5) |
| Beacon | `BeaconMenu`: payment slot (136,110), inventory (36,137), `DATA_COUNT` 3 (levels, primary, secondary); `BeaconScreen` 230x219, confirm (164,107), cancel (190,107); confirm sends `ServerboundSetBeaconPacket` | `beacon`; `container/beacon/*` (button, button_disabled, button_highlighted, button_selected, cancel, confirm); `mob_effect/` is already loaded (`minecraft_gui.rs:286`); labels `block.minecraft.beacon.primary`/`secondary` (prefix not loaded); `container.beacon` | Hybrid. The payment slot is bench-like; the effects belong to the level block entity. Nothing of the beacon is simulated (pyramid levels, effects, beam: `MINECRAFT-SURVIVAL.md:369`; no `beacon` in `crates/minecraft_*` code). The three values fit `MenuView.data`. Confirm fits `Choose(u16)` only if encoded; a new `MenuAction` variant is a wire change |

**Missing beyond the six** (from the vanilla `MenuType` list):
- `CRAFTER_3x3`: there is no `MenuKind::Crafter`, so it is a wire change, and the level does not simulate crafters (`container.rs:9`).
- Container entities: chest and hopper minecarts and chest boats are not simulated.

**Why the hopper does not fit the mount model** (a host copy written back whole or per slot, which e3b's shared store keeps):
- Hoppers move items on the server thread every 8 ticks, and dispensers fire there.
- A click applied to a host copy that is a tick or two old, then written back, duplicates or loses items.
- The merchant model avoids this: the server applies the operation to its truth, answers with the changed player slots, cursor and thrown items (`MerchantUpdate`), and the host holds further clicks while it waits (`MerchantScreen.held` `menus.rs:58`, busy `:1010-1011`, `pump_merchant` `:1294-1333`).
- The server needs a viewer list like `lectern_viewers` and pushes fresh cells when `collect_block_entity_changes` (`server.rs:4141`) touches a viewed position. Every viewer and every friend's `MenuSlots` then follow hopper moves, as in vanilla.
- The stack conversion helpers are `stack_of` and `components_of` (`server.rs:383`, `:410`).

---

## 3. The smallest set of files and functions a new screen touches

### 3a. Workbench screen (loom, cartography table)

1. **`crates/minecraft_terrain/src/stations.rs`**
   - `Workshop` variant, `of`, `title` and `inputs` (`:22-63`).
   - `Bench::offer` (`:491`), `choose` (`:684`), `take`'s shift loop (`:797`), `consume` with `Taken.sound` (`:804`), `quick_move_from_player` special cases (`:631`), `cell_limit` (`:936`), `accepts` (`:944`).
   - Jar data in `StationBook::from_jar` and `add` (`:138`, `:159`), or a new loader.
2. **`crates/minecraft_survival/src/menus.rs`**
   - `bench_title` (`:261`) and `kind_of` (`:1482-1488`).
   - The `snapshot` data and extras (`:1631-1665`).
   - `mirror_open`: the `bench` closure (`:1946-1958`) and its arm, replacing the name in the `None` list at `:1976-1981`.
   - For effects outside the menu: a new `Clicked` field (`:192-206`) resolved in `finish` (`:1788`), and its two callers (`minecraft_world.rs:1398`, `minecraft_players.rs:2226`).
3. **`crates/render_anim/src/minecraft_inventory.rs`**
   - `bench_screen` (`:1008`).
   - The bench part of `publish` (`:522-551`): choice icons and the result.
   - `droppable` follows `accepts_stack` without changes.
4. **`crates/frame/src/minecraft_ui.rs`**: the `McScreen` variant (`:68-95`).
5. **`crates/hud/src/minecraft_screens.rs`**: `layout` arm, `sheet` arm, `draw_background` widgets, `draw_labels` position; `station_tooltip` and a `hovered_recipe`-like tooltip if needed.
6. **`crates/hud/src/minecraft_inventory.rs`**: `station_input` arm (buttons and scroll); the `focused_choice` list (`:612`).
7. **`crates/render_anim/src/minecraft_gui.rs`**: `CONTAINER_SHEETS`, `SPRITE_FOLDERS`, and `TEXT_PREFIXES` if new keys are needed.
8. **Not touched:** `actions.rs`, `Acted`, `Opens` and `open_station`.

### 3b. Level-container screen (hopper, dispenser, dropper)

1. **`menus.rs`**
   - A new `Open` variant (`:45-52`, next to `Block`).
   - `still_valid` (`:394`), `stack_at` (`:483-488`), the `keeps_gun` variant list (`:528`; it must be added, or guns shift-click in), `valid_slot` sizes (`:591-597`).
   - `apply` routing (`:974-1072`, before the merchant block or after the bench block).
   - `close` (`:1146`), `contents` (`:1410`), `kind_of` (`:1467`), `snapshot` (`:1629`).
   - `mirror_open` arms (`:1976-1981`).
   - A per-frame pump like `merchant()` (`:1335`).
   - Queue fields after `merchant_waiting` (`:121`).
2. **`actions.rs`**
   - Detection beside `station`/`workshop` (`:719-744`).
   - An `Acted` field (`:117-128`) and a dispatch branch (`:865-923`).
   - Unless the brief chooses to send them through a `Station` arm, which brings every exhaustive `Station` match with it: `containers.rs:56-109`, `:318-331`, `:1060-1121`; `openers.rs:124-157`; `menus.rs:247`, `:1471`; `render_anim` `:1018`.
3. **`render_anim/src/minecraft_players.rs`**: `Opens` (`:2097-2101`), the `friend_hand` return (`:2065-2070`), the open call (`:1845-1857`), and the pump beside `merchant` at `:2181`.
4. **`render_anim/src/minecraft_world.rs`**: the destructure at `:974`, the open at `:1372-1377`, and the pump beside `:1347`.
5. **`minecraft_survival/src/entities.rs`**: forwarders beside `merchant` (`:1210-1221`).
6. **`minecraft_terrain/src/server.rs`**
   - A `Command` variant beside `Merchant` (`:4385`).
   - A `ServerHandle` method beside `:4808` and a `ServerSim` handler beside `:736` or `:2161`.
   - The dispatch near `:5155-5172`.
   - The viewer list and the push of fresh cells.
7. **`render_anim/src/minecraft_inventory.rs`**: `screen_for` (`:785`), the cells in `publish` (`:503-590`), and `droppable` (`:833`). Note that the friend's `Containers` is a stand-in.
8. **HUD:** as in 3a. The default `draw_background` blit is enough; the dispenser title is centred like the furnace and brewing stand (`draw_labels` `:854`); the hopper is `player_inventory(.., 51)` with size [176, 133].
9. **Title:** the `CustomName` from `terrain.stream.stored_block_entity(pos)` (`terrain.rs:1296`, as harvest reads it at `actions.rs:1812-1815`). E3's e3j makes those names survive placement (`track-e3.md:3725-3728`).

### 3c. Beacon, in addition

- Server simulation first: levels every 80 ticks, effects to players, the sounds, and saving `primary_effect`/`secondary_effect`. The level saves non-owned kinds by itself.
- The beam render.
- Then the screen: an atlas test height risk (below), the `block.minecraft.beacon.` text prefix, and the confirm path.

---

## 4. Existing tests to copy

**`crates/minecraft_survival/src/menus.rs`** (tests from `:2140`; helpers `books` `:2159`, `chest` `:2167`, `friend_click` `:2205`, `opened_views` `:2196`):
- `two_players_in_one_chest_see_each_others_clicks` (`:2221`): `MenuOpen`/`MenuSlots`/state, and the friend's `apply_slots` copy equals `view_of`.
- `a_stale_state_resends_everything` (`:2302`).
- `a_burning_furnace_does_not_make_a_click_stale` (`:2337`).
- Closed menu or other dimension ignored (`:2394`).
- Overflow dropped on close (`:2472`).
- Anvil rename (`:2518`).
- Grid dropped on death (`:2583`).
- `the_friend_mirrors_a_chest_a_table_and_a_stonecutter` (`:2609`).
- Merchant flow (`:2646-3040`): queued clicks (`:2893`), server close (`:2952`), offer wire (`:3017`).
- Horse: clicks to level updates and the friend mirrors (`:3043`); one viewer (`:3111`, needs the real registries).
- Friend trading (`:3177`).
- Gun and killstreak guards (`:3285`, `:3331`), grenade slots (`:3405-3555`), creative sets (`:3613-3802`).

**`crates/minecraft_terrain/src/containers.rs`** (from `:1522`):
- Stations and double chests (`:1572`), shift-move (`:1595`), double spread (`:1623`), furnace (`:1642`), brewing (`:1702`).
- Number keys and outputs (`:1979`), Q throw (`:1999`), F and drag (`:2015`), bottle drag (`:2044`).
- Ender and shulker (`:2070-2154`), lid room (`:2237`).
- Vanilla codec round trips (`:2319`, `:2564`), dirty writes (`:2393`), break clears (`:2421`), ownership (`:2451`, which asserts `hopper`, `dispenser` and `dropper` are level-saved).

**`crates/minecraft_terrain/src/stations.rs`**: `workshops_come_from_their_blocks` (`:1474`), stonecutter (`:1488`), smithing (`:1513`), grindstone (`:1532`), anvil (`:1553`), bench throw, swap and gather (`:1609`), enchanting slot (`:1637`).

**`crates/minecraft_terrain/src/server.rs`**: hopper changes go to the chunk map (`:10929`), broken dropper (`:10968`), dispenser rockets with `replace_block_item` and `block_container_items` (`:17034`).

**`crates/hud/src/minecraft_screens.rs`** (`:1137`): layout coordinates against vanilla (`:1145`, `:1170`, `:1187`, `:1223`), hover (`:1264`), click regions (`:1277`), progress rounding (`:1294`), cyclers (`:1314-1395`).

**Other HUD tests:**
- `crates/hud/src/minecraft_clicks.rs:365-616`: press and release, drag split, double click, number keys, Q, outside.
- `hud/src/minecraft_inventory.rs:1086`: pad focus.

**`render_anim`:**
- `minecraft_inventory.rs:1040`: `action_of` and `screen_for`.
- `minecraft_gui.rs:643`: the real atlas. It checks `sheets.len() == CONTAINER_SHEETS.len() + 2`, sprite sizes, and **atlas height ≤ 2048 (`:659`)**; five more 256x256 sheets plus M2's nautilus must still fit.

**Network and friend:**
- `minecraft_net/src/codec.rs`: `:782`, `:801`, `:820`, `:863`.
- `minecraft_survival/src/remote.rs:1372` `menus_and_gear_reach_the_friend`.

**Opening rules:** `actions.rs:3161-3250` (cat or conductor keeps a chest shut, ender chest ignores cats).

**Lids:** `openers.rs:194+`, `lids.rs:475-857`.

**End-to-end save:** `crates/minecraft_survival/tests/vanilla_world.rs`.

---

## 5. Shared files, parallel tracks and the merge rule

**The rule** (`track-e3.md:72`; CLAUDE.md track workflow): new variants, fields, arms and list entries go next to a related one, never at the end of a shared list. The exception is lists whose position is their meaning: `ToClient` variants (the codec pins their tags) and `McAction`.

**Track C's spots:**
- `Open` variants: next to `Block` (`menus.rs:45-52`).
- `Workshop` `Loom`/`Cartography`: after `Stonecutter`/`Smithing` (`stations.rs:24-28`).
- `McScreen`: `Hopper`/`Dispenser` beside `Chest`/`ShulkerBox`, `Loom`/`Cartography` beside `Stonecutter`, `Beacon` beside `Enchanting` (`frame/src/minecraft_ui.rs:68-95`).
- `CONTAINER_SHEETS`: `hopper`/`dispenser` beside `generic_54`, `loom`/`cartography_table` beside `stonecutter`, `beacon` beside `enchanting_table`.
- `mirror_open`: take names out of the `None` list one at a time (`:1976-1981`).

**Collisions to avoid:**
- **E3 e3b `mount-viewers` and `mount-figure`** (planned; `track-e3.md:977-1093`):
  - `containers.rs:603-613`: a `mounts` field beside `chests`. Put any track C field after `brewers` or `campfires`.
  - `menus.rs`: removes `mount_updates` (`:122`, `:141`), rewrites the mount block in `apply` (`:1022-1033`), `stack_at` `:486`, `contents` `:1438`, and `mount()` `:2026-2073` (gains `containers`).
  - Its callers `minecraft_world.rs:1346-1357` and `minecraft_players.rs:2199`: put track C's pump next to the `merchant` calls instead.
  - `minecraft_screens.rs:7` (`MOUNT_BOX`) and `:493-512`; `hud/src/minecraft_inventory.rs:643-650`, `:719`; `entities.rs` mount viewers.
- **E3 e3j `placed-names`** (`track-e3.md:3701-3736`): `menus.rs:331` title; `containers.rs` `name_placed` beside `place_shulker` (`:913`); `actions.rs` placement (`:1627-1641`) and harvest (`:1800-1818`); the vendored `components.rs` names for hoppers, dispensers, droppers and beacons.
- **E3 e3s `oven-book`** (`track-e3.md:6615+`): `menus.rs:582` `book_grid` and `:623`, `:999`; the recipe book files.
- **E3 e3d `friend-slot-changes`** (`track-e3.md:1878+`): appends `ToClient::PlayerSlots`, changes `codec.rs` `client_index` and the `seen` count, `session.rs`, `remote.rs:467` and `minecraft_remote.rs`. If track C adds a message, coordinate the appended `ToClient` tail.
- **E3 e3i `dispenser-blocks`**: vendored `level/dispenser.rs`.
- **M2 m2h nautiluses** (`track-m2.md:2732`): adds `nautilus` beside every `llama` in `frame/src/minecraft_ui.rs:90`, `messages.rs:584` (`MenuExtras::Mount`), the `menus.rs` mount lines, `render_anim/src/minecraft_inventory.rs:788-858`, `minecraft_screens.rs:254`, `:328`, and the `nautilus` sheet in `CONTAINER_SHEETS`.
- **M3 copper golems:** chest sorting touches the container truth (`track-m3.md:1026-1027`).
- **The server `Command` enum** (`server.rs:4346-4419`): every M track adds commands. Place a new one beside `Merchant` (`:4385`).

**Merging** (CLAUDE.md, `docs/PROGRESS.md` "How to resume" step 4): one track at a time in the track's worktree, keep both sides of a conflict, run the full suite, record conflicts in `docs/round10/decisions.md`.

---

## 6. Gotchas

- **`keeps_gun` (`menus.rs:528`)** lists `Block | Bench | Mount`. A new `Open` variant that is left out lets MW2 guns shift-click into it.
- **Atlas height.** `CONTAINER_SHEETS` grows the atlas. The `≤ 2048` height assert (`minecraft_gui.rs:659`) only runs with `MINECRAFTOSS_ROOT` set, which `cargo-slot.ps1` sets.
- **Beacon text.** Its labels need the `block.minecraft.beacon.` prefix in `TEXT_PREFIXES` (`minecraft_gui.rs:81-98`).
- **Hopper clicks and comparators.** `container_set_item` skips the comparator update for hoppers (`container.rs:247-250`); call `container_set_changed` after a menu click.
- **Results friends cannot compute.** Bench results are recomputed on the friend's side (1.5). Cartography needs the result sent.
- **Loom choice on the wire.** The loom's choice needs the `Stonecutter` special cases in `snapshot` (`:1635`) and `mirror_open` (`:1951`) widened.
- **Hoppers today.** Right-clicking a hopper or dispenser opens nothing, and the click falls through to `use_block` and the code after it (`actions.rs:841-864`). Vanilla opens the screen unless the player sneaks with an item.

## 3. Vanilla 26.3 rules of the missing screens

I read these at commit 20f1ce4e of `C:\Dev\Soscraft\mashup-crafting` (branch minecraft-survival). Two of the files are not committed: `docs/round10/decisions.md` has local changes and `docs/round10/release-7-tests.md` is untracked.

**Sources.** The jar is `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3\client.jar`. It is not obfuscated, so the class and method names below are the real ones. Data comes from `...\minecraft-26.3\datapacks\local\minecraft-26.3\data\minecraft\`.
- The javap on PATH is JDK 8 and cannot read these classes. I used `C:\Users\tsupp\.jdks\openjdk-26\bin\javap.exe -c -p -constants`.
- Raw dumps for re-checking are in `C:\Users\tsupp\AppData\Local\Temp\claude\C--Dev-Soscraft-mashup-crafting\0db10490-5b69-4a4e-ab4e-d60da7dcb4d8\scratchpad\track-c-brief\jp\` (one `<Class>.txt` per class).
- Helpers in the same folder: `C:\Users\tsupp\AppData\Local\Temp\claude\C--Dev-Soscraft-mashup-crafting\0db10490-5b69-4a4e-ab4e-d60da7dcb4d8\scratchpad\track-c-brief\f.sh` strips line tables; `C:\Users\tsupp\AppData\Local\Temp\claude\C--Dev-Soscraft-mashup-crafting\0db10490-5b69-4a4e-ab4e-d60da7dcb4d8\scratchpad\track-c-brief\pngsize.py` reads PNG sizes from the jar.
- I ran no cargo, wrote nothing into the repo and made no commits.

All GUI coordinates below are relative to the screen's left/top corner. "inv" means the 27 backpack slots and "hot" means the 9 hotbar slots.

---

## 0. What the fork has now (anchors at 20f1ce4e)

**Menus and network**
- `crates/minecraft_survival/src/menus.rs:1938-1983` `mirror_open`. Lines `:1976-1981` return `None` for Loom, Cartography, Hopper, Dispenser, Dropper and Beacon.
- `menus.rs:1467-1492` `kind_of`: no `Open`, `Station` or `Workshop` value exists for any of the seven blocks.
- `menus.rs:247` `station_title`, `:261` `bench_title`, `:281` `open_station`.
- `menus.rs:394-409` `still_valid`.
- `menus.rs:224` `Screen.data: [i32; 4]`; `:2089-2111` `mirror()` copies only data indexes 0..3. **The crafter needs 10 data values.**
- `crates/minecraft_net/src/messages.rs:567-590` `MenuKind` has Loom, Cartography, Hopper, Dispenser, Dropper and Beacon, but **no Crafter**.
- `messages.rs:180-210` `MenuAction` has `Choose(u16)`, which fits a loom pattern click. There is no action for the beacon's set-effects or the crafter's slot toggle.
- `messages.rs:7` `PROTOCOL = 12`. Adding variants changes the serde wire format.

**Containers and storage**
- `crates/minecraft_terrain/src/containers.rs:46-74` `Station` (chest, barrel, ovens, brewer, ender, shulker only).
- `containers.rs:96-109` `block_entity_kind`.
- `containers.rs:135-146` `owns_block_entity` and `leave_to_client`. Hopper, dispenser and dropper items are saved by the vendored level, not by `Containers`; the test at `:2470` checks this.
- `third_party/minecraftoss/world/src/level/container.rs:1-10`: the module header says it simulates hoppers, dispensers and droppers, but "not yet … crafters".
- In the same file: `container_at` `:128`, `block_container_items` `:191`, `container_set_item` `:241`.
- `third_party/minecraftoss/world/src/level/dispenser.rs:45,73` handle dispenser neighbour changes and dispensing.
- `crates/minecraft_terrain/src/stations.rs:23-44` `Workshop`.

**Reach**
- `crates/minecraft_terrain/src/crafting.rs:13` `TABLE_REACH = 8.5`; `:398-405` `within_table_reach` measures from the eye to the block's box.
- This equals vanilla `Player.isWithinBlockInteractionRange(pos, 4.0)`: the default reach of 4.5 plus 4.0.

**Maps**
- `crates/minecraft_terrain/src/maps.rs:24` `MAX_SCALE = 4`; `:41` `TAKE_RESULT_SOUND`.
- `maps.rs:103-126` `MapData::create_fresh` matches vanilla `createFresh`. There is no `scaled()` or `locked()`.

**Drawing**
- `crates/render_anim/src/minecraft_gui.rs:17` `MOB_EFFECTS`; `:19` `CONTAINER_SHEETS` (18 entries; dispenser, hopper, beacon, loom, cartography_table and crafter are missing); `:40` `SPRITE_FOLDERS` (container/beacon, loom, cartography_table and crafter are missing).
- `crates/hud/src/minecraft_screens.rs:122` `layout()`. The stonecutter at `:225` and `:770` already has a scroller and a recipe grid, a pattern the loom can follow.
- `crates/frame/src/minecraft_ui.rs:69-95` `McScreen` has none of the seven.

**Missing entirely**
- There is no beacon block entity, beam or effect code anywhere in `crates/` or `third_party/`. `elden_boss` only plays the `block.beacon.activate` sound.
- Crafters are not simulated.

**Related docs**
- `docs/round10/decisions.md:51` (local changes) records the Track C decision.
- `docs/round10/release-7-tests.md:626,634` (untracked): "no dispenser can be loaded", "there is no dispenser screen".

---

## 1. Rules all seven share

- **Player slots.** `AbstractContainerMenu.addStandardInventorySlots(inv,x,y)` adds the backpack slots (inventory 9..35) at `(x+col*18, y+row*18)`, then the hotbar (inventory 0..8) at `(x+col*18, y+58)`.
- **Screen defaults** (`AbstractContainerScreen`): 176×166. The title is drawn at (8,6). The "Inventory" label is drawn at (8, imageHeight−94). Both use colour `0xFF404040` with no shadow (`extractLabels`). The background is one `blit` of the 256×256 sheet at UV (0,0) with the image size.
- **Shift-click.** `AbstractContainerMenu.doClick` QUICK_MOVE needs `slot.mayPickup(player)`. It then calls `quickMoveStack` again and again while the returned stack is the same item as the slot's. So shift-clicking a result keeps crafting.
- **stillValid, two forms.**
  - Block-entity menus (dispenser, dropper, hopper, crafter) use `Container.stillValidBlockEntity(be, player, 4.0f)`. The block entity at the position must be the same object, and `isWithinBlockInteractionRange(pos, 4.0)` must hold.
  - Block menus (beacon, loom, cartography) use `AbstractContainerMenu.stillValid(access, player, Block)`. The block at the position must be `Blocks.BEACON`, `LOOM` or `CARTOGRAPHY_TABLE`, with the same range of 4.0.
- **Open and close sounds or game events: none for all seven.**
  - `Container.startOpen` and `stopOpen` default to no-ops, and none of these block entities overrides them (`grep` over the dumps finds 0 hits).
  - `ContainerOpenersCounter` is not used, so there is no lid sound and no CONTAINER_OPEN or CONTAINER_CLOSE game event.
  - `ServerPlayer.openMenu` sends only `ClientboundOpenScreenPacket(id, type, getDisplayName())`.
- **Lock, name and loot** (`BaseContainerBlockEntity` and `RandomizableContainerBlockEntity`).
  - Tags: `CustomName`, `lock` (via `LockCode.fromTag`/`addToTag`), `Items`, and `LootTable` plus `LootTableSeed`.
  - `RandomizableContainerBlockEntity.createMenu(id,inv,player)`: if `canOpen` (`lockKey.canUnlock`), it calls `unpackLootTable(player)` and then creates the menu.
  - Otherwise, if the player is not a spectator, `sendChestLockedNotifications`: an overlay message `container.isLocked` with the name, plus the `CHEST_LOCKED` sound at the block centre (BLOCKS, 1, 1). It returns null.
- **Inputs returned on close.** `AbstractContainerMenu.clearContainer` puts each item from the loom's or cartography table's input container back with `dropOrPlaceInInventory`.
- **MenuType registry ids** (`MenuType.<clinit>` order): generic_3x3=6, crafter_3x3=7, beacon=9, hopper=16, loom=18, cartography_table=23.

---

## 2. Dispenser and dropper (`DispenserMenu`, type GENERIC_3x3)

**Slots** (`DispenserMenu.<init>`, `add3x3GridSlots`)
- Container 0..8 at `(62+col*18, 17+row*18)`, index = col + row*3.
- inv 9..35 and hot 36..44 from `addStandardInventorySlots(inv, 8, 84)`.
- `checkContainerSize(9)`.

**Shift-click** (`DispenserMenu.quickMoveStack`)
- Index < 9: `moveItemStackTo(9, 45, reverse=true)`. Otherwise: `moveItemStackTo(0, 9, false)`. If either fails, it returns EMPTY.
- Then: if the stack is empty, `setByPlayer(EMPTY)`, else `setChanged`. If the count did not change, it returns EMPTY. Otherwise `onTake`.

**Validity and closing.** `stillValid` → `dispenser.stillValid` (block-entity form). `removed` → `stopOpen`, which is a no-op.

**Screen** (`DispenserScreen`)
- 176×166, `textures/gui/container/dispenser.png`.
- In `init`, the title is centred: `titleLabelX = (176 − width(title))/2`.
- The inventory label is at (8,72).

**Block entity** (`DispenserBlockEntity` / `DropperBlockEntity`)
- 9 items, saved as `Items` or as a loot table, plus `CustomName` and `lock`.
- Default names: `container.dispenser` and `container.dropper`. The dropper inherits `createMenu`, so both use the same menu.

**Opening.** `DispenserBlock.useWithoutItem` (server only) calls `openMenu` and awards the stat `INSPECT_DROPPER` or `INSPECT_DISPENSER`.

## 3. Hopper (`HopperMenu`, type HOPPER)

**Slots.** Container 0..4 at `(44+i*18, 20)`; `addStandardInventorySlots(inv, 8, 51)` gives inv 5..31 and hot 32..40. `checkContainerSize(5)`.

**Shift-click** (`HopperMenu.quickMoveStack`)
- Index < 5: `moveItemStackTo(5, slots.size()=41, true)`. Otherwise: `moveItemStackTo(0, 5, false)`. If either fails, it returns EMPTY.
- Then: if the stack is empty, `setByPlayer(EMPTY)`, else `setChanged`.
- **There is no count check and no `onTake`.** It returns the copy.

**Validity.** `stillValid` → container (block-entity form).

**Screen** (`HopperScreen`)
- 176×**133**, `textures/gui/container/hopper.png`.
- Title at (8,6). `inventoryLabelY = 133−94 = 39`.

**Block entity** (`HopperBlockEntity`)
- 5 items, a loot table, `CustomName`, `lock`, and `TransferCooldown` (read with default −1).
- Name `container.hopper`, which is "Item Hopper" in en_us.

**Opening.** `HopperBlock.useWithoutItem` calls `openMenu` and awards the stat `INSPECT_HOPPER`.

## 4. Beacon

### Menu (`BeaconMenu`, type BEACON)

**Slots**
- Slot 0 is a `PaymentSlot` at (136,110) in an inner `SimpleContainer(1)` (`BeaconMenu$1`).
- Both the slot and the container allow only `#minecraft:beacon_payment_items` and have a max stack size of **1**.
- The tag lists, in order: netherite_ingot, emerald, diamond, gold_ingot, iron_ingot.
- inv 1..27 and hot 28..36 from `addStandardInventorySlots(inv, 36, 137)`.
- **The payment item is not stored in the block entity.**

**Data slots** (`ContainerData`, 3 values)
- 0 = levels, 1 = primary, 2 = secondary.
- Effects are encoded as `encodeEffect` = MOB_EFFECT registry id + 1, with 0 meaning none.
- Registry ids: speed 0, haste 2, strength 4, jump_boost 7, regeneration 9, resistance 10. So the data values are speed=1, haste=3, strength=5, jump_boost=8, regeneration=10, resistance=11.

**Shift-click** (`BeaconMenu.quickMoveStack`)
- Index 0: `moveItemStackTo(1, 37, true)` and `onQuickCraft`.
- Otherwise, if the payment slot is empty, `mayPlace(stack)` holds and **`stack.getCount() == 1`**: `moveItemStackTo(0, 1, false)`. A stack of 2 or more ingots does not go to the payment slot.
- Otherwise, index 1..27 → (28, 37); index 28..36 → (1, 28); anything else → (1, 37).
- Any failure returns EMPTY. Then: if the stack is empty, `setByPlayer(EMPTY)`, else `setChanged`. If the count did not change, EMPTY. Otherwise `onTake`.

**Validity and closing**
- `stillValid` → `stillValid(access, player, Blocks.BEACON)`.
- `removed` (server only) takes the payment stack out and **drops it into the world**: `player.drop(stack, false, Prediction.SERVER_ONLY)`. It does not go back to the inventory.

**Setting effects** (`BeaconMenu.updateEffects(primary, secondary)`)
- It needs an item in the payment slot, and `BeaconBlockEntity.validateEffects(primary, secondary, levels)` must pass.
- Then it sets data[1] and data[2], removes 1 item from the payment slot, runs `access.execute(Level::blockEntityChanged)` and returns true. Otherwise it returns false.

**Server side** (`ServerGamePacketListenerImpl.handleSetBeaconPacket`)
- It acts only if `containerMenu` is a BeaconMenu.
- If `!stillValid`, it logs at debug level and returns.
- If `updateEffects` returns false, it **disconnects the player** with `multiplayer.disconnect.generic` and warns "tried to set invalid beacon effects". This includes the case of an empty payment slot.

**`validateEffects(p, s, levels)`** (`BeaconBlockEntity`)
- If s is set and levels < 4: false.
- `req(x)` is the tier index + 1 of the effect, 0 for null, and MAX_INT if the effect is unknown.
- If `req(p) > levels` or `req(s) > levels`: false.
- If `req(p) >= 4` (regeneration as primary): false.
- If `req(s)` is in 1..3 and s ≠ p: false.
- Otherwise true.

**Effect tiers** (`BEACON_EFFECTS`): [speed, haste], [resistance, jump_boost], [strength], [regeneration].

### Screen (`BeaconScreen`)

**Sheet and labels**
- 230×219, `textures/gui/container/beacon.png`.
- `extractLabels` is overridden: **no title and no "Inventory" label.**
- Instead, two centred labels in `0xFFE0E0E0`: `block.minecraft.beacon.primary` ("Primary Power") at (62,10) and `block.minecraft.beacon.secondary` ("Secondary Power") at (169,10).
- The background also draws item icons at y=109: netherite (20), emerald (41), diamond (63), gold ingot (86), iron ingot (108).

**Buttons**
- Every button is 22×22. The sprite is chosen in this order: `!active` → `container/beacon/button_disabled`; selected → `button_selected`; hovered or focused → `button_highlighted`; otherwise `button`. All four are 22×22.
- The icon is an 18×18 sprite at (+2,+2). Power buttons use `mob_effect/<id>` (`Hud.getMobEffectSprite`; files at `textures/mob_effect/<id>.png`).
- Pressing any button plays the standard client sound `ui.button.click` (`AbstractWidget.playButtonClickSound`, forUI pitch 1.0).

**Button positions** (`init`)
- Primary tiers t = 0..2, with n effects per tier: width w = n*22+(n−1)*2. Button j is at x = 76 + j*24 − w/2, y = 22 + t*25, with tier = t.
  - Tier 0: speed (53,22), haste (77,22).
  - Tier 1: resistance (53,47), jump_boost (77,47).
  - Tier 2: strength (65,72).
- Secondary: regeneration at (144,47), tier 3.
- An "upgrade" button at (168,47), tier 3, starts invisible.
  - It becomes visible once a primary is chosen and then shows that primary effect.
  - Its tooltip is the effect name plus " II".
  - It counts as selected when secondary == primary.
- Confirm at (164,107) with the `container/beacon/confirm` icon (18×18); tooltip `gui.done`.
- Cancel at (190,107) with the `container/beacon/cancel` icon; tooltip `gui.cancel`.
- Power button tooltips are the effect's description id.

**Button state** (`BeaconPowerButton.updateStatus(levels)`, run every `containerTick` and after a press)
- `active = tier < levels`, so secondary buttons work only at 4 levels.
- `selected = effect == (primary or secondary)`, depending on the button.

**Pressing a power button** (`BeaconPowerButton.onPress`)
- If the button is already selected, nothing happens.
- A primary button sets `primary = effect`, and **clears `secondary` unless secondary == this effect**.
- A secondary button sets `secondary = effect`.

**Confirm and cancel**
- Confirm is active when `menu.hasPayment() && primary != null`. Pressing it sends `ServerboundSetBeaconPacket(Optional primary, Optional secondary)` and then `closeContainer()`.
- Cancel calls `closeContainer()`.

**Data sync** (`BeaconScreen$1.dataChanged`): on any data change, the screen's `primary` and `secondary` are overwritten from the menu's data.

### Block entity (`BeaconBlockEntity`)

**Saved and loaded**
- Saved: `primary_effect` and `secondary_effect` (effect id strings), `Levels` (int), `CustomName`, `lock`.
- `loadAdditional` reads the effects through the VALID_EFFECTS filter, plus the name and lock, but **does not read `Levels`**. Levels are recomputed.
- The update tag is `saveCustomOnly`.

**Name, lock and menu**
- Default name `container.beacon`.
- `createMenu`: if `lockKey.canUnlock(player)`, it creates `BeaconMenu(id, inv, dataAccess, ContainerLevelAccess.create(level,pos))`. Otherwise it sends the locked notification and returns null; here there is no spectator check.

**Data setter** (`BeaconBlockEntity$1.set`)
- Index 0 sets levels.
- Index 1: on the server, if the beam is not empty, it plays **`block.beacon.power_select`**; then `primaryPower = filterEffect(decode(v))`.
- Index 2: `secondaryPower = filterEffect(decode(v))`.

**Opening.** `BeaconBlock.useWithoutItem` calls `openMenu` and awards the stat `INTERACT_WITH_BEACON`. The ticker runs on **both** sides.

**Tick** (`BeaconBlockEntity.tick`)
- **Beam scan, up to 10 blocks per tick.** If `lastCheckY < y`, a new scan starts at the beacon itself. The scan stops above the WORLD_SURFACE heightmap of the column.
  - A `BeaconBeamBlock` (the beacon, which is WHITE, stained glass and stained glass panes; tinted glass is not one) adds colour:
    - If the list has ≤ 1 section, a new section with that colour starts.
    - Else if the colour equals the last section's, the last section grows by 1.
    - Else a new section starts with `ARGB.average(last, colour)`.
  - Any other block: if `lightDampening < 15` or it is bedrock, the last section grows by 1.
  - Otherwise the beam is blocked: the sections are cleared and `lastCheckY = height`.
- **Every 80 game ticks** (`gameTime % 80 == 0`):
  - If the beam is not empty, `levels = updateBase(...)`.
  - If levels > 0 and the beam is not empty, `applyEffects` runs and **`block.beacon.ambient`** plays.
- **When a scan finishes:**
  - `lastCheckY = minY−1` and `beamSections = checking`.
  - On the server, it compares levels at the start of this tick (before the 80-tick update) with levels now.
  - 0 → >0: plays **`block.beacon.activate`** and triggers `CONSTRUCT_BEACON` for each ServerPlayer in `AABB(x,y,z → x,y−4,z).inflate(10,5,10)`.
  - >0 → 0: plays **`block.beacon.deactivate`**.
- `setRemoved` always plays `block.beacon.deactivate`.
- All beacon sounds use `playSound(null, pos, sound, BLOCKS, 1, 1)`.

**Pyramid levels** (`updateBase`)
- For layer 1..4 at y−layer (it stops if that is below minY), every block in the square x±layer, z±layer must be in `#beacon_base_blocks`: netherite, emerald, diamond, gold and iron blocks.
- levels = the last complete layer.

**Effects** (`applyEffects`, server only, needs a primary)
- Range r = levels*10+10. The box is `AABB(pos).inflate(r).expandTowards(0, level.getHeight(), 0)` and covers all `Player` entities.
- Duration = (9+levels*2)*20 ticks.
- Primary amplifier = 1 if levels ≥ 4 and primary == secondary, else 0.
- The secondary is applied at amplifier 0 if levels ≥ 4, it is set and it differs from the primary.
- Both use `MobEffectInstance(..., ambient=true, visible=true)`.

### Beam drawing (`BeaconRenderer`)
- Nothing is drawn when levels == 0: `getBeamSections()` returns an empty list.
- Texture `textures/entity/beacon/beacon_beam.png`.
- Each section is drawn from its start height. The **last** section is drawn 2048 high.
- The inner beam has radius 0.2 and the glow 0.25, both × `beamRadiusScale = max(1, horizontalDist/96)` (1 while scoping). The glow is drawn with alpha 32.
- `animationTime = floorMod(gameTime, 40) + partialTick`. The beam turns by `animationTime*2.25 − 45` degrees, and the texture V scrolls with time.
- Drawn off-screen too. View distance = render distance × 16, measured horizontally.

## 5. Loom (`LoomMenu`, type LOOM, opened through `SimpleMenuProvider`)

**Opening.** `LoomBlock` titles the menu `container.loom` and awards the stat `INTERACT_WITH_LOOM`. There is no block entity.

**Slots**
- 0 banner at (13,26): `mayPlace` = item is a `BannerItem` (the 16 `*_banner` items).
- 1 dye at (33,26): in `#loom_dyes` (= `#dyes`, the 16 dyes) **and** has the `minecraft:dye` component.
- 2 pattern at (23,45): in `#loom_patterns` **and** has `provides_banner_patterns`.
- 3 result at (143,57): `mayPlace` is false.
- inv 4..30 and hot 31..39 at (8,84).
- **Data slot 0** = the selected pattern index, −1 when nothing is selected.

**Pattern list** (`getSelectablePatterns`)
- With an empty pattern slot, the list is the `#minecraft:no_item_required` banner_pattern tag, in file order:
  - square_bottom_left, square_bottom_right, square_top_left, square_top_right
  - stripe_bottom, stripe_top, stripe_left, stripe_right, stripe_center, stripe_middle, stripe_downright, stripe_downleft, small_stripes
  - cross, straight_cross
  - triangle_bottom, triangle_top, triangles_bottom, triangles_top
  - diagonal_left, diagonal_up_right, diagonal_up_left, diagonal_right
  - circle, rhombus
  - half_vertical, half_horizontal, half_vertical_right, half_horizontal_bottom
  - border, gradient, gradient_up
- That is 32 patterns, and `base` is not among them.
- With a pattern item, the list is **only** that item's `provides_banner_patterns` holder set. Each item's tag holds one pattern:
  - flower→flower, creeper→creeper, skull→skull, mojang→mojang, globe→globe, piglin→piglin, flow→flow, guster→guster.
  - field_masoned→bricks, bordure_indented→curly_border.
  - This mapping comes from `Items.<clinit>` and `tags/banner_pattern/pattern_item/*.json`.
- The patterns themselves are `data/minecraft/banner_pattern/*.json` (asset_id and translation_key).

**When the inputs change** (`slotsChanged`)
- If the banner or the dye is empty: result EMPTY, list empty, selected −1, return without a broadcast.
- Otherwise it rebuilds the list:
  - If the list has exactly 1 entry, that one is selected (index 0).
  - Else if the old index was invalid, selected = −1.
  - Else the old pattern is kept when it is in the new list (with its new index), or selected = −1.
- If a pattern is chosen and the banner's `banner_patterns` already has **≥ 6 layers**: selected = −1 and the result is EMPTY. Otherwise `setupResultSlot`.
- Finally `broadcastChanges`.

**Result** (`setupResultSlot(p)`): with a banner, a dye and a `dye` colour present, the result is `banner.copyWithCount(1)` with `banner_patterns` = the existing layers plus `(p, dyeColor)`. It is set only if it differs (`ItemStack.matches`). The result is also computed on the client.

**Pattern click** (`clickMenuButton(id)`)
- If 0 ≤ id < list size: selected = id, `setupResultSlot`, return true.
- **It does not check the 6-layer cap again.**
- On the server it arrives through `handleContainerButtonClick` (same container id, not a spectator, `stillValid`), followed by `broadcastChanges`.

**Taking the result** (`LoomMenu$6.onTake`)
- 1 banner and 1 dye are removed. **The pattern item is never used up.**
- If the banner or dye slot is now empty, selected = −1.
- `ui.loom.take_result` plays at the loom (BLOCKS, 1, 1), at most once per game tick (`lastSoundTime`).

**Shift-click** (`quickMoveStack`)
- Result (3): `moveItemStackTo(4, 40, true)` and `onQuickCraft`.
- From slots 0, 1 or 2: (4, 40).
- Otherwise:
  - A BannerItem goes to (0, 1) only.
  - A dye item goes to (1, 2) only.
  - A pattern item goes to (2, 3) only.
  - **If that move fails, it returns EMPTY; there is no fallback to backpack or hotbar.**
  - Anything else: index 4..30 → (31, 40); index 31..39 → (4, 31).
- Then: if the stack is empty, `setByPlayer(EMPTY)`, else `setChanged`. If the count did not change, EMPTY. Otherwise `onTake`.

**Validity and closing.** `stillValid(access, Blocks.LOOM)`. `removed` returns the 3 inputs with `clearContainer`. The result container is not kept.

### Screen (`LoomScreen`)

**Sheet, title and empty-slot icons**
- 176×166, `textures/gui/container/loom.png`. `titleLabelY = 4`.
- A 16×16 icon is drawn on each empty slot: `container/slot/banner`, `container/slot/dye`, `container/slot/banner_pattern`.

**Pattern grid** (`displayPatterns`)
- Shown when banner and dye are present, the banner is under 6 layers and the list is not empty. This is recomputed only when one of the 3 input stacks changes.
- The grid is 4×4 cells of 14×14 at (60+col*14, 13+row*14). Index = (row+startRow)*4+col.
- Cell sprite: `container/loom/pattern_selected` for the selected index; `pattern_highlighted` when hovered; otherwise `pattern`.
- The hover tooltip key is `<pattern translation_key>.<dye colour name>`, for example `block.minecraft.banner.creeper.black`. The dye colour defaults to white.
- Inside each cell at (+4,+2), a 5×10 area is first filled with the GRAY dye's diffuse colour.
- Then that area gets the region u 0..21/64, v 1/64..41/64 of the pattern's 64×64 texture `textures/entity/banner/<asset path>.png`.

**Scroller**
- Drawn at (119, 13 + (int)(41*scrollOffs)), 12×15, sprite `container/loom/scroller`, or `scroller_disabled` unless there are more than 16 patterns.
- Mouse-down hit box: x 119..131, **y 9..65**.
- Dragging: `scrollOffs = (mouseY − top − 13 − 7.5)/41`, clamped to 0..1.
- The wheel subtracts `delta/(rows−4)` from scrollOffs.
- `startRow = max(0, (int)(scrollOffs*(rows−4)+0.5))`, where rows = ceil(n/4).

**Result preview and the 6-layer error**
- When there is a result and fewer than 6 layers: the banner flag model is drawn with the banner's base colour and the result's layers in the box (141,8)–(161,48).
- With ≥ 6 layers: the sprite `container/loom/error` (26×26) is drawn at (result.x−5, result.y−5) = (138,52).

**Clicking a pattern.** The client first runs `clickMenuButton` itself. If that returns true, it plays **`ui.loom.select_pattern`** as a local UI sound (pitch 1.0) and then sends the button click with the index.

## 6. Cartography table (`CartographyTableMenu`, type CARTOGRAPHY_TABLE)

**Opening.** `CartographyTableBlock` titles the menu `container.cartography_table` and awards the stat `INTERACT_WITH_CARTOGRAPHY_TABLE`. There is no block entity.

**Slots**
- 0 map at (15,15): `mayPlace` = has the `map_id` component. Any filled or explorer map qualifies; the menu does not use `#clonable_maps`.
- 1 additional at (15,52): `mayPlace` = `paper`, `map` (an empty map) or `glass_pane`.
- 2 result at (145,39) in a `ResultContainer`: `mayPlace` is false.
- inv 3..29 and hot 30..38 at (8,84).

**When the inputs change** (`slotsChanged`)
- If there is a result and the map or the additional slot is empty, the result is removed.
- Else, if both are present, `setupResultSlot` runs. It works **only on the server** (`access.execute`); the client's access is NULL.

**`setupResultSlot` order of checks**
- Look up the map data from `map_id`. If it is null, return with no change.
- `additional == paper` and `map ∈ #extendable_maps` (only `filled_map`) and not locked and `scale < 4`: result = `map.copyWithCount(1)` with `map_post_processing = SCALE`.
- Else `additional == glass_pane` and not locked: result = `copyWithCount(1)` with `map_post_processing = LOCK`.
- Else `additional == map`: result = `map.copyWithCount(2)`, a copy. **Locked maps can be copied.**
- Else: the result is removed.
- The result is set only if it differs.

**Taking the result** (`CartographyTableMenu$5.onTake`)
- 1 item each is removed from slot 0 and slot 1.
- `stack.getItem().onCraftedBy(stack, player)` → `MapItem.onCraftedPostProcess`. It removes `map_post_processing`, then on the server:
  - **LOCK** → `lockMap`: a new id from `getFreeMapId()`, data = `MapItemSavedData.locked()`. That is a copy with the same centre, scale, tracking and dimension, `locked=true`, the same colours, banner markers, decorations and tracked count. The stack's `map_id` is set to the new id.
  - **SCALE** → `scaleMap`: a new id, data = `scaled()` = `createFresh(centerX, centerZ, clamp(scale+1, 0, 4), tracking, unlimited, dimension)`. It is recentred and has **blank colours**.
- Then `ui.cartography_table.take_result` plays at the table (BLOCKS, 1, 1), at most once per game tick.
- `safeClone` (the creative pick) also calls `onCraftedBy`.
- `canTakeItemForPickAll` excludes the result container, so a double-click collect never takes the result.

**Shift-click** (`quickMoveStack`)
- Result (2): **first `onCraftedBy`** (the post-processing), then `moveItemStackTo(3, 39, true)` and `onQuickCraft`.
- From slot 0 or 1: (3, 39).
- Otherwise:
  - An item with `map_id` goes to (0, 1) only.
  - paper, map or glass_pane goes to (1, 2) only.
  - If that move fails, it returns EMPTY.
  - Index 3..29 → (30, 39); index 30..38 → (3, 30).
- Then: if the stack is empty, `setByPlayer(EMPTY)`. **`setChanged` runs every time.** If the count did not change, EMPTY. Otherwise `onTake` and `broadcastChanges`.

**Validity and closing.** `stillValid(access, Blocks.CARTOGRAPHY_TABLE)`. `removed` clears the result and returns slots 0 and 1 with `clearContainer`.

**Screen** (`CartographyTableScreen`)
- 176×166, `textures/gui/container/cartography_table.png`. `titleLabelY = 4`. No empty-slot icons.
- The screen reads the map data on the client.
- An "error" flag is set when the map is locked, or when paper is present and the map is not extendable or `scale ≥ 4`.
- The sprite `container/cartography_table/error` (28×21) is drawn at (35,31) when the map is locked with paper or glass, or for the paper case above.
- The preview (`extractResultingMap`):
  - Paper and no error: `scaled_map` (66×66) at (67,13); the map at (85,31), scaled 0.226.
  - Else an empty map: `duplicated_map` (50×66) at (83,13) with the map at (86,16), scaled 0.34; then a second `duplicated_map` at (67,29) with the map at (70,32), scaled 0.34.
  - Else glass: `map` (66×66) at (67,13); the map at (71,17), scaled 0.45; plus `locked` (10×14) at (118,60).
  - Else: `map` at (67,13); the map at (71,17), scaled 0.45.

## 7. Crafter (`CrafterMenu`, type CRAFTER_3x3; the fork has no MenuKind for it)

**Slots**
- `CrafterSlot` 0..8 at (26+col*18, 17+row*18). `mayPlace` = `!menu.isSlotDisabled(i) && Slot.mayPlace` (Slot.mayPlace accepts anything). Its `setChanged` → `menu.slotsChanged(container)`.
- inv 9..35 and hot 36..44 at (8,84).
- **45 = `NonInteractiveResultSlot`** at (134,35). It cannot be picked up, placed into, removed from or highlighted (`isFake`).

**Data** (10 values)
- Index 0..8 = slot state: 1 = disabled, 0 = enabled.
- Index 9 = triggered (1 when powered).

**Result preview** (`refreshRecipeResult`)
- Server only: `CrafterBlock.getPotentialResults(level, container.asCraftInput())` through the `RecipeCache`, then `recipe.assemble(input)` into the result.
- It runs once when the menu opens, and on **every** `slotChanged`, because the server-side menu is its own `ContainerListener`.

**Shift-click** (`quickMoveStack`)
- Index < 9: `moveItemStackTo(9, 45, true)`. Otherwise: `moveItemStackTo(0, 9, false)`.
- Then: if the stack is empty, `slot.set(EMPTY)` (**not** `setByPlayer`), else `setChanged`. If the count did not change, EMPTY. Otherwise `onTake`.
- The result slot (45) is never moved, because `mayPickup` is false.

**Validity.** `stillValid` → container (block-entity form).

**Slot toggle, client side** (`CrafterScreen.slotClicked`): this applies only to an empty `CrafterSlot` when the player is not a spectator.
- PICKUP: if the slot is disabled, it is enabled. Else, if the cursor is empty, the slot is disabled.
- SWAP (number key): if the slot is disabled and that hotbar slot holds an item, the slot is enabled.
- After that, the normal click is still sent.
- `updateSlotState` → `menu.setSlotState` locally plus `handleSlotStateChanged` → `ServerboundContainerSlotStateChangedPacket(slotId, containerId, newState)`.
- It plays the local sound `ui.button.click` at volume 0.4, pitch 1.0 when enabling and 0.75 when disabling (`Player.playSound`).

**Slot toggle, server side** (`handleContainerSlotStateChanged`)
- Not a spectator, matching container id, the menu is a `CrafterMenu` and its container is a `CrafterBlockEntity` → `setSlotState(slot, state)`. **There is no `stillValid` check.**
- `CrafterBlockEntity.setSlotState` acts only if the slot is 0..8 **and empty**. Then data = (enabled ? 0 : 1) and `setChanged`.
- `CrafterBlockEntity.setItem` into a disabled slot re-enables it first.

**Screen** (`CrafterScreen`)
- 176×166, `textures/gui/container/crafter.png`. Title centred.
- A disabled slot is drawn as the sprite `container/crafter/disabled_slot` (18×18) at (slot.x−1, slot.y−1) instead of the normal slot.
- A pointing-hand cursor shows over crafter slots.
- The tooltip `gui.togglable_slot` ("Click to disable slot") shows over an enabled, empty slot when the cursor is empty and the player is not a spectator.
- A redstone icon is drawn at screen centre (+9, −48), which is (97,35) relative to the screen: `container/crafter/powered_redstone` if data[9]==1, else `unpowered_redstone` (16×16).

**Block entity** (`CrafterBlockEntity`)
- Tags: `Items` or a loot table, `crafting_ticks_remaining`, `disabled_slots` (int array; on load a slot is applied only if it is empty), `triggered` (int), `CustomName`, `lock`.
- Name `container.crafter`.
- `canPlaceItem` (used by hoppers): false if the slot is disabled or the stack is full; true if the slot is empty; otherwise true only if no later enabled slot is empty or holds a smaller stack of the same item (`smallerStackExist`).
- Comparator signal (`getRedstoneSignal`) = the number of slots that are filled or disabled.

**Opening.** `CrafterBlock.useWithoutItem` calls `openMenu`. There is **no stat**.

**Triggering and crafting** (context for the triggered state)
- `neighborChanged`: on a rising edge, it schedules a tick in 4 ticks and sets TRIGGERED=true (flag 2) and data[9]=1. On a falling edge, it sets TRIGGERED and CRAFTING to false and data[9]=0.
- The tick → `dispenseFrom`:
  - If there is no recipe or the result is empty: level event **1050** (`block.crafter.fail`).
  - Otherwise: `crafting_ticks_remaining=6` and CRAFTING=true. The result and any leftover items are pushed into the container in front, or spawned 0.7 in front at speed 6. Each grid stack shrinks by 1.
  - It also triggers `CRAFTER_RECIPE_CRAFTED` for ServerPlayers in a 17³ box, and fires level events **1049** (`block.crafter.craft`) and **2010** (particles with the facing).

---

## 8. Asset list (sizes read from the jar)

**Sheets.** All are 256×256 in `textures/gui/container/`: dispenser, hopper, beacon, loom, cartography_table, crafter.

**Sprites** (in `textures/gui/sprites/`)
- **beacon:** button, button_disabled, button_highlighted, button_selected (22×22 each); confirm and cancel (18×18).
- **loom:** pattern, pattern_highlighted, pattern_selected (14×14); scroller and scroller_disabled (12×15); error (26×26).
- **loom empty-slot icons:** slot/banner, slot/dye, slot/banner_pattern (16×16).
- **cartography_table:** map and scaled_map (66×66), duplicated_map (50×66), error (28×21), locked (10×14).
- **crafter:** disabled_slot (18×18), powered_redstone and unpowered_redstone (16×16).

**Other textures**
- `textures/entity/banner/<asset>.png` (64×64).
- `textures/entity/beacon/beacon_beam.png`.
- `textures/mob_effect/<id>.png`.

**Language keys** (all present in en_us.json): `container.dispenser`, `container.dropper`, `container.hopper`, `container.beacon`, `container.loom`, `container.cartography_table`, `container.crafter`, `container.isLocked`, `block.minecraft.beacon.primary`, `block.minecraft.beacon.secondary`, `gui.togglable_slot`, `gui.done`, `gui.cancel`, `block.minecraft.banner.<pattern>.<colour>`.

**Sound ids** (checked in `SoundEvents`): `ui.loom.select_pattern`, `ui.loom.take_result`, `ui.cartography_table.take_result`, `block.beacon.activate`, `block.beacon.deactivate`, `block.beacon.ambient`, `block.beacon.power_select`, `ui.button.click`, `block.crafter.craft`, `block.crafter.fail`.

## 4. Overlaps with E3 and the M tracks

I read the code at `minecraft-survival` **20f1ce4e** (`git rev-parse --short HEAD`). The docs were also read with the owner's uncommitted working copy: `docs/round10/decisions.md` has new lines 50-51 (line 51 is the track C decision), and `docs/round10/runs/e3b.json`, `e3g.json` and `e3z.json` have changed. Other refs used: `track-e3` at 891ac88c, and e3a's merge **aa54bd0c**, which is in its full suite and not on main yet. I ran no cargo, wrote no files and made no commits. `track-c` and the `r10-c*-base` tags are both still free (round 9 had its own `docs/round9/track-c-log.md`).

## 1. Facts the C brief must be built around

- **The missing screens:** `crates/minecraft_survival/src/menus.rs:1976-1981` holds the `None` arm. It sits in `mirror_open` (`:1938-1983`), which is the friend-side mirror. On the host side, a block opens only through `Station::of` (`crates/minecraft_terrain/src/containers.rs:56-75`), `Opened::new` (`actions.rs:719-731`) or `Workshop::of` (`actions.rs:732-745`), so none of the six blocks opens anywhere.
- **Dispensers, droppers and hoppers are owned by the level (server thread), not by `Containers` (host thread):**
  - `containers.rs:135-137` (`owns_block_entity`) and the test at `:2470-2474` ("the level saves hopper/dispenser/dropper"), plus `:2356-2358`.
  - `docs/MINECRAFT-SURVIVAL.md:365` says the same.
  - Their items live in vendored `world/src/level/container.rs`: `replace_block_item` `:181`, `block_container_items` `:191`, `hopper_tick` `:320`, `has_ticker` `:483-487`.
  - The precedent for a level-owned screen is the lectern: `Command::Lectern` (`crates/minecraft_terrain/src/server.rs:4413`), `Output.lecterns` (`:4651`, filled at `:5214`) and `ServerHandle::lectern` (`:4979`).
- **"One container truth":** `server.rs:918` runs `leave_to_client`, which strips `Items` from the level's copy of chests, barrels, shulker boxes, furnaces and brewers (`containers.rs:139-146`). So hoppers can't feed those blocks (`docs/MINECRAFT-SURVIVAL.md:372`; E3's doors-8 is left out at `track-e3.md:286`).
- **`Station` keeps a fixed meaning: "the containers own these cells".** Its callers depend on that and spill `Containers` cells or keep shulker contents:
  - `actions.rs:1631`, `:1803`
  - `render_anim/src/minecraft_world.rs:1214-1245`
  - `render_anim/src/minecraft_levels.rs:483-513`
  - Adding level-owned blocks to `Station` would break those callers and E3's `placed-names` branch (below). C needs its own kind for them.

## 2. Planned groups of other tracks that touch C's files or blocks

| Group (track, segment, status) | Shared file:line at 20f1ce4e | What C does |
|---|---|---|
| **E3 `mount-viewers` and `mount-figure`** (E3-2, **e3b**, next E3 segment after aa54bd0c; `track-e3.md:977-1119`, files listed at `:1239-1244`) | See the e3b breakdown below this table | **Highest overlap. Start C's menus/containers work on a base that includes e3b** (see section 4). |
| e3a (merge aa54bd0c, in its full suite) | `menus.rs:45` and `hud/src/minecraft_screens.rs:887` get `#[allow(clippy::large_enum_variant)]` | Take a base after it: a new `Open` variant then needs no clippy fix, and terrain/survival start with no clippy warnings (e3b.json notes). |
| **E3 `placed-names`** (E3-8, **e3j**; `track-e3.md:3701-3760`) | See the e3j breakdown below this table | C's titles read the level block entity's `CustomName` and fall back to `container.dispenser`, `.dropper`, `.hopper` or `.beacon`. Keep level blocks out of `Station` so e3j's branch and `kept` stay as they are. Whichever merges second updates N-3 (see section 3). |
| E3 `placed-facing` (E3-8, e3i; `:3413-3492`) | Vendored `player/src/placement.rs` arms for dispenser, dropper, loom, crafter and barrel | C does not edit placement. Until e3i, hand-placed dispensers face north, and C's owner steps must allow for that. |
| E3 `dispenser-blocks` (E3-8, e3i; `:3600-3697`) | `world/src/level/dispenser.rs`: new arms after bone meal (`:122`) and `dispense_effects` before `container_set_item` (`:131`) | C does not edit `dispenser.rs`. The M lines there are M2's fish-bucket arm at `:111-118` (merged) and M1's optional hive arm after `honeycomb` `:120` (m1i); `OTHER_BEHAVIOURS` `:17-26` stays untouched. |
| **E3 `item-replace`** (E3-13, **e3r**; `:6392-6461`) | New `CommandWorld::replace_block_slots`, which covers only `Containers` cells. Its left-out list explicitly names dispensers and hoppers (`:6725`) | Whichever of C and e3r merges second adds the level-owned arm: dispenser/dropper 9 slots, hopper 5, crafter 9; a beacon gives "not a container". C will probably be first, so the orchestrator adds a note to `e3r.json`. |
| E3 `oven-book` (E3-13, e3s; `:6615-6713`) | `menus.rs` `book_grid` `:582`, `valid` `:610-627`, the `PlaceRecipe` branch `:999-1003`; hud `has_book` | C keeps off these lines. `has_book` has a wildcard, so new `McScreen` variants need no arm there. |
| E3 `sculk-sources` (E3-16, e3x; `:7978-8007`) | `menus.rs` `recheck_lids` `:1890` and `lid_effects` `:1903`; new `Command::GameEvent` beside `ChestOpeners` (`server.rs:4412`) | Vanilla posts no open or close game event for dispensers, droppers, hoppers, beacons, looms or cartography tables, so C adds no lid code. Put C's commands and outputs after `Lectern`/`BlockEntityComponents`/`VaultKey` (`server.rs:4413-4415`) and next to `lecterns` (`:4651`), not beside `ChestOpeners`. |
| **E3 `cartographer-maps`** (E3-10, **e3l**; `:4698-4825`) | `crates/minecraft_terrain/src/maps.rs` (store-backed `LootExplorer`, `Claims::merge`); `Globals.maps` (`crates/minecraft_survival/src/server.rs:46`) becomes `Arc<Mutex<MapStore>>`; `menus.rs:367-392` `loot_explorer`/`keep_explored` | The cartography table must take its new ids under `globals.maps.lock()` through `MapStore::free_id` (`maps.rs:1079`), never from a cached `next_id`, because after e3l the world thread takes ids from the same store. `.lock()` works with both types, so no wait is needed. Put C's lock/scale/copy functions beside `MapStore::create` (`maps.rs:1112`). Don't touch `menus.rs:367-392`. |
| E3 `map-banners` (E3-14, e3u; `:7148-7198`) | `maps.rs` `MapData.banners` (`:90`) becomes a typed `MapBanner` | javap 26.3 `MapItemSavedData.locked()` copies `bannerMarkers`, `decorations`, `trackedDecorationCount` and `colors`, but not frames. If C is first, its lock copies the raw `banners` list and e3u keeps that copy when it types the list. If e3u is first, C copies the `MapBanner`s. |
| E3 `map-frames` (E3-14, e3z; `:7202-7257`) | `maps.rs` `MapData.frames` (`:91`) becomes a typed `MapFrame` | A lock does not copy frames. Otherwise no shared lines. |
| E3 `component-limits` (E3-12, e3o; `:5443-5510`) | `CommandRegistries.banner_patterns`, read from `data/minecraft/banner_pattern` (`render_anim minecraft_world.rs:595`) | The loom needs the same pattern registry. Reuse e3o's list if it is merged; otherwise C writes one terrain helper and e3o reuses it (log it). |
| E3 `sculk-darkness-icon` (E3-16, e3w; `:7817-7859`) | `EffectGrant` (`crates/minecraft_terrain/src/warden_tracker.rs:177-182`) gets `visible`; `EffectInstance` (vendored `player/src/survival.rs:87`) gets `visible` and `show_icon`; `EffectView.hidden` | The beacon's effects (vanilla: ambient and visible) extend the same structs. Put any `ambient` field right after e3w's `visible`. Whichever merges second fills the literals. |
| E3 `trial-effects-players` (E3-12, e3z; `:6050-6097`) | `EffectKind` and `ALL` (`player/src/survival.rs:35-83`): four variants after `Darkness` | Players have no Speed, Jump Boost or Strength (`track-e3.md:7852`; `potions.rs:576`). A beacon gives only Haste, Resistance and Regeneration unless C adds the others. Any new `EffectKind` waits for M2's m2i and goes beside a neighbour other than `Darkness`. |
| E3 `friend-slot-changes` and the other friend groups (E3-4, e3d; `:1612`, `:1878-1961`) | They append `ToClient` variants (wire tags pinned by the codec tests); M2 appends `GameEvent` | C adds **no** `ToClient`/`ToServer` variant. The loom pick reuses `MenuAction::Choose(u16)` (`minecraft_net/src/messages.rs:205`). A beacon confirm becomes a new `MenuAction` variant right after `Choose`; no brief adds one. |
| E3 `sculk-pots-shelves` (E3-16, e3z; `:8204-8232`) | New vendored `level/pots.rs` and a survival→level command carrying a stack | It is the same bridge pattern as C's level containers. If C's command lands first, e3z can reuse it. Pick a `level/mod.rs` module line away from `pots`. |
| **M2 `nautiluses-taming` commit 3** (**m2h**; `track-m2.md:2731-2732`, `:2745`, table `:96-97`, protocol `:105`) | `nautilus` beside each `llama` (see the M2 breakdown below this table) | C's new `McScreen`, `MenuExtras` and `layout`/`sheet`/`draw_background` arms go next to related screens: Hopper/Dispenser/Dropper(/Crafter) beside `Chest`/`ShulkerBox`; Loom/Cartography beside `Stonecutter`; Beacon beside `Enchanting`. Never beside a `Mount` line. |
| M2 `dolphins-grace` and `guardians-curse` (**m2i**; `track-m2.md:35`, `:72`, `:1178`, `:1215`, `:1532`; `decisions.md:16`) | `EffectGrant.any_mode` and `cue`; M2 owns `EffectKind`/`ALL` until m2i (`track-e3.md:95`) | Vanilla `BeaconBlockEntity.applyEffects` gives the effect in every game mode, which needs M2's `any_mode`. **The beacon effect group waits for m2i.** No copy of M2's field (`decisions.md:14-17`). The beacon screen, levels and beam can ship before it. |
| **M1 `bees-hives` commit 1** (**m1i**, M1's last segment; `track-m1.md:98`, `:2664`, `:2886`, `:2913-2916`) | "Minecraft ticking block entities tick again after their chunk loads": `level/mod.rs` `insert_chunk` `:341-344` registers no ticker (only `ensure_block_entity` `:509-523` does); `container.rs` `has_ticker` `:483`; `piston.rs` `tick_block_entities` `:609` | **Hoppers in a loaded world never tick until this lands.** Recommended: C takes this commit under M1's exact subject. M1's rule (`track-m1.md:69`) then drops M1's copy, and the orchestrator adds a line to `m1i.json`. Otherwise C's hopper steps use hoppers placed in the same session and log loaded hoppers as waiting for m1i. |
| M1 `bees-honey` optional commit 3 (m1i; `:2918`, test `:2973`, kit `:2999`) | `dispenser.rs` after `"honeycomb"` | C does not touch it. Its steps can load a dispenser once C has merged (see section 3). |
| M1 shared sites (`track-m1.md:118-124`, `:131`) | `level/mod.rs` modules after `wither_build` and fields after `bell_rings`; `server.rs` `ItemOnBlock::HarvestHive` next to `FillBucket`, `ReleaseShoulder` beside `Reattach` (`:4407`); `dispenser.rs:93-95` (now `:120`) | Pick other neighbours. |
| **M3 `copper-golems-sorting`** (**m3c**, next after m3b, which is running; `track-m3.md:32`, `:1150-1169`, `:1188`, `:1282-1286`, table `:112`) | See the M3 breakdown below this table | Put C's level accessors beside `replace_block_item`/`block_container_items` (`container.rs:181-191`), not beside `container_at`. Stay off `lid_effects` and the lid lines. **Design clash to settle first:** m3c reads chest items through the `Level`, but `server.rs:918` strips them. So C's hoppers and M3's golems both meet the "one container truth" limit. Recommended: C moves items only between level-owned containers and keeps that limit, and the orchestrator either gives "one container truth" one owner before m3c or warns M3. |
| M3 copper chests (`track-m3.md:1026`) | `containers.rs:33-38` and `:62-64` already map copper chests (waxed too) to `Station::Chest` | Copper chests already open as chests: no C work and no edit on those lines. M3's statue and wake lines are `actions.rs:813`, `:1125` and `server.rs` `WakeStatue` beside `HangFrame`. |
| M3 module lines (`track-m3.md:110`) | `level/mod.rs`: `creaking_heart` after `container`, `lightning` after `fire`, golem modules after `golem_build`, `dried_ghast` after `ice` | A beacon module (if any) goes after another neighbour, for example `lectern` or `burning`. |

**e3b's edits in detail:**
- `mount-viewers`:
  - `menus.rs` Mount branch of `apply` `:1022-1032`, `stack_at` `:473`, `contents` `:1410-1439`, `mount()` `:2026-2080`.
  - `Menus.mount_updates` `:122`, `:141` is removed.
  - `Containers` (`containers.rs:603-613`) gets `mounts` and `changed_mounts` beside `chests`.
  - The calls at `render_anim/src/minecraft_world.rs:1357` and `minecraft_players.rs:2199` change.
- `mount-figure`:
  - `hud/src/minecraft_screens.rs:7` (`MOUNT_BOX`); `draw_background` `:493-512` gets a Mount arm beside `Inventory`.
  - `hud/src/minecraft_inventory.rs:643-649` and `:717-734`.
  - `frame/src/minecraft_ui.rs:570-577`.
  - `render_anim/src/minecraft_world.rs:147-201` and `:1517-1545`.
  - `minecraft_remote.rs:2015-2038`, `client_mobs.rs`, `entity_view.rs`, `render_gpu` drawsurf.

**What C takes from e3b:** use e3b's shared-copy pattern for level containers instead of a second one: one copy per block for all viewers, a changed set written back once through a server command, and the copy forgotten when nobody views it. If C starts before e3b is merged:
- New `Open` arms go before the `Open::Merchant` arms, not beside `Open::Mount`.
- New `Containers` fields go after `campfires` (`:609`), not beside `chests`.
- `draw_background` arms go only in its second match, beside `Stonecutter`/`Enchanting`.
- The per-frame call goes beside `open_station` (`minecraft_world.rs:1373`, `minecraft_players.rs:1847`).

**e3j's edits in detail:**
- World `level/components.rs:24-29` (`implicit`) and `:73-83` (`dropped_components`) gain names for hopper, dispenser, dropper, crafter, enchanting table and beacon.
- The `menus.rs:331` title.
- `containers.rs` `name_placed`/`name_value` beside `place_shulker` (`:913`).
- `actions.rs` `Station::of` branch (`:1625-1640`) and harvest `kept` (`:1799-1810`).

**M2's nautilus lines in detail:**
- `menus.rs`: `:1656-1663` (snapshot Mount extras, `:1662`), `mirror_mount` `:1985-2007` (`:1991`, `:2004`), tests `:3053`, `:3097`.
- `frame/src/minecraft_ui.rs:86-92`.
- `minecraft_net/src/messages.rs:614-620`.
- `render_anim/src/minecraft_inventory.rs:791`, `:859-861`.
- `hud/src/minecraft_screens.rs:240-258` (layout), `:328` (sheet `"nautilus"`), tests `:1188-1219`.
- `minecraft_net/src/codec.rs:606`, `:873`, `:892`.
- The `server.rs:4405` `MountInventory` line.

**M3 m3c's edits in detail:**
- Vendored `container.rs`: `chest_positions_in` beside `container_at` (`:128`).
- `World` methods beside E2's `chest_open_count`.
- `menus.rs:1903` `lid_effects` becomes `pub(crate)`.
- Survival `level.rs:24` gets `golem_openers`.
- `render_anim/src/minecraft_world.rs:1400` gets `golem_chests::lids` after `recheck_lids`.

**Lines no brief names, free for C:**
- `menus.rs`: `mirror_open`'s `None` arm `:1976-1981`, `station_title`/`bench_title` `:247-269`, `kind_of` `:1467-1492`.
- `containers.rs`, after the `Station`/`Opened` code (`:46-109`, `:294`). Use a new type for level-owned containers.
- `crates/minecraft_terrain/src/stations.rs` `Workshop` (`:23`): Loom and Cartography beside `Stonecutter`.
- `actions.rs:719-745`.
- `messages.rs` `MenuKind` `:567-590`. It already has all six variants; a `Crafter` goes beside `Dispenser`/`Dropper` and gets a "## Protocol" line.
- `MenuExtras` beside `Enchanting`/`Anvil` (`:596-603`).
- `render_anim/src/minecraft_inventory.rs` `screen_of` `:1018-1027`. Its wildcard `_ => McScreen::Chest` must not swallow new kinds.

**Not an owner overlap, but worth a line in the C brief:** many items have no dispense behaviour and stay inside with an `unsupported` log line (`dispenser.rs:17-26`, `:132-135`): arrows, snowballs, eggs, potions, fire charges, wind charges, TNT, spawn eggs, boats, minecarts, shulker boxes and armour equipping. No brief owns them; dispensers acting on mobs is left for later in `track-m1.md:905`, `track-m2.md:2933`, `:2964` and `track-m3.md:2794`, `:2825`. C's dispenser steps should load items whose behaviour exists: any plain block (default drop), water and lava buckets, fish buckets, bone meal, honeycomb and rockets.

## 3. E3 groups whose owner steps use a world prepared in vanilla, and other steps blocked by the missing screen

E3 runs its segments one after another (e3b is next; e3i is the 9th and e3k the 11th; `track-e3.md:35-62`). C's dispenser segment will likely merge before e3i and e3k start. The orchestrator then adds a note to their run specs (a run never changes its own brief): "the owner may build the stations in game and load the dispensers through the new screen; the prepared world stays as a fallback".

1. **`dispenser-blocks` (E3-8, e3i):**
   - The cause bullet "Nobody can load a dispenser in game yet" is `track-e3.md:3610`.
   - "Prepare the world in vanilla Minecraft 26.3" (world `e3-dispensers`, stations A-H) is `:3671-3682`. Steps D-2 to D-7 and D-MP (`:3686-3692`) use it; D-1 (`:3685`) does not.
   - Run spec: `docs/round10/runs/e3i.json:25`.
   - In game it needs `placed-facing` (same segment, first group) for east-facing and downward dispensers. A downward one is placed while looking up, or against a block's underside (F-1, `:3472`).
   - The kit (`:3695`, 15 stacks) would need `minecraft:bone_meal` (stations B and C), `minecraft:grass_block` and 4 more levers (8 stations).
2. **`rocket-end-targets` (E3-9, e3k):**
   - RE-2 is `:3989-3992`, and "Prepare the world" (`e3-rockets`, stations R and S) is `:4000-4005`.
   - Run spec: `e3k.json:13`.
   - The kit (`:4008`) already has the red rockets, plain rockets and cobblestone; add `minecraft:dispenser*2` and `minecraft:lever*2`.
3. **`angled-rockets` (E3-9, e3k):**
   - AR-1 (`:4129-4132`) uses station S; AR-F is `:4134`.
   - Run spec: `e3k.json:21`.
   - The kit (`:4137`, 4 stacks) needs a dispenser and a lever.
4. **`placed-names` (E3-8, e3j), step N-3 (`:3752`):** it says "They have no screens in this build". Once C has merged, the step opens the renamed hopper and dispenser and checks that their titles read `Feed` and `Gun`. Not a prepared world, but its wording depends on C.
5. **Gap-table row doors-13 (`:291`):** its left-out reason "Dispensers can't be loaded in game" no longer holds. It stays left out for its other reason (fire charges and wind charges are not dispensed).

The same block in other tracks and the release file:
- M2's merged `water-buckets` step 6 (`track-m2.md:416`) was dropped from release 7 (`docs/round10/release-7-tests.md:290`, `:634`).
- W's rocket-from-dispenser vibrations (`release-7-tests.md:626`).
- M1 m1i's optional dispenser hive harvest has no owner step yet (`track-m1.md:2918`, kit `:2999`).

The round 7 hopper/dropper save test also used a vanilla-prepared world (`docs/round7/track-d-log.md:357-359`).

## 4. What C must wait for, or how to keep merges small

- **Base:** take C's base after e3a's merge aa54bd0c reaches main. Strongly prefer a base after **e3b**, because `menus.rs` `apply`/`contents`/`mount`, `Containers` and `draw_background` change there. This is a wait like m3d's (`decisions.md:43`). If C must start before e3b, use the separate anchors in section 2, and whichever merges second keeps both sides.
- **Beacon effects wait for M2's m2i** (`any_mode`, `EffectKind`), and best also for E3's e3w (`visible`). The beacon screen, its levels and its beam can come first.
- **Hopper tickers:** take M1's m1i "tick again after their chunk loads" commit under M1's subject (and note it in `m1i.json`), or log loaded hoppers as waiting for m1i.
- **One container truth:** decide one owner before M3's m3c. C should not change it alone, because e3b, e3j, e3r and e3s all edit `Containers` while it is the host-side store.
- **Running alongside C:** C will likely overlap m3c (container.rs, `lid_effects`) and m2h (the nautilus lines). Keep to the separate anchors above.
- **Protocol:** a new `MenuExtras` or `MenuAction` variant, or a `MenuKind::Crafter`, gets a "## Protocol" line in the C log, with no bump (`decisions.md:7`).
