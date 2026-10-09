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

Commit: filled in by `git log` under this subject

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
