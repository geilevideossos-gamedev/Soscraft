# A2: chat editing and death messages

Owner, screenshot 4: "Ctrl+V only types 'v' (I wanted to paste /gamerule keepInventory true). Make editing work like vanilla: Ctrl+C, Ctrl+V, Ctrl+X, Ctrl+A, Ctrl+Backspace, the arrow keys, Home and End, and Up and Down for the messages I sent before." Also: "Death messages go to the Minecraft chat like vanilla (shown for a few seconds while the chat is closed), not to the MW2 kill feed on the left. The killcard and death animation stay."

**What to type now:** `/gamerule keep_inventory true`. 26.3 only knows the snake_case id: `GameRules.class` holds `keep_inventory`, and no class in the 26.3 jar contains the string `keepInventory`. Vanilla and the fork both answer the old name with red "Incorrect argument for command" (`minecraft_commands/src/args.rs:628` tests this).

## 1. What exists today

- **State.** `McChat` (`frame/src/minecraft_chat.rs:16-47`) holds `input`, a byte `cursor`, `history` (100 lines, this session only) and `draft`. It has no selection.
- **Input.** `update_chat` (`hud/src/minecraft_chat.rs:344-403`) reads `KeyboardInput` as `(logical_key, text)` and samples Ctrl once per frame (`:394`). `handle_frame` (`:113-132`) passes each key to `apply_key` (`:134-184`).
  - Already working: Enter, Esc, Up and Down (`move_in_history`, `:211-227`), PageUp and PageDown, Backspace and Delete (Ctrl for a word), Left and Right (Ctrl for a word), Home and End. `word_position` (`:259-279`) matches vanilla.
  - Every other key goes to `insert` (`:186-195`): `allowed_in_chat` filter (`minecraft_commands/src/protocol.rs:84`) and the 256 cap.
- **Why Ctrl+V types "v".** On Windows, winit fills `text` with the key's text without Ctrl (`winit-0.30.13/src/platform_impl/windows/keyboard.rs:234-253`), and `apply_key` inserts it. Ctrl+A, C and X type their letters too. winit drops AltGr's fake Ctrl (`:705-720`), so AltGr+Q (`@`) arrives with Ctrl off and right Alt on.
- **Clipboard.** The workspace bevy has `system_clipboard` (`Cargo.toml:100`). `Cargo.lock` has `bevy_clipboard 0.19.0`, which uses `arboard 3.6.1`. `DefaultPlugins` (`bootstrap/src/plugins.rs:109`) adds the `bevy::clipboard::Clipboard` resource. Its `fetch_text()` is ready at once on desktop and is read with `poll_result`; it also has `set_text`. The console uses it already (`console/src/plugin/mod.rs:1140-1190`). `Cargo.toml` needs no change.
- **Drawing.** `draw` (`:405-481`) draws the input box and the cursor. It has no selection.
- **Death to chat.** `minecraft_world.rs:1629-1646` builds `DeathLine` and pushes `death_message` (`render_anim/src/minecraft_chat.rs:319-324`, a `Text::literal`) into the chat when `show_death_messages` is on.
- **Death to the MW2 kill feed (two lines).**
  - `apply_block_world_damage` (`sim/src/damage.rs:225-232`) calls `push_print` with the line for every client. It shows as `KillfeedLine::Notify` (`hud/src/killfeed.rs:272-317`).
  - `minecraft_damage` (`sim/src/script/host/players.rs:74-90`) runs the gametype callbacks with a stand-in `trigger_hurt` killer (`minecraft_killer`, `:92-128`). The script's `obituary()` native (`natives/player.rs:527-551`) records `minecraft_obituary`, then `script_player::obituary` (`script_player.rs:355-386`) sends `EV_OBITUARY`. That shows as `KillfeedLine::Obituary` (`killfeed.rs:173-205`); nothing else reads it.
  - The card comes from `push_minecraft_death`, then `take_minecraft_deaths` (`minecraft_world.rs:652-655`), so it doesn't need the kill feed.
- **The chat is hidden while dead.** `ui.active = alive` (`minecraft_world.rs:1661`), and `update_minecraft_hud` (`hud/src/minecraft_inventory.rs:192-199`) then hides the whole Minecraft HUD, chat included. The death line fades unseen.

## 2. Vanilla 26.3 (javap)

- **`EditBox.keyPressed`** works on `KeyEvent.shortcutKey()`, the SDL keycode of the layout key.
  - Left and Right: with Ctrl (`hasControlDownWithQuirk`), `moveCursorTo(getWordPosition(±1), shift)`; without, `moveCursor(±1, shift)`.
  - Backspace and Delete: `deleteText(±1, ctrl)`.
  - Home and End: `moveCursorToStart` or `moveCursorToEnd(shift)`.
  - `isSelectAll`: `moveCursorToEnd(false)`, then `setHighlightPos(0)`. `isCopy`: `setClipboard(getHighlighted())`. `isPaste`: `insertText(getClipboard())`. `isCut`: copy, then `insertText("")`.
- **`InputWithModifiers.isSelectAll`, `isCopy`, `isPaste` and `isCut`:** the key is 'a', 'c', 'v' or 'x' (97, 99, 118, 120), Ctrl is down, and Shift and Alt are up.
- **`moveCursorTo(pos, extend)`:** unless extending, the highlight follows the cursor. A plain arrow with a selection steps from the cursor; it doesn't jump to the selection edge.
- **`deleteCharsToPos` and `deleteWords`:** with a selection, they delete just the selection.
- **`insertText`** replaces the selection. Room is `256 - len + selected`; if that is 0 or less, it does nothing. `StringUtil.filterText` drops `§` (167), everything below 32 (pasted newlines and tabs too) and 127. The text is cut to the room without splitting a surrogate pair, and both cursor and highlight end after it. Length counts UTF-16 units.
- **`KeyboardHandler.setClipboard("")`** does nothing.
- **Selection look.** `textHighlight(x0, textY-1, x1, textY+10, invert=true)` is a `GUI_INVERT` white fill, then an additive `0xFF0000FF` fill: a light box with blue text.
- **`ChatScreen`** has `setMaxLength(256)`. Up and Down call `moveInHistory`.
- **Saved command history.** `ChatComponent` loads `CommandHistory`: `command_history.txt` in the game folder, at most 50 lines, only lines starting with `/`, a repeat of the last line skipped.
- **`ServerPlayer.die`:** if `show_death_messages` is on, it sends `CombatTracker.getDeathMessage()` in `ClientboundPlayerCombatKillPacket`, plus `broadcastSystemMessage(msg, false)` to everyone, following the team visibility. If the rule is off, the packet carries `EMPTY`.
- **`Hud.extractChat`** draws whenever `player != null` and the HUD isn't hidden. It isn't gated on being alive, so the line is visible behind the death screen.

## 3. Commits

1. **"Minecraft chat selects text with Shift and Ctrl+A like vanilla"**
   - Add `highlight: usize` to `McChat`. `close`, `open` and `move_in_history` set it to the cursor.
   - New `hud/src/minecraft_edit.rs` (rustfmt-clean, pure): `move_to(pos, extend)`, `highlighted`, `insert` (replaces the selection), `delete_chars`, `delete_words` and `select_all` on `(input, cursor, highlight, limit)`. The anvil name box (`minecraft_inventory.rs:592-605`) can reuse it later.
   - Add `apply_key_with(chat, key, text, Mods { ctrl, shift, alt }, clipboard)`. `apply_key(.., ctrl)` stays as a wrapper, so the current tests keep working.
   - With Ctrl down and Alt up, no text is typed.
   - `draw` adds a light box over the selection and redraws the selected text in `0xFF0000FF` without shadow.
   - Update `MINECRAFT-SURVIVAL.md:93` and `:197`.
2. **"Minecraft chat copies, cuts and pastes with Ctrl+C, Ctrl+X and Ctrl+V like vanilla"**
   - A `ChatClipboard` trait for `bevy::clipboard::Clipboard` and a test `String`. `update_chat` takes `Option<ResMut<Clipboard>>`.
   - The shortcut letter is `logical_key` lowercased (Caps Lock safe). If that isn't ASCII, fall back to the physical `key_code`. Shift and Alt must be up.
   - Paste goes through `insert`. Copy or cut of an empty selection does nothing. Clipboard errors are logged with `diag`.
3. **"Minecraft chat remembers sent commands across restarts like vanilla"**
   - `<LaunchIdentity.artifacts>/command_history.txt` (`frame/src/session.rs:49-54`), next to `settings.cfg`, with vanilla's rules: 50 lines, only `/` lines.
4. **"Minecraft chat stays on screen while the player is dead like vanilla"**
   - `MinecraftUi.in_world` is set false at `minecraft_world.rs:561` and `:616`, and true beside `:1661`.
   - `update_minecraft_hud:192` draws only the unfocused chat when `in_world && !active`. The input still closes on death.
5. **"Minecraft death messages leave the MW2 kill feed"**
   - Remove `push_print` (`damage.rs:226-231`) and the `victim` local it no longer needs (`:192-194`). `push_minecraft_death` stays.
   - The `obituary` native still records `minecraft_obituary`, but skips `script_player::obituary` when the pure `hides_minecraft_obituary(minecraft_hit, attacker_arg, minecraft_killer)` holds, meaning the attacker is the stand-in.
   - A credited MW2 enemy still gets the normal kill feed line, killcam and card.
   - Update `MINECRAFT-SURVIVAL.md:40` and `:45`.

Order: 4 before 5, so the death line is never shown nowhere.

## 4. Unit tests

- **`minecraft_edit`:**
  - Shift with the arrows, Home and End extends. A plain arrow collapses vanilla-style.
  - Ctrl+Shift+Left selects a word.
  - Backspace, Delete and Ctrl+Backspace eat the selection first, and typing replaces it.
  - Insert room counts the selected length.
- **`minecraft_chat`:**
  - Ctrl+V with `"/gamerule keep_inventory true\r\n"` gives exactly that command, and `"a§b"` gives `ab`.
  - Paste at 250 characters with 10 selected stops at 256.
  - Ctrl+A then `x` gives `x`.
  - Copy with no selection leaves the clipboard alone.
  - Ctrl+X cuts.
  - Ctrl+B types nothing, while Ctrl+Alt with text `@` types `@`.
  - Ctrl+Shift+V doesn't paste, and an uppercase `V` with Ctrl does.
  - History keeps 50 `/` lines and skips a repeat.
- **`frame`:** `close` clears the highlight.
- **`sim`:** `hides_minecraft_obituary` is true only for a Minecraft hit credited to the stand-in.
- **`hud`:** a pure `minecraft_layers(active, in_world)` shows only the chat while dead.

## 5. In-game test steps

`.env`:

```
IW4L_MINECRAFT_KIT=lava_bucket,dirt*64,water_bucket
```

Start `iw4l-survival.exe map minecraft:overworld`.

1. In Notepad, copy `/gamerule keep_inventory true`. In game press T, then Ctrl+V. The line shows exactly that, with no `v`. Press Enter: "Game rule keep_inventory is now set to true". Screenshot 1.
2. Press T, type `/gamerule keepInventory true` and press Enter. You get red "Incorrect argument for command" and a grey line ending in `<--[HERE]`. Screenshot 2.
3. Press T and type `hello big world`. Press Ctrl+A: a light box with blue text. Screenshot 3. Press Ctrl+C, then `x` (the line becomes `x`), then Ctrl+V: `xhello big world`.
4. Press Ctrl+Shift+Left (selects `world`), then Ctrl+X, End and Ctrl+V. Next, press Shift+Home, then Left (the selection goes and the cursor moves one step). Then Ctrl+Backspace deletes a word, and Home and End jump. Screenshot 4.
5. Press Esc, then T, then Up: your last line. Up again gives older lines, and Down comes back to the draft.
6. Type `/kill @s` with AltGr+Q: the `@` appears. Ctrl+B types nothing.
7. Paste more than 256 characters: it stops at 256.
8. Restart, press T, then Up: `/gamerule keep_inventory true`.
9. Run `/gamerule keep_inventory false`, place lava and walk in. Bottom left shows "<name> tried to swim in lava" for about 10 s, also while dead. The MW2 kill feed stays empty, the Killed By card says "Lava", and the death animation plays. Screenshot 5.
10. Tower 20 dirt and jump off: "fell from a high place". Run `/summon minecraft:skeleton ~ ~ ~6`: "was shot by Skeleton". Screenshot 6.
11. Run `/gamerule show_death_messages false`, then `/kill`. No chat line and no kill feed line appear, but the card still shows.
12. Throw a frag at your feet. The MW2 kill feed still shows the MW2 line.

## 6. Risks and overlaps

- **Crate split.**
  - Editing is client-only (`frame`, `hud`) and doesn't move.
  - The death-to-chat code moves into `minecraft_survival`, so do commit 5's chat side after the split.
  - For multiplayer, `DeathLine` should keep the death key and the attacker id. The server then broadcasts `Text::translate` per `PlayerId`/UUID through `minecraft_net`, following the visibility rule, and each client resolves its own language.
- **Track C.** Tab completion takes Up, Down and Tab first, as `CommandSuggestions` does. Share `apply_key_with`.
- **Track B.** The anvil box needs `minecraft_edit`.
- **Phase 1 item 1.** The "Killed By FNG" investigation also touches `damage.rs` and the card.
- **Ctrl sampled once a frame.** A very fast Ctrl+V could still type "v". Folding Ctrl and Alt events in order fixes this.
- **Clipboard.** `ResMut<Clipboard>` serialises `update_chat` with the console systems. arboard fails while another program holds the clipboard; then paste does nothing.
- **Look and layout.**
  - The selection colours only approximate vanilla's blend.
  - The chat shown while dead may overlap the card or an MW2 killcam. Check screenshot 5.
- **Small differences from vanilla.**
  - Vanilla counts 256 in UTF-16 units; we count chars.
  - Rust's `is_control` also drops C1 characters.
- **Out of scope:** mouse selection, chat drafts and IME preedit.
