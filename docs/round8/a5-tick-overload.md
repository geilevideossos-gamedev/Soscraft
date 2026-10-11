# Track A, item a5: surviving overload, and `/tick` (design brief)

Paths and lines are as of e2313625. Vanilla facts were checked with javap on the 26.3 `client.jar` (`MinecraftServer`, `ServerTickRateManager`, `TickRateManager`, `TickCommand`, `ServerLevel`, `ServerChunkCache`, `CommandSourceStack`, `Minecraft`), and the messages with the jar's `assets/minecraft/lang/en_us.json`. No code was copied.

## 1. The owner's request

> Screenshot 5 (`scrennshots/screenshots-e231362/screenshots-e231362/5_random_tick_und_tick_befehl.png`): I set `random_tick_speed` very high (the chat shows 20000), then back to 20 and 3, and after that nothing moved anymore; items hung in the air. The server must survive overload like vanilla: skip ticks when it falls behind, log "Can't keep up! Is the server overloaded? Running Xms or Y ticks behind", and recover as soon as the load drops. Also add `/tick` (query, rate, freeze, unfreeze, step, sprint) with vanilla `ServerTickRateManager`/`TickCommand` semantics and messages.

The screenshot also shows `/tick rate 20` answered with `Unknown or incomplete command`, so `/tick` doesn't exist yet.

## 2. What exists today

### 2.1 The root cause of the hang

There are three faults, and the first two together cause the freeze:

1. **The client sends ticks blind.** `Entities::tick` (`crates/minecraft_survival/src/entities.rs:814-905`) adds the frame's `dt` to its own clock and sends one `Command::Tick` each time 50 ms have passed (`:825-827`, `:870-904`). It never checks whether the level thread has finished the last tick. The channel is an unbounded `std::sync::mpsc` (`crates/minecraft_terrain/src/server.rs:2432-2433`).
   - When one tick takes longer than 50 ms, ticks pile up in the queue without limit. With `random_tick_speed 20000`, `tick_chunks` (`third_party/minecraftoss/world/src/level/random_tick.rs:103-150`) rolls 20000 random positions in every randomly ticking section of every ticking chunk, which is about a second or more per tick.
   - Each queued tick also carries a boxed clone of the inventory (`TickInput.pickup`, `server.rs:2321`), so the queue leaks memory as it grows.
2. **The level thread never answers while ticks keep coming.** `server_loop` (`server.rs:2673-2852`) handles "everything already queued" before it sends one `Output` (`:2677-2679`): `next.take().or_else(|| commands.try_recv().ok())`. When the client adds a tick faster than the thread finishes one, that inner loop never ends.
   - So no `Output` is ever sent again. Client mobs, item entities, falling blocks and orbs keep their last snapshot, and client drops handed to the server (`Entities.server_handed`) stay where they were spawned. That is the "items hang in the air".
3. **Commands queue behind the backlog.** `/gamerule` runs on the main thread (`crates/minecraft_survival/src/commands.rs:977-1045`), and `entities.set_game_rules` (`:1019-1022`, `entities.rs:277-280`) sends `Command::GameRules` into the same FIFO.
   - The new `random_tick_speed` 20 or 3 only arrives after every queued 20000-speed tick has run. The backlog grows faster than that, so it never arrives.
   - Vanilla runs every queued task between ticks (`MinecraftServer.waitUntilNextTick` calls `runAllTasks` first), so there a rule change takes effect before the next tick.

There is also a second hang path. `Entities::finish_block_entity_changes` (`entities.rs:629-640`) calls `ServerHandle::wait_idle` (`server.rs:2646-2653`), which blocks the main thread until every command sent so far has been handled. This runs when a level closes (`minecraft_survival/src/server.rs:92`, `:534`) and when the world closes (`render_anim/src/minecraft_world.rs:2752`). With the backlog, closing the world would hang for as long as the backlog takes to drain.

### 2.2 Where the tick rate is assumed to be 20

| Place | File:line | Owner of the state | After this item |
|---|---|---|---|
| The server tick cadence (client clock sends ticks) | `minecraft_survival/src/entities.rs:31`, `:825-827` | level (server) | the server schedule in `Globals` (section 4, groups 2a and 2b) |
| The level thread's entity autosave every 6000 ticks | `minecraft_terrain/src/server.rs:2752-2757` | level | unchanged; counts every tick of a batch |
| World autosave every 6000 counted ticks | `minecraft_survival/src/save.rs:20`, `render_anim/src/minecraft_world.rs:606-609`, `:1716` | server | vanilla interval `max(100, rate x 300)` ticks, as a countdown |
| Overworld and End clocks advance by frame time x 20 | `minecraft_survival/src/server.rs:110-121` (`seconds * 20.0` at `:115`), `minecraft_terrain/src/day_cycle.rs:83-87`, called at `minecraft_world.rs:678` and `:2213` | server | by the server's normal (not frozen) ticks |
| Weather while no weather level ticks | `minecraft_survival/src/server.rs:72-84`, fed `mob_ticks` at `minecraft_world.rs:1718` | server | by the server's normal ticks |
| Empty-level timer (300 ticks) | `minecraft_survival/src/server.rs:489-520`, fed `level_ticks` at `minecraft_world.rs:1717`, `:2778` | level | by all server ticks, frozen ones too (vanilla `emptyTime` is not gated) |
| Containers: furnaces, brewing, campfire cooking | `minecraft_survival/src/level.rs:211-232`, fed `hand_ticks` at `minecraft_world.rs:986` | level (block entities) | by the server's normal ticks |
| The dragon fight (`EnderDragonFight.tick`) | `minecraft_survival/src/the_end/dragon_fight.rs:27`, `:345-350`, `:528-534`; called with frame time at `render_anim/src/minecraft_end.rs:63-80` from `minecraft_world.rs:716` | level | by the server's normal ticks (call site only) |
| Client tick: mob interpolation, particles, lids, display ticks, rain | `entities.rs:825-866`, `:1196`; `minecraft_world.rs:1719-1727`, `:2161` | client (per viewer) | vanilla client timing: `max(50 ms, ms per tick)` while running, 50 ms and held still while frozen |
| Player: hunger, air, hazards, item cooldowns, attack strength | `minecraft_world.rs:23`, `:922-978` | player | stays at 20 Hz (decision 7) |
| Player: portal timer, elytra, mining progress, swing | `minecraft_survival/src/portal.rs:12`, `elytra.rs:10`, `render_anim/src/minecraft_mining.rs:24`, `minecraft_world.rs:912`, `render_anim/src/minecraft_elytra.rs:55` | player | stays at 20 Hz |
| Sky and fog environment tick, block animation clock | `minecraft_world.rs:2225-2236`, `:745` | client | stays at 20 Hz |
| Loading-screen clock | `render_anim/src/minecraft_loading.rs:100` | UI | stays |

### 2.3 Other pieces this item builds on

| Piece | File:line |
|---|---|
| `Command` enum (`Tick(Box<TickInput>)` at `:2225`) | `minecraft_terrain/src/server.rs:2189-2240` |
| `TickInput` | `server.rs:2314-2341` |
| `Output`, which already has `tick_phases` with the tick's total ms | `server.rs:2356-2406` |
| `ServerHandle` (`sent`, `poll`, `wait_idle`) | `server.rs:2410-2664` |
| A tick on the level thread: `sim.tick()` and then mobs, projectiles, the End, pickups and the snapshot | `server.rs:2749-2839` |
| `Level::tick` (block and fluid ticks, random ticks, block events, entities, block entities) | `third_party/minecraftoss/world/src/level/mod.rs:777-819` |
| `Globals` (server-wide state) | `minecraft_survival/src/server.rs:34-53` |
| `Entities::poll_jobs` and `take_outputs`, which stash outputs in `unread` | `entities.rs:656-668` |
| The command tree (`/time` and `/weather` as models) | `minecraft_commands/src/commands.rs:91-176`, `:458-497` |
| `ArgKind`, `Arg` and `time()` (units `t`, `s`, `d` and the minimum check) | `minecraft_commands/src/args.rs:28-48`, `:87-111`, `:236-258` |
| `StringReader::read_float` and `read_bounded_int` (the model for float bounds) | `minecraft_commands/src/reader.rs:142`, `:197-215` |
| Suggestions per argument kind | `minecraft_commands/src/suggest.rs:383-447`, `:474-479` |
| The `CommandWorld` trait and `FakeWorld`, the test world | `minecraft_commands/src/server.rs:87-176`, `:602-634` |
| `CommandServer::feedback` (ops broadcast) | `minecraft_commands/src/server.rs:425-450` |
| `Host`, the game's `CommandWorld` | `minecraft_survival/src/commands.rs:37-55`, `:242` |
| Chat splits on `\n` (needed for `commands.tick.query.rate.*`) | `crates/frame/src/minecraft_gui.rs:69-80` |
| Log file | `iw4l-artifacts\logs\latest.log` (`crates/diag/src/lib.rs:98`, `:139`) |

## 3. Vanilla 26.3 behaviour (javap)

### 3.1 The server loop (`MinecraftServer.runServer`, `tickServer`, `waitUntilNextTick`)

- At start, `nextTickTimeNanos = now`.
- Each pass of the loop:
  1. **Sprint:** if `tickRateManager.isSprinting()` and `checkShouldSprintThisTick()`, then `nanosPerTick = 0`, `nextTickTimeNanos = now` and `lastOverloadWarningNanos = now`.
  2. **Otherwise, the overload check:** `nanosPerTick = tickRateManager.nanosecondsPerTick()` and `behind = now - nextTickTimeNanos`. The server counts as overloaded when `behind > OVERLOADED_THRESHOLD_NANOS + 20 * nanosPerTick`, where `OVERLOADED_THRESHOLD_NANOS = 20 * 1e9 / 20 = 1 s` (so 2 s at 20 TPS).
     - It warns when also `nextTickTimeNanos - lastOverloadWarningNanos >= OVERLOADED_WARNING_INTERVAL_NANOS + 100 * nanosPerTick`, where the interval is `10 s` (so 15 s at 20 TPS).
     - The warning: `ticks = behind / nanosPerTick`, log WARN `Can't keep up! Is the server overloaded? Running {behind / 1_000_000}ms or {ticks} ticks behind`, then **skip:** `nextTickTimeNanos += ticks * nanosPerTick` and `lastOverloadWarningNanos = nextTickTimeNanos`.
     - **Ticks are only skipped together with a warning.** Between warnings, a server that is behind runs its ticks back to back to catch up.
  3. `nextTickTimeNanos += nanosPerTick`, then the tick (`processPacketsAndTick`, `tickServer`).
  4. `waitUntilNextTick`: `runAllTasks()` first (every queued packet and command runs between two ticks, even when behind), then wait while `haveTime()` (`now < nextTickTimeNanos`). While sprinting, `endTickWork` adds the time since `sprintTickStartTime` to `sprintTimeSpend`.
- **`tickServer`:**
  - `tickCount++`, then `tickRateManager.tick()`, then `tickChildren`, then the autosave countdown (`ticksUntilAutosave--`, and `autoSave()` at 0).
  - Then the tick time `t = now - start` goes into the ring `tickTimesNanos[tickCount % 100]` (100 entries), and `aggregatedTickTimesNanos` is updated.
  - `getAverageTickTimeNanos = aggregated / min(100, max(tickCount, 1))`.
- **Autosave interval:** `computeNextAutosaveInterval = max(100, (int)(rate * 300))` ticks, where `rate` is the tick rate, or `1e9 / (averageTickTime + 1)` while sprinting. That is 5 minutes of real time. `onTickRateChanged` lowers `ticksUntilAutosave` to the new interval when the new one is smaller.
- **`tickChildren`:** clocks (`ServerClockManager.tick`) only `if runsNormally()`; then every level ticks; then connections, players and the player list run, frozen or not.

### 3.2 A frozen level (`ServerLevel.tick`, `ServerChunkCache.tick`, `Level.tickBlockEntities`)

When `runsNormally()` is false:
- No world border, no weather cycle (`advanceWeatherCycle`) and no `tickTime`, so game time and day time stand still.
- No block ticks and no fluid ticks (`LevelTicks.tick` with 65536), no raids, no block events (`runBlockEvents`), and no dragon fight tick.
- `ServerChunkCache.tickChunks` skips its inner tick, so there is no natural spawning and no random ticks.
- Entities: `TickRateManager.isEntityFrozen(e)` is `!runsNormally() && !(e instanceof Player) && e.countPlayerPassengers() == 0`. Frozen entities skip their whole tick, `checkDespawn` included. Players still tick, so they still touch and pick up items and orbs.
- Block entities don't tick.

### 3.3 `TickRateManager` and `ServerTickRateManager`

- Defaults: `tickrate = 20.0`, `nanosecondsPerTick = 1e9 / 20`, `frozenTicksToRun = 0`, `runGameElements = true`, `isFrozen = false`. `MIN_TICKRATE = 1.0`.
- `setTickRate(r)`:
  - `tickrate = max(r, 1)` and `nanosecondsPerTick = (long)(1e9 / tickrate)`;
  - then the server's `onTickRateChanged` and a `ClientboundTickingStatePacket` to every client.
- `tick()`:
  - `runGameElements = !isFrozen || frozenTicksToRun > 0`;
  - then, if `frozenTicksToRun > 0`, it drops by 1.
- `runsNormally() = runGameElements`. `isSteppingForward() = frozenTicksToRun > 0`.
- `stepGameIfPaused(n)`: returns false unless frozen; otherwise `frozenTicksToRun = n` and a `ClientboundTickingStepPacket` is sent.
- `stopStepping()`: if `frozenTicksToRun > 0`, it is set to 0 and the call returns true; otherwise false.
- `isSprinting() = scheduledCurrentSprintTicks > 0`.
- `requestGameToSprint(n)`:
  - `wasSprinting = remainingSprintTicks > 0`;
  - `sprintTimeSpend = 0`, `scheduledCurrentSprintTicks = n`, `remainingSprintTicks = n`;
  - `previousIsFrozen = isFrozen()`, then `setFrozen(false)`;
  - returns `wasSprinting`.
- `checkShouldSprintThisTick()`:
  - false if `!runGameElements`;
  - if `remainingSprintTicks > 0`: `sprintTickStartTime = now`, `remainingSprintTicks--`, true;
  - otherwise `finishTickSprint()`, false.
- `stopSprinting()`: if `remainingSprintTicks > 0` it runs `finishTickSprint()` and returns true; otherwise false.
- `finishTickSprint()`:
  - `done = scheduled - remaining` and `ms = max(1, sprintTimeSpend) / 1e6` (as a double);
  - `tps = (int)(1000 * done / ms)`, and the ms per tick string is `%.2f` of `done == 0 ? millisecondsPerTick() : ms / done`;
  - it resets `scheduled` and `sprintTimeSpend` to 0;
  - then the **server's command source** sends `commands.tick.sprint.report` with `allowLogging = true`;
  - finally `remainingSprintTicks = 0`, `setFrozen(previousIsFrozen)` and `onTickRateChanged()`.
- **The server source:** `createCommandSourceStack` has the display name `Server`. `sendSuccess` logs the line to the console and, with `allowLogging`, `broadcastToAdmins` sends `chat.type.admin` (`[%s: %s]`, grey italic) to every op player when `send_command_feedback` is on. For a non-server source, `log_admin_commands` gates the console copy.
- The tick state is not saved.

### 3.4 `TickCommand`

- `tick` requires `LEVEL_ADMINS`, permission level 3.
- **`query`**:
  - Sprinting: `commands.tick.status.sprinting`, then `commands.tick.query.rate.sprinting` (rate, average ms).
  - Otherwise, the status line first: `commands.tick.status.frozen` if frozen, else `commands.tick.status.lagging` if `nanosecondsPerTick < averageTickTimeNanos`, else `commands.tick.status.running`. Then `commands.tick.query.rate.running` (rate, average ms, target ms).
  - Then `commands.tick.query.percentiles` (P50, P95, P99, sample size): sort a copy of the whole 100-entry ring, zeros included, and read `[len / 2]`, `[(int)(len * 0.95)]` and `[(int)(len * 0.99)]`. The sample size is always 100.
  - The rate is `%.1f`. Times are `%.1f` of `nanos / 1e6` (`nanosToMilisString`). It returns `(int)rate`. None of these lines are broadcast.
- **`rate <rate>`**: a `FloatArgumentType.floatArg(1, 10000)` that suggests `20` (`DEFAULT_TICKRATE`). It sends `commands.tick.rate.success` (`%.1f`), broadcast, and returns `(int)rate`.
- **`step`** and **`step <time>`**: `TimeArgument.time(1)` (units `t`, `s`, `d`; at least 1 tick), suggesting `1t` and `1s`; the bare form steps 1 tick. It fails with `commands.tick.step.fail` when not frozen. On success it sends `commands.tick.step.success` with the tick count, broadcast, and returns 1.
- **`step stop`**: fails with `commands.tick.step.stop.fail` if not stepping, otherwise sends `commands.tick.step.stop.success` (broadcast) and returns 1.
- **`sprint <time>`**: `TimeArgument.time(1)`, suggesting `60s`, `1d` and `3d`. If a sprint was already running (`requestGameToSprint` returned true), it first sends `commands.tick.sprint.stop.success`. Then it sends `commands.tick.status.sprinting`, both broadcast, and returns 1. The interrupted sprint sends no report.
- **`sprint stop`**: fails with `commands.tick.sprint.stop.fail` if not sprinting; otherwise `stopSprinting()` (which sends the report) and `commands.tick.sprint.stop.success`, broadcast, and returns 1.
- **`freeze`** and **`unfreeze`** (`setFreeze`):
  - When freezing: if sprinting, `stopSprinting()` (with its report); if stepping, `stopStepping()`. Then `setFrozen(true)` and `commands.tick.status.frozen`, broadcast, returning 1.
  - When unfreezing: `setFrozen(false)` and `commands.tick.status.running`, broadcast, returning 0.
- Brigadier errors: `argument.float.low` (`Float must not be less than %s: found %s`) and `argument.float.big`, both with Java `Float.toString` numbers such as `1.0` and `0.5`. Also `argument.time.tick_count_too_low` and `argument.time.invalid_unit`, which already exist.
- English strings (en_us.json), for the test lang table only:
  - `commands.tick.query.percentiles`: `Percentiles: P50: %sms P95: %sms P99: %sms. Sample: %s`
  - `commands.tick.query.rate.running`: `Target tick rate: %s per second.\nAverage time per tick: %sms (Target: %sms)`
  - `commands.tick.query.rate.sprinting`: `Target tick rate: %s per second (ignored, reference only).\nAverage time per tick: %sms`
  - `commands.tick.rate.success`: `Set the target tick rate to %s per second`
  - `commands.tick.sprint.report`: `Sprint completed with %s ticks per second, or %s ms per tick`
  - `commands.tick.sprint.stop.fail`: `No tick sprint in progress`
  - `commands.tick.sprint.stop.success`: `Interrupted the current tick sprint`
  - `commands.tick.status.frozen`: `The game is frozen`
  - `commands.tick.status.lagging`: `The game is running, but can't keep up with the target tick rate`
  - `commands.tick.status.running`: `The game is running normally`
  - `commands.tick.status.sprinting`: `The game is sprinting`
  - `commands.tick.step.fail`: `Unable to step the game - the game must be frozen first`
  - `commands.tick.step.stop.fail`: `No tick step in progress`
  - `commands.tick.step.stop.success`: `Interrupted the current tick step`
  - `commands.tick.step.success`: `Stepping %s tick(s)`

### 3.5 The client (`Minecraft.getTickTargetMillis`, `Minecraft.tick`)

- The client tick length is `max(50 ms, millisecondsPerTick())` while the level runs normally, and 50 ms otherwise. So a rate above 20 doesn't speed the client up, and a rate below 20 slows it down.
- While frozen, the client skips `animateTick` and particle ticks (`isLevelRunningNormally`), and `ClientLevel` skips frozen entities.
- `DeltaTracker.Timer.updateFrozenState` holds the partial tick, so frozen mobs don't jitter.
- Players are never frozen and keep moving.

## 4. The plan

### Design in one paragraph

The server owns the schedule. A new `TickRate` in `Globals` holds vanilla's `ServerTickRateManager` state, the `runServer` timing (`next_tick`, `last_overload_warning`) and the 100-entry tick-time ring.
- **Planning:** once per frame, `Server::plan_ticks(now, ticking)` runs vanilla's loop logic and returns a `TickBatch`: one `runs_normally` flag per server tick due now. It returns nothing while any ticking level still has an unanswered batch. That is the back-pressure that removes the backlog, and the ticks then count as "behind", so vanilla's warning and skip apply.
- **Batches:** a batch holds at most as many ticks as fit in a 50 ms work budget at the mean tick time of the last answered batch (at least 1, at most 100). Not the 100-tick average: after a load drop that average stays high for about 100 ticks, so catching up would crawl at one tick per frame. Catching up and `/tick sprint` therefore run faster than the frame rate, while a command waits for at most one slow tick.
- **The level thread** runs the batch tick by tick, handling commands that arrive between two ticks, and answers with one `Output` per batch that carries each tick's nanoseconds.
- **Server-wide consumers** (clocks, weather, autosave, empty-level timers, containers, the dragon fight) advance by the batch.
- **The client part of `Entities`** ticks on its own vanilla client timing.

### Commit groups (in order, one commit each)

**Group 1. "The Minecraft level thread answers each tick before later commands"**
- Files: `crates/minecraft_terrain/src/server.rs`.
- `server_loop` (`:2673-2852`): the inner drain loop ends right after it handles a `Command::Tick`, and the `Output` is sent then. Non-tick commands queued before it still land in the same `Output`. Commands after it go into the next one. A stream of commands can no longer hold back an answer.
- `Output` gains `pub tick_nanos: Vec<u64>` (one entry per tick this output covers, from the existing `started.elapsed()` at `:2750`).
- `ServerHandle` gains `batches_sent` and `batches_answered`, counted in `tick()` (`:2519`) and in `poll()` (`:2637`) for every output with a non-empty `tick_nanos`, plus `pub fn ticks_in_flight(&self) -> u64`.
- Tests (in `server.rs` `mod tests`, gated on the data like their neighbours):
  - `each_tick_gets_its_own_output`: three `tick()` calls in a row, then `wait_idle`; `poll()` returns three outputs, each with `tick_nanos.len() == 1`, and `ticks_in_flight() == 0`.
  - `commands_after_a_tick_do_not_hold_its_output`: one tick followed by 200 `summon_orb` commands, then `wait_idle`. The first output has `tick_nanos.len() == 1` and `handled == 1`, and the orbs arrive in later outputs.

**Group 2a. "Minecraft has vanilla's server tick schedule, with the overload warning and skip"** (pure logic, not wired yet)
- Files: new `crates/minecraft_survival/src/tick_rate.rs` (it must pass `rustfmt --check`); `crates/minecraft_survival/src/lib.rs`.
- Split from 2b so the scheduler and its tests land and are reviewed on their own; 2b then only wires it.
- **`tick_rate.rs`** holds pure logic with no threads or clocks of its own. Every method that needs the time takes `now: Instant`.
  - **`TickRate`:**
    - Fields: `rate: f32`, `nanos_per_tick: u64`, `frozen: bool`, `frozen_ticks_to_run: i32`, `run_game_elements: bool`, the sprint fields (`remaining`, `scheduled`, `time_spent_nanos`, `previous_frozen`), `next_tick: Option<Instant>`, `last_overload_warning: Option<Instant>`, `tick_count: u64`, `samples: [u64; 100]` and `aggregated: u64`.
    - `pub fn plan(&mut self, now: Instant, ready: bool) -> TickBatch`. It returns an empty batch when `!ready`. Otherwise it runs section 3.1 steps 1-3 in a loop while a tick is due and the batch is under `max_batch()`:
      - the sprint check, or the overload check with the exact warning and skip;
      - due means `now >= next_tick`, or a sprint tick;
      - per planned tick it advances `next_tick`, calls the vanilla `tick()` (3.3), pushes `runs_normally()` and bumps `tick_count`.
    - The first `plan` sets `next_tick = now`, as `runServer` does.
    - The warning goes through `diag::warn!(World, "Can't keep up! Is the server overloaded? Running {}ms or {} ticks behind", ms, ticks)` and is also returned in `TickBatch.overload: Option<(u64, u64)>` so tests can see it.
    - `pub fn record(&mut self, nanos: &[u64])` puts each tick's time into the ring at `tick_count % 100` order (vanilla `tickServer` tallying) and adds it to `time_spent_nanos` while sprinting (`endTickWork`).
    - `average_nanos()` follows vanilla's formula (it feeds `/tick query` and the sprint autosave rate). `max_batch() = clamp(50_000_000 / max(last, 1), 1, 100)`, where `last` is the mean nanoseconds per tick of the most recent `record` call (1 before any).
    - Group 5b adds the command methods.
  - **`TickBatch { pub normal: Vec<bool>, pub overload: Option<(u64, u64)> }`**, with `len()` and `normal_count()`.
- Tests (`tick_rate.rs`, with `Instant` values built from one base plus `Duration`s):
  - Twenty calls 50 ms apart give one tick each, and a call at the same instant gives none.
  - After a 1 s gap the backlog of 20 ticks is caught up in batches limited by `max_batch`, with no warning.
  - `max_batch` follows the last batch: after 100 recorded ticks of 1 s, one batch recorded at 1 ms a tick gives a batch limit of 50, although `average_nanos` is still above 900 ms.
  - A 3 s gap warns once with `(3000 - 50, 59)` as computed by the vanilla formula, skips those ticks, and a second 3 s gap within 15 s catches up without a warning.
  - `ready = false` plans nothing, and `next_tick` stays, so the time behind grows.
  - `record` ring: the average over fewer than 100 ticks divides by the tick count. After 150 ticks only the last 100 count.

**Group 2b. "The Minecraft server ticks on a vanilla schedule and skips ticks when it falls behind"**
- Files: `crates/minecraft_survival/src/server.rs`; `crates/minecraft_survival/src/entities.rs`; `crates/minecraft_terrain/src/server.rs`; `crates/render_anim/src/minecraft_world.rs`; `crates/minecraft_survival/src/the_end/dragon_fight.rs` (the test's `entities.tick` call at `:1118` only).
- **`Globals`** (`server.rs:34-53`) gains `pub tick_rate: TickRate`. It is built in `test_globals` and at `render_anim/src/minecraft_world.rs:296-306`.
- **`Server::plan_ticks(&mut self, now: Instant, ticking: &[Dimension]) -> TickBatch`**:
  1. For each ticking level, `Entities::take_finished_ticks()` (see below). The element-wise maximum across levels goes to `tick_rate.record`, because the levels run in parallel and the slowest one limits the rate.
  2. `ready` = every ticking level has `Entities::ready_for_ticks()`.
  3. `tick_rate.plan(now, ready)`.
- **`Entities`:**
  - `pub fn ready_for_ticks(&mut self) -> bool` drains `server.poll()` into `unread` (as `poll_jobs` does at `:656`) and returns `ticks_in_flight() == 0`. It must poll, or a batch answered between frames would only be seen one frame later, which halves the rate.
  - `take_finished_ticks()` returns the `tick_nanos` gathered from outputs read so far.
  - `Entities::tick` (`:814`) takes `batch: &TickBatch` and stops deciding server ticks itself. When `batch.len() > 0` it builds the `TickInput` as today (`:870-904`) and sends `self.server.tick(input, batch.normal.clone())`. Its client tick keeps `self.clock`, still at `TICK_SECONDS` here. Group 4 changes that.
- **The level thread:**
  - `Command::Tick(Box<TickInput>)` becomes `Command::Tick(Box<TickInput>, Vec<bool>)`, and `ServerHandle::tick(input, normal)` changes to match.
  - In `server_loop`, the tick body (`:2749-2839`) moves into `fn run_tick(sim, input: &mut TickInput, out)` and runs once per flag.
  - Before the second and later ticks of a batch, it drains queued commands with `try_recv` and applies them through the same match. A `Tick` drained mid-batch (it can't happen with back-pressure) appends its flags and replaces the input.
  - The `pickup` inventory copy keeps the stacks this batch already picked up: write the mutated `inventory` back into `input.pickup`. `player_hurts` are applied on the first tick only (clear them after).
  - The snapshot fields (`out.mobs`, `out.weather`, `out.entities`, `tick_phases`) are built once, after the last tick of the batch. The event lists keep extending, as they do today when several ticks merge.
  - `ticks_since_save` (`:2753`) counts each tick.
  - Flags are ignored in this group (every tick runs normally). Group 4 uses them.
- **`render_anim/src/minecraft_world.rs`:** call `server.plan_ticks(Instant::now(), &ticking)` once per frame, right after the autosave check (`:606-609`).
  - `ticking` is the local player's dimension when its level has `entities`. That is today's "only the host's level ticks"; track E widens it.
  - Pass the batch to `entities.tick` (`:1707`). There are about ten early returns between `:606` and `:1707` (arriving, dead, menus). A planned batch must not be lost there: `TickRate` has already counted it (`tick_count`, `frozen_ticks_to_run`, sprint ticks), so dropping it would eat `/tick step` and sprint ticks. `Server` keeps the planned batch in `pending_batch` until `entities.tick` takes it, and `plan_ticks` treats a pending batch as "not ready". A frame that returns early leaves it for the next frame; a long stall then counts as being behind, as decided in section 6.
  - Test: `a_batch_left_by_an_early_return_is_sent_on_the_next_frame` (a batch of 3 planned on a frame that never reaches `entities.tick` arrives whole on the next one, and nothing new is planned meanwhile).
- Tests:
  - In `minecraft_terrain`: `a_batch_runs_each_tick` (a batch of 3 advances `level.game_time` by 3 and answers with `tick_nanos.len() == 3`) and `a_rule_sent_after_a_batch_applies_to_the_next_batch`.

**Group 3. "Minecraft clocks, weather, autosaves, containers and the dragon fight follow the server's ticks"**
- Files: `crates/minecraft_survival/src/server.rs`, `crates/minecraft_survival/src/save.rs`, `crates/minecraft_terrain/src/day_cycle.rs`, `crates/render_anim/src/minecraft_world.rs`, `crates/render_anim/src/minecraft_end.rs`.
- `Server::plan_ticks` applies the batch to server-wide state itself:
  - `globals.advance_clocks(normal_count)`: `Globals::advance_clocks` takes ticks instead of seconds. Add `DayCycle::advance_ticks(u32)`. The `shift_day` branch moves by the same tick count.
  - `globals.advance_weather(ticking, normal_count)`.
  - `globals.level_ticks += len`.
  - The autosave countdown. `autosave_ticks` becomes `ticks_until_autosave`, starting at `autosave_interval(&tick_rate)`, which is `max(100, (rate * 300) as i32)`, or with `1e9 / (average + 1)` as the rate while sprinting. It drops by `len`. At 0 or below, `plan_ticks` sets `globals.autosave_due = true` and restarts the countdown. `render_anim` (`:606`) checks and clears that flag instead of comparing with `AUTOSAVE_TICKS`. Remove `save::AUTOSAVE_TICKS` (`save.rs:20`), or keep it as the 20 TPS value used in tests.
  - `TickRate::set_rate` (group 5b) lowers the countdown like `onTickRateChanged`.
- Remove `globals.advance_clocks(dt)` at `minecraft_world.rs:678` and `:2213`, and the `autosave_ticks`, `level_ticks` and `advance_weather` lines at `:1716-1718`. `mob_ticks` stays for the client loop at `:1719-1727` and `:2161`.
- `tick_containers` (`minecraft_world.rs:986`) gets `batch.normal_count()` instead of `hand_ticks`.
- `minecraft_end::tick_fight` (`minecraft_end.rs:63`) gets the batch's normal count as seconds (`count as f64 * dragon_fight::TICK_SECONDS`), so `dragon_fight.rs` itself doesn't change.
- Tests:
  - Convert the clock test (`minecraft_survival/src/server.rs:1212-1250`) from seconds to ticks (1.0 s becomes 20 ticks).
  - `autosave_interval` is 6000 at 20 TPS, 100 at rate 1 (the `max`) and 3000 at rate 10. Lowering the rate shortens the countdown, and raising it doesn't.
  - Frozen ticks advance no clock and no weather, but count towards the autosave and empty-level timers.

**Group 4. "A frozen Minecraft tick only moves the players"**
- Files: `crates/minecraft_terrain/src/server.rs`, `crates/minecraft_survival/src/entities.rs`, `crates/render_anim/src/minecraft_world.rs`.
- **The level thread**, for a tick flagged `false`, keeps only the work a player does:
  - It keeps `set_clocks`, `set_players`, `living_players`, `set_simulation_area`, `mobs.tick_player(0)` with the hurts and views and vitals, `check_merchant`, `pickup` with `pickup_tridents` and `take_experience`, and the snapshot.
  - It skips `sim.tick()` (block, fluid and random ticks, spawning, block events, level entities, block entities), `take_spawned`, `despawn_mobs`, `tick_mobs`, `tick_clouds`, `tick_thrown`, `tick_tridents`, `tick_eyes`, `tick_end` and `spawn_trade_experience`.
  - Other track A items add per-tick level work in the same body: a1's gateway table tick (ages and cooldowns are block entities) and a1's pearl sweep (inside `tick_thrown`) are skipped on frozen ticks; a4's trial spawner pick runs inside `sim.tick()`. Whichever of a5 and a1 merges second puts a1's hooks inside `run_tick` behind the normal-tick flag.
  - Commands still run between ticks, so player hits on frozen mobs still land. A mob killed while frozen stays until the game runs again, as vanilla's frozen `deathTime` does.
- **The client:** `Entities::tick` takes `client: ClientTiming { tick_seconds: f64, runs_normally: bool }`. `render_anim` builds it from `globals.tick_rate.client_timing()`, which this group adds to `tick_rate.rs` (with a test: 0.05 at rate 20, 0.2 at rate 5, 0.05 at rate 40, and 0.05 with `runs_normally` false while frozen).
  - The tick length is `max(0.05, 1 / rate)` while running normally, otherwise 0.05.
  - While `runs_normally` is false, `self.clock` stops (the partial tick at `:1196` holds), and the client tick block (`:828-866`: falling, TNT smoke, lids, client mobs, portal, eyes, blaze sounds, spell, orb pickups) doesn't run. `client_ticks()` doesn't advance, so `rain_tick`, `animate_tick`, `campfire_tick` and `tick_scene` pause, as vanilla's frozen client does.
  - `runs_normally` is `TickRate.run_game_elements`: the flag of the last planned tick, kept on the frames in between until the next batch (true before the first). It must not fall back to true on a frame that plans no tick, or a frozen client would run on most frames.
- Tests (`minecraft_terrain`):
  - `a_frozen_tick_holds_items_and_falling_blocks`: an item spawned at y=210 and a falling sand block keep their positions over 20 frozen ticks and fall over 20 normal ones.
  - `a_frozen_tick_still_picks_up`: an item with pickup delay 0 at the player's feet is picked up on a frozen tick.
  - `primed_tnt_waits_while_frozen`: the fuse is unchanged after 10 frozen ticks.
  - `random_ticks_skip_frozen_ticks`: `level.game_time` doesn't move on frozen ticks.

**Group 5a. "Minecraft commands take decimal arguments and fixed suggestions like brigadier"**
- Files: `crates/minecraft_commands/src/args.rs`, `tree.rs`, `suggest.rs`.
- Split from 5b because it is command-tree plumbing that other commands can use; it has no `/tick` in it.
- **`args.rs`:**
  - `ArgKind::Float { min: f32, max: f32 }` and `Arg::Float(f32)`, parsed with `read_float`. Bounds give `argument.float.low` or `argument.float.big` with the minimum or maximum and the number found, both written as Java's `Float.toString` writes them (`1.0`, `0.5`, `10000.0`), at the argument's start like `read_bounded_int` (`reader.rs:197-215`).
  - Add the `Float` arm to `suggest_argument`'s no-op arm (`suggest.rs:447`).
- **`tree.rs`:** a `Node::suggests(self, &'static [&'static str]) -> Self`. Like a brigadier `SuggestionProvider`, it replaces the kind's own suggestions. `suggest.rs` offers them filtered by the typed prefix.
- Tests (`minecraft_commands`, on a test-only node): `0.5` against `Float { min: 1.0, max: 10000.0 }` gives `Float must not be less than 1.0: found 0.5` with the cursor at the argument's start, `20000` gives the `big` error, `40` parses as `40.0`; a node with `suggests(["20"])` offers `20` and not the kind's own suggestions.

**Group 5b. "Minecraft /tick query, rate, freeze, unfreeze, step and sprint like vanilla"**
- Files: `crates/minecraft_commands/src/server.rs`, `lib.rs`; new `crates/minecraft_commands/src/tick.rs` (it must pass `rustfmt --check`); `crates/minecraft_commands/src/commands.rs` (`crate::tick::node()` in `build`, `:96-175`); `crates/minecraft_survival/src/tick_rate.rs`; `crates/minecraft_survival/src/commands.rs`.
- **`tick.rs`:** the node from section 3.4.
  - `.requires(ADMINS)`.
  - `rate` takes `Float { min: 1.0, max: 10000.0 }` and suggests `["20"]`.
  - `step` executes with 1 tick, with `stop` and `time` (`ArgKind::Time { min: 1 }`, suggesting `["1t", "1s"]`).
  - `sprint` has `stop` and `time` (`Time { min: 1 }`, suggesting `["60s", "1d", "3d"]`).
  - `unfreeze` and `freeze`.
  - Every execute function follows `TickCommand` line for line, with its message order, broadcast flags and return values. `setFreeze` stops sprinting and stepping in the command before `set_tick_frozen(true)`.
  - Numbers are formatted with `format!("{:.1}")` and `format!("{:.2}")`.
- **`server.rs`:**
  - `pub struct TickQuery { pub rate: f32, pub frozen: bool, pub sprinting: bool, pub nanos_per_tick: u64, pub average_nanos: u64, pub samples: [u64; 100] }`.
  - `CommandWorld` methods with defaults, so other worlds still compile: `tick_query(&self) -> Option<TickQuery>` (when `None`, the command answers as unknown), `set_tick_rate(f32)`, `set_tick_frozen(bool)`, `step_ticks(i32) -> bool`, `stop_stepping() -> bool`, `sprint_ticks(i32) -> bool` (was sprinting), and `stop_sprinting() -> Option<(i32, String)>`.
  - `stop_sprinting` returns the finished sprint's report (TPS and ms per tick), or `None` when no sprint ran. `sprint stop` and `freeze` send that report with `server_feedback` *before* their own line, which keeps vanilla's order.
  - `CommandServer::server_feedback(&self, text, rules) -> Vec<Outgoing>`: `chat.type.admin` with the name `Server`, grey italic, to every op when `send_command_feedback` is on. It also pushes the plain line into `warnings`, so it reaches the log through the existing `take_warnings` drain (`minecraft_survival/src/commands.rs:1024-1026`). Use `diag::info!` there if a separate list reads better.
  - `FakeWorld` gets a small fake of these methods.
  - Add the 15 `commands.tick.*` keys and `argument.float.low` and `argument.float.big` to the test `lang` table (`:880`).
- **`tick_rate.rs`:**
  - `set_rate` (with the autosave shortening from group 3), `set_frozen`, `step(n) -> bool`, `stop_stepping`, `request_sprint(n) -> bool`, `stop_sprinting` and `query(&self) -> TickQuery`.
  - `finish_sprint` pushes `SprintReport { tps: i32, ms_per_tick: String }` into `reports`, which `take_reports()` drains.
- **`minecraft_survival/src/commands.rs`:**
  - `Host` gains `tick_rate: &'a mut TickRate`, built in `run` (`:977-1005`), and implements the trait methods.
  - After `pump`, `run` drains `globals.tick_rate.take_reports()` into `globals.chat.server_feedback(Text::translate("commands.tick.sprint.report", [tps, ms]))` and adds the result to `delivered`.
  - Sprints that end on their own finish in `plan_ticks` (group 2b), and their reports go out on the next frame's `run`. A sprint stopped by a command returns its report straight to the command (above) and doesn't go into `reports`.
- Tests:
  - **`minecraft_commands`** (through `FakeWorld`):
    - `/tick query` lines for running, lagging and frozen, with the percentile line built from a known ring.
    - `/tick rate 40` gives `Set the target tick rate to 40.0 per second` and returns 40. `/tick rate 0.5` gives `Float must not be less than 1.0: found 0.5`, and `/tick rate 20000` the `big` error.
    - `/tick step` while running gives the fail line. `/tick freeze` then `/tick step 1s` gives `Stepping 20 tick(s)`. `/tick step stop` twice gives success, then fail.
    - `/tick sprint 60s` twice: the second answers `Interrupted the current tick sprint` and then `The game is sprinting`.
    - `/tick sprint stop` with no sprint gives the fail line.
    - A level-2 player gets an unknown command.
    - Suggestions: `/tick ` gives `freeze`, `query`, `rate`, `sprint`, `step`, `unfreeze`; `/tick rate ` gives `20`; `/tick step ` gives `1s`, `1t`, `stop`; `/tick sprint ` gives `1d`, `3d`, `60s`, `stop`.
  - **`tick_rate.rs`:**
    - Rate clamping and `nanos_per_tick` for 1, 3 (333333333) and 10000.
    - The step state machine: frozen with 3 steps gives three normal ticks, then frozen ones.
    - `request_sprint` gives back the previous frozen state at the end.
    - The report numbers for 1200 ticks in 600 ms (2000 TPS and `0.50`), and for 0 ticks (rate 20 and `50.00`).
    - Sprint ticks plan without waiting and record their time.
    - Freezing while sprinting reports once.

**Group 6. "The Minecraft state doc describes overload, /tick and the server's ticks"**
- Files: `docs/MINECRAFT-SURVIVAL.md`.
- Add `/tick` to the command list (next to `/time`, `:122`).
- Add a paragraph on the server tick:
  - it falls behind, warns in the log and skips like vanilla;
  - frozen ticks;
  - the client timing.
- Update the "assumes one player" limits (`:229`, `:277`), because `plan_ticks` takes the list of ticking levels.
- Add the limitations from decision 7.

## 5. Owner test

Kit line for this item (11 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:oak_sapling*16,minecraft:cow_spawn_egg*4,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:sand*16,minecraft:water_bucket,minecraft:furnace,minecraft:coal*16,minecraft:raw_iron*16,minecraft:dirt*32
```

1. Start in survival on grass in daylight. Place 4 saplings, spawn 2 cows, and drop a few cobblestone (Q). The items fly, land and can be picked up again.
2. Type `/gamerule random_tick_speed 20000`. The Minecraft world slows down: mobs and dropped items move in jerky steps, and saplings grow almost at once. MW2 movement, aiming and the camera stay smooth. **Screenshot 1.**
3. While it is slow, type `/tick query`. It shows `The game is running, but can't keep up with the target tick rate`, then `Target tick rate: 20.0 per second.` with an average time per tick above 50 ms, then a percentile line ending in `Sample: 100`. **Screenshot 2.**
4. Type `/gamerule random_tick_speed 3`. Mobs and items may race for a moment first: like vanilla, the server runs missed ticks back to back, at most about 15 s of game time since its last warning, and if it is still more than 2 s behind it skips the rest with another `Can't keep up!` line. Within a second or two everything moves smoothly again. Drop an item: it falls and can be picked up. After about 10 s, `/tick query` shows `The game is running normally`. **Screenshot 3.**
5. Quit to the menu. `iw4l-artifacts\logs\latest.log` has at least one line `Can't keep up! Is the server overloaded? Running ...ms or ... ticks behind` from step 2. Send that line.
6. Back in the world, type `/tick freeze`. You get `The game is frozen`.
   - Drop cobblestone: it hangs in the air where it left your hand.
   - The cows stand still.
   - Light a TNT block: it flashes but doesn't explode.
   - Place sand in the air: it stays there.
   - Pour water: it doesn't spread.
   - The sun stops.
   - You can still walk, jump and shoot. Walking over a cobblestone lying on the ground picks it up. **Screenshot 4.**
7. Type `/tick step`: `Stepping 1 tick(s)`, and things move a tiny bit. Then `/tick step 5s`: `Stepping 100 tick(s)`; the TNT explodes (its fuse is 80 ticks), the hanging items fall and the water spreads, then everything stops again. Then type `/tick step stop`: red `No tick step in progress`. **Screenshot 5.**
8. Type `/tick unfreeze`: `The game is running normally`. Then type `/tick step`: red `Unable to step the game - the game must be frozen first`.
9. Place the furnace with coal and raw iron. Type `/tick rate 5`: `Set the target tick rate to 5.0 per second`. Mobs, items and the furnace's arrow run at a quarter speed in slow motion, while MW2 movement stays normal. `/tick query` shows `(Target: 200.0ms)`. **Screenshot 6.** Then type `/tick rate 20`.
10. Type `/tick rate 0`: red `Float must not be less than 1.0: found 0.0` with the `<--[HERE]` line.
11. Type `/tick sprint 1d`: `The game is sprinting`. A day and night pass much faster than normal: about one to three minutes, depending on the PC (the furnace finishes, saplings grow). MW2 stays smooth. Then a grey italic line appears: `[Server: Sprint completed with ... ticks per second, or ... ms per tick]`. **Screenshot 7.**
12. Type `/tick sprint 3d`, and after a second `/tick sprint stop`. The grey `[Server: Sprint completed ...]` report appears, then `Interrupted the current tick sprint`. Another `/tick sprint stop` gives red `No tick sprint in progress`.
13. Press Tab after `/tick `, `/tick sprint ` and `/tick step `. The lists are `freeze query rate sprint step unfreeze`, `1d 3d 60s stop` and `1s 1t stop`. **Screenshot 8.**
14. Type `/gamerule random_tick_speed 20000` again and leave it on for 30 s, then quit through the menu. The game closes normally within a few seconds instead of hanging.

## 6. Risks, shared files and decisions

### Risks

- **The per-player and server tick boundary:**
  - `TickRate` lives in `Globals`. It is server-wide, as vanilla's `ServerTickRateManager` is, and has no player in it.
  - `plan_ticks` takes the list of ticking levels. Track E's "every occupied level ticking" plugs in there, and readiness means every ticking level has answered.
  - `TickInput` still carries player `0`. When E gives it several players, each player's `pickup` inventory copy must be carried across the ticks of a batch, as group 2b does for one.
  - Client timing is per viewer and read from `Globals` for the host. Remote clients will need E's equivalents of `ClientboundTickingStatePacket` and `ClientboundTickingStepPacket` in `minecraft_net` (rate and frozen; steps).
- **The `Entities` split:** group 2b splits `Entities::tick` into "send a batch" (the level side) and "client tick" (the mirror side). That is the seam E needs, but both tracks edit `entities.rs:814-1000`. Merge track A before E starts its server-tick commits, or E rebases onto it.
- **Snapshots once per batch:** catch-up batches skip the in-between positions. That looks the same as today's merged outputs.
- **Clocks per batch:** `TickInput.clocks` is the clock after the whole batch, so a level's sky brightness can be up to one batch ahead inside it. That is harmless.
- **A single huge tick can't be interrupted:** with `random_tick_speed 2147483647`, one tick can take minutes. Vanilla can't recover from that either (its dedicated-server watchdog kills the server after 60 s). We recover after that one tick.
- **Main-thread stalls count as server lag:** the schedule runs on the main thread, so a stall longer than 2 s (alt-tab, a long load) warns and skips once, as a stalled vanilla server would. That is acceptable and vanilla-like.
- **The existing ops broadcast is gated by the wrong rule:** `CommandServer::feedback` sends the ops broadcast under `log_admin_commands` (`minecraft_commands/src/server.rs:436`). Vanilla gates it by `send_command_feedback` and uses `log_admin_commands` for the console copy only. `server_feedback` here follows vanilla. Fixing `feedback` is left to track E, where ops broadcasts first matter.

### Files shared with other round 8 tracks

| File | Also touched by | Keep the edit to |
|---|---|---|
| `minecraft_terrain/src/server.rs` | B (new mobs in `tick_mobs` and `Command`), C (shulkers, item frames and dragon effects in `EntitySnapshot` and `Output`), E (`TickInput` and `Output` per player) | `server_loop` (`:2673-2852`), the `Tick` variant, new fields at the end of `Output`, and `ServerHandle` (`:2410-2664`) |
| `minecraft_survival/src/entities.rs` | B, C, E | `Entities::tick` (`:814-905`) and two new methods |
| `minecraft_survival/src/server.rs` (`Globals`, clocks) | B (sleeping moves the clock to a time marker and needs `advance_clocks` in ticks), E | `Globals` fields, `advance_clocks`, and the new `plan_ticks` |
| `render_anim/src/minecraft_world.rs` | every track, D (skating) included | the lines listed in groups 2-4 |
| `render_anim/src/minecraft_end.rs` | C (dragon effects) | the `tick_fight` call |
| `minecraft_commands/src/{args,tree,suggest,commands,server}.rs` | A's other items, E (chat for several players) | additive enum arms, one `build` entry and new trait methods with defaults |
| `docs/MINECRAFT-SURVIVAL.md` | every track | the `/tick` and server-tick paragraphs |

### Decisions (the vanilla-like option)

1. **The fix is back-pressure plus the vanilla schedule** (one batch in flight per level, the schedule in `Globals`), not a cap on the channel. A capped channel would still let commands wait behind queued ticks and would lose the vanilla "behind" accounting.
2. **Ticks are skipped only together with the warning,** with vanilla's thresholds scaled by the tick rate. The log text is exactly vanilla's, at WARN level, in `latest.log` only, never in chat (vanilla doesn't show it in chat).
3. **The tick time is the slowest ticking level's level-thread time per tick.** The levels run in parallel, so that is what limits the rate. The main thread's share isn't counted.
4. **Batches are limited by a 50 ms work budget** (1-100 ticks) at the last batch's tick time. Catching up and sprints run above the frame rate, while a command waits for at most one slow tick.
5. **`/tick freeze` freezes the Minecraft world only.** MW2 movement, weapons, bots and MW2 projectiles keep running, as vanilla players are never frozen. Bullets and blasts still hurt frozen mobs, and a mob killed while frozen finishes dying after `unfreeze`.
6. **`/tick rate` speeds or slows the Minecraft level and the server-wide timers** (clocks, weather, containers, the dragon fight, the autosave interval). The client mirror follows vanilla's client timing (`max(50 ms, ms per tick)`).
7. **Player-side timers stay at 20 Hz** at any rate: hunger, air, effects, hazards, item cooldowns, the portal timer, elytra, mining and swing (section 2.2). They run with the MW2 player at the MW2 sim's own rate. Vanilla would run them at the server rate. This is listed as a limitation; a later item can drive them from the server's ticks once E has a fixed player tick.
8. **The tick state is not saved,** as in vanilla. Every world opens at 20 TPS and unfrozen.
9. **The percentiles use the whole 100-entry ring, zeros included,** and the sample is always 100, as vanilla does.
