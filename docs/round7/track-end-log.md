# Track END log: the End

Branch `end`, worktree `C:\Dev\mc-wt\end`. Briefs: `e3-end.md` (commits E3-1 to E3-29), E2-4 of `e2-nether.md` (the Nether sky, moved here) and `e0-phase3-plan.md` (groups X1a, X1b, X2, ...). Base: 3aef2ebe (the dimension switch merged into `minecraft-survival`).

A commit can't name its own hash, so each entry names the previous commit's hash; the last entry's hash is in `git log` under its subject.

## Group X1a: End pieces that need no travel

### 1. Minecraft end city shulkers are no longer deleted far from players (E3-1)

Previous commit: 3aef2ebe

- **Defect:** mobs the entity world doesn't simulate yet wait dormant on the server thread (`minecraft_terrain/src/server.rs`, `ServerSim.dormant`). Every tick, a dormant mob of the monster category was deleted when it was more than 128 blocks from every player, or in peaceful. Shulkers are of the monster category, so the shulkers an end city places, and the ones saved in its chunks, were deleted as soon as the player was 128 blocks away, and the chunk was saved without them.
- **Changed:**
  - New `minecraft_terrain/src/dormant.rs`: `despawns(kind, persistent, tag, peaceful, nearest_squared)` is vanilla's `Mob.checkDespawn` for a dormant mob, with the two rules looked up by type: `allowed_in_peaceful` (`EntityType.isAllowedInPeaceful`) and `removed_when_far_away` (`Mob.removeWhenFarAway`).
  - `minecraft_terrain/src/server.rs` (`despawn_mobs`, the dormant part only): the dormant list is filtered through `dormant::despawns`. Mobs of other categories stay, as before.
- **Checked with javap (26.3):**
  - `Shulker extends AbstractGolem`, and `AbstractGolem.removeWhenFarAway` returns false. `Warden.removeWhenFarAway` returns false too. `EnderDragon.checkDespawn` does nothing, and `WitherBoss.checkDespawn` only removes it in peaceful.
  - `ZombieVillager.removeWhenFarAway` is false while it converts or once it has trade XP.
  - The monster-category types built without `notInPeaceful()` in `EntityTypes`: camel husk, ender dragon, piglin, shulker, sulfur cube, zombie horse and zombie nautilus. So shulkers also stay in peaceful.
  - `Mob.checkDespawn` removes a mob in peaceful before it looks at persistence, so a name-tagged dormant zombie still goes in peaceful (it stayed before).
- **Tests:** `dormant`: `shulkers_stay_far_from_players_and_in_peaceful`, `plain_monsters_go_beyond_128_blocks_and_in_peaceful`, `peaceful_keeps_the_vanilla_exceptions_but_far_away_still_removes_them`, `a_curing_or_traded_zombie_villager_stays`, `other_categories_are_left_alone`. `server`: `dormant_shulkers_stay_far_from_players_while_dormant_monsters_go` (a summoned shulker and pillager wait dormant; with the player 3000 blocks away the pillager goes and the shulker stays, also in peaceful).
- **Decisions:**
  - The idle despawn (one in 800 after 600 idle ticks) still never applies to dormant mobs, since they don't tick.
  - A raider in a raid (`Raider.removeWhenFarAway`) is not handled, as there are no raids yet.
- **Test steps:** shulkers aren't drawn until the outer End group (X6), so this can't be seen in game yet; the unit tests cover it. Optional, with vanilla 26.3 Java:
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x1atest`. Start, and type `/summon minecraft:shulker ~ ~ ~2`.
  2. Type `/tp ~400 ~ ~`, wait 10 seconds, then `/tp ~-400 ~ ~`, and quit through the menu.
  3. Open `x1atest` in vanilla 26.3: the shulker sits where you summoned it.
- **Kit:** none needed.

### 2. Minecraft end portal frames take eyes of ender and open the portal (E3-2)

Previous commit: a4dbd42b

- **Changed:**
  - New vendored `third_party/minecraftoss/generator/src/feature/end_portal.rs` (MinecraftOSS patch): `EndPortalRing::find(cell, pos)` is `EndPortalFrameBlock.getOrCreatePortalShape().find(level, pos)`. It runs vanilla's `BlockPattern.find` as it is: the pattern `?vvv?`, `>???<` three times, `?^^^?` (`v` an eye-filled frame facing north, `^` south, `>` west, `<` east, `?` any loaded block), tried with its front top left at every block from `pos` to `pos + (4, 4, 4)` and in the 24 orientations, in vanilla's order. `interior()` is the 3x3 at `frontTopLeft.offset(-3, 0, -3)` in vanilla's fill order, and `centre()` the block vanilla's global event uses. `RingCell` (unloaded, other block, or a frame with its facing and eye) is what the caller reads.
  - New vendored `third_party/minecraftoss/world/src/level/end_portal.rs` (MinecraftOSS patch): `Level::place_ender_eye(pos)` is the server half of `EnderEyeItem.useOn`: on a frame without an eye it sets `eye=true` (flags 2), updates comparators (`updateNeighbourForOutputSignal`), plays level event 1503 (`block.end_portal_frame.fill` at the frame, volume 1, pitch 1), then looks for the ring; on a match each of the 9 inner blocks is destroyed with drops (`destroyBlock(pos, true)`) and becomes `end_portal` (flags 2), and event 1038 plays (`block.end_portal.spawn`, volume 1, pitch 1). It answers `Filled` or `Opened(ring)`, or nothing when the block takes no eye. `Level::ring_cell(pos)`.
  - `minecraft_terrain/src/server.rs`: `ItemOnBlock::EnderEye` runs `place_ender_eye` on the server thread (two lines).
  - New `minecraft_terrain/src/ender_eye.rs`: `eye_use(player, world, held)` is the client half: an ender eye aimed at a frame without an eye sends `ItemOnBlock::EnderEye` and is spent when the server says yes (not in creative), through the existing item-use answer.
  - `render_anim/src/minecraft_world.rs`: one call line, `ender_eye::eye_use` after bone meal in the item-on-block chain.
- **Tests:** generator `end_portal`: `a_full_ring_is_found_from_each_of_its_12_frames`, `the_portal_fills_the_3x3_inside_the_ring`, `eleven_eyes_find_nothing` (each of the 12 left empty, tried from every frame), `one_frame_facing_out_or_sideways_finds_nothing`, `a_ring_facing_outward_finds_nothing`, `an_unloaded_corner_finds_nothing`. `ender_eye`: `an_eye_on_an_empty_frame_asks_the_server_and_is_spent`, `a_filled_frame_or_another_block_passes_the_eye_on`. `server`: `twelve_eyes_in_an_inward_ring_of_frames_open_the_end_portal` (stone takes no eye, a filled frame takes no second one, 11 eyes give only the fill sounds, the 12th opens the 3x3 with the spawn sound at the centre, the frames keep their facing, the cobblestone inside drops as an item), `a_frame_facing_out_keeps_the_portal_shut`.
- **Checked with javap (26.3):** `EnderEyeItem.useOn` (PASS on a filled frame, `setBlock(.., 2)`, `shrink(1)`, `levelEvent(1503)`, the 3x3 loop with `destroyBlock(pos, true, null)` and `setBlock(end_portal, 2)`, `globalLevelEvent(1038, frontTopLeft.offset(-3,0,-3).offset(1,0,1))`), and `LevelEventHandler` (1503: `END_PORTAL_FRAME_FILL`, blocks, 1, 1, plus 16 smoke puffs; 1038: `END_PORTAL_SPAWN`, hostile, 1, 1, played 2 blocks from the camera toward the portal).
- **Decisions:**
  - The frames must face inward, as in vanilla: a frame placed while you face north faces south (toward you), so place each one standing inside the ring looking out.
  - Event 1038 is a global event in vanilla: every player in the level hears it at full volume. Here it is a sound at the portal centre, heard within 16 blocks; the one who places the last eye is always within reach of it. With several players it should go to everyone in the level (multiplayer hand-over).
  - Not done: the 16 smoke puffs of event 1503, and `Block.pushEntitiesUp` when the eye makes the frame taller (13 to 16 pixels high).
  - Until X1b's commit 3 the end portal blocks have no look of their own, and until X2 they don't take you anywhere.
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=x1atest` and the kit below. Start on flat ground.
  2. Clear a 5x5 square. Stand in its middle and place 3 frames on each side of the inner 3x3, turning to face each side before you place it, so all 12 face the middle. Leave the four corners empty.
  3. Put an eye in each frame. Each eye plays the fill sound and one eye goes from the stack. **Screenshot 1** after 11 eyes: the eleven eyes show, nothing opens.
  4. Put in the twelfth: the portal sound plays and the 3x3 inside changes (the end portal; it gets its starfield look in X1b). **Screenshot 2.**
  5. Click a filled frame with an eye: nothing happens and the eye stays in your hand.
  6. Optional: build a second ring with one frame turned outward. Twelve eyes go in, but it doesn't open.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*24,minecraft:ender_eye*32,minecraft:diamond_pickaxe+efficiency=5,minecraft:cobblestone*64
```

### 3. Minecraft live levels keep feature entities and gateway exits (E3-4)

Previous commit: 585bad58

- **Defect:** a feature placed in a running level (bone meal, a sapling, and from X2 on the End podium, gateways and islands) wrote through the live `World` of the vendored `Level`, which dropped every entity (`add_entity`), gave every entity the UUID 0 (`next_uuid`), and dropped spawner mobs, loot tables and gateway exits. Generation (the `Region`) had them all.
- **Changed:** vendored `third_party/minecraftoss/world/src/level/mod.rs` (MinecraftOSS patch), the live `World for Level` only:
  - `add_entity` is `ServerLevel.addFreshEntityWithPassengers`: the tag joins `Level.spawned`, which the server already hands to the entity world each tick (items and orbs as level entities, simulated mobs, the rest dormant), as natural spawning does.
  - `next_uuid` is `Mth.createInsecureUUID` from the level's entity random, the same draw natural spawning uses.
  - `set_spawner_entity`, `set_gateway_exit` and `set_loot_table` edit the block entity the feature just placed, as the `Region` does, and mark it changed so the client and the save get it.
  - New `Level::place_configured_feature(name, pos)`: `ConfiguredFeature.place` with the level random, for the End features X2 places at runtime (vanilla's `EnderDragonFight` places `end_gateway_delayed`, and the gateways `end_gateway_return` and `end_island`).
- **Tests:** `server`: `features_placed_in_a_live_level_keep_their_entities_spawners_and_gateway_exits` (`end_gateway_return` placed in a running level has `exit_portal` 100, 50, 0 and `ExactTeleport`, and its block entity is reported changed; a spawner takes its mob, and a gateway exit on a spawner is ignored; two UUIDs differ; an added shulker joins the server; nothing is reported unsupported).
- **Decisions:**
  - The brief names `add_entity` and `set_gateway_exit`; `set_spawner_entity` (orchestrator) and `set_loot_table` (a dungeon grown at runtime would lose its chest loot) come along as they are the same few lines. `set_brushable_loot`, `add_bees` and `load_block_entity` stay unsupported on the live level: no runtime feature uses them yet.
  - The server tests that build a level need the vanilla data: run them with `MINECRAFTOSS_ROOT=C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3`, as `scripts\full-suite.ps1` does. Without it they return early and pass. Entries 1 to 3 were run with it.
- **Test steps:** nothing to see in game yet: no feature placed at runtime has entities or gateways before X2 (the End podium and gateways). Check that nothing broke:
  1. Keep `IW4L_MINECRAFT_WORLD=x1atest`. Use bone meal on an oak sapling: the tree grows as before. **Screenshot 1.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:oak_sapling*4,minecraft:bone_meal*32,minecraft:dirt*8
```

### Group X1a summary

Previous commit: 5eae59cd

- **Commits:** entries 1 to 3 (E3-1, E3-2, E3-4).
- **Tests:** with `MINECRAFTOSS_ROOT` set, `minecraft_terrain` 645 pass (3 ignored), `minecraftoss-generator` 26, `minecraftoss-world` 59 and `render_anim` 67; `check -p launcher` is green.
- **Clippy:** `clippy -p minecraft_terrain -p minecraftoss-generator -p minecraftoss-world --all-targets --no-deps -- -A clippy::approx_constant` has no finding in the new files or the changed lines (the `approx_constant` errors and 38 warnings are older).
- **Left for X2:** the eye of ender throw. Brief E3 leaves it to track B's commit 8, which hasn't landed: `EnderEyeItem.use` (the nearest stronghold through `#eye_of_ender_located`, search radius 100), the `EyeOfEnder` flight (a target over 12 blocks away becomes the point 12 toward it and 8 up), its 80 ticks, the drop when `nextInt(5) > 0`, and the eye kept in the hand when there is no stronghold. Until then right-clicking with an eye anywhere but an empty frame does nothing.
- **Owner test for the group:** entry 2's steps with its kit, then entry 3's step 1 with its kit, in the world `x1atest`.

## Group X1b: skies and portals drawn

### 4. The Minecraft Nether has no sky, sun, stars or clouds (E2-4)

Previous commit: bbc88f0c

- **Defect:** the sky shader ignored the dimension's skybox, so the Nether showed the overworld sky: a sun overhead (the Nether has no timeline, so the sun angle stays at its default), stars and clouds. Clouds always drew.
- **Changed:**
  - `render_gpu/src/drawsurf/minecraft_world.rs` (the sky shader): `sky_fragment` returns the fog colour where the skybox is `none` (mode 2 in `environment.ambient.w`), as vanilla draws no sky pass then and the cleared colour is the fog colour. Clouds draw only while the cloud colour has alpha (`clouds_shown`), as `LevelRenderer` adds the cloud pass only when `ARGB.alpha(cloudColor) > 0`.
  - `render_anim/src/minecraft_world.rs`: the free `y` of the environment's `params` row carries `sky.cloud_alpha` (one literal).
  - The fog colour per biome and the fixed time were already right: the environment system samples each biome's `visual/fog_color` with vanilla's blending, and the Nether has no timeline, so nothing follows the clock.
- **Checked with javap (26.3):** `SkyRenderer.render` draws the End sky for `END`, the overworld disc, sunrise, sun, moon and stars otherwise, and `LevelRenderer.addSkyPass` skips the pass for `NONE`. `LevelRenderer` adds clouds only when `cloudStatus != OFF` and the cloud colour's alpha is above 0. `the_nether.json` has no `cloud_color`, whose default is transparent, and `the_end.json` neither.
- **Tests:** `render_gpu`: `only_a_dimension_without_a_skybox_shows_the_fog_colour_as_its_sky`, `clouds_show_only_while_the_cloud_colour_has_alpha`. `minecraft_terrain` `environment`: `the_nether_sky_keeps_no_time_and_takes_each_biome_fog` (the same sky at noon and midnight, `#330808` in the nether wastes and `#685f70` in the basalt deltas, no clouds), `only_the_overworld_has_clouds`.
- **Decisions:** vanilla's `CardinalLighting.NETHER` face shading stays with the Nether track (E2-5), as the brief says.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x1btest` (a new world) and add `IW4L_MINECRAFT_DIMENSION=the_nether`. Start. Look up and around: no sun, moon, stars or clouds; where no block is in view, the sky is the dark red of the fog. **Screenshot 1.**
  2. Optional: type `/locate biome minecraft:basalt_deltas` or `minecraft:soul_sand_valley` and walk there: the fog and the sky turn grey or blue-green as you enter. **Screenshot 2.**
  3. Quit, set `IW4L_MINECRAFT_DIMENSION=overworld` and start again: in the overworld the sun or moon, the stars at night and the clouds are back. **Screenshot 3.** Remove the line afterwards.
- **Kit:** none needed.

### 5. Minecraft end portals and gateways are drawn (E3-3)

Previous commit: 1597f7a8

- **Defect:** end portal and end gateway blocks have no block model (only a particle texture); vanilla draws them with block entity renderers, so they were invisible.
- **Changed:**
  - New `minecraft_terrain/src/end_portal_look.rs`: the geometry of `TheEndPortalRenderer` and `TheEndGatewayRenderer` as a block model. An end portal is its top and bottom face at heights 0.75 and 0.375, always drawn (`TheEndPortalBlockEntity.shouldRenderFace` is true for both Y faces, whatever is next to it). An end gateway is a full cube whose faces hide like any block's against an occluding neighbour (`TheEndGatewayBlockEntity.shouldRenderFace` is `Block.shouldRenderFace`). Their faces carry a marker vertex alpha (0.25 for the portal, 0.125 for the gateway), well apart from the 1.0 of blocks and the 0.5 of entity sheets.
  - `special_models.rs` and `mesh.rs`: one call line each (the model, and the marker alpha in `entity_face_alpha`, which the baked and the plain mesher both use). The faces stay in the opaque section pass, as vanilla's end portal pipelines don't blend.
  - `render_gpu/src/drawsurf/minecraft_world.rs` (the terrain shader): `rendertype_end_portal` ported. The section vertex shader passes `projection_from_position(gl_Position)`; a fragment with the marker alpha draws `end_sky.png` across the screen tinted by `COLORS[0]`, then adds 15 layers (16 for a gateway) of `end_portal.png`, each scaled, turned and drifting with `GameTime` as `end_portal_layer` does, then the fog as `apply_fog`. New bindings 8 to 10: the two textures and a repeating nearest sampler; the colours and `GameTime` ride in the terrain uniform.
  - New `render_anim/src/minecraft_end_look.rs`: loads `environment/end_sky`, `entity/end_portal/end_portal` and the 16 `COLORS` of `shaders/core/rendertype_end_portal.fsh` from the resource packs at runtime (nothing of Mojang's is committed), and `portal_time` (`GameTime` is the game time modulo 24000 over 24000). `minecraft_world.rs` gets call lines only: load, hand to the view, the time each frame, clear on stop. `render/src/extract.rs` converts the two images once per world.
- **Checked with javap (26.3):** `TheEndPortalRenderer` (`Transformation` translation 0, 0.375, 0 and scale 1, 0.375, 1 over the unit cube), `AbstractEndPortalRenderer` (`END_SKY_LOCATION`, `END_PORTAL_LOCATION`, the faces from `facesToShow`), `TheEndPortalBlockEntity.shouldRenderFace` (axis Y), `TheEndGatewayBlockEntity.shouldRenderFace` (`Block.shouldRenderFace`), `RenderPipelines.END_PORTAL` (`PORTAL_LAYERS` 15) and `END_GATEWAY` (16), both with the default colour target and depth state.
- **Tests:** `minecraft_terrain` `end_portal_look`: `an_end_portal_shows_its_top_and_bottom_at_vanilla_heights`, `an_end_gateway_is_a_cube_whose_covered_faces_are_hidden` (both through the plain and the baked mesher), `the_marker_alphas_stay_apart_from_every_other_section_alpha`, `other_blocks_keep_their_own_models`. `render_gpu`: `end_portal_faces_are_told_apart_by_their_vertex_alpha`, `the_end_textures_and_portal_uniforms_match_in_rust_and_wgsl`. `render_anim` `minecraft_end_look`: `the_portal_colours_fill_sixteen_slots_and_pad_with_black`, `the_portal_time_is_the_game_time_shader_uniform`, `the_vanilla_shader_lists_sixteen_portal_colours` (with `MINECRAFTOSS_ROOT`).
- **Decisions:**
  - `GameTime` comes from the overworld day clock (`globals.day.ticks`), as the End loading screen and the clouds already use it; vanilla uses the level's game time, which only differs after `/time set`.
  - Not done: the gateway's beacon beam while it spawns (its first 200 ticks) or cools down (40 ticks after a teleport), from `TheEndGatewayRenderer` with `end_gateway_beam.png`. It belongs with the gateways themselves (X6).
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=x1btest` (no `IW4L_MINECRAFT_DIMENSION` line) and the kit below. On flat ground, build the ring of entry 2 (12 frames facing inward around a cleared 3x3) and fill it with eyes. The 3x3 inside shows the dark starfield, slightly below the frames' tops. **Screenshot 4.**
  2. Walk around it and look at it from different heights: the speckles stay fixed on the screen rather than on the blocks, and drift slowly. Far away it fades into the fog like the ground around it. **Screenshot 5.**
  3. Dig down beside the ring and look up at the portal from under it: its bottom face shows the same starfield. **Screenshot 6.**
  4. Type `/setblock ~3 ~1 ~ minecraft:end_gateway`: a floating cube with the same starfield on all six sides, a little more blue. **Screenshot 7.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*12,minecraft:ender_eye*12,minecraft:diamond_shovel+efficiency=5,minecraft:diamond_pickaxe+efficiency=5,minecraft:cobblestone*64
```

### 6. Minecraft End sky is drawn with its texture (E3-7)

Previous commit: 436de3d8

- **Defect:** the End drew the overworld sky with the End's colours: a sky disc, a sun and moon at fixed angles. Its clouds and stars were already gone (entry 4: the End has no cloud colour and no star brightness).
- **Changed:** `render_gpu/src/drawsurf/minecraft_world.rs` (the sky shader): for the `end` skybox (mode 1), `sky_fragment` draws `SkyRenderer.renderEndSky` per pixel: the view ray picks the face of vanilla's 200-block cube (`buildEndSky`: the bottom quad turned by `rotationX(±π/2)`, `rotationX(π)` and `rotationZ(±π/2)`), the texture tiles 16 times across each face (UVs 0 to 16, repeating), its colour is multiplied by `#282828`, and it blends over the fog colour by its alpha (`END_SKY` pipeline: `position_tex_color` with translucent blending, over the cleared fog colour). No sun, moon, stars, sunrise or clouds, and no fog on the sky itself, as `position_tex_color` has none. The End's terrain fog was already vanilla's: `the_end.json`'s `#181318` mixed toward its black sky colour.
- **Checked with javap (26.3):** `SkyRenderer.render` (`END` → `renderEndSky`, then the end flash only when its intensity is above 1e-5), `SkyRenderer.buildEndSky` (the six turns, the corners at ±100 with UVs 0 and 16, colour -14145496 = `#ff282828`), `renderEndSky` (`END_SKY` pipeline, `textures/environment/end_sky.png`), `RenderPipelines.END_SKY` (`core/position_tex_color`, `BlendFunction.TRANSLUCENT`).
- **Tests:** `render_gpu`: `the_end_sky_tiles_its_texture_sixteen_times_on_each_face_of_vanillas_cube` (points across each of vanilla's six turned quads map to their interpolated UVs), `the_end_sky_is_drawn_for_the_end_skybox_only_and_shaded_like_vanilla`. `minecraft_terrain` `environment`: `the_end_sky_is_its_skybox_over_a_dark_purple_fog_with_no_stars`.
- **Decisions:** the end flash of 26.3 (`renderEndFlash`, brief E3-29) is left for later: the End sky has no flash yet.
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=x1btest` and set `IW4L_MINECRAFT_DIMENSION=the_end`. Start on the obsidian platform. Look around and up: a very dark grey-purple speckled sky on every side, with no sun, moon, stars or clouds. **Screenshot 8.**
  2. Look straight up, then straight down past the platform's edge into the void: the same speckled texture, tiled evenly, with straight seams where the cube's faces meet. **Screenshot 9.**
  3. Look toward the main island to the west: the end stone fades into the dark purple fog in the distance. **Screenshot 10.**
  4. Quit and set `IW4L_MINECRAFT_DIMENSION=overworld` (remove it afterwards): the overworld sky is unchanged.
- **Kit:** none needed.

### Group X1b summary

Written with entry 6, in its commit.

- **Commits:** entries 4 to 6 (E2-4, E3-3, E3-7).
- **Tests:** with `MINECRAFTOSS_ROOT` set, `minecraft_terrain` 652 pass (3 ignored), `render_gpu` 33, `render_anim` 70 and `render` 0; `check -p launcher` is green.
- **Clippy:** `clippy -p minecraft_terrain -p render_gpu -p render_anim -p render --all-targets --no-deps -- -A clippy::approx_constant` had one finding on this group's lines: the `Local` of entry 5 in `render/src/extract.rs` was a very complex type, now named `EndLookImages` (in entry 6's commit). The other warnings are older.
- **Not done in this group:** the gateway beam (X6) and the end flash (E3-29).
- **Owner test for the group:** in the world `x1btest`, entry 4's steps, then entry 5's with its kit, then entry 6's.

## Group X2: into the End

### 7. Minecraft end portals take players to the End (E3-5)

Previous commit: 32e3fe6b

- **Changed:**
  - New `minecraft_survival/src/the_end.rs`:
    - `touched_end_portal(feet, is_portal)` is the player's box (0.6 by 1.8, deflated by 1e-5) against `EndPortalBlock`'s shape `Block.column(16, 6, 12)`, the band 6/16 to 12/16 of the block.
    - `portal_tick(cooldown, touched)` is `Entity.processPortalCooldown` and `setAsInsidePortal` for a portal whose transition time is 0: the cooldown counts down every tick; touching the portal with no cooldown takes the player and sets it to 10 (`Player.getDimensionChangingDelay`), and touching it during the cooldown sets it back to 10.
    - `step` runs those ticks on a per-player clock (`Player.end`, `EndTravel`), for a living player who isn't in a remote killstreak.
    - `travel` is `EndPortalBlock.getPortalDestination`. Outside the End, the player switches to the End with `END_PORTAL` and arrives at `END_SPAWN_POINT` (100, 50, 0) as a player does: (100.5, 49, 0.5), facing west (yaw 90), keeping pitch and motion (`Relative.X_ROT` and `DELTA`). In the End, the player goes to the dimension of their respawn point, or of the world spawn without one, and the switch is marked as a respawn, so `respawn_in` places them at the point (or the level's spawn) once that level is shown, as `findRespawnPositionAndUseSpawnBlock` does. A respawn point in the End itself places them at once.
  - `minecraft_survival/src/player.rs`: one field, `Player.end`.
  - New `render_anim/src/minecraft_end.rs`: the adapter step, which logs each trip. `minecraft_world.rs` gets one call line after the player is placed, before the commands run.
- **Checked with javap (26.3):** `EndPortalBlock` (`SHAPE = Block.column(16, 6, 12)`, `getEntityInsideCollisionShape` is that shape, `entityInside`, `getPortalDestination`: `END_SPAWN_POINT` bottom centre minus 1 in y for a player, `createEndPlatform(below, true)`, `Direction.WEST.toYRot()`, relatives `DELTA` plus `X_ROT`; from the End `findRespawnPositionAndUseSpawnBlock(false, DO_NOTHING)`), `Entity.setAsInsidePortal` (on cooldown it only sets the cooldown again), `Entity.processPortalCooldown`, `Player.getDimensionChangingDelay` (10).
- **Tests:** `the_end`: `the_portal_is_touched_only_between_six_and_twelve_sixteenths_of_its_block`, `the_portal_takes_a_player_at_once_then_waits_ten_ticks_after_they_leave_it`, `the_travel_clock_counts_whole_ticks`, `the_end_arrival_is_on_the_platform_facing_west_keeping_pitch_and_motion`, `an_end_portal_in_the_overworld_or_the_nether_switches_to_the_end_platform`, `the_exit_portal_goes_to_the_respawn_point_s_dimension_or_the_world_spawn_s`.
- **Decisions:**
  - **Hand-over to the Nether track:** its `minecraft_survival/src/portal.rs` (with `PortalKind::End`) isn't on `minecraft-survival` yet, so the End keeps its own portal tick here. When it lands, the End arm goes into `PortalKind` (`touched_end_portal` as its check, `travel` as its destination), and `EndTravel.clock`, `the_end::step` and `portal_tick` go away, because the Nether's `Portal::tick` also counts the cooldown down. Two counters would halve it.
  - As in the Nether's `Portal`, the trip starts in the tick the portal is touched. Vanilla notices the portal during the move and teleports at the start of the next tick, one tick later.
  - The motion is kept on the arrival `Place`, but MW2 movement starts the player still: the switch holds the player frozen until the platform's chunks are in.
  - **Credits (X4):** vanilla shows the credits on the first exit (`seenCredits`). Until X4 the exit portal never shows them and leaves `seen_credits` unset, so X4's first exit still shows them.
  - The exit portal itself only appears with the dragon's death (X4); until then a test places one with `/setblock`.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x2test` (a new world) and the kit below. Start on flat ground.
  2. Build the ring of entry 2 (12 frames facing inward around a cleared 3x3) and fill it with eyes. Look at a pitch you will recognise (a little down) and step into the starfield. The end portal loading screen shows at once, then you stand on the obsidian platform over the void facing west, toward the main island, with the same pitch. **Screenshot 1.** Type `/tp ~ ~ ~`: about 100.5, 49, 0.5.
  3. Stand on the platform and type `/setblock ~2 ~ ~ minecraft:end_portal`, then walk into it. The loading screen shows and you arrive at the overworld spawn (no credits yet). **Screenshot 2.**
  4. Walk back through the ring's portal into the End, and in the End type `/kill`: you respawn at the overworld spawn as before.
  5. Optional: in the overworld type `/spawnpoint` somewhere away from the spawn, go to the End again, and use the exit block of step 3: you arrive at that point.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*12,minecraft:ender_eye*12,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_shovel+efficiency=5,minecraft:cobblestone*64
```
