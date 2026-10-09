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
