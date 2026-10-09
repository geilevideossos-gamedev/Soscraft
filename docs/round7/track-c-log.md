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
3. Run `/kit` six more times. The pickaxe, sword, stone, logs and torches take five new slots per kit while the bread stacks up, so the seventh kit is the first that no longer fits in the 36 slots: its torches lie at your feet. Screenshot.
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

## Minecraft resource errors point after the id like vanilla

Commit: the commit that adds this entry (branch `track-c`, group "commands").

### What changed

- `ArgKind::Resource` now records the error position after the id instead of before it. 26.3 `ResourceArgument.parse` reads the id and then throws `argument.resource.not_found` with the reader's context, which is the end of the id (checked with javap; there is no `setCursor` before the throw, unlike `ItemParser`, which resets to the start). The chat context line of `/effect` and `/enchant` with an unknown id now matches vanilla: "...@s glowing<--[HERE]" instead of "...t give @s glowing" with the id underlined.
- `/locate biome` and `poi` already did this in their own argument.

### Tests added

- `commands::tests::effect_defaults_to_thirty_seconds_amplifier_zero` and `enchant::tests::enchant_fails_in_the_vanilla_order` expect the vanilla context.

### In-game test steps

```
IW4L_MINECRAFT_KIT=stone*64
```

1. `/effect give @s nope`: red "Can't find element 'minecraft:nope' of type 'minecraft:mob_effect'" and a grey "...ve @s nope<--[HERE]" with nothing underlined. Screenshot.

Kit: none needed.

## Minecraft creative tabs are read from the game jar

Commit: the commit that adds this entry (branch `track-c`, group "creative-tabs").

### What changed

- New `crates/minecraft_terrain/src/class_file.rs`, a small Java class reader:
  - `ClassFile::parse` reads the constant pool (every tag, with the extra slot after a long or double), fields, methods with their `Code` (`max_locals` and the bytecode) and the `BootstrapMethods` table.
  - Lookups: `utf8`, `class_name`, `string`, `integer`, `member` (field, method and interface method references), `handle` (method handles), `invoke_dynamic` (name, descriptor, bootstrap and its arguments), `method` and `method_with`.
  - `instructions(code)` decodes the bytecode with each operand: constant pool indexes, local slots (also for the short `aload_1` forms), immediates, absolute jump targets, `iinc`, `wide`, and the padded `tableswitch` and `lookupswitch`. Truncated code and jumps out of the method are errors.
  - `parameters(descriptor)` and `returns_value(descriptor)`.
- New `crates/minecraft_terrain/src/creative_tabs.rs`, which reads the vanilla creative tabs from the owner's 26.3 `client.jar` at runtime. No Mojang data is committed. It was checked with javap against 26.3 `CreativeModeTabs`, `CreativeModeTab$Builder`, `Items`, `Blocks`, `references/BlockItemIds`, `references/ItemIds`, `references/BlockItemId`, `ColorCollection`, `WeatheringCopperCollection` and its `$ByState`, `DyeColor`, `Raid.getOminousBannerTemplate` and `CreativeModeInventoryScreen`.
  - **API**
    - `creative_tabs(jar) -> CreativeTabs` never fails. If the reader fails it logs "creative tabs fall back to registry order: <why>" and returns `fallback_tabs`. Entries it had to skip are logged in one line.
    - `read_creative_tabs(jar)` is the strict form. `read_tabs(load, known)` takes a class loader and the known item ids, so the tests run without a jar.
    - `CreativeTabs { tabs, fallback, skipped }`.
    - Each `CreativeTab` has `id` (`minecraft:building_blocks`), `title` (`itemGroup.buildingBlocks`), `row` (`Top` or `Bottom`), `column`, `kind` (`Category`, `Hotbar`, `Search` or `Inventory`), `aligned_right`, `show_title`, `scroll_bar`, `background` (`items`, `item_search` or `inventory`), `icon` (an item id), `operator` and `entries` in vanilla order.
    - `TabEntry { item, variants, in_tab, in_search }` follows vanilla's `TabVisibility`.
    - `CreativeTab::displayed()` and `item_ids()` give what the tab itself shows. `CreativeTabs::tab(id)`, `shown(operator)` and `search(operator)`.
    - `registry_order(load, known)` and `fallback_tabs(items, reason)`.
  - **Item ids**
    - The known ids are the jar's `assets/minecraft/items/*.json`, the same list the commands use. An entry or icon outside it is skipped and logged.
    - An `Items` or `Blocks` field becomes an id from the first try that is a known item:
      1. the field name in lower case;
      2. the strings of the matching `net/minecraft/references` statement, last first (`BlockItemId.create(block, item)` puts the item last);
      3. the reference field name in lower case.
    - This covers the three odd field names (`CUT_STANDSTONE_SLAB` is `cut_sandstone_slab`, `DRY_SHORT_GRASS` is `short_dry_grass`, `DRY_TALL_GRASS` is `tall_dry_grass`).
    - In 26.3 the id strings sit in `BlockItemIds` and `ItemIds`, not in `Items.<clinit>` as the brief expected.
    - A `ColorCollection` field is `<color>_<base>` for the 16 dye colours in record order, where `base` is the reference string (`DYED_TERRACOTTA` is `terracotta`).
    - A `WeatheringCopperCollection` field is the four weathering states, then the four waxed ones, with the prefixes of `WeatheringCopperCollection.PREFIXES`. `COPPER_BLOCK` takes its special names from `COPPER_BLOCK_SPECIAL_NAMES` (`copper_block`, `exposed_copper` and so on).
  - **Tabs**
    - A small symbolic interpreter runs `CreativeModeTabs.<clinit>` (the tab keys and textures) and `bootstrap`. That gives the colour order list, each builder call (row, column, title, icon, display lambda, aligned right, hidden title, no scroll bar, type, background) and each `Registry.register`.
    - Then it runs each tab's icon lambda and display lambda.
    - It follows lambdas and method references (`output::accept`), `registerColoredItems` with the bootstrap's gameplay colour order, `copperBlockFamilies`, `ColorCollection` and `WeatheringCopperCollection` `forEach`, `pick` and the accessors, `Optional.ifPresent` on registry lookups, and `new ItemStack(item)`.
  - **Generators** become one entry each with `Variants`:
    - `Potions`
    - `EnchantedBooks { all_levels }`: the maximum level is in the tab only, all levels in search only.
    - `Instruments { tag }`
    - `SuspiciousStews`, `OminousBottles` and `Fireworks`
    - `Paintings { tag, in_tag }`: the predicate lambda is run for a painting inside and outside the tag.
    - `OminousBanner`: the white banner.
    - `TestBlockModes` and `LightLevels`: the op tab's loops.
  - **Operator tab:** a display lambda that asks `hasPermissions()` marks its tab `operator`. Its content is read as if allowed.
  - **Search** is computed, not interpreted, as vanilla's search lambda does at runtime: the search-visible entries of every category tab in registry order, then the Search tab's own entries. Duplicates of item and variants are dropped, and operator tabs count only when `operator` is true.
  - **Shown tabs** follow vanilla's `shouldDisplay`: a category tab without displayed items is hidden. Operator tabs are hidden unless allowed.
- **Result on the owner's 26.3 jar**
  - 14 tabs at vanilla's rows and columns.
  - Icons: bricks, cyan wool, grass block, oak sign, redstone, bookshelf, compass, diamond pickaxe, netherite sword, golden apple, iron ingot, creeper spawn egg, command block and chest.
  - Entries per tab: Building 475, Colored 278, Natural 254, Functional 238, Redstone 70, Tools 149, Combat 79, Food 47, Ingredients 153 (152 shown), Spawn Eggs 89, Operator 13.
  - Search has 1638 entries, or 1625 without the operator tab.
  - Nothing is skipped.
  - The 23 items in no tab are vanilla's too: air, 16 explorer maps, `filled_map`, `knowledge_book`, `petrified_oak_slab`, `written_book`, and the wither and ender dragon spawn eggs.
  - Reading takes well under a second.
- `lib.rs` gains `pub mod class_file;` and `pub mod creative_tabs;`.

### Tests added

- `class_file::tests` (5):
  - A synthetic class with a long constant, a field, a method and an invokedynamic bootstrap.
  - Operand decoding with negative immediates, short load forms, `wide` loads and `iinc`, and backward and forward jumps.
  - Switch padding.
  - Broken code and classes.
  - Descriptor parameters.
- `creative_tabs::tests` (6), on synthetic bytecode built by a small test assembler (classes `ItemIds`, `Items`, `Blocks` and `CreativeModeTabs`):
  - **Copper and colour id rules.**
  - **A full read:**
    - three tabs with keys from `createKey`, titles, rows, columns, aligned right, a search background and type;
    - an icon from `new ItemStack(Blocks.BRICKS)`;
    - stone, two wool colours in the bootstrap's colour order, waxed copper through `waxed().forEach(output::accept)`;
    - the `CUT_STANDSTONE_SLAB` typo resolved through the reference, and melon seeds from `create(stem, seeds)`;
    - an unknown field logged, a potion generator, and enchanted books through `Optional.ifPresent` with their visibilities;
    - cut copper's eight states;
    - an operator tab with a `hasPermissions` branch, a loop run once, `setModeOnStack`, and a negated painting predicate.
  - **Search:** union, de-duplication and the operator filter; shown tabs.
  - **Unknown ids:** an unknown entry or icon is skipped and logged.
  - **Fallback:** a missing tabs class; registry order from `Items` and without it; the fallback tabs; an unreadable jar.
  - **`the_real_tabs_match_vanilla`**, against the 26.3 jar and skipped without it:
    - the layout equals the fallback table, and the icons and per-tab counts match;
    - the first Building and Colored entries are right, and the copper order is right;
    - every generator entry with its visibility;
    - the search sizes, and the exact list of items in no tab;
    - the registry order.

Run it with `MINECRAFTOSS_ROOT=C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3`.

### In-game test steps

Nothing changes in game yet. The reader has no caller until the creative screen lands after the split (see below), so there is nothing to screenshot.

Kit: none needed.

### Decisions

- **Interpretation, not a hard-coded list.** The tab contents are code in 26.3, so the reader interprets just enough bytecode, and every result is checked against the jar's item list. Only naming rules that are stable code conventions are written in Rust: the 16 dye colours in `ColorCollection` record order, and the copper prefixes. The real-jar test checks both.
- **Forward-only interpretation.**
  - A loop body runs once, because backward jumps are never taken.
  - A condition on a known number is evaluated, and one on an unknown value falls through.
  - So `hasPermissions()` reads as allowed and marks the tab, and the op tab's test block and light loops each give one entry with their `Variants`.
- **Generators are not expanded here.** Each becomes one entry that names its item and the kind of variants. Expanding them needs registry data (potions, enchantments, painting variants, instruments, stew effects), and the screen reads that after the split.
- **The ominous banner is `minecraft:white_banner` with `OminousBanner`.** 26.3 `Raid.getOminousBannerTemplate` uses `Items.BANNER.white()`.
- **Fallback.**
  - Any failure gives the 14-tab layout with empty category tabs. Vanilla hides those, so only Saved Hotbars, Search and Inventory show.
  - The Search tab then holds every item in registry order: the `Items.<clinit>` order when that class is readable, otherwise the sorted ids.
  - The layout table is the one place the tab layout is written down, and the real-jar test checks it against the jar.
- **One broken tab does not sink the others.** A display lambda that fails leaves its tab empty and is logged.

### After the split

The creative screen goes in after the split. That covers the UI, the search box, the tabs and saving hotbars.

1. **Data**
   - Each client reads `minecraft_terrain::creative_tabs::creative_tabs(&jar)` once at world load, off the main thread, and keeps it in the client state.
   - The tabs are client-side in vanilla too: the server never sends them.
2. **Opening**
   - In creative (`hasInfiniteMaterials`), E opens `McScreen::Creative { tab, scroll }` instead of the survival inventory.
   - The selected tab is kept between openings, as vanilla's static `selectedTab`. It starts at Building Blocks and falls back to it when the kept tab is not shown.
3. **Layout** (26.3 `CreativeModeInventoryScreen`, javap)
   - The image is 195 x 136.
   - Tabs are 26 x 32.
     - x = `column * 27`, or `195 - 27 * (7 - column) + 1` when aligned right.
     - y = -32 for the top row and 136 for the bottom.
     - The sprites are `container/creative_inventory/tab_top_selected_<n>`, `tab_top_unselected_<n>` and the same for `tab_bottom_`, with n from 1 to 7.
   - The grid is 9 x 5 slots from (9, 18). The scroller is 12 x 15, `container/creative_inventory/scroller` or `scroller_disabled`.
   - The background is `textures/gui/container/creative_inventory/tab_<background>.png`. The title shows unless `show_title` is false.
   - The search box sits at (82, 6), 80 wide, at most 50 characters, without a border and with white text. It shows only on the Search tab.
   - The Inventory tab has the armour slots, the offhand slot, the player model and the trash slot (tooltip `inventory.binSlot`).
4. **Which tabs show**
   - `shown(operator)`, with `operator` = the player may use game master blocks (level 2 or more) and the "Operator Items Tab" option is on. Vanilla's default for that option is off, so add the option.
   - The tab tooltip is the title.
   - Hovering an item shows its tooltip. On the search tab it also shows the tags matching a `#` query (`updateVisibleTags`).
5. **Contents**
   - A category tab shows `tab.displayed()`.
   - Search shows `search(operator)`, filtered by the query:
     - Empty shows everything.
     - `#tag` uses the item tags the commands already load, with an optional namespace before `:`.
     - Otherwise it is vanilla's name search over the localised names and the ids (`SessionSearchTrees.creativeNameSearch`).
     - Each change scrolls back to the top.
   - Saved Hotbars shows the nine saved rows.
   - Scrolling moves by rows (`canScroll` when there are more than 5 rows).
6. **Variants**
   - Expand each `Variants` entry into stacks. Until a kind is expanded, show its plain item once, and drop it when the tab already has the plain item (the painting).
   - What each kind holds:
     - `Potions`: every potion in registry order with `potion_contents`, filtered by the enabled features.
     - `EnchantedBooks`: the `data/minecraft/enchantment` registry. Without `all_levels` only the maximum level; with it, every level from the minimum to the maximum, all as `stored_enchantments`.
     - `Instruments { tag }`: the members of `data/minecraft/tags/instrument/goat_horns.json` with `instrument`.
     - `SuspiciousStews`: one stew per distinct effect list of the suspicious-effect blocks (the flowers).
     - `OminousBottles`: amplifier 0 to 4.
     - `Fireworks`: flight duration 1, 2 and 3.
     - `Paintings { tag, in_tag }`: `data/minecraft/painting_variant` entries in or out of `#minecraft:placeable`, sorted by area and then width (26.3 `PAINTING_COMPARATOR`).
     - `OminousBanner`: the white banner with the raid patterns and the name `block.minecraft.ominous_banner`.
     - `TestBlockModes`: one test block per `mode` (start, log, fail, accept).
     - `LightLevels`: light blocks from level 15 down to 0.
7. **Clicks (server-owned)**
   - Add `McClick::CreativeSet { slot, stack }`, vanilla's `ServerboundSetCreativeModeSlotPacket`.
   - The server accepts it only from a player in creative, for a valid slot or a drop (-1), with a known item id and a count no larger than the item's maximum stack size.
   - It writes the slot in that player's inventory, keyed by `PlayerId` and UUID.
   - Picking, shift-click, Q and the trash slot follow 26.3 `CreativeModeInventoryScreen.slotClicked`. Check it with javap when building it.
8. **Hotbars**
   - The file is per client: `hotbar.nbt` next to the game, not in the world, with nine hotbars of nine stacks (`HotbarManager.NUM_HOTBAR_GROUPS`).
   - In creative, holding C (`saveToolbarActivator`) and pressing 1 to 9 saves the current hotbar. It shows the overlay `inventory.hotbarSaved` with both keys.
   - Holding X (`loadToolbarActivator`) and pressing a digit loads it. Each of the nine stacks goes through `CreativeSet` slots 36 to 44.
9. **Multiplayer**
   - Every friend reads their own jar, so the tabs need no network messages.
   - The host checks every `CreativeSet` again, as above.

## Minecraft suggestion engine passes rustfmt again

Commit: the commit that adds this entry (branch `track-c`, adversarial review of the whole track).

### What changed

- One test assertion in the new `crates/minecraft_commands/src/suggest.rs`, added with `/execute` in 410015a, was wider than rustfmt allows. From that commit on, the file printed a diff under `git show :crates/minecraft_commands/src/suggest.rs | rustfmt --check --edition 2024`, which breaks the rule for new files. The assertion now has rustfmt's layout. Nothing else changes.

### Tests added

None. The existing tests are unchanged.

### In-game test steps

Nothing changes in game.

Kit: none needed.

### Decisions

- **The check is the printed diff, not the exit code.** rustfmt reading stdin with `--check` exits 0 even when it prints a diff. That is how the violation slipped through. A new file passes only when the command prints nothing.

## Minecraft block suggestions offer [ only for blocks with properties

Commit: the commit that adds this entry (branch `track-c`, adversarial review of the whole track).

### What changed

- A complete block id in `/setblock`, `/fill` or a `/fill` filter suggested `[` for every block. 26.3 `BlockStateParser.suggestOpenPropertiesOrNbt` (checked with javap) suggests `[` only when the block's state definition has properties. So `stone`, `glass` and `gold_block` get nothing, and `oak_stairs` gets `[`. Item ids are different: `ItemParser.suggestStartComponents` always suggests `[`, so `/give` is unchanged.
- `CommandHints` gains `blocks_with_properties` (serde default). `CommandHints::from_server` fills it from `CommandData::blocks` with the ids that have at least one property, so the glue needs nothing new.

### Tests added

- `suggest::tests::block_arguments_suggest_whole_coordinates_and_block_ids` now expects nothing after `glass`, and `[` at 26..26 after `oak_stairs`, both as a block and as a `/fill` filter.
- `suggest::tests::hints_come_from_the_server_players_and_registries` checks `blocks` and `blocks_with_properties` from the command data.

### In-game test steps

The chat does not use the engine yet (see the first entry). Once it does:

```
IW4L_MINECRAFT_KIT=stone*64
```

1. Type `/setblock ~ ~ ~ stone`. No `[` suggestion shows.
2. Type `/setblock ~ ~ ~ oak_stairs`. The list shows `[`. Screenshot.

Kit: none needed.

### Decisions

- **No `{` suggestion yet.** Vanilla also suggests `{` for a block with a block entity, and `[` or `{` after a block tag whose members have them. The hints do not carry block entities or tag members, so those stay out until the glue sends them.

## Minecraft experience math wraps like vanilla at huge levels

Commit: the commit that adds this entry (branch `track-c`, adversarial review of the whole track).

### What changed

- `Experience::needed` computed `112 + (level - 30) * 9` with plain `i32` arithmetic. From level 238,609,312 up, that overflows. A debug or test build then panics, for example after `/xp set @s 2147483647 levels` and any `/xp query @s points`, `/xp add` or `/xp set ... points`. Java's `Player.getXpNeededForNextLevel` wraps silently. The multiplication and addition now wrap the same way, so every build gives vanilla's number and none crashes.

### Tests added

- `experience::tests::levels_saturate_and_reset_below_zero` checks Java's wrapped values at levels `i32::MAX` (2,147,483,481), 477,218,606 (0) and 1,000,000,000 (410,065,250), plus giving points at `i32::MAX`.
- `experience::tests::xp_adds_sets_and_queries_like_vanilla` runs `/xp set @s 2147483647 levels`, `/xp query @s points` and `/xp set @s 100 points` end to end.

### In-game test steps

`/xp` needs the glue (see the `/xp` entry). Once it is in:

```
IW4L_MINECRAFT_KIT=stone*64
```

1. `/xp set @s 2147483647 levels`, then `/xp query @s points`: "Steve has 0 experience points", and the game keeps running.
2. `/xp add @s 10`: "Gave 10 experience points to Steve".

Kit: none needed.

## Minecraft block tag errors point at the last value like vanilla

Commit: the commit that adds this entry (branch `track-c`, adversarial review of the whole track). It also corrects the overflow level in the previous entry (238,609,312, where the addition overflows first).

### What changed

- A block tag with loose properties whose input ends right after a value, such as `/fill ... replace #minecraft:logs[axis=y`, put the "Expected closing ]" error at the end of the input. 26.3 `BlockStateParser.readVagueProperties` (checked with javap) keeps the start of the last value and moves the cursor back to it, so the context line ends at `[axis=` with `y` underlined. `read_properties` now does the same for tags. Plain blocks (`readProperties`) have no such reset and still point at the end, and so does a tag list that ends after a comma.

### Tests added

- `blocks::tests::block_states_parse_properties_and_nbt_like_vanilla`: `#logs[axis=y ` points at 11 (the `y`), `#logs[axis=y,` at 13 (the end), and `oak_stairs[facing=east ` at 23 (the end).

### In-game test steps

`/fill` needs the glue and the block tags (see the `/setblock` and `/fill` entries). Once they are in:

```
IW4L_MINECRAFT_KIT=stone*64
```

1. `/fill ~1 ~ ~1 ~2 ~1 ~2 stone replace #minecraft:logs[axis=y`: red "Expected closing ] for block state properties", and the grey line ends in `[axis=` followed by a red underlined `y` and `<--[HERE]`. Screenshot.

Kit: none needed.

### Decisions

- **Known difference, not fixed: malformed input can report a different error first.** The `/setblock` entry says block data is checked when the command runs with the same red line. That holds for a well-formed block state. When the state is also malformed, the parse-time error wins here: `stonee[` gives "Expected closing ]" where vanilla, which checks the id while parsing, gives "Unknown block type 'minecraft:stonee'". In the same way, `oak_stairs[foo]` gives "Expected value for property 'foo'" where vanilla gives "Block minecraft:oak_stairs does not have property 'foo'". Fixing this needs block data inside the argument parser, which the parser does not have.

## Minecraft /kit test steps overflow the inventory for real

Commit: the commit that adds this entry (branch `track-c`, adversarial review of the whole track). It changes only this log.

### What changed

- Step 3 of the `/kit` entry said three more `/kit` runs fill the inventory. They do not. The pickaxe, sword, stone, logs and torches take five slots per kit, while 16 bread per kit merge into one stack (`Inventory::add_item` merges first). Four kits use 21 of the 36 slots and nothing drops. The step now says six more runs: the seventh kit needs 37 slots, and its torches drop at the player's feet.

### Tests added

None. Only the log changed.

### In-game test steps

The corrected steps are in the `/kit` entry above.

Kit: as in the `/kit` entry.

## Minecraft block suggestions pass the workspace clippy rules

Commit: the commit that adds this entry (branch `track-c`, adversarial review of the whole track).

### What changed

- The block suggestion fix above used `Result::unwrap_or_default`, which the workspace `clippy.toml` disallows, so `clippy -p minecraft_commands --all-targets` warned. `block_state` now keeps the parsed id as an `Option` and checks it against both hint lists. The suggestions are unchanged.

### Tests added

None. `suggest::tests::block_arguments_suggest_whole_coordinates_and_block_ids` covers the same behaviour, and clippy reports nothing on `minecraft_commands`.

### In-game test steps

Nothing changes in game. The steps in the block suggestion entry above still apply.

Kit: none needed.

## Testing the whole track

Commit: the commit that adds this section (branch `track-c`, final verification). It changes only this log.

### Final verification

- `clippy -p minecraft_commands -p minecraft_terrain --locked --all-targets -- -A clippy::approx_constant`: nothing on `minecraft_commands`. The warnings left in `minecraft_terrain` are all in files the track did not touch (`mesh.rs`, `server_mobs.rs`, `item_icon.rs` and others); `class_file.rs`, `creative_tabs.rs` and `locate.rs` have none.
- `test -p minecraft_commands --locked`: 113 passed. `test -p minecraft_terrain --locked`: 478 passed, 3 ignored (the existing timing and local-pack diagnostics).
- Every new `.rs` file passes `rustfmt --check` on its committed content, and the track adds no comments.

### Commits, oldest first

| Commit | Subject |
| --- | --- |
| e1ae2b8 | Minecraft commands suggest completions like vanilla |
| bfc330e | Minecraft /clear removes items like vanilla |
| 3cc8bb8 | Minecraft commands can redirect like vanilla |
| 4adb791 | Minecraft /xp and /experience add, set and query experience like vanilla |
| 470433a | Minecraft /enchant enchants the held item like vanilla |
| 8424078 | Minecraft /setblock places blocks like vanilla |
| c9f6934 | Minecraft /fill fills areas like vanilla |
| d4ea858 | Minecraft /spawnpoint sets each player's respawn point like vanilla |
| 9ebf5d3 | Minecraft /locate finds structures and biomes like vanilla |
| 410015a | Minecraft /execute in parses dimensions like vanilla |
| fe350b8 | Minecraft /kit gives the .env kit again |
| fbc0253 | Minecraft resource errors point after the id like vanilla |
| 5aa18ba | Minecraft creative tabs are read from the game jar |
| fe4fe8a | Minecraft suggestion engine passes rustfmt again |
| 18a4b0f | Minecraft block suggestions offer [ only for blocks with properties |
| 4e8a127 | Minecraft experience math wraps like vanilla at huge levels |
| ebc7f49 | Minecraft block tag errors point at the last value like vanilla |
| 680f714 | Minecraft /kit test steps overflow the inventory for real |
| 19a00cc | Minecraft block suggestions pass the workspace clippy rules |

### One kit for every entry

```
IW4L_MINECRAFT_KIT=diamond_pickaxe+efficiency=5,diamond_sword,stick,stone*64,stone*30,oak_log*10,iron_boots,torch*64,bread*16
```

9 stacks: `minecraft:diamond_pickaxe` 1 (Efficiency V), `minecraft:diamond_sword` 1, `minecraft:stick` 1, `minecraft:stone` 64 and 30, `minecraft:oak_log` 10, `minecraft:iron_boots` 1, `minecraft:torch` 64, `minecraft:bread` 16. It covers the `/clear`, `/enchant` and `/kit` steps. Every other entry needs no items.

### Order

Load a fresh world with this kit and follow the entries' own steps in this order. Two numbers differ from the entries with this kit; they are given below.

1. **Works now, with this branch alone:**
   1. The redirect entry: `/help`, `/tp ~ ~5 ~`, `/tp`.
   2. The `/execute` entry, steps 1 to 4.
   3. The resource error entry: `/effect give @s nope`.
   4. Without the glue, every other new command gives its default answer, which is the check for now:
      - `/clear`: "No items were found on player Steve".
      - `/xp` and `/spawnpoint`: "No player was found".
      - `/enchant @s sharpness`: "Can't find element 'minecraft:sharpness' of type 'minecraft:enchantment'".
      - `/setblock` and `/fill`: "That position is not loaded".
      - `/locate structure minecraft:village_plains`: "Could not find a structure of type "minecraft:village_plains" nearby".
      - `/kit`: "No IW4L_MINECRAFT_KIT in the .env next to the game".
2. **Once the glue from the "After the split" notes is in**, in this order, because `/clear` needs the fresh kit and `/spawnpoint`'s `/kill` drops the inventory:
   1. The `/clear` entry, steps 1 to 5. Step 4 now reads "Removed 108 item(s) from player Steve" (24 stone, the pickaxe, sword, stick, boots, 64 torches and 16 bread), and the inventory is empty afterwards.
   2. The `/kit` entry, steps 2 to 4. Skip its step 1: the inventory is already empty. With this kit the sixth `/kit` is the first that no longer fits, and its 64 torches drop at your feet. Restore the `.env` line afterwards.
   3. The `/enchant` entry. Hold one of the swords, then the sticks.
   4. The `/xp` entry, then the experience wrap entry.
   5. The `/setblock` entry, the `/fill` entry and the block tag error entry.
   6. The `/locate` entry.
   7. The `/spawnpoint` entry, last.
3. **Once track A's chat uses the suggestion engine**, run the suggestion entry, the block suggestion entry and step 5 of the `/execute` entry.
4. **Creative tabs** have nothing to test in game until the creative screen lands.

# Second half: the glue

Branch `track-c`, group "glue". The crate split moved the command glue to `crates/minecraft_survival/src/commands.rs`, so the methods the first half left on their defaults are implemented there, against the `Server`, its `Level`s and its `Player`s.

## Minecraft commands act on the player they name

Commit: the commit that adds this entry.

### What changed

- `Host` no longer holds one player's survival, inventory and game mode. It borrows the server's `levels` (a `Level` per `Dimension`) and `players` (a `Player` per `PlayerUuid`), and every `CommandWorld` method finds the player by the `PlayerId` it is given: `/kill`, `/gamemode`, `/effect`, `/give` and `/tp` now act on the target, not on whoever typed the command.
- What the game knows of each player's body comes in as `Presence { pose, alive }` per UUID. `/tp` moves the target's pose, so a later `~` in the same batch starts from there, as before.
- `/give` drops what does not fit in the level the target stands in, `/summon` spawns in the source's level, and `/weather` goes to the overworld level, as vanilla keeps the weather there.
- `run(globals, levels, players, from, sent, presence)` replaces `run(globals, level, player, ...)`. `pump` takes the sender's id. `Handled.teleports` is keyed by UUID; `render_anim` applies only the local player's.
- `render_anim/src/minecraft_world.rs`: the command call moves above the lookup of the local player and its level, and reads the fallback pose through `players`.

### Tests added

- `commands::tests::commands_act_on_the_player_they_name_and_not_the_host`: the host runs `/gamemode creative Alex`, `/effect give Alex regeneration 10`, `/tp Alex ~ ~5 ~` and `/kill Alex`. Each line names Alex, Alex alone changes (the `~` are the host's), unknown ids answer nothing, and only the changed player is marked for saving.
- `host_effects_reach_survival_and_teleports_wait_for_the_world` now runs through the shared `Parts` helper, which builds real `Player`s by UUID.

### In-game test steps

Nothing new to see: the same commands must still work on yourself.

```
IW4L_MINECRAFT_KIT=stone*64
```

1. `/gamemode creative`, then `/gamemode survival`: both answer and switch.
2. `/tp ~ ~5 ~`: you go up 5 blocks.
3. `/give @s diamond 3`: the diamonds arrive with the pickup pop.
4. `/effect give @s regeneration 10`, then `/kill`: you die and respawn as before.

Kit: none needed.

### Decisions

- **Presence is per UUID and comes from the game each frame.** The MW2 sim owns the bodies, so the command server reads poses from it instead of storing a second copy.
- **`/weather` acts on the overworld**, like vanilla's `MinecraftServer.setWeatherParameters`, whatever level the source is in.

## Minecraft /clear empties the named player's inventory

Commit: the commit that adds this entry.

### What changed

- `Host::clear` finds the target by id and runs `minecraft_commands::clear_inventory` on that player's `Inventory`: the 41 storage, armour and offhand slots, the 2x2 crafting grid and the stack on the cursor. The inventory screen reads the same `Inventory` every frame, so it follows at once.
- New `commands::command_data(&TerrainStream) -> CommandData`, called once on the loading thread (`render_anim` `load()`, next to `registries`). It fills `item_tags` from the game data's `tags/item` (`minecraft_terrain::locate::tags`, nested `#` references expanded), so `/clear @s #minecraft:logs` and the `#` suggestions know the tags.

### Tests added

- `commands::tests::clear_takes_items_from_the_named_player_like_vanilla`: counting with 0, a partial clear, a tag, another player by name, the failure line, which slots are left (the boots, 24 stone, the cursor), and an unknown id.
- `commands::tests::command_data_reads_the_item_tags_of_the_game_data` (needs the 26.3 data, skipped without it): `#minecraft:logs` holds oak logs and crimson stems, with no `#` left.

### In-game test steps

```
IW4L_MINECRAFT_KIT=stone*64,stone*30,oak_log*10,iron_boots
```

1. `/clear @s minecraft:stone 0`: "Found 94 matching item(s) on player Steve" (your name), nothing disappears. Screenshot.
2. `/clear @s stone 70`: "Removed 70 item(s) from player Steve", 24 stone left.
3. `/clear @s #minecraft:logs`: "Removed 10 item(s) from player Steve".
4. `/clear`: "Removed 25 item(s) from player Steve" (24 stone and the boots). A second `/clear` answers in red "No items were found on player Steve". Screenshot.

Kit: `minecraft:stone` 64 and 30, `minecraft:oak_log` 10, `minecraft:iron_boots` 1.

### Decisions

- **The crafting grid and the cursor are only covered by the unit test.** The chat cannot be open while the inventory screen holds items there, so there is no in-game step for them. (An earlier draft of this entry had a garbled step 5 about the grid; it was removed in the next commit.)
- **Tags come from the unpacked data pack the world generator reads** (`datapacks/local/minecraft-26.3`), not from the jar: it is the same 26.3 data, already on disk, and read on the loading thread.

## Minecraft /xp fills the named player's experience bar and chimes like vanilla

Commit: the commit that adds this entry.

### What changed

- `Host::experience` and `set_experience` read and write the target's `survival.status` (`experience_level`, `experience_progress`, `total_experience`). The command never sets negative values; the glue clamps at 0 anyway.
- **The level-up chime.** New `CommandWorld::level_up(player)` (default: nothing). `experience.rs` calls it after an `add` whose vanilla `giveExperienceLevels` would play `minecraft:entity.player.levelup`: for levels, a positive amount landing on a multiple of 5; for points, a positive amount crossing one (`chimes`, checked against 26.3 `Player.giveExperienceLevels` and `giveExperiencePoints`, which call it once per level crossed). `set` never chimes, as `ServerPlayer.setExperienceLevels` and `setExperiencePoints` do not.
- `Survival::level_up_chime` is the rest of vanilla's check, moved out of `gain_experience` (which now calls it): nothing within 100 ticks of the last chime, volume `0.75 * min(level, 30) / 30`. The glue pushes the sound at the player's feet into its level's sounds, which `render_anim` plays.

### Tests added

- `experience::tests::only_added_experience_landing_on_every_fifth_level_chimes` (`minecraft_commands`): add 5 levels chimes, 3 more does not, 2 more (to 10) does, removing levels, `set` levels and points, a small points add, a large one crossing level 20, negative points and 0 levels; and `chimes` at its edges.
- `commands::tests::experience_commands_fill_the_named_players_bar` (`minecraft_survival`): `/xp add Alex 5 levels` changes Alex only and chimes for Alex, the query, 10 points to Steve (level 1, 3 of 9), the refused `set`, unknown ids, and clamping at 0.

### In-game test steps

```
IW4L_MINECRAFT_KIT=stone*64
```

1. `/xp add @s 5 levels`: "Gave 5 experience levels to Steve", the bar shows 5 and the level-up chime plays. Screenshot.
2. `/xp add @s 3 levels`: no chime (level 8). `/xp add @s 2 levels`: chime (level 10).
3. `/xp query @s levels`: "Steve has 10 experience levels".
4. `/xp set @s 30 points`: red "Cannot set experience points above the maximum points for the player's current level".
5. `/experience set @s 30 levels`: the bar shows 30, no chime. `/xp add @s -40 levels`: the bar is back at 0. Screenshot.
6. `/xp set @s 2147483647 levels`, then `/xp query @s points`: "Steve has 0 experience points" and the game keeps running.

Kit: none needed.

### Decisions

- **The chime is decided by the command, the rest timer by the player.** Only the command knows whether it added or set; only the player knows its tick count, so `level_up` carries the one to the other.

## Minecraft /enchant enchants the named player's held item

Commit: the commit that adds this entry.

### What changed

- `Host::enchantment(id)` reads the enchantment registry the world already loads (`GameData::enchantments`, from the jar): `max_level`, and `curse` when `#minecraft:curse` holds it.
- `Host::enchant(player, id, level)` works on the target's main-hand slot (`inventory.slots[selected]`), as 26.3 `EnchantCommand` does (checked with javap): an empty hand is `Itemless`; the item must be in the enchantment's supported items (`canEnchant`) and every enchantment it already has must be compatible (`isEnchantmentCompatible` over `getEnchantmentsForCrafting`: `stored_enchantments` for an enchanted book, which also makes the same enchantment twice incompatible), otherwise `Incompatible` with the item's name. Then `ItemEnchantments.Mutable.upgrade`: the level becomes the higher of the two, capped at 255, through `minecraftoss_entities::enchant::add_enchantment`, so the glint and tooltips follow the components as for any enchanted item.
- `registries().enchantments` was already filled from the same registry when the world loads (`render_anim`, since the split), so suggestions after `/enchant @s ` and `+` in `/give` list the ids.

### Tests added

- `commands::tests::enchant_upgrades_the_named_players_held_item`, on a small registry: Sharpness V, the level cap, Smite against Sharpness, Sharpness again at a lower level, Unbreaking III next to it (the component holds both), a curse on a stick, another player's selected slot, an empty hand, the curse flag and an unknown id, and the stored enchantments of a book.

### In-game test steps

```
IW4L_MINECRAFT_KIT=diamond_sword,stick
```

1. Hold the sword. `/enchant @s sharpness 5`: "Applied enchantment Sharpness V to Steve's item", the sword glints and its tooltip lists Sharpness V. Screenshot.
2. `/enchant @s sharpness 6`: red "6 is higher than the maximum level of 5 supported by that enchantment".
3. `/enchant @s sharpness`: red "Diamond Sword cannot support that enchantment".
4. `/enchant @s smite`: the same red line.
5. `/enchant @s unbreaking 3`: applied; the tooltip lists both. Screenshot.
6. Hold the stick, `/enchant @s sharpness`: red "Stick cannot support that enchantment". Select an empty slot, `/enchant @s unbreaking`: red "Steve is not holding any item".

Kit: `minecraft:diamond_sword` 1, `minecraft:stick` 1.

### Decisions

- **A stick takes no enchantment, not even Unbreaking**, because vanilla's `supported_items` for Unbreaking is `#minecraft:enchantable/durability`. The test registry lets a stick take Unbreaking only to reach the other player's slot.

## Minecraft /setblock and /fill change blocks on the level's server thread

Commit: the commit that adds this entry.

### What changed

- **The edit runs on the level's server thread**, where the vanilla `Level.setBlock` with its update flags already lives (`minecraftoss_world::level::Level`). The answer comes back a few frames later, as vanilla's would in multiplayer.
  - `minecraft_terrain/src/server.rs`: a generic `Command::Job` (`ServerHandle::run_job`), a closure run on the server thread whose result comes back in `Output::jobs`; and `ServerSim` primitives for commands: `command_block`, `command_registries`, `command_destroy` (`destroyBlock(pos, true)`: drops with no tool, the break particles and sound through `destroyed`), `command_place` (26.3 `BlockInput.place`, checked with javap: `Block.updateFromNeighbourShapes` unless strict, the given state again if that turns to air, the typed properties written back over it, then `setBlock` with `2 | 256`, or `2 | 816` when strict) and `command_updates` (`ServerLevel.updateNeighboursOnBlockSet`, which `SetBlockCommand` and `FillCommand` call afterwards unless strict). `command_block_entity` reads a block entity's tag.
  - `minecraft_survival/src/command_blocks.rs` (new): `SimBlocks`, the `minecraft_commands::BlockAccess` over a `ServerSim`, so the first half's `apply_edit` loop (filters, modes, counting, the deferred neighbour updates) runs unchanged; `describe` and `state_of` between block states and the commands' names and properties; `nbt_matches`, vanilla's `NbtUtils.compareNbt` with partial lists, for `/fill ... replace chest{...}`; and `loaded_at`, vanilla's `getLoadedBlockPos` order: the chunk must be loaded in the level, then y inside the build height and x and z inside 30,000,000.
  - `Entities` (`minecraft_survival/src/entities.rs`) gains `run_job` and `command_jobs`, where the server's answers wait.
- `Host::edit_blocks` sends the edit to the level named by the edit's dimension and answers `Answer::Later`; `Host::loaded` uses `loaded_at` on that level's scene; `Host::dimension` is the player's own level. Each frame `run` collects the finished jobs of every level and turns them into vanilla's lines with `edit_answer` and `CommandServer::reply`, broadcast to ops as `SetBlockCommand` and `FillCommand` do.
- The changed blocks come back to the client the way the server's own changes always do (`Output::changes`), so the scene, the saved chunks, the MW2 collision and the block sounds follow without new code in `render_anim`.
- `command_data` now also fills `blocks` (every block with its properties and values, in catalog order) and `block_tags`, so block ids and properties are checked like vanilla, the suggestions list block ids, `[` shows only for blocks with properties, and `#` tags work in `/fill ... replace`.
- **Dimension lookup:** `loaded_dimension(levels, id)` and `level_of(levels, id)` are the one place a dimension id becomes a loaded level. Only the overworld is loaded until phase 3, which only has to add its levels to the map.

### Tests added

- `server::tests::command_placements_shape_like_vanilla_and_jobs_answer` (`minecraft_terrain`, real terrain, skipped without the data): two fences connect, placing the same state again fails, strict fences stay unconnected, typed properties survive (the chain of shape updates connects the third fence as vanilla's does), destroying drops the fence and disconnects its neighbour, and a job sent through a `ServerHandle` answers.
- `command_blocks::tests` (`minecraft_survival`):
  - `nbt_predicates_compare_like_vanilla_compare_nbt`: empty predicates, partial and full list matching, an empty list, wrong values, missing keys and types, no block entity.
  - `positions_are_loaded_only_in_streamed_chunks_inside_the_world`: chunk edges, the build height and the world border.
  - `setblock_and_fill_run_on_the_level_like_vanilla` (skipped without the data): setblock, the same block again, keep, destroy with the gold block dropped, a fence fill that connects, a strict one that does not, a hollow 5x5x5 glass box (98) and again (0), void air above the world, and one destroyed block reported.
- `commands::tests::block_edits_answer_once_the_level_has_run_them`: no level gives "That position is not loaded", the Nether is an unknown dimension, and the late answers for both commands, success and failure, with the op broadcast for another player's edit.
- `commands::tests::command_data_reads_the_item_tags_of_the_game_data` also checks the block properties in catalog order and the block tags.

### In-game test steps

```
IW4L_MINECRAFT_KIT=oak_fence*16
```

1. `/setblock ~ ~-1 ~ minecraft:gold_block`: "Changed the block at x, y, z" and gold appears under your feet. Screenshot.
2. The same command again: red "Could not set the block".
3. `/setblock ~ ~-1 ~ stone keep`: red "Could not set the block".
4. `/setblock ~ ~-1 ~ stone destroy`: the gold breaks with particles and sound, a gold block drops, stone takes its place. Screenshot.
5. `/setblock ~2 ~ ~ oak_stairs[facing=east,half=top]`: an upside-down stair facing east.
6. `/fill ~1 ~ ~1 ~5 ~4 ~5 glass hollow`: "Successfully filled 98 block(s)", a glass box with air inside. The same command again: red "No blocks were filled". Screenshot.
7. `/fill ~1 ~ ~1 ~5 ~4 ~5 stone replace glass`: "Successfully filled 98 block(s)".
8. `/fill ~1 ~ ~1 ~5 ~4 ~5 air destroy`: the stone breaks with particles and drops cobblestone. Screenshot.
9. `/fill ~1 ~ ~3 ~5 ~ ~3 oak_fence`: a connected fence line. `/fill ~1 ~ ~5 ~5 ~ ~5 oak_fence strict`: five unconnected posts. Screenshot.
10. `/fill ~ ~ ~ ~100 ~100 ~100 stone`: red "Too many blocks in the specified area (maximum 32768, but specified 1030301)".
11. `/setblock ~ ~ ~ oak_stairs[half=up]`: red "Block minecraft:oak_stairs does not accept 'up' for half property" with the context line.
12. `/setblock 0 ~ 100000 stone` (far away): red "That position is not loaded".

Kit: `minecraft:oak_fence` 16 (only to compare with hand-placed fences; the commands need no items).

### Decisions

- **Server thread, not the client scene.** The level thread has vanilla's `setBlock` with every flag, the shape updates, the neighbour updates and the drops; doing it on the main thread would have needed a second copy of all of that.
- **One fill runs in one job**, as vanilla runs it in one tick; 32768 blocks is the cap, and the client applies the changes in the frame they arrive.
- **Block entity NBT in `/setblock ... chest{...}` is not written.** The fork's containers keep their items on the main thread (`Containers`), not in the level's block entities, so there is nothing for the tag to land in yet. NBT predicates compare against the level's block entity tags.
- **Unloaded chunks inside a fill are skipped.** Only the two corners are checked, as in 26.3; `setBlock` on a chunk the level does not hold changes nothing and does not count.

### After track D

- Containers replaced by `/setblock` or `/fill` (not in `destroy` mode) drop their items, because `render_anim` spills the main-thread `Containers` of any position that stops being a station. Vanilla 26.3 deletes them (flag 256, `UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS`). Once containers are block entities in the level, the command path should discard their contents instead of spilling.

## Minecraft /locate searches on the level's server thread

Commit: the commit that adds this entry.

### What changed

- `Host::locate` runs the search as a job on the server thread of the level the request names (vanilla runs `/locate` on the server thread too, and the server pauses while it searches), and answers `Answer::Later`. The job captures the level's `WorldGen` and gives up after `LOCATE_TIME_CAP` (20 s) from when it starts:
  - **structure**: `minecraft_terrain::locate::nearest_structure`, radius 100 (`findNearestMapStructure`);
  - **biome**: `nearest_biome`, radius 6400 (`findClosestBiome3d`);
  - **poi**: new `ServerSim::command_closest_poi`, vanilla's `PoiManager.findClosestWithType(predicate, pos, 256, ANY)` over the points of interest the level tracks for its villagers (beds, job sites, bells, beehives, portals, lodestones and lightning rods in loaded chunks).
- `run` turns a finished search into vanilla's line with `locate_answer` and `CommandServer::reply`, to the source only. A search that ran out of time answers like nothing was found and logs "Minecraft /locate ... gave up after 20 s".
- `command_data` now fills `structures`, `structure_tags`, `biomes`, `biome_tags` (from the level's `WorldGen`), `poi_types` (the 21 types of 26.3 `PoiTypes`, as `minecraftoss_entities::poi::PoiType::ALL`) and `poi_tags`, so unknown ids and tags are refused as vanilla refuses them, and the suggestions list them.

### Tests added

- `commands::tests::locate_answers_once_the_level_has_searched`: without a level the answer is "not found" at once; then a tag with its found member and the horizontal distance, a biome with its height and the 3D distance, not found, a timeout, and a POI miss.
- `commands::tests::command_data_reads_the_item_tags_of_the_game_data` also checks the structure, biome and POI ids and tags (skipped without the data).
- `server::tests::command_placements_shape_like_vanilla_and_jobs_answer` (`minecraft_terrain`) also finds a lectern placed by a command as the nearest librarian site, and no home.

### In-game test steps

```
IW4L_MINECRAFT_KIT=lectern,white_bed
```

1. `/locate structure minecraft:village_plains`: after a moment, "The nearest minecraft:village_plains is at [x, ~, z] (N blocks away)" with the coordinates in green. Screenshot.
2. `/tp @s x 120 z` with those numbers: a plains village is below you. Screenshot.
3. `/locate structure #minecraft:village`: the line names the tag and the village type in brackets.
4. `/locate biome minecraft:cherry_grove`, then `/tp` there: pink cherry trees. Screenshot.
5. `/locate structure minecraft:fortress`: red "Could not find a structure of type "minecraft:fortress" nearby".
6. Place the lectern somewhere, wait a second, `/locate poi minecraft:librarian`: its position, with `~` as the height. Screenshot.
7. `/locate poi minecraft:home` with no bed placed: red "Could not find a point of interest of type "minecraft:home" within a reasonable distance". Place the bed and repeat: its position.
8. `/locate structure minecraft:nope`: red "There is no structure with type "minecraft:nope"".

Kit: `minecraft:lectern` 1, `minecraft:white_bed` 1.

### Decisions

- **Server thread rather than a thread of its own.** That is where vanilla searches, it already holds the POIs, and the answer travels the same way as the block edits. A long search stalls mobs (not the game), as vanilla's stalls the server; the 20 s cap bounds it.
- **POI ids are the code registry of 26.3** (`PoiTypes`), listed from `PoiType::ALL`; the tags come from the data like the others.

## Minecraft /kit reads the .env again for every call

Commit: the commit that adds this entry.

### What changed

- `Host::kit_line` reads `IW4L_MINECRAFT_KIT` from the `.env` the game started with, found the way `asset_transport::load_dotenv` finds it (next to the exe, otherwise the nearest one above the working directory), with `minecraft_commands::kit::read_env_file`. A changed line counts at the next `/kit` without a restart. Without any `.env` it falls back to the process environment, so a kit given on the command line still works.
- `commands::env_file()` is that search. `run` only does it in a frame that has chat input.
- `Host::give` no longer needs the level's entities: the stack is built from the world's recipe book (`GameData::recipes`, the one the entities use), it goes into the target's inventory, and only the drops of what does not fit and the pickup pops need the level. So `/give` and `/kit` work the same before the mobs are up.

### Tests added

- `commands::tests::kit_reads_the_env_file_again_and_gives_like_give`: a CRLF `.env` with another key first, `/kit` and `/kit Alex` with `/give`'s lines, 70 stone split into stacks and an enchanted sword in both inventories, the file changed to `torch*8`, and the key removed.

### In-game test steps

```
IW4L_MINECRAFT_KIT=diamond_pickaxe+efficiency=5,diamond_sword,stone*64,oak_log*64,torch*64,bread*16
```

1. Load the world, then `/clear`.
2. `/kit`: one "Gave ..." line per entry, and the items are back, the pickaxe enchanted. Screenshot.
3. Run `/kit` six more times: the seventh is the first that no longer fits the 36 slots, and its torches drop at your feet. Screenshot.
4. Without restarting, change the `.env` line to `IW4L_MINECRAFT_KIT=torch*8` and save. `/kit`: "Gave 8 [Torch] to Steve". Put the line back afterwards.

Kit: `minecraft:diamond_pickaxe` 1 (Efficiency V), `minecraft:diamond_sword` 1, `minecraft:stone` 64, `minecraft:oak_log` 64, `minecraft:torch` 64, `minecraft:bread` 16.

### Decisions

- **The `.env` found at start is the one read again.** Reading only the exe's folder would miss the developer setup, where the `.env` sits in the repository above the working directory.

## Minecraft /execute in looks up the server's loaded levels

Commit: the commit that adds this entry.

### What changed

- `Host::dimension_scale` answers only for a level the server has loaded, through `level_of` and `loaded_dimension`, the one lookup from a dimension id to a level (added with `/setblock`). Until phase 3 only the overworld is in `Server::levels`, so `/execute in minecraft:the_nether` and `minecraft:the_end` answer vanilla's red "Unknown dimension '...'" (`DimensionArgument.getDimension` throws it for a level the server does not have). Phase 3 only has to put its levels in the map; nothing in the commands changes.
- The scale is the dimension type's `coordinate_scale`, read from the game data (`commands::coordinate_scale`: 1 in the overworld, 8 in the Nether, 1 in the End), so `execute in` divides or multiplies x and z like `CommandSourceStack.withLevel`.
- `Host::dimension(player)` (added with `/setblock`) is the player's own level, which is where `~`, `/setblock`, `/fill`, `/locate` and `/spawnpoint` act without `execute in`.
- `teleport_to_dimension` stays on its default (refused): there is no second level to move a player into until the dimension switch.

### Tests added

- `commands::tests::execute_in_finds_only_loaded_levels`: with no level even the overworld is unknown; with the overworld loaded (needs the data), `in minecraft:overworld run seed` runs and the Nether and the End are unknown; the coordinate scales read from the data (8 for the Nether, 1 for the End and for an unknown id); `dimension_scale` for a loaded and an unloaded level.

### In-game test steps

```
IW4L_MINECRAFT_KIT=stone*64
```

1. `/execute in minecraft:the_nether run tp ~ ~ ~`: red "Unknown dimension 'minecraft:the_nether'", and you do not move. Screenshot.
2. `/execute in minecraft:the_end run seed`: red "Unknown dimension 'minecraft:the_end'".
3. `/execute in minecraft:overworld run tp ~ ~10 ~`: you go up 10 blocks.
4. `/execute in minecraft:overworld run setblock ~ ~-1 ~ gold_block`: the gold appears under you, as without `execute`.
5. `/execute positioned 0 100 0 run tp ~ ~ ~`: you land at 0.5 100 0.5.

Kit: none needed.

### Decisions

- **The lookup is by the level map, not by the `Dimension` enum.** `Dimension::parse` also accepts `nether` and `end`; vanilla only knows `minecraft:the_nether` and `minecraft:the_end`, so the lookup compares full ids.

## Minecraft chat keeps the command hints up to date

Commit: the commit that adds this entry.

### What changed

- `frame::McChat` gains `hints` (`minecraft_commands::CommandHints`) and `command_level` (the local player's permission level), which the chat UI passes to `minecraft_commands::complete`. `clear_messages` resets both when a match ends. `frame` now depends on `minecraft_commands` (one line in `Cargo.lock`).
- New `commands::refresh_hints(&mut hints, &chat, loaded dimensions, aim)`, called by `render_anim` every frame:
  - the registry and data lists (items, entities, effects, enchantments, item and block tags, blocks, structures, biomes, POI types and their tags) are rebuilt with `CommandHints::from_server` only when the server's lists change size, which happens once while the world loads;
  - the online player names and the loaded dimensions (only the overworld until phase 3) are replaced when they differ;
  - `hit` is the exact point where the crosshair ray meets the block (`Hit::point`, vanilla `hitResult.getLocation()`) and `hit_block` that block, both `None` when no block is in reach. The ray uses the player's block reach (4.5, or 5 in creative) and the crouching eye, as the hand's targeting does.
- `render_anim` sets `command_level` from the local player's id each frame.

### Tests added

- `commands::tests::hints_follow_the_server_and_the_point_in_view`: names and the dimension at first, the lists rebuilt after the registries and block data change (with `blocks_with_properties`), a second player and the Nether, the hit point and block, `/tp ` suggesting "3.50 64.00 -2.13" from them, and a player leaving and the hit clearing without the lists being rebuilt.
- `frame` `clearing_messages_keeps_the_sent_history` checks that the hints and the level are reset.

### In-game test steps

Nothing shows yet: the chat UI that draws the suggestions comes in the next group of this track and reads `McChat.hints`. The suggestion entries' steps (`/ga`, `/give @s pick`, `/tp ` while looking at a block, `/execute in `) are the check once it is in.

Kit: none needed.

### Decisions

- **The hints live in `McChat`**, next to the input they complete, as the brief planned; in multiplayer the host sends each friend theirs and the friend's client writes them there.
- **The static lists are compared by length, not rebuilt every frame.** They only change while a world loads; cloning about 5,000 ids per frame would be wasted work.

## Minecraft hands stay still while an MW2 grenade is up

Commit: the commit that adds this entry. It is the leftover from track A's "After the split" notes (track A commit 4, the grenade showing over a Minecraft item).

### What changed

- `render_anim/src/minecraft_world.rs` `update`: while the local player's MW2 weapon state is in the offhand family (`WeaponState::OffhandInit` to `OffhandEnd`, 0x10 to 0x15, the same range as `weapon_iw4`'s `in_offhand_family`), the hand counts as busy, so the left button neither punches, nor breaks in creative, nor mines; and it is not idle, so the right button neither uses nor places the selected Minecraft item. The state is read from that player's own playerstate (`ps.weaponstate_primary`).

### Tests added

None: the check is one range over `weapon_iw4`'s named states inside the frame update, which has no unit harness. The in-game steps cover it.

### In-game test steps

Pick a class with Frag and Flash.

```
IW4L_MINECRAFT_KIT=cobblestone*64,diamond_pickaxe
```

1. Select the cobblestone, look at the ground and hold G (cook the frag). While it is up, right-click: no block is placed. Release to throw. Screenshot mid-throw.
2. Select the pickaxe, look at a block, press G and hold the left button during the throw: the block does not start breaking and the crack shows only after the gun is back.
3. Look at a cow, press B (flash) and left-click during the throw: the cow is not hit.
4. After each throw, right-click places and left-click mines again as before.

Kit: `minecraft:cobblestone` 64, `minecraft:diamond_pickaxe` 1.

### Decisions

- **Small and local, so it is done here rather than left for track B**: two lines in `update`, no new state.

## After track D (glue group)

- **`/spawnpoint`**: track D's per-player respawn point is not on `minecraft-survival` yet (no respawn commit in `git log minecraft-survival`), so `CommandWorld::set_respawn` keeps its default and `/spawnpoint` still answers "No player was found". Once track D lands:
  1. `Host::set_respawn(player, respawn)`: find the player by id (`player_mut`) and store the `Respawn` in track D's per-player respawn field, keyed by the player's UUID; return `true`. Track D's player file writes it under vanilla's `respawn` key (`{dimension, pos: [x, y, z], yaw, pitch, forced: true}`).
  2. Respawning reads it before the world spawn, as the `/spawnpoint` entry's "After the split" notes describe (forced: only room to stand is checked, otherwise the world spawn and `block.minecraft.spawn.not_valid`), and faces the stored yaw and pitch.
  3. Add a test next to `commands::tests::experience_commands_fill_the_named_players_bar`: `/spawnpoint Alex 10 64 -4 90 0` sets Alex's point only, and `/spawnpoint` without a position takes the source's feet.
- **Containers replaced by `/setblock` or `/fill`** drop their items instead of vanishing (see the `/setblock` and `/fill` entry).

## Testing the glue group

### Final verification

- `test -p minecraft_commands -p minecraft_survival --locked`: 114 and 144 passed. `test -p minecraft_terrain --locked`: 479 passed, 3 ignored (the existing timing and local-pack diagnostics). `test -p frame --locked`: 14 passed. The tests that need the 26.3 data ran (`MINECRAFTOSS_ROOT` set by `scripts/cargo-slot.ps1`).
- `clippy -p minecraft_survival -p minecraft_commands -p minecraft_terrain -p frame -p render_anim --all-targets -- -A clippy::approx_constant`: nothing on the lines this group touched; the warnings left are older (`entities.rs` 75 and 587, `server.rs` 313, 1106 and 1246, `minecraft_world.rs` 122, 378 and 387, and files the group did not touch).
- `check -p launcher --locked` passes after every commit. The new `command_blocks.rs` passes `rustfmt --check` on its committed content, and the group adds no comments.

### Commits, oldest first

| Commit | Subject |
| --- | --- |
| eefc73b | Minecraft commands act on the player they name |
| 2fec35a | Minecraft /clear empties the named player's inventory |
| 6fafe57 | Minecraft /xp fills the named player's experience bar and chimes like vanilla |
| 8b6ca64 | Minecraft /enchant enchants the named player's held item |
| cf013fd | Minecraft /setblock and /fill change blocks on the level's server thread |
| d36e020 | Minecraft /locate searches on the level's server thread |
| 131ccc8 | Minecraft /kit reads the .env again for every call |
| 62e91aa | Minecraft /execute in looks up the server's loaded levels |
| c0d10ad | Minecraft chat keeps the command hints up to date |
| 6f9b45c | Minecraft hands stay still while an MW2 grenade is up |

### One kit for the whole group

```
IW4L_MINECRAFT_KIT=diamond_pickaxe+efficiency=5,diamond_sword,stick,stone*64,stone*30,oak_log*10,iron_boots,torch*64,bread*16,cobblestone*64,oak_fence*16,lectern,white_bed
```

13 stacks: `minecraft:diamond_pickaxe` 1 (Efficiency V), `minecraft:diamond_sword` 1, `minecraft:stick` 1, `minecraft:stone` 64 and 30, `minecraft:oak_log` 10, `minecraft:iron_boots` 1, `minecraft:torch` 64, `minecraft:bread` 16, `minecraft:cobblestone` 64, `minecraft:oak_fence` 16, `minecraft:lectern` 1, `minecraft:white_bed` 1.

### Order

Load a fresh world with this kit, then follow the entries' own steps in this order:

1. The grenade entry (it uses the cobblestone and the pickaxe; place a few blocks only).
2. The `/clear` entry, steps 1 to 3 as written ("Found 94", "Removed 70", "Removed 10"). Step 4 then removes everything left, so it reads "Removed 190 item(s) from player Steve" with this kit if you placed no cobblestone (24 stone, the pickaxe, sword, stick, boots, 64 torches, 16 bread, 64 cobblestone, 16 fences, the lectern and the bed), fewer by each cobblestone you placed.
3. The `/kit` entry, steps 2 to 4: `/kit` gives the whole kit back, one line per entry. Step 3's count is for that entry's own kit; with this one the fifth `/kit` after the `/clear` is the first that no longer fits the 36 slots: from its sword on, what does not merge into a stack drops at your feet.
4. `/clear`, then `/kit` once, then the `/enchant`, `/xp`, `/setblock` and `/fill`, `/locate` and `/execute in` entries.
5. The suggestion steps wait for the chat UI group.

# Second half: the chat UI

Branch `track-c`, group "chat-ui". The previous commit is 2a3d60e (the glue group's test order).

## Minecraft chat suggests commands like vanilla

Commit: the commit that adds this entry.

### What changed

- `frame::McChat` gains `suggestions` (`McSuggestions`: the pending completion, the input colours, the usage lines and their start, the shown list, the grey inline text and vanilla's `allowSuggestions` and `keepSuggestions` flags) and `language` (the 26.3 language, shared, to resolve usage and error lines). `close()` forgets the suggestions; `clear_messages` keeps the language.
- New `crates/hud/src/minecraft_suggestions.rs`, vanilla 26.3 `CommandSuggestions` and its `SuggestionsList` (checked with javap, with `ChatScreen` and `EditBox`), on top of `minecraft_commands::complete` and the `CommandHints` the glue group keeps in `McChat`:
  - **When it updates** (`updateCommandInfo`): on every edit of the input, not on cursor moves, as the `EditBox` responder only fires on a value change. Opening the chat computes it with suggestions not yet allowed, so `/` opens with no list; the first typed character shows it.
  - **Command lines** show the list by themselves at most 10 lines of 12 px, above the input at the x of the word it completes (clamped to the input box), with `0xD0000000` behind each line, the selected line yellow and the others grey, and dotted white lines above or below when more lines are hidden.
  - **Chat lines** (no `/`) complete online player names at the last word, on Tab only.
  - **Keys** (`SuggestionsList.keyPressed`, then `CommandSuggestions.keyPressed`): with the list shown, Up and Down move the selection without writing; Tab writes the selection (the first Tab writes the selected line, later ones move first), Shift+Tab moves back; Esc hides the list and the chat stays open. With the list hidden, Tab shows it, Up and Down walk the history and Esc closes the chat.
  - **Writing a suggestion** (`useSuggestion`) keeps the list, moves the cursor to the end of the written text and recolours the input; the next edit rebuilds the list.
  - **History** (`moveInHistory`): a line from the history hides the list until the next edit or Tab.
  - **Grey inline text** (`EditBox.setSuggestion`): the rest of the selected suggestion in `0xFF808080` right after the text, only while the cursor is at the end.
  - **Usage** (`extractUsage`): with the list hidden, the usage lines from the engine (grey node usage, or the parse error with its position and context) stack upward from 27 px above the bottom, at the start of the argument, each on `0xD0000000`.
  - **Input colours** (`formatText`): the engine's spans colour the input, shifted with the part of the input in view.
- `hud/src/minecraft_chat.rs`: `apply_typed` gives the suggestions the key first and calls the update after an edit; `draw` colours the input and draws the inline text and the list or usage.
- `render_anim/src/minecraft_world.rs`: one line fills `McChat.language` from the inventory UI's language once it is loaded.

### Tests added (`hud` `minecraft_suggestions::tests`, 11; `frame` `clearing_messages_keeps_the_sent_history` extended)

- `/ga` shows `gamemode` and `gamerule`, the first selected, "memode" grey inline, `/` grey and `ga` red; `/` alone shows no list until Tab.
- Tab writes `/gamemode` with the cursor at 9, Tab cycles to `/gamerule` and back, Shift+Tab goes back; a typed space rebuilds the list.
- Up and Down move in the list without writing and reset the Tab cycle; they walk the history only while the list is hidden, and a history line keeps the list hidden.
- Esc hides the list first and closes the chat second.
- A chat line completes `st` to `Steve` on Tab only; a blank line has nothing.
- `/give ` has the grey usage `<targets> <item> [<count>]` at position 6; `/gamemode god` has the error "... at position 13: ...memode god<--[HERE]" and `god` red.
- `/kill Steve` colours `Steve` aqua, and the colours follow the part of the input in view.
- 14 suggestions scroll ten lines at a time and wrap both ways.
- The list rectangle, the usage position and rows, and the clamp to the input box match vanilla's numbers.
- Closing the chat forgets the suggestions; reopening starts clean.

### In-game test steps

Kit: none needed.

1. Press `/`. The chat opens with `/` and no list. Type `ga`: the list above the input shows `gamemode` (yellow) and `gamerule` (grey), `memode` shows grey after the cursor, and `ga` is red. Screenshot 1.
2. Press Tab: the input reads `/gamemode`. Tab again: `/gamerule`. Shift+Tab: `/gamemode`.
3. Type a space: the game modes show. Press Down twice: the yellow line moves and the input does not change. Tab writes the yellow one.
4. Clear the line and type `/give @s pick`: the pickaxes show. Esc: the list goes, the chat stays open. Esc again closes the chat.
5. Type `/give ` and press Esc: the grey line `<targets> <item> [<count>]` shows above the input, starting under the space after `give`. Screenshot 2.
6. Type `/gamemode god`: `god` is red, and the white line reads "Unknown game mode: god at position 13: ...memode god<--[HERE]". Screenshot 3.
7. Type `/give @s stone 5`: `/give` grey, `@s` aqua, `stone` yellow, `5` green. Screenshot 4.
8. Clear to `/` and press Tab: every command shows, ten at a time. Hold Down: the list scrolls, with a dotted white line on the side that has more. Screenshot 5.
9. Close the chat, press T, type `hi` and the first two letters of your player name, then Tab: your name shows; Tab again writes it.
10. Look at a block and type `/tp `: the block's coordinates show with two decimals.
11. Send `/seed`, open the chat with T and press Up: `/seed` comes back with no list over it.

### Decisions

- **Error lines in the usage area stay white.** `extractUsage` draws them with colour -1 and the error components carry no style; the red is the input past the parse error, as `formatText` does. (Same decision as the suggestion engine's entry.)
- **Keyboard only.** Vanilla also selects a line under the mouse, writes it on a click and scrolls it with the wheel when the mouse is over it. The chat UI has no mouse position yet; the wheel still scrolls the chat history. Left for a later commit.
- **The "Command Suggestions" option is always on**, its vanilla default; there is no options screen for it.
- **Suggestions follow the input, not the cursor**, as vanilla: moving the cursor with the arrows or Home keeps the list until the next edit.
- **The language lives in `McChat`** as a shared map, filled once by `render_anim`; the usage lines are resolved when drawn, so a language that loads late still shows.

## Minecraft chat display tests build against the per-player command host

Commit: the commit that adds this entry. It fixes the `render_anim` test build that eefc73b ("Minecraft commands act on the player they name") broke: the glue group ran the `minecraft_survival` tests but not `render_anim`'s, whose `minecraft_chat` tests still built the old one-player `Host`.

### What changed

- `render_anim/src/minecraft_chat.rs` tests only: `Parts` now holds the levels, the players by UUID and their presence, as `minecraft_survival`'s own test helper does, and builds the host player from the command server's profile. `pump` takes the sender's id, `mode_changed` is the list of changed players, and the checks read the player's survival and game mode through `Parts::player`.

### Tests added

None; the five existing tests (`sent_lines_come_back_as_resolved_chat_lines`, `host_commands_reach_survival_the_clock_and_the_rules`, `gamemode_switches_the_host_player_mode`, `death_messages_are_plain_system_lines`, `summon_answers_follow_the_feedback_rule`) build and pass again. `test -p render_anim --locked`: 35 passed.

### In-game test steps

Nothing changes in game. Kit: none needed.

# Second half: the creative inventory

Branch `track-c`, group "creative". The previous commit is a299c9d (the chat UI group's last commit).

## Minecraft creative inventory has the vanilla tabs

Commit: the commit that adds this entry.

### What changed

Checked with javap against 26.3 `CreativeModeInventoryScreen` (and its `ItemPickerMenu`, `SlotWrapper` and `CustomCreativeSlot`), `AbstractContainerScreen` and `InputConstants`.

- **Shared types** (new `crates/frame/src/minecraft_creative.rs`): `McCreative` in `MinecraftUi.creative` holds the shown tabs (`McCreativeTab`: id, title, row, column, aligned right, kind, title shown, scroll bar, background, icon), the selected tab id, the search text with its cursor, the scroll offset and the grid's stacks.
  - The selected tab is kept between openings like vanilla's static `selectedTab`, and falls back to the first category tab when it is not shown.
  - The vanilla scroll maths live here too: `hidden_rows`, `can_scroll` (more than 45 items), `first_row` (`getRowIndexForScroll`), `scroll_after_wheel` (`subtractInputFromScroll`), `scroll_for_drag` and `scroller_y`.
  - `McScreen::Creative(McTabKind)`, `McSlot::Creative(index)` (an index into the tab's items, so the grid's slots follow the scroll), `McSlot::Trash` and `McClick::Clone(slot)` (the middle button).
- **Server-owned slot setting** (new `crates/minecraft_survival/src/creative.rs`): `set_slot(mode, inventory, slot, stack, known)` is vanilla's `SetCreativeModeSlot` handling, on the player the caller looked up by UUID.
  - It refuses a player without infinite materials, an item id that is not in the item registry, a count above the item's maximum stack size (the `max_stack_size` component first), and a menu slot outside 1 to 45.
  - Menu slots map like `InventoryMenu`: 1-4 crafting, 5-8 head to feet, 9-35 main, 36-44 hotbar, 45 offhand.
  - `CreativeSlot::Carried` sets the cursor and `CreativeSlot::Drop` returns the stack to throw. `apply` runs a list of sets and reports refusals.
- **Click rules** (same file, pure), from `slotClicked`:
  - `grid_click` for a slot of the creative grid: with an empty cursor a click takes the shown stack (one item) and shift a full stack; clicking the same item again adds one (shift fills the stack) and the right button takes one away; clicking another item or an empty grid slot deletes the cursor stack (the right button takes one away).
  - A number key (or F) puts a full stack in that slot, Q throws one and Ctrl+Q a full stack, the middle button takes a full stack.
  - `trash_click` deletes the cursor stack, and with shift empties menu slots 1 to 45.
  - `hotbar_quick_move` deletes a hotbar stack shift-clicked under an item tab (`ItemPickerMenu.quickMoveStack`).
  - `load_hotbar` is ready for the saved hotbars.
- **Client side** (new `crates/render_anim/src/minecraft_creative.rs`):
  - Reads the tabs from the game jar once, on its own thread, with the `creative_tabs` reader.
  - Publishes the shown tabs (`shown(operator)`, titles from the language, icons as stacks).
  - Builds the selected tab's stacks when the tab, search text or operator flag changes. A variant entry shows its plain item once until variants are expanded, and is dropped when the tab already has that plain item.
  - Turns the HUD's creative clicks into `set_slot` calls on the local player's inventory, and throws what `Drop` returns.
  - In creative it turns the plain inventory screen into the creative one. Icons are made only for the 45 shown stacks and the tab icons.
- `render_anim/src/minecraft_inventory.rs`: publishes the creative tabs and items through the same stack conversion as every other slot. The inventory tab accepts armour and the offhand like the survival inventory.
- `render_anim/src/minecraft_world.rs`: the creative clicks run just before the vanilla container clicks, with the item registry as the known-item check. The operator flag is "level 2 or more" and the new "Operator Items Tab" option.
- `render_anim/src/minecraft_gui.rs`: the atlas also holds `textures/gui/container/creative_inventory/tab_items`, `tab_item_search` and `tab_inventory` and the `container/creative_inventory/` sprites. The `itemGroup.` and `inventory.` texts are loaded.
- **HUD** (new `crates/hud/src/minecraft_creative.rs`, hooked into `minecraft_inventory.rs`):
  - In creative, E opens the creative screen on the kept tab, with the search text and scroll cleared (vanilla `init` selects the tab again).
  - Layout: 195 x 136. Tabs are 26 x 32 at `column * 27`, or `195 - 27 * (7 - column) + 1` when aligned right. Top tabs are drawn at y -28 and bottom ones at 132; they are clicked over y -32 and 136 and hovered over the inner 21 x 27. Unselected tabs sit behind the background and the selected one in front, with the icon at (x + 5, y + 8 +/- 1).
  - Item tabs: the 9 x 5 grid from (9, 18), the hotbar at y 112, the scroller at x 175 (`scroller` or `scroller_disabled`).
  - The inventory tab: helmet (54, 6), chestplate (54, 33), leggings (108, 6), boots (108, 33) with their empty-slot icons, offhand (35, 20), main inventory from (9, 54), hotbar at 112, trash at (173, 112) with the tooltip `inventory.binSlot`, and the player's MW2 body in (73, 6)-(105, 49) at vanilla's smaller scale (20 against 30).
  - The title is at (8, 6) in 0x404040 when the tab shows it. Tab tooltips are the titles.
  - Tabs are selected on the left button's release over a tab (26.3's left button is `MOUSE_BUTTON_LEFT = 1`). The wheel and the scroller move by rows. A click on a tab is never a click outside.
  - The search box is at (82, 6), 80 wide, 50 characters, white text with a shadow, a blinking `_` or bar cursor and a blue selection. It edits with the chat's editing keys (arrows, Home, End, Ctrl+A/C/V/X, Backspace and Delete with Ctrl for words).
  - On the search tab every key goes to the box except Esc, and a number key over a grid stack still puts it on the hotbar. The chat key on another tab opens the search tab.
- **Option** `minecraft_operator_items` in `settings.cfg` (`GameSettings.minecraft_operator_items`), off by default like vanilla's "Operator Items Tab".

### Tests added

- `frame` `minecraft_creative::tests` (2): the kept tab and its fallback; the scroll maths (rows, wheel, drag, scroller, first shown index).
- `minecraft_survival` `creative::tests` (8):
  - menu slot mapping both ways;
  - refusals (survival, unknown id, too many, slot 0) and every target (hotbar, helmet, crafting, cursor, drop, empty);
  - the `max_stack_size` component;
  - grid clicks (one, shift, add one, full, right, another item, empty slot);
  - number key, F, throw, Ctrl+throw and middle button;
  - the trash and shift-trash (the cursor is kept);
  - shift on the hotbar; loading nine slots.
- `render_anim` `minecraft_creative::tests` (2): variant entries show their plain item once; tab titles from the language. `minecraft_gui::tests::the_real_gui_textures_and_fonts_load_into_one_atlas` also checks the creative sheets and sprites (the atlas still fits 2048 high).
- `hud` `minecraft_creative::tests` (5): the item tab grid and its scroll offset; the inventory tab slots and empty icons; tab positions, hit areas, hover areas and sprites; selecting a tab clears the search and scroll (not when the search tab is chosen again), and so does reopening; the search box typing, deleting, Ctrl+A and the 50 character cap.
- `console` `the_operator_items_tab_option_is_off_until_chosen_and_survives_a_restart`.

### In-game test steps

Kit:

```
IW4L_MINECRAFT_KIT=stone*64,diamond_sword,iron_helmet
```

`minecraft:stone` 64, `minecraft:diamond_sword` 1, `minecraft:iron_helmet` 1.

1. Load the world and type `/gamemode creative`. Press E: the creative screen opens on Building Blocks (bricks icon) with "Building Blocks" at the top left. Five tabs sit above on the left, Saved Hotbars and Search above on the right, five tabs below and the chest (Survival Inventory) below on the right. Screenshot 1.
2. Hover the tabs: each shows its name. Click Colored Blocks: wool, terracotta, concrete and glass show. Scroll with the wheel and drag the scroller: the grid moves by rows. Screenshot 2.
3. Click a white wool: one wool on the cursor. Click it again: two. Shift-click it: 64. Right-click it: 63. Click an orange wool: the cursor is empty.
4. Hover a block and press 3: a full stack lands in hotbar slot 3. Press F over another: a full stack in the offhand. Middle-click a block: a full stack on the cursor; put it in the hotbar row at the bottom.
5. Hover a block and press Q: one is thrown. Ctrl+Q throws a full stack.
6. Shift-click a stack in the bottom hotbar row: it is deleted.
7. Click the chest tab: armour slots, offhand, inventory, hotbar, the trash at the bottom right and your MW2 body. Screenshot 3. Hover the trash: "Destroy Item". Pick up the stone and click the trash: it is gone. Shift-click the trash: the whole inventory is emptied (the guns come back on their own).
8. Click Building Blocks, then press T: the Search tab opens with its text box and every item. Type `wool`: the text shows in the box (filtering comes with the next commit). Esc closes the screen; E opens it again on Search with the box empty.
9. `/gamemode survival`, then E: the survival inventory.
10. Optional: add `minecraft_operator_items=true` to `iw4l-artifacts\settings.cfg` before starting the game. In creative, the Operator Utilities tab (command block icon) shows at the bottom right, next to the chest.

### Decisions

- **A left click takes one item, shift-click a full stack.** 26.3's `slotClicked` gives the shown stack's count (one) on a plain click, and the maximum stack size only with shift, the middle button or a number key. The brief's "left takes a full stack" is how Bedrock does it, so the Java bytecode wins.
- **The cursor stack is server-owned.** Vanilla keeps the creative cursor on the client and only sends slot sets. Here the cursor already lives in the server's inventory, so the creative cursor goes through the same `set_slot` checks (`CreativeSlot::Carried`).
- **No drop throttle.** Vanilla's throttle guards against a modified client; here a throw needs a key press or a click.
- **Variants** (potions, enchanted books, goat horns and the rest) show their plain item once for now. Expanding them needs registry data and is its own step.
- **The pad cannot pick tabs yet**: tabs and the scroller take the mouse. The pad still moves between slots and clicks them.
- **The middle button works in the creative screen only**, not yet in chests.

## Minecraft creative inventory searches items like vanilla

Commit: the commit that adds this entry. The previous commit is 20ebfb8 (the creative tabs).

### What changed

Checked with javap against 26.3 `CreativeModeInventoryScreen.refreshSearchResults`, `updateVisibleTags`, `getTooltipFromContainerItem`, `FullTextSearchTree`, `IdSearchTree` and `SuffixArray`.

- **Search rules** (new `crates/minecraft_terrain/src/creative_search.rs`, pure):
  - An empty box shows the whole search tab.
  - Text without `:` matches items whose tooltip text contains it, ignoring case (26.3's `FullTextSearchTree.searchPlainText` only searches the text, not the id).
  - With `:`, the part before must be in the item's namespace, and the part after in its id path or its text (`searchIdentifier`, both parts trimmed).
  - A leading `#` searches the item tags instead (`IdSearchTree` over each item's tags): without `:` the tag path must contain the text, with `:` some tag's namespace and some tag's path must contain the two parts.
  - `visible_tags` picks the tags a `#` query names, as `updateVisibleTags` does.
  - Results keep the search tab's order.
- **Client side** (`render_anim/src/minecraft_creative.rs`):
  - The search tab's stacks and their text lines (the item's name from the language) are built once and kept; each change of the box filters them.
  - Each item's tags come from the item tags the commands already load (`CommandData.item_tags`), turned around once into item to tags.
  - For a `#` query, each shown item gets the shown tags it has (`McCreative.tags`).
  - `McCreative.tabs_of` lists, for each item id, the titles of the shown category tabs that hold it, in tab order.
- **HUD**: a stack's tooltip in the creative screen gains vanilla's extra lines after the name: the blue names of the tabs that hold it (everywhere except the grid of a category tab), then, on the search tab's grid, the purple `#tag` lines for a `#` query. The box itself came with the previous commit; every edit scrolls the grid back to the top.

### Tests added

- `minecraft_terrain` `creative_search::tests` (4): plain text over tooltip lines ignoring case, and that the id alone does not match; `namespace:path` against the id, the text and the namespace; `#` tag searches with and without a namespace; the shown tags for a `#` query.
- `render_anim` `minecraft_creative::tests` (2 more): the tab titles each item lists (the operator tab only when shown); the search list filtering by name, by tag and empty.
- `hud` `minecraft_creative::tests::tooltips_name_the_tabs_and_shown_tags_outside_the_item_tabs`.

### In-game test steps

Kit: none needed.

1. `/gamemode creative`, E, then click the Search tab (compass, top right). Type `wool`: only the 16 wools show. Screenshot 4.
2. Clear the box and type `#logs`: every log and wood shows. Hover an oak log: under its name the blue "Building Blocks" and "Natural Blocks" lines, then a purple `#minecraft:logs` line (and the other log tags). Screenshot 5.
3. Type `minecraft:dia`: the diamond items and blocks show. Type `#minecraft:swo`: the swords show.
4. Type more than 50 characters: the box stops at 50. Select text with Shift+arrows and type over it.
5. Hover a stack in the bottom hotbar row on Building Blocks: its tooltip lists the tabs that hold it; a stack in the Building Blocks grid shows only its own lines.

### Decisions

- **The searched text is the item's name.** Vanilla searches every tooltip line; until variants are expanded every searched stack is a plain item, whose tooltip is just its name.
- **Tags are shown in their sorted order.** Vanilla inserts them from a hash set, so its order is arbitrary.

## Minecraft creative players save and load hotbars like vanilla

Commit: the commit that adds this entry. The previous commit is 1cbbb6d (the search).

### What changed

Checked with javap against 26.3 `HotbarManager`, `Hotbar`, `CreativeModeInventoryScreen.handleHotbarLoadOrSave` and `selectTab` (the hotbar tab), and `Hud.setOverlayMessage` and `extractOverlayMessage`.

- **The file** (new `crates/minecraft_terrain/src/hotbars.rs`, pure): `Hotbars` holds nine hotbars of nine stacks (`HotbarManager.NUM_HOTBAR_GROUPS`, `Inventory.getSelectionSize`).
  - `hotbar.nbt` is vanilla's layout: uncompressed NBT (`NbtIo.write`) with an unnamed root compound holding `DataVersion` and the keys `"0"` to `"8"`, each a list of nine item compounds. An empty slot is `{}` (`ItemStack.OPTIONAL_CODEC`); a stack is `{id, count, components}`, with `components` left out when there are none.
  - Reading tolerates a missing or broken file (empty hotbars, as vanilla logs "Failed to load creative mode options") and a row that is not nine long (empty).
  - `server_mobs::json_tag` is now visible inside the crate so the components turn into NBT the same way the world save does.
- **Client side** (`render_anim/src/minecraft_creative.rs`):
  - The file is per client, `iw4l-artifacts\hotbar.nbt` next to the game (vanilla keeps it in the game folder, not in the world). It is read the first time a hotbar is needed.
  - **Saving** (C held, then 1 to 9): the hotbar's nine stacks are stored in that row and the file is written at once. The MW2 guns are left out, as they are not Minecraft items. The overlay message `inventory.hotbarSaved` ("Saved hotbar (restore with X+1)") shows.
  - **Loading** (X held, then 1 to 9): each of the nine stacks goes through the server's `set_slot` on menu slots 36 to 44, so the server checks every one (creative only, known item, stack size). An empty saved row empties the hotbar, as in vanilla.
  - **The Saved Hotbars tab** shows the nine rows. An empty row shows a paper named `inventory.hotbarInfo` ("Save hotbar with C+n") in its own column; it cannot be taken (`CREATIVE_SLOT_LOCK`). The tab refreshes after each save.
  - A click on any grid slot past the last item counts as an empty grid slot, as in vanilla (it deletes the cursor stack).
- **HUD**: in creative, with no screen open, a number key while C or X is held saves or loads instead of selecting a slot (X wins when both are held, as vanilla checks loading first). The overlay message shows 72 px above the bottom, centred, white with a shadow, for 60 ticks, fading over the last 20.

### Tests added

- `minecraft_terrain` `hotbars::tests` (3): empty hotbars and storing nine stacks; the file layout (`DataVersion`, nine rows of nine, `{}` for empty, the count, components only when present); a round trip through the disk (an unnamed root compound) and rows that are too short or have no count.
- `render_anim` `minecraft_creative::tests` (2 more): the two hotbar texts with their keys; the Saved Hotbars tab with a locked, named paper for each empty row and the stored stacks in their row, a click on the paper doing nothing and a shift-click on a stored stack taking it.
- `hud` `minecraft_creative::tests::the_saved_message_shows_three_seconds_and_fades_over_the_last_second`.

### In-game test steps

Kit:

```
IW4L_MINECRAFT_KIT=torch*64,stone*64,diamond_pickaxe
```

`minecraft:torch` 64, `minecraft:stone` 64, `minecraft:diamond_pickaxe` 1.

1. `/gamemode creative`. With no screen open, hold C and press 1: "Saved hotbar (restore with X+1)" shows above the hotbar and fades out after three seconds. Screenshot 6. (C also crouches in MW2.)
2. Press E and open Saved Hotbars (the bookshelf, top right): row 1 holds your hotbar; rows 2 to 9 each show a paper "Save hotbar with C+2" and so on, in their own column. Clicking a paper does nothing. Shift-click the torch in row 1: 64 torches on the cursor. Screenshot 7.
3. Close the screen, empty the hotbar with `/clear`, then hold X and press 1: the torches, stone and pickaxe come back in their slots.
4. Hold X and press 2: the hotbar is emptied (row 2 is empty).
5. Quit and start the game again: row 1 is still saved (`iw4l-artifacts\hotbar.nbt`).
6. `/gamemode survival`, hold C and press 3: slot 3 is selected and nothing is saved.

### Decisions

- **C and X are fixed keys**, like the minimap's M: they are vanilla's defaults and there is no Minecraft key binding screen for them yet. C is also MW2's crouch, so the player crouches while saving.
- **Guns are not saved.** A gun is the player's MW2 weapon, not a Minecraft item, and the server would refuse to load it.
- **No data fixing.** Vanilla runs a file from an older version through DataFixerUpper; the file is read as 26.3's layout.

## Minecraft creative tabs hold every potion, enchanted book and variant like vanilla

Commit: the commit that adds this entry. The previous commit is c87d3e5 (the saved hotbars).

### What changed

Checked with javap against 26.3 `Potions`, `PotionIds`, `TestBlockMode` and the `DataComponents` names, and against the jar's `data/minecraft/enchantment`, `painting_variant`, `tags/instrument` and `tags/painting_variant` files.

- **Variant data** (new `crates/minecraft_terrain/src/creative_variants.rs`), read once from the owner's 26.3 jar on the creative tabs' thread. No Mojang data is committed.
  - `potion_ids`: the potion registry in registration order. In 26.3 `Potions.<clinit>` registers each potion under a `PotionIds` key, so the order comes from its `getstatic PotionIds.X` instructions and each id from the string `PotionIds.<clinit>` stores in that field (46 potions, water first).
  - Enchantments: `data/minecraft/enchantment/*.json` in id order (a data-driven registry loads its files sorted) with their `max_level`.
  - Goat horn instruments from `tags/instrument`, painting variants (width and height) from `painting_variant` and their tags, with nested `#tag` entries expanded.
  - `enum_names`: `TestBlockMode`'s constants (start, log, fail, accept).
- `VariantData::expand(variants)` is each `CreativeModeTabs` generator:
  - `Potions`: every potion as `potion_contents` (potions, splash and lingering potions, tipped arrows).
  - `EnchantedBooks`: `stored_enchantments` at the maximum level for the tab, every level from 1 to the maximum for the search tab.
  - `Instruments { tag }`: each goat horn as `instrument`.
  - `OminousBottles`: `ominous_bottle_amplifier` 0 to 4. `Fireworks`: `fireworks` with `flight_duration` 1 to 3.
  - `Paintings { tag, in_tag }`: the variants in (or outside) the tag, sorted by area and then width (`PAINTING_COMPARATOR`, stable over registry order), as `painting/variant`.
  - `LightLevels`: the light block's `block_state` level 15 down to 0. `TestBlockModes`: the test block's `block_state` mode.
  - Suspicious stews and the ominous banner are not expanded: they keep showing their plain item once.
- **Client side**: tab and search contents expand every variant entry into its stacks (each item and component set once). The search text of a stack now also holds its potion name, its enchantments (with the level unless the enchantment has a single level), a goat horn's instrument, a painting's title and author, and a rocket's flight duration.

### Tests added

- `minecraft_terrain` `creative_variants::tests` (4): potions, books (maximum and all levels) and horns through nested tags; bottles, rockets, light levels, test block modes and the painting order in and outside the tag, with stews and the banner left alone; nested tags expanding once; `the_real_variants_match_vanilla` against the 26.3 jar (46 potions with water first, Sharpness V, enchantments in id order, eight goat horns, the four test block modes, Alban first among the placeable paintings), skipped without the jar.
- `render_anim` `minecraft_creative::tests::expanded_variants_replace_their_entry_and_unexpanded_ones_stay_plain`: expanded potions and books in order, the stew and banner plain once, and the search lines of a potion and of books with and without a level.

### In-game test steps

Kit: none needed.

1. `/gamemode creative`, E, Food & Drinks (golden apple): the potions, splash potions and lingering potions follow the foods, one per potion, each with its own name ("Potion of Swiftness" and so on). Screenshot 8. All potions share one icon colour for now.
2. Ingredients (iron ingot): the enchanted books at their highest level, glinting, each tooltip naming its enchantment ("Sharpness V").
3. Search tab, type `sharpness`: Sharpness I to V show. Type `swiftness`: the swiftness potions, splash, lingering and tipped arrows show.
4. Tools & Utilities (diamond pickaxe): the eight goat horns and three firework rockets show. Search `sing`: the Sing goat horn shows.
5. Functional Blocks (oak sign): the paintings after the plain painting, small ones first. Search `kebab`: the Kebab painting shows.
6. Take a Potion of Swiftness and a Sharpness V book: both keep their effect and enchantment in the hotbar (hover them).

### Decisions

- **The registry order is read, not written down**: the potion order from the bytecode, and data-driven registries (enchantments, paintings) in id order as 26.3 loads them.
- **Suspicious stews and the ominous banner stay plain.** The stews' effects are code in each flower's constructor and the banner's patterns are code in `Raid`; reading them needs more of the bytecode interpreter than the tabs reader has. The plain items still show once.
- **Variant icons share their item's icon** (potion colours, painting pictures and light levels look alike), as the icon cache is per item id.

## Minecraft creative inventory code passes clippy

Commit: the commit that adds this entry. The previous commit is d1f0062 (the variants).

### What changed

- `clippy -p frame -p hud -p minecraft_survival -p render_anim -p minecraft_terrain -p console --all-targets -- -A clippy::approx_constant` found four warnings in the group's code:
  - three uses of the workspace's disallowed `unwrap_or_default` (`creative_variants.rs` and twice in `render_anim/src/minecraft_creative.rs`), now `unwrap_or_else(Vec::new)` or `unwrap_or_else(String::new)`;
  - the HUD system's parameter tuple that the clipboard made too complex, now the `Extras` type next to the existing `Typing` one.
- The warnings left are in code the group did not touch (`render_anim/src/minecraft_inventory.rs` 587 and `console/src/user_settings.rs` 58 and 237).

### Tests added

None. `test -p minecraft_terrain -p minecraft_survival -p console -p frame -p hud --locked` and `test -p render_anim --locked` pass.

### In-game test steps

Nothing changes in game. Kit: none needed.

## Testing the creative group

### Commits, oldest first

| Commit | Subject |
| --- | --- |
| 20ebfb8 | Minecraft creative inventory has the vanilla tabs |
| 1cbbb6d | Minecraft creative inventory searches items like vanilla |
| c87d3e5 | Minecraft creative players save and load hotbars like vanilla |
| d1f0062 | Minecraft creative tabs hold every potion, enchanted book and variant like vanilla |
| (this one) | Minecraft creative inventory code passes clippy |

### One kit for the whole group

```
IW4L_MINECRAFT_KIT=stone*64,diamond_sword,iron_helmet,torch*64,diamond_pickaxe
```

`minecraft:stone` 64, `minecraft:diamond_sword` 1, `minecraft:iron_helmet` 1, `minecraft:torch` 64, `minecraft:diamond_pickaxe` 1.

### Order

Load a fresh world with this kit and type `/gamemode creative`, then follow the entries' own steps in this order:

1. The tabs entry, steps 1 to 9 (screenshots 1 to 3). Step 7's shift-click on the trash empties the inventory, so run `/kit` afterwards to get the kit back.
2. The search entry, steps 1 to 5 (screenshots 4 and 5).
3. The variants entry, steps 1 to 6 (screenshot 8).
4. The saved hotbars entry, steps 1 to 6 (screenshots 6 and 7); its kit items are in this kit.
5. Last, the operator tab (tabs entry, step 10) if wanted: it needs `minecraft_operator_items=true` in `iw4l-artifacts\settings.cfg` and a restart.

# Second half: review fixes

Branch `track-c`, adversarial review of the whole track (base 44759b7). The previous commit is 555f750 (the creative group's clippy pass).

## Minecraft command answers reach the player who sent the command

Commit: the commit that adds this entry.

### What changed

- **The defect:** `commands::run` is called once per player (`from`), but it emptied every level's `command_jobs` and answered them for `from` only. Each answer is a reply to the job's own `source`, so once a second player runs commands, the first `run` of a frame would take the friend's finished `/setblock`, `/fill` and `/locate` jobs, filter the friend's own answer out (it does not reach `from`) and drop it for good.
- New `commands::take_jobs_of(jobs, source)`: takes the finished jobs that `source` started and leaves the others in the level for their own player's `run`. A result that is not a command job is taken by whoever comes first, as before. `run` uses it for every level.

### Tests added

- `commands::tests::each_player_takes_only_the_answers_to_their_own_commands`: Steve's two edits and Alex's edit and `/locate` sit in one queue. Steve's `run` takes his two and leaves Alex's, Alex's takes the rest, each gets their own lines, and a foreign result is taken.

### In-game test steps

Nothing changes for one player; this matters once friends join. Kit: none needed.

1. `/setblock ~ ~-1 ~ gold_block`: "Changed the block at x, y, z" still shows.
2. `/locate structure minecraft:village_plains`: the answer still shows.

## Minecraft creative screen turns back into the inventory when creative ends

Commit: the commit that adds this entry. The previous commit is 896e289 (command answers per player).

### What changed

- **The defect:** the creative screen only ever switched one way. `CreativeUi::refresh` turns the open survival inventory into the creative screen when the player becomes creative (vanilla `InventoryScreen.containerTick`), but nothing turned the creative screen back when the player stopped being creative. A player whose mode another op changes while the screen is open would keep the creative grid, with every grid click refused by the server.
- `CreativeUi::apply` (`render_anim/src/minecraft_creative.rs`): after the frame's creative clicks, a player without infinite materials gets the plain inventory screen, as 26.3 `CreativeModeInventoryScreen.containerTick` opens an `InventoryScreen` when `hasInfiniteMaterials()` is false (checked with javap). The clicks that are not creative ones stay for the inventory's own handling.

### Tests added

- `render_anim` `minecraft_creative::tests::losing_creative_turns_the_creative_screen_into_the_inventory`: the screen stays while creative, becomes the inventory once creative ends, and the other click is kept.

### In-game test steps

One player cannot change game mode with a screen open (the chat cannot open over it), so this shows only once a friend with op can run `/gamemode` on you. Kit: none needed.

1. In creative, press E: the creative screen opens as before. Screenshot not needed.
2. Close it, `/gamemode survival`, press E: the survival inventory, as before.

## Minecraft creative search tab swaps on number keys only where vanilla does

Commit: the commit that adds this entry. The previous commit is c4a1f8e (the creative screen after creative ends).

### What changed

- **The defect:** on the Search tab a number key swapped only over a grid slot holding a stack, and did so even with a stack on the cursor. 26.3 `CreativeModeInventoryScreen.keyPressed` (checked with javap) tries the hotbar swap when the hovered slot is not a grid slot or holds an item, and `AbstractContainerScreen.checkHotbarKeyPressed` only swaps with an empty cursor and a hovered slot; otherwise the digit is typed into the search box. So over a hotbar slot vanilla swaps (we typed the digit), and with a stack on the cursor vanilla types the digit (we put a full stack in the hotbar).
- New `hud` `minecraft_creative::search_digit_swaps(hovered, has_stack, carrying)` holds that rule; `minecraft_inventory.rs` uses it for the Search tab. A swap over a hotbar slot goes through the inventory's own number swap, as on the other tabs.

### Tests added

- `hud` `minecraft_creative::tests::a_number_key_on_the_search_tab_swaps_only_where_vanilla_does`: a grid stack and a hotbar slot (empty or not) swap, an empty grid slot and no slot type the digit, and nothing swaps with a stack on the cursor.

### In-game test steps

```
IW4L_MINECRAFT_KIT=torch*64,stone*64
```

`minecraft:torch` 64, `minecraft:stone` 64.

1. `/gamemode creative`, E, Search tab. Hover a grid stack and press 3: a full stack lands in hotbar slot 3 and no `3` is typed.
2. Hover the torches in the bottom hotbar row and press 5: they swap with hotbar slot 5, and no `5` is typed. Screenshot.
3. Pick up the stone onto the cursor, hover a grid stack and press 4: `4` is typed into the box and the hotbar does not change.
4. With the mouse over no slot, press 7: `7` is typed.

## Minecraft creative search shows tag lines only for the case vanilla matches

Commit: the commit that adds this entry. The previous commit is b064dc0 (number keys on the Search tab).

### What changed

- **The defect:** `creative_search::visible_tags` lowercased the `#` query before picking the purple tag lines. 26.3 `CreativeModeInventoryScreen.updateVisibleTags` (checked with javap, including its three lambdas) compares the text after `#` exactly as typed with `Identifier.getNamespace().contains` and `getPath().contains`; only the item search itself (`refreshSearchResults`) lowercases. So `#LOGS` finds the logs in vanilla but shows no `#minecraft:logs` line under them, and we showed one.
- `visible_tags` now keeps the case; `search` still lowercases.

### Tests added

- `minecraft_terrain` `creative_search::tests::the_shown_tags_follow_the_hash_query`: `#LOG` shows no tag line while `#LOGS` still finds the oak log.

### In-game test steps

Kit: none needed.

1. `/gamemode creative`, E, Search tab, type `#LOGS`: the logs show; hovering an oak log shows the blue tab names but no purple `#minecraft:logs` line.
2. Change it to `#logs`: the purple line is back. Screenshot.
