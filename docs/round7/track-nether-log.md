# Track NETHER log: the Nether

Branch `nether`, worktree `C:\Dev\mc-wt\nether`. Briefs: `e2-nether.md` (commits E2-1 to E2-30) and `e0-phase3-plan.md` (groups N1-N5). Base: 3aef2eb (the dimension switch merged).

A commit can't name its own hash, so each entry is written with its commit and its hash is filled in by the next commit; the last entry's hash is in `git log` under its subject.

## Group N1: a Nether level that runs

E2-1, E2-2, E2-3, E2-5 and E2-12. E2-4 (no sky, sun, stars or clouds) moved to the End track's X1b.

### 1. Minecraft levels know their dimension's rules (E2-1)

Commit: f39da09

- **Changed:**
  - New `minecraft_survival/src/dimension.rs`: `DimensionRules` read from the 26.3 data pack for a `Dimension`: `coordinate_scale`, `min_y`, `height`, `logical_height`, `has_skylight`, `has_ceiling`, `nether_cardinal_light` (`cardinal_light` is `nether`), the `gameplay/bed_rule` as a `BedRule` (`can_sleep`, `can_set_spawn` as `BedRuleTest` always, when dark or never, `destroy_on_use`, `destroy_on_leave`, the error message's translation key), `respawn_anchor_works`, `water_evaporates`, `fast_lava` and `nether_portal_spawns_piglin`.
  - `minecraft_terrain/src/environment.rs`: `dimension_value(system, name)` is vanilla's `EnvironmentAttributeReader.getDimensionValue`: the dimension type's value over the default, without any biome layer.
  - `minecraft_survival/src/level.rs`: `Terrain.rules`, loaded by `Terrain::open` on the loader thread. A dimension whose rules can't be read fails to open like one whose environment can't.
- **Tests:** `dimension`: `a_dimension_type_gives_its_scale_heights_lighting_and_gameplay_rules` (synthetic Nether-like type), `a_dimension_without_a_bed_rule_sleeps_when_dark_and_sets_the_spawn` (the default `BedRule.CAN_SLEEP_WHEN_DARK`, and the three `BedRule$Rule` tests), `the_three_vanilla_dimensions_have_their_own_rules` (the real 26.3 data under `MINECRAFTOSS_ROOT`: overworld 1:1, 384 high from -64, beds sleep when dark, portal piglins; Nether 8:1, logical height 128, cardinal light, beds and straw beds explode, anchors work, water evaporates, fast lava; End 1:1, 256, beds explode). `environment`: `a_dimension_value_ignores_the_biomes`.
- **Decisions:**
  - The plan names `Level.rules`; the rules sit on `Level.terrain.rules` instead, because `Terrain::open` is where a level can still fail to open and because the portal search in N2 runs in `prepare`, which only gets the `Terrain`.
  - The gameplay rules are read as dimension values, as vanilla reads them (`getDimensionValue` for `fast_lava`, checked with javap on `LavaFluid`). No 26.3 biome overrides any of them.
- **Test steps:** nothing changes in game; the rules are used by the next entries and by N2 and N5.
- **Kit:** none.

### 2. Minecraft Nether lava flows faster and its sea level is 32 (E2-2)

Commit: 24c89d4

- **Changed:** `minecraft_terrain/src/server.rs` (`ServerSim::new` only): the level's `sea_level` is its chunk generator's (`ServerLevel.getSeaLevel` reads `NoiseGeneratorSettings.seaLevel`: 63 in the overworld, 32 in the Nether, 0 in the End), and `fast_lava` is the dimension's `gameplay/fast_lava` value (true in the Nether), so Nether lava ticks every 10 ticks instead of 30 and loses one level a block instead of two (the vendored `fluid.rs` already had both numbers). Before, every level kept 63 and slow lava.
- **Tests:** `server`: `nether_lava_spreads_farther_and_faster_and_the_nether_sea_is_at_32` (a Nether `ServerSim` has sea level 32 and fast lava, an End one 0 and slow lava; a lava source on a stone floor reaches 3 blocks in the overworld and 7 in the Nether after 300 ticks, and after 75 ticks at most 2 in the overworld but 6 or more in the Nether).
- **Test steps** (after N2, or now with `IW4L_MINECRAFT_DIMENSION=the_nether` and a fresh world, e.g. `IW4L_MINECRAFT_WORLD=n1`):
  1. Start in the Nether, build a 17x17 cobblestone floor and pour the lava bucket in its middle.
  2. The lava spreads 7 blocks out from the source within about 4 seconds. **Screenshot 1.**
  3. In the overworld the same pour spreads only 3 blocks, and slower.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:lava_bucket,minecraft:bucket
```

### 3. Minecraft Nether terrain and structures generate in a test (E2-3)

Commit: 955de45

- **Changed:** a test only; nothing had to be fixed (S1's E1-4 already ran the Nether generator around 0,0).
- **Tests:** `terrain`: `nether_fortresses_bastions_and_every_nether_feature_generate` (seed 77, real 26.3 data):
  - `fortress`, `bastion_remnant`, `nether_fossil` and `ruined_portal_nether` are registered with a generator of their own, and no placed feature of the five Nether biomes is `Feature::Unsupported`.
  - `/locate`'s search (track C) finds the nearest fortress at (-144, -416) and bastion at (320, 0). The 5x5 chunks around each generate with nothing above the roof and no overworld or End block: the fortress has nether bricks, nether brick fences and a spawner whose block entity spawns blazes; the bastion has polished blackstone bricks, blackstone, gilded blackstone and a `chests/bastion` loot chest.
  - The bastion's generated mobs are piglins and piglin brutes, all with `PersistenceRequired`, so the dormant despawn (`server.rs`, `despawn_mobs`) keeps them (the brief's risk); the fortress generates no mob.
  - A Nether `ServerSim` loads all of those chunks and ticks 40 times.
- **Decisions:** the test checks the top-level placed features only; features nested in selectors run in the block checks.
- **Test steps** (with `IW4L_MINECRAFT_DIMENSION=the_nether`, creative is easiest):
  1. `/locate structure minecraft:fortress`, then `/tp` to the answer at y 70. Nether brick bridges and halls stand there. **Screenshot 2.**
  2. `/locate structure minecraft:bastion_remnant`, then `/tp` there: blackstone walls with gold. **Screenshot 3.** (Its piglins wait dormant until N6.)
- **Kit:** none needed; any kit works.

### 4. Minecraft Nether blocks and mobs are lit like vanilla's Nether (E2-5)

Commit: e87a02b

- **Changed:**
  - `minecraft_terrain/src/terrain.rs`: `BlockStates.nether_shade`, set by `TerrainStream::for_dimension_at` when the dimension type's `cardinal_light` is `nether`. Each section build (`compile_section`) hands it to its worker thread.
  - `minecraft_terrain/src/mesh.rs`: `face_shade(direction)` is vanilla's `CardinalLighting.byFace`: DEFAULT is down 0.5, up 1.0, north and south 0.8, west and east 0.6; NETHER (checked with javap) is up and down 0.9. Block faces (`bake_quads`, `append_block`, `block_preview`) and fluids use it: a fluid's top is `up`, its bottom `down`, and its sides `up` times `north` or `west`, as 26.3's `FluidRenderer` does. So Nether lava sides are 0.72 and 0.54 instead of 0.8 and 0.6. Section workers read their own level's flag; other block meshes (falling blocks, open lids, the placement preview) follow the player's level.
  - `minecraft_terrain/src/cow_render.rs`: `nether_entity_lighting()`. `mesh.rs`'s dropped-item shading (`level_item_shade`) is now the entity shade, so items on the ground take the Nether's second light from below too (`Lighting.updateLevel` sets the level entry to `NETHER_DIFFUSE_LIGHT_0/1`, checked with javap).
  - `render_anim/src/minecraft_world.rs` (one call line): every frame sets `set_nether_entity_lighting` from the player's level's `rules.nether_cardinal_light`. It existed since upstream but was never called, so mobs, armour, orbs and items in the Nether are now lit from below instead of from above.
- **Tests:** `mesh`: `nether_faces_are_shaded_from_above_and_below_alike` (a cube's six faces give 0.5, 1.0, 0.8, 0.8, 0.6, 0.6 by default and 0.9, 0.9, 0.8, 0.8, 0.6, 0.6 in the Nether; the fluid side products). `terrain`: the Nether stream has `nether_shade`, the End stream not.
- **Not done:** the MW2 player and weapon models keep the default two lights (`render_gpu`'s `mc_entity_shade`): telling that shader the dimension needs a slot in the environment uniform, which the End track owns with the sky shader this round.
- **Test steps** (with `IW4L_MINECRAFT_DIMENSION=the_nether`):
  1. Look at a netherrack overhang: its underside is as bright as the floor (both 0.9), where in the overworld an underside is half as bright as a floor. Walls facing north and south stay a little darker than the floor, walls facing east and west darker still. **Screenshot 4.** (Corrected in entry 5.)
  2. Drop a stack of cobblestone and look at it on the ground: its bottom and top are lit alike.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:netherrack*64
```

### 5. Minecraft explosions can start fires (E2-12)

Commit: 99b4cd5

- **Changed:**
  - Vendored `third_party/minecraftoss/world/src/level/explosion.rs` (MinecraftOSS patch): `ServerExplosion.createFire`. An explosion with `fire` no longer logs "fire from explosions" as unsupported: after the blocks break, each picked position draws `random.nextInt(3)`, and on 0 an air block over a solid-render block becomes `BaseFireBlock.getState` (soul fire on soul fire bases, else fire), set with `setBlockAndUpdate`. As in vanilla (javap), `interactWithBlocks` shuffles the picked list in place, and the fire pass walks that shuffled list.
  - Vendored `third_party/minecraftoss/entities/src/world.rs` (MinecraftOSS patch): `EntityWorld::blast(world, blast, players)`, the public entity-world blast: `apply_explosion` takes an optional source, so a blast nobody caused hurts and pushes every mob (`Blaster::Nobody`, no kill credit) and hands each player in reach an explosion hit with no attacker. A creeper's blast is unchanged.
  - `minecraft_terrain/src/server.rs`: `Blast { center, power, fire }`, `ServerSim::blast` and `Command::Blast` (`ServerHandle::blast`). Blasts run at the end of the mob tick, after the creepers': the entity world hurts the mobs and players, their loot drops, then `Level::explode` breaks the blocks with the `block_explosion_drop_decay` game rule (`ExplosionInteraction.BLOCK`, now read in `apply_game_rules`) and starts the fires; the client hears it like a creeper's.
  - `minecraft_survival/src/entities.rs`: `Entities::explode(Blast)` sends it (`Entities::blast` was already the MW2 hit on one mob).
- **Tests:** `server`: `a_blast_hurts_mobs_and_players_breaks_blocks_and_may_start_fires` (a power 5 blast with fire on an obsidian floor: the client hears one blast, the player takes an explosion hit of more than 10 with no attacker, a zombie is hurt, the planks beside it break, and fire lands on the obsidian; the same blast without fire starts none). The vendored `minecraftoss-world` and `minecraftoss-entities` tests pass with the full suite's stand-in files.
- **Decisions:**
  - Plan decision 5: beds, anchors and crystals hurt players and mobs through this blast (vanilla explosion damage), not the MW2 blast rule. The player's death cause for a bed (`bad_respawn_point`, "Intentional Game Design") comes with E2-13 in N5; until then a hit with no attacker reads as a plain explosion.
  - The blast waits for the next server tick's mob step, because the players' bodies it needs come with the tick.
- **Test steps:** nothing in game makes a fiery blast yet (beds and anchors come in N5). Check that creepers and TNT still blow up as before:
  1. Place TNT on grass and light it: it explodes, breaks blocks and starts no fire. **Screenshot 5.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:tnt*4,minecraft:flint_and_steel
```

## Group N1 summary

- **Commits:** f39da09 (E2-1), 24c89d4 (E2-2), 955de45 (E2-3), e87a02b (E2-5), 99b4cd5 (E2-12), and this log entry.
- **Checks:** `test -p minecraft_survival` (258 and 1 passed), `test -p minecraft_terrain` (639 passed, 3 ignored as before), `test -p render_anim` (67 passed), `minecraftoss-world` and `minecraftoss-entities` with the full suite's stand-in files (123 passed), and `check -p launcher` are green. `clippy -p minecraft_survival -p minecraft_terrain -p minecraftoss-world -p minecraftoss-entities --tests --no-deps -- -A clippy::approx_constant` has no finding on a line of this group (the older ones in `mesh.rs`, `server.rs`, `entities.rs` and the mob renderers stay).
- **MinecraftOSS patches in this group:** `world/src/level/explosion.rs` (fire from explosions, the in-place shuffle), `entities/src/world.rs` (the public `EntityWorld::blast`).
- **Handed on:** `Terrain.rules` (`coordinate_scale`, `logical_height`) for N2's portal search and build; `BedRule`, `respawn_anchor_works` and `Entities::explode` for N5's beds and anchors (with the `bad_respawn_point` death cause); `ServerSim::blast` for the End's crystals (X3), which will want a source and attacker.
- **Not done:** the MW2 player and weapon models keep the overworld's two lights in the Nether (entry 4).
- **Owner test for the whole group** (fresh world `IW4L_MINECRAFT_WORLD=n1` with `IW4L_MINECRAFT_DIMENSION=the_nether`; portals come with N2): entry 2 steps 1-2, entry 3 steps 1-2, entry 4 step 1, entry 5 step 1 (in the overworld: start once without the dimension line).
- **Kit (8 stacks):**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:netherrack*64,minecraft:lava_bucket,minecraft:tnt*4,minecraft:flint_and_steel
```

## Group N2: portal travel

E2-6 to E2-11.

### 6. Minecraft players wait in nether portals like vanilla (E2-6)

Previous commit: 6a3ceba

- **Changed:**
  - New `minecraft_survival/src/portal.rs` (owned by this track; the End track fills only the End arm):
    - `PortalKind { Nether, End, Gateway }` with `transition_time(invulnerable, rules)`: `NetherPortalBlock.getPortalTransitionTime` (checked with javap) is the game rule `players_nether_portal_creative_delay` (0) for an invulnerable player and `players_nether_portal_default_delay` (80) otherwise, never below 0. End portals and gateways use `Portal`'s default 0 until the End track fills their arm.
    - `PortalProcess { kind, entry, time, inside }` and `Portal::set_inside` / `Portal::tick`, vanilla's `Entity.setAsInsidePortal` and `Entity.handlePortal` with `PortalProcessor` (javap): on cooldown, touching a portal only resets the cooldown to `DIMENSION_CHANGING_DELAY` (10, `Player.getDimensionChangingDelay`); otherwise it starts the process or flags it inside this tick, moving its entry only on a tick that starts outside. Each tick the cooldown drops by one; inside, `portalTime++ >= transitionTime` triggers (so 81 ticks in survival, the first tick in creative) and sets the cooldown to 10; outside, the time drops by 4 and the process is dropped at 0.
    - `touched_portal(feet, axis_at)`: the player's 0.6 x 1.8 box, deflated by 1e-5 as `Entity.checkInsideBlocks`, against the portal block's shape (6/16 to 10/16 across its axis, `NetherPortalBlock.SHAPES`). `portal_axis_in(terrain, pos)` reads the `axis` of a `minecraft:nether_portal` in the level's scene.
    - `step(player, terrain, rules, feet, seconds)`: the player's own 20 Hz clock (`Portal.clock`), so it never depends on the mob tick; a dead player, one in a remote killstreak (the passenger analogue of `canUsePortal`) or one with no body neither touches nor counts.
  - `minecraft_survival/src/player.rs`: one field, `Player.portal` (the cooldown was already `Player.portal_cooldown`, saved as `PortalCooldown`).
  - New `render_anim/src/minecraft_portal.rs` and one call line in `minecraft_world.rs` after the commands: each frame the local player's portal steps with the presented feet. While the player is switching the frame returns before it, so the cooldown and the timer freeze behind the loading screen, as vanilla's `ServerPlayer.processPortalCooldown` skips while `isChangingDimension`. For now a finished wait is only logged (`Minecraft player stood long enough in the Nether portal at ...`); travel comes with entry 10.
- **Tests:** `portal`: `a_survival_player_travels_after_eighty_ticks_and_a_creative_one_at_once` (81 ticks, 1 tick, a game rule of 20 gives 21), `stepping_out_takes_four_ticks_off_a_tick_and_forgets_the_portal_at_zero` (41 inside, 5 outside leaves 21, the rest counts), `the_cooldown_is_refreshed_inside_and_blocks_travel_until_it_runs_out` (300 ticks in the arrival portal start nothing, then 10 ticks outside), `a_player_who_cannot_use_portals_waits_without_counting`, `the_entry_moves_only_on_a_tick_that_starts_outside_and_another_kind_starts_afresh`, `a_player_touches_a_portal_only_through_its_thin_middle`, `the_clock_hands_out_whole_ticks`.
- **Decisions:** the process runs on the player (by UUID) and ticks on its own clock in the adapter's frame, not inside a level's mob tick, so a level that is not ticking (the player is held behind the loading screen) never advances it.
- **Test steps:** nothing visible yet; travel and the overlay come with entries 10 and 11, whose steps cover this entry.
- **Kit:** none.

### 7. Minecraft portal travel scales coordinates by eight (E2-7)

Previous commit: 118bb95c

- **Changed:** `minecraft_survival/src/portal.rs`, all checked with javap on the 26.3 jar:
  - `destination(feet, scale)`: `NetherPortalBlock.getPortalDestination` scales x and z by `DimensionType.getTeleportationScale` (the data pack's `coordinate_scale`s through `switch::teleport_scale`, 1/8 into the Nether, 8 out of it) and keeps y, then `WorldBorder.clampToBounds` with the default border (centre 0, size 59,999,968: from -29,999,984 to 29,999,984 less 1e-5) and `BlockPos.containing`. `within_border` is `WorldBorder.isWithinBounds`. `nether_portal_leads_to`: the Nether leads to the overworld, anything else to the Nether.
  - `largest_rectangle_around(center, axis, 21, 21, matches)`: `BlockUtil.getLargestRectangleAround` with its per-column bounds and `getMaxRectangleLocation`'s histogram stack, ties kept by the first (taller) rectangle as vanilla's strict comparisons do.
  - `relative_position`: `PortalShape.getRelativePosition` for the player's 0.6 x 1.8 box (where along the portal, how high, and how far in front of its middle). `exit_position` and `PortalTravel::exit`: `NetherPortalBlock.createDimensionTransition`: the same place in the exit portal, the yaw turned by 0 when both portals share an axis and by 90 when they don't, the pitch kept, and the momentum kept and turned with the yaw (`Relative.DELTA` includes `ROTATE_DELTA`; `PositionMoveRotation.calculateAbsolute` turns it by `Vec3.yRot(-turn)`).
  - `PortalTravel` (kind, target dimension, target block, entry axis, relative place, yaw, pitch, motion, and whether the trip may build a portal: not for a spectator, as `getExitPortal`) and `depart(player, terrain, registries, due)`, which measures the entry portal in the player's level (`getDimensionTransitionFromExit`; a player no longer in a portal block counts as axis X at the middle, as vanilla).
  - `render_anim/src/minecraft_portal.rs`: a finished wait now logs where the portal leads (`Minecraft portal at ... leads to minecraft:the_nether near (100, 70, -200)`).
- **Tests:** `portal`: `eight_hundred_blocks_out_is_a_hundred_in_the_nether_and_back` (and the border clamp at both edges), `the_largest_rectangle_is_found_around_a_block_of_any_portal_shape` (a 4x5 portal, an L shape from four starting blocks, a lone block, the 21-block limit, the histogram), `the_player_keeps_its_place_in_the_portal_and_turns_with_its_axis` (relative place, clamping, a portal too narrow, the exit on the same and the other axis, the yaw, the turned momentum).
- **Decisions:** the world border is vanilla's default; the fork has no `/worldborder` yet, so a later border command only needs to replace `within_border` and `clamp_to_border`.
- **Test steps:** nothing visible yet (entry 10).
- **Kit:** none.

### 8. Minecraft finds the nearest nether portal like vanilla (E2-8)

Previous commit: b39687f9

- **Changed:**
  - New `minecraft_survival/src/portal_forcer.rs`:
    - `PortalWorld` (a portal block's axis, the portal blocks of a chunk) and `find_closest(world, target, radius)`: `PortalForcer.findClosestPortalPosition` (javap): every `nether_portal` block in the square of `radius` around the target at any height (`PoiManager.getInSquare` keeps `|dx| <= radius` and `|dz| <= radius`), inside the world border, whose block still has an axis; the smallest `distSqr` wins and a tie goes to the lowest Y. `search_radius`: 16 into the Nether, 128 anywhere else.
    - `ChunkWorld`, the search's view of a level: a chunk is the chunk map's loaded chunk with its edits, or else the saved FULL chunk from the region file, and never generated (`generates` stays off for the search). A section is scanned only when it holds anything, and its palette tells at once when it holds no portal.
  - `minecraft_terrain/src/terrain.rs`: `TerrainStream::loaded_chunk` (the chunk map's chunk with its edits, nothing loaded or generated) and `TerrainStream::load_chunk_now` (the chunk map's `load_now`, for the portal build of entry 9).
- **Tests:** `portal_forcer`: `the_search_looks_sixteen_blocks_into_the_nether_and_a_hundred_and_twenty_eight_out` (a portal 16 blocks away is found, 17 not; 3x3 chunks are read into the Nether and 17x17 into the overworld), `the_closest_portal_block_wins_and_a_tie_goes_to_the_lowest`, `broken_portals_and_portals_past_the_border_are_skipped`, `a_level_s_chunks_are_searched_as_loaded_or_saved_and_never_generated` (a real Nether stream of seed 77: a search over chunks nobody loaded finds nothing and generates nothing; two lit portal blocks recorded into a loaded chunk are found with their axis).
- **Decisions:** plan decision 7: no POI files; the search reads loaded and saved chunks and never generates, as vanilla's `PoiManager.ensureLoadedAndValid` loads the square at `ChunkStatus.EMPTY` and rebuilds missing POIs from the blocks. A chunk the chunk map evicted moments ago whose save has not reached the region file yet is not seen (a gap of the save thread's queue; the portal is found once it is written).
- **Test steps:** nothing visible yet (entry 10).
- **Kit:** none.

### 9. Minecraft builds a nether portal when none is near (E2-9)

Previous commit: 08d856f3

- **Changed:** `minecraft_survival/src/portal_forcer.rs`:
  - `create_portal(world, pos, axis, min_y, top)`: `PortalForcer.createPortal` step by step (javap). `build_top` is `min(maxY, minY + logical_height - 1)`: 127 in the Nether, 319 in the overworld. It walks `BlockPos.spiralAround(pos, 16, EAST, SOUTH)` (`spiral_around`, the same leg state machine, 33x33 columns from the middle out) and goes down each column from the `MOTION_BLOCKING` height (capped at the top): a run of blocks that can be replaced (`canBeReplaced` and no fluid) ending on its floor, at least 4 below the top, not a 1 or 2 block gap, whose frame fits (`canHostFrame`: solid under the 4 columns, replaceable above, for the portal's own row and both rows beside it). The closest spot with room on both sides wins; else the closest spot where only the frame's row fits; else the fallback at `clamp(y, max(minY + 1, 70), top - 9)` one block before the target along the axis, clamped into the border, with a 3x2 obsidian floor and air above it. Then the 14-block obsidian frame and the 2x3 portal of the entry's axis. `None` only when the fallback has no room (vanilla logs "Unable to create a portal").
  - `BuildWorld` (replaceable, solid, `MOTION_BLOCKING` height) for `ChunkWorld`, which with `generates` set loads and generates chunks through `TerrainStream::load_chunk_now`, as vanilla's `getHeight` and `getBlockState` load the chunk at FULL. The height is read from the blocks with the registries' heightmap masks, so a chunk's edits always count.
  - `build(terrain, placed)`: the blocks go into the level's scene, into the chunk map (`record_edits`, so the region file saves them and the server level gets the chunk with them when it loads it) and to the section rebuild (`mark_edited`). On the loader thread no chunk has been sent yet, so the chunks reach the client and the server already built.
- **Tests:** `portal_forcer`: `the_build_spiral_walks_out_from_the_middle_like_spiral_around` (radius 1 in order, 33x33 unique columns, the build tops), `a_portal_is_built_on_the_nearest_floor_with_room_around_it` (a flat floor for both axes with the 14 frame blocks and 6 portal blocks, and a one-block ledge taken when nothing better is near), `a_portal_over_water_or_under_the_nether_roof_stands_on_its_own_platform` (a lake gives the platform at 70 one block back; a cave room is used; a room touching the 127 roof is refused and the platform goes to 118; no room at all builds nothing), `a_portal_is_built_in_real_nether_terrain_below_the_roof` (seed 77: built under the roof within 17 blocks, the scene shows it, and a fresh non-generating search over the chunk map finds it with its frame).
- **Decisions:** none new; the build happens where the trip is resolved (entry 10).
- **Test steps:** nothing visible yet (entry 10).
- **Kit:** none.

### 10. Minecraft players travel through nether portals (E2-10)

Previous commit: f837e707

- **Changed:**
  - `minecraft_survival/src/portal.rs`: `Portal.travel` (the trip being resolved) and `Portal.travel_sound`. `set_off(player, travel)` starts the switch with `Player::travel_to` (so it waits for a remote killstreak like any trip) and the loading reason of the pair (`NETHER_PORTAL`). `may_enter`: the `allow_entering_nether_using_portals` game rule, which vanilla's `ServerLevel.isAllowedToEnterPortal` checks for the Nether only.
  - `minecraft_survival/src/portal_forcer.rs`:
    - `arrive(terrain, travel)`: vanilla's `getExitPortal` in the target level: the closest portal (entry 8) measured with `largest_rectangle_around`, or else, unless the player is a spectator, a new one (entry 9, built into the level with `build`); then the exit spot (entry 7) made free with `collision_free`, `PortalShape.findCollisionFreePosition`: the player's centre may move within its own width and one block up, and the closest free point of that box (the box minus every block collision box grown by half the player) wins, as `CollisionGetter.findFreePosition` and `VoxelShape.closestPointTo` do, cut on the boxes' edges.
    - `travel_into(players, dimension, registries)`: the trip of the first player waiting for that level. A player waiting for the Nether with no trip and no arrival (only `IW4L_MINECRAFT_DIMENSION=the_nether` does that) comes as vanilla's `getDimensionTransitionFromExit` brings an entity that is not in a portal block: from where it stands, scaled by 1/8, axis X, the middle of the portal.
    - `arrive_through_portals(server)`: a trip into a level that is already open (a return within its 15 seconds, or a level opened for a search) is resolved on the main thread once the player has left the old level, so behind the loading screen; blocks it builds also go to the open level's server as a job (`ServerSim::command_place` with the strict flags), so a level that is not ticking gets them too. A trip whose player is no longer bound there is dropped.
  - `minecraft_survival/src/server.rs` (`open_levels` and `finish_opening` only): a level opened for a trip takes the trip's target block as its known spawn (no stand-in search) and resolves the trip in `prepare` on the loader thread, so the portal search, the build and the new chunks are done before the scene streams; a player whose own trip is still waiting is not given another player's arrival.
  - `minecraft_terrain/src/terrain.rs`: the stand-in `nether_spawn` search is gone. A Nether stream with no known spawn (only a level opened for `/locate`) keeps (0, 64, 0) and generates nothing for it.
  - `render_anim/src/minecraft_portal.rs`: a finished wait measures the entry portal (`depart`), honours the game rule and sets off; `arriving` plays `block.portal.travel` once the arrival is known (volume 0.25, pitch 0.8 to 1.2, vanilla's level event 1032 after `PLAY_PORTAL_SOUND`). `minecraft_world.rs` has two more call lines: `arrive_through_portals` in `follow_levels` after the levels finish opening, and `arriving` in the switch's hold before the player is placed.
- **Tests:** `portal_forcer`: `an_exit_spot_inside_a_wall_moves_to_the_nearest_free_place` (an exit 0.1 into a wall moves to touch it, on either axis; a free spot stays; a spot with no free place nearby stays as vanilla keeps it). `server`: `a_portal_trip_builds_its_exit_and_the_next_trip_finds_it` (seed 1: a trip from (800.5, -1599.5) opens the Nether with (100, 70, -200) as its known spawn, builds a Z portal and puts the player in it with the yaw and pitch kept; a second player from an X portal while the Nether is open waits until it has left, then lands in the same portal turned by 90; a debug start into the Nether comes through a portal at (-5, 64, 10) for a player standing at (-40, 64, 80); the End takes no portal trip). `terrain`: the Nether stream test now checks that no stand-in spawn is searched (nothing generated, (0.5, 64, 0.5)).
- **Decisions:**
  - With the game rule off, vanilla still computes the destination (and may build the exit portal) before refusing the teleport; here the trip is refused before anything is searched or built, so nothing appears in a level nobody can reach.
  - The trip's momentum is kept and turned with the yaw in the arrival `Place`, but the MW2 movement starts the player at rest after the loading screen, as every teleport does.
  - `IW4L_MINECRAFT_DIMENSION=the_nether` with no saved place in the Nether now arrives through a portal at the scaled position of where the player stood (vanilla's not-in-a-portal fallback), so the Nether has no stand-in spawn left.
  - Portal piglins (`NetherPortalBlock.randomTick` spawning zombified piglins in overworld portals): not done, as zombified piglins come with N3 (E2-20); that commit adds them.
- **Test steps** (a new world; the purple overlay and the trigger sound come with entry 11):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=n2` and no `IW4L_MINECRAFT_DIMENSION` line. Start. Type `/gamemode creative`, `/tp 800 120 -1600`, fly down to the ground and type `/gamemode survival`.
  2. Build an obsidian frame 4 wide and 5 high (a 2x3 opening), light it with the flint and steel, and walk into it. Stand still: after about 4 seconds the purple portal loading screen shows "Loading terrain...", then you stand in a new obsidian portal in the Nether and hear the portal travel whoosh. **Screenshot 1.**
  3. Type `/tp ~ ~ ~`: the chat shows about (100, y, -200). **Screenshot 2.**
  4. Stay in the arrival portal for 10 seconds: nothing happens. Step out, wait a second, step back in: after about 4 seconds you are back in the overworld, standing in the portal you built. **Screenshot 3.**
  5. Walk 10 blocks away and back into the same portal: you arrive in the same Nether portal, no second one is built.
  6. In the Nether, walk at least 40 blocks away, build and light a second portal and go through: the overworld gets a new portal near 8 times those coordinates (about 320 blocks or more from the first). **Screenshot 4.**
  7. `/gamemode creative` and walk into a portal: you travel at once. `/gamemode survival` afterwards.
  8. `/gamerule allow_entering_nether_using_portals false` in the overworld: standing in a portal does nothing. Set it back to `true`.
  9. Quit, set `IW4L_MINECRAFT_WORLD=n2start` (a new world) and `IW4L_MINECRAFT_DIMENSION=the_nether`, start: you stand in an obsidian portal in the Nether (no portal was there before; it was built for you). **Screenshot 5.** Remove the dimension line afterwards.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*64,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:torch*64
```

### 11. Minecraft nether portals tint the screen and sound when you step in (E2-11)

Previous commit: eeceab2e

- **Changed:**
  - `minecraft_survival/src/portal.rs`: `PortalEffect`, `LocalPlayer.handlePortalTransitionEffect` (javap): each tick inside a nether portal (the `CONFUSION` transition) the intensity rises by 0.0125, outside it falls by 0.05, between 0 and 1; stepping in at 0 triggers the sound. `shown(partial)` lerps the last two ticks as `Gui` does, and `overlay_alpha` is `extractPortalOverlay`'s `i^4 * 0.8 + 0.2` below 1. The effect follows touching the portal, not the trip's process, as vanilla's client does, so the arrival portal tints the screen too while its cooldown keeps the player from going back. `step` now returns `Stepped { due, triggered, inside }`, `Portal.age` counts the ticks for the animation, and `set_off` resets the effect, as vanilla starts a new `LocalPlayer` in the new level.
  - `frame/src/minecraft_ui.rs` (appended): `MinecraftUi.portal_frames` (the `block/nether_portal` animation strip, loaded once with the world by a call line in `minecraft_world.rs`) and `MinecraftUi.portal_overlay: Option<McPortalOverlay { alpha, frame }>`.
  - New `hud/src/minecraft_portal.rs` (with its line in `hud/src/lib.rs`, `gpu_list.rs` and `plugin.rs`): one quad over the whole screen with the strip's current frame (one a tick, as the sprite animates), white with the overlay's alpha, drawn above the fire overlay and below the hotbar, in first and third person alike (vanilla draws it outside the first-person camera overlays).
  - `render_anim/src/minecraft_portal.rs`: `block.portal.trigger` at volume 0.25 and pitch 0.8 to 1.2 (`SimpleSoundInstance.forLocalAmbience`), container screens close while inside (`Screen.isAllowedInPortal`; the chat stays open, as `ChatScreen` allows it), and the overlay value each frame. The portal loading screen behind a trip is S3's `NETHER_PORTAL` background (E1-9).
- **Tests:** `portal`: `the_overlay_fades_in_over_eighty_ticks_and_out_over_twenty_and_sounds_from_zero` (the sound only from 0, full after 80 ticks, gone 20 ticks after, the lerp, the alpha curve). `hud::minecraft_portal`: `the_portal_overlay_covers_the_screen_with_one_animation_frame`.
- **Decisions:** none new.
- **Test steps** (the world `n2` of entry 10 and its kit; these also cover entries 6 to 10):
  1. Stand in front of a lit portal and step in: a faint purple swirl covers the screen at once and the portal trigger sound plays; the swirl grows stronger over about 4 seconds. **Screenshot 6** (about 2 seconds in).
  2. Step out after about 2 seconds: the swirl fades away within half a second. Step back in: the sound plays again.
  3. Stand in the portal and press E: the inventory closes at once. Press T: the chat stays open (close it again).
  4. Stay in the portal: at full purple the portal loading screen comes, then the Nether. You arrive in the exit portal: the travel sound plays, then the trigger sound and the swirl start again while you stand there, and you do not go back. **Screenshot 7.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*64,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:torch*64
```

### 12. Minecraft blocks changed before a trip keep their collision when the player comes back

Previous commit: 70ae2991

- **Defect:** a level shown again within its 15 seconds (S3's `TerrainStream::reshow`) handed the MW2 collision the scene's chunks as they were first sent, without the scene's edits. So after a portal trip there and back, blocks the player had placed in that level (the portal frame, a pillar, a floor) had no collision, and blocks they had broken were invisible walls, until the chunk was sent again. Portal round trips make such quick returns the normal case.
- **Changed:** `minecraft_terrain/src/terrain.rs`: `reshow` hands back each chunk with the scene's placed and cleared blocks written into a copy (`apply_scene_edits`); the scene's own chunks and the section rebuild are unchanged.
- **Tests:** `terrain`: `a_level_shown_again_sends_its_chunks_and_light_again` now breaks a block and places obsidian before the reshow: the chunks handed back have the air and the obsidian, and the scene's chunk is left alone. Checked to fail before the fix.
- **Test steps** (the world `n2` and the kit of entry 10):
  1. In the overworld, build a 3-block cobblestone pillar beside your portal and dig a 2-deep hole in front of it.
  2. Go through the portal, step out of the Nether portal, wait 2 seconds, step back in and return within 15 seconds.
  3. Back in the overworld, jump onto the pillar: it holds you. Walk into the hole: you drop into it. **Screenshot 8.**
- **Kit:** the kit of entry 10.

### 13. Minecraft portal travel passes clippy

Previous commit: 245fdc4b

- **Changed:** the two clippy findings on this group's lines: `portal_forcer::arrive` returns a named `Arrived` (`type_complexity`), and the adapter's trigger sound takes the sounds without a needless `as_deref_mut`.
- **Clippy:** `clippy -p minecraft_survival -p minecraft_terrain -p render_anim -p hud -p frame --locked --tests --no-deps -- -A clippy::approx_constant` has no other finding on a line of this group; the rest are older (`entities.rs`, `fluid.rs`, `minecraft_inventory.rs`, `minecraft_sounds.rs` and many more).
- **Tests:** none new.
- **Test steps:** none; nothing changes in game.
- **Kit:** none.

## Group N2 summary

- **Commits:** 118bb95c (E2-6), b39687f9 (E2-7), 08d856f3 (E2-8), f837e707 (E2-9), eeceab2e (E2-10), 70ae2991 (E2-11), 245fdc4b (the collision fix of entry 12), and this entry.
- **Checks:** `test -p minecraft_survival` (279 and the vanilla world test), `test -p minecraft_terrain` (639 passed, 3 ignored as before), `test -p render_anim` (67), `test -p hud` (131), `test -p frame` (16) and `check -p launcher` are green; clippy as in entry 13.
- **MinecraftOSS patches in this group:** none.
- **Handed on:**
  - The End track's X2 fills `PortalKind::End` (and `Gateway`) in `portal.rs`: their transition time (0 today, vanilla's `Portal` default) and their trip; `Portal::set_inside` and `Portal::tick` already take any kind, and `set_off` takes any `PortalTravel`.
  - N3 (E2-20, zombified piglins) adds the portal piglins of `NetherPortalBlock.randomTick` (`nether_portal_spawns_piglin`, `nextInt(2000) < difficulty`, outside peaceful); they need the zombified piglin first.
  - Multiplayer: every piece of state is on the `Player` (by UUID) or the level, and `arrive_through_portals` and the loader serve every player; only the adapter steps the local player's portal so far.
- **Known gaps:**
  - A spectator who finds no portal arrives at the scaled target point; vanilla does not teleport a spectator then (the search runs after the loading screen has started here).
  - The trip's momentum (kept and turned in the arrival `Place`) is not handed to the MW2 movement; the player starts at rest after the loading screen, as with every teleport.
  - A portal in a chunk evicted from memory moments ago, whose save has not reached the region file yet, is not found until it is written.
- **Owner test for the whole group** (one new world, one kit; the screenshots are numbered across the entries):
  1. Entry 10 steps 1 to 3 (to the Nether through a portal you build at 800, -1600), with entry 11 step 1 while you wait in the portal.
  2. Entry 11 steps 2 and 3 in the Nether arrival portal's neighbourhood (build a second lit portal there if you like, or use the arrival portal after its cooldown: step out first).
  3. Entry 11 step 4 and entry 10 step 4 (back to the overworld portal).
  4. Entry 12 steps 1 to 3 (a quick return keeps your blocks solid).
  5. Entry 10 steps 5 to 9 (the same portal again, a new overworld portal from 40 blocks away in the Nether, creative, the game rule, and the debug start).
- **Kit (6 stacks):**

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*64,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:torch*64
```

## Group N3: Nether mobs on existing AI

E2-18 to E2-22, then E2-25.

### 14. Minecraft mobs can be immune to fire (E2-18)

Previous commit: 2f111564

- **Changed:**
  - Vendored `third_party/minecraftoss/entities/src/world/hazards.rs` (MinecraftOSS patch): `fire_immune(kind)`, the types whose `EntityType.Builder.fireImmune` is called in 26.3's `EntityTypes` (checked with javap): area effect cloud, blaze, ender dragon, end crystal, ghast, magma cube, shulker, strider, TNT, vex, warden, wither, wither skeleton, zoglin and zombified piglin. The hazards ask each mob's `Exposed::fire_immune` (zombies and skeletons by their kind's type ID; the other mobs are never immune): as `Entity.baseTick` does, a burning immune mob's fire is cleared without damage; `isInvulnerableTo` turns every `#is_fire` hazard away (in fire, on fire, lava, hot floor, campfire); fire and lava do not set it alight (`BaseFireBlock.fireIgnite`, `Entity.lavaIgnite`); and lava neither hurts it nor plays the burn sound (`Entity.lavaHurt`). `world.rs` re-exports `fire_immune`.
  - Vendored `third_party/minecraftoss/world/src/natural_spawner/spawning.rs` (MinecraftOSS patch): `MobCategory::of_type` knows wither skeletons, piglin brutes and zoglins as monsters, so the caps count them and peaceful refuses them.
- **Tests:** `hazards`: `the_nether_mobs_and_the_bosses_are_immune_to_fire`, `a_fire_immune_mob_puts_its_fire_out_and_shrugs_off_fire_but_not_cactus`. `spawning`: `every_nether_monster_has_the_monster_category`.
- **Decisions:** the End's endermites, dragon and wither stay out of `of_type` here; the End track adds them with their mobs.
- **Test steps:** nothing visible yet; the fire-immune mobs come with entries 16, 17 and 19, whose steps cover this entry.
- **Kit:** none.

### 15. Minecraft fortresses spawn their own monsters (E2-19)

Previous commit: dfb165d1

- **Changed:**
  - Vendored `third_party/minecraftoss/world/src/natural_spawner.rs` and `natural_spawner/spawning.rs` (MinecraftOSS patch): natural spawning's `mobsAt` now sees structures, as 26.3's `NaturalSpawner.mobsAt` and `ChunkGenerator.getMobsAt` do (checked with javap):
    - `isInNetherFortressBounds`: a monster spot whose block below is nether bricks, inside the bounds of a fortress start, draws from `NetherFortressStructure.FORTRESS_ENEMIES` (blaze 10 for 2-3, zombified piglin 5 for 4, wither skeleton 8 for 5, skeleton 2 for 5, magma cube 3 for 4; javap of its static block).
    - Otherwise every structure referencing the spot's chunk whose `spawn_overrides` (read from each `worldgen/structure` file of the data pack) has the category replaces the biome's list when the spot is inside one of the start's pieces (`bounding_box: piece`, `structureHasPieceAt`) or its whole bounds (`full`). So fortress pieces spawn the same five, swamp huts witches and cats, monuments guardians, outposts pillagers, and ancient cities and trial chambers nothing.
    - `CreatureSpawns::kinds_at`, the list a spot draws from, for tests.
  - `SpawnContext.structures`, `StructureHere` (a structure referencing the spot's chunk: inside a start's bounds, inside one of its pieces) and `NaturalSpawning.structures`, set by the server.
  - Vendored `third_party/minecraftoss/world/src/level/spawning.rs` (MinecraftOSS patch): `structures_of(worldgen)`, the server level's finder: the generator's structure references of the chunk (`Structures::references`, the starts within eight chunks, which are a pure function of the seed and already made for the chunks around the player), kept per chunk.
  - `minecraft_terrain/src/server.rs` (`ServerSim::new` only): natural spawning gets the finder.
- **Tests:** `server`: `fortress_monsters_spawn_on_its_nether_bricks_and_structures_override_their_biome` (seed 77's fortress: a spot in a piece draws the five on netherrack, a spot inside the bounds but outside every piece draws them on nether bricks and the biome's list on netherrack, a spot 2000 blocks off sees no structure, creatures are not overridden).
- **Decisions:** structures referencing a chunk are checked in registry order (vanilla walks a hash map, so any order is as good). Blazes, magma cubes, zombified piglins and wither skeletons the fortress draws wait dormant until their own entries (16, 17 and 19, and N4 for blazes) make them live.
- **Test steps** (after entry 18, with `IW4L_MINECRAFT_DIMENSION=the_nether`; see the group summary): in a fortress at night-like darkness, wither skeletons, zombified piglins and magma cubes appear on its bridges and halls.
- **Kit:** none.

### 16. Minecraft zombified piglins (E2-20)

Previous commit: cdcaf137

- **Changed** (all checked with javap on the 26.3 jar: `ZombifiedPiglin`, `Zombie`, `NetherPortalBlock`, `AdultPiglinModel`, `BabyPiglinModel`, `AbstractPiglinModel`, `PlayerModel.createMesh`, `ZombifiedPiglinRenderer`):
  - Vendored `third_party/minecraftoss/entities` (MinecraftOSS patch):
    - `zombie.rs`: `ZombieKind::ZombifiedPiglin`: 0.6 x 1.95 with eyes at 1.79 (a baby's at 0.78), 5 attack damage, never converts in water, sounds `entity.zombified_piglin.*`, steps like a zombie. It does not burn in daylight (corrected in entry 18).
    - `monster_ai.rs`: `MonsterKind::ZombifiedPiglin`: `Zombie.registerGoals` with `ZombifiedPiglin.addBehaviourGoals` (the spear goal at 1, `ZombieAttackGoal` at 2, the stroll at 7), and as targets `HurtByTargetGoal` with `setAlertOthers`, players only when it is angry at them (`NearestAttackableTargetGoal(Player, 10, true, false, isAngryAt)`), and `ResetUniversalAngerTargetGoal` (universal anger is off). Lava costs 8 on its paths. `zombified_piglin_step` is `customServerAiStep`: the first anger sound 0 to 20 ticks after it first takes a target (`entity.zombified_piglin.angry` at twice the volume and 1.8 times the pitch), `NeutralMob.updatePersistentAnger` (20 to 39 seconds, as the enderman's), and `maybeAlertOthers` every 4 to 6 seconds while it sees its target.
    - `world.rs`: the zombie tick runs that step, adds the 0.05 speed of an angry adult, says `entity.zombified_piglin.angry` instead of its ambient sound while angry, alerts the piglins within its follow range (35) and 10 up or down that have no target (only its own kind, as `getEntitiesOfClass(ZombifiedPiglin)`), and hits for 5 plus the golden sword's 3. A player's kill may drop its weapon (8.5%, worn at random as a bow is) and its experience counts 1 to 3 more for each item it wears or holds (`Mob.getBaseExperienceReward`).
  - Vendored `third_party/minecraftoss/world` (MinecraftOSS patch):
    - Natural spawning knows the type: fire immune, `checkZombifiedPiglinSpawnRules` (not in peaceful, not on a nether wart block, no darkness test), the monster walk target, follow range 35. `finalizeSpawn` is the zombie's with `ZombifiedPiglin.populateDefaultEquipmentSlots` (a golden spear one time in 20, else a golden sword, and no armour) and `randomizeReinforcementsChance` (0, with no draw).
    - Portal piglins (`NetherPortalBlock.randomTick`, in `level/random_tick.rs` and `level/spawning.rs`): with `spawn_mobs` and `spawn_monsters` on, not in peaceful, where the dimension's `gameplay/nether_portal_spawns_piglin` holds (the overworld), `nextInt(2000) < difficulty`, a player within 128 blocks of the chunk, it walks down the portal to its floor and, when a zombified piglin may spawn on that block, summons one standing on it (`EntitySpawnReason.STRUCTURE`, a random yaw) with `PortalCooldown` 300.
  - `minecraft_terrain/src/server_mobs.rs`: `minecraft:zombified_piglin` is simulated (spawn eggs, `/summon`, natural spawns), and its `anger_end_time` is loaded and saved.
  - New `minecraft_terrain/src/piglin_render.rs`: `AdultPiglinModel` (the player mesh with its jacket, sleeves and trousers, the 10-wide piglin head with snout and tusks, and the ears) and `BabyPiglinModel`, with `zombified_piglin.png` and `zombified_piglin_baby.png` from the pack, the zombie arms (`AnimationUtils.animateZombieArms`), the ears' flap (`AbstractPiglinModel.setupAnim`: 30 degrees for adults, 5 for babies), the held weapon and adult armour. `zombie_render.rs` hands the kind to it; `mesh.rs` loads the two textures.
- **Tests:** `world` (entities): `a_zombified_piglin_leaves_players_alone_until_one_hits_it_and_then_its_group_turns_on_them` (a hit piglin and one 8 blocks off turn on the player, one 60 blocks off stays calm), `a_zombified_piglin_puts_out_its_fire_and_hits_harder_with_its_golden_sword`. `piglin_render`: `a_zombified_piglin_has_the_piglin_head_ears_and_the_player_body_with_its_outer_layer` (16 cubes and 384 vertices for an adult, 9 cubes for a baby), `its_ears_hang_out_at_thirty_degrees_and_flap_as_it_walks`. `server`: `zombified_piglins_hold_gold_keep_their_anger_and_come_out_of_overworld_portals` (a summoned one holds gold and no armour, its anger survives a save, an overworld portal on hard makes about 30 in 20000 random ticks standing on the portal floor with a portal cooldown of 300, none in peaceful or with nobody near).
- **Decisions:**
  - Who a piglin was angry at is not saved, as for endermen, golems and wolves here; only how long its anger lasts. After a reload it calms down unless hit again.
  - Only the zombified piglin's golden sword adds to its damage; the other zombies' held weapons stay as before (a gap of the zombie port, not of this entry).
  - "A player close enough for spawning" for portal piglins is any non-spectator player within 128 blocks of the chunk's centre (`ChunkMap.anyPlayerCloseEnoughForSpawningInternal`), without the distance manager's tri-state shortcut.
- **Test steps** (any world; creative is easiest for the eggs):
  1. In the overworld, `/difficulty normal`, use 4 zombified piglin spawn eggs close together and walk among them: they ignore you, golden swords in hand. **Screenshot 11.**
  2. Hit one with the sword (or shoot it): all four turn on you with an angry grunt and chase you for a while. **Screenshot 12.**
  3. Push one into lava or set it on fire with flint and steel: it does not burn or lose health.
  4. Kill them in survival: rotten flesh and gold nuggets drop, sometimes a gold ingot or the sword.
  5. Portal piglins: build and light a nether portal, `/difficulty hard` and `/gamerule random_tick_speed 1000`, stand near: within about a minute zombified piglins step out of the portal. Set `/gamerule random_tick_speed 3` again. **Screenshot 13.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:zombified_piglin_spawn_egg*4,minecraft:diamond_sword,minecraft:lava_bucket,minecraft:flint_and_steel,minecraft:obsidian*14
```

### 17. Minecraft magma cubes (E2-21)

Previous commit: 91e2589c

- **Changed** (checked with javap on the 26.3 jar: `MagmaCube`, `AbstractCubeMob`, `MagmaCubeModel`, `MagmaCubeRenderer`, `AbstractCubeMobRenderer`):
  - Vendored `third_party/minecraftoss/entities` (MinecraftOSS patch): `Slime.magma`, a slime variant on the same cube goals and move control (`MagmaCube` extends `AbstractCubeMob` as `Slime` does):
    - attack damage is its size plus 2 (`getAttackDamage`), and it hurts even at size 1 (`canDealDamage` is only `isEffectiveAi`), so a small one also chases with the aggressive hop timing;
    - armour 3 per size step (`setSize`), applied to player hits, arrows, blasts (MW2 included) and mob hits;
    - fire immune (entry 14);
    - hops four times as rarely (`getJumpDelay` x4), jumps 0.1 per size higher (`jumpFromGround`), and in lava jumps out at 0.22 plus 0.05 per size (`jumpInLiquid`);
    - its squash fades slower (`decreaseSquish` x0.9);
    - sounds `entity.magma_cube.*` (the jump has no small version);
    - it splits into magma cubes, saves and dies as `minecraft:magma_cube` (its loot table and experience from the size).
  - Vendored `third_party/minecraftoss/world` (MinecraftOSS patch): natural spawning knows it: fire immune, `checkMagmaCubeSpawnRules` (only not in peaceful), the cube mobs' walk target and despawn, and the slime's `finalizeSpawn` (sizes 1, 2 or 4) with the armour attribute.
  - `minecraft_terrain/src/server_mobs.rs`: `minecraft:magma_cube` is simulated, loaded and saved with its size.
  - New `minecraft_terrain/src/magma_cube_render.rs`: `MagmaCubeModel`, eight 8x1x8 slices round a 4-block core from `entity/slime/magmacube`, the slices spreading 1.7 pixels per step as it stretches on take-off (`setupAnim`), sized and squashed by `applySizeAndSquish` (no slight downscale), lit at block light 15 (`MagmaCubeRenderer.getBlockLightLevel`). `slime_render.rs` skips magma cubes; `minecraft_survival/src/entities.rs` draws them (one call line); `mesh.rs` loads the texture.
- **Tests:** `world` (entities): `a_magma_cube_is_an_armoured_fire_proof_slime_that_hurts_even_when_tiny_and_splits_into_magma`. `magma_cube_render`: `a_magma_cube_is_eight_slices_round_a_core_that_spread_as_it_leaps` (9 cubes, 216 vertices, the slice UVs and spread). `server`: `magma_cubes_spawn_in_three_sizes_and_come_back_as_magma_cubes` (40 summoned in the Nether: sizes 1, 2 and 4, armour 3 per size saved, each loads back as a magma cube of its size).
- **Decisions:** the landing particles stay undrawn for magma cubes as for slimes (only their random draws are kept); vanilla's are flames.
- **Test steps** (any world):
  1. Use 4 magma cube spawn eggs on flat ground: cubes of different sizes appear, dark red and glowing in the dark, hopping slowly and high. **Screenshot 14.**
  2. Let them come: even the smallest hurts you. Pour the lava bucket next to them: they swim in it and leap out, unhurt.
  3. Kill a big one: it splits into smaller magma cubes; the medium and big ones drop magma cream sometimes. **Screenshot 15.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:magma_cube_spawn_egg*4,minecraft:diamond_sword,minecraft:lava_bucket,minecraft:cobblestone*64
```

### 18. Minecraft zombified piglins do not burn in daylight

Previous commit: 27534fff

- **Defect:** entry 16 left zombified piglins sun sensitive, as `Zombie.isSunSensitive` reads. But 26.3's `Mob.aiStep` burns a mob in daylight only when its type is in `#minecraft:burn_in_daylight` (checked with javap; the tag lists skeletons, strays, wither skeletons, bogged, zombies, zombie horses, zombie villagers, drowned, zombie nautili and phantoms), and zombified piglins are not in it. Their fire immunity hid the flames, but a helmet on one still lost durability in the overworld sun.
- **Changed:** vendored `entities/src/zombie.rs` (MinecraftOSS patch): `ZombieKind::burns_in_daylight` is false for zombified piglins, as for husks. Entry 16's text is corrected.
- **Tests:** `zombie`: `zombified_piglins_and_husks_never_burn_in_daylight`.
- **Test steps:** none beyond entry 16 (a helmeted zombified piglin in the overworld sun keeps its helmet whole).
- **Kit:** none.

### 19. Minecraft wither skeletons and the wither effect (E2-22)

Previous commit: 899359fa

- **Changed** (checked with javap on the 26.3 jar: `WitherSkeleton`, `AbstractSkeleton`, `WitherMobEffect`, `Gui.HeartType`, `EntityTypes`, `LayerDefinitions`):
  - Vendored `third_party/minecraftoss/entities` (MinecraftOSS patch):
    - `skeleton.rs`: `SkeletonKind::WitherSkeleton`: 0.7 x 2.4 with eyes at 2.1, 20 health, sounds `entity.wither_skeleton.*`, no bow; `Skeleton.main_hand` (the held item other than a bow) and `melee_damage` (4 for a wither skeleton after `finalizeSpawn`, 2 for other skeletons, plus the sword's bonus: stone 4).
    - `monster_ai.rs`: `MonsterKind::WitherSkeleton`: `AbstractSkeleton.registerGoals` with the melee goal `reassessWeaponGoal` picks without a bow (`MeleeAttackGoal(this, 1.2, false)`, which raises its arms while attacking), and as targets `WitherSkeleton.registerGoals`'s piglins (none live yet: the goal only rolls) before the skeleton's own. Lava costs 8 on its paths. `MeleeAttackGoal` takes the speed vanilla gives it (1.0 for the others, as before). Iron golems now also hunt zombified piglins, magma cubes and wither skeletons (`Enemy`).
    - `world.rs`: a skeleton's melee hit lands on its target (a player as a `PlayerHit`, a mob through `mob_hits_mob`); a wither skeleton's hit gives a mob wither for 200 ticks (`WitherSkeleton.doHurtTarget`). A player's kill may drop the sword (8.5%, worn at random) and adds 1 to 3 experience for it.
    - `effects.rs`: `MobEffect::Wither` (`minecraft:wither`, colour 7561558): 1 damage every 40 ticks, halved a level, which can kill. `add_mob_effect` turns it away from wither skeletons (`canBeAffected`).
  - Vendored `third_party/minecraftoss/player/src/survival.rs` (MinecraftOSS patch): `EffectKind::Wither` for players, ticking the same way.
  - Vendored `third_party/minecraftoss/world` (MinecraftOSS patch): natural spawning knows the type (fire immune, the monster rules, so the fortress's draws of entry 15 now spawn them live), and its `finalizeSpawn` is `AbstractSkeleton`'s with `WitherSkeleton.populateDefaultEquipmentSlots` (a stone sword), no equipment enchantments, and `ATTACK_DAMAGE` 4.
  - `minecraft_survival`: a wither skeleton's melee blow that lands gives the player wither for 200 ticks (`DamageCause::melee_effect`); wither damage is `DamageCause::Wither` ("withered away"); `/effect` and saving know `minecraft:wither`; `Vitals.withered`.
  - `hud/src/minecraft_vitals.rs` and `frame/src/minecraft_ui.rs` (one field): withered hearts (`hud/heart/withered_*`), which win over normal and absorbing hearts but not over poisoned ones (`Gui.HeartType.forPlayer`). `render_anim/src/minecraft_world.rs`: one field line in the vitals hand-over.
  - `minecraft_terrain`: `minecraft:wither_skeleton` is simulated, loaded and saved with its sword; `skeleton_render.rs` draws it on the skeleton model with `wither_skeleton.png`, scaled 1.2 (`LayerDefinitions`' `MeshTransformer.scaling(1.2)`), its armour scaled alike, and the held sword (any skeleton's held weapon is drawn now).
- **Tests:** `world` (entities): `a_wither_skeleton_strikes_with_its_stone_sword_and_wither_hurts_mobs_but_not_it` (it closes in on a player and hits for 8, is immune to wither, and wither takes 2 health off a zombie in 41 ticks). `survival`: `a_wither_skeleton_s_blow_withers_and_wither_hurts_every_two_seconds_to_the_end` (200 ticks of wither, black hearts, wither kills from 3 health, every 20 ticks at level II). `death`: `wither_withers_away_and_only_a_wither_skeleton_s_blow_brings_it`. `minecraft_vitals`: `wither_turns_the_hearts_black_unless_poison_turns_them_green`. `server`: `wither_skeletons_carry_stone_swords_and_come_back_with_them` (attack damage 4 saved).
- **Decisions:**
  - A wither skeleton that picks up a bow keeps the bow goals of a plain skeleton (no flaming arrows); none spawns with one.
  - The player's other mob-hit effects (a husk's hunger, tipped arrows) are still not applied on the player side; only the wither blow is, through the damage cause, so the effect needs no new field in the hit.
  - `poof_particles::tests::a_puff_rises_and_fades_within_its_lifetime` failed once in the full `minecraft_terrain` run and passed twice on its own: it draws from an unseeded random (older than this track).
- **Test steps** (survival, `/difficulty normal`):
  1. Use 4 wither skeleton spawn eggs: tall black skeletons with stone swords appear and run at you with raised arms. **Screenshot 16.**
  2. Let one hit you: your hearts turn black and you lose half a heart every two seconds for 10 seconds. **Screenshot 17** (black hearts).
  3. Kill them with the sword: coal and bones drop, a wither skeleton skull rarely (2.5%), sometimes the stone sword.
  4. In the Nether, find a fortress (`/locate structure minecraft:fortress`, then `/tp`): wither skeletons, zombified piglins and magma cubes spawn on its bridges in the dark. **Screenshot 18.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:wither_skeleton_spawn_egg*4,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:golden_apple*4,minecraft:cooked_beef*16,minecraft:diamond_chestplate
```

### 20. Minecraft monster spawners spawn their mob (E2-25)

Previous commit: d37cfa4b

- **Changed** (checked with javap on the 26.3 jar: `BaseSpawner.serverTick`, `delay`, `isNearPlayer`, `ServerLevel.isSpawnerBlockEnabled`, `Monster.checkMonsterSpawnRules`, `EntitySpawnReason.ignoresLightRequirements`):
  - Vendored `third_party/minecraftoss/world/src/level/spawning.rs` (MinecraftOSS patch): `Level::tick_spawners`, each `minecraft:mob_spawner` block entity in the simulated chunks, from its saved fields (`Delay`, `MinSpawnDelay`, `MaxSpawnDelay`, `SpawnCount`, `MaxNearbyEntities`, `RequiredPlayerRange`, `SpawnRange`, `SpawnData`, `SpawnPotentials`):
    - it does nothing unless a non-spectator player is within `RequiredPlayerRange` (16) of its centre and the `spawner_blocks_work` game rule is on;
    - a delay of -1 is drawn first; while the delay is above 0 it counts down;
    - then up to `SpawnCount` (4) tries: the entity's saved `Pos`, or a random spot within `SpawnRange` (4) across and one block up or down (the level random's draws in vanilla's order); its box must be free; with `custom_spawn_rules` only the block and sky light limits (and no monsters in peaceful) count, otherwise the type's own spawn rules as for a `SPAWNER` reason (darkness for monsters, no check of the block below) with its walk target and spawn obstruction; when `MaxNearbyEntities` (6) of its type already stand within its range around it, it waits again; a lone `id` is finalized like a new spawn, a fuller tag loaded as given; the mob joins the level facing a random way;
    - after a round that spawned anything it waits 200 to 799 ticks and picks its next mob from `SpawnPotentials` by weight.
    - The block entity's `Delay` and `SpawnData` are written back; a new wait is sent to the client like other block entity changes (so it is saved).
  - Vendored `natural_spawner/spawning.rs` (MinecraftOSS patch): `CreatureSpawns::spawner_may_spawn`, the spawner's form of the type's spawn checks. A type natural spawning does not know yet (a blaze until N4) only needs room.
  - `minecraft_terrain/src/server.rs`: `ServerSim::tick` runs the spawners after the level's tick, and `apply_game_rules` passes `spawner_blocks_work`.
- **Tests:** `server`: `a_fortress_spawner_spawns_blazes_near_a_player_and_waits_between_rounds` (seed 77's fortress spawner, a player 3 blocks off: blazes within its range, at most 6, then a wait of 200 to 799; a player 40 blocks off: nothing and the first wait of 20 does not run down; `spawner_blocks_work` off: nothing).
- **Decisions:** blazes from fortress spawners wait dormant until N4 brings blazes to life, and count toward the spawner's limit of 6 meanwhile. The flames and smoke of a spawn (level event 2004), the mob turning inside the cage and the spin-up on the client are not drawn.
- **Test steps** (overworld, survival, `/difficulty normal`):
  1. Find a dungeon (a mossy cobblestone room with a spawner and chests, underground, often open to a cave; corrected in entry 21: a spawn egg used on a spawner does not change its mob yet). Stand within 16 blocks in the dark: zombies (or the dungeon's skeletons or spiders) appear around it, two to four at a time, then nothing for 10 to 40 seconds. **Screenshot 19.**
  2. Kill them and stay: new ones come after the wait, never more than 6 of its kind around it at once.
  3. `/gamerule spawner_blocks_work false`: nothing more spawns. Set it back to `true`.
  4. In the Nether, a fortress spawner (a cage on a nether brick stair room) needs N4's blazes to show them; until then nothing visible happens there.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword,minecraft:torch*64,minecraft:cobblestone*64
```

### 21. Minecraft Nether mobs pass clippy

Previous commit: bf1a728f

- **Changed:** the one clippy finding on this group's lines: `piglin_render`'s ear tuple is a named `Ear` (`type_complexity`). Entry 20's test step 1 and kit are corrected: a spawn egg used on a spawner does nothing yet (the egg's `EggUse::Spawner` was never handled), so the steps use a generated dungeon.
- **Clippy:** `clippy -p minecraft_terrain -p minecraft_survival -p hud -p frame -p minecraftoss-entities -p minecraftoss-world -p minecraftoss-player --locked --tests --no-deps -- -A clippy::approx_constant` has no other finding on a line of this group; the rest are older.
- **Tests:** none new.
- **Test steps:** none; nothing changes in game.
- **Kit:** none.

## Group N3 summary

- **Commits:** dfb165d1 (E2-18, fire immunity), cdcaf137 (E2-19, fortress and structure spawns), 91e2589c (E2-20, zombified piglins and portal piglins), 27534fff (E2-21, magma cubes), 899359fa (a fix to entry 16: zombified piglins do not burn in daylight), d37cfa4b (E2-22, wither skeletons and the wither effect), bf1a728f (E2-25, monster spawners), and this entry.
- **Checks:** `test -p minecraft_terrain` (647 passed, 3 ignored as before; `poof_particles` failed once on its unseeded random and passed when run again), `test -p minecraft_survival` (281), `test -p hud` (132), `test -p frame` (16), `minecraftoss-entities` (71), `minecraftoss-world` (60) and `minecraftoss-player` (184) with the full suite's stand-in files, and `check -p launcher` are green; clippy as in entry 21.
- **MinecraftOSS patches in this group:** `entities` (`world/hazards.rs` fire immunity; `zombie.rs`, `skeleton.rs`, `slime.rs` the new kinds; `monster_ai.rs` the zombified piglin and wither skeleton goals, the melee goal's speed, `is_enemy`; `world.rs`, `world/slimes.rs`, `world/combat.rs`, `world/witches.rs` their ticks, hits, armour and drops; `effects.rs` and `potion.rs` the wither effect; `golem_ai.rs`, `wolf_ai.rs` the melee speed field), `world` (`natural_spawner.rs`, `natural_spawner/spawning.rs` categories, structure overrides, the new types' rules and finalizing, `spawner_may_spawn`; `level/spawning.rs` the structure finder, portal piglins and spawners; `level/random_tick.rs` the portal's random tick; `level/time.rs` `environment_flag`), `player` (`survival.rs` the player's wither).
- **Handed on to N4 (ghasts and blazes):** `fire_immune` already lists blazes and ghasts; fortress spawners already spawn blazes (dormant until simulated) and the fortress override already draws them; `spawner_may_spawn` treats a type without spawn rules as needing room only; `MobEffect::Wither` and `Target::Mob` melee hits are in place.
- **Known gaps:** who a zombified piglin was angry at is not saved (only for how long); a spawn egg used on a spawner does nothing; spawner flames, the turning mob in the cage and magma cube landing flames are not drawn; a wither skeleton holding a bow shoots plain arrows; a husk's hunger and tipped arrows still do not reach the player (older than this group).
- **Owner test for the whole group** (one new world `IW4L_MINECRAFT_WORLD=n3`, survival, `/difficulty normal`; screenshots numbered across the entries):
  1. Entry 16 steps 1 to 4 (zombified piglins: neutral, the group's anger, fire, drops), screenshots 11 and 12.
  2. Entry 17 steps 1 to 3 (magma cubes), screenshots 14 and 15.
  3. Entry 19 steps 1 to 3 (wither skeletons and black hearts), screenshots 16 and 17.
  4. Entry 16 step 5 (portal piglins: build and light the portal, `/difficulty hard`, `/gamerule random_tick_speed 1000`, wait; then set both back), screenshot 13.
  5. Go through the portal, then entry 19 step 4 (a fortress's wither skeletons, zombified piglins and magma cubes), screenshot 18.
  6. Entry 20 steps 1 to 3 in the overworld (a dungeon's spawner), screenshot 19.
- **Kit (15 stacks):**

```
IW4L_MINECRAFT_KIT=minecraft:zombified_piglin_spawn_egg*4,minecraft:magma_cube_spawn_egg*4,minecraft:wither_skeleton_spawn_egg*4,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_chestplate,minecraft:diamond_pickaxe+efficiency=5,minecraft:lava_bucket,minecraft:flint_and_steel,minecraft:obsidian*14,minecraft:cooked_beef*32,minecraft:golden_apple*4,minecraft:torch*64,minecraft:cobblestone*64,minecraft:cobblestone*64
```

## Group N4: ghasts and blazes

E2-23 and E2-24, with plan decision 2 (MW2 bullets and melee send ghast fireballs back along the aim).

### 22. Minecraft ghasts and their fireballs (E2-23)

Previous commit: 6d2abe6b

- **Changed** (all checked with javap on the 26.3 jar: `Ghast` and its `GhastMoveControl`, `RandomFloatAroundGoal`, `GhastLookGoal`, `GhastShootFireballGoal`; `AbstractHurtingProjectile`, `Fireball`, `LargeFireball`, `Projectile` (`hitTargetOrDeflectSelf`, `onHit`, `onRedirectProjectile`, `deflect`), `ProjectileDeflection.AIM_DEFLECT`, `Player.deflectProjectile`, `LivingEntity.travelFlying`, `EntityTypes`, `SpawnPlacements`, `LevelEventHandler` 1015 and 1016, `GhastModel`, `GhastRenderer`):
  - Vendored `third_party/minecraftoss/entities` (MinecraftOSS patch):
    - New `ghast.rs`: 10 health, 4 by 4 with eyes at 2.6, flying speed 0.06, follow range 100, sound volume 5, `ExplosionPower` (1). `suitable_fly_to` (`getSuitableFlyToPosition` with no home: a point up to 16 blocks off on each axis; the heightmap mirror in that method only runs after a happy ghast's search fails), `can_reach` (`GhastMoveControl.canReach`: the ghast's box swept to the point meets no block's collision box, blocks it already touches aside), `float_impulse` (the point's direction times 0.06 * 5 / 3), `view_vector`, `face_toward`.
    - New `fireball.rs`: `Fireball` (`LargeFireball` for now): it starts at a tenth of a block a tick along its aim, then each tick `applyInertia` (its direction times `accelerationPower` 0.1 added, times 0.95, or 0.8 in water: at most about 1.9 blocks a tick), the hit along its motion (blocks by collision shape; living mobs and players by their boxes grown by `computeMargin`; not its shooter until it has flown clear, `checkLeftOwner`), and `deflect` (`AIM_DEFLECT`: the motion becomes the deflector's aim, the deflector its owner, and `onDeflection` resets the acceleration to 0.1 for an attack or halves it). `pick_box`: its box grown by its pick radius of 1 (`#redirectable_projectile`).
    - `movement.rs`: `Body::travel_flying` (`LivingEntity.travelFlying`: no gravity, the motion times 0.91 in air, 0.8 in water, 0.5 in lava).
    - `monster_ai.rs`: `MonsterKind::Ghast` with `GhastState`: `RandomFloatAroundGoal` (5, move), `GhastLookGoal` (7, look: facing its motion, or its target within 64 blocks), `GhastShootFireballGoal` (7, every tick: with its target within 64 blocks and in sight it charges, warns at 10, fires at 20 from four blocks in front of it and half a block above its middle towards the target's middle, and cools down from -40; its mouth shows open above 10), and `NearestAttackableTargetGoal` for players within four blocks up or down. Ghasts count as enemies for iron golems.
    - New `world/ghasts.rs`: the ghast in the entity world: `Mob.baseTick` (fire immune, the ambient roll), the goals, `GhastMoveControl` (every 2 to 6 ticks a push towards its point, or it gives the point up when something is in the way), then `travelFlying`; no fall damage; the warn and shoot sounds at volume 10 (`entity.ghast.warn`, `entity.ghast.shoot`); its fireball joins the world after it.
    - New `world/fireballs.rs`: fireballs in the entity world. A fireball whose ghast is gone vanishes (`getOwner().isRemoved()`). A hit on a player is a `PlayerHitKind::Fireball` of 6 (`LargeFireball.onHitEntity`); on a mob it is 6 as a projectile, which fire-immune mobs shrug off (the `fireball` damage type is `#is_fire`) and endermen dodge. Then it bursts with its power and, where mobs may grief, fire (`Level.explode(..., MOB)`): mobs and players in reach take the blast as a creeper's, credited to its owner. A fireball a player sent back (`isReflectedFireball`) deals a ghast 1000, by hit or by blast. `deflect_fireball`, `push_fireball`, `fireball_on_ray`, `take_fireball_blasts`.
    - `world.rs` and `world/{combat,pushing,hazards}.rs`: the ghast and fireball wiring (keys, hits, blasts, deaths with the `minecraft:ghast` loot table and 5 experience, sounds, the census, despawning as a monster, the client copy, flames on flying fireballs).
  - Vendored `third_party/minecraftoss/world` (MinecraftOSS patch): natural spawning knows the ghast: 4 by 4, fire immune, `checkGhastSpawnRules` (not in peaceful, one time in 20, a valid block below; under a spawner only the one in 20), one to a group (`getMaxSpawnClusterSize`), no walk target, follow range 100, and `Mob`'s `finalizeSpawn`.
  - `minecraft_terrain`:
    - `server_mobs.rs`: `minecraft:ghast` is simulated, loaded and saved with `ExplosionPower`; fireballs in flight are saved and loaded (`minecraft:fireball` with `Motion`, `acceleration_power`, `ExplosionPower`, `LeftOwner`).
    - `server.rs`: the fireballs' blasts reach the level after the creepers' (blocks break with `mob_explosion_drop_decay`, and fire starts, only where mobs may grief) and the client hears them; `Command::DeflectFireball` and `Command::PushFireball`.
    - New `ghast_render.rs`: `GhastModel` (a 16-pixel cube over nine 2 by 2 tentacles 8 to 14 long, the lengths from `RandomSource(1660)`, all scaled 4.5 as its layer is), the tentacles swaying (`animateTentacles`), `ghast_shooting.png` while charging, shadow 1.5; fireballs as `ThrownItemRenderer` draws them: the `fire_charge` sprite facing the camera at three times its ground size, full bright. `client_mobs.rs` tracks ghasts; `mesh.rs` loads the textures.
  - `minecraft_survival`:
    - `entities.rs`: ghasts are hit boxes for bullets, and so is each flying ghast fireball (its pick box, in `shot_boxes`; `render_anim/src/minecraft_world.rs` hands `shot_boxes` to the bullets instead of `boxes`, one changed call line, so fireballs stay out of the boxes that keep blocks from being placed: projectiles do not block building). A bullet on it sends it along the bullet's path (from the muzzle to the fireball) as the shooter's (`AIM_DEFLECT`); a grenade's blast only pushes it (its hurt does nothing: `AbstractHurtingProjectile.hurtServer`); a left-click swing at it sends it along the player's look and plays `entity.player.attack.nodamage`. A fireball's hit reaches the player as `MobAttack::Fireball`.
    - `death.rs`: "was fireballed by" (`death.attack.fireball`). `survival.rs`: fire resistance and the `fire_damage` game rule turn a fireball's hit away (its blast still hurts).
- **Tests:** `ghast`: `a_ghast_floats_to_a_point_within_sixteen_blocks_unless_a_block_is_in_the_way`. `fireball`: `a_fireball_speeds_up_towards_two_blocks_a_tick`, `a_fireball_strikes_the_first_wall_or_body_in_its_way_but_not_its_shooter_at_once`, `a_deflected_fireball_flies_along_the_aim_and_belongs_to_whoever_struck_it`. `world::ghasts`: `a_ghast_floats_about_warns_and_fires_at_a_player_level_with_it` (the warning, the open mouth, a hit of 6 and a blast credited to the ghast, a power 1 blast with fire, no target 30 blocks up), `a_fireball_struck_back_along_the_aim_kills_its_ghast_and_credits_the_player` (the player's kill, 5 experience). `spawning`: `a_ghast_spawns_alone_in_its_four_block_box_on_any_floor_and_goes_when_far`. `ghast_render`: `a_ghast_is_a_big_cube_over_nine_tentacles_of_fixed_lengths_that_sway` (240 vertices, the tentacles' pivots and lengths, the sway between 0.2 and 0.6). `server`: `ghast_fireballs_blow_up_walls_unless_mobs_may_not_grief_and_ghasts_keep_their_power` (a fireball bursts on a planks wall and breaks it, but not without mob griefing; `ExplosionPower` 3 and a flying fireball survive a save; the server deflects). `death`: `a_ghast_fireball_fireballs_the_player_and_its_blast_blows_them_up`. `survival`: `fire_resistance_and_the_fire_damage_rule_keep_fireballs_off`.
- **Decisions:**
  - Plan decision 2: a bullet that meets a ghast fireball sends it back as vanilla's `onRedirectProjectile` does for an arrow, with the bullet's own path from the muzzle to the fireball as the shooter's aim (the MW2 shot carries no look angle; its path is the aim with the weapon's spread). Bullets and the swing find a fireball by its pick box (its 1-block box grown by its pick radius of 1, as the crosshair finds it), not its bare box.
  - The MW2 knife has no hit on Minecraft mobs at all, so the Minecraft left-click swing is the melee that deflects.
  - Vanilla arrows and snowballs deflect fireballs too; players have no bow here and thrown snowballs do not look for fireballs, so neither deflects yet.
  - A fireball does not strike another fireball (vanilla can, through `onRedirectProjectile`), and its smoke trail is not drawn.
  - A saved fireball comes back without its owner (its UUID is not resolved), so its hits name nobody.
- **Test steps** (survival, `/difficulty normal`; the Nether is best: `IW4L_MINECRAFT_DIMENSION=the_nether` or through a portal):
  1. Stand in an open place, use a ghast spawn egg a few blocks in front of you and step back about 15 blocks. The big white ghast drifts about, turns to you, cries, opens its red-eyed mouth and spits a fireball. **Screenshot 20** (the open mouth or the fireball in flight, flames on it).
  2. Let one land near you: it bursts, breaks blocks and leaves fire. **Screenshot 21.**
  3. When the next fireball comes, left-click it with the sword while looking at the ghast: it flies back and the ghast dies at once (ghast tears or gunpowder drop). **Screenshot 22.**
  4. Spawn another ghast. Shoot an incoming fireball with the MW2 rifle, aiming at the ghast: it flies back the way you shot and kills the ghast.
  5. Throw an MW2 grenade under an incoming fireball: the blast pushes it off course but does not send it back.
  6. `/gamerule mob_griefing false`: fireballs still burst and hurt, but break nothing and start no fire. Set it back to `true`.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ghast_spawn_egg*4,minecraft:diamond_sword,minecraft:shield,minecraft:diamond_chestplate,minecraft:golden_apple*4,minecraft:cooked_beef*16,minecraft:cobblestone*64,minecraft:water_bucket
```

### 23. Minecraft blazes and small fireballs (E2-24)

Previous commit: 45b0216c

- **Changed** (all checked with javap on the 26.3 jar: `Blaze`, `Blaze$BlazeAttackGoal`, `SmallFireball`, `Monster.checkAnyLightMonsterSpawnRules`, `SpawnPlacements`, `EntityTypes`, `LevelEventHandler` 1018, `BlazeModel`, `BlazeRenderer`):
  - Vendored `third_party/minecraftoss/entities` (MinecraftOSS patch):
    - New `blaze.rs`: 20 health, 0.6 by 1.8 (eyes at 1.53), attack damage 6, speed 0.23, follow range 48, 10 experience. `slow_fall` (`Blaze.aiStep`: falling in the air it keeps only 0.6 of its fall a tick), `rise_towards` (`customServerAiStep`: every 100 ticks a new `allowedHeightOffset` of `triangle(0.5, 6.891)`, and with its target's eyes above its own plus that offset it climbs, `y += (0.3 - y) * 0.3`), `small_fireball_aim` (`triangle` spread of `2.297 * sqrt(sqrt(distance squared)) * 0.5` on x and z), `triangle`.
    - `fireball.rs`: `FireballKind::Small` (`SmallFireball`: 0.3125 wide, not redirectable, 5 damage, 100 ticks of fire).
    - `monster_ai.rs`: `MonsterKind::Blaze` with `BlazeState` and `BlazeAttackGoal` (4, move and look): within 2 blocks and in sight it strikes every 20 ticks and walks at its target; within follow range and in sight it flares up (`setCharged`) for 60 ticks, fires three small fireballs 6 ticks apart (`entity.blaze.shoot` at volume 2) and rests for 100, looking at its target; out of sight for under 5 ticks it moves at it. Then `MoveTowardsRestrictionGoal` (5, no home: never runs), `WaterAvoidingRandomStrollGoal(1.0, 0.0)` (7), looking at players (8) and around (8); targets: `HurtByTargetGoal` alerting other blazes, and players in sight. Water costs -1 and lava 8 on its paths. Blazes count as enemies for iron golems.
    - New `world/blazes.rs`: the blaze in the entity world: fire immune, its idle clock always running fast (`getLightLevelDependentMagicValue` 1), the slow fall, the goals, the move control towards the attack goal's spot, the climb, the walk, then water and rain hurting it (`isSensitiveToWater`); a strike is a 6 melee hit; its small fireballs start half a block above its middle; a hurt blaze sets the blazes around it on its attacker; it drops `minecraft:blaze` loot with 10 experience.
    - `world/fireballs.rs`: a small fireball sets what it strikes alight for 5 seconds and hurts it 5 (`fireball`, `#is_fire`: fire-immune mobs shrug it off and their fire is put back as it was, as `SmallFireball.onHitEntity` restores it); on a block it lights the block in front of the face it hit, unless a mob shot it where mobs may not grief (`onHitBlock`), and it never bursts. `take_fireball_fires`.
    - `world.rs` and `world/{combat,pushing,hazards}.rs`: the blaze wiring; a flaring blaze is drawn burning (`isOnFire` is `isCharged`).
  - Vendored `third_party/minecraftoss/world` (MinecraftOSS patch): natural spawning knows the blaze: 0.6 by 1.8, fire immune, a monster that spawns in any light (`checkAnyLightMonsterSpawnRules`: not in peaceful, a valid block below; a spawner only needs room), follow range 48, `Mob`'s `finalizeSpawn`. So fortress spawners and the fortress's own spawns now make live blazes. `level/explosion.rs`: `Level::light_fire` (`BaseFireBlock.getState` set where there is air).
  - `minecraft_terrain`: `minecraft:blaze` is simulated, loaded and saved, and small fireballs in flight too (`minecraft:small_fireball`); the server lights the fires small fireballs ask for; new `blaze_render.rs`: `BlazeModel` (the head turning with the look, three rings of four rods circling at 9, 7 and 5 pixels at their own speeds and bobbing), always lit at block light 15 (`BlazeRenderer`), shadow 0.5; small fireballs are the `fire_charge` sprite at 0.75 of its ground size. `client_mobs.rs` tracks blazes; `mesh.rs` loads the texture.
  - `minecraft_survival`: blazes are hit boxes for bullets (`MobHit::Blaze`); the client plays `entity.blaze.burn` one tick in 24 (volume 1 to 2, pitch 0.3 to 1); a small fireball's hit is `MobAttack::SmallFireball` ("was fireballed by"), which sets the player alight for 100 ticks when it hurts (`survival.rs`; fire resistance turns it away and no fire is left).
- **Tests:** `blaze`: `a_blaze_falls_slowly_and_rises_towards_a_target_above_it`. `world::blazes`: `a_blaze_burns_bright_and_fires_three_small_fireballs_that_set_the_player_alight` (the flare, three shots a volley, a hit of 5 with 100 ticks of fire, a strike of 6 up close), `a_blaze_rises_towards_a_player_above_it`, `a_small_fireball_lights_the_block_beside_where_it_lands_and_spares_fire_immune_mobs` (a blaze untouched, a zombie hurt and burning, fire in front of the wall, none without mob griefing, no deflection). `spawning`: `a_blaze_spawns_as_a_monster_in_any_light`. `blaze_render`: `a_blaze_is_a_head_over_three_rings_of_rods_that_turn_and_bob` (312 vertices at block light 15). `server`: `blazes_live_and_their_small_fireballs_set_fire_beside_what_they_strike` (fire in front of an obsidian wall, none without mob griefing, no blast; a blaze and a small fireball survive a save). `death`: the small fireball's line. `survival`: `a_blaze_fireball_sets_the_player_alight_only_when_it_hurts`.
- **Test changed:** `server::a_fortress_spawner_spawns_blazes_near_a_player_and_waits_between_rounds` now turns natural spawning off: with blazes simulated, the fortress's own natural spawns put blazes outside the spawner's range into the list it checks.
- **Decisions:**
  - The blaze's smoke particles and the small fireballs' smoke trail are not drawn; the burn sound is played from the client's own random, as vanilla's client does.
  - The blaze's look, its shots and its strike follow the goal order of vanilla's `serverAiStep`; the fireballs start from where it stood when the goal fired them.
- **Test steps** (survival, `/difficulty normal`):
  1. Use a blaze spawn egg on flat ground and step back about 10 blocks. The yellow blaze glows in the dark, its rods circling and bobbing, and it crackles now and then. **Screenshot 23.**
  2. It flares up (flames around it) for about 3 seconds, then fires three small fireballs in quick succession. A hit sets you on fire. **Screenshot 24** (the flaring blaze or a fireball in flight).
  3. Fireballs that miss leave fire on the ground or wall where they land. **Screenshot 25.**
  4. Pillar up 6 blocks with cobblestone: the blaze rises towards you.
  5. Walk up to it: up close it strikes you. Pour the water bucket on it: it takes damage. Kill it with the sword: blaze rods drop.
  6. `/gamerule mob_griefing false`: its fireballs still burn you but light no fires. Set it back to `true`.
  7. In the Nether, at a fortress spawner (`/locate structure minecraft:fortress`, then `/tp`): blazes appear around the cage and come at you; more blazes appear on the fortress's nether bricks in the dark. **Screenshot 26.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:blaze_spawn_egg*4,minecraft:diamond_sword,minecraft:shield,minecraft:water_bucket,minecraft:cobblestone*64,minecraft:diamond_chestplate,minecraft:golden_apple*4,minecraft:cooked_beef*16
```

## Group N4 summary

- **Commits:** 45b0216c (E2-23, ghasts and their fireballs, with plan decision 2), e9e65b32 (E2-24, blazes and small fireballs), and this entry.
- **Checks:** `test -p minecraftoss-entities --tests` (81), `test -p minecraftoss-world --tests` (62), `test -p minecraft_terrain` (651 passed, 3 ignored as before), `test -p minecraft_survival` (284 and the vanilla world test), `test -p render_anim` (67) and `check -p launcher` are green. `clippy -p minecraft_terrain -p minecraft_survival -p minecraftoss-entities -p minecraftoss-world -p render_anim --locked --tests --no-deps -- -A clippy::approx_constant` has no finding on a line of this group; the rest are older (`server.rs`, `server_mobs.rs`, `mesh.rs`, `entities.rs`, the render_anim occupancy files and more).
- **MinecraftOSS patches in this group:** `entities` (new `ghast.rs`, `blaze.rs`, `fireball.rs`, `world/ghasts.rs`, `world/blazes.rs`, `world/fireballs.rs`; `movement.rs` `travel_flying`; `monster_ai.rs` the ghast and blaze goals and states; `world.rs`, `world/combat.rs`, `world/pushing.rs`, `world/hazards.rs` the wiring; `bin/check_zombie_player_hits.rs` the new hit kind), `world` (`natural_spawner.rs`, `natural_spawner/spawning.rs` the ghast and blaze spawn rules; `level/explosion.rs` `light_fire`).
- **Handed on:**
  - The End track's X3 rebases on these mob wiring files (`minecraft_survival/src/entities.rs` keys 16 to 18, `server_mobs.rs`, `client_mobs.rs`, `mesh.rs`, vendored `world.rs`), keeping both sides. `fireball.rs` (an `AbstractHurtingProjectile` with `Shooter`, `deflect` and the pick box) can take the dragon's fireball as a further kind; `EntityWorld::fireball_on_ray`, `deflect_fireball` and `push_fireball` already reach MW2 bullets, grenades and the swing.
  - Multiplayer: deflection, hits and blasts carry the player's id (`Shooter::Player`), never `PLAYER`; only the adapter's local player sends them so far.
- **Known gaps:** arrows and snowballs do not send ghast fireballs back (players have no bow; thrown snowballs do not look for fireballs); a fireball never strikes another fireball; the fireballs' smoke trails and the blaze's smoke are not drawn; a saved fireball comes back without its owner; the MW2 knife has no hit on Minecraft mobs at all, so the left-click swing is the melee that deflects.
- **N6** (piglins, hoglins, striders, bartering, brutes, zoglins, lodestones) stays out of scope this round.
- **Owner test for the whole group** (one new world, survival, `/difficulty normal`, best in the Nether: `IW4L_MINECRAFT_WORLD=n4` with `IW4L_MINECRAFT_DIMENSION=the_nether`; screenshots numbered across the entries):
  1. Entry 22 steps 1 to 6 (ghasts: the open mouth and fireballs, a burst that breaks blocks and leaves fire, a fireball swung back to kill its ghast, one shot back with the MW2 rifle, a grenade that only pushes one, no griefing without `mob_griefing`), screenshots 20 to 22.
  2. Entry 23 steps 1 to 7 (blazes: the glow and rods, the flare and three-shot volleys that set you alight, fire where they land, the climb, the strike up close, water, no fires without `mob_griefing`, a fortress spawner's blazes), screenshots 23 to 26.
- **Kit (9 stacks):**

```
IW4L_MINECRAFT_KIT=minecraft:ghast_spawn_egg*4,minecraft:blaze_spawn_egg*4,minecraft:diamond_sword,minecraft:shield,minecraft:diamond_chestplate,minecraft:golden_apple*4,minecraft:cooked_beef*16,minecraft:cobblestone*64,minecraft:water_bucket
```

## Group N5: Nether rules

E2-13 to E2-16, then E2-30, and a check that water evaporates in the Nether.

### 24. Minecraft water evaporates in the Nether

Previous commit: 9d787dbd

- **Changed:** tests only; nothing had to be fixed. The brief found `water_evaporates` already wired: the client's bucket reads the player's level's environment (`minecraft_world.rs`, `bucket::Catalog`), and the server level reads its own dimension's `gameplay/water_evaporates` for mined ice (`ice.rs`) and dispensers; a placed wet sponge dries where the level is the Nether (`plants.rs`, `WetSpongeBlock.onPlace`). Since N1 every level carries its own dimension, so these now run in a real Nether level.
- **Tests:** `server`: `in_the_nether_mined_ice_leaves_no_water_and_a_wet_sponge_dries` (an overworld level and a Nether level side by side: ice mined on stone turns to water in the overworld and to air in the Nether; a placed wet sponge stays wet in the overworld and turns into a dry sponge in the Nether). `environment`: `water_evaporates_once_the_nether_environment_is_sampled` (the Nether's sampled environment says water evaporates, the overworld's and the End's do not). The older `bucket::water_evaporates_where_the_dimension_says_so_but_lava_does_not` covers the bucket's hiss.
- **Decisions:** none.
- **Known gaps:** the bucket's 8 large smoke puffs (`BucketItem.emptyContents`) are not drawn, only the hiss plays.
- **Test steps** (in the Nether: through a portal, or `IW4L_MINECRAFT_DIMENSION=the_nether`):
  1. Pour the water bucket on netherrack: it hisses and no water appears; the bucket is empty. **Screenshot 27.**
  2. Place ice on netherrack and mine it without silk touch: it leaves nothing. The ice block drops nothing either, as in vanilla.
  3. Place a wet sponge: it turns into a dry sponge at once.
  4. In the overworld the same ice leaves water and the wet sponge stays wet.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:water_bucket,minecraft:ice*4,minecraft:wet_sponge*2,minecraft:netherrack*16,minecraft:diamond_pickaxe
```

### 25. Minecraft beds explode in the Nether and the End (E2-13)

Previous commit: 56c1e7a7

- **Changed** (checked with javap on the 26.3 jar: `AbstractBedBlock.useWithoutItem`, `BedBlock.destroyOnUse`, `StrawBedBlock.destroyOnUse`/`destroyBed`, `BedRule`, `EnvironmentAttributes.STRAW_BED_RULE`, `DamageSources.badRespawnPointExplosion`, `CombatTracker.getDeathMessage`):
  - New `minecraft_survival/src/respawn_block.rs`: `use_bed(world, registries, rules, pos)`, `AbstractBedBlock.useWithoutItem`: a bed whose other half is missing or is another bed does nothing; a foot is used through its head; the bed's rule (`gameplay/bed_rule`, or `gameplay/straw_bed_rule` for the 26.3 straw bed) with `destroy_on_use` removes the head and, when it is the same bed, the foot, without drops, and a bed (not a straw bed) explodes at the head's centre with power 5 and fire (`badRespawnPointExplosion`); a straw bed only breaks with `block.straw_bed.break_leave`. Otherwise an occupied bed and a free one are told apart for entry 26. `BlockView` reads a level's blocks.
  - `minecraft_survival/src/dimension.rs`: `DimensionRules.straw_bed_rule` (the overworld's never sets the spawn and is destroyed on leaving; the Nether's and the End's are destroyed on use).
  - Vendored `third_party/minecraftoss/entities/src/world.rs` (MinecraftOSS patch): `PlayerHitKind::BadRespawnPoint` and `EntityWorld::bad_respawn_point_blast`, the public blast whose player hits carry the bed's damage type; mobs take it as an explosion with no attacker, as before. `bin/check_zombie_player_hits.rs` takes the new kind as an explosion.
  - `minecraft_terrain/src/server.rs`: `ServerSim::respawn_point_blast` and `Command::RespawnPointBlast`: E2-12's blast (vanilla explosion damage, plan decision 5, block breaking with `block_explosion_drop_decay`, fire) with the bed's damage type.
  - `minecraft_survival`: `DamageCause::BadRespawnPoint` (`badRespawnPoint`): the death line is `death.attack.badRespawnPoint.message` with `[Intentional Game Design]` (`death.attack.badRespawnPoint.link` in square brackets), whatever hurt the player before, as `CombatTracker` does for `intentional_game_design`; it scales with difficulty as `bad_respawn_point` does (`scaling: always`). `Entities::explode_respawn_point`.
  - New `render_anim/src/minecraft_respawn.rs` and one arm in `minecraft_world.rs`'s use chain (main hand, not while sneaking with an item, before the generic block use): the bed's blocks go through `write_blocks` (scene, collision, server), then the blast or the straw bed's sound.
- **Tests:** `respawn_block`: `a_bed_explodes_from_its_head_in_the_nether_and_the_end_but_not_the_overworld` (either half, both dimensions; the overworld sleeps or finds the bed occupied), `a_straw_bed_just_breaks_and_half_a_bed_does_nothing`. `death`: `a_bed_blast_is_intentional_game_design_even_after_a_mob_hit`. `dimension`: the straw bed rules of the three dimensions. `server`: `a_blast_hurts_mobs_and_players_breaks_blocks_and_may_start_fires` also blows up a respawn point: one blast heard, the player hit as `BadRespawnPoint` with no source, mobs and blocks as before.
- **Decisions:**
  - Using a bed in the overworld does nothing yet (entry 26 sets the spawn there).
  - A villager asleep in the bed is not woken (`kickVillagerOutOfBed`); the bed counts as occupied.
- **Test steps** (in the Nether, survival; take off nothing yet):
  1. Put on the diamond armour and hold the totem in the offhand. Place a red bed on netherrack and use it (right-click): it explodes with fire, the bed is gone and drops nothing. **Screenshot 28.**
  2. Use the second bed with no armour and no totem at full health: you die, and the chat says "<name> was killed by [Intentional Game Design]". **Screenshot 29.**
  3. Place the straw bed and use it: it breaks with a rustle and does not explode.
  4. In the End (`IW4L_MINECRAFT_DIMENSION=the_end` for one start), a bed explodes the same way.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:red_bed,minecraft:red_bed,minecraft:red_bed,minecraft:straw_bed,minecraft:totem_of_undying,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:netherrack*32,minecraft:cooked_beef*16
```

### 26. Minecraft beds set your respawn point

Previous commit: 549e0168

- **Why:** the owner wants to respawn at their bed after dying in the Nether or the End; nothing set a bed point before (only `/spawnpoint`). Outside the brief, needed by entry 28.
- **Changed** (checked with javap on the 26.3 jar: `ServerPlayer.startSleepInBed`, `bedInRange`, `isReachableBedBlock`, `bedBlocked`, `Player.freeAt`, `ServerPlayer.setRespawnPosition`, `BedRule.asProblem`, `Player$BedSleepingProblem`, `Level.isDarkOutside`, `Gui.setOverlayMessage` and its overlay drawing):
  - `minecraft_survival/src/respawn_block.rs`: `rest_in_bed(world, registries, rules, bed, sleeper)`, the checks of `ServerPlayer.startSleepInBed` in order: the player's feet within 3 blocks across and 2 up or down of the head's or the foot's bottom centre, else "You may not rest now; the bed is too far away"; no suffocating block above the head or above the foot, else "This bed is obstructed"; then, when the bed rule's `can_set_spawn` holds, the point at the bed's head with the player's yaw and pitch (not forced); then the rule's `error_message` when `can_sleep` does not hold ("You can sleep only at night or during thunderstorms" by day in the overworld). `set_respawn` tells whether the point moved, as `RespawnConfig.isSamePosition`. "Dark" is the sky light level at 11 or below (`skyDarken >= 4`).
  - `render_anim/src/minecraft_respawn.rs`: an occupied bed says "This bed is occupied"; a bed in reach sets the player's respawn point (`Player.respawn`, saved by track D's `player_nbt`) and, when it moved, shows "Respawn point set" in the chat; the bed rules' messages show as overlay messages. `minecraft_world.rs`: `respawn` is taken from the player with the other fields (one destructuring line), and one call line fades the overlay each frame.
  - Overlay messages (`Gui.setOverlayMessage`, the "action bar"), which the fork did not show at all: `frame/src/minecraft_ui.rs` (appended) `MinecraftUi.overlay_message: Option<McOverlayMessage { text, ticks }>`, 60 ticks; `hud/src/minecraft_hotbar.rs` draws it centred 72 GUI pixels above the bottom, white with a shadow, its alpha `ticks * 255 / 20` capped at 255 and drawn above 8, as vanilla's overlay message.
- **Tests:** `respawn_block`: `a_bed_in_reach_sets_the_spawn_by_day_and_says_you_can_sleep_only_at_night` (in reach from 2.5 blocks past the foot, too far across, along and up, obstructed by stone but not by glass, a straw bed never sets the spawn), `the_spawn_message_comes_only_when_the_point_moves`. `minecraft_hotbar`: `the_overlay_message_holds_for_two_seconds_then_fades_over_one`.
- **Decisions:**
  - Sleeping itself (lying down, skipping the night, waking up, `NOT_SAFE` with monsters near) is not part of this round: by night a bed sets the spawn and shows nothing more. It belongs to "finish survival".
  - Until entry 28 the bed point is saved but the respawn still goes to the world spawn.
- **Test steps** (overworld, survival, by day):
  1. Place a bed, stand next to it and use it: the chat says "Respawn point set" and "You can sleep only at night or during thunderstorms" shows above the hotbar for about 3 seconds. **Screenshot 30.**
  2. Use it again: only the overlay message shows, not the chat line.
  3. Step 5 blocks away and use it from there (it is in reach of your crosshair at up to 5 blocks): "You may not rest now; the bed is too far away".
  4. Put a stone block on top of the bed's pillow end and use it: "This bed is obstructed".
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:red_bed,minecraft:red_bed,minecraft:stone*16
```

### 27. Minecraft respawn anchors charge with glowstone and set your spawn (E2-14)

Previous commit: 9d9f8b7e

- **Changed** (checked with javap on the 26.3 jar: `RespawnAnchorBlock.useItemOn`, `useWithoutItem`, `charge`, `canSetSpawn`, `explode`, `isWaterThatWouldFlow` and its `RespawnAnchorBlock$1` damage calculator, `ServerPlayer.setRespawnPosition`, `ServerExplosion.explode`):
  - `minecraft_survival/src/respawn_block.rs`: `use_anchor(world, registries, rules, pos, dimension, fuel)`: glowstone in the hand in use charges an anchor below 4 by one; with the main hand, glowstone in the offhand of a chargeable anchor passes the use on to the offhand; an anchor with no charge passes (so the held item is used, a block placed against it); otherwise, where `gameplay/respawn_anchor_works` holds (the Nether), it sets the respawn point at the anchor (yaw and pitch 0, not forced); anywhere else it explodes. `shielded_by_water`: source water or flowing water of amount 2 or more over no water beside it, or any water right above, as `isWaterThatWouldFlow`.
  - Vendored `third_party/minecraftoss/world/src/level/explosion.rs` (MinecraftOSS patch): `Level::explode_shielded(explosion, pos)`: the rays read water's resistance (100) at that block, as the anchor's `ExplosionDamageCalculator` does when water is near, so the rays die at the centre: no block breaks and no fire starts, while mobs and players are hurt as before.
  - `minecraft_terrain/src/server.rs`: `ServerSim::respawn_point_blast(blast, shield)` and `Command::RespawnPointBlast { blast, shield }` (the bed of entry 25 passes no shield). `Entities::explode_respawn_point(blast, shield)`.
  - `render_anim/src/minecraft_respawn.rs`: charging writes the anchor's next charge (scene, collision, server, light), plays `block.respawn_anchor.charge` and spends one glowstone outside creative (`ItemStack.consume`); setting the spawn shows "Respawn point set" and plays `block.respawn_anchor.set_spawn` only when the point moved (else nothing, `CONSUME`); exploding removes the anchor without a drop and sends the bad respawn point blast (power 5, fire) from its centre, shielded when water is near. The use chain's arm now runs for either hand (beds stay main hand only).
- **Tests:** `respawn_block`: `glowstone_charges_an_anchor_up_to_four_from_either_hand` (0 to 4, the offhand charging, the main hand passing to the offhand's glowstone, a full anchor used as with an empty hand), `a_charged_anchor_sets_the_spawn_in_the_nether_and_explodes_elsewhere` (an empty anchor passes; the Nether's point; the offhand never uses it empty-handed; overworld and End blasts), `flowing_water_beside_or_any_water_above_keeps_the_blocks_around_an_anchor` (source, flowing amounts 2 and 1, waterlogged slab, flowing over water, water above, lava, water diagonally above). `server`: the blast test also blows up a shielded respawn point: the player and the zombie are hurt, all 5 planks stay and no fire starts.
- **Decisions:** the anchor's light (`charges` 1 to 4 give light 3, 7, 11, 15) and comparator output come with the block state; its ambient sound and portal particles (`animateTick`) are not played or drawn.
- **Test steps** (survival):
  1. In the Nether, place a respawn anchor and use it with glowstone in your hand 4 times: each time it hums and its top glows brighter; the glowstone stack drops by 4. **Screenshot 31** (4 charges).
  2. With an empty hand (or glowstone, now that it is full), use it: "Respawn point set" shows in the chat with a deep sound. Use it again: nothing more.
  3. Put a second anchor down, put glowstone in your offhand and use the anchor with your hand empty: the offhand's glowstone charges it.
  4. In the overworld, charge an anchor once and use it with an empty hand: it explodes (bring the totem and armour), breaks blocks and sets fires. **Screenshot 32.**
  5. Charge another overworld anchor, pour water next to it, then use it: it explodes and hurts you but breaks no block and starts no fire.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:respawn_anchor*4,minecraft:glowstone*32,minecraft:water_bucket,minecraft:totem_of_undying,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

### 28. Minecraft players respawn at their anchor or bed, else in the Overworld (E2-15)

Previous commit: 6c0c6570

- **Changed** (checked with javap on the 26.3 jar: `ServerPlayer.findRespawnAndUseSpawnBlock`, `findRespawnPositionAndUseSpawnBlock`, `RespawnPosAngle.of`/`calculateLookAtYaw`, `RespawnAnchorBlock.findStandUpPosition` and its `RESPAWN_HORIZONTAL_OFFSETS`/`RESPAWN_OFFSETS`, `AbstractBedBlock.findStandUpPosition`, `findBunkBedStandUpPosition`, `bedStandUpOffsets`, `bedSurroundStandUpOffsets`, `bedAboveStandUpOffsets`, `isBunkBed`, `Direction.isFacingAngle`, `DismountHelper.findSafeDismountLocation`/`nonClimbableShape`, `BlockGetter.getBlockFloorHeight`, `EntityType.isBlockDangerous`, `NodeEvaluator.isBurningBlock`, `PlayerList.respawn`):
  - `minecraft_survival/src/respawn_block.rs`: `spawn_block(world, registries, rules, point, dark, consume)`, the `spawn_block` hook of S3's `respawn.rs`:
    - an anchor with a charge (or any anchor for a forced point) where anchors work: the first safe spot of its 25 offsets (the 8 around it, the same one lower, one higher, then on top), first avoiding dangerous blocks (fire, lava, magma, lit campfires, lava cauldrons, wither roses, berry bushes, cactus, powder snow), then not; a point that is not forced spends a charge;
    - a bed whose rule sets the spawn there: the bed's 10 spots around it and the 2 above it, starting on the side away from where the player looked when they set it, with the bunk-bed order when a bed is below;
    - a spot is safe when its floor (the block's collision top, or the block below's when it reaches 1) is below 1, the player's 0.6 by 1.8 box there meets no collision, neither it nor the block above is in `#invalid_spawn_inside`, and it is inside the world border;
    - the player faces the block (`calculateLookAtYaw`) with pitch 0.
    - `respawn_at_block(globals, level, player)` reads the respawn level's loaded or saved chunks (`ChunkWorld`, never generating), respawns through `respawn_in`, writes the spent charge (scene, chunk map, server) and plays `block.respawn_anchor.deplete` there.
  - `minecraft_survival/src/respawn.rs` (S3's): `respawn_point` now hands back beds and anchors too, not only forced points, so a dead player's level opens around their anchor or bed; `respawn` (same level) goes through `respawn_at_block` and takes the level mutably.
  - `render_anim/src/minecraft_world.rs`: the respawn after a switch calls `respawn_at_block` (one changed call line).
  - A missing point (bed gone, anchor empty or obstructed, an anchor outside the Nether) is cleared and "You have no home bed or charged respawn anchor, or it was obstructed" shows, then the player goes to the overworld's spawn through S3's switch.
- **Tests:** `respawn_block`: `an_anchor_respawn_stands_beside_it_facing_it_and_spends_a_charge` (north first, the charge spent, a forced point keeps it, past stone and lava, an empty anchor and an overworld anchor are no spawn), `an_anchor_walled_in_on_its_level_lets_the_player_up_or_down` (on top last; nothing when that is walled too), `a_bed_respawn_stands_on_the_side_away_from_the_player_s_look_or_around_it` (west or east of the head by the saved yaw, past the foot when walled, none in the Nether or with the bed gone), `the_respawn_looks_at_the_block`. `server`: `a_nether_death_respawns_beside_a_charged_anchor_and_spends_a_charge` (a real Nether level: two deaths beside the anchor spend 2 then 1, the third finds it empty, clears the point and switches to the overworld).
- **Decisions:**
  - The facing yaw keeps vanilla's float-rounded degree constant, so a player north of the block faces -0.000005 degrees, as vanilla.
  - `Mth.atan2`'s table approximation is replaced by the exact `atan2` (they agree on the four axes; elsewhere by far less than a degree).
  - Respawning in creative also spends a charge, as vanilla's death respawn does.
- **Test steps** (survival; one new world, e.g. `IW4L_MINECRAFT_WORLD=n5`):
  1. In the overworld, place a bed by day and use it ("Respawn point set"). Walk 30 blocks away, `/kill`: you stand next to the bed, facing it. **Screenshot 33.**
  2. Go to the Nether (build and light a portal). Place an anchor, charge it 4 times, use it ("Respawn point set"). `/kill`: you respawn next to it facing it, hear its deep deplete sound, and its top shows 3 charges. **Screenshot 34.**
  3. `/kill` three more times: each time beside the anchor, the charges drop to 0. `/kill` once more: the chat says "You have no home bed or charged respawn anchor, or it was obstructed" and you respawn at the overworld spawn after the loading screen (not at the bed: the anchor replaced it). **Screenshot 35.**
  4. Use the bed again, go back to the Nether and `/kill` there: the loading screen, then you stand next to your bed in the overworld.
  5. Break the bed, `/kill`: the no-home-bed message, and the overworld spawn.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:red_bed,minecraft:respawn_anchor*2,minecraft:glowstone*16,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:netherrack*32,minecraft:diamond_pickaxe,minecraft:cooked_beef*16
```

### 29. Minecraft compasses and clocks move, and spin outside the Overworld (E2-16)

Previous commit: 83d9b9f2

- **Changed** (checked with javap on the 26.3 jar: `CompassAngleState` (`calculate`, `getRotationTowardsCompassTarget`, `getRandomlySpinningRotation`, `isValidCompassTargetPos`, `getAngleFromEntityToPos`, `getWrappedVisualRotationY`), `NeedleDirectionHelper` and its standard wobbler `NeedleDirectionHelper$1`, `Time` and `Time$TimeSource` (random, daytime from `visual/sun_angle`, moon phase), `CompassAngleState$CompassTarget`; the 26.3 `items/compass.json`, `clock.json` and `recovery_compass.json`):
  - New `minecraft_terrain/src/dials.rs`: `Dials` evaluates the item definitions of `minecraft:clock`, `minecraft:compass` and `minecraft:recovery_compass` once per 20 Hz tick against a `DialContext` (the player's dimension, feet and yaw, the world spawn, the last death, the sun angle): `minecraft:condition` takes its false branch (no lodestones yet), `minecraft:select` on `minecraft:context_dimension` picks the case of the player's dimension, and `minecraft:range_dispatch` picks the last entry whose threshold is at most the property's value times `scale`. The `minecraft:compass` value points at its target (`spawn` the world spawn, `recovery` the last death) from the player's feet, relative to their yaw, through a wobbling needle (factor 0.8); with no target in the player's dimension, or standing on it, the needle chases a random value each tick and spins. `minecraft:time` with `daytime` is the sun angle over 360 through a needle of factor 0.9, with `random` it spins. The chosen leaf model is carried in the item's key, `minecraft:compass#minecraft:item/compass_07`.
  - `minecraft_terrain`: `item_icons::item_icon_at` draws a keyed item with its leaf model and marks the icon animated (`SpriteClock::mark_animated`), so the inventory's animated icon cells redraw it every tick as the needle turns; `ItemVisuals` (held and posed items) builds the keyed leaf through the new `model::resolve_item_leaf` (the tail of `resolve_item_model_in`, which now calls it), caching one model per leaf.
  - `render_anim`: `InventoryUi.dials`; the icons use `dials.key(id)`; `minecraft_world.rs` steps the dials once a frame (one call line after the player's yaw) and the first-person held item takes its key (one changed line).
- **Tests:** `dials`: `a_needle_swings_toward_its_target_and_settles_there`, `a_compass_points_at_the_overworld_spawn_and_spins_in_the_nether` (spawn ahead shows `compass_16`, behind `compass_00`, the Nether and standing on the spawn spin over many frames), `a_clock_tells_the_sun_in_the_overworld_and_spins_elsewhere` (sun angle 90 is `clock_16`, 0 is `clock_00`, the End spins), `vanilla_compasses_and_clocks_get_their_frame_as_icon_and_held_model` (the 26.3 pack: keys for all three, a different icon behind than ahead, marked animated, the leaf model resolves), `a_dial_key_carries_the_model_it_shows`.
- **Decisions:**
  - Vanilla updates a needle only on a tick its item is drawn; here the needles turn every tick whether shown or not (they settle the same way).
  - All compasses in the inventory share one needle per target, as vanilla's one property instance per item model does; the random spin is not offset per item (`hash(seed)`).
  - Dropped compasses and clocks, and those held in third person, keep the default frame.
  - Lodestone compasses stay with N6 (lodestones are out of scope this round).
- **Test steps** (survival or creative):
  1. In the overworld, hold the compass and turn on the spot: the red needle keeps pointing at the world spawn, swinging a little as it settles; in the hotbar the icon turns the same way. **Screenshot 36.**
  2. Hold the clock: the dial shows the sun's position (noon at the top); `/time set midnight` turns it to the night side over a moment.
  3. Go to the Nether: compass and clock both spin wildly, in the hand and in the hotbar. **Screenshot 37.**
  4. `/kill` in the Nether, respawn, hold the recovery compass there: it points at where you died; in the overworld it spins.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:compass,minecraft:clock,minecraft:recovery_compass,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:cooked_beef*16
```

### 30. Minecraft Nether rules pass clippy

Previous commit: ed0e28f5

- **Changed:** the three clippy findings on this group's lines: `dials.rs` picks a range entry with `rev().find` instead of `filter().last()` (`double_ended_iterator_last`), `respawn_block.rs` names the bunk bed's tries `StandTry` (`type_complexity`), and `minecraft_respawn.rs` loads the language with `unwrap_or_else` (the workspace denies `unwrap_or_default`).
- **Clippy:** `clippy -p minecraft_terrain -p minecraft_survival -p render_anim -p hud -p frame -p minecraftoss-entities -p minecraftoss-world --locked --tests --no-deps -- -A clippy::approx_constant` has no other finding on a line of this group (checked against `git diff -U0 9d787dbd`); the rest are older.
- **Correction to entry 29, test step 4:** a death in the Nether respawns in the overworld, where the recovery compass spins. Test it instead in the overworld: die there (`/kill`), respawn, and the recovery compass points back at where you died; take it through a portal and it spins in the Nether.
- **Tests:** none new.
- **Test steps:** none; nothing changes in game.
- **Kit:** none.

### 31. Docs: Minecraft Nether state (E2-30)

Previous commit: 764dfa21

- **Changed:**
  - `docs/MINECRAFT-SURVIVAL.md`: a new section "The Nether (round 7, track NETHER)" with the dimension rules, the level, portals, the Nether mobs, the rules of this group, and what is left for N6; the Limitations' way in and out of the Nether (portals, the respawn at an anchor or bed); the roadmap's "Nether travel" now points at that section and lists N6; the Mobs limitation lists only the missing Nether mobs; the `IW4L_MINECRAFT_DIMENSION` row says the Nether arrival comes through a found or built portal (since entry 10); the Crate layout names `dimension.rs`, `portal.rs`, `portal_forcer.rs`, `respawn_block.rs`, the two adapters and `dials.rs`.
  - `docs/NETHER-END.md`: the strider correction of the brief (natural spawning gives striders `Rules::Other`, which refuses, so they never spawn; they do not go dormant).
- **Tests:** none.
- **Test steps:** none; docs only.
- **Kit:** none.

## Group N5 summary

- **Commits:** 56c1e7a7 (water evaporates in the Nether, tests), 549e0168 (E2-13, beds explode in the Nether and the End), 9d9f8b7e (beds set your respawn point, with overlay messages), 6c0c6570 (E2-14, respawn anchors), 83d9b9f2 (E2-15, respawning at an anchor or bed, else the overworld), ed0e28f5 (E2-16, compasses and clocks), 764dfa21 (clippy), and this entry (E2-30, docs).
- **Checks:** `test -p minecraft_terrain` (658 passed, 3 ignored as before), `test -p minecraft_survival` (297 and the vanilla world test), `test -p render_anim` (67), `test -p hud` (133), `test -p frame` (16), `test -p minecraftoss-entities -p minecraftoss-world --tests` (81 and 62) and `check -p launcher` are green; clippy as in entry 30. `minecraftoss-player` was not touched by this group.
- **MinecraftOSS patches in this group:** `entities` (`world.rs`: `PlayerHitKind::BadRespawnPoint` and `EntityWorld::bad_respawn_point_blast`; `bin/check_zombie_player_hits.rs` takes the new kind), `world` (`level/explosion.rs`: `Level::explode_shielded`).
- **Handed on:**
  - The End track: `ServerSim::respawn_point_blast` is the bad respawn point blast; the End's crystals keep using `ServerSim::blast`. Beds explode in the End through this group's code, and anchors explode there as everywhere outside the Nether. `MinecraftUi.overlay_message` (appended to `frame/src/minecraft_ui.rs`) and `render_anim::minecraft_respawn::show_overlay` can carry any action bar line (for example the End's credits or a boss message).
  - "Finish survival" (N6 and after): sleeping through the night (lying down, skipping it, `NOT_SAFE`), lodestones and lodestone compasses (`dials.rs` already evaluates the `lodestone` target as spinning and takes the compass definition's false branch), piglins and their bartering, brutes, hoglins, zoglins and striders.
  - Multiplayer: the respawn point, the anchor depletion and the bed rules work on the `Player` (by UUID) and the level; only the adapter's local player uses beds and anchors and sees dials so far.
- **Known gaps:** no sleeping; no anchor particles or ambient hum; no bucket smoke when water evaporates; dropped and third-person compasses and clocks keep their default frame; a villager asleep in a bed is not woken by a player using it.
- **N6** (piglins, hoglins, striders, bartering, brutes, zoglins, lodestones) stays out of scope this round, as the plan says; group N5 is the last group of this track this round.
- **Owner test for the whole group** (one new world `IW4L_MINECRAFT_WORLD=n5`, survival, by day; screenshots numbered across the entries):
  1. Overworld beds: entry 26 steps 1 to 4 (screenshot 30), then entry 28 step 1 (respawn beside the bed, screenshot 33).
  2. Dials in the overworld: entry 29 steps 1 and 2 (screenshot 36) and the recovery compass as corrected in entry 30.
  3. Overworld anchors: entry 27 steps 4 and 5 (screenshot 32).
  4. Build and light a portal and go to the Nether: entry 24 steps 1 to 3 (screenshot 27) and entry 29 step 3 (screenshot 37).
  5. Beds in the Nether: entry 25 steps 1 to 3 (screenshots 28 and 29); after the bed death you respawn beside your overworld bed, so go back through the portal.
  6. Anchors in the Nether: entry 27 steps 1 to 3 (screenshot 31), then entry 28 steps 2 and 3 (screenshots 34 and 35).
  7. Entry 28 steps 4 and 5 (the bed respawn after a Nether death, and a broken bed).
- **Kit (23 stacks):**

```
IW4L_MINECRAFT_KIT=minecraft:water_bucket,minecraft:ice*4,minecraft:wet_sponge*2,minecraft:netherrack*32,minecraft:red_bed,minecraft:red_bed,minecraft:red_bed,minecraft:straw_bed,minecraft:totem_of_undying,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:respawn_anchor*4,minecraft:glowstone*32,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:compass,minecraft:clock,minecraft:recovery_compass,minecraft:cobblestone*64,minecraft:cooked_beef*16,minecraft:diamond_pickaxe
```

## Adversarial review of the track

Fixes found by reviewing the whole track (base 3aef2ebe to ea8214c7).

### 32. Minecraft zombified piglins count no experience for a sword they dropped

Previous commit: ea8214c7

- **Changed:** `third_party/minecraftoss/entities/src/world.rs` (`take_deaths`): a zombified piglin's golden sword drop is drawn before its experience, and the experience counts only the equipment still held. Vanilla's `LivingEntity.dropAllDeathLoot` runs `dropCustomDeathLoot` (which empties the slot of a dropped item) before `dropExperience` (checked with javap), and `Mob.getBaseExperienceReward` adds 1 to 3 only for an item still in a slot. Before, a dropped sword still added 1 to 3 experience, and the random draws came in the other order. Wither skeletons already did it this way.
- **Tests:** `world::tests::a_zombified_piglin_s_dropped_sword_no_longer_adds_experience` (400 player kills with seeded randoms: a kill that drops the sword gives 5, one that keeps it 6 to 8).
- **Test steps:** none worth a screenshot; the sword drops on 8.5% of player kills, and the orb count is then a little lower.
- **Kit:** none.

### 33. Minecraft dimension rules pass rustfmt

Previous commit: bbf1db34

- **Changed:** `minecraft_survival/src/dimension.rs`, new in this track, failed `rustfmt --check` on one line of its overworld test (an `assert!` added by entry 25, 549e0168). It is wrapped as rustfmt wants; nothing else in the file changes. Every other new `.rs` file of the track passes.
- **Tests:** none new.
- **Test steps:** none; nothing changes in game.
- **Kit:** none.

## Final verification of the track

### 34. Minecraft Nether mobs and spawns pass the vendored crates' clippy

Previous commit: 87567d46

- **Why:** `cargo clippy -p minecraftoss-entities -p minecraftoss-world` from the repo root lints only workspace members, and the vendored crates are excluded from the workspace, so the earlier clippy entries (13, 21, 30) never linted them. Run as their own workspace (`clippy --manifest-path third_party\minecraftoss\<crate>\Cargo.toml --offline --all-targets -- -A clippy::approx_constant`, with the full suite's stand-in files; the `Cargo.lock` this makes in each crate folder was deleted afterwards), they had six findings on this track's lines.
- **Changed** (MinecraftOSS patches, behaviour unchanged):
  - `entities/src/world.rs` and `entities/src/world/combat.rs`: the blaze arm, last in the mob hit chains of E2-24 (e9e65b32), finds its blaze with `?` instead of `if let ... else { return None }` (`question_mark`).
  - `entities/src/world.rs` (`take_deaths`): the zombified piglin's and wither skeleton's sword drops (91e2589c, d37cfa4b) take the hand with `unwrap_or_else(String::new)`; the workspace's `clippy.toml` denies `unwrap_or_default`.
  - `world/src/natural_spawner.rs`: a `StructureSpawns` alias for a structure's spawn override lists (cdcaf137, `type_complexity`).
  - `world/src/natural_spawner/spawning.rs`: the structure override check is one `if let ... &&` (cdcaf137, `collapsible_if`; the crate is edition 2024).
- **Clippy:** the workspace crates (`clippy -p frame -p hud -p minecraft_survival -p minecraft_terrain -p render_anim --locked --all-targets -- -A clippy::approx_constant`) and the three vendored crates now have no finding on a line the track added or changed (checked with `git blame` against 3aef2ebe, and every finding within a few lines of a track hunk read by hand); the rest are older.
- **Checks:** `test -p minecraft_survival` (297 and the vanilla world test), `test -p minecraft_terrain` (658 passed, 3 ignored as before), `test -p render_anim` (67), `test -p hud` (133), `test -p frame` (16), `test -p minecraftoss-entities -p minecraftoss-world -p minecraftoss-player --tests` with the stand-in files (82, 62 and 184) and `check -p launcher` are green.
- **Test steps:** none for this commit; nothing changes in game.
- **Kit:** none.

### Owner test for the whole track

One new world, survival, `/difficulty normal`, started by day: `IW4L_MINECRAFT_WORLD=n7` and no `IW4L_MINECRAFT_DIMENSION` line. Number the screenshots 1 onward in this order. The entries' own steps say what each one shows.

1. **Portals and travel (N2):** entry 10 steps 1 to 4 with entry 11 steps 1 to 4 while you wait (the 4 second wait, the purple swirl and sounds, the loading screen, about 100, y, -200 in the Nether from 800, -1600, the return). Screenshots 1 to 4. Then entry 12 steps 1 to 3 and entry 10 steps 5 to 8 (the same portal again, a second portal 40 blocks away in the Nether, creative travels at once, the game rule). Screenshot 5 for step 6.
2. **The Nether itself (N1), in the Nether:** pour the lava bucket on flat netherrack: it spreads 7 blocks within about 4 seconds (entry 2, screenshot 6). Look at a netherrack overhang (entry 4, screenshot 7). In creative, `/locate structure minecraft:fortress` and `/tp` there (entry 3, screenshot 8), then back to survival.
3. **Nether mobs (N3), at the fortress:** entry 19 step 4 (its wither skeletons, zombified piglins and magma cubes, screenshot 9); then with the eggs entry 16 steps 1 to 4 (zombified piglins, screenshot 10), entry 17 steps 1 to 3 (magma cubes, screenshot 11) and entry 19 steps 1 to 3 (wither skeletons and black hearts, screenshot 12).
4. **Ghasts and blazes (N4), in the Nether:** entry 22 steps 1 to 6 (screenshots 13 and 14: a fireball swung back, one shot back with the MW2 rifle) and entry 23 steps 1 to 7, leaving out the water of step 5, which only hisses away in the Nether (screenshots 15 and 16).
5. **Nether rules (N5):** in the Nether, the water bucket hisses on netherrack and mined ice leaves nothing (entry 24 steps 1 and 2, screenshot 17); hold the compass and the clock: both spin (entry 29 step 3, screenshot 18). With the totem in the offhand and the chestplate on, use a red bed: it explodes with fire (entry 25 step 1, screenshot 19). Place an anchor, charge it 4 times and use it (entry 27 steps 1 and 2, screenshot 20), then entry 28 steps 2 and 3 (respawning beside it until it is empty, then the overworld spawn; screenshots 21 and 22).
6. **Back in the overworld:** entry 26 steps 1 to 4 with a second bed (screenshot 23), entry 29 steps 1 and 2 (screenshot 24), then go to the Nether and `/kill` there: you respawn beside your overworld bed (entry 28 step 4, screenshot 25). Hold the recovery compass in the overworld (it spins) and where you died (it points there).
7. **Overworld anchor:** charge an anchor once and use it with an empty hand: it explodes, breaks blocks and sets fires (entry 27 step 4, screenshot 26). The totem is used up by then, so this blast may kill you; you then respawn at your bed.

- **Kit (27 stacks; three beds, one each for the overworld spawn, the Nether blast and a spare):**

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*64,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_chestplate,minecraft:totem_of_undying,minecraft:golden_apple*8,minecraft:cooked_beef*64,minecraft:cobblestone*64,minecraft:netherrack*64,minecraft:lava_bucket,minecraft:water_bucket,minecraft:ice*4,minecraft:zombified_piglin_spawn_egg*4,minecraft:magma_cube_spawn_egg*4,minecraft:wither_skeleton_spawn_egg*4,minecraft:ghast_spawn_egg*4,minecraft:blaze_spawn_egg*4,minecraft:red_bed,minecraft:red_bed,minecraft:red_bed,minecraft:respawn_anchor*4,minecraft:glowstone*32,minecraft:compass,minecraft:clock,minecraft:recovery_compass
```
