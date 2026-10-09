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

Commit: 382dcc5

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

### 5. The Minecraft kit only goes to a player without a saved inventory

Commit: ce55298

- **Changed:** `joining_inventory` (`minecraft_survival/src/player.rs`) gives the `IW4L_MINECRAFT_KIT` stacks only when the joining player's file has no `Inventory` key. A file with an `Inventory`, even an empty list, keeps exactly its stacks and the log says `Minecraft kit skipped: the player file already holds an inventory`. Round-6 worlds have player files with only the game mode in them, and those players still get the kit once. A temporary world (`temp`) has no files, so its player gets the kit on every start, as before.
- **Tests:** `player`: `the_kit_only_goes_to_a_player_without_a_saved_inventory` (a saved stack stays alone, an emptied inventory stays empty, a new player gets the kit). `server`: `a_mode_only_player_file_still_gets_the_kit_and_a_saved_inventory_does_not` (a mode-only file gets the kit and keeps its creative mode, a saved empty inventory gets nothing, a second UUID still gets the kit).
- **Decisions:** "No saved data" is judged by the `Inventory` key, not by the file, as the brief says; vanilla always writes `Inventory`, so any file saved by this build or by Minecraft counts as saved.
- **Test steps:**
  1. Use a new world: set `IW4L_MINECRAFT_WORLD=kittest` and the kit below in the `.env`. Start. The log says `Minecraft kit: 3 stacks from IW4L_MINECRAFT_KIT`.
  2. Throw the dirt away (Q) and place the oak log. **Screenshot 1:** the open inventory.
  3. Quit through the menu and start again with the same `.env`.
  4. Expected: no new kit (no dirt, one oak log fewer), and the log says `Minecraft kit skipped: the player file already holds an inventory`. **Screenshot 2:** the open inventory.
  5. Set `IW4L_MINECRAFT_WORLD=temp` and start: the kit is there again, as in a temporary world it always is.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:dirt*16,minecraft:oak_log*2,minecraft:stone_pickaxe
```

### 6. Minecraft health, hunger, experience and effects are saved

Commit: 1397259

- **Changed:**
  - `player_nbt.rs`: `PlayerData::stats` (`Stats`: the `SurvivalStatus`, the fire ticks and the score) is written with the keys and types of 26.3 (checked with javap: `LivingEntity.addAdditionalSaveData`/`readAdditionalSaveData`, `Player.addAdditionalSaveData`, `FoodData.addAdditionalSaveData`/`readAdditionalSaveData`, `Entity.saveWithoutId`): `Health`, `AbsorptionAmount`, `foodSaturationLevel`, `foodExhaustionLevel` and `XpP` as floats, `foodLevel`, `foodTickTimer`, `XpLevel`, `XpTotal` and `Score` as ints, `Air` and `Fire` as shorts. Missing keys read with vanilla's defaults (food 20, saturation 5, air 300). A file without `Health` has no stats, so a round-6 player keeps the survival carried over in the process.
  - `active_effects` holds our nine effects as `MobEffectInstance.CODEC` writes them (`id`, `amplifier` byte, `duration` int with -1 for infinite, `ambient`, `show_particles`, `show_icon`) and is left out when empty. Effects this game does not simulate (speed, haste, ...) stay in the file untouched.
  - `attributes`: slowness, weakness and absorption put vanilla's effect modifiers (`minecraft:effect.slowness` on `movement_speed`, `effect.weakness` on `attack_damage`, `effect.absorption` on `max_absorption`, amount times amplifier + 1, checked in `MobEffects`) into the attribute list, and an ended effect takes its modifier out. Vanilla does not rebuild effect modifiers when it loads a player, so without this a file opened in Minecraft would lose its absorption hearts at the first hit, or keep a slowness modifier forever. Other attributes and modifiers stay as they are.
  - On load the effects are added first and `AbsorptionAmount` is set after them, since `add_effect` raises the absorption.
  - `survival.rs`: `Survival::restore` puts saved stats back and keeps MW2's health from overwriting them (the first sync sets MW2 health from the saved hearts, without a damage flash); `fire_ticks()`; a `score` that grows with the experience points gained (vanilla's `increaseScore` in `giveExperiencePoints`), is reset by a death and kept with `keep_inventory`.
  - Vendored `third_party/minecraftoss/player/src/survival.rs` (MinecraftOSS patch): `FoodData::tick_timer`/`set_tick_timer`.
  - `player.rs`: `Player::dead` and `Player::arrived`, kept by `observe_life(pm_type)`. A dead player is saved with `Health` 0, as vanilla saves one.
  - `server::join`: saved stats come back; a player saved dead (health 0 or less) loads as a fresh respawn, keeping experience under `keep_inventory`; otherwise the survival carried over in the process, as before.
  - `render_anim/src/minecraft_world.rs`: `observe_life` runs before the frame's game code; the survival is not reset while the player is dead or spectating before they first arrive in the world after a load (that reset would wipe the restored stats during the MW2 spawn menu), and MW2 health is not synced into it then; `close_world` carries the survival over only for a temporary world, because a saved world has it in the player file.
- **Tests:** `player_nbt`: `health_hunger_and_experience_use_vanilla_keys_and_tag_types`, `stats_round_trip_and_absorption_is_restored_after_the_effects`, `effects_are_saved_like_mob_effect_instance_with_infinite_as_minus_one`, `effects_this_game_does_not_simulate_survive_and_ended_ones_are_removed`, `effect_attribute_modifiers_follow_the_effects`, `missing_food_and_air_keys_take_vanilla_defaults`. `player`: `a_player_arrives_when_first_alive_and_is_dead_until_respawned`, `a_player_saves_its_stats_and_a_dead_one_saves_no_health`. `server`: `a_joining_player_gets_back_the_health_hunger_and_experience_they_saved` (including the first MW2 sync: health set to 33 of 100, no damage), `a_player_saved_dead_comes_back_with_full_health_and_nothing_carried` (also with `keep_inventory`).
- **Decisions:**
  - Known limit, unchanged MW2 match flow: a match that ends goes through intermission, which resets health, hunger and experience before the world is saved. Quitting through the menu, or the next match after a quit, keeps them.
  - `carried_survival` stays for the temporary world only; in a saved world the player file is the truth, as in vanilla.
  - The fire ticks are saved as they run in `Survival` (-20 when not burning, vanilla's value for players).
- **Clippy:** no new findings in `minecraft_survival` or `render_anim` (`--no-deps`). The vendored `minecraftoss-player` tests do not compile in this repo (they read scenario CSV files that are not vendored), before and after this commit.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=survivaltest` and the kit below. Start.
  2. Throw experience bottles until you are level 5 or more. Build a 6-block pillar and jump off it (fall damage). Sprint around until the hunger bar drops by two or more. Eat a golden apple: yellow absorption hearts appear. **Screenshot 1:** the HUD (hearts, hunger, level).
  3. Within two minutes (before the absorption runs out), quit through the menu and start again.
  4. Expected: the same red and yellow hearts, the same hunger and the same level and bar, and no damage flash or hurt sound when the loading screen goes away. **Screenshot 2:** the HUD.
  5. Type `/effect give @s minecraft:poison 60`, quit and start again: still poisoned (green hearts), and it wears off within the minute.
  6. Optional: open `saves\survivaltest` in Minecraft 26.3: the same hearts (with the yellow ones), hunger and level.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:experience_bottle*64,minecraft:golden_apple*4,minecraft:cobblestone*64,minecraft:bread*16
```

### 7. Minecraft players come back where they left, facing the same way

Commit: f484535

- **Changed:**
  - `player_nbt.rs`: `Place` (dimension, feet, yaw, pitch, motion, on the ground, fall distance) and the player's `UUID`, written as 26.3's `Entity.saveWithoutId` and `ServerPlayer.addAdditionalSaveData` write them (checked with javap): `Pos` and `Motion` as lists of doubles, `Rotation` as two floats (yaw, pitch), `OnGround` as a byte, `fall_distance` as a double, `Dimension` as the level id string and `UUID` as four ints. Vanilla reads a missing `Pos` as 0,0,0, so a player who never spawned is saved at the world spawn. An unknown dimension reads as the overworld, as vanilla's "Unknown respawn dimension, defaulting to overworld".
  - `player.rs`: `Player::place` (where the player stands, as last seen alive) and `Player::arrival`; `Player::arrive(place)` sets the dimension and queues the move. This is the call the dimension switch can use to put a player into a level at a position.
  - `server.rs`: `join` takes a `Joining` (profile, world spawn, chosen mode, kit, carried survival). A saved overworld place becomes the arrival; a place in the Nether or the End starts at the world spawn until the dimension switch exists (the log says so, and the file keeps that place until the player has stood in the overworld); a player saved dead starts at the world spawn.
  - sim (`frame.rs`, `carrier.rs`): `teleport_facing` moves a player like `teleport` and turns the view the way a spawn does (`delta_angles` from the last command angles, as `script_player::spawn`); `set_minecraft_abilities` now also turns flying on when it is asked for together with may-fly (it still cannot fly without it).
  - `render_anim/src/minecraft_world.rs`: `place_player` records the place every frame the player is alive (MW2 velocity in blocks per tick for `Motion`, the fall since the highest point in the air for `fall_distance`), and on the first alive frame after a load moves the player there through `teleport_facing` (yaw turned back with `minecraft_yaw`, which is its own inverse), keeps a creative player flying, and restores the fall height. Before the player spawns, the world streams and the spawn gate waits around the saved place instead of the world spawn; if an arrival lands where the chunks are not loaded, the loading screen comes back until they are. The gate checks the chunks around the player's feet instead of around the map origin.
- **Tests:** `player_nbt`: `the_place_is_saved_as_entity_pos_rotation_and_dimension`, `a_place_round_trips_and_an_unknown_place_keeps_the_file_s_position`, `a_broken_or_unknown_position_reads_safely`. `server`: `a_joining_player_arrives_where_they_were_saved_facing_the_same_way` (overworld place, Nether place back at the spawn, new player at the spawn), and `a_player_saved_dead_comes_back_with_full_health_and_nothing_carried` now also checks that a dead player does not arrive at their death spot. `sim`: `a_teleport_facing_a_way_turns_the_view_as_a_spawn_does`, `a_player_who_may_fly_can_be_put_back_in_the_air_flying`. `render_anim`: `the_yaw_turns_from_mw2_to_minecraft_and_back_with_the_same_formula` (0, 90, 180 and -90 both ways).
- **Decisions:**
  - `Motion` and `OnGround` are saved for Minecraft but not pushed into MW2: the arrival stops the player, as a spawn does, and MW2 finds the ground itself.
  - The yaw is stored in Minecraft's convention (0 faces +Z), the pitch is MW2's pitch unchanged (both are positive looking down).
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=survivaltest` (or any saved world). Start.
  2. Type `/tp ~200 ~ ~200`, wait for the terrain, then build a 3-block pillar and stand on it. Turn to face a landmark (a tree or a hill) and look a little down. **Screenshot 1.**
  3. Quit through the menu and start again.
  4. Expected: the loading screen stays until the terrain there is in, then you are on the pillar facing the same landmark at the same angle. **Screenshot 2.**
  5. Type `/gamemode creative`, double-jump to fly, hover ten blocks up, quit and start again: you are still hovering there, flying. **Screenshot 3.** Type `/gamemode survival` afterwards.
  6. Optional: open the world in Minecraft 26.3: you stand at the same spot facing the same way.
- **Kit:** any; `minecraft:cobblestone*64` is enough.

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64
```

### 8. Each Minecraft player has their own ender chest

Commit: 4bffa75

- **Changed:**
  - `player.rs`: `Player::ender`, 27 slots, owned by the player as in vanilla (`Player.enderChestInventory`).
  - `player_nbt.rs`: `EnderItems`, written like `Inventory` (`PlayerEnderChestContainer.storeAsSlots`: a `Slot` byte per stack, checked with javap) and always present, even empty, as vanilla writes it. `join` gives a player back their own list.
  - `minecraft_terrain/src/containers.rs`: `Containers::with_ender(items, act)` lends a player's ender items to the level's containers for the length of `act` and takes them back. The level keeps no ender items of its own any more.
  - `render_anim/src/minecraft_world.rs`: the inventory screen's input (`apply_input`) and its picture (`publish`) run inside `with_ender` with the local player's list, so every container action works on that player's ender chest.
- **Tests:** `player_nbt`: `the_ender_chest_is_saved_as_ender_items_with_slot_bytes` (slots 0 and 26, read back, an emptied chest writes an empty list). `server`: `each_player_gets_back_their_own_ender_chest` (a second UUID starts with an empty one). `containers`: `each_player_opens_their_own_ender_inventory`. The older `every_ender_chest_opens_the_players_one_ender_inventory` still passes unchanged.
- **Decisions:** lending the list (a swap in and out) keeps every `Containers` function as it was instead of adding a player parameter to each of them; with several players, each one's screen runs with their own list.
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=survivaltest` and use the kit below in a new world (or `/give` the items). Start.
  2. Place the two ender chests twenty blocks apart. Put the diamonds into the first. Open the second: the diamonds are in it. **Screenshot 1.**
  3. Break the first ender chest with the pickaxe: nothing of its contents drops.
  4. Quit through the menu and start again. Open the second ender chest: the diamonds are still there. **Screenshot 2.**
  5. Optional: open the world in Minecraft 26.3 and open an ender chest: the same diamonds.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ender_chest*2,minecraft:diamond*8,minecraft:diamond_pickaxe
```

### 9. Minecraft respawn points are saved

Commit: f025a7a

- **Changed:**
  - `player_nbt.rs`: `GlobalPos` and `Respawn` (a `GlobalPos`, yaw, pitch and `forced`), saved as 26.3's `respawn` (`ServerPlayer$RespawnConfig.CODEC`: `dimension` string, `pos` int array, `yaw` and `pitch` floats, `forced` as a byte and only when true, since it is an `optionalFieldOf` with default false) and `LastDeathLocation` (`GlobalPos.CODEC`: `dimension`, `pos`), checked with javap. Both are removed from the file when they are cleared. A `respawn` without a dimension is ignored, an unknown dimension reads as the overworld.
  - `player.rs`, the respawn API for tracks B and C:
    - `Player::respawn` and `Player::last_death`.
    - `set_respawn(Option<Respawn>) -> bool` sets or clears the point and returns true when the position moved, which is when vanilla says "Respawn point set". `/spawnpoint` sets `forced: true` (vanilla's `SpawnPointCommand`), a bed `forced: false`.
    - `arrive_at_respawn(loaded, open, bed) -> bool` runs vanilla's respawn choice (`ServerPlayer.findRespawnPositionAndUseSpawnBlock`/`findRespawnAndUseSpawnBlock`, `PlayerList.respawn`): the `bed` resolver comes first; a forced point needs its block and the one above open (vanilla's `isPossibleToRespawnInThis`, taken as no collision and no fluid) and puts the player at the block's bottom centre with the point's yaw and pitch. A point in a dimension that is not loaded is kept and the world spawn is used (`createDefault`). A missing point is cleared (vanilla copies it to the respawned player only when it was found) and the call returns true for the chat message.
    - `respawn_place` and `missing_respawn_message` (`block.minecraft.spawn.not_valid`, what the client shows for `NO_RESPAWN_BLOCK_AVAILABLE`).
    - `observe_life` now returns whether the player has just respawned, and remembers `LastDeathLocation` at the block where the player last stood alive (`Player.die` stores `blockPosition()`).
  - `level.rs`: `open_for_respawn(terrain, registries, pos)`, which answers "unknown" for a chunk that is not loaded (such a point is trusted, and the spawn gate holds the player until it loads).
  - `server::join`: brings back the point and the last death; a player saved dead now loads dead, so their first spawn is a respawn and goes to the point.
  - `render_anim/src/minecraft_world.rs`: when MW2 respawns the player, they arrive at their respawn point through the arrival of commit 7, or get the chat message and the world spawn when the point is blocked. Bed points (`forced: false`) are left alone until track B passes the bed stand-up search (vanilla's `BedBlock.findStandUpPosition`) as the `bed` resolver and drops the `respawn.forced` condition next to `arrive_at_respawn`; until then such a point (for example one set by sleeping in Minecraft) is kept and the world spawn is used.
- **Tests:** `player_nbt`: `the_respawn_point_and_last_death_use_vanilla_s_layout`, `a_cleared_respawn_point_and_death_leave_the_file`. `player`: `a_death_is_remembered_where_the_player_last_stood_and_a_respawn_is_reported`, `setting_a_respawn_point_tells_whether_it_moved`, `a_forced_respawn_point_needs_two_open_blocks_and_a_bed_needs_a_stand_up_spot`, `a_respawn_goes_to_the_point_or_clears_a_missing_one`. `server`: `the_respawn_point_comes_back_and_a_player_saved_dead_respawns_there`; `a_player_saved_dead_comes_back_with_full_health_and_nothing_carried` now expects the player to load dead, with the death spot remembered.
- **Decisions:** nothing in this track sets a respawn point; `/spawnpoint` (track C) and beds (track B) call `set_respawn`. An MW2 team switch counts as a death for `LastDeathLocation`, as MW2 kills the player for it.
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=survivaltest`. Walk away from the spawn, note the spot, and type `/kill`. While the death screen shows, quit through the menu.
  2. Start again. Expected: you appear at the world spawn with full health and hunger (not at the death spot), and your items lie at the death spot. **Screenshot 1.**
  3. Optional: open the world in Minecraft 26.3 and hold a recovery compass (`/give @s minecraft:recovery_compass`): it points to where you died.
  4. Once track C's `/spawnpoint` is in the build: stand somewhere, type `/spawnpoint`, walk away and `/kill`: you respawn on that spot facing the way you faced. Quit and start again, `/kill` again: still there. Fill the spot with two blocks of stone and `/kill`: the chat says "You have no home bed or charged respawn anchor, or it was obstructed" and you respawn at the world spawn.
- **Kit:** any; none is needed for steps 1-3.

## Group: block entities

### 10. Changed Minecraft block entities are saved with their chunk

Commit: 50f39ce

- **Changed:**
  - Vendored `third_party/minecraftoss/world/src/chunk_map.rs` (MinecraftOSS patch): `ChunkMap::set_block_entities(&[(BlockPos, Option<Tag>)])` beside `set_blocks`. Like block edits, the changes wait per chunk until the chunk is sent again, saved, autosaved or dropped, so a shared chunk is copied once and not every frame. The latest change for a position wins (`None` clears it), a changed chunk is queued again by the next autosave (`queue_unsaved`), and a chunk dropped from memory (`Evicted`) carries its changes to the saver or to its reload.
  - `apply_edits` writes the block edits, then the block entity changes, then keeps every touched position's block entity in step with its block, as 26.3's `LevelChunk.setBlockState` does: a block entity whose block no longer takes its type (from the block entity catalog) is removed, and a block that takes one and has none gets its default (`EntityBlock.newBlockEntity`). So a broken chest's contents leave the file, and a placed chest, sign, lectern or banner gets the block entity vanilla expects.
  - `minecraft_terrain/src/terrain.rs`: `TerrainStream::record_block_entities` beside `record_edits`. Nothing calls it yet; the next commits do.
- **Tests:** `chunk_map`: `changed_block_entities_are_saved_with_their_chunk_and_follow_their_blocks` (an autosave snapshot writes a changed block entity, drops the one of a broken block, gives a placed block its default, leaves the shared chunk and untouched blocks alone; read back from a new `ChunkStorage`), `a_cleared_block_entity_whose_block_stays_comes_back_as_its_default` (through a dropped chunk's `resolve`, also replacing a pending tag of another type). The four older `queue_snapshots` tests pass an empty change map.
- **Decisions:** the type check and the default come from the block entity catalog the game already loads (`artifacts/block-entity-catalog/26.3.json`); without a catalog block entities are written as given and not checked.
- **Test steps:** nothing changes in our game with this commit alone.
  1. Optional: in a saved world, place a chest, a sign and a lectern, quit through the menu and open the world in Minecraft 26.3: all three are there and the chest opens (empty).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:chest,minecraft:oak_sign,minecraft:lectern
```

### 11. Minecraft chests, barrels and shulker boxes keep their contents

Commit: f6822d8

- **Changed:**
  - `minecraft_terrain/src/containers.rs`: `Containers` now loads its chests, trapped and copper chests, barrels and shulker boxes from their chunk's block entities and writes them back.
    - `load_chunk`/`import` read a container block entity when its chunk loads: `Items` with their `Slot` bytes and components, and `CustomName`. The tag is kept, so keys the game does not use (`lock`, `components`, ...) are written back unchanged.
    - `block_entity_tag(pos, kind)` writes the block entity as 26.3 saves it (`RandomizableContainer`, `BaseContainerBlockEntity.saveAdditional`, checked with javap and the block entity catalog): `id`, `x`, `y`, `z`, `components`, `Items` (always for chests and barrels, only when not empty for shulker boxes, `ContainerHelper.saveAllItems(..., false)`) and `CustomName` (a plain text component as a string).
    - Every change (a click, a filled loot chest, a placed or broken shulker box, a broken container) marks the position; `take_block_entities` hands the changed ones to the chunk map once a frame through `TerrainStream::record_block_entities`. A position whose block is no container of ours is never written, so a hopper or other block the server level owns is left alone; a container block the containers no longer hold goes back to its default.
    - `forget_chunk` drops a chunk's containers when it unloads; they are read again when it comes back.
    - Loot: a chest whose block entity has a `LootTable` is not read (vanilla's `tryLoadLootTable` wins over `Items`), so the first opening still rolls its loot from the chunk. Rolling fills the chest, which is then saved with `Items` and without `LootTable`, so a looted structure chest never fills again (the bug in the brief).
    - `leave_to_client`: the server level's copy of a chunk keeps no `Items` of these containers. Hoppers and comparators work on the server level's copies, which do not follow the player's chests (brief risk "Two container truths"). Before this commit those copies were always empty, because nothing was saved; with saved chests a hopper under one would otherwise have pulled items the chest still showed.
  - `minecraft_terrain/src/item_nbt.rs`: `stack_tag`, `stack_of`, `slot_list` and `slots` moved here from `minecraft_survival/src/player_nbt.rs` (which re-exports them), so player files and block entities write stacks the same way. Only items in the `minecraft` namespace are saved, which still leaves out the MW2 guns (`iw4:weapon/<n>`).
  - `minecraft_survival/src/level.rs`: `save_block_entities`; `stream_tick` takes the level's containers, saves the changed ones before the chunk map ticks, forgets those of unloaded chunks and reads those of loaded ones. `server.rs`: the autosave and the closing save write the changed containers before the chunks are queued.
  - `render_anim/src/minecraft_world.rs`: three lines pass the containers to `stream_tick` (and through `stream_around_spawn`).
- **Tests:** `containers`: `chests_barrels_and_shulker_boxes_save_what_vanilla_loads_and_saves_again` (chest, trapped chest, barrel and shulker box, full, empty and named three ways: `BlockEntities::load_and_save` gives the same tag back, with a synthetic catalog and with the real one under `MINECRAFTOSS_ROOT`), `a_saved_chest_comes_back_with_its_items_its_name_and_keys_it_does_not_know`, `an_opened_loot_chest_saves_its_items_and_no_loot_table`, `only_changed_containers_are_written_and_never_a_block_they_do_not_own`, `only_the_containers_own_block_entities_and_the_level_leaves_their_items_alone`. The `player_nbt` tests run unchanged on the moved stack functions.
- **Decisions:**
  - Containers are read when their chunk loads instead of when they are first opened, so a chest broken without being opened still drops what it holds.
  - A guns stack put into a chest is not saved, like in the player file.
  - Hoppers feeding chests stay for "Finish survival" (one container truth on the server), as the brief says.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=savetest` and the kit below. Start.
  2. Place a chest, a trapped chest, a barrel and the shulker box, and two chests side by side as a double chest. Put the sword in a middle slot of the chest, cobblestone in the last slot of each container, and fill both halves of the double chest a little. **Screenshot 1:** each open container.
  3. Quit through the menu and start again.
  4. Expected: every container holds the same stacks in the same slots, and the sword is still enchanted. **Screenshot 2:** each open container.
  5. Break the shulker box: it drops with its contents. Place it again and open it: the same contents.
  6. Walk 400 blocks away, come back and open the chests again: still the same.
  7. Open a village or other structure chest, take one item, quit and start again: the chest still lacks that item and is not filled again. **Screenshot 3.**
  8. Optional: open `saves\savetest` in Minecraft 26.3: the same contents in every container.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:chest*3,minecraft:trapped_chest,minecraft:barrel,minecraft:red_shulker_box,minecraft:diamond_sword+sharpness=5,minecraft:cobblestone*64,minecraft:dirt*64,minecraft:diamond_pickaxe+efficiency=5
```

### 12. Minecraft furnaces, brewing stands and campfires keep cooking after a restart

Commit: (filled in by the next entry). It finishes b681b6d, a work-in-progress commit made before a PC shutdown that only added the vendor accessors and the campfire time getters.

- **Changed:**
  - `minecraft_terrain/src/containers.rs`:
    - `Oven::from_tag` and its block entity entries: `cooking_time_spent`, `cooking_total_time`, `lit_time_remaining`, `lit_total_time`, `speed_multiplier`, `Items` (always, `ContainerHelper.saveAllItems`) and `RecipesUsed` (recipe id to count), as 26.3's `AbstractFurnaceBlockEntity.saveAdditional` writes them (checked with javap). The same for smokers and blast furnaces.
    - `Oven.experience` (a float) became `recipes_used`, a map from cooking recipe id to the number of finished smelts, like vanilla. Taking the output works the experience out from the recipes (`Oven::take_experience`), and a broken furnace spills it (`Oven::experience`), so experience earned before a restart is still paid.
    - `Brewer::from_tag` and its entries: `BrewTime`, `total_brew_time`, `Items`, `Fuel`, `total_fuel`, `speed_multiplier` (`BrewingStandBlockEntity`). As in vanilla's `loadAdditional`, a brewing stand loaded mid-brew remembers its ingredient from slot 3, so it goes on brewing instead of starting over.
    - `block_entity_kind` now also gives `minecraft:furnace`, `minecraft:smoker`, `minecraft:blast_furnace`, `minecraft:brewing_stand` and `minecraft:campfire` (for campfires and soul campfires); `block_entity_tag` and `import` handle them. A named furnace or brewing stand keeps its `CustomName`, and the screen shows it.
    - Changes are marked for saving when a station is clicked, when food is put on a campfire, when it is broken, and on every tick that changes its times, fuel or items. An idle furnace or a cold campfire is not written again.
    - `forget_chunk` also drops the furnaces, brewing stands and campfires of an unloaded chunk (after they were written), and `load_chunk` reads them back when it returns. So they only tick in loaded chunks, like vanilla block entities.
    - `remove` takes the recipe book to work out a broken furnace's experience; `minecraft_survival/src/level.rs` `spill` passes it.
  - `minecraft_terrain/src/campfire.rs`: `Campfire::from_tag` and `entries`: `Items` (always), `CookingTimes` and `CookingTotalTimes` as int arrays of four (`CampfireBlockEntity`). This replaces b681b6d's `with_times`.
  - Vendored `third_party/minecraftoss/player/src/crafting.rs` (from b681b6d): `RecipeBook::cooking_entry_for` (the recipe with its id) and `cooking_recipe(id)`.
- **Tests:** `containers`: `furnaces_brewing_stands_and_campfires_save_what_vanilla_loads_and_saves_again` (a furnace, brewing stand and soul campfire mid-cook, plus every station empty and named: `BlockEntities::load_and_save` gives the same tag back, with a synthetic catalog and with the real one under `MINECRAFTOSS_ROOT`), `a_saved_furnace_brewing_stand_and_campfire_go_on_where_they_stopped` (the exact saved times after 250 ticks; a copy loaded from the tags runs the next 400 ticks exactly like the original, finishes the brew and both beefs, and pays all three smelts' experience on taking), `only_working_stations_are_written_and_an_unloaded_chunk_stops_them`. `experience_rounds_by_chance` and `a_furnace_smelts_with_fuel_and_pays_experience_on_taking` now check `recipes_used`.
- **Decisions:**
  - Vanilla rolls the experience fraction once per recipe; we use the click's one roll for each recipe, which gives the same odds per recipe.
  - Our brewing stand only knows 400-tick brews and 20 fuel uses, so it writes `total_brew_time` 400, `total_fuel` 20 and `speed_multiplier` 1, and a loaded `BrewTime` or `Fuel` above that is capped.
  - An empty campfire is not held, so its block entity goes back to the empty default.
  - Behaviour change from the brief: furnaces, brewing stands and campfires stop while their chunk is unloaded (vanilla) instead of cooking anywhere in the world.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=savetest` and the kit below. Start.
  2. Place a furnace and put 20 raw iron and 2 coal in it. Place a smoker with the potatoes and a coal, and a blast furnace with 10 raw iron and a coal. Do not take anything out.
  3. Place the water bucket on the ground and fill the 3 bottles. Place the brewing stand and put in the 3 water bottles, the nether wart and the blaze powder.
  4. Place the campfire and put 4 beef on it, and the soul campfire with 2 beef.
  5. After about 10 seconds: **Screenshot 1:** the furnace screen (flame and arrow). **Screenshot 2:** the brewing stand screen (bubbles about halfway). **Screenshot 3:** the campfires with the beef on them.
  6. Quit through the menu at once and start again.
  7. Expected: the furnaces are still lit and go on from the same arrow and output count, the brewing stand finishes its brew (about 20 seconds after it started, not 20 seconds after the restart), and the beef on both campfires is cooked about 30 seconds after it was put on, not 30 seconds after the restart. **Screenshots 4 to 6:** the same views as 1 to 3.
  8. Take the iron ingots out of the furnace with a shift-click: you get experience for every ingot, including those smelted before the restart (0.7 each). **Screenshot 7:** the XP bar before and after.
  9. Put more raw iron in, `/tp` 400 blocks away, wait 20 seconds, and come back: the furnace has not moved on while you were away, and goes on once you are back.
  10. Optional: open `saves\savetest` in Minecraft 26.3: the furnace, brewing stand and campfires show the same progress.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:furnace,minecraft:smoker,minecraft:blast_furnace,minecraft:brewing_stand,minecraft:campfire,minecraft:soul_campfire,minecraft:coal*64,minecraft:raw_iron*64,minecraft:potato*16,minecraft:beef*8,minecraft:blaze_powder*4,minecraft:nether_wart*8,minecraft:glass_bottle*3,minecraft:water_bucket,minecraft:diamond_pickaxe+efficiency=5,minecraft:cobblestone*64
```
