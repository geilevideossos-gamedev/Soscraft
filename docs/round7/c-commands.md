# Track C: commands, tab completion and the creative inventory

Research brief, 2026-10-09, base 29f722d. `/give`, `/tp`, `/effect` and `/summon` landed in 3c2c62b. After the crate split, `Host` and its `CommandWorld` impl (now in `render_anim/src/minecraft_chat.rs`) move to `crates/minecraft_survival`, and messages move to `crates/minecraft_net`.

## 1. What exists today

- **Parser** (`crates/minecraft_commands`)
  - `tree.rs`: `Node` (14) has no redirects. `parse` and `parse_nodes` (97, 124) backtrack like brigadier; `runnable` is at 184 and `smart_usage` at 203.
  - `args.rs`: `ArgKind` (17) and `ArgKind::parse` (79). `Resource` doesn't name its registry.
  - `commands.rs`: `build()` (88). `teleport_node` (32) duplicates `tp` instead of redirecting it.
  - `server.rs`: `CommandRegistries` (49), the `CommandWorld` trait (60), `CommandServer` (102) with one `Profile` (offline UUID) per player, `Ctx` (129) and `perform` (389).
  - `protocol.rs:40`: `Clientbound::Commands { level }` exists but nothing uses it yet.
- **Glue** (`render_anim/src/minecraft_chat.rs`)
  - `Host` (14), `registries()` (30), `give_stacks` (71), `summon_answers` (102), the `CommandWorld` impl (124) and `pump` (301).
  - `minecraft_world.rs:289-290` creates the server, and 572-587 builds `Host` and calls `pump`.
  - Every method answers only for `HOST`.
- **Chat UI**
  - `frame/src/minecraft_chat.rs:16`: `McChat` has no suggestion state.
  - `hud/src/minecraft_chat.rs`: `apply_key` (134) maps Up and Down to history and doesn't handle Tab. `draw` (405) draws the input box at line 456. `hud` already depends on `minecraft_commands`.
- **Kit**
  - `minecraft_entities.rs:213` reads `IW4L_MINECRAFT_KIT` once, on load.
  - `minecraft_terrain/src/crafting.rs`: `kit` (402) and `give_kit` (423, which fills empty backpack slots only).
  - `asset_transport::load_dotenv` (`discover.rs:21`) doesn't override variables already set.
- **Creative inventory:** there is none. `McScreen` (`frame/src/minecraft_ui.rs:63`) and `McClick` (100) have no creative variants.
- **Server thread** (`minecraft_terrain/src/server.rs`)
  - `ServerSim::new` (196) holds an `Arc<WorldGen>`.
  - `Command` (1087) is the request enum, and `Output.changes` (1167) syncs blocks back to the client.
  - `command_summons` is the only async command answer.
- **Worldgen pieces for /locate**
  - `placement.rs`: `potential_chunk` (186), `ring_chunks` (200) and `locate_pos` (208).
  - `structure/mod.rs:403`: `Structures::starts_in` makes starts without loading the chunk.
  - `terrain.rs:522`: `biome_at_quart`.
  - Also useful: `core/block.rs:726` `parse_state`, and `entities/enchant.rs` (`max_level`, `can_enchant` at 43, `compatible` at 166).
- **Stash** (`C23 partial`, 12 lines): `Parsed.ranges`, `CommandServer::player_names()`, and public `Node::is_literal` and `allowed_in_id`. It applies cleanly.

## 2. Vanilla 26.3, checked with javap and en_us.json

### Tab completion
- **`ChatScreen`** builds `CommandSuggestions(.., lineStartOffset=1, suggestionLineLimit=10, anchorToBottom=true, fill=0xD0000000)`.
- **Matching:** suggestions show automatically. A candidate matches if it starts with the typed text, or if the text after any `.`, `_` or `/` in it does (`SharedSuggestionProvider.MATCH_SPLITTER`).
- **List:** at most 10 lines of 12 px, drawn above the input at the x of the current word. The unused rest of the selected suggestion shows grey inline (`EditBox.setSuggestion`).
- **Keys** (`SuggestionsList.keyPressed`): Up and Down move the selection. Tab and Shift+Tab call `cycle` and write the suggestion into the input. Esc hides the list. Tab with the list hidden calls `showSuggestions`.
- **Usage and errors:** with no suggestions, grey usage lines show (`USAGE_OFFSET_FROM_BOTTOM = 27`); a parse error shows red as `command.context.parse_error`.
- **Input colours** (`formatText`): arguments cycle aqua, yellow, green, light purple and gold. The rest is grey, and anything past an error is red.

### Creative inventory
**Tabs** (`CreativeModeTabs.bootstrap`):
- Top row:
  - 0-4: `itemGroup.buildingBlocks`, `coloredBlocks`, `natural`, `functional`, `redstone`.
  - 5: `hotbar` (Saved Hotbars), aligned right.
  - 6: `search`, aligned right.
- Bottom row:
  - 0-4: `tools`, `combat`, `foodAndDrink`, `ingredients`, `spawnEggs`.
  - 5: `op`, aligned right.
  - 6: `inventory`, aligned right, with no title and no scroll bar.

**Contents** are Java lambdas, not data:
- About 1,307 `Output.accept(Items.X)` calls.
- 21 `registerColoredItems(ColorCollection)` calls, in the 16-colour order built in `bootstrap`.
- Copper families through `weathering()` and `waxed()` with `forEach`.
- Generators: potions, enchanted books, goat horns, paintings, fireworks, stews and ominous bottles.

The jar isn't obfuscated. 1,191 of the 1,194 `Items` field names, lowercased, are item ids; the exceptions are `CUT_STANDSTONE_SLAB`, `DRY_SHORT_GRASS` and `DRY_TALL_GRASS`.

**Screen** (`CreativeModeInventoryScreen`):
- 9x5 grid, 26x32 tabs, 12x15 scroller.
- The search box is at (82, 6), 80 wide, with at most 50 characters. A query starting with `#` searches item tags.
- The Operator Utilities tab needs `canUseGameMasterBlocks()` and the "Operator Items Tab" option.

### Commands
All of these need level 2.

- **`/locate`**: `structure <id|#tag>`, `biome <id|#tag>` or `poi <id|#tag>`.
  - The structure search radius is 100 (`findNearestMapStructure`). Biomes use `findClosestBiome3d` with radius 6400, step 32 across and step 64 up.
  - It answers "The nearest %s is at %s (%s blocks away)" with green bracketed coordinates. A structure shows y as `~` and a horizontal distance; a biome shows the real y and a 3D distance.
  - A tag prints as `#minecraft:village (minecraft:village_plains)`.
- **`/execute ... in <dimension> ... run <command>`**: `withLevel` scales x and z by `getTeleportationScale` (1/8 in the Nether). A dimension that isn't loaded gives `argument.dimension.invalid`.
- **`/clear [<targets>] [<item>|#tag|*] [<maxCount>]`**: 0 only counts. It also clears the carried stack and the crafting grid. Messages: `commands.clear.success|test.*` and `clear.failed.*`.
- **`/experience`** (`/xp` redirects to it): `add|set <targets> <amount> [points|levels]`, `query <target> points|levels`.
  - Setting points at or above `getXpNeededForNextLevel()` fails with `set.points.invalid`.
  - A points query answers `floor(progress*needed)`.
- **`/enchant <targets> <enchantment> [<level>]`**: checks `failed.level`, then `failed.itemless`, then `failed.incompatible`. Success reads "Applied enchantment Sharpness V to %s's item".
- **`/setblock <pos> <block> [destroy|keep|replace|strict]`** and **`/fill <from> <to> <block> [destroy|hollow|keep|outline|replace [<filter>]|strict]`**
  - Both use `getLoadedBlockPos` (errors `argument.pos.unloaded` and `outofworld`).
  - The fill limit is the `max_block_modifications` game rule (`commands.fill.toobig`).
- **`/spawnpoint [<targets>] [<pos>] [<rotation>]`**
  - Saved as `RespawnConfig(forced=true)` in the player file's `respawn` key.
  - Vanilla passes seven arguments to `success.single`, whose text has six `%s`. We pass the same seven.

## 3. Commits

1. **"Minecraft commands suggest completions like vanilla"**
   - Pop the stash.
   - Add `minecraft_commands/src/suggest.rs`: `complete(input, cursor, level, &CommandHints) -> Completion { start, list, usage, error }`. It returns literal children plus `ArgKind::suggest` (selectors and names, enums, ids, `~`).
   - `ArgKind::Resource` becomes `Resource(Registry)`.
2. **"Minecraft chat shows command suggestions and usage like vanilla"**
   - Add `McChat.hints` and `McChat.suggestions`.
   - Update `apply_key` and `draw`.
   - `render_anim` fills the hints every frame. In multiplayer they arrive in `minecraft_net` messages, after `Clientbound::Commands`.
3. **"Minecraft commands can redirect like vanilla"**: add `Node.redirect`; `tp` becomes a redirect, so `/help` shows `/tp -> teleport`.
4. **"Minecraft /kit gives the .env kit again"** (level 2): `asset_transport::dotenv_value` reads `.env` again, and each entry goes through `CommandWorld::give`, so whatever doesn't fit drops at the feet.
5. **"Minecraft /clear removes items like vanilla"**: add `CommandWorld::clear` and load item tags.
6. **"Minecraft /xp and /experience add, set and query experience like vanilla"**: add `experience_*` methods to the trait and to `survival::Status`.
7. **"Minecraft /enchant enchants the held item like vanilla"**: add `CommandWorld::enchant` on top of `enchant.rs`.
8. **"Minecraft /setblock places blocks like vanilla"**
   - Add `ArgKind::BlockState` and `ArgKind::BlockPos`.
   - Add a server `Command::SetBlocks { source, edits, mode }` with the vanilla flags.
   - Add `Output.command_answers: Vec<(PlayerId, Result<i32, String>)>`, which replaces `command_summons`.
9. **"Minecraft /fill fills areas like vanilla"**: all the modes and the limit, sent through `SetBlocks`.
10. **"Minecraft /spawnpoint sets each player's respawn point like vanilla"**: a per-player `respawn` point that the respawn code reads before the world spawn.
11. **"Minecraft /locate finds structures and biomes like vanilla"**: `Command::Locate` runs on the server thread with a time cap. `poi` is left out until there is a POI manager.
12. **"Minecraft /execute in parses dimensions like vanilla"**
    - Adds `Ctx.dimension` and the x/z scaling.
    - Until Phase 3, `the_nether` and `the_end` answer `argument.dimension.invalid`, as vanilla does for a level that isn't loaded.
    - Phase 3 adds a dimension to `CommandWorld::teleport`.
13. **"Minecraft creative tabs are read from the game jar"**
    - Add `minecraft_terrain/src/class_file.rs` (constant pool, `Code`, `BootstrapMethods`) and `creative_tabs.rs`.
    - It walks `bootstrap` to find each tab, then interprets each lambda.
    - Colour, copper and generator calls expand through the item list and the data the game already reads (`enchantment/`, `painting_variant/`, `instrument/`, the potion language keys).
    - Field names map to ids by lowercasing, with a fallback to the `ldc` before `putstatic` in `Items.<clinit>` and `Blocks.<clinit>`.
    - Anything it doesn't know is logged and skipped. No Mojang data is committed.
14. **"Minecraft creative inventory has the vanilla tabs"**
    - Add `McScreen::Creative { tab, scroll }` and `McClick::CreativeSet { slot, stack }`.
    - The server accepts a creative set only from a creative player, and caps the stack at the item's maximum stack size.
    - Operator Utilities shows only at level 2 or higher with the new option.
15. **"Minecraft creative inventory searches items like vanilla"**: name search, `#tag` search, at most 50 characters.
16. **"Minecraft creative players save and load hotbars like vanilla"**: C plus a number saves a hotbar and X plus a number loads one, from a per-client `hotbar.nbt`.

## 4. Unit tests

- **Completion:**
  - Root literals are filtered by level.
  - `/gamemode c` completes to `creative`, and `/give @s pick` suggests `minecraft:diamond_pickaxe`.
  - The suggestion's start is the start of the argument.
  - Usage reads `<targets>`, and text past an error gives the red error.
- **Chat:**
  - Tab cycles forward and Shift+Tab back.
  - Esc hides the list, then closes the chat.
  - Up and Down move through history only while the list is closed.
- **Redirects:** `tp -> teleport` in `/help`, `/xp query @s levels`, and `execute in minecraft:overworld run seed`.
- **One `FakeWorld` test per command:**
  - `/clear` with maxCount 0 counts without removing.
  - Setting too many points fails.
  - An enchantment above its maximum level fails.
  - A `/fill` over the limit gives `toobig`.
  - The `/spawnpoint` message gets seven arguments.
  - `/locate` prints `~` for structures.
- **`/kit`:** a changed `.env` is read again.
- **`creative_tabs`** (against the real jar, skipped without it):
  - 14 tabs at the vanilla rows and columns.
  - Building Blocks starts with `oak_log`, and Colored Blocks with `white_wool`.
  - Every token resolves to a known item.
- **Locate:** a seeded `WorldGen` finds `village_plains` and `desert` at positions checked against vanilla.

## 5. In-game test steps

```
IW4L_MINECRAFT_KIT=diamond_pickaxe+efficiency=5,diamond_sword,stone*64,oak_log*64,torch*64,bread*16
```

1. Type `/ga`: the list shows `gamemode` and `gamerule`, Tab cycles them, and grey text completes the selected one. Screenshot.
2. Type `/give @s pick`: the pickaxes show. Esc hides the list, and a second Esc closes the chat.
3. `/xp add @s 5 levels`, then `/xp query @s levels`: "Steve has 5 experience levels".
4. Hold the sword. `/enchant @s sharpness 5` succeeds, and `/enchant @s sharpness 6` gives the red max-level error.
5. `/clear @s minecraft:stone 0` says "Found 64 matching item(s)". Then `/clear @s minecraft:stone` removes the stone.
6. `/kit`: the kit comes back, and anything that didn't fit lies at your feet.
7. `/setblock ~ ~-1 ~ gold_block` and `/fill ~1 ~ ~1 ~5 ~4 ~5 glass hollow` build a glass box. `/fill ~ ~ ~ ~100 ~100 ~100 stone` gives the red "Too many blocks" error.
8. `/spawnpoint`, walk 50 blocks away and `/kill`: you respawn at the point you set.
9. `/locate structure minecraft:village_plains`, then `/tp` to the position: there is a village. `/locate biome minecraft:cherry_grove` works the same. Screenshot.
10. `/execute in minecraft:the_nether run tp ~ ~ ~` gives "Unknown dimension" until Phase 3.
11. `/gamemode creative`, then E: the tabs show. Scroll Colored Blocks, search `wool` and `#logs`, take a stack, and use the trash slot. Screenshots.

## 6. Risks and overlaps

- **Crate split:** `Host` moves, so start after the split.
- **Track A** owns the chat keys in `hud/minecraft_chat.rs`. Merge A first.
- **Track B:** the creative screen depends on item icons, names and stack sizes.
- **Track D:** the `respawn` key goes in the player file.
- **Phase 3:** `/execute in` and `/locate` or `/setblock` in other dimensions need a server that runs several levels at once.
- **Multiplayer:** the host sends the hints, checks every creative set-slot and command again, and sends each async answer to the `PlayerId` that asked.
- **Cost:** `/locate` builds villages through jigsaw generation and can visit thousands of cells, so it must run off the main thread with a cap. A large `/fill` should be split across ticks.
- **Fragility:** the class reader is written for 26.3 only. If it fails, the tabs fall back to registry order and the log says so.
