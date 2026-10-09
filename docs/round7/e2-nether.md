# Track E2: the Nether (design brief)

Owner, screenshot 8: the portal lights but travel does nothing. The goal is vanilla 26.3 travel, look, rules, mobs and structures. State is per `Player` (UUID) and per `Level`, and several levels must be loadable at once. Paths are as of a43d0fc. Vanilla facts were checked with javap on the 26.3 `client.jar`; no code was copied.

## 0. What this track needs from other tracks

**E1, the dimension switch:**
- `Server::load_level(&Globals, Dimension, arrival: Option<BlockPos>) -> Level`, run on the loader thread.
  - The Nether stream takes the arrival point instead of running the stand-in `nether_spawn` (`minecraft_terrain/src/terrain.rs:977-1004`, `:1058-1061`).
  - `Entities::new` takes the dimension type (`minecraft_survival/src/entities.rs:160` hard-codes the overworld).
- An adapter `switch_level(uuid, to, feet, yaw, pitch)`. It must:
  - keep the `Player`, set `player.dimension`, and save and drop an old level once it is empty;
  - re-activate `sim::voxel` with a new `view.origin` and bump `view.generation`;
  - reset the light, light volume, clouds and environment;
  - start `SpawnGate` and teleport with D's `teleport_facing`.
- A `prepare: FnOnce(&mut Level) -> Arrival` hook. It runs on the loader thread after the target level is built and before the player is placed, and holds the portal search and build.
- The debug start `IW4L_MINECRAFT_DIMENSION=the_nether`, and server-wide clocks (the Nether has none).

**Track D, saving:**
- Commit 7: `Dimension` and `Pos` are read at join, and E1 loads that level first. Drop D's fallback that sends a non-overworld position to the world spawn.
- Commit 9: `Player.respawn: Option<RespawnConfig>` with a setter. The respawn resolver also needs a hook, `spawn_block(&mut Level, pos) -> Option<[f64; 3]>`, that runs before the world spawn; anchors use it to check and spend a charge.
- Add `PortalCooldown` (int, from `Entity.saveWithoutId`) to the keys `player_nbt` owns.

## 1. What exists

| Area | State |
|---|---|
| Portal frame, lighting and breaking | Present (`minecraftoss/generator/src/feature/portal.rs:7-10`, `:60-153`). Lighting works in the Overworld and Nether only (`world/src/level/fire.rs:107-120`). |
| Portal looks | Present: hum and particles (`minecraft_terrain/src/display_ticks.rs:5`, `:50-62`) and the animated texture. |
| Portal POIs | Only on the server thread (`minecraft_terrain/src/server.rs:220`, `:411-433`), never saved. `PoiManager::in_square` is at `entities/src/poi.rs:714`. |
| Entering a portal | Missing. It is logged as unsupported (`world/src/level/entity.rs:1263-1269`). |
| Portal game rules | Declared and saved only (`world/src/game_rules.rs:115`, `:146-147`). |
| Nether generation | Exists (generator, biomes, fortress, bastion, ruined portal, fossil) but has never run (`terrain.rs:1017-1061`). |
| Dimension rules | `water_evaporates` works (client `environment.rs:148`, server `ice.rs`). `fast_lava` and sea level are never set (`world/src/level/mod.rs:238`, `:271`), though the lava numbers exist (`fluid.rs:59-78`). `bed_rule`, `respawn_anchor_works` and `nether_portal_spawns_piglin` are only declared (`core/src/environment.rs:138-141`). |
| Sky | The skybox mode is sent (`render_anim/src/minecraft_world.rs:1955-1959`) and ignored (`render_gpu/src/drawsurf/minecraft_world.rs:926-956`). Clouds always draw (`:979`). |
| Fog and lightmap | Present: biome fog colour, fog from 10 to 96, ambient `#302821`, sky factor 0. |
| Face and entity shading | Face shading is hard-coded to the default (`mesh.rs:438-443`, `:2045-2049`, `:2259`, `:2724`). `set_nether_entity_lighting` (`cow_render.rs:335`) is never called. |
| Explosions | `fire: true` is unsupported (`explosion.rs:114-116`). The entity-world blast is private (`entities/src/world.rs:2721`). |
| Beds and anchors | No use handling. Anchors only have comparator output (`redstone.rs:478`). |
| Compass and clock | `item_model_leaf` always picks the fallback (`interface.rs:9-34`; also `special_models.rs:482-494`), so neither moves anywhere. |
| Mobs | Simulated kinds are at `server_mobs.rs:388`. Enderman and skeleton are present. `ZombieKind`/`SkeletonKind` have no Nether variants. There is no fire immunity (`world/hazards.rs:15-22`), no wither effect, no spawner ticking and no structure `spawn_overrides`. |
| Spawning | `type_info` only knows enderman, skeleton and strider (`natural_spawner.rs:252-288`). `of_type` lacks wither_skeleton, piglin_brute and zoglin (`spawning.rs:97-111`). **Correction to NETHER-END.md:** the strider gets `Rules::Other`, which returns false (`spawning.rs:573-576`), so striders never spawn. |
| Renderers | Hand-written cuboids from javap of `createBodyLayer` (`slime_render.rs`, `skeleton_render.rs`). Textures are registered at `mesh.rs:480-603`, meshes built at `entities.rs:784-884`, hit boxes at `:715-773`, `encode`/`decode` at `:939`/`:961`. The pack has every Nether mob texture (`piglin/`, `hoglin/`, `ghast/`, `blaze/`, `slime/magmacube`, `skeleton/wither_skeleton`, `strider/`). Fireballs are the `fire_charge` sprite (`thrown::append_sprites`). |
| HUD overlay | `hud/src/minecraft_fire.rs` draws from an animated strip (`ui.fire_frames`); the portal overlay copies it. |
| Block meshing | Generic, from pack models, with generic animated textures (`mesh.rs:3366-3385`). |

## 2. Vanilla 26.3 behaviour (javap)

**Timer**
- `NetherPortalBlock.entityInside` calls `Entity.setAsInsidePortal`.
  - On cooldown it resets the cooldown and returns, so standing in the arrival portal never sends you back.
  - Otherwise it starts or refreshes the `PortalProcessor` and flags the player inside this tick.
- `Entity.handlePortal` runs each tick:
  - the cooldown drops by 1 (`ServerPlayer` skips this while changing dimension);
  - inside, `processPortalTeleportation` returns `portalTime++ >= transitionTime`;
  - outside, `portalTime` drops by 4, and the processor goes at 0.
- A trigger sets the cooldown to `getDimensionChangingDelay`: 10 for `Player`, 300 for other entities.
- `getPortalTransitionTime`: `abilities.invulnerable` gives `players_nether_portal_creative_delay` (0), otherwise `players_nether_portal_default_delay` (80).
- `ServerLevel.isAllowedToEnterPortal` checks `allow_entering_nether_using_portals`.

**Destination**
- `NetherPortalBlock.getPortalDestination`: x and z scale by `DimensionType.getTeleportationScale`, then `WorldBorder.clampToBounds`.
- `PortalForcer.findClosestPortalPosition`:
  - It looks through `nether_portal` POIs with `getInSquare`: radius 16 into the Nether, 128 into the Overworld, at every height.
  - It keeps those inside the border whose block still has `horizontal_axis`.
  - The closest by `distSqr` wins, then the lowest Y.
  - The portal is measured with `BlockUtil.getLargestRectangleAround(pos, axis, 21, Y, 21)`.
- `PortalForcer.createPortal`, when nothing is found (never for spectators):
  - Its top is `min(maxY, minY + logical_height - 1)`, which is 127 in the Nether.
  - It walks `spiralAround(pos, 16, EAST, SOUTH)` down from the `MOTION_BLOCKING` height. A spot needs replaceable blocks with no fluid and must pass `canHostFrame`; the closest wins.
  - Fallback: y is `clamp(y, max(minY+1, 70), maxY-9)`. It builds a 2x3 obsidian floor with air above, the frame, and a 2x3 portal.
- `createDimensionTransition`:
  - The player's relative place in the entry portal (`PortalShape.getRelativePosition`) maps into the exit portal.
  - Yaw turns by 0 when the axes match and by 90 when they don't. Momentum is kept.
  - The spot is checked with `PortalShape.findCollisionFreePosition`.

**Sounds and overlay**
- `LocalPlayer.handlePortalTransitionEffect`:
  - intensity goes up 0.0125 per tick inside and down 0.05 per tick outside;
  - at 0, stepping in plays `block.portal.trigger` (volume 0.25, pitch 0.8-1.2);
  - stepping in closes container screens.
- Arrival is level event 1032: `block.portal.travel` with the same values.
- `Hud.extractPortalOverlay`: below 1, the alpha is `i^4 * 0.8 + 0.2`, over a full-screen `nether_portal` sprite. The loading screen shows a portal background (`LevelLoadingScreen.Reason.NETHER_PORTAL`).
- `NetherPortalBlock.randomTick` makes Overworld portals spawn zombified piglins: with `nether_portal_spawns_piglin`, outside peaceful, when `nextInt(2000) < difficulty`.

**Look** (`the_nether.json`): no skybox, fixed time, `ambient_light` 0.1, and `CardinalLighting.NETHER` (up and down shaded 0.9, against 1.0 and 0.5). The fog colour comes from each biome.

**Rules**
- Beds: `BedBlock.destroyOnUse` (when `bed_rule.destroy_on_use`) removes both halves and explodes with power 5 and fire, as `badRespawnPointExplosion`.
- `RespawnAnchorBlock`:
  - glowstone charges it, up to 4;
  - used with a charge where `respawn_anchor_works`, it sets the spawn;
  - used anywhere else, it explodes with power 5 and fire, and keeps the blocks next to flowing water (`isWaterThatWouldFlow`);
  - respawning at it spends a charge.
- Compass: `CompassAngleState` spins when its target's dimension differs.
- Clock: `items/clock.json` uses `context_dimension` (the overworld reads the daytime, anywhere else is random).

**Mobs**
- Fire immune (`EntityTypes`): blaze, ghast, magma_cube, strider, wither_skeleton, zoglin and zombified_piglin.
- Fortress monsters: `NaturalSpawner.mobsAt` calls `isInNetherFortressBounds`, which uses `FORTRESS_ENEMIES` on nether bricks.
- Spawn rules: `check{Ghast,ZombifiedPiglin,MagmaCube,Strider}SpawnRules`.
- AI: `PiglinAi` (bartering through `gameplay/piglin_bartering`), `HoglinAi` and `PiglinBruteAi`.
- Models: `GhastModel`, `BlazeModel`, `MagmaCubeModel`, `PiglinModel` (also used for the zombified piglin), `HoglinModel` (also the zoglin), `StriderModel`, and `SkeletonModel` at scale 1.2 for the wither skeleton.

## 3. Commits (in order)

New logic goes into `minecraft_survival` as pure modules, kept per `Player` or `Level`. `minecraft_terrain` and the vendored crates are edition 2021: no let-chains.

**A. A Nether level that runs** (needs E1's `load_level` and debug start)
1. **"Minecraft levels know their dimension's rules"**: `dimension.rs` reads `DimensionRules` from the registries: coordinate scale, logical height, bed rule, anchors, water, lava, portal piglins and cardinal light. It lives in `Level.rules`.
2. **"Minecraft Nether lava flows faster and its sea level is 32"**: `ServerSim::new` (`server.rs:196-210`).
3. **"Minecraft Nether terrain and structures generate in a test"**: a headless test around a fortress and a bastion; fix what it finds.
4. **"The Minecraft Nether has no sky, sun, stars or clouds"**: `sky_fragment` branches on the skybox, and clouds hide at alpha 0.
5. **"Minecraft Nether blocks and mobs are lit like vanilla's Nether"**: a cardinal flag in `BlockStates` (`terrain.rs:1026`) and `set_nether_entity_lighting`.

**B. Travel**

6. **"Minecraft players wait in nether portals like vanilla"**: `portal.rs` `PortalProcess`, with the cooldown on `Player`. It ticks at `minecraft_world.rs:766`, and the cooldown freezes while `SpawnGate` holds.
7. **"Minecraft portal travel scales coordinates by eight"**: the scale, the default border clamp (±29,999,984), the exit spot, the yaw and momentum.
8. **"Minecraft finds the nearest nether portal like vanilla"**: `find_closest` and `largest_rectangle_around` over a `PortalWorld` trait.
   - `TerrainStream::portal_view` reads loaded chunks and saved chunks (`ChunkStorage::load`) and never generates.
   - Worldgen never makes lit portals, so this finds what POIs would.
9. **"Minecraft builds a nether portal when none is near"**: `create_portal`, using `load_now` and `ChunkMap::set_blocks` before the scene streams. The client, the server and the save all see the result.
10. **"Minecraft players travel through nether portals"**: `load_level` with a `prepare` that finds or builds the portal, then `switch_level`. Plays the travel sound, honours the game rule, and drops the stand-in spawn.
11. **"Minecraft nether portals tint the screen and sound when you step in"**: `hud/src/minecraft_portal.rs`, the trigger sound, screens closing, and the loading-screen background.

**C. Rules**

12. **"Minecraft explosions can start fires"**: a fire pass in vendored `explosion.rs`, a public entity-world blast, and `Command::Blast`.
13. **"Minecraft beds explode in the Nether and the End"**: with the `badRespawnPoint` death cause ("Intentional Game Design"). Beds go into the `uses` table (`server.rs:1280-1286`).
14. **"Minecraft respawn anchors charge with glowstone and set your spawn"**: charging, setting the spawn through D, exploding elsewhere, and the sounds.
15. **"Minecraft players respawn at their anchor or in the Overworld"**: the D resolver with an anchor `spawn_block` (`findStandUpPosition`, then `respawn_anchor.deplete`). Otherwise it shows `block.minecraft.spawn.not_valid` and uses the Overworld spawn through E1.
16. **"Minecraft compasses and clocks move, and spin outside the Overworld"**: the `compass` and `time` properties and the `context_dimension` select, with icons keyed by `id#frame`.
17. (optional) **"Minecraft compasses lock onto lodestones"**.

**D. Mobs.** Wire each as in NETHER-END.md "Wiring in a new mob kind", using `minecraft_survival/src/entities.rs` and `commands.rs:61`.

18. **"Minecraft mobs can be immune to fire"**, and the missing kinds go into `of_type`.
19. **"Minecraft fortresses spawn their own monsters"**: `isInNetherFortressBounds` and structure `spawn_overrides`.
20. **"Minecraft zombified piglins"**: a zombie variant with group anger and a golden sword, then the portal piglins.
21. **"Minecraft magma cubes"**: a slime variant with the 8-segment model.
22. **"Minecraft wither skeletons and the wither effect"**: melee, `EffectKind::Wither`, scale 1.2 and the skull drop.
23. **"Minecraft ghasts and their fireballs"**: flight without pathfinding, and `LargeFireball` (power 1, fire, deflectable).
24. **"Minecraft blazes and small fireballs"**.
25. **"Minecraft monster spawners spawn their mob"**: `BaseSpawner`.
26. **"Minecraft striders walk on lava"**: no riding, since there is no mount system.
27. **"Minecraft hoglins"**, which turn into zoglins in the Overworld.
28. **"Minecraft piglins barter for gold"**.
29. **"Minecraft piglin brutes and zoglins"**.
30. **"Docs: Minecraft Nether state"**: `MINECRAFT-SURVIVAL.md`, and the strider correction in `NETHER-END.md`.

## 4. Unit tests

- Rules for all three dimensions (gated on `MINECRAFTOSS_ROOT`). Nether `ServerSim`: fast lava, sea level 32.
- Nether generation: no panic and no `Feature::Unsupported`.
- `PortalProcess`:
  - 80 ticks in survival, 0 in creative, and the decay of 4;
  - the 10-tick cooldown, refreshed while inside;
  - the game rule and the cooldown both block travel.
- `destination`: (800, 70, -1600) goes to (100, 70, -200) and back. Also the border clamp, the yaw turn of 0 or 90, and the relative exit spot.
- `find_closest`: radius 16 or 128, the closest wins, ties go to the lowest Y, broken portals are ignored. `portal_view` generates nothing.
- `create_portal`: placed on a floor, the fallback platform, and the Nether top at 127.
- The overlay ramp and alpha, and the trigger sound only from 0.
- Explosion fire. The bed and anchor decision tables, with charges capped at 4. Respawning with or without an anchor charge.
- Compass and clock frames per dimension.
- Each mob: a save round-trip, fire immunity, its spawn rule, and its renderer's vertex count.

## 5. Owner test

Use `IW4L_MINECRAFT_WORLD=nether`. `/tp ~ ~ ~` prints your coordinates in the chat.

Kit for travel and rules (24 stacks; beds don't stack):

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*64,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword,minecraft:shield,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:water_bucket,minecraft:lava_bucket,minecraft:ice*4,minecraft:wet_sponge*2,minecraft:red_bed*2,minecraft:respawn_anchor*2,minecraft:glowstone*32,minecraft:compass,minecraft:clock,minecraft:lodestone,minecraft:torch*64,minecraft:totem_of_undying,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots
```

1. `/tp 800 ~ -1600`. Build and light a portal, then step in. The purple overlay and the trigger sound come (**1**). After about 4 seconds you travel and the travel sound plays.
2. In the Nether there is no sky, sun or clouds, the fog is red, and the lava sea shows (**2**). `/tp ~ ~ ~` shows about (100, y, -200) (**3**).
3. Stay in the arrival portal: you don't go back. Step out and in again: you return to your first Overworld portal (**4**).
4. In the Nether, walk 10 blocks, build a portal and go through. You arrive at the first Overworld portal (**5**). A portal 40 blocks away gets a new Overworld portal instead.
5. `/gamemode creative` and step in: you travel at once. Then `/gamemode survival`.
6. Pour the water bucket: it hisses away. Break ice: no water. The wet sponge dries. Lava spreads fast (**6**).
7. Hold the compass and the clock: both spin (**7**). In the Overworld they work.
8. Put on the armour, with the totem in the offhand. Use a bed: it explodes and sets fire (**8**). Charge an anchor 4 times with glowstone (**9**) and use it. `/kill`: you respawn at the anchor, which now has 3 charges (**10**).
9. Use an anchor in the Overworld: it explodes.
10. Quit and restart: you are back in the Nether.

Kit for the mobs (26 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:golden_helmet,minecraft:gold_ingot*64,minecraft:diamond_chestplate,minecraft:totem_of_undying,minecraft:zombified_piglin_spawn_egg*4,minecraft:magma_cube_spawn_egg*4,minecraft:ghast_spawn_egg*2,minecraft:blaze_spawn_egg*4,minecraft:wither_skeleton_spawn_egg*4,minecraft:piglin_spawn_egg*4,minecraft:piglin_brute_spawn_egg*2,minecraft:hoglin_spawn_egg*4,minecraft:zoglin_spawn_egg*2,minecraft:strider_spawn_egg*4,minecraft:warped_fungus*16,minecraft:crimson_fungus*16,minecraft:saddle,minecraft:water_bucket,minecraft:obsidian*14,minecraft:flint_and_steel
```

11. Use each egg and take one screenshot per mob (**11-20**):
    - hitting one zombified piglin angers the group;
    - magma cubes split;
    - a hit sends a ghast's fireball back;
    - blazes fire bursts;
    - a wither hit gives black hearts;
    - striders walk on lava;
    - with the golden helmet, piglins barter for dropped gold;
    - hoglins attack and back away from warped fungus.
12. Fortress: the spawner makes blazes, and wither skeletons appear (**21**). A bastion with piglins (**22**), a ruined portal and a fossil (**23**).

## 6. Risks

- **E1 and D come first.** Group A only needs E1's debug start; travel needs `prepare`.
- **One voxel world per process** (`sim/src/voxel.rs:41`). Friends in another dimension have no MW2 collision. The server output has no player key, and friends' overlays need a message.
- **Search cost.** A 128-block search can read 19x19 saved chunks. Vanilla `poi/*.mca` files would be cheaper.
- **Threads.** Each `TerrainStream` starts its own workers (`terrain.rs:1028-1037`), so two loaded levels need a shared budget.
- **Untried generation.** Bastion piglins and hoglins must keep `PersistenceRequired`, or the dormant despawn deletes them (`server.rs:532-544`).
- **Compass and clock frames** touch the icon cache and held meshes.
- **AI size.** `PiglinAi` is the largest port; brutes and zoglins can slip.
- **Owner decision:** do MW2 bullets deflect ghast fireballs? The proposed default is melee only.
- **Not covered:** strider riding, Nether ambient particles and sounds, lava fog, and the mob shown inside a spawner. Check that `iron_chain` and the dried ghast render.
