# Track A, item a2: eyes of ender that fly (design brief)

Paths and lines are as of e2313625. Vanilla facts were checked with javap on the obfuscation-free 26.3 `client.jar` (classes named in section 3). Mojang code is described, never copied.

## 1. The owner's request

> Broken (screenshot): ... 2 eye of ender is not thrown

Screenshot `scrennshots/screenshots-e231362/screenshots-e231362/2_enderauge_werfen.png` ("throw ender eye"): survival HUD, standing high above a forest in the overworld, an ender eye stack (two-digit count, the digits are cut off) selected, the eye drawn large at the lower right, nothing in the air. The test step was `docs/round7/release-2-tests.md:56`: "`/locate structure minecraft:stronghold`, then throw an eye: it flies about 12 blocks that way with a purple trail, then drops or shatters."

The goal: right-click throws the eye. It rises from the chest and floats about 12 blocks toward the nearest stronghold, climbing 8, with a purple trail. After 4 s it drops as an item 4 times in 5, or shatters. With no stronghold (the Nether, the End) nothing happens and the eye stays in the hand, as in vanilla.

## 2. What exists today

Round 7 built the whole throw in commit 2f684111 ("Minecraft eyes of ender fly toward the nearest stronghold like vanilla", `docs/round7/track-end-log.md:349-414`). The owner never tested it before the final build. Each hop below reads correct (section 4 says what that leaves).

| Hop | Where | What it does |
|---|---|---|
| Click | `render_anim/src/minecraft_world.rs:1157-1169` | Right mouse (or the tactical key for the offhand) sets `place`. Holding it repeats every 4 ticks (`hand.place_delay = 4`). |
| Gate | `minecraft_world.rs:1316-1319` | The item chain runs only when `!entities.awaits_item_use()` and `game_mode.may_build()`. |
| Item half | `minecraft_world.rs:1329`, `minecraft_terrain/src/ender_eye.rs:40-59` | `eye_use`: an empty frame is filled (`Change::Server`), a filled frame passes, anything else (including air) gives `Change::ThrowEye(eye_throw(..))`. `eye_throw` (`:28-34`) sends the feet, the body middle (height 1.8, 1.5 crouching, 0.6 swimming, times 0.5) and the stack's components. |
| Send | `minecraft_world.rs:1348`, `minecraft_survival/src/entities.rs:587-590` | `Entities::throw_eye` sends `Command::ThrowEye { owner: PLAYER, .. }` (`PLAYER` is the constant 0, `entities.rs:36`) and sets `item_use_waiting`. |
| Server | `minecraft_terrain/src/server.rs:2735`, `:1065-1069` | `ServerSim::throw_eye`: `Eyes::locate`, then `Eyes::launch`. Answers one `bool` in `Output.item_uses` (`server.rs:2394`, no player key). |
| Search | `minecraft_terrain/src/eye_of_ender.rs:202-217`, `locate.rs:140-255` | The tag `#minecraft:eye_of_ender_located` is read once per `Eyes`. `locate::nearest_structure` with radius 100 and a 20 s cap (`eye_of_ender.rs:17-18`). For the stronghold's concentric rings it walks every ring chunk (`locate.rs:233-255`). |
| Rings | `third_party/minecraftoss/generator/src/structure/placement.rs:200-205` | `ring_chunks` re-runs `resolve_ring` (a 57 x 57 quart biome search) for all 128 slots on every call. The per-slot cache `RingSlot.resolved` (`:84-89`) is only used by `ring_contains` (`:258-266`), so every throw and every `/locate` repeats about 416,000 biome samples on the server thread. |
| Flight | `eye_of_ender.rs:49-121`, `:231-260` | `signal_target` (12 out, 8 up), `steer` (vanilla `updateDeltaMovement`), life 80, then the death sound and a drop or event 2003. `tick_eyes` runs each server tick (`server.rs:2788`); the snapshot carries the eyes (`server.rs:2837`). |
| Answer | `entities.rs:950-953`, `minecraft_world.rs:1825-1831`, `minecraft_survival/src/level.rs:124-167` | A `true` answer becomes `item_uses_done`: swing, and one eye leaves the stack (not in creative). A `false` answer drops the wait silently. Nothing is logged either way. |
| Client view | `entities.rs:842`, `:905`, `:1280`; `eye_of_ender.rs:312-405` | `EyeTrails` takes the snapshot, emits one portal particle per eye per tick, draws a 0.5-block camera-facing sprite at full brightness (hidden for 2 ticks within 3.5 blocks), and the shatter crumbs and ring. |
| Held item | `render_anim/src/minecraft_hand.rs:11-13`, `minecraft_world.rs:2120-2160` | `item_pose` is vanilla's `applyItemArmTransform` (0.56, -0.52, -0.72) times the model's `firstperson_righthand` display, drawn with a 70 degree vertical projection (`:2149`). |
| Offhand | `minecraft_survival/src/offhand.rs:100` | An eye counts as an item with a use, so the tactical key throws from the offhand. |
| Tests | `eye_of_ender.rs:407-686`, `ender_eye.rs:61-150` | The maths and a live `Level` are tested. The only test where a real overworld search finds a stronghold is `#[ignore]`d (`eye_of_ender.rs:640-654`), and nothing tests the click, the server thread and the answer together. |

## 3. Vanilla 26.3 (javap)

**`EnderEyeItem`**
- `useOn` (frame fill) is unchanged from round 7 (`docs/round7/track-end-log.md:43`).
- `getUseDuration` returns 0.
- `use(level, player, hand)`:
  1. `getPlayerPOVHitResult(level, player, Fluid.NONE)`; on a block hit whose state is any `end_portal_frame` (filled or not) it returns `PASS`.
  2. `player.startUsingItem(hand)`, on both client and server, before the stronghold search. With a use duration of 0 the player stays "using" the eye until the use key is released, so `Minecraft.startUseItem` does not repeat while the key is held: one eye per click.
  3. On the server: `findNearestMapStructure(StructureTags.EYE_OF_ENDER_LOCATED, player.blockPosition(), 100, false)`. When it finds nothing it returns `CONSUME`: no swing, no sound, nothing spent.
  4. Otherwise: `new EyeOfEnder(level, getX(), getY(0.5), getZ())`, `setItem(stack)`, `signalTo(Vec3.atLowerCornerOf(found))`, game event `projectile_shoot`, `addFreshEntity`, the `used_ender_eye` trigger, then `ENDER_EYE_LAUNCH` at the player's feet (`SoundSource.NEUTRAL`, volume 1, pitch `Mth.lerp(nextFloat, 0.33, 0.5)`), `stack.consume(1, player)` and the `item_used` statistic.
  5. Returns `SUCCESS_SERVER`, whose swing source is the server: the hand swings once, when the eye flies.
- Adventure mode: `ServerPlayerGameMode.useItemOn` refuses the frame (`mayBuild` is false), then `use` runs, so adventure players throw eyes but cannot fill frames.

**`EyeOfEnder`** (an `Entity`, not a projectile)
- `TOO_FAR_DISTANCE` 12, `TOO_FAR_SIGNAL_HEIGHT` 8, `MIN_CAMERA_DISTANCE_SQUARED` 12.25.
- `signalTo(to)`: if the horizontal distance from the eye to `to` is over 12, the target is the eye's position plus (dx/d·12, 8, dz/d·12); otherwise `to` itself (the ring chunk's corner at y 0, so standing over a stronghold the eye dives into the ground). `life = 0`; `surviveAfterDeath = random.nextInt(5) > 0` (4 in 5).
- `tick`: `newPos = position + delta`. Server: `delta = updateDeltaMovement(delta, newPos, target)`. Client: `spawnParticles(newPos - delta·0.25, delta)`. Then `setPos(newPos)`. Server: `++life > 80` plays `ENDER_EYE_DEATH` (volume 1, pitch 1), discards, then either `new ItemEntity(level, x, y, z, item)` or `levelEvent(2003, blockPosition(), 0)`.
- `updateDeltaMovement(old, pos, target)`: horizontal `h = (target - pos)` with y 0, length `L`; speed `lerp(0.0025, |old|h, L)`; `vy = old.y`; within 1 block both times 0.8; wanted `+1` if `pos.y - old.y < target.y`, else `-1`; result `h·(speed/L) + (0, vy + (wanted - vy)·0.015, 0)`.
- `spawnParticles`: in water 4 `bubble` particles at `p - motion·0.25`; else one `portal` at `p ± 0.3` (x and z), `p.y - 0.5`, with the motion as its speed.
- `shouldRenderAtSqrDistance`: hidden while `tickCount < 2` and within 12.25 squared.
- `getLightLevelDependentMagicValue` 1 (full bright); `isAttackable` false; `hurtServer` false. Saves only `Item`.
- Event 2003 (`LevelEventHandler`): 8 ender eye item particles at `x + 0.5, y, z + 0.5`, and 80 portal particles in a ring 5 blocks out (checked in round 7).
- `EntityRenderers`: `ThrownItemRenderer(ctx, 1.0, true)`, the item in the `ground` display (`item/generated`: translation 0, 2, 0 px, scale 0.5).

**Where the stronghold is**
- `ChunkGenerator.getNearestGeneratedStructure(.., ConcentricRingsStructurePlacement)`: for each ring chunk, the distance is measured from the chunk's middle at y 32; a chunk no closer than the best so far is skipped; otherwise `getStructureGeneratingAt`. The answer is the placement's locate position (the chunk's lower corner, y 0, plus `locate_offset`).
- `ChunkGeneratorStructureState.ensureStructuresGenerated` starts `generateRingPositions` for every concentric set at world load, with `CompletableFuture.supplyAsync(.., Util.backgroundExecutor())`. `getRingPositionsFor` then `join()`s the cached list. The ring positions are computed once per world, off the server thread.

**The held eye** (`models/item/generated.json`, `Camera.calculateHudFov`)
- `firstperson_righthand`: rotation (0, -90, 25), translation (1.13, 3.2, 1.13) px, scale 0.68. `applyItemArmTransform`: (0.56, -0.52 - 0.6·equip, -0.72). The hand is drawn at a fixed 70 degree FOV (changed only when dying or under water).
- Projected on the owner's 1833 x 1043 screenshot, the sprite's top-left corner lands at about (1343, 627) and the rest runs off the lower right edge. The screenshot shows it at about (1410, 590). So the size the owner calls "big" is vanilla's: every flat item is held like that. No change.

## 4. Why it fails: what the owner's log shows and what is left

From `iw4l-artifacts/logs/latest.log` (the e2313625 session in `r7final`):
- The world seed is 4986579828511054607 (`:827`).
- `/locate structure minecraft:stronghold` answered "The nearest minecraft:stronghold is at [-496, ~, -1936] (1998 blocks away)" (`:11367`). The owner then teleported there, went into spectator, and entered that stronghold's own end portal at (-530, -30, -1915) (`:11643`). So strongholds generate, and `/locate`'s search works on that seed.
- No panic, no warning about eyes or structures. The current code logs nothing about a throw, so the log can't say which hop failed.

Reading every hop (section 2) finds no bug: the client chain is the one that fills frames (which worked in the same session), and the server search is `/locate`'s own. Three explanations remain. Commit 1 tells them apart:
1. **The server answered no.** That looks exactly like the screenshot: no sound, no swing, the stack unchanged. `/locate` also runs on the level's server thread (`commands::locate_job`, `minecraft_survival/src/commands.rs:723-745`), with the same radius 100 and the same 20 s cap (`LOCATE_TIME_CAP`, `:29`). So `Eyes::locate` differs from `/locate` only in how it gets its targets: it expands `#minecraft:eye_of_ender_located` through `locate::tags` (`eye_of_ender.rs:156-162`), while `/locate` gets the parsed structure id. Each throw, like each `/locate`, repeats the full 128-ring biome search (`placement.rs:202`). A timeout or an empty tag expansion would answer no silently.
2. **The click never reached the server.** If an earlier item use was never answered, `awaits_item_use()` (`minecraft_world.rs:1318`) blocks every chain use. Frames were filled later in that session, so the gate was not stuck then, but the screenshot was taken in an earlier survival stretch (`:8421-:10796`).
3. **The eye flew but wasn't seen.** It starts at the chest, is hidden for 2 ticks, and heads for the stronghold, which may be behind the player. A stack of 5x eyes still shows an eye in the hand after a throw.

## 5. Plan (commit groups, in order)

### Group 1. "Minecraft eye of ender throws are logged and tested from the click to the flying eye"

If one of the new tests fails, the fix for that hop goes in this commit, and the subject names the fix instead (for example "Minecraft eyes of ender find the stronghold from the level's server thread").

- `minecraft_terrain/src/eye_of_ender.rs`:
  - `Eyes::locate` returns `EyeSearch { Found(DVec3), NotFound, TimedOut }` plus the elapsed time, instead of `Option`. An empty tag counts as `NotFound` and is reported as such.
  - New `pub struct EyeAnswer { pub owner: u64, pub found: Option<DVec3>, pub target: Option<DVec3>, pub outcome: &'static str, pub millis: u32 }`. `outcome` is "flew", "no structure" or "timed out".
  - `Eyes` keeps the answers until `take_answers()`.
- `minecraft_terrain/src/server.rs`:
  - `ServerSim::throw_eye` (`:1065`) records the answer and still returns the `bool` for `item_uses`.
  - `Output` gains `eye_answers: Vec<EyeAnswer>`, filled at the `Command::ThrowEye` arm (`:2735`).
- `minecraft_survival/src/entities.rs` (in `poll`, next to `:950`):
  - Each answer for `PLAYER` logs `diag::info!(World, "Minecraft eye of ender: flew from {feet} toward {x} {z}, target {target} ({ms} ms)")`.
  - A no or a timeout logs `diag::warn!(World, "Minecraft eye of ender: {outcome} ({ms} ms), the eye stays in the hand")`.
  - New `pub fn eye_views(&self) -> &[EyeView]` for tests.
- **Tests:**
  - `eye_of_ender`, `a_thrown_eye_goes_through_the_server_thread_and_flies_toward_the_stronghold`: `ServerHandle::spawn(ServerSim::new(..))` on an overworld `WorldGen` with seed 4986579828511054607. `throw_eye(7, throw)` from feet (-300.5, 80, -1700.5), `wait_idle`, `poll`: `item_uses == [true]`, and the answer has owner 7, outcome "flew", `found == (-496, 0, -1936)` (the owner's `/locate`), and `millis` under 20,000. Then send ticks: the snapshot holds one eye whose target is 12 blocks toward the stronghold and 8 up. It skips without the data, like the other data tests.
  - `eye_of_ender`, `the_eye_search_and_locate_agree`: `Eyes::locate` and `locate::nearest_structure(.., ["minecraft:stronghold"], ..)` give the same corner from three points on the owner's seed.
  - `eye_of_ender`, `a_throw_with_no_stronghold_answers_no_with_a_reason`: Nether, outcome "no structure", nothing in the snapshot. This extends `eyes_of_ender_look_for_the_stronghold_and_find_none_in_the_nether`.
  - `minecraft_survival` `server` tests, `an_eye_thrown_from_the_hand_leaves_the_stack_and_flies`:
    - Use `test_globals` with the owner's seed and open the overworld (`server_with`, `open_levels`, `wait_for_levels`).
    - Call `ender_eye::eye_use` with a `Player` at (-300.5, 80, -1700.5) aiming at the sky and 16 eyes in slot 0. Route its `Change::ThrowEye` to `Entities::throw_eye`.
    - Pump `Entities::tick` (as in `the_end/dragon_fight.rs:1118`) until `item_uses_done` holds the use. `level::finish_item_use` leaves 15.
    - After a few more ticks `eye_views()` has one eye.
  - Remove the `#[ignore]` from `an_overworld_eye_finds_a_stronghold_ring_chunk_corner` once group 2 makes it cheap. Until then leave it.

### Group 2. "Minecraft stronghold rings are worked out once per world, away from the server tick"

- `third_party/minecraftoss/generator/src/structure/placement.rs:202` (MinecraftOSS patch): `ring_chunks` takes each slot through `slot.resolved.get_or_init(|| resolve_ring(..))`, the cache `ring_contains` already uses. `/locate`, the eye and chunk generation then share one resolution per `WorldGen`.
- `minecraft_survival/src/server.rs:269-292`, `start_level`: when the level's `WorldGen` has a concentric-ring set, spawn one named background thread ("Stronghold rings") that calls `ring_chunks(&gen.terrain, usize::MAX)` on a clone of the `Arc<WorldGen>`. This is vanilla's `supplyAsync` at world load. A throw that comes first simply waits in `get_or_init`.
  - The overworld is the only vanilla dimension with such a set.
  - The thread holds an `Arc`, so a level closed early just lets it finish.
- Tests:
  - `placement`, `ring_chunks_fill_the_slot_cache_and_agree_with_ring_contains`: after `ring_chunks`, every slot is resolved, and `ring_contains` is true exactly at the returned chunks.
  - `locate`, `a_second_stronghold_search_reuses_the_rings`: two searches on one `WorldGen` give the same answer, and the second takes under a quarter of the first.
  - Now drop the `#[ignore]` from `an_overworld_eye_finds_a_stronghold_ring_chunk_corner`.

### Group 3. "Minecraft eyes of ender are thrown once per click, and in adventure mode, like vanilla"

- `minecraft_terrain/src/ender_eye.rs`: split `eye_use` in two.
  - `frame_use(player, world, held)` fills an empty frame. It stays in the `may_build` chain.
  - `throw_use(player, world, held)` gives `None` when aiming at any `end_portal_frame` (vanilla's PASS), else `Change::ThrowEye`.
- `render_anim/src/minecraft_world.rs`:
  - `throw_use` moves out of the `may_build` chain (`:1316-1329`) into its own branch just after it. That branch needs `game_mode.interacts()` (no spectators) and `!entities.awaits_item_use()`, so adventure players throw and spectators don't.
  - It keeps the same `SentItemUse` and `entities.throw_eye` call.
- `minecraft_survival/src/player.rs:48-63`: `Hand` gains `using_eye: Option<usize>`, the slot of an eye being "used" (vanilla's `startUsingItem` with duration 0). It is per `Player`.
- `minecraft_survival/src/offhand.rs`: a pure helper `pub fn eye_held(hand: &Hand, slot: usize, use_down: bool, selected_slot_now: usize) -> bool` decides whether the use is still held.
- `minecraft_world.rs:1157-1169`:
  - When an eye throw is sent (either answer), set `using_eye = Some(slot)`.
  - While it is set and the button or tactical key is still down, a held repeat (`idle && use_down && place_delay == 0`) does not fire for that hand.
  - A release, or a change of the selected slot, clears it. A fresh press always fires.
- Tests:
  - `ender_eye`, `adventure_mode_throws_but_frames_need_building`: `throw_use` on stone and air throws, `throw_use` on a frame passes, `frame_use` on an empty frame fills.
  - `offhand`, `holding_the_use_button_throws_one_eye`: `eye_held` is true while down on the same slot, false after release or a slot change.
  - Update `anywhere_else_the_eye_is_thrown_from_the_middle_of_the_body` to call `throw_use`.

No change to the held item's size (section 3), the eye's maths, sounds or particles, or saving.

## 6. Owner test steps

Setup: in the `.env` set `IW4L_MINECRAFT_WORLD=r8eye` (a new world) and `IW4L_MINECRAFT_SEED=4986579828511054607` (the seed of `r7final`, whose stronghold is known). Remove any `IW4L_MINECRAFT_DIMENSION` line. Use the kit below. After the test, send the screenshots and `iw4l-artifacts\logs\latest.log`.

1. Start by day. Type `/gamemode creative`, then `/tp @s -300 120 -1700`, wait for the ground to load, land, and type `/gamemode survival`.
2. Type `/locate structure minecraft:stronghold`. It answers about [-496, ~, -1936].
3. Face roughly that way (north-west) and right-click once with an eye. You hear the launch sound and your hand swings. Then the eye rises from your chest and floats about 12 blocks toward the stronghold, climbing about 8, glowing, with a purple trail. The stack loses one eye. **Screenshot 1** while it flies, with the `/locate` answer still in the chat.
4. After about 4 s you hear a short sound. The eye either falls as an item you can pick up, or breaks into crumbs and a purple ring.
5. Turn your back to the stronghold and throw again: the eye flies behind you, toward the stronghold.
6. Hold right-click for 3 s: only one eye flies. Release and click again: another flies.
7. Throw 10 eyes, one click each: about 8 drop and 2 break.
8. Type `/gamemode creative` and `/tp @s -496 120 -1936`, then throw. The eye sinks straight down into the ground under you; vanilla does this when you stand over the stronghold. **Screenshot 2.**
9. Type `/gamemode adventure` and throw: the eye flies. Place an end portal frame first (in creative), then in adventure right-click it with an eye: nothing happens. Type `/gamemode survival`, and right-click the frame: the eye goes in. Right-click the filled frame: nothing happens.
10. Type `/execute in minecraft:the_nether run tp ~ 128 ~` (on top of the Nether's bedrock roof, so you don't land inside netherrack) and right-click with an eye: no sound, no swing, the stack stays. **Screenshot 3.** Then `/execute in minecraft:overworld run tp ~ 120 ~`.
11. Compare the held eye with another flat item, such as `/give @s minecraft:snowball 1`: they're held the same way. It's as large and as far to the lower right as vanilla draws it.
12. In `latest.log`, every throw has a line starting "Minecraft eye of ender:".

Kit (3 stacks; eyes stack to 64, so 48 eyes are one stack):

```
IW4L_MINECRAFT_KIT=minecraft:ender_eye*48,minecraft:end_portal_frame*2,minecraft:cobblestone*64
```

## 7. Risks, shared files and decisions

**Risks**
- If group 1's tests all pass, the click-side gate is still the suspect. The `using_eye` work in group 3 touches the same lines, and the new log line will show in the owner's next log whether the server was asked at all (no line means it never was). Don't add speculative fixes beyond that.
- Group 1's seeded tests each build a `WorldGen` and resolve the rings: about 2.4 s each at opt-level 1 (round 7's measure). They stay un-ignored, since they guard the owner's bug. Group 2 makes repeated searches cheap within one `WorldGen`.
- The background ring thread competes with chunk generation for a second or so at level start. Name the thread so it shows in profiles.
- A search that still takes seconds blocks the level thread, as vanilla's does. Before a5 (overload) lands, the client keeps queueing ticks behind it; after a5 it shows as one long tick and, past 2 s, one `Can't keep up!` line. Either way the answer arrives. If a5 is merged first, the `Command::ThrowEye` arm is unchanged (commands are handled between ticks).

**Files shared with other round 8 tracks**
- `render_anim/src/minecraft_world.rs`: every track. Track E is moving player actions out of `render_anim` (`docs/CRATE-SPLIT.md` section 6). Keep the edits to the chain at `:1316-1349` and the repeat at `:1157-1169`, and put the logic in `minecraft_terrain::ender_eye` and `minecraft_survival::offhand`, so E can move the call lines as they are.
- `minecraft_terrain/src/server.rs`:
  - Track E: the several-player tick.
  - Track A's overload and `/tick` item: `server_loop`.
  - Track B: mob commands.
  - Group 1 adds one `Output` field and one line at `:2735`.
- `minecraft_survival/src/entities.rs`: track E (per-player outputs). Group 1 adds a logging loop and an accessor only.
- `minecraft_survival/src/player.rs` `Hand`: track E (per-player hands). One field.
- `minecraft_survival/src/server.rs` `start_level`: track E (levels ticking with several players). One spawn call.
- `third_party/minecraftoss/generator/src/structure/placement.rs`: track C (end cities use random-spread placement, not rings). One line. Record it in the MinecraftOSS patch list.

**Per-player and server tick boundary (for track E)**
- `Eyes` lives in each level's `ServerSim`, and each eye keeps its thrower's key. That is per level and per player already.
- `Entities::throw_eye` sends `PLAYER` (0), and `Output.item_uses` is a bare `bool` with no player key. `EyeAnswer.owner` carries the key, so E can route answers per player. E should give `item_uses` a player key the same way.
- `Hand.using_eye` is per `Player`.

**Decisions** (each the vanilla-like option)
- With no stronghold nothing happens: no message, no sound, no swing, as `CONSUME` does. The log line is the only trace.
- One throw per click, as `startUsingItem` with duration 0 makes it.
- Adventure players throw but can't fill frames. Spectators do neither.
- The held eye keeps its current size. It is vanilla's pose and FOV.
- Eyes in flight are still not saved (vanilla saves only the item, and a reloaded eye flies on blind for 80 ticks). Under water the trail stays purple, because there is no bubble particle yet.
- Still not done: the `used_ender_eye` trigger, the `item_used` statistic and the `projectile_shoot` game event (no advancements, statistics or sculk yet).
