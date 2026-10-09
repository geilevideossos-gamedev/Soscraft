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

Commit: (filled in by the next commit)

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
