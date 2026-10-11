# Track D: skating in Minecraft worlds (design brief)

Paths and line numbers are as of e2313625. Vanilla facts were checked with javap on the 26.3 `client.jar`; no Mojang code is copied. Nothing under `skate/` (the vendored Skate 3 engine, excluded from the workspace) is edited by this track.

## 1. The owner's request

> Skating in Minecraft worlds. Upstream has a skate mode (`IW4L_SKATE` in the owner's `.env` next to the exe, currently off). The owner wants: an on/off switch in the settings menu, saved; while it's on, a free, rebindable key puts the board down and picks it up again in Minecraft worlds.

The round's scope adds: Minecraft terrain for the board (block collision, stairs and slabs as ramps, grinds on block edges and rails), and a stated decision on guns while skating.

**Before anything else, ask the owner two things** (both are hard requirements of the Skate 3 engine and nothing in this track removes them):
1. Do they have an **extracted** Skate 3 (Xbox 360) `default.xex` with its `data` folder beside it? An ISO does not work. Their `.env` has `IW4L_SKATE=off` and there is no `skate-data` folder in `C:\Games\Mashup\2010-Rust-Rewrite-Mashup`, so they declined (or skipped) the setup so far. The converter is already installed: `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\skate\iw4l-skate-convert.exe` (28 MB with `skate\licenses\`).
2. Do they have an Xbox/XInput controller? Today skating reads only a controller. Group 7 adds a keyboard and mouse fallback.

## 2. What exists today

Skating already works on Minecraft block collision (upstream commits 433909de, e0d5e10a, 9b7f3849, c36ef0a9; fork commits 31d642b4, 96c25c96). What it lacks is the switch, the rebindable key, ramps, Minecraft-aware rails and any Minecraft rules while riding.

| Area | State | Where |
|---|---|---|
| Setup | `first_run::prepare` runs before every launch. `IW4L_SKATE=off` in `.env` skips it; otherwise, without converted data, it asks "Do you have Skate 3?", picks `default.xex`, runs the converter into `<exe dir>\skate-data` and writes `IW4L_SKATE_ASSETS=".../skate-data/assets"`. A "No" writes `IW4L_SKATE=off`. | `launcher/src/first_run.rs:14`, `:31-54`, `:136-145` (`skate_ready`), `:148-195` (`convert_skate`), `launcher/src/main.rs:21` |
| What enables skating | Only the presence of `IW4L_SKATE_ASSETS` in the environment. There is no setting. | `render_anim/src/skate.rs:51`, `:125`, `:361`, `:365`; `assets/src/bot_model.rs:107` |
| Skate data needed | `private/skater.glb`, `private/game.json`, `private/stock/physics-skeletons.json`, `private/stock/skater-collections.json`, plus `board.json`/`rig.json` derived on first run | `first_run.rs:136-145`, `assets/src/skate_board.rs` |
| Startup preload | Decodes the animation banks on a thread at `Startup` whenever `IW4L_SKATE_ASSETS` is set | `skate.rs:50-72` |
| Per-map session | When a map's clip collision appears, one worker thread (32 MB stack) builds a `skate_host::bridge::Session` from the MW2 clip mesh, brushes and static models (`collision::extract`), then waits. One session per process (the local viewer). | `skate.rs:123-282`, `skate/collision.rs:22-131` |
| Minecraft collision | On activation and while riding, a second thread builds collision from `sim::voxel::collision_triangles(centre, 40, 20)`: box faces of every block within 40 blocks across and 20 up/down, faces against full blocks culled. Rebuilt when the skater moves 14 blocks from the centre or a block change reaches the region (`changed_near`, at most every 0.25 s). | `skate.rs:77-79`, `:84-119`, `:153-246`; `sim/src/voxel.rs:605-674`, `:77-102` |
| Stairs and slabs | Plain collision boxes: a stair is a half slab plus half-block cubes, a slab is half a block (46 cm at 36 map units per block, `voxel.rs:15`). The board cannot roll up a half-block riser; it is a curb to ollie. No ramps. | `voxel.rs:605-674` |
| Grinds | `rails::find` turns walkable-face edges with a drop of 3+ units beyond and nothing rising into rails, merged and chained; slopes over 0.7 rise per length are dropped (`MAX_SLOPE`). The same detector runs on block faces, so block edges and slab edges grind. Fences give two parallel lips 9 units apart. Minecraft track rails have no collision, so they are never rails. | `skate/rails.rs:17-33`, `:43-113`, `skate.rs:96` |
| Unloaded chunks | No collision there; a fast skater can roll off into the void | `voxel.rs:773-784` (`shape_at` returns nothing) |
| Toggle | Hard-coded **J**, or L3+R3 together, outside the console, script menus, inventory and chat. Console `skate [on|off|status]`. | `console/src/plugin/mod.rs:358-372`, `console/src/debug_move.rs:49`, `:130-138` |
| HUD | Overlay text says "J: return to MW2" and "Connect a controller" | `debug_move.rs:815-823` |
| Input | Controller only (`pad_frame`, XInput layout). No keyboard path: without a pad the skater gets `InputFrame::neutral()`. A Minecraft screen sends a neutral pad so the skater coasts. | `skate.rs:376-385`, `:507-545` |
| Player motion | On activation the local client gets `external_motion`; `sim` stops running PMove for it and the skater root is written into `ps.origin` with `set_origin` every pose (velocity zeroed). Host-only: it needs `net::AuthorityWorld`. | `skate.rs:302-324`, `:339-341`, `:414`; `sim/src/step.rs:300`; `sim/src/frame.rs:367-388` |
| Guns | `cmd.forwardmove`, `rightmove` and **all buttons** are zeroed while skating, so no fire, ADS, grenade or melee. The first-person view is replaced. The third-person body already drops its gun and attachments while skating (`job.gun = None; job.attachments.clear()`), and draws the board instead. What still shows in the skater's hands is a **held Minecraft item**: `BodyGear.main`/`off` are set for the local player every frame from the hotbar and drawn by `append_body_gear`. | `net/src/client/runtime.rs:1173-1175`; `render_anim/src/occupancy/fpv_present.rs:418`; `occupancy/remote_body.rs:906-913`, `:966-971`, `:1302-1303`; `minecraft_world.rs:2202-2206`; `minecraft_terrain/src/body_gear.rs:26-31` |
| Minecraft actions | Mining, attacking, eating and placing read the mouse buttons and the **pad triggers directly**, so skating with RT/LT (Skate grabs) mines and places | `render_anim/src/minecraft_world.rs:999`, `:1050`, `:1159` |
| Minecraft rules | Nothing knows about the board. `on_ground` comes from `ps.ground_entity_num`, frozen while PMove is off, so no fall damage ever while skating; footsteps play while rolling (`horizontal < 2.0`); `Motion.sprinting/jumping` come from frozen `pm_flags`, so a player who pressed J while sprinting pays sprint exhaustion for the whole ride (`survival.rs:331-335`). | `minecraft_world.rs:833`, `:839-906`, `:936-948` |
| Mounts | None today. Track B (`b3-striders.md`, group 7) adds strider riding with a new `pm_flags::MC_RIDING = 0x0100_0000` and asks that **the skate toggle is refused while riding**. Track B (`b4-sleeping.md`) asks that skating does not start while sleeping. | `docs/round8/b3-striders.md:283-284`, `:398`; `b4-sleeping.md:338` |
| Camera and culling | The Minecraft camera stands aside while skating; terrain culls from the skate camera at 120 degrees | `render_anim/src/minecraft_camera.rs:52`; `minecraft_world.rs:1640-1652` |
| Teleports | Not watched. A `/tp`, ender pearl, respawn or dimension switch leaves the skater where it was and the next pose writes the old place back. `teleport` (and `teleport_facing`, which calls it) flips `eflags::TELEPORT`; `set_origin` does not. **Order:** `skate::update` runs before `crate::sync_camera_from_presented` and `minecraft_world::update` after it, so a teleport lands after that frame's `present`, and the next frame's queued `Reply::Pose` would write the old place back before any check on the presented snapshot sees the flip. | `sim/src/frame.rs:390-415`; `minecraft_world.rs:1836`, `:1843`, `:2477`; `skate.rs:43-47`, `minecraft_world.rs:195-197` |
| Settings | `GameSettings` (`frame/src/settings.rs:27-59`), `settings.cfg` header v4, `key=value` lines then binds (`console/src/user_settings.rs:344-386` write, `:389-520` read), menu dvars `ui_*` (`:524-655`). Template: `minecraft_minimap` / `ui_mc_minimap` and `show_blood`. | as listed |
| Controls menu | Options > Controls > Minecraft (`pc_options_minecraft`, `ui/menus/settings.json:130-160`): six key rows (item_type 14) at y 28-128. Tests: `ui/src/menu.rs:197-204`, `:344-373`. | as listed |
| Minecraft key layer | Commands from id 78 (`input_iw4/src/names.rs:80-90`) live in `KeyBinds.minecraft`; `McAction` (`frame/src/minecraft_ui.rs:323-357`); `minecraft_presses` returns nothing outside Minecraft maps (`console/src/binds.rs:558-597`); defaults in `console/assets/default_controls.cfg:29-34`; `apply_minecraft_defaults` (`binds.rs:329-337`) fills unbound commands into free keys. | as listed |
| Release install | `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\.env`: `IW4L_GAMES`, `IW4L_SKATE=off`, `IW4L_MINECRAFT_WORLD`, `IW4L_MINECRAFT_KIT`. Settings at `iw4l-artifacts\settings.cfg` (v4). Converted data would land in `skate-data\assets`. | - |

## 3. Vanilla 26.3 behaviour (javap)

There is no skateboard in vanilla. The rules below are the closest vanilla analogues (riding a vehicle) and the exact block shapes the board meets. All shape numbers are in 1/16 block.

**Keys** (`net.minecraft.client.Options` constructor, USB HID codes): W 26, S 22, D 7, Space 44, LShift 225 (sneak), LCtrl 224 (sprint), E 8, F 9, Q 20, T 23, Tab 43, Slash 56, O 18 (`key.friends`), P 19 (`key.socialInteractions`), F2 59, F5 62, F11 68, L 15 (`key.advancements`), **G 10 (`key.quickActions`)**, F1 58, F4 61, digits 30-38, C 6 (save toolbar), X 27 (load toolbar). **J (HID 13) is unbound in vanilla**, so J stays the default skate key.

**Riding** (what the board copies):
- `ServerPlayer.checkMovementStatistics` returns at once while `isPassenger()`; `rideTick` calls `checkRidingStatistics`, which only awards `*_ONE_CM` stats. The `isPassenger()` return comes before the swimming branch, so **a rider spends no food on movement** (no 0.1/m sprint, no 0.01/m swim, no 0.05/0.2 jump exhaustion).
- A passenger does not run its own `Entity.move`, so it makes **no footstep sounds**.
- `LivingEntity.baseTick`: when the eye is in water (`isEyeInFluid(WATER)`, not a bubble column) and the vehicle's `dismountsUnderwater()` is true, the passenger `stopRiding()`. **The board is picked up when the eye goes under water.**
- `AbstractHorse.causeFallDamage` damages the horse and calls `propagateFallToPassengers`, so **a rider takes fall damage when the mount lands**.
- Fall damage: `LivingEntity.calculateFallPower` = `fallDistance + 1e-6 - SAFE_FALL_DISTANCE (3.0)`, times `FALL_DAMAGE_MULTIPLIER` (1.0), floored. An 8-block drop deals 5 (2.5 hearts). `HayBlock.fallOn` multiplies by 0.2. The fork already implements this (`minecraft_survival/src/survival.rs:402-415`).

**Block shapes the board meets**:
- `StairBlock` static init: `SHAPE_OUTER` = `column(16, 0, 8)` (a bottom slab) or `box(0, 8, 0, 8, 16, 8)` (one 8x8x8 cube); `SHAPE_STRAIGHT` = outer or outer turned 90 degrees (slab plus a half-block step); `SHAPE_INNER` = straight or straight turned 90 degrees; top-half stairs are the same flipped (`INVERT_Y`). Every stair is built from half-block cubes, so a straight bottom stair has two 8-pixel risers: slab front 0-8, step 8-16.
- `SlabBlock`: `SHAPE_BOTTOM` = `column(16, 0, 8)`, `SHAPE_TOP` = `column(16, 8, 16)`.
- `SnowLayerBlock.getCollisionShape` returns `SHAPES[layers - 1]`: collision is one layer lower than the outline, so **a single snow layer has no collision**, two layers collide at 2/16, and so on in 2/16 steps. `CarpetBlock`: `column(16, 0, 1)`. Dirt path and farmland: `column(16, 0, 15)`.
- `FenceBlock` → `CrossCollisionBlock(4, 16, 4, 16, 24)`: posts and arms 4 wide, **collision 24 (1.5 blocks)**. `IronBarsBlock` → `(2, 16, 2, 16, 16)`: 2 wide, collision 16. `WallBlock.makeShapes(24, 24)` for collision: post `column(8, 0, 24)`, sides 6 wide (5-11).
- `Blocks.RAIL`, `POWERED_RAIL`, `DETECTOR_RAIL`, `ACTIVATOR_RAIL`: `Properties.of().noCollision()`. `BaseRailBlock` outline: flat `column(16, 0, 2)`, ascending `column(16, 0, 8)`. Plain rails have 10 shapes (`north_south`, `east_west`, four `ascending_*`, four curves `south_east`, `south_west`, `north_west`, `north_east`); the other three have the 6 straight and ascending shapes.

## 4. The plan

Nine commits, in order (groups 3 and 5 are split in two, 3a/3b and 5a/5b, so no commit mixes unrelated rules; the numbers 1-7 are kept so the test steps still refer to them). Each one also updates the matching lines of `docs/MINECRAFT-SURVIVAL.md` (line 105 "Skate 3 works on block collision", line 109 "The Skate toggle is blocked while a screen is open"). `docs/SKATE.md` is upstream's and stays unedited.

### Group 1. "Skating has an on/off switch in the Minecraft controls, saved, and stays off until turned on"
Files: `frame/src/settings.rs`, `console/src/user_settings.rs`, `ui/menus/settings.json`, `ui/src/menu.rs` (tests), `render_anim/src/skate.rs`, `console/src/debug_move.rs`, `launcher/src/first_run.rs`.
- `GameSettings.skating: bool`, default **false**. `serialize_settings` writes `skating=<bool>` beside `minecraft_minimap`; `parse_settings` reads it. Menu dvar `ui_skating` in `native_menu_settings` (set and mirrored like `ui_mc_minimap`, `user_settings.rs:594`, `:653`). No header bump for a new key (upstream `iw4l.exe` only logs "ignored unknown setting").
- `pc_options_minecraft` gets a choice row (item_type 12, `["@MENU_DISABLED","0"],["@MENU_ENABLED","1"]`, copied from the `ui_mc_minimap` row) at y 148 with the label "Skating".
- `skate.rs`: read `Res<frame::GameSettings>`. The `Startup` preload (`:50-72`) moves into `update` behind a `Host.preloaded_banks` flag, so it waits for the loaded settings (`load_user_settings` runs at `Startup`, `console/src/plugin/mod.rs:100-103`). While `skating` is off: no map session is built (`:364-375`), the `!same_map` branch (`:356-362`) leaves `preload_pending` false, a toggle request is dropped with a notice, and turning it off while riding calls `stop` and then drops the session (`host.send = None`, `receive = None`, `clip = None`, `ready = false`, `mode.preloaded = false`) so both worker threads (`iw4l-skate`, `iw4l-skate-blocks`) end and the map session's collision is freed. The animation banks decoded by `Session::preload` stay in the engine's cache until the process ends; say so in the log line rather than claiming the memory is freed. Turning it on mid-match builds the session on the next frame.
- **Notices clear themselves.** The overlay shows "Skate unavailable: {status}" for as long as `status` is non-empty while nothing is preloaded (`debug_move.rs:816-819`), so a notice set by a key press would stay on screen for good. Notices ("Skating is off (Options > Controls > Minecraft)", and the setup one below) are cleared by `skate.rs` 3 s after they are set (`Host.notice_at`). Errors from the worker keep today's behaviour.
- On with no `IW4L_SKATE_ASSETS`: the notice is "Skate 3 data is not set up: restart to select your default.xex".
- `first_run::prepare`: if the settings file has `skating=true` and the assets are not ready, ask about Skate 3 **even when `IW4L_SKATE=off`**. The settings path follows `user_settings.rs:309-320` on Windows, but `.env` is not loaded yet when `prepare` runs (`load_dotenv` runs later, inside `games_root_from_env`), so look up `IW4L_SETTINGS_PATH` in the process environment, then in the `EnvFile`, else `<exe dir>\iw4l-artifacts\settings.cfg`. A pure `skating_wanted(text: &str) -> bool` parses the file. `IW4L_SKATE` keeps its meaning "setup declined" and only skips the question while skating is off. When the conversion succeeds, drop the stale `IW4L_SKATE=off` line. When the player answers No while the switch is on, they will be asked again on every launch, so the question text adds "To stop this question, set Skating to Disabled in Options > Controls > Minecraft."
- Tests: `skating_is_off_until_chosen_and_survives_a_restart` and `the_skating_menu_row_switches_the_setting_and_mirrors_it` (`user_settings.rs`, next to the blood tests at `:919-950`); `the_minecraft_page_offers_a_skating_switch` (`menu.rs`); `skating_wanted_reads_only_a_true_skating_line` (`first_run.rs`: true, false, missing, commented out, `skating=true` inside a bind line).

### Group 2. "The skateboard key is a rebindable Minecraft control, J by default"
Files: `input_iw4/src/names.rs`, `frame/src/minecraft_ui.rs`, `console/assets/default_controls.cfg`, `console/src/binds.rs`, `console/src/user_settings.rs`, `console/src/plugin/mod.rs`, `console/src/debug_move.rs`, `ui/menus/settings.json`, `ui/src/menu.rs`.
- Append `"mc_skate"` after `mc_perspective` in `INPUT_COMMAND_NAMES` (id 84, so it is in the Minecraft layer); the array type becomes `[&str; 85]` (`names.rs:1`). The existing test `minecraft_commands_follow_the_mw2_ones` asserts `!is_minecraft_command(84)` and `(78..=83)` (`names.rs:161-182`), and `minecraft_commands_do_nothing_to_mw2_input` loops `78..=83`: move both to 84/85. Ids are positional; no other round 8 brief adds input commands (checked: A, B, C and `docs/MULTIPLAYER.md`), but tell track E if it starts to.
- `McAction::Skate` (`"mc_skate"`); `ALL` becomes 7 (still fits the `u8` bit sets). Any exhaustive `match` on `McAction` gets an arm.
- `default_controls.cfg`: `bind J mc_skate`; add `(KeyJ, "mc_skate")` to the defaults test list (`binds.rs:724-729`).
- `user_settings.rs`: header **v5** (`:347`). For v1-v4 files add `J mc_skate` only when `mc_skate` is unbound and J is free in **both** layers (a new idempotent `KeyBinds::add_skate_key`). `apply_minecraft_defaults` (`binds.rs:329-337`, still called for v1/v2 at `user_settings.rs:495-500`) only checks the Minecraft layer, so it must skip `mc_skate` and leave it to `add_skate_key`. Do not call the general `apply_minecraft_defaults` for v3/v4, so keys the player unbound stay unbound. Existing tests that change: `older_settings_get_the_vanilla_minecraft_keys` (6 becomes 7), `current_settings_keep_their_minecraft_keys_unbound` (v3/v4 now get J only; add v5, which gets nothing), and every `"// IW4L user settings v4"` expectation of a saved file (`:786`, `:793`, `:858`). **No other round 8 brief changes `settings.cfg`** (checked), so this track owns the v5 bump.
- Menu row "Skateboard Down/Up" (item_type 14, dvar `mc_skate`) at y 168, under the switch. `MINECRAFT_KEYS` in the menu test grows to 7, `the_minecraft_page_lists_the_six_minecraft_keys` is renamed for seven, and the expected lines become `[28, 48, 68, 88, 108, 128, 168]` (the switch at 148 is item_type 12). `sync_binding_view` walks every command (`user_settings.rs:192`), so `ui_bind_mc_skate` appears without further code; extend `the_controls_menu_shows_the_minecraft_keys` with it.
- `plugin/mod.rs`: the toggle at `:366-372` moves below `minecraft_presses` (`:385-395`). In a Minecraft map it fires on `mc_keys.just_pressed(McAction::Skate)`. In an MW2 map, and in a Minecraft map while the player is dead (`MinecraftUi.active` is false then, so the layer is off), it fires when any button bound to `mc_skate` in the Minecraft layer was just pressed, through a new `KeyBinds::minecraft_buttons_for(id)`. L3+R3 stays. Same blocks as today (console, script menu, inventory, chat): `minecraft_presses` gates only the console, script menus, chat and focus (`:388`), and records presses even with the inventory open (`binds.rs:583-585`), so the inventory check stays explicit.
- Overlay (`debug_move.rs:815-823`): show the bound key from `KeyBinds` instead of the literal "J", and "pick up" instead of "return to MW2" in a Minecraft map.
- Tests: `mc_skate_is_command_84` (`names.rs`); McAction round trip covers Skate (`minecraft_ui.rs:457-463`); `the_defaults_bind_the_vanilla_minecraft_keys_beside_the_mw2_ones` expects 7; `older_settings_get_the_skate_key_on_a_free_j`, `a_j_bound_in_the_mw2_layer_keeps_the_skate_key_unbound`, `an_unbound_skate_key_stays_unbound_in_v5` and `a_rebound_skate_key_survives_a_restart` (`user_settings.rs`); `the_skate_key_works_on_mw2_maps_through_its_minecraft_binding` (a pure `skate_toggle_pressed(binds, mc_keys, minecraft_map, just_pressed)` in `plugin/mod.rs` or `binds.rs`).

### Group 3a. "The skateboard is picked up by teleports, deep water, flying, gliding, mounts and spectator mode"
Files: `frame/src/minecraft_ui.rs`, `render_anim/src/skate.rs`, `render_anim/src/minecraft_world.rs`.
- `MinecraftUi.board_allowed: bool`, computed in `minecraft_world::update` by a pure `board_allowed(...)`: alive, not spectator, not `MC_FLYING`, not gliding (`minecraft_elytra.rs:57`), eye not in water (`water.eye`), and, once track B lands them, not riding (`pm_flags::MC_RIDING`, `b3-striders.md` group 7) and not asleep (`b4-sleeping.md`). Whichever of B and D merges second adds the missing condition. `skate.rs` refuses to put the board down when it is false and calls `stop` when it turns false while riding (only when `minecraft.active`, so MW2 maps are unchanged). `minecraft_world::update` runs after `skate::update` in the same frame, so `skate.rs` sees the previous frame's value; one frame is fine.
- **Teleports.** At the top of `skate::update`, before the replies are drained (`skate.rs:387-400`), read `authority.0.player(local.0)` (`sim/src/frame.rs:298`) and compare its `e_flags & eflags::TELEPORT` with the bit kept in `Host` (refreshed every frame, also while not skating). When it flipped while riding, entering or activating, call `stop` first: the epoch bump makes the queued `Reply::Pose` stale, so `present` never writes the old place over the new one. Reading the presented snapshot instead is too late (section 2, Teleports). `/tp`, ender pearls, chorus fruit and dimension switches (`teleport_facing`) then leave the player standing at the new place; respawn is already covered by `alive`.
- Tests: `the_board_is_refused_flying_gliding_dead_spectating_or_under_water` (`minecraft_world.rs` pure helper); `a_teleport_flip_picks_the_board_up` (a pure `teleported(last: Option<i32>, now: i32) -> bool` in `skate.rs`, false on the first frame).

### Group 3b. "On the board, items are put away and the player rides like a vehicle"
Files: `frame/src/skate.rs`, `render_anim/src/skate.rs`, `render_anim/src/minecraft_world.rs`, `minecraft_survival/src/survival.rs`.
- `SkateMode.airborne: bool`, set in `present` from a pure `airborne(state: &str) -> bool`: true for `PhysicsAir`, `KnownAir`, `PhysicsAirSecondary` and `BipedAir` (`skate-core/src/player/state.rs:6-31`; `Pose.state` is `format!("{:?}", PhysicalStateId)`, `skate-host/src/physics/bridge.rs:185`). Grinds, wipeouts and ground states are not airborne.
- `minecraft_world.rs` while `skate.active`:
  - `on_ground` (`:833`) = `!skate.airborne`, so `air_peak` builds in the air and the existing landing code (`:878-899`) deals fall damage, with hay, water, slime and feather falling as today and `crouching` false.
  - No footsteps: skip the step block (`:853-874`).
  - `attack_held`, `use_held`, the left and right `just_pressed` and the pad trigger reads (`:999`, `:1050`, `:1058`, `:1159`) are false. No mining, punching, placing, eating, blocking or bow drawing on the board.
  - The local `BodyGear` (`:2202-2206`) is set with `main` and `off` cleared (armour kept), so no Minecraft item is drawn in the skater's hands. The MW2 gun is already left off the skating body (`remote_body.rs:966-971`), so **this group does not edit `remote_body.rs`**.
  - `Motion` gets a new field `riding: bool` (it derives `Default`; the literal at `:936` sets it, test literals use `..Motion::default()`). `Survival::tick` skips the walk, sprint, swim and jump exhaustion when it is set (`survival.rs:317-346`; vanilla `checkMovementStatistics` returns before the swim branch). Breath, drowning and damage rules are unchanged.
- MW2: `runtime.rs:1173-1175` already zeroes the buttons. The selected slot and weapon come back unchanged on pick-up. Hotbar keys, the wheel, Q (drop), E (inventory, the skater coasts) and F5 keep working.
- Tests: `airborne_states_are_the_air_states` (`skate.rs`); `a_rider_spends_no_food_on_movement` (walking, sprinting and swimming with `riding` set) and `a_rider_still_loses_air_under_water` (`survival.rs`); `a_landing_skater_takes_vanilla_fall_damage` (pure `skate_on_ground(skating, airborne, ground_entity)`, plus a `survival.fall(8.0, ...)` check that it deals 5); `a_skater_holds_no_minecraft_item` (pure gear filter, armour kept).

### Group 4. "Stairs, slabs, carpets and snow layers become ramps for the skateboard"
Files: `sim/src/voxel.rs`, `render_anim/src/skate.rs`.
- **Rule:** every riser of at most half a block between two walkable surfaces gets a 45-degree fillet on its low side. Its run equals its height. Full-block risers stay ledges to ollie and grind. On real stairs this is the plane a plank laid on the step edges makes: a staircase of bottom stairs (each one block higher) becomes one continuous 45-degree slope through the nosings, starting on the ground half a block in front of the first step. A single stair is a one-block kicker. A slab on the ground, or a full block behind a slab, gets a half-block toe. Carpets (1/16), stacked snow layers (2/16 steps; a single layer has no collision at all, section 3), dirt paths and farmland (15/16 next to 16/16) get tiny fillets, so they roll smoothly.
- **Lips that become ramps stop being grind rails.** With the fillet in the triangle list, `rails::find`'s drop probe (`rails.rs:152-160`, 2 units out, at least 3 units of drop) lands on the fillet, so the front edges of a slab on the ground, a single stair and a stair step no longer grind. That is the intended trade (a ramp, not a ledge); full-block ledges and anything higher than half a block above its floor keep their lips.
- `voxel.rs`: a pure `fn riser_fillets<'a>(shape_at: impl Fn(i32, i32, i32) -> &'a [[f32; 6]], lo: [i32; 3], hi: [i32; 3]) -> Vec<[[f64; 3]; 3]>` in block space (the named lifetime is needed: an `impl Fn` returning a borrowed slice does not elide), and `pub fn skate_ramps(centre: [f32; 3], radius: i32, depth: i32) -> Vec<[[f32; 3]; 3]>` mapping it to map units under the `WORLD` read lock (as `collision_triangles` does).
  - For each box of each block in the region, and each of its four horizontal faces (outward direction `d`, width `[w0, w1]` along the tangent, top `T` = box max y):
    1. Skip when the column just inside the face at `T + 1/64` is not free (the box top is covered).
    2. Sample two points 1/64 outside the face, at a quarter and three quarters of its width. At each, find the floor `F`: the highest box top below `T - 1e-4` in that column, looking down at most half a block, with no box between `F` and `T`.
    3. Make a fillet only when both samples give the same `F` and `0 < T - F <= 0.5 + 1e-4`. Re-sample the column at `r/2` and `0.95 r` out (with `r = T - F`): the floor must still be `F` and the space up to `T` free.
    4. The fillet is a wedge: slope quad from the face line at `T` to the line `r` out at `F`, plus the two side triangles. Wind every triangle counterclockwise from outside, like `collision_triangles`.
  - Top-half stairs and top slabs only make fillets where those rules hold (usually never). Fences (riser 1.5) and doors make none.
- `skate.rs block_collision` (`:84-119`): append `skate_ramps` to the map triangles **before** `rails::find`. Log the triangle count and the build time at debug; the build must stay under about 100 ms at radius 40 (it runs on the `iw4l-skate-blocks` thread).
- Tests (`voxel.rs`, on a hand-built `shape_at`):
  - `a_bottom_stair_on_the_ground_is_a_45_degree_ramp_from_half_a_block_in_front`: the slopes are coplanar, from (-0.5, 0) to (0.5, 1) along the facing.
  - `a_staircase_of_stairs_is_one_plane`: three stairs, each one higher.
  - `a_slab_on_the_ground_gets_a_half_block_toe`.
  - `a_full_block_and_a_fence_get_no_fillet`.
  - `a_carpet_gets_a_one_sixteenth_fillet` and `one_snow_layer_gets_none_and_three_get_a_quarter_block_one` (layers 1 and 3: collision 0 and 4/16).
  - `a_slab_on_the_ground_is_no_longer_a_lip` (`skate.rs` or `rails.rs`: `rails::find` over a slab's boxes plus its fillets finds no rail on the filleted edges).
  - `a_fillet_is_never_built_into_another_block`: a stair facing a wall, and a slab with a block in front.
  - `fillet_triangles_face_outward`.

### Group 5a. "Fences, walls, iron bars and the sides of stair ramps are grind rails"
Files: `render_anim/src/skate/rails.rs`, `render_anim/src/skate.rs`.
- `rails.rs`: `find` takes a `Limits { max_slope, merge_ridges }`. MW2 maps keep `max_slope 0.7` and no ridge merge (no behaviour change). The block world uses `max_slope 1.01`, so the side edges of a 45-degree stair ramp grind like a hubba, and `merge_ridges: true`: two parallel lips at the same height, at most 18 map units (half a block) apart (compare with a small tolerance: a wall post is exactly 18 wide) and overlapping along their length, become one rail on their centre line. A fence (4/16 wide), iron bars and glass panes (2/16), wall sides (6/16) and wall posts (8/16) then give one centre rail at their collision top. Fences and walls grind at **1.5 blocks** (their vanilla collision height; the board can't pass through it anyway), bars and panes at 1 block.
- Tests: `two_close_parallel_lips_become_one_centre_rail` and `a_one_block_wall_keeps_two_rails` (`rails.rs`); `mw2_limits_drop_a_45_degree_edge_and_block_limits_keep_it`.

### Group 5b. "Minecraft rails are grind rails for the skateboard"
Files: `sim/src/voxel.rs`, `render_anim/src/minecraft_world.rs` (`shape_id`, `:2501-2540`), `render_anim/src/skate/rails.rs`, `render_anim/src/skate.rs`.
- Minecraft track rails: a rail-shape field in the per-shape flags (`VoxelWorld.fluids`, read by `fluid_at`; constants at `voxel.rs:546-557`; bits `0x7800`, values 1-10 for the ten `RailShape`s, 0 for none), filled in `shape_id` from the `shape` property of `minecraft:rail`, `powered_rail`, `detector_rail` and `activator_rail`. The flags are already part of the `shape_ids` key and a shape with flags but no boxes already gets its own id (`minecraft_world.rs:2527-2537`), so OR-ing the bits into `fluid` is enough. Track C (`c5-limitations.md`) takes `0x8000` for powder snow in the same field; after both, the u16 is full.
- Non-zero flags on a block without water: `free_spawn_spot` treats any non-zero flag as wet (`voxel.rs:768`, `fluid_at(...) != 0`), so a respawn spot on a rail would be refused. Mask the rail bits out there (`& !RAIL_SHAPE`), and check the other `fluid_at` readers only test their own bits (they do today: `:791`, `:806`, `:1200-1205`, `:1239`, `:1255`).
- `pub fn track_rails(centre, radius, depth) -> Vec<Vec<[f32; 3]>>` in `voxel.rs`: per rail block, a polyline 2/16 above its floor.
  - Straight: across the block centre.
  - Ascending: from the low edge at 2/16 to the high edge at 1 + 2/16.
  - Curve: a quarter arc of radius 0.5 between the two edge midpoints, in 4 segments.
  - Polylines whose ends meet within 1/32 block are chained (reuse the chaining in `rails.rs`, made `pub(crate)` with a point list input).
- `skate.rs block_collision` adds `track_rails` to the found rails.
- Tests: `track_rails_follow_straight_ascending_and_curved_shapes` and `three_straight_rails_chain_into_one` (`voxel.rs`); `rail_shapes_get_their_own_shape_ids` (`minecraft_world.rs` pure part of `shape_id`, or the flag encoding in `voxel.rs`); `a_rail_does_not_make_a_respawn_spot_wet`.

### Group 6. "The skateboard stops at chunks that have not loaded yet"
Files: `sim/src/voxel.rs`, `render_anim/src/skate.rs`.
- `pub fn missing_chunk_walls(centre, radius, depth) -> Vec<[[f32; 3]; 3]>`: for every chunk column in the region that is loaded next to one that is not, a vertical quad on their shared edge spanning the region's height, facing into the loaded chunk. This keeps the skater in step with the walking player, whom the fork already freezes in an unloaded chunk (`awaiting_ground`, `sim/src/step.rs:300`). Chunks stream 8 chunks around the player (`VIEW_DISTANCE`, `minecraft_world.rs:22`), well beyond the 40-block skate region, so the walls only show when the skater outruns the streaming.
- `block_collision` appends them after `rails::find` has run on the blocks and fillets (the detector only starts rails on walkable faces, `rails.rs:56-58`, but its wall probe would cancel lips right at a wall).
- Tests: `walls_stand_only_between_loaded_and_missing_chunks` and `a_fully_loaded_region_has_no_walls`.

### Group 7. "Keyboard and mouse can skate when no controller is connected"
Files: `render_anim/src/skate.rs`, `frame/src/skate.rs`, `console/src/debug_move.rs`.
- When `pad` is `None` (`skate.rs:378-383`), build the frame from the keyboard and mouse with a pure `keyboard_frame(keys: KeySample, mouse: [f32; 2], state: &mut FlickState, dt) -> InputFrame` in XInput layout:
  - W/A/S/D → left stick (±32767).
  - Mouse motion → right stick, for the flick-it tricks: the delta times 600 per pixel, clamped to ±32767, easing back to centre over 0.1 s once the mouse stops.
  - Space → A (`0x1000`), LShift → X (`0x4000`), LCtrl → B (`0x2000`), R → Y (`0x8000`).
  - Left mouse → RT and right mouse → LT (255 while held). Z → LB (`0x0100`), V → RB (`0x0200`). Tab → Back (`0x0020`).
  - Start is never sent: Esc stays the MW2 pause menu.
  - The skating keys are fixed for this round. Rebindable skate controls are out of scope.
- A screen or chat sends a centred, connected frame (`InputFrame::from_pad(0, [0; 2], [0; 2], [0; 2], packet)`, as the pad path does at `:382`), not `InputFrame::neutral()`: `neutral()` reports every controller slot as disconnected (`skate-host/src/physics/bridge.rs:296-300`). The console and script menus keep suspending input through `mode.input_blocked`.
- A keyboard frame is a connected controller to the engine, so `mode.controller` (`skate.rs:384`) becomes `Some(0)`. Add `SkateMode.keyboard: bool`, set when the frame came from the keyboard, and let the overlay (`debug_move.rs:821`) read "SKATE | Keyboard: WASD steer, mouse flick, Space push | <key>: pick up" from it instead of "Connect a controller".
- The agent checks the button meanings against `skate-core/src/input/riding_intentions.rs` and `offboard_intentions.rs`, keeping push on Space. If one of A/B/X/Y turns out to step off the board, it goes on the least reachable key (Tab), so the board isn't left by accident. Expect Y: the retail Skate 3 layout gets off the board with Y, so R → Y is likely wrong.
- Tests: `w_pushes_the_left_stick_forward`, `a_mouse_flick_reaches_the_right_stick_and_settles`, `space_presses_a_and_the_mouse_buttons_pull_the_triggers` and `no_key_gives_a_centred_frame`.

## 5. Owner's in-game test steps

Kit (13 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:oak_stairs*64,minecraft:stone_slab*64,minecraft:oak_fence*64,minecraft:cobblestone_wall*64,minecraft:iron_bars*64,minecraft:rail*64,minecraft:powered_rail*16,minecraft:white_carpet*64,minecraft:snow*64,minecraft:water_bucket,minecraft:hay_block*16
```

You need an extracted Skate 3 `default.xex` with its `data` folder, and an Xbox controller (until group 7; the keyboard steps are marked). Use survival unless a step says otherwise; `/gamemode creative` makes building quicker.

1. Start the new build once. Options > Controls > Minecraft shows "Skating: Disabled" and "Skateboard Down/Up: J" (screenshot). In the world, J only shows "Skate unavailable: Skating is off (Options > Controls > Minecraft)" for about 3 seconds.
2. Set Skating to Enabled and quit. `iw4l-artifacts\settings.cfg` starts with `// IW4L user settings v5` and holds `skating=true` and `bind j mc_skate`.
3. Start again. Although `.env` says `IW4L_SKATE=off`, the game asks "Do you have Skate 3?". Answer Yes, pick `default.xex`, and wait for the converter. `.env` now has `IW4L_SKATE_ASSETS=".../skate-data/assets"`.
4. In the Overworld, on flat ground, press J: the board goes down, the skate camera takes over and the overlay names J for "pick up". Hold a cobblestone in the hotbar first: neither the gun nor the block is in the skater's hands (screenshot). J again: you stand where the skater was, with the same hotbar slot and gun.
5. On the board, look at a block and pull RT and LT (grabs): no block is mined or placed. Mouse clicks don't shoot.
6. Build a 5-block-high staircase of oak stairs on flat ground, each stair one block higher than the last and standing on a cobblestone column, all facing the same way. Push into it from the ground: the board rolls up without a bump at the bottom and rides down again (screenshot halfway, wheels on the step edges).
7. Place a single stair on flat ground and ride over it fast: you get air off it like a kicker.
8. Ride up a stone slab placed on the ground, then a cobblestone-slab-cobblestone staircase (rising half a block per block).
9. Ride across a row of white carpet, then a patch of snow three layers deep (right-click snow onto the same spot three times; a single layer has no collision in vanilla either): no bumps.
10. A one-block cobblestone ledge stops the board (you have to ollie). Grind along its edge (screenshot). A slab on the ground is now a small ramp, not a ledge: its edge no longer grinds.
11. A 10-block oak fence line with a stair in front as a kicker: launch, land on the fence and grind along its middle (screenshot). Repeat with the cobblestone wall and the iron bars.
12. Lay 10 straight rails, then an ascending rail onto a block and a curve. Ollie onto the track and grind it, through the curve.
13. Ride the side edge of the stair ramp from step 6: it grinds like a hubba.
14. Build a tower 8 blocks high on flat stone ground, with a 6-block runway on top and no kicker. Roll off the edge without ollieing, so the drop is exactly 8 blocks: you lose 2.5 hearts (5 points). Then dig out the stone where you landed and set hay bales into the holes, flush with the ground (a hay bale standing on the ground shortens the drop to 7 and deals nothing), and roll off again: half a heart (1 point). An ollie or a kicker adds height and more damage.
15. In creative (the bucket stays full there), dig a 3 by 3 pool 2 blocks deep and fill every block of it with the water bucket, then `/gamemode survival`. Ride into it: the board is picked up once your eyes go under, and you swim.
16. Note the hunger bar, then ride around for a minute (no eating): it doesn't go down from riding. Walk and sprint the same minute: it does.
17. On the board, press E: the inventory opens and the skater coasts. Close it and press Q: the item drops.
18. On the board, open chat and run `/tp ~ ~ ~20`: you arrive standing, with the board picked up.
19. Options > Controls > Minecraft: rebind "Skateboard Down/Up" to K and restart. K puts the board down and J does nothing. Set it back to J.
20. Set Skating to Disabled while riding (Esc menu): the board is picked up and J only shows the "Skating is off" line again. The log has no new "Skate map session preloaded" line until you enable it again.
21. Fly in creative and press J: nothing happens. `/gamemode spectator` and J: nothing. `/gamemode survival`.
22. (Group 7, keyboard) Unplug the controller and press J: W pushes, A/D steer, a quick mouse flick down then up ollies, and the overlay shows the keyboard line.

## 6. Risks, shared files and decisions

**Decisions taken**
- **Guns:** while on the board, guns and Minecraft items are put away. No firing, aiming, grenades, melee, mining, placing, eating or blocking, and neither the gun (already the case) nor a held Minecraft item is drawn in the skater's hands. Pick the board up to fight; it takes one key press. Why: Skate's controls use both sticks and both triggers, and the skate camera is third person with nothing to aim with. MW2 already zeroes all buttons while skating, and the Minecraft side now does the same.
- **The switch** lives in `settings.cfg` (`skating=`, default off), on the Minecraft controls page next to its key. `.env` `IW4L_SKATE=off` keeps meaning "setup declined" and is overridden when the switch is on and the data is missing.
- **The key** is a Minecraft-layer command (`mc_skate`, default J, which vanilla leaves unbound). It also works on MW2 maps. L3+R3 stays for the pad.
- **Riding rules follow vanilla vehicles:** no movement exhaustion, no footsteps, the board is picked up when the eye goes under water, and the rider takes fall damage on landing (as from a horse). Teleports, dimension changes, death, spectator mode, flying and gliding pick the board up, and the board can't go down while riding a strider or asleep once track B adds them.
- **Ramps:** 45-degree fillets on risers of half a block or less (the plank-on-stairs plane). Full blocks stay ledges.
- **Rails:** block lips as today, one centre rail on narrow tops, and Minecraft track rails grind at rail height. Fences and walls grind at their 1.5-block collision height.
- **The board is not an item and is not saved.** A player who saves while riding is saved standing at the skater's position (their `ps.origin`).
- **No edits under `skate/`.** The engine stays at SK8-ENGINE/skate-3-rust-engine `cb79689` (`skate/Cargo.toml:1`). One friction material is used for every block, so ice and soul sand feel like stone; that is a known limit.

**Risks**
- **The owner may not own Skate 3, or may have no extracted `default.xex`.** Then only steps 1-2 can be tested. Ask first (section 1).
- **Grinds at ground height** (track rails 2/16 up) and **fence grinds at 1.5 blocks** are untested in the Skate 3 engine. The ollie may not reach 1.5 blocks without a kicker, hence the kickers in the test steps. If ground rails misbehave, drop group 5b and record it; 5a stands alone.
- **Fillet volume:** a 81x81x41 region with many non-full blocks (a village, a snowy forest) might take longer than the 0.25 s rebuild throttle. Measure it, and cap the fillet pass to the 24 blocks nearest the skater if needed.
- **The keyboard flick mapping** (group 7) needs tuning by feel. The constants (600 per pixel, 0.1 s settle) are a starting point.
- **Hiding held items** is done on the local `BodyGear` (group 3b), so only the skating player's own body changes; corpses keep their gear through `body_gear.died`.
- **Settings v5** is this track's alone today; a later track that bumps the header goes to v6.

**The per-player and server-tick boundary (for track E)**
- `SkateMode` and the skate worker are per viewer (one local session per process). They are not world state.
- What skating does to the world: (1) it writes the player's origin through `AuthorityWorld.set_origin` with `external_motion` on, which exists only on the host. A joined client has no `AuthorityWorld`, so `skate::update` returns early (`skate.rs:339-341`) and **joined friends cannot skate until E adds a client-reported origin** (an "external motion" usercmd or message validated by the host). (2) The Minecraft rules read `skate.active`/`airborne` in the adapter. In E's server tick these become per-player intent bits in `minecraft_net` (`skating`, `airborne`) that set `Motion.riding`, the landing check and the action gate for that UUID. `Motion.riding` (group 3b) is already per player, in `minecraft_survival`.
- Remote viewers do not see the skate pose or the board, which are not networked. A skating friend shows as a soldier sliding in an idle pose. Networking the pose is out of scope for round 8.
- `sim::voxel` is one global block world (`voxel.rs:41`). Skate collision samples whatever level the local viewer stands in. If E keeps several levels' collision on the host, `collision_triangles`, `skate_ramps`, `track_rails` and `missing_chunk_walls` must take the viewer's level.
- No new global or player `0` state: the new state is `GameSettings.skating` (client-local, like the minimap), `SkateMode.airborne` (per viewer) and `MinecraftUi.board_allowed` (per viewer, computed from that viewer's player).

**Files shared with other round 8 tracks**
- `render_anim/src/minecraft_world.rs`: A (mining speed, eye of ender), B (sleeping and strider riding; `board_allowed` needs "not asleep" and "not riding", and b3 also changes the steps and falls block at `:839` that group 3b edits), C, and E (heavy). Keep the edits to the lines named in groups 3a and 3b and `shape_id`.
- `frame/src/minecraft_ui.rs` (`MinecraftUi`, `McAction`): A, B, C and E.
- `render_anim/src/occupancy/remote_body.rs`: **not edited by this track** (the gun is already left off the skating body, `:966-971`). C4 (elytra body pose) skips the glide while skating there.
- `sim/src/voxel.rs`: E (per-level collision), and A if `/tick` or mining touches it. Groups 4-6 only add functions and four flag bits (`0x7800`). C (`c5-limitations.md`) takes `0x8000` in the same flags and also edits `shape_id`; whichever lands second adds its bits beside the other's.
- `console/src/binds.rs`, `user_settings.rs`, `input_iw4/src/names.rs`, `ui/menus/settings.json`, `ui/src/menu.rs`: no other round 8 brief touches them today (checked A, B, C and `docs/MULTIPLAYER.md`). If E adds keys or a multiplayer menu, it rebases onto this track's command id 84 and v5 header.
- `launcher/src/first_run.rs`: E if it adds host/join setup to the first run.
- `docs/MINECRAFT-SURVIVAL.md`: every track. Edit only the skate lines (105, 109) and add one short "Skating" bullet.
