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

Commit: (see the next entry, or `git log` under this subject)

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
