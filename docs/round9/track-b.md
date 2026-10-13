# Track B: items and the screen

The owner tested round 8's release 2 (03e0c9be). Screenshots are in `scrennshots/screenshots-03e0c9b/screenshots-03e0c9b/` (untracked, in `C:\Dev\mashup-crafting`). Screenshot N is the file whose name starts with N. Track B has three items:

- **B8:** books.
- **B9:** potion colours.
- **B10:** the damage overlay.

Track B runs beside tracks A (multiplayer fixes), C (guns and grenades as items) and D (finishing survival). It is merged together with track A before the round's first release build.

Paths and line numbers are as of f1dddf78, so re-check them before editing. Vanilla facts were checked with javap on the 26.3 `client.jar`. No Mojang code or data is copied.

Every section below has the same parts: the owner's finding, the cause, what vanilla does, the plan, the tests, and the owner's in-game steps with a kit line. The commit subjects are the ones listed in each plan.

---

## B8. Books: read from the hand, write, sign, lecterns

### The owner's finding
The owner's words: "readable written books, books and quills, lecterns". There is no screenshot. Release 2's kit (`docs/round8/release-2-tests.md:157`) gives a written book "Test", and right-clicking it does nothing.

### What already exists (do not rebuild)
- **Lecterns.** Both book kinds work for the host and for friends (round 8 track C groups 8 and 9, `docs/round8/c5-limitations.md:294-345`):
  - putting a book down (`actions.rs:718-735`, `:827-856`);
  - the server menu, page turns with a 2-tick pulse and the comparator output;
  - Take Book, the drop on break and saving;
  - friends through `ToServer::Lectern` and `ToClient::Lectern(BookView)`.
- **The read-only book screen.** `crates/hud/src/minecraft_book.rs`, with layout tests against vanilla.
- **Book components.** `/give` and the kit already take `writable_book_content` and `written_book_content` (`third_party/minecraftoss/world/src/book.rs`, `args.rs:953-1004`, `crafting.rs:696`).

### Cause
1. **No hand-use branch for books.** In `crates/minecraft_survival/src/actions.rs` the `use_hands` closure (`:641-1480`) checks these in order: frames, mobs, table, station, workshop, campfire, lectern, `used` (`:909`), then the item uses.
   - No arm matches `minecraft:written_book` or `minecraft:writable_book`, and a book is not placeable, so it falls through to `HandUse::Passed` (`:1478`).
   - `Acted` (`:117-126`) has no field that asks for a book screen.
   - The same `act` runs for the host (`render_anim/src/minecraft_world.rs:951`) and for each friend on the host (`render_anim/src/minecraft_players.rs:1693`), so both are affected.
2. **The UI only knows lectern books.**
   - `McScreen` (`frame/src/minecraft_ui.rs:65-82`) has only `Lectern`, and `McBook` (`:93-97`) is read-only.
   - `InventoryUi` tracks only `lectern: Option<BlockPos>` (`render_anim/src/minecraft_inventory.rs:49-50`, `:165-212`, `:238-273`), and every page button becomes a server `LecternAction`.
3. **Nothing writes book content.** These are all missing:
   - an edit screen and a sign screen;
   - a `McClick` for the edit;
   - a host function that does what vanilla's `handleEditBook` does;
   - a `ToServer` message for a friend's edit.

   `docs/MINECRAFT-SURVIVAL.md:437` lists this as a limitation.
4. **Smaller gaps:**
   - The name and tooltip ignore `written_book_content`: there is no title as the name, no "by <author>" and no generation line (`display_name`, `render_anim/src/minecraft_inventory.rs:716`).
   - `gui.cancel` is missing from `TEXT_PREFIXES` (`render_anim/src/minecraft_gui.rs:69-81`).
   - The hud text `Edit` drops `'\n'` (`hud/src/minecraft_edit.rs:63-66` filters with `allowed_in_chat`).
   - `McFont::split` (`frame/src/minecraft_gui.rs:69`) returns strings, not byte ranges, so a multi-line cursor cannot be placed.

### Vanilla 26.3
**Using a book from the hand.**
- `WrittenBookItem.use` and `WritableBookItem.use` both call `player.openItemGui(stack, hand)`, award `ITEM_USED`, and return SUCCESS, which swings the hand.
- On the server, `ServerPlayer.openItemGui` only acts when the stack has `WRITTEN_BOOK_CONTENT`. It then sends `ClientboundOpenBookPacket(hand)`. A written book without content opens nothing, but the hand still swings.
- The client opens `BookViewScreen` for a written book. For a book and quill, `LocalPlayer.openItemGui` opens `BookEditScreen` on the client alone.
- Spectators cannot use items. Adventure players can.
- The held `BookViewScreen` has one Done button, 200 wide, at (w/2-100, 196). Page turns are local. Otherwise it is laid out like the lectern screen that already exists.

**BookEditScreen.**
- Layout:
  - background `gui/book.png`, 192x192, at ((w-192)/2, 2);
  - a `MultiLineEditBox` at x = (w-114)/2-8, y = 28, size 122x134, inner padding 4;
  - the text area is 114 wide, with black text and cursor (-16777216), no shadow and no background.
- Limits:
  - 1024 characters per page;
  - 14 lines (126/9). An insert that would make `splitLines(value, 114).size` exceed 14 is rejected whole (+1 line if the text ends in `'\n'`).
- Input goes through `StringUtil.filterText(s, true)`, which keeps `'\n'` and drops `§` and control characters.
- Page indicator: `book.pageIndicator`, right-aligned at (left+148, top+16).
- Arrows:
  - Back is a `PageButton` at (left+43, top+157), shown only when page > 0.
  - Forward is at (left+116, top+157) and always shown. On the last page it appends an empty page (while there are fewer than 100) and moves to it.
  - PageUp and PageDown press them.
- "Sign" (`book.signButton`) is at (w/2-100, 196) and Done (`gui.done`) at (w/2+2, 196), both 98 wide.
- **Done:**
  - erases trailing empty pages;
  - sends `ServerboundEditBookPacket(slot, pages, Optional.empty())`, where the slot is the selected slot for the main hand and 40 for the offhand.
- **Esc closes without saving.** The screen has no `onClose` or `removed` override.

**BookSignScreen.**
- The title box:
  - an `EditBox` at ((w-114)/2-3, 50), 114x20;
  - unbordered, centred, black, no shadow;
  - at most 15 characters.
- "Enter Book Title:" (`book.editTitle`) is centred in [left+36, left+150] at y 34.
- "by <name>" (`book.byAuthor`, dark gray) is centred at y 60.
- `book.finalizeWarning` is wrapped to 114 at (left+36, 82).
- Buttons:
  - "Sign and Close" (`book.finalizeButton`) at (w/2-100, 196), 98 wide, active only while the title is not blank;
  - "Cancel" (`gui.cancel`) at (w/2+2, 196), which goes back to the edit screen and keeps the title.
- Enter with a non-empty title signs. Signing sends `ServerboundEditBookPacket(slot, pages, Optional.of(title.trim()))`. Trailing empty pages are **not** erased here.
- The cursor blinks while (millis - focusedTime) / 300 % 2 == 0.

**Server: `handleEditBook`.**
- Limits:
  - at most 100 pages, 1024 characters each;
  - a title of at most 32 characters;
  - the slot must be 0-8 or 40.
- Edit: the stack must have `WRITABLE_BOOK_CONTENT`. It is set to the new pages, and an empty list means the default, so no component is written.
- Sign: the stack must have `WRITABLE_BOOK_CONTENT`. Then:
  - `transmuteCopy(WRITTEN_BOOK)` keeps the other components and the count, and `WRITABLE_BOOK_CONTENT` is removed;
  - it gets `WrittenBookContent(title, author = player.getPlainTextName(), generation 0, pages, resolved true)`;
  - the book goes back into the same slot.

**Name and tooltip.**
- A written book is named by its title when it has no `custom_name` and the title is not blank. The name is not italic.
- The tooltip adds "by %s" in gray when the author is not blank. It then always adds `book.generation.N` in gray, so generation 0 shows "Original".

### Plan
1. **"Minecraft written books are read from the hand like vanilla"**
   - **`minecraft_survival`:**
     - A new module `books.rs`, registered in `lib.rs`, with `pub enum BookUse { Read, Write }` and `pub struct OpenBook { pub slot: usize, pub kind: BookUse }`.
     - `pub fn book_use(stack) -> Option<BookUse>`. A written book with `minecraft:written_book_content` gives Read. A writable book gives Write, with or without components. Anything else gives None.
   - **`actions.rs`:**
     - Add `pub open_book: Option<OpenBook>` to `Acted`, and `let mut open_book = None;` next to `open_table` (`:326`).
     - Add a new arm right after the `used` arm (`:909`), so doors and lecterns keep priority. When the held stack is either book and `game_mode.interacts()`:
       - on `use_pressed`, set `open_book` from `book_use`;
       - swing the hand;
       - return `HandUse::Consumed`.
     - A written book without content still swings and does nothing else.
     - Fill the field in the `Acted` construction (`:1494-1503`).
   - **`render_anim`:**
     - Destructure `open_book` at `minecraft_world.rs:951`.
     - After the `open_station` block (`:1290-1296`), when the inventory is not open and the kind is Read, call `inventory_ui.show_held_book(...)` with `McBook { pages: lectern::pages_text(components), page: 0, take: false }`.
   - **`minecraft_inventory.rs`:**
     - Replace `lectern: Option<BlockPos>` with `book: Option<BookSource>`, where `enum BookSource { Lectern(BlockPos), Held(usize) }`. Keep `lectern()` returning only the Lectern case, so existing callers and tests keep working.
     - `show_held_book` sets `McScreen::Lectern`, `inventory_open` and `screen_opened`.
     - For a Held source:
       - `Choose(1)` and `Choose(2)` turn pages locally on `ui.book.page`, clamped like `BookViewScreen.setPage`;
       - `Choose(3)` (Take Book) is ignored;
       - Close clears the book and sends no `LecternAction`.
     - Rename `lectern_clicks` to `book_clicks`. It stays true for any book screen, so `minecraft_remote.rs:1012` keeps bypassing `show_menus`.
   - **`minecraft_players.rs`:** carry `acted.open_book` out of the friend step (`:1741-1745`) into `Stepped` as `open_book`. Group b8-friend sends it.
   - The read-only view keeps the `McScreen::Lectern` variant, with Done drawn wide and Take Book hidden when there is no lectern, as vanilla's held view does. This leaves round 8's lectern code and tests alone.
2. **"Minecraft books and quills keep their pages and sign into written books on the host"**
   - `books.rs`: `pub fn edit_book(inventory, slot, pages: &[String], title: Option<&str>, author: &str) -> bool`, following `handleEditBook`.
   - It refuses, with no change, when:
     - the slot is not 0..=8 and not 40;
     - there are more than 100 pages, a page is over 1024 characters, or the title is over 32;
     - the stack is not `minecraft:writable_book`.
   - It strips `§` and control characters other than `'\n'` from the pages and the title. Not vanilla: vanilla's server trusts its client's filter. This is hardening only.
   - **Without a title:** write `minecraft:writable_book_content = {"pages":[{"raw":p},...]}`. Remove the key when there are no pages, and set `components` to None when it is then empty.
   - **With a title:**
     - the id becomes `minecraft:written_book`, and the other components and the count are kept;
     - `writable_book_content` is removed;
     - `minecraft:written_book_content` becomes `{"title":{"raw":t},"author":author,"pages":[{"raw":p}...],"resolved":true}`, with no generation key (as `book.rs:51-53` does);
     - `max` comes from `recipes.stack("minecraft:written_book", 1).max` (16).
   - **Frame:** add `McClick::EditBook { pages: Vec<String>, title: Option<String> }`.
   - **render_anim:** for a Held source, `apply_input` handles `EditBook` by calling `edit_book(&mut player.inventory, slot, ..., &player.name)` and closing.
3. **"Minecraft books and quills open vanilla's edit and sign screens"**
   - **Frame:**
     - `McScreen::BookEdit` and `McScreen::BookSign`;
     - `MinecraftUi.book_edit: Option<McBookEdit { pages, page, cursor, highlight, title, title_cursor, author, since }>`;
     - `McFont::split_ranges(text, width) -> Vec<Range<usize>>`, with the same rules as `split`. Make `split` map over it, so `pages_wrap_at_114_pixels_and_14_lines` stays green.
   - **hud, a new file `minecraft_book_edit.rs`:** layout, input and drawing, with the numbers above.
     - **Typing:** use `minecraft_chat::fold_typed` and `minecraft_edit::Edit`. Add `Edit::multiline(self)`, which lets `'\n'` through insert, so Enter and keypad Enter insert a newline.
     - **The 14-line check:** run it on a copy, and reject the whole insert if the copy overflows, like vanilla.
     - **Keys:**
       - Left and Right (with Ctrl, by word);
       - Up and Down by display line, through `split_ranges`;
       - Home and End per line;
       - Ctrl+A, C, V and X through the chat clipboard;
       - Backspace and Delete;
       - PageUp, PageDown and the pad triggers, like the lectern.
     - **Cursor:** a 1-pixel black bar inside the text and `_` at the end, blinking at 300 ms.
     - **Buttons:**
       - the forward arrow appends a page;
       - Done trims trailing empty pages, pushes `EditBook { title: None }` and closes;
       - Sign switches to BookSign;
       - Esc pushes only Close, which discards the edits.
     - **BookSign:**
       - a centred title box of at most 15 characters;
       - "Sign and Close" drawn with `widget/button_disabled` while the title is blank;
       - Cancel goes back to BookEdit and keeps the title;
       - Enter with a non-empty title pushes `EditBook { pages with trailing empties kept, title: Some(trimmed) }`.
   - **Other hud files:**
     - `hud/src/minecraft_inventory.rs:242`: the `typing` gate also covers BookEdit and BookSign, so E, the digit keys and Q type text;
     - draw branches go beside `:367`;
     - `minecraft_screens.rs`: size [192,192] and sheet "book" for both new screens.
   - **render_anim:**
     - `minecraft_gui.rs`: add `gui.cancel` to `TEXT_PREFIXES`.
     - On kind Write, `inventory_ui.show_book_editor(ui, slot, pages_text(stack), player.name)`. This sets `BookSource::Held(slot)` and `ui.book_edit`. A book and quill with no pages starts with one empty page.
4. **"A friend reads, writes and signs books through the host"**
   - **`minecraft_net`:**
     - `ToServer::EditBook { slot: u8, pages: Vec<String>, title: Option<String> }` and `ToClient::OpenBook { slot: u8 }`, as the next free indices;
     - extend the round-trip lists (`codec.rs:294-302`, `:554-560`);
     - `PROTOCOL` is 4 once in this track (B9's world commit bumps it from 3; do not bump it again).
   - **Host (`minecraft_host.rs`):**
     - queue `edit_books` per session like `lecterns` (`:161-165`), and drop the queue on leave (`:315`);
     - in the session loop (`:419-438`), push `ToClient::OpenBook { slot }` from `stepped.open_book`;
     - apply each queued edit with `books::edit_book(&mut player.inventory, ..., &player.name)` before `step_friend`, so the next `PlayerView` carries it.
   - **Friend (`minecraft_remote.rs`):**
     - `remote.rs` gains `open_books: Vec<u8>` (`:173`, `:493`);
     - for each slot, read `view.slots[slot]`: a written book opens `show_held_book`, and a book and quill opens `show_book_editor` with the friend's own name;
     - `InventoryUi.take_book_edits()` becomes `ToServer::EditBook`, like `take_lectern_actions` at `:1018-1025`.
5. **"Written books show their title, author and generation like vanilla"**
   - `display_name` (`render_anim/src/minecraft_inventory.rs:716`): with no `custom_name` and a non-blank title, return the title. `named` stays false, so the name is not italic.
   - Add gray `book.byAuthor` (only for a non-blank author) and `book.generation.N` (0 to 3) through a new `McStack.lore: Vec<String>`, drawn gray after `contents` in hud `tooltip_lines`.

**Not in scope** (log them as open limits):
- book cloning, because there are no special crafting recipes;
- styled and clickable page text, because pages are flattened to plain text;
- `resolveForItem` (selectors, scores and nbt in pages);
- the `minecraft:used` statistic;
- sending only the inventory slots that changed. `PlayerView` resends all 41 slot NBTs whenever anything changes (`views.rs:78`, `session.rs:301-309`), so a full 100-page book costs 100-400 KB per resend. Measure the size of a `PlayerView` with a 100-page book in a test, record it in the track log, and leave slot diffs to track A or a later round.

### Tests
**`minecraft_survival/src/books.rs`:**
- `a_written_book_reads_and_a_quill_writes`
- `editing_a_quill_stores_its_pages_and_an_empty_one_has_no_component`: also checks that a `custom_name` is kept.
- `signing_turns_a_quill_into_an_original_written_book`:
  - the id becomes written_book with max 16;
  - the exact component, with no generation key;
  - custom_name and count are kept;
  - trailing empty pages are kept.
- `book_edits_follow_vanilla_slot_and_size_rules`:
  - slots 9 and 39 are refused and 40 is accepted;
  - a written book or stone in the slot is refused;
  - 101 pages, a 1025-character page and a 33-character title are each refused with no change;
  - `§` and control characters are dropped and `'\n'` is kept.
- `a_signed_book_survives_the_item_nbt_round_trip`: `item_nbt::stack_tag` then `stack_of`, and `resolved` becomes Byte 1.

**`actions.rs`**, with the harness of the spyglass test at `:2218-2275`:
- `right_clicking_with_a_written_book_opens_it_for_that_hand`
- `a_quill_in_the_offhand_opens_the_editor_for_slot_40`
- `a_lectern_still_takes_a_held_book_before_it_opens`
- `a_spectator_does_not_open_a_book`
- `holding_use_opens_a_book_only_once`

**Frame:** `split_ranges_match_split`.

**hud `minecraft_book_edit.rs`**, with the font stub from `minecraft_book.rs:268`:
- `edit_layout_matches_vanilla` at width 427:
  - the text origin is (152, 32);
  - back is (160,159,23,13) and forward is (233,159,23,13);
  - Sign is [113,196,98,20] and Done is [215,196,98,20];
  - the sign screen's title box is at x 153, y 50.
- `typing_stops_at_14_lines_and_1024_chars`
- `enter_inserts_a_newline_and_up_down_move_by_display_line`
- `forward_on_the_last_page_adds_one_up_to_100`
- `done_trims_trailing_empty_pages_and_esc_discards`
- `sign_and_close_waits_for_a_title_and_cancel_keeps_it`
- `typing_in_a_book_does_not_toggle_the_inventory`

**render_anim:**
- `a_held_book_turns_pages_locally_and_closes_without_a_lectern_action`
- `a_held_book_edit_reaches_the_inventory_slot`
- `a_written_book_is_named_by_its_title_with_author_and_generation_lines`
- The existing lectern tests stay green, including `a_friends_lectern_screen_sends_its_buttons_and_closes_once` after the rename.

**`minecraft_net`:** EditBook (100 pages and a title) and OpenBook round-trip, with their index numbers asserted.

**Host level:** `a_friends_book_edit_on_the_host_changes_their_slot_and_goes_back_in_the_player_view`.

### Owner's steps (idea)
1. **Reading from the hand.** Hold "Test" and right-click at the sky.
   - "Page 1 of 3" opens, with only the forward arrow and one wide Done button.
   - The arrows and PageUp/PageDown turn pages with the page sound.
   - Done and Esc close it.
   - Press F to move it to the offhand, select an empty slot and right-click: it opens again.
2. **Tooltip.** Hover over the book in the inventory: "Test", then gray "by Owner" and "Original".
3. **Writing.** Right-click a book and quill.
   - There is a blinking cursor, "Page 1 of 1", and the Sign and Done buttons.
   - Type two sentences and press Enter for a new line. Text wraps at the edge.
   - Hold a key: typing stops after 14 lines.
   - Click forward: "Page 2 of 2". Type on it, then click Done and reopen: both pages are kept.
   - Type more and press Esc: the new text is gone, as in vanilla.
4. **Signing.** Click Sign.
   - "Enter Book Title:", "by <you>" and the warning are shown.
   - "Sign and Close" is grey until there is a title, and the title stops at 15 characters.
   - Cancel keeps the title. Sign as "Diary".
   - The quill becomes the written book "Diary", by you, Original, and right-clicking reads it.
5. **Crafting.** Craft a book and quill from a book, an ink sac and a feather, then write in it and sign it.
6. **Lecterns.**
   - A lectern with "Diary" on it, with a comparator, redstone and a lamp next to it. Page turns change the signal, and Take Book works.
   - A book and quill with pages reads on the second lectern.
7. **Saving.** Save, quit and reload: the signed book, the quill's pages and the lectern book are kept.
8. **Game modes.**
   - In adventure mode, reading, writing and signing work.
   - In spectator mode, right-clicking with a book does nothing.
9. **A friend** (`docs/round8/multiplayer-quickstart.md`, same kit).
   - The friend repeats steps 1, 3, 4 and 6.
   - The host sees the friend's quill turn into a written book.
   - After the friend rejoins, their books are kept.
   - Both players read the same lectern.

```
IW4L_MINECRAFT_KIT=minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}],minecraft:writable_book,minecraft:writable_book,minecraft:lectern*2,minecraft:comparator,minecraft:redstone*16,minecraft:redstone_lamp,minecraft:book*2,minecraft:ink_sac*2,minecraft:feather*2,minecraft:crafting_table,minecraft:stone*32
```

---

## B9. Potion colours everywhere, and effect particles

### The owner's finding
Screenshot 10 (`10_trankfarben.png`) shows a "Potion of Healing" and the potions next to it, all with the same blue liquid. The owner wants vanilla potion colours on every potion, splash potion, lingering potion and tipped arrow, wherever one is drawn:
- inventory and creative icons;
- the hotbar;
- the first- and third-person hand;
- dropped items and item frames;
- other players' hands.

The owner also wants the missing effect particles around entities that have effects.

### Cause
The blue is 0x385DC6, the `default` of the `minecraft:potion` tint (-13083194).
- **The tint ignores the stack.** `crates/minecraft_terrain/src/interface.rs:66-69` (`item_model_tints`) handles `minecraft:potion` in one arm with dye, firework, map_color, team and custom_model_data, and returns only the tint's `default`.
- **Every renderer caches item visuals by the bare id**, so `minecraft:potion_contents` never reaches the tint:
  - **GUI icons.** `item_icons.rs:15-32` (`item_icon_at`) gets only an id key. `render_anim/src/minecraft_inventory.rs:338` collects `s.id`, and `:494-524` keeps one atlas cell per id. `hud/src/minecraft_gui.rs:729-733` looks the icon up by `stack.id`. This also covers creative, the hotbar and the cursor.
  - **World meshes.** `mesh.rs:898-930` (`ItemVisuals::model(id)`) caches tints by id.
  - **Dropped items** (`mesh.rs:974`) ignore `entity.stack.components`, which are present on the host and the friend (`minecraft_survival/src/entity_view.rs:233-237`).
  - **Item frames.** `FrameView.item` is only an id (`item_frame.rs:153`, `:360`).
  - **First person.** `render_anim/src/minecraft_world.rs:1423` passes `dials.key(&stack.id)`.
  - **Third person and other players.**
    - `body_gear.rs:8-23` (`GearItem`) carries the id, glint and dye only, and `body_render.rs:28-34` sets `first_tint: None`.
    - The net `GearItem` (`minecraft_net/src/messages.rs:494-498`) has no colour, so neither side can tint the other's held potion.
- **Effect swirls.** Nothing spawns the `entity_effect` swirl around a living entity that has effects. The only `SpellParticles` callers are the splash burst and the area cloud.

**Already correct (do not touch):**
- `PotionContents::color` (`potions.rs:330-350`) and the effect colour table (`potions.rs:70-111`);
- thrown bottles (`thrown.rs:399-401`);
- splash burst particles (`server.rs:1459`, `entities.rs:1310`, `spell_particles.rs:102`);
- area cloud particles (`effect_cloud.rs:81`, `entity_view.rs:153-160`);
- the witch's held potion (`witch_render.rs:140-142`).

### Vanilla 26.3
**Item tint.**
- `client.color.item.Potion.calculate` returns `ARGB.opaque(contents.getColorOr(default))` when the stack has `POTION_CONTENTS`, and the default (0xFF385DC6) otherwise.
- `potion`, `splash_potion`, `lingering_potion` and `tipped_arrow` each have this tint on tint index 0:
  - for the bottles, layer0 is `item/potion_overlay` (the liquid);
  - for the tipped arrow, layer0 is `item/tipped_arrow_head`.
- Every item render uses the per-stack tint: GUI, hand, ground, frame and thrown.

**The colour itself.**
- `getColorOr` returns `customColor` if present.
- Otherwise it averages the visible effects' colours, weighted by amplifier+1, and falls back to the default when there are none.
- The fork already does this.

**Effect particles.**
- `LivingEntity` keeps one particle option per visible effect, with alpha 38 for ambient effects and 255 otherwise, plus "all ambient".
- Client `tickEffects`: when the list is not empty and `random.nextInt((invisible ? 15 : 4) * (ambient ? 5 : 1)) == 0`, it adds an `entity_effect` particle in a random colour from the list at (`getRandomX(0.5)`, `getRandomY()`, `getRandomZ(0.5)`).
- The particle is translucent and uses the `effect_7..effect_0` sprites.

### Plan
1. **"Minecraft potion and tipped arrow icons show their vanilla potion colour"**
   - **`potions.rs`:** `pub fn stack_color(components: Option<&Value>) -> Option<u32>`. It returns `Some(color)` when `minecraft:potion_contents` is present (string or object form) and `None` otherwise.
   - **A new file `minecraft_terrain/src/item_tint.rs`:**
     - `key(id, potion) -> String` gives `id` or `id~rrggbb`;
     - `split(key) -> (&str, Option<u32>)` returns None on a bad suffix;
     - `of_stack(stack)`.
     - `~` must not collide with `dials::KEY_MARK` (`#`). Split the tint first, then the dial.
   - **`interface.rs:51`:** `item_model_tints` takes `potion: Option<u32>`. Split `minecraft:potion` out of the shared arm and use `potion.or(default)` for it. Dye, firework and the rest stay on their default. Update the callers: `item_icons.rs:30`, `mesh.rs:415` (pass None), `mesh.rs:925` and the interface tests.
   - **`item_icons.rs:21`:** split the tint, then the dial, and pass the colour on.
   - **Frame:** `McStack.icon: String` (the atlas key). Update the hud test literals:
     - `minecraft_clicks.rs:309`, `minecraft_creative.rs:692`, `minecraft_inventory.rs:951` and `minecraft_screens.rs:1197`;
     - `minecraft_gui.rs:1071`, `:1130` and `:1197`.

     `hud/src/minecraft_gui.rs:729-733` looks icons up by `stack.icon`. Names still use `stack.id`.
   - **`render_anim/src/minecraft_inventory.rs`:**
     - In `convert` (`:336-362`), set `icon = item_tint::of_stack(s)` and push it to the icon list.
     - In the atlas loop (`:487-536`), names are keyed by the base id, and the icon is rendered from `item_tint::key(dials.key(base), potion)`.
     - In the animated refresh (`:469-485`), use `of_stack`.
     - `minecraft_creative.rs:558` (`icon_ids`) returns `stack.icon`.
   - About 110 more atlas cells fit easily within `ICON_ATLAS_LIMIT`.
2. **"Held, dropped and framed potions show their colour, also on other players"**
   - **`mesh.rs:898`:** `ItemVisuals::model` splits the tint first and passes it to `item_model_tints`. The cache stays keyed by the full key.
   - **Dropped items** (`mesh.rs:974`): `self.model(&item_tint::of_stack(&entity.stack), packs)`.
   - **`body_gear.rs`:**
     - `GearItem.potion: Option<u32>`, set in `GearItem::of`;
     - fix the literals in `armor_render.rs:347` and `:406`, `body_render.rs:81` and the tests;
     - `body_render.rs:31` uses `id: item_tint::key(&item.id, item.potion)`, and `first_tint` stays for the witch only.
   - **`item_frame.rs`:**
     - `FrameView.potion: Option<u32>`, set in `view()` (around `:360`), with the other literals fixed;
     - `mesh.rs:1232-1239` (`append_frames`) uses the tinted key.
   - **First person** (`render_anim/src/minecraft_world.rs:1423`): `item_tint::key(dials.key(&stack.id), potions::stack_color(...))`.
   - **Multiplayer:**
     - net `GearItem.potion: Option<u32>`;
     - `minecraft_survival/src/gear.rs:17-31` maps it both ways;
     - fix the literals at `remote.rs:1190`, `session.rs:1811` and `codec.rs:367`, `:478`;
     - `PROTOCOL` goes from 3 to 4 (once for the whole track).
3. **"Minecraft entities with effects give off coloured effect particles"**. Optional: it may be deferred if the track runs long, but the owner did ask for it.
   - **`minecraft_terrain`:**
     - `SwirlView { feet, width, height, colors, ambient, invisible }` (serde, rustfmt-clean);
     - `EntitySnapshot.swirls` (`server.rs:224`), filled at `server.rs:3533` for each mob whose `mob_effects` is not empty, using the visible effects' `effect_color`.
   - **Players:** append a swirl entry per player that has active `SurvivalStatus` effects, where minecraft_survival takes each level's snapshot. Conduit power counts as ambient.
   - **`mob_sender.rs:159-198` (`near()`):** filter swirls by the mob tracking range. Do not clone them unfiltered.
   - **`SpellParticles::swirl(view)`:** the vanilla chance and position. Add a per-particle alpha, 38/255 when ambient, that `append_mesh` writes into the vertex colour (`spell_particles.rs:245` writes 1.0 today).
     - First check that the translucent pass blends by vertex alpha.
     - If it does not, keep full alpha and log it as Not vanilla.
   - **`entity_view.rs`:** keep `swirls` from the snapshot like `clouds` (around `:196`), and call `swirl` in `tick` next to the cloud loop (`:152-160`). The host and the friend both see every body's swirls, including their own in F5.
   - **Accepted limits:**
     - `World::mob_effects` misses ghasts, blazes, magma cubes and endermites. Do not change mob AI files for them (track D owns them).
     - Players only have the 15 simulated effect kinds, so swiftness, strength and the like show no swirl on players.

**Not in scope:**
- dyed leather icons, which have the same default-tint bug for `minecraft:dye` (a later follow-up with the same key);
- the tipped arrow's flight trail (no arrow entity is drawn).

### Tests
**`potions.rs` (`stack_color`):**
- None without the component;
- F82423 for healing, in both forms;
- a custom colour;
- the default for water;
- strong_turtle_master gives 0x8D85E6.

**`item_tint.rs`:**
- the key/split round trip;
- a dial key is left alone;
- a bad suffix gives None;
- a poison splash potion gives `minecraft:splash_potion~87a363`.

**`interface.rs`:**
- healing gives [[F8,24,23]] and None gives [[38,5D,C6]];
- a dye tint ignores the colour (extend `dyed_and_firework_tints_use_their_default_colour_through_selects`).

**`item_icons.rs`**, gated on `MINECRAFTOSS_ROOT`:
- a tinted potion icon differs from the plain one;
- an overlay pixel equals the overlay times F82423/255;
- the same for the tipped arrow's head.

**`mesh.rs`**, with a synthetic pack: a dropped healing potion and a framed one give tinted vertex colours, and a plain potion gives the default.

**`body_gear.rs`, `body_render.rs` and `item_frame.rs`:** the colour is carried, and `HeldItem.id` is `minecraft:potion~87a363`.

**`codec.rs` and `gear.rs`:** GearItem with a colour round-trips, and `PROTOCOL == 4`.

**render_anim:** two potion stacks get different `McStack.icon` and the same `id`, and names are keyed by the base id. Factor out a small `icon_key` helper if `publish` is too heavy to test.

**Swirls:**
- with a fixed seed over 4000 calls:
  - a normal view spawns about 1 in 4;
  - an ambient one about 1 in 20;
  - an invisible one about 1 in 15;
- colours come only from the list;
- an ambient particle has alpha 38/255 (if the alpha is built);
- a cow with poison has a swirl entry [0x87A363];
- `near()` drops swirls that are out of range.

### Owner's steps (idea)
**Host alone:**
1. **Inventory (E).** Each bottle has its own liquid colour:
   - Healing is red, Poison olive green and Swiftness cyan;
   - Fire Resistance is orange, Night Vision lime and Strong Turtle Master lavender blue;
   - Invisibility is almost white, Leaping pale yellow and Weakness dark grey;
   - Water and Awkward keep the old blue.

   Also check the splash and lingering bottles and the tipped arrows' heads. The names still show on hover.
2. **Hotbar and cursor.** Potions in the hotbar and one carried on the cursor keep their colours.
3. **Creative.** In `/gamemode creative`, the potions in Food & Drinks and the tipped arrows in Combat each show their own colour.
4. **Held.** A held Healing potion is red in first person, in the offhand (F) and in third person (F5).
5. **Dropped and framed.**
   - Press Q on Poison and Fire Resistance: they lie green and orange on the ground.
   - In an item frame, a Poison tipped arrow has a green head, and a Night Vision bottle is lime.
6. **Thrown.** A thrown Splash Harming bottle and its burst are dusky red. Lingering Poison leaves a green cloud.
7. **Swirls (commit 3).**
   - Drink Poison and press F5: green swirls rise around you.
   - A cow hit with Splash Regeneration gets pink swirls.

**Multiplayer:**

8. The friend's potions are coloured in their inventory.
9. Each player sees the other's held potion in its colour.
10. The friend sees the host's dropped and framed potions in their colours.
11. The host sees green swirls around a poisoned friend.

```
IW4L_MINECRAFT_KIT=minecraft:potion@healing,minecraft:potion@poison,minecraft:potion@swiftness,minecraft:potion@fire_resistance,minecraft:potion@night_vision,minecraft:potion@strong_turtle_master,minecraft:potion@invisibility,minecraft:potion@water,minecraft:potion@awkward,minecraft:potion@leaping,minecraft:potion@weakness,minecraft:splash_potion@harming*2,minecraft:splash_potion@regeneration*2,minecraft:splash_potion@poison,minecraft:lingering_potion@poison*2,minecraft:lingering_potion@slow_falling,minecraft:tipped_arrow@poison*16,minecraft:tipped_arrow@strength*16,minecraft:tipped_arrow@leaping*16,minecraft:item_frame*4,minecraft:cow_spawn_egg*2
```

---

## B10. A much weaker and shorter damage overlay

### The owner's finding
Screenshot 11 (`11_rotes_overlay.png`) shows a strong red screen with blurry red blobs while the hearts are low. The owner wants a much weaker and shorter damage overlay, which can still be switched off.

### Cause
The red is MW2's low-health blood overlay, not a hit flash. It depends only on how low the health is.
- **The health mapping.** `minecraft_survival/src/survival.rs:199-201` and `:886-887` set MW2's `ps.health` to Minecraft health × max/20. One heart left gives a health fraction of 0.10.
- **The overlay.** `crates/hud/src/blood.rs:87` reads that fraction, and `:123-129` calls `hud_iw4::blood_overlay_lerp` (`crates/hud_iw4/src/blood.rs:17-35`).
  - Its target is `1 - health`. It rises instantly and falls at only 0.3 per second, and only once health rises.
  - `:184-208` draws a full-window `splatter_alt` quad (`blood_defocus_color`, `blood_defocus_mask`) with that strength as its alpha.
  - At one heart the alpha is about 0.9, and Minecraft heals slowly, so it stays.
- **Pain vision on top.** `crates/render_frontend/src/assemble/drawsurf/film_vision_view.rs:350-358` blends in MW2's pain vision at strength `1 - health` whenever health is at most 0.5.
- **The switch.** Show Blood (`GameSettings::show_blood`, Options > Advanced Video) gates only the blood overlay (`hud/src/blood.rs:41-43`, `:106-121`), not the pain vision.
- **What a hit already gives.** A Minecraft hit already bumps the replicated `ps.damage_event` (`sim/src/damage.rs:180-227`, `script_player.rs:283-285`, `net/src/transport/netfields.rs:284`), which drives MW2's view kick.

### Vanilla 26.3
- There is no full-screen red on damage or at low health. `Hud.extractVignette` reads only the world border.
- On a hit in first person:
  - the hurt camera tilt (`GameRenderer.bobHurt`, 10 ticks = 500 ms, curve sin(t^4·π) with t from 1 down to 0, peaking about 80 ms after the hit);
  - the hearts blink white (the fork does this already);
  - the red tint shows on entity models only.
- At 4 HP or less the hearts only jitter (the fork does this already, `hud/src/minecraft_vitals.rs:109`).

### Plan
Change only the Minecraft case. MW2 maps stay exactly as they are.

1. **"A Minecraft hit shows a faint half-second red flash instead of MW2's lasting low-health blood"** (`crates/hud/src/blood.rs`)
   - **State:** a `HurtFlash { last_event: Option<u32>, hit_at: Option<i32> }` in `BloodOverlayLatch`, with a pure `observe(damage_event, now_ms) -> f32`:
     - the first event seen only sets the baseline, so there is no flash on joining or respawning;
     - a changed event (compared with `!=`, so a wrapped counter still counts) sets `hit_at`.
   - **The curve:** `mc_hurt_flash(elapsed_ms)` returns `MC_HURT_FLASH_PEAK * sin(t^4·π)` with t = 1 - elapsed/500, and 0 outside 0..500.
     - `MC_HURT_FLASH_PEAK = 0.3`, one constant for tuning.
     - Not vanilla: vanilla has no screen flash. This keeps MW2-style feedback at the owner's request.
   - **Wiring in `update_blood_overlay`:**
     - add `minecraft: Option<Res<frame::MinecraftUi>>`;
     - on a Minecraft map (`ui.active || ui.credits.is_some()`), the intensity is `flash.observe(ps.damage_event, cg_clock.time())`;
     - otherwise the existing `blood_overlay_lerp` runs unchanged;
     - reset the flash on `LifeStarted` (`:75-79`) and in the branches that zero the intensity (`:81-96`, `:114-117`).
   - The `should_draw_blood_overlay` gate stays, so Show Blood off hides the flash.
2. **"No MW2 pain vision on Minecraft maps"** (`film_vision_view.rs`)
   - Add `minecraft: Option<Res<frame::MinecraftUi>>` to `update_film_vision_view` (`:228-238`).
   - Add a pure `fn pain_wanted(health: Option<f32>, scripted: bool, minecraft: bool) -> bool`, so `pain_strength` stays 0 on Minecraft maps.

This is client-side presentation only, with nothing on the wire. Host and friend each read their own replicated `ps.damage_event` and `MinecraftUi.active` (`minecraft_remote.rs:998` sets it for the friend), so each flashes only for their own hits.

Do not edit `docs/MINECRAFT-SURVIVAL.md:431` in this track. Put the new wording in the track log's "State doc changes" section instead: a hit gives a faint red flash of about half a second, Show Blood turns it off, and MW2's low-health blood and pain vision no longer show on Minecraft maps.

### Tests
**hud `blood.rs`:**
- `the_first_damage_count_seen_does_not_flash`
- `a_hit_flashes_faintly_and_fades_within_half_a_second`:
  - 0 at the hit;
  - above 0.25 and at most the peak at +80 ms;
  - below 0.05 at +499 ms;
  - 0 at +500 ms and at +2000 ms.
- `the_flash_peak_stays_far_below_the_mw2_low_health_overlay`: a peak of at most 0.35, against `blood_overlay_lerp(0.0, 0.1, 16, 0.3)` ≈ 0.9.
- `a_wrapped_damage_counter_still_flashes`
- `low_health_alone_shows_no_flash_on_minecraft`
- `a_new_life_forgets_the_last_hit`
- The existing Show Blood tests stay as they are.

**`render_frontend`:**
- `pain_wanted(Some(0.1), true, true)` is false;
- `(Some(0.1), true, false)` is true;
- `(Some(0.9), true, false)` is false.

### Owner's steps (idea)
1. Check that Show Blood is on.
2. **One hit.** A zombie hits you once: a faint red flash, gone in about half a second, with the view kick and blinking hearts.
3. **Low hearts.** Go down to 1-2 hearts with the zombie and a cactus. Between hits the screen is completely clear (the screenshot 11 case).
4. **Standing at 1 heart.** Stand still at 1 heart for 10 seconds: the screen stays clear and only the hearts jitter.
5. **Healing.** Eat golden apples: nothing changes on screen.
6. **Show Blood off.** Step onto a magma block: no flash.
7. **The totem.** Survive a lethal fall with a totem: at most one short flash.
8. **Respawning.** Die and respawn: no flash is replayed.
9. **A friend.**
   - When the friend is hit, only the friend's window flashes.
   - The friend's screen is clear at 1 heart.
   - The friend's own Show Blood switch works only on their PC.
10. **MW2.** A plain MW2 map still has MW2's blood and pain vision.

```
IW4L_MINECRAFT_KIT=minecraft:zombie_spawn_egg*8,minecraft:cactus*4,minecraft:magma_block*4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:totem_of_undying,minecraft:cobblestone*64,minecraft:ladder*32,minecraft:diamond_sword,minecraft:shield
```

---

## Shared points for the track
- **The protocol.** `minecraft_net::PROTOCOL` goes from 3 to 4 once in this track, in B9's world commit. B8's friend commit adds messages without a second bump.
  - If track A also bumps it, the second merge takes the next number and renumbers the new message indices.
  - Record the final number in the log, for the `docs/MULTIPLAYER.md` update at the end of the round.
- **`McStack` changes twice.** B9 adds `icon` and B8 adds `lore`. Track C also touches `McStack` and `convert`, so whoever merges second keeps both fields.
- **State docs.** Do not edit `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md`, `CLAUDE.md` or `docs/round9/decisions.md`. Write the wording for them in the track log under "State doc changes".

## Rules
One commit per fix or feature, with tests. Run clippy, and rustfmt on new files. No code comments. Use gentle cargo mode. Owner steps and the kit line for each commit go in `docs/round9/track-b-log.md`.
