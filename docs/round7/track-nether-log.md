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

Commit: see `git log` under this subject.

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
