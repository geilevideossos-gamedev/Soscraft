# Track C log (round 7)

Commands, tab completion and the creative inventory. Design brief: [c-commands.md](c-commands.md).

## Minecraft commands suggest completions like vanilla

Commit: the commit that adds this entry (branch `track-c`, group "suggestions").

### What changed

- New `crates/minecraft_commands/src/suggest.rs`, the suggestion engine. It follows vanilla 26.3 `CommandSuggestions`, brigadier's `CommandDispatcher.getCompletionSuggestions` and `SharedSuggestionProvider`, all checked with javap.
  - `complete(input, cursor, level, &CommandHints) -> Completion` takes the whole chat line. With a leading `/` it parses the command from offset 1, so every position counts the slash, as vanilla does.
  - `Completion` holds:
    - `command`: true for a command line (the list shows by itself), false for a chat line (the list shows on Tab only).
    - `start`: the start of the range the list replaces.
    - `list`: the suggestions in display order.
    - `usage`: the usage lines, and `usage_start`, the position they are drawn at.
    - `spans`: the input colours.
  - `Suggestion { range, text }` has `apply(original)` (vanilla `Suggestion.apply`) and `cursor()` (the cursor after `useSuggestion`). `suggestion_suffix(current, applied)` is vanilla `calculateSuggestionSuffix`, the grey text shown inline.
  - `CommandHints` holds the online player names, the item, entity, effect and enchantment ids and the point in view. `CommandHints::from_server` fills it from `CommandServer`. It is serde, so the host can send it to friends later.
- How the engine follows vanilla:
  - **Where to suggest:** the suggestion context is brigadier's `findSuggestionContext`, using the node ranges from the stash.
  - **Literals:** case-insensitive prefix, filtered by permission level.
  - **Matching:** names, game modes, time units and coordinates use `matchesSubStr`, which also matches after `.`, `_` or `/`. Ids use `filterResources`: the namespace or the path, or the full id once a `:` is typed.
  - **Results:** a suggestion equal to the typed text is dropped. The suggestions are merged into one range, de-duplicated and sorted case-insensitively. Then the ones that start with the last word, or with `minecraft:` plus it, come first (`sortSuggestions`).
  - **Usage lines** (`updateUsageInfo`):
    - At the end of the input, when nothing matches and the parse failed, the parse errors show as `command.context.parse_error`, with brigadier's 10-character context and `<--[HERE]`.
    - Otherwise the grey smart usage of the non-literal children shows.
    - When there is none of that and text is left over, the parse exception shows.
  - **Input colours** (`formatText`): the arguments cycle aqua, yellow, green, light purple and gold. Literals and the slash are grey, and everything after the parse cursor is red.
- What each argument suggests:
  - **Players:** online names, plus `@p @a @r @s` when the input is empty or after `@` (level 2 and up).
  - **Game modes:** the four mode names.
  - **Time:** the units `d s t` after a number.
  - **`/time set`:** the four markers with `show_in_commands` (day, midnight, night, noon).
  - **Game rules:** both literal forms, `keep_inventory` and `minecraft:keep_inventory`, as vanilla registers two literals per rule. Boolean rules suggest `true` and `false`.
  - **Booleans:** `true` and `false`.
  - **Items:**
    - An unknown or partly typed id suggests item ids. A known id suggests `[`.
    - Inside `[`, it suggests `minecraft:enchantments=` and `minecraft:stored_enchantments=`, then `=`, then `,` and `]`, as vanilla's `ItemParser.State` does.
    - After the fork's `+` kit syntax, it suggests enchantment ids.
  - **Coordinates:** `~`, `~ ~` and `~ ~ ~`, or `^` forms after `^`. When a block is in view, they become the hit point with two decimals, rounded half up like Java's `%.2f`.
  - **Effects and `/summon`:** the registry ids.
- `ArgKind::Resource` is now `Resource(Registry)`, with `EntityType`, `MobEffect` and `Enchantment`. `tree::parse_from` parses from an offset, and `parse` calls it with 0.
- The stash `C23 partial` (`Parsed.ranges`, `CommandServer::player_names`, public `Node::is_literal` and `allowed_in_id`) is folded in. The stash itself is kept.

### Tests added (`suggest::tests`, 16)

- Root literals are filtered by level. `/ga` gives `gamemode` and `gamerule` over the range 1..3.
- `/gamemode c` completes to `creative` from position 10, the start of the argument. A fully typed value is not suggested again.
- Items:
  - `/give @s pick` gives the diamond and iron pickaxes.
  - `/give @s s` puts `minecraft:s…` first.
  - `/give @s stone` gives `[`.
  - Components go keys, then `=`, then `,` and `]`, then keys again.
- The `+` kit syntax suggests enchantments.
- Players and selectors: `@`, `@x`, `@s` and names typed in any case.
- Usage:
  - `/give ` shows `<targets> <item> [<count>]` in grey, and `/kill ` shows `[<targets>]`.
  - `/kill @e` and `/gamemode god` show the parse error with its position and context.
  - `/frob` shows the unknown-command error at position 1.
- Input colours for arguments and red unparsed text.
- Coordinates with tildes, `^`, and the hit point rounded half up (0.125 shows as 0.13).
- Time units after a number, and the command time markers.
- Game rule names in both forms, and the boolean values.
- Registry ids match on the namespace or a path word.
- A chat line completes player names at its last word.
- `apply`, `cursor` and `suggestion_suffix`, including vanilla's quirk when the cursor is in the middle of the line.
- Merging expands every suggestion to one range and drops duplicates.
- `matches_sub_str` and `last_word_index`.
- `CommandHints::from_server`, including the JSON round trip.

### In-game test steps

The chat does not use the engine yet. Track A owns the chat UI in `hud`, and the glue goes into the new crate after the split (see below), so this commit changes nothing visible. Once both are wired:

1. Type `/ga`. The list shows `gamemode` and `gamerule`, and grey text completes the selected one. Screenshot.
2. Type `/give @s pick`. The pickaxes show.
3. Type `/give @s diamond_sword[`. The two enchantment components show. Screenshot.
4. Type `/gamemode god`. The white line reads "Unknown game mode: god at position 13: ...memode god<--[HERE]", and `god` is red in the input. Screenshot.
5. Look at a block and type `/tp `. The block's coordinates show with two decimals.

Kit: none needed for this commit.

### Decisions

- **Error lines in the usage area are white, not red.** `CommandSuggestions.extractUsage` draws every usage line with colour -1, and only the node usage has the grey style. The brief said red; the 26.3 bytecode decides.
- **Selector suggestions are `@p @a @r @s` only.** There is no `[` after a selector either. Vanilla also suggests `@e` and `@n` and the options bracket, but our parser rejects both, so suggesting them would only lead to errors. Add them back when entity selectors and options exist.
- **No `!` suggestion inside item components.** Vanilla 26.3 suggests `!` for removing a component, but our parser has no removal syntax.
- **`/op` and `/deop` suggest every online player.** Vanilla asks the server for non-ops and ops respectively. This can follow once the hints carry op status.
- **A chat line without `/` completes online player names at its last word, on Tab only**, as `getCustomTabSuggestions` does.
- **Positions are byte offsets into the UTF-8 input, not UTF-16 indices.** The number in "at position N" counts characters, which matches vanilla for everything outside the astral planes.
- **Enchantment ids after `+` are a fork extension.** They follow the kit syntax that `/give` already accepts.

### After the split

The trait is unchanged, so `render_anim` compiles as it is. The glue is:

1. **Each frame in the new crate**, wherever `Host` and the `CommandServer` live:
   - Build `CommandHints { hit, ..CommandHints::from_server(&server) }`. `hit` is the exact point where the crosshair ray meets the block face (vanilla `hitResult.getLocation()`), not the block position, and `None` when no block is targeted.
   - Pass it with `server.level(HOST)` to the chat.
   - In multiplayer, the host sends each friend its hints and `Clientbound::Commands { level }`.
2. **Fill the enchantment ids in `registries()`.** It now sets `enchantments: Vec::new()`. Read the ids from the enchantment registry in the 26.3 jar (`data/minecraft/enchantment/*.json`, as `minecraft:<name>`) so the `+` and component suggestions work. `registered()` treats an empty list as "accept anything", so filling it also makes `/give` reject unknown enchantments, as vanilla does.
3. **Chat UI** (track A, `hud/src/minecraft_chat.rs`):
   - Call `minecraft_commands::complete(&input, cursor, level, &hints)` whenever the input or cursor changes.
   - When `command` is true, show the list at once. For a chat line, show it only on Tab.
   - Draw at most 10 lines, anchored at `input[..start]`.
   - Tab writes `list[i].apply(original)` into the input, with the cursor at `list[i].cursor()`. `original` is the input as it was when the list opened. The first Tab writes the selected suggestion, and each later Tab or Shift+Tab moves forward or back first (vanilla `tabCycles`). Up and Down move the selection without writing.
   - The grey inline text is `suggestion_suffix(current, &list[i].apply(original))`.
   - When the list is hidden or empty, show `usage` (resolved with the language), anchored at `usage_start`.
   - Colour the input with `spans`.
