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

## Minecraft /clear removes items like vanilla

Commit: the commit that adds this entry (branch `track-c`, group "commands").

### What changed

- New `crates/minecraft_commands/src/clear.rs`, checked against 26.3 `ClearInventoryCommands`, `Inventory.clearOrCountMatchingItems` and `ContainerHelper` with javap:
  - The tree is `/clear [<targets>] [<item>] [<maxCount>]` at level 2. `maxCount` is an integer from 0. Without it everything matching is removed (vanilla passes -1). With 0 the items are only counted.
  - `ItemFilter` (`Any` or a list of ids) is what the world matches against.
  - `clear_stack`, `clear_stacks` and `clear_inventory` are the vanilla loop, so the glue only calls one function. `clear_inventory` walks the 36 storage slots, the 4 armour slots and the offhand (the first 41 slots, the vanilla `Inventory` container), then the 2x2 crafting grid, then the carried stack, each with the remaining maximum.
  - Messages follow 26.3's `CommandResponseTracker`: nothing found gives `clear.failed.single` (one target) or `clear.failed.multiple`. Otherwise the single or multiple `commands.clear.test.*` or `commands.clear.success.*` line goes to the source and is broadcast to ops. It is single when exactly one player had matching items.
- New `crates/minecraft_commands/src/tracker.rs`: `Tracker`, the vanilla `CommandResponseTracker` (total, the only element, the only non-zero element). The commands that come next use it too.
- `ArgKind::ItemPredicate` parses `*`, `#tag` and an item id into `Arg::ItemPredicate`. An unknown item gives `argument.item.id.invalid` and an unknown tag `arguments.item.tag.unknown`, both pointing at the argument as vanilla's context does.
- `CommandData` is new server data next to `CommandRegistries`, reached through `CommandServer::data` and `data_mut`. It starts with `item_tags` (tag id to member ids). `CommandRegistries` is unchanged, so `render_anim` builds it as before.
- `CommandWorld::clear(player, &ItemFilter, max) -> Option<i32>` defaults to `None`, which counts as nothing found, so `/clear` answers "No items were found on player Steve" until the glue lands.
- Suggestions: an item predicate suggests the item ids, and `#` plus the item tags when the argument is empty or starts with `#`. `CommandHints` gains `item_tags`, filled by `from_server`.

### Tests added

- `clear::tests` (5):
  - Stacks clear in order up to the maximum; a dry run counts without removing.
  - The inventory clears storage, armour, offhand, the crafting grid and the cursor, and leaves slots 41 and 42 alone.
  - `/clear @s minecraft:stone 0` counts, then `100`, then the rest, then the failure line, then `/clear` with no arguments.
  - Two players: single and multiple lines, a tag, an unknown tag with its context line, an unknown player, and level 0.
  - Unknown item ids against the registry, trailing component syntax, and a world without inventories.
- `tracker::tests` (1): single and counts follow the vanilla tracker.
- `suggest::tests::item_predicates_suggest_items_and_hash_tags`, plus `item_tags` in the hints test.

### In-game test steps

`/clear` reaches the inventory only once the glue is in (see "After the split"). Until then every `/clear` answers "No items were found on player Steve", which is the first check.

```
IW4L_MINECRAFT_KIT=stone*64,stone*30,oak_log*10,iron_boots
```

1. `/clear @s minecraft:stone 0`: "Found 94 matching item(s) on player Steve". Nothing disappears. Screenshot.
2. `/clear @s stone 70`: "Removed 70 item(s) from player Steve", and 24 stone are left.
3. `/clear @s #minecraft:logs`: "Removed 10 item(s) from player Steve".
4. `/clear`: "Removed 25 item(s) from player Steve" (the 24 stone and the boots). A second `/clear` answers in red "No items were found on player Steve". Screenshot.
5. `/clear @s #minecraft:nope`: red "Unknown item tag 'minecraft:nope'" and the context line.

Kit: `minecraft:stone` 64 and 30, `minecraft:oak_log` 10, `minecraft:iron_boots` 1.

### Decisions

- **Component predicates are not parsed.** Vanilla accepts `stone[count=1]` and `*[damage~...]`; here the `[` is trailing data and gives "Expected whitespace to end one argument". Nothing in the fork's items needs component tests yet.
- **No `*` suggestion.** Vanilla's packrat grammar suggests ids and tags; `*` is typed by hand.
- **The error cursor of an unknown item or tag is the start of the argument** (the `#` for a tag), where vanilla's `ResourceLookupRule` stores the error.
- **A target the world cannot clear counts as 0**, so the vanilla failure line shows instead of an invented message.

### After the split

1. **`CommandWorld::clear`** in the glue: find the player's `Inventory` (the host's is `entities.inventory`) and return `Some(minecraft_commands::clear_inventory(inventory, filter, max))`. When anything was removed, mark the inventory changed so the client and the save see it. In multiplayer each player's inventory is keyed by its `PlayerId` and UUID.
2. **Item tags**: fill `server.data_mut().item_tags` once at load from the 26.3 jar, `data/minecraft/tags/item/**.json`, resolving nested `#` references (the same walk as `minecraftoss_core::tags::Tags::load`, but keeping ids). Keys are `minecraft:<path>` without the `#`.

## Minecraft commands can redirect like vanilla

Commit: the commit that adds this entry (branch `track-c`, group "commands"). `/xp` and `/execute` need it, so it comes before them.

### What changed

- `crates/minecraft_commands/src/tree.rs` follows brigadier's redirects:
  - `Node` gains `redirect: Option<Redirect>` and `modifier: Option<Modifier>`, built with `.redirect(target)` or `.redirect_with(target, modifier)`. `Redirect::Root` is the dispatcher root, and `Redirect::Command(name)` a root command, as `xp` and `tp` point at `experience` and `teleport`.
  - `Parsed` gains `stages`, one per brigadier `CommandContextBuilder`: the redirect target, its start and where its nodes, arguments and literals begin. `stage_nodes(i)` gives a stage's node range.
  - `parse_nodes` follows a redirect when a separator follows the node (`canRead(1)`) and returns the redirected parse at once, as `CommandDispatcher.parseNodes` does.
  - `smart_usage` prints `-> name` for a command redirect and `...` for the root, so `/help` shows `/tp -> teleport` like vanilla.
- `CommandServer::perform` runs each stage's modifier with that stage's arguments and literals, then the command with the last stage's. A modifier error stops the command and is reported like any command error.
- `/tp` is now a redirect to `/teleport` (vanilla `TeleportCommand`), instead of a copy of its tree.
- `/help <command>` uses the nodes of the first stage, as vanilla's `HelpCommand` does.
- Suggestions follow brigadier's `findSuggestionContext` through the stages, and the input colours use the last stage's arguments only (`CommandSuggestions.formatText` takes `getLastChild()`).

### Tests added

- `server::tests::redirects_run_each_modifier_with_its_own_arguments`: a test tree with a root redirect that has a modifier and a command redirect. It checks the modifier order and arguments, a failing modifier, an unfollowed redirect and the stage start.
- `suggest::tests::redirects_suggest_and_colour_the_target_command`: `/tp ` suggests what `/teleport ` does, with the same usage and colours.
- `commands::tests::help_lists_smart_usages_of_visible_commands` now expects `/tp -> teleport`.

### In-game test steps

```
IW4L_MINECRAFT_KIT=stone*64
```

1. `/help`: the list shows `/tp -> teleport`. Screenshot.
2. `/tp ~ ~5 ~`: you are teleported 5 blocks up with "Teleported Steve to ...", as before.
3. `/tp`: red "Unknown or incomplete command. See below for error" and `tp<--[HERE]`.

Kit: none needed (the line above is any valid kit).

### Decisions

- **Stages instead of nested contexts.** The parse stays one flat list of nodes, arguments and literals, with the stage boundaries recorded. It keeps the existing lookups (`ctx.arg`) working and is enough for redirects whose modifier changes one source. Forking modifiers that run the rest once per target (`execute as @a`) are not supported.

## Minecraft /xp and /experience add, set and query experience like vanilla

Commit: the commit that adds this entry (branch `track-c`, group "commands").

### What changed

- New `crates/minecraft_commands/src/experience.rs`, checked against 26.3 `ExperienceCommand`, `ExperienceCommand$Type`, `Player.giveExperiencePoints`, `giveExperienceLevels`, `getXpNeededForNextLevel` and `ServerPlayer.setExperiencePoints` with javap:
  - The tree is `/experience add <target> <amount> [points|levels]`, `/experience set <target> <amount> [points|levels]` and `/experience query <target> (points|levels)`, at level 2. `/xp` redirects to `/experience`. The argument is called `target` in 26.3, even where it takes several players. `set` takes amounts from 0, `query` takes one player.
  - `Experience { level, progress, total }` holds vanilla's three fields. Its methods are the vanilla float math: `needed`, `points` (`Mth.floor(progress * needed)`), `give_points` (with negative points walking levels down), `give_levels` (saturating, and a reset below 0), `set_points` (refused at or above the points for the next level, then clamped to `(needed - 1) / needed`) and `set_levels`.
  - Messages follow the response tracker: `add` counts every target, `set` counts the targets that changed, and nothing changed gives `commands.experience.set.points.invalid`. The query goes to the source only.
- `CommandWorld::experience(player) -> Option<Experience>` and `set_experience(player, Experience) -> bool`. They default to `None` and `false`, so until the glue lands every target counts as not found and the answer is "No player was found".

### Tests added

- `experience::tests` (5):
  - Points cross level thresholds, including vanilla's float rounding (giving back 6 of 11 points leaves 4 showing, not 5) and 1395 points landing exactly on level 30.
  - Levels saturate at `i32::MAX` and reset below zero.
  - Setting points stays below the next level.
  - `/xp add`, `/xp query`, `/experience add`, a refused `set`, `set` points and levels, negative levels and the integer bound.
  - Two players with single and multiple lines, a `set` that only one player passes, the single-player query, a world without experience, and level 0.
- `suggest::tests::redirects_suggest_and_colour_the_target_command` covers `/xp ` and its units and usage.

### In-game test steps

The experience bar only follows once the glue is in. Until then `/xp ...` answers "No player was found".

```
IW4L_MINECRAFT_KIT=stone*64
```

1. `/xp add @s 5 levels`: "Gave 5 experience levels to Steve", and the bar shows 5. Screenshot.
2. `/xp query @s levels`: "Steve has 5 experience levels".
3. `/xp add @s 10`: "Gave 10 experience points to Steve", and the bar fills to about half (17 points are needed at level 5).
4. `/xp set @s 30 points`: red "Cannot set experience points above the maximum points for the player's current level".
5. `/experience set @s 30 levels`, then `/xp add @s -40 levels`: the bar is back at 0. Screenshot.

Kit: none needed.

### Decisions

- **A target the world has no experience for is skipped.** When none is left the answer is "No player was found"; vanilla never gets there because every player has experience.
- **The level-up sound is the glue's job** (see below), since it depends on the player's tick count.

### After the split

1. **`CommandWorld::experience`**: `Some(Experience { level, progress, total })` from the player's `survival::Status` (`experience_level`, `experience_progress`, `total_experience`).
2. **`CommandWorld::set_experience`**: write the three fields back (levels and totals are `u32` there, and the command never gives negative values) and return `true`.
3. **Sound**: vanilla plays `minecraft:entity.player.levelup` when a positive level change lands on a multiple of 5 and the last one was more than 100 ticks ago, at volume `0.75 * min(level, 30) / 30`. The glue can compare the old and new level in `set_experience`.

## Minecraft /enchant enchants the held item like vanilla

Commit: the commit that adds this entry (branch `track-c`, group "commands").

### What changed

- New `crates/minecraft_commands/src/enchant.rs`, checked against 26.3 `EnchantCommand` and `Enchantment.getFullname` with javap:
  - The tree is `/enchant <targets> <enchantment> [<level>]` at level 2, with the level an integer from 0 and 1 by default.
  - The order is vanilla's. An unknown enchantment is an argument error with its context. Then the targets are resolved, then the level is checked against the maximum (`commands.enchant.failed.level`). Then each target: no item gives `failed.itemless`, an item that cannot take it or already has a clashing one gives `failed.incompatible` (with the item's plain name), and a target the world cannot enchant gives `failed.entity`. These three only stop the command when there is one target; with several, that target is skipped. Nothing applied gives `commands.enchant.failed`.
  - `full_name` is `Enchantment.getFullname`: the description in grey (red for a curse), plus a space and `enchantment.level.N` unless both the level and the maximum are 1.
- `CommandWorld::enchantment(id) -> Option<EnchantmentInfo { max_level, curse }>` and `enchant(player, id, level) -> Option<Enchanted>`, where `Enchanted` is `Applied`, `Itemless` or `Incompatible(Text)`. Both default to `None`, so `/enchant` answers "Can't find element ... of type 'minecraft:enchantment'" until the glue lands.
- `Text::hover_name(id)` (new `Content::HoverName`) resolves to the item's name without brackets, for `getHoverName().getString()`.

### Tests added

- `enchant::tests` (4):
  - Sharpness V with its grey span, Mending with no numeral, Curse of Binding in red, Unbreaking I.
  - Failures in order: the level, an unknown player before the level, the same enchantment twice, a stick, an empty hand, an unknown id with context, and the integer bound.
  - Several players: one skipped without an item, a clashing enchantment (Smite after Sharpness) skipped, "to 2 entities", "Nothing changed", a world that cannot enchant, and a player below level 2.
  - `full_name` with and without the numeral, and a missing numeral key.
- `text::tests::item_names_try_item_then_block_keys_in_brackets` covers `hover_name`.

### In-game test steps

`/enchant` needs the glue; until then it answers "Can't find element 'minecraft:sharpness' of type 'minecraft:enchantment'".

```
IW4L_MINECRAFT_KIT=diamond_sword,stick
```

1. Hold the sword. `/enchant @s sharpness 5`: "Applied enchantment Sharpness V to Steve's item" with the name in grey, and the sword glints. Screenshot.
2. `/enchant @s sharpness 6`: red "6 is higher than the maximum level of 5 supported by that enchantment".
3. `/enchant @s sharpness`: red "Diamond Sword cannot support that enchantment" (vanilla refuses an enchantment the item already has).
4. `/enchant @s smite`: the same red line, since Smite and Sharpness exclude each other.
5. Hold the stick, `/enchant @s unbreaking`: red "Stick cannot support that enchantment". Empty hand: red "Steve is not holding any item". Screenshot.

Kit: `minecraft:diamond_sword` 1, `minecraft:stick` 1.

### Decisions

- **A target the world cannot enchant takes vanilla's non-living path** (`commands.enchant.failed.entity`), since that is vanilla's answer for a target without a hand.
- **Item names in the incompatible line come from the world as `Text`**, so a renamed item can show its own name later.

### After the split

1. **`CommandWorld::enchantment`**: from `minecraftoss_entities::enchant::Enchantments` (the list the enchanting table already loads): `max_level` of the entry, and `curse` from `tag_contains("minecraft:curse", id)`.
2. **`CommandWorld::enchant`**: the player's main-hand stack (`inventory.slots[selected]` for the host). Empty: `Itemless`. Otherwise read its enchantments (`stored_enchantments` for an enchanted book, as `getEnchantmentsForCrafting` does). It is incompatible unless `can_enchant(item)` holds and every existing enchantment passes `compatible(existing, new)`, which is false for the same enchantment. Then `add_enchantment` with `max(existing, level)` capped at 255, skipping level 0 (vanilla's `ItemEnchantments.Mutable.upgrade`), and answer `Applied`. The incompatible name is `Text::hover_name(item_id)`.
3. **Registry ids**: fill `registries().enchantments` from the same list so suggestions and `registered()` know them.

## Minecraft /setblock places blocks like vanilla

Commit: the commit that adds this entry (branch `track-c`, group "commands").

### What changed

- New `crates/minecraft_commands/src/blocks.rs`, checked against 26.3 `SetBlockCommand`, `BlockPosArgument`, `WorldCoordinate.parseInt` and `BlockStateParser` with javap:
  - `/setblock <pos> <block> [destroy|keep|replace|strict]` at level 2.
  - **Block states**: `parse_block` reads `id[name=value,...]{nbt}` with vanilla's whitespace rules and errors (`property.novalue`, `property.unclosed`, and `tag.disallowed` for `#` where a tag is not allowed). `resolve_block` then checks the id, each property name, duplicates and each value against `CommandData::blocks`, in vanilla's order and at vanilla's cursor positions (`id.invalid`, `property.unknown`, `property.duplicate`, `property.invalid` with the arguments in vanilla's order). Empty block data accepts anything, like `registered()`.
  - **Order**: the block is checked first (vanilla parses it), then the position. `getLoadedBlockPos` gives `argument.pos.unloaded`, then `argument.pos.outofworld`.
  - **The edit**: `BlockEdit { kind, dimension, from, to, block, mode, filter, strict }` goes to the world. `apply_edit` is vanilla's `setBlock` over a `BlockAccess` (read a state, destroy with drops, place, update the neighbours), so the glue only implements those four:
    - `keep` filters on air (`isEmptyBlock`: air, cave air, void air) and fails on anything else.
    - `destroy` breaks the block with drops first, then places unless both the new block and the spot are air.
    - Placing the same state fails ("Could not set the block"), as `BlockInput.place` does.
    - `strict` skips the neighbour update (vanilla flags 816 instead of 256).
  - `edit_answer` turns the count into "Changed the block at x, y, z" (broadcast to ops) or the failure. With `Answer::Later` the world may answer after the server thread ran the edit.
- `ArgKind::BlockPos` reads three integers or `~` offsets (which may be fractional), or `^` local coordinates, with vanilla's `argument.pos.missing.int`, `argument.pos.mixed` and `argument.pos3d.incomplete`. The block is the floor of the position, as `BlockPos.containing`. `ArgKind::BlockState` reads a block state.
- `CommandData` gains `blocks` (id to its properties and their values) and `block_tags`. `Answer<T>` (`Now` or `Later`) is new in `server.rs`, with `OVERWORLD`.
- `CommandWorld` gains `dimension(player)` (default the overworld), `loaded(dimension, pos) -> Loaded` (default `Unloaded`) and `edit_blocks(source, &BlockEdit) -> Answer<i32>` (default 0). `Ctx::dimension()` is the source's dimension.
- Suggestions: a block position suggests `~`, `~ ~`, `~ ~ ~`, or the targeted block's whole coordinates from the new `CommandHints::hit_block`. A block state suggests block ids from the new `CommandHints::blocks`, then `[` once the id is complete.

### Tests added

- `blocks::tests` (5): block states parse properties and NBT with cursor positions; they resolve against the data with every error and position; `apply_edit` places, refuses the same state, keeps, destroys, clears with air and skips updates when strict; and `/setblock` end to end with `~` floors, the failure lines, a bad value with its context, a fractional integer, too few coordinates, a tag, and unloaded and out-of-world positions.
- `args::tests::block_positions_read_integers_or_relative_offsets`.
- `suggest::tests::block_arguments_suggest_whole_coordinates_and_block_ids`.

### In-game test steps

`/setblock` needs the glue. Until then it answers "That position is not loaded" for every position.

```
IW4L_MINECRAFT_KIT=stone*64
```

1. `/setblock ~ ~-1 ~ minecraft:gold_block`: "Changed the block at x, y, z" and gold appears under your feet. Screenshot.
2. The same command again: red "Could not set the block".
3. `/setblock ~ ~-1 ~ stone keep`: red "Could not set the block" (the spot is not air).
4. `/setblock ~ ~-1 ~ stone destroy`: the gold breaks with particles and drops a gold block, and stone takes its place. Screenshot.
5. `/setblock ~2 ~ ~ oak_stairs[facing=east,half=top]`: an upside-down stair facing east.
6. `/setblock ~ ~ ~ oak_stairs[half=up]`: red "Block minecraft:oak_stairs does not accept 'up' for half property" with the context line.

Kit: none needed.

### Decisions

- **Block data is checked when the command runs, with vanilla's cursor**, not while parsing, because the parser has no registry access (as `/give` already does for items). The red line and its context are the same; only the live red colouring of a bad property in the chat box is missing.
- **No property suggestions.** Ids and `[` are suggested; property names and values would need the full state list in `CommandHints`.

### After the split

1. **Block data**: fill `server.data_mut().blocks` once from `Registries.blocks` (each block id with its properties and their values, in the catalog's order), and `block_tags` from `data/minecraft/tags/block/**.json` like the item tags.
2. **`CommandWorld::loaded`**: `OutOfWorld` when y is outside the dimension's build height or x or z is outside ±30,000,000 (`isInWorldBounds`); otherwise `Yes` when the chunk at (x >> 4, z >> 4) is loaded in that dimension (`hasChunkAt`), else `Unloaded`. Vanilla checks the chunk first.
3. **`CommandWorld::edit_blocks`**: implement `BlockAccess` over the level's chunks and call `minecraft_commands::apply_edit`, either on the main thread against the host's copy (then send the changed blocks to the server thread like player edits), or on the server thread through a new `Command::EditBlocks { source, edit }` with the count coming back in `Output` and `Answer::Later` returned meanwhile. When it comes back, `minecraft_commands::blocks::edit_answer(&edit, count)` gives the line: send `Ok` through `server.feedback(source, text, true, rules)` and `Err` through `error_line`.
   - `state`: the block id and all its property values.
   - `destroy`: vanilla `destroyBlock(pos, true)`: false for air, otherwise break with the break particles and sound, drop the loot like a player break without a tool, and set air.
   - `place`: start from the block's default state with the given properties, update the shape from the neighbours unless strict (stairs and fences connect), keep the given properties, write it with the NBT for a block entity, and return false when nothing changed.
   - `update_neighbours`: the block updates that a player placement sends.

## Minecraft /fill fills areas like vanilla

Commit: the commit that adds this entry (branch `track-c`, group "commands").

### What changed

- `crates/minecraft_commands/src/blocks.rs` gains `/fill`, checked against 26.3 `FillCommand`, `FillCommand$Mode`, `BlockPredicateArgument`, `BlockStateParser.readVagueProperties` and `BlockPos.betweenClosed` with javap:
  - The tree is 26.3's: `/fill <from> <to> <block> [outline|hollow|destroy|strict|keep|replace [<filter> [outline|hollow|destroy|strict]]]` at level 2. `strict` is replace without block updates.
  - **Order**: the block, then the filter (both parse-time in vanilla), then the two corners (`getLoadedBlockPos` each), then the size. The size is the box's volume as a 64-bit number, and anything above the `max_block_modifications` game rule (32768 by default) gives `commands.fill.toobig` with the limit and the size.
  - **The loop** (`apply_edit` for `EditKind::Fill`): x fastest, then y, then z, as `betweenClosed`. Each position: the filter, the old state, `destroy` breaks with drops first, then the mode picks the block (`outline` skips the inside, `hollow` puts air inside), and a placement that changes nothing does not count unless the block was destroyed. The neighbour updates run after the whole loop, unless strict. 0 gives `commands.fill.failed`, otherwise `commands.fill.success` with the count.
  - **Filters**: `keep` is air only. `replace <filter>` takes a block predicate: a block with exact properties, or `#tag` with loose properties (a block without that property does not match) and an optional `{nbt}` that the world compares (`BlockAccess::block_entity_matches`, false by default). An unknown tag gives `arguments.block.tag.unknown` with its context.
- `ArgKind::BlockPredicate` parses like a block state but allows `#`.
- `CommandHints::block_tags`; a predicate suggests `#` plus the block tags when empty or after `#`, otherwise block ids.
- The glue for `/setblock` covers `/fill`: the same `edit_blocks` and `BlockAccess`.

### Tests added

- `blocks::tests` (3 more):
  - The loop order, counting, refills that change nothing, outline (26 of 27), hollow over a solid cube (26 glass and the core cleared, 27 changes), destroy and strict.
  - Filters: air, a tag with a loose property that does and does not match, an exact block, an NBT predicate, an unknown tag, and the box helpers.
  - `/fill` end to end: a 5x5x5 glass box (98), the same box again, `replace glass strict`, `keep`, too big at the default and a lowered limit, an unknown tag with its context, a tag as the block, and an unloaded corner.
- `suggest::tests::block_arguments_suggest_whole_coordinates_and_block_ids` covers the filter suggestions and the modes after it.

### In-game test steps

`/fill` needs the same glue as `/setblock`; until then it answers "That position is not loaded".

```
IW4L_MINECRAFT_KIT=stone*64
```

1. `/fill ~1 ~ ~1 ~5 ~4 ~5 glass hollow`: "Successfully filled 98 block(s)" and a glass box with air inside appears next to you. Screenshot.
2. The same command again: red "No blocks were filled".
3. `/fill ~1 ~ ~1 ~5 ~4 ~5 stone replace glass`: the box turns to stone, "Successfully filled 98 block(s)".
4. `/fill ~1 ~ ~1 ~5 ~4 ~5 air destroy`: the stone breaks with particles and drops. Screenshot.
5. `/fill ~ ~ ~ ~100 ~100 ~100 stone`: red "Too many blocks in the specified area (maximum 32768, but specified 1030301)".
6. `/gamerule max_block_modifications 8`, then `/fill ~1 ~ ~1 ~2 ~1 ~3 stone`: red "... (maximum 8, but specified 12)". Set it back with `/gamerule max_block_modifications 32768`.

Kit: none needed.

### Decisions

- **Only the two corners must be loaded**, as in 26.3 (`getLoadedBlockPos` on `from` and `to`). Vanilla then loads whatever lies between; the glue decides how to handle unloaded chunks inside the box (see below).
- **The whole fill runs in one call.** The brief suggested spreading a large fill over ticks; vanilla does it in one tick too, and 32768 blocks is the cap.

### After the split

1. Same as `/setblock`. In addition, `BlockAccess::block_entity_matches(pos, nbt)` should parse the SNBT and compare it with the block entity's saved tag the way `NbtUtils.compareNbt(expected, actual, true)` does (every given key must match; lists match as subsets). Returning false keeps NBT predicates from matching.
2. **Chunks inside the box that are not loaded**: vanilla loads them synchronously. The glue can generate or load them on the server thread before running the loop, or skip them and count only what changed; skipping is the safer first step.
3. **Block tags**: as in the `/setblock` notes, `block_tags` from `data/minecraft/tags/block/**.json`.

## Minecraft /spawnpoint sets each player's respawn point like vanilla

Commit: the commit that adds this entry (branch `track-c`, group "commands").

### What changed

- New `crates/minecraft_commands/src/spawnpoint.rs`, checked against 26.3 `SetSpawnCommand`, `RotationArgument`, `WorldCoordinates.ZERO_ROTATION`, `Level.isInSpawnableBounds` and `Mth.wrapDegrees` with javap:
  - `/spawnpoint [<targets>] [<pos>] [<rotation>]` at level 2. Without a position it is the block at the source's feet; without a rotation it is yaw 0 and pitch 0 (`ZERO_ROTATION` is absolute).
  - A given position must be in spawnable bounds (x and z within ±30,000,000, y within ±20,000,000), otherwise `argument.pos.outofbounds`. Unlike `/setblock` it need not be loaded.
  - The rotation is `<yaw> <pitch>`, each a number or a `~` offset from the source's own angle. The yaw is wrapped to [-180, 180) and the pitch clamped to [-90, 90].
  - `Respawn { dimension, pos, yaw, pitch, forced: true }` goes to every target (vanilla `RespawnConfig(respawnData, true)`).
  - **The message keeps vanilla's quirk**: 26.3 passes seven arguments (x, y, z, yaw, pitch, dimension, player or count) to `commands.spawnpoint.success.single` and `.multiple`, whose en_us texts have six `%s`. So the line reads "Set spawn point to 10, 64, -4 [0.0] in 0.0 for minecraft:overworld", exactly as in vanilla. The newer `.new` keys with both angles are unused in 26.3.
  - `java_float` prints floats as Java's `Float.toString` does ("0.0", "-45.5", "1.0E-4").
- `ArgKind::Rotation` reads `<yaw> <pitch>` with `argument.rotation.incomplete`.
- `CommandWorld::set_respawn(player, &Respawn) -> bool`, default `false`, so `/spawnpoint` answers "No player was found" until the glue lands.

### Tests added

- `spawnpoint::tests` (3): Java float printing, `wrap_degrees` and the bounds; the seven-argument line with defaults, a position and rotation, relative angles that wrap and clamp, out of bounds, an incomplete rotation; two players, a world without respawn points, and level 0.
- `args::tests::rotations_read_yaw_then_pitch`.

### In-game test steps

`/spawnpoint` needs the glue and the per-player respawn point. Until then it answers "No player was found".

```
IW4L_MINECRAFT_KIT=stone*64
```

1. Stand somewhere recognisable and run `/spawnpoint`: "Set spawn point to x, y, z [0.0] in 0.0 for minecraft:overworld" (the missing player name is vanilla's). Screenshot.
2. Walk 50 blocks away and `/kill`: you respawn at the point from step 1, not at the world spawn. Screenshot.
3. `/spawnpoint @s ~ ~ ~ 90 0`, `/kill`: you respawn there looking west (yaw 90).
4. `/spawnpoint @s 0 30000000 0`: red "That position is outside the allowed boundaries".

Kit: none needed.

### Decisions

- **The vanilla seven-argument message is kept**, as the brief asked, including the multiple form ("for minecraft:overworld players").
- **A target the world cannot store a respawn point for is skipped**; with none left the answer is "No player was found".

### After the split

1. **`CommandWorld::set_respawn`**: store the `Respawn` per player (keyed by its UUID) and return `true`. Track D's player file saves it under the vanilla `respawn` key: `{dimension, pos: [x, y, z], yaw, pitch, forced: true}`.
2. **Respawning**: when the player respawns, use the stored point before the world spawn. Vanilla (`ServerPlayer.findRespawnPositionAndUseSpawnBlock`) checks a forced point only for room to stand (no bed or anchor needed); if the spot is blocked it falls back to the world spawn and says `block.minecraft.spawn.not_valid`. Face the stored yaw and pitch.
3. **Dimension**: the stored dimension decides the respawn level once there are several (phase 3).

## Minecraft /locate finds structures and biomes like vanilla

Commit: the commit that adds this entry (branch `track-c`, group "commands").

### What changed

- New `crates/minecraft_commands/src/locate.rs`, checked against 26.3 `LocateCommand`, `ResourceOrTagArgument`, `ResourceOrTagKeyArgument` and their string recipes with javap:
  - `/locate structure <id|#tag>`, `/locate biome <id|#tag>` and `/locate poi <id|#tag>` at level 2.
  - **Structures** are checked when the command runs: an unknown id or tag gives `commands.locate.structure.invalid` with `#` for a tag. **Biomes and POI** are checked like vanilla's parser: `argument.resource.not_found` or `argument.resource_tag.not_found`, with the registry name and the context after the id.
  - `LocateRequest { kind, printable, tag, targets, dimension, origin }` goes to `CommandWorld::locate`, with the tag already expanded to its members and the origin the block at the source's feet. The world answers `Answer::Now(Option<Located { pos, id }>)` or `Answer::Later`.
  - `locate_answer` builds vanilla's line: "The nearest %s is at %s (%s blocks away)". The name is the id, or for a tag `#tag (found id)`. The coordinates are green and bracketed (`chat.square_brackets` around `chat.coordinates`), with `~` as y for structures and POI and the real y for biomes. The distance is vanilla's float math: horizontal for structures and POI (`Mth.floor(Mth.sqrt((float) (dx*dx + dz*dz)))`), 3D for biomes. Nothing found gives `commands.locate.<kind>.not_found`. The line goes to the source only.
  - `CommandServer::reply(source, result, broadcast, rules)` turns a late answer (from `/locate`, `/setblock` or `/fill`) into the same lines a direct answer gives.
- `ArgKind::ResourceOrTag(Registry)`, with `Registry::Structure`, `Biome` and `PointOfInterest`. `CommandData` gains `structures`, `structure_tags`, `biomes`, `biome_tags`, `poi_types` and `poi_tags`; `CommandHints` gains the same as id lists and `tag_ids(registry)`. Suggestions give `#` plus the tags when the argument is empty or starts with `#`, otherwise the ids.
- New `crates/minecraft_terrain/src/locate.rs` (edition 2021): pure searches over `minecraftoss_world::chunk_map::WorldGen` with a time cap, checked against 26.3 `ChunkGenerator.findNearestMapStructure`, `getNearestGeneratedStructure`, `getStructureGeneratingAt`, `BiomeSource.findClosestBiome3d`, `Mth.outFromOrigin` and `BlockPos.spiralAround`:
  - `nearest_structure(gen, targets, origin, radius, deadline) -> Search`:
    - It groups the targets by the structure sets that place them, skipping sets that cannot appear in the dimension's biomes (vanilla's `possibleStructureSets`).
    - Concentric rings (strongholds) check every ring position, nearest centre first. Random spread placements search square rings of grid cells out to radius 100 and stop at the first ring with a find, keeping the first hit inside a ring as vanilla does.
    - A chunk counts when `Structures::starts_in` really makes a start of a wanted structure there, which covers frequency, exclusion zones, the biome check and the weighted choice inside a set.
    - The answer is the placement's locate position (the chunk corner plus `locate_offset`).
  - `nearest_biome(gen, targets, origin, radius, deadline) -> Search`: only biomes the dimension can make; the spiral of 32-block steps out to 6400 blocks; at each column the heights from `out_from_origin(y, minY + 1, maxY + 1, 64)`; the first sample in a wanted biome wins.
  - `Search` is `Found(pos, id)`, `NotFound` or `TimedOut`. `structure_names`, `biome_names` and `tags(gen, registry)` (tag ids to members, nested tags expanded) fill the command data.
- `CommandWorld::locate` defaults to `Answer::Now(None)`, so until the glue lands `/locate structure minecraft:village_plains` answers "Could not find a structure of type "minecraft:village_plains" nearby".

### Tests added

- `locate::tests` in `minecraft_commands` (4): structures with the tilde, the green span, the tag form and the request's origin and members, not found and invalid ids and tags; biomes with their height and 3D distance, tags, not found, unknown ids and tags with context; POI without a manager, a late answer and level 0; vanilla's float distances.
- `suggest::tests::locate_suggests_ids_and_tags_of_its_registry`.
- `locate::tests` in `minecraft_terrain` (3): the spiral order (the first nine steps, and 401x401 for radius 200), `out_from_origin` for three cases, and seed 0 with the 26.3 data: the nearest village from (0, 64, 0) is `village_plains` at (272, 0, 944) whether asked by tag or by id, the nearest desert sample is (608, 64, -1632), a Nether fortress is not found in the overworld, and a passed deadline times out. It is skipped without the data; run it with `MINECRAFTOSS_ROOT=C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3`.

Seed 0 from (0, 64, 0) also gives (not in the test, for comparing with vanilla): stronghold (-208, ~, -1696) in 2.3 s, mansion (-3536, ~, -832) in 0.5 s, cherry grove (-1024, 128, 1344), mushroom fields (-672, 64, -896).

### In-game test steps

`/locate` needs the glue; until then every search answers "Could not find ...".

```
IW4L_MINECRAFT_KIT=stone*64
```

1. `/locate structure minecraft:village_plains`: "The nearest minecraft:village_plains is at [x, ~, z] (N blocks away)" with the coordinates in green. Screenshot.
2. `/tp @s x 120 z` with those numbers: a plains village is below you. Screenshot.
3. `/locate structure #minecraft:village`: the line names the tag and the village type in brackets.
4. `/locate biome minecraft:cherry_grove`, then `/tp` there: pink cherry trees. Screenshot.
5. `/locate structure minecraft:fortress`: red "Could not find a structure of type "minecraft:fortress" nearby" (no fortresses in the overworld).
6. `/locate poi minecraft:home`: red "Could not find a point of interest of type "minecraft:home" within a reasonable distance".
7. Optional vanilla comparison: in vanilla 26.3, a new world with seed 0, `/execute positioned 0 64 0 run locate structure #minecraft:village` should give 272 ~ 944.

Kit: none needed.

### Decisions

- **Not checked against a running vanilla server.** This machine has no Java 25 runtime or the 26.3 libraries, so the seed 0 positions are what the ported search and MinecraftOSS's generator give; step 7 is the check.
- **A time cap.** Vanilla searches without one. Here a timed-out search answers like nothing was found; the glue should log `TimedOut`. Searches measured on seed 0 took 0.2 to 2.3 s (debug build of the tests), so 20 s is a generous cap.
- **No click or hover on the coordinates.** Vanilla's green coordinates suggest `/tp @s x y z` on click and show `chat.coordinates.tooltip` on hover; the fork's `Text` has neither yet.
- **POI answers "not found"** because there is no POI manager; ids and tags are not checked while `poi_types` is empty (vanilla's POI types are a code registry, so the glue would have to list them).

### After the split

1. **Command data** once at load, from the level's `WorldGen` (`TerrainStream::world_gen()`): `structures = minecraft_terrain::locate::structure_names(&gen)`, `structure_tags = tags(&gen, "worldgen/structure")`, `biomes = biome_names(&gen)`, `biome_tags = tags(&gen, "worldgen/biome")`. Tags read like this cover every dimension, since the registries are shared.
2. **`CommandWorld::locate`**: return `Answer::Later` and run the search off the main thread, for the request's dimension:
   - `LocateKind::Structure`: `nearest_structure(&gen, &request.targets, request.origin, STRUCTURE_RADIUS, now + 20 s)`.
   - `LocateKind::Biome`: `nearest_biome(&gen, &request.targets, request.origin, BIOME_RADIUS, now + 20 s)`.
   - `LocateKind::Poi`: answer `None` until there is a POI manager.
3. **The answer**, back on the main thread: `Found(pos, id)` becomes `Some(Located { pos, id })`, the other two `None`. Then `server.reply(source, minecraft_commands::locate_answer(&request, found.as_ref()).map(|(text, _)| text), false, rules)` gives the lines for that player.
4. **Threads**: `Structures::starts_in` caches the starts it makes in the shared `WorldGen`, which the chunk generator reuses; the search only adds starts the generator would make anyway.

## Minecraft /execute in parses dimensions like vanilla

Commit: the commit that adds this entry (branch `track-c`, group "commands").

### What changed

- New `crates/minecraft_commands/src/execute.rs`, checked against 26.3 `ExecuteCommand`, `DimensionArgument`, `CommandSourceStack.withLevel` and `DimensionType.getTeleportationScale` with javap:
  - `/execute run <command>` redirects to the root, so any command the source may use can follow (and `/execute ... run execute ...` nests).
  - `/execute in <dimension> ...` redirects back to `/execute` with a modifier. The dimension is read as an id. If the server has not loaded that level, the modifier fails with `argument.dimension.invalid` ("Unknown dimension 'minecraft:the_nether'"), which is vanilla's answer for a level that does not exist. Otherwise, when the level differs from the source's, the source moves there and its x and z are multiplied by the source level's coordinate scale divided by the target's (1/8 into the Nether, 8 out of it), as `withLevel` does.
  - `/execute positioned <pos> ...` (cheap, so included) sets the source position, keeping the rotation and level.
  - The other subcommands (`as`, `at`, `align`, `anchored`, `facing`, `rotated`, `if`, `unless`, `store`, `summon`, `on`) are not parsed; they need forking sources, conditions or entities.
- `Ctx` gains `at` (a position override) and `dimension` (a level override). `Ctx::pose()` and `Ctx::dimension()` use them, so `~` coordinates, `/setblock`, `/fill`, `/locate` and `/spawnpoint` after `execute in` or `positioned` work in that level and position.
- `/tp` teleports into the source's level (vanilla `teleportToPos(source.getLevel())`), and `/tp <player>` into that player's level. A target in another level goes through `CommandWorld::teleport_to_dimension`.
- `CommandWorld::dimension_scale(dimension) -> Option<f64>` is `Some(coordinate scale)` for a loaded level. The default loads only the overworld (scale 1). `teleport_to_dimension(player, dimension, position) -> bool` defaults to `false`.
- `Registry::Dimension` with `CommandHints::dimensions` (the overworld by default), suggested after `in` as vanilla suggests `source.levels()`.

### Tests added

- `execute::tests` (3):
  - The Nether and End are unknown until loaded; `in minecraft:overworld run tp ~ ~10 ~`, `run seed`, nested `execute`, an incomplete `execute in`, and level 0.
  - With a loaded Nether at scale 8: in from the overworld (80, 64, -16) gives (10, 64, -2) and a dimension teleport; `/locate` under `execute in` asks in the Nether from the scaled origin; back out multiplies by 8; the same level is a plain teleport; `/tp <player>` goes to that player's level.
  - `positioned` alone and before `in`.
- `suggest::tests::execute_suggests_its_subcommands_dimensions_and_any_command_after_run`: subcommands, dimensions, the root commands after `run` from the right position, a command's own suggestions after `run`, the colours of the last stage only, and `<pos> -> execute`.
- `commands::tests::help_lists_smart_usages_of_visible_commands` lists `/execute (in|positioned|run)`.

### In-game test steps

These work as soon as this commit is in; the overworld is the only loaded level until phase 3.

```
IW4L_MINECRAFT_KIT=stone*64
```

1. `/execute in minecraft:the_nether run tp ~ ~ ~`: red "Unknown dimension 'minecraft:the_nether'", and you do not move. Screenshot.
2. `/execute in minecraft:the_end run seed`: red "Unknown dimension 'minecraft:the_end'".
3. `/execute in minecraft:overworld run tp ~ ~10 ~`: you go up 10 blocks with "Teleported Steve to ...".
4. `/execute positioned 0 100 0 run tp ~ ~ ~`: you land at 0.5 100 0.5.
5. Type `/execute in ` and press Tab once the chat shows suggestions (track A): only `minecraft:overworld` is offered.

Kit: none needed.

### Decisions

- **`in` checks loaded levels, not the dimension registry**, exactly as `DimensionArgument.getDimension` (`server.getLevel(key)`). Until phase 3 only the overworld answers.
- **No forks.** `as` and `at` run the rest once per entity in vanilla; the flat stages only carry one source, so they stay out until there are entities to select.

### After the split

1. **Phase 3**: `CommandWorld::dimension_scale` returns `Some(scale)` for every loaded level (the dimension type's `coordinate_scale`: overworld 1, Nether 8, End 1), and `dimension(player)` returns the player's level.
2. **`teleport_to_dimension`**: move the player to that level at the position (vanilla `teleportTo(level, x, y, z, ...)`: load the target chunk, keep velocity at zero, send the respawn and position packets in multiplayer) and return `true`.
3. **Hints**: fill `CommandHints::dimensions` with the loaded levels.

## Minecraft /kit gives the .env kit again

Commit: the commit that adds this entry (branch `track-c`, group "commands").

### What changed

- New `crates/minecraft_commands/src/kit.rs`, a fork-only command:
  - `/kit [<targets>]` at level 4 (owners only, so the host but not an op at level 2 or 3).
  - `CommandWorld::kit_line() -> Option<String>` asks the world for the `IW4L_MINECRAFT_KIT` value, re-read from the `.env` next to the exe each time, so a changed `.env` counts without a restart. It defaults to `None`.
  - `kit_entries` reads the line with the same syntax as the load-time kit (`item[+enchantment[=level]...][*count]`, comma-separated) through the `/give` item parser. Entries that do not parse, or have a count of 0, are skipped as at load.
  - Each entry goes through `CommandWorld::give` like `/give`, so what does not fit drops at the player's feet. Each given entry prints `/give`'s own line ("Gave 64 [Stone] to Steve"); an id the registry does not know prints `/give`'s red "Unknown item" line and the rest continue.
  - No kit gives a red "No IW4L_MINECRAFT_KIT in the .env next to the game", and a kit without valid entries "IW4L_MINECRAFT_KIT has no items".
- `env_value(text, key)` and `read_env_file(path, key)` read one key from `.env` text the way `dotenvy` does: `export` prefixes, `#` comment lines, single or double quotes, an inline ` #` comment, and the first definition wins.

### Tests added

- `kit::tests` (3): `.env` parsing (CRLF, comments, `export`, quotes, inline comments, empty values, a missing key) and a re-read file that changed; kit entries with counts, enchantments, a namespace, and every kind of bad entry; `/kit` giving each entry with its line, an unknown item, `/kit @a`, a level 3 op refused, an empty kit and no kit.
- `commands::tests::help_lists_smart_usages_of_visible_commands` lists `/kit [<targets>]`.

### In-game test steps

`/kit` needs the glue (below); until then it answers "No IW4L_MINECRAFT_KIT in the .env next to the game".

```
IW4L_MINECRAFT_KIT=diamond_pickaxe+efficiency=5,diamond_sword,stone*64,oak_log*64,torch*64,bread*16
```

1. Load the world with the kit above, then `/clear` (or drop everything).
2. `/kit`: one "Gave ..." line per entry, and the items are back. Screenshot.
3. Fill the inventory (`/kit` three more times): the stacks that do not fit lie at your feet. Screenshot.
4. Change the `.env` line to `IW4L_MINECRAFT_KIT=torch*8` without restarting, `/kit`: "Gave 8 [Torch] to Steve".

Kit: `minecraft:diamond_pickaxe` 1 (Efficiency V), `minecraft:diamond_sword` 1, `minecraft:stone` 64, `minecraft:oak_log` 64, `minecraft:torch` 64, `minecraft:bread` 16.

### Decisions

- **Level 4.** The orchestrator asked for an op-only command; the brief's level 2 would let any game master fill their inventory.
- **One line per entry**, using `/give`'s messages, instead of an invented summary line. A 27-stack kit is long, but every line is a vanilla text.
- **The two failure lines are plain English**; there is no vanilla key for a fork-only command.
- **`.env` is read in `minecraft_commands`** (`read_env_file`) rather than through `dotenvy`, which only sets variables that are not set yet and would keep the old value.

### After the split

1. **`CommandWorld::kit_line`**: `std::env::current_exe()`'s directory joined with `.env`, then `minecraft_commands::kit::read_env_file(&path, KIT_KEY)`. When the exe has no `.env`, fall back to `std::env::var(KIT_KEY).ok()` so a kit given on the command line still works.
2. `/kit` gives through the existing `give`, so nothing else is needed. In multiplayer the host's `.env` is the only kit; friends get it through `/kit <name>` from the host.
