# Track E1: the dimension switch (design brief)

Goal: one server holds all three levels. The player moves between them behind a vanilla-like loading screen, a level nobody is in saves and closes, death in the Nether or End respawns at the right spot, and MW2 weapons work in every dimension. Paths are as of a43d0fc. This brief needs track D (saving) merged first and track C's `/execute in` parser (C commit 12).

## 1. What exists today

- **Server** (`minecraft_survival/src/server.rs:19-23`) already has `levels: BTreeMap<Dimension, Level>`, but only the overworld is ever inserted (`render_anim/src/minecraft_world.rs:280-281`). Load completion looks up the overworld by name (:434).
- **Overworld hard-wiring that must go:**
  - `load()` opens only the overworld stream and environment (`minecraft_world.rs:225-233`, :249-250).
  - `Entities::new` starts `ServerSim::new(.., "minecraft:overworld")` (`entities.rs:160`) and applies `IW4L_MINECRAFT_WEATHER` to every level (:162-168).
  - `Player::new` sets `Dimension::Overworld` (`player.rs:65`).
  - `save.rs:16` `OVERWORLD` is used for the spawn's dimension (:150), the clock (:168), the only folder created (:259), the only spawn read back (:262) and the clock read back (:276).
  - **Bug:** `Server::save_and_close` **writes neither level.dat nor the player file when the overworld isn't loaded** (`server.rs:44-50`).
  - **Bug:** `autosave` saves one level and writes that level's `world_spawn` (:108-147, called at `minecraft_world.rs:1462`), so an autosave in the Nether would save the Nether stand-in spawn.
- **Per frame**, `update()` already picks the level by `player.dimension` (`minecraft_world.rs:556`). So block shots (`minecraft_mining.rs:297-305`; bedrock's destroy speed is -1) and craters (`weapon_blast` on the level's own `ServerSim`, :1306-1309) already hit the current level with vanilla blast resistance.
- **Collision:** one process-wide voxel world (`sim/src/voxel.rs:41`), set up once with `activate(brushes, origin, shapes)` (:403-414) and cleared by `deactivate` (:416-429). `view.origin` (the block at MW2 map origin) is the overworld `player_spawn` (`minecraft_world.rs:446-447`). The MW2 spawn is always map origin (`voxel.rs:604-610`, `sim/src/spawn.rs:244-247`, `script_player.rs:417-430`). A player whose chunk has no collision is frozen (`sim/src/step.rs:296`, `voxel.rs:592-594`).
- **SpawnGate** (`render_anim/src/minecraft_spawn_gate.rs`) already matches vanilla's `LevelLoadTracker` timings, but it watches only the origin's chunk (`minecraft_world.rs:2115-2124`). `ui.loading_world` only gates admission (`session/src/admission.rs:33`), so nothing is drawn mid-match.
- **Terrain:** `TerrainStream::for_dimension_at` picks the Nether or End generator (`minecraft_terrain/src/terrain.rs:1017-1064`), and that code has never run (NETHER-END.md, re-checked). For the Nether it always runs a stand-in floor search around chunk 0,0 and ignores `known_spawn` (:977-1004, :1058-1061). Each stream starts about 1.25 threads per core (:1028-1037).
- **MinecraftOSS keeps clocks and weather per level** (re-checked, `third_party/minecraftoss/world/src/level/time.rs:62-65`, :93-99, :126-165). A Nether level's overworld clock stays 0.
- **Commands:** `CommandWorld::teleport` has no dimension (`minecraft_commands/src/server.rs:76`). The adapter teleports within the current level (`minecraft_world.rs:1551-1554`).
- **Killstreaks** are the stock MW2 scripts on the `mp_rust` proxy map (`assets/src/minecraft_map.rs:6-9`), flying at heights relative to map origin. Blasts crater through `voxel::push_explosion` (`sim/src/damage.rs:106-113`, :412-419). `ExplosionBlast.killcam_entity_start_time` (:77) is the projectile's spawn time.

## 2. Vanilla 26.3, checked with javap

- **All levels always exist.** `MinecraftServer` keeps every `ServerLevel`. `ServerLevel.tick` stops ticking entities once `emptyTime` reaches `EMPTY_TIME_NO_TICK` = 300 with no active tickets. Chunks unload when their tickets go.
- **No spawn chunks in 26.x.** `TicketType` has no start ticket. `portal` lasts 300 ticks with `FLAG_KEEP_DIMENSION_ACTIVE`, and `ender_pearl` lasts 40.
- **Weather and clocks are server-wide.** `ServerLevel.getWeatherData` and `clockManager` both return the server's. `advanceWeatherCycle` changes the weather only in a level where `canHaveWeather` is true, which in practice is the overworld, and the overworld always ticks.
- **Loading screen.** `ClientPacketListener.determineLevelLoadingReason(dead, from, to)` gives `OTHER` when dead, `NETHER_PORTAL` when either side is the Nether, `END_PORTAL` when either side is the End, and `OTHER` otherwise.
  - `LevelLoadingScreen.extractBackground` draws the `nether_portal` block sprite over the whole screen, the end portal effect (`end_sky` plus `end_portal` textures), or the panorama, blur and menu background.
  - It shows `multiplayer.downloadingTerrain` ("Loading terrain..."). The progress bar shows only when the server reports a load stage.
  - It closes `LEVEL_LOAD_CLOSE_DELAY_MS` = 500 after the level is ready, and `CLIENT_WAIT_TIMEOUT_MS` is 30 s.
- **Invulnerable while arriving.** `ServerPlayer.isInvulnerableTo` is true while `isChangingDimension()` (except pearl damage) or until `hasClientLoaded()`, which times out after `CLIENT_LOADED_TIMEOUT_TIME` = 60 ticks.
- **Respawn.** `ServerPlayer.findRespawnPositionAndUseSpawnBlock` takes the dimension from `RespawnConfig.getDimensionOrDefault` (overworld when unset) and tries the bed or anchor there. A missing block gives `TeleportTransition.missingRespawnBlock`, and no config gives `createDefault`. Both use `MinecraftServer.findRespawnDimension()`, which is the level.dat spawn's dimension with the overworld as fallback.
- **`/execute in`.** `CommandSourceStack.withLevel` scales x and z by `DimensionType.getTeleportationScale` (old `coordinate_scale` / new). `/time` in a level without a default clock fails with `commands.time.no_default_clock` (`TimeCommand`).

## 3. Design

- **Levels on demand.** A new `minecraft_survival::server::LevelLoader` builds a `Terrain` (stream, scene, `DimensionEnvironment::load(.., dimension.dimension_type())`) on a `minecraft-level-load` thread. It first joins any save still closing that dimension, as `load()` does (`minecraft_world.rs:411-413`). `Entities::new(.., dimension)` and the containers follow on the main thread. One `ServerSim` thread per level stays; one loop for all levels is multiplayer work.
- **Closing.** A level nobody is in or switching into counts `empty_ticks`. At 300 (vanilla's empty time and portal ticket) it leaves `levels` and drops on a `minecraft-level-save` thread kept in `Globals.closing`, entities first, then chunks. A return within 15 s reuses it. Empty levels are not ticked.
- **Globals own the world spawn** (`RespawnData` with its dimension) **and the weather.** Both are written from `Globals`.
  - While the overworld is loaded, its `ServerSim` runs the weather and reports it back.
  - While it isn't, `Globals` advances the timers itself with a vendor helper lifted out of `advance_weather_cycle`.
  - Every level's overworld clock is set from `Globals.day`.
- **Collision.** Keep one voxel world, mirroring the local player's level. A new `voxel::switch_level(origin)` drops the chunks and shapes, keeps the brush pointer, drains the queued events and records the switch time.
  - The adapter keeps one origin per loaded level. The overworld keeps `player_spawn`, so the MW2 respawn at map origin is still the overworld spawn. The Nether and End use their first arrival point.
- **Switching** (headless state in `Player.switching: Option<Switch { to, arrival, reason }>`, presentation in the adapter):
  1. **Start.** Clear the voxel chunks, so `awaiting_ground` freezes the player. Raise the loading screen and set `MC_INVULNERABLE` (vanilla's `isChangingDimension`).
  2. **Leave.** Close any container screen. Stacks the old level already picked up (`entities.rs:75`) go into the inventory, a waiting item use (:96) is cancelled, and pending pearl landings (:89) are dropped.
  3. **Arrive.** `switch_level(origin)`; reset the shape tables, light, clouds, environment, mining and minimap; bump `view.generation`. Set `steps.air_peak = None`, clear `hand.recent`, and teleport with track D's `teleport_facing`.
  4. **Hold.** The gate watches the arrival chunk. Mob hits are skipped until it releases, or for at most 60 ticks.
  - A level that fails to load leaves the player where they were, with a chat line.
- **Respawn.** On death, the target is track D's `respawn` config, or else `Globals.spawn`, and its level starts loading at once. When MW2 shows the player alive again, they switch if the dimension differs (reason `OTHER`), then teleport.
  - The overworld spawn needs no teleport: it is map origin.
  - Bed or anchor validity (`findRespawnAndUseSpawnBlock`) is checked on the loaded level. A missing block falls back to the world spawn, as `missingRespawnBlock` does.
- **Air support.** Under the Nether roof (`has_ceiling`), killstreak aircraft would sit in netherrack or above the bedrock, and their ordnance would only hit the roof. The decision follows vanilla's "no sky, no sky things" (beds explode, no weather) and MW2's EMP.
  - **Nether:** airstrikes, stealth bomber, harrier, AC-130, helicopters, chopper gunner, predator and care packages are refused before use. The streak is kept, and a line says air support is unavailable.
  - **End and overworld:** they work. Bombs that miss the End islands fall into the void.
  - **Hook:** a new `pm_flags::MC_NO_SKY` bit (unused by IW4 and the fork; see `playerstate.rs:148-187`), set in `ability_bits`. The client action-slot select (`net/src/client/runtime.rs:1015-1040`) skips air-support `killstreak_*` weapons while it is set. The authority check comes with multiplayer.
- **Ordnance left behind.**
  - A switch waits while the player is in a remote killstreak (AC-130 `other_flags::AC130`, the predator camera, the chopper gunner). That is the analogue of vanilla's rule that a passenger can't use a portal.
  - Blasts whose `killcam_entity_start_time`, or whose script inflictor's spawn time, is before the switch time make no crater and no mob blast.
- **Debug start.** `IW4L_MINECRAFT_DIMENSION=overworld|the_nether|the_end` (with or without `minecraft:`) overrides the saved dimension. The player arrives at their saved position if it is in that dimension; otherwise at the stand-in Nether floor or the End platform (`terrain.rs:1063`).

## 4. What E1 needs from track D (and C)

- `PlayerData` read at join with `dimension`, `pos` and `rotation`. `Player::new` takes the dimension, and the first arrival uses the position.
- `Player.respawn: Option<RespawnConfig { dimension, pos, yaw, pitch, forced }>` plus `LastDeathLocation`.
- `ServerSim::weather_data` / `set_weather_data`, and `LevelStorage` read/write of `weather.dat` (D commit 2).
- `Frame::teleport_facing` and a SpawnGate that watches a target chunk (D commit 7). If D lands without the target, E1 adds `SpawnGate::start_at(now, block)`.
- `WorldSave::write_player` (D commit 4) writing `Dimension`.
- From C: `Ctx.dimension` and the scaling (C commit 12). E1 adds the dimension to `CommandWorld::teleport`.

## 5. Commits

1. **Minecraft world spawn lives in the server, not the overworld level**: `Globals.spawn`. `save_and_close` and `autosave` use it, `autosave` queues every loaded level, and `create_dimension` runs per level.
2. **Minecraft levels share the world's clocks**: a vendor patch makes `set_time` write the overworld clock in every level. `/time` without a default clock answers `commands.time.no_default_clock`.
3. **Minecraft weather keeps changing while nobody is in the overworld**: `Globals.weather`, with the vendor `Weather::advance_timers` helper.
4. **Minecraft Nether and End terrain generates in a test**: a headless generation test, and `for_dimension_at` keeps `known_spawn` for the Nether and End. This is an edition 2021 file, so no let-chains.
5. **Minecraft levels open on demand for any dimension**: `LevelLoader`, `Entities::new(.., dimension)`, and the weather variable applied only where weather is possible.
6. **Minecraft levels nobody is in save and close after 15 seconds**: `empty_ticks`, `Globals.closing`, and `close_world` joins them.
7. **MW2 collision switches to the player's new Minecraft level**: `voxel::switch_level`, and the origin per level.
8. **Minecraft players travel between dimensions behind a loading screen**: the switch from section 3.
9. **Minecraft level loading screen looks like vanilla**: `frame::McLevelLoading { reason }` and a `hud/src/minecraft_loading.rs`. The text comes from the pack's language file and the textures from the packs at runtime.
10. **Minecraft players respawn in their respawn dimension**: the death preload and the respawn switch.
11. **Minecraft /execute in moves players between dimensions**: `CommandWorld::teleport(player, dimension, pos)`.
12. **IW4L_MINECRAFT_DIMENSION starts the player in the Nether or the End**
13. **MW2 ordnance from before a dimension switch leaves the new level alone**
14. **MW2 air support is unavailable under the Nether roof**
15. **Docs: Minecraft dimension switch**: MINECRAFT-SURVIVAL.md (Crate layout, Limitations, the switches table) and the "hard-wired" list in NETHER-END.md.

## 6. Unit tests

- **Server:** an empty level closes at exactly 300 ticks; a level with a player, or with one switching in, never closes; a return resets the count. `save_and_close` writes level.dat and the player file with no overworld loaded, keeping the spawn's dimension.
- **`loading_reason(dead, from, to)`:** every pair, plus death, as `determineLevelLoadingReason`.
- **Teleport scale:** 1/8 into the Nether, 8 out of it, 1 between the overworld and the End.
- **Respawn target:** no config gives the world spawn's dimension, a Nether anchor gives the Nether, and a missing block falls back to the world spawn.
- **Weather and clocks:** the `Globals` timers advance with no overworld loaded and are handed to it on load; a Nether level's overworld clock follows the day.
- **Voxel:** after `switch_level` there are no chunks, the origin is new, `awaiting_ground` is true and older events are gone.
- **Damage:** a blast from a projectile spawned before the switch makes no crater.
- **Air support:** `ability_bits` sets `MC_NO_SKY` only under a ceiling, and the action-slot select skips air-support weapons only then.
- **Terrain** (with `MINECRAFTOSS_ROOT`): Nether and End chunks generate, and a known Nether spawn skips the search.
- **Commands** (`FakeWorld`): `/execute in minecraft:the_nether run tp ~ ~ ~` from (800, 70, -1600) asks for (100, 70, -200) in the Nether.
- **`IW4L_MINECRAFT_DIMENSION`:** `the_nether` and `minecraft:the_end` parse, anything else warns.

## 7. Owner test

`.env`: `IW4L_MINECRAFT_WORLD=dimtest`, plus:

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:obsidian*64,minecraft:cobblestone*64,minecraft:netherrack*64,minecraft:torch*64,minecraft:cooked_beef*32,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:chest,minecraft:ender_pearl*16,minecraft:zombie_spawn_egg*2,minecraft:totem_of_undying,minecraft:water_bucket,minecraft:glass*64
```

1. Start normally and walk about 100 blocks from spawn. Type `/execute in minecraft:the_nether run tp ~ 128 ~`. You see the purple portal background with "Loading terrain...", then you stand on the bedrock roof at about a tenth of your x and z. **Screenshots 1 (loading screen) and 2 (Nether).**
2. Type `/execute in minecraft:overworld run tp ~ ~ ~`. You are back in the overworld at 8 times those x and z. After about 15 s the log says the Nether level saved and closed.
3. Quit. Add `IW4L_MINECRAFT_DIMENSION=the_nether` and start. You start inside a Nether cave.
   - Shoot netherrack: it breaks fast.
   - Build an obsidian wall and throw a frag against it: netherrack craters, but obsidian holds.
   - Light TNT next to netherrack.
   - **Screenshot 3.**
4. Open the console, `give killstreak/predator` (accept the suggestion), then use it: an "unavailable" line shows, and the streak stays. **Screenshot 4.**
5. `/kill`. Respawn: the loading screen (menu background) shows, then you stand at the overworld spawn. **Screenshot 5.** If track C is in: `/spawnpoint` somewhere else, go to the Nether, `/kill`, and you respawn at that point.
6. Remove the dimension variable. Go to the Nether with step 1, then quit through the menu and restart: you are back in the Nether at the same spot.
7. `/execute in minecraft:the_end run tp 100 50 0`. The end portal background shows, then you are on the main island. Use the predator: it works. **Screenshot 6.**

## 8. Risks

- **First run of the Nether and End generators:** expect panics or bad chunks. A load error must fail the switch, not crash.
- **Two `ChunkStorage`s on one dimension** (a closing save and a reopen) corrupt region files. Always join the closing save first.
- **Threads:** at most two streams for one player; friends need the shared budget (NETHER-END groundwork 7).
- **One voxel world:** bots and all MW2 entities follow the host's level. Friends in other dimensions need a voxel world per dimension, keyed by client (CRATE-SPLIT risk 10).
- **Gaps:** helicopter turret bullets after a switch can still chip blocks. Check the killstreak weapon names and the "unavailable" string against the zone. Server outputs have no player key, so whatever isn't flushed at a switch is lost, and cross-dimension pearls (vanilla 1.21.2+) are dropped.
- **Overlaps:** track D (spawn, weather, respawn, gate target), C (`/execute`, `/spawnpoint`), B (beds) and the Nether track (portals, sky, clouds, fog). The Nether sky is still wrong in this test.
