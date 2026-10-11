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

## The whole track's owner test, one kit

Steps in order: group 1 steps 1-4, group 2 steps 1-4, group 3a steps 1-3, group 3b steps 1-5. Groups 1 and 2 need no items; 3a needs the water bucket and the ender pearls; 3b the cobblestone, hay bales and helmet.

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:hay_block*16,minecraft:iron_helmet,minecraft:water_bucket,minecraft:ender_pearl*16
```
