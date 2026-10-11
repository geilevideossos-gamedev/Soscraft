# Track A, item a1: end gateways (design brief)

Paths and lines are as of e2313625. Vanilla facts were checked with javap on the 26.3 `client.jar` (no code copied). State stays per `Level` (the gateway table) and per `Player` (the portal process and cooldown). Nothing here adds global or "player 0" state; the places that still go through `PLAYER = 0` today are named in section 6.

## 1. The owner's request

> End gateway (screenshot 1, `scrennshots/screenshots-e231362/screenshots-e231362/1_end_gateway_kein_teleport.png`): after the dragon dies, I can't get through the gateway to the outer islands. Gateways should teleport like vanilla when you walk or fly through them or throw an ender pearl in, to a gateway on the outer islands that is generated if needed.

Scope from the orchestrator also covers:
- the exit search and placement on the outer islands, and the terrain needed about 1000 blocks out (does the End generator already make it?);
- the gateway beam, the cooldown, the return gateway;
- saving `exit_portal` and `ExactTeleport` in the block entity.

The screenshot shows the gateway's starfield cube in its bedrock frame. Walking into it does nothing, because the player portal step only looks for nether portals and end portals.

## 2. What exists today

| Area | State |
|---|---|
| Portal kinds | `PortalKind::Gateway` exists with transition time 0 (`minecraft_survival/src/portal.rs:28-48`), but nothing ever sets it. |
| Player portal step | `portal::step` checks nether portals (`portal.rs:249-260`) and end portals (`:261-268`), using one feet box per tick with a fixed height of 1.8. It never sweeps the movement between ticks, and never checks gateways. |
| Travel adapter | `render_anim/src/minecraft_portal.rs:25-79`. It takes `levels: &BTreeMap` (read only), and has an End branch (`:58-62`) and a Nether branch (`:63-77`). |
| End touch helpers | `touched_end_portal` and `is_end_portal` (`minecraft_survival/src/the_end.rs:37-65`). `the_end::travel` (`:164`) only handles the end portal. |
| Same-dimension arrival | `Player::arrive(place)` (`player.rs:110-114`). Then `place_player` (`render_anim/src/minecraft_world.rs:2455-2488`) teleports with `teleport_facing`, which stops the player (`sim/src/frame.rs:388-397`): `place.motion` is never applied. It starts the `SpawnGate` when chunks are missing. |
| Gateway block entity | Its saved keys `Age` (long), `exit_portal` (int array) and `ExactTeleport` (byte, only when true) round-trip (`third_party/minecraftoss/core/src/block_entity.rs:241-249`). |
| Gateway feature | `EndGateway` places the 3x5x3 frame and calls `set_gateway_exit` (`generator/src/feature/kinds/the_end.rs:256-300`). `EndIsland` matches 26.3 (`:306-330`). |
| Live level features | `Level::place_configured_feature` uses the level random (`world/src/level/mod.rs:875-878`), and `set_gateway_exit` edits a loaded block entity and marks it changed (`:989-998`). Both only work in chunks the server level has loaded. |
| Ring gateways | After a kill, `ServerSim::dragon_killed` places `end_gateway_delayed` (no exit) at `end_gateway_position(i)` (`minecraft_terrain/src/server.rs:1824-1840`, `:2244-2252`), with the shuffled list in `the_end/dragon_fight.rs:136-146`, `:302-311`. |
| Gateway drawing | A starfield cube with hidden covered faces (`minecraft_terrain/src/end_portal_look.rs:6-24`; shader marker `render_gpu/src/drawsurf/minecraft_world.rs:975`). There is no beam and no particles: `display_ticks.rs:75-83` has no `end_gateway` arm. |
| Runtime gateway state | None. Nothing ticks `Age` or keeps a teleport cooldown. The block entity is never read at runtime. |
| Ender pearls | Simulated on the server thread (`server.rs:1087-1121`). A landing is reported as `(owner, pos)` (`:1044`, `:2794`), filtered to `PLAYER` (`minecraft_survival/src/entities.rs:919`), and applied to the local player in `minecraft_world.rs:1839-1849` (teleport, 5 damage, sound). |
| Chunk access on the main thread | `ChunkWorld` reads loaded chunks, then saved Full chunks, and generates with `load_chunk_now` only when `generates` is set (`minecraft_survival/src/portal_forcer.rs:305-360`). `ChunkMap::load_now` generates and decorates on the calling thread (`world/src/chunk_map.rs:585-611`). |
| Writing to unloaded chunks | `ChunkMap::set_blocks` and `set_block_entities` only keep edits for chunks in `self.chunks` (`chunk_map.rs:820-840`). `TerrainStream::record_edits` reads states from the scene (`minecraft_terrain/src/terrain.rs:1314-1324`), so a chunk missing from the scene would be written as air. |
| Server level chunks | The server thread only has the chunks the stream sent (`level.rs:200-205`, `server.rs:480-498`). Block entity changes flow back to the chunk map every stream tick (`level.rs:176-182`). |
| Outer End terrain | Generated: `end_outer_islands` is ported as `Df::EndIslands` (`generator/src/density/ir.rs:308`; `sampler.rs:118-139` matches 26.3's `EndIslandFunction`, which now starts at -100 with no centre term), `TheEndBiomeSource` is ported (`generator/src/terrain.rs:249-286`), and so are end cities (`structure/kinds/end_city.rs`). No test looks further out than chunk ±2 (`terrain.rs:2355-2405`). |
| Beam helpers | `end_crystal_render::beam_spans` splits a V range at texture repeats (`end_crystal_render.rs:118-131`). The entity mesh output has `models` and `translucent` lists (`entities.rs:1179-1240`). |

## 3. Vanilla 26.3 behaviour (javap)

**Entering** (`EndGatewayBlock.entityInside`, `Entity.setAsInsidePortal`, `Entity.handlePortal`, `Entity.isAffectedByBlocks`, `BlockBehaviour.getEntityInsideCollisionShape`)
- **The entity check:** the entity must pass `canUsePortal(false)`: alive and not a passenger.
  - The gateway must not be cooling down (`teleportCooldown > 0`).
  - Then `setAsInsidePortal(this, pos)` runs, and **`triggerCooldown`** with it: cooldown 40, block event (1, 0), `setChanged`.
- **What counts as inside:** the inside shape is the full block (`Shapes.block()`).
  - Blocks are visited along the tick's movement (`Entity.checkInsideBlocks` → `BlockGetter.forEachBlockIntersectedBetween`), so a fast entity can't skip the block.
  - Entities with `noPhysics` (spectators) are never inside a block.
- **The player's own cooldown:** on cooldown, `setAsInsidePortal` only refreshes it to 10 for players (`getDimensionChangingDelay`).
  - `EndGatewayBlock` doesn't override the transition time, so `handlePortal` teleports on the next portal tick.
  - It sets the player cooldown to 10 again.
- **The frame:** `EndGatewayFeature.place`, over x -1..1, y -2..2, z -1..1:
  - the gateway is at the centre;
  - the rest of the y 0 ring is air;
  - bedrock is directly above and below at ±2;
  - at ±1, bedrock forms a plus (dx = 0 or dz = 0), with air in the corners.
- So the only way in is the one-block-high side gap. That fits an elytra flyer, a pearl or a crawling pose. A prone MW2 player (30 units, under one 36-unit block) fits too.

**Destination** (`EndGatewayBlock.getPortalDestination`, `TheEndGatewayBlockEntity.getPortalPosition`)
- **No exit yet, in the End:**
  1. `pos = findOrCreateValidTeleportPos(level, gatewayPos).above(10)`.
  2. `spawnGatewayPortal(level, pos, EndGatewayFeature.knownExit(gatewayPos, false))`, which builds the return gateway with its exit back to this gateway, not exact.
  3. `setExitPosition(pos, this.exactTeleport)`.
- **No exit, outside the End:** returns null. There is no teleport, but the cooldowns still trigger.
- **With an exit:** the target is `exact ? exit : findExitPosition(exit)`, at `Vec3.atBottomCenterOf`.
- **Players** get `TeleportTransition(..., Vec3.ZERO, 0, 0, Relative.union(DELTA, ROTATION), PLACE_PORTAL_TICKET)`. Velocity, yaw and pitch are all relative zero, so all three are kept. There is no sound.
- **Ender pearls** (`instanceof ThrownEnderpearl`) get absolute velocity zero and rotation 0, 0. The pearl is moved there, then falls under gravity and lands as usual (`ThrownEnderpearl.teleport` also places a portal ticket).
- **Advancement:** `minecraft:end/enter_end_gateway`. The fork has no advancements, so it is skipped.

**Exit search** (`findOrCreateValidTeleportPos`, `findExitPortalXZPosTentative`, `isChunkEmpty`, `findValidSpawnInChunk`, `findTallestBlock`, `findExitPosition`)
1. **Direction:** `dir = normalize(gx, 0, gz)` (zero if the gateway is at 0, 0), and `p = dir * 1024`.
   - **Back:** up to 16 times, while the chunk at `p` (`floor(p/16)`) is **not** empty, `p -= dir*16`.
   - **Forward:** the counter resets to 16. Up to 16 times, while the chunk is empty, `p += dir*16`.
   - The check is `i-- > 0`, after the emptiness test.
2. **Empty chunk:** `getHighestFilledSectionIndex() == -1`, so every section is air. The chunk is fetched with `getChunk`, which loads or fully generates it.
3. **`findValidSpawnInChunk`** on the final chunk.
   - It walks `betweenClosed((minX, 30, minZ), (maxX, top, maxZ))`, where `top = highestFilledSection's minY + 15`. `betweenClosed` sorts each axis, so a top below 30 walks top..30. x varies fastest, then y, then z.
   - A spot is end stone, with no full-collision block (`isCollisionShapeFullBlock`) at +1 and +2.
   - It keeps the smallest `distToCenterSqr(0,0,0)` of the block centre, y included; a tie keeps the first found.
4. **No spot:**
   - `pos = BlockPos.containing(p.x + 0.5, 75, p.z + 0.5)`.
   - Place the configured feature `minecraft:end_island` at `pos` with `RandomSource.create(pos.asLong())` (a `LegacyRandomSource`). `EndIslandFeature`: `size = nextInt(3) + 4`; while `size > 0.5`, fill end stone where `x² + z² <= (size+1)²` for x, z in `floor(-size)..=ceil(size)`, then `size -= nextInt(2) + 0.5` and step y down one.
5. **Then, either way:** `findTallestBlock(level, pos, 16, true)`.
6. **`findTallestBlock(center, r, allowBedrock)`:**
   - Columns: dx from -r..=r (outer loop), dz from -r..=r (inner loop). The centre column is skipped when `!allowBedrock`.
   - Each column is scanned from `maxY` (255 in the End) down to, but not including, `best == null ? minY : best.y`.
   - The first block with a full collision shape is taken, unless bedrock is not allowed. It becomes the new best and that column stops.
   - It returns the best, or `center` when nothing is found.
7. **`findExitPosition(exit)`:** `findTallestBlock(exit + (0,2,0), 5, false).above()`.
   - Bedrock and the column under the gateway are skipped, so the player lands on the tallest end stone within 5, under the return gateway.
   - With nothing there, it is the exit + 3, on top of the frame's upper bedrock.
8. **`spawnGatewayPortal`:** `EndGatewayFeature.place` with an unseeded random, which the feature doesn't use.

**Ticking and cooldown** (`portalTick` on the server, `beamAnimationTick` on the client, `isSpawning`, `isCoolingDown`, `getSpawnPercent`, `getCooldownPercent`)
- **Each tick:** remember spawning and cooling, then `age++`.
  - If cooling: `teleportCooldown--`.
  - Otherwise, if `age % 2400 == 0`: `triggerCooldown`. This is the "attention" flash every 2 minutes.
  - If spawning or cooling flipped: `setChanged`.
- **States:** spawning is `age < 200`; cooling is `teleportCooldown > 0`; `COOLDOWN_TIME` is 40.
- **Saved and not saved:** `Age` is saved and `teleportCooldown` is not.
  - Block event 1 sets the client's cooldown to 40.
  - The client ages on its own; the update packet carries `Age` (`getUpdateTag` = `saveCustomOnly`).
- **Percentages:** `getSpawnPercent(p) = clamp((age + p) / 200, 0, 1)`, and `getCooldownPercent(p) = 1 - clamp((cooldown - p) / 40, 0, 1)`.

**Beam** (`TheEndGatewayRenderer.extractRenderState` and `submit`, `BeaconRenderer.submitBeaconBeam`)
- It is drawn only while spawning or cooling, and only when the level is known.
  - `scale = sin(π · (spawning ? spawnPercent : cooldownPercent))`.
  - `height = floor(scale · (spawning ? level.getMaxY() : 50))`. The End's `getMaxY` is 255.
  - The colour is `DyeColor.MAGENTA` diffuse (13061821) while spawning, else `PURPLE` (8991416).
  - `animationTime = floorMod(gameTime, 40) + partial`.
- **The beam call:** `submitBeaconBeam(pose, BEAM = textures/entity/end_portal/end_gateway_beam.png, textureScale = scale, animationTime, yOffset = -height, height = 2·height, colour, beamRadius 0.15, glowRadius 0.175)`.
  - It is centred at the block's x+0.5, z+0.5.
  - `t = -animationTime` (the height is ≥ 0), and `frac = frac(t·0.2 − floor(t·0.1))`. V starts at `-1 + frac`.
  - **Inner beam:** opaque (`beaconBeam(tex, false)`), with the four corners (0, r), (r, 0), (−r, 0), (0, −r) where r = 0.15. It is turned about Y by `animationTime·2.25 − 45` degrees, and V runs to `2·height·scale·(0.5/0.15) + v0`.
  - **Glow:** translucent (`beaconBeam(tex, true)`), colour alpha 32, an unturned square ±0.175, with V to `2·height·scale + v0`. Both are full bright.
- **View distance:** 256 blocks (`getViewDistance`).
- The cube itself (`submitCube`, `RenderTypes.endGateway`) is already drawn by round 7.

**Particles** (`EndGatewayBlock.animateTick`)
- The particle count is `getParticleAmount()`: the number of the 6 faces `shouldRenderFace` shows.
- **Each particle:**
  - Start at `pos + (rand, rand, rand)`, with velocity `((rand−0.5)·0.5)` on each axis.
  - `j = nextInt(2)·2 − 1`.
  - If `nextBoolean()`: `z = pos.z + 0.5 + 0.25·j` and `vz = nextFloat()·2·j`. Otherwise do the same on x.
  - The particle is `ParticleTypes.PORTAL`.
- The draws come in exactly this order.

**Worldgen gateways:** the placed feature `end_gateway_return` has rarity 1/700, `in_square`, `MOTION_BLOCKING` heightmap, a y offset of uniform 3..=9 and a biome filter, in `end_highlands`. Its config is exit (100, 50, 0), exact true. `end_gateway_delayed` is exact false with no exit.

## 4. Plan (one commit per group, in order)

New logic goes into new files: `minecraft_survival/src/the_end/gateway.rs` (main-thread search, build and travel) and `minecraft_terrain/src/end_gateway.rs` (server-thread table) plus `end_gateway_render.rs`. They must pass `rustfmt --check`. Hooks into existing files change only the lines they need, in the surrounding style. `minecraft_terrain` is edition 2021, so no let-chains.

**Where the work runs.**
- The exit search and the far writes run on the main thread through the `Level`'s `TerrainStream`, the only owner of the `ChunkMap`, which can generate. This is the same place `portal_forcer::arrive` builds nether portals.
- The cooldown, `Age` and the pearls run on the level's server thread (`ServerSim`), which owns block entities and thrown entities.
- They talk through the existing `Job` channel and the per-tick `EntitySnapshot`.

### Group 1. "The Minecraft outer End islands generate a thousand blocks out like vanilla"
- **Files:** a test in `minecraft_terrain/src/terrain.rs` (tests module next to `end_chunks_generate_the_main_island_around_the_origin`, `:2355`). Fix vendored generator code only if the test finds a defect, and log each patch.
- **Tests** (real data; skip without `MINECRAFTOSS_ROOT`, like the neighbouring tests):
  - `the_outer_end_islands_generate_past_the_void`, on an End stream at a fixed seed:
    - chunks along +x from 40 to 80, z 0..2, have no blocks between x 700 and 900 (the void ring) for that seed's sample;
    - there is end stone past x 1000 somewhere in a 16 x 16 chunk patch from chunk (64, -8) (islands are sparse, so a three-chunk strip can miss them on some seeds);
    - the column biomes include `end_highlands` and `small_end_islands`;
    - `chorus_plant` grows on highlands.
  - `outer_end_gateways_keep_their_exact_exit`: search a few hundred highland chunks for an `end_gateway` block entity. If one is found, it has `exit_portal` [100, 50, 0] and `ExactTeleport` 1. If the sample finds none, the test asserts the placed feature parses with rarity 700.
  - `end_island_height_matches_vanilla`: `sampler::end_island_height` at about 6 section coordinates (outer and inner) for seed 0, against values the implementer computes once with `jshell` on the 26.3 jar (`EndIslandFunction`). Note the jshell command in the track log.
- **Done when:** the tests pass and the generation time per far chunk is noted in the track log. It feeds the decision about blocking in group 3.

### Group 2. "Minecraft end gateways find their exit on the outer islands like vanilla"
- **Files:** new `minecraft_survival/src/the_end/gateway.rs`; `the_end.rs` (`pub mod gateway;`).
- **Contents, pure and over a trait:**
  - `trait GatewayWorld`:
    - `chunk_is_empty((cx, cz)) -> bool`;
    - `highest_filled_section_top((cx, cz)) -> Option<i32>`;
    - `is_end_stone(pos)`;
    - `full_block(pos)`, a single box from 0,0,0 to 1,1,1 out of `registries.blocks.collision_boxes`;
    - `is_bedrock(pos)`;
    - `min_y()` and `max_y()`.
  - `impl GatewayWorld for portal_forcer::ChunkWorld<'_>` (its `chunk` and `state` are already public). A section counts as empty when `section.is_empty()`.
  - `exit_xz_tentative(world, gateway) -> [f64; 3]`, `valid_spawn_in_chunk`, `tallest_block(world, center, radius, allow_bedrock)`, `find_exit_position(world, exit)`.
  - `find_or_create_teleport_pos(world, gateway) -> (pos, Option<island_origin>)`.
  - `end_island_blocks(origin) -> Vec<pos>`, using `LegacyRandom::new(BlockPos::pack(origin))`.
  - `gateway_frame(origin) -> Vec<(pos, Frame::{Gateway, Bedrock, Air})>`.
  - `GatewayExit { exit, exact }` and `fn destination(exit, world) -> [f64; 3]`, at the bottom centre.
- **Tests:**
  - `the_exit_search_starts_1024_out_along_the_gateway_s_direction`: gateway 6 at (−30, 75, 91) gives about (−320.6, 0, 972.5). Confirm the digits with jshell.
  - `a_full_chunk_steps_back_and_an_empty_one_steps_forward_sixteen_times_at_most`, on a fake world.
  - `the_spawn_spot_is_the_end_stone_nearest_the_origin_with_two_free_blocks_above` (y counts; a tie keeps the first in x, y, z order; the y range runs from 30 to the top section).
  - `no_spot_places_a_seeded_end_island_at_75`: `end_island_blocks((-320, 75, 972))` matches the vendored `EndIsland` feature placed with the same `LegacyRandom` seed.
  - `the_tallest_block_skips_bedrock_and_the_middle_only_when_asked`.
  - `a_loose_exit_lands_on_the_tallest_block_within_five_or_three_above_the_gateway`.
  - `the_gateway_frame_is_a_bedrock_plus_with_an_open_side_ring`.
  - Real data: `a_ring_gateway_finds_an_outer_island` (End stream, `generates = true`, `end_gateway_position(0)`). The spot is end stone at least 768 blocks out, and the test logs the time taken.

### Group 3. "Minecraft end gateways keep their age and cool down like vanilla"
- **Files:** new `minecraft_terrain/src/end_gateway.rs`; `minecraft_terrain/src/lib.rs`; `server.rs` (a `gateways` field on `ServerSim`, with hooks in `load_chunk` `:480`, `unload_chunk` `:1209` and the tick at `:2767`, the snapshot at `:2837` and a new field on `EntitySnapshot` `:211`); `minecraft_survival/src/entities.rs` (keep `gateways` from the snapshot, as with `crystals` at `:908`).
- **`Gateways`:** a table `pos → Gateway { age: i64, cooldown: i32, exit: Option<(i32, i32, i32)>, exact: bool }`.
  - It is filled from `end_gateway` block entities when a chunk loads, and emptied when it unloads.
  - It is updated for positions in the level's block changes and block entity changes each tick, which covers runtime features and `/setblock`.
- **`tick()`:** `portalTick` as in section 3.
  - `Age` is written into the block entity's NBT in place each tick.
  - The position is marked changed (`changed_block_entities`, so the chunk map and the save get it) only when spawning or cooling flips, and when a cooldown is triggered, as vanilla's `setChanged`.
- **Other methods:** `trigger_cooldown(pos) -> bool` (false when already cooling), `set_exit(pos, exit, exact)` (through the level's `World::set_gateway_exit`, which marks it changed, and the table), and `views() -> Vec<GatewayView { pos, age, cooldown, exit, exact }>`.
- **Jobs:** `ServerSim::trigger_gateway(pos)` and `ServerSim::set_gateway_exit(pos, exit, exact)` are public for jobs.
- **Main thread:** `Entities::gateway(pos) -> Option<&GatewayView>`, plus `Entities::mark_cooling(pos)`, a per-level list cleared when the next snapshot shows that gateway cooling. This stops a second player from going in during the job round trip.
- **Tests:**
  - `a_gateway_spawns_for_200_ticks_and_flashes_every_2400`;
  - `a_triggered_gateway_cools_for_40_ticks_and_ignores_a_second_trigger`;
  - `age_is_saved_and_a_flip_reports_the_block_entity` (reading `Age` 150 back, it stops spawning after 50 ticks);
  - `gateways_join_and_leave_with_their_chunks_and_setblock`;
  - `setting_an_exit_writes_vanilla_keys` (`exit_portal` int array, and `ExactTeleport` only when true).

### Group 4a. "Minecraft players keep their speed when they arrive somewhere"
- **Files:** `render_anim/src/minecraft_world.rs` `place_player` (`:2455`) only.
- Split from group 4 because it changes every arrival (nether and end portals, joins with a saved `Motion`), not only gateways.
- **Change:** after `teleport_facing`, push `place.motion` back as MW2 velocity (`[mx, −mz, my] · BLOCK · 20`, the inverse of the conversion at `:2471`).
- **Tests:** a pure helper for the conversion, round-tripping with the `motion` written at `:2471`; zero motion leaves the player still.
- Note for tracks B and C in the track log.

### Group 4. "Minecraft players travel through end gateways"
- **Files:**
  - `minecraft_survival/src/portal.rs`: `step` takes a gateway check, sweeps the box, and `Portal` gains `last_feet`;
  - `the_end/gateway.rs`: `travel`;
  - `minecraft_terrain/src/terrain.rs`: a new `TerrainStream::write_states`;
  - `render_anim/src/minecraft_portal.rs`: `levels` becomes `&mut`, plus a gateway branch;
  - `render_anim/src/minecraft_end.rs`: a `gateway` adapter with a diag line;
  - `render_anim/src/minecraft_world.rs:723`, the call site (group 4a already applies `place.motion`).
- **Touch:**
  - `touched_gateway(from, to, is_gateway)` sweeps the 0.6 × 1.8 box from last tick's feet to now, in steps of at most 0.25. It hits any `end_gateway` cell the box overlaps (a full cube).
  - `last_feet` is cleared when an arrival is placed, or when the step is longer than 8 blocks, so a teleport is never swept.
  - It is skipped for spectators (`noPhysics`), for the dead and for remote players.
  - If the gateway is not cooling (`Entities::gateway` and `mark_cooling`): `set_inside(Gateway)`, send the job `trigger_gateway(pos)`, and `mark_cooling(pos)`.
  - The nether overlay (`effect.tick`) stays nether only.
- **Due:** `gateway::travel(level, player, pos)`.
  1. **Exit:** read it from the gateway's view, or from the chunk map's block entity at `pos`. If there is none and the level is the End:
     - run `find_or_create_teleport_pos` with `ChunkWorld { generates: true }`, add 10, and build the island (if any) and `gateway_frame`;
     - write them with `write_states` and a gateway block entity `{id: "minecraft:end_gateway", Age: 0L, exit_portal: [I; source]}` through `record_block_entities`;
     - send the job `set_gateway_exit(source, new, source.exact)`, plus `command_place` jobs for any edited cell the server level has loaded.
  2. **Destination:** `exact ? exit : find_exit_position(exit)`, at the bottom centre.
  3. **Arrival:** `player.arrive(Place { dimension: End, feet, yaw and pitch kept, motion kept, on_ground: false })`. `place_player` then teleports, starts the `SpawnGate` if the island's chunks aren't streamed, and with this change pushes the kept motion back as MW2 velocity (`[mx, −mz, my] · BLOCK · 20`).
  4. **Outside the End without an exit:** nothing happens, apart from the cooldowns (vanilla).
  5. The diag line is `Minecraft end gateway at {pos} leads to {feet} (exit {exit}{, built})`.
- **`write_states(scene, &[(BlockPos, BlockStateId)])`:**
  - For each chunk: `load_now` it if it isn't in the chunk map (a chunk read from disk by `ChunkWorld` is not in `self.chunks`, and its edits would be dropped).
  - Then `server.set_blocks` with the given states.
  - For cells whose chunk is in the scene: `scene.set` and `mark_edited`.
  - Don't use `record_edits` here: it reads the scene and would write air to far chunks.
- **Tests:**
  - `portal`: `a_box_moving_through_a_gateway_between_ticks_touches_it`, `a_teleport_is_not_swept`, `spectators_and_cooling_gateways_are_not_entered`, `the_nether_overlay_ignores_gateways`.
  - `gateway` (real data, End stream at a fixed seed):
    - `a_first_trip_builds_one_return_gateway_pointing_back`: the return gateway's block entity holds the source, not exact; the source job is queued; a second trip reads the exit and builds nothing.
    - `an_exact_exit_lands_on_it_and_a_loose_one_on_the_tallest_block`.
    - `far_edits_survive_the_chunk_leaving_memory`: write to a far chunk, run `server_tick` around 0, 0 until it is evicted, reload it, and the blocks and the block entity are there.
  - `player`: `a_gateway_trip_keeps_speed_and_facing`.

### Group 5. "Minecraft ender pearls fly through end gateways"
- **Files:** `minecraft_terrain/src/server.rs` (`tick_thrown` `:1087`, plus an `Output.pearl_gateways` next to `pearl_teleports` `:2395`); `minecraft_survival/src/entities.rs` (collect them per owner, beside `:919`); `the_end/gateway.rs` (`pearl_landing`); `render_anim/src/minecraft_world.rs` (feed the result into the existing landing path at `:1839`).
- **Server side:**
  - After a pearl moves, sweep its 0.25 box from its old position to its new one, up to its hit point.
  - The first `end_gateway` cell that isn't cooling triggers that gateway's cooldown. The pearl is removed and `(owner, gateway_pos)` is reported.
  - A cooling gateway lets the pearl fly on.
- **Main thread:**
  - `pearl_landing(level, gateway) -> Option<[f64; 3]>` resolves the destination as in group 4, creating the exit if needed.
  - Then it drops straight down: with zero velocity the pearl falls from there, so it takes the first full-collision block below and lands on top. It returns `None` when it reaches `min_y − 64` (lost in the void, with no teleport).
  - The owner's landing then runs the existing code (teleport, 5 damage, `entity.player.teleport`).
- **Decision:** vanilla carries the pearl over (zero velocity, portal ticket) and lets it fall over later ticks. Here the landing is worked out at once. The place is the same, and the server never has to tick a pearl in unloaded far chunks.
- **Tests:**
  - `server`: `a_pearl_through_a_gateway_is_reported_and_triggers_its_cooldown`, `a_cooling_gateway_lets_a_pearl_through`, `a_pearl_that_hits_a_wall_first_lands_normally`;
  - `gateway`: `a_pearl_lands_on_the_first_block_below_the_exit_or_is_lost_in_the_void`.

### Group 6. "Minecraft end gateways shine their beam and give off portal particles"
- **Files:** new `minecraft_terrain/src/end_gateway_render.rs`; `minecraft_terrain/src/lib.rs`; `mesh.rs:503-504` (register `minecraft:entity/end_portal/end_gateway_beam` as an entity texture); `minecraft_survival/src/entities.rs:1232` (`append_beams(&mut out.models, &mut out.translucent, &self.gateways, atlas, partial, game_time)`); `minecraft_terrain/src/display_ticks.rs:75` (an `end_gateway` arm).
- **The beam:**
  - Build it as in section 3, with fullbright light. The inner beam goes in `models`, the glow in `translucent`.
  - V ranges split at texture repeats with `end_crystal_render::beam_spans`.
  - Use the server's `age` and `cooldown` with the frame's partial tick. Skip gateways more than 256 blocks from the camera.
- **Particles:** the face count comes from the scene neighbours, using the same face-hiding rule as `end_portal_look`. The draws come in vanilla's order and are pushed to `portal_particles`.
- **Tests:**
  - `the_spawn_beam_rises_to_the_build_height_and_falls_over_ten_seconds`: height at ages 0, 100 and 199 is 0, 255 and about 4; magenta.
  - `the_cooldown_beam_is_purple_and_fifty_high_at_its_peak`.
  - `the_beam_scrolls_and_turns_with_game_time`.
  - `a_gateway_in_its_frame_gives_four_particles_and_a_lone_one_six`.

### Group 7. "Docs: Minecraft end gateways"
- `docs/MINECRAFT-SURVIVAL.md`: an end gateway entry under the End; the X6 line (`:282`) loses "end gateways …" and keeps item frames, shulkers, levitation and flashes; any new limitation (only players and pearls go through).
- The track log entry.

## 5. Owner test

`.env`: the round 7 End test world with the dragon already killed (`IW4L_MINECRAFT_WORLD=endtest`, as in screenshot 1), plus this kit (9 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:ender_pearl*16,minecraft:ender_pearl*16,minecraft:elytra,minecraft:firework_rocket*64,minecraft:end_stone*64,minecraft:cobblestone*64,minecraft:diamond_pickaxe+efficiency=5,minecraft:golden_apple*16,minecraft:diamond_boots+feather_falling=4
```

1. Fly or bridge to the gateway from the dragon kill: a starfield cube in a bedrock frame at height 75, about 96 blocks from the centre. Purple portal particles drift from its open sides. From a few blocks away, throw a pearl into its side gap.
   - The gateway flashes a short purple beam (about 2 s).
   - You land on end stone on an outer island well over 900 blocks from the centre. A loading screen may show first.
   - A new gateway in a bedrock frame floats 10 blocks above the highest block nearby, flashing purple.
   - The log has `Minecraft end gateway at … leads to …`.
   - **Screenshot 1.**
2. Build up with end stone next to the new gateway and go prone in its side gap (crawl in). You come back to the main island next to the first gateway, standing on the highest end stone within 5 blocks, or on the frame's top bedrock if there is none. **Screenshot 2.**
3. Put on the elytra, glide off the island's edge with a rocket, and fly into the first gateway's side gap. You come out at the same outer island still gliding, at the same speed and facing the same way. **Screenshot 3.**
4. Right after a trip, while the purple beam still shows, throw a pearl through the gateway you came out of. The pearl flies straight through without teleporting anything.
5. On the main island, about 20 blocks east of 0, 0, type `/setblock ~2 ~ ~ minecraft:end_gateway`.
   - A gateway appears with a tall magenta beam that grows up to the sky and shrinks again over 10 seconds. **Screenshot 4.**
   - Walk into it (it has no frame): you go to the outer islands east of the centre, and a return gateway is built there.
6. Wait 2 minutes near a gateway: it flashes its purple beam by itself.
7. Quit to the menu and start again. Use the first gateway again: you arrive at the same island under the same return gateway, and no second return gateway is built.
8. Optional, with vanilla 26.3 Java: make a world with the same seed (`/seed`), `/setblock` a gateway at the same block as in step 5, and walk in. Your position (F3) matches the `leads to` coordinates the log printed for step 5, to the block.

## 6. Risks, shared files and decisions

**Risks**
- **Blocking generation:** the first trip through a gateway generates up to about 33 chunks along the search line, plus their neighbours, on the main thread. Vanilla does the same on its server thread. Group 1 measures it.
  - If it is over about 2 s, keep it synchronous but hold the `SpawnGate` loading screen before the search starts (one frame later), rather than adding a worker thread.
  - With friends (track E), it stalls the host's tick for everyone, as in vanilla.
- **Chunk map edits:**
  - `set_blocks` drops edits to chunks outside `self.chunks`, so `write_states` must `load_now` first.
  - An evicted chunk must keep its edits (`Evicted { edits, entities }`); group 4 tests it.
  - Don't route these writes through `record_edits`.
- **Two copies of block entities:** the server level and the chunk map. The far return gateway exists only in the chunk map until the player arrives and the chunk streams into the server. The source gateway's exit must go through the server job, not straight into the chunk map, or the next server flush overwrites it.
- **Main-thread gateway view** is a snapshot one tick old. Hence `mark_cooling`; without it, two players could go through in the same tick.
- **Momentum on arrival:** applying `place.motion` in `place_player` (group 4a) also changes nether and end portal arrivals and joins. Vanilla keeps momentum there too, but tell track B (Nether) and C (End) in the log.
- **Order with a5 (overload and `/tick`):** a5 moves the level thread's tick body into `run_tick` and adds frozen ticks. If a5 merges first, group 3's `Gateways::tick` hook and group 5's pearl sweep go inside `run_tick` and run only on normal ticks (a frozen gateway neither ages nor cools down, a frozen pearl doesn't move), and group 3's tests send `Tick` with its flag list. Player touches (group 4) still work while frozen, as vanilla players are never frozen, but the cooldown job then waits until the game runs.

**Shared with other round 8 tracks**
- `minecraft_terrain/src/server.rs` (B, C, E): the hooks only, with the logic in `end_gateway.rs`.
- `minecraft_survival/src/entities.rs` (B, C, E): the snapshot field, `pearl_gateways` and the render call.
- `render_anim/src/minecraft_portal.rs`, `minecraft_world.rs` (`place_player`, the portal call, pearl landings) and `minecraft_end.rs`: **E** moves player actions out of `render_anim` and runs the step for every player. Keep the gateway logic in `minecraft_survival` so E only moves the call.
- `minecraft_survival/src/portal.rs` (E).
- `the_end.rs` (C works on the rest of X6: item frames, shulkers, levitation).
- `display_ticks.rs` and `mesh.rs` texture registration (B and C add mobs and particles).
- **C must not also do the gateway beam;** it is in this item.

**The tick boundary (for track E)**
- Per-player state:
  - `Portal.last_feet`, the portal process and `portal_cooldown` live on `Player`;
  - the touch runs in `portal::step` for one player;
  - travel ends in `Player::arrive`.
- Per-level state: the `Gateways` table is in each `ServerSim`, the view list and `mark_cooling` are in each `Level`'s `Entities`, and the search uses that `Level`'s stream.
- **Still player 0:** pearl owners are the server's `u64` player ids, and landings are filtered by `PLAYER` (`entities.rs:36`, `:919`) and applied to the local player (`minecraft_world.rs:1839`). Report `(owner, gateway)` and leave the owner-to-UUID mapping to E's pearl work.
- **Networking:** `GatewayView` is plain data (pos, age, cooldown, exit, exact), so E can send it as is.

**Decisions taken (vanilla-like)**
1. Every rule in section 3 is followed: the search, the island seed, the 40-tick cooldown, the 2400-tick flash, the player cooldown of 10, no sound, `Age` saved and the cooldown not.
2. Only players and ender pearls go through gateways in this item. Vanilla also takes items, mobs and other projectiles. Add that as a limitation in the state doc.
3. A prone MW2 player fits the one-block gap, so "walking through" is crawling in. Elytra, pearls and an unframed `/setblock` gateway work as in vanilla.
4. Touching is swept between ticks (vanilla `forEachBlockIntersectedBetween`) so fast elytra flight can't skip the block.
5. A pearl's landing is worked out at once from a straight drop (group 5).
6. The return gateway and the island are written into the chunk map and saved, so a restart finds them. A gateway with an exit never searches again.
7. The beam is drawn as entity geometry from server views, with no block event. Vanilla's block event 1 only syncs the cooldown, which the views carry.
8. The advancement `end/enter_end_gateway` is not done (the fork has no advancements).
