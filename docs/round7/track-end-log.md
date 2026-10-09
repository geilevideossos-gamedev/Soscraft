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
