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

Commit: (see the next entry, or `git log` under this subject)

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
