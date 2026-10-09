# Nether and End: what exists and what to build

This page is for whoever builds Nether travel and the End (roadmap items 1.3 and 1.5 in [../CLAUDE.md](../CLAUDE.md)). It lists what the repo already has, what is missing, and an order to build it in.

- **MinecraftOSS** is a Rust port of the Minecraft Java 26.3 server logic. It is vendored in `third_party/minecraftoss` at commit 4013a68, and the fork has patched about 24 of its files. Its crates are `core`, `generator`, `world`, `entities` and `player`.
- **The fork's own code** is in `crates/minecraft_terrain` (streaming, the server thread, meshing, mob renderers), `crates/render_anim` (loading, saving, input, the Bevy frame), `crates/render_gpu` (the world shaders) and `crates/sim` (collision).
- **Vanilla facts** were checked with javap on the 26.3 `client.jar` and against the data pack inside it. They are cited by class name or by `data/minecraft/...` path ("26.3 jar"). Those files are not in the repo, and none of their code may be copied in.
- **Checked against** commit d426239 on `minecraft-survival`, 2026-10-09.

Status words used below:
- **present**: exists and runs today.
- **not wired**: exists, but nothing in the game calls it.
- **partial**: some of it exists.
- **missing**: nothing in the repo does it (checked with grep).

## Dimension types and rules

Values from `data/minecraft/dimension_type/*.json` (26.3 jar):

| Field | Overworld | Nether | End |
| --- | --- | --- | --- |
| `min_y`, `height` | -64, 384 | 0, 256 | 0, 256 |
| `logical_height` | 384 | 128 | 256 |
| `coordinate_scale` | 1 | 8 | 1 |
| `has_skylight` | true | false | true |
| `has_ceiling` | false | true | false |
| `has_fixed_time` | not set | true | true |
| `has_ender_dragon_fight` | false | false | true |
| `ambient_light` | 0.0 | 0.1 | 0.25 |
| `default_clock` | `minecraft:overworld` | not set | `minecraft:the_end` |
| `skybox` | not set (overworld) | `none` | `end` |
| `cardinal_light` | not set | `nether` | not set |

- **The 26.3 End has sky light.** Older versions had `has_skylight: false`. The fork's `crates/minecraft_terrain/src/terrain.rs:1052` (`sky_light = dimension != Nether`) matches 26.3.
- **26.3 moved most rules into `attributes`.** `ultrawarm`, `natural`, `piglin_safe`, `bed_works`, `respawn_anchor_works` and `effects` are gone from `dimension_type`.
  - Nether: `gameplay/fast_lava` true, `gameplay/water_evaporates` true, `gameplay/respawn_anchor_works` true, `gameplay/piglins_zombify` false, `gameplay/sky_light_level` 4.0, `gameplay/can_start_raid` false, `gameplay/snow_golem_melts` true.
  - Nether and End: `gameplay/bed_rule` and `gameplay/straw_bed_rule` are `can_sleep: never`, `can_set_spawn: never`, `destroy_on_use: true`.
  - End: `gameplay/respawn_anchor_works` false.
  - Overworld: `gameplay/nether_portal_spawns_piglin` true. Zombified piglins come out of portals in the Overworld, not in the Nether.

What reads these today:
- **present:** `has_skylight`. It is read at `third_party/minecraftoss/world/src/chunk_map.rs:166`, `third_party/minecraftoss/generator/src/feature/mod.rs:237-241` and `third_party/minecraftoss/world/src/level/time.rs:60`.
- **present:** `has_ceiling` gates weather (`third_party/minecraftoss/world/src/level/time.rs:61`). The same line excludes the End by name.
- **present:** `ambient_light` (`third_party/minecraftoss/world/src/level/time.rs:74`) and `infiniburn` (`third_party/minecraftoss/world/src/level/time.rs:81`, used at `third_party/minecraftoss/world/src/level/fire.rs:169`).
- **present:** `min_y` and `height` set the generator's chunk range (`third_party/minecraftoss/generator/src/terrain.rs:287-292`).
- **present:** `skybox`, `default_clock`, `cardinal_light` and `timelines` are parsed (`third_party/minecraftoss/core/src/environment.rs:825-867`). The client sends `skybox` to the sky shader, which ignores it (see Rendering).
- **present:** `water_evaporates`.
  - On the server: `third_party/minecraftoss/world/src/level/ice.rs:6-15`.
  - In the client's bucket: `crates/minecraft_terrain/src/environment.rs:148`, passed in at `crates/render_anim/src/minecraft_world.rs:1151`.
  - The wet sponge checks the Nether by name instead (`third_party/minecraftoss/world/src/level/plants.rs:275`).
- **not wired:** fast lava.
  - `Level.fast_lava` sets the lava tick delay, drop-off and slope distance (`third_party/minecraftoss/world/src/level/fluid.rs:53-82`, `third_party/minecraftoss/world/src/level/mod.rs:874-876`).
  - It starts false (`third_party/minecraftoss/world/src/level/mod.rs:238`) and nothing sets it. `set_dimension` does not read the attribute (`third_party/minecraftoss/world/src/level/time.rs:53-83`).
- **not wired:** sea level.
  - `Level::new` sets 63 (`third_party/minecraftoss/world/src/level/mod.rs:271`) and nothing changes it.
  - The Nether generator knows its own sea level of 32 (`third_party/minecraftoss/generator/src/terrain.rs:30`, `:121`).
  - Code that reads `Level.sea_level`: `third_party/minecraftoss/world/src/level/random_tick.rs:199` and `third_party/minecraftoss/world/src/natural_spawner/spawning.rs:571`.
- **missing:** nothing reads `coordinate_scale`, `logical_height`, `has_fixed_time` or `has_ender_dragon_fight`.
- **missing:** nothing reads `bed_rule`, `respawn_anchor_works` or `nether_portal_spawns_piglin`. They are only declared (`third_party/minecraftoss/core/src/environment.rs:138-141`).
- **partial:** the game rules `allow_entering_nether_using_portals`, `players_nether_portal_default_delay` (80) and `players_nether_portal_creative_delay` (0) are declared and saved (`third_party/minecraftoss/world/src/game_rules.rs:115`, `:146-147`). Nothing reads them.

Level setup:
- `Level::set_dimension` loads the environment, the weather gate, sky light, ambient light and infiniburn (`third_party/minecraftoss/world/src/level/time.rs:53-83`).
- `ServerSim::new` calls it (`crates/minecraft_terrain/src/server.rs:203`) and loads that dimension's natural spawning (`crates/minecraft_terrain/src/server.rs:209`).

Clocks are per level, but vanilla keeps one server-wide clock manager:
- Every level gets its own copy of the overworld and the_end clocks (`third_party/minecraftoss/world/src/level/time.rs:62-65`).
- `set_time` writes the client's day ticks into the level's default clock (`third_party/minecraftoss/world/src/level/time.rs:93-98`, called each tick from `crates/minecraft_terrain/src/server.rs:1493`).
- The Nether has no default clock, so its level gets no time at all. An End level would run its `the_end` clock on overworld day ticks.
- Mobs take their day time from the level's own overworld clock (`crates/minecraft_terrain/src/server.rs:441`, `:652`, `:659`). In a Nether level that clock stays 0.

## Terrain and biomes

- **Generators (not wired).**
  - `TerrainGenerator::nether` uses the `minecraft:nether` noise in the `the_nether` type (`third_party/minecraftoss/generator/src/terrain.rs:243-247`).
  - `TerrainGenerator::end` uses the `minecraft:end` noise and a port of `TheEndBiomeSource` (`third_party/minecraftoss/generator/src/terrain.rs:251-263`).
  - `TerrainStream::for_dimension_at` picks one by `Dimension` (`crates/minecraft_terrain/src/terrain.rs:1045-1049`).
  - Nothing in the game or in any test builds a Nether or End stream. The only uses of `Dimension::Nether` and `Dimension::End` are inside `for_dimension_at` (`crates/minecraft_terrain/src/terrain.rs:1047-1089`). This code has never run in the fork, so expect bugs the first time.
- **Nether noise (not wired).**
  - The 26.3 `nether` noise settings have no `aquifers` key, so the generator runs without aquifers (`third_party/minecraftoss/generator/src/terrain.rs:155-165`). The global fluid picker fills lava below sea level 32 (`third_party/minecraftoss/generator/src/terrain.rs:230`).
  - `old_blended_noise` is supported (`third_party/minecraftoss/generator/src/density/ir.rs:298`).
- **Nether biomes (not wired).** `ParameterList::nether()` holds nether_wastes, soul_sand_valley, crimson_forest, warped_forest and basalt_deltas (`third_party/minecraftoss/generator/src/biome_source.rs:334`).
- **Nether carver (present).** `nether_cave` is a `minecraft:cave` carver in 26.3 and parses (`third_party/minecraftoss/generator/src/carver.rs:107`).
- **Nether features (not wired).** Every feature type the five biomes use parses:
  - `delta_feature`, `stepped_column_cluster` and `huge_fungus` (`third_party/minecraftoss/generator/src/feature/kinds/nether.rs:16-23`);
  - `netherrack_replace_blobs`, `ore`, `scattered_ore`, `overlay`, `random_neighbor_spread`, `single_block_pillar`, `block_column`, `simple_block` and `weighted_random_selector` (`third_party/minecraftoss/generator/src/feature/kinds/mod.rs:77-99`).
  - An unknown type becomes `Feature::Unsupported`, which places nothing and only logs (`third_party/minecraftoss/generator/src/feature/kinds/mod.rs:100-110`, `:141-144`).
- **End terrain (not wired).** The `end_outer_islands` density function makes the main island and the outer islands (`third_party/minecraftoss/generator/src/density/ir.rs:308`).
- **End biomes (not wired).**
  - Within 64 chunks of 0,0 the biome is `the_end`. Further out, erosion picks `end_highlands` (above 0.25), `end_midlands` (from -0.0625), `small_end_islands` (below -0.21875) or `end_barrens` (`third_party/minecraftoss/generator/src/terrain.rs:266-284`).
  - The `ParameterList::nether()` passed at `third_party/minecraftoss/generator/src/terrain.rs:252` is a placeholder. `end_biomes` overrides it for the biome step, the carver lookups and the possible-biome list (`:296`, `:380`, `:421`).
- **End features (not wired).** `end_spike`, `end_platform`, `end_gateway`, `end_island` and `chorus_plant` parse (`third_party/minecraftoss/generator/src/feature/kinds/the_end.rs:13-22`). `end_podium` does not (see The dragon fight).
- **One WorldGen per dimension (present).**
  - `WorldGen::for_dimension` loads the features, structures, sky light and creature spawns for one dimension type (`third_party/minecraftoss/world/src/chunk_map.rs:159-169`).
  - Running three dimensions means three `TerrainGenerator`/`WorldGen`/`ChunkMap` sets that share one `Arc<Registries>`.
- **World settings (present).** `world_gen_settings` lists all three dimensions with one seed (`third_party/minecraftoss/world/src/settings.rs:35-37`).

## Structures

Placement is generic. Each `Structures` (one per `WorldGen`) keeps its own start cache and filters structure sets by the biome source (`third_party/minecraftoss/generator/src/structure/mod.rs:266-283`, `:365`). So the Nether and End structures below start generating as soon as a Nether or End level runs.

Nether (all not wired):
- **Fortress.**
  - It is moved inside Y 48-70 (`third_party/minecraftoss/generator/src/structure/kinds/fortress.rs:136-140`).
  - Chests use `nether_bridge` loot (`third_party/minecraftoss/generator/src/structure/kinds/fortress.rs:599`, `:639`). Nether wart is placed (`:350`).
  - A blaze spawner is set (`:741`). Spawners are not simulated anywhere: no `BaseSpawner` code exists in `third_party/minecraftoss/world/src` or `crates/minecraft_terrain/src`.
- **Bastion remnant.**
  - It is a generic jigsaw with template entities on and finalized (`third_party/minecraftoss/generator/src/structure/jigsaw.rs:171-172`). The entities are placed at `third_party/minecraftoss/generator/src/feature/template/mod.rs:399`.
  - The `blackstone_replace` and `lava_submerged_block` processors parse (`third_party/minecraftoss/generator/src/feature/template/processor.rs:345-346`).
  - Template piglins get vanilla's structure finalize: the hunt memory and the gold armour rolls (`third_party/minecraftoss/generator/src/feature/entities.rs:223-257`).
- **Structure set.** The 26.3 `nether_complexes` set is fortress weight 2 and bastion weight 3, with spacing 27 and separation 4 (26.3 jar, `worldgen/structure_set/nether_complexes.json`).
- **Ruined portal, Nether variant.** The `in_nether` placement and blackstone replacement parse (`third_party/minecraftoss/generator/src/structure/kinds/ruined_portal.rs:30`, `:41`, `:74`, `:84`). The Overworld variants already generate.
- **Nether fossil.** Some get a dried ghast (`third_party/minecraftoss/generator/src/structure/kinds/nether_fossil.rs:118`).

Overworld structures that lead to the End:
- **Stronghold (present, generating today).** The portal room is at `third_party/minecraftoss/generator/src/structure/kinds/stronghold.rs:839-876`:
  - each of the 12 frames gets an eye when `next_f32() > 0.9`;
  - the 3x3 portal is filled only when all 12 have eyes;
  - it has a silverfish spawner.
- **Locate (partial).** `ring_chunks` and `locate_pos` exist (`third_party/minecraftoss/generator/src/structure/placement.rs:200-211`), but nothing calls them. The repo has no `/locate` and no nearest-structure search.

End (all not wired):
- **End city.**
  - It is rejected when the lowest corner is below Y 60 (`third_party/minecraftoss/generator/src/structure/kinds/end_city.rs:35-38`).
  - One ship per city (`:233-237`).
  - Chests use `end_city_treasure` loot (`:297`).
  - "Sentry" markers spawn shulkers (`:301-306`).
  - The "Elytra" marker places an item frame holding an elytra (`:307-335`).
- **Spikes, platform, gateways, islands and chorus plants** are covered under Portals and The dragon fight. The feature code is in `third_party/minecraftoss/generator/src/feature/kinds/the_end.rs`.

Entities made by worldgen:
- They are kept per chunk and loaded by `ServerSim::load_chunk` (`crates/minecraft_terrain/src/server.rs:404`).
- Only the 23 simulated kinds become live (`crates/minecraft_terrain/src/server_mobs.rs:388-412`). Items and orbs load as level entities (`crates/minecraft_terrain/src/server_mobs.rs:1328-1357`).
- Everything else goes dormant (`crates/minecraft_terrain/src/server.rs:449`): saved unchanged, never ticked, never drawn, never hit. That covers piglins, hoglins, shulkers, end crystals and the elytra item frame.
- **Bug: generated shulkers get deleted.**
  - The dormant despawn removes non-persistent Monster mobs more than 128 blocks from every player (`crates/minecraft_terrain/src/server.rs:534-544`).
  - `shulker` is in the Monster list (`third_party/minecraftoss/world/src/natural_spawner/spawning.rs:101`), and end city shulkers are created without `PersistenceRequired` (`third_party/minecraftoss/generator/src/structure/kinds/end_city.rs:304`).
  - In vanilla, `Shulker` extends `AbstractGolem`, whose `removeWhenFarAway` returns false (26.3 jar). End city shulkers never despawn by distance.

## Portals and travel

### Nether portal

Present:
- **Frame detection**, 2-21 wide and 3-21 tall (`third_party/minecraftoss/generator/src/feature/portal.rs:7-10`, `:60-153`, tests from `:270`).
- **Lighting** by fire or flint and steel fills the frame. It works only in the Overworld and the Nether, checked by name (`third_party/minecraftoss/world/src/level/fire.rs:107-142`). The client predicts it (`crates/minecraft_terrain/src/flint.rs:83-92`).
- **Breaking:** the portal breaks when its frame does (`third_party/minecraftoss/generator/src/feature/update.rs:586-596`).
- **Looks:** portal particles and the ambient hum (`crates/minecraft_terrain/src/display_ticks.rs:5`, `:50-62`), and the animated translucent mesh (`crates/minecraft_terrain/src/mesh.rs:3376`).
- **POIs:**
  - The `nether_portal` POI type exists (`third_party/minecraftoss/entities/src/poi.rs:93`, `:153`). The fork keeps POIs in step with blocks (`crates/minecraft_terrain/src/server.rs:411-435`).
  - `PoiManager::in_square` is vanilla's `getInSquare`, the query the portal search uses (`third_party/minecraftoss/entities/src/poi.rs:714`).
  - POIs are rebuilt from blocks when a chunk loads and are never saved. The `poi` folder is created but nothing writes `poi/*.mca` (`third_party/minecraftoss/world/src/level_storage.rs:18`, `:142-149`).

Missing:
- **Entering.** Entities inside `NetherPortalBlock`, `EndPortalBlock` or `EndGatewayBlock` are only logged as unsupported (`third_party/minecraftoss/world/src/level/entity.rs:1263-1271`). The fork's player has no portal check.
- **Timer and cooldown** (vanilla `PortalProcessor`). The wait comes from the game rules above: 80 ticks in survival, 0 in creative.
- **Destination** (vanilla `NetherPortalBlock.getPortalDestination`, 26.3 jar):
  1. Scale the position with `DimensionType.getTeleportationScale` (x8 or /8).
  2. Search with `PortalForcer.findClosestPortalPosition`: radius 16 into the Nether, 128 into the Overworld (`NETHER_PORTAL_RADIUS`, `OVERWORLD_PORTAL_RADIUS`). It clamps to the target's `logical_height` and the world border.
  3. Measure the found portal with `BlockUtil.getLargestRectangleAround` (up to 21 x 21).
  4. If none is found, build one with `PortalForcer.createPortal`.
  5. Place the entity with `PortalShape.getRelativePosition` and `findCollisionFreePosition`.
- **World border.** There is none, only a comment (`third_party/minecraftoss/world/src/natural_spawner.rs:588`). Vanilla's default border is far enough out to ignore at first.
- **Portal piglins.** Zombified piglins spawning from the portal's random tick in the Overworld.
- **Sounds and overlay.** The trigger and travel sounds and the portal screen overlay.

The search cost needs a decision:
- Vanilla loads POI data within the search radius. Here POIs exist only for chunks the `ServerSim` has loaded.
- A 128-block search covers 17 x 17 chunks. Either write POI files, or load those chunks first. `ChunkMap::load_now` can do that synchronously (used at `crates/minecraft_terrain/src/terrain.rs:1012`).

### End portal and eyes of ender

- **present:** the stronghold portal room (above), and comparator output from an end portal frame (`third_party/minecraftoss/world/src/level/redstone.rs:470-476`).
- **missing:** `EnderEyeItem`. That covers both putting an eye into a frame (with the frame-ring check that fills the portal) and throwing one.
- **missing:** the `EyeOfEnder` entity. The fork's throwables are the snowball, egg and ender pearl only (`crates/minecraft_terrain/src/thrown.rs:35-41`).
- **missing:** entering an end portal. It is only logged as unsupported (`third_party/minecraftoss/world/src/level/entity.rs:1265`).
- **missing:** drawing the end portal and end gateway blocks. The only fork code that names them is the bucket rule (`crates/minecraft_terrain/src/bucket.rs:68`).

Vanilla destination (`EndPortalBlock.getPortalDestination`, 26.3 jar):
- **Into the End:**
  - Take `ServerLevel.END_SPAWN_POINT`, which is (100, 50, 0), at its bottom centre: (100.5, 50, 0.5).
  - Call `EndPlatformFeature.createEndPlatform(level, (100, 49, 0), breakBlocks = true)`.
  - Players are then moved down one block, so their feet land at (100.5, 49, 0.5) facing west. Other entities arrive at y 50.
- **Out of the End:** players go to `ServerPlayer.findRespawnPositionAndUseSpawnBlock`. Other entities go to the respawn data's position.

### Spawn platform

- **Worldgen (not wired):**
  - In 26.3 the platform is the placed feature `end_platform`, fixed at (100, 49, 0) in the `the_end` biome (26.3 jar, `worldgen/placed_feature/end_platform.json`).
  - MinecraftOSS places obsidian at y 48 and air at y 49-51 (`third_party/minecraftoss/generator/src/feature/kinds/the_end.rs:146-173`).
- **Runtime (partial):**
  - Vanilla rebuilds the platform on every arrival and drops the blocks it breaks.
  - MinecraftOSS's `EndPlatform` is private and places blocks without drops (`third_party/minecraftoss/generator/src/feature/kinds/the_end.rs:147`, `:157-172`).
- **Fork arrival point (correct):**
  - `crates/minecraft_terrain/src/terrain.rs:1089` puts the player at (100.5, 49.0, 0.5). That matches vanilla for players.
  - The comment above it (`:1088`) calls (100, 49, 0) `END_SPAWN_POINT`. In fact (100, 49, 0) is the platform origin, and `END_SPAWN_POINT` is (100, 50, 0).

### End gateways

- **Worldgen gateways (not wired):**
  - The `end_gateway` feature reads `exit` and `exact` (`third_party/minecraftoss/generator/src/feature/kinds/the_end.rs:175-223`).
  - `end_gateway_return` has exit (100, 50, 0) and exact true, and is placed at rarity 1 in 700 in `end_highlands` (26.3 jar).
  - The exit is written by `Region::set_gateway_exit` (`third_party/minecraftoss/generator/src/feature/region.rs:273-282`). The block entity keeps `Age`, `exit_portal` and `ExactTeleport` (`third_party/minecraftoss/core/src/block_entity.rs:241-249`).
- **Dragon-fight gateways (partial):** the `end_gateway_delayed` config (no exit) parses. Nothing spawns it (see The dragon fight).
- **missing:** gateway teleporting. That includes vanilla's `TheEndGatewayBlockEntity` `findExitPosition`, `findOrCreateValidTeleportPos` and `spawnGatewayPortal`, and the cooldown and beam ticks.
- **Runtime feature writes are dropped.**
  - The live `Level`'s `World` impl (`third_party/minecraftoss/world/src/level/mod.rs:856-930`) does not override `add_entity`, `set_spawner_entity` or `set_gateway_exit`.
  - The defaults only log them as unsupported (`third_party/minecraftoss/generator/src/feature/mod.rs:647-667`).
  - So a feature placed at runtime (podium, gateway, platform) loses its entities and gateway exits.

### Respawn

- **MW2 respawn** is at the map origin (`crates/sim/src/voxel.rs:596-610`). The map origin is the Overworld player spawn (`crates/render_anim/src/minecraft_world.rs:457-458`).
- **missing: beds and anchors.** The fork has no sleeping and no spawn point. Beds don't explode in the Nether or End, and respawn anchors only have comparator output (`third_party/minecraftoss/world/src/level/redstone.rs:478`).
- **partial: explosions.** The engine can be reused (`third_party/minecraftoss/world/src/level/explosion.rs:108-118`), but `fire: true` is only logged as unsupported (`:114-116`). Bed and anchor blasts start fires.
- **partial: level.dat spawn.**
  - `RespawnData` has a `dimension` field (`third_party/minecraftoss/world/src/level_storage.rs:28-33`).
  - The fork always writes the Overworld (`crates/render_anim/src/minecraft_save.rs:149-153`), and on reading it ignores a spawn in any other dimension (`crates/render_anim/src/minecraft_save.rs:262`).
- **missing: player position.** The player file stores only the game mode, with no `Dimension` and no `Pos` (`crates/render_anim/src/minecraft_save.rs:109-124`).

## Mobs and AI

Natural spawns in 26.3 (biome attribute `gameplay/natural_mob_spawns`, 26.3 jar):
- **Every Nether biome:** strider (creature).
- **nether_wastes:** ghast, zombified piglin, magma cube, enderman, piglin.
- **soul_sand_valley:** skeleton, ghast, enderman.
- **crimson_forest:** zombified piglin, hoglin, piglin.
- **warped_forest:** enderman.
- **basalt_deltas:** ghast, magma cube.
- **Every End biome:** enderman only. Every other End mob comes from structures or the fight.

How natural spawning treats them here:
- Natural spawning only places types its `type_info` knows. Strider, enderman and skeleton are listed (`third_party/minecraftoss/world/src/natural_spawner.rs:274`, `:278`, `:283`).
- Any other type is logged as unsupported and skipped (`third_party/minecraftoss/world/src/natural_spawner/spawning.rs:427-430`).
- So a Nether level today would spawn endermen, skeletons and striders, and the striders would go dormant.

| Mob | Server AI | Renderer | Status |
| --- | --- | --- | --- |
| Enderman | `third_party/minecraftoss/entities/src/enderman.rs`, anger at `third_party/minecraftoss/entities/src/monster_ai.rs:2198`, simulated (`crates/minecraft_terrain/src/server_mobs.rs:395`) | `crates/minecraft_terrain/src/enderman_render.rs` | present. It covers every End natural spawn. |
| Skeleton (soul sand valley) | simulated (`crates/minecraft_terrain/src/server_mobs.rs:403`) | `crates/minecraft_terrain/src/skeleton_render.rs` | present |
| Strider | none. `InLava` placement exists (`third_party/minecraftoss/world/src/natural_spawner.rs:274`), and so does `can_stand_on_fluid` (`third_party/minecraftoss/entities/src/walk_path.rs:28`) | none | missing. Spawns, then goes dormant. |
| Zombified piglin | none. `ZombieKind` has no variant (`third_party/minecraftoss/entities/src/zombie.rs:8-16`). Reuse `zombie_goals` (`third_party/minecraftoss/entities/src/monster_ai.rs:1379`) and the enderman anger | none | missing |
| Magma cube | none. Reuse the slime (`third_party/minecraftoss/entities/src/slime.rs`, `slime_goals` at `third_party/minecraftoss/entities/src/monster_ai.rs:977`) | none. `crates/minecraft_terrain/src/slime_render.rs` can be adapted (vanilla `MagmaCubeModel` has 8 segments on a 64x64 sheet) | missing, close to slime |
| Ghast + fireball | none. There is only ground navigation (`third_party/minecraftoss/entities/src/navigation.rs:255`), and arrows are the only projectile (`third_party/minecraftoss/entities/src/projectile.rs:13`) | none | missing |
| Blaze + small fireball | none. The fortress places its spawner (`third_party/minecraftoss/generator/src/structure/kinds/fortress.rs:741`), but spawners never tick | none | missing |
| Wither skeleton | none. `SkeletonKind` has no variant (`third_party/minecraftoss/entities/src/skeleton.rs:9-17`), and `skeleton_goals` always adds the bow (`third_party/minecraftoss/entities/src/monster_ai.rs:1096-1106`) | `crates/minecraft_terrain/src/skeleton_render.rs` can draw it at scale 1.2 | missing |
| Piglin | none. The only brain framework is `third_party/minecraftoss/entities/src/villager_brain/` | none. Only the head block model exists (`crates/minecraft_terrain/src/special_models.rs:827`) | partial. Generated in bastions (`third_party/minecraftoss/generator/src/feature/entities.rs:223-257`), then dormant. |
| Piglin brute | none | none | missing. Not in `MobCategory::of_type` (`third_party/minecraftoss/world/src/natural_spawner/spawning.rs:97-111`). |
| Hoglin | none | none | missing |
| Zoglin | none | none | missing. Not in `MobCategory::of_type`. |
| Shulker + bullet | none. No levitation effect (`third_party/minecraftoss/entities/src/effects.rs:17-27`, `third_party/minecraftoss/player/src/survival.rs:36-48`) | none. The shulker box lid and base exist (`crates/minecraft_terrain/src/special_models.rs:784`) | partial. Generated in end cities, dormant, and deleted by the despawn bug above. |
| Endermite | none. The enderman's endermite target only rolls (`third_party/minecraftoss/entities/src/monster_ai.rs:1980-1990`) | none | missing |
| End crystal | none | none | partial. One is generated on each spike with no beam target (`third_party/minecraftoss/generator/src/feature/kinds/the_end.rs:120-128`), then dormant. |
| Ender dragon | none | none. Only the head block model exists (`crates/minecraft_terrain/src/special_models.rs:808`) | missing (see The dragon fight) |
| Wither | none | none | missing. Not native to either dimension. Listed because it is the other boss. |

Gaps that affect several mobs:
- **Fire immunity: missing.** The hazard trait has no per-type fire immunity (`third_party/minecraftoss/entities/src/world/hazards.rs:15-22`). The spawner's `type_info` has a `fire_immune` flag, but only for spawn placement (`third_party/minecraftoss/world/src/natural_spawner.rs:254`).
- **Mob categories: partial.** `MobCategory::of_type` lacks wither_skeleton, piglin_brute, zoglin, endermite, ender_dragon and wither (`third_party/minecraftoss/world/src/natural_spawner/spawning.rs:97-111`).
- **Spawners: missing.** Fortress blaze spawners and the stronghold silverfish spawner never spawn.
- **Boss bar: missing.** No crate has one.
- **Item frames: missing.** The end city elytra frame goes dormant, so the elytra cannot be reached.
- **/summon and spawn eggs.** Both only accept the simulated kinds (`crates/render_anim/src/minecraft_chat.rs:50`, `crates/minecraft_terrain/src/server_mobs.rs:388`). `/summon minecraft:ghast` fails until the ghast is simulated.

Wiring in a new mob kind:
1. **MinecraftOSS `entities`:**
   - a state module, declared in `third_party/minecraftoss/entities/src/lib.rs`;
   - a `MonsterKind` with its goal selectors (`third_party/minecraftoss/entities/src/monster_ai.rs:36`);
   - storage in `EntityWorld` (`third_party/minecraftoss/entities/src/world.rs:2238`), a `MobHit` entry (`:1902`) and an `entity_type` entry (`:3760`).
2. **Server:**
   - add the kind to `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:388`);
   - add a `spawn_saved` arm (`crates/minecraft_terrain/src/server_mobs.rs:416`, which falls through to `return None` at `:875`);
   - add a save block in `mob_tags` (`crates/minecraft_terrain/src/server_mobs.rs:1029`).
3. **Client:** a push in `server_mobs()` (`crates/minecraft_terrain/src/client_mobs.rs:1024`) and an entry in `shadow_casters` (`crates/minecraft_terrain/src/client_mobs.rs:1115`).
4. **Rendering:**
   - a new `*_render.rs`;
   - its textures in the atlas list (`crates/minecraft_terrain/src/mesh.rs:496-498` shows the pattern);
   - a call in `meshes()` (`crates/render_anim/src/minecraft_entities.rs:822`);
   - a hit box in `boxes()` (`crates/render_anim/src/minecraft_entities.rs:754`) with `encode`/`decode` entries (`:999`, `:1021`).
   - The 26.3 jar has no JSON entity models, so each model's cuboids come from javap on its `createBodyLayer` and `setupAnim`.
5. **Spawning:** `type_info` (`third_party/minecraftoss/world/src/natural_spawner.rs:252-323`) and `MobCategory::of_type` (`third_party/minecraftoss/world/src/natural_spawner/spawning.rs:97-111`).

## The dragon fight

Vanilla (26.3 jar):
- **Where it lives.** `net.minecraft.world.level.dimension.end.EnderDragonFight`, with `DragonRespawnStage` beside it.
- **When it exists.** `ServerLevel` creates it only when `DimensionType.hasEnderDragonFight()` is true. It loads it from the level's own data storage (`ServerChunkCache.getDataStorage`), so it is saved as `ender_dragon_fight` in the End's own `data` folder, not in the world root's.
- **Saved fields:** `needs_state_scanning`, `dragon_killed`, `previously_killed`, `respawn_stage`, `respawn_time`, `dragon_uuid`, `exit_portal_location`, `gateways`, `respawn_crystals`.
- **Constants:**
  - `GATEWAY_COUNT` 20 and `GATEWAY_DISTANCE` 96. New gateways go on that ring at y 75.
  - `DRAGON_SPAWN_Y` 128.
  - `ARENA_SIZE_CHUNKS` 8.
  - `TIME_BETWEEN_CRYSTAL_SCANS` 100 and `TIME_BETWEEN_PLAYER_SCANS` 20.
  - `MAX_TICKS_BEFORE_DRAGON_RESPAWN` 1200.
- **Main methods:** `tick`, `scanState`, `findOrCreateDragon`, `findExitPortal`, `spawnExitPortal`, `setDragonKilled`, `spawnNewGateway`, `onCrystalDestroyed`, `tryRespawn`, `respawnDragon` and `resetSpikeCrystals`.
- **Phases:** `entity/boss/enderdragon/phases/` holds holding pattern, strafe player, landing approach, landing, sitting scanning, sitting flaming, sitting attacking, charge player, takeoff, hover and dying, run by `EnderDragonPhaseManager`.
- **Exit portal.** `worldgen/feature/end_podium_active.json` and `end_podium_inactive.json` have type `minecraft:end_podium` (class `EndPodiumFeature`).

What the repo has:
- **The ten spikes (present, not wired).**
  - `spikes_for_seed` matches `EndSpikeFeature.getSpikesForLevel` (`third_party/minecraftoss/generator/src/feature/kinds/the_end.rs:34-55`).
  - Each spike is obsidian with bedrock and fire on top, and iron-bar cages on the guarded ones (`third_party/minecraftoss/generator/src/feature/kinds/the_end.rs:83-131`).
  - Each spike gets one end crystal with no beam target, which can be destroyed (`third_party/minecraftoss/generator/src/feature/kinds/the_end.rs:120-128`). The crystal goes dormant (see Structures).
- **Gateway block entity data (present):** `exit_portal` and `ExactTeleport` survive saving (`third_party/minecraftoss/core/src/block_entity.rs:241-249`).
- **Dragon head model (present):** the head block, with the `entity/enderdragon/dragon` texture (`crates/minecraft_terrain/src/special_models.rs:158`, `:808`).
- **Explosion engine (present):** an end crystal blast can call `level.explode` (`third_party/minecraftoss/world/src/level/explosion.rs:108`).

What is missing:
- **`end_podium`.** It is not in the feature parser (`third_party/minecraftoss/generator/src/feature/kinds/the_end.rs:13-22`), so it falls to `Feature::Unsupported` (`third_party/minecraftoss/generator/src/feature/kinds/mod.rs:109`).
- **The fight manager:** all of `EnderDragonFight` and `DragonRespawnStage`.
- **The dragon:** the entity, its phase machine and `DragonFlightHistory`, its multipart body (`EnderDragonPart`) and crystal healing.
- **The crystals:** a live crystal entity, its beam and its explosion.
- **Gateways:** the 20-gateway ring and gateway teleporting.
- **The rest:** the dragon egg, the boss bar and the dragon's death XP and animation.
- **Saving it.** The saved-data API only writes to the world root's `data` folder (`third_party/minecraftoss/world/src/level_storage.rs:161-182`). It needs a per-dimension path.

Fork-specific work:
- **Hit boxes.** `boxes()` gives one box per entity id (`crates/render_anim/src/minecraft_entities.rs:754`). MW2 weapons hit through these boxes, so the dragon's parts need several boxes per id.
- **Runtime placement.** The podium and gateways are placed at runtime, which needs working `add_entity` and `set_gateway_exit` on the live `Level` (see End gateways).

## Rendering per dimension

- **Environment (present).**
  - `DimensionEnvironment::load` works for any dimension type (`crates/minecraft_terrain/src/environment.rs:97-107`), and a test loads the Nether and End (`:293-298`).
  - `sky_state` samples fog colour, fog start and end, sky colour, ambient colour, sky light colour and factor, and cloud colour and alpha (`crates/minecraft_terrain/src/environment.rs:157-222`).
  - The values reach the shader through `crates/render_anim/src/minecraft_world.rs:1969-2023`.
- **Lightmap (present).** The shader adds ambient light, sky light colour times its factor, and tinted block light (`crates/render_gpu/src/drawsurf/minecraft_world.rs:775-789`). The Nether's `#302821` ambient with sky factor 0, and the End's `#3f473f` ambient with factor 0, pass through it unchanged.
- **Skybox (not wired).**
  - The mode is sent as the `w` of the ambient slot: 0 overworld, 1 end, 2 none (`crates/render_anim/src/minecraft_world.rs:2012-2016`).
  - `sky_fragment` ignores it and always draws the gradient, stars, sun and moon (`crates/render_gpu/src/drawsurf/minecraft_world.rs:926-956`).
  - In vanilla the Nether draws no sky, only fog colour. The End draws its `end_sky` texture, which is never loaded here: only the sun and moon image is (`crates/render_anim/src/minecraft_world.rs:301`).
  - The Nether and End have no day timeline, so the sun angle stays at its default and the sun would sit overhead.
- **Clouds (not wired).**
  - Cloud alpha is a constant 0.8 in the shader (`crates/render_gpu/src/drawsurf/minecraft_world.rs:978-980`), and the height is fixed at 192.33 (`crates/minecraft_terrain/src/clouds.rs:48-66`).
  - `cloud_alpha` is computed (`crates/minecraft_terrain/src/environment.rs:205`) but never sent. The slot's `w` carries the brightness option instead (`crates/render_anim/src/minecraft_world.rs:1999`).
  - The Nether and End set no `visual/cloud_color` (26.3 jar), and vanilla draws no clouds there. Here they would still be drawn.
- **Block face shading (missing).**
  - Vanilla `CardinalLighting.NETHER` shades the top and bottom faces 0.9, where the default is 1.0 and 0.5 (26.3 jar).
  - The mesher hard-codes the default (`crates/minecraft_terrain/src/mesh.rs:438-443`, also `:2047`, `:2259`, `:2724`).
  - The flag is parsed as `nether_cardinal_light` (`third_party/minecraftoss/core/src/environment.rs:831`, `:867`).
- **Entity lighting (not wired).** `set_nether_entity_lighting` moves the second entity light below (`crates/minecraft_terrain/src/cow_render.rs:335-350`). Nothing calls it.
- **Fog (present).** It follows the environment. The Nether fog colour comes from each biome, with start 10 and end 96. The End fog is `#181318`.
- **Old day cycle.** `crates/minecraft_terrain/src/day_cycle.rs` is the older authored Overworld cycle. Its skybox is fixed to Overworld (`:245`). The client still keeps it in `Runtime` (`crates/render_anim/src/minecraft_world.rs:130`), but the sky colours come from `DimensionEnvironment`.

## What the game hard-wires to the overworld today

- **Dimension literals:**
  - `crates/render_anim/src/minecraft_world.rs:279` opens the stream for `Dimension::Overworld`, and `:300` loads only the Overworld environment.
  - `crates/render_anim/src/minecraft_entities.rs:163` starts `ServerSim::new(..., "minecraft:overworld")`.
  - `crates/render_anim/src/minecraft_save.rs:16` defines `OVERWORLD`. It is used for the spawn's dimension (`:150`), the only clock written (`:168`), the only dimension folder created (`:259`), the only spawn read back (`:262`) and the only clock read back (`:276`).
  - `/time` always uses the overworld clock (`crates/minecraft_commands/src/commands.rs:15`). Vanilla uses the executing level's default clock.
  - `crates/assets/src/minecraft_map.rs:6` names the MW2 map `minecraft:overworld`. It is a map id, not a dimension, and can stay.
- **One level, one `Loaded`.** `load()` builds a single `Loaded` (`crates/render_anim/src/minecraft_world.rs:256-324`). That struct mixes per-world state (packs, registries, seed, save, rules, chat) with per-dimension state (`stream`, `scene`, `environment`) (`crates/render_anim/src/minecraft_world.rs:73-91`).
- **Server and client in one stream.** `TerrainStream` holds the server `ChunkMap` together with the client's sections, light and workers (`crates/minecraft_terrain/src/terrain.rs:943-971`).
- **One simulated level.** `ServerSim` holds one `Level` and one `EntityWorld` (`crates/minecraft_terrain/src/server.rs:39-67`), and `server_loop` owns exactly one (`crates/minecraft_terrain/src/server.rs:1425-1561`).
- **One player.** The chunk map tracks a single player (`third_party/minecraftoss/world/src/chunk_map.rs:610-612`). The server tick assumes player 0 (`crates/minecraft_terrain/src/server.rs:1508-1517`).
- **Saved terrain comes from the client.** The host scene's block edits are pushed into the `ChunkMap` (`crates/render_anim/src/minecraft_world.rs:1567-1568`), so the saved terrain is the client copy, not the `Level`'s.
- **Time comes from the client.** `DayCycle` lives in the client `Runtime` (`crates/render_anim/src/minecraft_world.rs:130`), and the server only receives day ticks (`crates/minecraft_terrain/src/server.rs:1493`).
- **Per-level copies of server-wide state.**
  - Clocks are copied into every level (`third_party/minecraftoss/world/src/level/time.rs:62-65`).
  - Entity ids are counted per level (`third_party/minecraftoss/world/src/level/mod.rs:152`, `:260`) and per entity world (`third_party/minecraftoss/entities/src/world.rs:2266`, `:2362`).
- **Containers.** They are keyed by `BlockPos` alone, and the per-player ender chest sits in the same struct (`crates/minecraft_terrain/src/containers.rs:428-437`).
- **Collision.** There is one global voxel world (`crates/sim/src/voxel.rs:41`), switched with `activate` and `deactivate` (`crates/sim/src/voxel.rs:403-429`).
- **Spawn.**
  - The Nether spawn is a fork stand-in that searches for a floor near chunk 0,0 (`crates/minecraft_terrain/src/terrain.rs:1001-1030`, `:1084-1087`). Vanilla has no Nether spawn: players arrive through `PortalForcer`.
  - Respawn goes to the Overworld origin (see Respawn).
- **Resources per stream.**
  - Each `TerrainStream` starts three quarters of the hardware threads for generation and half minus one for section building (`crates/minecraft_terrain/src/terrain.rs:1054-1063`), plus a chunk saver thread (`third_party/minecraftoss/world/src/chunk_map.rs:519`).
  - Each makes its own temporary session folder when there is no save (`crates/minecraft_terrain/src/terrain.rs:1068`).
  - Each `ServerSim` leaks its `WorldGen` (`crates/minecraft_terrain/src/server.rs:197`).

## Reuse plan

Labels:
- **call:** the code exists, just call it.
- **port:** vanilla logic that is not in the repo. Port it from javap and the data pack, without copying code.
- **write:** fork code with no vanilla counterpart.

The roadmap puts multiplayer (1.4) between Nether travel and the End. Everything below has to keep working with several levels loaded at once.

### 1. Groundwork (with the crate split, roadmap 1.2)

1. **write:** move `Dimension` (`crates/minecraft_terrain/src/terrain.rs:973-999`) into the headless crate as the level key. Replace the literals listed above.
2. **write:** split `TerrainStream` (`crates/minecraft_terrain/src/terrain.rs:943-971`).
   - The server side keeps the `ChunkMap`, spawn and storage.
   - The client side keeps sections, light and workers, only for the player's dimension.
3. **write:** one `server_loop` that owns every level, with the dimension in each `Command` (`crates/minecraft_terrain/src/server.rs:1425-1561`). Vanilla ticks every level on one thread.
4. **write:** make clocks server-wide.
   - Drop the per-level copies (`third_party/minecraftoss/world/src/level/time.rs:62-65`).
   - Let `set_time` and `/time` act on the server clock map.
   - Feed mobs from that map instead of the level's own overworld clock (`crates/minecraft_terrain/src/server.rs:441`).
5. **write:** in `ServerSim::new`, set `level.sea_level` from `worldgen.terrain.sea_level` and `level.fast_lava` from `gameplay/fast_lava`. Today neither is set (`third_party/minecraftoss/world/src/level/mod.rs:238`, `:271`).
6. **write:** storage.
   - Call `create_dimension` for each level (`crates/render_anim/src/minecraft_save.rs:259`).
   - Add a per-dimension saved-data path next to `third_party/minecraftoss/world/src/level_storage.rs:161`.
   - Write and read every clock (`crates/render_anim/src/minecraft_save.rs:164-169`, `:276`).
   - Save the spawn's real dimension (`crates/render_anim/src/minecraft_save.rs:150`).
   - Add `Dimension` and `Pos` to the player file (`crates/render_anim/src/minecraft_save.rs:109-124`).
7. **write:** one session folder per world (`crates/minecraft_terrain/src/terrain.rs:1068`), and one generation thread budget shared by all levels (`crates/minecraft_terrain/src/terrain.rs:1054-1063`).
8. **write:** fix the shulker despawn now, because it deletes saved data. Keep dormant types whose vanilla `removeWhenFarAway` is false (`crates/minecraft_terrain/src/server.rs:534-544`).

### 2. A Nether level that runs (no travel yet)

1. **call:**
   - `TerrainStream::for_dimension_at(.., Dimension::Nether, ..)` (`crates/minecraft_terrain/src/terrain.rs:1043`);
   - `ServerSim::new(.., "minecraft:the_nether")` (`crates/minecraft_terrain/src/server.rs:196`);
   - `DimensionEnvironment::load(.., "minecraft:the_nether")` (`crates/minecraft_terrain/src/environment.rs:99`).
2. **write:** a headless test that generates Nether and End chunks around 0,0, like the Overworld one at `crates/minecraft_terrain/src/terrain.rs:2153`. It is the first time that code runs.
3. **write:** a debug way to start in the Nether, so it can be tested before portals work.
4. **write:** the sky.
   - Make `sky_fragment` branch on the skybox mode: fog colour only for `none`, the `end_sky` texture for `end` (`crates/render_gpu/src/drawsurf/minecraft_world.rs:926-956`).
   - Hide clouds when the cloud alpha is 0.
5. **write:** Nether face shading of 0.9 for the top and bottom faces (`crates/minecraft_terrain/src/mesh.rs:438-443` and the other three places), and call `set_nether_entity_lighting` (`crates/minecraft_terrain/src/cow_render.rs:335`).

### 3. Nether travel (roadmap 1.3)

1. **port:** the portal timer and cooldown (`PortalProcessor`), reading the three game rules (`third_party/minecraftoss/world/src/game_rules.rs:115`, `:146-147`).
2. **port:** `NetherPortalBlock.getPortalDestination` with `DimensionType.getTeleportationScale`, clamped to `logical_height`.
3. **port:** `PortalForcer.findClosestPortalPosition` on top of `PoiManager::in_square` (`third_party/minecraftoss/entities/src/poi.rs:714`).
   - Decide first between writing POI files and loading the search area with `ChunkMap::load_now`.
   - Also port `BlockUtil.getLargestRectangleAround`, `PortalForcer.createPortal`, `PortalShape.getRelativePosition` and `PortalShape.findCollisionFreePosition`.
   - **call:** `PortalShape` for the frame (`third_party/minecraftoss/generator/src/feature/portal.rs:60-153`).
4. **write:** the switch, which keeps the player and swaps the level.
   - Dropping a `ChunkMap` saves it (`third_party/minecraftoss/world/src/chunk_map.rs:908`).
   - Bump `view.generation` to drop stale sections (`crates/render_anim/src/minecraft_world.rs:505`).
   - Swap collision (`crates/sim/src/voxel.rs:403-429`).
   - Hold the player with `SpawnGate` until the chunks around them have collision (`crates/render_anim/src/minecraft_spawn_gate.rs`).
5. **write:** remove the stand-in Nether spawn (`crates/minecraft_terrain/src/terrain.rs:1001-1030`).
6. **port:** the portal trigger and travel sounds, the portal overlay, and portal piglins (`nether_portal_spawns_piglin`).

### 4. Nether rules and mobs

1. **port:**
   - beds that explode (`bed_rule.destroy_on_use`);
   - respawn anchors;
   - fire from explosions (`third_party/minecraftoss/world/src/level/explosion.rs:114`);
   - per-type fire immunity.
2. **port** the mobs in this order, each wired as in Mobs and AI:
   - zombified piglin (a zombie variant with anger);
   - magma cube (a slime variant);
   - ghast and its fireball (needs flying movement and a hurting projectile);
   - wither skeleton (a skeleton variant with melee);
   - blaze, together with monster spawner ticking;
   - piglin and hoglin (brain AI);
   - strider;
   - piglin brute and zoglin.

### 5. Getting to the End

1. **call:** `ring_chunks` and `locate_pos` (`third_party/minecraftoss/generator/src/structure/placement.rs:200-211`) to find the nearest stronghold.
2. **port:** `EnderEyeItem`.
   - Using it on a frame fills the frame and checks the frame ring.
   - Throwing it spawns the `EyeOfEnder` entity. Build it next to the other throwables in `crates/minecraft_terrain/src/thrown.rs`.
3. **port:** end portal entry (`EndPortalBlock.getPortalDestination`). Rebuild the platform with drops on every arrival. Make MinecraftOSS's `EndPlatform` usable from outside, or write a version that drops the blocks it breaks (`third_party/minecraftoss/generator/src/feature/kinds/the_end.rs:146-173`).
4. **write:** the end portal and end gateway block renderers.

### 6. The End level (roadmap 1.5)

1. **call:** the same three calls as in step 2, for `Dimension::End`. The arrival point is already right (`crates/minecraft_terrain/src/terrain.rs:1089`).
2. **write:** `add_entity`, `set_spawner_entity` and `set_gateway_exit` for the live `Level` (`third_party/minecraftoss/world/src/level/mod.rs:856-930`).
3. **port:** the shulker, its bullet and levitation; the end crystal (entity, beam and explosion); and the endermite, including endermites from pearl landings (`crates/minecraft_terrain/src/server.rs:727`).
4. **port:** item frames, so the end city elytra can be taken.

### 7. The dragon fight

1. **port:** the `end_podium` feature (`EndPodiumFeature`), as a new arm in `third_party/minecraftoss/generator/src/feature/kinds/the_end.rs:13-22`.
2. **port:** `EnderDragonFight`, saved as `ender_dragon_fight` in the End's `data` folder. That includes the crystal scan, the exit portal, the egg, the gateway ring and the respawn with four crystals.
3. **port:** the dragon with its phases, `DragonFlightHistory` and `EnderDragonPart`, and the boss bar.
4. **write:** several hit boxes per entity id (`crates/render_anim/src/minecraft_entities.rs:754`).
5. **port:** gateway teleporting (`TheEndGatewayBlockEntity`: `findExitPosition`, `findOrCreateValidTeleportPos`, `spawnGatewayPortal`), using the stored exits (`third_party/minecraftoss/core/src/block_entity.rs:241-249`).

### 8. Leaving the End and respawning

1. **port:** the exit portal sends players to `findRespawnPositionAndUseSpawnBlock`. That needs a real respawn point with a dimension, saved in the player file and level.dat.
2. **write:** MW2 respawn at that point instead of the Overworld origin (`crates/sim/src/voxel.rs:596-610`).
