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

Commit: filled in by the next entry

- **Changed:**
  - `minecraft_terrain/src/terrain.rs`: `TerrainStream::for_dimension_at` keeps a known spawn for the Nether and the End too, at the block's bottom centre, instead of always searching the stand-in Nether floor around chunk 0,0 or always using the End platform. The search and `ServerLevel.END_SPAWN_POINT` (100, 49, 0) stay the fallbacks. S2's level loader passes the saved or arrival position here.
  - The new tests are the first run of the Nether and End generators in the fork (NETHER-END.md said they had never run). Nothing panicked and nothing had to be patched: no MinecraftOSS patch in this commit.
- **Tests:** `terrain`: `nether_chunks_generate_light_and_tick_around_the_origin` generates the 5x5 chunks around 0,0 of seed 77 into a real world folder. Every column has bedrock at y 0 and 127 and air above the roof, the dimension is 256 blocks from y 0 without sky light, the stand-in spawn has two air blocks under the roof, there is netherrack and lava and no overworld or End block; then the chunks are lit, a Nether `ServerSim` ticks them 40 times with the world clocks, the region files land in `dimensions/minecraft/the_nether/region`, and a known spawn is kept with no chunk generated. `end_chunks_generate_the_main_island_around_the_origin` does the same for the End: the platform spawn with no search, end stone under 0,0 and more than 10,000 blocks of it, obsidian from the end spikes, no overworld or Nether block, lighting, 40 ticks of an End `ServerSim`, its region folder, and a known spawn kept.
  - What the run produced (seed 77, chunks -2..2): the Nether has netherrack, lava, magma, blackstone, gravel, soul sand and soil, quartz and gold ore, ancient debris, glowstone and a crimson forest (nylium, stems, wart blocks, shroomlights, roots, fungi, weeping vines); the End has end stone and the end spikes with their iron bar cages, bedrock tops and fire.
- **Decisions:** the smoke tests stop at what the headless crates can run (generation, light, the server level). Section meshes need the resource pack and run in game; they are not tested here.
- **Test steps:** none in game; the Nether and the End can be entered from S2 on (`IW4L_MINECRAFT_DIMENSION`).
- **Kit:** none.
