# Track B4: sleeping through the night (design brief)

Paths and line numbers are as of e2313625. Vanilla facts were checked with javap on the 26.3 `client.jar` and in the 26.3 data pack. No code was copied. All new state is per `Player` (UUID) or per `Level`. Multiplayer (track E) is being built in parallel, so every place this track crosses the boundary between the server tick and the player is marked **[E]**.

## 1. The owner's request

> Sleeping in beds to skip the night with the vanilla rules: can sleep only at night (time 12542..23459 or thunderstorm), monsters within 8 blocks horizontal / 5 vertical prevent it ("You may not rest now; there are monsters nearby"), too far away (bed must be within 3 blocks horizontal 2 vertical), occupied, sets spawn ("Respawn point set"), players_sleeping_percentage game rule (multiplayer!), the 100-tick sleep timer, skipping to morning and clearing weather, the sleep screen fade and "Leave Bed" button, the lying pose for the MW2 body, phantoms' time_since_rest reset, beds exploding in the Nether/End (round 7 may already do that).

**Correction to the request:** 26.3 has no fixed 12542..23459 window. The night is data-driven (section 3.1). In clear weather it works out to ticks 12523 to 23477 of the day. A thunderstorm allows sleep at any time of day. Rain alone never does.

## 2. What exists today

| Area | State (file:line) |
|---|---|
| Bed use | `respawn_block::use_bed` (`crates/minecraft_survival/src/respawn_block.rs:145-200`) finds the head and handles half beds, `destroy_on_use` (`:183-195`) and `occupied` (`:196-198`). It returns `BedUse::Sleep(bed)`. |
| Rest checks | `rest_in_bed` (`respawn_block.rs:212-244`) checks reach (`reachable`, `:202-206`, the same 3/2/3 box as vanilla `isReachableBedBlock` on the head or the foot) and obstruction (`:222-226`, block above the head and above the foot suffocating). It returns the respawn point and the `can_sleep` problem text. **Nobody ever lies down.** |
| Night test | `DARK_SKY_LIGHT = 11.0` (`respawn_block.rs:41`, again at `render_anim/src/minecraft_respawn.rs:25`). It is taken from `terrain.environment.sky_light_level()`, which is sampled **at the camera** (`minecraft_respawn.rs:111`, `respawn_block.rs:1079`, `minecraft_world.rs:1704`). **[E]** This is client state and only ticks for the level that is shown (`minecraft_world.rs:2231`). |
| Bed rules | `BedRule`/`BedRuleTest` (`crates/minecraft_survival/src/dimension.rs:8-71`) read `gameplay/bed_rule` and `gameplay/straw_bed_rule` per dimension (`:128-129`), with tests at `:202-247`. |
| Bed explosions | Done in round 7: Nether and End beds explode with power 5 and fire (`minecraft_respawn.rs:92-103`, `respawn_block.rs:183-195`), and straw beds break with `block.straw_bed.break_leave` (`respawn_block.rs:16`). Tests are at `respawn_block.rs:299-380`. |
| Respawn point | `set_respawn` (`respawn_block.rs:246-250`) and "Respawn point set" in chat (`minecraft_respawn.rs:187-196`). `Player::set_respawn` is at `player.rs:148`. |
| Messages | `TOO_FAR_AWAY`, `OBSTRUCTED`, `OCCUPIED` and `SPAWN_SET` (`respawn_block.rs:17-20`) go out through `show_overlay` (`minecraft_respawn.rs:208-213`). `not_safe`, `sleep.*` and `multiplayer.stopSleeping` are never used. |
| Where bed use runs | The player-action block in the adapter (`crates/render_anim/src/minecraft_world.rs:1266-1270`). **[E]** CRATE-SPLIT section 6 moves this block to `minecraft_survival::actions`. |
| Stand-up spot | `bed_stand_up` (`respawn_block.rs:969-1001`) and `look_at_yaw` (`:1003-1020`) already follow `AbstractBedBlock.findStandUpPosition` (the respawn uses them). |
| Day clock | `Globals.day: DayCycle` (`crates/minecraft_survival/src/server.rs:41`), advanced by `advance_clocks` (`:110-120`). `/time set` goes through `Host::set_time` (`commands.rs:283-293`, which also calls `save.shift_day`). The marker maths is `minecraft_commands::commands::ticks_to_marker` (`crates/minecraft_commands/src/commands.rs:405-414`), with `wake_up_from_sleep = 0` (`args.rs:13`). |
| Weather | Lives on the level thread. The main thread has `Entities.weather` (`WeatherView`), `weather_data` and `set_weather_data` (`entities.rs:114-116`, `:288-291`), plus `Globals.weather` (`server.rs:43`). `/weather` is at `commands.rs:309-327`. |
| Game rule | `players_sleeping_percentage` (default 100, min 0) is declared at `third_party/minecraftoss/world/src/game_rules.rs:148` and read by nothing. |
| Mob mirror | `Entities.world: EntityWorld` holds the server's mobs as of the last tick. `boxes()` (`entities.rs:1084-1152`) lists every kind the fork has (bats, zombies of every kind, skeletons of every kind, creepers, spiders, slimes and magma cubes, endermen, witches, golems, wolves, villagers, cows, sheep, pigs, chickens, ghasts, blazes). |
| Villagers in beds | Villagers set and clear `occupied` (`third_party/minecraftoss/entities/src/world/villagers.rs:591`, `:869`). A player can't wake them. |
| Server tick | One server tick per `mob_ticks` (`minecraft_world.rs:1715-1727`, the `for _ in 0..mob_ticks` loop at `:1719`), counted on the local player's level only. **[E]** |
| Teleports into MW2 | Command teleports for the local UUID (`minecraft_world.rs:1835-1838`) and `teleport_facing` (`:2477`, `sim/src/frame.rs:400-415`). |
| Camera | `sync_camera_from_presented` (`render_anim/src/occupancy/view_kick.rs:239`): the skate camera, killcam, missile, the detached F5 camera (`:340-393`), then first person (`:420` on). |
| Body root | `sync_remote_bodies` (`render_anim/src/occupancy/remote_body.rs:326-457`) builds each body's root (`:416-441`). Skate (`:426-428`) and the inventory puppet (`:439-441`) already override it. |
| Screens | `MinecraftUi` (`crates/frame/src/minecraft_ui.rs:123-201`) and the chat (`frame/src/minecraft_chat.rs:38-56`, `hud/src/minecraft_chat.rs:290` `open`). An open chat captures input (`minecraft_world.rs:998`). GUI sprites and texts are allow-listed (`render_anim/src/minecraft_gui.rs:53-68`); `widget/button*` and the `sleep.` keys are not loaded. |
| Player file | `OWNED` keys (`player_nbt.rs:122-144`), `PlayerData` (`:146-158`), `Player::data` (`player.rs:200-235`) and `WorldSave::write_player` (`save.rs:121-124`). There are no `SleepTimer`, `sleeping_pos` or stats files. |
| Phantoms | Not in the fork, and there are no statistics. |

## 3. Vanilla 26.3 behaviour (javap)

### 3.1 When you can sleep
- `BedRule.Rule.test(Level)`: `always` gives true, `never` false, and `when_dark` gives `Level.isDarkOutside()`.
- `Level.isDarkOutside()` is `!dimensionType.hasFixedTime() && skyDarken >= 4`.
- `Level.updateSkyBrightness()` sets `skyDarken = (int)(15 - getDimensionValue(SKY_LIGHT_LEVEL))`, so it is dark when `sky_light_level <= 11`. The existing constant 11 is right. The value comes from the dimension (no position) and runs once per level tick.
- `sky_light_level`: the base is 15, multiplied by the `timeline/day.json` track (keyframes 133 → 1.0, 11867 → 1.0, 13670 → 0.26666668, 22330 → 0.26666668, period 24000, linear).
  - `WeatherAttributes` alpha-blends it towards 4 by 0.3125 in rain and by 0.52734375 in thunder (scaled by the rain and thunder levels).
  - In clear weather that gives dark from tick 12523 to 23477. The test pins the exact ticks against the real data.
  - At noon in full rain the level is 11.5625 (not dark). At noon in a thunderstorm it is 9.2 or lower (dark).
- `gameplay/bed_rule`: the overworld has `can_sleep: when_dark`, `can_set_spawn: always` and error `block.minecraft.bed.no_sleep`. The Nether and the End have `destroy_on_use`.
- `straw_bed_rule` in the overworld: `when_dark`, `never` sets the spawn, `destroy_on_leave: true`.

### 3.2 Using the bed (`AbstractBedBlock.useWithoutItem`, `ServerPlayer.startSleepInBed`)
1. A half bed does nothing. The foot is redirected to the head. If `destroy_on_use`, the bed is destroyed (round 7).
2. If the bed is `occupied`, `kickVillagerOutOfBed` wakes the first sleeping villager inside the head block's box and returns. Only when no villager is woken does it show `block.minecraft.bed.occupied`.
3. `startSleepInBed(bed, state, rule, headPos)`, in this order:
   - Already sleeping or dead: `OTHER_PROBLEM` (no message).
   - `bedInRange`: feet within 3 / 2 / 3 of the bottom centre of the head or the foot, else `TOO_FAR_AWAY`.
   - `bedBlocked`: head+up or foot+up not free, else `OBSTRUCTED`.
   - If `canSetSpawn`, the respawn is set (with "Respawn point set" when it moved). This happens **before** the night and monster checks.
   - If not `canSleep`, `rule.asProblem()` gives `no_sleep` (or no message when the rule has none).
   - If not creative, look for monsters: `getEntitiesOfClass(Monster, AABB(head bottom centre ± (8, 5, 8)), m -> m.isPreventingPlayerRest(level, player))`. The box test is on the entity's bounding box. A hit gives `NOT_SAFE`, "You may not rest now; there are monsters nearby".
   - `Player.startSleeping`, then `sleepCounter = 0`.
   - If `players_sleeping_percentage > 100`, the overlay `sleep.not_possible` ("No amount of rest can pass this night") is shown, but the player still sleeps.
   - `ServerLevel.updateSleepingPlayerList()`.
4. `Monster.isPreventingPlayerRest` is true. `ZombifiedPiglin` overrides it with `isAngryAt(player)`.
   - Classes that extend `Monster` (checked): Zombie and its kinds, AbstractSkeleton and its kinds, Creeper, Spider, CaveSpider, Enderman, Witch (through Raider and PatrollingMonster), Blaze, Endermite, Silverfish, Guardian, ElderGuardian, Vex, Zoglin, Breeze, Creaking, Warden, and AbstractPiglin.
   - **Not** monsters: Slime and MagmaCube (`AbstractCubeMob` extends `AgeableMob`), Ghast and Phantom (`Mob`), Shulker (golem), Hoglin and Strider (`Animal`).

### 3.3 Lying down (`LivingEntity.startSleeping`, `setPosToBed`)
- The bed head gets `occupied=true`, the pose becomes `SLEEPING`, `sleeping_pos` is set to the head, and motion is zeroed.
- The position is `(head.x + 0.5, head.y + sleepHeight + 0.125, head.z + 0.5)`.
  - `sleepHeight` is the top of the head block's shape: 9/16 for beds, so the feet are at y + 0.6875.
  - A straw bed uses its foot's base shape, 4/16, so the feet are at y + 0.375.
- `ServerPlayer.startSleeping` resets the `minecraft:time_since_rest` stat.
- The sleeping hitbox is 0.2 x 0.2 with eye height 0.2.

### 3.4 Each tick
**Level tick** (`ServerLevel.tick`, before the players and before `tickTime`):
- `pct = players_sleeping_percentage`.
- The night is skipped if `areEnoughSleeping(pct)` and `areEnoughDeepSleeping(pct, players)`:
  - if `advance_time` is on and the dimension has a default clock, `ServerClockManager.moveToTimeMarker(clock, wake_up_from_sleep)` moves forward to the next total where `total % 24000 == 0` (not moved if already there);
  - `wakeUpAllPlayers()`: the count is cleared and every sleeping player gets `stopSleepInBed(false, false)`;
  - if `advance_weather` is on and `isRaining()` (rain level above 0.2), `resetWeatherCycle()` sets `rainTime = 0`, `raining = false`, `thunderTime = 0` and `thundering = false`. `clearWeatherTime` is untouched, and the rain level then fades by itself.

**`SleepStatus`:**
- `activePlayers` counts the non-spectators in the level and `sleepingPlayers` those of them sleeping.
- `sleepersNeeded(pct) = max(1, ceil(active * pct / 100))`.
- Enough are sleeping when `sleeping >= needed`. Enough are deep asleep when the count of `isSleepingLongEnough` players (sleeping with counter >= 100) is `>= needed`.
- `update` reports a change only when someone sleeps (before or after) and a count changed.
- `ServerLevel.announceSleepStatus`:
  - It runs only if `pct <= 100`, and not in an unpublished single-player world.
  - It sends the overlay `sleep.skipping_night` ("Sleeping through this night") when enough are sleeping, else `sleep.players_sleeping` ("%s/%s players sleeping", sleeping/needed), to every player in the level.

**Player tick** (`Player.tick`):
- While sleeping, `sleepCounter = min(sleepCounter + 1, 100)`. On the server, if the bed rule at the player's block says `canSleep` is false (it got light), the player wakes with `stopSleepInBed(false, true)`.
- While not sleeping with `sleepCounter > 0`, the counter goes up by 1 and is reset to 0 at 110 (`WAKE_UP_DURATION` 10).
- `ServerPlayer.doTick` adds 1 to `time_since_rest` every tick the player is not sleeping. `Player.die` resets it, along with `time_since_death`.
- `LivingEntity.tick` wakes the player (`stopSleeping`) when the block at `sleeping_pos` is no longer a bed.
- `LivingEntity.hurtServer` wakes a sleeping entity when it is hit (after the invulnerability checks).

**Waking:**
- `Player.stopSleepInBed(wakeImmediately, updateLevelList)` sets `sleepCounter` to 0 when waking at once, else 100 (which starts the 10-tick fade).
- `stopSleeping()` (hurt, bed gone) is `stopSleepInBed(true, true)`.
- The "Leave Bed" packet (`ServerboundPlayerCommandPacket` STOP_SLEEPING) gives `stopSleepInBed(false, true)`.
- `LivingEntity.stopSleeping`:
  - the bed head gets `occupied=false`;
  - `onStopSleeping` destroys a `destroy_on_leave` bed (the straw bed's `destroyBed`);
  - stand-up: `findStandUpPosition(type, level, head, facing, yaw)`, falling back to `(head.x + 0.5, head.y + 1.1, head.z + 0.5)`;
  - the yaw is set to look at the head's bottom centre (`atan2 * 57.2957763671875 - 90`, wrapped) and the pitch to 0;
  - the pose is `STANDING`.

### 3.5 Client
- `Hud.extractSleepOverlay`: when `sleepTimer > 0`, `a = timer / 100`. If `a > 1`, then `a = 1 - (timer - 100) / 10`. It fills the whole GUI with ARGB `((int)(220 * a) << 24) | 0x101020`.
- `InBedChatScreen` is a chat screen with a `multiplayer.stopSleeping` ("Leave Bed") button.
  - The button is at `(width / 2 - 100, height - 40)`, 200 x 20. Clicking it, or Escape, sends wake-up.
  - Enter with text sends the chat line and keeps the screen.
  - The `Gui` opens it whenever the player sleeps and no screen is open, and `onPlayerWokeUp` closes it.
- `widget/button` and `widget/button_highlighted` are nine-slice sprites (width 200, height 20, border 3, from their `.mcmeta`).
- First-person camera (`Camera.setup`): when not detached and sleeping, the rotation is `(bedFacing.toYRot() - 180, 0)` (looking along the bed towards the foot), then the camera moves 0.15 up. The eye is at the feet + 0.2, so it ends at head.y + sleepHeight + 0.125 + 0.35. F5 still orbits normally.
- Body (`LivingEntityRenderer`), when `SLEEPING` with a bed orientation:
  - translate by `-(facing step) * (standingEyeHeight - 0.1)`, which is 1.52 blocks towards the foot;
  - rotate Y by `sleepDirectionToRotation(facing)` (vanilla: south 90, west 0, north 270, east 180; confirm the switch map with javap `LivingEntityRenderer$1`);
  - rotate Z by `getFlipDegrees()` = 90, then rotate Y by 270.
- Language strings (26.3 `en_us.json`): `block.minecraft.bed.not_safe`, `sleep.not_possible`, `sleep.players_sleeping`, `sleep.skipping_night`, `multiplayer.stopSleeping`.

### 3.6 Saving
- `Player` writes `SleepTimer` (short).
- `LivingEntity` writes `sleeping_pos` (int array) while sleeping.
- Stats go in `players/stats/<uuid>.json` (`LevelResource.PLAYER_STATS_DIR`) as `{"stats": {"minecraft:custom": {"minecraft:time_since_rest": N, ...}}, "DataVersion": V}`.
- `PhantomSpawner` reads `time_since_rest`, clamped to `[1, i32::MAX]`. Phantoms are possible once a random draw against it reaches 72000 (3 days). Phantoms are out of scope here.

## 4. Plan: commit groups, in order

Each group is one commit. New files must pass `rustfmt --check`. Existing files get minimal edits in their surrounding style. No comments.

### Group 1: "Minecraft beds check the vanilla sleep rules: the night by sky light, monsters nearby and the sleep percentage"
**Files**
- New `crates/minecraft_survival/src/sleep.rs` (declared in `lib.rs`).
- `respawn_block.rs`.
- `crates/minecraft_terrain/src/environment.rs` (one new method).

**Changes**
- `sleep::dark_outside(sky_light: f32, fixed_time: bool) -> bool`: `!fixed_time && (15.0 - sky_light) as i32 >= 4`.
- `DimensionEnvironment::level_sky_light(&self, clock_ticks: i64, rain: f32, thunder: f32) -> f32` samples `gameplay/sky_light_level` with no biome (as `dimension_value` does, `environment.rs:225-229`).
- `Level::dark_outside(&self, globals: &Globals) -> bool` takes the level's default clock (`Globals::default_clock_ticks`), the level's `entities.weather` rain and thunder, and the Nether's fixed time (`default_clock(...) == None`).
  - It replaces the camera sample in `rest_in_bed` callers.
  - The existing respawn uses (`respawn_block.rs:1079`) and `bright_outside` (`minecraft_world.rs:1704`) may switch too. Do that only if it is a one-line change, and note it in the track log.
- `sleep::SleepStatus { active, sleeping }` with `update(players: impl Iterator<Item = (spectator, sleeping)>) -> bool`, `sleepers_needed(pct)`, `enough_sleeping(pct)` and `enough_deep_sleeping(pct, deep_count)`, exactly as section 3.4.
- `sleep::monster_box(head: Pos) -> [f64; 6]` is `[x + 0.5 - 8, y - 5, z + 0.5 - 8, x + 0.5 + 8, y + 5, z + 0.5 + 8]`.
- `sleep::prevents_rest(kind: &str, angry_at_sleeper: bool) -> bool` follows the class list in 3.2. Zombified piglins count only when angry at the sleeper. Kinds the fork doesn't have yet are listed too, so B1-B3 mobs work when they land.
- `Entities::monster_near(&self, area: [f64; 6], sleeper: u64) -> bool` intersects mob boxes from the mirror (as in `boxes()`) with `prevents_rest`. For a zombified piglin, "angry" means its monster target is that player.
- `rest_in_bed` gains `creative: bool` and `monsters: impl FnOnce([f64; 6]) -> bool`. Its result becomes `Rest { respawn, outcome }`, where `outcome` is `Sleep` or `Problem(Option<String>)`. The order is exactly 3.2 step 3, and `NOT_SAFE = "block.minecraft.bed.not_safe"`.
- `use_bed`'s `Occupied` carries the head so the caller can try the villager kick-out (group 2).

**Unit tests** (`sleep.rs`, `respawn_block.rs`)
- `the_sky_is_dark_at_light_eleven_or_less`: 11.0 dark, 11.01 not, 15 not, 4 dark, any value with `fixed_time` not.
- `the_overworld_night_runs_from_tick_12523_to_23477_in_clear_weather`: with real data, gated on the registries as the existing tests are; pin the first and last dark tick.
- `a_thunderstorm_darkens_noon_but_rain_alone_does_not`.
- `sleepers_needed_rounds_up_and_needs_at_least_one`:
  - (0 players, 100) → 1;
  - (1, 100) → 1;
  - (4, 50) → 2;
  - (3, 50) → 2;
  - (4, 0) → 1;
  - (2, 101) → 3, never enough.
- `the_sleep_count_changes_only_when_someone_sleeps`, which ignores spectators.
- `monsters_count_in_an_eight_by_five_box_around_the_head`: zombie boxes at dx 8.2 (edge overlap), 8.4 (out), dy 5 and dy 6; a slime, a ghast and an unangry zombified piglin don't count; an angry one does.
- `the_rest_checks_run_in_vanilla_order`:
  - too far beats obstructed;
  - the spawn is set even when it is day or monsters are near;
  - creative ignores monsters;
  - by day you get `no_sleep`.

### Group 2: "Minecraft players lie down in beds and the night passes when enough of them sleep"
**Files**
- `sleep.rs`, `player.rs` (fields), `level.rs` (`Level.sleep_status`), `server.rs` (`Server::tick_sleep`, `Globals::set_clock`), `commands.rs` (`Host::set_time` calls `Globals::set_clock`), `entities.rs` (`reset_weather_cycle`, `wake_villager_in`).
- `crates/minecraft_terrain/src/server.rs` (one `Command::WakeVillagerIn(BlockPos)`).
- `render_anim/src/minecraft_respawn.rs` and `minecraft_world.rs` (calls only).

**State**
- `Player.sleeping: Option<Sleeping { head: Pos, facing: (i32, i32), straw: bool }>`, `Player.sleep_timer: i32` and `Player.time_since_rest: i32`.
- `Level.sleep_status: SleepStatus`.
- No globals and no player-0 state.

**Starting to sleep**
- `sleep::start(globals, level, player, bed) -> SleepEvents`:
  - a player riding a strider (b3's `Player.riding`) is dismounted first with `Entities::dismount`, as vanilla's `startSleeping` calls `stopRiding`;
  - write `occupied=true` on the head through `level::write_blocks`;
  - set `player.place` to the bed feet (3.3), with zero motion;
  - set `sleep_timer = 0` and `time_since_rest = 0`;
  - emit `sleep.not_possible` when pct > 100;
  - update the level status and announce.
- `SleepEvents` carries `moves: Vec<(PlayerUuid, Place)>`, `messages: Vec<(PlayerUuid, Clientbound)>` (overlay `System` lines) and `writes` (block writes the adapter must push through `render_anim`'s `write_blocks` so MW2 collision follows, as `respawn_use.writes` does).

**Waking**
- `sleep::stop(globals, level, player, wake_at_once, update_list) -> SleepEvents`:
  - write `occupied=false` if the head is still a bed;
  - a straw bed with `destroy_on_leave` loses both halves with `STRAW_BED_BREAK_SOUND`;
  - stand up with `bed_stand_up(head, facing, yaw)`, or fall back to `head + (0.5, 1.1, 0.5)`, facing the bed via `look_at_yaw`, pitch 0;
  - `sleep_timer` becomes 0 or 100.

**Occupied beds**
- If a sleeping villager is in the head block's box, `Entities::wake_villager_in(head)` wakes it (a level-thread command) and nothing is shown.
- Otherwise the player sees `OCCUPIED`.

**The tick**
- `Server::tick_sleep(dimension)` runs once per server tick for the level, from the adapter's `for _ in 0..mob_ticks` loop (`minecraft_world.rs:1719-1727`). **[E]** Track E's server tick calls it for every occupied level.
- Level part first, as in 3.4:
  - the skip uses `ticks_to_marker(total, 0)` through `Globals::set_clock(OVERWORLD_CLOCK, total)`, which keeps `save.shift_day` in step as `Host::set_time` does;
  - wake every sleeper with `stop(false, false)`;
  - if `advance_weather` and `weather.rain_level > 0.2`, `Entities::reset_weather_cycle()` sends `set_weather_data` with rain and thunder times 0 and both flags false, keeping `clear_weather_time`, and mirrors it into `Globals.weather`.
- Then each player in the level:
  - `time_since_rest += 1` when not sleeping;
  - the timer rules of 3.4;
  - wake on a gone bed `(true, true)`, on lost darkness `(false, true)`, on a landed hurt `(true, true)` (a `Survival` hurt tick: use the combat tracker's last damage tick, `death.rs:269-316`) and on death (`time_since_rest = 0`).

**Adapter**
- `minecraft_respawn::use_block` calls `sleep::start` on `Outcome::Sleep`. The adapter applies `moves` for the local UUID with `authority.teleport_facing` (as the command teleports at `minecraft_world.rs:1835`) and shows the local player's `messages`. **[E]** Remote players get theirs over the network later.
- Announcements: `Server.published: bool`, default false. Track E sets it when friends can join. While false, `announce` sends nothing (vanilla's unpublished single-player rule).

**Unit tests** (`sleep.rs`, using the `Blocks` test world from `respawn_block.rs` and test players)
- `lying_down_marks_the_bed_occupied_and_puts_the_feet_on_it`: red bed 0.6875, straw bed 0.375.
- `the_sleep_timer_counts_to_100_and_the_wake_fade_to_110`.
- `the_night_skips_to_the_next_morning_once_everyone_has_slept_100_ticks`:
  - day total 13000 + 24000 * 2 → 72000;
  - the timers become 100 and the beds free;
  - no skip at 99 ticks;
  - no clock move with `advance_time` off, but the players still wake.
- `a_skip_clears_rain_and_thunder_only_when_it_rains`.
- `half_the_players_are_enough_at_fifty_percent_and_none_at_101`.
- `leaving_the_bed_stands_up_beside_it_facing_it`, and `a_straw_bed_breaks_when_left`.
- `a_broken_bed_or_daylight_or_a_hit_wakes_the_sleeper`.
- `time_since_rest_counts_while_awake_and_resets_on_sleep_and_death`.
- `an_occupied_bed_with_a_sleeping_villager_wakes_the_villager_instead_of_complaining`.

### Group 3: "The Minecraft sleep screen fades to dark with a Leave Bed button"
**Files**
- `crates/frame/src/minecraft_ui.rs` (`McSleep { timer: i32 }` in `MinecraftUi.sleep: Option<McSleep>`, and `leave_bed: bool`), `frame/src/minecraft_chat.rs` (`in_bed: bool`).
- New `crates/hud/src/minecraft_sleep.rs` (the overlay and button), `hud/src/minecraft_chat.rs` (Escape and the button while `in_bed`), `hud/src/lib.rs` or `plugin.rs` (registration).
- `render_anim/src/minecraft_gui.rs` (`LOOSE_SPRITES` gets `widget/button` and `widget/button_highlighted`; `TEXT_PREFIXES` gets `multiplayer.stopSleeping` and `sleep.`).
- `minecraft_world.rs` (it sets `ui.sleep` from `player.sleep_timer`, opens the chat with `in_bed` when the local player starts sleeping, closes it on wake, and turns `ui.leave_bed` into `sleep::stop(false, true)`).

**Behaviour**
- Overlay alpha as in 3.5, drawn over the world and HUD but under the chat.
- Button: nine-slice 200x20 with border 3, highlighted on hover, label centred white, placed in GUI pixels at `gui_scale`.
- Enter with text sends the line and keeps the bed screen. **[E]** `leave_bed` becomes a `minecraft_net` action message later.

**Unit tests** (`hud/src/minecraft_sleep.rs`)
- `the_sleep_overlay_darkens_over_100_ticks_and_fades_in_10`: 0 → none, 50 → alpha 110, 100 → 220, 105 → 110, 110 → none, colour 0x101020.
- `the_leave_bed_button_sits_40_pixels_above_the_bottom`.
- `escape_in_bed_asks_to_leave`.
- `a_nine_slice_keeps_three_pixel_borders`.

### Group 4: "Sleeping players lie on the bed, seen from the pillow or from outside"
**Files**
- `crates/frame/src/minecraft_ui.rs` or a new `frame/src/minecraft_sleep.rs` (resource `McSleepers { bodies: Vec<(u32, Mat4)>, camera: Option<WorldCameraPose> }`).
- `render_anim/src/occupancy/remote_body.rs` (one override after the puppet at `:439-441`).
- `render_anim/src/occupancy/view_kick.rs` (a first-person branch before `:420`, when not detached).
- New `render_anim/src/minecraft_sleep.rs` (pure pose maths).
- `minecraft_world.rs` (fills the resource for the local player; **[E]** for every player that track E reports asleep).

**Behaviour**
- `sleep_camera(head, facing, straw)`: the eye from 3.5, converted with `sim::voxel::to_map`, yaw `facing.toYRot() - 180` in Minecraft degrees turned into MW2 yaw as `minecraft_yaw` does (`minecraft_world.rs:2477`), pitch 0.
- `lying_root(head, facing, straw)`: the vanilla transform from 3.5 in MW2 axes (Z up), applied to the standing MW2 body.
- No MW2 movement, look, jump or fire while in bed. Hold the MW2 player with b3 group 7a's `SimWorld::ride(id, Some(feet))` (`MC_RIDING`: no move, jump or sprint, zero velocity, no gravity), set at start and cleared with `None` at wake, so the hull no longer settles 0.125 lower. Do not add a second pm_flags bit. Also check that the chat capture stops firing (`minecraft_world.rs:998`); if not, add the sleeping gate where the inventory gate is.
- F5 still orbits around the bed eye.

**Unit tests** (`render_anim/src/minecraft_sleep.rs`)
- `the_sleep_camera_looks_down_the_bed_from_the_pillow` for all four facings.
- `the_lying_body_runs_from_the_pillow_towards_the_foot`: the transformed body top is near the head block, its feet are about 1.8 blocks towards the foot, and it lies level at the sleep height.
- `a_straw_bed_lies_lower`.

### Group 5: "Minecraft players keep their sleep timer, bed and time since rest in the world save"
**Files**
- `player_nbt.rs` (`OWNED` gains `SleepTimer` and `sleeping_pos`; `PlayerData` gains `sleep_timer`, `sleeping_pos` and `time_since_rest`).
- `player.rs` (`data()` and the join).
- `save.rs` (`WorldSave::read_stats` and `write_stats` for `players/stats/<uuid>.json`, merging: only `minecraft:custom/minecraft:time_since_rest` is ours; other stats and `DataVersion` from an imported vanilla world are kept; a new file gets the fork's data version).
- `server.rs` (`save_player` and `autosave` also write stats).

**Join:** a player saved asleep stands up at once at the bed, as `stop(true, true)` does, and the bed's `occupied` is cleared. This is a decision; see 6.

**Unit tests**
- `sleep_timer_and_sleeping_pos_round_trip`.
- `the_stats_file_keeps_unknown_stats_and_updates_time_since_rest`.
- `a_player_saved_in_bed_wakes_beside_it_on_join`.

### Group 6: "The Minecraft state doc describes sleeping"
- `docs/MINECRAFT-SURVIVAL.md`:
  - replace `:167` ("sleeping itself is not done");
  - drop the sleep sentence at `:280`;
  - add a sleeping bullet (rules, skip, screen, pose and save);
  - add limitations: no phantoms (time since rest is only counted), no sleeping animation on the MW2 body (it lies in its idle pose), the friends' sleep messages wait for E, and no `sleep_in_bed` statistic or advancement.
- Add the track-log line.

## 5. Owner's test (release build)

Kit (8 stacks):
```
IW4L_MINECRAFT_KIT=minecraft:red_bed,minecraft:blue_bed,minecraft:straw_bed,minecraft:zombie_spawn_egg*8,minecraft:slime_spawn_egg*4,minecraft:cobblestone*64,minecraft:diamond_sword,minecraft:clock
```

Steps (screenshot each numbered step):
1. **By day:** place the red bed on open ground and right-click it. You should see "You can sleep only at night or during thunderstorms" above the hotbar, and the chat line "Respawn point set" the first time.
2. **At dusk:** `/time set 12400`, wait about 6 seconds (to tick 12523), and right-click. You lie down.
   - The view sits on the pillow looking down the bed.
   - The screen darkens over 5 seconds.
   - A "Leave Bed" button appears at the bottom, with the chat bar open.
3. **The skip:** stay in bed. After about 5 seconds the sky jumps to sunrise and the screen fades back in about half a second. You stand beside the bed facing it, and `/time query daytime` gives a few ticks past 0.
4. **Leave Bed:** `/time set night`, lie down, and click "Leave Bed" before the screen is fully dark. You stand up at once and it is still night. Lie down again and press Escape: same result.
5. **Monsters:** at night, place a zombie egg about 6 blocks from the bed, behind a cobblestone wall. Right-click the bed: "You may not rest now; there are monsters nearby". Kill the zombie, place 2 slimes next to the bed, and right-click: you sleep (slimes don't count).
6. **Creative:** `/gamemode creative`, place a zombie beside the bed, and sleep. It works. Go back with `/gamemode survival`.
7. **Distance and obstruction:** stand 4 blocks from the bed (still in reach) and right-click: "You may not rest now; the bed is too far away". Put cobblestone on the pillow block's top neighbour: "This bed is obstructed".
8. **Weather:**
   - `/time set noon` and `/weather rain`, then right-click: no sleep.
   - `/weather thunder`, wait about 5 seconds for the storm to build, then right-click: you sleep. The night skip lands at the next morning and the storm clears (the rain fades out over a few seconds).
9. **Percentage:** `/gamerule players_sleeping_percentage 101` at night, then sleep: "No amount of rest can pass this night", and nothing is skipped. Leave the bed and set it back with `/gamerule players_sleeping_percentage 100`.
10. **Hurt wakes:** at night, sleep, then type `/effect give @s minecraft:poison 5` in the bed chat. You wake at the first poison hit.
11. **Third person:** press F5 while in bed. The MW2 body lies flat on the bed, head on the pillow.
12. **Straw bed:** sleep in the straw bed at night and leave the bed. The straw bed breaks. It never says "Respawn point set".
13. **Saving:** sleep, then quit while lying down. Reload: you stand beside the bed, and the bed is free (a second right-click at night works).
14. **Regression:** in the Nether a bed still explodes (round 7).

The multiplayer messages ("1/2 players sleeping", "Sleeping through this night") can only be tested after track E.

## 6. Risks, shared files and decisions

**Risks**
- **Camera and the MW2 hull.** The MW2 player stands on the bed top (0.5625) while vanilla's feet are at 0.6875. With b3's `MC_RIDING` pin (group 4) the hull stays where it was put and gravity doesn't move it. Only the camera and body use the vanilla heights. The pin is set at start and cleared at wake.
- **Input capture.** If the chat capture does not stop MW2 movement and firing, sleeping players will slide or shoot. Group 4 must check this in game code, not only in a unit test.
- **The level thread lags one tick.** The monster check reads the mirror as of the last server tick, and the weather reset goes through `set_weather_data`. Both are one tick late at most, which is acceptable.
- **Clock and save.** Moving `Globals.day` must also call `save.shift_day`, or `game_time` jumps (see `save.rs:132-137` and its test at `:679-690`).
- **Darkness source.** The sleep test must use the new level-wide sky light, not the camera sample. Otherwise a friend sleeping in the overworld while the host is elsewhere reads the wrong level. **[E]**

**Files shared with other round 8 tracks**
- **E (multiplayer):**
  - `minecraft_world.rs` (the action block at `:1266` and the tick loop at `:1714`), `server.rs`, `player.rs`, `player_nbt.rs` and `minecraft_terrain/src/server.rs`.
  - E must carry the sleep state (sleeping head and timer) to clients and a `LeaveBed` action to the server, and call `Server::tick_sleep` for every occupied level.
  - E also sets `Server.published`.
- **B (mobs):** `entities.rs` (the mirror lists and `monster_near`), `minecraft_terrain/src/server.rs` (`Command`) and the vendored villager code. B4 lands last in track B (after b3, b2 and b1), so the new monster kinds (piglins, brutes, zoglins) are in the mirror when `sleep::prevents_rest` and `monster_near` are written; hoglins and striders must not count. b3 group 7a's `MC_RIDING` pin is reused in group 4, and a strider rider is dismounted before lying down.
- **C (End and visual leftovers):** `remote_body.rs` `sync_remote_bodies` (an elytra glide pose would override the same root) and `view_kick.rs`. Keep the sleeping override as a separate small block after the puppet override.
- **D (skating):** `remote_body.rs` (skate root, `:426-428`), `view_kick.rs` (skate camera) and `frame`. Skating must not start while sleeping; check `player.sleeping` where skating starts.
- **A (fixes):** `minecraft_respawn.rs`, `minecraft_gui.rs` (the allow-lists) and `docs/MINECRAFT-SURVIVAL.md`.

**Decisions** (the vanilla-like option)
1. **The night** is 26.3's data-driven rule (sky light 11 or less, per level, with weather), not the older fixed window. In clear weather that is 12523 to 23477.
2. **Monsters** are the classes that extend `Monster`, as in 3.2. Slimes, magma cubes, ghasts, phantoms, shulkers, hoglins and striders don't block sleep. Zombified piglins block it only when angry at the sleeper (their target).
3. **The sleep messages** go only to a world shared with friends. Solo play shows none, as in a vanilla single-player world. `sleep.not_possible` always shows.
4. **Phantoms** are not in the fork. `time_since_rest` is counted, reset on sleep and death, and saved in the vanilla stats file, so a later phantom spawner can use it.
5. **A player saved asleep** stands up beside the bed on join (vanilla keeps them in bed). This avoids a join-in-bed path through the MW2 spawn and the spawn gate.
6. **The MW2 body** lies in its standing idle pose, rotated as vanilla rotates the model. There is no sleeping animation.
7. **Villagers** in the bed are woken like vanilla's `kickVillagerOutOfBed` (included in group 2).
8. **Not in scope:** the `sleep_in_bed` statistic, the `slept_in_bed` advancement trigger and the `adventure/sleep_in_bed` advancement (the fork has no statistics UI or advancements).
