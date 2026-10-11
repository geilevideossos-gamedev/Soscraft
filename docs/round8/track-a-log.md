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

Commit: (see the next entry, or `git log` under this subject)

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
