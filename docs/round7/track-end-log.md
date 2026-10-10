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

### 8. Minecraft End spawn platform is rebuilt on every arrival and drops what it breaks (E3-6)

Previous commit: 40f26f27

- **Changed:**
  - Vendored `third_party/minecraftoss/world/src/level/end_portal.rs` (MinecraftOSS patch): `end_platform_cells(origin)` and `Level::create_end_platform(origin, break_blocks)` are `EndPlatformFeature.createEndPlatform`. They go over the 5x5 around the origin, z then x then y from -1 to 2, and want obsidian at y -1 and air above. A block that isn't already the wanted block (compared as a block, not a state) is destroyed with its drops (`destroyBlock(pos, true)`) and replaced (`setBlockAndUpdate`, flags 3).
  - `minecraft_terrain/src/server.rs` (End region): `ServerSim::create_end_platform(origin)` with `break_blocks` true, as `EndPortalBlock` asks for it.
  - `minecraft_survival/src/the_end.rs`: entering the End sets `EndTravel.platform`. `build_platform(level, player)` waits until the player is in the End level and both chunks of the platform, (6, -1) and (6, 0), are in its scene. The scene only gets chunks the server thread was already sent, so the platform job then reaches a server that has them. It sends `platform_job` (origin 100, 49, 0) once and clears the flag. A player who ends up in another dimension (a level that failed to open) drops the flag.
  - `render_anim/src/minecraft_end.rs`: `arriving` runs `build_platform` and logs `Minecraft End platform rebuilt at (100, 49, 0)`. `minecraft_world.rs` gets one call line in the switch hold, after the player is placed; the adapter step also runs it.
- **Checked with javap (26.3):** `EndPlatformFeature.createEndPlatform` (the loops, `BlockState.is(Block)`, `destroyBlock(pos, true, null)` only when `breakBlocks`, `setBlockAndUpdate`), `EndPortalBlock.getPortalDestination` (`createEndPlatform(level, END_SPAWN_POINT.below(), true)` before the transition, on every trip into the End).
- **Tests:** world `end_portal`: `the_end_platform_is_25_obsidian_under_75_air_in_vanilla_order`. `minecraft_terrain` `server`: `the_end_platform_is_rebuilt_and_drops_what_it_breaks` (an End level of seed 0: cobblestone at 101, 50, 0, a missing obsidian and a dirt block in the floor; afterwards the 25 obsidian and 75 air are whole, the cobblestone and the dirt dropped, nothing dropped for the obsidian already there, and a second call changes nothing). `the_end`: `the_platform_lies_in_two_chunks_below_the_end_spawn_point`, `an_arrival_rebuilds_the_platform_once_its_chunks_are_in` (a real End level: nothing before the chunks stream in, then one job that clears the cobblestone and drops it, never a second one).
- **Decisions:**
  - Vanilla builds the platform before the player arrives, loading its chunks at once. Here it waits for the chunks to stream in and runs on the server thread while the loading screen still holds the player, so the platform is whole before they can move. The scene shows the server's changes from the first frame after the hold.
  - The flag is per player (`Player.end`), so two players arriving each rebuild it, as each trip does in vanilla.
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=x2test` and the kit below. Go into the End through the ring's portal (entry 7).
  2. On the platform, place cobblestone at 101, 50, 0 (one block east of where you stand, at head height: `/setblock 101 50 0 minecraft:cobblestone` works too), and mine one obsidian block of the floor at its edge. **Screenshot 3.**
  3. Type `/kill`, respawn in the overworld, and go back through the ring's portal. You stand on a whole 5x5 obsidian floor with nothing around your head, and a cobblestone item lies on the platform (pick it up). The obsidian you mined is back; it doesn't drop a second time. **Screenshot 4.** The log says `Minecraft End platform rebuilt at (100, 49, 0)`.
  4. Wait 20 seconds in the overworld before going back, so the End level closes (the log says it saved and closed): the platform is rebuilt the same way in the reopened level.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*12,minecraft:ender_eye*12,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_shovel+efficiency=5,minecraft:cobblestone*64
```

### 9. Minecraft End arena chunks stay loaded while a player is near (E3-8)

Previous commit: 5022ed08

- **Changed:**
  - Vendored `third_party/minecraftoss/world/src/chunk_map.rs` (MinecraftOSS patch): the chunk map takes ticket squares beside the player's view, `set_tickets(&[(centre, radius)])`. A chunk is tracked when the view or a ticket covers it: it is generated with its 3x3 decorated (block ticking, as the view's chunks are), kept while a ticket covers it within 5 rings, and sent. `update_tracking` sends what enters the view or a ticket and forgets what leaves both, never a chunk the other still covers. `schedule` builds the wanted grid over the view and the tickets, nearest the player first, so the view comes before the arena. `is_idle` waits for the ticket chunks too. With no ticket, every step is as before.
  - `minecraft_terrain/src/terrain.rs`: `TerrainStream::set_tickets`, one method that hands the tickets to the chunk map. The file is E1's, but the stream owns the chunk map privately and nothing else reaches it.
  - New `minecraft_survival/src/the_end/dragon_fight.rs` (a submodule of `the_end`, so `lib.rs` keeps its order): `DragonFight` with the player part of `EnderDragonFight.tick`. Every 20 ticks (the first tick at once, `ticksSinceLastPlayerScan` starts at 21) it takes the players of the End level who are alive and within 192 blocks of (0, 128, 0) (`EntitySelector.withinDistance`, feet distance squared). While there is one, the level's stream holds the ticket at chunk (0, 0) with radius 8: vanilla's `addTicketWithRadius(DRAGON, (0, 0), 9)` gives level 24 there, so chunks -8 to 8 are block ticking (level 32) and ring 9 is FULL, which the chunk map's 3x3 rule gives too. With nobody near, the ticket goes (`removeTicketWithRadius`).
  - `minecraft_survival/src/level.rs`: one field, `Level.dragon_fight`, made on the End level's first tick. `commands.rs`: the test level gets the field.
  - `render_anim/src/minecraft_end.rs`: `tick_fight` ticks the End level's fight with every player and logs `Minecraft End arena held for N players` and `Minecraft End arena released`. `minecraft_world.rs` gets one call line after the commands run.
- **Checked with javap (26.3):** `EnderDragonFight.tick` (the player scan every 20 ticks, the ticket while the boss event has players, its removal otherwise), `EnderDragonFight.init` (`validPlayer` is `ENTITY_STILL_ALIVE` and `withinDistance(origin.x, 128 + origin.y, origin.z, 192)`), `ARENA_TICKET_LEVEL` 9, `TicketStorage.addTicketWithRadius` (`ChunkLevel.byStatus(FULL) - radius`), `EntitySelector.withinDistance` (`distanceToSqr <= range²`).
- **Tests:** `dragon_fight`: `the_arena_ticket_keeps_chunks_minus_eight_to_eight_ticking`, `players_count_within_192_blocks_of_the_dragon_spawn_while_alive`, `the_ticket_follows_the_players_scanned_every_twenty_ticks`, `the_fight_clock_counts_whole_ticks`, `the_arena_ticket_loads_the_arena_beside_the_view_and_lets_it_go` (a real End stream at view distance 2 on the platform: the island centre is out of view; a player on the platform holds the arena and chunks (-8, -8) to (8, 8) stream in while ring 9 stays out; 200 blocks south the next scan releases it and the arena chunks are forgotten while the view stays). The chunk map, terrain and survival suites pass as before.
- **Decisions:**
  - Here the client and the server share one scene, so the arena's chunks go to the scene too, as the view's do (vanilla keeps ticket chunks on the server). From the platform the main island is built and drawn out to the fog.
  - Only levels that a player is in tick in this game. The fight therefore ticks while any player is in the End, and with nobody near it releases the ticket at the next scan, as vanilla does before the level goes idle.
  - The scan with `isArenaLoaded`, the saved fight and the podium follow in entry 10. The dragon and the crystals come in X3.
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=x2test`. Go into the End through the ring's portal. The log says `Minecraft End arena held for 1 players` within a second of arriving.
  2. Look west from the platform: the main island and its obsidian pillars are drawn all the way into the fog, beyond the usual 8 chunks around you. **Screenshot 5.**
  3. Type `/gamemode creative`, fly 200 blocks south (`/tp 100 60 220`) and hover. Within a second the log says `Minecraft End arena released`. Fly back to the platform (`/tp 100 50 0`): `held for 1 players` again.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*12,minecraft:ender_eye*12,minecraft:diamond_pickaxe+efficiency=5,minecraft:cobblestone*64
```

### 10. Minecraft End dragon fight is saved per End level (E3-10)

Previous commit: 7f4e5a3b

- **Changed:**
  - `minecraft_survival/src/the_end/dragon_fight.rs`:
    - `FightData` is `EnderDragonFight`'s saved data with its 26.3 codec keys: `needs_state_scanning` (default true), `dragon_killed`, `previously_killed`, `respawn_time` (always written), `respawn_stage`, `dragon_uuid`, `exit_portal_location` (left out when unset), `gateways` (an int array, left out when empty) and `respawn_crystals` (kept as read). Reading also takes the gateways as a list of ints.
    - `init(seed)` is `EnderDragonFight.init`: with no saved gateways, the numbers 0 to 19 are shuffled by `Util.shuffle` with a random of the world seed (`RandomSource.createThreadLocalInstance(seed)`).
    - The End level's fight is read from the End's data folder (`dimensions/minecraft/the_end/data/minecraft/ender_dragon_fight.dat`, `read_saved_in`) on its first tick, and written there (`write_saved_in`) whenever it changes.
    - The tick goes on as vanilla's: with players near and `isArenaLoaded` (the quirky vanilla check: chunks -8 to 8 along z = 8, and the 3x3 around the origin), a fight that `needs_state_scanning` runs `scanState` once, as a job on the server thread. Its answer sets the flags, the dragon and the exit portal location, clears `needs_state_scanning`, and the fight is saved.
  - `minecraft_terrain/src/server.rs` (End region): `ServerSim::scan_dragon_fight` is `scanState`. `hasActiveExitPortal`, then (without one) `findExitPortal` or `spawnExitPortal(false)`. The dragons are the dormant `minecraft:ender_dragon` entities, as nothing simulates one yet: one is kept as the fight's dragon with an exit portal and discarded without one. A first fight is never "killed". `DragonFightScan` is the answer. `dormant_uuids` and `discard_dormant` serve it.
  - Vendored `world/src/level/end_portal.rs` (MinecraftOSS patch): `has_active_exit_portal` (an `end_portal` block entity in chunks -8 to 8), `find_exit_portal` (the pattern at every end portal block entity around the origin, then down the column at the podium from the `MOTION_BLOCKING` height), `exit_portal_location` (the `MOTION_BLOCKING_NO_LEAVES` top, down through bedrock above y 63, at least min y + 1) and `spawn_exit_portal` (places `end_podium_active` or `end_podium_inactive` there).
  - Vendored `generator/src/feature/end_portal.rs`: `ExitPortal::find` is `BlockPattern.find` for the fight's `exitPortalPattern` (five aisles of 7x7, `#` bedrock, tried from every block of the 7x7x7 ahead and in the 24 orientations), and `centre` is `getBlock(3, 3, 3)`.
  - Vendored `generator/src/feature/kinds/the_end.rs`: the new `end_podium` arm, `EndPodiumFeature.place`. Over the box from (-4, -1, -4) to (4, 32, 4), blocks within 2.5 of the origin or within 3.5 get bedrock below (inner) or end stone (outer), air above, and a bedrock rim at the origin's level with air inside (end portal when active). Then a four-block bedrock pillar and four wall torches facing out at height 2. Active, it drops what it replaces (`dropPreviousAndSetBlock`) through a new `World::drop_and_destroy` hook (`destroyBlock(pos, true)` on the live level, a noted no-op in world generation). `feature/mod.rs`: the podiums, `end_gateway_delayed` and `end_island` join `GAMEPLAY_FEATURES`, so a live level can place them by name. No placed feature refers to them, so they weren't loaded before.
  - `render_anim/src/minecraft_end.rs`: `tick_fight` passes the save and the seed, and logs the read (`Minecraft End dragon fight read from the save`), the scan (`Minecraft End dragon fight scanned: exit portal at ...`) and any read or write error. `minecraft_world.rs`: the call line gains `globals`.
- **Checked with javap (26.3):** `EnderDragonFight` (`CODEC` field names and defaults, `TYPE` id `ender_dragon_fight`, `createDefault`, `init`, `tick`, `scanState`, `hasActiveExitPortal`, `findExitPortal`, `isArenaLoaded`, `spawnExitPortal`, the `exitPortalPattern` aisles), `ServerLevel` (the fight is read with `computeIfAbsent` and `init(level, seed, BlockPos.ZERO)` when the dimension type has an ender dragon fight; it ticks while the level isn't idle), `EndPodiumFeature.place` and `dropPreviousAndSetBlock`, `Util.shuffle`, `RandomSource.createThreadLocalInstance(long)` (`SingleThreadedRandomSource`, the same generator as `java.util.Random`: the expected gateway orders in the test were made with `jshell`).
- **Tests:** `dragon_fight`: `a_new_fight_needs_scanning_and_shuffles_its_twenty_gateways_by_the_seed` (seeds 0 and 1 against `jshell`), `the_fight_is_saved_with_vanilla_s_keys_and_read_back`, `a_scan_records_the_exit_portal_and_keeps_an_unseen_dragon_s_uuid`, `the_state_scan_waits_for_a_player_and_the_arena_and_runs_once`, `the_arena_is_loaded_with_the_far_row_and_the_three_by_three_around_the_origin`, `the_first_visit_scans_once_builds_the_podium_and_saves_the_fight` (a saved world: the gateways are saved on the first tick, then once the arena is in, one scan finds no portal, builds it at (0, y, 0) and saves; later ticks never scan again, and the fight reads back the same). Generator `end_portal`: `the_exit_portal_is_found_from_its_pillar_top_and_names_its_centre`, `a_podium_missing_one_bedrock_is_no_exit_portal`. `minecraft_terrain` `server`: `a_first_fight_scan_builds_the_inactive_podium_and_later_scans_find_it` (the inactive podium's pillar, rim, floor, end stone and torches; a second scan finds it and changes nothing; a dormant dragon is discarded without an exit portal; with the active podium the fight counts as killed before, and a dragon there is kept).
- **Decisions:**
  - **Stubbed for X3:** `findOrCreateDragon` (no dragon spawns yet: the dragon and its entity come with X3), the crystal count, and the boss event's bar. A fresh End therefore ends its scan as vanilla's does (`dragon_killed` false, no dragon), and X3 spawns the dragon from there. The respawn stages, the gateways' placement and the active podium after a kill come with X4.
  - The scan runs on the server thread and answers a few ticks later, where vanilla runs it within the tick. Nothing of the fight waits on it until X3.
  - The fight is written when it changes, not with the level's save: the End's data then never lags its chunks, and no save path of E1's changes. A temporary world keeps it in memory.
  - `findExitPortal` goes through block entities, as vanilla's does. The vendored level makes an `end_portal` block entity wherever an end portal is placed, and the scan test checks that.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x2fight` (a new world) and the kit below. Build the ring of entry 2, fill it and jump in.
  2. Within a few seconds of arriving the log says `Minecraft End dragon fight scanned: exit portal at Some((0, ..., 0)), dragon killed before false`.
  3. Type `/gamemode creative` and `/tp 0 90 0`, then fly down: in the middle of the main island stands the bedrock exit portal, a bowl of bedrock with a four-block bedrock pillar in its middle and four torches on the pillar. The bowl is empty (no portal yet). **Screenshot 6.**
  4. Quit and start again (you come back in the End). The log says `Minecraft End dragon fight read from the save` and no new scan. The podium is the same, not a second one on top. **Screenshot 7.**
  5. The file `dimensions/minecraft/the_end/data/minecraft/ender_dragon_fight.dat` is in the world folder.
  6. Optional, with vanilla 26.3 Java: open `x2fight`. The podium is there and vanilla goes on with the fight from the saved data.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*12,minecraft:ender_eye*12,minecraft:diamond_pickaxe+efficiency=5,minecraft:cobblestone*64
```

### 11. The Minecraft End fight data passes clippy

Previous commit: ade7b05a

- **Changed:** the clippy findings on this group's lines, all in `minecraft_survival/src/the_end/dragon_fight.rs`. The saved gateways are read without `unwrap_or_default` (disallowed by `clippy.toml`), and four test checks of a bool use `assert!`.
- **Tests:** none new; the fight and End tests pass as before.
- **Test steps:** none; nothing changes in game.
- **Kit:** none.

### Group X2 summary

Written with entry 11, in its commit.

- **Commits:** entries 7 to 11 (E3-5, E3-6, E3-8, E3-10, and the clippy entry).
- **Tests:** with `MINECRAFTOSS_ROOT` set, `minecraft_survival` 274 (and the vanilla world test), `minecraft_terrain` 654 (3 ignored), `minecraftoss-generator` 28, `minecraftoss-world` 60 and `render_anim` 70 pass; `check -p launcher` is green.
- **Clippy:** `clippy -p minecraft_survival -p minecraft_terrain -p render_anim -p minecraftoss-generator -p minecraftoss-world --all-targets --no-deps -- -A clippy::approx_constant` has no finding on this group's lines after entry 11. The others are older (`server.rs` 332, 1483 and 1695, `minecraft_world.rs` 133, 405 and 414, and more in those crates).
- **MinecraftOSS patches in this group:** `world/src/chunk_map.rs` (tickets), `world/src/level/end_portal.rs` (the platform and the exit portal scan), `world/src/level/mod.rs` (`drop_and_destroy` on the live level), `generator/src/feature/end_portal.rs` (the exit portal pattern), `generator/src/feature/kinds/the_end.rs` (`end_podium`), `generator/src/feature/mod.rs` (`World::drop_and_destroy`, the End features in `GAMEPLAY_FEATURES`).
- **Hand-overs:**
  - **Nether track:** when `portal.rs` with `PortalKind::End` reaches `minecraft-survival`, entry 7's portal tick moves into its End arm and `EndTravel.clock`, `the_end::step` and `portal_tick` go (see entry 7).
  - **X3:** the dragon (`findOrCreateDragon` after the scan, the 1200-tick search), the crystal count every 100 ticks and the boss bar hook into `DragonFight::tick`. Its dragon goes into the fight's `data.dragon_uuid`.
  - **X4:** the credits on the first exit (`seen_credits` is never set yet), the active podium and the egg on a kill (`spawn_exit_portal(.., true)` is ready), the gateways from `data.gateways`, and the respawn stages.
- **Not done in this group:**
  - Throwing an eye of ender toward a stronghold. Brief E3 leaves it to track B's commit 8, which hasn't landed on `minecraft-survival`, and it isn't one of X2's commits. Until it lands, `/locate structure minecraft:stronghold` finds one.
  - The end gateway beam (X6) and the end flash (E3-29), as in X1b.
- **Owner test for the group:** entry 7's steps, then entry 8's and entry 9's, in the world `x2test` with entry 7's kit; then entry 10's steps in the new world `x2fight`.

## Review of groups X1 and X2

### 12. Minecraft End gateways count as an exit portal in the dragon fight scan

Previous commit: 8a6d4d1c

- **Defect:** `hasActiveExitPortal` and `findExitPortal` look for `instanceof TheEndPortalBlockEntity`, and `TheEndGatewayBlockEntity extends TheEndPortalBlockEntity`. Entry 10's vendored `Level::end_portals_in` only took block entities with the id `minecraft:end_portal`, so an End whose arena holds a gateway but no lit exit portal (a vanilla save without fight data after a kill, or a gateway set by a command) scanned as never killed and got a second, inactive podium.
- **Changed:** vendored `world/src/level/end_portal.rs` (MinecraftOSS patch): `end_portals_in` takes `minecraft:end_portal` and `minecraft:end_gateway` block entities (new `END_GATEWAY`), for both `has_active_exit_portal` and `find_exit_portal`.
- **Checked with javap (26.3):** `EnderDragonFight.hasActiveExitPortal` and `findExitPortal` (`instanceof TheEndPortalBlockEntity`), `TheEndGatewayBlockEntity extends TheEndPortalBlockEntity`.
- **Tests:** `minecraft_terrain` `server`: `an_end_gateway_in_the_arena_counts_as_an_active_exit_portal` (a gateway placed in the arena of an End level: the scan says killed before and builds no podium). It fails without the fix.
- **Decision:** vanilla's `getChunk` loads every arena chunk the scan walks; here the scan reads the chunks the server already has once `isArenaLoaded` holds (the far row and the 3x3 at the origin). The lit exit portal sits at the origin, so it is always seen; a lone gateway in an arena chunk that hasn't streamed in yet is missed. Left as is.
- **Test steps:** optional, as no gateway exists before X4:
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x2gate` (a new world) and the kit below. Build the ring of entry 2, fill it and jump in. Wait until the log says `Minecraft End dragon fight scanned: exit portal at Some((0, ..., 0)), dragon killed before false`.
  2. Type `/setblock 92 52 0 minecraft:end_gateway` (a gateway floating west of the platform), then quit through the menu.
  3. With the game closed, delete `dimensions/minecraft/the_end/data/minecraft/ender_dragon_fight.dat` from the world folder, and start again (you come back in the End).
  4. Within a few seconds the log says `Minecraft End dragon fight scanned: exit portal at None, dragon killed before true`. Type `/gamemode creative` and `/tp 0 90 0`: there is still only the one podium of step 1. **Screenshot 1.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*12,minecraft:ender_eye*12,minecraft:diamond_pickaxe+efficiency=5,minecraft:cobblestone*64
```

## Final check of the track

### 13. The track END log sums up its final check

Previous commit: 849a5103

- **Clippy:** `clippy -p minecraft_survival -p minecraft_terrain -p render -p render_anim -p render_gpu -p minecraftoss-generator -p minecraftoss-world --locked --all-targets -- -A clippy::approx_constant` exits 0. No warning falls on a line the track added: the ones in touched files (`minecraft_terrain` `mesh.rs` and `server.rs` 332, 1483 and 1695, `render` `extract.rs`, `render_anim` `minecraft_world.rs` 133, 405 and 414) all blame to commits before 3aef2ebe, and the new files have none.
- **Tests** (`test -p <crate> --locked`, `MINECRAFTOSS_ROOT` set by `cargo-slot.ps1`): `minecraft_survival` 274 and the vanilla world test, `minecraft_terrain` 655 (3 ignored), `minecraftoss-generator` 28, `minecraftoss-world` 60, `render_anim` 70, `render_gpu` 33 and `render` 0 pass. No fixes were needed.
- **Test steps:** none new. The owner's test of the whole track is each group's owner test in order: X1a (entries 2 and 3 in `x1atest`), X1b (entries 4 to 6 in `x1btest`), X2 (entries 7 to 9 in `x2test`, entry 10 in `x2fight`), and the review (entry 12 in `x2gate`). Each is a new world, and the kit goes to a player only in a world where they have no inventory yet, so the one kit line below serves every world.
- **Kit for the whole track** (9 stacks; the 32 eyes fill two stacks of 16):

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*24,minecraft:ender_eye*32,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_shovel+efficiency=5,minecraft:cobblestone*64,minecraft:oak_sapling*4,minecraft:bone_meal*32,minecraft:dirt*8
```

## Second run, group eye-throw: eyes of ender find the stronghold

### 14. Minecraft eyes of ender fly toward the nearest stronghold like vanilla (E3 eye throw, B1-8)

Previous commit: d49f75d2

- **Changed:**
  - New `minecraft_terrain/src/eye_of_ender.rs`:
    - `EyeOfEnder` is vanilla's entity. `launch` puts it at the middle of the thrower's body (`getY(0.5)`) and calls `signal_to`: a target more than 12 blocks away horizontally becomes the point 12 blocks toward it and 8 up, `life` goes back to 0, and the eye will drop when `nextInt(5) > 0`. `tick` is the server half of `EyeOfEnder.tick`: the new position is the old one plus the motion, the motion is steered (`steer`, vanilla's `updateDeltaMovement`: the horizontal speed eases toward the distance by 0.0025, the vertical speed by 0.015 toward +1 or -1, both times 0.8 within a block), and the eye ends on its 81st tick.
    - `Eyes` holds the level's eyes on the server thread. `locate` is `findNearestMapStructure(#eye_of_ender_located, blockPosition, 100)`: the tag is read from the data pack once per level, and the search is `/locate`'s own (`locate::nearest_structure`: rings by chunk centre, aiming at the ring chunk's corner at y 0). `launch` adds the eye and plays `entity.ender_eye.launch` at the thrower's feet (volume 1, pitch `lerp(r, 0.33, 0.5)`). `tick` plays `entity.ender_eye.death` where an eye ends (volume 1, pitch 1). The eye then either drops as one ender eye item (`ItemEntity(level, x, y, z, stack)`: launch `(r·0.2−0.1, 0.2, r·0.2−0.1)`, no pickup delay, the thrown stack's components kept) or records level event 2003 at its block. `snapshot` sends the eyes and the breaks.
    - Client side: `EyeTrails` keeps the eyes of the last snapshot. Every client tick each eye leaves one portal particle half a block below and a quarter step back, with ±0.3 jitter and its motion as the particle's motion (vanilla's `spawnParticles`). A break (event 2003) gives 8 ender eye crumbs (`gaussian·0.15, r·0.2, gaussian·0.15`) and a ring of 80 portal particles 5 blocks out (motions −5 and −7 toward the centre). `append_eyes` draws the eye as a camera-facing ender eye sprite at full brightness, the size of a snowball (`ThrownItemRenderer(ctx, 1.0, true)`), hidden for its first 2 ticks within 3.5 blocks of the camera.
  - `minecraft_terrain/src/ender_eye.rs`: `eye_use` now also does `EnderEyeItem.use`. An eye aimed at an empty frame still fills it, and an eye aimed at a filled frame passes (vanilla's PASS). Anywhere else it asks the server to throw the eye (`Change::ThrowEye(EyeThrow { feet, middle, components })`). The middle comes from the body height: 1.8, crouching 1.5, swimming 0.6.
  - `minecraft_terrain/src/bucket.rs`: the `Change::ThrowEye` arm.
  - `minecraft_terrain/src/server.rs`:
    - `ServerSim.worldgen` (the level's generator, kept from `new`) and `ServerSim.eyes`.
    - `throw_eye` searches, launches and answers whether an eye flew. `Command::ThrowEye { owner, throw }` answers through the existing `item_uses`.
    - `tick_eyes` runs after the tridents, and `EntitySnapshot.eyes` carries the eyes.
  - `minecraft_survival/src/entities.rs`:
    - `Entities::throw_eye` sends the throw for this player and waits for the answer, as for an item on a block.
    - The trails tick before the portal particles, the snapshot's eyes and breaks are taken in, and the eyes are drawn after the thrown items.
  - `minecraft_survival/src/offhand.rs`: an eye of ender counts as an item with a use, so the tactical key uses it from the offhand (both throwing and filling a frame).
  - `render_anim/src/minecraft_world.rs`: one call line, the `Change::ThrowEye` arm calling `entities.throw_eye`.
- **Checked with javap (26.3):**
  - `EnderEyeItem.use`: `getPlayerPOVHitResult(.., Fluid.NONE)`; PASS on any `end_portal_frame`; `findNearestMapStructure(EYE_OF_ENDER_LOCATED, player.blockPosition(), 100, false)`; CONSUME without a result; `new EyeOfEnder(level, getX(), getY(0.5), getZ())`, `setItem`, `signalTo(Vec3.atLowerCornerOf(found))`; `ENDER_EYE_LAUNCH` at the player (NEUTRAL, volume 1, pitch `Mth.lerp(nextFloat, 0.33, 0.5)`); `consume(1, player)`.
  - `EyeOfEnder`: `TOO_FAR_DISTANCE` 12, `TOO_FAR_SIGNAL_HEIGHT` 8, `MIN_CAMERA_DISTANCE_SQUARED` 12.25; `signalTo`, `tick`, `updateDeltaMovement`; `spawnParticles` (4 bubbles in water, else one portal particle at `p ± 0.3`, `p.y − 0.5`); `life > 80`, `ENDER_EYE_DEATH` (1, 1), then `ItemEntity` or level event 2003; `getLightLevelDependentMagicValue` 1.
  - `ItemEntity(Level, x, y, z, ItemStack)`: pickup delay 0 and the launch above.
  - `LevelEventHandler` event 2003: 8 ender eye item particles at `x + 0.5, y, z + 0.5`. Then for `a` from 0 in steps of π/20 while below 2π, two portal particles at `cos·5, −0.4, sin·5` with motions `cos·−5` and `cos·−7`: 40 steps, 80 particles.
  - `EntityRenderers`: `EYE_OF_ENDER` is `lambda$static$13`, which is `ThrownItemRenderer(ctx, 1.0F, true)`.
- **Tests:**
  - `eye_of_ender`:
    - `a_far_target_aims_12_out_and_8_up`, which also checks that a target exactly 12 blocks away is kept.
    - `the_first_ticks_match_hand_worked_vanilla_numbers`: motion 0.03, 0.015, then 0.05985, 0.029775.
    - `steering_sinks_toward_a_lower_target_and_brakes_within_a_block`.
    - `the_eye_ends_on_its_81st_tick_and_a_new_signal_restarts_its_life`.
    - `four_eyes_in_five_drop`: 10000 seeds, each matching `nextInt(5) > 0`.
    - `the_launch_pitch_spans_a_third_to_a_half`.
    - `a_shattered_eye_leaves_8_crumbs_and_a_ring_of_80_portal_particles`.
    - `the_trail_starts_half_a_block_below_and_a_quarter_step_back`.
    - `a_thrown_eye_sounds_flies_and_then_drops_or_shatters`: on a live level with 12 seeds. The launch sound plays at the feet. After 80 ticks the eye still flies, toward its target and above the hand. Then the death sound plays where it ends, and it leaves either one named eye item with the vanilla launch or one break. Both cases happen.
    - `eyes_of_ender_look_for_the_stronghold_and_find_none_in_the_nether`: the tag is the stronghold. A Nether level finds none, and its `ServerSim::throw_eye` answers no and launches nothing.
    - Ignored: `an_overworld_eye_finds_a_stronghold_ring_chunk_corner`. It resolves all 128 rings, which takes 2.4 s in a test build.
  - `ender_eye`:
    - `a_filled_frame_passes_the_eye_on`. It was `a_filled_frame_or_another_block_passes_the_eye_on`; its stone and air cases now throw.
    - `anywhere_else_the_eye_is_thrown_from_the_middle_of_the_body`: 0.9 up when standing, 0.75 when crouching, and the stack's name is kept.
  - `offhand`: `only_items_with_a_use_take_the_tactical_key` now checks the eye too.
- **Decisions:**
  - **Server-owned:** the client only asks. The server searches, launches and answers. The eye leaves the stack only on the server's yes, through the item-use answer the frame fill already uses. Each eye keeps its thrower's player key (`owner`), and the answer goes back to the thrower who waits for it. Vanilla's eye has no owner; the key is kept for multiplayer tracking.
  - **Search time:** the search runs on the server thread like `/locate`, with `/locate`'s 20 s cap. A timeout counts as "not found", so the eye stays in the hand. The first throw in a level resolves all 128 stronghold rings (vanilla does that at world load); later throws reuse them.
  - **Nether and End:** the stronghold can't generate there, so nothing is found and the eye stays in the hand without a sound, as in vanilla. A world without structures finds nothing either.
  - **Saving:** eyes in flight are not saved, like the other thrown items here. Vanilla saves the item but neither the target nor the life, so a reloaded eye flies on for 80 ticks without steering. Here an eye in flight at quit (at most 4 s) is lost.
  - **Under water:** vanilla leaves 4 bubbles a tick instead of the portal particle. There is no bubble particle yet, so the portal trail shows under water too.
  - **Trail timing:** the trail starts from the server's latest position and motion each client tick, while vanilla's client moves its own copy by the synced motion. They differ by at most one tick.
  - **Straight above:** vanilla divides by zero when the eye is exactly above or below its target (NaN). Here the horizontal motion is then zero.
  - **Adventure mode:** throwing goes through the same item-on-block chain as the frame fill, which needs a game mode that may build. So in adventure mode the eye isn't thrown, although vanilla allows it. Spectators can't throw in either.
  - **Not done:** the `used_ender_eye` advancement trigger, the `used` statistic and the `projectile_shoot` game event (there are no advancements, statistics or sculk yet).
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=eyetest` (a new world) and the kit below. Start in the overworld.
  2. Type `/locate structure minecraft:stronghold` and note the coordinates.
  3. Look straight ahead and right-click with an eye of ender. The launch sound plays, and the eye leaves your chest. It flies about 12 blocks toward the stronghold while rising about 8, glowing and leaving a purple particle trail. One eye goes from the stack. **Screenshot 1** while it flies, with the `/locate` answer still in the chat.
  4. After about 4 seconds it plays a short sound. It either falls as an item you can pick up, or breaks into eye crumbs and a purple ring.
  5. Walk about 100 blocks to the side and throw again, then do the same from a third place. Each eye flies toward the `/locate` coordinates. **Screenshot 2** from the third place.
  6. Throw 10 eyes in a row: about 8 drop and 2 break.
  7. Place an end portal frame and right-click it with an eye: the eye goes in with the fill sound. Right-click the filled frame: nothing happens and the stack stays the same.
  8. Put eyes in the offhand slot, hold a gun and press the tactical key: an eye flies.
  9. Type `/gamemode creative` and throw: the stack doesn't shrink. Type `/gamemode survival`.
  10. Quit, add the line `IW4L_MINECRAFT_DIMENSION=the_end` to the `.env` and start: you are in the End. Right-click with an eye: nothing happens, there is no sound, and the stack stays. **Screenshot 3.** Quit and remove the line again.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ender_eye*32,minecraft:end_portal_frame*2,minecraft:diamond_pickaxe+efficiency=5
```

## Second run, group elytra: gliding and firework rockets

### 15. Minecraft test kits can set an item's damage

Previous commit: 2f684111

- **Why:** the elytra steps need an elytra one point from worn out. Gliding there takes over 7 minutes, and `/give` takes no components yet.
- **Changed:** `minecraft_terrain/src/crafting.rs`: in `kit_stack`, a `+damage=N` part sets the `minecraft:damage` component (at least 0) instead of an enchantment. `minecraft:damage` is not an enchantment, so no real kit entry changes.
- **Tests:** `crafting`: `kit_damage_wears_the_item_down` (`elytra+damage=430` and `diamond_sword+sharpness=5+damage=12`).
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=kittest` (a new world) and the kit below. Start, open the inventory: the sword's durability bar is just short of full, and the elytra's bar is red and almost empty. **Screenshot 1.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:elytra+damage=430,minecraft:diamond_sword+sharpness=5+damage=12
```

### 16. Minecraft elytra glide on MW2 movement (E3-27)

Previous commit: 9a9a5777

- **Changed:**
  - `playerstate_iw4/src/playerstate.rs`: two new pm_flags bits, the next free ones below `MC_NO_SKY`: `MC_MAY_GLIDE` (0x0400_0000, set by the game while the chest slot holds a usable glider) and `MC_GLIDING` (0x0200_0000, the glide itself). They travel in `pm_flags`, so deltas and prediction carry them.
  - New `movement_iw4/src/glide.rs`, a pmove mode beside creative flight (decision 4):
    - `prepare` runs each pmove after the fluid and climb samples. A fresh jump press (jump down now, up in the last command) in the air starts the glide when `MC_MAY_GLIDE` is set, as `LocalPlayer.aiStep` calls `tryToStartFallFlying`. Being on the ground, in water or lava, on a climbable, flying (creative), mantling or without `MC_MAY_GLIDE` stops it (`canGlide`, `isInLiquid`, the `onClimbable` check of `travelFallFlying`). While gliding, the jump, stance and sprint keys are dropped, so the player stays standing and MW2 guns stay ready.
    - `glide_move` steps `LivingEntity.updateFallFlyingMovement` in blocks per tick: gravity 0.08 scaled by `-1 + 0.75·cos²(pitch)`, the lift when falling, the climb when looking up (`speed·-sin(pitch)·0.04`, times 3.2 upward), the turn toward the look (0.1), then the drag 0.99, 0.98, 0.99 (the float constants widened as Java does). For a frame of `t` ticks the push is applied with the exact fraction of the drag, `v·dᵗ + a·d·(1−dᵗ)/(1−d)`, which equals the vanilla tick at `t = 1`, as `fly.rs` scales by frame time. The move is the MW2 `step_slide_move` without MW2 gravity, so walls clip the speed as vanilla's `move` zeroes the blocked axis. Landing (the ground trace finds ground) ends the glide. In a cobweb the step is vanilla's: one tick of motion scaled by the web, then no momentum.
  - `movement_iw4/src/single.rs`: the `prepare` call, and the glide branch before the mantle and walk code (after the linked-player branch).
  - `sim/src/frame.rs`, `carrier.rs`: `set_may_glide(id, may)`, which also ends a glide when the elytra goes.
  - New `minecraft_survival/src/elytra.rs`:
    - `can_glide` is the item half of `LivingEntity.canGlide`: the chest slot (38) holds an item with the `glider` component (the elytra, unless a `!minecraft:glider` patch removes it, or any item given one) whose next damage won't break it (`nextDamageWillBreak`). So an elytra stops gliding at 1 durability and gliding never destroys it.
    - `Elytra::step` (one per player, `Player.elytra`) counts the gliding ticks (`fallFlyTicks`) on a 20 Hz clock and wears the elytra 1 point on every 20th (`updateFallFlying`: `(ticks+1) % 10 == 0`, and every second one of those). Unbreaking spares the point with chance `level/(level+1)` (the non-armour rule of `unbreaking.json`: the elytra isn't in `#enchantable/armor`). A player with infinite materials (creative) doesn't wear it.
    - `wall_damage` is `handleFallFlyingCollisions`: `(old horizontal speed − new)·10 − 3` when above 0. `Elytra::step` takes the speeds from the server's player state of the last frame and this one.
    - `fall_peak` is `checkFallDistanceAccumulation`: while gliding slower than 0.5 blocks per tick downward, at most 1 block of fall counts, so a gentle landing doesn't hurt and a dive does.
  - `minecraft_survival/src/death.rs`: `DamageCause::FlyIntoWall` with the message id `flyIntoWall` ("experienced kinetic energy"). `survival.rs`: it costs no exhaustion (`fly_into_wall.json`: `exhaustion` 0). It bypasses armour (`Armor::default()`), and it isn't a fall, so the `fall_damage` rule doesn't turn it off.
  - New `render_anim/src/minecraft_elytra.rs`: each frame it sets `MC_MAY_GLIDE` (alive, not a spectator, `can_glide`), applies `fall_peak` to the fall tracking, runs `Elytra::step`, and on a wall hit hurts the player and plays the fall sound by damage and the hurt sound. `minecraft_world.rs`: one call line after the End step.
- **Checked with javap (26.3):** `LivingEntity.travel` (fluid first, then `travelFallFlying`), `travelFallFlying` (climbable: `travelInAir` and stop; old horizontal speed, `updateFallFlyingMovement`, `move`, `handleFallFlyingCollisions` on the server), `updateFallFlyingMovement` (as above), `handleFallFlyingCollisions` (`getFallDamageSound((int)f)`, `flyIntoWall`), `updateFallFlying` (only while fall flying: `checkFallDistanceAccumulation`, `canGlide` or stop, the wear on multiples of 20 of `fallFlyTicks+1` on a random glider slot), `canGlide` (not on ground, no vehicle, no levitation, a slot passing `canGlideUsing`), `canGlideUsing` (`GLIDER`, the `EQUIPPABLE` slot matches, `!nextDamageWillBreak`), `Entity.checkFallDistanceAccumulation` (`y > -0.5` and `fallDistance > 1` give 1), `Player.tryToStartFallFlying` (not fall flying, `canGlide`, not in liquid), `LocalPlayer.aiStep` (jump pressed this tick and not last tick, no fly toggle this tick, not on a climbable). Data: `fly_into_wall.json`, the `bypasses_armor` and `no_knockback` tags, `unbreaking.json`.
- **Tests:**
  - `glide`: `one_tick_matches_hand_worked_vanilla_numbers` (level flight at 1 block per tick gives 0.99178201, −0.01764000; climbing at −30° from 1.5 gives 1.46060889, 0.06235250), `gliding_from_rest_covers_the_vanilla_path_at_any_frame_rate` (40 vanilla ticks cover 7.79 forward and 4.87 down; 50 ms and 8 ms frames land within 0.3 blocks of it at the same speed), `a_fresh_jump_in_the_air_starts_gliding_only_with_a_usable_elytra`, `water_climbables_flight_and_a_worn_out_elytra_stop_the_glide`, `landing_ends_the_glide`, `a_wall_takes_the_horizontal_speed_but_not_the_glide`, `cobwebs_hold_a_glider_without_momentum`.
  - `sim` `frame`: `an_elytra_lets_a_player_glide_until_it_is_taken_off` (also that both bits are free of every other flag).
  - `elytra`: `only_a_worn_glider_short_of_its_last_point_lets_a_player_glide`, `gliding_wears_one_point_every_twenty_ticks_but_not_in_creative`, `unbreaking_spares_its_share_of_the_wear`, `flying_into_a_wall_hurts_by_the_speed_lost_like_vanilla`, `a_slow_glide_counts_at_most_one_block_of_fall`.
  - `death`: `flying_into_a_wall_is_kinetic_energy_even_after_a_long_fall` (also the `.player` form while a zombie has kill credit).
- **Decisions:**
  - **Hull:** the glider keeps the standing MW2 hull. The vanilla fall-flying pose is 0.6 blocks high, so vanilla gliders pass 1-block holes; here they don't.
  - **Creative flight wins:** a double tap in the air first starts a glide (the first press), then the second press turns creative flight on and the glide ends. Vanilla lets a flying player keep the fall-flying flag; here the two never mix.
  - **Lava** stops a glide like water (vanilla's start check `isInLiquid` covers both; decision 4 names water).
  - **Levitation and slow falling** don't exist for players yet, so they are not checked. When they come (E3-26), the levitation check goes next to `can_glide` and slow falling into the glide's gravity.
  - **Wall damage** is measured between frames from the server's player state. The natural speed loss of a frame stays far below the 0.3 blocks per tick that would hurt, so only a wall hit counts, as in vanilla.
  - **Server-owned:** the glide runs in the authority's pmove, and the wear, the wall damage and the fall tracking run on the host's per-player state (`Player.elytra`, keyed by UUID like the rest).
  - **Not done:** the elytra on the MW2 body (no wing model on the MW2 rig yet; the body plays its in-air animation), the `elytra_glide` game event (no sculk yet) and the `aviate_one_cm` statistic.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=elytratest` (a new world) and the kit below. Start in survival.
  2. Open the inventory and put the plain elytra in the chest armour slot.
  3. Type `/tp ~ ~40 ~`. While falling, press jump once: the fall turns into a glide. Look down to dive and speed up, look level to glide far, look up to trade speed for height. **Screenshot 1** mid-glide.
  4. On the ground, jump and keep jump held: nothing happens in the air. Release it and press it again at the top: the glide starts.
  5. Glide in, look slightly down and land at a shallow angle: no damage. Then `/tp ~ ~40 ~`, glide and dive steeply into the ground: fall damage.
  6. Build a cobblestone wall 4 blocks high, `/tp` 30 blocks away and 15 up, glide and dive toward it to pick up speed, then hit it head on: you take damage with the fall sound and stop. **Screenshot 2** of the health bar. If it kills you, the chat says "... experienced kinetic energy".
  7. Glide into water (empty the water bucket below a ledge): the glide stops and you swim. Glide onto a ladder on a wall: the glide stops and you hold on.
  8. While gliding, fire an MW2 gun: it shoots; sprint and crouch do nothing.
  9. After a few seconds of gliding, the elytra in the inventory shows a durability bar. **Screenshot 3.**
  10. Swap in the elytra with `+damage=420` and glide from high up: after about 11 seconds the glide stops and you fall. The elytra stays, with a red bar, and pressing jump in the air does nothing more. **Screenshot 4** of the elytra in the slot.
  11. Wear the diamond chestplate instead: pressing jump in the air does nothing.
  12. `/gamemode creative`, wear the plain elytra and glide for 20 seconds: its bar doesn't move. Double-tap jump while gliding: creative flight takes over.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:elytra,minecraft:elytra+damage=420,minecraft:elytra+unbreaking=3,minecraft:diamond_chestplate,minecraft:water_bucket,minecraft:cobblestone*64,minecraft:ladder*16,minecraft:golden_apple*8
```

### 17. Minecraft firework rockets boost elytra flight (E3-28)

Previous commit: 20afd35e

- **Changed:**
  - `playerstate_iw4/src/playerstate.rs`: a new field `mc_rocket_time`, the milliseconds of rocket boost left. `net/src/transport/netfields.rs`: it is replicated and validated exactly (appended at the end of the table), so the countdown sits in the pmove state and prediction matches, as decision 4 asks.
  - `movement_iw4/src/glide.rs`:
    - `launch_rocket(ps, ticks)` lights a rocket only while gliding (`FireworkRocketItem.use` passes otherwise).
    - `boost` is the attached `FireworkRocketEntity.tick`: each tick the motion becomes `v + look·0.1 + (look·1.5 − v)·0.5`. For a frame of `t` ticks it uses the exact fraction, `v·0.5ᵗ + look·1.7·(1 − 0.5ᵗ)`, which equals the vanilla tick at `t = 1`. `glide_move` spends the countdown by the frame time and boosts for the part of the frame it covers, then glides. Off the glide the countdown still burns down (the attached rocket lives on but pushes only a fall-flying player).
  - `sim/src/frame.rs`, `carrier.rs`: `launch_rocket(id, ticks)`.
  - New `minecraft_terrain/src/firework.rs`: `flight_duration` (the `fireworks` component's `flight_duration`; the item's default component is 1, and a `fireworks` patch without the field is 0), `lifetime` (`10·(1 + flight) + nextInt(6) + nextInt(7)`), `boost_ticks` (the lifetime plus one: the rocket pushes on its ticks 0 to `lifetime` and explodes after), and `rocket_use`, an item use for a gliding player that is spent and plays `entity.firework_rocket.launch` at volume 3.
  - `minecraft_terrain/src/bucket.rs`: `Change::Rocket(ticks)`.
  - `render_anim/src/minecraft_elytra.rs`: `launch_rocket` lights it on the authority's player state, swings the hand and spends the rocket (not in creative) with the launch sound. `minecraft_world.rs`: two call lines, the `rocket_use` link of the item-use chain and the `Change::Rocket` arm.
  - `minecraft_survival/src/offhand.rs`: a firework rocket counts as an item with a use, so rockets in the offhand boost with the tactical key while an MW2 gun stays in the hand.
- **Checked with javap (26.3):** `FireworkRocketItem.use` (only while `isFallFlying`: a `FireworkRocketEntity(level, stack, player)` attached to the player, `consume(1, player)`, SUCCESS; PASS otherwise), `FireworkRocketItem.useOn` (PASS while fall flying), `FireworkRocketEntity.<init>` (`lifetime = 10·(1 + flightDuration) + nextInt(6) + nextInt(7)`, starting from 1 plus the `fireworks` component's duration), `FireworkRocketEntity.tick` (attached and fall flying: the push above with 1.5 and 0.1; the launch sound `FIREWORK_ROCKET_LAUNCH` at life 0, volume 3, pitch 1; `life++`, explode when `life > lifetime`).
- **Tests:**
  - `glide`: `rocket_pushes_converge_and_level_flight_settles_at_33_5_metres_a_second` (one push from rest is 0.85; pushes alone converge on 1.7; level flight with a rocket settles at 1.6726 blocks per tick forward, 33.5 m/s, matching the hand-run vanilla ticks; two half-tick pushes equal one), `a_rocket_burns_down_by_frame_time_and_only_lights_while_gliding` (only a glider lights it; a second, shorter rocket doesn't cut the first; frame time spends it, also after landing; two ticks of boost at 50 ms and 10 ms frames match vanilla's).
  - `sim` `frame`: `an_elytra_lets_a_player_glide_and_a_rocket_lights_only_while_gliding` (was `an_elytra_lets_a_player_glide_until_it_is_taken_off`).
  - `firework`: `a_rocket_flies_one_unless_its_fireworks_say_otherwise`, `the_boost_lasts_vanillas_lifetime_and_one_tick_more` (21 to 32 ticks for a plain rocket), `a_rocket_boosts_only_a_gliding_player_and_is_spent`.
  - `offhand`: `only_items_with_a_use_take_the_tactical_key` now checks the rocket too.
- **Decisions:**
  - **Overlapping rockets:** in vanilla two attached rockets both push every tick. Here one countdown holds the longer of the two, so a second rocket extends the boost but doesn't double it.
  - **Not done:**
    - Rockets with firework stars don't hurt the wearer when they end (vanilla: 5 + 2 per explosion, `fireworks` damage). Plain rockets, the only ones a kit can give, have none.
    - No rocket model at the hand and no spark trail: there is no firework particle yet.
    - Using a rocket on a block while not gliding doesn't launch a free rocket into the sky (`useOn`): that needs a new entity, its renderer and the firework particles, more than this group could take on. Until then nothing happens and the rocket stays in the hand, as for any use that passes.
  - **Adventure mode:** the rocket goes through the same item-use chain as the eye of ender, which needs a game mode that may build, so in adventure mode it does nothing (vanilla allows it).
  - **Server-owned:** the countdown lives in the authority's player state. The client asks through the item use and the authority lights it; prediction replays it from the replicated field.
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=elytratest` (or a new world) and the kit below. Put the elytra in the chest armour slot.
  2. Type `/tp ~ ~40 ~`, press jump to glide, select the rockets in the hotbar and right-click: the launch sound plays, you shoot forward along your look at about 34 m/s for 1 to 1.5 seconds, and one rocket leaves the stack. **Screenshot 1** during the boost.
  3. Look up 45° and fire a rocket: you climb steeply. Keep firing: you can climb as high as you like.
  4. On the ground or falling without gliding, right-click a rocket: nothing happens and the stack stays.
  5. Put the rockets in the offhand slot, select an MW2 gun, glide and press the tactical key: you boost while the gun stays in your hands and fires. **Screenshot 2.**
  6. Fire a rocket and land at once: the push stops with the glide.
  7. `/gamemode creative`: rockets boost without leaving the stack. `/gamemode survival` afterwards.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:elytra,minecraft:firework_rocket*128,minecraft:cobblestone*64,minecraft:golden_apple*8
```

### 18. The Minecraft elytra wear passes clippy

Previous commit: a38f4ca7

- **Changed:** the one clippy finding on this group's lines, in `minecraft_survival/src/elytra.rs`: the every-20th-tick check uses `is_multiple_of` (`manual_is_multiple_of`).
- **Tests:** none new; the elytra tests pass as before.
- **Test steps:** none; nothing changes in game.
- **Kit:** none.

### Group elytra summary

Written with entry 18, in its commit.

- **Commits:** entries 15 to 18 (a kit helper, E3-27, E3-28, and the clippy entry).
- **Tests** (`test -p <crate> --locked`, `MINECRAFTOSS_ROOT` set by `cargo-slot.ps1`): `movement_iw4` 49, `sim` 71, `minecraft_survival` 280 and the vanilla world test, `minecraft_terrain` 670 (4 ignored), `render_anim` 70 pass; `playerstate_iw4` and `net` have no unit tests of their own and build. `check -p launcher` is green.
- **Clippy:** `clippy -p movement_iw4 -p minecraft_survival -p minecraft_terrain -p render_anim -p sim -p playerstate_iw4 -p net --all-targets --no-deps --locked -- -A clippy::approx_constant` has no finding on a line this group changed after entry 18 (checked against `git diff 2f684111`). The other warnings are older.
- **New files** (`rustfmt --check` on the staged content is empty): `movement_iw4/src/glide.rs`, `minecraft_survival/src/elytra.rs`, `minecraft_terrain/src/firework.rs`, `render_anim/src/minecraft_elytra.rs`.
- **Not done in this group, and why:**
  - **The elytra on the MW2 body:** the wings need their texture in the entity atlas, whose list sits in `minecraft_terrain/src/mesh.rs` (the Nether track's file this run), plus the `ElytraModel` wings and their glide pose on the MW2 rig. Left for after the Nether track merges.
  - **A rocket placed on the ground** flying up as an entity: a new entity with its renderer and the firework particles; not small (entry 17).
  - **Rocket stars hurting the wearer**, the spark trail and the rocket at the hand (entry 17).
  - **The 0.6-block glide hitbox**, levitation and slow falling (entry 16).
- **Owner test for the group:** in the new world `elytratest` with the kit below: entry 16's steps, then entry 17's. Entry 15's step is optional (its kit line is in entry 15).
- **Kit for the group** (10 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:elytra,minecraft:elytra+damage=420,minecraft:elytra+unbreaking=3,minecraft:diamond_chestplate,minecraft:water_bucket,minecraft:cobblestone*64,minecraft:ladder*16,minecraft:golden_apple*8,minecraft:firework_rocket*128
```

## Review of the second run (eye-throw and elytra)

### 19. Minecraft elytra hurt only when they hit a wall

Previous commit: 4bd081e8

- **Defect:** `Elytra::step` hurt the glider whenever the horizontal speed fell by more than 0.3 blocks between two samples. Vanilla's `LivingEntity.handleFallFlyingCollisions` first checks `horizontalCollision` (javap 26.3: `getfield horizontalCollision`, `ifeq` to the return), and entry 16 left that out. So a glider took "kinetic energy" damage where vanilla gives none: flying into a cobweb at speed (the web zeroes the motion, 1.5 blocks per tick cost 12 health), an explosion or other push against the flight, and a long frame while pulling up hard after a dive.
- **Changed:**
  - `sim/src/voxel.rs`: `touches_wall(origin, mins, maxs)`: the player's hull, widened by 0.05 blocks on its four sides, with its top lowered and its bottom raised by the same, overlaps a block. That reaches past the 0.125-unit gap that MW2 traces keep from a wall and leaves the floor and the ceiling out.
  - `minecraft_survival/src/elytra.rs`: `Flight.against_wall`; the wall damage counts only while it is set.
  - `render_anim/src/minecraft_elytra.rs`: sets `against_wall` while gliding, from the authority's player hull and origin.
- **Tests:**
  - `sim` `voxel`: `a_body_against_a_wall_touches_it_but_not_the_floor_below_or_open_air`.
  - `elytra`: `speed_lost_away_from_a_wall_does_not_hurt` (a web-like stop without a wall: no damage; the same stop against a wall: 12).
  - `flying_into_a_wall_hurts_by_the_speed_lost_like_vanilla` now flies against a wall.
- **Decision:** vanilla knows the collision of the tick's move; here the host samples the server's player state once a frame, so a wall touched in that sample stands in for it. A glider that hits a wall and lands in the same tick still takes no wall damage (the glide has ended by then), as before this entry.
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=elytratest` (or a new world) and the kit below, elytra in the chest armour slot.
  2. Hang a few cobwebs in the air: stand on a 3-block pillar of cobblestone and place them on its side, then break the pillar. `/tp` 30 blocks away and 15 up, glide, dive to pick up speed and fly into the webs: you stop in the web without damage and without the fall sound. **Screenshot 1** of the full health bar while in the web.
  3. Repeat entry 16's step 6 (dive into a cobblestone wall): you still take damage with the fall sound. **Screenshot 2.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:elytra,minecraft:cobweb*16,minecraft:cobblestone*64,minecraft:golden_apple*8
```

## Final check of the second run

### 20. The track END log sums up its second final check

Previous commit: c36cf09f

- **Clippy:** `clippy -p minecraft_survival -p minecraft_terrain -p movement_iw4 -p net -p playerstate_iw4 -p render_anim -p sim --locked --all-targets -- -A clippy::approx_constant` exits 0. No warning falls on a line the track added since d49f75d2 (every reported location checked against `git diff -U0 d49f75d2..HEAD`). The ones in touched files (`minecraft_terrain` `server.rs` 337, 1502 and 1715, `sim` `frame.rs` 81, 626, 724 and 932, `sim` `voxel.rs` 47, 325 and 381) are on older lines, and the new files have none.
- **Tests** (`test -p <crate> --locked`, `MINECRAFTOSS_ROOT` set by `cargo-slot.ps1`): `minecraft_survival` 281 and the vanilla world test, `minecraft_terrain` 670 (4 ignored), `movement_iw4` 49, `sim` 72, `render_anim` 70 pass; `playerstate_iw4` and `net` have no unit tests of their own and build. No fixes were needed.
- **Test steps:** none new. The owner's test of the second run is, in order: entry 14 in the new world `eyetest`; entry 15 in `kittest` (optional); entries 16, 17 and 19 in `elytratest`. The first run's steps are listed in entry 13. Each is a new world, and the kit goes to a player only in a world where they have no inventory yet, so the one kit line below serves every world of the whole track.
- **Kit for the whole track** (21 stacks; the 32 eyes fill two stacks of 16 and the 128 rockets two of 64):

```
IW4L_MINECRAFT_KIT=minecraft:ender_eye*32,minecraft:end_portal_frame*24,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_shovel+efficiency=5,minecraft:cobblestone*64,minecraft:oak_sapling*4,minecraft:bone_meal*32,minecraft:dirt*8,minecraft:elytra,minecraft:elytra+damage=420,minecraft:elytra+unbreaking=3,minecraft:elytra+damage=430,minecraft:diamond_sword+sharpness=5+damage=12,minecraft:diamond_chestplate,minecraft:water_bucket,minecraft:ladder*16,minecraft:golden_apple*8,minecraft:firework_rocket*128,minecraft:cobweb*16
```

## Third run, group X3: crystals and the flying dragon

E3-9 and E3-11 to E3-14 of the brief, with plan decisions 5, 6 and 9, and the End entry's travel sound. Base: af3cc119 (the Nether merged into `minecraft-survival`).

### 21. Minecraft end portals play the portal travel sound on the way into the End

Previous commit: af3cc119

- **Defect:** walking into an end portal took the player to the End in silence. Vanilla's `EndPortalBlock.getPortalDestination` gives the trip into the End the post-teleport step `PLAY_PORTAL_SOUND` (then `PLACE_PORTAL_TICKET`), which is level event 1032: `block.portal.travel` as local ambience at volume 0.25 and pitch 0.8 to 1.2, the sound a nether portal trip already plays here. The way out of the End uses `DO_NOTHING`, so it stays silent.
- **Changed:** `minecraft_survival/src/the_end.rs`: `enter_the_end` sets `Player.portal.travel_sound`, the flag the Nether's `minecraft_portal::arriving` already plays once the arrival is known.
- **Checked with javap (26.3):** `EndPortalBlock.getPortalDestination` (`TeleportTransition(..., PLAY_PORTAL_SOUND.then(PLACE_PORTAL_TICKET))` into the End, `findRespawnPositionAndUseSpawnBlock(false, DO_NOTHING)` out of it), `TeleportTransition.playPortalSound` (level event 1032).
- **Tests:** `the_end`: `an_end_portal_in_the_overworld_or_the_nether_switches_to_the_end_platform` also checks the sound flag, and `the_exit_portal_goes_to_the_respawn_point_s_dimension_or_the_world_spawn_s` that leaving sets none.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x3travel` (a new world) and the kit below. Build the ring of entry 2 (12 frames facing inward around a cleared 3x3), fill it with eyes and step in.
  2. When the loading screen goes and you stand on the obsidian platform, you hear the portal travel whoosh, as after a nether portal trip.
  3. Type `/setblock ~2 ~ ~ minecraft:end_portal` and walk into it: you arrive at the overworld spawn without the whoosh.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*12,minecraft:ender_eye*12,minecraft:diamond_pickaxe+efficiency=5,minecraft:cobblestone*64
```

### 22. Minecraft end crystals sit on the pillars, burn and explode (E3-9)

Previous commit: 63ff26c8

- **Defect:** the End's pillars generate an end crystal each, but nothing simulated it: the crystals waited dormant on the server thread, invisible and indestructible.
- **Changed:**
  - New `minecraft_terrain/src/end_crystal.rs`: `EndCrystal` (`EndCrystal` of 26.3: position, `time`, `beam_target`, `ShowBottom`, `Invulnerable`, the tag it came from), read from and written to its saved tag (`beam_target` as an int array, `ShowBottom` a byte, the rest kept). `Crystals` holds a level's crystals: `tick` (`time` goes up by one), `hurt` and `blast`, saving per chunk. `destroyed_by` is `EndCrystal.hurtServer`: any damage destroys it, except fire (it is fire immune), a hit by the dragon and an invulnerable crystal; it explodes with power 6 unless the damage was an explosion. `in_blast` is `ServerExplosion.hurtEntities`' reach: feet within twice the power (a blast always does at least 1 damage, so every crystal in reach goes). `crystal_use` is the client half of `EndCrystalItem.useOn` (only on obsidian or bedrock), `on_ray` the crystal a look meets (its 2 by 2 box), `fresh_tag` a new crystal's tag with a random version-4 UUID.
  - `minecraft_terrain/src/server.rs`: `ServerSim.crystals`. A saved or generated crystal joins them instead of waiting dormant (`add_saved_entity`), and is saved with its chunk. `tick_end` (one call line in the server loop, after the eyes): every explosion of the tick (creepers, ghast fireballs, TNT, beds, anchors and crystals) destroys the crystals in its reach without a blast, then each crystal ticks and, in a level whose dimension type has `has_ender_dragon_fight`, sets fire at its block when that is air (`EndCrystal.tick`). `end_hit` (`Command::EndHit`, `ServerHandle::end_hit`) applies a player's hit; a crystal that explodes queues a power 6 blast without fire (`level.explode(crystal, explosion(crystal, attacker), .., 6.0, false, BLOCK)`) with the new `BlastSource::Player`, so the blocks break with `block_explosion_drop_decay` and the mobs and players take vanilla explosion damage (plan decision 5) credited to the player who set it off. `place_end_crystal` is the server half of `EndCrystalItem.useOn`: obsidian or bedrock, air above, nothing in the 1 by 2 by 1 box above (crystals, mobs, players, items), then a crystal at the block's top centre without its base (`setShowBottom(false)`). `ItemOnBlock::EndCrystal` answers the client as the ender eye does.
  - Vendored `third_party/minecraftoss/entities/src/world.rs` (MinecraftOSS patch): `EntityWorld::blast_by_player`, the public blast with a player as its attacker (`Blaster::Player`): mobs take `player_explosion` damage, turn on the player and credit their death to them (experience and player-only loot), as `player_blast` does for MW2 blasts.
  - New `minecraft_terrain/src/posed_cube.rs`: a model cuboid under any pose matrix, with `ModelPart.Cube`'s faces, UVs and mirror as `cow_render::cube_scaled` has them, for models that are not flipped like living entities or nest deeper (the crystal now, the dragon next).
  - New `minecraft_terrain/src/end_crystal_render.rs`: `EndCrystalModel` and `EndCrystalRenderer`: scaled by 2 and lowered half a block, the base (shown with `ShowBottom`), the outer glass bobbing (`getY`) and turning 3 degrees a tick about Y then tilted 60 degrees about the (1, 0, 1) axis, the inner glass (0.875) and the core (0.765625) each turned again inside it, from `entity/end_crystal/end_crystal` (64 by 32). `append_beam` is `EnderDragonRenderer.submitCrystalBeams`: an eight-sided beam two blocks above its start, 0.15 wide and black there, 0.75 wide and white at the far end, its `end_crystal_beam` texture scrolling with time; a long beam is cut where the texture repeats, as the atlas cannot wrap. A crystal with a beam target draws its beam to it.
  - `minecraft_terrain`: `mesh.rs` loads the two textures; `server_mobs.rs`: `minecraft:end_crystal` is in `SIMULATED_KINDS` (so `/summon` lists it; the list stays sorted).
  - `minecraft_survival/src/entities.rs`: the client keeps the snapshot's crystals, draws them, and gives each a bullet box of key kind 19 (kind 17 is the ghast fireball's) in `boxes`, as crystals block building (`blocksBuilding`). An MW2 bullet on one is an attack, an MW2 grenade an explosion, and a left-click swing (`punch`) an attack with the swing's damage, all through `end_hit`.
  - `render_anim/src/minecraft_world.rs`: one call line, `end_crystal::crystal_use` in the item-on-block chain after the ender eye.
- **Checked with javap (26.3):** `EndCrystal` (`time` from `nextInt(100000)`, `tick`, `hurtServer`, `kill`, `onDestroyedBy`, `beam_target` and `ShowBottom`), `EntityTypes` (2 by 2, fire immune, tracking range 16 chunks), `EndCrystalItem.useOn`, `EndCrystalModel.createBodyLayer` and `setupAnim`, `EndCrystalRenderer` (`submit`, `getY`), `EnderDragonRenderer.submitCrystalBeams` and its geometry lambda, `ServerExplosion.hurtEntities`, the tags `#is_explosion` and `#always_hurts_ender_dragons`.
- **Tests:** `end_crystal`: `any_hit_but_fire_or_the_dragon_destroys_a_crystal_and_only_a_non_explosion_blows_it_up`, `a_blast_reaches_crystals_whose_feet_lie_within_twice_its_power`, `crystals_keep_their_beam_target_and_base_through_a_save`, `a_crystal_goes_on_obsidian_or_bedrock_one_block_up_in_its_middle`, `an_end_crystal_used_on_obsidian_or_bedrock_asks_the_server_and_is_spent`, `a_look_at_a_crystal_finds_its_two_block_box`. `end_crystal_render`: `the_crystal_bobs_between_vanilla_heights`, `the_crystal_is_a_base_and_three_turning_cubes_about_a_block_above_its_feet`, `a_beam_runs_from_two_above_its_start_to_two_above_the_far_end`, `a_long_beam_is_cut_where_its_texture_repeats`. `posed_cube`: `a_posed_cube_matches_the_living_entity_cuboid_under_its_flip` (both mirrors), `a_part_pose_turns_z_then_y_then_x_about_its_offset`. `server`: `end_crystals_burn_on_the_spikes_and_explode_unless_a_blast_destroyed_them` (an End level of seed 0: ten live crystals, none dormant; a crystal relights its fire; one placed on obsidian beside it, not twice, not on end stone and not where a crystal stands; a shot crystal queues a power 6 blast credited to the player, which hurts a player 4 blocks off for more than 20 and destroys the placed crystal without a second blast; a grenade's blast destroys one without a blast; the crystals save and load back). Vendored `entities`: `a_blast_a_player_set_off_credits_the_player_with_its_kills`. `entities` (survival): `end_crystals_have_their_own_bullet_key`.
- **Decisions:**
  - **No fire from a crystal's explosion:** the brief and the plan say "power 6 with fire", but 26.3's `EndCrystal.hurtServer` passes `fire = false` to `explode` (javap). Vanilla wins; the only fire is the one each crystal keeps lit on its pillar.
  - **Key kind 19:** the brief's key kind 17 is taken by the ghast fireball (Nether track), so crystals use 19. The dragon will use 20.
  - **One tick late:** a crystal's blast reaches the mobs, players and blocks at the next server tick's mob step (it queues like a bed's blast), and the crystals in its reach go after that tick's blasts. Vanilla does all of it in the hit's tick. A blast-destroyed crystal never explodes, so nothing chains.
  - **The player hurt by a crystal blast** gets an explosion hit with no source, so the death line is "blown up", not "blown up by <player>" (vanilla names the player who set it off; in single player that is yourself).
  - **Placement box:** vanilla refuses a crystal when any entity's box meets the 1 by 2 by 1 box above the block. Here crystals, mobs and players are tested with their boxes, and item, orb and TNT entities as 0.25-block boxes at their feet.
  - **Creative and invulnerable crystals:** an invulnerable crystal (only the respawn sequence makes them, X4) ignores every hit, also a creative player's (vanilla lets creative players break it).
  - **`tryRespawn`:** a crystal placed in the End does not try the dragon respawn yet (X4, E3-22).
- **Test steps** (survival; MW2 weapons as usual):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x3crystal` (a new world), `IW4L_MINECRAFT_DIMENSION=the_end` and the kit below. Start on the obsidian platform, type `/gamemode creative`, then `/tp 0 110 60` and look north toward the pillars.
  2. On top of each pillar a crystal turns and bobs inside its glass cubes over a fire on bedrock; the caged ones sit behind iron bars. **Screenshot 1.**
  3. Fly up beside one pillar top, type `/gamemode survival` while standing on it (or on cobblestone you place), step a few blocks back and shoot the crystal with an MW2 rifle: it explodes, breaks blocks around the pillar top and hurts you when close. **Screenshot 2** just after.
  4. Back on the platform (`/tp 100 50 0`), place obsidian and right-click it with an end crystal: the crystal stands on it without its grey base, and one crystal leaves the stack. Right-click the same block again: nothing (the crystal is in the way). Right-click end stone with one: nothing.
  5. Place a second crystal 4 blocks from the first and shoot the first: both go, but only the first explodes (one bang).
  6. Place a crystal and throw an MW2 grenade at it: it vanishes without exploding.
  7. Place a crystal, step back to sword reach and hit it with the sword (left-click): it explodes.
  8. Quit while a crystal you placed stands, and start again: it is still there.
  9. Remove the `IW4L_MINECRAFT_DIMENSION` line afterwards.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_crystal*16,minecraft:obsidian*16,minecraft:end_stone*4,minecraft:cobblestone*64,minecraft:diamond_sword,minecraft:golden_apple*8,minecraft:diamond_chestplate
```

### 23. Minecraft ender dragon flies its holding pattern over the End (E3-11)

Previous commit: 039c31d8

- **Changed:**
  - New `minecraft_terrain/src/ender_dragon.rs`, the server's `EnderDragon`:
    - `FlightHistory` is `DragonFlightHistory`: 64 samples of height and yaw, newest first, the first one filling all; `sample_at` its `get(i, partial)` lerp (`Mth.lerp`, `Mth.rotLerp`).
    - The 24 path nodes of `findClosestNode`: rings of 12 at 60 blocks, 8 at 40 and 4 at 20 (float angles, `Mth.cos`/`Mth.sin`), each at least y 73 and 5, 15 or 5 above the `MOTION_BLOCKING_NO_LEAVES` height there; vanilla's adjacency bit masks; `closest_node` (only the inner 12 once no crystal is alive or outside a fight) and `find_path`, vanilla's A* with its `BinaryHeap` (up and down heap, `changeCost`) and the fall back to the closest node reached.
    - `Phase`: `HoldingPattern` (`DragonHoldingPatternPhase`: a new target when the old one is nearer than 10 or farther than 150 blocks; a new path from the closest node, one in 8 times turning the circling round and jumping 6 nodes, then one node on; the outer ring with a fight, the inner without; each node's target 0 to 20 blocks above it), `Hovering` (`DragonHoverPhase`, vanilla's default phase, so `/summon` makes a hovering dragon; it counts as sitting), and `Dying` (`DragonDeathPhase`: it flies to the podium at fly speed 3 and its health drops to 0 there).
    - `tick` is `Entity.tick`/`LivingEntity.baseTick`/`Mob.baseTick`/`EnderDragon.aiStep` for these: the hurt and damage cooldown timers, the ambient sound (`entity.ender_dragon.ambient`, volume 5, one in 1000 against a timer from -80), the flap time (0.2 over the horizontal speed, doubled per block of climb), the flight (`aiStep`'s climb toward the target, the turn of at most 50 degrees a tick by `getTurnSpeed`, the push along the yaw by `moveRelative`, the move without collision as `noPhysics`, the drag of 0.8 to 0.95 sideways and 0.91 upward), then the eight parts placed by `tickPart` (body, two wings 4.5 out and 2 up, head and neck 6.5 and 5.5 ahead raised by the history, three tail parts trailing by the history's yaw and height). The head and neck strike non-creative players their boxes (grown by 1) touch for 10 (`mobAttack`), unless the dragon was hurt within 10 ticks. `tick_death` rises 0.1 a tick and removes it after 200 ticks (the rest of the death is X4's).
    - Saved and loaded as vanilla's tag (`DragonPhase`, `DragonDeathTime`, `sitting_damage_received`, `Health`, `Pos`, `Motion`, `Rotation`, the rest kept). Ids start at 2^40, so they never meet the entity world's.
  - New `minecraft_terrain/src/dragon_render.rs`: `EnderDragonModel` and `EnderDragonRenderer` built in code from `createBodyLayer` and `setupAnim`: the root lifted and tilted with the breath of the flap, five neck segments and the head with its jaw following the flight history, the body with its spikes, the wings and wing tips beating (`flapTime`), the four legs posed by `poseLimbs`, twelve tail segments trailing the history. The entity pose is the renderer's: turned by the history's yaw 7 ticks back, tilted by 10 times the climb between 5 and 10 ticks back, one block forward, flipped as a living entity. `entity/enderdragon/dragon` (256 by 256) then `dragon_eyes` at full brightness over it; red while hurt.
  - `minecraft_terrain/src/server.rs` (End region): `ServerSim.dragons`. A saved, generated or summoned dragon is live (no longer dormant) and saved with its chunk. `tick_end` now ticks the dragons with the players of the tick (plan decision 9: the players in the End level, never `PLAYER = 0`), the `MOTION_BLOCKING_NO_LEAVES` heights for the nodes and the podium's `MOTION_BLOCKING` height. Their sounds and hits reach the client with the mob sounds and player hits. Every 100 ticks in a dragon fight level it counts the crystals standing on a spike (`updateCrystalCount` over `EndSpikeFeature.getSpikesForLevel`'s top boxes); the fight's dragon (`fight_dragon`) uses that count. `find_or_create_dragon` is `findOrCreateDragon`: a dragon in the level, or a new one at (0, 128, 0) facing a random yaw in the holding pattern. `scan_dragon_fight` now finds the live dragons. `EntitySnapshot.dragons` carries `DragonView`s (position, flap times, 26 history samples, health, hurt time, parts).
  - `minecraft_terrain/src/end_crystal.rs`: `Crystals::on_spike`.
  - `minecraft_survival/src/the_end/dragon_fight.rs`: `EnderDragonFight.tick`'s dragon part: once the state scan is done and the dragon is not killed, with players near and the arena loaded, a fight without a dragon, or one whose dragon was not seen for 1200 ticks, runs `find_or_create_dragon` as a job and saves the answer as `dragon_uuid`. A dragon in the snapshot counts as seen (`updateDragon`). `link` tells the server which dragon is the fight's after a reload. `FightEvents.spawned`.
  - `minecraft_survival/src/entities.rs`: the client keeps the snapshot's dragons and draws them; the wing flap (`entity.ender_dragon.flap` at volume 5 when `cos(2π·flapTime)` rises through -0.3, `isFlapping`/`onFlap`) and the growl (`entity.ender_dragon.growl` at 2.5, every 200 to 399 ticks after a first 100, not while sitting) are the client's own, as in vanilla. A dragon's hit names it ("was slain by Ender Dragon"). `dragons`, `dragon_alive`.
  - `minecraft_terrain`: `mesh.rs` loads the two dragon textures; `minecraft:ender_dragon` is in `SIMULATED_KINDS` (sorted). `render_anim/src/minecraft_end.rs` logs the fight's dragon.
- **Checked with javap (26.3):** `EnderDragon` (constructor parts and sizes, `aiStep`, `tickPart`, `getHeadYOffset`, `hurt(List)`, `findClosestNode` both forms, `findPath`, `reconstructPath`, `tickDeath`, `checkDespawn`, save keys, sounds and volume), `DragonFlightHistory` and `Sample`, `AbstractDragonPhaseInstance`, `DragonHoldingPatternPhase`, `DragonHoverPhase`, `DragonDeathPhase`, `EnderDragonPhase` (ids), `EnderDragonPhaseManager.setPhase`, `BinaryHeap`, `EnderDragonFight` (`tick`, `findOrCreateDragon`, `createNewDragon`, `updateDragon`, `updateCrystalCount`, `scanState`), `EntityTypes` (16 by 8, tracking range 10 chunks), `EnderDragonModel` (`createBodyLayer`, `setupAnim`, `poseLimbs`), `EnderDragonRenderer` (`submit`, `extractRenderState`), `EnderDragonRenderState` (`getHistoricalPos`, `getHeadPartYOffset`), `LivingEntity.wasHurtRecently`.
- **Tests:** `ender_dragon`: `the_flight_history_keeps_sixty_four_samples_newest_first`, `the_twenty_four_nodes_ring_the_island_at_sixty_forty_and_twenty_blocks`, `a_path_follows_the_node_graph_to_its_goal`, `a_holding_dragon_circles_the_island_between_its_nodes` (3000 ticks: at least 4 targets, above y 60, within 100 blocks of the centre, parts where vanilla puts them), `a_dragon_without_ai_stays_put_and_a_summoned_one_hovers`, `a_dragon_keeps_its_phase_health_and_uuid_through_a_save`, `the_head_and_neck_strike_an_attackable_player_they_touch` (not a creative one, not while hurt). `dragon_render`: `a_level_dragon_lays_its_neck_and_tail_out_in_a_straight_line` (36 parts, the cube count, 10 pixels between segments), `the_wings_beat_with_the_flap_time_and_mirror_each_other`, `the_dragon_faces_its_flight_history_heading`. `server`: `the_fight_dragon_spawns_over_the_island_and_circles_it_while_the_arena_is_loaded` (seed 0: one dragon at (0, 128, 0), reused; 1200 ticks circling above y 60; the 10 spike crystals counted; only ambient sounds; its view and tag), and `a_first_fight_scan_builds_the_inactive_podium_and_later_scans_find_it` now with live dragons. `dragon_fight`: `a_dragon_is_due_once_the_scan_is_done_without_one_or_after_1200_ticks_unseen`, and `the_first_visit_scans_once_builds_the_podium_and_saves_the_fight` now waits for the dragon to spawn, saves its UUID and sees it in the snapshot above y 100, with no second one. `entities`: `a_dragon_flaps_when_its_wings_pass_the_top_of_their_beat`.
- **Decisions:**
  - **Holding pattern only (X5 is out of scope):** vanilla's holding pattern turns to the landing approach (one in `crystals + 3` at a path's end) or to strafing a player. Those phases come with X5, so this dragon keeps circling; it neither lands, strafes nor breathes, and `checkWalls` (block breaking) and the wing knockback are not done either, so it flies through blocks without slowing (`inWall` stays false). The head and neck contact damage is done (it is no X5 commit's).
  - **Vanilla's quirks kept:** `DragonHoldingPatternPhase` picks the inner ring only when the fight's crystal count is below 0, so a fight dragon always picks outer ring goals (javap `ifge`), while `findClosestNode` and `findPath` leave the outer ring out when no crystal is alive. Head and neck both take full damage later (E3-13), as `hurt(part)` tests both.
  - **Where the parts sit:** the server keeps them; the client draws the model from the flight history like vanilla, so the boxes and the model agree as in vanilla.
  - **Death:** a dragon brought to 0 health (E3-13) flies to the podium, rises for 10 seconds and goes; its rays, sounds, experience and the exit portal come with X4 (E3-18, E3-19).
  - **Random draws:** the dragon has its own random seeded from its UUID; the spawn yaw comes from the End level's own random (`ServerSim.end_random`), not the level's.
  - **Seen:** a dragon counts as seen while it is in the snapshot (the server sends every dragon of the level), where vanilla counts its ticks.
  - Dragons saved dormant by the X2 builds load live now.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x3dragon` (a new world), `IW4L_MINECRAFT_DIMENSION=the_end` and the kit below. Start on the obsidian platform in survival.
  2. Within a few seconds the log says `Minecraft ender dragon of the fight: [...]`. Look west: the black dragon with purple glowing eyes circles the main island high up, beating its wings, its neck and tail swinging as it turns. You hear its wing beats and now and then a growl. **Screenshot 1.**
  3. Watch it for a minute: it flies from pillar to pillar around the island, sometimes turning to circle the other way, never landing.
  4. Type `/gamemode creative`, fly up to it and look at it from close by. **Screenshot 2** (head, wings and legs in view).
  5. Type `/gamemode survival` while you fly into its head's path (stand on a pillar it passes): when its head touches you, you take damage. If it kills you, the chat says "... was slain by Ender Dragon".
  6. Quit and start again: the same dragon circles on (the log names the same UUID only once, at the first spawn), not a second one.
  7. Type `/summon minecraft:ender_dragon ~ ~10 ~`: a second dragon appears and hovers where it was summoned, wings slowly beating.
  8. Remove the `IW4L_MINECRAFT_DIMENSION` line afterwards.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:golden_apple*8,minecraft:diamond_chestplate
```

### 24. Minecraft ender dragon has a boss bar (E3-12)

Previous commit: 5a6fd423

- **Changed:**
  - `minecraft_survival/src/the_end/dragon_fight.rs`: the fight's boss event (`EnderDragonFight.dragonEvent`): `DragonFight.progress` is the dragon's health over 200 whenever the fight's dragon is in the snapshot (`updateDragon`'s `setProgress`), 1 until then; `boss_bar(player)` shows it to the players the fight counts (alive and within 192 blocks of (0, 128, 0), rescanned every 20 ticks, `updatePlayers`) while the dragon is not killed (`setVisible(!dragonKilled)`). Per player, by UUID.
  - `frame/src/minecraft_ui.rs` (appended): `MinecraftUi.boss_bars: Vec<McBossBar>`; `McBossBar` is `LerpingBossEvent`: a new progress slides over 100 ms from where the bar stood (`setProgress`, `getProgress`), and `world_fog` (`setCreateWorldFog`).
  - New `hud/src/minecraft_boss.rs` (with its `mod` line in `hud/src/lib.rs` and one call line at the top of `minecraft_hotbar::draw_hud`, before the game mode check, as the boss overlay shows in every mode): `BossHealthOverlay`: from y 12 down, 19 GUI pixels a bar, stopping past a third of the screen; the 182 by 5 `boss_bar/pink_background` sprite, the `pink_progress` sprite cut to `Mth.lerpDiscrete(progress, 0, 182)`, and the name centred 9 pixels above in white with a shadow.
  - `render_anim/src/minecraft_gui.rs`: the two pink sprites join the loose GUI sprites read from the pack.
  - `render_anim/src/minecraft_end.rs`: `tick_fight` sets the local player's bar ("Ender Dragon" from the language file, `entity.minecraft.ender_dragon`) while they are in the End and not switching, and clears it otherwise (the client's `BossHealthOverlay.reset` on a level change). `minecraft_world.rs`: the call line passes the local player and the UI, and while a shown bar wants world fog the sky shader gets the `none` skybox, so the fog colour stands in for the End sky (`GameRenderer` passes `!shouldCreateWorldFog` as `LevelRenderer.render`'s sky pass flag).
- **Checked with javap (26.3):** `EnderDragonFight` (the boss event: `PINK`, `PROGRESS`, `setPlayBossMusic(true)`, `setCreateWorldFog(true)`; `tick`'s `setVisible`, `updatePlayers`, `updateDragon`), `ServerBossEvent`, `BossEvent`, `LerpingBossEvent`, `BossHealthOverlay` (`extractRenderState`, `extractBar`, the sprite tables), `Mth.lerpDiscrete`, `GameRenderer.renderLevel` and `tick` (`shouldCreateWorldFog`, `shouldDarkenScreen`), `LevelRenderer.render`, `LightmapRenderStateExtractor`.
- **Tests:** `dragon_fight`: `the_boss_bar_shows_the_dragon_s_health_to_the_players_near_until_it_is_killed`. `minecraft_ui` (frame): `a_boss_bar_slides_to_its_new_progress_over_a_tenth_of_a_second`. `minecraft_boss` (hud): `the_bar_fills_182_pixels_in_steps_like_lerp_discrete`, `boss_bars_stack_down_from_the_top_until_a_third_of_the_screen`.
- **Decisions:**
  - **"Darkening fog":** the dragon's boss event sets `createWorldFog`, not `darkenScreen` (javap), so the screen is not darkened; the sky pass is dropped and the End's dark fog colour shows instead of its sky. The End flash (not drawn yet, E3-29) is the only other thing it changes.
  - **No boss music:** the game plays no Minecraft music at all yet, so `setPlayBossMusic` has nothing to switch.
  - The bar's progress moves when the dragon is hurt (E3-13) or healed (E3-14); until those commits it stays full.
- **Test steps:**
  1. Keep `IW4L_MINECRAFT_WORLD=x3dragon` (entry 23) or a new world, with `IW4L_MINECRAFT_DIMENSION=the_end`. Start on the platform.
  2. At the top of the screen a pink bar named "Ender Dragon" shows, full. The sky behind the island is now the plain dark fog colour instead of the speckled End sky. **Screenshot 3.**
  3. Type `/gamemode creative` and `/tp 100 60 220` (more than 192 blocks from the island's centre): within a second the bar goes and the speckled sky is back. `/tp 100 50 0`: it comes back.
  4. Type `/gamemode spectator`: the bar stays. `/gamemode survival` afterwards.
  5. Quit, remove the `IW4L_MINECRAFT_DIMENSION` line and start again in the overworld: no bar.
- **Kit:** none needed.

### 25. MW2 bullets, grenades and Minecraft hits hurt the ender dragon through its body parts (E3-13)

Previous commit: 2b3167b1

- **Changed:**
  - `minecraft_terrain/src/ender_dragon.rs`: `hurt_part` is `EnderDragon.hurt(part, source, damage)`: nothing while dying; a part other than the head and the neck takes `d / 4 + min(d, 1)`; below 0.01 nothing; only damage with a player behind it or an explosion counts (`#always_hurts_ender_dragons` holds `#is_explosion`); then `LivingEntity.hurtServer`'s damage cooldown (a full hit sets 20 ticks and `hurtTime` 10, a hit within the first 10 ticks only adds what it exceeds the last one by), the hurt sound (`entity.ender_dragon.hurt`, volume 5, only for a full hit) or the death sound, and `handleKillingBlow` (a flying dragon is held at 1 health and turns to its dying phase; a sitting one dies where it is). A sitting dragon that took more than 50 since it sat takes off. `blast` is `ServerExplosion.hurtEntities` on the dragon and each part: feet within twice the power, damage `(i² + i) / 2 · 7 · 2p + 1` for `i = 1 - distance / 2p`, each through `hurt_part`, so the cooldown keeps the best one.
  - `minecraft_terrain/src/end_crystal.rs`: `EndTarget::DragonPart { id, part }`.
  - `minecraft_terrain/src/server.rs` (End region): `end_hit` hurts a dragon part; every explosion of the tick (`tick_end`) blasts the dragons too.
  - `minecraft_survival/src/entities.rs`: each living dragon's 8 parts are bullet boxes in `shot_boxes` (not in `boxes`: parts don't block building) with key kind 20, the dragon id and the part, and the flat hit bit. An MW2 bullet is an attack by the player (`damage · 0.2`), an MW2 grenade an explosion (`damage · 0.8`, plan decision 6), and a left-click swing (`punch`, also `mob_distance`) on the nearest part an attack with the swing's damage (`EnderDragonPart.isPickable`).
  - `sim/src/voxel.rs`: `FLAT_HIT_KEY`: a box whose key carries it is struck in the upper torso wherever the bullet lands (`hit_height`), so MW2's head and neck multipliers never stack on the dragon's own head rule (plan decision 6).
  - `minecraft_terrain/src/server.rs`: a dragon that finished dying is reported in `Output.dragons_killed`; `minecraft_survival/src/entities.rs` keeps them for `take_dragons_killed`, and `the_end/dragon_fight.rs`'s `set_dragon_killed` marks the fight's dragon killed (`FightEvents.killed`).
  - `render_anim/src/minecraft_end.rs`: logs the kill, and a test that the survival crate's bit is the one the bullets read.
- **Checked with javap (26.3):** `EnderDragon.hurt(part)`, `hurtServer`, `reallyHurt`, `handleKillingBlow`, `knockback`; `EnderDragonPart` (`hurtServer`, `isPickable`); `LivingEntity.hurtServer` (cooldown, sounds, `die`); `ServerExplosion.hurtEntities`; the tags `#always_hurts_ender_dragons` and `#is_explosion`.
- **Tests:** `ender_dragon`: `only_the_head_and_neck_take_full_damage_and_only_from_players_and_explosions` (head 20 takes 20, body 20 takes 6, no player takes nothing, an explosion always hurts), `hits_within_the_damage_cooldown_only_count_their_extra_damage`, `a_killing_blow_sends_a_flying_dragon_to_die_and_a_hovering_one_down_at_once` (it flies down, rises and goes), `a_sitting_dragon_takes_off_after_a_quarter_of_its_health`, `a_blast_hurts_the_dragon_once_through_its_nearest_part`. `entities`: `dragon_parts_are_flat_hit_boxes_keyed_by_dragon_and_part`. `sim` `voxel`: `a_flat_hit_box_takes_every_bullet_in_the_upper_torso`. `render_anim` `minecraft_end`: `dragon_parts_carry_the_flat_hit_bit_the_bullets_read`. `server`: `mw2_shots_and_blasts_hurt_the_dragon_through_its_parts_until_it_dies_and_is_reported` (an End level of seed 0: a body shot of 20 takes 6 with the hurt sound, a head shot 20, a blast at the head more than 40; a killing blow sends it to die, it rises, goes and is reported once). `dragon_fight`: the boss bar test also kills the fight's dragon (not another one), which hides the bar, saves the flags and keeps a new dragon from being made.
- **Decisions:**
  - **The neck takes full damage too:** the brief says only the head does, but 26.3's `hurt(part)` tests `part != head && part != neck` (javap) before cutting the damage. Vanilla wins.
  - **The flat hit location** is the upper torso (MW2 `torso_upper`), whose multiplier is 1 for the weapons; it is a bit in the key (bit 55) that `sim` reads, so `sim` learns nothing else about Minecraft.
  - **No knockback on the dragon:** vanilla's `dealDefaultKnockback` halves a flying dragon's speed on a full hit by a player; the hit here carries no attacker position yet. Left for X5 with the wing knockback.
  - **Takeoff:** a sitting dragon past 50 damage takes off; with no takeoff phase yet (X5) it goes straight back to circling.
  - **Explosion exposure:** the dragon flies in the open, so a blast sees all of it (exposure 1, no ray test against blocks).
  - **Death:** a dragon whose health runs out flies to the podium, its health drops to 0 there, it rises for 10 seconds and is gone. The server reports it (`Output.dragons_killed`, `Entities::take_dragons_killed`) and the fight takes the flags part of `setDragonKilled` (`dragon_killed`, `previously_killed`, the bar's progress 0, saved), so the bar goes and no new dragon is made after 1200 ticks. Experience, the exit portal, the egg and the gateway are X4's (E3-18, E3-19).
- **Test steps** (survival; the MW2 class with an assault rifle, a sniper rifle and frag grenades; `/difficulty easy` keeps the head's contact hits survivable):
  1. Keep `IW4L_MINECRAFT_WORLD=x3dragon` with `IW4L_MINECRAFT_DIMENSION=the_end`, or a new world. On the platform, `/gamemode creative`, fly to the island centre and pillar a cobblestone tower about 30 blocks high near the middle; `/gamemode survival` on top.
  2. Shoot the dragon as it passes: it flashes red and roars on a full hit, and the boss bar drops; hits on the head and neck take more off than hits on the body, wings or tail. **Screenshot 4** of the bar after a few hits.
  3. Fire a long burst at its head: the bar falls about as fast as with single shots spaced half a second apart (the damage cooldown), and a sniper headshot takes no more than a sniper body shot on the head part.
  4. Throw a frag grenade so it bursts beside it: the bar drops (explosions always hurt it).
  5. When it passes close, hit it with the sword (left-click on any part): the bar drops a little.
  6. Bring the bar to zero: it stops circling, flies down toward the podium in the island's middle, then rises slowly and vanishes about 10 seconds later; the log says `Minecraft ender dragon of the fight killed`, and the bar goes. No experience or portal yet (X4). **Screenshot 5** while it rises.
  7. Stay a minute and quit and start again: no new dragon comes, and no bar shows.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:diamond_sword+sharpness=5,minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:water_bucket
```

### 26. Minecraft end crystals heal the ender dragon with a beam (E3-14)

Previous commit: 8bf65034

- **Changed:**
  - `minecraft_terrain/src/ender_dragon.rs`: `check_crystals` is `EnderDragon.checkCrystals`, run in `aiStep` before the flight: a nearest crystal that is gone is dropped, otherwise every tenth tick it heals 1 (up to 200); and one tick in ten (`nextInt(10)`) the dragon picks the nearest crystal (by feet) whose box meets its own 16 by 8 box grown by 32 (`crystal_in_reach`), or none. `crystal_destroyed` is `EnderDragon.onCrystalDestroyed`: when the lost crystal was its nearest, the head takes 10 as an explosion (`explosion(crystal, player)`, which always hurts it). `DragonEnv.crystals`; `DragonView.nearest_crystal` and `tick_count`.
  - `minecraft_terrain/src/server.rs` (End region): the dragons tick with the level's crystals. A crystal destroyed by a hit or by a blast (`crystal_destroyed`, now for blast-destroyed crystals too) runs `EnderDragonFight.onCrystalDestroyed` in a dragon fight level: the spike crystals are counted again (`updateCrystalCount`), and the fight's dragon (only that one, `getEntity(dragonUUID)`) is told.
  - `minecraft_terrain/src/dragon_render.rs`: `append_beams`, `EnderDragonRenderer`'s heal beam: from two blocks above the dragon's feet to its crystal's bobbing glass (`beamOffset` with `EndCrystalRenderer.getY`), through `end_crystal_render::append_beam` (black and thin at the dragon, white and wide at the crystal, scrolling with the dragon's age). `minecraft_survival/src/entities.rs`: one line draws them.
- **Checked with javap (26.3):** `EnderDragon` (`checkCrystals`, `onCrystalDestroyed`, `CRYSTAL_DESTROY_TARGETING`), `EnderDragonFight.onCrystalDestroyed` and `updateCrystalCount`, `EndCrystal.onDestroyedBy`, `EnderDragonRenderer` (`extractRenderState`'s `beamOffset`, `submit`, `submitCrystalBeams`).
- **Tests:** `ender_dragon`: `a_crystal_within_32_blocks_heals_the_dragon_a_point_every_ten_ticks` (the nearer of two crystals in reach, 10 health in 100 ticks, never above 200, dropped when gone, the reach box's edges), `destroying_the_crystal_that_heals_it_blasts_the_dragon_s_head_for_10`. `dragon_render`: `the_heal_beam_reaches_from_two_above_the_dragon_to_the_bobbing_glass`. `server`: `spike_crystals_heal_the_fight_dragon_and_breaking_its_crystal_hurts_it` (an End level of seed 0: the circling fight dragon finds a spike crystal and heals; shooting its crystal costs it 10 on the head, recounts 9 crystals and queues the crystal's blast; a crystal destroyed by a blast while it heals the dragon hurts it too and leaves 8).
- **Decisions:**
  - **The player behind a crystal's loss** is the one who broke it or nobody; vanilla also falls back to the nearest player within 64 blocks for the holding pattern's strafe, which comes with X5. The dragon is hurt either way.
  - The beam is drawn from the dragon's own feet as vanilla's is (the beam does not start at the head).
- **Test steps** (survival; MW2 weapons):
  1. Keep `IW4L_MINECRAFT_WORLD=x3dragon` with `IW4L_MINECRAFT_DIMENSION=the_end`, or a new world. Climb a tower near the island centre as in entry 25.
  2. Shoot the dragon until the bar is at about half. Watch it fly past the pillars: when it passes within about 32 blocks of a crystal, a beam reaches from just above its back to the crystal, black near the dragon and white at the crystal, its pattern sliding along it. The bar slowly fills again (a point every half second). **Screenshot 6** (the beam).
  3. While the beam stands, shoot that crystal: it explodes, the beam goes, the dragon flashes red and the bar drops a little (10 points).
  4. Shoot the other crystals (the caged ones need the bars shot or broken first): with none left, no beam forms any more and the bar no longer refills.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:diamond_sword+sharpness=5,minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:water_bucket
```

### 27. The Minecraft dragon fight passes clippy

Previous commit: af14e52b

- **Changed:** the two clippy findings on this group's lines, in `minecraft_survival`: `entities.rs`'s `end_target_of` builds the dragon part with `then_some` (`unnecessary_lazy_evaluations`), and a `dragon_fight` test makes its fight with the players in the struct (`field_reassign_with_default`).
- **Clippy:** `clippy -p minecraft_terrain -p minecraft_survival -p render_anim -p hud -p frame -p sim --all-targets --no-deps --locked -- -A clippy::approx_constant` has no other finding on a line this group added or changed (every reported location checked against `git diff -U0 af3cc119`); the rest are older. The vendored `minecraftoss-entities`, linted as its own workspace (`clippy --manifest-path third_party\minecraftoss\entities\Cargo.toml --offline --all-targets`, the `Cargo.lock` it made deleted afterwards), has none on this group's lines either.
- **Tests:** none new; the touched crates pass as before.
- **Test steps:** none; nothing changes in game.
- **Kit:** none.

### Group X3 summary

Written with entry 27, in its commit.

- **Commits:** 63ff26c8 (the End entry's travel sound), 039c31d8 (E3-9, end crystals), 5a6fd423 (E3-11, the dragon's holding pattern), 2b3167b1 (E3-12, the boss bar), 8bf65034 (E3-13, damage through the parts, and the kill reported to the fight), af14e52b (E3-14, crystal healing), and this entry (clippy).
- **Tests** (`test -p <crate> --locked`, `MINECRAFTOSS_ROOT` set by `cargo-slot.ps1`): `minecraft_terrain` 728 (4 ignored), `minecraft_survival` 328 and the vanilla world test, `render_anim` 71, `hud` 135, `frame` 17, `sim` 73 and `minecraftoss-entities` 83 pass; `check -p launcher` is green.
- **New files** (`rustfmt --check` on the staged content is empty): `minecraft_terrain/src/end_crystal.rs`, `end_crystal_render.rs`, `posed_cube.rs`, `ender_dragon.rs`, `dragon_render.rs` (edition 2021), `hud/src/minecraft_boss.rs` (2024). The track's earlier files edited here (`the_end/dragon_fight.rs`, `render_anim/src/minecraft_end.rs`) still pass.
- **MinecraftOSS patches in this group:** `entities/src/world.rs` (`EntityWorld::blast_by_player`, the public blast with a player attacker, through `apply_explosion_by`).
- **Plan decisions kept:** 5 (crystal blasts are vanilla explosions through the public blast, credited to the player), 6 (one flat MW2 hit location on every part, the vanilla part rule, MW2 grenades count as explosions, no headshot stacking), 9 (the dragon, the crystals and the boss bar take the players of the End level from the tick, never `PLAYER = 0`; the fight, its dragon UUID, its kill and its crystal count are saved or kept per End level, and each player's bar comes from the fight's own player list by UUID).
- **Not done in this group, and why:**
  - **X5 (out of scope this round):** the landing approach, perching, sitting phases, breath and fireballs, strafing, the charge, the takeoff phase, the wing knockback, `checkWalls` block breaking, and the egg teleport. The dragon only circles and strikes with its head and neck.
  - **X4 (the next group):** the death's rays, sound and experience (E3-18; the dragon already rises and goes), the exit portal, egg and gateway on the kill (E3-19; the fight already marks the kill), the exit portal home with the credits (E3-21), and the respawn with four crystals (E3-22; placing crystals in the End does not try it yet).
  - **Small gaps:** no knockback on the dragon from players' hits; no boss music (the game has no Minecraft music); a crystal's blast reaches mobs, players and other crystals one tick late; a player killed by a crystal blast is "blown up" without the name of who set it off.
- **Owner test for the group** (one new world, survival; screenshots numbered across the entries):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x3` and the kit below; no `IW4L_MINECRAFT_DIMENSION` line. Build the ring of 12 frames around a cleared 3x3, fill it with eyes and step in: the portal whoosh plays as you land on the platform (entry 21).
  2. Entry 23 steps 2 to 5 (the dragon circling, its sounds, its head's strike), screenshots 1 and 2; entry 24 steps 2 to 4 (the boss bar and the plain fog sky), screenshot 3.
  3. Entry 22 steps 2 to 8 on the platform and one pillar (crystals, their fire, blasts, placing, grenades, the sword), using the end crystals of the kit; then entry 26 steps 2 to 4 (the heal beam), screenshot 6, before you shoot all the pillar crystals.
  4. Entry 25 steps 1 to 7 (hits on the parts, grenades, the sword, the death and its report), screenshots 4 and 5.
  5. Entry 23 step 7 (a summoned dragon hovers) after the fight. Then `/kill`: you respawn in the overworld (the exit portal home comes with X4), and no bar shows there (entry 24 step 5).
- **Kit for the group** (22 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*12,minecraft:ender_eye*12,minecraft:end_crystal*16,minecraft:obsidian*16,minecraft:end_stone*4,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:water_bucket,minecraft:cooked_beef*32,minecraft:shield,minecraft:totem_of_undying,minecraft:ender_pearl*16,minecraft:torch*64,minecraft:oak_planks*64
```

## Third run, group X4: the dragon dies, going home

E3-18, E3-19, E3-21 and E3-22 of the brief, with plan decisions 3 and 9, and the End docs. Base: 89f9fd97 (group X3).

### 28. Minecraft ender dragon dies like vanilla (E3-18)

Previous commit: 89f9fd97

- **Changed:**
  - `minecraft_terrain/src/ender_dragon.rs`: `tick_death` is `EnderDragon.tickDeath`. Its first tick plays `entity.ender_dragon.death` (level event 1028, `DRAGON_DEATH_SOUND`); every fifth tick after tick 150 it awards `floor(xp · 0.08)` and on tick 200 `floor(xp · 0.2)` as experience orbs at the dragon, where `xp` is 12000 for the fight's dragon before any was killed (`!hasPreviouslyKilledDragon`) and 500 otherwise, and nothing with `mob_drops` off (`death_experience`, `DragonEnv.death_experience`, `DragonEvents.experience`). It still rises 0.1 a tick and goes after 200 ticks. `heard_at` places the death roar as vanilla's global event does: 2 blocks from the listener toward the dragon's block, at volume 5, so it is heard in the whole level.
  - `minecraft_terrain/src/server.rs` (End region): `FightLink` (the fight's dragon and whether one was killed before) replaces `set_fight_dragon` with `link_fight`; the dragons tick with their death experience and the orbs go to the level (`ExperienceOrb.award`). `apply_game_rules` keeps `mob_drops` for the dragon (one line).
  - `minecraft_survival/src/the_end/dragon_fight.rs`: `link_state` sends the dragon and `previously_killed` to the server whenever either changes.
  - `minecraft_terrain/src/dragon_render.rs`: `append_rays` is `EnderDragonRenderer.submitRays`: `floor((t + t²)/2 · 60)` rays (t the death time over 200) from (0, -1, -2) in the dragon's pose, each a three-sided fan turned by the last one's turn and two more random Euler turns from a random seeded 432 (the same rays every frame), 5 to 25 blocks long and 1 to 3 wide, growing by up to 10 and 2 and fading out over the last fifth; white at the body, transparent magenta at the tips. `mesh.rs`: a generated white tile (`WHITE_TEXTURE`) joins the entity atlas so untextured geometry can be drawn there.
  - `minecraft_survival/src/entities.rs`: the rays go into the translucent mesh (one line), and mob sounds pass through `heard_at` (one line).
- **Checked with javap (26.3):** `EnderDragon.tickDeath` (the 180-200 explosion emitter, the 12000/500 rule, `MOB_DROPS`, `ExperienceOrb.award`, event 1028 at tick 1 unless silent, `move(0, 0.1, 0)`, `setDragonKilled` and removal at 200), `EnderDragon.aiStep` (the explosion particles while dying), `LevelEventHandler.globalLevelEvent` (1028: `ENDER_DRAGON_DEATH`, hostile, volume 5, pitch 1, 2 blocks from the camera), `EnderDragonRenderer.submit` and `lambda$submitRays$0`.
- **Tests:** `ender_dragon`: `the_death_adds_up_to_12000_experience_on_the_first_kill_and_500_after` (shares of 960 and 2400, or 40 and 100, on ticks 155 to 200; nothing with `mob_drops` off), `the_dying_dragon_roars_once_and_rises_twenty_blocks`, `the_death_roar_is_heard_two_blocks_from_the_camera_toward_the_dragon`. `dragon_render`: `the_death_rays_grow_in_number_and_fade_in_the_last_fifth`. `server`: `the_first_dragon_drops_12000_experience_and_a_later_one_500` (an End level of seed 0: the orbs of the first kill hold 12000, a later fight dragon adds 500, none with `mob_drops` off; one death roar).
- **Decisions:**
  - **No dissolve and no explosion particles:** vanilla draws the dying dragon with the `dragon_explosion_alpha` cut-out that eats it away, and puffs explosion particles around it every tick (emitters from tick 180). The entity shader has no dissolve pass and the game has no Minecraft explosion particle yet, so the dragon stays whole under its rays.
  - **The rays blend over the scene** in the translucent entity pass (vanilla adds them, `LIGHTNING` blending). On the dark End sky they look the same.
  - **The roar reaches the End level only:** vanilla's global event goes to every player on the server; here sounds go to the players of the level.
- **Test steps** (survival; MW2 weapons):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x4dragon` (a new world), `IW4L_MINECRAFT_DIMENSION=the_end` and the kit below. Start on the platform. Note your experience level (0).
  2. `/gamemode creative`, fly to the island centre and pillar up near the middle, `/gamemode survival` on top. Shoot the crystals, then the dragon until the bar is empty.
  3. It flies to the podium and stops: a loud roar plays wherever you stand, and white rays burst from its body, more and longer each second, while it rises. **Screenshot 1.**
  4. In the last seconds a flood of experience orbs falls from it. Go down and collect them: from level 0 you reach level 68. **Screenshot 2** of the level number.
- **Kit** (the end crystals are for entry 31 in the same world):

```
IW4L_MINECRAFT_KIT=minecraft:end_crystal*16,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:diamond_sword+sharpness=5,minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:water_bucket
```

### 29. Minecraft exit portal opens and the dragon egg and a gateway appear (E3-19)

Previous commit: 4484b1eb

- **Changed:**
  - `minecraft_terrain/src/server.rs` (End region): `ServerSim::dragon_killed` is the block half of `EnderDragonFight.setDragonKilled`: `spawnExitPortal(true)` (the active podium, whose bowl fills with end portal, at the fight's exit portal location), then `spawnNewGateway` for the gateway the fight hands over (level event 3000: `block.end_gateway.spawn` at volume 10 and pitch `(1 + (r − r)·0.2)·0.7` at the gateway, then the `end_gateway_delayed` feature), then on the first kill a dragon egg on top of the podium's `MOTION_BLOCKING` height (on the pillar). `end_gateway_position(i)` is vanilla's `floor(96·cos(2(−π + 0.15707963267948966·i)))`, 75, `floor(96·sin(…))`.
  - `minecraft_survival/src/the_end/dragon_fight.rs`: when the server reports the fight's dragon dead, `tick_level` notes whether one was killed before (the egg), marks the kill, and `open_exit_portal` sends the job with the next gateway: `next_gateway` takes the last of the 20 shuffled numbers (`gateways.remove(size − 1)`), so the list shrinks and is saved. The answer records the exit portal location and saves the fight (`FightEvents.opened`).
  - `render_anim/src/minecraft_end.rs`: logs `Minecraft exit portal opened at ...`.
- **Checked with javap (26.3):** `EnderDragonFight.setDragonKilled` (progress 0, bar hidden, `spawnExitPortal(true)`, `spawnNewGateway()`, the egg at `getHeightmapPos(MOTION_BLOCKING, getPodiumLocation(origin))` with `setBlockAndUpdate` only without `hasPreviouslyKilledDragon`), `spawnNewGateway()` and `spawnNewGateway(BlockPos)` (event 3000, `END_GATEWAY_DELAYED` with a fresh random), `spawnExitPortal`, `LevelEventHandler` event 3000 (explosion emitter, `END_GATEWAY_SPAWN`, blocks, 10, `(1 + (r − r)·0.2)·0.7`). The gateway positions were checked with `jshell`.
- **Tests:** `server`: `the_twenty_end_gateways_ring_the_island_96_blocks_out_like_vanilla` (all 20 against `jshell`), `a_kill_opens_the_exit_portal_lays_the_egg_on_the_pillar_and_spawns_a_gateway` (an End level of seed 0: the inactive podium's bowl fills with end portal, the egg on the pillar, the gateway and its sound; a second kill places its gateway, keeps the first egg and lays none). `dragon_fight`: `each_kill_takes_the_last_of_the_shuffled_gateways_until_none_is_left`, `the_server_learns_the_fight_s_dragon_and_whether_one_was_killed_before`.
- **Decisions:**
  - The exit portal opens a tick or two after the dragon is gone, when the server's report reaches the fight; vanilla does it in the dragon's last tick.
  - Not drawn yet: the gateway's purple beam while it spawns (X6) and the explosion emitter of event 3000 (no explosion particle). The gateway doesn't throw players yet (X6, E3-23).
- **Test steps** (continue in `x4dragon` after entry 28's kill, or kill the dragon again there):
  1. When the dragon is gone, the log says `Minecraft exit portal opened at (0, ..., 0)` and a loud whoosh plays. Go to the podium: the bowl is filled with the end portal starfield, and a black dragon egg sits on top of the bedrock pillar. **Screenshot 3.**
  2. Look out over the island's edge about 96 blocks from the centre at height 75: a floating end gateway (a starfield cube in a small bedrock frame) has appeared. **Screenshot 4.**
  3. Quit and start again: the portal, the egg and the gateway are still there, and no dragon or boss bar comes back.
- **Kit:** the kit of entry 28.

### 30. Minecraft exit portal takes players to their respawn point and shows the credits once (E3-21)

Previous commit: 033a52e1

- **Changed:**
  - `minecraft_survival/src/the_end.rs`: leaving the End through an end portal now goes through `show_credits` first, `EndPortalBlock.entityInside`'s check: a player without `seenCredits` gets `showEndCredits()`, which sets `seen_credits` at once (saved as `seenCredits` with the player) and `EndTravel.credits`, the per-player hold. The trip to the respawn point (or the world spawn) starts as before (entry 7). `holding_for_credits` and `credits_done`; `Trip::OutOfTheEnd` says whether the credits roll.
  - `render_anim/src/minecraft_world.rs` (E1's file, four lines): the switch is not finished while the player is held for the credits, so the trip home loads behind them and the player arrives only when they close (vanilla removes the player from the End and respawns them on `PERFORM_RESPAWN`); one call line runs `minecraft_end::credits` each frame; a stopped world clears them.
  - `render_anim/src/minecraft_end.rs`: `credits` starts the roll when the local player is held (the texts are read once from the resource packs at runtime: `texts/end.txt`, `texts/credits.json`, `texts/postcredits.txt`, and the `gui/title/minecraft` and `edition` logos; nothing of Mojang's is committed), and ends the hold when the screen says it is finished. `credit_sections` reads `credits.json`. `minecraft_loading.rs`: `gpu_image` is `pub(crate)`.
  - New `frame/src/minecraft_credits.rs`: `McCredits`, the model of `WinScreen`. `poem_lines` is `addPoemFile`: `PLAYERNAME` becomes the player's name, each obfuscation token (white, obfuscated, green, aqua) becomes white obfuscated `XXXXXXXX` cut to `nextInt(4) + 3` letters from a random seeded 8124371, each line wraps at 256 pixels (`Font.split`, colour codes taking no width and carrying over), an empty line follows each, and eight close the file. `credits_lines` is `addCreditsFile`: per section a row of `=`, the section in yellow and another row of `=`, centred, then two empty lines; per discipline its name in yellow centred and two empty lines; per title the title in grey, each name in white behind 11 spaces, two empty lines. `styled` applies the legacy codes as vanilla's `applyLegacyFormat` does (a colour ends obfuscation). The scroll moves 0.5 pixels a tick (the poem's speed), times `5 + 15 x ctrl keys` while Space is held and backward while Up is held, never below 0; the screen is finished once it passes `12 x lines + 2 x height + 24`. The logo sits at `height + 50` and the text 100 below, 12 pixels a line, and the last line stops in the middle of the screen. `MinecraftUi.credits` (appended).
  - New `hud/src/minecraft_credits.rs`: draws the logo (256 by 44 of the 256 by 64 texture) with the edition (128 by 14) overlapping its foot by 7, then each line on screen (centred lines around the middle, the rest from `width/2 - 128`) with shadows, obfuscated letters swapped each frame for random ones of the same width; Escape skips, Space, Ctrl and Up steer as in vanilla. It draws over the End portal loading screen of the held trip, which stands in for the End portal background of `WinScreen`. `plugin.rs`, `gpu_list.rs`, `lib.rs`: its update, flush, raster and job (appended). `minecraft_loading.rs`: no "Loading terrain" text under the credits. `menus/mod.rs`: Escape doesn't open the MW2 menu while the credits roll.
- **Checked with javap (26.3):** `EndPortalBlock.entityInside` (`showEndCredits` for a player in the End without `seenCredits`, else `setAsInsidePortal`), `ServerPlayer.showEndCredits` (`wonGame`, `WIN_GAME`, `seenCredits = true`), `WinScreen` (`<init>` speed 0.5 for the poem, `calculateScrollSpeed`, the end in `tick`, `keyPressed`/`keyReleased` with Space, both Ctrl keys and Up, `onClose` to `respawn`, `init`, `addPoemFile`, `addCreditsFile`, `addCreditsLine`, the layout of `extractRenderState`, the 8124371 seed, `OBFUSCATE_TOKEN`, `NAME_PREFIX`, `SECTION_HEADING`), `LogoRenderer` (256 by 44 of 64, the edition 128 by 14 of 16 at `y + 44 - 7`).
- **Tests:** `minecraft_credits` (frame): `colour_codes_set_the_colour_and_end_obfuscation_like_vanilla`, `the_poem_names_the_player_and_hides_the_unspeakable_words`, `long_poem_lines_wrap_at_256_pixels_on_a_space`, `the_credits_list_sections_disciplines_titles_and_names_like_vanilla`, `the_credits_scroll_half_a_pixel_a_tick_and_end_two_screens_past_the_last_line`, `the_last_line_stops_in_the_middle_of_the_screen`. `minecraft_credits` (hud): `the_logo_sits_centred_with_the_edition_overlapping_its_foot`, `a_line_keeps_its_span_colours_and_hides_obfuscated_words`, `the_credits_listen_to_escape_space_control_and_up`. `minecraft_end` (render_anim): `the_credits_json_becomes_sections_disciplines_titles_and_names`, `the_vanilla_credits_and_poem_are_read_from_the_jar` (with `MINECRAFTOSS_ROOT`: the real texts give over 1000 lines, "Steve?" and hidden words). `the_end`: `the_first_exit_rolls_the_credits_once_and_holds_the_trip_until_they_close`.
- **Decisions:**
  - **Plan decision 3 (vanilla credits):** once per player, by UUID (`seenCredits` on the player), set when they start, so quitting during them doesn't show them again (vanilla sets it in `showEndCredits` too); Escape skips; the trip home loads behind them.
  - **While they roll** the player is in the held switch: frozen, invulnerable and not ticking, as the vanilla player is out of every level until the screen closes.
  - **Not drawn:** the `credits_vignette` darkening (it needs the `VIGNETTE` multiply blend, which the HUD has no pass for) and the credits music (the game plays no Minecraft music yet).
  - A respawn point in the End itself (only `/spawnpoint` can set one there) shows the credits over the End without a switch, so the player isn't held.
- **Test steps:**
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x4credits` (a new world) and the kit below. In the overworld walk about 50 blocks from the spawn and type `/spawnpoint`.
  2. Build the ring (12 frames facing inward around a cleared 3x3), fill it and jump in. On the platform type `/setblock ~2 ~ ~ minecraft:end_portal` and walk into it (or use the exit portal after killing the dragon).
  3. The credits roll over the moving End starfield: the Minecraft logo scrolls up, then the End Poem in aqua and green with your name, a few words shown as flickering white letters, then the credits sections in yellow with grey titles and white names. **Screenshot 5** of the poem, **screenshot 6** of the credits.
  4. Hold Space: they scroll five times faster; Space with Ctrl faster still; hold the Up arrow: they scroll back.
  5. Press Escape: the credits close and you stand at your `/spawnpoint` with everything you carried. **Screenshot 7.**
  6. Go into the End again and walk into the exit block: no credits, you arrive at your spawn point at once.
  7. Quit, start again, go to the End and out: still no credits (`seenCredits` was saved).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*12,minecraft:ender_eye*12,minecraft:diamond_pickaxe+efficiency=5,minecraft:cobblestone*64
```

### 31. Minecraft end crystals on the exit portal respawn the dragon (E3-22)

Previous commit: 4c7667cf

- **Changed:**
  - New `minecraft_terrain/src/dragon_respawn.rs`: `Respawn` is the respawn part of `EnderDragonFight` with `DragonRespawnStage`'s five stages (saved by their vanilla names), ticked with the stage time handed over before it counts up, as `respawnStage.tick(.., respawnTime++)` is. START turns the crystals' beams to (0, 128, 0) and moves on; PREPARING_TO_SUMMON_PILLARS growls (level event 3001) on ticks 0, 50 to 52 and 95 to 99, and moves on at 100; SUMMONING_PILLARS takes 40 ticks a pillar: on the first it beams at the pillar's top, on the last it rebuilds the pillar, and after the last pillar it moves on; SUMMONING_DRAGON beams at the sky on tick 0, growls on ticks 1 to 4 and 80 to 99, and on tick 100 summons the dragon. `crystal_cells` are the four cells one above the exit portal and 3 out to the north, east, south and west (`Direction.Plane.HORIZONTAL`).
  - `minecraft_terrain/src/server.rs` (End region): `try_respawn` is `EnderDragonFight.tryRespawn` and `respawnDragon`, run when an end crystal is placed in a dragon fight level (`EndCrystalItem.useOn`): only once the dragon is killed and no respawn runs, with the exit portal from the fight (or found, or built active), a crystal in each of the four cells, then the exit portal pattern is turned to end stone (`dismantle_exit_portal`), the inactive podium is built, and the respawn starts with those crystals. `tick_respawn` runs it each tick: a growl is `entity.ender_dragon.growl` at (0.5, 128.5, 0.5), volume 64, pitch 0.8 + r·0.3; a rebuilt pillar clears the blocks within 10 of its top, explodes there with power 5 (which destroys the old crystal), and places `end_spike` for it, whose new crystal is invulnerable with its beam on (0, 128, 0); the summoning makes a new dragon at (0, 128, 0) in its holding pattern, resets the pillar crystals (vulnerable, no beam) and blows up the four crystals with power 6 without breaking blocks (`ExplosionInteraction.NONE`, `BlastSource::KeepBlocks`). Losing a respawn crystal (`onCrystalDestroyed`), or all of them going, aborts: the pillar crystals are reset and the portal lights again. `FightReport` (`Respawning` at each stage, `Aborted`, `Respawned`) goes to the fight through `Output.fight_reports`; `FightLink` now also carries `dragon_killed` and the exit portal.
  - Vendored `world/src/level/end_portal.rs` (MinecraftOSS patch): `find_exit_portal_match` and `dismantle_exit_portal` (the bedrock and end portal blocks of each match become end stone until no match is left, at most 16 rounds). Vendored `generator/src/feature/end_portal.rs`: `ExitPortal::cells`, the 7 by 7 by 5 cells of a match.
  - `minecraft_terrain/src/end_crystal.rs`: crystals save `Invulnerable`; `uuid`, `in_cell`, `by_uuid`, `discard`, `reset_on_spike`; `Destroyed.uuid`.
  - `minecraft_survival/src/the_end/dragon_fight.rs`: `receive_report` saves the stage, the four crystals' UUIDs (`respawn_crystals`) and the exit portal (the stage time is saved as 0), clears them on an abort, and on a new dragon clears `dragon_killed`, takes its UUID and fills the bar. A saved respawn is handed back to the server once after the first link (`saved_respawn`, `resume_job`). `entities.rs`: the reports are kept for the fight (`take_fight_reports`). `render_anim/src/minecraft_end.rs` logs the stages and the new dragon.
- **Checked with javap (26.3):** `EnderDragonFight` (`tryRespawn`, `respawnDragon`, `tick`'s respawn part, `setRespawnStage`, `abortRespawnSequence`, `resetSpikeCrystals`, `onCrystalDestroyed`, `createNewDragon`, the codec's `respawn_stage`, `respawn_time` and `respawn_crystals`), `DragonRespawnStage` and its five subclasses' `tick`, `LevelEventHandler` event 3001 (`ENDER_DRAGON_GROWL`, hostile, 64, `0.8 + r·0.3`), `EndCrystalItem.useOn` (`tryRespawn` in a level with a dragon fight).
- **Tests:** `dragon_respawn`: `the_stages_keep_vanilla_names`, `the_respawn_beams_growls_rebuilds_each_pillar_and_summons_like_vanilla` (every action on its vanilla tick, 20 pillar actions, the summon 100 ticks into the last stage), `the_four_crystals_stand_three_blocks_out_one_above_the_portal`. `server`: `four_crystals_on_the_exit_portal_rebuild_the_pillars_and_summon_a_new_dragon` (an End level of seed 0 after a kill with two pillar crystals shot: three crystals do nothing, the fourth starts it, the portal goes out and the inactive podium stands, the stages are reported in order, the new dragon is the fight's, every pillar has one vulnerable crystal again and a bedrock top, the four crystals blow up without breaking blocks, 33 growls), `breaking_a_respawn_crystal_stops_the_respawn_and_lights_the_portal_again`. `dragon_fight`: `a_respawn_is_saved_by_stage_and_a_new_dragon_ends_it`. Generator `end_portal`: `the_exit_portal_is_found_from_its_pillar_top_and_names_its_centre` also checks the cells.
- **Decisions:**
  - **The new dragon:** it is the fight's dragon with `previously_killed` still set, so it gives 500 experience and lays no egg (E3-18, E3-19), and its kill places the next gateway.
  - **Saved mid-respawn:** the stage is saved when it changes, with its time as 0, so a world quit during the respawn restarts the current stage when loaded (vanilla saves the tick within it). The crystals are saved with their chunks and found by UUID.
  - **Where it ticks:** the respawn runs on the server thread while the End level runs (a player is in it); vanilla also waits for the arena to be loaded.
  - **Explosion order:** a rebuilt pillar's blast hurts the mobs, players and blocks around it and destroys the old crystal at once, before the new pillar is placed, as vanilla does; the four crystals' final blasts go off on the next server tick.
- **Test steps** (continue in `x4dragon` after the kill of entries 28 and 29, survival):
  1. Stand by the exit portal with 4 end crystals. On each side of the bowl, place one crystal on the bedrock rim block in the middle of the side (north, east, south and west, 3 blocks from the pillar). After the fourth, the portal's starfield goes out and the four crystals beam into the sky above the island; deep growls sound. **Screenshot 8.**
  2. Watch the pillars: one after another the beams swing to a pillar top, which explodes and is rebuilt with a new crystal (the caged ones get their cages back). The log says `Minecraft ender dragon respawn: summoning_pillars`. **Screenshot 9.**
  3. About 30 seconds after the fourth crystal, the four crystals explode (the rim stays whole), a new dragon circles the island, and the boss bar is back. The log says `Minecraft ender dragon respawned: [...]`.
  4. Kill it: it gives only 500 experience (a fraction of a level now), no second egg appears (the first stays on the pillar), and a second gateway appears elsewhere on the ring. **Screenshot 10.**
  5. Optional: place the four crystals again and break one of them during the pillar rebuilds: the beams stop, the portal lights again, and no dragon comes.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_crystal*16,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:diamond_sword+sharpness=5,minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:water_bucket
```

### 32. The Minecraft dragon's death and the way home pass clippy

Previous commit: ffd73868

- **Changed:** the four clippy findings on this group's lines, all `unwrap_or_default` (disallowed by `clippy.toml`): the respawn report's exit portal in `minecraft_terrain/src/server.rs` falls back to `(0, 0, 0)` explicitly, and `render_anim/src/minecraft_end.rs` reads the credits lists and texts with `map_or_else(Vec::new, ..)` and `unwrap_or_else(String::new)`.
- **Clippy:** `clippy -p minecraft_terrain -p minecraft_survival -p render_anim -p hud -p frame -p minecraftoss-generator -p minecraftoss-world --all-targets --no-deps --locked -- -A clippy::approx_constant` has no other finding on a line this group added or changed (every reported location checked against `git diff -U0 89f9fd97`); the rest are older.
- **Tests:** none new; the touched crates pass as before.
- **Test steps:** none; nothing changes in game.
- **Kit:** none.

### 33. The Minecraft state doc covers the End

Previous commit: 2ab82999

- **Changed:** `docs/MINECRAFT-SURVIVAL.md`: a new section "The End (round 7, track END)" under Implemented (getting there, the dragon fight, going home with the credits, elytra and rockets), the simulated mobs line names the ender dragon and end crystals, and the roadmap's End part now says what is done and what is left: X5 (strafing and fireballs with breath clouds, the landing, perching with flame breath and the sitting phases, takeoff, wing knockback, block breaking, knockback from players' hits, the egg teleport), X6 (gateway teleports with the return gateway and the beam, item frames, shulkers and their bullets, levitation, the End flashes) and the smaller gaps.
- **Tests:** none; docs only.
- **Test steps:** none.
- **Kit:** none.

### Group X4 summary

Written with entry 33, in its commit.

- **Commits:** 4484b1eb (E3-18, the death), 033a52e1 (E3-19, exit portal, egg and gateway), 4c7667cf (E3-21, the exit portal home and the credits), ffd73868 (E3-22, the respawn with four crystals), 2ab82999 (clippy), and this entry (docs).
- **Tests** (`test -p <crate> --locked`, `MINECRAFTOSS_ROOT` set by `cargo-slot.ps1`): `minecraft_terrain` 740 (4 ignored), `minecraft_survival` 332 and the vanilla world test, `render_anim` 73, `hud` 138, `frame` 23, `minecraftoss-generator` 28 and `minecraftoss-world` 63 pass; `check -p launcher` is green.
- **Clippy:** see entry 32.
- **New files** (`rustfmt --check` on the staged content is empty): `minecraft_terrain/src/dragon_respawn.rs` (edition 2021), `frame/src/minecraft_credits.rs` and `hud/src/minecraft_credits.rs` (2024). The track's earlier files edited here (`ender_dragon.rs`, `dragon_render.rs`, `end_crystal.rs`, the vendored `end_portal.rs` pair, `the_end.rs`, `the_end/dragon_fight.rs`, `render_anim/src/minecraft_end.rs`) still pass.
- **MinecraftOSS patches in this group:** `world/src/level/end_portal.rs` (`find_exit_portal_match`, `dismantle_exit_portal`), `generator/src/feature/end_portal.rs` (`ExitPortal::cells`).
- **Plan decisions kept:** 3 (credits like vanilla: once per player through `seenCredits`, Escape skips, the trip home loads behind them, the texts read from the jar at runtime), 9 (the dragon's experience, the kill, the exit portal, the egg, the gateways and the respawn all go through the fight of the End level and its saved data; the credits and the hold are per player by UUID; nothing uses `PLAYER = 0`).
- **Not done in this group, and why:**
  - **X5 and X6** are out of scope this round (see entry 33 and the state doc). No X5 commit (E3-16 perching, E3-20 egg teleport) was started in group X4; whether X5 runs as an extra group is the orchestrator's call.
  - **Small gaps:** the dragon's dissolve and explosion particles while dying (no dissolve pass, no explosion particle), the gateway beam and event 3000's emitter, the credits vignette and music, the respawn's stage time is saved as the start of its stage.
- **Owner test for the group** (one new world, survival; screenshots numbered as in the entries):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x4` (a new world) and the kit below; no `IW4L_MINECRAFT_DIMENSION` line. In the overworld walk about 50 blocks from the spawn and type `/spawnpoint`.
  2. Build the ring of 12 frames facing inward around a cleared 3x3, fill it with eyes and step in.
  3. Entry 28 steps 2 to 4: shoot the crystals and kill the dragon; the roar, the rays and the experience flood to level 68. Screenshots 1 and 2.
  4. Entry 29 steps 1 and 2: the lit exit portal, the egg on the pillar and the gateway. Screenshots 3 and 4.
  5. Entry 31 steps 1 to 4: four crystals on the rim respawn the dragon; kill it again for 500 experience and no second egg. Screenshots 8 to 10.
  6. Quit while in the End and start again: you are still in the End, the portal and the egg are there, and no dragon or bar comes back (brief step 13).
  7. Entry 30 steps 3 to 7: walk into the exit portal; the credits roll, Escape takes you to your `/spawnpoint` with everything; later exits have no credits, also after a restart. Screenshots 5 to 7.
- **Kit for the group** (14 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*12,minecraft:ender_eye*12,minecraft:end_crystal*16,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:water_bucket,minecraft:cooked_beef*32
```

## Review of the third run (X3 and X4)

An adversarial review of af3cc119..3e3bc785: rules (no comments, rustfmt on the new files, no Mojang data), vanilla checks with javap (`EnderDragon.hurt(part)`, `handleKillingBlow`, `tickDeath`, `checkCrystals`, `DragonDeathPhase`, `DragonHoverPhase.isSitting`, `EndCrystal.hurtServer`, `EnderDragonFight.tick`, `tryRespawn`, `respawnDragon`, `DragonRespawnStage$3`, `Player.canCriticalAttack`, `GameRenderer`'s sky pass flag), MW2 regressions (the flat hit bit only rides on dragon part keys; MW2 maps build no mob boxes) and per-player state. Each defect found is fixed in its own commit below.

### 34. Minecraft dragon respawn waits for players near a loaded arena like vanilla

Previous commit: 3e3bc785

- **Defect:** the respawn with four crystals ran on the server whenever the End level ticked. `EnderDragonFight.tick` runs it only while the fight has players (within 192 blocks) and `isArenaLoaded()`; it returns before the respawn otherwise. Worst case: a world quit during a respawn, after the first visit's state scan, sent the saved respawn back to the server on the first frame, before the chunks holding the four crystals were loaded; the next tick found none of them and aborted, so the saved respawn was lost and the portal lit again (entry 31's "a world quit during the respawn restarts the current stage" did not hold). Walking more than 192 blocks away also kept it running while the arena could unload.
- **Changed:** `minecraft_terrain/src/server.rs`: `FightLink.ticking`; `tick_respawn` returns at once while the fight does not tick. `minecraft_survival/src/the_end/dragon_fight.rs`: `DragonFight.arena_loaded` (set each frame from the same arena check the scan and the dragon use), and `link_state` sends `ticking` = players near and the arena loaded, so the link is resent when either changes. Chunks go to the server before a job sent after them, so a ticking fight has the crystals loaded.
- **Checked with javap (26.3):** `EnderDragonFight.tick` (`isArenaLoaded()` returns before the respawn stage tick, all inside the non-empty player branch).
- **Tests:** `server`: `a_respawn_waits_while_the_fight_does_not_tick_and_a_reloaded_one_waits_for_its_crystals` (an End level of seed 0: four crystals start a respawn that stays at its start for 300 ticks without a ticking fight; a respawn resumed before its crystals load is kept, not aborted; once the crystals are back and the fight ticks, it moves to `preparing_to_summon_pillars`). `dragon_fight`: `the_fight_ticks_on_the_server_only_with_players_near_and_the_arena_loaded`. `respawn_world` links a ticking fight.
- **Track log:** the X4 summary no longer says X3 and X4 took the whole run.
- **Test steps** (survival; continue in `x4dragon` after a kill, or kill the dragon there first):
  1. Place the four crystals on the rim (entry 31 step 1). When the log says `Minecraft ender dragon respawn: summoning_pillars`, quit the game at once.
  2. Start again: you are in the End. Within a few seconds the beams come back from the four crystals, the pillars are rebuilt from the first one again, and about 30 seconds later the new dragon comes (the log says `Minecraft ender dragon respawned: [...]`, not `respawn: aborted`). **Screenshot 11.**
  3. Optional: place four crystals again after killing it, then `/gamemode creative` and fly more than 192 blocks from the island centre (`/tp 0 100 250`) right away: the beams stay where they are and no pillar is rebuilt while you are away; fly back and they go on.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_crystal*16,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:diamond_sword+sharpness=5,minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:water_bucket
```

### 35. Minecraft end crystals placed by players get a new UUID every session

Previous commit: b2e8a03d

- **Defect:** a placed end crystal took its UUID from the level's crystal random, which started from the same constant seed every session and also drew each loaded crystal's `time`. With the same number of crystals loaded before a placement, a crystal placed after a restart got the UUID of one placed in an earlier session, and both stood in the world. The respawn finds its four crystals by UUID, so a twin elsewhere took the beams, was blown up and discarded with them at the summon, or aborted the respawn when it broke; a vanilla load of the save would also drop one of the twins.
- **Changed:** `minecraft_terrain/src/end_crystal.rs`: `Crystals` draws new UUIDs from their own random seeded once per session (`session_seed`: the wall clock in nanoseconds mixed with a per-process counter), as vanilla's `Mth.createInsecureUUID` draws from an entity random seeded uniquely per session. The crystals' `time` keeps the old seeded random.
- **Tests:** `end_crystal`: `crystals_placed_in_two_sessions_never_share_a_uuid` (the same load and placement history in two sessions gives two different version-4 UUIDs).
- **Decision:** the dragon's UUID (`Dragons::fresh_tag` from `ServerSim.end_random`) keeps its seeded draws, because the dragon's flight random is seeded from its UUID and the server tests rely on a reproducible flight. Two dragons with one UUID would need the fight to make a new dragon while an earlier one of the same draw index still lives, which the held arena ticket rules out.
- **Test steps** (survival): a twin needs the same number of random draws before two placements, which no in-game step can arrange on purpose, so the unit test covers the fix. As a regression check:
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x4uuid` (a new world), `IW4L_MINECRAFT_DIMENSION=the_end` and the kit below. On the platform place an obsidian block and put an end crystal on it; it stands there without its base. Quit and start again: it is still there.
  2. Shoot it: it explodes as before.
  3. Remove the `IW4L_MINECRAFT_DIMENSION` line afterwards.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:end_crystal*8,minecraft:obsidian*8,minecraft:golden_apple*8,minecraft:diamond_chestplate
```

### 36. Minecraft ender dragon growls on while it dies

Previous commit: f62433d7

- **Defect:** the client stopped the dragon's growl once its health reached 0, so the dying dragon fell silent as it rose. 26.3's `EnderDragon.aiStep` counts `growlTime` down on the client before its dead-or-dying branch, held back only while the phase sits (`isSitting`), and `DragonDeathPhase` does not sit, so vanilla's dragon keeps growling through its death.
- **Changed:** `minecraft_survival/src/entities.rs` (`dragon_voices`, one line): only a sitting dragon skips the growl.
- **Checked with javap (26.3):** `EnderDragon.aiStep` (the client growl: `isSilent`, `isSitting`, `--growlTime < 0`, volume 2.5, `200 + nextInt(200)`, before `isDeadOrDying`).
- **Tests:** none new: the growl timer lives in `Entities::dragon_voices`, which runs only on a server snapshot; the `minecraft_survival` tests pass as before.
- **Test steps** (survival; in `x4dragon` or any End world with a dragon to kill): kill the dragon and listen while it rises under its rays for 10 seconds: besides the death roar, a growl may sound now and then, as before its death.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:diamond_sword+sharpness=5,minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:water_bucket
```

## Final check of the third run

### 37. The track END log sums up its third final check

Previous commit: 8dddc195

- **Commits of the third run** (since af3cc119): 63ff26c8 (the End entry's travel sound), 039c31d8 (E3-9), 5a6fd423 (E3-11), 2b3167b1 (E3-12), 8bf65034 (E3-13), af14e52b (E3-14), 89f9fd97 (clippy, X3), 4484b1eb (E3-18), 033a52e1 (E3-19), 4c7667cf (E3-21), ffd73868 (E3-22), 2ab82999 (clippy, X4), 3e3bc785 (state doc), b2e8a03d, f62433d7 and 8dddc195 (review fixes), and this entry.
- **Touched crates** (`git diff --stat af3cc119..8dddc195`): `frame`, `hud`, `minecraft_survival`, `minecraft_terrain`, `render_anim`, `sim`, `minecraftoss-entities`, `minecraftoss-generator`, `minecraftoss-world`.
- **Clippy:** `clippy -p frame -p hud -p minecraft_survival -p minecraft_terrain -p render_anim -p sim -p minecraftoss-entities -p minecraftoss-generator -p minecraftoss-world --locked --all-targets -- -A clippy::approx_constant` has no finding on a line the third run added: each of the 33 locations reported in its files blames to af3cc119 or earlier (`git blame af3cc119..8dddc195`). No fix needed.
- **Tests** (`test -p <crate> --locked`; the minecraftoss crates with the stand-in scenario files of `full-suite.ps1`): `minecraft_terrain` 742 (4 ignored), `minecraft_survival` 334, `render_anim` 73, `hud` 138, `frame` 23, `sim` 73, `minecraftoss-entities` 83, `minecraftoss-generator` 28 and `minecraftoss-world` 63 pass, none failed. The full suite runs at merge time.
- **X5 was not started** this run: E3-16 (perching and breath) and E3-20 (the egg teleport) stay out of scope, as do the rest of X5 and X6 (see entry 33).
- **Test steps:** none new. The owner's test of the third run is, in order: the X3 owner test in the new world `x3`; the X4 owner test in the new world `x4`; entry 34 in `x4` after a kill (place the four crystals, quit during `summoning_pillars`, restart); entry 35 in the new world `x4uuid` (with `IW4L_MINECRAFT_DIMENSION=the_end`, removed afterwards); entry 36 during any kill. The first and second runs' tests are listed in entries 13 and 20. A kit goes to a player only in a world where they have no inventory yet, so the one line below serves every world of the third run (its items cover each entry's kit from 21 to 36).
- **Kit for the third run** (22 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:end_portal_frame*12,minecraft:ender_eye*12,minecraft:end_crystal*16,minecraft:obsidian*16,minecraft:end_stone*4,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:water_bucket,minecraft:cooked_beef*32,minecraft:shield,minecraft:totem_of_undying,minecraft:ender_pearl*16,minecraft:torch*64,minecraft:oak_planks*64
```

## Fourth run, group X5-phases: a dragon that fights back

E3-15 and E3-16 of the brief: the phase machine beyond the holding pattern, with plan decision 9 (the dragon targets the players of the End level, never `PLAYER = 0`). Base: 88cb6d83.

### 38. Minecraft ender dragon strafes players and spits fireballs (E3-15)

Previous commit: 88cb6d83

- **Changed:**
  - `minecraft_terrain/src/ender_dragon.rs`:
    - `Phase::StrafePlayer` (id 1) is `DragonStrafePlayerPhase`: the target player by id, its own path and clockwise flag, and `fireball_charge`. Each tick a finished path re-aims at the player, `strafe_lift` (`min(0.4 + horizontal distance / 80 − 1, 10)`) above them; a target nearer than 10 or farther than 150 blocks finds a new path round the rings (inner ring while no crystal is alive). Within 64 blocks and in sight (dragon eye at 6.8 to the player's eye, at most 128 blocks, `LivingEntity.hasLineOfSight` through `minecraftoss_entities::sight`) the charge rises by one a tick, otherwise it falls; at 5 with the player within 10 degrees ahead (`acos` of the facing and the horizontal direction, plus 0.5) it plays `entity.ender_dragon.shoot` (level event 1017: volume 10, at the dragon's block), spits a fireball from one block behind the head's look and one block above the head toward the player's middle, ends its path and goes back to the holding pattern. A strafe whose player is gone goes back to the holding pattern.
    - The holding pattern at the end of a path picks the attackable player nearest to the podium (`TargetingConditions.forCombat().ignoreLineOfSight()`, any distance) and strafes them if `nextInt((int)(d + 2)) == 0` or `nextInt(crystals + 2) == 0`, with `d` the squared distance from the podium block's centre over 512. `strafePlayer`'s path is vanilla's `findPath(closest node, node closest to the player, final node over the player's block)`; `find_path` takes the final node (`reconstructPath` appends it, also to a partial path).
    - `crystal_destroyed`: after the head damage, a circling dragon turns to strafe the player behind the loss (`DragonHoldingPatternPhase.onCrystalDestroyed`, only an attackable player).
    - `aiStep`'s second `doServerTick` when the first one changed the phase. `view_vector` is `Entity.calculateViewVector`; `nearest_player`, `distance_to_center_sqr`, `block_center`; `PlayerBody.eye_height`; `DragonEnv.sees`; `DragonEvents.fireballs`. The holding pattern's circle goal and waypoint draw are shared helpers now (same draws as before).
  - New `minecraft_terrain/src/dragon_fireball.rs`: `DragonFireball` (`DragonFireball` of 26.3: the vendored `AbstractHurtingProjectile` flight of `minecraftoss_entities::fireball`, 1 by 1, acceleration 0.1, inertia 0.95) and `DragonFireballs`. A burst (`onHit`) leaves `breath_cloud` at the first living entity whose box meets the fireball's box grown by 4, 2, 4 with its feet within 4 blocks, else where it burst (`cloud_position`), plays `entity.dragon_fireball.explode` (level event 2006: volume 1, pitch 0.9 to 1). `append_fireballs` is `DragonFireballRenderer`: a camera-facing quad of `entity/enderdragon/dragon_fireball` scaled by 2, full bright.
  - `minecraft_terrain/src/effect_cloud.rs`: `EffectCloud::dragon_breath` and `Breath` (fireball or a dragon's flame): the `AreaEffectCloud` defaults (wait 20 ticks, reapply after 20, no shrinking on use) with the breath colour. The fireball's cloud: Harming II, radius 3 growing to 7 over its 600 ticks.
  - `minecraft_terrain/src/server.rs` (End region): `tick_dragon_fireballs` before the dragons (a fireball goes when its dragon is gone or its chunk is not loaded, like `AbstractHurtingProjectile.tick`); the dragons tick with the line of sight through `MobWorld`; their fireballs are spawned; a destroyed crystal without a player behind it names the attackable player nearest to it (`CRYSTAL_DESTROY_TARGETING`: none in peaceful). `EntitySnapshot.dragon_fireballs` and `breath_bursts`. `mesh.rs` loads the fireball texture.
  - `minecraft_survival/src/entities.rs`: draws the fireballs and a burst of breath-coloured spell particles where one bursts (four lines).
- **Checked with javap (26.3):** `DragonStrafePlayerPhase` (`doServerTick`, `findNewTarget`, `setTarget`, `navigateToNextPathNode`, `begin`), `DragonHoldingPatternPhase` (`findNewTarget`, `strafePlayer`, `onCrystalDestroyed`, `NEW_TARGET_TARGETING`), `EnderDragon` (`aiStep`, `findClosestNode` both forms, `findPath`, `reconstructPath`, `onCrystalDestroyed`, `CRYSTAL_DESTROY_TARGETING` range 64, `canAttack`), `EnderDragonPhase` (ids 0 to 10), `EnderDragonPhaseManager.setPhase`, `DragonFireball` (`onHit`, the cloud), `AreaEffectCloud` (constructor defaults), `TargetingConditions.test`, `Entity.calculateViewVector`, `LevelEventHandler` events 1017 and 2006, `DragonFireballRenderer`, `DragonBreathParticle` (its colours).
- **Tests:** `ender_dragon`: `the_view_vector_and_strafe_lift_match_vanilla`, `a_path_can_end_at_a_node_of_its_own`, `a_circling_dragon_at_the_end_of_its_path_strafes_the_nearest_attackable_player` (never a creative one), `a_strafing_dragon_spits_a_fireball_after_five_ticks_with_the_player_in_sight_ahead` (where it starts, where it aims, the sound), `without_sight_or_out_of_range_or_facing_away_the_charge_runs_down`, `a_strafe_without_its_player_falls_back_to_circling` (also a saved strafe), `breaking_a_crystal_makes_a_circling_dragon_strafe_the_player_behind_it`. `dragon_fireball`: `a_dragon_fireball_flies_like_a_hurting_projectile_from_its_aim`, `the_breath_cloud_lands_on_a_living_entity_within_four_blocks`, `the_fireball_cloud_harms_for_6_and_grows_from_3_to_7_over_600_ticks`. `server`: `a_strafing_dragon_s_fireball_bursts_into_a_breath_cloud_that_harms_the_player` (an End level of seed 0: the shot and burst sounds, the cloud on the player, Harming II for 6 after 20 ticks and again 20 later, a fireball goes with its dragon); `the_fight_dragon_spawns_over_the_island_and_circles_it_while_the_arena_is_loaded` now also hears the strafe's sounds.
- **Decisions:**
  - **The landing roll comes with entry 39:** until then a circling dragon at the end of a path only rolls for the strafe.
  - **Players by id:** the strafed player is looked up by id among the End level's players each tick; one who left the level ends the strafe (vanilla keeps the stale entity and flies at its last position).
  - **A lost crystal turns the dragon on its next tick** (vanilla in the hit's tick); the head damage still comes at once.
  - **What a fireball hits:** players and mobs, not end crystals or the dragon's own parts (vanilla can burst one on a crystal it passes).
  - **Not saved:** fireballs and breath clouds are not saved with their chunk (the lingering potion clouds here are not either).
  - **Drawn:** the clouds with the spell particles in the dragon breath colour (#CB00E5, the middle of `DragonBreathParticle`'s range), the burst as the potion splash burst in that colour (100 particles, vanilla 200); the fireball's breath trail is not drawn.
  - **Death message:** breath harm is magic damage, "was killed by magic" (vanilla: "was killed by Ender Dragon using magic"), as a cloud's harm here can only name a player.
  - The shoot sound's pitch is drawn from the dragon's random (vanilla draws it on the client).
- **Test steps** (survival; `/difficulty normal`):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x5strafe` (a new world), `IW4L_MINECRAFT_DIMENSION=the_end` and the kit below. Start on the obsidian platform and walk onto the main island toward the pillars.
  2. Watch the dragon: now and then it leaves its circle and swoops at you; when it faces you within about 64 blocks you hear a short roar and it spits a purple fireball, then circles again. **Screenshot 1** (a fireball in flight).
  3. Where it bursts (on you, or the ground near you) a purple cloud spreads with a burst of purple sparks and a hiss. Standing in it hurts 3 hearts about every second (after a second's delay); it slowly widens over 30 seconds and then goes. **Screenshot 2** (the cloud).
  4. Step out of it: no more damage.
  5. Shoot a pillar crystal while the dragon circles: it turns and swoops at you at once.
  6. Type `/gamemode creative`: it never swoops at you or spits at you. `/gamemode survival` afterwards.
  7. Remove the `IW4L_MINECRAFT_DIMENSION` line afterwards.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:cobblestone*64,minecraft:water_bucket,minecraft:diamond_sword+sharpness=5
```

### 39. Minecraft ender dragon perches on the portal and breathes fire (E3-16)

Previous commit: f3608d26

- **Changed:**
  - `minecraft_terrain/src/ender_dragon.rs`, all eleven `EnderDragonPhase`s now (saved by their vanilla ids 0 to 10; `isSitting` for the three sitting phases and hovering; each phase's state lives on the dragon like vanilla's per-phase instances, reset by `begin`):
    - **The landing roll:** the holding pattern at the end of a path first rolls `nextInt(crystals + 3) == 0` for `LandingApproach`, then the strafe roll of entry 38.
    - `LandingApproach` (`DragonLandingApproachPhase`): a path from the closest node to the node closest to 40 blocks out on the far side of the island from the attackable player nearest the podium (height 105), or to (40, podium height, 0) with nobody near, ending at the podium's `MOTION_BLOCKING_NO_LEAVES` top; once the path is flown it turns to `Landing`.
    - `Landing` (`DragonLandingPhase`): flies at fly speed 1.5 with the turn speed `min(s, 40) / s` to the podium top's bottom centre; within one block it resets the flame count and sits scanning.
    - `SittingScanning`: the nearest attackable player within 20 blocks (times 0.8 while crouching, `getVisibilityPercent`), within 10 blocks up or down and in sight; after 25 ticks with one it turns to `SittingAttacking`, before that it turns its head and body toward them (`yRotA` toward the head-to-player bearing, at most 100 degrees). With nobody for 100 ticks it takes off, and charges the nearest player in sight within 150 blocks if there is one.
    - `SittingAttacking`: the roar, 40 ticks, then `SittingFlaming`.
    - `SittingFlaming`: on its tenth tick a breath cloud (`AreaEffectCloud`: Harming I, radius 5, 200 ticks, waits 20) on the ground 2.5 blocks beyond the head, found by stepping down from the head until a block is not air; after 200 ticks it scans again, or takes off after its fourth flame. Leaving the phase puts its flame out (`end()` discards the cloud).
    - `Takeoff`: a path to the ring node closest to 40 blocks along the head's look (`getHeadLookVector`: pitched down by `6 / max(distance / 4, 1) · 7.5` degrees at the podium), and the holding pattern once it is 10 blocks from the podium.
    - `ChargingPlayer`: flies at fly speed 3 at where the player stood; once within 10 blocks it turns back to circling 10 ticks later (`timeSinceCharge`).
    - A sitting dragon that takes more than 50 now takes off (`TAKEOFF`) instead of circling at once. `head_look_vector`, `DragonEnv.empty`, `PlayerBody.crouching`, `DragonEvents.clouds`, `DragonView.phase`.
  - `minecraft_terrain/src/server.rs` (End region): the dragons tick with the air test of the level; their flame clouds join the level's clouds, and a dragon whose flame went out has it removed (`Breath::Flame` of that dragon). A player counts as crouching at an eye height from 1 to 1.5.
  - `minecraft_terrain/src/effect_cloud.rs`: clouds compare equal (for the dragon's events).
  - `minecraft_survival/src/entities.rs`: while the dragon sits attacking, the client plays `entity.ender_dragon.growl` every tick at volume 2.5 (`DragonSittingAttackingPhase.doClientTick`). `dragon_render.rs`: its test view has a phase.
- **Checked with javap (26.3):** `DragonHoldingPatternPhase.findNewTarget` (the `nextInt(i + 3)` landing roll first), `DragonLandingApproachPhase` (`doServerTick`, `findNewTarget`, `NEAR_EGG_TARGETING`), `DragonLandingPhase` (`doServerTick`, `getFlySpeed` 1.5, `getTurnSpeed`), `DragonTakeoffPhase` (`doServerTick`, `findNewTarget`, `navigateToNextPathNode`), `AbstractDragonSittingPhase` (`isSitting`, `onHurt`), `DragonSittingScanningPhase` (`doServerTick`, the 20-block and 10-high scan, `CHARGE_TARGETING` range 150), `DragonSittingAttackingPhase` (`doServerTick`, `doClientTick`), `DragonSittingFlamingPhase` (`doServerTick`, `begin`, `end`, `resetFlameCount`), `DragonChargePlayerPhase` (`doServerTick`, `getFlySpeed` 3, `setTarget`), `EnderDragon` (`hurt(part)`'s `TAKEOFF`, `getHeadLookVector`), `TargetingConditions.test` (range times `getVisibilityPercent`, line of sight through `Sensing`), `LivingEntity.getVisibilityPercent` (0.8 while discrete).
- **Tests:** `ender_dragon`: `the_new_phases_keep_vanilla_speeds_and_sitting`, `a_circling_dragon_at_the_end_of_its_path_sometimes_turns_to_land`, `a_landing_dragon_flies_in_perches_on_the_podium_and_takes_off_with_nobody_near` (approach, landing within a block of the podium top, 99 ticks of scanning, takeoff, circling 10 blocks out), `a_perched_dragon_turns_to_a_player_roars_and_breathes_flame_four_times` (scan 25 ticks, roar 40, four flames of 200 ticks each put out when they stop, then takeoff; the flame on its tenth tick on the ground, Harming I for 3 at half strength), `a_perched_dragon_with_nobody_near_charges_a_player_it_sees_within_150` (or takes off when it sees nobody; a crouching player 17 blocks away is not seen), `a_charging_dragon_turns_back_ten_ticks_after_it_reaches_its_target`, `hurting_a_flaming_dragon_past_50_puts_its_flame_out_and_it_takes_off` (and the head look vectors); the phase id test now checks all eleven ids, and `a_sitting_dragon_takes_off_after_a_quarter_of_its_health` expects `Takeoff`. `server`: `a_landing_dragon_perches_on_the_podium_breathes_flame_and_the_flame_goes_out_when_it_takes_off` (an End level of seed 0 with the podium built: approach, landing on the pillar top, scanning a player beside the podium, the roar, the flame cloud in front of the head; two head shots of 30 make it take off and the flame goes); `the_fight_dragon_spawns_over_the_island_and_circles_it_while_the_arena_is_loaded` follows the targets of every flying phase and loads the dragon back in the phase it was saved in.
- **Decisions:**
  - **One tick, two phase ticks:** as vanilla's `aiStep`, a phase that changes runs the new phase's tick in the same tick, so the roar lasts 40 ticks after the switch and a flame 199.
  - **The sitting roar** is a growl started on every client tick of `SittingAttacking`, as vanilla does; the growls overlap into one long roar.
  - **Arrows:** a sitting dragon's immunity to arrows and wind charges (`AbstractDragonSittingPhase.onHurt`) has nothing to act on yet, as no arrow hits the dragon here; MW2 bullets and grenades hurt a sitting dragon like a flying one.
  - **Crouching** is read from the eye height (1.27 while crouching); invisibility does not shrink the scan (no invisibility effect on the player's body here).
  - **Not drawn:** the breath particles streaming from the mouth while landing and flaming (`doClientTick`'s dragon breath particles); the flame cloud itself is drawn like the fireball's (entry 38).
  - **A flame put out by a hit** (takeoff on damage) goes at the next server tick; vanilla discards it at once.
- **Test steps** (survival; `/difficulty normal`; MW2 weapons):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x5perch` (a new world), `IW4L_MINECRAFT_DIMENSION=the_end` and the kit below. Start on the obsidian platform.
  2. Shoot all ten pillar crystals (the caged ones too), so the dragon lands more often (a landing roll of one in 3 at each path's end). Walk to the island centre and stand on the podium's rim about 5 blocks from the bedrock pillar.
  3. Wait: the dragon leaves its circle, swings round to approach from across the island, and lands on top of the bedrock pillar. **Screenshot 1** (perched).
  4. It turns its head to you, roars for two seconds, then a purple cloud of breath spreads on the ground in front of its head for 10 seconds; standing in it costs 1.5 hearts a second. **Screenshot 2** (the breath on the podium). Step out of it.
  5. Stay near: it roars and breathes three more times, then takes off and circles again.
  6. On its next landing, shoot its head while it sits: after about 50 damage (a few rifle bursts) it takes off at once, and its breath cloud vanishes.
  7. On another landing, walk more than 20 blocks away (or 10 blocks above or below it): after 5 seconds it takes off and charges straight at where you stand, then circles again. **Screenshot 3** (the charge).
  8. Remove the `IW4L_MINECRAFT_DIMENSION` line afterwards.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:cobblestone*64,minecraft:water_bucket,minecraft:diamond_sword+sharpness=5
```

### 40. The track END log sums up group X5-phases

Previous commit: 011bc34c

- **Commits:** f3608d26 (E3-15, strafing and fireballs with their breath clouds), 011bc34c (E3-16, the landing, the sitting phases with the flame on the podium, takeoff and the charge), and this entry.
- **Tests** (`test -p <crate> --locked`, `MINECRAFTOSS_ROOT` set by `cargo-slot.ps1`): `minecraft_terrain` 761 (4 ignored) and `minecraft_survival` 333 with the vanilla world test pass; `check -p launcher` is green.
- **Clippy:** `clippy -p minecraft_terrain -p minecraft_survival --all-targets --no-deps --locked -- -A clippy::approx_constant` reports nothing in `ender_dragon.rs`, `dragon_fireball.rs` or `effect_cloud.rs`; each location it reports in `server.rs`, `entities.rs` and `mesh.rs` blames to 88cb6d83 or earlier. No fix needed.
- **New files** (`rustfmt --check` on the staged content is empty): `minecraft_terrain/src/dragon_fireball.rs` (edition 2021). `ender_dragon.rs` still passes.
- **Plan decision 9 kept:** the strafe, the landing approach, the scan and the charge pick from the players of the End level each tick, by id; the breath clouds are level state; nothing uses `PLAYER = 0`.
- **Not done in this group** (the other X5 groups): the wing knockback and `checkWalls` block breaking (E3-17), the knockback on the dragon from players' hits, the egg teleport (E3-20) and bottling dragon's breath (the clouds carry `Breath` for it). Also not drawn: the dragon breath particles of the mouth and the fireball's trail.
- **Owner test for the group** (survival, `/difficulty normal`, in one new End world `x5` with `IW4L_MINECRAFT_DIMENSION=the_end`, removed afterwards; the kit of entry 39):
  1. Entry 38 steps 2 to 6: strafes, fireballs and their clouds; screenshots 1 and 2.
  2. Entry 39 steps 2 to 7: shoot the crystals, the perch, the roar, the flame, the takeoff on damage, the charge; screenshots 3 to 5 (entry 39's 1 to 3).

## Fourth run, group X5-body-egg: the dragon's body, the egg and its breath

E3-17 and E3-20 of the brief, with bottling dragon's breath. Base: 38da4a26 (group X5-phases).

### 41. Minecraft ender dragon's wings throw players and mobs (E3-17)

Previous commit: 38da4a26

- **Changed:**
  - `minecraft_terrain/src/ender_dragon.rs`: `EnderDragon.aiStep`'s contact pass while the dragon was not hurt recently. After the body and wings take their new places, each wing's box grown by 4, 2, 4 and moved 2 down (`knockBack`) pushes every attackable player and every mob in it by `wing_push`: `(dx / d · 4, 0.2, dz / d · 4)` away from the body's centre, with `d` the squared horizontal distance but at least 0.1 (`Entity.push`), and hurts it for 5 (`mobAttack`) unless the dragon sits. The head and neck (their boxes grown by 1, from where they were before this tick, as before) now also bite mobs for 10. `PlayerBody.bounds`, `DragonEnv.mobs` (each mob's box), `DragonEvents.player_pushes` and `mob_hits` (push, then damage, in vanilla's order), `knockback` (`LivingEntity.knockback(0.4, ...)`: half the old motion less 0.4 toward the attacker, and up by 0.4 off the ground).
  - `minecraft_terrain/src/server.rs` (End region): the dragons see the mobs' boxes (`thrown::targets`). A player's push goes out with the other pushes (`players_pushed`, which MW2 movement takes through `push_velocity` as for a wind charge); a mob's push adds to its motion, then its damage lands as a hurt without an attacker's blame, and a full hit knocks it back from the dragon.
- **Checked with javap (26.3):** `EnderDragon` (`aiStep`: the wing boxes `inflate(4, 2, 4).move(0, -2, 0)` after `tickPart` of the body and wings, the head and neck boxes `inflate(1)` before theirs, `wasHurtRecently`, `EntitySelector.NO_CREATIVE_OR_SPECTATOR`; `knockBack`: the body box's centre, `max(dx² + dz², 0.1)`, `push(dx / d · 4, 0.2, dz / d · 4)`, 5 damage unless sitting and unless the target was hurt by a mob in the last 2 ticks; `hurt`: 10 damage), `LivingEntity.knockback`.
- **Tests:** `ender_dragon`: `the_wing_push_and_the_knockback_match_vanilla` (hand-worked pushes at 2 blocks, at 0.1 blocks and at (3, 4); the knockback on the ground and in the air), `the_wings_throw_players_and_mobs_and_hurt_them_only_in_flight` (a player beside the right wing is pushed out and hurt for 5, a creative one is left alone, a mob beside the left wing is pushed the other way and hurt for 5, a mob at the head is bitten for 10, nothing while the dragon was hurt recently, a sitting dragon pushes without harm). `server`: `the_dragon_s_wings_throw_a_player_and_an_enderman_and_hurt_them` (an End level of seed 0: the player's push and the 5 damage, an enderman down to 35 health and thrown away from the dragon).
- **Decisions:**
  - **The wing's 5 damage** lands on a player whenever the client's hurt cooldown lets it; vanilla also skips it within 2 ticks of the player's last hurt by a mob (`getLastHurtByMobTimestamp`), which the hurt cooldown covers in all but rare cases (the dragon's own hits are forgotten by the server's fight memory at once, as the dragon is not one of its mobs).
  - **A player's push is not halved** by the hit's knockback (vanilla's knockback halves the motion when the 5 damage lands): no mob hit knocks the MW2 player back here, so the push alone throws them, at its full vanilla strength.
  - **Mobs:** the dragon's damage reaches a mob as a hurt without blame (the shared blast hurt with no push), so armour counts as for `mobAttack` and an enderman does not teleport, as for a hit with a living attacker.
- **Test steps** (survival; `/difficulty normal`; MW2 weapons):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x5wings` (a new world), `IW4L_MINECRAFT_DIMENSION=the_end` and the kit below. Start on the obsidian platform, walk onto the main island and build a cobblestone tower about 15 blocks high under the dragon's circle (about 30 to 60 blocks from the centre).
  2. Stand on top and wait for the dragon to fly past close: when a wing sweeps over or beside you, you are flung far away from its body and lose 2.5 hearts (less with the armour on). **Screenshot 1** (in the air, thrown).
  3. Let it come at you head first (a strafe or a charge): its head bites for 5 hearts.
  4. Watch an enderman near its flight path: it is flung away the same way.
  5. Shoot all crystals, wait for it to perch on the podium, and walk up beside a wing: you are pushed off, but take no damage from the wing while it sits.
  6. Remove the `IW4L_MINECRAFT_DIMENSION` line afterwards.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:cobblestone*128,minecraft:water_bucket,minecraft:diamond_sword+sharpness=5
```

### 42. Minecraft ender dragon breaks blocks it flies through (E3-17)

Previous commit: 4cb7fa52

- **Changed:**
  - `minecraft_terrain/src/ender_dragon.rs`: `checkWalls` at the end of `aiStep`, on the head, neck and body boxes where they now are (the order of vanilla, each box checked even after a hit). Each block cell the box touches (`floor` of its corners, inclusive) is left alone when it is air or `#dragon_transparent` (light, fire); a `#dragon_immune` block, or any block while `mob_griefing` is off, holds the dragon; any other block is removed (`removeBlock`, no drops), unless removing it changes nothing (a water or lava block, which becomes itself). A box that removed something sends level event 2008 (an explosion particle) at a cell drawn from the dragon's random within the box. The dragon is `inWall` when a box was held: it flaps at half speed and flies at 0.8 of its step, as the flight code already did with the flag. `Wall` (`Pass`, `Immune`, `Breaks`, `Stays`), `DragonEnv.wall` and `griefing`, `DragonEvents.broken` and `wall_bursts`. A block one box removed counts as gone for the next box of the same tick, as in vanilla. The lines entry 41 added are now as `rustfmt` lays them out.
  - `minecraft_terrain/src/server.rs` (End region): the dragons see each block through the vanilla tags `#dragon_immune` and `#dragon_transparent` (read from the data at runtime) and the level's `mob_griefing` rule; the blocks a dragon broke are removed from the level at once (`Level::remove_block`), so the client gets them as block changes.
- **Checked with javap (26.3):** `EnderDragon.checkWalls` (the cell loop, `isAir`, `DRAGON_TRANSPARENT`, `MOB_GRIEFING`, `DRAGON_IMMUNE`, `removeBlock(pos, false)`, the random cell for level event 2008), `EnderDragon.aiStep` (`inWall = checkWalls(head) | checkWalls(neck) | checkWalls(body)` after the parts tick), `LevelEventHandler` (2008: `EXPLOSION` at the cell's centre), the tags `dragon_immune` and `dragon_transparent` of the 26.3 data.
- **Tests:** `ender_dragon`: `the_dragon_breaks_what_its_head_neck_and_body_fly_through_but_not_immune_blocks` (every cell of the three boxes once, head first, a burst inside each box, not held; a single block in the body box alone; immune blocks, or griefing off, hold it without breaking; water neither breaks nor holds; a held dragon flies a shorter step). `server`: `the_dragon_breaks_cobblestone_it_flies_through_but_not_end_stone` (an End level of seed 0 filled with cobblestone around the dragon: more than 50 blocks broken, all within its head, neck and body, sent as air; filled with end stone: nothing changes and the dragon is held).
- **Decisions:**
  - **No drops:** vanilla's `removeBlock` drops nothing, so nothing drops here either (the brief's "drops vanilla gives" are none).
  - **Not drawn:** the explosion particle of level event 2008 (no explosion particle is drawn in the Minecraft world yet, not even for TNT); the dragon still draws its cell from its random (`DragonEvents.wall_bursts`), but the server does not send it to the clients yet.
  - **Unloaded cells** count as air (vanilla would load the chunk; the dragon only flies over the loaded arena).
- **Test steps** (survival; `/difficulty normal`):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x5walls` (a new world), `IW4L_MINECRAFT_DIMENSION=the_end` and the kit below. Start on the obsidian platform and walk onto the main island.
  2. Under the dragon's circle (about 40 blocks from the centre), build a wall of cobblestone and oak planks about 6 wide and 20 high, the top at about 80 to 90 (watch the height the dragon passes at). Next to it, build the same wall from end stone and obsidian.
  3. Wait for the dragon to pass through the cobblestone wall: it tears a dragon-shaped hole in it without dropping anything. **Screenshot 1** (the hole).
  4. When it passes through the end stone wall: nothing breaks, and it slows down while inside.
  5. Type `/gamerule mob_griefing false` and repair the cobblestone wall: the dragon now passes through it without breaking it, slowed down. `/gamerule mob_griefing true` afterwards.
  6. Remove the `IW4L_MINECRAFT_DIMENSION` line afterwards.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*256,minecraft:oak_planks*128,minecraft:end_stone*128,minecraft:obsidian*64,minecraft:golden_apple*16,minecraft:diamond_chestplate,minecraft:diamond_boots+feather_falling=4,minecraft:diamond_pickaxe+efficiency=5
```

### 43. Minecraft dragon eggs teleport when clicked or hit (E3-20)

Previous commit: 81dec3e3

- **Changed:**
  - New `minecraft_terrain/src/dragon_egg.rs`: `DragonEggBlock.teleport`. Up to 1000 tries (`TRIES`) at an offset of `nextInt(16) − nextInt(16)`, `nextInt(8) − nextInt(8)`, `nextInt(16) − nextInt(16)` from the egg (`offset`, the draws in vanilla's order), the first into air above a block that is not air, inside the build height and the world border (`teleport_target`, `within_border`). `teleport_particles` is level event 2015 on the client (`singleBlockTeleportationParticles`): 128 portal particles along the line between the two spots, each with a motion of up to 0.1 a tick. `is_egg`.
  - `minecraft_terrain/src/server.rs`: `ServerSim::teleport_egg` runs for an attack (`attack`) and a use (`useWithoutItem`) on a dragon egg, with the level's random: the egg is set at its new spot (flags 2, `update::CLIENTS`) and removed from the old one (`removeBlock`); an egg that finds no spot stays, and the click is still used. The server's use and attack tables take the egg's states, so a client sends its clicks on an egg. `Output.egg_teleports` carries each jump (old and new spot) to the clients.
  - `minecraft_survival/src/entities.rs`: `Entities::attack_block`; each jump in an output emits the portal particles.
  - `render_anim/src/minecraft_world.rs` (one filter): a shot or hand-mining hit on a dragon egg is an attack on it instead of mining progress, so the egg jumps away rather than cracking (vanilla's `startDestroyBlock` calls `attack` first, and the egg is gone from the spot being mined). Right-clicking an egg already went through `use_block`.
- **Checked with javap (26.3):** `DragonEggBlock` (`useWithoutItem`, `attack`, `teleport`: 1000 tries, the six draws, `isAir` at the spot and not below, `WorldBorder.isWithinBounds`, `isInsideBuildHeight`, level event 2015 with `packDifferenceInPosition(.., 16, 8, 16)`, `setBlock(.., 2)`, `removeBlock(.., false)`), `LevelEventHandler` (2015: `unpackDifferenceInPosition`, `singleBlockTeleportationParticles`: 128 `PORTAL` particles, `lerp(t, new, old)`, motion `(nextFloat − 0.5) · 0.2`).
- **Tests:** `dragon_egg`: `only_the_dragon_egg_teleports`, `an_offset_draws_two_horizontal_two_vertical_and_two_horizontal_ints` (the bounds in order, offsets within 15, 7 and 15), `the_egg_lands_on_the_first_free_spot_drawn_or_stays_after_a_thousand_tries` (an egg in a full box stays put after 6000 draws; the border), `the_portal_particles_stream_between_the_two_spots` (128, within the box between the spots, the first particle hand-worked from the same random). `server`: `a_dragon_egg_jumps_to_a_free_spot_nearby_when_clicked_or_hit` (an End level of seed 0: an attack and then a use each move the egg within 15, 7 and 15 onto a solid block, leave air behind, send both changes and one jump; air is no egg).
- **Decisions:**
  - **What counts as a hit:** a MW2 bullet or a hand-mining tick on the egg is an attack (vanilla's left click); every such hit makes it jump again, as holding the button on a new spot starts a new attack in vanilla. A creative player's instant break of the egg with a Minecraft item still breaks it, as vanilla's creative left click does.
  - **No sound:** vanilla's teleport plays none, only the particles.
- **Test steps** (survival):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x5egg` (a new world) and the kit below. On flat ground, place the dragon egg.
  2. Right-click it: it vanishes and reappears up to 15 blocks away on the ground, with a stream of purple portal particles between the two spots. **Screenshot 1** (the particles).
  3. With the egg selected or any Minecraft item in hand, hold left-click on it: it jumps away again instead of cracking.
  4. Switch to an MW2 weapon and shoot it: it jumps away at each hit, without cracks.
  5. In creative (`/gamemode creative`), left-click it with a Minecraft item: it breaks instead. `/gamemode survival` afterwards.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:dragon_egg,minecraft:cobblestone*64,minecraft:torch*16
```

### 44. Minecraft glass bottles catch the dragon's breath

Previous commit: 0733e118

- **Changed:**
  - `minecraft_terrain/src/effect_cloud.rs`: `bottle_breath` is `BottleItem.use`'s first half: the first live dragon's cloud (one with `Breath`, as vanilla's owner is an `EnderDragon`: the fireball's and the perched flame's) whose box meets the player's box grown by 2 (`bottle_reach`) loses half a block of radius (`setRadius`, clamped at 0). `CloudView.breath` and `breath_near` let the client see a breath cloud in reach.
  - `minecraft_terrain/src/bottle.rs`: `bottle_use` takes whether a breath cloud is in reach: a glass bottle not used on a cauldron or mud (a block's `useItemOn` comes first) fills with dragon's breath before any water (`BottleUse::Breath`); `breath_use` is the item use: one `minecraft:dragon_breath` in place of the bottle and `item.bottle.fill_dragonbreath` at the player (volume 1, pitch 1). `minecraft_terrain/src/bucket.rs`: `Change::Breath`.
  - `minecraft_terrain/src/server.rs`: `Command::BottleBreath` with the player's feet and height; the server shrinks its own cloud and answers the item use (so a cloud gone in the meantime fills nothing).
  - `minecraft_survival/src/entities.rs`: `dragon_breath_near`, `bottle_breath` (the waiting item use, answered like an eye throw). `render_anim/src/minecraft_world.rs` (three lines): the bottle check passes the breath test, and a breath use goes to the server and fills the bottle when it answers.
- **Checked with javap (26.3):** `BottleItem.use` (the `AreaEffectCloud` search in the player's box `inflate(2.0)`, `isAlive() && getOwner() instanceof EnderDragon`, the first found, `setRadius(getRadius() − 0.5)`, `BOTTLE_FILL_DRAGONBREATH` at the player, volume 1, pitch 1, `turnBottleIntoItem` with `Items.DRAGON_BREATH`, then the water ray only when no cloud was found), `AreaEffectCloud.setRadius` (clamped to 0 to 32).
- **Tests:** `effect_cloud`: `a_glass_bottle_takes_half_a_block_from_a_dragon_s_breath_within_two_blocks` (the reach box, a potion's cloud is passed over, the shrink, a cloud shrunk out of reach, the radius stops at 0, a dead cloud fills nothing). `bottle`: `a_glass_bottle_in_reach_of_dragon_s_breath_fills_with_it_before_water` (breath before a water source, the result and sound, a water bottle drinks, a cauldron still comes first); the other bottle tests pass `false`. `server`: `a_glass_bottle_near_the_dragon_s_breath_fills_and_shrinks_the_cloud` (a fireball's cloud: 3 to 2.5, none out of reach or 4.5 blocks above).
- **Decisions:**
  - **The client asks first:** the client offers a breath use only when its last snapshot shows a breath cloud in reach and no other item use is waiting; the server decides with its own clouds.
  - **A cloud shrunk to nothing** stays until its time is up, as in vanilla (a cloud without a radius change per tick, like the perched flame, is only removed by its duration).
  - **Creative:** the bottle becomes the breath as a water bottle does here (vanilla's creative keeps the empty bottle).
- **Test steps** (survival; `/difficulty normal`; MW2 weapons):
  1. In the `.env`, set `IW4L_MINECRAFT_WORLD=x5breath` (a new world), `IW4L_MINECRAFT_DIMENSION=the_end` and the kit below. Walk onto the main island.
  2. Wait for a purple fireball to burst near you (entry 38), or for the perched dragon's flame on the podium (entry 39).
  3. Stand next to the cloud's edge (within 2 blocks), select the glass bottles and right-click: you get a bottle of dragon's breath with a bubbly fill sound, and the cloud gets smaller. **Screenshot 1** (the bottles in the hotbar, the smaller cloud).
  4. Right-click a few more times: each bottle shrinks it by half a block; once it is too far, nothing happens.
  5. Away from any cloud, pour the water bucket and right-click the water with a glass bottle: a water bottle, as before.
  6. Remove the `IW4L_MINECRAFT_DIMENSION` line afterwards.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:glass_bottle*16,minecraft:water_bucket,minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:cobblestone*64
```
