# Track D log: skating in Minecraft worlds

Branch `track-d`, worktree `C:\Dev\mc-wt\track-d`, target dir `D:\iw4l-wt\track-d`. Brief: `d-skate.md` (section 4 is the plan, section 5 the owner's steps, section 6 the decisions). Base: `080b5c1e` (track A merged).

A commit can't name its own hash, so each entry is written with its commit and names the commit before it; the last entry's hash is in `git log` under its subject.

## Decisions for the whole track

- **Guns while skating** (brief section 6, kept as decided): on the board, guns and Minecraft items are put away. No firing, aiming, grenades, melee, mining, placing, eating or blocking, and neither the gun nor a held Minecraft item is drawn in the skater's hands. Pick the board up to fight; it takes one key press. Skate's controls use both sticks and both triggers, and the skate camera is third person with nothing to aim with. MW2 already zeroes every button while skating (`net/src/client/runtime.rs`); group 3b does the same on the Minecraft side.
- **Per player:** the switch (`GameSettings.skating`) is client-local like the minimap. `SkateMode` and the skate worker stay per viewer (one local session per process). Nothing here is world state or player `0` state.

## Group 1. Skating has an on/off switch in the Minecraft controls, saved, and stays off until turned on

Commit: the one after `080b5c1e`.

- **Changed:**
  - `frame/src/settings.rs`: `GameSettings.skating`, default `false`.
  - `console/src/user_settings.rs`: `serialize_settings` writes `skating=<bool>` after `show_blood`, `parse_settings` reads it, and the menu dvar `ui_skating` sets it and is mirrored every frame (like `ui_mc_minimap`). No header bump for the new key: an older build only logs "ignored unknown setting".
  - `ui/menus/settings.json`: the Minecraft controls page (`pc_options_minecraft`) gets a "Skating" choice row (Disabled/Enabled, dvar `ui_skating`) at y 148, under the six keys.
  - `render_anim/src/skate.rs`:
    - Reads `GameSettings`. The animation bank preload no longer runs at `Startup`; it runs from `update` the first frame the switch is on (`Host.preloaded_banks`), so it waits for the loaded settings.
    - While the switch is off no map session is built, `preload_pending` stays false on a new map, and a toggle (J, L3+R3 or the console `skate`) only sets the notice "Skating is off (Options > Controls > Minecraft)".
    - With the switch on and no `IW4L_SKATE_ASSETS`, a toggle sets "Skate 3 data is not set up: restart to select your default.xex".
    - Notices clear themselves 3 s after they are set (`Host.notice_at`); worker and preload errors keep today's behaviour (they stay) and cancel a pending notice clear.
    - Turning the switch off while riding calls `stop` and then drops the map session (`send`, `receive`, `clip`, `ready`, `preloaded`, `preload_pending`), so both worker threads (`iw4l-skate`, `iw4l-skate-blocks`) end. The log says "Skating turned off: map session dropped; decoded animation banks stay cached until exit": the banks `Session::preload` decoded stay in the engine's cache until the process ends. Turning it back on builds a new session on the next frame.
  - `launcher/src/first_run.rs`: `prepare` reads the settings file (`IW4L_SETTINGS_PATH` from the process environment, then from `.env`, else `iw4l-artifacts\settings.cfg` beside the exe, since `.env` is not loaded yet when `prepare` runs). When it says `skating=true` and the Skate 3 data is not ready, the Skate 3 question is asked even with `IW4L_SKATE=off`, and the question adds "To stop this question, set Skating to Disabled in Options > Controls > Minecraft." A finished conversion removes the stale `IW4L_SKATE=off` line (`EnvFile::remove`). With the switch off, `IW4L_SKATE=off` still skips the question as before.
  - `docs/MINECRAFT-SURVIVAL.md`: a "Skating" bullet after the Options line.
- **Tests:**
  - `console`: `skating_is_off_until_chosen_and_survives_a_restart` (default off, `skating=false`/`true` lines, a v4 file without the key loads as off) and `the_skating_menu_row_switches_the_setting_and_mirrors_it`.
  - `ui`: `the_minecraft_page_offers_a_skating_switch` (item type 12, Disabled/Enabled, y 148, one "Skating" label on its line).
  - `launcher`: `skating_wanted_reads_only_a_true_skating_line` (true, false, missing, commented out, `skating=true` inside a bind line, a later `skating=false` wins as in `parse_settings`) and `a_finished_conversion_drops_the_declined_line`.
  - `render_anim`: `a_toggle_is_refused_while_skating_is_off_or_its_data_is_missing`.
- **Decisions:**
  - `skating_wanted` takes the last `skating=` line, exactly as `parse_settings` does (later assignments win), and compares the key without trimming it, like `parse_settings`.
  - `debug_move.rs` needed no change in this group: the overlay already shows "Skate unavailable: {status}" while nothing is preloaded, which is how the notices appear.
  - A stale `IW4L_SKATE=off` is only removed after a successful conversion, as the brief says; when the data is already converted and the switch is on, the line is left alone (it then has no effect).
- **Owner's test steps** (brief section 5, steps 1, 2, 3 and 20; the key row "Skateboard Down/Up" and the v5 header come with group 2):
  1. Start the new build once. Options > Controls > Minecraft shows "Skating: Disabled" under the six keys (screenshot). In the world, J only shows "Skate unavailable: Skating is off (Options > Controls > Minecraft)" for about 3 seconds.
  2. Set Skating to Enabled and quit. `iw4l-artifacts\settings.cfg` holds `skating=true`.
  3. Start again. Although `.env` says `IW4L_SKATE=off`, the game asks "Do you have Skate 3?", with the extra line about the Skating switch. Answer Yes, pick `default.xex`, and wait for the converter. `.env` now has `IW4L_SKATE_ASSETS=".../skate-data/assets"` and no `IW4L_SKATE=off` line. (Without Skate 3: answer No; the game starts, J shows "Skate 3 data is not set up: restart to select your default.xex" for about 3 seconds, and the question comes again on the next launch until Skating is set to Disabled.)
  4. With the data converted, press J on flat ground: the board goes down. Then set Skating to Disabled while riding (Esc menu): the board is picked up and J only shows the "Skating is off" line again. `latest.log` has "Skating turned off: map session dropped" and no new "Skate map session preloaded" line until you enable it again.
- **Kit:** none needed.

## Group 2. The skateboard key is a rebindable Minecraft control, J by default

Commit: the one after `ac3479e4` (group 1).

- **Changed:**
  - `input_iw4/src/names.rs`: `"mc_skate"` appended after `mc_perspective` as command 84, so it sits in the Minecraft binding layer; `INPUT_COMMAND_NAMES` is now `[&str; 85]`. No other round 8 brief adds input commands; track E rebases onto id 84 if it adds any.
  - `frame/src/minecraft_ui.rs`: `McAction::Skate` (`"mc_skate"`), `ALL` has 7 entries (still fits the `u8` bit sets). No exhaustive `match` on `McAction` exists outside `command()`.
  - `console/assets/default_controls.cfg`: `bind J mc_skate`.
  - `console/src/binds.rs`:
    - `apply_minecraft_defaults` skips `mc_skate`; the new idempotent `KeyBinds::add_skate_key` binds J to it only when `mc_skate` has no key and J is free in both layers.
    - `KeyBinds::minecraft_buttons_for(id)` and `KeyBinds::skate_key_label()` (the bound keys in capitals, "J OR K" for two, "L3+R3" when only the stick click is left).
    - The pure `skate_toggle_pressed(binds, mc_keys, minecraft_map, just_pressed)`: on the Minecraft map (`MinecraftUi.active`) it is `mc_keys.just_pressed(McAction::Skate)`; on MW2 maps, and on the Minecraft map while dead (the Minecraft layer is off then), any button bound to `mc_skate` in the Minecraft layer that was just pressed.
  - `console/src/user_settings.rs`: header **v5**. A v1-v4 file gets `add_skate_key` once; `apply_minecraft_defaults` stays for v1/v2 only, so keys a player unbound in v3/v4 stay unbound. In v5 an unbound skate key stays unbound.
  - `console/src/plugin/mod.rs`: the hard-coded J is gone. The toggle moved below `minecraft_presses` and fires on `skate_toggle_pressed` or L3+R3, with the same blocks as before (console, script menus, inventory, chat).
  - `console/src/debug_move.rs`: the skating overlay names the bound key (`skate_key_label`) instead of "J", and says "pick up" on the Minecraft map and "return to MW2" elsewhere. The console `skate` usage line no longer says "J".
  - `ui/menus/settings.json`: "Skateboard Down/Up" key row (item type 14, dvar `mc_skate`) at y 168, under the Skating switch. `sync_binding_view` already publishes `ui_bind_mc_skate`.
  - `docs/MINECRAFT-SURVIVAL.md`: the key list, the controls page (seven keys) and the Skating bullet (key, v5 rule).
- **Tests:**
  - `input_iw4`: `mc_skate_is_command_84`; `minecraft_commands_follow_the_mw2_ones` and `minecraft_commands_do_nothing_to_mw2_input` now cover 78-84.
  - `frame`: `every_minecraft_action_has_its_own_command` checks `mc_skate`.
  - `console` (`binds.rs`): `the_defaults_bind_the_vanilla_minecraft_keys_beside_the_mw2_ones` expects J too (7 keys), `only_just_pressed_keys_fire_their_action` sees 7 suppressed keys, `the_skate_key_works_on_mw2_maps_through_its_minecraft_binding`, `the_overlay_names_the_bound_skate_key`, `the_skate_key_is_added_only_on_a_free_j`.
  - `console` (`user_settings.rs`): `older_settings_get_the_vanilla_minecraft_keys` (7 keys), `current_settings_keep_their_minecraft_keys_unbound` (v3/v4 get J only, v5 nothing), the saved-header expectations are v5, `the_controls_menu_shows_the_minecraft_keys` shows `ui_bind_mc_skate` = "j", and new `older_settings_get_the_skate_key_on_a_free_j`, `a_j_bound_in_the_mw2_layer_keeps_the_skate_key_unbound`, `an_unbound_skate_key_stays_unbound_in_v5`, `a_rebound_skate_key_survives_a_restart`.
  - `ui`: `the_minecraft_page_lists_the_seven_minecraft_keys` (lines 28-128 and 168).
- **Decisions:**
  - The skate key on an MW2 map fires both its Minecraft action and any MW2 binding the same key has; `add_skate_key` never puts it on a key the MW2 layer uses, so this only happens when the player binds it that way.
  - On the Minecraft map, a key bound to `mc_skate` takes the key from its MW2 binding like every Minecraft key (`minecraft_presses` suppresses it).
  - With no key bound, the overlay names "L3+R3", which still toggles on a controller.
  - The `skate_toggle_pressed` helper lives in `binds.rs` beside `minecraft_presses`, so its test sits with the other key-layer tests.
- **Owner's test steps** (brief section 5, steps 1, 2, 4 and 19):
  1. Start the new build once. Options > Controls > Minecraft shows "Skating: Disabled" and, under it, "Skateboard Down/Up: J" (screenshot).
  2. Set Skating to Enabled and quit. `iw4l-artifacts\settings.cfg` starts with `// IW4L user settings v5` and holds `skating=true` and `bind j mc_skate`.
  3. With the Skate 3 data converted (group 1 step 3), press J on flat ground in the Overworld: the board goes down and the overlay ends in "J: pick up". J again picks it up. On an MW2 map the overlay ends in "J: return to MW2".
  4. Options > Controls > Minecraft: rebind "Skateboard Down/Up" to K and restart. K puts the board down and J does nothing; the overlay names K. Set it back to J.
- **Kit:** none needed.

## Group 3a. The skateboard is picked up by teleports, deep water, flying, gliding, mounts and spectator mode

Commit: the one after `1daef351` (group 2).

- **Changed:**
  - `frame/src/minecraft_ui.rs`: `MinecraftUi.board_allowed` (per viewer, computed from that viewer's player).
  - `render_anim/src/minecraft_world.rs`: the pure `board_allowed(alive, spectator, pm_flags, eye_in_water)` sets it every frame next to `ui.active`: alive, not in spectator mode, neither `MC_FLYING` nor `MC_GLIDING`, and the eyes not under water (`water.eye`, vanilla's `isEyeInFluid(WATER)` dismount).
  - `render_anim/src/skate.rs`:
    - On the Minecraft map (`MinecraftUi.active`), the skate key does nothing while `board_allowed` is false (no notice, as vanilla does nothing either), and a board that is down, or going down, is picked up as soon as it turns false. `minecraft_world::update` runs after `skate::update`, so skating sees the previous frame's value; one frame late is fine. MW2 maps are unchanged.
    - Teleports: at the top of `update`, before the worker's replies are drained, the `eflags::TELEPORT` bit of the local player in `AuthorityWorld` is compared with the bit kept in `Host.teleport_bit` (refreshed every frame, also while not skating). A flip while riding, entering or activating calls `stop`, whose epoch bump makes the queued pose stale, so the old place is never written back over the new one. `/tp`, ender pearls, chorus fruit, end gateways and dimension changes all go through `SimWorld::teleport`, so the player stands at the new place with the board picked up. Respawn was already covered by the `alive` check.
  - `docs/MINECRAFT-SURVIVAL.md`: the Skating bullet says when the board can't go down and what picks it up.
- **Tests:**
  - `render_anim`: `the_board_is_refused_flying_gliding_dead_spectating_or_under_water` (`minecraft_world.rs`) and `a_teleport_flip_picks_the_board_up` (`skate.rs`, the pure `teleported(last, now)`, false on the first frame).
- **Decisions:**
  - Mounts and sleeping: track B (strider riding with `pm_flags::MC_RIDING`, and sleeping) is not merged into this track's base, so `board_allowed` has no "riding" or "asleep" condition yet. Whichever of B and D merges second adds them to `board_allowed` (one more flag test and one more argument).
  - A refused board makes no notice: in vanilla a blocked mount just does nothing, and the brief's step 21 expects "nothing happens".
  - Gliding is the `MC_GLIDING` flag alone (the elytra code also checks it may glide, but the flag is only set while gliding is allowed).
- **Owner's test steps** (brief section 5, steps 15, 18 and 21):
  1. In creative, dig a 3 by 3 pool 2 blocks deep and fill every block of it with the water bucket, then `/gamemode survival`. Put the board down and ride into it: the board is picked up once your eyes go under, and you swim.
  2. On the board, open chat and run `/tp ~ ~ ~20`: you arrive standing, with the board picked up. Throw an ender pearl high into the air and press J while it flies: you land standing where it hits.
  3. `/gamemode creative`, double-tap Space to fly and press J: nothing happens. `/gamemode spectator` and J: nothing. `/gamemode survival`.
- **Kit:** `minecraft:water_bucket`, `minecraft:ender_pearl*16`.

## Group 3b. On the board, items are put away and the player rides like a vehicle

Commit: the one after `ffe7dd74` (group 3a).

- **Changed:**
  - `frame/src/skate.rs`: `SkateMode.airborne` (per viewer).
  - `render_anim/src/skate.rs`: `present` sets `airborne` from the pure `airborne(state)`: true for `PhysicsAir`, `KnownAir`, `PhysicsAirSecondary` and `BipedAir` (the engine's `PhysicalStateId` names); grinds, wipeouts and ground states are not airborne. `stop` clears it.
  - `render_anim/src/minecraft_world.rs`, while `skate.active`:
    - `on_ground` comes from the pure `skate_on_ground(skating, airborne, ground_entity)` = not airborne, instead of the ground entity PMove left frozen when the board went down. The fall peak then builds in the air and the existing landing code deals the damage (hay, water, slime, feather falling and the wind charge grace as before); `crouching` is false on the board (the crouch bit PMove left frozen is ignored), so a slime block still cancels the fall damage.
    - No footsteps: the step sound block is skipped.
    - The mouse buttons and the pad triggers are not read for Minecraft actions (`use_held`, `attack_held`, both just-pressed clicks, the offhand key): no mining, punching, placing, eating, blocking, bow drawing or offhand use. Eating and blocking in progress stop; a charged trident is dropped without being thrown or launching a riptide (`trident::released` sees `alive && !skate.active`).
    - The local `BodyGear` goes through the pure `board_gear`, which clears `main` and `off` and keeps the armour, so no Minecraft item is drawn in the skater's hands. The MW2 gun was already left off the skating body (`remote_body.rs`, not edited).
    - `Motion.riding = skate.active`.
  - `minecraft_survival/src/survival.rs`: `Motion.riding`. `Survival::tick` skips the walk, sprint, swim and jump exhaustion while it is set (vanilla `ServerPlayer.checkMovementStatistics` returns for a passenger before the swim branch). Air, drowning, effects, regeneration and damage are unchanged.
  - `docs/MINECRAFT-SURVIVAL.md`: the Skating bullet says how the rider is treated.
- **Tests:**
  - `render_anim`: `airborne_states_are_the_air_states` (`skate.rs`); `a_landing_skater_takes_vanilla_fall_damage` (`skate_on_ground`, and `Survival::fall(8.0, ..)` deals 5) and `a_skater_holds_no_minecraft_item` (`board_gear`: main and offhand cleared, armour and `hides_gun` kept, unchanged off the board) in `minecraft_world.rs`.
  - `minecraft_survival`: `a_rider_spends_no_food_on_movement` (sprinting, jumping and swimming with `riding` set) and `a_rider_still_loses_air_under_water`.
- **Decisions:**
  - Guns while skating stay as the brief decided (see the top of this log): MW2 already zeroes every button on the board (`net/src/client/runtime.rs`), and now the Minecraft side ignores the mouse buttons and triggers too.
  - A trident being charged when the board goes down is dropped, not thrown: in vanilla a released charge throws, but here the release is caused by putting the board down, not by letting go of the button.
  - The other `on_ground` readers (the digger speed, critical hits, trident riptide, thrown items) see the board's grounding too; they can't act on the board anyway.
  - The sprint and jump bits stay frozen while skating; `riding` makes them cost nothing, which fixes the frozen sprint bit the brief noted (a player who pressed J while sprinting used to pay sprint exhaustion for the whole ride).
- **Owner's test steps** (brief section 5, steps 4, 5, 14, 16 and 17):
  1. In the Overworld, on flat ground, hold a cobblestone in the hotbar and press J: the board goes down and neither the gun nor the block is in the skater's hands (screenshot; armour you wear still shows). J again: you stand where the skater was, with the same hotbar slot. Do the same with a gun selected: the same gun is raised after pick-up.
  2. On the board, look at a block and pull RT and LT (or click both mouse buttons): no block is mined or placed and nothing is shot.
  3. Build a tower 8 blocks high on flat stone ground, with a 6-block runway on top and no kicker. Roll off the edge without ollieing, so the drop is exactly 8 blocks: you lose 2.5 hearts (5 points). Dig out the stone where you landed and set hay bales into the holes, flush with the ground, and roll off again: half a heart (1 point). An ollie or a kicker adds height and more damage.
  4. Note the hunger bar, then ride around for a minute (no eating): it doesn't go down from riding. Walk and sprint the same minute: it does.
  5. On the board, press E: the inventory opens and the skater coasts. Close it and press Q: the selected item drops. Rolling makes no footstep sounds.
- **Kit:** `minecraft:cobblestone*64` (twice, for the tower), `minecraft:hay_block*16`, `minecraft:iron_helmet`.

## Group 4. Stairs, slabs, carpets and snow layers become ramps for the skateboard

Commit: the one after `006d0a4c` (group 3b).

- **Changed:**
  - `sim/src/voxel.rs`:
    - The pure `riser_fillets(shape_at, lo, hi)` (block space) finds every riser of at most half a block between two walkable surfaces and builds a 45-degree wedge in front of it, whose run equals its height: a slope quad from the face's top edge down to the floor, plus the two side triangles, all wound counterclockwise from outside.
    - A face gets a wedge only when:
      - its top is uncovered just inside the face;
      - the floor 1/64 outside, at a quarter and three quarters of its width, is the same and 0 to 0.5 blocks below its top;
      - the floor is still the same at half and 0.95 of the run;
      - no collision box reaches into the wedge's box (this catches a fence post between the samples).
    - Faces are cut at the block's middle, so the uncovered half of a stair's slab side gets its own wedge, whichever way the stair's boxes are split. Wedges on the same plane that touch are merged, so a row of slabs or a wide staircase has one long ramp per side and no inner walls.
    - `skate_ramps(centre, radius, depth)` runs it under the `WORLD` read lock and maps it to map units, as `collision_triangles` does.
  - `render_anim/src/skate.rs`: `block_collision` appends `skate_ramps` to the block triangles before `rails::find`, and logs the triangle count and build time at debug ("Skate block ramps: N triangles in M ms").
  - `render_anim/src/skate/rails.rs`: the lip probe now looks out horizontally (`with_z(0.)`). It used to look out along the face. On a sloped walkable face that pointed down at the slope's foot, so the drop probe started under the floor and every ramp's toe on the ground counted as a lip, which included the new wedges. Flat faces and the side edges of slopes probe exactly as before.
  - `docs/MINECRAFT-SURVIVAL.md`: the Skating bullet describes the ramps.
- **Tests:**
  - `sim` (`voxel.rs`, on a hand-built `shape_at` with a ground of full blocks):
    - `a_bottom_stair_on_the_ground_is_a_45_degree_ramp_from_half_a_block_in_front` (both fillets on `y = z + 0.5`, from z -0.5 to 0.5, total slope area sqrt 2)
    - `a_staircase_of_stairs_is_one_plane` (three stairs on cobblestone columns: one plane from z -0.5 to 2.5, area 3 sqrt 2)
    - `a_slab_on_the_ground_gets_a_half_block_toe` (one wedge per side, plus a full block behind a slab: toe, flat slab top, toe)
    - `a_full_block_and_a_fence_get_no_fillet`
    - `a_carpet_gets_a_one_sixteenth_fillet` (and none between carpets in a row)
    - `one_snow_layer_gets_none_and_three_get_a_quarter_block_one` (layer 1 is the zero-height box the catalog gives, layer 3 is 4/16)
    - `a_fillet_is_never_built_into_another_block` (a stair facing a wall, a slab with a block in front, and a slab with a fence post in front)
    - `fillet_triangles_face_outward` (every triangle faces away from its wedge's inside, none faces down)
  - `render_anim` (`skate.rs`): `a_slab_on_the_ground_is_no_longer_a_lip`. `rails::find` over a slab and the ground finds lips without the fillets and none with them, and a full block keeps its lips.
- **Decisions:**
  - `riser_fillets` is `pub` (the brief had a private `fn`), so the rails test in `render_anim` can build a slab's fillets without touching the global block world.
  - The lip probe fix in `rails.rs` also applies to MW2 maps. There it only stops counting the foot of a ramp as a lip, and an edge whose ground is less than 3 units lower: the probe now matches its documented intent ("the ground must drop at least 3 units just past the edge"). Ledges and kickers with a real drop are found as before.
  - The fillets of a stair are its slab front and its step front. The front half of a bottom stair's slab side also gets a small sideways wedge, because it is a half-block riser too. The step's sides stay vertical, so the hubba edge of group 5a keeps its drop.
  - A half-block slot between a wall and a stair step (a stair placed against a wall) gets a wedge from each side; they cross in the slot. That is harmless and rare, so it is left.
  - Build time: in an unoptimised test build, a whole 81 x 81 x 41 region took 81 ms for flat terrain, and 262 ms for a pathological one with a slab on every other block and stairs in between (52,332 wedge triangles). The release build is many times faster, so the 24-nearest-blocks cap from the brief's risks was not needed. The debug log line reports the real time.
  - Brief step 8's cobblestone-slab-cobblestone staircase rises half a block per block, so every riser gets a fillet, and the slab tops stay flat between them.
- **Owner's test steps** (brief section 5, steps 6 to 10):
  1. On flat ground, build a 5-block-high staircase of oak stairs, each stair one block higher than the last and standing on a cobblestone column, all facing the same way. Push into it from the ground: the board rolls up without a bump at the bottom and rides down again (screenshot halfway, wheels on the step edges).
  2. Place a single oak stair on flat ground and ride over it fast: you get air off it like a kicker.
  3. Ride up a stone slab placed on the ground. Then build a staircase that rises half a block per block (cobblestone slab, cobblestone, cobblestone with a slab on top, and so on) and ride up it.
  4. Ride across a row of white carpet, then a patch of snow three layers deep (right-click snow onto the same spot three times; a single layer has no collision in vanilla either): no bumps.
  5. A one-block cobblestone ledge stops the board (you have to ollie). Grind along its edge (screenshot). A slab on the ground is now a small ramp, not a ledge: its edge no longer grinds.
  6. Optional: add `IW4L_LOG_LEVEL=debug` to the `.env` next to the exe and skate near a village or a snowy forest. Every skate collision rebuild writes "Skate block ramps: N triangles in M ms" to the newest log in `iw4l-artifacts\logs`. Send the largest M, then remove the line again.
- **Kit:** `minecraft:cobblestone*64` (twice), `minecraft:oak_stairs*64`, `minecraft:stone_slab*64`, `minecraft:cobblestone_slab*64`, `minecraft:white_carpet*64`, `minecraft:snow*64`.

## Fix. Skate grind rails no longer run across the middle of sloped faces

Commit: the one after `d602c978` (group 4). Found while building group 5a.

- **Changed:** `render_anim/src/skate/rails.rs`. Group 4 made the lip probe look out horizontally. On an edge inside a sloped face (the diagonal of a ramp's quad, or of a group 4 wedge), when the probe looked out on the uphill side the face rose more than one unit within the probe's two units, so the drop probe started under the face, missed it and saw a drop: the diagonal became a grind rail across the ramp. Whether it did depended on which of the two triangles came first. The drop probe now starts above the face's own continuation (`rise`, the face's climb per unit along the probe) and reaches as far below the edge as before, so a real lip still needs the same 3-unit drop. Toes, flat edges and the side edges of slopes probe exactly as before.
- **Tests:** `render_anim`: `the_middle_of_a_sloped_face_is_no_lip` (`rails.rs`, ramps rising half a block, one block and two blocks over two blocks of width: no rail between the ground and the top edge).
- **Decisions:** the fix also applies to MW2 maps, where it only removes rails across the middle of steep sloped faces (the same defect group 4 introduced there).
- **Owner's test steps:** covered by group 5a step 4 (the stair ramp has rails only on its two sides).
- **Kit:** none beyond group 5a.

## Group 5a. Fences, walls, iron bars and the sides of stair ramps are grind rails

Commit: the one after `1d739aac` (the lip probe fix).

- **Changed:**
  - `render_anim/src/skate/rails.rs`:
    - `find` takes `Limits { max_slope, merge_ridges }`. `Limits::MW2` (max slope 0.7, no ridge merge) keeps MW2 maps as they were; `Limits::BLOCKS` (max slope 1.01, ridge merge) is for the block world.
    - Ridge merge: the lips are merged along their lines per facing side first. Then two parallel lips at the same height (within half a unit), at most 18 map units apart (18.25, so a wall post's exactly 18 passes), facing away from each other and overlapping by at least one unit become one rail on their centre line over the overlap. The narrowest pairs go first, and a stretch of a lip is used once. Whatever is left of a lip outside its pairs stays a lip. The centre rails and the leftovers are merged along their lines again.
    - A rail with a segment steeper than the MW2 limit is dropped unless it rises more than half a block (18.5 units), so a slab's toe corners (half a block high) are not rails while a single stair's sides (one block) and a staircase's are.
  - `render_anim/src/skate.rs`: `block_collision` uses `Limits::BLOCKS`. `render_anim/src/skate/collision.rs`: MW2 clip collision uses `Limits::MW2`.
  - `docs/MINECRAFT-SURVIVAL.md`: the Skating bullet describes the new rails.
- **Tests:**
  - `render_anim` (`rails.rs`): `two_close_parallel_lips_become_one_centre_rail` (tops 4.5, 9, 13.5 and 18 units wide: one centre rail with block limits, two with MW2 limits), `a_wall_post_and_its_sides_make_one_centre_rail`, `a_one_block_wall_keeps_two_rails`, `lips_facing_each_other_across_a_gap_stay_apart`, `mw2_limits_drop_a_45_degree_edge_and_block_limits_keep_it`, `a_45_degree_edge_rising_half_a_block_is_no_rail`.
  - `render_anim` (`skate.rs`, on real stair boxes with their group 4 ramps): `the_sides_of_a_stair_ramp_grind_like_a_hubba` (a three-stair staircase in all four facings: no sloped rail with MW2 limits, exactly two with block limits, each on one side of the ramp and as long as it) and `a_fence_line_is_one_rail_down_its_middle` (six fence blocks: one rail at 1.5 blocks over the fence's centre line).
- **Decisions:**
  - Two lips only merge when they face away from each other (each has the other on its inner side). The brief's rule (parallel, same height, within half a block, overlapping) would also merge two ledges facing each other across a half-block gap, for example a block and a stair step with the slab between them, into a rail floating over the gap.
  - The sloped-rail minimum (more than half a block of rise) is new. Without it, the side edges of every group 4 toe (a slab on the ground, the sideways wedge beside a stair's slab) became 25-unit rails at the slab's corners, against group 4's "a slab on the ground no longer grinds".
  - Fences and walls grind at their collision top, 1.5 blocks, as the brief decided; iron bars and glass panes at 1 block.
- **Owner's test steps** (brief section 5, steps 10, 11 and 13):
  1. A one-block cobblestone ledge: grind along its edge (screenshot). A one-block-wide cobblestone wall two blocks high (a row of cobblestone): you can grind either edge, not the middle.
  2. Build a 10-block oak fence line with an oak stair in front of it as a kicker. Launch off the stair, land on the fence and grind along its middle (screenshot). Repeat with a cobblestone wall line (with a post at each end) and an iron bar line.
  3. Ride the side edge of the 5-step stair ramp from group 4 step 1: it grinds like a hubba, on both sides. Riding up the middle of the ramp never snaps you into a grind.
  4. Ride over the corners of a stone slab on the ground: no grind starts there.
- **Kit:** `minecraft:cobblestone*64` (twice), `minecraft:oak_stairs*64`, `minecraft:oak_fence*64`, `minecraft:cobblestone_wall*64`, `minecraft:iron_bars*64`, `minecraft:stone_slab*64`.

## Group 5b. Minecraft rails are grind rails for the skateboard

Commit: the one after `af560fec` (group 5a).

- **Changed:**
  - `sim/src/voxel.rs`:
    - `RAIL_SHAPE = 0x7800` in the per-shape flags: values 1-10 for the ten vanilla `RailShape`s in their enum order (`north_south`, `east_west`, the four `ascending_*`, then `south_east`, `south_west`, `north_west`, `north_east`), 0 for none. The pure `rail_flags(block, shape)` gives them for `minecraft:rail`, `powered_rail`, `detector_rail` and `activator_rail`, and 0 for anything else.
    - `free_spawn_spot` masks the rail bits out of its "wet" test, so a respawn spot on a rail is not refused. The other flag readers only test their own bits.
    - `track_rails(centre, radius, depth)` and the pure `rail_polylines(flags_at, lo, hi)`: per rail block a polyline 2/16 above its floor: straight across the block centre; ascending from the low edge at 2/16 to the high edge at 1 + 2/16; a curve as a quarter arc of radius 0.5 around its corner, between the two edge midpoints, in 4 segments. Polylines whose ends meet (within 1/32 block) are joined into one.
  - `render_anim/src/minecraft_world.rs` (`shape_id`): the four rail blocks OR their `rail_flags` into the shape's flags. A shape with flags and no boxes already gets its own id, so each rail shape is its own shape id.
  - `render_anim/src/skate.rs`: `block_collision` adds `track_rails` to the rails found on the block faces, and the debug line counts them.
  - `docs/MINECRAFT-SURVIVAL.md`: the Skating bullet mentions track rails.
- **Tests:**
  - `sim` (`voxel.rs`): `track_rails_follow_straight_ascending_and_curved_shapes` (all ten shapes: ends, heights, arc radius), `three_straight_rails_chain_into_one` (three straight rails; a straight, an ascending, a straight, a curve and a straight rail join into one line; side-by-side rails and rails a block apart in height stay apart), `rail_shapes_get_their_own_shape_ids` (ten distinct non-zero values inside `RAIL_SHAPE`, the same for all four rail blocks, 0 for other blocks and unknown shapes, no overlap with the other flags), `a_rail_does_not_make_a_respawn_spot_wet`.
- **Decisions:**
  - The chaining lives in `sim` (`chain_rails`), not in `render_anim/src/skate/rails.rs` as the brief suggested: `sim` can't depend on `render_anim`, and the brief's chaining tests sit in `voxel.rs`. It joins rails at any angle, not only up to the 35 degrees of the lip chaining, because only two rail blocks ever meet at a point and Minecraft itself connects them there; so a track runs as one grind over the 45-degree bend onto an ascending rail.
  - Track rails are added after `rails::find` and are not shortened, merged or filtered: a single rail block (36 units) is longer than the shortest lip rail anyway.
  - Track C takes `0x8000` in the same flags for powder snow; whichever of C and D merges second adds its bits beside the other's (the u16 is then full).
- **Owner's test steps** (brief section 5, step 12):
  1. Lay 10 straight rails on flat ground, then an ascending rail onto a cobblestone block, two more rails on top, and a curve. Ollie onto the track and grind it: up the slope and through the curve without dropping off (screenshot).
  2. Do the same with powered rails.
  3. Optional: with `IW4L_LOG_LEVEL=debug` in `.env`, the newest log in `iw4l-artifacts\logs` shows "Skate block rails: ... N track rails" with N above 0 near the track.
- **Kit:** `minecraft:rail*64`, `minecraft:powered_rail*16`, `minecraft:cobblestone*64`.

## Clippy. Track D rail code passes clippy without new warnings

Commit: the one after `09e57cea` (group 5b).

- **Changed:** `render_anim/src/skate/rails.rs`: `merge_ridges` groups the lips by side in a plain `HashMap<[i32; 2], Vec<lip>>` (clippy's `type_complexity` flagged the tuple map from group 5a). No behaviour change. The remaining `type_complexity` warnings in `rails.rs` and the ones in `skate.rs` are upstream's.
- **Tests:** none new; `render_anim` passes (91).
- **Owner's test steps:** none. **Kit:** none.

## Group 6. The skateboard stops at chunks that have not loaded yet

Commit: the one after `095c7a40` (the clippy cleanup).

- **Changed:**
  - `sim/src/voxel.rs`:
    - The pure `chunk_walls(loaded, lo, hi)` (block space): for every loaded chunk column next to one that is not loaded, a vertical quad on their shared edge. It spans the region's full height and faces into the loaded chunk, wound counterclockwise from that side like `collision_triangles`. Only edges inside the region get a wall, cut to the region's width along the edge, so a newly loaded chunk always reaches the region (`changed_near`) and the skate collision is rebuilt without the wall.
    - `missing_chunk_walls(centre, radius, depth)` runs it on the live block world under the `WORLD` read lock and maps it to map units.
  - `render_anim/src/skate.rs`: `block_collision` adds the walls after `rails::find` has run on the blocks and ramps, so the lip probe never sees them, and logs "Skate chunk walls: N triangles at chunks not loaded yet" at debug when there are any.
  - `docs/MINECRAFT-SURVIVAL.md`: the Skating bullet says the board stops at chunks that have not loaded yet.
- **Tests:** `sim` (`voxel.rs`): `walls_stand_only_between_loaded_and_missing_chunks` (three loaded chunks in an L: eight walls, all vertical, full region height, each facing into a loaded chunk with a missing one behind it, none between the loaded chunks; a region cutting one chunk gets only the walls inside it, cut to its width) and `a_fully_loaded_region_has_no_walls` (everything loaded, loaded neighbours with the missing chunks beyond the region, and nothing loaded).
- **Decisions:**
  - The walls keep the skater in step with a walking player, whom the fork already holds still in a chunk that has not loaded (`awaiting_ground`, `sim/src/step.rs`). Chunks stream 8 chunks around the player and the skate region is 40 blocks, so a wall only shows when the skater outruns the streaming, for example in fresh terrain.
  - The walls are one-sided, facing the loaded side the skater rides in, as the brief says. A region with no loaded chunk at all already gets no collision ("no blocks around the skater yet").
- **Owner's test steps** (new; not in the brief's section 5):
  1. Optional: add `IW4L_LOG_LEVEL=debug` to the `.env` next to the exe. Far from spawn, in land you have never visited, put the board down and push in a straight line at full speed for a minute. If the world can't generate as fast as you ride, the board bumps to a stop at an invisible wall at a chunk edge instead of falling into the void, and rolls on once the land appears. The newest log in `iw4l-artifacts\logs` then has "Skate chunk walls: N triangles at chunks not loaded yet". If the land always keeps up, no wall ever shows, which is also right. Remove the log line again afterwards.
- **Kit:** none.

## Group 7. Keyboard and mouse can skate when no controller is connected

Commit: the one after `37cc3663` (group 6).

- **Changed:**
  - `frame/src/skate.rs`: `SkateMode.keyboard` (per viewer), true while the skate input comes from the keyboard and mouse.
  - `render_anim/src/skate.rs`:
    - With no active controller, `update` builds the skate input from the keyboard and mouse through the pure `keyboard_frame(keys: KeySample, mouse, &mut FlickState, dt) -> PadState` (XInput layout; `PadState::frame` makes the `InputFrame`). `key_sample` reads the physical keys (`KeyCode`) and mouse buttons; the mouse motion is bevy's `AccumulatedMouseMotion`.
      - W/A/S/D: the left stick, full deflection (opposite keys cancel).
      - Mouse motion: the right stick, 600 per pixel, clamped to full deflection. The stick always eases back to centre at a full deflection per 0.1 s, and a move against the current deflection starts from centre, so a quick down-then-up flick snaps through while a slow drift stays inside the engine's 25% dead zone. Mouse down pulls the stick down.
      - Space A (`0x1000`), Left Ctrl B (`0x2000`), Left Shift X (`0x4000`), Tab Y (`0x8000`), Z LB (`0x0100`), V RB (`0x0200`); left mouse RT and right mouse LT (255 while held). Start, Back, the d-pad and the stick clicks are never sent, so Esc stays the MW2 pause menu.
    - An open Minecraft screen or chat, and an unfocused window, send a centred frame from a connected controller (`InputFrame::from_pad` with zeros) instead of `InputFrame::neutral()`, which reported every controller as disconnected, and reset the flick. The console and script menus still suspend the input through `mode.input_blocked`. A connected controller is used exactly as before.
    - `mode.keyboard` is set from the source; `mode.controller` is `Some(0)` for a keyboard frame too.
  - `console/src/debug_move.rs`: the overlay reads "SKATE | Keyboard: WASD steer, mouse flick, Space push | <key>: pick up" (or "return to MW2" on MW2 maps) while the keyboard skates.
  - `docs/MINECRAFT-SURVIVAL.md`: the Skating bullet lists the keyboard and mouse controls instead of "and a controller".
- **Tests:** `render_anim` (`skate.rs`): `w_pushes_the_left_stick_forward` (W, S, W+S, W+D, A), `a_mouse_flick_reaches_the_right_stick_and_settles` (a 60-pixel move down is full down, back up is full up at once, a sideways nudge and the flick settle to centre 0.1 s after the mouse stops, a 1-pixel-per-frame drift at 144 fps stays under the dead zone), `space_presses_a_and_the_mouse_buttons_pull_the_triggers` (every key's bit, both triggers, and no Start, Back, d-pad or stick-click bit with every key held) and `no_key_gives_a_centred_frame` (all zero, and the frame is a connected controller with no buttons).
- **Decisions:**
  - Button meanings were checked in the engine (`skate-core/src/input/gameplay_map.rs`, `controller.rs`, `riding_intentions.rs`, `offboard_intentions.rs`): A is the right-foot push (`RightPush`), X the left-foot push (`LeftPush`), B the brake, RB grabs the world (`GrabWorld`), and **Y toggles the off-board state** (`ToggleOffBoardState`, steps off the board), as the brief expected. So Y goes on Tab, the least reachable key, as the brief's rule says, and R sends nothing. The brief's Tab-to-Back is dropped: the engine's gameplay map never reads Back (or Start), so it did nothing.
  - Push stays on Space. The triggers (grabs and crouch) are the mouse buttons as the brief says: left mouse is RT, right mouse is LT.
  - The keys are physical positions: on the owner's German keyboard, "Z" is the key labelled Y. They are fixed this round (rebindable skate controls are out of scope). The mouse flick ignores "Invert mouse": down is always down, like the pad's stick.
  - The flick constants (600 per pixel, 0.1 s) are the brief's starting point. Reversal from centre and a constant settle rate were added so the result barely depends on the frame rate and a slow mouse drift does not hold the stick over.
  - A controller always wins when one is the active pad; there is no mixing of pad and keyboard in one frame.
  - `update` takes one more system parameter (a tuple behind the `KeyboardInput` type alias, which keeps clippy's `type_complexity` quiet). Its `too_many_arguments` warning was already there (12, now 13).
- **Owner's test steps** (brief section 5, step 22, spelled out):
  1. Unplug or switch off the controller. On flat ground in the Overworld press J: the board goes down and the overlay reads "SKATE | Keyboard: WASD steer, mouse flick, Space push | J: pick up" (screenshot).
  2. Tap Space to push, hold Left Ctrl to brake, steer with A and D.
  3. While rolling, move the mouse down quickly and straight back up: the skater ollies. Try it off a single oak stair as a kicker.
  4. In the air, hold the left or right mouse button: the skater grabs the board.
  5. Press Tab: the skater steps off the board and walks with W/A/S/D; Tab again gets back on. J still picks the board up.
  6. Press E: the inventory opens and the skater coasts; the keys and the mouse don't move it until it is closed. Esc opens the pause menu.
  7. Alt-Tab out of the game while rolling: the skater coasts and doesn't keep turning.
  8. Plug the controller back in: the overlay switches to "Original controller controls" and the pad steers again.
- **Kit:** `minecraft:oak_stairs*64` (already in the combined kit).

## Review fix 1. A skater who bails in the air takes the fall damage of the whole drop

Commit: the one after `d6553124` (group 7). Found by the track's adversarial review.

- **Defect:** group 3b counted every state but the four air states as standing on the ground, `WipeoutGround` included. The engine switches to `WipeoutGround` in the air: on a bail, and 0.05 s before a predicted impact (`skate-core` `select_physics_air`/`select_known_air`). So a bail in the air landed at once, at the height of the bail, and the rest of the drop was never counted: a bail high up dealt almost nothing, and a rough landing off an 8-block tower dealt 4 instead of 5.
- **Changed:** `render_anim/src/skate.rs`: `airborne(state, was_airborne, drop)`. A wipeout that starts in the air stays in the air while the skater's root still falls (more than 0.05 map units since the last pose); once it stops falling it has landed, and the existing landing code deals the damage of the whole drop. A wipeout that starts on the ground stays on the ground, so a ragdoll tumbling down a slope is no fall. `present` passes the root's drop since the previous pose.
- **Tests:** `render_anim`: `a_wipeout_in_the_air_falls_until_it_lands`; `airborne_states_are_the_air_states` uses the new arguments.
- **Decisions:** in a wipeout the engine puts the root 0.8 m under the ragdoll's centre of mass (`wipeout_state/skeleton.rs`), so a bail can count up to about half a block more than the drop; vanilla rounds the fall down, so an 8-block drop still deals 5. A bail that starts on the ground and then goes over an edge takes no fall damage (the ragdoll can't be told apart from one rolling down a slope).
- **Owner's test steps:**
  1. From the 8-block tower of group 3b step 3, ride off the edge badly (turn the board sideways or lean back hard) so the skater bails in the air: you lose 2.5 hearts (5 points), the same as a clean landing, not less.
- **Kit:** none beyond group 3b.

## Clippy 2. The launcher's skating check passes clippy without new warnings

Commit: the one after `a01029cf` (review fix 1). Found by the track's final clippy run on every touched crate (`console`, `frame`, `input_iw4`, `launcher`, `minecraft_survival`, `render_anim`, `sim`, `ui`).

- **Changed:** `launcher/src/first_run.rs`: `skating_wanted` takes the last `skating=` line with `next_back()` instead of `last()` (clippy's `double_ended_iterator_last`, from group 1). No behaviour change. Every other warning in the touched crates is on code the track did not add.
- **Tests:** none new; `launcher` passes.
- **Owner's test steps:** none. **Kit:** none.

## The whole track's owner test, one kit

Steps in order: group 1 steps 1-4, group 2 steps 1-4, group 3a steps 1-3, group 3b steps 1-5, group 4 steps 1-6, group 5a steps 1-4, group 5b steps 1-3, group 6 step 1, group 7 steps 1-8, review fix 1 step 1. Groups 1, 2 and 6 need no items, and group 7 only the oak stairs. 3a needs the water bucket and the ender pearls, 3b the cobblestone, hay bales and helmet, 4 the cobblestone, stairs, slabs, carpet and snow, 5a the fence, wall and iron bars, and 5b the rails.

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:hay_block*16,minecraft:iron_helmet,minecraft:water_bucket,minecraft:ender_pearl*16,minecraft:oak_stairs*64,minecraft:stone_slab*64,minecraft:cobblestone_slab*64,minecraft:white_carpet*64,minecraft:snow*64,minecraft:oak_fence*64,minecraft:cobblestone_wall*64,minecraft:iron_bars*64,minecraft:rail*64,minecraft:powered_rail*16
```
