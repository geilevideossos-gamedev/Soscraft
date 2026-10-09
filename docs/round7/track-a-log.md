# Track A log: controls and chat

Branch `track-a`, worktree `C:\Dev\mc-wt\track-a`. Briefs: `a1-grenade-keys.md`, `a2-chat.md`.

A commit can't name its own hash, so each entry is written with its commit and its hash is filled in by the next commit; the last entry's hash is in `git log` under its subject.

## Group: grenade keys

### 1. Minecraft keys never take a grenade key outside a Minecraft screen

Commit: ffca2a5

- **Changed:** `console/src/binds.rs minecraft_presses`. The old rule let the swap-hands key keep its MW2 binding only while a gun was selected (`holding_item`), so with a Minecraft item or an empty hand F swapped hands instead of throwing. Now any Minecraft key that is also bound to `+frag` or `+smoke` in the MW2 layer keeps its throw while no Minecraft screen is open, whatever is in hand. Inside a screen the Minecraft action still wins (F swaps the hovered slot like vanilla). `holding_item` is gone from `MinecraftKeyState` and from `console/src/plugin/mod.rs`.
- **Tests:** `a_grenade_key_keeps_its_throw_outside_minecraft_screens` replaces `swap_hands_takes_its_key_only_while_a_minecraft_item_is_in_hand`; `a_rebound_swap_hands_key_follows_the_same_rule` and `only_just_pressed_keys_fire_their_action` updated.
- **Decision:** the throw wins over every Minecraft action on the same key, not only swap hands, because the owner wants the grenade to work whatever is held. A key bound only to a Minecraft action still takes the key as before.
- **Test steps** (pick a class with Frag and Flash):
  1. Select cobblestone and press G: a frag flies, nothing is placed or swapped.
  2. Select an empty slot and press G: a frag flies.
  3. Open the inventory (E), hover the cobblestone and press F: it goes to the offhand. Press G: nothing is thrown while the inventory is open.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64
```

### 2. Lethal and tactical grenades move to G and B, so F only swaps hands

Commit: f7e1a42

- **Changed:** `console/assets/default_controls.cfg` binds `G +frag` and `B +smoke`; F keeps only `mc_swaphands`. The pad layouts are unchanged (RB lethal, LB tactical). `settings.cfg` is now written as v4. New `KeyBinds::move_old_grenade_keys` (`console/src/binds.rs`): when a v1, v2 or v3 file has `+frag` on exactly F and `+smoke` on exactly G (keyboard and mouse; pad buttons don't count) and B is unbound in both layers, it moves them to G and B. Any other choice is left alone, and a second run changes nothing. `parse_settings` returns whether it moved them, and `load_user_settings` then leaves `last_payload` empty so the upgraded file is written once at start; other old files are still only rewritten when something changes, as before.
- **Tests:** binds: `the_defaults_put_the_grenades_on_g_and_b_and_leave_f_to_swap_hands`, `the_old_grenade_pair_moves_once_and_only_when_untouched`; `the_defaults_bind_the_vanilla_minecraft_keys_beside_the_mw2_ones`, `rebinding_swap_hands_leaves_the_mw2_key_alone`, `unbinding_a_key_clears_both_layers` and `only_just_pressed_keys_fire_their_action` updated. user_settings: `old_grenade_keys_move_to_g_and_b` (v1 to v3, saved as v4, loading again changes nothing), `only_a_moved_grenade_pair_asks_for_the_file_to_be_rewritten`, `customised_grenade_keys_are_left_alone` (MOUSE4 lethal, B taken, v4 file, lethal on H), `a_rebound_grenade_key_survives_a_restart`; `older_settings_get_the_vanilla_minecraft_keys`, `current_settings_keep_their_minecraft_keys_unbound` (v3 and v4) and the controls-menu test (`ui_bind_+frag` is `g | R-BUMPER`, `ui_bind_+smoke` is `b | L-BUMPER`) updated.
- **Decisions:** the move needs the exact old pair and a free B in both layers (a Minecraft action on B counts as taken). Off the Minecraft map MW2 plays exactly as before apart from the keys: G lethal, B tactical, F nothing. MW2's own retail PC default for the lethal is G too.
- **Test steps** (pick a class with Frag and Flash):
  1. Start once with your current `settings.cfg` and quit. `iw4l-artifacts\settings.cfg` starts with `// IW4L user settings v4` and holds `bind g +frag`, `bind b +smoke` and `bind f mc_swaphands`, and no `bind f +frag`.
  2. Options > Controls > Actions: the lethal row ("Throw Frag/Use Equipment") starts with g and the tactical row ("Throw Special Grenade") with b (the pad button follows after a bar). Controls > Minecraft shows f for Swap Item With Off Hand. Screenshot both.
  3. On the Minecraft map with a gun selected: G throws a frag, B a flash, F does nothing visible.
  4. On an MW2 map (not Minecraft): G throws a frag, B a flash.
  5. Actions page: rebind the lethal to MOUSE4, restart: it is still MOUSE4 and G does nothing. Rebind it back to G.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64
```

### 3. With a gun selected, the swap-hands key uses the offhand item, so the tactical key always throws

Commit: 0d87b80

- **Changed:** before, the tactical key (`+smoke`) latched `MinecraftUi.offhand_key` and lost its SMOKE bit when a gun was selected and the offhand item had a use, so with food in the offhand G ate instead of throwing. Now `frame::McActionInput` also carries held bits (`hold`, `is_held`), filled by `minecraft_presses` for a Minecraft key it took, outside screens only (`console/src/binds.rs`, fed with `inputs.pressed` in `console/src/plugin/mod.rs`). `sample_client_input` (`net/src/client/runtime.rs`) latches the offhand use from the held swap-hands key through the new pure `frame::minecraft_ui::offhand_key_latch` and no longer strips SMOKE. `render_anim` reads `offhand_key` as before, so no `minecraft_*` file changed. Swapping with a gun selected stays a no-op (`minecraft_offhand::swap_hands`).
- **Tests:** frame: `a_held_minecraft_key_is_held_apart_from_its_press`, `the_offhand_key_latches_what_it_could_do_when_pressed` (press, hold, release, not usable at the press stays not usable). binds: `a_held_swap_hands_key_stays_held_until_released` (not in screens, not off the map, not for B or G, not when the key also throws).
- **Decision:** the held bit is only set outside Minecraft screens, so holding F in the inventory and closing it doesn't start eating, and a key that keeps its grenade throw never holds swap hands.
- **Test steps** (pick a class with Frag and Flash):
  1. Put the golden apples in the offhand (F with them selected), select a gun and hold F: you eat (gun hidden, walking slowed). Release early: eating stops.
  2. Still with the apples in the offhand and the gun selected, press B: a flash flies and nothing is eaten.
  3. Put the shield in the offhand, gun selected, hold F: the shield goes up while held.
  4. Put cobblestone in the offhand, gun selected, hold F while looking at the ground: blocks are placed.
  5. Put the snowballs in the offhand, gun selected, press F: a snowball is thrown.
  6. Put the stick in the offhand, gun selected, press F: nothing happens, the gun stays and the stick stays in the offhand.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:golden_apple*16,minecraft:shield,minecraft:snowball*16,minecraft:stick
```

### 4. MW2 grenade throws show while a Minecraft item or an empty hand is selected

Commit: 8ee0cd0

- **Changed:** new `pub fn mw2_throw_in_view(ps)` in `render_anim/src/occupancy/fpv_present.rs`: true while `weap_flags & OFFHAND_VIEW` is set, which MW2 sets from OffhandPrepare to the throw and clears at OffhandInit (gun going down) and OffhandEnd (gun coming up). While it is true: `occupy_fpv_scene` no longer hides the MW2 viewmodel for a selected Minecraft item, `skin_fpv_geometry` keeps the grenade draws for an empty hand (`hands_only` false), and `apply_fpv_placement` leaves the hands in MW2's place instead of Minecraft's arm pose. `render/src/extract.rs extract_minecraft_world` sends no first-person Minecraft item (main hand and offhand) to the GPU while it is true. No `minecraft_*` file changed.
- **Tests:** `fpv_present.rs a_grenade_shows_only_while_the_offhand_view_is_up` (weapon states 0x0F to 0x16, with and without the flag).
- **Decision:** the grenade shows only while the offhand view is up, not during the quick drop and quick raise around it, because with a Minecraft item selected those would show the hidden gun going down and coming back up.
- **After the split** (needs `render_anim/src/minecraft_world.rs`): Minecraft attack, mining and use still work during the throw. See "After the split" below.
- **Test steps** (pick a class with Frag and Flash):
  1. Select cobblestone and press G: MW2's hand with the frag comes up, the cobblestone disappears, the frag is thrown and the cobblestone comes back. Screenshot mid-throw.
  2. Hold G with cobblestone selected: the frag cooks in view until you release it.
  3. Select cobblestone and press B: the flash shows and flies the same way.
  4. Select an empty slot and press G: both MW2 hands and the frag show in MW2's position for the throw, then the empty hand goes back to the Minecraft arm position.
  5. With a gun selected, G and B look exactly as before.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64
```

### 5. The pad's tactical button always throws, and holding D-pad down uses the offhand item with a gun

Commit: e06f414

- **Changed:** the pad's tactical button (LB in the default layout) already throws since commit 3, because the offhand latch no longer reads `+smoke`. `minecraft_presses` (`console/src/binds.rs`) now also holds swap hands while the fixed D-pad down (the pad's swap-hands button, `hud/src/minecraft_inventory.rs:266`) is held outside screens, so with a gun selected holding D-pad down eats, blocks or places with the offhand item like F.
- **Tests:** `holding_the_pads_d_pad_down_holds_swap_hands_but_its_tactical_button_does_not` (not in screens, not off the map, not for LB or RB).
- **Decision:** D-pad down keeps its MW2 `+actionslot 2` too, as before. MW2's multiplayer scripts give out action slots 1, 3 and 4 (night vision, attachment mode, killstreaks), so slot 2 has nothing to clash with. Not checked in game data, only from memory of the MW2 scripts; if it ever does something, the owner will see it on step 3.
- **Test steps** (controller, default layout, a class with Frag and Flash):
  1. Cobblestone selected: RB throws a frag, LB a flash, both visible.
  2. Golden apples in the offhand, gun selected: hold D-pad down, you eat. Press LB: a flash flies and nothing is eaten.
  3. Gun selected, nothing in the offhand: tap D-pad down, nothing happens.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:golden_apple*16
```

### 6. The input sampler takes the Minecraft keys beside the view subject

Commit: (filled in by the next commit, or see `git log`)

- **Changed:** commit 3 put `McActionInput` into the first parameter tuple of `sample_client_input` (`net/src/client/runtime.rs`), which made clippy report a new `type_complexity` there. It now sits in a pair with `view`. No behaviour change.
- **Checks:** `clippy -p console -p frame -p net -p render_anim -p render --tests -- --cap-lints warn` shows no warning on a line this group added; the rest are older (`too_many_arguments` and `type_complexity` on system signatures). Without `--cap-lints warn` clippy stops at older `approx_constant` errors in `minecraft_terrain`, which this track doesn't touch.
- **Test steps:** none of its own (commit 3's steps cover it).

## After the split

Changes this track needs in files the crate split owns, with the exact edit.

- **Block Minecraft attack, mining and use while an MW2 grenade is up** (from commit 4). In `minecraft_world.rs update` (or wherever the split moves it), after `let busy = survival.eating().is_some() || hand.blocking.is_some() || hand.miss_time > 0;` (about line 898) compute `let mw2_offhand = (0x10..=0x15).contains(&ps.weaponstate_primary);` (OffhandInit to OffhandEnd; or make `weapon_iw4::offhand::in_offhand_family` at `weapon_iw4/src/offhand.rs:128` `pub` and call it) and add `|| mw2_offhand` to `busy`, so the hits at about 901, 961 and 976 stop. Change `let idle = survival.eating().is_none() && hand.blocking.is_none();` (about line 997) to `... && !mw2_offhand`, so no item is used or placed. In the split this is the per-player server check: the player's own MW2 weapon state, read from that UUID's playerstate.
