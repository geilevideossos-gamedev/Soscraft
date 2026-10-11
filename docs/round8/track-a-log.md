# Track A log: the owner's test failures of build e231362

Branch `track-a`, worktree `C:\Dev\mc-wt\track-a`, target dir `D:\iw4l-wt\track-a`. Briefs: `a5-tick-overload.md`, `a2-eye-of-ender.md`, `a3-mining-speed.md`, `a4-spawner-render.md`, `a1-end-gateway.md`.

A commit can't name its own hash, so each entry is written with its commit and its hash is filled in by the next commit; the last entry's hash is in `git log` under its subject.

## Item a5: surviving overload, and `/tick`

### 1. The Minecraft level thread answers each tick before later commands

Commit: d16d0215

- **Changed:** `minecraft_terrain/src/server.rs`.
  - `server_loop` still handles every command queued before a `Tick` in the same `Output`, but the inner loop now ends right after the tick and the output goes out then. Commands sent after the tick go into the next output, so a stream of commands can no longer hold an answer back.
  - `Output` gains `tick_nanos`: one entry per tick it covers, with that tick's time on the level thread.
  - `ServerHandle` counts the ticks it sent and the ticks answered in the outputs `poll` returned, and `ticks_in_flight()` gives the difference.
- **Tests** (`minecraft_terrain`, they need the game data like their neighbours): `each_tick_gets_its_own_output` (three ticks in a row give three outputs of one tick each, nothing in flight after the poll) and `commands_after_a_tick_do_not_hold_its_output` (one tick then 200 orb summons: the first output holds only the tick, the orbs come in later outputs).
- **Decision:** the handle counts ticks, not batches (the brief's `batches_sent`/`batches_answered`). It is the same while each tick is a batch of one, and it stays right in group 2b if a second `Tick` ever joins a batch that is already running, where counting outputs would leave one batch in flight for ever.
- **Test steps:** none on their own; this commit changes no behaviour in game. Steps 1-5 of the a5 owner test (brief section 5) cover it together with groups 2a and 2b.

### 2a. Minecraft has vanilla's server tick schedule, with the overload warning and skip

Commit: c87e6fbf

- **Changed:** new `minecraft_survival/src/tick_rate.rs`, registered in `lib.rs`. Pure logic, not wired into the game yet (group 2b does that).
  - `TickRate` holds vanilla's `ServerTickRateManager` state (rate, nanoseconds per tick, frozen, steps, sprint), `runServer`'s timing (`next_tick`, `last_overload_warning`) and the 100-entry tick-time ring.
  - `plan(now, ready)` runs `runServer`'s loop for every tick due at `now`: the sprint check, or the overload check (more than 1 s plus 20 ticks behind, and at least 10 s plus 100 ticks since the last warning), which logs `Can't keep up! Is the server overloaded? Running {ms}ms or {ticks} ticks behind` at WARN and skips those ticks. Then it moves `next_tick` on, runs `TickRateManager.tick` and records whether the tick runs normally. The first call starts the schedule at `now`, and `ready = false` plans nothing, so the time behind keeps growing while a level is still busy.
  - A batch holds at most `max_batch()` ticks: as many as fit in 50 ms at the mean tick time of the last recorded batch (1 to 100).
  - `record(nanos)` fills the ring like `tickServer` and adds to the sprint time while sprinting; `average_nanos()` is vanilla's `getAverageTickTimeNanos`.
  - `TickBatch { normal, overload }` with `len`, `is_empty` and `normal_count`.
- **Tests** (`tick_rate.rs`): `calls_50_ms_apart_give_one_tick_each`, `a_one_second_backlog_is_caught_up_in_batches_without_a_warning` (5, 5, 5, 5 at 10 ms a tick), `the_batch_limit_follows_the_last_batch_not_the_average`, `three_seconds_behind_warns_once_and_skips` (warns `(2950, 59)`, and a second 3 s gap right after catches up 60 ticks without a warning), `a_level_still_busy_plans_nothing_and_falls_behind`, `the_first_plan_starts_the_schedule`, `the_average_covers_the_last_100_ticks`, `a_batch_counts_its_normal_ticks`.
- **Decision:** the ring and the average count the ticks recorded, not the ticks planned, so ticks still in flight don't lower the average. The sprint and step fields are there already because `plan` follows vanilla's sprint branch; the commands that set them come with `/tick` (group 5b).
- **Test steps:** none on their own; nothing calls it yet.

### 2b. The Minecraft server ticks on a vanilla schedule and skips ticks when it falls behind

Commit: 38e78baf

- **Changed:**
  - `minecraft_survival/src/server.rs`: `Globals.tick_rate` (built in `test_globals` and in `render_anim`). `Server::plan_ticks(now, ticking)` reads every ticking level (`Entities::ready_for_ticks`, `take_finished_ticks`), records the slowest level's time per tick, and plans with `TickRate::plan`, ready only when every ticking level has answered its last batch and no planned batch is still waiting. The planned batch waits in `Server.pending_batch` until `Entities::tick` takes it, so a frame that returns early before the mobs (arriving, no player state, no light) keeps it for the next frame. `take_batch` hands it out.
  - `minecraft_survival/src/entities.rs`: `Entities::tick` takes the batch and no longer decides the server ticks itself. Its client tick still runs on its own 50 ms clock. When the batch has ticks it sends one `Tick` with the batch's flags. `ready_for_ticks` polls the level thread (as `poll_jobs` does) and says whether no tick is in flight; every poll gathers the outputs' `tick_nanos` for `take_finished_ticks`.
  - `minecraft_terrain/src/server.rs`: `Command::Tick` carries the batch's flags and `ServerHandle::tick(input, normal)` sends nothing for an empty batch. On the level thread the tick body moved into `run_tick`, which runs once per tick of the batch; before the second and later ticks it handles every command already queued, so a `/gamerule` sent during a slow batch lands between two ticks like vanilla's `runAllTasks`. The player's inventory copy carries the stacks it picked up across the batch, and the player's hurts land on the first tick only. The tracked mobs, the weather and the entity snapshot are built once by `finish_batch` after the last tick; the event lists keep growing over the batch. The entity autosave still counts each tick. The flags are not used yet (group 4).
  - `render_anim/src/minecraft_world.rs`: `plan_ticks` runs once per frame before the autosave check, and the mobs' `entities.tick` gets `pending_batch`.
  - `minecraft_survival/src/the_end/dragon_fight.rs`: the fight test sends a tick only when the level has answered the last one.
- **Tests:** `minecraft_terrain`: `a_batch_runs_each_tick` (a batch of 3 moves `game_time` by 3 and answers once with three tick times) and `a_rule_sent_after_a_batch_applies_to_the_next_batch` (`random_tick_speed 0` sent behind a batch of 5 is in force before the next batch). `minecraft_survival`: `a_batch_left_by_an_early_return_is_sent_on_the_next_frame` (a batch of 3 waits whole while nothing new is planned, and is handed out once). `each_tick_gets_its_own_output` and `commands_after_a_tick_do_not_hold_its_output` send one-tick batches now.
- **Decisions:**
  - `ticking` is the local player's dimension, and nothing while the player is switching dimensions. Then no level holds the batch: it is planned (so the schedule doesn't fall behind) and dropped, as no level ticked during a switch before either, and a planned batch left over from before the switch is dropped too. Without this, every dimension switch that takes more than 2 s would end in a `Can't keep up!` line. Other early returns (no player state, no light) keep the batch, as the brief says.
  - If opening the world keeps the frame from reaching the mobs for more than 2 s after the player exists, the first batch can end in one `Can't keep up!` line, as a vanilla server often logs one while it starts.
  - The level is polled before its tick times are taken, so a batch answered between two frames counts on the next frame and isn't seen one frame late.
  - `tick_phases`' total is now the last tick's time on the level thread, without the snapshot that follows the batch.
- **Test steps** (brief section 5, steps 1-5; the rest of the owner test comes with groups 3-5):
  1. Start in survival on grass in daylight. Place 4 saplings, spawn 2 cows, and drop a few cobblestone (Q). The items fly, land and can be picked up again.
  2. Type `/gamerule random_tick_speed 20000`. The Minecraft world slows down: mobs and dropped items move in jerky steps, and saplings grow almost at once. MW2 movement, aiming and the camera stay smooth. **Screenshot 1.**
  3. Type `/gamerule random_tick_speed 3`. Mobs and items may race for a moment first: like vanilla, the server runs missed ticks back to back, at most about 15 s of game time since its last warning, and if it is still more than 2 s behind it skips the rest. Within a second or two everything moves smoothly again. Drop an item: it falls and can be picked up. **Screenshot 2.**
  4. Type `/gamerule random_tick_speed 20000` again and leave it on for 30 s, then quit through the menu. The game closes within a few seconds instead of hanging.
  5. `iw4l-artifacts\logs\latest.log` has at least one line `Can't keep up! Is the server overloaded? Running ...ms or ... ticks behind`. Send that line.
- **Kit** (3 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:oak_sapling*16,minecraft:cow_spawn_egg*4
```

### 3. Minecraft clocks, weather, autosaves, containers and the dragon fight follow the server's ticks

Commit: 8913015c

- **Changed:**
  - `minecraft_survival/src/server.rs`: `Server::plan_ticks` applies each planned batch to the server-wide state itself: the Overworld and End clocks move by the batch's normal ticks (`Globals::advance_clocks` takes ticks now, and the paused-day branch shifts the game time by the same ticks), the weather of a server with no weather level ticking moves by the normal ticks, `level_ticks` (the empty-level timer) by every tick, and the autosave countdown by every tick. Each ticking level that holds the batch gets its normal ticks added to its new `Level.container_ticks`. `Globals.autosave_ticks` is replaced by `autosave_due`.
  - `minecraft_survival/src/tick_rate.rs`: vanilla's autosave countdown (`ticksUntilAutosave`, starting at 6000) with `autosave_interval` (`max(100, (int)(rate * 300))`, with `1e9 / (average + 1)` as the rate while sprinting), `count_autosave(ticks)` (counts a batch down and restarts at the interval when it reaches 0), `set_rate` (vanilla `setTickRate`: at least 1, nanoseconds per tick as a double division, then `onTickRateChanged`, which only ever shortens the countdown), `set_frozen` and `is_frozen`. A finished sprint calls `onTickRateChanged` too, as vanilla's `finishTickSprint` does.
  - `minecraft_terrain/src/day_cycle.rs`: `DayCycle::advance_ticks`.
  - `minecraft_survival/src/save.rs`: `AUTOSAVE_TICKS` is gone; the interval comes from the tick rate.
  - `render_anim/src/minecraft_world.rs`: the autosave runs when `autosave_due` is set; the two frame-time clock advances and the `autosave_ticks`, `level_ticks` and weather lines after the mobs' tick are gone; the containers tick by the level's `container_ticks` instead of the hand's 20 Hz clock; the dragon fight gets the frame's planned normal ticks as seconds (`count * dragon_fight::TICK_SECONDS`).
- **Tests:** `tick_rate.rs`: `the_autosave_interval_is_five_minutes_at_the_tick_rate` (6000 at 20, 300 at 1, 3000 at 10, rates below 1 clamp to 1), `the_autosave_interval_never_drops_below_100_ticks` (a sprint averaging 10 s a tick), `lowering_the_rate_shortens_the_autosave_countdown_and_raising_it_does_not`, `the_autosave_counts_down_by_whole_batches`. `server.rs`: `frozen_ticks_move_no_clock_and_no_weather_but_count_for_the_autosave_and_empty_levels` (a frozen batch of 10 leaves the clocks and the rain timer alone, adds 10 to `level_ticks` and the autosave countdown, and 6000 frozen ticks set `autosave_due`); `a_batch_left_by_an_early_return_is_sent_on_the_next_frame` also checks that the holding level's `container_ticks` grows by the batch once. `day_cycle.rs`: `day_advances_by_server_ticks_until_paused`. The clock test `the_end_clock_runs_beside_the_day_and_each_level_reads_its_default_clock` now advances by 20 ticks instead of 1 s.
- **Decisions:**
  - The autosave countdown lives in `TickRate`, not in `Globals` as the brief has it, so that `set_rate` (here, and used by `/tick rate` in group 5b) and a finished sprint can shorten it the way `onTickRateChanged` does without reaching into `Globals`. `Globals` only keeps the `autosave_due` flag for `render_anim`. `set_rate` and `set_frozen` are added now because the countdown and the frozen-tick test need them; `/tick` (group 5b) only calls them.
  - The countdown restarts at the interval when it reaches 0, as vanilla's `autoSave` does; a batch that overshoots 0 does not carry the rest over.
  - Containers keep their ticks in `Level.container_ticks` until the frame reaches them, so a frame that returns early (arriving, menus) neither loses nor repeats them. They only count while the level ticks, as the level's block entities do.
  - The dragon fight takes the frame's planned normal ticks whether or not the End is the ticking level, as it took the frame time before: vanilla ticks the fight with the End level, and every vanilla level ticks. Frames that return before the fight lose those ticks, as they lost their frame time before.
  - Clocks, weather and the timers move during a dimension switch too (no level holds the batch then), as the server keeps ticking while a vanilla player is in the loading screen. Before, the clocks moved by frame time while arriving and the weather and timers stood still.
- **Test steps:**
  1. Start in survival in daylight. Type `/time query daytime`, wait 10 s on a watch and type it again: it has grown by about 200.
  2. Place the furnace, put coal in the fuel slot and 16 raw iron on top. Each iron takes 10 s, as before.
  3. Type `/gamerule random_tick_speed 20000`. The furnace arrow, the sun and `/time query daytime` slow down together with the mobs; MW2 movement stays smooth. **Screenshot 1** of the furnace with the arrow part-way.
  4. Type `/gamerule random_tick_speed 3`. Within a few seconds the furnace and the clock run at normal speed again.
  5. Go through a Nether portal and back. The sky in the Overworld is at the time the clock would have reached; `/time query daytime` keeps growing while you are in the Nether.
  6. Stay in the world for more than 5 minutes, then quit. `iw4l-artifacts\logs\latest.log` has a line `Minecraft autosave: ... chunks queued for ...` about every 5 minutes. Send the lines.
- **Kit** (4 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:furnace,minecraft:coal*16,minecraft:raw_iron*16,minecraft:obsidian*14
```

### 4. A frozen Minecraft tick only moves the players

Commit: 93d48f00

- **Changed:**
  - `minecraft_terrain/src/server.rs`: `run_tick` takes the tick's flag. A frozen tick keeps the player's part (clocks, players, the simulation area, `mobs.tick_player` with the hurts, views, vitals and main hand, `check_merchant`, item, trident and orb pickup, the entity autosave count and the snapshot after the batch) and skips the level's: `prepare_spawning` and `sim.tick()` (block, fluid and random ticks, block events, level entities, block entities, spawners), `take_spawned`, `despawn_mobs`, `tick_mobs`, `tick_clouds`, `tick_thrown`, `tick_tridents`, `tick_eyes`, `tick_end` and `spawn_trade_experience`. Commands still run between ticks, so hits on frozen mobs land.
  - `minecraft_survival/src/tick_rate.rs`: `ClientTiming { tick_seconds, runs_normally }` and `TickRate::client_timing()`: vanilla's `getTickTargetMillis`, `max(50 ms, ms per tick)` while the last planned tick ran normally, and 50 ms with `runs_normally` false otherwise. It reads the flag of the last planned tick, which stays until the next batch.
  - `minecraft_survival/src/entities.rs`: `Entities::tick` takes the client timing. The client clock runs at its tick length, and while the level is frozen the clock stops (the partial tick holds, so frozen mobs don't jitter) and the client tick (falling blocks, TNT smoke, lids, client mobs, portal and spell particles, eyes, blaze sounds, orb pickups) doesn't run. `client_ticks()` stops with it, so rain, display ticks, campfire smoke and `tick_scene` pause like vanilla's frozen client. The render's partial tick divides by the client tick length.
  - `render_anim/src/minecraft_world.rs`: passes `globals.tick_rate.client_timing()`. `the_end/dragon_fight.rs`: its test passes the default timing.
- **Tests:** `minecraft_terrain` (need the game data like their neighbours): `a_frozen_tick_holds_items_and_falling_blocks` (an item at y=210 and falling sand keep their positions over 20 frozen ticks and fall over 20 normal ones), `a_frozen_tick_still_picks_up` (an item with pickup delay 0 at the player's feet is picked up on a frozen tick), `primed_tnt_waits_while_frozen` (the fuse is unchanged after 10 frozen ticks and drops by 10 over 10 normal ones), `random_ticks_skip_frozen_ticks` (`game_time` stands still over 5 frozen ticks and moves by 5 over 5 normal ones). `minecraft_survival` (`tick_rate.rs`): `the_client_ticks_at_the_rate_but_never_faster_than_20_a_second` (0.05 at 20, 0.2 at 5, 0.05 at 40) and `a_frozen_client_ticks_50_ms_and_holds_the_level_until_a_tick_runs_normally` (frozen gives 0.05 with `runs_normally` false from the first frozen tick planned, kept on a frame that plans nothing).
- **Decisions:**
  - `prepare_spawning` is skipped with `sim.tick()`, because it only feeds the natural spawning inside it (vanilla's frozen `tickChunks` skips the spawn census too).
  - The client's orb pickup animations pause while frozen, as vanilla's particle engine does.
  - At a rate above 20 the client mirror still ticks every 50 ms and only sees the batches' snapshots, as a vanilla client does.
- **Test steps:** none in game on their own: nothing can freeze the game or change the rate until `/tick` lands (group 5b), whose steps 6-9 in brief section 5 test this commit. At the normal rate the game must play as before: mobs, dropped items, falling sand and primed TNT move smoothly, rain and campfire smoke show.
- **Kit:** none needed.

### 5a. Minecraft commands take decimal arguments and fixed suggestions like brigadier

Commit: 7af8c86f

- **Changed:** `minecraft_commands`, command-tree plumbing only; nothing uses it yet (`/tick` comes in 5b).
  - `args.rs`: `ArgKind::Float { min, max }` and `Arg::Float`, parsed like brigadier's `FloatArgumentType`: `read_float`, then `argument.float.low` or `argument.float.big` with the bound and the number written as Java's `Float.toString` writes them (`1.0`, `0.5`, `10000.0`), with the cursor put back at the argument's start. `ArgKind` and `NodeKind` lose `Eq` (a float bound has none); nothing needed it.
  - `tree.rs`: `Node::suggests(&'static [&'static str])`, like a brigadier `SuggestionProvider` built from a fixed list.
  - `suggest.rs`: an argument node with a fixed list offers it, filtered by the typed text as `SharedSuggestionProvider.suggest` does, instead of its kind's own suggestions. `Float` suggests nothing of its own. `complete` runs on the game's tree through a new `complete_in(roots, ...)`, so a test can complete on a tree of its own.
- **Tests:** `args.rs`: `decimal_arguments_report_bounds_like_brigadier` (`0.5` against 1-10000 gives `argument.float.low` with `1.0` and `0.5` at the argument's start, `20000` the `big` error with `10000.0` and `20000.0`, `40` parses as 40.0). `suggest.rs`: `fixed_suggestions_replace_the_argument_kinds_own` (on a test tree, a float with `suggests(["20"])` offers `20`, a float without offers nothing, and a time argument with `["1t", "1s"]` offers those instead of its units after `1`).
- **Test steps:** none in game on their own; `/tick` (5b) uses this.
- **Kit:** none needed.

### 5b. Minecraft /tick query, rate, freeze, unfreeze, step and sprint like vanilla

Commit: 79467ca2

- **Changed:**
  - New `minecraft_commands/src/tick.rs`: the `/tick` node (permission level 3) with `query`, `rate <rate>` (a float from 1 to 10000, suggesting `20`), `step`, `step stop`, `step <time>` (at least 1 tick, suggesting `1t` and `1s`), `sprint stop`, `sprint <time>` (suggesting `60s`, `1d` and `3d`), `freeze` and `unfreeze`. Each one follows `TickCommand` (javap): message keys and order, which lines go to other ops, and the return values (`query` and `rate` the rate as an int, `freeze` 1, `unfreeze` 0, the rest 1). Registered in `commands.rs`'s tree.
  - Numbers are written like Java's `String.format(Locale.ROOT, "%.1f")` and `"%.2f"`: `java_fixed` rounds half up on the shortest decimal form of the double, so 2.25 ms shows as `2.3` as in vanilla, where Rust's `{:.1}` would give `2.2`. Times go through vanilla's `nanosToMilisString` (a float division by 1e6).
  - `minecraft_commands/src/server.rs`: `TickQuery` (rate, frozen, sprinting, nanoseconds per tick, the average, the 100-tick ring) and `SprintReport` (ticks per second, ms per tick). `CommandWorld` gains `tick_query`, `set_tick_rate`, `set_tick_frozen`, `step_ticks`, `stop_stepping`, `sprint_ticks` and `stop_sprinting`, with defaults; a world without `tick_query` answers `/tick` as an unknown command. `CommandServer::server_feedback` sends a line from the server's own command source: `[Server: ...]` in grey italic to every op, only while `send_command_feedback` is on (vanilla's `broadcastToAdmins` for the server source). The test lang table has the 15 `commands.tick.*` keys and the float bound errors, and `FakeWorld` a small fake of the tick state.
  - `minecraft_survival/src/tick_rate.rs`: vanilla's `stepGameIfPaused`, `stopStepping`, `requestGameToSprint` (it keeps the frozen state to give back, and a second sprint takes the then unfrozen state, as vanilla does), `stopSprinting` and `finishTickSprint`, which now builds the report (`(int)(1000 * done / max(1, spent ns) / 1e6)` ticks per second, and the ms per tick, or the target ms when no tick ran), logs it at INFO as the server console would, and puts back the frozen state. A sprint that ends by itself during `plan` leaves its report in `take_reports()`; a sprint stopped by a command gives the report to the command. `query()` fills `TickQuery`.
  - `minecraft_survival/src/commands.rs`: `Host` holds `&mut TickRate` (built in `run` from `globals.tick_rate`) and implements the new methods on it. `run` takes the reports of sprints that ended by themselves and sends each as a server line to the ops (`report_sprints`), in the same frame they ended (`plan_ticks` runs before `run`).
  - `render_anim/src/minecraft_chat.rs`: its test `Host` gets a tick rate.
- **Tests:**
  - `minecraft_commands` (`tick.rs`): `numbers_round_half_up_like_java_format`, `query_shows_the_status_the_rates_and_the_percentiles` (running, lagging, frozen and sprinting lines, with P50/P95/P99 from a known ring and the return value 20), `the_percentiles_read_the_whole_ring_with_its_zeros`, `rate_sets_the_target_and_reports_float_bounds` (`40.0`, the ops broadcast, return 12 for 12.5, `Float must not be less than 1.0: found 0.5` with its context line, the `big` error for 20000), `steps_need_a_frozen_game_and_stop_once`, `a_second_sprint_interrupts_the_first_without_a_report`, `stopping_a_sprint_reports_it_from_the_server_first`, `freezing_stops_a_sprint_with_its_report_and_any_steps`, `server_reports_stay_silent_without_command_feedback`, `tick_needs_level_three_and_a_world_that_ticks` (level 2 gets an unknown command, level 3 the answer), `tick_suggests_its_subcommands_rates_and_times`. `/tick` added to the existing `/help` and root-suggestion lists.
  - `minecraft_survival` (`tick_rate.rs`): `the_rate_is_at_least_one_tick_a_second` (1, 3 and 10000 per second, and 0.5 clamps to 1), `steps_run_while_frozen_and_then_freeze_again`, `sprint_ticks_plan_without_waiting_and_record_their_time`, `a_sprint_reports_its_ticks_per_second_and_time_per_tick` (1200 ticks in 600 ms give 2000 and 0.50; a sprint stopped before its first tick gives 0 and 50.00), `a_sprint_gives_back_the_frozen_state_it_found`, `freezing_while_sprinting_reports_once`. (`commands.rs`): `tick_drives_the_server_tick_rate` and `a_finished_sprint_is_reported_to_the_operators_from_the_server`.
- **Decisions:**
  - A batch ends before the tick that finishes a sprint, so that sprint's last ticks are recorded before the report is made; vanilla records each tick's time right after it. Without this, a sprint that ends inside a batch would leave its last batch out of its ticks per second.
  - The report carries numbers, and the command crate writes them (`commands.tick.sprint.report` with `%.2f`), so the formatting lives in one place; the brief had `(i32, String)`.
  - The server console copy of a sprint report is a `diag::info!` line from `TickRate` (it reaches `latest.log` for both ends of a sprint), rather than a line pushed into the command server's warnings, which are logged at WARN.
  - `freeze` stops any sprint and steps by calling `stop_sprinting` and `stop_stepping` outright: they do nothing when nothing runs, which is the same as vanilla's `isSprinting`/`isSteppingForward` checks first.
  - Player lines (`Set the target tick rate ...` to other ops) keep the existing `feedback`, gated by `log_admin_commands` as before (brief section 6, left to track E).
- **Test steps** (brief section 5, steps 3 and 6-13):
  1. Start in survival on grass in daylight. Place 4 saplings, spawn 2 cows, and drop a few cobblestone (Q).
  2. Type `/gamerule random_tick_speed 20000`, wait 5 s, then type `/tick query`. It shows `The game is running, but can't keep up with the target tick rate`, then `Target tick rate: 20.0 per second.` with an average time per tick above 50 ms on the next line, then a percentile line ending in `Sample: 100`. **Screenshot 1.** Type `/gamerule random_tick_speed 3`; after about 10 s `/tick query` shows `The game is running normally`.
  3. Type `/tick freeze`. You get `The game is frozen`. Drop cobblestone: it hangs in the air where it left your hand. The cows stand still. Light a TNT block: it flashes but doesn't explode. Place sand in the air: it stays there. Pour water: it doesn't spread. The sun stops. You can still walk, jump and shoot, and walking over a cobblestone lying on the ground picks it up. **Screenshot 2.**
  4. Type `/tick step`: `Stepping 1 tick(s)`, and things move a tiny bit. Then `/tick step 5s`: `Stepping 100 tick(s)`; the TNT explodes (its fuse is 80 ticks), the hanging items fall and the water spreads, then everything stops again. Then type `/tick step stop`: red `No tick step in progress`. **Screenshot 3.**
  5. Type `/tick unfreeze`: `The game is running normally`. Then type `/tick step`: red `Unable to step the game - the game must be frozen first`.
  6. Place the furnace with coal and raw iron. Type `/tick rate 5`: `Set the target tick rate to 5.0 per second`. Mobs, items and the furnace's arrow run at a quarter speed, while MW2 movement stays normal. `/tick query` shows `(Target: 200.0ms)`. **Screenshot 4.** Then type `/tick rate 20`.
  7. Type `/tick rate 0`: red `Float must not be less than 1.0: found 0.0`, then the grey `tick rate 0<--[HERE]` line with the `0` in red.
  8. Type `/tick sprint 1d`: `The game is sprinting`. A day and night pass much faster than normal: about one to three minutes, depending on the PC (the furnace finishes, saplings grow). MW2 stays smooth. Then a grey italic line appears: `[Server: Sprint completed with ... ticks per second, or ... ms per tick]`. **Screenshot 5.**
  9. Type `/tick sprint 3d`, and after a second `/tick sprint stop`. The grey `[Server: Sprint completed ...]` report appears, then `Interrupted the current tick sprint`. Another `/tick sprint stop` gives red `No tick sprint in progress`.
  10. Press Tab after `/tick `, `/tick sprint ` and `/tick step `. The lists are `freeze query rate sprint step unfreeze`, `1d 3d 60s stop` and `1s 1t stop`. **Screenshot 6.**
  11. Quit to the menu. `iw4l-artifacts\logs\latest.log` has the two `Sprint completed with ...` lines. Send them.
- **Kit** (11 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:oak_sapling*16,minecraft:cow_spawn_egg*4,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:sand*16,minecraft:water_bucket,minecraft:furnace,minecraft:coal*16,minecraft:raw_iron*16,minecraft:dirt*32
```

### 6. The Minecraft state doc describes overload, /tick and the server's ticks

Commit: 1decd828

- **Changed:** `docs/MINECRAFT-SURVIVAL.md` only.
  - World simulation: a paragraph on the server tick (vanilla's schedule in `TickRate`, one batch per frame to each ticking level, back-pressure, the 50 ms batch budget, the `Can't keep up!` warning and skip with its thresholds, the tick time, what moves by server ticks, the autosave interval, the tick state not being saved), frozen ticks, and the client's vanilla tick timing.
  - Commands: `/tick` after `/weather`.
  - Saving: the autosave interval follows the tick rate.
  - The two "only the host's level ticks" places (multiplayer today and the multiplayer limitation) say that `Server::plan_ticks` already takes the list of ticking levels.
  - Limitations (commands): the player's own timers stay at 20 a second at any rate and while frozen (brief decision 7), MW2, the sky and the animation clock never freeze, remote friends have no tick state yet, one huge tick can't be cut short, and main-thread stalls count as falling behind.
- **Tests:** none (documentation).
- **Test steps:** none on their own; the a5 owner steps are in entries 2b, 3 and 5b.
- **Kit:** none needed.

## Item a2: eyes of ender that fly

### Root cause

No hop of the throw is broken. The tests of entry 1 drive a throw on the owner's seed (4986579828511054607) from `ender_eye::eye_use` with an eye in the hand, through `Entities::throw_eye`, the level thread, the answer, `level::finish_item_use` and the snapshot: the server answers yes, the stack goes from 16 to 15, the launch sound plays once and the eye flies north-west toward the owner's `/locate` answer (-496, -1936), 12 out and 8 up. The click side in `minecraft_world.rs` is the same chain that filled frames in the same session, and reads correct.

The real fault is the time a throw took, which hid the throw:
- Every throw ran the full stronghold search on the level thread, and that search resolved all 128 ring slots again each time (`placement.rs` `ring_chunks`, about 416,000 biome samples). It took 3.6 s in the test build (2.9 s for `/locate` from the same spot). Nothing shows before the answer: the swing, the launch sound, the lost eye and the eye itself all come with it.
- In build e231362 (before item a5), the level thread then answered only once it had worked through every tick queued during the search, while new ticks kept arriving, and the client saw one snapshot for that whole backlog. So the answer came well after the click, and the eye first showed already tens of ticks into its 80-tick flight: high up, several blocks out, and above the top of the view when looking level or down (it climbs 8 over 12 blocks). The frame fills of the same session answer at once, which is why only the eye looked broken.
- Item a5 (entries 1 and 2b) already answers each tick on its own and sends no new tick while one is in flight, so the eye now shows from its first tick. What stays is the search itself, which entry 2 moves off the tick: the rings are worked out once per world on a background thread, as vanilla does at world load.
- The brief's other two suspects are ruled out: the server never answers no on that seed (the tag `#minecraft:eye_of_ender_located` holds the stronghold, and `/locate` and the eye agree from three points), and a stuck `awaits_item_use` would also have blocked the frames, which were filled.
- The owner's log can't confirm this (nothing was logged per throw); the new log line will, with its milliseconds.

### 1. Minecraft eye of ender throws are logged and tested from the click to the flying eye

Commit: e7db22ac

- **Changed:**
  - `minecraft_terrain/src/eye_of_ender.rs`: `Eyes::locate` returns `EyeSearch` (`Found`, `NotFound`, `TimedOut`) and the time it took; an empty tag counts as `NotFound`. New `Eyes::throw` searches, launches and records an `EyeAnswer` (owner, feet, found, target, outcome "flew", "no structure" or "timed out", milliseconds), kept until `take_answers()`. `EyeAnswer::line()` writes the log line.
  - `minecraft_terrain/src/server.rs`: `ServerSim::throw_eye` goes through `Eyes::throw` and still answers the `bool` in `item_uses`; `Output` gains `eye_answers`, filled at the `ThrowEye` command.
  - `minecraft_survival/src/entities.rs`: each answer for the player is logged: `Minecraft eye of ender: flew from {feet} toward {x} {z}, target {target} ({ms} ms)` at INFO, or `Minecraft eye of ender: {outcome} from {feet} ({ms} ms), the eye stays in the hand` at WARN. New `eye_views()`.
- **Tests:**
  - `minecraft_terrain` (`eye_of_ender.rs`, they skip without the game data): `a_thrown_eye_goes_through_the_server_thread_and_flies_toward_the_stronghold` (a `ServerHandle` on the owner's seed: `item_uses == [true]`, the answer for owner 7 is "flew" to (-496, 0, -1936) under 20 s with the target 12 out and 8 up to the north-west, one launch sound, and after 20 ticks the snapshot has the eye, 20 ticks old, risen and moved that way), `the_eye_search_and_locate_agree` (three points, the same corner as `locate::nearest_structure`), `a_throw_with_no_stronghold_answers_no_with_a_reason` (the Nether: `[false]`, "no structure", the warning line, no eye in the snapshot). The two older tests read the new `locate` result.
  - `minecraft_survival` (`level.rs`): `an_eye_thrown_from_the_hand_leaves_the_stack_and_flies` (owner's seed, a `Player` at (-300.5, 80, -1700.5) looking up with 16 eyes: `eye_use` gives `ThrowEye`, `Entities::throw_eye` waits, `Entities::tick` brings the answer into `item_uses_done`, `finish_item_use` leaves 15, and `eye_views()` shows the eye rising toward the north-west).
- **Decision:** the log line also names the thrower's feet, and a no names them too, so a later log shows where a throw found nothing.
- **Test steps:** none on their own; brief section 6 step 12 (every throw has a line starting "Minecraft eye of ender:") checks this once entry 3 lands.
- **Kit:** none needed.

### 2. Minecraft stronghold rings are worked out once per world, away from the server tick

Commit: 7375ca79

- **Changed:**
  - `third_party/minecraftoss/generator/src/structure/placement.rs` (a MinecraftOSS patch, new to the patched list): `ring_chunks` takes each slot through its `resolved` cache, the one `ring_contains` already used. `/locate`, the eye and chunk generation now share one resolution per `WorldGen`.
  - `minecraft_terrain/src/locate.rs`: `has_rings` (structures on, and a concentric-ring set with a structure whose biomes the dimension can make, as vanilla's `ChunkGeneratorStructureState` keeps only those sets) and `resolve_rings` (works out every ring position of those sets, giving how many).
  - `minecraft_survival/src/server.rs`: `start_level` starts `resolve_rings_in_background`, a thread named "Stronghold rings" that works the rings out and logs `Minecraft stronghold rings: 128 worked out in {ms} ms`. This is vanilla's `ensureStructuresGenerated` at world load. A throw or `/locate` that comes first waits on the same cache; the thread holds the `Arc<WorldGen>`, so a level closed early just lets it finish. Only the overworld has rings; elsewhere the thread ends at once.
- **Measured** (test build, owner's seed): the rings take 2.5 s on their own thread; with them worked out, a second stronghold search from the same spot takes under a quarter of the first, so a throw now answers in a fraction of the earlier 3.6 s, and nothing of that runs on a tick.
- **Tests:**
  - `minecraft_terrain` (`locate.rs`): `rings_are_worked_out_once_and_agree_with_the_placement_check` (128 rings, the second call under a quarter of the first, `is_structure_chunk` true at each ring chunk and, around each, exactly where another ring chunk is), `a_second_stronghold_search_reuses_the_rings` (both find (-496, 0, -1936), the second under a quarter of the time), `only_the_overworld_has_rings` (the Nether and the End have none and resolve 0).
  - `minecraft_terrain` (`eye_of_ender.rs`): `an_overworld_eye_finds_a_stronghold_ring_chunk_corner` is no longer ignored.
  - `minecraft_survival` (`server.rs`): `the_stronghold_rings_are_worked_out_on_their_own_thread` (the thread's name, 128 rings, and a later `resolve_rings` takes under 200 ms).
- **Decision:** one background thread for all rings, not one task per ring as vanilla's `supplyAsync` per position; the result is the same and a single named thread is easy to find in a profile.
- **Test steps:** none on their own; brief section 6 steps 1-5 (entry 3) cover it. In `latest.log`, a `Minecraft stronghold rings: 128 worked out in ... ms` line follows each overworld load.
- **Kit:** none needed.

### 3. Minecraft eyes of ender are thrown once per click, and in adventure mode, like vanilla

Commit: eed77a31

- **Changed:**
  - `minecraft_terrain/src/ender_eye.rs`: `eye_use` is split like vanilla's `useOn` and `use`. `frame_use` fills an empty frame and stays in the build chain. `throw_use` passes (`None`) when aiming at any end portal frame, filled or not, and otherwise gives `Change::ThrowEye`.
  - `render_anim/src/minecraft_world.rs`: the build chain calls `frame_use`; a new branch right after it calls `throw_use` and needs only `game_mode.interacts()` (no spectators) and no item use waiting, so adventure players throw. It sends the same `SentItemUse` through `entities.throw_eye` and marks the hand as using the eye. While that mark holds, a held button (or tactical key) does not repeat any use; releasing it or selecting another slot clears it, and a fresh press always fires.
  - `minecraft_survival/src/player.rs`: `Hand.using_eye: Option<usize>`, the slot of the eye being "used" (vanilla's `startUsingItem` with a use duration of 0), per player.
  - `minecraft_survival/src/offhand.rs`: `eye_held(hand, use_down, selected)`: the mark holds while the button is down and the eye's slot is the selected slot or the offhand.
  - `docs/MINECRAFT-SURVIVAL.md`: the eye paragraph says one eye per click, adventure and spectator, the no-stronghold case, the background rings and the log line.
- **Tests:** `minecraft_terrain` (`ender_eye.rs`): `adventure_mode_throws_but_frames_need_building` (`throw_use` throws on stone and air and passes on both frames, `frame_use` fills only an empty frame); `a_filled_frame_passes_the_eye_on` and `anywhere_else_the_eye_is_thrown_from_the_middle_of_the_body` use the split functions. `minecraft_survival` (`offhand.rs`): `holding_the_use_button_throws_one_eye`; `level.rs`'s throw test calls `throw_use`.
- **Decisions:**
  - The mark is set when the throw is sent, whatever the answer, as vanilla starts using the item before its stronghold search; so holding the button over a world without strongholds sends one search, not one every 4 ticks.
  - While the mark holds, no use repeats in either hand, as vanilla's `startUseItem` doesn't run while the player is using an item.
  - In adventure mode, right-clicking a frame with an eye does nothing (the frame refuses, then the throw passes on the frame), as vanilla.
- **Test steps** (brief section 6; a new world on the owner's seed). In the `.env` set `IW4L_MINECRAFT_WORLD=r8eye` and `IW4L_MINECRAFT_SEED=4986579828511054607`, and remove any `IW4L_MINECRAFT_DIMENSION` line. After the test, send the screenshots and `iw4l-artifacts\logs\latest.log`.
  1. Start by day. Type `/gamemode creative`, then `/tp @s -300 120 -1700`, wait for the ground to load, land, and type `/gamemode survival`.
  2. Type `/locate structure minecraft:stronghold`. It answers about [-496, ~, -1936].
  3. Face north-west (toward the stronghold), look level, and right-click once with an eye. Within a moment you hear the launch sound and your hand swings, the stack loses one eye, and the eye rises from your chest and floats about 12 blocks toward the stronghold, climbing about 8, glowing, with a purple trail. **Screenshot 1** while it flies, with the `/locate` answer still in the chat.
  4. After about 4 s you hear a short sound. The eye either falls as an item you can pick up, or breaks into crumbs and a purple ring.
  5. Turn your back to the stronghold and throw again: the eye flies behind you, toward the stronghold (turn round to see it).
  6. Hold right-click for 3 s: only one eye flies. Release and click again: another flies.
  7. Throw 10 eyes, one click each: about 8 drop and 2 break.
  8. Type `/gamemode creative` and `/tp @s -496 120 -1936`, then throw. The eye sinks straight down into the ground under you; vanilla does this when you stand over the stronghold. **Screenshot 2.**
  9. Type `/gamemode adventure` and throw: the eye flies. Place an end portal frame first (in creative), then in adventure right-click it with an eye: nothing happens. Type `/gamemode survival` and right-click the frame: the eye goes in. Right-click the filled frame: nothing happens.
  10. Type `/execute in minecraft:the_nether run tp ~ 128 ~` (on top of the Nether's bedrock roof) and right-click with an eye: no sound, no swing, the stack stays. Hold the button: still nothing. **Screenshot 3.** Then `/execute in minecraft:overworld run tp ~ 120 ~`.
  11. Compare the held eye with another flat item, such as `/give @s minecraft:snowball 1`: they're held the same way, as large and as far to the lower right as vanilla draws them.
  12. In `latest.log`: one `Minecraft stronghold rings: 128 worked out in ... ms` line after the world loads, and for every throw a line starting `Minecraft eye of ender:`, "flew" in the overworld with its time in ms, "no structure" in the Nether (one line for the held button, not one every 4 ticks).
- **Kit** (3 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:ender_eye*48,minecraft:end_portal_frame*2,minecraft:cobblestone*64
```

## Item a3: vanilla mining speed

### 1. Minecraft haste, mining fatigue and conduit power are real effects

Commit: dd3f92d4

- **Changed:**
  - `third_party/minecraftoss/player/src/survival.rs` (a MinecraftOSS patch; the file was already on the patched list): `EffectKind` gains `Haste`, `MiningFatigue` and `ConduitPower` at the end, and `EffectKind::ALL` lists every kind in order. The effect array is sized from it and `tick_effects` walks it, so a later kind (track C's levitation and slow falling) is added in the enum and `ALL` only. New `SurvivalStatus::attack_speed_multiplier()`: `(1 + 0.1 (haste + 1)) x (1 - 0.1 (fatigue + 1))` with vanilla's `0.1F` widened to a double, 1 for a missing effect.
  - `minecraft_survival/src/player_nbt.rs`: the three ids in `EFFECTS` (sized from `EffectKind::ALL`), and the `minecraft:effect.haste` and `minecraft:effect.mining_fatigue` modifiers on `minecraft:attack_speed` (base 4.0, `add_multiplied_total`, +/-0.10000000149011612 a level), as `MobEffects` adds them.
  - `minecraft_survival/src/commands.rs`: `effect_kind` knows `haste`, `mining_fatigue` and `conduit_power`; `/effect clear` with no effect checks `EffectKind::ALL`. `minecraft_survival/src/survival.rs`: `clear_effects` (milk, death, `/effect clear`) walks `EffectKind::ALL`.
  - `minecraft_terrain/src/potions.rs`: custom potion contents with the three ids reach the player.
  - `render_anim/src/minecraft_world.rs`: the attack's speed is the item's attack speed times `attack_speed_multiplier`, clamped to the attribute's 0 to 1024, before `attack_strength`.
- **Tests:** `minecraftoss-player` (`survival.rs`): `every_effect_kind_is_listed_once_at_its_own_index` (each kind at its own index, all 13 can be added and run out), `haste_and_mining_fatigue_scale_the_attack_speed` (1.1 for Haste I, 1.2 for Haste II, 0.8 for Mining Fatigue II, 1.1 x 0.9 for both at I, conduit power 1). `minecraft_survival`: `commands.rs` `haste_mining_fatigue_and_conduit_power_are_given_and_cleared_one_by_one` (`/effect give @s minecraft:haste 60 1` gives amplifier 1 for 1200 ticks, `/effect clear @s minecraft:mining_fatigue` removes only that one, a bare clear the rest); `player_nbt.rs` `haste_mining_fatigue_and_conduit_power_are_saved_with_their_attack_speed_modifiers` (the three round-trip, the haste modifier is 0.2 for amplifier 1 and fatigue's -0.1, both go once the effects end and the attribute keeps its base); `survival.rs`'s milk test also clears the three. `minecraft_terrain` (`potions.rs`): the three ids map to the player's kinds.
- **Decision:** the multiplier clamps at the attack speed (0 to 1024, the attribute's range), so Mining Fatigue X or more gives speed 0 as vanilla's clamped attribute does. The item tooltip's attack speed stays the item's own, as vanilla's tooltip shows it without effects.
- **Test steps** (the mining side comes with groups 3 and 4):
  1. Type `/effect give @s minecraft:haste 60 1`. The chat answers `Applied effect Haste to ...` (before this build: red `Unable to apply this effect ...`). Do the same with `minecraft:mining_fatigue 60 2` and `minecraft:conduit_power 60 0`. **Screenshot 1** of the three answers.
  2. Type `/effect clear @s minecraft:mining_fatigue`: `Removed effect Mining Fatigue from ...`. Type it again: red `Target doesn't have the requested effect`.
  3. Quit to the menu and load the world again within the minute. Type `/effect clear @s minecraft:haste`: `Removed effect Haste ...`, so the effect was saved.
  4. Type `/effect give @s minecraft:haste 60 1` again, drink the milk bucket, then type `/effect clear @s minecraft:haste`: red `Target doesn't have the requested effect`, the milk cleared it.
- **Kit** (1 stack):

```
IW4L_MINECRAFT_KIT=minecraft:milk_bucket
```

### 2. Minecraft reads which blocks need the right tool from the game jar

Commit: 62072e02

- **Changed:**
  - New `minecraft_terrain/src/mining_speed.rs` (registered in `lib.rs`): `MiningBook::from_jar` reads `requiresCorrectToolForDrops` out of the jar's `Blocks` bytecode with `class_file`, as no data file holds it.
    - Ids: `id_fields` reads `BlockItemIds.<clinit>` and `BlockIds.<clinit>`: `create(name)` and `create(block, item)` give the block id, `createSimpleColored` the 16 dyed ids in `DyeColor` order, `createSimpleCopper` the four weather states and their `waxed_` twins, and the copper block's `same(ByState)` its special names (`copper_block`, `exposed_copper`, ...). Any other shape is reported as a warning.
    - Segments: `Blocks.<clinit>` is cut at every `putstatic` of a `Block`, `ColorCollection` or `WeatheringCopperCollection` field; the segment's id is its first `getstatic` of `BlockItemIds` or `BlockIds`.
    - Flag: a segment needs the right tool when it, a lambda it reaches through `invokedynamic` (`REF_invokeStatic Blocks.*`), or a `Blocks` helper returning `Properties` calls `Properties.requiresCorrectToolForDrops`, followed recursively, each method once.
    - Copies: `ofFullCopy`, `ofLegacyCopy`, `registerStair`, `registerLegacyStair`, `registerSlab` and `registerWall` take the flag from the nearest `getstatic Blocks.<block or collection>` before them in the same method body; inheritance is resolved after the scan, so chains and collection copies (concrete stairs from concrete, cut copper stairs from cut copper) work.
    - The result is 470 ids. A failed scan leaves the set empty with a warning, so blocks then drop by hand as before.
  - `minecraft_survival/src/data.rs`: `GameData.mining`, loaded beside `protection`; each warning is logged, then `Minecraft mining: 470 blocks need the right tool to drop` at INFO.
- **Tests:** `minecraft_terrain` (`mining_speed.rs`): `dyed_ids_follow_the_dye_colour_order`, `copper_ids_have_every_weather_state_and_its_waxed_twin`, `the_copper_block_uses_its_special_names`, `a_book_made_from_a_list_answers_for_it`; with the game data: `the_jar_names_the_blocks_that_need_the_right_tool` (no warnings, more than 400 ids, the brief's 27 blocks that need a tool and 20 that don't) and `every_block_that_needs_the_right_tool_is_a_real_block` (all 470 ids are in the block registry). `minecraft_survival` (`data.rs`): no mining book without a jar or with an unreadable one.
- **Decision:** the brief only names `BlockItemIds`, but the 114 blocks without an item (wall torches, wall signs, fluids, coral wall fans and so on) take their id from `BlockIds`, which has the same `create` and `createSimpleColored` shapes; both are read, so a wall variant that copies a tool block would be found too.
- **Test steps:** none in game on their own; nothing uses the book yet (group 4 does). After loading a world, `iw4l-artifacts\logs\latest.log` has the line `Minecraft mining: 470 blocks need the right tool to drop` and no `Minecraft mining:` warning.
- **Kit:** none needed.

### 3. Minecraft tools mine at their vanilla speeds with Efficiency, Haste, Mining Fatigue and the water and air penalties

Commit: 201eca45

- **Changed:** `minecraft_terrain/src/mining_speed.rs` only; nothing in game calls it yet (group 4 does).
  - `MiningBook::from_jar` also reads `data/minecraft/enchantment/*.json`: each enchantment's `minecraft:attributes` entries for `mining_efficiency`, `submerged_mining_speed` and `block_break_speed` (amounts: constant, `linear`, `levels_squared`, `clamped`; operations `add_value`, `add_multiplied_base`, `add_multiplied_total`) and its `slots` groups. `MiningBook::attribute(attribute, base, range, equipment)` counts an enchantment only in a slot its group fits (`any`, `hand`, `armor`, or the slot's own name) and applies vanilla's `AttributeInstance.calculateValue`: `((base + sum add) x (1 + sum multiplied_base)) x product (1 + multiplied_total)`, clamped to the range. Amounts are `f32` like `LevelBasedValue`, then widened.
  - `Tool` and `ToolRule` (`RuleBlocks::Tag` or `Blocks`, optional speed and correct flag, default speed, `damage_per_block`, `destroys_in_creative`). `tool_of(stack)` builds `ToolMaterial`'s diggers (`<wooden|stone|copper|iron|diamond|golden|netherite>_<pickaxe|axe|shovel|hoe>`: deny drops on `#incorrect_for_<material>_tool`, then mine and drop `#mineable/<kind>` at 2, 4, 5, 6, 8, 12 or 9), swords (cobweb 15 and correct, `#sword_instantly_mines` at `f32::MAX`, `#sword_efficient` 1.5, 2 per block, not in creative), shears (cobweb 15 and correct, leaves 15, wool 5, vines and glow lichen 2), and the mace and trident (no rules, 2 per block). A stack's own `minecraft:tool` component wins, and `!minecraft:tool` removes it. `Tool::speed` and `Tool::correct` take the first matching rule that has the value, as `Tool.getMiningSpeed` and `isCorrectForDrops` do.
  - `Digger` (held stack, offhand, the four armour slots, the haste, conduit power and mining fatigue amplifiers, eye in water, on ground), `correct_for_drops` (`!requiresCorrectToolForDrops || tool correct`), `destroy_speed` (vanilla `Player.getDestroySpeed`, checked with javap, all `f32` in its order: tool speed; Efficiency only above 1; `x (1 + (amp + 1) x 0.2)` with the larger of the haste and conduit amplifiers; `x (float) 0.3^(amp + 1)`; `x block_break_speed`; `x submerged_mining_speed` with the eye in water; `/ 5` off the ground), `destroy_progress` (0 for hardness -1, else `speed / hardness / 30 or 100`) and `ticks_to_break` (an `f32` running sum from the click tick, `None` when it never gets there).
- **Tests** (`mining_speed.rs`, a fake `BlockFacts` with the needed tags): `netherrack_breaks_at_once_with_an_efficient_or_golden_pickaxe` (Efficiency V 2.83 and instant, II instant, I and plain 2 ticks, golden exactly 1.0, wooden 6, hand 41 and not correct), `stone_follows_efficiency_and_the_mining_effects` (hand 151, plain 6, Efficiency V 2, Haste II instant, Conduit Power I 2, Haste I with Conduit Power II instant, Mining Fatigue I 5, III 50 at 0.0204), `water_and_air_slow_mining_unless_aqua_affinity_helps` (7, 34, 7 with Aqua Affinity, 7 dry in the air), `hard_blocks_need_the_right_tier` (obsidian 45 with Efficiency V, the iron pickaxe divides by 100 and isn't correct, iron ore 151 with wood and not correct, 23 with stone), `swords_shears_and_axes_have_their_own_blocks` (cobweb 8 with sword or shears, 401 by hand, bamboo instant and finite, wool 5 with shears, oak log 10 with an iron axe, dirt 15 with an Efficiency V pickaxe and by hand), `free_and_unbreakable_blocks` (a torch with Mining Fatigue III is instant, bedrock never breaks), `efficiency_counts_only_in_the_main_hand`, `tools_come_from_the_item_or_its_tool_component`, `without_a_book_every_block_drops_and_nothing_is_enchanted`, `level_values_and_slot_groups_follow_vanilla`, and with the game data `efficiency_and_aqua_affinity_read_from_the_jar` (Efficiency V gives `mining_efficiency` 26, Aqua Affinity I `submerged_mining_speed` 1.0).
- **Decisions:**
  - `Digger` holds the held stack, the offhand and the armour, and builds the slot list for the attributes from them, instead of taking a separate equipment list (brief), so the held stack can't disagree with the main hand's enchantments.
  - Without a mining book (no jar) no block needs a tool and no enchantment counts, which is how mining and drops worked before.
  - Spears and every other item have no tool, as in vanilla; the creative sword rule stays in `game_mode.rs`, `destroys_in_creative` is kept for it.
- **Note:** during this group's `minecraft_survival` run, `the_end::tests::an_arrival_rebuilds_the_platform_once_its_chunks_are_in` failed once while another track's build used the machine, and passed on two reruns; this commit doesn't touch `minecraft_survival`.
- **Test steps:** none in game on their own; group 4 puts this into hand mining.
- **Kit:** none needed.

### 4. Minecraft hand mining follows vanilla's dig loop: instant breaks, the 5-tick delay, cracks, hit sounds and particles

Commit: 68ddafb0

- **Changed:**
  - New `minecraft_survival/src/digging.rs` (registered in `lib.rs`): `Dig`, the per-player dig state (`Hand.dig` in `player.rs`, so it lives on the `Player` of each UUID): the block being destroyed with its face and the item that started it, the `f32` progress, the dig ticks, the destroy delay and a click waiting for the next tick. `Dig::tick(DigInput)` follows `MultiPlayerGameMode` and `Minecraft.continueAttack` (brief section 3.5): a click starts (`startDestroyBlock`: abort a running dig, attack the block, break at once when one tick's progress reaches 1 with no crack and no delay, else start at 0 and record the item), and then, unless the start broke the block or the button is up, the tick continues (`continueDestroyBlock`: the delay first, then progress on the same block and the same item, `isSameItemSameComponents`, so a changed count doesn't restart; a break sets the delay to 5; another block or item starts again with no progress that tick). A target that turned to air ends the dig quietly; no target or a released button stops it (`stopDestroyBlock`) and says so, so the adapter resets the attack strength. `Dig::crack()` gives the stage `(int)(progress x 10)` only while progress is above 0. `digger(...)` builds the `mining_speed::Digger` from the player's inventory (held, offhand, armour 36-39), the Haste, Conduit Power and Mining Fatigue amplifiers, the eye in water and the ground.
  - `render_anim/src/minecraft_world.rs`: the survival hand no longer pushes fake bullets. The click on a block (`creative_press`, now `block_press`) also queues a dig click. Each hand tick runs `hand.dig.tick` with the block the player aims at within the game mode's reach (a mob in front counts as no target, as in creative), the held stack, and `mining_speed::destroy_progress` with the world's `MiningBook`, the block's hardness from its state and the `Digger`. Events: an attack goes to `entities.attack_block` (the server only acts on blocks that handle it, such as the dragon egg); a break goes to `hand_broken` and `hand_mined` and through `mining.break_now` (the same burst, break sound and server path as before), and the staged world forgets the block so a later tick of the same frame aims past it; a start plays the hit sound and every hit tick adds a chip, with the hit sound on every fourth dig tick, at `(volume + 1) / 8` and pitch x 0.5. The swing restarts on each dig tick, delay ticks included. Outside the dig branch (button up, gun, creative, busy, menus, dead) the dig stops and the attack strength resets if a dig was running. The crack mesh takes `hand.dig.crack()` beside the bullet cracks. Bullets keep their own MW2 rules and hit sound.
  - `render_anim/src/minecraft_mining.rs`: `crack_mesh` also draws the dig's stage; `hit_particle` wraps the new chip.
  - `minecraft_terrain/src/block_particles.rs`: `spawn_hit` (vanilla `addBreakingParticles`): one terrain particle at a random point of the outline's bounds, pushed 0.1 off the hit face, with zero speed, `setPower(0.2)` and `scale(0.6)`.
  - `third_party/minecraftoss/player/src/lib.rs` (a MinecraftOSS patch; the file was already on the patched list): `outline_bounds(block)`, the bounds of the block's outline boxes, for the chip.
- **Tests:** `minecraft_survival` (`digging.rs`): `dirt_by_hand_starts_on_the_click_and_breaks_on_the_fifteenth_tick` (attack, start and the first progress on the click tick, stages 0 to 9, the break on tick 15 with delay 5), `holding_on_waits_five_ticks_then_starts_the_next_block_without_progress` (breaks on ticks 15 and 36, ticks 16-20 idle, tick 21 a start with no progress), `an_instant_block_breaks_every_tick_with_no_crack_and_no_delay`, `an_instant_click_ends_the_tick_without_a_dig`, `the_hit_sound_plays_on_the_start_and_every_fourth_tick_with_a_particle_each_tick` (sounds on the start and ticks 4, 8, 12; 14 chips), `letting_go_forgets_the_crack_and_a_new_click_starts_from_nothing`, `another_item_restarts_the_dig_but_another_count_does_not`, `a_block_gone_to_air_ends_the_dig_without_a_break`, `losing_the_target_while_digging_stops_like_letting_go`, `a_moved_target_aborts_and_starts_again_with_an_attack`, `a_press_waits_for_the_next_tick`, `the_digger_reads_the_hands_the_armour_and_the_effects`. `minecraft_terrain` (`block_particles.rs`): `a_hit_chip_starts_just_off_the_face_that_was_hit`. `minecraftoss-player`: `outline_bounds_cover_every_box_of_the_outline`.
- **Decisions:**
  - Every start attacks the block, as the server's `START_DESTROY_BLOCK` does whatever the client predicted; the client's own `destroyProgress == 0` check only guards its prediction.
  - A break tick gives the break only, no hit: vanilla's server sends the hit events only while the dig runs.
  - A click is kept until the next hand tick, as vanilla's `consumeClick` waits for the client tick; a click shorter than a tick still starts (and an instant block still breaks), then the released button stops the dig.
  - `Dig::tick` returns the events with a `swing` and a `stopped` flag (the brief had only the events), so the adapter can swing like `continueAttack` and reset the attack strength like `stopDestroyBlock`.
  - While the hand is busy (eating, blocking, charging, the miss delay, an MW2 offhand) the dig stops, as the brief says; vanilla can't start those while digging.
  - Drops, the correct-tool rule, exhaustion and tool wear are unchanged in this commit (group 5).
- **Test steps** (brief section 5, steps 1-6, 9 and 10; the drop and wear steps come with group 5):
  1. In the Nether (or on placed netherrack), select the Efficiency V diamond pickaxe and click netherrack: it breaks the instant the button goes down, with no crack. Hold the button and sweep the crosshair along a wall: one block a tick, about 20 a second, no pause between blocks. **Screenshot 1** of a long tunnel dug while holding.
  2. The plain diamond pickaxe on netherrack: a short crack flash, 2 ticks a block; holding on, each next block waits a short beat (5 ticks) before it starts. The golden pickaxe is instant too; the wooden pickaxe takes about 0.3 s a block.
  3. A bare hand (an empty hotbar slot) on netherrack takes about 2 s, on stone 7.5 s. The crack climbs through 10 stages, the hit sound ticks about 5 times a second and small chips fly off the face you hit. Let go halfway and click again: the crack starts over. **Screenshot 2** of a half-cracked stone block.
  4. Stone with the Efficiency V pickaxe: 2 ticks a block. `/effect give @s minecraft:haste 60 1`: stone breaks instantly. `/effect give @s minecraft:mining_fatigue 60 2`: about 2.5 s. Drink the milk: 2 ticks again. `/effect give @s minecraft:conduit_power 60 0`: still 2 ticks.
  5. Dive into water: standing on the bottom with the Efficiency V pickaxe a stone takes about 0.35 s, swimming without touching the bottom about 1.7 s; with the Aqua Affinity helmet on, swimming is back to about 0.35 s.
  6. Climb a ladder and mine stone beside it: about 0.35 s instead of 2 ticks.
  7. Obsidian with the Efficiency V pickaxe: about 2.3 s; with the plain diamond pickaxe about 9.4 s. Iron ore with the wooden pickaxe about 7.5 s, with the stone pickaxe about 1.2 s.
  8. Cobweb: by hand about 20 s, sword or shears 0.4 s. Bamboo with the sword: instant. Wool: shears 0.25 s, hand 1.2 s. Oak log: iron axe 0.5 s, hand 3 s. Dirt: the Efficiency V shovel tunnels like step 1; the Efficiency V pickaxe takes 0.75 s, like a bare hand.
  9. `/gamemode creative`: a click breaks any block at once, holding breaks one every 6 ticks, the sword breaks nothing. `/gamemode survival` afterwards.
- **Kit** (22 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_pickaxe,minecraft:golden_pickaxe,minecraft:wooden_pickaxe,minecraft:stone_pickaxe,minecraft:netherrack*64,minecraft:stone*64,minecraft:obsidian*16,minecraft:iron_ore*8,minecraft:diamond_shovel+efficiency=5,minecraft:dirt*64,minecraft:shears,minecraft:white_wool*16,minecraft:cobweb*8,minecraft:diamond_sword,minecraft:bamboo*16,minecraft:diamond_helmet+aqua_affinity,minecraft:water_bucket,minecraft:ladder*8,minecraft:milk_bucket,minecraft:oak_log*16,minecraft:iron_axe
```

### 5. Minecraft blocks mined with the wrong tool drop nothing, and mining wears the tool, like vanilla

Commit: c14b293f

- **Changed:**
  - `minecraft_survival/src/digging.rs`: `wrong_tool(book, facts, broken, held)` lists the hand-broken blocks the held stack isn't correct for (`Player.hasCorrectToolForDrops`: the block needs the right tool and the tool's rules don't say yes). `wear_after_break(inventory, selected, hardness, random)` is `Item.mineBlock`: when the block's hardness isn't 0 and the held stack has a tool (`mining_speed::tool_of`, so a `minecraft:tool` component wins), it costs the tool's `damage_per_block`, each point spared with chance `unbreaking / (unbreaking + 1)` as the elytra does; it gives the item id back when the tool broke.
  - `render_anim/src/minecraft_world.rs`: the dig's breaks go through `break_now` on their own, so the frame knows which blocks the hand broke. Before anything changes the held stack, the wrong-tool ones are found; they are left out of the drops and the block experience and cost no exhaustion (`Block.playerDestroy` isn't reached). The server still hears every break (`player_mines`), containers still spill. After the drops each dug block wears the held tool, wrong tool or not, as vanilla's `mineBlock` runs before the correct-tool check; a tool that breaks plays the item break sound and its crumbs (`broken_items`), as a worn-out flint and steel does. Creative breaks never reach this path.
  - `minecraft_survival/src/entities.rs`: hand loot rolls with the real held stack (none for an empty hand) instead of an enchanted diamond pickaxe, so shears on leaves or grass drop the block and a sword on cobweb drops string. Bullets and blasts keep the diamond pickaxe (`LOOT_TOOL`). `hand_tool` and `loot_tool_with` are gone; their test now checks silk touch on the held stack.
- **Tests:** `minecraft_survival` (`digging.rs`): `a_wrong_tool_or_a_bare_hand_breaks_without_drops` (a fake tag table: a bare hand on stone and cobweb and a wooden pickaxe on iron ore are wrong; a stone pickaxe on iron ore, a wooden pickaxe on stone, a sword or shears on cobweb and a hand on dirt are right; without a mining book nothing is wrong), `mining_wears_a_tool_by_its_damage_per_block` (a pickaxe 1 a block, a sword and a trident 2, a torch nothing, a stick nothing, an empty hand nothing), `unbreaking_three_spares_about_three_points_in_four` (4000 blocks with a seeded random cost 850-1150), `the_last_point_breaks_the_tool` (the slot empties and the id comes back), and with the game data `the_real_tool_rolls_the_loot` (the jar's loot and tool rules: no drop by hand on stone or cobweb or with a wooden pickaxe on iron ore; raw iron with a stone pickaxe, string with a sword, a cobweb with shears, dirt by hand, cobblestone with a diamond pickaxe). `entities.rs`: `silk_touch_is_read_from_the_held_enchantments` replaces `the_hand_loot_tool_carries_the_held_enchantments`.
- **Decisions:**
  - The wear runs after the drops and the experience, so the loot and the correct-tool check see the stack the block was mined with, as `destroyBlock` takes `hasCorrectToolForDrops` and a copy of the main hand before `mineBlock`.
  - A creative player's hand loot (only shulker boxes with contents drop in creative) also rolls with the held stack; the shulker box table has no tool condition, so nothing changes there.
  - Hand drops still come from the local adapter's `drop_blocks`, per player through the held stack; the server never sees the tool (track E's action move).
- **Test steps** (brief section 5, steps 3, 7, 8, 9 and 11, plus wear):
  1. A bare hand (an empty hotbar slot) on netherrack: about 2 s and nothing drops. On stone: 7.5 s and nothing drops. Dirt by hand still drops dirt.
  2. Obsidian with the Efficiency V pickaxe: about 2.3 s and it drops.
  3. Iron ore with the wooden pickaxe: about 7.5 s and nothing drops. With the stone pickaxe: about 1.2 s and raw iron drops. **Screenshot 1** of the raw iron on the ground.
  4. Cobweb: by hand about 20 s, no drop; with the sword 0.4 s and string drops; with the shears 0.4 s and the cobweb itself drops. **Screenshot 2** of the string and the cobweb on the ground.
  5. Break a few oak leaves with the shears: the leaves themselves drop.
  6. Mine 10 stone with the plain diamond pickaxe and hover over it in the inventory: the durability bar shows a little wear (10 of 1561). Break a torch with it: no wear. Mine 2 blocks with the diamond sword: it loses 4.
  7. Select the worn wooden pickaxe from the kit (1 use left) and mine one stone: the pickaxe breaks with the item break sound and crumbs.
  8. In 26.3 iron doors, bells, lanterns, ender chests, brewing stands, copper doors, stone pressure plates and stone buttons drop even when broken by hand (slowly); this is vanilla, not a bug.
  9. `/gamemode creative`, break blocks with the pickaxe: no wear. `/gamemode survival` afterwards.
- **Kit** (13 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_pickaxe,minecraft:wooden_pickaxe,minecraft:wooden_pickaxe+damage=58,minecraft:stone_pickaxe,minecraft:netherrack*64,minecraft:stone*64,minecraft:obsidian*16,minecraft:iron_ore*8,minecraft:shears,minecraft:cobweb*8,minecraft:diamond_sword,minecraft:oak_leaves*16
```

### 6. Docs: Minecraft mining speed

Commit: 7258b0f2

- **Changed:** `docs/MINECRAFT-SURVIVAL.md` only.
  - Mining and placing: a paragraph on survival hand mining: the progress per tick (/ 30 or / 100), the tool speeds and rules, Efficiency, Haste, Conduit Power, Mining Fatigue, the water and air penalties and Aqua Affinity, the right-tool list read from the jar (with 26.3's hand-droppable blocks), the dig loop (instant breaks and tunnelling, the crack, the hit sound every 4 ticks, the chips, the 5-tick delay, what starts it over), wrong-tool breaks without drops, experience or exhaustion, hand loot with the real held item, tool wear with Unbreaking, and bullets and creative keeping their own rules.
  - Commands: `/effect` applies haste, mining fatigue and conduit power; the limitation counts twelve simulated effects.
  - Crate layout: the player's hand holds the dig in progress.
  - Limitations: the old "hand mining counts as a diamond pickaxe" line is gone. New: `block_break_speed` from item attribute modifiers is not read, other players won't see a crack, chips or hit sounds until multiplayer, the client decides the break with no server tolerance check, Conduit Power only speeds mining, Haste and Mining Fatigue only come from `/effect` and custom potions, and the water and ground tests come from MW2 movement.
- **Vendored code (item a3):** `third_party/minecraftoss/player/src/survival.rs` (entry 1: the three effect kinds, `EffectKind::ALL`, `attack_speed_multiplier`) and `third_party/minecraftoss/player/src/lib.rs` (entry 4: `outline_bounds`) were patched. Both were already on the fork's patched list, so item a3 adds no new patched or added MinecraftOSS file.
- **Tests:** none (documentation).
- **Test steps:** none on their own; the a3 owner steps are in entries 4 and 5 (brief section 5).
- **Kit:** none needed.

## Item a4: the mob inside a spawner

### 1. Minecraft spawners show their mob spinning inside like vanilla

Commit: 723ef47a

- **Changed:**
  - `minecraft_terrain/src/client_mobs.rs`: `MobPose.turn`, an outer turn that `body_rotation` puts before the body's own turn and death flip (the outer `PoseStack` of `SpawnerRenderer`); `ClientMob::pose` sets it to identity. `ClientMobs::show_in_spawner(id, turn, light_probe)` stores a display's turn and light probe, and `ClientMobs::pose` applies them. `piglin_render.rs`'s test literal gets `turn: Quat::IDENTITY` (Track B: every new `MobPose` literal needs it).
  - New `minecraft_terrain/src/spawner_display.rs`: `SpawnerDisplays`, the client's spawner block entities of one level (`BaseSpawner` on the client). Per spawner: the `SpawnData.entity` compound, the client's delay, `MinSpawnDelay`, `RequiredPlayerRange`, `spin`/`oSpin`, and the display mob, built once per spawn data from a copy of the compound placed at the cage's feet (`server_mobs::spawn_saved` into a one-mob `EntityWorld`, our `loadEntityRecursive`) and never ticked, so its age is the partial tick as in vanilla.
    - `load_chunk` reads the chunk's `mob_spawner` tags (then pending tags), `unload_chunk` drops them, `block_entities` follows the server's block entity changes: removed or no longer a spawner drops it, a new one is read, a changed `SpawnData` entity rebuilds the mob and takes `Delay` (`load()`), any other update of a known spawner is a spawn round and sets the delay to `MinSpawnDelay` (the update packet then block event 1).
    - `tick(players)` is `BaseSpawner.clientTick`: nobody within range of the block centre keeps the spin; with a mob, the delay counts down and the spin steps by `1000 / (delay + 200)` degrees, wrapped at 360 before the x10.
    - `append` draws each spawner within 64 blocks of the camera whose block is still `minecraft:spawner`: the mob through its own renderer, turned by `rotY(spin x 10) rotX(-30)` about its feet at block + (0.5, 0.2, 0.5), lit from the spawner cell, and scaled about the feet by 0.53125 (divided by the longest side of its box above 1).
  - `minecraft_survival/src/entities.rs`: `Entities` owns the level's `SpawnerDisplays` (client state per level, like the lids): loaded and unloaded with the stream's chunks, told of every server block entity change, ticked in the client tick (not while the game is frozen) with the player's feet when alive and not a spectator, and drawn into the mob meshes beside the blazes.
  - `docs/MINECRAFT-SURVIVAL.md`: the spawner mob under World and rendering; spawners leave the "not simulated" list; a Rendering limitation for kinds without a model, jockeys and the blaze's full brightness.
- **Tests:** `minecraft_terrain` (`spawner_display.rs`): `a_display_scale_fits_the_longest_side_into_the_cage` (zombie 0.53125/1.95, spider /1.4, chicken and a 1x1 box 0.53125), `spin_steps_a_thousand_over_delay_plus_two_hundred` (+2.5 at 200, +5 at 0, 1000/199 at -1, the wrap), `a_far_spawner_freezes_its_spin_and_delay` (17 blocks away: frozen; 15: delay -1 and spin grows; no player: `oSpin = spin`), `an_empty_spawner_neither_spins_nor_draws`, `a_spawn_round_restarts_the_spin_at_the_minimum_delay` (same mob with `Delay` 600 ends at 200 and keeps the spin; a new mob takes `Delay` and rebuilds; `None`, a chest tag and `unload_chunk` remove), `a_chunk_brings_its_spawners_and_their_delay` (entities before pending, chests ignored), `a_zombie_and_a_blaze_sit_inside_their_cage` (every vertex inside the cell with a 0.05 margin at 0, 90 and 200 degrees, and the turn moves them), `a_display_is_lit_by_its_spawner_cell`, `spawners_further_than_64_blocks_are_not_drawn` (also not once the block is gone). `client_mobs.rs`: `a_spawner_turn_comes_before_the_body_turn`.
- **Decisions:**
  - The display is drawn with the existing mob renderers on a one-mob entity world per spawner (brief decision 1); the uniform scale is applied to the new vertices about the feet after the renderer, which equals vanilla's `scale(s)` after the two turns.
  - Spin and countdown run whenever `SpawnData.entity` has an `id`, even for a kind we can't draw; vanilla's display entity exists for any valid id (brief decision 6).
  - The display state is client state per level in `Entities`; the near-player test takes a slice of player feet (only the local player today). Track E: that slice must become every player the client knows in this level.
  - A display never ticks, so it has no flames yet (group 2) and blaze rods twitch rather than turn, as in vanilla.
- **Test steps** (spawn eggs on spawners come with group 3; until then use generated spawners):
  1. `IW4L_MINECRAFT_DIMENSION=the_nether`, then `/locate structure minecraft:fortress` and `/tp` there. Find the blaze spawner (on a small platform of nether bricks with stairs, often at a corridor's end). A small blaze sits in the middle of the cage, leaning forward, and turns quickly (**screenshot 1**, compare with screenshot 4 of e231362).
  2. Stand right by it: the blaze turns fast; when blazes come out (`/difficulty normal`), the turn slows right after the spawn, then speeds up again.
  3. Walk more than 16 blocks away and look with the spyglass: the blaze stops turning. Walk back: it turns again.
  4. Break the spawner with the pickaxe: the blaze disappears with the cage.
  5. Quit and load the world again: the spawner still shows its blaze.
  6. A stronghold or mineshaft spawner shows an empty cage for now (silverfish and cave spider have no model yet).
- **Kit** (6 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:spyglass,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

### 2. Minecraft spawners give off flames and smoke like vanilla

Commit: 05d56a4b

- **Changed:**
  - `minecraft_terrain/src/ambient_particles.rs`: `Kind::Flame` (vanilla `FlameParticle`, a `RisingParticle`, checked with javap): the usual `Particle` start with zero motion, its velocity times `0.01f` plus the given motion, the position (not the previous one) jittered by `(nextFloat - nextFloat) * 0.05f` on each axis, friction 0.96, no gravity, lifetime `(int)(8 / (nextFloat * 0.8 + 0.2)) + 4` (12 to 44). It moves its box with no collision, so it passes into blocks. Size `quadSize * (1 - t^2 * 0.5)`; block light `LightCoordsUtil.addSmoothBlockEmission`: the cell's block light in sixteenths plus `(int)(t * 240)`, capped at full, so it glows to 15 as it ages. Opaque, sprite `particle/flame` (added to `textures()`). `kinds_at()` lists the particles' kinds and positions.
  - `minecraft_terrain/src/spawner_display.rs`: `SpawnerDisplays::new(seed)` has its own random; `tick` takes the level's `AmbientParticles` and, for a near spawner with a mob, adds `SMOKE` then `FLAME` at one point `pos + (nextDouble, nextDouble, nextDouble)` with zero motion, before the countdown and the spin (`BaseSpawner.clientTick`).
  - `minecraft_survival/src/entities.rs`: the displays get a seed from the world seed, as the ambient particles do, and tick into `self.ambient`.
  - `docs/MINECRAFT-SURVIVAL.md`: the flames and smoke next to the spawner mob; "spawner" leaves the Nether details limitation.
- **Tests:** `minecraft_terrain`: `ambient_particles.rs` `a_flame_hardly_moves_shrinks_and_glows_as_it_ages` (200 seeds: lifetime 12 to 44, the jitter within 0.05 with the previous position unjittered, less than 0.1 of drift over its life, lives `lifetime + 1` ticks, half the size and block light 15 at the end, and some flame started below a stone block rises into it; the quad is opaque and lit by its cell at age 0); `spawner_display.rs` `a_near_spawner_puts_one_smoke_and_one_flame_at_one_point_each_tick` (a silverfish spawner, which has no model, still smokes: two particles a tick, smoke then flame, at one point inside the block; none far away; none for an empty spawner).
- **Decision:** the brief said a flame rises "less than 0.05" over its life; the vanilla start velocity (up to about 0.28 up, times 0.01, with friction 0.96) allows up to about 0.07, so the test bounds it at 0.1.
- **Test steps:**
  1. At the fortress blaze spawner (entry 1, step 1): flames and dark smoke rise inside the cage around the turning blaze (**screenshot 1**). The flames are bright even in a dark corridor and shrink as they fade.
  2. Walk more than 16 blocks away: the flames stop with the turn. Walk back: they start again.
  3. Find a stronghold spawner (`/locate structure minecraft:stronghold`, the portal room): no mob shows (silverfish has no model yet) but flames and smoke rise in its cage (**screenshot 2**).
- **Kit** (6 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:spyglass,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

### 3. Minecraft spawn eggs change a spawner's mob like vanilla

Commit: 837c88cf

- **Changed:**
  - `minecraft_terrain/src/server.rs`: `Command::SpawnerEgg { pos, kind }` and `ServerHandle::spawner_egg`; the answer goes into `Output::summoned` like a hatched egg's, so `EggsWaiting` spends the egg only on `Ok`. `ServerSim::spawner_egg` is `SpawnEggItem.useOn` on a spawner: a monster in peaceful fails first with no change (the same rule `/summon` and hatched eggs use, our `EntityType.canSpawn`), then `spawner_blocks_work` off fails with `advMode.notEnabled.spawner`, then anything but a `minecraft:mob_spawner` block entity fails (trial spawners are left out), else `World::set_spawner_entity` sets `SpawnData.entity.id`, keeps the compound's other keys and marks the tag changed, so the client's display (entry 1) rebuilds with the tag's `Delay`.
  - `minecraft_survival/src/entities.rs`: `Entities::egg_on_spawner`, through `eggs_waiting` like `hatch_egg`.
  - `render_anim/src/minecraft_world.rs`: `EggUse::Spawner(pos)` sends the egg's kind (`spawn_egg::spawned_kind`) to `egg_on_spawner` instead of being ignored; the swing is unchanged (vanilla's client swings on any spawn egg use).
  - `docs/MINECRAFT-SURVIVAL.md`: the egg on a spawner under Items; the limitation now only says the refusal has no chat message.
- **Tests:** `minecraft_terrain` (`server.rs`, these skip without the game data like their neighbours): `an_egg_on_a_placed_spawner_sets_its_mob_and_keeps_it` (a placed spawner has no mob; a zombie egg sets it and reports the change, a ghast egg replaces it, no spawner one block up is an error; with a player 3 blocks away the spawner makes ghasts and still holds the ghast after its round, with `Delay` reset into 100..800, as a placed spawner's `SpawnPotentials` is empty), `an_egg_does_nothing_when_spawner_blocks_do_not_work` (`Err("advMode.notEnabled.spawner")`, no mob, no change), `a_monster_egg_does_nothing_to_a_spawner_in_peaceful` (a zombie egg fails with no change, a pig egg works, and through `ServerHandle` a creeper egg answers `Err` and a cow egg `Ok` with the cow in the block entity change).
- **Decisions:**
  - The brief's spawn round test used a zombie; at y 205 in daylight no zombie (or blaze, whose spawn rules also need darkness through `getWalkTargetValue`) can spawn, so the round is run with a ghast, which spawns in any light.
  - The refusal with `spawner_blocks_work` off is only logged (`Minecraft <egg> hatched nothing: advMode.notEnabled.spawner`); our egg answers have no chat path yet. Recorded as a limitation.
  - Trial spawners refuse eggs here; vanilla lets an egg set a trial spawner's mob, which needs the trial spawner simulation.
- **Test steps** (brief section 5, steps 1-3, 6, 7, 9):
  1. Use a new world `IW4L_MINECRAFT_WORLD=spawner8`, `/difficulty normal`. Place a spawner on the ground: an empty cage, no flames (**screenshot 1**).
  2. Use the zombie egg on it: a small zombie appears in the middle of the cage, leaning forward, and turns fast, with flames and smoke inside the cage (**screenshot 2**). In survival the egg is used up (4 -> 3).
  3. Place a row of spawners and give each a different egg: skeleton, spider, blaze, creeper, slime, enderman, ghast, wither skeleton, pig and husk (**screenshots 3-5**, several per screenshot). Each mob fits the cage: the tall enderman and the wide spider are shrunk, the ghast is tiny, the pig and slime keep the plain size. Spider legs may poke out a little; vanilla does the same.
  4. Silverfish and cave spider eggs: flames and smoke but no mob yet (the cave spider comes with the next commit).
  5. Stand near the zombie spawner at night or under a roof: when zombies come out, the turn slows right after the spawn, then speeds up again.
  6. Break a spawner with the pickaxe: the mob disappears with the cage. Quit and load again: every spawner still shows its mob.
  7. `/gamerule spawner_blocks_work false`, then use an egg on an empty spawner: nothing changes and the egg is kept. `/gamerule spawner_blocks_work true`.
  8. `/difficulty peaceful`, then use the creeper egg on an empty spawner: nothing changes and the egg is kept. The pig egg still works. The mobs already in the cages keep showing and turning. `/difficulty normal` afterwards.
- **Kit** (21 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:spawner*16,minecraft:zombie_spawn_egg*4,minecraft:skeleton_spawn_egg*4,minecraft:spider_spawn_egg*4,minecraft:blaze_spawn_egg*4,minecraft:creeper_spawn_egg*4,minecraft:slime_spawn_egg*4,minecraft:enderman_spawn_egg*4,minecraft:ghast_spawn_egg*4,minecraft:wither_skeleton_spawn_egg*4,minecraft:pig_spawn_egg*4,minecraft:husk_spawn_egg*4,minecraft:cave_spider_spawn_egg*4,minecraft:silverfish_spawn_egg*4,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:torch*64,minecraft:cobblestone*64,minecraft:cooked_beef*32,minecraft:spyglass
```

### 4. Minecraft cave spiders show inside mineshaft spawners

Commit: a4453997

- **Changed:**
  - `minecraft_terrain/src/spawner_display.rs`: a `minecraft:cave_spider` spawner builds its display from a copy of the compound with `id` set to `minecraft:spider`, the cave spider's box (0.7 x 0.5, so the display scale is 0.53125) and the model scale 0.7 multiplied in (`LayerDefinitions`' `CAVE_SPIDER` layer is the spider body at `MeshTransformer.scaling(0.7)`, which keeps the feet on the ground, so it equals a uniform 0.7 about the feet). After `append_spiders`, the new vertices whose UV lies in the `entity/spider/spider` region are moved to the same place in `entity/spider/cave_spider` (both skins are 64 x 32); the eyes layer keeps its UVs. The live cave spider stays unsimulated (`SIMULATED_KINDS` unchanged).
  - `minecraft_terrain/src/mesh.rs`: `minecraft:entity/spider/cave_spider` is in the atlas.
  - `docs/MINECRAFT-SURVIVAL.md`: the cave spider under the spawner mob; it leaves the "no model" limitation.
- **Tests:** `minecraft_terrain` (`spawner_display.rs`): `a_cave_spider_display_is_a_smaller_spider_with_its_own_skin` (an atlas with distinct spider, cave spider and eyes skins: the same vertex count as a spider display, each vertex at 0.98 of the spider display's distance from the feet, as 0.53125 x 0.7 against 0.53125 / 1.4, skin UVs moved into the cave spider region at the same relative place, eyes UVs unchanged).
- **Decision:** the brief's "the extent is 0.7 times the spider display's" compares unequal display scales: the spider is divided by its 1.4 width and the cave spider is not, so the cave spider display is 0.98 of the spider's, as in vanilla. The test checks that ratio.
- **Group 5 (trial spawners) is not done:** it needs a trial spawner config book read from the jar with a JSON-to-NBT conversion, a server pass in the vendored `spawning.rs`, two more particle kinds, the ambient sound and the per-state client spin. That is not small, so it is left for "Finish survival" with the rest of the trial spawner simulation (brief section 4, group 5).
- **Test steps** (brief section 5, steps 4 and 11):
  1. Use the cave spider egg on a placed spawner (kit of entry 3): a smaller, blue-grey spider turns in the cage (**screenshot 1**).
  2. In the Overworld, `/locate structure minecraft:mineshaft`, `/tp` there and find a cobweb room. Its spawner shows a cave spider (**screenshot 2**).
  3. The silverfish egg still gives only flames and smoke (no model).
- **Kit** (5 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:spawner*4,minecraft:cave_spider_spawn_egg*4,minecraft:silverfish_spawn_egg*4,minecraft:diamond_pickaxe+efficiency=5,minecraft:torch*64
```

## Item a1: end gateways

### 1. The Minecraft outer End islands generate a thousand blocks out like vanilla

Commit: ae01d7a4

- **Changed:** tests only, in `minecraft_terrain/src/terrain.rs`. They found no defect, so no vendored generator code was patched.
- **Tests** (`minecraft_terrain`, real data, skipped without the game data like their neighbours):
  - `the_outer_end_islands_generate_past_the_void`: an End stream of seed 77, chunks 40 to 80 along +x at z 0 and 1. Nothing stands between x 700 and 900 (for this seed the first block is at chunk 62, x 992), more than 10,000 end stone blocks lie past x 1000, the column biomes include `end_highlands` and `small_end_islands`, and chorus plants stand in highland cells.
  - `outer_end_gateways_keep_their_exact_exit`: chunk (2, -73) of seed 77 is highlands and holds one `end_gateway_return` gateway at (45, 67, -1160) with `exit_portal` [100, 50, 0] and `ExactTeleport` 1, in its bedrock frame (bedrock above and below at 2 and a plus at 1, the side ring and the corners open). The chunk was found once by decorating highland chunks nearest the origin in order (714 chunks, 69 s); the test loads only that chunk.
  - `end_island_height_matches_vanilla`: `sampler::end_island_height` for seed 0 at ten section coordinates (inside the 64-chunk ring, on its edge and outside) against 26.3's `EndIslandFunction.getHeightValue`.
- **How the vanilla values were made:** `jshell` from a JDK 25 (`~/.jdks/openjdk-25.0.2`, the 26.3 classes are version 69), with the 26.3 `client.jar` and the libraries of its `version.json` (downloaded from `libraries.minecraft.net` into the scratchpad, SHA-1 checked, not committed) on the class path:
  ```
  net.minecraft.SharedConstants.tryDetectVersion(); net.minecraft.server.Bootstrap.bootStrap();
  var r = new LegacyRandomSource(0L); r.consumeCount(17292); var n = new SimplexNoise(r, true);
  var m = Class.forName("net.minecraft.world.level.levelgen.densityfunction.generator.EndIslandFunction").getDeclaredMethod("getHeightValue", SimplexNoise.class, int.class, int.class); m.setAccessible(true);
  m.invoke(null, n, 256, 17)
  ```
  This printed -100.0, -100.0, -98.25235, -100.0, -100.0, -70.847885, -77.369675, -90.0, 55.0 and -69.49925 for the ten points in the test's order.
- **Generation time:** 82 outer chunks (with the neighbours their decoration needs) took 3.26 s in the debug test build, about 40 ms a chunk, while another test ran beside it. A first gateway trip loads at most about 33 chunks along the search line, so about 1.3 s in a debug build and less in release. **Decision:** the exit search stays synchronous on the main thread, with no loading screen held before it (brief section 6: only above about 2 s).
- **Decision:** the brief's 16 x 16 chunk patch from (64, -8) was meant to make the test hold on any seed; with the fixed seed 77 the strip along +x already holds the outer islands, both biomes and chorus, so the test keeps the strip (82 chunks, not 256).
- **Test steps:** none in game; the gateway trips come with entry 4.
- **Kit:** none needed.

### 2. Minecraft end gateways find their exit on the outer islands like vanilla

Commit: 91e26f03

- **Changed:**
  - New `minecraft_survival/src/the_end/gateway.rs` (`pub mod gateway;` in `the_end.rs`), pure and over a trait, so group 4 only has to call it:
    - `GatewayWorld`: `highest_filled_section(chunk)` (the bottom Y of the highest non-empty section, `None` for an empty chunk), `is_end_stone`, `full_block`, `is_bedrock`, `min_y`, `max_y`, with `chunk_is_empty` (`getHighestFilledSectionIndex() == -1`) and `highest_section_position` (`getHighestSectionPosition`) on top. `ChunkWorld` implements it: a missing chunk (not loaded, not saved, not generating) counts as empty, `full_block` is a collision shape of exactly one unit box (`isCollisionShapeFullBlock`).
    - `exit_xz_tentative` (`findExitPortalXZPosTentative`: the direction normalised with vanilla's 1e-5f cut-off, 1024 out, back 16 at most while the chunk is not empty, then forward 16 at most while it is, each `i-- > 0` after the emptiness test), `valid_spawn_in_chunk` (`findValidSpawnInChunk`: z, y, x order from y 30 to the highest section's top, end stone with no full block at +1 and +2, the strictly nearest block centre to 0, 0, 0), `tallest_block` (`findTallestBlock`), `find_exit_position`, `find_or_create_teleport_pos` (returns the spot and the island origin when one has to be built; the tallest-block step then sees the island through an overlay, as vanilla places it before that step), `new_exit` (the spot plus 10), `end_island_blocks` (`EndIslandFeature` with `LegacyRandom::new(BlockPos.asLong)`), `gateway_frame` (`EndGatewayFeature`'s 45 cells in its order), `GatewayExit` and `destination` (`exact ? exit : findExitPosition(exit)` at the bottom centre).
  - `minecraft_survival/src/portal_forcer.rs`: `ChunkWorld::vertical_range`.
- **Tests** (`minecraft_survival`, `the_end::gateway`):
  - `the_exit_search_starts_1024_out_along_the_gateway_s_direction`: ring gateway 6 at (-30, 75, 91) starts at (-320.6093751268596, 0, 972.5151045514741), chunk (-21, 60), and with only that chunk filled the walk checks (-21, 60), (-20, 59) twice and (-21, 60) again and ends on the same point; a gateway at 0, 0 stays at 0, 0.
  - `a_full_chunk_steps_back_and_an_empty_one_steps_forward_sixteen_times_at_most`: due east, all full ends at x 768, all empty at x 1280 (18 chunk checks each), filled chunks 60 to 66 at x 960, a lone filled chunk 70 at x 1120.
  - `the_spawn_spot_is_the_end_stone_nearest_the_origin_with_two_free_blocks_above`: a full block two above rules a spot out and a slab does not, y counts in the distance, y 29 is never looked at, a tie keeps the first in z, y, x order, a chunk whose highest section is 0 is walked from 15 to 30, a spot at y 200 is found, and stone alone gives none.
  - `no_spot_places_a_seeded_end_island_at_75`: islands at (-320, 75, 972), (1000, 75, 0), (5, 75, -1030) and (-321, 75, 973) have vanilla's block counts, layers, first and last blocks; a chunk with only stone in it builds the island at (-321, 75, 973) and lands on its first top block (-327, 75, 970), with the new gateway 10 above.
  - `the_tallest_block_skips_bedrock_and_the_middle_only_when_asked`, `a_loose_exit_lands_on_the_tallest_block_within_five_or_three_above_the_gateway` (a bare frame gives the exit + 3; end stone within 5 wins, one 6 away and one under the gateway do not; exact exits land on the exit), `the_gateway_frame_is_a_bedrock_plus_with_an_open_side_ring` (vanilla's order of the 12 bedrock cells).
  - Real data, `a_ring_gateway_finds_an_outer_island`: seed 77, ring gateway 0 at (96, 75, 0): the tentative point is x 992 (chunk 62 is the first non-empty one, as group 1 found), the spot is end stone at (999, 56, -8), no island is built, and it took 360 ms in the debug test build with generation on.
- **How the vanilla values were made:** `jshell` as in entry 1, calling the 26.3 classes themselves: `new Vec3(-30, 0, 91).normalize().scale(1024.0)` and the walked point `p0.add(dir.scale(-16)).add(dir.scale(16))` with `BlockPos.containing(p.x + 0.5, 75, p.z + 0.5)`; `new EndIslandFeature().place(level, null, RandomSource.create(pos.asLong()), pos)` and `EndGatewayFeature.knownExit(new BlockPos(1, 2, 3), false).place(level, null, RandomSource.create(), BlockPos.ZERO)`, where `level` is a `java.lang.reflect.Proxy` for `WorldGenLevel` that records every `setBlock*` call in order.
- **Checked with javap (26.3):** `TheEndGatewayBlockEntity.getPortalPosition`, `findExitPosition`, `findOrCreateValidTeleportPos` (the island is placed at `BlockPos.containing(p.x + 0.5, 75, p.z + 0.5)` with `RandomSource.create(pos.asLong())`, a `LegacyRandomSource`), `findExitPortalXZPosTentative`, `isChunkEmpty`, `findTallestBlock`, `getChunk`, `findValidSpawnInChunk`, `spawnGatewayPortal`; `Vec3.normalize`; `EndIslandFeature.place`; `RandomSource.create(long)`; `BlockPos.asLong`.
- **Decisions:**
  - The brief's `end_island_blocks` test compares against the vendored `EndIsland` feature; placing that needs a whole generation context, so the test compares against vanilla's own `EndIslandFeature` through `jshell` instead, which is the stronger check (the vendored feature was read and does the same loops).
  - The brief's example origin (-320, 75, 972) is not where a walked search builds the island for gateway 6: the walk lands on x -320.609 and `containing(x + 0.5)` is -321, so the search test uses (-321, 75, 973) and the island test covers both.
  - `full_block` is a collision shape of exactly one unit box; a shape made of several boxes that fill the cell would count in vanilla and not here (no such vanilla End block).
- **Test steps:** none in game yet; the trips come with entry 4.
- **Kit:** none needed.

### 3. Minecraft end gateways keep their age and cool down like vanilla

Commit: (see the next entry, or `git log` under this subject)

- **Changed:**
  - New `minecraft_terrain/src/end_gateway.rs` (`pub mod end_gateway;` in `lib.rs`):
    - `Gateway { age, cooldown, exit, exact }` read from a block entity tag (`Age` long, default 0; `exit_portal` int array kept only inside `Level.isInSpawnableBounds`: y within +-20,000,000 and x, z within +-30,000,000; `ExactTeleport` byte). Its tick is `portalTick`: remember spawning (`age < 200`) and cooling (`cooldown > 0`), `age++`, then count the cooldown down or, when not cooling, flash every `age % 2400 == 0` (`triggerCooldown`: 40 ticks); it reports a change on a flash and whenever spawning or cooling flipped (vanilla's `setChanged`).
    - `Gateways`, the server level's table by position: `sync(pos, tag)` adds a gateway it does not know, takes a known one's exit and exactness and keeps its age and cooldown, and drops a position whose tag is gone or not a gateway; `load_chunk` (pending tags, then block entities) and `unload_chunk`; `tick` returns the positions to mark with their age; `trigger_cooldown` (none while cooling); `set_exit`; `views()`.
    - `GatewayView { pos, age, cooldown, exit, exact }`, plain data for the client (and for track E's network messages).
    - `ClientGateways`, the main thread's view list with `mark_cooling`: a mark counts as cooling until a snapshot shows that gateway cooling, until a snapshot no longer has it, or for 40 client ticks, so a second player cannot go in while the trigger job is on its way.
    - `tag_at` and `write_age` (`Age` written into the level's block entity, which marks it changed only when the value differs).
  - `minecraft_terrain/src/server.rs`: a `gateways` field on `ServerSim`; hooks in `load_chunk`, `unload_chunk`, `sync_pois` (every block set since the last call is synced, which covers `/setblock`, breaking and the ring gateways a dragon kill places) and `collect_block_entity_changes` (every changed block entity is synced); `tick_gateways` (sync, then tick, writing `Age` for each change) runs in `run_tick` after `tick_end`, only on normal ticks; `trigger_gateway(pos) -> bool` and `set_gateway_exit(pos, exit, exact)` (the vendored `World::set_gateway_exit`, which writes vanilla's keys and marks the block entity) for jobs; `gateway_views()`; `EntitySnapshot.gateways`.
  - `minecraft_survival/src/entities.rs`: `Entities` keeps the level's `ClientGateways` from each snapshot, with `gateway(pos)`, `gateway_views()`, `mark_cooling(pos)` and `gateway_cooling(pos)`.
  - Vendored: `third_party/minecraftoss/world/src/level/container.rs`: `Level::set_block_entity_value` is `pub` (was `pub(super)`). The file was already patched by the fork (c5efeed0), so no new patched file.
- **Tests:**
  - `minecraft_terrain` `end_gateway`: `a_gateway_spawns_for_200_ticks_and_flashes_every_2400` (from age 0 the changes come at ages 200, 2400, 2440, 4800 and 4840; a gateway at 2399 flashes on its next tick and counts down after), `a_triggered_gateway_cools_for_40_ticks_and_ignores_a_second_trigger`, `exits_are_read_with_vanilla_keys_and_bounds`, `a_synced_gateway_keeps_its_age_and_cooldown_and_takes_the_new_exit`, `a_marked_gateway_counts_as_cooling_until_the_server_shows_it`.
  - `minecraft_terrain` `server` (skip without the game data): `age_is_saved_and_a_flip_reports_the_block_entity` (a chunk saved with `Age` 150 reports nothing for 49 ticks and the block entity with `Age` 200 on the 50th), `gateways_join_and_leave_with_their_chunks_and_setblock` (placed, broken, unloaded, loaded again at its saved age, sent again unchanged, and an `end_gateway_delayed` feature's gateway ticks the tick it was placed), `setting_an_exit_writes_vanilla_keys` (`exit_portal` int array, `ExactTeleport` only when exact, the current `Age`; a trigger reports the block entity, a second one and an unknown position are refused, the cooldown is not saved), `a_frozen_gateway_neither_ages_nor_cools_and_the_snapshot_shows_it` (through `run_tick` and `finish_batch`).
- **Decisions:**
  - `Age` is written into the level's block entity whenever the gateway is marked (a flip, a flash, a trigger, a new exit), not on every tick: nothing reads the level's copy between marks, and only marked block entities reach the chunk map and the save, so the saved `Age` is the one at the last mark. Vanilla saves the age a chunk has when it is next saved after a `setChanged`, so a reloaded gateway can be up to one flash (2400 ticks) younger here than there; its beam timing only depends on the 2400-tick rhythm and the first 200 ticks, which this keeps.
  - The trigger's block event (1, 0) is not sent: the client reads the cooldown from the views each snapshot (brief decision 7).
  - A frozen game neither ages nor cools a gateway (brief section 6, a5 merged first).
  - The full `minecraft_terrain` run had three load-sensitive eye of ender tests (`SEARCH_CAP` timing, from item a2) fail while another track was building; all 14 eye of ender tests pass when run on their own.
- **Test steps:** none in game yet (the gateway's state is not shown until entry 6's beam, and nothing teleports until entry 4).
- **Kit:** none needed.

### 4a. Minecraft players keep their speed when they arrive somewhere

Commit: the one after 4e512a51 (entry 3).

- **Changed:** `render_anim/src/minecraft_world.rs` only. `place_player` used to stop the player on every arrival, because `teleport_facing` clears the MW2 velocity. It now pushes the arrival's `motion` back as MW2 velocity right after the teleport. Two small helpers do the conversion: `player_motion` (MW2 velocity to blocks a tick, the conversion the saved `Place` always used) and `arrival_velocity` (its inverse: `[mx, -mz, my]` times 36 units a block times 20 ticks a second).
- **Tests** (`render_anim`, `minecraft_world::arrival_tests`): `an_arrival_gives_back_the_motion_a_departure_saved` (three velocities round-trip, and 0.5 east, 0.25 up, 0.1 south a tick is 360, -72, 180 units a second), `no_motion_leaves_the_player_still`.
- **Note for tracks B (Nether) and C (End):** this changes every arrival, not only gateways. A nether portal trip, an end portal trip into the End (whose arrival already kept the motion) and a join with a saved `Motion` now keep the player's speed, as vanilla does (`TeleportTransition` keeps the delta for portals, and `Motion` is read on load). A player standing still arrives standing still.
- **Test steps:**
  1. Walk into a nether portal while sprinting. You arrive moving and slide on a little instead of stopping dead.
  2. Glide with the elytra and a rocket, quit to the menu in the air and start again. You go on gliding at about the same speed.
- **Kit:** `minecraft:elytra`, `minecraft:firework_rocket*64`, `minecraft:obsidian*14`, `minecraft:flint_and_steel`.

### 4. Minecraft players travel through end gateways

Commit: the one after 41db58c7 (entry 4a).

- **Changed:**
  - `minecraft_survival/src/portal.rs`: `Portal.last_feet`; `touched_gateways(from, to, is_gateway)` sweeps the player's 0.6 x 1.8 box (deflated like the other portal touches) from last tick's feet to this tick's in steps of at most 0.25 and returns every `end_gateway` cell it overlaps, in the order they are passed (vanilla `checkInsideBlocks` along the movement, with the gateway's full-block inside shape); `sweep_start` drops a step longer than 8 blocks, so a teleport is never swept. `step` takes an `enter_gateway` callback: for each touched gateway it asks the level (not cooling: trigger and mark it) and, when the answer is yes, sets the player inside a `Gateway` portal. Spectators (vanilla `noPhysics`), the dead and remote players touch nothing. `last_feet` is kept until a tick runs (a frame shorter than a tick sweeps on the next one) and cleared without feet. The nether overlay and its sound stay nether only.
  - `minecraft_survival/src/the_end/gateway.rs`: `is_end_gateway`; `enter` (the gateway's cooldown from the server's view plus the main thread's marks; when it is free, the job `trigger_gateway(pos)` and `mark_cooling(pos)`); `known_exit` (the server's view, else the chunk map's block entity); `exit_states` and `create_exit` (search with generation on, the island if one is needed, then the 45-cell frame at the spot plus 10; written with `write_states` and a new block entity `{id, x, y, z, Age: 0L, exit_portal: [source]}`; one job then places the same cells where the server level has the chunk, gives the new gateway its exit there, and sets the source's exit with the source's own exactness); `arrival` (the player's place with the new feet: yaw, pitch, motion and fall distance kept, not on the ground); `travel`, returning `GatewayTrip { exit, feet, built }`.
  - `minecraft_terrain/src/terrain.rs`: `TerrainStream::write_states(scene, states, block_entities)`: loads each touched chunk into the chunk map first (`load_now`; a chunk only read from disk is not in the map and its edits would be dropped), sets the blocks and block entities, loads again so the edits are written into the chunk (later reads and the eviction save see them), and sets and marks only cells whose chunk is in the scene. It never goes through `record_edits`, which reads the scene and would write air into far chunks.
  - `render_anim/src/minecraft_portal.rs`: `levels` is `&mut`, the gateway callback goes to `gateway::enter`, and a due `Gateway` goes to the new `minecraft_end::gateway`, which logs `Minecraft end gateway at {pos} leads to {feet} (exit {exit}, built)` or `... leads nowhere`. `render_anim/src/minecraft_world.rs`: `place_player` clears `last_feet`.
- **Tests:**
  - `minecraft_survival` `portal`: `a_box_moving_through_a_gateway_between_ticks_touches_it` (a 7-block straight pass, a fast diagonal pass, the head reaching in, the 0.6 width, two gateways in passing order), `a_teleport_is_not_swept`; with the game data (skipped without it): `spectators_and_cooling_gateways_are_not_entered` (a cooling gateway is asked and refused; a free one sends the player at once with the player cooldown at 10; standing in it again triggers the gateway but only refreshes the player cooldown; spectators are never asked; the dead never go), `a_flight_through_a_gateway_between_ticks_enters_it_and_a_teleport_does_not`, `the_nether_overlay_ignores_gateways`.
  - `minecraft_survival` `the_end::gateway`, End stream of seed 77 (skipped without the game data): `a_first_trip_builds_one_return_gateway_pointing_back` (ring gateway 0: built at (999, 66, -8) in about 0.5 s in the debug test build, the player lands at (999.5, 57, -6.5) on end stone within 5 of it, keeping yaw, pitch and motion; the return gateway's block entity has `exit_portal` [96, 75, 0], `Age` 0 and no `ExactTeleport`; a job is queued; once the source holds the exit a second trip builds nothing and lands on the same spot, and the way back lands within 5 of the source), `an_exact_exit_lands_on_it_and_a_loose_one_on_the_tallest_block` (exit (100, 50, 0) exact lands on (100.5, 50, 0.5); loose (20, 50, 0) lands on the tallest non-bedrock block within 5 and not under the gateway; outside the End a gateway with no exit goes nowhere and builds nothing), `far_edits_survive_the_chunk_leaving_memory` (a gateway, its block entity and a glass block written into chunk (62, 0) of a saved world are still there after the chunk map dropped the chunk and loaded it again).
- **Decisions:**
  - Vanilla triggers a gateway's cooldown whenever a player who can use portals is inside it, even when the player's own cooldown stops the trip (`EndGatewayBlock.entityInside` calls `triggerCooldown` after `setAsInsidePortal`, checked with javap). Kept.
  - Vanilla does not reset the fall distance on a same-dimension teleport (`ServerPlayer.teleport` only calls `connection.teleport` and `resetPosition`, checked with javap), so the arrival keeps it, like the speed and the facing.
  - The second trip in the test reads the source's exit from the chunk map, written there as the server's block entity flush writes it in game; the test level has no server chunks to run the job against.
  - The box is the standing 0.6 x 1.8 box for every pose (as the other portal touches here); a prone or gliding MW2 player still overlaps the gateway cell when crawling or flying through the gap.
- **Test steps** (`.env`: the round 7 End test world with the dragon already killed, `IW4L_MINECRAFT_WORLD=endtest`):
  1. Bridge or fly to the gateway of the kill: a starfield cube in a bedrock frame at height 75, about 96 blocks from the centre. Go prone and crawl into its side gap. You land on end stone on an outer island well over 900 blocks out (a loading screen may show first), next to a new gateway in a bedrock frame floating about 10 blocks above the highest block. The log has `Minecraft end gateway at ... leads to ... (exit ..., built)`. **Screenshot 1.**
  2. Build up with end stone to the new gateway and crawl into its side gap. You come back next to the first gateway, on the highest end stone within 5 blocks, or on the frame's top bedrock if there is none. **Screenshot 2.**
  3. Put on the elytra, glide off the island's edge with a rocket and fly into the first gateway's side gap. You come out at the same outer island still gliding, at the same speed and facing the same way. **Screenshot 3.**
  4. Quit to the menu and start again. Use the first gateway again: you arrive at the same island under the same return gateway, no second one is built, and the log line has no `built`.
  5. On the main island, about 20 blocks east of 0, 0, type `/setblock ~2 ~ ~ minecraft:end_gateway` and walk into it (it has no frame): you go to the outer islands east of the centre, and a return gateway is built there.
- **Kit:** see the combined line at the end of item a1.

### 5. Minecraft ender pearls fly through end gateways

Commit: the one after a8171da7 (entry 4).

- **Changed:**
  - `minecraft_terrain/src/end_gateway.rs`: `pearl_gateway(from, to, enter)` sweeps the pearl's 0.25 box from where it was at the start of the tick to where it is now (or to its hit point) in steps of 0.125, and returns the first cell the `enter` callback takes.
  - `minecraft_terrain/src/server.rs`: in `tick_thrown`, after each pearl moves, the sweep asks the level's gateway table: a gateway that is not cooling triggers its 40-tick cooldown (its `Age` is written and the block entity marked after the loop) and takes the pearl, which is removed and reported as `(owner, gateway)` in the new `pearl_gateways` (`take_pearl_gateways`, `Output.pearl_gateways`). A cooling gateway lets the pearl fly on. A pearl that hits something before the gateway lands as before, as the sweep stops at the hit point. `minecraft_terrain/src/thrown.rs`: `Thrown::discard`.
  - `minecraft_survival/src/entities.rs`: `Entities.pearl_gateways`, collected for player 0's pearls like `pearl_landings` (and cleared with it when the player leaves).
  - `minecraft_survival/src/the_end/gateway.rs`: `resolve_exit` (shared with `travel`: the known exit, else a new one in the End), `collision_spans`, `pearl_drop` and `pearl_landing`, and `pearls_through(level)`, which turns each reported gateway into a landing in `pearl_landings`, so the existing landing code teleports the player, deals the 5 damage and plays the teleport sound. `render_anim`: `minecraft_end::pearls_through` runs it after the portal step and logs `Minecraft ender pearl through the end gateway at ... lands at ...` or `... is lost`.
- **How the landing is worked out:** the pearl arrives at the gateway's destination with no speed (vanilla `ThrownEnderpearl` gets zero velocity), then falls tick by tick like a thrown pearl (gravity 0.03, then times 0.99), and the first tick whose drop line meets a collision box in its column lands the thrower at where the pearl was at the start of that tick, as the server's own pearls do. A pearl that falls below the world's bottom minus 64 is lost, with no teleport (vanilla `onBelowWorld`).
- **Tests:**
  - `minecraft_terrain` `end_gateway`: `a_pearl_sweeps_its_small_box_along_its_flight` (a falling diagonal path, the quarter-high and quarter-wide box, the first gateway on the way and the cells asked in order).
  - `minecraft_terrain` `server` (skipped without the game data): `a_pearl_through_a_gateway_is_reported_and_triggers_its_cooldown`, `a_cooling_gateway_lets_a_pearl_through`, `a_pearl_that_hits_a_wall_first_lands_normally` (a gateway under the floor is never reached).
  - `minecraft_survival` `the_end::gateway`: `a_pearl_lands_on_the_first_block_below_the_exit_or_is_lost_in_the_void` (a full block, a slab, inside a block, the column edge and the void, with the landing height checked against the same fall worked out in the test); with the game data, `a_pearl_through_a_gateway_lands_its_thrower_below_the_exit` (an exact exit at (20, 120, 0) lands the thrower at x 20.5, z 0.5 within one fall step above the main island, through `pearl_landings`).
- **Decisions:**
  - Vanilla moves the pearl to the exit and lets it fall over later ticks; here the fall is worked out at once on the main thread (brief decision 5). The place is the same, and the server never has to tick a pearl in far chunks that are not loaded.
  - In vanilla the portal is handled in the same tick before the pearl's hit (`ThrowableProjectile.tick` moves, applies block effects, runs `Entity.tick` with `handlePortal`, then `hitTargetOrDeflectSelf`; checked with javap), so a gateway on the way to the hit point wins, as here.
  - Landings still go to player 0 only, like the other pearl landings (`(owner, gateway)` is reported so track E can map owners to players).
  - The landing teleports the player straight there like any pearl; the voxel world holds a player whose chunk is still missing until it streams in, so a far landing waits for the ground instead of falling.
- **Test steps:**
  1. From a few blocks away, throw an ender pearl into the side gap of the gateway of the dragon kill. The gateway flashes a short purple beam (with entry 6), and you land on an outer island well over 900 blocks out, under a new gateway (built if this is the first trip), taking the pearl's damage. The log has `Minecraft ender pearl through the end gateway at ... lands at ...`. **Screenshot 1.**
  2. Right after a trip, while the gateway you came out of still cools down (2 seconds), throw a pearl through it. The pearl flies straight through and lands behind it like a normal pearl.
- **Test runs:** `minecraft_survival` 400 passed; `minecraft_terrain` 908 passed and 3 ignored on the full run, with the same three load-sensitive eye of ender timing tests as in entry 3 failing while another build ran; all 22 eye tests pass on their own.
- **Kit:** see the combined line at the end of item a1.

### 6. Minecraft end gateways shine their beam and give off portal particles

Commit: the one after de5bcda4 (entry 5).

- **Changed:**
  - New `minecraft_terrain/src/end_gateway_render.rs` (`pub mod end_gateway_render;` in `lib.rs`): `beam(view, max_y, game_time, partial)` is `TheEndGatewayRenderer.extractRenderState`: only while spawning or cooling, `scale = Mth.sin(pi * percent)` with `getSpawnPercent` (`(age + p) / 200`) or `getCooldownPercent` (`1 - (cooldown - p) / 40`), height `floor(scale * (spawning ? maxY : 50))`, magenta (13061821) while spawning, else purple (8991416), and the animation time `floorMod(gameTime, 40) + p`. `append_beam` is `BeaconRenderer.submitBeaconBeam` with the gateway's arguments: centred on the block, from `y - height` to `y + height`; the opaque core (radius 0.15, corners (0, r), (r, 0), (-r, 0), (0, -r), turned by `animationTime * 2.25 - 45` degrees, V from `-1 + frac(t * 0.2 - floor(t * 0.1))` with `t = -animationTime`, over `2 * height * scale * (0.5 / 0.15)`) goes to the entity models, the glow (an unturned square of 0.175, alpha 32, V over `2 * height * scale`) to the translucent mesh, both full bright, with vanilla's four quads per part and their U and V corners. V is split at the texture's repeats with `end_crystal_render::beam_spans`, as the atlas does not wrap. `append_beams` skips gateways more than 256 blocks from the camera (`getViewDistance`).
  - `minecraft_terrain/src/mesh.rs`: `minecraft:entity/end_portal/end_gateway_beam` is an entity texture. `minecraft_survival/src/entities.rs`: `meshes` draws the beams of the level's gateway views, with the scene's top block as `maxY` and the world's game time.
  - `minecraft_terrain/src/display_ticks.rs`: an `end_gateway` arm (`EndGatewayBlock.animateTick`): one `portal` particle per face that a solid neighbour does not hide (`getParticleAmount`), each at a random point of the block with velocity `(nextDouble - 0.5) * 0.5` on each axis, then moved to 0.25 off the centre on x or z (`nextInt(2)`, `nextBoolean`) with `nextFloat * 2` outward, in vanilla's draw order (checked with javap: the velocities are doubles, not floats as for nether portals).
- **Tests:**
  - `minecraft_terrain` `end_gateway_render`: `the_spawn_beam_rises_to_the_build_height_and_falls_over_ten_seconds` (heights 0, 255 and 4 at ages 0, 100 and 199, the partial tick counted, magenta, none from age 200), `the_cooldown_beam_is_purple_and_fifty_high_at_its_peak` (50 at cooldown 20, 0 at 40, spawning wins over cooling), `the_beam_scrolls_and_turns_with_game_time`, `the_beam_is_an_opaque_turned_core_inside_a_faint_glow` (heights, radii, alpha, full bright, V inside the texture, nothing at height 0).
  - `minecraft_terrain` `display_ticks`: `a_gateway_in_its_frame_gives_four_particles_and_a_lone_one_six`.
- **Decisions:**
  - The beam uses the server's age and cooldown from each snapshot with the frame's partial tick; vanilla's client ages its own copy and learns of a trigger through block event 1, which the views replace (brief decision 7).
  - A face counts as hidden when its neighbour is solid to render (the scene's `solid_render_at`), the rule the mesher's culling of the cube follows. In a frame, bedrock above and below hides two faces.
- **Test steps:**
  1. Stand a few blocks from the gateway of the dragon kill: purple portal particles drift from its four open sides.
  2. Go through it (entry 4) and look back at the gateway you came out of: a short purple beam rises and falls through it for about 2 seconds. The gateway you entered flashes the same way.
  3. On the main island, about 20 blocks east of 0, 0, type `/setblock ~2 ~ ~ minecraft:end_gateway`. A tall magenta beam grows up to the sky and shrinks again over 10 seconds, and six faces' worth of particles come off it. **Screenshot 4.**
  4. Wait 2 minutes near a gateway: it flashes its purple beam by itself.
- **Test runs:** `minecraft_survival` 400, `render_anim` 75, and `minecraft_terrain` 902 passed with 3 ignored, run with the 14 load-sensitive eye of ender tests skipped (they pass on their own, entry 5).
- **Kit:** see the combined line at the end of item a1.

### 7. Docs: Minecraft end gateways

Commit: the one after 7aae6815 (entry 6).

- **Changed:** `docs/MINECRAFT-SURVIVAL.md`: an end gateway entry under The End (travel, the exit search and return gateway, exact and loose exits, pearls, the cooldown, the flash, the beam and particles, `Age` saved); the X6 line keeps only item frames, shulkers, levitation and the sky flashes; a new "End gateways" limitation (only players and pearls go through, the pearl landing worked out at once and for the host only, the standing box in every pose, the first trip's generation on the main thread, no advancement).
- **Tests:** none (docs only).
- **Item a1, the owner's test** (`.env`: the round 7 End test world with the dragon already killed, `IW4L_MINECRAFT_WORLD=endtest`, as in screenshot 1 of e231362):
  1. Fly or bridge to the gateway of the kill: a starfield cube in a bedrock frame at height 75, about 96 blocks from the centre. Purple portal particles drift from its open sides. From a few blocks away, throw a pearl into its side gap. The gateway flashes a short purple beam (about 2 s); you land on end stone on an outer island well over 900 blocks from the centre (the ground may take a moment to stream in); a new gateway in a bedrock frame floats 10 blocks above the highest block nearby, flashing purple; the log has `Minecraft ender pearl through the end gateway at ... lands at ...`. **Screenshot 1.**
  2. Build up with end stone next to the new gateway and go prone in its side gap (crawl in). You come back to the main island next to the first gateway, standing on the highest end stone within 5 blocks, or on the frame's top bedrock if there is none; the log has `Minecraft end gateway at ... leads to ...`. **Screenshot 2.**
  3. Put on the elytra, glide off the island's edge with a rocket, and fly into the first gateway's side gap. You come out at the same outer island still gliding, at the same speed and facing the same way. **Screenshot 3.**
  4. Right after a trip, while the purple beam still shows, throw a pearl through the gateway you came out of. The pearl flies straight through without teleporting you through the gateway, and lands behind it as a normal pearl.
  5. On the main island, about 20 blocks east of 0, 0, type `/setblock ~2 ~ ~ minecraft:end_gateway`. A gateway appears with a tall magenta beam that grows up to the sky and shrinks again over 10 seconds. **Screenshot 4.** Walk into it (it has no frame): you go to the outer islands east of the centre, and a return gateway is built there (the log line ends in `, built`).
  6. Wait 2 minutes near a gateway: it flashes its purple beam by itself.
  7. Quit to the menu and start again. Use the first gateway again: you arrive at the same island under the same return gateway, and no second return gateway is built (no `, built` in the log).
  8. Optional, with vanilla 26.3 Java: make a world with the same seed (`/seed`), `/setblock` a gateway at the same block as in step 5, and walk in. Your position (F3) matches the `leads to` coordinates the log printed for step 5, to the block.
- **Kit for item a1** (9 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:ender_pearl*16,minecraft:ender_pearl*16,minecraft:elytra,minecraft:firework_rocket*64,minecraft:end_stone*64,minecraft:cobblestone*64,minecraft:diamond_pickaxe+efficiency=5,minecraft:golden_apple*16,minecraft:diamond_boots+feather_falling=4
```

## Adversarial review of the whole track

Base `0bcbfb17`, reviewed up to `1f6cbfe2` (entry a1-7). Checked against 26.3 with javap: the haste and mining fatigue attack speed modifiers (`ADD_MULTIPLIED_TOTAL`, 0.1 a level), `Player.getDestroySpeed` (mining fatigue is `0.3^(amplifier+1)` in 26.3), the dig loop in `MultiPlayerGameMode` and `ServerPlayerGameMode.tick` (hit sound on the start and every 4th tick through level event 2020, chips through 2019), `SpawnEggItem.useOn` and the spawner renderer's spin times 10. They match. No comments in added code, the new files pass the rustfmt check, and no Mojang files are committed.

### R1. Minecraft eye of ender tests no longer time out while the machine is busy

Commit: the one after 1f6cbfe2 (entry a1-7).

- **Defect:** four tests let the timed stronghold search resolve all 128 ring slots on the spot: `an_overworld_eye_finds_a_stronghold_ring_chunk_corner`, `the_eye_search_and_locate_agree` and `a_thrown_eye_goes_through_the_server_thread_and_flies_toward_the_stronghold` (`minecraft_terrain`), and `an_eye_thrown_from_the_hand_leaves_the_stack_and_flies` (`minecraft_survival`). That takes about 3.6 s in the debug build and more than `SEARCH_CAP` (20 s) under load, so the search answered "timed out" and the tests failed, as in entries a1-3 and a1-5. The full suite after the merge would fail the same way. The last test waits up to 300 s for an answer before failing.
- **Fix:** each of these tests now resolves the rings with `locate::resolve_rings` before the timed search. In the game, `start_level` does the same on the "Stronghold rings" thread. The ring work itself stays covered by `locate.rs`'s `rings_are_worked_out_once_and_agree_with_the_placement_check` and `server.rs`'s `the_stronghold_rings_are_worked_out_on_their_own_thread`.
- **Tests:** the 14 eye of ender tests in `minecraft_terrain` pass in 2.95 s, and `an_eye_thrown_from_the_hand_leaves_the_stack_and_flies` passes in 2.98 s.
- **Test steps:** none in game (tests only).
- **Kit:** none.
