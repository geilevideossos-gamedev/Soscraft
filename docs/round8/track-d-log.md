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
