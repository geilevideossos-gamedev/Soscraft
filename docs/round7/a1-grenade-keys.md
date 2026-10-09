# A1. Grenade keys that work whatever is in hand

Owner report (screenshot 3): with a Minecraft item or an empty hand selected, F swaps hands, so a frag can't be thrown. Throwing must work whatever is held. The owner suggested Ctrl+G for the lethal and Shift+G for the tactical, all rebindable.

**Recommendation:** don't use chords. Lethal goes on **G** (retail MW2's own key), tactical on **B**, and **F** becomes only the vanilla swap-hands key. With a gun selected, F uses the offhand item instead (a gun can never go to the offhand), so the tactical key always throws.

## 1. What exists today

- **Defaults** (`crates/console/assets/default_controls.cfg`):
  - MW2 layer: `F +frag` (line 21), `G +smoke` (22), `CTRL +prone`, `SHIFT +breath_sprint`, `C +movedown`, `V +melee`, `E +activate`.
  - Minecraft layer: `E mc_inventory`, `Q mc_drop`, `F mc_swaphands` (31), `T mc_chat`, `SLASH mc_command`, `F5 mc_perspective`.
  - Fixed: M, J (skate), 1-9. Free: B, H, K, L, N, O, P, U, X, Y, Z, MOUSE4, MOUSE5.
- **Pad** (`binds.rs:191 pad_layout`): RB `+frag`, LB `+smoke`. D-pad down swaps hands (hard-coded, `hud/src/minecraft_inventory.rs:266`).
- **Two layers**: `KeyBinds { map, minecraft }` (`console/src/binds.rs:317`) holds one command per button in each layer. Commands with id 78 or higher (`input_iw4/src/names.rs:90 MINECRAFT_COMMAND_FIRST`) go to the Minecraft layer.
- **The handover**, `minecraft_presses` (`binds.rs:531-560`):
  - A Minecraft key suppresses the MW2 binding on the same key.
  - Exception `gun_keeps_key` (549-550): swap hands lets go of its key only while no screen is open and `holding_item` is false.
  - `holding_item` (`render_anim/src/minecraft_world.rs:762-767`, inside `update`) is true for any selected slot that isn't a gun, empty slots included. So F frags only with a gun; with an item or an empty hand, F swaps. This is the bug.
  - The state comes from `console/src/plugin/mod.rs:356,390-397`.
- **Tactical overload** (`net/src/client/runtime.rs:1081-1090`, `sample_client_input`):
  - The `+smoke` key latches `ui.offhand_key = Some(ui.offhand_usable)` and strips `SMOKE` when it is true.
  - `offhand_usable` (`minecraft_world.rs:1847`) is true when a gun is selected and the offhand item has a use, such as food, a shield or a block.
  - So with a gun and food in the offhand, G eats instead of throwing. This is the second way a throw fails.
- **Throw path**:
  - Commands to buttons: `+frag`/`+smoke` go to `KbuttonSet.frag/smoke`, then `cmd_buttons` (`input_iw4/src/lib.rs:461-462`) sets `buttons::FRAG` (0x4000) and `SMOKE`.
  - Which grenade: `runtime.rs:1115-1125` puts `loadout.lethal` or `loadout.tactical` in `cmd.off_hand_index`.
  - Throwing: `weapon_iw4/src/offhand.rs:397 weapon_check_for_offhand` throws `offhand_primary` on a FRAG rising edge and `offhand_secondary` on SMOKE (428-433).
  - The server strips FRAG/SMOKE only for spectators (`sim/src/script_player.rs:796 untouchable_buttons`) and for script-disabled offhands (`constrain_cmd`). A Minecraft item in hand doesn't block it.
- **Invisible throw**:
  - `occupy_fpv_scene` (`render_anim/src/occupancy/fpv_present.rs:419`) hides the whole MW2 viewmodel while a non-empty Minecraft item is selected.
  - With an empty hand, `skin_fpv_geometry` keeps only hand draws (`hands_only`, line 803), which drops the grenade mesh. `apply_fpv_placement` (1132) also moves the hands to the Minecraft arm pose.
- **Chords**:
  - The bind system has no modifier chords. `parse_button_name` (`binds/key_names.rs:69`) knows single keys only; CTRL and SHIFT are ordinary keys (176-189).
  - MW2 `Kbutton`s fire on key down, so Ctrl would already have sent `+prone` before G arrives.
- **Controls menu**: the retail Actions page already has the `+frag` and `+smoke` rows ("Throw Frag/Use Equipment", "Throw Special Grenade"). The Minecraft page has six rows (`ui/src/menu.rs:198-203`). `consume_menu_binding` (`user_settings.rs:57`) rebinds within one layer.
- **settings.cfg**: `serialize_settings` (`user_settings.rs:343`) writes header v3, `key=value` lines, `unbindall` and both layers' `bind` lines. `parse_settings` (387) gives v1 and v2 files the Minecraft defaults (492-497). The owner's file is v2 with `bind f +frag` and `bind g +smoke`, and upstream `iw4l.exe` shares it.

## 2. Verified reference behaviour

- **MW2 retail PC defaults**: `default_mp_controls.cfg` rawfile in `zone/english/localized_code_pre_gfx_mp.ff`, decoded from the owner's Steam install:
  - `bind g "+frag"`, `bind MOUSE3 "+frag"`, `bind q "+smoke"`.
  - `bind f "+activate"`, `bind e "+melee"`.
  - `bind SHIFT "+breath_sprint"`, `bind CTRL "toggleprone"`, `bind c "togglecrouch"`.
  - So MW2's lethal key really is G. Its tactical key, Q, is Minecraft's drop key.
- **MW2 sprint**: `movement_iw4 sprint_ending_buttons` mask 0xcf35 includes FRAG and SMOKE. A throw ends a sprint, and `weapon_check_for_offhand` refuses a throw while `SPRINTING` is still set (`offhand.rs:417`).
- **Minecraft 26.3** (javap on `client.jar`):
  - `Options` (`InputConstants` scancodes): `keySwapOffhand` = F, `keyDrop` = Q, `keySprint` = LCONTROL, `keyShift` (sneak) = LSHIFT, `keyLoadHotbarActivator` = X, and **`keyQuickActions` = `KEY_G` (10)**. So G *is* bound in vanilla, contrary to the task note. H and B are unbound.
  - `Minecraft.handleKeybinds`: Quick Actions only opens `getQuickActionsDialog()` when the server sent one (none here, so G is free in practice). Swap offhand sends `SWAP_ITEM_WITH_OFFHAND` unless `isSpectator()`. Drop passes `hasControlDown()`.
  - `KeyMapping` has no modifier field. The only chords are the F3 debug keys.

**Why not Ctrl+G or Shift+G:**
- Ctrl is MW2 prone here, and in creative flight Ctrl and C sink. In vanilla, Ctrl is sprint.
- Shift is MW2 sprint and hold breath (vanilla sneak). Shift+G starts a sprint that the throw then cancels.
- Chords would need a new `BindButton` kind, its own host key number, chord capture in the menu and a delayed modifier. Even then, the MW2 prone or sprint fires first.

## 3. Proposed commits

1. **"Minecraft keys never take a grenade key outside a Minecraft screen"**
   - In `binds.rs minecraft_presses`, replace `gun_keeps_key` with `throw_keeps_key = !state.screen_open && matches!(binds.binding_name(button), Some("+frag" | "+smoke"))`.
   - Remove `holding_item` from `MinecraftKeyState` and `plugin/mod.rs:356,396`.
   - Inside screens, F still swaps the hovered slot like vanilla.
2. **"Lethal and tactical grenades move to G and B, so F only swaps hands"**
   - `default_controls.cfg`: `bind G +frag`, `bind B +smoke`, and drop `bind F +frag`.
   - `pad_layout` stays as it is.
   - `user_settings.rs`: write header v4. Add `KeyBinds::move_old_grenade_keys` to `binds.rs`. For a v1, v2 or v3 file whose keyboard `+frag` is exactly F and `+smoke` exactly G, with B free in the MW2 layer, it moves them to G and B. Customised files are left alone, and running it twice changes nothing.
3. **"With a gun selected, the swap-hands key uses the offhand item, so the tactical key always throws"**
   - `frame/src/minecraft_ui.rs:271 McActionInput` gets `held` bits (`hold`, `is_held`). `minecraft_presses` fills them from `inputs.pressed`.
   - In `runtime.rs:1081-1090`, the latch reads `mc_keys.is_held(SwapHands)` instead of `kb.smoke`, and no longer strips `SMOKE`.
   - Factor the latch into a pure `offhand_key_latch(prev, held, usable) -> Option<bool>`.
   - Swapping a gun stays a no-op (`minecraft_offhand::swap_hands`).
4. **"MW2 grenade throws show while a Minecraft item or an empty hand is selected"**
   - New `MinecraftUi.mw2_offhand` field, set in `minecraft_world::update` from `ps.weaponstate` in OffhandInit..OffhandEnd (0x10-0x15). Make `weapon_iw4::offhand::in_offhand_family` (`offhand.rs:128`) pub.
   - `fpv_present.rs:419`: don't return early while `ps.weap_flags & OFFHAND_VIEW` is set.
   - Line 803: `hands_only` is false during the throw. Line 1132: skip `minecraft_hand_placement`.
   - `minecraft_world.rs:1859-1860`: hide the Minecraft main-hand and offhand items. Line 878: skip Minecraft attack and use while `mw2_offhand` is set.
5. **"The pad's tactical button always throws, and holding D-pad down uses the offhand item with a gun"**
   - Feed the D-pad down hold (`minecraft_inventory.rs:266`) into the same latch.
   - Check the overlap with `+actionslot 2` on D-pad down first.

Each commit updates the matching lines of `docs/MINECRAFT-SURVIVAL.md` ("MW2 integration and UI": the swap-hands key and the offhand paragraphs).

## 4. Unit tests

- **`binds.rs`**:
  - Replace `swap_hands_takes_its_key_only_while_a_minecraft_item_is_in_hand` with `a_grenade_key_keeps_its_throw_outside_minecraft_screens`. F is bound to both `+frag` and `mc_swaphands`: not suppressed in the world, suppressed with SwapHands pressed in a screen.
  - Add `the_defaults_put_the_grenades_on_g_and_b_and_leave_f_to_swap_hands`.
  - Add `a_held_swap_hands_key_stays_held_until_released`.
  - Update `the_defaults_bind_the_vanilla_minecraft_keys_beside_the_mw2_ones` and `rebinding_swap_hands_leaves_the_mw2_key_alone` (`binding_name(F)` is now None).
  - Update `only_just_pressed_keys_fire_their_action` (6 suppressed keys).
- **`user_settings.rs`**: `old_grenade_keys_move_to_g_and_b` (v1-v3, and twice changes nothing), `customised_grenade_keys_are_left_alone` (MOUSE4 `+frag`, or B taken). Update the v3 headers to v4 and `older_settings_get_the_vanilla_minecraft_keys` (F `+frag` now moves to G).
- **`runtime.rs`**: `offhand_key_latch` covers press, hold, release, and "not usable at press stays not usable".
- **`fpv_present.rs` / `minecraft_world.rs`**: pure `mw2_throw_visible(ps)` and `mw2_offhand_busy(weaponstate)` helpers, tested over states 0x0F-0x16 and the OFFHAND_VIEW flag.
- **sim**: `a_grenade_button_reaches_the_weapon_with_minecraft_flags_set`. A survival player's FRAG survives `constrain_cmd`. The existing spectator test stays.

## 5. In-game test steps

Kit (5 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:golden_apple*16,minecraft:shield,minecraft:torch*16,minecraft:snowball*16
```

Pick a class with Frag and Flash (or Semtex and Stun).

1. Start once and quit. `iw4l-artifacts/settings.cfg` starts with `// IW4L user settings v4` and holds `bind g +frag`, `bind b +smoke` and `bind f mc_swaphands`, with no `bind f +frag`.
2. Options > Controls > Actions: the lethal row shows G and the tactical row shows B. The Minecraft page shows F for Swap Item With Off Hand. (Screenshot.)
3. Gun selected: G throws a frag and B a flash.
4. Cobblestone selected: G throws a frag with the grenade and arms visible and the cobblestone hidden until the throw ends (screenshot mid-throw). B throws a flash. Holding G cooks it until release.
5. Empty slot selected: G and B throw, with hands and grenade visible.
6. Cobblestone selected, F: it goes to the offhand. Empty hand, F: it comes back.
7. Golden apple in the offhand, gun selected: holding F eats (gun hidden, slowed). B throws a flash without eating.
8. Shield in the offhand, gun selected: holding F raises the shield.
9. Open the inventory, hover an item and press F: it swaps to the offhand. G and B throw nothing while the inventory is open.
10. Shift-sprint, then G: the sprint ends and the frag flies.
11. Rebind the lethal to MOUSE4 on the Actions page and restart: it is kept. Rebind back to G.
12. `/gamemode spectator`: G and B do nothing. `/gamemode survival`.
13. Open the chat (T) and type "gb": no throws.

## 6. Risks and overlaps

- **Muscle memory**: F no longer frags. The owner must know about G and B and the F-with-gun offhand use.
- **Shared settings.cfg**: upstream `iw4l.exe` writes v2 and the old pair. The move runs again on the next survival start, and only touches the exact F/G pair.
- **Future G conflict**: 26.3 uses G for Quick Actions. If server dialogs are ever added, they need another key.
- **Ctrl+Q**: whole-stack drop already presses MW2 prone, because Ctrl is `+prone`. This is an existing clash of the same kind; note it for track A.
- **Pad**: D-pad down is also `+actionslot 2`.
- **Crate split**:
  - `holding_item`, `offhand_usable` and `offhand_key` are computed in `render_anim minecraft_world::update` and move to `crates/minecraft_survival`.
  - The "use offhand" hold must become a per-player intent bit in `crates/minecraft_net`, applied by the host to that UUID's inventory.
  - FRAG and SMOKE are already usercmd bits in the host-authoritative sim, and key bindings stay client-local (`settings.cfg`, not player files).
- **Other tracks**: track A shares `binds.rs`, `user_settings.rs` and the menu (agree on one v4 bump). Track B shares `fpv_present.rs` and held-item drawing; also check remote players' third-person body.
- **Melee**: the knife (V) has the same hidden-viewmodel problem with an item. The `mw2_offhand_busy` helper could cover MeleeInit..MeleeEnd later.
