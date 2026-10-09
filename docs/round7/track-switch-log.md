# Track SWITCH log: the dimension switch

Branch `dim-switch`, worktree `C:\Dev\mc-wt\switch`. Briefs: `e1-dimension-switch.md` (commits E1-1 to E1-15) and `e0-phase3-plan.md` (groups S1-S4). Base: fc6d4bd (track D merged).

A commit can't name its own hash, so each entry is written with its commit and its hash is filled in by the next commit; the last entry's hash is in `git log` under its subject.

## Group S1: server-wide state

### 1. Minecraft world spawn lives in the server, not the overworld level (E1-1)

Commit: filled in by the next entry

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
