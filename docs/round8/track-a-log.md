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

Commit: (see the next entry)

- **Changed:** new `minecraft_survival/src/tick_rate.rs`, registered in `lib.rs`. Pure logic, not wired into the game yet (group 2b does that).
  - `TickRate` holds vanilla's `ServerTickRateManager` state (rate, nanoseconds per tick, frozen, steps, sprint), `runServer`'s timing (`next_tick`, `last_overload_warning`) and the 100-entry tick-time ring.
  - `plan(now, ready)` runs `runServer`'s loop for every tick due at `now`: the sprint check, or the overload check (more than 1 s plus 20 ticks behind, and at least 10 s plus 100 ticks since the last warning), which logs `Can't keep up! Is the server overloaded? Running {ms}ms or {ticks} ticks behind` at WARN and skips those ticks. Then it moves `next_tick` on, runs `TickRateManager.tick` and records whether the tick runs normally. The first call starts the schedule at `now`, and `ready = false` plans nothing, so the time behind keeps growing while a level is still busy.
  - A batch holds at most `max_batch()` ticks: as many as fit in 50 ms at the mean tick time of the last recorded batch (1 to 100).
  - `record(nanos)` fills the ring like `tickServer` and adds to the sprint time while sprinting; `average_nanos()` is vanilla's `getAverageTickTimeNanos`.
  - `TickBatch { normal, overload }` with `len`, `is_empty` and `normal_count`.
- **Tests** (`tick_rate.rs`): `calls_50_ms_apart_give_one_tick_each`, `a_one_second_backlog_is_caught_up_in_batches_without_a_warning` (5, 5, 5, 5 at 10 ms a tick), `the_batch_limit_follows_the_last_batch_not_the_average`, `three_seconds_behind_warns_once_and_skips` (warns `(2950, 59)`, and a second 3 s gap right after catches up 60 ticks without a warning), `a_level_still_busy_plans_nothing_and_falls_behind`, `the_first_plan_starts_the_schedule`, `the_average_covers_the_last_100_ticks`, `a_batch_counts_its_normal_ticks`.
- **Decision:** the ring and the average count the ticks recorded, not the ticks planned, so ticks still in flight don't lower the average. The sprint and step fields are there already because `plan` follows vanilla's sprint branch; the commands that set them come with `/tick` (group 5b).
- **Test steps:** none on their own; nothing calls it yet.
