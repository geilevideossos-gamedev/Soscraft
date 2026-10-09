# Track D log: saving

Branch `track-d`, worktree `C:\Dev\mc-wt\track-d`. Brief: `d-saving.md`.

A commit can't name its own hash, so each entry is written with its commit and its hash is filled in by the next commit; the last entry's hash is in `git log` under its subject.

## Group: world basics

### 1. Minecraft dropped items save their components as vanilla NBT

Commit: 33a04dd

- **Changed:**
  - New `minecraft_terrain/src/item_nbt.rs` holds `tag_json` and `json_tag`, moved out of `server_mobs.rs`, plus `components_tag`, `components_from_text`, `saved_components` and `components_text`.
  - The server level's item stacks now always keep their components as an NBT compound, the way vanilla's `DataComponentPatch` is saved. `stack_of`, `keep_dropped_contents` and the player world's `spawn_item`/`spawn_popped_item` convert the client's JSON into a compound, and `components_of` turns a compound back into JSON text for the client. Before, the level kept the JSON as an NBT string, `level_entity_tags` wrote `Item.components` as that string (vanilla rejects the stack), and a stack loaded with a compound reached the client as Rust debug text.
  - `load_level_entity` reads a compound or the old string form, and `level_entity_tags` always writes a compound.
  - `glint.rs` also reads `enchantment_glint_override` saved as a byte (0 or 1), since NBT has no booleans.
- **Tests:** `item_nbt`: `enchantments_and_a_name_become_a_compound_of_ints_and_strings`, `a_shulker_box_keeps_its_nested_items_and_their_potions`, `a_plain_potion_round_trips_through_its_text`, `components_saved_as_text_by_older_builds_still_load`, `booleans_become_bytes_and_large_numbers_stay_whole`. `server`: `an_enchanted_dropped_item_saves_its_components_as_vanilla_nbt` (save, load, and an old string-form save that is written back as a compound). `glint`: `a_glint_override_saved_as_a_byte_still_counts`. `a_shulker_box_the_server_blows_up_drops_once_with_its_contents` now spells its contents in NBT key order, which is the order the JSON text now comes back in.
- **Decisions:**
  - A stack loaded from disk keeps its NBT exactly, so its tag types (bytes, shorts, int arrays) survive. Only a stack that passed through the client's JSON is rebuilt from JSON: whole numbers become ints (a long when they don't fit), other numbers doubles and booleans bytes. That is what vanilla's codecs accept; `custom_data` can come back with other number types (brief risk "Types").
  - Empty component maps are left out, as vanilla leaves out an empty `components`.
  - Server-level stacks that carry components now merge and stack by their real components, because every one of them holds the same compound form.
- **Clippy:** `minecraft_terrain` has no clippy lints set and was not clippy-clean before this track (`approx_constant` errors in the mob renderers, `type_complexity` and `unwrap_or_default` warnings). None of the findings is on a line this commit touches.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=savetest` (any name but `temp`) and the kit below. Start.
  2. Put the cobblestone into the shulker box and break the box, so it drops with its contents.
  3. Throw (Q) the sword, the pickaxe and the enchanted book next to the shulker box. **Screenshot 1:** the four items on the ground; the sword, pickaxe and book glint.
  4. Quit through the menu and start again with the same world. **Screenshot 2:** the same four items in the same place, still glinting.
  5. Pick them up. Hover the sword (Sharpness V), the pickaxe (Efficiency V) and the book (Mending). Place the shulker box and open it: the cobblestone is inside. **Screenshot 3:** the sword's tooltip; **screenshot 4:** the open shulker box.
  6. Optional: copy `iw4l-artifacts\saves\savetest` into `.minecraft\saves` and open it in Minecraft 26.3. Before this commit vanilla threw the dropped sword away; now it lies there enchanted.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword+sharpness=5,minecraft:diamond_pickaxe+efficiency=5,minecraft:enchanted_book+mending,minecraft:red_shulker_box,minecraft:cobblestone*64
```

### 2. Minecraft weather is saved like vanilla

Commit: a8ed2a4

- **Changed:**
  - Vendored `level_storage.rs` (MinecraftOSS patch): `WeatherData` and `LevelStorage::read_weather`/`write_weather`. The file is `data/minecraft/weather.dat`, a saved-data file like `world_clocks.dat` (`data` compound plus `DataVersion`) with the five keys of 26.3's `WeatherData.CODEC` (checked with javap): `clear_weather_time`, `rain_time`, `thunder_time` (ints), `raining`, `thundering` (bytes). All five are required, as in vanilla, and a broken file is read from its `_old` copy.
  - `ServerSim::weather_data`/`set_weather_data` (`minecraft_terrain/src/server.rs`), and `Output::weather_data` after every tick beside the `WeatherView`. Loading copies vanilla's `ServerLevel.prepareWeather`: when the save says it rains, the rain level starts at 1 (and the thunder level when it also thunders), so the world opens already raining instead of fading in.
  - `save.rs`: `OpenedWorld::weather` is read by `open_world` (an unreadable file gives a warning and the weather starts again), and `WorldSave::write_weather`.
  - `server.rs`: `Globals::weather` holds the world's weather, as vanilla's `MinecraftServer` owns `WeatherData`. `autosave` and `Server::save_and_close` write it.
  - `entities.rs`: `Entities::new` takes the saved weather and logs `Minecraft weather: <clear|rain|thunder> from weather.dat`. `IW4L_MINECRAFT_WEATHER` is applied after it, so it overrides the save only when it is set; an empty value now counts as not set. `Entities::weather_data` follows the server's output.
  - `render_anim/src/minecraft_world.rs`, three lines: `Globals` gets `opened.weather` in `load()`, `Entities::new` gets `globals.weather`, and `globals.weather` follows `entities.weather_data` after each `entities.tick`.
- **Tests:** `level_storage`: `weather_lives_in_data_minecraft_weather_dat_with_its_five_keys` (path, `DataVersion`, exact keys and tag types, `_old` fallback, a file missing a key is an error). `server`: `saved_weather_comes_back_already_raining_and_keeps_counting_down`. `save`: `the_weather_survives_reopening_a_world` (round trip, broken file and its `_old` give one warning, `temp` has none). `entities`: `saved_weather_is_named_like_the_weather_command`.
- **Decisions:**
  - The weather belongs to the server, not to a level: in 26.3 `MinecraftServer.getWeatherData()` is one file for the world, and only levels that `canHaveWeather` use it. Today it is mirrored from the overworld's server thread; the dimension switch can keep `Globals::weather` and feed it to the overworld level only.
  - The weather now also carries over from one MW2 match on the Minecraft map to the next, because each match loads the world from disk. Before, every match started with fresh weather.
  - `IW4L_MINECRAFT_WEATHER` still sets weather that lasts 1,000,000,000 ticks, as before. That weather is now saved like any other, so after removing the line, `/weather clear` brings back the normal cycle.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=savetest` and remove the `IW4L_MINECRAFT_WEATHER` line (or leave it empty). Start.
  2. Type `/weather thunder`. Wait until it rains and thunders. **Screenshot 1.**
  3. Quit through the menu. `iw4l-artifacts\saves\savetest\data\minecraft\weather.dat` now exists. Start again.
  4. Expected: it is already raining and thundering when the loading screen goes away (no slow fade-in), and the log says `Minecraft weather: thunder from weather.dat`. **Screenshot 2.**
  5. Type `/weather clear`, quit and start again: the sky is clear, and the log says `Minecraft weather: clear from weather.dat`.
  6. Add `IW4L_MINECRAFT_WEATHER=rain` to the `.env` and start again: it rains, and the log says `Minecraft weather: rain from IW4L_MINECRAFT_WEATHER`. Remove the line, start again and type `/weather clear`.
  7. Optional: open the world in Minecraft 26.3 after step 3: it is still storming there.
- **Kit:** none needed; any kit works.

### 3. Minecraft /time pause survives a restart

Commit: 2c5e8d3

- **Changed:**
  - `WorldSave::write` takes the whole `DayCycle` instead of its ticks, and the overworld clock in `data/minecraft/world_clocks.dat` gets its `paused` flag from it. Before, `paused` was always written as false. The key and its type (`Codec.BOOL`, a byte) are those of 26.3's `ClockState.CODEC`, checked with javap.
  - `OpenedWorld::time_paused` is read from the overworld clock, and `load()` in `render_anim/src/minecraft_world.rs` sets `day.paused` from it (one line).
  - `autosave` and `Server::save_and_close` pass `globals.day`.
- **Tests:** `save`: `a_paused_clock_stays_paused_when_the_world_is_opened_again` (the file holds `paused: 1b`, the world opens paused at the same tick, a resumed clock is saved with `paused: 0b`, `temp` is never paused). `a_new_world_keeps_its_seed_spawn_and_time_when_opened_again` and `unreadable_level_data_and_clocks_keep_the_world_and_its_seed` now pass a `DayCycle`.
- **Decisions:**
  - Only the overworld clock is written, as before. The clock's `rate` stays 1, because the game has no `/time rate`.
  - Known limit, unchanged by this commit: `level.dat`'s `Time` (game time) is counted from the day clock, so it stands still while time is paused, where vanilla keeps counting it. It only shows in `/time query gametime`.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=savetest` and remove any `IW4L_MINECRAFT_TIME` line (it sets the time at every start). Start.
  2. Type `/time set noon`, then `/time pause`. Wait a minute: the sun does not move. **Screenshot 1.**
  3. Quit through the menu and start again.
  4. Expected: still noon, and the sun still does not move after a minute. `/time pause` answers that time is already paused. **Screenshot 2.**
  5. Type `/time resume`, quit and start again: the sun moves again.
  6. Optional: open the world in Minecraft 26.3 after step 3: time stands still there too.
- **Kit:** none needed; any kit works.

## Group: player file

### 4. Minecraft player files hold the inventory, armour and offhand like vanilla

Commit: (filled in by the next entry)

- **Changed:**
  - New `minecraft_survival/src/player_nbt.rs`: `PlayerData` is what a player file holds, with `read` (from the file's NBT) and `entries`/`merged` (to it). Slots 0-35 go to `Inventory` as `ItemStackWithSlot` (a `Slot` byte, then `id`, `count` as an int and `components` as a compound), and slots 36-42 go to `equipment` under `feet`, `legs`, `chest`, `head`, `offhand`, `body` and `saddle`, as 26.3's `EntityEquipment.CODEC` writes them (checked with javap: `Player.addAdditionalSaveData`, `Inventory.save`/`load`, `ItemStackWithSlot.CODEC` with `ExtraCodecs.UNSIGNED_BYTE`, `ItemStack.MAP_CODEC` with `count` in 1-99, `EquipmentSlot` names). `SelectedItemSlot` is an int.
  - The merge writer keeps every key it does not own (`recipeBook`, `XpSeed`, `seenCredits`, ...) and first removes the keys it owns, so armour that was taken off leaves `equipment`, and an empty `equipment` is left out as vanilla leaves it out. `Inventory` is always written, even empty, as vanilla does.
  - `save.rs`: `WorldSave::read_player(uuid)` and `write_player(uuid, &PlayerData)` replace `host_mode`/`write_host`; `default_mode` is the level's game type for a player without a file.
  - `server.rs`: `join` builds a joining player from their file (game mode, inventory and selected slot), for any UUID; an unreadable file gives a warning and the player starts again. `save_and_close` and `autosave` write the file of every player in `Server.players`, and a game mode change (`save_player`) writes the whole file instead of the mode only. `autosave` now queues every level's chunks.
  - `player.rs`: `Player::data` and `joining_inventory`.
  - `render_anim/src/minecraft_world.rs`: the load completion calls `server::join`; the autosave check moved to the top of the frame, where every level and player can be borrowed (it runs one frame later than before); `close_world` no longer passes the local UUID.
  - `commands.rs`: one line, a game mode change saves the player through `server::save_player`.
- **Tests:** `player_nbt`: `storage_slots_go_to_inventory_with_a_slot_byte_and_armour_to_equipment`, `the_inventory_comes_back_from_its_file_in_the_same_slots`, `guns_and_empty_stacks_are_not_saved`, `an_empty_inventory_is_still_written_and_read_back_as_present`, `merging_keeps_unknown_keys_and_drops_armour_that_was_taken_off`, `slots_read_as_unsigned_bytes_and_bad_entries_are_skipped`, `counts_are_ints_capped_at_vanilla_s_limit_and_old_string_components_load`, `a_selected_slot_outside_the_hotbar_reads_as_the_first`. `player`: `a_saved_inventory_comes_back_in_its_slots_with_the_world_recipes`, `a_player_saves_its_mode_and_slots_with_loose_stacks_put_back`. `server`: `a_joining_player_gets_the_inventory_and_mode_their_file_saved`, `an_unreadable_player_file_starts_the_player_again`, `a_temporary_world_gives_the_kit_and_the_survival_carried_over`. The three game-mode tests in `save` now go through `write_player`/`read_player`/`default_mode` (same assertions; the merge test also checks that a foreign `seenCredits` survives).
- **Decisions:**
  - MW2 guns (`iw4:weapon/<n>`) are not saved: the number is the match's weapon index and vanilla rejects unknown items. On every spawn `sync_weapons` puts the guns you own back into free hotbar slots, as before.
  - Stacks left on the cursor or in a crafting grid go into the saved inventory, as vanilla's menu does when a player leaves (`clearContainer`); the inventory in play is not changed.
  - Until commit 5 the kit is still added to free backpack slots on every start, also on top of a saved inventory.
- **Clippy:** `clippy -p minecraft_survival --tests --no-deps` has only the two older `entities.rs` findings (`type_complexity`, `manual_is_multiple_of`). Without `--no-deps` it stops in `minecraft_terrain`, which was not clippy-clean before this track.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=savetest` and the kit below. Start.
  2. Put on the four armour pieces and put the shield in the offhand. Move the sword to hotbar slot 4 and select it, and put the cobblestone in the top row of the backpack. **Screenshot 1:** the open inventory.
  3. Quit through the menu. Remove the `IW4L_MINECRAFT_KIT` line (until the next commit the kit is still added on every start), then start again.
  4. Expected: the same armour, the shield in the offhand, every item in the same slot, hotbar slot 4 selected and the sword still enchanted. **Screenshot 2:** the open inventory.
  5. Optional: copy `iw4l-artifacts\saves\savetest` into `.minecraft\saves` and open it in Minecraft 26.3: the inventory, armour and offhand are the same there.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:shield,minecraft:diamond_sword+sharpness=5,minecraft:cobblestone*64,minecraft:torch*16
```
