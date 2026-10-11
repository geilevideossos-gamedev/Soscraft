# Track A, item a4: the mob inside a spawner (design brief)

Paths and line numbers are as of e2313625. Vanilla facts were checked with javap on the obfuscation-free 26.3 `client.jar`; no code was copied. Nothing here was built or run.

## 1. The owner's request

> "4 spawners show no mob inside" (round 8 test of e231362, screenshot `scrennshots/screenshots-e231362/screenshots-e231362/4_spawner_ohne_mob.png`)

The screenshot shows a Nether fortress blaze spawner: the black cage is drawn, but it is empty. There are no flames or smoke either. Vanilla shows a small copy of the spawner's mob, tilted forward and spinning inside the cage, with flames and smoke around it while a player is near. Scope: monster spawners first, then trial spawners if cheap.

## 2. What exists

| Area | State |
|---|---|
| Spawner block | It is meshed as a cut-out block model. Its occlusion is fixed (`minecraft_terrain/src/face_culling.rs:414-415`). There is no block entity renderer. |
| Server spawner tick | Works. `Level::tick_spawners` / `spawner_tick` / `spawner_spawns` are at `third_party/minecraftoss/world/src/level/spawning.rs:294-425`, with `Spawner` read/pick/delay at `:227-280`. `ServerSim::tick` calls it (`minecraft_terrain/src/server.rs:2043-2047`), gated by the `spawner_blocks_work` rule (`server.rs:2029-2031`). `Delay` and `SpawnData` are written back into the block entity tag. The tag is marked changed only when the delay is reset or the spawn data changes (`spawning.rs:333-342`). |
| Spawner tag defaults | `block_entity.rs:200-221` handles a loaded tag (`Delay` 20, `MinSpawnDelay` 200, `MaxSpawnDelay` 800, `SpawnCount` 4, `MaxNearbyEntities` 6, `RequiredPlayerRange` 16, `SpawnRange` 4). When `SpawnPotentials` is missing it becomes `[SpawnData]`. A placed spawner gets the catalog default (`world/src/level/mod.rs:472-486`, `core/src/block_entity.rs:88-93`), which is vanilla's fresh `SpawnPotentials: []`. Trial spawner tags are copied as they are (`block_entity.rs:302-310`). |
| Worldgen mob | `World::set_spawner_entity` (`world/src/level/mod.rs:979-986`) sets `SpawnData.entity.id` and marks the tag changed. A server test places a blaze spawner this way (`server.rs:5864-5882`). |
| Block entity changes to the client | `ServerSim::collect_block_entity_changes` (`server.rs:2151-2167`) sends every non-container change as `(pos, Option<Tag>)` in `Output::block_entities` (`server.rs:2360`). `Entities::tick` collects them (`minecraft_survival/src/entities.rs:966`) only to save them (`minecraft_survival/src/level.rs:176-182`). No visual reads them. |
| Chunks on the client side | `stream_tick` hands each loaded `Arc<Chunk>` to `Entities::load_chunk` (`level.rs:199-207`, `entities.rs:326-332`), which only forwards it to the level thread. Chunk block entities are in `chunk.block_entities.entities` (and `pending` for proto chunks) (`core/src/chunk.rs:284-287`). |
| Mob renderers | One `append_<mob>s(mesh, entities, poses: &ClientMobs, atlas, light, partial)` per family (blaze `blaze_render.rs:52-104`, zombie `zombie_render.rs:52`, skeleton `skeleton_render.rs:129`, spider `spider_render.rs:49`, and so on; full list via `grep "pub fn append_" *_render.rs`). Each looks up `poses.pose(entity.id, partial)` and places cuboids with `cube_scaled(mesh, feet, rotation, scale, ...)` (`cow_render.rs:522-663`). That function computes the face shade from the rotated normal (`:620-625`), so any rotation passed in shades correctly. Every renderer takes its rotation from `MobPose::body_rotation` (`client_mobs.rs:855-863`) and its light from `MobPose::light_block()` (`:880-882`). The calls are made in `Entities::meshes` (`entities.rs:1179-1300`). |
| Poses | `MobPose` (`client_mobs.rs:759-787`) is built by `ClientMob::pose` (`:733-757`). `ClientMobs` (`:885-963`) keys `ClientMob`s by entity id. `spawn_missing` adds mobs from `server_mobs(&EntityWorld)` (`:1024`) without ticking them. A fresh `ClientMob` has `tick_count` 0. `piglin_render.rs:334-343` builds a `MobPose` literal in a test. |
| Entities from NBT | `server_mobs::spawn_saved(&mut EntityWorld, &Tag)` (`server_mobs.rs:424`) is our `EntityType.loadEntityRecursive`. It needs `Pos`, reads `Rotation`, `IsBaby`, `Size`, variants and equipment, and returns `None` for kinds it does not simulate (`SIMULATED_KINDS`, `:389-420`). |
| Particles | `AmbientParticles` (`ambient_particles.rs`) has `Smoke` but no flame (`Kind`, `:28-39`; `add`, `:676-694`; `tick`, `:696-729`; `append_mesh`, `:732-800`). Its sprites are registered through `textures()` (`:452-462`, used at `mesh.rs:681-684`). `Entities` owns one (`entities.rs:70`) and ticks it in `tick_scene` (`:1170-1175`). |
| Entity textures | Registered in `mesh.rs:480-520`. `entity/spider/cave_spider` is missing. |
| Spawn eggs on spawners | `spawn_egg::egg_use` already returns `EggUse::Spawner(pos)` (`spawn_egg.rs:13-15`, `:72-78`), but `render_anim/src/minecraft_world.rs:1353-1361` ignores it. The state doc lists this: "Clicking a spawner with an egg doesn't change its mob" (`docs/MINECRAFT-SURVIVAL.md:331`). |
| Trial spawners | Trial chambers generate with trial spawners in `trial_spawner_state=waiting_for_players`, `ominous=false`, and tags `normal_config: "minecraft:trial_chamber/<group>/<mob>/normal"` and `ominous_config`. Nothing ticks them (`docs/MINECRAFT-SURVIVAL.md:265`). The configs are in the jar at `data/minecraft/trial_spawner/trial_chamber/**.json`. |
| State doc | `docs/MINECRAFT-SURVIVAL.md:281` says "no spawner and magma cube flames". |

## 3. Vanilla 26.3 behaviour (javap)

**Display entity** (`BaseSpawner.getOrCreateDisplayEntity`)
- It is created once from `SpawnData.entity` and only when that compound has an `id`, by `EntityType.loadEntityRecursive` (reason `SPAWNER`). Its id is set to -1.
- It is never ticked, so `tickCount` stays 0 and `ageInTicks = 0 + partialTick` (`EntityRenderer.extractRenderState`). A blaze's rods therefore twitch within one tick's motion instead of turning. Keep this.
- It gets no `finalizeSpawn`: no equipment, no random baby, default variants. Its yaw comes from an NBT `Rotation`, else 0.
- `BaseSpawner.load` sets `displayEntity = null`, so new block entity data rebuilds it.

**Render state** (`TrialSpawnerRenderer.extractSpawnerData`, also used by `SpawnerRenderer`)
- `displayEntity.lightCoords` = the spawner block's own `lightCoords`. A blaze's fixed block light 15 is overwritten.
- `spin = (float) Mth.lerp(partial, oSpin, spin) * 10`.
- `scale = 0.53125`, `m = max(bbWidth, bbHeight)`, and if `m > 1.0` then `scale /= m`.

**Draw** (`SpawnerRenderer.submitEntityInSpawner`)
- `translate(0.5, 0.4, 0.5)`, then `rotate(YP, spin degrees)`, then `translate(0, -0.2, 0)`, then `rotate(XP, -30 degrees)`, then `scale(s, s, s)`.
- After that, the entity renders at the origin with its own transforms (the body turn `180 - bodyRot`, the model flip, and so on).
- The Y translation is unaffected by the Y rotation, so the entity's feet sit at block + (0.5, 0.2, 0.5).
- `BlockEntityRenderer.getViewDistance` is 64: it draws only while the block centre is closer than 64 blocks to the camera. No shadow is drawn for the display.

**Client tick** (`BaseSpawner.clientTick`)
1. Not near a player: `oSpin = spin`, and nothing else happens.
2. Near a player with a display entity:
   - Take `x = pos.x + nextDouble()`, `y = pos.y + nextDouble()`, `z = pos.z + nextDouble()`.
   - Add `SMOKE` and `FLAME` at that same point, with zero motion.
   - If `spawnDelay > 0`, `spawnDelay--`.
   - `oSpin = spin`, then `spin = (spin + 1000f / (spawnDelay + 200f)) % 360`.
   - The step is 2.5 degrees per tick just after a spawn (delay 200) and 5 at delay 0, or 25 to 50 display degrees per tick after the x10.
3. Near a player without a display entity: nothing happens.

"Near" is `Level.hasNearbyAlivePlayer(pos + 0.5, RequiredPlayerRange)`: any player that is not a spectator, is alive, and has `distanceToSqr(feet) < range²` (any player if the range is negative).

The `% 360` wraps before the x10, so `lerp(358, 2) * 10` swings back once per turn. This is a vanilla quirk; keep it.

**How the client learns the delay**
- `SpawnerBlockEntity.getUpdateTag` is the full save minus `SpawnPotentials`. `load()` then sets `spawnDelay = Delay` and clears the display entity.
- The server's `delay()` first runs `setNextSpawnData` (`SpawnerBlockEntity$1` calls `sendBlockUpdated`), then `broadcastEvent(1)`.
- On the client, `onEventTriggered(1)` sets `spawnDelay = minSpawnDelay`.
- `ServerLevel.tick` runs `ServerChunkCache.tick` (block entity data) before `runBlockEvents`, so after each spawn round the client ends at `MinSpawnDelay` and the spin slows to 2.5 degrees per tick.

**Flame particle** (`FlameParticle` extends `RisingParticle`; sprite `particle/flame`)
- Motion: the usual `Particle` 7-argument constructor motion (our `Particle::base`), times 0.01, plus the given motion.
- The position is jittered by `(r - r) * 0.05` on each axis after the previous position is set.
- `friction` 0.96, no gravity.
- `lifetime = (int)(8 / (r * 0.8 + 0.2)) + 4`, which is 12 to 44.
- `move` shifts the box with no collision.
- Quad size: `quadSize * (1 - t² * 0.5)` with `t = (age + partial) / lifetime`.
- Light: `addSmoothBlockEmission(base, t)` adds `t * 240` to the block light (in 1/16 levels), capped at full. So the block light rises by `15 * t` levels.
- Layer `OPAQUE`.
- `SMALL_FLAME` is the same particle scaled by 0.5. `SOUL_FIRE_FLAME` uses sprite `particle/soul_fire_flame`.

**Spawn egg on a spawner** (`SpawnEggItem.useOn`, block entity `instanceof Spawner`)
- First, before looking at the block: if `EntityType.canSpawn(level)` is false the use fails (`FAIL`, no message, egg kept). `canSpawn` is false for a disabled feature flag, and for a kind that is not `isAllowedInPeaceful` while the difficulty is peaceful. So in peaceful a zombie, skeleton, spider, blaze, creeper, slime, enderman, ghast, wither skeleton, husk, cave spider or silverfish egg does nothing to a spawner, while a pig egg works.
- If `spawner_blocks_work` is false: system message `advMode.notEnabled.spawner`, and the use fails.
- Otherwise `setEntityId(type)`: `SpawnData.entity.id` is set and its other keys are kept. Then `sendBlockUpdated`, game event `BLOCK_CHANGE`, and `stack.shrink(1)`.
- No mob is spawned and creative is not required.

**Cave spider** (`CaveSpiderRenderer`)
- The spider renderer with layer `CAVE_SPIDER`, which is the spider body layer with `MeshTransformer.scaling(0.7)` (`LayerDefinitions`). That scale is about the model origin, plus a Y offset that keeps the feet on the ground, so it equals a uniform 0.7 about the feet.
- Texture `entity/spider/cave_spider`, the spider eyes layer, box 0.7 x 0.5.

**Trial spawner** (`TrialSpawnerRenderer`, `TrialSpawner.tickClient`, `TrialSpawnerState`, `TrialSpawnerStateData`)
- It draws with the same `extractSpawnerData`, using the state data's `oSpin`/`spin`.
- The display exists only while `state.hasSpinningMob()`, that is `spinningMobSpeed >= 0`.
- Per state (light level, particles, mob speed, capable of spawning):

  | State | Light | Particles | Mob speed | Capable |
  |---|---|---|---|---|
  | `inactive` | 0 | NONE | -1 | no |
  | `waiting_for_players` | 4 | SMALL_FLAMES | 200 | yes |
  | `active` | 8 | FLAMES_AND_SMOKE | 1000 | yes |
  | `waiting_for_reward_ejection` | 8 | SMALL_FLAMES | -1 | no |
  | `ejecting_reward` | 8 | SMALL_FLAMES | -1 | no |
  | `cooldown` | 0 | SMOKE_INSIDE_AND_TOP_FACE | -1 | no |

- `tickClient` runs every client tick, with no near-player check:
  1. Emit the state's particles.
  2. If spinning: `oSpin = spin`, then `spin = (spin + speed / (max(0, nextMobSpawnsAt - gameTime) + 200)) % 360`. The update tag sends `next_mob_spawns_at` only in `active`; otherwise the client value is 0, so a waiting spawner turns 1 degree per tick.
  3. If capable and `nextFloat() <= 0.02`: play `block.trial_spawner.ambient` (`ambient_ominous` when ominous) at the block, volume `nextFloat() * 0.25 + 0.75`, pitch `nextFloat() + 0.5`.
- `SMALL_FLAMES`: if `nextInt(2) == 0`, one `SMALL_FLAME` (`SOUL_FIRE_FLAME` when ominous) at the block centre plus `offsetRandom(0.9)`, which adds `(nextFloat() - 0.5) * 0.9` per axis.
- `FLAMES_AND_SMOKE`: `SMOKE` plus `FLAME` (or `SOUL_FIRE_FLAME`) at the same centre plus `offsetRandom(1.0)`.
- The update tag carries `spawn_data` (key checked) and, in `active` only, `next_mob_spawns_at`. The configs never reach the client.
- On the server, in the waiting and active states, `getOrCreateNextSpawnData` picks from `activeConfig().spawnPotentials` with the level random (normal or ominous by the block's `ominous` property), stores it as `spawn_data` and calls `markUpdated`.
- `canSpawnInLevel` = `spawner_blocks_work` && (`overridePeacefulAndMobSpawnRule` || (difficulty is not peaceful && `spawn_mobs`)).
- Config files hold, for example, `{"spawn_potentials":[{"data":{"entity":{"id":"minecraft:zombie"}},"weight":1}], ...}`. The baby zombie config has `"IsBaby":1`. The slime config has `"Size":1` (weight 3) and `"Size":2` (weight 1).

**Which spawners exist:**
- Dungeons: zombie, skeleton or spider.
- Mineshafts: cave spider.
- Fortresses: blaze.
- Strongholds: silverfish.
- Trial chambers: breeze, husk, spider, zombie, bogged ("poison_skeleton"), skeleton, stray, baby zombie, cave spider, silverfish and slime.

We have no silverfish or breeze model.

## 4. Plan: commit groups in order

General rules for every group:
- No code comments.
- Existing files are edited only on the lines needed, in their style. The new files must pass `rustfmt --check`.
- Display state is client visual state, per `Level` (owned by `Entities`, like `lids`). The server stays the owner of `SpawnData`, `Delay` and `spawn_data`.
- Nothing may use a "player 0" global. The near-player test takes a slice of player feet.

### Group 1: "Minecraft spawners show their mob spinning inside like vanilla"

**`crates/minecraft_terrain/src/client_mobs.rs`**
- `MobPose` gets `pub turn: glam::Quat`. `ClientMob::pose` sets it to `Quat::IDENTITY`.
- `body_rotation` returns `self.turn * <the current turn and death flip>`, so a display turn comes first, as the outer `PoseStack` does.
- `ClientMobs` gets a `displays: HashMap<u64, (Quat, DVec3)>` field and `pub fn show_in_spawner(&mut self, id: u64, turn: Quat, light_probe: DVec3)`. `pose()` applies a stored entry by overwriting `turn` and `light_probe`. `light_probe` must point into the spawner cell, so `light_block()` returns the spawner block.

**`crates/minecraft_terrain/src/piglin_render.rs:334-343`**
- Add `turn: glam::Quat::IDENTITY` to the test literal. This is a one-line touch in Track B's file.

**New file `crates/minecraft_terrain/src/spawner_display.rs`** (and `pub mod spawner_display;` in `lib.rs`)
- Free functions, unit-tested:
  - `pub fn display_scale(width: f32, height: f32) -> f32`: 0.53125, divided by the larger side when that side is over 1.0.
  - `pub fn spin_step(spin: f64, delay: i32) -> f64`: `(spin + f64::from(1000.0_f32 / (delay as f32 + 200.0))) % 360.0`.
  - `pub fn display_turn(spin_degrees: f32) -> Quat`: `Quat::from_rotation_y(spin.to_radians()) * Quat::from_rotation_x((-30f32).to_radians())`.
  - `pub fn display_feet(pos) -> DVec3`: `pos + (0.5, 0.2, 0.5)`.
- `pub struct SpawnerDisplays { displays: BTreeMap<BlockPos, SpawnerDisplay>, random: LegacyRandom }`.
- Per spawner it keeps:
  - the `SpawnData.entity` compound (`Option<Tag>`);
  - `delay`, `min_delay` and `range` (`RequiredPlayerRange`);
  - `spin` and `o_spin` (`f64`);
  - a lazily built `Shown { world: EntityWorld, mobs: ClientMobs, id: u64, size: (f32, f32) }`.
- Building `Shown`:
  - Clone the entity compound and insert `Pos` as a list of three doubles at `display_feet(pos)`.
  - Call `server_mobs::spawn_saved(&mut world, &tag)`. On `None` (a kind we cannot draw) there is no `Shown`, but spin and particles still run, as vanilla's display entity exists for any valid id.
  - Call `mobs.spawn_missing(&server_mobs(&world))` and take `size` from that `ServerMob`. Never call `mobs.tick()`, so `age_in_ticks = partial`, as in vanilla.
- `pub fn load_chunk(&mut self, chunk: &Chunk)`:
  - For every `minecraft:mob_spawner` tag in `block_entities.entities`, then `pending` if not already found, add a display.
  - Take `delay = Delay` (default 20), `min_delay = MinSpawnDelay` (200) and `range = RequiredPlayerRange` (16). Start the spin at 0.
- `pub fn unload_chunk(&mut self, pos: ChunkPos)`: drop that chunk's displays.
- `pub fn block_entities(&mut self, changes: &[(BlockPos, Option<Tag>)])`:
  - `None`, or a tag that is not a spawner: remove the display.
  - A new position: add it as `load_chunk` does.
  - A known position whose `SpawnData.entity` changed: replace the compound, drop `Shown`, and take `Delay` from the tag (vanilla `load()`). Keep the spin.
  - A known position with only the delay changed: this was a spawn round (update tag, then block event 1), so `delay = min_delay` from the tag.
- `pub fn tick(&mut self, players: &[DVec3])`: vanilla `clientTick` as in section 3. The "has a display entity" test is "the entity compound has an `id`".
- `pub fn append(&mut self, models, culled, translucent: &mut ChunkMesh, scene: &impl Scene, atlas, light, camera: DVec3, partial: f32)`. For each display:
  1. Skip it unless the scene block at its position is `minecraft:spawner` (this covers the gap between a break and the block entity removal), and unless its centre is within 64 blocks of `camera`.
  2. Call `mobs.show_in_spawner(id, display_turn(spin), center)` with `spin = lerp(partial, o_spin, spin) as f32 * 10.0` and `center = pos + 0.5`.
  3. Note the three meshes' vertex counts.
  4. Call the renderer for the kind, matched on the `id` string: the zombie family goes to `append_zombies`, the skeleton family to `append_skeletons`, `spider` to `append_spiders`, `blaze` to `append_blazes`, and so on through every mob in `SIMULATED_KINDS` except `end_crystal` and `ender_dragon`.
     - Bats go into `culled`. Slimes and horses also pass `translucent`.
     - Villagers get a facing closure that returns `None`.
     - Endermen get seed 0. Wolves get game time 0.
     - Held items, carried blocks and poppies that are returned are dropped (a display has none).
  5. Scale every new vertex uniformly about the feet: `p = feet + s * (p - feet)` with `s = display_scale(size)`. This equals the vanilla `scale(s)` after the rotations, because the scale is uniform; shading needs no change.

**`crates/minecraft_survival/src/entities.rs`**
- The struct gets a field `spawners: minecraft_terrain::spawner_display::SpawnerDisplays` (next to `lids`, `:105`), initialised next to `:249` with a seed derived from `seed`, like `ambient`.
- `load_chunk` and `unload_chunk` (`:326-332`) also call the displays.
- In `tick`, inside `if ticked` after `self.lids.tick()` (`:834`), call `self.spawners.tick(&players)`. Here `players` is `[feet]` when `player.alive && !player.spectator`, else empty. **Track E boundary:** this slice must become every player the client knows in this level.
- In the output loop, before `:966`, call `self.spawners.block_entities(&output.block_entities)`.
- In `meshes`, next to the blazes (`:1236`), call `self.spawners.append(&mut out.models, &mut out.culled, &mut out.translucent, scene, atlas, light, camera, partial)`.

**`docs/MINECRAFT-SURVIVAL.md`**
- Add a line under the Nether mobs or visuals: spawners show their mob spinning like vanilla.
- In `:265`, say that monster spawners spawn mobs and show their mob, and that trial spawners and vaults are not simulated.

**Unit tests** (in `spawner_display.rs`, and the `client_mobs.rs` tests module). For an atlas, copy `orb_atlas` (`orb_render.rs:242-262`) with an empty pack; regions fall back to the whole sheet.
- `a_display_scale_fits_the_longest_side_into_the_cage`:
  - zombie (0.6, 1.95): 0.53125 / 1.95;
  - spider (1.4, 0.9): 0.53125 / 1.4;
  - chicken (0.4, 0.7): 0.53125;
  - (1.0, 1.0): 0.53125.
- `spin_steps_a_thousand_over_delay_plus_two_hundred`: +2.5 at 200, +5 at 0, `1000/199` at -1, and it wraps at 360.
- `a_far_spawner_freezes_its_spin_and_delay`:
  - with a player 17 blocks from the centre and range 16, `o_spin == spin` and the delay is unchanged;
  - at 15 blocks, the delay drops by 1 and the spin grows;
  - spectators and dead players are not in the slice.
- `an_empty_spawner_neither_spins_nor_draws`: no `SpawnData` id gives no vertices and no spin.
- `a_spawn_round_restarts_the_spin_at_the_minimum_delay`:
  - a known display that gets a tag with the same `SpawnData` and `Delay` 600 ends at delay 200;
  - a new display reads `Delay`;
  - a changed `SpawnData` reads `Delay` and rebuilds;
  - `None` removes the display;
  - `unload_chunk` removes its displays.
- `a_zombie_and_a_blaze_sit_inside_their_cage`: every vertex is within the spawner cell (with a 0.05 margin) at spins of 0, 90 and 200 degrees. The display turn moves vertices between spins.
- `a_display_is_lit_by_its_spawner_cell`: with a `SkyLight::from_levels` that differs above and inside, the zombie's vertices carry the spawner cell's sky and block light.
- `spawners_further_than_64_blocks_are_not_drawn`.
- In `client_mobs.rs`, `a_spawner_turn_comes_before_the_body_turn`: `show_in_spawner` then `pose().body_rotation(90.0) == turn * Quat::from_rotation_y(PI)` (body_rot 0), and `light_block()` is the given probe's block.

### Group 2: "Minecraft spawners give off flames and smoke like vanilla"

**`crates/minecraft_terrain/src/ambient_particles.rs`**
- Add `Kind::Flame`, built by `Particle::flame`. Use `Particle::base` with zero motion, then:
  - velocity times 0.01;
  - a `(r - r) * 0.05` jitter of `position` only (not `previous`);
  - `lifetime = (8.0 / (r * 0.8 + 0.2)) as i32 + 4`;
  - friction 0.96, gravity 0.
- `tick` moves it without collision.
- `size_at` returns `quad_size * (1 - t² * 0.5)`.
- In `append_mesh`, the block light is `min(15, block + 15 * t)` with `t = clamp((age + partial) / lifetime, 0, 1)`. The flame is opaque.
- `textures()` gains `flame`.

**`spawner_display.rs`**
- `tick` takes `particles: &mut AmbientParticles`. When near and showing, it adds `Kind::Smoke` and `Kind::Flame` at one point `pos + (nextDouble, nextDouble, nextDouble)` from its own random, with zero motion.
- `entities.rs:834` passes `&mut self.ambient`.

**`docs/MINECRAFT-SURVIVAL.md:281`**
- Drop "spawner" from "no spawner and magma cube flames".

**Unit tests**
- `a_flame_hardly_moves_shrinks_and_glows_as_it_ages`:
  - the lifetime is within 12..=44 over many seeds;
  - it rises by less than 0.05 over its life;
  - it passes into a stone block;
  - the size at the end is half the start, quad-wise;
  - the block light is the cell's at age 0 and 15 at the end.
- `a_near_spawner_puts_one_smoke_and_one_flame_at_one_point_each_tick`: two particles per tick, both inside the cell; none when far; none for an empty spawner.

### Group 3: "Minecraft spawn eggs change a spawner's mob like vanilla"

This group gives the owner a way to test every mob in a spawner. It also removes the limitation at `MINECRAFT-SURVIVAL.md:331`.

**`render_anim/src/minecraft_world.rs:1356-1358`**
- `EggUse::Spawner(pos)` calls `entities.egg_on_spawner(inventory, slot, kind, pos, game_mode.consumes_items())`.
- `EggUse` needs the kind there. Either make `Spawner` carry the kind (`spawn_egg.rs:14`, `:77`), or call `spawn_egg::spawned_kind(&stack.id)`.

**`minecraft_survival/src/entities.rs`**
- Add `pub fn egg_on_spawner(...)`, mirroring `hatch_egg` (`:618-623`) through `eggs_waiting.send_from`.

**`minecraft_terrain/src/server.rs`**
- Add `Command::SpawnerEgg { pos, kind: String }` (not an `ItemOnBlock` variant, which is `Copy`) and `ServerHandle::spawner_egg`.
- The handler pushes the result of `sim.spawner_egg(pos, &kind)` into `out.summoned`. `EggsWaiting::answer` then spends the egg only on `Ok`.
- `ServerSim::spawner_egg`:
  - fails first, with no message, when the kind may not spawn at this difficulty: the same peaceful rule `ServerSim::summon` already applies to eggs (`server.rs:3995-3996`), vanilla's `EntityType.canSpawn`;
  - fails with `Err("advMode.notEnabled.spawner")` when `spawner_blocks_work` is off;
  - fails when the block entity is not `minecraft:mob_spawner` (trial spawners are left out);
  - otherwise calls `World::set_spawner_entity`, which marks the tag changed, so group 1 rebuilds the display with the tag's `Delay`.

**Unit tests**
- In `server.rs` tests, `an_egg_on_a_placed_spawner_sets_its_mob_and_keeps_it`, modelled on `:5864`:
  - place a spawner, run the egg with zombie;
  - the `SpawnData` id is zombie and the change is reported;
  - after the spawner ticks past a spawn round with a player near (as `:6696`), `SpawnData` is still zombie, because the catalog default has empty `SpawnPotentials`.
- `an_egg_does_nothing_when_spawner_blocks_do_not_work`.
- `a_monster_egg_does_nothing_to_a_spawner_in_peaceful`: difficulty 0, a zombie egg fails with no message and the `SpawnData` is unchanged; a pig egg sets it.

### Group 4: "Minecraft cave spiders show inside mineshaft spawners" (display only, cheap)

**`spawner_display.rs`**
- For `minecraft:cave_spider`, build the `Shown` from a copy of the tag with `id` set to `minecraft:spider`, `size = (0.7, 0.5)` and an extra model scale of 0.7 (multiplied into `s`).
- After `append_spiders`, remap the UVs of the new vertices that lie inside the `entity/spider/spider` region to the same relative place in `entity/spider/cave_spider`. Leave the eyes vertices alone. Both skins are 64 x 32.

**`mesh.rs:494`**
- Register `minecraft:entity/spider/cave_spider`.

The live cave spider mob stays unsimulated (no change to `SIMULATED_KINDS`).

**Unit tests**
- `a_cave_spider_display_is_a_smaller_spider_with_its_own_skin`:
  - the same vertex count as a spider;
  - the extent is 0.7 times the spider display's;
  - skin UVs are in the cave spider region, eyes UVs unchanged.

### Group 5: "Minecraft trial spawners show their mob spinning like vanilla" (optional; skip if the track runs long)

**New file `minecraft_terrain/src/trial_spawner.rs`**
- `TrialSpawnerBook::from_jar(jar)`, following the `TransformBook::from_jar` pattern (`transform.rs:296-314`). It maps `minecraft:<path>` from `data/minecraft/trial_spawner/<path>.json` to `Vec<(Tag, i32)>`, the spawn potentials' `data` and `weight`.
- It needs a small JSON-to-NBT conversion (numbers that are integers become `Int`, booleans become `Byte`). The generator's `json_to_tag` is private (`generator/src/feature/template/processor.rs:135`); write a local one.
- `pub fn potentials(&self, config: &Tag) -> Vec<(Tag, i32)>` accepts a string id or an inline compound with `spawn_potentials`.

**Server**
- `ServerSim::load_loot` (`server.rs:331`) also loads the book.
- After `tick_spawners` in `ServerSim::tick` (`:2045`), call a new `Level::pick_trial_spawn_data(&mut self, potentials: &dyn Fn(&Tag) -> Vec<(Tag, i32)>)` in `world/src/level/spawning.rs`, next to `tick_spawners`. It:
  - walks the same simulation chunks;
  - for each `minecraft:trial_spawner` tag without `spawn_data`, whose block's `trial_spawner_state` is `waiting_for_players` or `active`, and while `canSpawnInLevel` holds (`spawner_blocks_work`, not peaceful, `spawn_mobs`): picks from `ominous_config` if the block has `ominous=true`, else `normal_config`, weighted with `self.random`;
  - stores `spawn_data` and marks the tag changed.
- The trial itself (player detection, waves, rewards, cooldown) stays unsimulated.

**Client** (`spawner_display.rs`)
- It also keeps `minecraft:trial_spawner` tags (entity from `spawn_data.entity`).
- It reads `trial_spawner_state` and `ominous` from the scene block each tick:
  - It spins every tick with no player check, at speed 200 (waiting) or 1000 (active), divided by `max(0, next_mob_spawns_at - game_time) + 200`. Use 0 for the remaining time unless the state is active and the tag has `next_mob_spawns_at`.
  - It draws only in the waiting and active states.
- Particles go through `ambient_particles.rs`, which gains `Kind::SmallFlame` (a flame at half `quad_size`) and `Kind::SoulFireFlame` (sprite `soul_fire_flame`):
  - SMALL_FLAMES when `nextInt(2) == 0`;
  - FLAMES_AND_SMOKE in active.
- Ambient sound: with a 2% chance, push `minecraft:block.trial_spawner.ambient` (or `ambient_ominous`) at the block centre into `Entities::sounds` (`entities.rs:92`), with vanilla's volume and pitch. `tick` returns them.

**GameData** (`minecraft_survival/src/data.rs`)
- Not needed if the server loads the book from the jar itself.

**Unit tests**
- `the_book_reads_every_trial_chamber_config`: gated on `MINECRAFTOSS_ROOT`. 28 files; zombie normal is `[({entity:{id:zombie}}, 1)]`; slime has weights 3 and 1 with `Size` as `Int`; baby zombie has `IsBaby` 1.
- `a_waiting_trial_spawner_gets_spawn_data_from_its_config`: an inline config, and the result is marked changed. None in peaceful or with `spawner_blocks_work` off. None when `inactive`.
- `a_waiting_trial_spawner_turns_one_degree_a_tick_and_sometimes_flickers`: spin +1 per tick even with no player; small flames on about half the ticks over 2000 ticks; ominous gives soul fire flames.

## 5. Owner test

Use a new world `IW4L_MINECRAFT_WORLD=spawner8`. Give the spawners their mobs first (steps 2-4): in peaceful, vanilla refuses monster eggs, even on a spawner. After that you may run `/difficulty peaceful`, so no mobs come out while you take screenshots; the mob inside still shows and spins.

Kit (21 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:spawner*16,minecraft:zombie_spawn_egg*4,minecraft:skeleton_spawn_egg*4,minecraft:spider_spawn_egg*4,minecraft:blaze_spawn_egg*4,minecraft:creeper_spawn_egg*4,minecraft:slime_spawn_egg*4,minecraft:enderman_spawn_egg*4,minecraft:ghast_spawn_egg*4,minecraft:wither_skeleton_spawn_egg*4,minecraft:pig_spawn_egg*4,minecraft:husk_spawn_egg*4,minecraft:cave_spider_spawn_egg*4,minecraft:silverfish_spawn_egg*4,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:torch*64,minecraft:cobblestone*64,minecraft:cooked_beef*32,minecraft:spyglass
```

1. Place a spawner on the ground. It is an empty cage, with no flames (**1**).
2. Use the zombie egg on it. A small zombie appears in the middle of the cage, leaning forward, and turns fast. Flames and smoke rise inside the cage (**2**). In survival the egg is used up.
3. Place a row of spawners and give each a different egg: skeleton, spider, blaze, creeper, slime, enderman, ghast, wither skeleton, pig and husk (**3-5**, several per screenshot).
   - Each mob fits the cage. The tall enderman and the wide spider are shrunk to fit. The ghast is tiny. The pig and slime keep the plain size.
   - Spider legs may poke out a little; vanilla does the same.
4. Cave spider egg: a smaller, blue-grey spider (**6**). Silverfish egg: only flames and smoke, no mob (no model yet).
5. Walk more than 16 blocks away and look with the spyglass: the mob stops turning and the flames stop. Walk back: it turns again.
6. `/difficulty normal` and stand near the zombie spawner. When zombies come out, the spin slows right after the spawn, then speeds up again.
7. Break a spawner with the pickaxe: the mob disappears with the cage.
8. Quit and restart: every spawner still shows its mob.
9. `/gamerule spawner_blocks_work false`, then use an egg on an empty spawner: nothing changes and the egg is kept. Set it back to `true`.
   - `/difficulty peaceful`, then use the creeper egg on an empty spawner: nothing changes and the egg is kept. The pig egg still works. `/difficulty normal` afterwards.
10. Owner's original case: `IW4L_MINECRAFT_DIMENSION=the_nether`, then `/locate structure minecraft:fortress` and `/tp` there. The fortress spawner shows a blaze turning inside, with flames (**7**, compare with screenshot 4 of e231362).
11. In the Overworld, `/locate structure minecraft:mineshaft`, `/tp` there and find a cobweb room. Its spawner shows a cave spider (**8**).
12. If group 5 is in: `/locate structure minecraft:trial_chambers`, `/tp` there. Each trial spawner shows its mob turning slowly (about half a turn per second), with an occasional small flame and now and then its humming sound (**9**).

## 6. Risks, shared files and decisions

### Risks
- **Per-frame cost.** Every visible display calls one renderer on a one-mob world. Up to 64 blocks there are rarely more than a few spawners, but a dense trial chamber has about 10. Build `Shown` once per spawn-data change, never per frame.
- **`spawn_saved` side effects.** Some branches start mob AI (`spawn_zombie_active` and similar). Displays are never ticked, so that is harmless.
- `Passengers` (jockeys) return `None` from `spawn_saved`, so a jockey spawner shows nothing; vanilla shows the rider stack.
- **Fixed full block light in two renderers.** `blaze_render` and `magma_cube_render` pass block light 15. Vanilla's spawner overwrites that with the cage's light, so our display blaze is brighter than vanilla in a dark fortress. Accepted. Fixing it means touching every renderer.
- **Spin wrap.** The x10 wrap swings the display backwards for one tick once per turn. This is vanilla; don't "fix" it.
- **Block light in the cage.** The light probe reads our `SkyLight` at the spawner cell. If the light engine stores 0 inside non-full blocks, the mob is dark. Check in test step 2 with a torch nearby.
- **Trial spawners.** Group 5 gives only the look. Without the trial state machine the mob spins forever in `waiting_for_players`. Ominous spawners never appear, because nothing makes them ominous.

### Files shared with other round 8 tracks
- `minecraft_survival/src/entities.rs`:
  - **Track E** rewrites `Entities::tick`'s player input (today `PlayerView`, player `PLAYER`) and moves code out. Our hooks are one line each in five places. The near-player slice is the per-player boundary: E must pass every player in this level.
  - Track A's eye-of-ender and gateway items also touch this file. A runs its items in order, so there are no parallel conflicts.
  - Track A's a5 (overload and `/tick`) splits `Entities::tick` into the batch it sends and the client tick, and pauses the client tick while the game is frozen. `self.spawners.tick` belongs in that client tick (vanilla's frozen client doesn't tick block entities), and group 5's trial spawner pick sits inside `sim.tick()`, which frozen ticks skip. If a5 lands first, put the hook in its client tick block instead of after `:834`.
- `render_anim/src/minecraft_world.rs:1353-1361` (group 3): **Track E** moves player actions and menus out of `render_anim` (`docs/CRATE-SPLIT.md` section 6). If E lands first, put the `EggUse::Spawner` branch wherever the egg use went.
- `minecraft_terrain/src/server.rs`:
  - the `Command` enum, the handler match and `ServerSim::tick` are shared with A5 (`/tick` and overload) and Track E;
  - only additions are made: a variant, a handler arm and one call after `tick_spawners`.
- `minecraft_terrain/src/client_mobs.rs` (the `MobPose.turn` field) and `piglin_render.rs:334-343`: **Track B** adds piglin brutes, hoglins, zoglins and striders, possibly with new `MobPose` literals. Every new literal needs `turn: Quat::IDENTITY`. New renderers should get a match arm in `spawner_display::append` (one line each).
- `minecraft_terrain/src/ambient_particles.rs` and `mesh.rs` texture lists: Track C (End and visual leftovers) may add particle kinds and textures. These are append-only edits.
- `third_party/minecraftoss/world/src/level/spawning.rs` (group 5): vendored. If it was not already among the patched files, update the "patched N files" count in `CLAUDE.md`.
- `docs/MINECRAFT-SURVIVAL.md`: every track edits it. Keep the edits to the lines named above.

### Decisions (vanilla-like where there was a choice)
1. **Draw with the existing mob renderers.** Use a one-entity `EntityWorld` per spawner, loaded with `spawn_saved` (vanilla `loadEntityRecursive`). Add an outer `turn` in `MobPose::body_rotation` and a uniform post-scale about the feet. The alternative, splitting about 20 renderers into per-mob functions, was rejected: it is churn in files that Track B also edits.
2. The display is never ticked, so `age_in_ticks = partial`, as vanilla. Blaze rods twitch rather than turn.
3. The display is lit from the spawner cell (`light_probe = pos + 0.5`), as vanilla's `lightCoords` override. There is no shadow, no held item and no armour (vanilla's display has no `finalizeSpawn`).
4. **Delay sync.** A new display reads `Delay`. A changed `SpawnData` reads `Delay` (`load()`). Any other update for a known spawner is a spawn round, so the delay becomes `MinSpawnDelay` (block event 1 after the update packet, as `ServerLevel.tick` orders them). The server is unchanged for groups 1, 2 and 4.
5. **Near players.** A player counts when alive, not a spectator, and with feet within `RequiredPlayerRange` of the block centre. Only players in this level count.
6. **Showing rules.** Particles and spin run whenever `SpawnData.entity` has an `id`, even for a kind we cannot draw (silverfish, breeze). The model is drawn only for kinds `spawn_saved` knows, plus the cave spider.
7. **View distance** is 64 blocks from the camera to the block centre, as `BlockEntityRenderer.getViewDistance`.
8. **Spawn eggs** set the spawner's mob for any kind in any game mode that may build, refuse monster kinds in peaceful (as `canSpawn` does) and everything when `spawner_blocks_work` is off, and are spent only when the server accepts. Trial spawners are left out of egg use.
9. **Trial spawners.** The server owns the choice of `spawn_data`, which is saved and the same for every client. The client never picks.
