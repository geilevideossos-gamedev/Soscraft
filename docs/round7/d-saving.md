# Track D: saving everything (design brief)

Goal: save what vanilla 26.3 saves, in its format, so `iw4l-artifacts\saves\<world>` opens in real Minecraft. Paths are as of 29f722d. After the crate split, the player and container code below moves to `crates/minecraft_survival`; names are given so the plan survives the move. All player state is keyed by UUID and owned by the server.

## 1. What exists today

- **Storage**: `LevelStorage` (`third_party/minecraftoss/world/src/level_storage.rs`) writes `level.dat` (:240), saved data `data/minecraft/<id>.dat` (:169-182), the clocks, the game rules and `players/data/<uuid>.dat` (:404), each through `safe_replace` with a `_old` copy. The NBT library is `minecraftoss_core::nbt::Tag`, and `DATA_VERSION` is 5023.
- **Player file**: `WorldSave::write_host` (`render_anim/src/minecraft_save.rs:109`) merges only the game mode and `abilities` (`game_mode.rs:183`). It runs from `autosave` (`minecraft_world.rs:2393`), `save_player_mode` (:2358) and `save_and_close` (:2458).
- **Not saved**: `Entities.inventory` and `selected` (`minecraft_entities.rs:69-71`; slots 0-35 storage, 36-39 feet to head, 40 offhand, 41 body, 42 saddle), `Survival.status` (`minecraft_survival.rs:92`), position, rotation and weather. The kit is given on every load (`minecraft_entities.rs:213`).
- **Containers** (`minecraft_terrain/src/containers.rs:428`):
  - `Containers` holds chests, barrels and shulker boxes, `Oven`, `Brewer` and `Campfire` in `HashMap`s, plus one shared `ender` list (:431). It lives in `Runtime.containers` (`minecraft_world.rs:146`) and is rebuilt empty on every load.
  - Structure loot is rolled on the first open from the chunk's `LootTable` (`minecraft_world.rs:1705-1714`), and nothing is written back. **Bug:** after a restart a looted chest fills again.
- **Chunks**:
  - `write_chunk` (`core/src/chunk_nbt.rs:181`) already writes `block_entities` from `chunk.block_entities`. `BlockEntities::load_and_save` (`core/src/block_entity.rs:98-328`) knows the vanilla layout of every container.
  - The chunk map only takes block-state edits (`ChunkMap::set_blocks`, `world/src/chunk_map.rs:762`, `apply_edits` :1005), so player containers never reach disk and the block entities of broken blocks stay in the file.
- **Server level** (`minecraft_terrain/src/server.rs`):
  - Its own chunk copies hold block entities that hoppers, dispensers and droppers change (`world/src/level/container.rs`). None of that reaches the chunk map.
  - Mobs, items and orbs are saved per chunk (`save_chunk_entities` :456).
  - **Bug:** item components travel as a JSON string (`stack_of` :179), so `level_entity_tags` (`server_mobs.rs:1285`) writes `Item.components` as a string. Vanilla rejects that stack.
- **Weather**: kept in `level/time.rs:20` `Weather` (`clear_time`, `rain_time`, `thunder_time`, `raining`, `thundering`) and never saved. `/time pause` is not saved either (`ClockState.paused` is always false, `minecraft_save.rs:168`).

## 2. Vanilla 26.3, checked with javap

- **Player** (`Player.addAdditionalSaveData`): `Inventory` (`ItemStackWithSlot`: a `Slot` byte plus `id`, `count`, `components`, for the 36 main slots only), `SelectedItemSlot`, `XpLevel`, `XpTotal`, `XpSeed`, `Score` (ints), `XpP` (a float), `SleepTimer`, `abilities`, `EnderItems` (27 slots), and `LastDeathLocation` when set.
- **Food** (`FoodData`): `foodLevel`, `foodTickTimer` (ints), `foodSaturationLevel`, `foodExhaustionLevel` (floats).
- **Living** (`LivingEntity.addAdditionalSaveData`): `Health`, `AbsorptionAmount` (floats), `HurtTime`, `DeathTime` (shorts), `attributes`, `active_effects` (left out when empty; `effect_tag`, `server_mobs.rs:987`, already writes this layout), and `equipment`.
  - `equipment` is `EntityEquipment.CODEC`: slot name (`head`, `chest`, `legs`, `feet`, `offhand`, `body`, `saddle`) to `ItemStack`, left out when empty.
  - `PlayerEquipment` routes `mainhand` to the selected slot, so armour and offhand are not in `Inventory`.
- **Entity** (`Entity.saveWithoutId`): `Pos` and `Motion` (doubles), `Rotation` (floats), `fall_distance` (a double, no longer `FallDistance`), `Fire`, `Air` (shorts), `OnGround`, and `UUID` (4 ints).
- **ServerPlayer**: `Dimension` string, and `respawn` (`RespawnConfig.CODEC`: `dimension`, `pos`, `yaw`, `pitch` and an optional `forced`). It also writes `recipeBook` (`recipes`, `toBeDisplayed`), `seenCredits` and `entered_nether_pos`.
- **Host file**: `PlayerList.loadPlayerData` loads `players/data/<singleplayer_uuid>.dat` for the singleplayer owner, whatever the account's UUID, and `PrimaryLevelData` has no `Player` key. Our host file is exactly what real Minecraft loads.
- **Weather**: `WeatherData`, in `data/minecraft/weather.dat`. All five fields are required: `clear_weather_time`, `rain_time`, `thunder_time`, `raining`, `thundering`.
- **Block entities** (`saveAdditional`):
  - Chest, barrel, shulker box, hopper: `LootTable` or `Items`, `CustomName`; the hopper adds `TransferCooldown`.
  - Furnaces: `cooking_time_spent`, `cooking_total_time`, `lit_time_remaining`, `lit_total_time`, `speed_multiplier`, `Items`, `RecipesUsed`.
  - Brewing stand: `BrewTime`, `total_brew_time`, `Items`, `Fuel`, `total_fuel`, `speed_multiplier`.
  - Campfire: `Items`, `CookingTimes`, `CookingTotalTimes`.
  - The ender chest saves no items; they belong to the player.

## 3. Commits

1. **Minecraft dropped items save their components as vanilla NBT**
   - Move `tag_json`/`json_tag` (`server_mobs.rs:930-962`) into a shared `item_nbt.rs`.
   - `stack_of` and `components_of` (`server.rs:179-191`) convert JSON to and from a compound. Loading still accepts the old string form.
2. **Minecraft weather is saved like vanilla**
   - Add `LevelStorage::read_weather`/`write_weather` (the five keys) and `ServerSim::weather_data`/`set_weather_data`. `ServerHandle` returns them with the `WeatherView`.
   - Written by `WorldSave::write` and loaded in `Entities::new`. `IW4L_MINECRAFT_WEATHER` overrides it only when set.
3. **Minecraft /time pause survives a restart**: pass `DayCycle`'s paused flag into `ClockState` (`minecraft_save.rs:168`).
4. **Minecraft player files hold the inventory, armour and offhand like vanilla**
   - New `player_nbt.rs` with `PlayerData` (inventory, selected slot, ender chest and the stats below) and `to_tag`/`apply`.
   - `WorldSave::write_player(uuid, &PlayerData)` replaces `write_host`. It merges, keeps unknown keys and removes keys we own that are now empty.
   - Slots 0-35 go to `Inventory` and 36-42 to `equipment`.
5. **The Minecraft kit only goes to a player without a saved inventory**: `minecraft_entities.rs:213` checks that `Inventory` is missing, not that the file is (round-6 worlds already have mode-only files), and logs `kit skipped`.
6. **Minecraft health, hunger, experience and effects are saved**: `Health`, `AbsorptionAmount`, the food keys, `Xp*`, `Score`, `Air`, `Fire` and `active_effects` (`u32::MAX` saved as -1 for infinite). It needs a small vendor accessor for the private `FoodData.tick_timer` (`survival.rs:61`). Absorption is restored after the effects, since `add_effect` raises it, and health ≤ 0 loads as a fresh respawn.
7. **Minecraft players come back where they left, facing the same way**
   - `Pos`, `Rotation` (the inverse of `minecraft_yaw`, `minecraft_world.rs:372`), `Motion`, `OnGround`, `Dimension` and `UUID`.
   - The first spawn after a load queues a teleport through the `/tp` path (`minecraft_world.rs:1606`), plus a new `Frame::teleport_facing` beside `teleport` (`sim/src/frame.rs:390`), modelled on `script_player.rs:46`.
   - `SpawnGate` keeps the loading screen up until the target chunk has collision.
   - A creative player saved in the air keeps `abilities.flying`.
8. **Each Minecraft player has their own ender chest**: `Containers.ender` moves into `PlayerData` as `EnderItems`, and `Containers::cells` takes the opener's list.
9. **Minecraft respawn points are saved**
   - `RespawnConfig` round-trips `respawn` and `LastDeathLocation`, and an MW2 respawn teleports there when it is set.
   - Setting it needs beds or `/spawnpoint` (tracks B and C).
10. **Changed Minecraft block entities are saved with their chunk**
    - Vendor patch: `ChunkMap::set_block_entities(&[(pos, Option<Tag>)])` next to `set_blocks`. The latest change wins, it is applied in `write_edits`, and it marks the chunk unsaved.
    - `apply_edits` also drops a block entity whose block no longer has that type.
    - Add `TerrainStream::record_block_entities` beside `record_edits` (`terrain.rs:1307`).
11. **Minecraft chests, barrels and shulker boxes keep their contents**
    - New `Containers::block_entity_tag(pos)` and `import(pos, &Tag)`, plus a `dirty` set that is drained every frame into `record_block_entities`.
    - The open path reads `Items` before `LootTable`. Rolling the loot writes `Items` and drops `LootTable`, which fixes the refill. `CustomName` comes from `names`.
12. **Minecraft furnaces, brewing stands and campfires keep cooking after a restart**
    - `Oven`, `Brewer` and `Campfire` to and from NBT. `Oven.experience` becomes a `RecipesUsed` map, with experience worked out when the output is taken.
    - They are imported when their chunk loads, tick only in loaded chunks (as in vanilla), and are flushed and forgotten when the chunk unloads.
13. **Minecraft hoppers, dispensers and droppers keep their items**: `ServerSim::take_block_entity_changes` sends the server level's changed block entities (a dirty set in the vendored `level/container.rs`) to the chunk map, for the types `Containers` doesn't own.
14. **Minecraft worlds saved by iw4l open in vanilla 26.3**: an end-to-end test, plus the Saving section of `docs/MINECRAFT-SURVIVAL.md` (:130-138).

## 4. Unit tests

- `item_nbt`: enchantments, `custom_name`, nested `container` and `potion_contents` round-trip; old string components still load.
- `player_nbt`: keys and tag types exactly as in section 2 (`Slot` byte, only 0-35 in `Inventory`, `equipment` names, infinite effect -1); round-trip; a foreign `recipeBook` survives; empty `active_effects`, `equipment` and `respawn` are removed.
- Kit: a mode-only player file still gets the kit; a file with `Inventory` doesn't.
- Weather: `weather.dat` round-trips with the five keys and falls back to `_old`.
- Containers: `block_entity_tag` is unchanged by `BlockEntities::load_and_save` for every container type (synthetic, plus the real catalog under `MINECRAFTOSS_ROOT`); an opened loot chest saves `Items` and no `LootTable`.
- `ChunkMap`: a changed block entity comes back from a new `ChunkStorage`; a broken block's one disappears.
- Server: an enchanted dropped item saves a `components` compound. The yaw conversion runs both ways at 0, 90, 180 and -90.

## 5. In-game test

`.env`: `IW4L_MINECRAFT_WORLD=savetest`, plus:

```
IW4L_MINECRAFT_KIT=chest,barrel,red_shulker_box,furnace,brewing_stand,campfire,ender_chest,hopper,coal*64,raw_iron*64,beef*8,blaze_powder*4,nether_wart*8,glass_bottle*3,diamond_helmet,diamond_chestplate,diamond_leggings,diamond_boots,shield,diamond_sword+sharpness=5,golden_apple*4,experience_bottle*64,cobblestone*64,diamond_pickaxe+efficiency=5,water_bucket
```

1. Start. The log says `Minecraft kit: 25 stacks`. Type `/tp 100 80 100` and build there.
2. Put on the armour and put the shield in the offhand. Select hotbar slot 4. Throw XP bottles until level 5 or more. Eat a golden apple. Take some fall damage and sprint until hunger drops. **Screenshot 1:** the inventory. **Screenshot 2:** the HUD.
3. Place and fill a chest (a few stacks in odd slots), a barrel and the shulker box.
   - Furnace: 20 raw iron and coal; screenshot 3 while it cooks.
   - Brewing stand: fill the bottles with water, then add nether wart and blaze powder.
   - Campfire: beef. Ender chest: 3 stacks.
4. Drop the enchanted sword (Q) beside the chest. Type `/weather thunder` and `/time set night`. Look at the chest. Quit through the menu.
5. Restart.
   - Expected: same spot and view direction; same hearts, hunger, XP, effects, armour, offhand and selected slot. The log says `kit skipped`, and no new kit appears.
   - **Screenshots 4 and 5:** the same views as 1 and 2.
6. Check that the contents are all there: the furnace kept its progress and finishes, the brewing stand goes on, the campfire still has its beef, and the ender chest is the same. The sword lies on the ground with its glint. It is night and thundering. **Screenshots 6 onward.**
7. Open a village or structure chest, take one item and restart. The chest must not fill up again.
8. Optional: copy `saves\savetest` into `.minecraft\saves` and open it in Minecraft 26.3. You should find the same position, inventory, chests, furnace progress and weather.

## 6. Risks and overlaps

- **Crate split**:
  - Land this track after the split, so `PlayerData`, `item_nbt` and `Containers` are written once in `minecraft_survival`.
  - `Players: HashMap<Uuid, PlayerData>` gives multiplayer one file per friend (offline UUIDs, so a rename starts a new file).
- **Two container truths**:
  - The UI uses `Containers`, while hoppers and comparators read the server level's chunk NBT. Commit 13 only forwards the types `Containers` doesn't own.
  - Hoppers feeding chests needs both unified on the server (track B / "Finish survival"). Hoppers have no screen yet, so their contents can't be tested in game.
- **Vendor patches** (`chunk_map.rs`, `level/container.rs`, `survival.rs`) join the list of patched MinecraftOSS files.
- **Dimensions**: `Dimension`, `respawn.dimension` and per-level containers are ready for the switch. Until then, a non-overworld position falls back to the world spawn.
- **Merge writer**: keys we own must be deleted when empty, or stale effects and armour come back.
- **Types**: JSON to NBT loses tag types. Vanilla codecs accept any number, but `custom_data` comes back changed.
- **Behaviour changes**: ovens stop cooking in unloaded chunks, and furnace XP moves to `RecipesUsed`. A match end still resets XP.
- **Track C** (`/spawnpoint`, `/time`) and **track B** (beds, hoppers) touch the same code.
