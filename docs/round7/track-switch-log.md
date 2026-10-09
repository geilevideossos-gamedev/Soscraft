# Track SWITCH log: the dimension switch

Branch `dim-switch`, worktree `C:\Dev\mc-wt\switch`. Briefs: `e1-dimension-switch.md` (commits E1-1 to E1-15) and `e0-phase3-plan.md` (groups S1-S4). Base: fc6d4bd (track D merged).

A commit can't name its own hash, so each entry is written with its commit and its hash is filled in by the next commit; the last entry's hash is in `git log` under its subject.

## Group S1: server-wide state

### 1. Minecraft world spawn lives in the server, not the overworld level (E1-1)

Commit: 771827f

- **Changed:**
  - `minecraft_survival/src/server.rs`: `Globals.spawn` holds the world spawn as level.dat's `RespawnData` (dimension, position, yaw, pitch), as vanilla's `MinecraftServer` owns it. A new private `WorldFiles` (spawn, clock, rules, difficulty, weather and every player's file) is taken from `Globals` and written by both `Server::save_and_close` and `autosave`.
  - **Bug fixed:** `save_and_close` and `autosave` returned without writing level.dat, `weather.dat` or any player file when the overworld level was not loaded, and they took the spawn from the overworld's terrain stream. They now always write from `Globals`, whatever levels are loaded.
  - `minecraft_survival/src/save.rs`: `WorldSave::write` takes the whole `RespawnData` instead of an overworld position, so a spawn in another dimension keeps its dimension and angle. `OpenedWorld::spawn` is the saved `RespawnData` of any dimension (before, a spawn outside the overworld was dropped). New `world_spawn(dimension, pos)`, `spawn_dimension(spawn)` (an unknown dimension counts as the overworld, as vanilla's `MinecraftServer.findRespawnDimension`), `WorldSave::create_dimension(dimension)` and `WorldSave::read_saved_in`/`write_saved_in`.
  - Vendored `third_party/minecraftoss/world/src/level_storage.rs` (MinecraftOSS patch): per-dimension saved data. `dimension_data_path(dimension, id)`, `read_saved_in` and `write_saved_in` read and write `dimensions/<namespace>/<dimension>/data/<namespace>/<path>.dat` in the same `data` + `DataVersion` layout as the world's own `data/` files, with the same `_old` fallback. Checked with javap: `ServerChunkCache` builds each level's `SavedDataStorage` on `getDimensionPath(dimension).resolve("data")`, and `SavedDataStorage.getDataFile` resolves the id as `<namespace>/<path>.dat`. Track X2 saves `ender_dragon_fight` through it.
  - `render_anim/src/minecraft_world.rs` (`load()` only): the overworld stream gets the saved spawn as its known spawn only when the spawn is in the overworld; `Globals.spawn` is the saved spawn, or the spawn the overworld stream found for a new world.
- **Tests:** `level_storage`: `a_dimension_keeps_its_own_saved_data_under_its_data_folder` (exact path, `DataVersion`, other dimensions and the world's `data/` stay empty, a broken file is an error, then the `_old` copy is read). `save`: `a_world_spawn_in_another_dimension_comes_back_with_its_dimension` (a Nether spawn with a yaw round-trips through level.dat, the Nether folders are created, End saved data round-trips, an unknown dimension counts as the overworld). `server`: `closing_with_no_overworld_loaded_still_writes_level_dat_and_the_player_files` (a `Server` with no level at all writes level.dat with the Nether spawn, the clock, difficulty, `weather.dat` and the player file with its Nether place and inventory). The other `save` tests and `vanilla_world` now pass a `RespawnData`.
- **Decisions:**
  - The overworld folder is still created when the world opens, as before; `WorldSave::create_dimension` is there for the level loader of S2, which creates a dimension's folders when its level opens.
  - When level.dat's spawn is outside the overworld (only possible from vanilla's `/setworldspawn` today), the overworld level searches its own stand-in spawn for the MW2 map origin, and level.dat keeps the saved spawn untouched.
- **Test steps:** nothing changes in game when everything is in the overworld; this is groundwork for the Nether and End. Check that saving still works:
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=dimtest`. Start.
  2. Walk about 30 blocks from spawn, place the cobblestone as a small tower, and note where you stand. **Screenshot 1.**
  3. Quit through the menu and start again. Expected: you stand where you left, the tower is there, and the log says `Minecraft world saved in ... ms`. **Screenshot 2.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64
```

### 2. Minecraft levels share the world's clocks (E1-2)

Commit: 0c4e7fb

- **Changed:**
  - Vendored `third_party/minecraftoss/world/src/level/time.rs` (MinecraftOSS patch): `Level::set_clock(name, ticks)` sets one named clock of the level, whatever its default clock is.
  - `minecraft_terrain/src/server.rs`: `ClockTicks { overworld, the_end }` with `OVERWORLD_CLOCK` and `THE_END_CLOCK`. `TickInput.day_ticks` became `TickInput.clocks`, and every server tick sets both clocks in the level through `ServerSim::set_clocks`, instead of only the level's default clock. Before, a Nether level (no default clock) kept its own overworld clock counting from 0, so mobs' day schedule and the regional difficulty there ignored the time of day.
  - `minecraft_survival/src/server.rs`: `Globals.end_clock` is the server's `minecraft:the_end` clock beside `Globals.day` (the overworld clock), as 26.3's `ServerClockManager` keeps one set of clocks for the whole server (checked with javap: `tick()` advances every clock when `advance_time` is on, each clock keeps its own `paused`). New `Globals::clock_ticks`, `Globals::default_clock_ticks(dimension)` and `Globals::advance_clocks(seconds)` (the old frame code for the overworld clock and the game time shift, moved from the adapter, plus the End clock), and `default_clock(dimension)`: `minecraft:overworld`, none for the Nether, `minecraft:the_end` (the `default_clock` of each 26.3 dimension type).
  - `minecraft_survival/src/save.rs`: `WorldSave::write` takes `WorldClocks { overworld, the_end }`, and `data/minecraft/world_clocks.dat` now holds both clocks. `OpenedWorld::end_clock` reads the End clock back (0 and running for a world that has none, as `ServerClockManager.init` starts a missing clock).
  - `minecraft_survival/src/entities.rs`: `Entities::tick` takes the `ClockTicks` (two lines).
  - `render_anim/src/minecraft_world.rs`: `Globals` gets `opened.end_clock`, the entity tick gets `globals.clock_ticks()`, the frame calls `globals.advance_clocks(dt)`, and the client environment of a level runs on that level's default clock (the End clock in the End).
- **Tests:** `minecraft_terrain::server`: `every_level_follows_the_world_clocks_and_time_set_moves_only_a_default_clock` (overworld, Nether and End levels all take both clocks, both run on a tick, and `set_time` moves the overworld clock, the End clock or nothing in the Nether). `save`: `the_end_clock_is_saved_beside_the_overworld_clock` (`total_ticks` and `paused` in `world_clocks.dat`, read back, a temporary world starts at 0). `server`: `the_end_clock_runs_beside_the_day_and_each_level_reads_its_default_clock` (`default_clock` for each dimension, both clocks advance 20 ticks a second, a paused End clock stays, `advance_time` false stops both), and `closing_with_no_overworld_loaded_still_writes_level_dat_and_the_player_files` now also checks the paused End clock.
- **Decisions:**
  - **After track C:** `/time` in a level without a default clock answering `commands.time.no_default_clock`, and `/time` in the End naming and moving `minecraft:the_end`, need the command's dimension (track C's `Ctx.dimension`, not merged here). `default_clock(dimension)` is the lookup they will use; until then `/time` keeps acting on the overworld clock from every dimension.
  - The MW2 `game_time` shift still follows the overworld clock only, as before.
- **Test steps:** nothing changes in the overworld. Check that the clock still runs and saves:
  1. Keep `IW4L_MINECRAFT_WORLD=dimtest` and remove any `IW4L_MINECRAFT_TIME` line. Start.
  2. Type `/time set 12000`. Wait 30 seconds: the sun goes down. **Screenshot 1.**
  3. Quit through the menu and start again: the sun is where it was. Type `/time query daytime`: about 12600 or more.
  4. Optional: open `iw4l-artifacts\saves\dimtest\data\minecraft\world_clocks.dat` with an NBT viewer: it holds `minecraft:overworld` and `minecraft:the_end`.
- **Kit:** none needed; any kit works.

### 3. Minecraft weather keeps changing while nobody is in the overworld (E1-3)

Commit: 70ad059

- **Changed:**
  - Vendored `third_party/minecraftoss/world/src/level/time.rs` (MinecraftOSS patch): the rain and thunder timers of `advance_weather_cycle` moved unchanged into `Weather::advance_timers(random)`, which the level still calls, so the server can run them without a level.
  - `minecraft_terrain/src/server.rs`: `advance_weather_timers(&mut WeatherData, random)` runs those timers on the saved form. `ServerSim::weather_data` answers only in a level that can have weather (`canHaveWeather`: sky light, no ceiling, not the End), so a Nether or End level never writes its frozen copy back over the world's weather.
  - `minecraft_survival/src/server.rs`: `Globals.weather_random` and `Globals::advance_weather(ticking, ticks)`: when none of the ticking levels has weather (`has_weather`: the overworld), the server's `WeatherData` counts down and changes by itself, honouring `advance_weather`; a world with no weather yet starts it as vanilla does from an empty `WeatherData`. While the overworld ticks, its `ServerSim` keeps running the weather and reporting it back, as before.
  - `render_anim/src/minecraft_world.rs`: `Globals` gets its weather random (from the seed), and each frame passes the server ticks of the player's level to `advance_weather`.
- **Tests:** `minecraft_terrain::server`: `weather_timers_run_outside_a_level_like_the_overworld_runs_them` (rain stops when its timer ends, a new delay is drawn in vanilla's range, `/weather clear` time clears both, an empty `WeatherData` draws both timers), `only_a_level_that_can_have_weather_reports_the_world_s_weather` (overworld yes, Nether and End no, and they never rain). `server`: `the_weather_keeps_changing_while_no_level_that_has_weather_ticks` (frozen while the overworld ticks, counts in the Nether, stops with `advance_weather` false, a fresh world starts its timers).
- **Decisions:**
  - Vanilla always ticks the overworld, so its level random draws the next durations. Here the overworld may be closed, so the server keeps its own random (seeded from the world seed); the durations follow the same ranges.
  - Rain and thunder levels (the fade) stay per level, as in vanilla. A level that opens takes the server's `WeatherData` through `Entities::new`, which starts it already raining when it rains (track D's `prepareWeather`). Handing the weather back to an overworld level that stayed open while the player was away comes with the level reuse of S2.
- **Test steps:** nothing visible changes in the overworld until the Nether can be entered (S2/S3). Check that the weather still works:
  1. Keep `IW4L_MINECRAFT_WORLD=dimtest` and remove any `IW4L_MINECRAFT_WEATHER` line. Start.
  2. Type `/weather rain 60s` (a plain 60 would be 60 ticks, 3 seconds). It rains. **Screenshot 1.**
  3. Wait about a minute: the rain stops by itself.
  4. Quit and start again: still clear, and the log says `Minecraft weather: clear from weather.dat`.
- **Kit:** none needed; any kit works.

### 4. Minecraft Nether and End terrain generates in a test (E1-4)

Commit: b10d6d6

- **Changed:**
  - `minecraft_terrain/src/terrain.rs`: `TerrainStream::for_dimension_at` keeps a known spawn for the Nether and the End too, at the block's bottom centre, instead of always searching the stand-in Nether floor around chunk 0,0 or always using the End platform. The search and `ServerLevel.END_SPAWN_POINT` (100, 49, 0) stay the fallbacks. S2's level loader passes the saved or arrival position here.
  - The new tests are the first run of the Nether and End generators in the fork (NETHER-END.md said they had never run). Nothing panicked and nothing had to be patched: no MinecraftOSS patch in this commit.
- **Tests:** `terrain`: `nether_chunks_generate_light_and_tick_around_the_origin` generates the 5x5 chunks around 0,0 of seed 77 into a real world folder. Every column has bedrock at y 0 and 127 and air above the roof, the dimension is 256 blocks from y 0 without sky light, the stand-in spawn has two air blocks under the roof, there is netherrack and lava and no overworld or End block; then the chunks are lit, a Nether `ServerSim` ticks them 40 times with the world clocks, the region files land in `dimensions/minecraft/the_nether/region`, and a known spawn is kept with no chunk generated. `end_chunks_generate_the_main_island_around_the_origin` does the same for the End: the platform spawn with no search, end stone under 0,0 and more than 10,000 blocks of it, obsidian from the end spikes, no overworld or Nether block, lighting, 40 ticks of an End `ServerSim`, its region folder, and a known spawn kept.
  - What the run produced (seed 77, chunks -2..2): the Nether has netherrack, lava, magma, blackstone, gravel, soul sand and soil, quartz and gold ore, ancient debris, glowstone and a crimson forest (nylium, stems, wart blocks, shroomlights, roots, fungi, weeping vines); the End has end stone and the end spikes with their iron bar cages, bedrock tops and fire.
- **Decisions:** the smoke tests stop at what the headless crates can run (generation, light, the server level). Section meshes need the resource pack and run in game; they are not tested here.
- **Test steps:** none in game; the Nether and the End can be entered from S2 on (`IW4L_MINECRAFT_DIMENSION`).
- **Kit:** none.

### 5. Minecraft server-wide state passes clippy

Commit: d75e873

- **Changed:** three clippy findings on lines of entries 1 and 3: `spawn_dimension` names its overworld fallback instead of `unwrap_or_default` (disallowed by `clippy.toml`), `Globals::advance_weather` starts a world without weather from a named `NEW_WORLD_WEATHER` (all timers 0, as a new vanilla `WeatherData`), and the weather random's seed constant is grouped in equal digits.
- **Clippy:** `clippy -p minecraft_survival --tests --no-deps` has only the two older `entities.rs` findings (`type_complexity`, `manual_is_multiple_of`). `minecraft_terrain` and `render_anim` were not clippy-clean before this track (`approx_constant` errors in the mob renderers); with `-A clippy::approx_constant` none of their findings is on a line of this group. `minecraftoss-world` has no finding in `level_storage.rs` or `level/time.rs`.
- **Tests:** none new; `minecraft_survival`, `minecraft_terrain` (489 passed, 3 ignored as before), `minecraftoss-world` and `render_anim` pass, and `check -p launcher` is green.
- **Test steps:** none; nothing changes in game.
- **Kit:** none.

## Group S1 summary

- **Owner test for the whole group** (one start, kit below, `IW4L_MINECRAFT_WORLD=dimtest`, no `IW4L_MINECRAFT_TIME` or `IW4L_MINECRAFT_WEATHER` line): run the steps of entries 1, 2 and 3 in order in the same session. Nothing in the Nether or the End can be seen yet; that starts with S2's `IW4L_MINECRAFT_DIMENSION`.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64
```

- **After track C:** `/time` per dimension (`commands.time.no_default_clock` in the Nether, the `minecraft:the_end` clock in the End) through `server::default_clock`.
- **Handed to S2:** `WorldSave::create_dimension` when a level opens; `for_dimension_at` with the arrival as known spawn for the Nether and End; giving `Globals.weather` back to an overworld level that stayed open (a `set_weather_data` command on its `ServerSim`); `IW4L_MINECRAFT_WEATHER` applied only where `has_weather`.
- **MinecraftOSS patches in this group:** `world/src/level_storage.rs` (per-dimension saved data), `world/src/level/time.rs` (`Level::set_clock`, `Weather::advance_timers`).

## Group S2: levels open and close

### 6. Minecraft levels open on demand for any dimension (E1-5)

Commit: 9c0b000

- **Changed:**
  - `minecraft_survival/src/level.rs`: `Level::new(dimension, terrain)` and `Terrain::open(registries, seed, view_distance, dimension, save, known_spawn)`, which creates the dimension's folders (`WorldSave::create_dimension`), builds the terrain stream, the streamed scene and the dimension's environment. `load()` builds the overworld through it.
  - `minecraft_survival/src/server.rs`:
    - `Arrival { feet, yaw, pitch, motion }`, where a player stands when a level opens (E2's portal search and the End's kept velocity fill it later).
    - `LevelLoader::open(globals, dimension, view_distance, known_spawn, prepare)` builds the `Terrain` on a `minecraft-level-load` thread and then runs `prepare: FnOnce(&mut Terrain) -> Arrival + Send` there, before any player is placed. `poll` hands back the terrain and the arrival, or an error; a loader that stops (a panic in a generator or in `prepare`) is an error, never a crash. Dropping a loader joins its thread.
    - `Server.opening` holds the loaders by dimension. `Server::open_levels(view_distance)` opens the level of every player whose dimension has none (one loader per dimension, whoever asks). The known spawn is the world spawn for the overworld (when the world spawn is there) and the player's arrival block for the Nether and the End; without one the stream's own spawn is the arrival (the stand-in Nether floor or `END_SPAWN_POINT`).
    - `Server::finish_opening()` builds the level's mobs (`start_level`: `Entities`, game rules, station book) on the main thread, inserts the level, and places each waiting player who has no place in that dimension at the arrival. A level that fails to open sends its players back to where they stood in another dimension, or else to the world spawn, with a warning in the log.
  - `minecraft_survival/src/entities.rs` (`Entities::new` only): takes the dimension, starts its `ServerSim` with that dimension type instead of always `minecraft:overworld`, and applies `weather.dat` and `IW4L_MINECRAFT_WEATHER` only in a dimension that has weather (the overworld).
  - `render_anim/src/minecraft_world.rs`: `load()` uses `Terrain::open` and `Level::new`; the load completion uses `start_level`; each frame `follow_levels` opens and finishes levels, and `ui.loading_world` also holds while the local player's level is still opening.
- **Tests:** `server`: `a_level_opens_on_its_own_thread_for_the_players_in_its_dimension` (a player saved in the Nether and one sent to the End with no place there: one loader per dimension, never twice, both levels open with their mobs and folders, the Nether keeps the saved place as its known spawn and arrival, the End arrival is `END_SPAWN_POINT`), `a_level_that_fails_to_open_sends_its_players_back` (a `prepare` that panics: the error comes back, no level is inserted, one player returns to their overworld place and the other, with nowhere else, to the world spawn with its yaw), `an_arrival_keeps_its_angles_and_motion`.
- **Decisions:** the overworld still opens with the world, as every vanilla level exists while the server runs; it is also where a new world's spawn is searched. A level that fails to open retries when a player asks for it again.
- **Test steps:** nothing changes in game yet; the Nether and the End can be entered from entry 9 on. Check that the world still loads and the weather variable still works:
  1. Keep `IW4L_MINECRAFT_WORLD=dimtest` and add `IW4L_MINECRAFT_WEATHER=rain`. Start: it rains, the log says `Minecraft weather: rain from IW4L_MINECRAFT_WEATHER`. **Screenshot 1.**
  2. Remove the weather line again.
- **Kit:** none needed; any kit works.

### 7. Minecraft levels nobody is in save and close after 15 seconds (E1-6)

Commit: eaf9f07

- **Changed:**
  - `minecraft_survival/src/level.rs`: `Level.empty_ticks`.
  - `minecraft_survival/src/server.rs`:
    - `EMPTY_TIME` = 300 ticks, vanilla's `ServerLevel.EMPTY_TIME_NO_TICK` and the length of the `portal` ticket (checked with javap in the brief).
    - `Server::close_empty_levels(ticks)`: a level no player is in counts the server ticks; at 300 it leaves `levels` and `Globals::close_level` saves and drops it on a `minecraft-level-save` thread (server block entities with the closing save, containers, then the mobs, then the chunks), logging `Minecraft <dimension> level saved and closed in N ms`. A player in the level, alive or dead, resets the count. Returns the closed dimensions.
    - `Globals.closing` keeps those threads by dimension. `LevelLoader::open` takes the ones of its dimension and joins them on its own thread before it builds the level, so two chunk storages never hold the same region files. `Server::save_and_close` joins the rest; the adapter's `close_world` already runs it.
    - `Globals.level_ticks` gathers the server ticks of the frame for the next count.
    - A level that has weather and was empty takes the server's `WeatherData` back when a player returns (the S1 hand-off), so the weather that changed while nobody was in the overworld goes on there.
  - `minecraft_terrain/src/server.rs`: `Command::WeatherData` and `ServerHandle::set_weather_data`; `Entities::set_weather_data` also updates its cached copy so the next frame doesn't write the old weather back.
  - `render_anim/src/minecraft_world.rs`: the frame adds its server ticks to `level_ticks`; `follow_levels` closes empty levels before it opens any.
- **Tests:** `server`: `a_level_nobody_is_in_saves_and_closes_after_300_ticks` (299 ticks keep it, a player coming back resets the count, the 300th closes it, the occupied Nether stays, the reopening loader takes the closing save and the End's chunks are in its region folder when it opens again), `a_temporary_world_keeps_its_empty_levels_open`, `the_overworld_takes_the_world_s_weather_back_when_a_player_returns` (an empty overworld keeps its old weather, the player's return hands it the server's storm, and the count is reset).
- **Decisions:**
  - A temporary world (`IW4L_MINECRAFT_WORLD=temp`) keeps its empty levels open: its chunks live in a session folder that is deleted when the level closes, so closing would regenerate the level on the next visit.
  - Only the player's level ticks, as before; vanilla stops ticking entities in a level empty for 300 ticks, and here an empty level is not ticked at all.
- **Test steps:** the overworld closes when the player starts in another dimension (entry 9); then the log shows `Minecraft minecraft:overworld level saved and closed` about 15 seconds after the start. Nothing to test on its own.
- **Kit:** none.

### 8. MW2 collision switches to the player's new Minecraft level (E1-7)

Commit: 4ac9b0e4

- **Changed:**
  - `sim/src/voxel.rs`: `switch_level(origin)` keeps the one voxel world and its brush table, moves the block at map origin to `origin`, drops every chunk and every shape but the empty one, drains the queued block events and the mob boxes, and bumps the revision. Until the new level's chunks arrive, `awaiting_ground` holds the player still, as at a world load.
  - `render_anim/src/minecraft_world.rs`: the adapter keeps one map origin per loaded level (`Runtime.origins`) and the level it shows (`Runtime.shown`). When the local player's level is open and is not the one shown, `follow_levels` switches the voxel world to that level's origin and starts its presentation afresh: shape tables, light, light volume, clouds, environment, mining, minimap, particles and mob meshes, a new `view.generation` (the renderer drops the old sections), and the spawn gate. `ui.loading_world` holds until the player's level is shown. A closed level forgets its origin.
  - The overworld's origin stays its `player_spawn`, so the MW2 spawn at map origin is still the overworld spawn. The Nether and the End take the player's first arrival point in them as their origin.
- **Tests:** `voxel`: `a_level_switch_drops_the_old_level_and_waits_for_ground_at_the_new_origin` (no chunk, the new origin, only the empty shape, the player at map origin waits for ground, map points convert from the new origin, the brush table stays). The drained events are a process-wide queue other sim tests use, so the test checks the world itself.
- **Decisions:**
  - The time of the switch, for MW2 ordnance fired before it (E1-13), is recorded with E1-13 in S4, where it is used.
  - **Handed to S3:** coming back to a level that is still open (within its 15 seconds) needs its collision, sections and light sent again from its scene; the switch of E1-8 does that, with the loading screen and leaving the old level.
- **Test steps:** with entry 9 (start in the Nether).
- **Kit:** none.

### 9. IW4L_MINECRAFT_DIMENSION starts the player in the Nether or the End (E1-12)

Commit: c739d2b7

- **Changed:**
  - `minecraft_survival/src/server.rs`:
    - `join` puts a player saved in the Nether or the End back there, at the saved place (track D's fallback to the overworld spawn is gone); their level opens through `open_levels`. A player saved dead still respawns at the world spawn.
    - `debug_dimension(value)` reads `IW4L_MINECRAFT_DIMENSION`: `overworld`, `the_nether` or `the_end`, with or without `minecraft:` and in any case; empty means unset, anything else is an error naming the variable.
    - `start_in(player, dimension, overworld_spawn)`: a player already in that dimension keeps their saved place; the overworld puts them at its spawn (map origin); the Nether or the End leaves them waiting for that level with no arrival, so they arrive at the stream's spawn (the stand-in Nether floor, or `END_SPAWN_POINT` on the End's obsidian platform). Their old place stays as where to go back to if the level fails to open.
  - `render_anim/src/minecraft_world.rs` (load completion): applies the variable after the join and logs `Minecraft player starts in <dimension> from IW4L_MINECRAFT_DIMENSION`, or warns.
- **Tests:** `server`: `the_debug_dimension_names_a_dimension_with_or_without_its_namespace`, `a_debug_start_keeps_a_saved_place_in_that_dimension_and_waits_for_its_level_otherwise`; `a_joining_player_arrives_where_they_were_saved_facing_the_same_way` now expects a player saved in the Nether to come back in the Nether.
- **Checked:** the End generator places the 5x5 obsidian platform at y 48 around (100, 0) in the void east of the main island, whose edge is near x 90 (a probe of seed 77, not committed).
- **Decisions:**
  - The overworld still opens with the world, as in vanilla, so a new world's spawn is found; it closes 15 seconds after a start elsewhere.
  - **Until S3** (E1-10), dying in the Nether or the End respawns at that dimension's map origin (its first arrival point), because the MW2 spawn is map origin; the respawn switch to the overworld comes with S3.
  - The Nether and End skies, fog and clouds are still the overworld's (X1b).
- **Test steps** (the owner test for the whole group S2):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=s2test` (a new world) and add `IW4L_MINECRAFT_DIMENSION=the_nether`. Start. After the loading you stand in a Nether cave on netherrack under the bedrock roof (the sky still looks like the overworld's). The log says `Minecraft player starts in minecraft:the_nether`, `Minecraft minecraft:the_nether level opened in ... ms` and `Minecraft level shown: minecraft:the_nether`. **Screenshot 1.**
  2. About 15 seconds after the start the log says `Minecraft minecraft:overworld level saved and closed in ... ms`.
  3. Shoot netherrack: it breaks. Build a 3x3 obsidian wall with netherrack beside it and throw a frag at it: the netherrack craters, the obsidian holds. Light a TNT next to netherrack. **Screenshot 2.**
  4. Walk about 20 blocks, build a cobblestone tower, and stand beside it facing it. Quit through the menu. Remove the `IW4L_MINECRAFT_DIMENSION` line and start again: you are back in the Nether beside the tower, facing it. **Screenshot 3.**
  5. Quit. Add `IW4L_MINECRAFT_DIMENSION=the_end` and start: you stand on a 5x5 obsidian platform over the void, with the main island to the west. Don't step off. **Screenshot 4.**
  6. Quit. Set `IW4L_MINECRAFT_DIMENSION=overworld` and start: you stand at the overworld spawn. **Screenshot 5.**
  7. Quit. Set `IW4L_MINECRAFT_DIMENSION=moon` and start: the log warns `Minecraft dimension `moon` unknown`, and you start where you quit (the overworld spawn). Remove the line afterwards.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:obsidian*64,minecraft:netherrack*64,minecraft:cobblestone*64,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:torch*64,minecraft:cooked_beef*32
```

### 10. The Minecraft debug start passes clippy

Commit: 079cb532

- **Changed:** the one clippy finding on this group's lines: the adapter reads `IW4L_MINECRAFT_DIMENSION` with `as_deref().unwrap_or("")` instead of `unwrap_or_default` (disallowed by `clippy.toml`).
- **Clippy:** `clippy -p minecraft_survival -p sim -p minecraft_terrain -p render_anim --tests --no-deps -- -A clippy::approx_constant` has no other finding on a line of this group; the rest are older (`entities.rs` 76 and 613, `voxel.rs` 47, 325 and 381, and many in `minecraft_terrain`, `sim` and `render_anim`).
- **Tests:** none new; `minecraft_survival` (186 and the vanilla world test), `minecraft_terrain` (489 passed, 3 ignored as before), `sim` (66) and `render_anim` pass, and `check -p launcher` is green.
- **Test steps:** none; nothing changes in game.
- **Kit:** none.

## Group S2 summary

- **Owner test for the whole group:** the steps and kit of entry 9 (one new world `s2test`, several starts).
- **Handed to S3:**
  - The switch (E1-8) sets `player.dimension` and the arrival and lets `open_levels`, `finish_opening` and `follow_levels` do the rest; a player switching in counts as being in the level, so it never closes under them.
  - Coming back to a level that is still open needs its collision, sections and light sent again (`follow_levels` only starts a level's presentation afresh).
  - The respawn switch (E1-10): until then, death in the Nether or the End respawns at that dimension's map origin.
  - A chat line when a level fails to open during a switch (today only the log warns, and the player goes back).
- **Handed to S4:** the switch time for E1-13 (`voxel::switch_level` is where to record it).
- **After track C:** nothing new in this group.
- **MinecraftOSS patches in this group:** none.

## Group S3: the switch

### 11. Minecraft players travel between dimensions behind a loading screen (E1-8)

Commit: 7789ff18

- **Changed:**
  - New `minecraft_survival/src/switch.rs`: `LoadingReason { NetherPortal, EndPortal, Other }` and `loading_reason(dead, from, to)`, as vanilla's `ClientPacketListener.determineLevelLoadingReason` (checked with javap: dead gives `OTHER`, then the Nether on either side gives `NETHER_PORTAL`, then the End on either side `END_PORTAL`, else `OTHER`). `Switch { from, to, arrival, reason, respawn, left }`, and `failed_message`, the red chat line of a level that failed to load.
  - `minecraft_survival/src/player.rs`: `Player.switching` and `Player::switch_to(to, arrival, reason)`: the player keeps everything (inventory, health, UUID, respawn point) and only their level changes. Their old place stays as where to go back to; an arrival outside the target dimension is not taken (`None` lets the level loader choose, as a portal search will). `Player.respawning` is added for entry 13.
  - `minecraft_survival/src/server.rs`: a level that fails to open turns a switching player's switch back to where they came from.
  - `minecraft_survival/src/entities.rs` (one method and its helper): `Entities::leave` puts the stacks the old level already let the player pick up into the inventory, cancels a waiting item use and drops pending pearl landings, so nothing of the old level lands in the new one.
  - `minecraft_terrain/src/terrain.rs`: `TerrainStream::reshow(scene)`: a level shown again (a return within its 15 seconds) hands every chunk it holds back for collision, sends its light again and rebuilds every section, as the renderer and the voxel world dropped them.
  - `render_anim/src/minecraft_world.rs` (load/level handling only):
    - **Leave**, the frame after `switch_to`: the container screen closes (loose crafting items go back or drop in the old level), `Entities::leave` runs on the old level, eating and blocking stop, the step and fall state and the recent placements clear, the voxel world drops its chunks (so `awaiting_ground` holds the player still), and `MC_INVULNERABLE` is set (vanilla's `isChangingDimension`).
    - **Arrive:** `follow_levels` shows the new level when it is open (switch_level to its origin, fresh shape tables, light, clouds, environment, mining and minimap, a new `view.generation`, `reshow`, and the spawn gate).
    - **Hold:** while the player is switching, the player is teleported to the arrival facing its angles, the level streams around the arrival and nothing else of the level runs (no mob ticks, so no mob hits); the world clocks keep going. When the gate has the arrival's chunks with collision and a visible section, the switch ends and the game goes on; the loading flag stays up for the gate's 500 ms close delay.
    - A level that fails to load sends the player back with a red chat line (`minecraft:the_end could not be loaded: ...`).
- **Tests:** `switch`: `the_loading_reason_follows_determine_level_loading_reason` (all nine pairs alive, and death in each), `a_switch_keeps_only_an_arrival_in_its_target_dimension`, `a_level_that_failed_to_load_says_so_in_red`. `player`: `a_switching_player_keeps_where_it_stood_until_it_arrives`. `server`: `a_switching_player_opens_its_new_level_and_arrives_where_it_was_sent` (one loader, the arrival as the Nether's known spawn, the player placed at it, the level stays open, then an End that panics while opening sends the player home with the switch pointing back). `entities`: `stacks_the_old_level_let_the_player_pick_up_reach_the_inventory_on_leaving`. `terrain`: `a_level_shown_again_sends_its_chunks_and_light_again`.
- **Decisions:**
  - The player is held (frozen and invulnerable, the level not ticking) until the gate lets them in, instead of vanilla's 60-tick `hasClientLoaded` timeout with the level ticking: here the client and the server are one frame loop, so a level that ticks while its chunks are missing would let mobs hit a player who cannot move.
  - A remote killstreak (AC-130, predator, chopper gunner) does not hold a switch yet; that rule comes with the ordnance work of S4 (E1-13).
- **Test steps:** nothing starts a switch on its own yet; entry 13 (respawning after death in the Nether) is the first trigger, and its steps test this entry too.
- **Kit:** none.

### 12. Minecraft level loading screen looks like vanilla (E1-9)

Commit: 7dcc3aae

- **Changed:**
  - `frame/src/minecraft_ui.rs` (appended): `MinecraftUi.level_loading: Option<McLevelLoading { reason, background, background_st, menu_background }>` with `McLoadingReason { NetherPortal, EndPortal, Other }`.
  - New `hud/src/minecraft_loading.rs`, drawn last so it covers the whole HUD: the background over the whole screen, the in-world menu background tiled in 32 GUI pixel squares for `Other`, and `multiplayer.downloadingTerrain` ("Loading terrain..." from the pack's language file) in white with its shadow, centred 50 GUI pixels above the middle. These are the numbers of vanilla's `LevelLoadingScreen.extractRenderState` with no chunk status view, checked with javap. `hud/src/{lib,plugin,gpu_list}.rs` register it, three lines and a field.
  - New `render_anim/src/minecraft_loading.rs`, which builds the backgrounds of `LevelLoadingScreen.extractBackground` (checked with javap) from the packs at run time:
    - **`NETHER_PORTAL`:** the `block/nether_portal` sprite stretched over the screen, opaque as `GUI_OPAQUE_TEXTURED_BACKGROUND` draws it, one animation frame a tick.
    - **`END_PORTAL`:** the end portal effect. `environment/end_sky` times the first colour, plus 15 layers of `entity/end_portal/end_portal` (`RenderPipelines.END_PORTAL` sets `PORTAL_LAYERS` 15), each scaled, turned and shifted as the pack's `rendertype_end_portal.fsh` does. The colours are read from that shader in the pack at run time, so no Mojang numbers are in our code. It is painted on the CPU into a 960-pixel-wide picture once each time the screen opens.
    - **`OTHER`:** the title panorama cube (`panorama_0..5` on the cube faces in `CubeMapTexture`'s order). It is drawn upside down and tilted 10 degrees at an 85 degree field of view, as `CubeMap.render`, and spins 0.1 degrees a tick as `Panorama`. It is painted into a 480-pixel-wide picture, so the upscale gives the blur, and `gui/inworld_menu_background` goes on top.
    - Pictures that change are double-buffered, as the minimap is, so the screen never shows a missing texture.
  - `render_anim/src/minecraft_gui.rs`: the GUI's language strings also take `multiplayer.downloadingTerrain`.
  - `render_anim/src/minecraft_world.rs`: the backgrounds load with the world, and `ui.level_loading` shows from the moment a switch leaves its level until 500 ms after the gate lets the player in.
- **Tests:** `render_anim::minecraft_loading`: `the_end_portal_colours_come_from_the_shader_source` (a made-up shader text), `an_end_portal_layer_scales_turns_and_shifts_the_screen_like_the_shader`, `a_painted_end_portal_is_the_sky_and_the_layers_added_up`, `the_nether_portal_background_steps_one_frame_a_tick`, `a_cube_map_direction_picks_its_face_like_gl`, `the_panorama_looks_from_the_upside_down_cube_tilted_ten_degrees`, `the_canvas_keeps_the_window_shape_and_the_spin_wraps`. `hud::minecraft_loading`: `the_menu_background_tiles_the_screen_in_thirty_two_pixel_squares`, `loading_terrain_sits_centred_fifty_pixels_above_the_middle`.
- **Decisions:**
  - The downloaded 26.3 pack holds the jar's 1x1 placeholder panorama (the real pictures are asset-index objects the setup does not fetch), so `OTHER` shows a flat colour under the menu background today. The cube is drawn properly, so a pack with a real panorama shows it.
  - The end portal drifts with `GameTime` (the overworld clock's place in its 24000-tick day) by less than a texel a second, so the picture is painted once a screen instead of every frame. This keeps a debug build smooth.
  - No progress bar: vanilla shows it only when the server reports a load stage, and a remote client has none.
- **Test steps:** with entry 13 (the respawn switch shows the `OTHER` screen). The Nether and End portal screens can't be reached in game until `/execute in` (S4, after track C) or portals (N2).
- **Kit:** none.

### 13. Minecraft players respawn in their respawn dimension (E1-10)

Commit: 4e05214a

- **Changed:**
  - New `minecraft_survival/src/respawn.rs` (the pure respawn rules of the plan's ownership table):
    - `respawn_dimension(world spawn, point)`: the point's dimension, or the world spawn's (overworld when unknown), as `RespawnConfig.getDimensionOrDefault` and `MinecraftServer.findRespawnDimension`.
    - `respawn_target(world spawn, point, open, spawn_block)`: the point if its block (bed or anchor, through `spawn_block`) or a forced point's two open blocks are there, else the world spawn with `missing` set, as `findRespawnPositionAndUseSpawnBlock` gives `TeleportTransition.missingRespawnBlock`.
    - `level_spawn` is a level's own spawn, facing the world spawn's angles when the world spawn is in that level; for the overworld that is MW2 map origin.
    - `respawn_in(globals, level, player, spawn_block)` places the respawn in the loaded level, or switches on when the point is missing and the world spawn is in another dimension. It clears the point and answers whether "block.minecraft.spawn.not_valid" is due.
    - `respawn(globals, level, player)` respawns in the level the player is in, or starts a switch with reason `OTHER` (dead) and `Switch.respawn` set.
  - `minecraft_survival/src/server.rs`: `Server::prepare_respawns` gives a dead player their `respawning` dimension. `open_levels` opens it at once (with a point there as the known spawn), and `close_empty_levels` keeps it open while the player is dead, so the level is ready by the time MW2 shows them alive again.
  - `render_anim/src/minecraft_world.rs`: when MW2 shows the player alive again, `respawn` decides. In the same dimension they are teleported to the point or the spawn, facing its angles. In another dimension the switch of entry 11 runs, and once the target level is shown the point is checked there (`respawn_in`), before the player is placed. A switching player with no arrival and no place in the level goes to the level's spawn.
- **Tests:** `respawn`: `the_respawn_dimension_is_the_point_s_or_else_the_world_spawn_s`, `a_respawn_goes_to_the_point_or_else_the_world_spawn_and_says_when_the_point_is_gone` (no point, a Nether anchor, a missing one, a bed's stand-up spot). `server`: `a_death_in_the_nether_loads_the_respawn_level_at_once_and_the_respawn_switches_there` (death in the Nether opens the overworld while the player is still dead and keeps it open; the respawn switches with `OTHER` and arrives at the overworld's spawn facing the spawn yaw; a Nether point from the overworld switches to the Nether and arrives at the point; a point inside the bedrock floor is missing, is cleared and sends the player to the overworld spawn).
- **Decisions:**
  - Until N5 plugs beds and anchors into `spawn_block`, a bed or anchor point from a vanilla save (not forced) is kept but not used: the player respawns at the world spawn, as before this entry in the overworld.
  - A point in a level that has not loaded its chunks there yet counts as open (the check needs the blocks; vanilla loads the chunk synchronously). That only matters for forced points in chunks never visited; N5's anchor check will load the chunk first.
  - The respawn always teleports to the spawn facing its angles, as vanilla's respawn does, even in the overworld where MW2 already put the player at map origin.
- **Test steps** (the owner test for the whole group S3, entries 11 to 13; `/execute in` comes with S4, so the Nether and End portal screens can't be reached yet):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=s3test` (a new world) and `IW4L_MINECRAFT_DIMENSION=the_nether`. Start. Within 5 seconds of standing in the Nether cave, type `/kill`. While you are dead, the log says `Minecraft minecraft:overworld level opened` if it had closed, or nothing if it is still open.
  2. When MW2 respawns you: the loading screen shows a flat dark background with the darker tiled menu pattern and "Loading terrain..." in white above the middle. Then you stand at the overworld spawn on solid ground, with all terrain drawn. You did not fall through. **Screenshot 1 (loading screen), screenshot 2 (overworld).** This is the return to a level that was still open (entry 11's `reshow`). The log says `Minecraft player leaves minecraft:the_nether for minecraft:overworld`, `Minecraft level shown: minecraft:overworld` and `Minecraft player arrived in minecraft:overworld`.
  3. About 15 seconds later the log says `Minecraft minecraft:the_nether level saved and closed`.
  4. Quit. Keep `IW4L_MINECRAFT_DIMENSION=the_nether` and start again. Wait 20 seconds (the log says the overworld saved and closed), then build a small netherrack tower and `/kill`. While dead the log says `Minecraft minecraft:overworld level opened in ... ms`. On respawn: the loading screen again, then the overworld spawn. **Screenshot 3.**
  5. Shoot and blow up some netherrack before step 4's `/kill` if you like: craters and block breaks still work in the Nether, as in S2.
  6. Quit. Set `IW4L_MINECRAFT_DIMENSION=the_end` and start on the obsidian platform. Type `/kill`. On respawn: the loading screen, then the overworld spawn. **Screenshot 4.**
  7. Quit. Remove the `IW4L_MINECRAFT_DIMENSION` line and start: you are at the overworld spawn where you respawned.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:obsidian*64,minecraft:netherrack*64,minecraft:cobblestone*64,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:torch*64,minecraft:cooked_beef*32
```

### 14. The Minecraft dimension switch passes clippy

Commit: 33350004

- **Changed:** the two clippy findings on lines of this group. `Entities::leave`'s helper takes `Vec<Picked>` with a `type Picked` (`type_complexity`), and the loading backgrounds read the end portal colours with `map_or_else` instead of `unwrap_or_default` (disallowed by `clippy.toml`).
- **Clippy:** `clippy -p minecraft_survival -p minecraft_terrain -p render_anim -p hud -p frame --tests --no-deps -- -A clippy::approx_constant` has no other finding on a line of this group. The rest are older: `entities.rs` 76 and 620, `terrain.rs` 657, `minecraft_world.rs` 126, 384 and 393, `hud/src/plugin.rs` and `gpu_list.rs`, and many more in those crates.
- **Tests:** none new. `minecraft_survival` (195 and the vanilla world test), `minecraft_terrain` (490 passed, 3 ignored as before), `render_anim` (43), `hud` (111) and `frame` (14) pass, and `check -p launcher` is green.
- **Test steps:** none; nothing changes in game.
- **Kit:** none.

## Group S3 summary

- **Owner test for the whole group:** the steps and kit of entry 13 (one new world `s3test`, several starts). It covers the switch (entry 11), the `OTHER` loading screen (entry 12) and the respawn into another dimension (entry 13), including a return to a level that was still open.
- **Not reachable in game yet:** the `NETHER_PORTAL` and `END_PORTAL` loading screens. Only `/execute in` (E1-11) and the portals (N2, X2) start a switch while alive.
- **Handed to S4:**
  - E1-11 (`/execute in` moves players between dimensions) still waits for track C's `Ctx.dimension` (C commit 12). Without it the command can't name a dimension. The switch API it will call is `Player::switch_to(to, Some(arrival), loading_reason(false, from, to))`, with the teleport scale of `DimensionType.getTeleportationScale`.
  - E1-13: a switch should wait while the player is in a remote killstreak. Ordnance fired before a switch must leave the new level alone; the switch moment to record is the leave step in `minecraft_world.rs`, where `voxel::switch_level` runs.
- **After track C:** E1-11 as above, plus `/time` per dimension from S1.
- **MinecraftOSS patches in this group:** none.

## Group S4: MW2 in other dimensions

### 15. MW2 ordnance from before a dimension switch leaves the new level alone (E1-13)

Commit: 0511fa8b

- **Changed:**
  - `sim/src/voxel.rs`: `LevelSwitch { Never, Pending, At { ms, models } }`. `switch_level` marks a switch as pending, and the first authority tick after it stamps the level time and the next script model id (`stamp_level_switch`). `activate` and `deactivate` forget it. `fired_before_switch(born_ms)` is true for a projectile born before that tick, and `spawned_before_switch(inflictor)` for a script model the scripts spawned before it (spawned script model ids count up from `SPAWNED_PRESENCE_BASE`; the map's own models never count).
  - `sim/src/step.rs` (`advance_time_system`, four lines): stamps the switch at the start of every authority tick.
  - `sim/src/damage.rs` (two conditions): a projectile blast (`ExplosionBlast.killcam_entity_start_time`, the projectile's spawn time) or a script `RadiusDamage` whose inflictor was spawned before the switch makes no crater and no mob blast. MW2 player damage, glass and MW2 entities are untouched: they live on the MW2 map, which every dimension shares.
  - The adapter already calls `switch_level` when the player leaves a level and again when the new level is shown, so ordnance fired during the loading screen is left behind too.
- **Tests:** `voxel`: `ordnance_from_before_a_level_switch_is_left_behind` (no switch leaves everything alone; a pending switch leaves everything behind; the stamp keeps the first tick; a grenade born a tick before the switch is left behind, one born in the switch's tick is not; a bomb spawned before it is, a later one and the map's own models are not).
- **Decisions:**
  - The stamp is taken by the sim, not the adapter, so it is the sim's own level time, and a switch between two sim ticks counts from the next one.
  - A script blast with no inflictor (a level-wide `RadiusDamage`) still craters, as there is nothing to date it by. Killstreak bullets (helicopter turrets) still chip blocks after a switch: bullets carry no spawn time. Both are left as known gaps (brief section 8).
- **Test steps:** a living switch can't be started in game yet (`/execute in` waits for track C, portals for N2), so this is checked by the unit test. Check that MW2 craters still work in every dimension:
  1. Keep `IW4L_MINECRAFT_WORLD=s3test` and set `IW4L_MINECRAFT_DIMENSION=the_nether`. Start. Throw a frag at netherrack: it craters. **Screenshot 1.**
  2. `/kill`. After the respawn in the overworld, throw a frag at the ground: it craters. **Screenshot 2.**
  3. After track C (`/execute in`): in the overworld, throw a frag at the ground, then at once open chat and type `/execute in minecraft:the_nether run tp ~ 128 ~` before it explodes (about 3 seconds; type the command once beforehand so Up and Enter bring it back). In the Nether there is no crater near your arrival from that frag. **Screenshot 3.** The overworld spot has no crater either: the frag's blast happened after the overworld stopped being shown, and MW2 ordnance only reaches the level the player sees.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:netherrack*64,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

### 16. Minecraft travel waits for a remote killstreak to end (E1-13)

Commit: e001c779

- **Changed:**
  - `minecraft_survival/src/player.rs`: `Player.remote` (the player is flying a remote killstreak) and `Player.held`. `Player::travel_to(to, arrival, reason)` is the way a living player travels (`/execute in` after track C, the portals of N2 and X2): while `remote` is set the switch is held, and `Player::set_remote(false)` starts it then, from the dimension the player is in at that moment. `switch_to` stays immediate, as the respawn and a level that fails to load use it.
  - `render_anim/src/minecraft_world.rs`: each frame the adapter sets `remote` from the presented snapshot: the player is linked to something (`PM_TYPE_NORMAL_LINKED`, the chopper gunner and the AC-130 link the player to the aircraft), has the AC-130 flag (`other_flags::AC130`), or steers a predator missile (`remote_missile`). A dead player's held switch is dropped.
- **Tests:** `player`: `travel_waits_while_the_player_is_in_a_remote_killstreak` (held while remote, starts when it ends with the right `from`, no wait without a killstreak, a respawn `switch_to` never waits). `render_anim`: `a_linked_player_or_one_in_the_ac130_is_in_a_remote_killstreak`.
- **Decisions:**
  - Vanilla's rule is `Entity.canUsePortal(allowPassengers)`, which is `(allowPassengers || !isPassenger()) && isAlive()` (checked with javap). A player in a remote killstreak is the analogue of a passenger. The brief asks for a wait rather than a refusal, so a `/execute in` typed while steering a predator happens when the missile is gone instead of being lost.
  - Being linked counts as remote whatever the link is: in MW2 multiplayer only killstreaks link a player.
- **Test steps:** nothing in game starts a living switch yet (`/execute in` waits for track C). After track C:
  1. In the overworld, `give killstreak/predator` in the console and select the predator on the hotbar to fly it.
  2. While steering the missile, open chat and send `/execute in minecraft:the_nether run tp ~ 128 ~`. Nothing happens until the missile hits; then the Nether loading screen shows. **Screenshot 1.**
- **Kit:** none needed.

### 17. MW2 air support is unavailable under the Nether roof (E1-14)

Commit: b7ca1a57

- **Changed:**
  - `playerstate_iw4/src/playerstate.rs`: `pm_flags::MC_NO_SKY` = `0x0800_0000`, the bit below the other `MC_*` bits, unused by IW4 and the fork. Glide takes the next free bit after this group.
  - `sim/src/frame.rs` (`set_minecraft_abilities`): `MC_NO_SKY` is one of the bits the abilities set and clear.
  - `sim/src/voxel.rs`: `needs_sky(weapon)` names the air support weapons: the predator, precision airstrike, stealth bomber, harrier, attack helicopter, pave low, chopper gunner and AC-130 (`killstreak_<name>_mp`), and the care package, emergency airdrop and sentry gun markers (`airdrop_marker_mp`, `airdrop_mega_marker_mp`, `airdrop_sentry_marker_mp`, the weapon column of `mp/killstreakTable.csv`). `refused_without_sky(pm_flags, weapon)` is the rule both selects use. The UAV, counter-UAV, EMP and nuke stay usable.
  - Vendored `third_party/minecraftoss/core/src/environment.rs` (MinecraftOSS patch): `DimensionPresentation.has_ceiling`, read from the dimension type's `has_ceiling` in the data pack (true only for `minecraft:the_nether`).
  - `render_anim/src/minecraft_world.rs`: the abilities carry `MC_NO_SKY` in a level whose dimension type has a ceiling, also while switching into it. When the hotbar asks for an air support weapon there, the weapon is not raised, and "Air support is unavailable under this dimension's ceiling" shows in the chat once per selection (`switch::air_support_unavailable`). The streak stays in the hotbar and works again under the sky.
  - `net/src/client/runtime.rs`: the client's weapon select skips an air support weapon while `MC_NO_SKY` is set, both for the hotbar's weapon request and for MW2's action slots (which are also no longer sent to the server for it).
- **Tests:** `voxel`: `only_air_support_needs_the_sky` (every air support weapon, in any case, refused only with `MC_NO_SKY`; UAV, counter-UAV, EMP, nuke, guns and the AC-130's own cannons are not). `frame`: `the_no_sky_flag_is_set_and_cleared_with_the_abilities` (and it shares no bit with the other `MC_*` flags, sprint blocking or last stand). `minecraft_terrain::environment`: `only_the_nether_has_a_ceiling`. `switch`: `refused_air_support_says_why_in_the_chat`.
- **Decisions:**
  - Taken by the owner: air support is refused where the dimension type has a ceiling (the Nether), the streak is kept, and a line says why; the End and the overworld allow it. Bombs that miss the End islands fall into the void.
  - The line is our own English text: vanilla has no such rule, and an MW2 string would need the zone's localisation in the Minecraft chat. It is a normal chat line, as overlay lines are not drawn.
  - The authority check (the server refusing the weapon) comes with multiplayer, as the plan says; here the host's own client is the only one.
  - **Check in game:** the weapon names are MW2's from memory of the stock scripts; they could not be read from the zone here. If one air support streak still works in the Nether, its weapon name is different and goes into `NEEDS_SKY`.
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=s3test` and set `IW4L_MINECRAFT_DIMENSION=the_nether`. Start.
  2. Open the console, type `give killstreak/predator` (accept the suggestion) and close it. The predator lands on the hotbar. Select it: it is not raised, and the chat says "Air support is unavailable under this dimension's ceiling". **Screenshot 1.** It stays on the hotbar.
  3. `give killstreak/airdrop` and select it: the same line, no smoke grenade in hand.
  4. `give killstreak/uav` and select it: the UAV works in the Nether.
  5. `/kill`. After the respawn in the overworld, select the predator: you fly it as usual. **Screenshot 2.**
  6. Quit. Set `IW4L_MINECRAFT_DIMENSION=the_end` and start on the obsidian platform. `give killstreak/predator` and use it: it works over the End. **Screenshot 3.**
- **Kit:** none needed; any kit with a free hotbar slot.

### 18. Minecraft players can be teleported into another dimension (E1-11, the part without track C)

Commit: d53120c6

- **Changed:**
  - `minecraft_survival/src/switch.rs`:
    - `dimension_named(id)`: the dimension a `DimensionArgument` id names (`minecraft:the_nether`, or `the_nether` with the default namespace, as `Identifier` parses it). Lenient names like `nether` are not ids and give `None`, as vanilla's `argument.dimension.invalid`.
    - `coordinate_scale(registries, dimension)` reads the dimension type's `coordinate_scale` from the data pack at run time, and `teleport_scale(registries, from, to)` divides them, as `DimensionType.getTeleportationScale` (checked with javap: `from.coordinateScale() / to.coordinateScale()`). `scaled(feet, scale)` scales x and z and keeps y, as `CommandSourceStack.withLevel` moves the command's position (checked with javap).
  - `minecraft_survival/src/player.rs`: `Player::teleport_into(place)`: a place in the player's own dimension is a plain teleport (`false`); a place in another dimension starts `travel_to` with that place as the arrival and the reason of `loading_reason` (so it waits for a remote killstreak like any travel) and answers `true`.
- **Tests:** `switch`: `a_dimension_is_named_by_its_id_with_or_without_the_namespace`, `the_teleport_scale_divides_the_coordinate_scales` (1/8 into the Nether, 8 out of it, 1 between the overworld and the End, and (800, 70, -1600) becomes (100, 70, -200) in the Nether). `player`: `a_teleport_into_another_dimension_travels_there` (same dimension is plain, the Nether arrival and reason, a remote killstreak holds an End teleport until it ends).
- **After track C** (`Ctx.dimension` and `/execute in`, C commit 12, merged on `minecraft-survival` but not on this branch): give `CommandWorld::teleport` the target dimension (`minecraft_commands/src/server.rs`), and in the adapter's teleport (`minecraft_world.rs`, where `commands::run` hands back `teleports`) call `player.teleport_into(Place::standing(dimension, feet, yaw, pitch))` and only do the plain MW2 teleport when it answers `false`. If track C's `Ctx.dimension` is a string, `dimension_named` turns it into a `Dimension`; if C does not scale the position, `teleport_scale` and `scaled` do it. The brief's `FakeWorld` test (`/execute in minecraft:the_nether run tp ~ ~ ~` from (800, 70, -1600) asks for (100, 70, -200) in the Nether) goes with that wiring, in `minecraft_commands`.
- **Decisions:** none new. A teleport into another dimension uses the loading screen of that pair (`NETHER_PORTAL` with the Nether on either side, `END_PORTAL` with the End), as vanilla's client picks it from the two dimensions whatever moved the player.
- **Test steps:** none in game until the wiring after track C; then the brief's owner steps 1, 2 and 7 (`/execute in ... run tp`).
- **Kit:** none.

### 19. Docs: Minecraft dimension switch (E1-15)

Commit: 0b0e254b

- **Changed:**
  - `docs/MINECRAFT-SURVIVAL.md`: the Crate layout (`Server` with `Globals`, `opening` and level closing, `Level.dimension` and `empty_ticks`, the switch on `Player`, `switch.rs`, `respawn.rs`, and the adapter's `follow_levels`, leave, hold and loading screen); the Limitations (how the Nether and End can be entered today, one MW2 collision world, MW2 in other dimensions with the air support rule and the ordnance gaps); saving of the Nether and End; the `IW4L_MINECRAFT_DIMENSION` row in the switches table; the roadmap's runtime switch marked done.
  - `docs/NETHER-END.md`: the "What the game hard-wires to the overworld today" list rechecked, each item marked fixed or still open.
- **Decisions:** the bullet "Only the overworld level and the host's player exist so far" under "The Minecraft world is single player" is left as it is: the line above it changed on `minecraft-survival` (track B), and editing the next line would make the merge conflict. The orchestrator can drop that sentence at merge time; the Crate layout already says that levels open for any dimension.
- **Tests:** none (docs only).
- **Test steps:** none.
- **Kit:** none.

## Group S4 summary

- **Commits:** entries 15 to 19. No clippy fix was needed: `clippy -p sim -p minecraft_survival -p render_anim -p net -p playerstate_iw4 -p minecraft_terrain --tests --no-deps -- -A clippy::approx_constant` and `clippy -p minecraftoss-core --tests --no-deps` have no finding on a line of this group (the older ones are `entities.rs` 77 and 621, `voxel.rs` 47, 325 and 381, `minecraft_world.rs` 127, 385 and 394, and many more in those crates).
- **Tests:** `sim` (69), `minecraft_survival` (200 and the vanilla world test), `minecraft_terrain` environment tests, `minecraftoss-core` (23), `render_anim` (44), `playerstate_iw4` and `net` (no tests) pass, and `check -p launcher` is green.
- **Owner test for the whole group** (one world, two starts):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=s4test` (a new world) and `IW4L_MINECRAFT_DIMENSION=the_nether`. Start. Throw a frag at netherrack: it craters. Build a small obsidian wall and throw a frag at it: the obsidian holds. **Screenshot 1.**
  2. Open the console, `give killstreak/predator` (accept the suggestion), close it, and select the predator on the hotbar: it is not raised, and the chat says "Air support is unavailable under this dimension's ceiling". **Screenshot 2.** Do the same with `give killstreak/airdrop`: the same line.
  3. `give killstreak/uav` and select it: the UAV works in the Nether.
  4. `/kill`. After the respawn in the overworld, throw a frag at the ground (it craters), then select the predator: you fly it. **Screenshot 3.**
  5. Quit. Set `IW4L_MINECRAFT_DIMENSION=the_end` and start on the obsidian platform. `give killstreak/predator` and fly it at the main island to the west: it works and craters the end stone. **Screenshot 4.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*64,minecraft:netherrack*64,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

- **After track C:**
  - E1-11's wiring: `CommandWorld::teleport` with the dimension and `Player::teleport_into` in the adapter (entry 18), with the brief's `FakeWorld` test. Then the owner steps of entries 15 (step 3) and 16, and the brief's steps 1, 2 and 7.
  - `/time` per dimension from S1 (`server::default_clock`).
- **Known gaps (brief section 8):** a level-wide script `RadiusDamage` and killstreak turret bullets from before a switch still reach the new level; MW2 player damage from old ordnance is not filtered (the MW2 map is shared); the air support weapon names are from MW2's stock scripts and are checked in game (entry 17).
- **MinecraftOSS patches in this group:** `core/src/environment.rs` (`DimensionPresentation.has_ceiling`).
- **For the later tracks:** a living player travels with `Player::travel_to` (portals in N2 and X2), which waits for a remote killstreak; `switch_to` is only for the respawn and failed loads. The next free `pm_flags` bit for the glide is below `MC_NO_SKY` (`0x0400_0000`).

## Review fixes

An adversarial review of the whole track (base fc6d4bd, last track commit ef2cb99b). Each fix is its own commit; an entry names the previous commit's hash, its own hash is in `git log` under its subject.

### 20. Minecraft /weather changes the world's weather from every dimension

Previous commit: ef2cb99b

- **Defect:** since S1 the world's weather lives in `Globals.weather`, but `/weather` still went only to the `ServerSim` of the level the command ran in. In the Nether or the End that level cannot have weather and never reports it, so the command answered as done and changed nothing: back in the overworld the old weather went on. Vanilla's `WeatherCommand` calls `MinecraftServer.setWeatherParameters` (checked with javap), which sets the server's weather whatever level the source is in.
- **Changed:**
  - `minecraft_terrain/src/server.rs`: `weather_parameters(weather, duration)`, the `WeatherData` that `setWeatherParameters` writes (clear: the clear time; rain and thunder: the rain and thunder time both the duration), the same numbers `ServerSim::set_weather_for` gives a level.
  - `minecraft_survival/src/entities.rs`: `Entities.world_weather`; `Entities::set_weather` still sends the command to its level and also keeps the world's new weather there.
  - `minecraft_survival/src/server.rs`: `Server::follow_weather_commands` takes that weather from whichever level ran the command into `Globals.weather` and hands it to every other open level that has weather (the overworld when the command ran elsewhere). The level that ran it keeps its own command, so its rain fades in as before.
  - `render_anim/src/minecraft_world.rs` (`follow_levels`, one line): runs it every frame before the empty levels are counted.
- **Tests:** `minecraft_terrain::server`: `a_weather_command_sets_the_world_s_weather_like_set_weather_parameters` (the three kinds, an unknown one, and the same numbers as an overworld `ServerSim` after `set_weather_for`). `server`: `a_weather_command_in_any_dimension_sets_the_world_s_weather` (thunder typed in the Nether reaches `Globals` and the open overworld, the Nether still reports no weather; clear typed in the overworld reaches `Globals` and is not copied back over the overworld's own command; no command changes nothing).
- **After track C:** C's `Host::set_weather` sends the command to the overworld's `Entities` when the overworld is loaded. With this fix that reaches `Globals` from any dimension while the overworld is open; when it is closed, C's version should fall back to the command level's `Entities` (any level's `set_weather` now carries the world weather).
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=s4test`, `IW4L_MINECRAFT_DIMENSION=the_nether`, and no `IW4L_MINECRAFT_WEATHER` line. Start.
  2. In the Nether, type `/weather thunder 600s`. The chat says the weather changed.
  3. Type `/kill`. After the respawn in the overworld it is raining with thunder. **Screenshot 1.**
  4. Quit and start again: the dimension line is still there, so you start in the Nether. Type `/weather clear 600s`, then `/kill`: after the respawn the overworld is clear. **Screenshot 2.**
- **Kit:** none needed; any kit works.

### 21. Minecraft players switching into one level each arrive where they were sent

Previous commit: c07acad6

- **Defect (per-player state):** `Server::finish_opening` gave every player waiting for a level the one arrival its loader was opened with, which is the first waiting player's. A second player switching into the same Nether at another place (another `/execute in`, a portal later, or a saved place) was put at the first player's arrival instead of their own.
- **Changed:** `minecraft_survival/src/server.rs`: `LevelLoader.requested` is the arrival the loader was opened for (`open_levels` sets it). When the level opens, a waiting player with no arrival in that level, or with exactly that one, takes the loader's arrival (which a portal search in `prepare` will fill later); a player with an arrival of their own keeps it and is placed there.
- **Tests:** `server`: `players_switching_into_one_level_each_arrive_where_they_were_sent` (two players switching into the Nether at two roofs and a third with no arrival: one loader, each keeps their own roof, the third takes the level's arrival). Checked to fail before the fix.
- **Test steps:** none in game: it needs two players, which comes with multiplayer. Single player behaves as before.
- **Kit:** none.

### 22. IW4L_MINECRAFT_WEATHER sets the weather once at the start, not at every overworld reopen

Previous commit: c96e3a93

- **Defect:** `Entities::new` read `IW4L_MINECRAFT_WEATHER` every time a level with weather started. Since S2 the overworld closes 15 seconds after the player leaves it and starts again on the way back, so the variable overrode the world's live weather on every return (a `/weather clear` typed earlier came back as rain). The switches table says it overrides the saved weather at the start.
- **Changed:** `minecraft_survival/src/server.rs`: `Globals.forced_weather` holds the variable, read once with the world (`render_anim/src/minecraft_world.rs`, `load()`, one line). `start_level` hands it to the first level that has weather and clears it; `Entities::new` takes it as a parameter instead of reading the environment (`minecraft_survival/src/entities.rs`, two lines). The log line `Minecraft weather: rain from IW4L_MINECRAFT_WEATHER` is unchanged.
- **Tests:** `server`: `the_weather_variable_starts_the_overworld_once_and_not_again_when_it_reopens` (a Nether start keeps it, the overworld takes it and thunders, and after closing and reopening the overworld goes on with the world's clear weather).
- **Test steps** (with entry 20's fix):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=s4test`, `IW4L_MINECRAFT_WEATHER=rain` and `IW4L_MINECRAFT_DIMENSION=the_nether`. Start. The log says `Minecraft weather: rain from IW4L_MINECRAFT_WEATHER` (the overworld opens with the world and takes it).
  2. In the Nether, type `/weather clear 600s`.
  3. Wait 20 seconds (the log says `Minecraft minecraft:overworld level saved and closed`), then `/kill`. After the respawn the overworld is clear, not raining. **Screenshot 1.**
  4. Remove the weather and dimension lines afterwards.
- **Kit:** none.

### 23. Minecraft stacks picked up while leaving a level are dropped there when they don't fit

Previous commit: 198ea8e7

- **Defect:** `Entities::leave` puts the stacks the old level already let the player pick up into the inventory, but threw away whatever did not fit. The old level had already removed those item entities, so with a full inventory they were lost. The normal pickup drops the rest back at the player's feet.
- **Changed:** `minecraft_survival/src/entities.rs`: `take_picked` returns what did not fit, and `Entities::leave(inventory, selected, feet)` spawns it as an item at the player's feet in the level they leave, as the normal pickup does. `render_anim/src/minecraft_world.rs` (`leave_level`, one line) passes the player's place.
- **Tests:** `entities`: `stacks_that_no_longer_fit_on_leaving_come_back_to_be_dropped` (a full inventory gives the whole picked stack back with its components); `stacks_the_old_level_let_the_player_pick_up_reach_the_inventory_on_leaving` now also checks nothing comes back when everything fits.
- **Known gap left as it was (entry 11's decision):** an item use already sent to the old level's server when the player leaves (a bucket on a block) is forgotten: if that server still carries it out, the block changes but the item stays in hand. A leave right after a use needs a living switch, which only `/execute in` (after track C) and the portals (N2) start; N2 should let the waiting use finish before the leave.
- **Test steps:** none reliable in game: it needs a pickup in the very frame a switch starts.
- **Kit:** none.

## Final verification

Previous commit: a9e2a2e7

- **Crates touched since fc6d4bd:** `frame`, `hud`, `minecraft_survival`, `minecraft_terrain`, `net`, `playerstate_iw4`, `render_anim`, `sim`, `minecraftoss-core` and `minecraftoss-world`.
- **Clippy:** `clippy -p frame -p hud -p minecraft_survival -p minecraft_terrain -p net -p playerstate_iw4 -p render_anim -p sim -p minecraftoss-core -p minecraftoss-world --locked --all-targets -- -A clippy::approx_constant` has no finding on a line this track added (checked against `git diff -U0 fc6d4bd..HEAD`). The findings left are older: `entities.rs` 77 and 628, `minecraft_terrain/src/server.rs` 311, 1087 and 1278, `terrain.rs` 657, `minecraft_world.rs` 127, 386 and 395, `voxel.rs` 47, 325 and 381, and many more in those crates. No fix was needed.
- **Tests:** `frame` 14, `hud` 111, `minecraft_survival` 204 and the vanilla world test, `minecraft_terrain` 492 (3 ignored as before), `net` and `playerstate_iw4` (no tests), `render_anim` 44, `sim` 69, `minecraftoss-core` 23 and `minecraftoss-world` 59 (with the full suite's stand-in `third_party/scenarios` files) all pass.
- **Owner test for the whole track:** the group tests in order, each in its own new world: entry 9 (S2, `s2test`), entry 13 (S3, `s3test`), the S4 summary (`s4test`), then entries 20 and 22 in `s4test`. Entries 21 and 23 have no in-game steps. Each step list is in its entry.
- **Kit for the whole track** (8 stacks, the hotbar stays free for the killstreaks of entry 17):

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:obsidian*64,minecraft:netherrack*64,minecraft:cobblestone*64,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:torch*64,minecraft:cooked_beef*32
```

## After track C

Track C (`/execute`, `Ctx.dimension`, `/locate`, `/kit`) is merged on this branch (merge 9dbec02a). These entries do what the earlier entries left "After track C". Each names the previous commit's hash; its own hash is in `git log` under its subject.

### 24. Minecraft /execute in moves players into another dimension (E1-11)

Previous commit: 9dbec02a

- **Vanilla, checked with javap:** `CommandSourceStack.withLevel` scales the source position's x and z by `DimensionType.getTeleportationScale(old, new)` and keeps y, and `/execute in` does nothing else to it. `TeleportCommand.teleportToPos` resolves the location against that scaled source and calls `Entity.teleportTo(source.getLevel(), x, y, z, ...)`: the relative flags only matter inside one level, so `~` in another dimension is the scaled position. `DimensionArgument.getDimension` takes any level of the server, and vanilla's server always has all three. Track C's `/execute in` already scales like `withLevel` and asks `teleport_to_dimension` when the target is in another dimension, so nothing changed in `minecraft_commands` but a test.
- **Changed:**
  - `minecraft_survival/src/commands.rs` (`Host`):
    - `dimension_scale` answers every real dimension (`switch::dimension_named`), open or closed, with its `coordinate_scale` from the data pack (`Host.registries`, the server's `Registries`). Before, a closed level was `Unknown dimension`.
    - `teleport_to_dimension` calls `Player::teleport_into` with the position and the player's own angles, as vanilla keeps them without a rotation argument. The switch opens the level, shows the loading screen of that pair and waits for a remote killstreak. A dead player is not moved, as with a plain `/tp`.
    - `locate` in a closed level keeps the request in `Globals.waiting_locates` and answers later. `locate_job` builds the search, shared with the level start.
    - `run` collects job answers from every open level, not only the one being ticked (`Entities::poll_jobs`), so `/execute in` a level the player is not in still answers `/locate`, `/setblock` and `/fill`.
  - `minecraft_survival/src/entities.rs`: `poll_jobs` takes the job answers from the level's server output and keeps the rest of that output for the next tick, which handles it as before. `runs_jobs` is true while a job sent to the level has not answered.
  - `minecraft_survival/src/server.rs`: `open_levels` opens the level of a waiting search, `start_level` hands it its searches, a level that fails to open drops them, and `close_empty_levels` keeps an empty level open while it still searches.
  - `render_anim/src/minecraft_world.rs`: when a command starts a switch, the frame shows the command's chat lines and stops there, as the respawn switch does, so nothing of the old level runs with the new dimension. The chat suggests all three dimensions after `/execute in`, as vanilla lists every level (`switch::DIMENSIONS`).
  - `docs/MINECRAFT-SURVIVAL.md`: the `/execute in` limitation and the roadmap line.
- **Tests:** `minecraft_commands::execute`: `execute_in_the_nether_asks_for_an_eighth_of_the_overworld_position` (the brief's `FakeWorld` test: from (800, 70, -1600), `tp ~ ~ ~` asks for (100, 70, -200) in the Nether). `commands`: `execute_in_teleports_the_player_into_another_dimension_through_the_switch` (the Nether roof at an eighth with the player's angles and the `NETHER_PORTAL` switch; a teleport inside the same dimension stays plain; a remote killstreak holds the way back; a dead player stays), `execute_in_finds_every_real_level_open_or_not` (was `execute_in_finds_only_loaded_levels`: closed Nether and End are found, a lookup opens nothing, an unknown dimension is still refused), and `locate_answers_once_the_level_has_searched` now expects a search in a closed level to wait. `server`: `a_search_in_a_closed_level_opens_it_and_keeps_it_open_until_answered` (opens the Nether for it, keeps it open past 300 empty ticks while searching, the answer is a Nether biome, then it closes), and `a_level_that_fails_to_open_sends_its_players_back` also drops the searches for that level only.
- **Decisions:**
  - A level opens when a command needs it (a teleport through the switch, a `/locate`), not on every lookup: `/execute in minecraft:the_end run seed` opens nothing. A level opened for a search closes 15 s after it answered, like a level a player left.
  - `/setblock` and `/fill` in a closed level answer `That position is not loaded`. Vanilla 26.x has no spawn chunks, so with nobody in the Nether vanilla answers the same.
- **Test steps** (brief steps 1, 2 and 7, then the waiting steps of entries 15 and 16):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=e24test` (a new world) and remove any `IW4L_MINECRAFT_DIMENSION` line. Start, type `/gamemode creative` (so the falls below don't kill), and walk about 100 blocks from spawn.
  2. Open the chat and type `/execute in ` : the suggestions list `minecraft:overworld`, `minecraft:the_nether` and `minecraft:the_end`. **Screenshot 1.**
  3. Send `/execute in minecraft:the_nether run tp ~ 128 ~`. The chat says `Teleported Steve to ...` with about an eighth of your x and z, the purple portal background shows with "Loading terrain...", then you stand on the bedrock roof. **Screenshots 2 (loading screen) and 3 (Nether roof).**
  4. Send `/execute in minecraft:overworld run tp ~ ~ ~`. You are back in the overworld at 8 times those x and z, at y 128. After about 15 s the log says `Minecraft minecraft:the_nether level saved and closed`.
  5. Send `/execute in minecraft:the_nether run locate structure minecraft:fortress`. After a moment the chat answers `The nearest minecraft:fortress is at [...]` with Nether coordinates, and the log says the Nether level opened and, 15 s later, closed. **Screenshot 4.**
  6. Send `/execute in minecraft:the_end run tp 100 50 0`. The end portal background shows, then you are on the obsidian platform. Type `give killstreak/predator` in the console (accept the suggestion) and use it: it works. **Screenshot 5.**
  7. `/kill`, and after the respawn in the overworld throw a frag at the ground and at once send `/execute in minecraft:the_nether run tp ~ 128 ~` (type it once beforehand so Up and Enter bring it back). In the Nether there is no crater from that frag. **Screenshot 6.**
  8. Send `/execute in minecraft:overworld run tp ~ ~ ~`. Back in the overworld, `give killstreak/predator` in the console, select it to fly it, and while steering open chat and send `/execute in minecraft:the_nether run tp ~ 128 ~`. Nothing happens until the missile hits; then the Nether loading screen shows. **Screenshot 7.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:obsidian*64,minecraft:netherrack*64,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

### 25. Minecraft /time acts on the clock of the command's dimension (S1, after track C)

Previous commit: f0c30259

- **Vanilla, checked with javap:** every clock node of `TimeCommand` (`set`, `add`, `pause`, `resume`, `query time`) first takes `getDefaultClock(source)`: the `default_clock` of the source level's dimension type, or `commands.time.no_default_clock` ("There is no default clock in dimension %s", with the dimension type's id). `query gametime` is not a clock node and works everywhere. 26.3's data pack gives `minecraft:overworld` the `minecraft:overworld` clock, `minecraft:the_end` the `minecraft:the_end` clock and the Nether none. Time markers come from the timelines of a clock; only the overworld clock has any (`timeline/day.json`), so a marker in the End answers `commands.time.no_time_marker_found` with `minecraft:the_end`.
- **Changed:**
  - `minecraft_commands/src/server.rs`: `CommandWorld::clock`, `set_time` and `set_time_paused` take the clock id, and `CommandWorld::default_clock(dimension)` names it (by default the overworld clock, so a world without dimensions behaves as before).
  - `minecraft_commands/src/commands.rs`: every clock node takes the default clock of `Ctx::dimension()` first and fails with `commands.time.no_default_clock` without one; the messages name that clock; time markers belong to the overworld clock only. `query gametime` is unchanged.
  - `minecraft_survival/src/commands.rs` (`Host`): `default_clock` goes through `server::default_clock`; `Host.end_clock` is `Globals.end_clock`, which `clock`, `set_time` and `set_time_paused` act on for `minecraft:the_end`. The overworld clock keeps its game time shift; the End clock has none, as before.
  - `docs/MINECRAFT-SURVIVAL.md`: the `/time` line.
- **Tests:** `minecraft_commands::commands`: `time_acts_on_the_default_clock_of_the_command_s_dimension` (every clock node refused in the Nether, `query gametime` still answers, `/execute in minecraft:overworld run time set` reaches the overworld clock from the Nether, and in the End `set`, `add`, `pause` and `query time` act on `minecraft:the_end` while a marker is refused). `minecraft_survival::commands`: `time_follows_the_default_clock_of_the_player_s_dimension` (through `Host` and `server::default_clock`: nothing changes from the Nether, the End sets and pauses only `Globals.end_clock`, the overworld only the day).
- **Decisions:** the chat still suggests the overworld's time markers after `/time set` in the End and the Nether; vanilla suggests none there. The suggestions don't know the dimension yet.
- **Test steps** (with entry 24's world and kit):
  1. Keep `IW4L_MINECRAFT_WORLD=e24test` and remove any `IW4L_MINECRAFT_DIMENSION` line. Start in the overworld and type `/time query time`: `Clock minecraft:overworld is at ... tick(s)`.
  2. Send `/execute in minecraft:the_nether run tp ~ 128 ~`. In the Nether, type `/time set day`: a red `There is no default clock in dimension minecraft:the_nether`. `/time query gametime` still answers. **Screenshot 1.**
  3. Send `/execute in minecraft:the_end run tp 100 50 0`. Type `/time query time`: `Clock minecraft:the_end is at ... tick(s)`. `/time set noon`: a red `Time marker minecraft:noon does not exist for clock minecraft:the_end`. **Screenshot 2.**
  4. Type `/time set 18000`, then `/time pause`: `Paused clock minecraft:the_end`. Send `/execute in minecraft:overworld run time query time`: the overworld clock did not move. **Screenshot 3.**
  5. Quit through the menu and start again (you start in the End where you left). `/time query time` still says about 18000, as the clock is paused and saved. `/time resume`: `Resumed clock minecraft:the_end`. **Screenshot 4.**
- **Kit:** none needed; any kit works.

### 26. Minecraft /kit gives potion and colour entries like the start kit

Previous commit: 2d9fe216

- **Defect:** track C's `/kit` read the kit line with the `/give` item parser, which stops at `@`, so `splash_potion@poison` or `leather_leggings@red` were skipped without a word, while the start kit gives them (track B's `@` syntax).
- **Changed:**
  - `minecraft_commands/src/args.rs`: `ItemArg.potion`, the potion or dye named after `@`. The `/give` parser leaves it empty.
  - `minecraft_commands/src/server.rs` and `kit.rs`: `/kit` reads its entries through `CommandWorld::kit_entries(line)`. By default that is the old `kit_entries`, so a world that doesn't override it behaves as before.
  - `minecraft_survival/src/commands.rs` (`Host`): `kit_entries` reuses the start kit's parser (`crafting::kit`, `kit_enchantments` and `kit_potion`, in the order `give_kit` uses them), and `give` builds the stack with `kit_potion_stack`, so a potion gets `minecraft:potion_contents` and a dyeable item `minecraft:dyed_color`, exactly as at the start. `/give` passes no potion and builds the same stacks as before.
- **Tests:** `minecraft_survival::commands`: `kit_gives_potions_and_colours_like_the_start_kit` (`/kit` with potions, tipped arrows, a coloured chestplate, an empty `@` and a zero count gives four entries, and the stacks equal `give_kit`'s for the same line, the healing potion's `potion_contents` included). Track C's `kit` tests still pass unchanged.
- **Test steps:**
  1. In the `.env`, set the kit below and `IW4L_MINECRAFT_WORLD=e26test` (a new world). Start: the backpack holds the kit.
  2. Drop or `/clear` everything, then type `/kit`. The chat shows one `Gave ...` line per entry, eight in all, and the splash potions, the lingering potion, the tipped arrows and the red leather leggings come back like at the start: hover each to see its effect, and the leggings are red. **Screenshot 1.**
  3. Throw a splash potion of poison at your feet: you get poisoned. **Screenshot 2.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:splash_potion@poison*2,minecraft:splash_potion@strong_healing,minecraft:lingering_potion@regeneration,minecraft:tipped_arrow@slowness*8,minecraft:leather_leggings@red,minecraft:bow,minecraft:diamond_sword+sharpness=5,minecraft:cooked_beef*16
```

## After track C: final checks

- **Commits:** entries 24 to 26 on top of the merge 9dbec02a.
- **Clippy:** `clippy -p minecraft_commands -p minecraft_survival -p render_anim --locked --all-targets --no-deps -- -A clippy::approx_constant` has no finding on a line of entries 24 to 26 (one in a new test was fixed before the commit). The rest are older: `entities.rs` 78 and 706, `minecraft_world.rs` 131, 401 and 410, and many more in `render_anim`.
- **Tests:** `minecraft_commands` 116, `minecraft_survival` 255 and the vanilla world test, `render_anim` 67, `hud` 130 and `frame` 16 pass, and `check -p launcher` is green.
- **Owner test for all three:** entry 24 (world `e24test`), then entry 25 in the same world, then entry 26 in a new world `e26test`, each with its own kit.
