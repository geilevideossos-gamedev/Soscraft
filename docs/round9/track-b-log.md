# Round 9, track B log: items and the screen

Brief: `docs/round9/track-b.md`. Branch `track-b`, worktree `C:\Dev\mc-wt\track-b`, target dir `D:\iw4l-wt\track-b`, base 24dfdf71.

## Commits

### B10.1 A Minecraft hit shows a faint half-second red flash instead of MW2's lasting low-health blood

Base: 24dfdf71.

What changed (`crates/hud/src/blood.rs`):
- `BloodOverlayLatch` holds a `HurtFlash { last_event, hit_at }`. Its pure `observe(damage_event, now_ms)` takes the first damage count it sees as the baseline (no flash on joining or respawning), starts a flash whenever the replicated `ps.damage_event` changes (compared with `!=`, so a wrapped counter still counts) and returns `mc_hurt_flash(now - hit_at)`.
- `mc_hurt_flash(elapsed_ms)` is `MC_HURT_FLASH_PEAK * sin(t^4 * PI)` with `t = 1 - elapsed / 500`, and 0 outside 0..500 ms. `MC_HURT_FLASH_MS = 500`, `MC_HURT_FLASH_PEAK = 0.3`. The curve is vanilla's hurt-tilt curve (`GameRenderer.bobHurt`), so the flash peaks about 80 ms after the hit.
- `update_blood_overlay` reads `Option<Res<frame::MinecraftUi>>`. On a Minecraft map (`ui.active || ui.credits.is_some()`) the overlay alpha is the flash; health no longer feeds it. On MW2 maps `blood_overlay_lerp` runs exactly as before.
- The flash is reset on `LifeStarted`, when there is no player state, outside the game and when Show Blood is off. The `should_draw_blood_overlay` gate is unchanged, so Show Blood off hides the flash.

Tests added (hud): `the_first_damage_count_seen_does_not_flash`, `a_hit_flashes_faintly_and_fades_within_half_a_second`, `the_flash_peak_stays_far_below_the_mw2_low_health_overlay`, `a_wrapped_damage_counter_still_flashes`, `low_health_alone_shows_no_flash_on_minecraft`, `a_new_life_forgets_the_last_hit`. The two Show Blood tests are unchanged and green.

Decisions:
- **Not vanilla:** vanilla 26.3 has no full-screen red on a hit or at low health (`Hud.extractVignette` reads only the world border). The faint flash keeps MW2-style feedback at the owner's request. The peak is one constant, `MC_HURT_FLASH_PEAK`, for tuning.
- While the overlay is hidden for another reason (HUD hidden, a menu that hides the HUD, dead or spectating pm types) with Show Blood on, the flash still follows the damage count. A hit taken while hidden then shows at most the rest of its half second when the HUD comes back, instead of a full flash that starts late.
- An old `hit_at` is dropped once its half second is over, so a frame clock that restarts (a new map) cannot replay an old flash.
- Client-side only, nothing on the wire: the host and each friend read their own replicated `ps.damage_event` and their own `MinecraftUi.active`, so each window flashes only for its own hits, and Show Blood is per PC.

### B10.2 No MW2 pain vision on Minecraft maps

Base: f7cfddd4 (B10.1).

What changed (`crates/render_frontend/src/assemble/drawsurf/film_vision_view.rs`):
- `update_film_vision_view` reads `Option<Res<frame::MinecraftUi>>`.
- The pure `pain_wanted(health, scripted, minecraft)` is true only for a scripted pain vision at half health or less, off Minecraft maps. On a Minecraft map `pain_strength` stays 0, so the pain vision never blends in. MW2 maps keep the old rule, including the fade back down while healing.

Tests added (render_frontend): `no_mw2_pain_vision_on_minecraft_maps` (`(Some(0.1), true, true)` false, `(Some(0.1), true, false)` true, `(Some(0.9), true, false)` false, plus no health and no script both false).

### B9.1 Minecraft potion and tipped arrow icons show their vanilla potion colour

Base: 05578b70 (B10.2).

What changed:
- `minecraft_terrain/src/potions.rs`: `stack_color(components)` is `Some(PotionContents::from_components(..).color())` when the stack has `minecraft:potion_contents` (string or object form), else `None`. The colour maths is unchanged.
- New `minecraft_terrain/src/item_tint.rs`: an icon key is the item id plus `~rrggbb` when the stack has potion contents (`key`, `split`, `of_stack`, `KEY_MARK = '~'`). `split` takes the last `~`, so a dial key (`#`) stays whole for `dials::split` afterwards; a bad or empty suffix gives the id with no colour.
- `interface.rs` `item_model_tints(packs, model, potion)`: the `minecraft:potion` tint uses the stack colour, or its `default` (0x385DC6) without one. Dye, firework, map colour, team and custom model data still use their default. `mesh.rs` passes `None` at both callers for now (the world commit is next).
- `item_icons.rs` `item_icon_at`: splits the tint, then the dial, and tints layer 0 with the colour, so the potion overlay and the tipped arrow head take it.
- `frame::McStack.icon`: the icon atlas key. `hud/src/minecraft_gui.rs` looks icons up by `stack.icon`; names stay keyed by `stack.id`.
- `render_anim/src/minecraft_inventory.rs`: `convert` sets `icon = item_tint::of_stack(s)` and collects icon keys. The atlas loop names items by the base id and renders each icon from `item_tint::key(dials.key(base), potion)` (the small `icon_source` helper); the animated refresh uses `of_stack` too. `minecraft_creative.rs` `icon_ids` returns `stack.icon`. The gun handling is untouched.
- The friend's inventory goes through the same `publish`, so friends get the colours too.

Tests added:
- minecraft_terrain: `a_stack_colour_needs_potion_contents` (None without the component, F82423 for healing in both forms, a custom colour, water's default, strong_turtle_master 0x8D85E6); `item_tint::a_key_splits_back_into_its_id_and_colour`, `a_dial_key_is_left_for_the_dial_split`, `a_bad_suffix_has_no_colour`, `a_poison_splash_potion_keys_its_colour` (`minecraft:splash_potion~87a363`); `interface::a_potion_tint_takes_the_stack_colour_or_its_default` and two dye/firework asserts that ignore a colour; `item_icons::potion_and_tipped_arrow_icons_take_the_stack_colour_on_layer_zero` (on the real 26.3 pack: tinted differs from plain, and every bare layer-0 pixel equals the sprite times F82423/255, for the potion overlay and the tipped arrow head).
- render_anim: `two_potions_share_an_id_and_a_name_but_not_an_icon`.

Decisions:
- Every stack with potion contents gets its own atlas cell, even when its colour is the default (water, awkward). That is vanilla's tint result and costs one cell per potion kind; `ICON_ATLAS_LIMIT` has room.
- Dyed leather keeps its default tint (out of scope in the brief; the same key can carry it later).

### B9.2 Held, dropped and framed potions show their colour, also on other players

Base: cfe6e2a4 (B9.1).

What changed:
- `minecraft_terrain/src/mesh.rs`: `ItemVisuals::model(key)` splits the tint (`item_tint::split`) first, then the dial, and passes the colour to `item_model_tints`. The cache stays keyed by the full key, so each colour is its own cached visual. Dropped items look up `item_tint::of_stack(&entity.stack)`. `append_frames` looks the framed item up by `item_tint::key(id, view.potion)` for both `model` and `append_model`.
- `body_gear.rs`: `GearItem.potion: Option<u32>`, set in `GearItem::of` from `potions::stack_color`. `body_render.rs` `held_items`: only the `HeldItem.id` line changed, to `item_tint::key(&item.id, item.potion)`; `first_tint` stays `None` (it is the witch's). The test literals in `armor_render.rs`, `body_render.rs`, `body_gear.rs` and render_anim (`anim/minecraft_gear.rs`, `minecraft_world.rs`) gained `potion: None`.
- `item_frame.rs`: `FrameView.potion: Option<u32>`, set in `view()` from the framed stack. `FrameView` travels to friends inside the entity snapshot, so a friend's frames carry the colour too.
- `render_anim/src/minecraft_world.rs` first person: the posed item key is `item_tint::key(dials.key(&stack.id), potions::stack_color(..))`.
- Multiplayer: the net `GearItem` (`minecraft_net/src/messages.rs`) gained `potion: Option<u32>`, and `minecraft_survival/src/gear.rs` maps it both ways. `minecraft_net::PROTOCOL` goes from 3 to 4 (the track's only bump; `GearItem` and `FrameView` both changed on the wire). Literals fixed in `remote.rs`, `session.rs` and `codec.rs` tests.

Tests added:
- minecraft_terrain `mesh::dropped_and_framed_potions_take_their_stack_colour`: a synthetic pack (`minecraft:potion` with the `minecraft:potion` tint, default -13083194, on a one-pixel `builtin/generated` layer0). A dropped Healing potion's vertices are F82423/255 times their shade, a plain one 385DC6/255 times shade, and the two keys are cached apart; a framed (invisible frame) potion with `potion: Some(0xF82423)` and one with `None` give the same two colours.
- `body_gear::a_held_potion_carries_its_colour_and_a_sword_none` (Poison is `Some(0x87A363)`, a plain offhand potion and a diamond sword `None`).
- `body_render::a_held_potion_keys_its_colour` (`HeldItem.id` is `minecraft:potion~87a363`, `first_tint` stays `None`, a plain potion keeps its bare id).
- `item_frame::a_framed_potion_view_carries_its_colour` (Night Vision `Some(0xC2FF66)`, a plain potion and an elytra `None`).
- minecraft_net `codec::a_held_potion_colour_round_trips_on_protocol_4` (asserts `PROTOCOL == 4`), and `to_client()`'s `Equipment` now carries an offhand potion with `Some(0xF82423)` through the existing round trip.
- minecraft_survival `gear::a_held_potion_keeps_its_colour_both_ways` (`gear_view` then `gear_of`).

Decisions:
- The thrown bottle, the splash burst, the area cloud and the witch's potion were already right and are untouched.
- The held-item pose code in `body_render.rs` and the `mesh.rs` held-item append functions are track A's; only the `HeldItem.id` line and the `ItemVisuals::model` key split changed there.

### B8.1 Minecraft written books are read from the hand like vanilla

Base: bd7a398d (B9.2).

What changed:
- New `minecraft_survival/src/books.rs` (registered in `lib.rs`): `BookUse { Read, Write }`, `OpenBook { slot, kind }`, `is_book(stack)` (written book or book and quill) and `book_use(stack)`: a written book with `minecraft:written_book_content` reads, a book and quill writes with or without components, anything else is `None`.
- `actions.rs`: `Acted.open_book: Option<OpenBook>`. A new arm in `use_hands` right after the `used` arm, so frames, mobs, tables, stations, lecterns, campfires, respawn blocks and doors keep priority. When the hand holds either book, a press (`use_pressed`) sets `open_book` (slot = the selected slot, or 40 for the offhand), swings that hand and the arm returns `Consumed`; a held use that repeats every 4 ticks consumes without opening again. A written book without content only swings. Spectators never reach `use_hands` (`game_mode.interacts()`); adventure players do.
- `render_anim/src/minecraft_inventory.rs`: `lectern: Option<BlockPos>` became `book_source: Option<BookSource>` with `BookSource { Lectern(BlockPos), Held(usize) }` (named `book_source` because `InventoryUi.book` is already the station recipe book). `lectern()` still returns only the lectern case. New `show_held_book(ui, slot, McBook)` opens `McScreen::Lectern` (refused while another screen is open). For a held source `Choose(1)`/`Choose(2)` turn `ui.book.page` locally, clamped to the pages like `BookViewScreen`, `Choose(3)` is ignored and Close sends no `LecternAction`. `lectern_clicks` is renamed `book_clicks` (the friend path in `minecraft_remote.rs` keeps bypassing `show_menus` with it).
- `minecraft_world.rs`: the host destructures `open_book`; after the station block, when no screen is open and the kind is Read, it shows the slot's pages (`lectern::pages_text`) with `page: 0, take: false`. The hud's existing `take: false` layout is vanilla's held `BookViewScreen`: one Done button 200 wide at (w/2-100, 196) and no Take Book.
- `minecraft_players.rs`: `friend_hand` returns `acted.open_book` and `step_friend` stores it in `Stepped.open_book` (sent to the friend by a later group).

Tests added:
- minecraft_survival `books::a_written_book_reads_and_a_quill_writes`.
- minecraft_survival `actions`: `right_clicking_with_a_written_book_opens_it_for_that_hand` (slot 3, swing, the book stays; a written book without content swings and opens nothing), `a_quill_in_the_offhand_opens_the_editor_for_slot_40` (offhand swing), `a_lectern_still_takes_a_held_book_before_it_opens` (no `open_book`, the book goes to the lectern), `a_spectator_does_not_open_a_book` (and adventure does), `holding_use_opens_a_book_only_once`.
- render_anim `a_held_book_turns_pages_locally_and_closes_without_a_lectern_action`. The lectern tests stay green after the rename.

Decisions:
- A book and quill (`BookUse::Write`) opens nothing yet on the host: the edit screen comes with the B8.3 group. Right-clicking one swings the hand, as vanilla does before its screen opens.
- The held view reuses `McScreen::Lectern` and `McBook`, so the lectern code and its tests are unchanged.
- Page turns with PageUp/PageDown play no sound, as for the lectern (vanilla's `PageButton.onPress` from a key plays none either); clicking an arrow plays `item.book.page_turn`.

### B8.2 Minecraft books and quills keep their pages and sign into written books on the host

Base: d2d4c403 (B8.1).

What changed:
- `minecraft_survival/src/books.rs`: `edit_book(inventory, slot, pages, title, author) -> bool`, following vanilla's `ServerGamePacketListenerImpl.handleEditBook`:
  - refused with no change when the slot is not 0-8 or 40, there are more than 100 pages (`MAX_PAGES`), a page is over 1024 characters (`PAGE_LENGTH`) or the title is over 32 (`TITLE_LENGTH`), or the slot does not hold `minecraft:writable_book`;
  - pages and title lose `§` and control characters other than `'\n'` (vanilla's `StringUtil.filterText(s, true)` rule: below space and DEL);
  - without a title the stack keeps its other components and gets `minecraft:writable_book_content = {"pages":[{"raw":p},...]}`; no pages removes the key, and an empty component map becomes `None`;
  - with a title the same stack (count and other components kept) becomes `minecraft:written_book`: the writable content is removed and `minecraft:written_book_content = {"title":{"raw":t},"author":author,"pages":[{"raw":p}...],"resolved":true}` is written, with no `generation` key (0, "Original", as `book.rs` saves it) and no `pages` key when there are none; `max` comes from the recipe book (16 with the item catalog).
- `frame`: `McClick::EditBook { pages: Vec<String>, title: Option<String> }`. `action_of` maps it to no menu action.
- `render_anim/src/minecraft_inventory.rs` `apply_input`: for a held book (`BookSource::Held(slot)`) an `EditBook` calls `edit_book(&mut player.inventory, slot, .., &player.name)` and closes the book screen; a lectern ignores it. Nothing pushes `EditBook` yet: the edit and sign screens (B8.3) and the friend's `ToServer::EditBook` (B8.4) do.

Tests added:
- minecraft_survival `books`: `editing_a_quill_stores_its_pages_and_an_empty_one_has_no_component` (custom name kept), `signing_turns_a_quill_into_an_original_written_book` (offhand slot 40, exact component, no generation key, trailing empty pages kept, custom name and count kept, max equals the recipe book's and is 16 when the 26.3 item catalog is found, a signed book refuses further edits), `book_edits_follow_vanilla_slot_and_size_rules` (slots 9 and 39 refused, 0, 8 and 40 accepted, an empty slot, a written book and stone refused, 101 pages, a 1025-character page and a 33-character title refused with no change, 100 pages and 1024 two-byte characters accepted, `§`, `\u{1}`, DEL and tab dropped with `'\n'` kept, the title filtered too), `a_signed_book_survives_the_item_nbt_round_trip` (`item_nbt::stack_tag` then `stack_of`: `resolved` comes back as 1, `pages_text` reads both pages).
- render_anim `a_held_book_edit_reaches_the_inventory_slot` (pages, then signing with the player's name, then a lectern ignoring the edit).

Decisions:
- **Not vanilla (hardening only):** the host filters `§` and control characters itself. Vanilla's server trusts its client's filter; a friend's client is ours, so the result is the same.
- The title is filtered but not trimmed on the host; the sign screen (B8.3) trims it before sending, as vanilla's client does.
- No in-game steps for this commit alone: until B8.3 there is no screen that sends an edit.

### B8.3 Minecraft books and quills open vanilla's edit and sign screens

Base: 02da42c3 (B8.2).

What changed:
- `frame`:
  - `McScreen::BookEdit` and `McScreen::BookSign`.
  - `MinecraftUi.book_edit: Option<McBookEdit { pages, page, cursor, highlight, title, title_cursor, title_highlight, author, since }>` (`title_highlight` added so the title box can select like vanilla's `EditBox`).
  - `McFont::split_ranges(text, width) -> Vec<Range<usize>>` (byte ranges, same wrapping rules); `split` now maps over it.
- New `hud/src/minecraft_book_edit.rs` (staged content passes the edition 2024 rustfmt check with empty output): layout, input and drawing of both screens with the vanilla 26.3 numbers.
  - BookEdit: book at ((w-192)/2, 2); `MultiLineEditBox` area ((w-114)/2-8, 28, 122x134), text origin +4; `book.pageIndicator` right-aligned at (left+148, top+16); back arrow (left+43, top+157) only when page > 0; forward (left+116, top+157) always, appending an empty page on the last page while there are fewer than 100; Sign (`book.signButton`) at w/2-100 and Done (`gui.done`) at w/2+2, 98x20 at y 196.
  - Typing through `minecraft_chat::fold_typed` and `minecraft_edit::Edit`; new `Edit::multiline(self)` lets `'\n'` through `insert`, so Enter and keypad Enter add a line. Every insert, paste, cut and delete runs on a copy and is refused whole when the copy needs more than 14 display lines (114 px, +1 when it ends in `'\n'`); 1024 characters per page.
  - Keys: Left/Right (Ctrl by word), Up/Down by display line through `split_ranges` (vanilla's `seekCursorLine`: x of the cursor + 2, then `plainSubstrByWidth` on the target line), Home/End per display line (Ctrl: whole page), Shift extends the selection, Ctrl+A/C/V/X through the chat clipboard, Backspace/Delete (Ctrl by word), PageUp/PageDown and the pad triggers press the arrows. Clicking in the text moves the cursor (Shift-click selects).
  - Cursor: a black 1 px bar from y-1 to y+10 inside the text and `_` after the last character, blinking at 300 ms from when the screen opened.
  - Done trims trailing empty pages, pushes `EditBook { title: None }` and `Close`. Esc pushes only `Close` (the generic hud Escape), which discards the edits as vanilla does.
  - BookSign: `book.editTitle` centred in [left+36, left+150] at y 34; the title box at ((w-114)/2-3, 50), centred, black, at most 15 characters; `book.byAuthor` dark gray centred at y 60; `book.finalizeWarning` wrapped to 114 at (left+36, 82); "Sign and Close" (`book.finalizeButton`) drawn with `widget/button_disabled` and inactive while the title is blank; Cancel (`gui.cancel`) goes back to BookEdit with the title kept. Enter with a non-empty title (or the button) pushes `EditBook { pages with trailing empties kept, title: Some(trimmed) }` and `Close`.
- `hud/src/minecraft_inventory.rs`: the `typing` gate covers BookEdit and BookSign (E, Q and the digit keys type text); a draw and input branch sits beside the lectern branch.
- `hud/src/minecraft_screens.rs`: size [192,192] and sheet `book` for both screens.
- `render_anim`:
  - `minecraft_gui.rs`: `gui.cancel` added to `TEXT_PREFIXES` (the atlas test now expects it).
  - `minecraft_inventory.rs`: `show_book_editor(ui, slot, pages, author)` sets `BookSource::Held(slot)` and `ui.book_edit`, clips each page to 1024 characters and starts a book with no pages on one empty page, cursor at the end of page 1 (refused while another screen is open). `book_screen` keeps BookEdit/BookSign shown while an editor is open; `close_book` also clears `book_edit`. `EditBook` goes through the existing `apply_input` path to `books::edit_book`.
  - `minecraft_world.rs`: on `OpenBook { kind: Write }` with no screen open, the host opens the editor with the slot's pages (`lectern::pages_text`) and the player's name.

Tests added:
- frame `split_ranges_match_split` (13 texts at 7 widths against the old string-based `split`, ordered and in-bounds ranges, exact ranges for 5 cases). `pages_wrap_at_114_pixels_and_14_lines` stays green.
- hud `minecraft_edit`: the existing filter test also checks that `multiline()` keeps `'\n'`.
- hud `minecraft_book_edit`: `edit_layout_matches_vanilla` (width 427: text origin (152,32), back (160,159,23,13), forward (233,159,23,13), Sign [113,196,98,20], Done [215,196,98,20], title box x 153 y 50, the warning at (153,82), the back arrow hidden on page 1), `typing_stops_at_14_lines_and_1024_chars` (Enter stops at 13 newlines, a 25-letter line stops at 19, an overflowing paste is refused whole, 1024 characters, `§` and tab dropped), `enter_inserts_a_newline_and_up_down_move_by_display_line` (also Home/End per wrapped line, Ctrl+Home/End, Ctrl+Left/Right, Ctrl+Backspace, keypad Enter, a click in the text), `forward_on_the_last_page_adds_one_up_to_100`, `done_trims_trailing_empty_pages_and_esc_discards`, `sign_and_close_waits_for_a_title_and_cancel_keeps_it` (blank title refused, 15-character limit, Cancel keeps the title and puts the cursor at the page end, Enter and the button sign with the trimmed title and trailing empty pages kept), `typing_in_a_book_does_not_toggle_the_inventory`, `copy_cut_and_select_all_use_the_clipboard`.
- render_anim `a_book_and_quill_opens_the_editor_and_keeps_its_screen` (empty book starts with one page, refused over another screen, BookSign survives `apply_input`, Esc keeps the quill unchanged, a 1100-character page is clipped to 1024, Done writes the pages into the offhand quill).

Checked with javap (26.3):
- `TextCursorUtils.extractInsertCursor(g, x, y, color, 9 + 1)` fills (x, y-1, x+1, y+10); `extractAppendCursor` draws `_`; `isCursorVisible(ms)` is `ms / 300 % 2 == 0`. The insert x is inner left + `font.width(line start..cursor)`.
- `BookSignScreen.keyPressed` signs on Enter when the title box value is not empty (`isEmpty`, not `isBlank`), so a title of only spaces signs with the empty trimmed title, as in vanilla. The button stays inactive for a blank title (`StringUtil.isBlank`).
- `BookEditScreen.updatePageContent` calls `MultiLineEditBox.setValue(page, true)`, which puts the cursor at the page end; the screen is re-initialised when Cancel returns to it, so Cancel puts the cursor at the end of the page.

Decisions:
- Cancel on the sign screen puts the edit cursor back at the end of the page (vanilla re-initialises the edit screen).
- **Not vanilla (approximation):** a selection is drawn as a solid blue box with white text. Vanilla's `textHighlight` uses a colour-logic highlight pipeline the hud painter does not have.
- The editor needs no scrolling: 14 lines of 9 px fit the 126 px inner height, as in vanilla.
- Switching between BookEdit and BookSign restarts the cursor blink and drops input for that one frame (the hud treats a screen change as a fresh open).
- A friend cannot open the editor yet: `ToClient::OpenBook` and `ToServer::EditBook` come with the B8.4 group.

## Owner's test steps (B9.1)

1. **Inventory (E).** Each bottle has its own liquid colour:
   - Healing is red, Poison olive green and Swiftness cyan;
   - Fire Resistance is orange, Night Vision lime and Strong Turtle Master lavender blue;
   - Invisibility is almost white, Leaping pale yellow and Weakness dark grey;
   - Water and Awkward keep the old blue.

   The splash and lingering bottles and the tipped arrows' heads are coloured the same way. Hovering still shows each potion's name.
2. **Hotbar and cursor.** Potions in the hotbar and one carried on the cursor keep their colours.
3. **Creative.** In `/gamemode creative`, the potions in Food & Drinks and the tipped arrows in Combat each show their own colour.

Held, dropped and framed potions still show the old blue until the next commit (B9.2).

```
IW4L_MINECRAFT_KIT=minecraft:potion@healing,minecraft:potion@poison,minecraft:potion@swiftness,minecraft:potion@fire_resistance,minecraft:potion@night_vision,minecraft:potion@strong_turtle_master,minecraft:potion@invisibility,minecraft:potion@water,minecraft:potion@awkward,minecraft:potion@leaping,minecraft:potion@weakness,minecraft:splash_potion@harming*2,minecraft:splash_potion@regeneration*2,minecraft:splash_potion@poison,minecraft:lingering_potion@poison*2,minecraft:lingering_potion@slow_falling,minecraft:tipped_arrow@poison*16,minecraft:tipped_arrow@strength*16,minecraft:tipped_arrow@leaping*16,minecraft:item_frame*4,minecraft:cow_spawn_egg*2
```

## Owner's test steps (B9.2)

Numbered as in the brief's B9 steps (1-3 are B9.1's, 7 and 11 come with the swirls).

**Host alone:**

4. **Held.** A held Healing potion is red in first person, in the offhand (F) and in third person (F5). Poison held in first person is olive green; Water stays the old blue.
5. **Dropped and framed.**
   - Press Q on Poison and Fire Resistance: they lie green and orange on the ground.
   - Place an item frame and put a Poison tipped arrow in it: the head is green. A Night Vision bottle in a second frame is lime.
6. **Thrown.** A thrown Splash Harming bottle and its burst are dusky red. Lingering Poison leaves a green cloud. (Unchanged, a check that nothing broke.)

**Multiplayer** (both games must be this build: protocol 4; an older friend build is refused at the join):

8. The friend's potions are coloured in their inventory, hotbar and first-person hand.
9. Each player sees the other's held potion in its colour: the host holds Healing (red) in the main hand and Poison (green) in the offhand, and the friend sees both; then the other way round.
10. The friend sees the host's dropped Poison and Fire Resistance and the framed Night Vision bottle and Poison arrow in their colours.

```
IW4L_MINECRAFT_KIT=minecraft:potion@healing,minecraft:potion@poison,minecraft:potion@swiftness,minecraft:potion@fire_resistance,minecraft:potion@night_vision,minecraft:potion@strong_turtle_master,minecraft:potion@invisibility,minecraft:potion@water,minecraft:potion@awkward,minecraft:potion@leaping,minecraft:potion@weakness,minecraft:splash_potion@harming*2,minecraft:splash_potion@regeneration*2,minecraft:splash_potion@poison,minecraft:lingering_potion@poison*2,minecraft:lingering_potion@slow_falling,minecraft:tipped_arrow@poison*16,minecraft:tipped_arrow@strength*16,minecraft:tipped_arrow@leaping*16,minecraft:item_frame*4,minecraft:cow_spawn_egg*2
```

## Owner's test steps (B10)

1. Options > Advanced Video: check that Show Blood is on.
2. **One hit.** Let a zombie hit you once: a faint red flash, gone in about half a second, with the view kick and the blinking hearts.
3. **Low hearts.** Go down to 1-2 hearts with the zombie and a cactus. Between hits the screen is completely clear (the screenshot 11 case).
4. **Standing at 1 heart.** Stand still at 1 heart for 10 seconds: the screen stays clear, with no red and no pain vision, and only the hearts jitter.
5. **Healing.** Eat golden apples: nothing changes on screen.
6. **Show Blood off.** Turn Show Blood off and step onto a magma block: no flash. Turn it back on.
7. **The totem.** Hold the totem and survive a lethal fall: at most one short flash.
8. **Respawning.** Die and respawn: no flash is replayed.
9. **A friend.**
   - When the friend is hit, only the friend's window flashes.
   - The friend's screen is clear at 1 heart.
   - The friend's own Show Blood switch works only on their PC.
10. **MW2.** A plain MW2 map still has MW2's low-health blood and pain vision.

```
IW4L_MINECRAFT_KIT=minecraft:zombie_spawn_egg*8,minecraft:cactus*4,minecraft:magma_block*4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:totem_of_undying,minecraft:cobblestone*64,minecraft:ladder*32,minecraft:diamond_sword,minecraft:shield
```

## Owner's test steps (B8)

Numbered as in the brief's B8 steps. Steps 2 and 9 (the tooltip and the friend) come with later groups of this track; until the tooltip group lands, the signed book's name and "by"/"Original" lines in step 4 are not shown yet. B8.2 (`edit_book`) is tested through steps 3-5.

1. **Reading from the hand.** Hold "Test" and right-click at the sky.
   - "Page 1 of 3" opens, with only the forward arrow and one wide Done button (no Take Book).
   - The arrows and PageUp/PageDown turn the pages; clicking an arrow plays the page sound.
   - Done and Esc close it.
   - Press F to move it to the offhand, select an empty hotbar slot and right-click: it opens again.
   - Holding right-click does not reopen it after Done.
3. **Writing.** Right-click an empty book and quill.
   - The book opens with a blinking `_` cursor, "Page 1 of 1", only the forward arrow, and the Sign and Done buttons.
   - Type two sentences and press Enter (also keypad Enter) for a new line. Text wraps at the right edge of the page. E, Q and the number keys type letters and do not close the book.
   - Hold a letter key: typing stops when the page has 14 lines.
   - Left/Right, Up/Down (by wrapped line), Home/End, Ctrl+A, Ctrl+C and Ctrl+V work; clicking in the text moves the cursor.
   - Click the forward arrow: "Page 2 of 2" and the back arrow appears. Type on it, then click Done and right-click the quill again: both pages are kept.
   - Type more and press Esc, then reopen: the new text is gone, as in vanilla.
   - Right-click the second book and quill ("Quill page one"): it opens on its own text, and PageDown/PageUp turn its pages.
4. **Signing.** In the first quill, click Sign.
   - "Enter Book Title:", "by <your name>" in dark gray and the warning text are shown, with "Sign and Close" and "Cancel".
   - "Sign and Close" is grey and does nothing until there is a title; the title stops at 15 characters.
   - Type "Diary", click Cancel: the edit screen is back. Click Sign again: "Diary" is still there.
   - Click "Sign and Close". The quill becomes a written book in the same slot, and right-clicking it reads both pages.
5. **Crafting.** Craft a book and quill from a book, an ink sac and a feather at the crafting table, then write in it and sign it (Enter in the title box also signs).
6. **Lecterns.**
   - Place a lectern and right-click it with "Test": the book goes onto the lectern instead of opening in the hand.
   - Put a comparator behind the lectern, redstone and a lamp after it. Page turns change the signal, and Take Book works.
   - The book and quill with pages ("Quill page one") goes onto the second lectern and reads there.
7. **Saving.** Save, quit and reload: the lectern books and the written book are kept.
8. **Game modes.**
   - `/gamemode adventure`: right-clicking "Test" reads it.
   - `/gamemode spectator`: right-clicking with a book does nothing.

```
IW4L_MINECRAFT_KIT=minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}],minecraft:writable_book[writable_book_content={pages:["Quill page one","Quill page two"]}],minecraft:writable_book,minecraft:writable_book,minecraft:lectern*2,minecraft:comparator,minecraft:redstone*16,minecraft:redstone_lamp,minecraft:book*2,minecraft:ink_sac*2,minecraft:feather*2,minecraft:crafting_table,minecraft:stone*32
```

## State doc changes

`docs/MINECRAFT-SURVIVAL.md` line 431 (MW2 perks and scripts that heal or resize health): keep the first sentence and replace the last one ("Since MW2's maximum health stays at its spawn value, MW2's blood overlay and pain vision stay on while the hearts are low.") with:
"MW2's low-health blood overlay and pain vision no longer show on Minecraft maps, for the host or for friends; MW2 maps keep both. Instead a hit gives a faint red flash of about half a second (MW2-style feedback, not vanilla: vanilla has no screen red), and Show Blood (Options > Advanced Video) turns it off."

`docs/MULTIPLAYER.md` line 162 (the `PROTOCOL` constant in the message listing): `pub const PROTOCOL: u16 = 4;`. Line 1415: "... `minecraft_net::PROTOCOL` in `Hello` is the check that counts, and it is 4 (E3, E14, E22; round 9 track B, B9.2: held potion colours in `GearItem` and `FrameView`)." Section 20 or the gear section: the net `GearItem` carries `potion: Option<u32>` (the stack's potion colour) next to `dye`, so other players' held potions are drawn in their colour.

`docs/MINECRAFT-SURVIVAL.md` line 437 (books), after B8.1 and B8.2: replace "and a book can't be read from the hand." with "A written book is read from the hand (main hand or offhand) on vanilla's book screen with one wide Done button; page turns are local." Keep "Books can't be written or signed in game" until the B8.3 group lands, and add: "The host already stores a book and quill's pages and signs it into an original written book (`books::edit_book`, vanilla's `handleEditBook` limits)."

After B8.3, replace "Books can't be written or signed in game" with: "The host writes in a book and quill on vanilla's edit screen (14 lines and 1024 characters a page, up to 100 pages; Done keeps the pages, Esc discards them) and signs it on vanilla's sign screen into an original written book. Not vanilla: a text selection is a plain blue box (vanilla uses an inverting highlight). Friends cannot write yet (B8.4)."
