# Track SPLIT log

Round 7, Phase 1 item 4: the crate split of [../CRATE-SPLIT.md](../CRATE-SPLIT.md), done as pure moves with no behaviour change. Branch `crate-split`, worktree `C:\Dev\mc-wt\split`, based on `70a01b2`.

A commit can't name its own hash, so each entry gives the hash of the commit before it. The hash of the last commit is in the next entry or in the track's return.

## Baseline test totals (at `70a01b2`)

Measured with `cargo-slot.ps1 test -p <crate> --locked` before any change. Every total stays constant across the split: tests move between crates, never disappear.

| Crate | Passed | Ignored |
|---|---|---|
| `render_anim` | 153 | 0 |
| `minecraft_terrain` | 464 | 3 |
| `minecraft_commands` | 53 | 0 |

`render_anim` per module: `minecraft_survival` 64, `minecraft_death` 25, `minecraft_save` 12, `minecraft_offhand` 10, `minecraft_camera` 7, `minecraft_chat` 7, `minecraft_gui` 7, `minecraft_entities` 6, `minecraft_hand` 5, `minecraft_spawn_gate` 5, `minecraft_mining` 2, `minecraft_inventory` 1, `minecraft_sounds` 1, `skate::collision` 1.

The plan's section 3 table was written at `d426239` (149). Recomputed for this base:

| After | `minecraft_survival` | `render_anim` |
|---|---|---|
| base `70a01b2` | - | 153 |
| A2 | 12 | 141 |
| A3 | 37 | 116 |
| A4 | 47 | 106 |
| A6 | 111 | 42 |
| A8 | 117 | 36 |
| A9 | 119 | 34 |

## Regression kit

Every commit of this track is a pure move, so the whole game must play exactly as before. Kit line for the `.env` next to `iw4l-survival.exe` (27 stacks, from the plan's section 7):

```
IW4L_MINECRAFT_KIT=diamond_pickaxe+silk_touch=1,diamond_sword,shield,cooked_beef*32,cobblestone*64,oak_planks*64,crafting_table,furnace,chest*2,ender_chest,coal*16,raw_iron*16,water_bucket,lava_bucket,flint_and_steel,ender_pearl*16,egg*16,zombie_spawn_egg*2,cow_spawn_egg*2,totem_of_undying,shulker_box,anvil,enchanting_table,bookshelf*15,experience_bottle*64,torch*64,glass_bottle*4
```

## A1. Empty minecraft_survival and minecraft_net crates

- **What changed:**
  - New `crates/minecraft_net` (serde only) and `crates/minecraft_survival` (the dependencies of the plan's section 1.2, with `glam = "0.29"`), each with an empty `src/lib.rs` (a single newline, which is what `rustfmt --check` accepts for an empty file).
  - Root `Cargo.toml`: both crates are workspace members and workspace dependencies.
  - `render_anim` depends on `minecraft_survival`.
  - `Cargo.lock` only gains the two path packages and `render_anim`'s new dependency line.
- **Tests added:** none. Totals unchanged (153, 464 + 3 ignored, 53).
- **Checks:** `check -p launcher --offline` (lock update), then `check -p launcher --locked` and `check -p minecraft_net -p minecraft_survival --locked`.
- **Owner test** (nothing visible changes):
  1. Start `iw4l-survival.exe` and load a Minecraft match. The world, the kit, hearts, hunger and XP show as before.
- **Kit:** any; the regression kit above works.
- **Decisions:**
  - `serde_json` in `minecraft_survival` uses the workspace entry (`serde_json = "1"`, as `render_anim` pins it). `diag`, `minecraft_commands`, `minecraft_net` and `playerstate_iw4` use `.workspace = true`; `minecraft_terrain` and the `minecraftoss-*` crates are path dependencies as in `render_anim`.

## A2. Minecraft world saving code moves to minecraft_survival

A1 is commit `5a7a4e4`.

- **What changed:**
  - `git mv crates/render_anim/src/minecraft_save.rs crates/minecraft_survival/src/save.rs` (git reports `R`), with its 28 `pub(crate)` turned into `pub`. No other line of it changed.
  - `minecraft_survival/src/lib.rs` is `pub mod save;`.
  - `render_anim`: `mod minecraft_save;` is gone, and `crate::minecraft_save::` became `minecraft_survival::save::` in `minecraft_world.rs` (7 sites) and `minecraft_chat.rs` (1).
- **Tests:** the 12 `save` tests moved with the file. `minecraft_survival` 12, `render_anim` 141 (was 153), total unchanged.
- **Checks:** `check -p launcher --locked`, `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `lib.rs` passes `rustfmt --check --edition 2024`.
- **Owner test** (saving and loading must work exactly as before):
  1. Set `IW4L_MINECRAFT_WORLD=split` in the `.env` and load a Minecraft match. The log says "Minecraft world: created ..." (or "opened ..." if the folder exists) with the seed.
  2. Place a few cobblestone and oak planks blocks, then type `/gamemode creative`.
  3. Leave the match. The log says "Minecraft world saved".
  4. Load the match again. The same seed and terrain, the placed blocks and creative mode are back.
  5. Stay 5 minutes in the world. The log says "Minecraft autosave: N chunks queued".
- **Kit:** `minecraft:cobblestone` x64 and `minecraft:oak_planks` x64 (both in the regression kit).
- **Decisions:** none. The `save` tests keep their `render_anim-saves-` temp folder prefix, because the move changes no other line.

## A3. Minecraft death rules move to minecraft_survival

A2 is commit `86fa3d4`.

- **What changed:**
  - `git mv crates/render_anim/src/minecraft_death.rs crates/minecraft_survival/src/death.rs` (`R`), with its 30 `pub(crate)` turned into `pub`. No other line of it changed.
  - `minecraft_survival/src/lib.rs` gains `pub mod death;`.
  - `render_anim`: `mod minecraft_death;` is gone, and `crate::minecraft_death::` became `minecraft_survival::death::` in `minecraft_world.rs` (8 sites on 7 lines), `minecraft_entities.rs` (5), `minecraft_survival.rs` (4: the `use` at line 1 and three tests; the plan counted 3 at `d426239`) and `minecraft_chat.rs` (2).
- **Tests:** the 25 `death` tests moved with the file (24 in the plan, plus the void test from `65f6402`). `minecraft_survival` 37, `render_anim` 116, total unchanged.
- **Checks:** `check -p launcher --locked`, `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `lib.rs` passes `rustfmt --check --edition 2024`.
- **Owner test** (deaths, death messages and drops must work exactly as before):
  1. Load a Minecraft match with the regression kit and drink a few experience bottles to get some levels.
  2. Fall from 6 blocks or more: fall damage and the fall sound. Fall again until you die: the message is "... fell from a high place" and the killed-by card names it.
  3. Respawn. Items (apart from guns) and XP lie where you died, and the guns are still in the hotbar.
  4. Put the totem of undying in the offhand and die again: the totem pops and you live.
  5. Spawn a zombie from the egg and let it kill you: "... was slain by Zombie".
  6. Stand in lava until you die: "... tried to swim in lava".
- **Kit:** `minecraft:totem_of_undying` x1, `minecraft:zombie_spawn_egg` x2, `minecraft:lava_bucket` x1, `minecraft:experience_bottle` x64, `minecraft:cobblestone` x64 (all in the regression kit).
- **Decisions:** none.

## A4. Minecraft gun items and offhand rules move to minecraft_survival

A3 is commit `dd6291c`.

- **What changed:**
  - New `minecraft_survival/src/guns.rs` (no comments, rustfmt-clean): `WEAPON_PREFIX`, `weapon_of` and `weapon_item` moved from `render_anim/src/minecraft_inventory.rs`, now `pub`, plus `pub const HOTBAR: usize = 9`, vanilla's selection size and the value of `frame::minecraft_ui::MC_HOTBAR`, which `hud` and `minecraft_inventory.rs` keep using.
  - `minecraft_inventory.rs` loses those lines (and the doc line of `WEAPON_PREFIX`) and imports them with `use minecraft_survival::guns::{WEAPON_PREFIX, weapon_item, weapon_of};`.
  - `git mv crates/render_anim/src/minecraft_offhand.rs crates/minecraft_survival/src/offhand.rs` (`R`): its 22 `pub(crate)` became `pub`, `crate::minecraft_inventory::weapon_of` became `crate::guns::weapon_of` and `frame::minecraft_ui::MC_HOTBAR` became `crate::guns::HOTBAR`. No other line changed, so `minecraft_survival` still has no `frame` (bevy) dependency.
  - `minecraft_survival/src/lib.rs` gains `pub mod guns;` and `pub mod offhand;`.
  - `render_anim`: `mod minecraft_offhand;` is gone; `crate::minecraft_offhand::` became `minecraft_survival::offhand::` in `minecraft_world.rs` (24 sites on 23 lines) and `minecraft_survival.rs` (2); `crate::minecraft_inventory::weapon_of` became `minecraft_survival::guns::weapon_of` in `minecraft_world.rs` (2).
- **Tests:** the 10 `offhand` tests moved with the file; `guns.rs` brings none (the plan lists 0). `minecraft_survival` 47, `render_anim` 106, total unchanged (153).
- **Checks:** `check -p launcher --locked`, `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `guns.rs` and `lib.rs` pass `rustfmt --check --edition 2024`.
- **Owner test** (guns in the inventory, the offhand, the shield and the totem must work exactly as before):
  1. Load a Minecraft match with the regression kit. The MW2 guns sit in the hotbar; select one with the number keys and it is raised.
  2. Open the inventory and move a gun to another hotbar slot; select it there and the same gun is raised.
  3. Put the shield in the offhand (swap-hands key with the shield selected). Swap hands with a gun selected: nothing happens.
  4. Spawn a zombie from the egg, hold the tactical key to raise the shield and face the zombie: its hits are blocked with the shield block sound.
  5. Swap the shield back and put cooked beef in the offhand. With some hunger lost, the tactical key eats it. Eggs or torches in the offhand are thrown or placed the same way.
  6. Select the totem in the hotbar and take lethal damage (a long fall): the totem in the main hand pops with the totem sound and you live.
- **Kit:** `minecraft:shield` x1, `minecraft:totem_of_undying` x1, `minecraft:zombie_spawn_egg` x2, `minecraft:cooked_beef` x32, `minecraft:egg` x16, `minecraft:torch` x64 (all in the regression kit).
- **Decisions:**
  - The `WEAPON_PREFIX` doc line (`/// Items that are MW2 guns ...`) is dropped rather than carried into `guns.rs`, because `guns.rs` is a new file and new files have no comments.
  - The `use` line goes right after `use glam::DVec3;`, where rustfmt's sort order puts it.

## A5. Minecraft vitals are plain data

A4 is commit `96d91f6`.

- **What changed:**
  - `render_anim/src/minecraft_survival.rs` gains `pub(crate) struct Vitals`, with exactly the fields and derives of `frame::McVitals` (`Clone, Debug, Default, PartialEq`), and `Survival::vitals()` builds it instead of `frame::McVitals`. The file no longer names `frame`, so it can move to the bevy-free crate in A6.
  - `minecraft_world.rs` gains `fn mc_vitals(vitals: Vitals) -> frame::McVitals` (after `ui_game_mode`), a field-by-field copy, and the HUD update calls `alive.then(|| mc_vitals(survival.vitals()))`. It is a function, not `impl From`, because both types will live in other crates (orphan rule).
- **Tests:** none added; the 15 test uses of `.vitals().on_fire` and similar compile unchanged. `render_anim` 106, `minecraft_survival` 47, total unchanged (153).
- **Checks:** `check -p launcher --locked`, `test -p render_anim --locked`.
- **Owner test** (the HUD must show the same vitals as before):
  1. Load a Minecraft match with the regression kit. Hearts, the hunger bar and the XP bar show.
  2. Drink a few experience bottles: the XP bar fills and the level number goes up.
  3. Fall a few blocks: the hearts blink and drop. Sprint and jump for a while: the hunger bar drops.
  4. Swim under water: the air bubbles show and run down.
  5. Step into lava briefly: the fire overlay shows and the hearts drop; it goes out in water.
  6. Eat cooked beef: the hunger bar fills.
- **Kit:** `minecraft:experience_bottle` x64, `minecraft:water_bucket` x1, `minecraft:lava_bucket` x1, `minecraft:cooked_beef` x32, `minecraft:cobblestone` x64 (all in the regression kit).
- **Decisions:**
  - `Vitals` sits right after `Motion`, the other plain-data struct the adapter fills or reads, and its fields stay `pub(crate)` until the A6 move turns them `pub` with the rest of the file.

## A6. Minecraft health, hunger and hazards move to minecraft_survival

A5 is commit `493fea3`.

- **What changed:**
  - `git mv crates/render_anim/src/minecraft_survival.rs crates/minecraft_survival/src/survival.rs` (`R`), with its 94 `pub(crate)` turned into `pub`, and its `minecraft_survival::death::` (4) and `minecraft_survival::offhand::` (2) paths turned into `crate::death::` and `crate::offhand::`. No other line of it changed (checked by normalising the rename diff: every removed line has its added twin).
  - `minecraft_survival/src/lib.rs` gains `pub mod survival;`.
  - `render_anim`: `mod minecraft_survival;` is gone, and `crate::minecraft_survival::` became `minecraft_survival::survival::` in `minecraft_world.rs` (16 lines: the plan's 15 plus `mc_vitals` from A5) and `minecraft_chat.rs` (3).
- **Tests:** the 64 `survival` tests moved with the file (61 in the plan at `d426239`, plus the void-damage tests from `65f6402`). `minecraft_survival` 111, `render_anim` 42, total unchanged (153). Matches the recomputed table.
- **Checks:** `check -p launcher --locked`, `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `lib.rs` passes `rustfmt --check --edition 2024`.
- **Owner test** (health, hunger and every hazard must work exactly as before):
  1. Load a Minecraft match with the regression kit. Hearts, hunger and XP show.
  2. Sprint and jump around for a minute: the hunger bar drops. Eat cooked beef: it fills, with the eating sound and the burp.
  3. With full hunger, take a little damage (a 4-block fall): the hearts come back by natural regeneration.
  4. Fall from 6 blocks or more: fall damage and the big fall sound. Fall the same height into water: no damage.
  5. Stay under water until the air bubbles run out: drowning damage.
  6. Step into lava: you catch fire and take damage; jump into water and the fire goes out. Light a fire with flint and steel and stand in it: burning damage.
  7. Hit a cow with the sword: the hunger bar drops faster than when standing still.
  8. Type `/gamerule fall_damage false`, fall 10 blocks: no damage. Then `/gamerule fall_damage true`.
  9. Type `/effect give @s minecraft:poison 5`: the hearts turn green and drop by about two hearts while it lasts (poison never kills).
- **Kit:** `minecraft:cooked_beef` x32, `minecraft:water_bucket` x1, `minecraft:lava_bucket` x1, `minecraft:flint_and_steel` x1, `minecraft:diamond_sword` x1, `minecraft:cow_spawn_egg` x2, `minecraft:cobblestone` x64 (all in the regression kit).
- **Decisions:** none.

## A7. Minecraft entities take the asset root from the caller

A6 is commit `a47014b`.

- **What changed:**
  - `Entities::new(stream, seed)` became `Entities::new(stream, seed, root: Option<&std::path::Path>, jar: Option<std::path::PathBuf>)`. Its `let jar = data_jar();` line is gone (the parameter takes its place), and the item catalog lookup uses `root` instead of `assets::minecraft_map::root().as_deref()`.
  - `data_jar()` moved from `minecraft_entities.rs` into `minecraft_world.rs` (right after `load()`) as `pub(crate) fn data_jar()`, unchanged apart from dropping its doc line and its inline comment.
  - Callers: the load completion in `minecraft_world.rs` passes `assets::minecraft_map::root().as_deref()` and `data_jar()`, and `minecraft_inventory.rs` calls `crate::minecraft_world::data_jar()` for the GUI assets.
  - `minecraft_entities.rs` no longer names `assets`, its last tie to `render_anim`.
- **Tests:** none added. `render_anim` 42, `minecraft_survival` 111, total unchanged (153).
- **Checks:** `check -p launcher --locked`, `test -p render_anim --locked`.
- **Owner test** (loot, recipes, stations and enchantments come from the jar exactly as before):
  1. Load a Minecraft match with the regression kit. The log has "Minecraft recipes: N with the item catalog ...", and no "Minecraft data JAR not found" warning.
  2. Open the inventory: the item icons and the GUI textures show. Place the crafting table and craft sticks from planks: the recipe works and the stack sizes are right (sticks stack to 64).
  3. Place the furnace and smelt raw iron with coal: an iron ingot comes out.
  4. Mine stone with the pickaxe: it drops cobblestone; with silk touch, an ore drops itself.
  5. Kill a cow from the spawn egg: it drops beef and leather (mob loot).
  6. Place the enchanting table with bookshelves around it and open it with some levels: three enchantment offers show.
- **Kit:** `minecraft:oak_planks` x64, `minecraft:crafting_table` x1, `minecraft:furnace` x1, `minecraft:raw_iron` x16, `minecraft:coal` x16, `minecraft:diamond_pickaxe` x1 with silk touch, `minecraft:cow_spawn_egg` x2, `minecraft:enchanting_table` x1, `minecraft:bookshelf` x15, `minecraft:experience_bottle` x64 (all in the regression kit).
- **Decisions:**
  - The `Mob and block loot and the recipes ...` comment above the jar block in `minecraft_entities.rs` stays where it is: the file is an existing file and only the lines the seam needs changed.
  - `data_jar()` goes after `load()` in `minecraft_world.rs`, next to the other asset-root code of the loader.
  - The call to `data_jar()` now runs just before `Entities::new` instead of just after the level server thread spawns. It only reads the file system, so nothing a player can see changes.
  - A6's owner test step 9 was reworded (poison drops about two hearts and never kills); the step itself is unchanged.

## A8. Minecraft mobs and the level server handle move to minecraft_survival

A7 is commit `7e88587`.

- **What changed:**
  - `git mv crates/render_anim/src/minecraft_entities.rs crates/minecraft_survival/src/entities.rs` (`R`), with its 51 `pub(crate)` turned into `pub` and its 5 `minecraft_survival::death::` paths turned into `crate::death::`. No other line of it changed (checked by normalising the rename diff). Its `//!` and `///` comments stay (owner decision 1).
  - `minecraft_survival/src/lib.rs` gains `pub mod entities;`.
  - `render_anim`: `mod minecraft_entities;` is gone, and `crate::minecraft_entities::` became `minecraft_survival::entities::` in `minecraft_world.rs` (14 sites) and `minecraft_chat.rs` (1).
  - `Entities` moves whole (owner decision 3): the client mirrors and mesh code stay in it, since they come from `minecraft_terrain` and use no bevy.
- **Tests:** the 6 `entities` tests moved with the file. `minecraft_survival` 117, `render_anim` 36, total unchanged (153). Matches the recomputed table.
- **Checks:** `check -p launcher --locked` (no new warning in `minecraft_survival` or `render_anim`), `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `lib.rs` passes `rustfmt --check --edition 2024`.
- **Owner test** (mobs, items, XP orbs and the level server must work exactly as before):
  1. Load a Minecraft match with the regression kit. Passive mobs spawn around you; at night, hostile mobs spawn and come for you.
  2. Spawn a cow and a zombie from the eggs. Shoot the cow with a gun and hit the zombie with the sword: both take damage, flash red and die with their drops and XP orbs; the orbs fly to you and the XP bar fills.
  3. Let the zombie hit you: you take damage and the killed-by card names the zombie if it kills you.
  4. Throw an egg and an ender pearl: the egg breaks (sometimes a chick), the pearl teleports you with the portal particles.
  5. Drop an item with the drop key: it lies on the ground, bobs, and you pick it up again by walking over it.
  6. Mine a block: it drops as an item entity. Pour the lava bucket and throw a few planks into it: they burn up.
  7. Type `/weather rain`: rain starts. Type `/summon minecraft:cow`: a cow appears.
- **Kit:** `minecraft:cow_spawn_egg` x2, `minecraft:zombie_spawn_egg` x2, `minecraft:diamond_sword` x1, `minecraft:egg` x16, `minecraft:ender_pearl` x16, `minecraft:oak_planks` x64, `minecraft:lava_bucket` x1, `minecraft:cobblestone` x64 (all in the regression kit).
- **Decisions:** none beyond the owner decisions.

## A9. Minecraft chat commands run from minecraft_survival

A8 is commit `1d8f688`.

- **What changed:**
  - **Seam:** `pump(chat: &mut McChat, ...)` became `pump(sent: &mut Vec<String>, ...)` and takes the lines with `std::mem::take(sent)`. The caller in `minecraft_world.rs` passes `&mut ui.chat.sent`.
  - **New `minecraft_survival/src/commands.rs`** (no comments, formatted with `rustfmt --edition 2024`, the staged content passes `rustfmt --check`): `HOST`, `Host` and its `CommandWorld` impl, `registries`, `effect_kind`, `give_stacks`, `nbt_error` (private), `summon_answers`, `pump` and `death_message`, moved from `render_anim/src/minecraft_chat.rs`, now `pub`, with `minecraft_survival::x::` paths turned into `crate::x::`. Compared with the old lines with whitespace, braces and commas stripped, the code is identical apart from the `pump` seam (rustfmt only added closure braces and trailing commas).
  - `minecraft_survival/src/lib.rs` gains `pub mod commands;`.
  - **`render_anim/src/minecraft_chat.rs` keeps** `show` and the 5 tests that call it. Its top-level imports shrink to what `show` needs (`HashMap`, `McChat`, `McChatLine`, `Clientbound`), and its test module imports the rest, including `HOST`, `Host`, `death_message`, `pump` and `summon_answers` from `minecraft_survival::commands`. Its 4 `pump(&mut chat, ...)` calls became `pump(&mut chat.sent, ...)`.
  - `minecraft_world.rs`: `crate::minecraft_chat::{HOST, registries, Host, pump, summon_answers, death_message}` became `minecraft_survival::commands::` at all 9 sites; the 3 `crate::minecraft_chat::show` calls stay.
- **Tests:** `leftover_given_items_drop_at_the_feet` and `host_effects_reach_survival_and_teleports_wait_for_the_world` moved to `commands.rs`; the second uses a plain `Vec<String>` instead of `McChat::default()`. No test was added or weakened. `minecraft_survival` 119, `render_anim` 34, total unchanged (153). Matches the recomputed table.
- **Checks:** `check -p launcher --locked` (no new warning in `minecraft_survival` or `render_anim`), `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `commands.rs` and `lib.rs` pass `rustfmt --check --edition 2024` with empty output.
- **Owner test** (chat and commands must work exactly as before; this is also the plan's checkpoint after A9, so the full regression from section 7 follows):
  1. Load a Minecraft match in a world folder (`IW4L_MINECRAFT_WORLD=split`). The kit is in the backpack, and hearts, hunger and XP show.
  2. Open the chat, type `hello`: it shows as `<name> hello`. Type `/frob`: the red unknown-command error and the grey `frob<--[HERE]` line show.
  3. Type `/gamemode creative`, `/give @s minecraft:diamond 5`, `/tp ~ ~10 ~`, `/time set 13000`, `/weather rain`, `/summon minecraft:cow` and `/effect give @s minecraft:regeneration 10 1`. Each works and answers in the chat (the diamonds land in the backpack with the pickup sound, you are lifted 10 blocks, night falls, rain starts, a cow appears). Then `/gamemode survival`.
  4. Mine stone with the pickaxe and shoot a block with a gun. Both drop items, and a silk-touch ore drops itself.
  5. Place a crafting table, a furnace and a chest. Craft something, smelt raw iron with coal, and put items in the chest.
  6. Place the ender chest and put an item in it. Break it: the item doesn't drop. Place it again, and the item is still there.
  7. Eat beef. Hold the shield up against a zombie from the spawn egg. Throw a pearl (teleport and fall damage) and an egg.
  8. Fall from 6 blocks or more (fall damage and sound), then swim in water and stand in lava briefly.
  9. Die with the totem in the offhand (it pops). Die again without it: items and XP drop, guns stay, and the chat shows the death message. Respawn and type `/kill`: the chat answers "Killed <name>" and you die.
  10. Leave the match, and check the log for "Minecraft world saved". Start the match again: the same seed and terrain, your placed blocks and your game mode are back.
  11. Wait 5 minutes (autosave, "Minecraft autosave: N chunks queued"), then quit the game from the menu. On the next start, the world loads with your edits.
- **Kit:** the regression kit above (27 stacks).
- **Not done here:** the plan's checkpoint release build. The track may not pass `--release`; the orchestrator builds it.
- **Decisions:**
  - The test helper `Parts` (with `Parts::host`) is needed by tests on both sides of the split, so it is duplicated: `commands.rs` has its own copy for the moved test, and `minecraft_chat.rs` keeps the original for its 4 tests that pump commands. A `#[cfg(test)]` helper can't be shared across crates.
  - `nbt_error` stays private in `commands.rs`, as it was in `minecraft_chat.rs`; only the items the adapter calls are `pub`.

## Test totals after A9

| Crate | Baseline (`70a01b2`) | After A9 |
|---|---|---|
| `minecraft_survival` | - | 119 |
| `render_anim` | 153 | 34 |
| sum | 153 | 153 |

No test was lost: save 12, death 25, offhand 10, survival 64, entities 6 and commands 2 moved (119). `minecraft_terrain` (464 passed, 3 ignored) and `minecraft_commands` (53) were not touched by A1-A9.

## B1. Minecraft dimension and player ids live in minecraft_net

A9 is commit `ec1bb14`.

- **What changed:**
  - `Dimension` moved from `minecraft_terrain/src/terrain.rs` into `minecraft_net/src/lib.rs` (new file, no comments, rustfmt-clean; its doc line is dropped). Same variants, `parse` and `dimension_type`, plus the derives `Hash`, `PartialOrd`, `Ord`, `Serialize` and `Deserialize`.
  - New `PlayerUuid(pub String)` in `minecraft_net`: the hyphenated offline UUID that `minecraft_commands::offline_uuid` returns, ordered and hashable so it can key the server's player map.
  - `terrain.rs` re-exports it with `pub use minecraft_net::Dimension;` where the enum was, so `minecraft_terrain::terrain::Dimension` (`minecraft_world.rs`, `examples/smoke.rs`) still resolves. `minecraft_terrain/Cargo.toml` gains the `minecraft_net` path dependency. No let-chains in the edition-2021 crate.
  - `minecraft_net` gains `serde_json` as a dev-dependency. `Cargo.lock` only gains the two dependency lines (`minecraft_net` -> `serde_json`, `minecraft_terrain` -> `minecraft_net`).
- **Tests added** (2, in `minecraft_net`): `dimension_types_parse_back` (`parse(dimension_type())` gives back each of the three dimensions, plus the short names and an unknown name) and `dimensions_and_uuids_survive_serde` (JSON round trip of the three dimensions and of a `PlayerUuid`). Totals: `minecraft_net` 2, `minecraft_terrain` 464 passed + 3 ignored (unchanged), `minecraft_survival` 119 and `render_anim` 34 (not touched).
- **Checks:** `check -p launcher --offline` (lock update), `check -p launcher --locked`, `test -p minecraft_net --locked`, `test -p minecraft_terrain --locked`; `rustfmt --check --edition 2024` on the new `lib.rs` prints nothing.
- **Owner test** (nothing visible changes; the overworld must generate exactly as before):
  1. Load a Minecraft match in the world folder used before (`IW4L_MINECRAFT_WORLD=split`). The log says "Minecraft world: opened ..." with the same seed, and the terrain around the spawn is the same as in your last screenshot of it.
  2. Walk a few chunks out: new chunks stream in with sky light, as before.
- **Kit:** any; the regression kit above works.
- **Decisions:**
  - Serde writes a dimension as its vanilla resource key (`"minecraft:overworld"`, `"minecraft:the_nether"`, `"minecraft:the_end"`, the same strings as `dimension_type()`) through `#[serde(rename)]`, and `PlayerUuid` as a plain string (`#[serde(transparent)]`), the way vanilla writes dimension keys and UUIDs in its JSON. The plan only asked for the derives; nothing serialises either type yet, so this changes no behaviour.
  - `PlayerUuid` derives `Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord`, enough for the `BTreeMap<PlayerUuid, Player>` of B5.

## B2. Minecraft terrain, mobs and containers belong to a per-dimension Level

B1 is commit `4ac0b2e`.

- **What changed:**
  - New `minecraft_survival/src/level.rs` (no comments, rustfmt-clean): `Level { dimension, entities: Option<Entities>, containers, terrain }` and `Terrain { stream, scene, environment }`, in that field order so a level drops its mobs (and their server thread, which saves entities) before its chunks.
  - New `minecraft_survival/src/server.rs` (same rules): `Server { levels: BTreeMap<Dimension, Level> }`. `globals` and `players` come in B3 and B5.
  - `minecraft_survival/src/lib.rs` gains `pub mod level;` and `pub mod server;`.
  - `render_anim/src/minecraft_world.rs`:
    - `Loaded` loses `stream`, `scene` and `environment`; `load()` returns `(Loaded, Terrain)` and the loader channel carries that pair.
    - `Runtime` loses `entities` and `containers` and gains `server: Option<Server>`.
    - Load completion builds the containers as a local (`Containers::default()` plus `set_book`, as before), and stores `Server { levels: {Overworld: Level { dimension: Overworld, entities: Some(entities), containers, terrain }} }` right before `runtime.world`, in the same frame as before.
    - `update()` binds the overworld level together with the world check, then `let Level { entities, containers, terrain, .. } = level;`, so the `entities` and `containers` bindings keep their types and their uses don't change. `world.scene`, `world.stream` and `world.environment` became `terrain.scene`, `terrain.stream` and `terrain.environment` (119 token replacements, three of them split over two lines).
    - Helpers: `stream_around_spawn(world: &Loaded, terrain, ...)`, `observe_spawn(terrain, ...)`, `write_blocks(terrain, registries, ...)` (its 7 callers pass `&world.registries`) and `autosave(world, terrain, ...)`.
    - `close_world` takes `runtime.server` instead of `runtime.entities`. `save_and_close` keeps the old order: it takes and drops every level's `entities`, joins the loader and drops `loading`, joins the level writer, reads the overworld's `world_spawn`, drops the server (chunks are saved) and the client assets, then writes `level.dat` and the player file.
- **Tests added:** none (`Level`, `Terrain` and `Server` are plain data). `minecraft_survival` 119, `render_anim` 34, total unchanged (153).
- **Checks:** `check -p launcher --locked` (no new warning), `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `level.rs`, `server.rs` and `lib.rs` pass `rustfmt --check --edition 2024` with empty output.
- **Owner test** (this is the plan's checkpoint after B2, focused on saving and reloading; the whole game must play as before):
  1. Load a Minecraft match with `IW4L_MINECRAFT_WORLD=split`. The kit is in the backpack, the terrain streams in around you and the spawn screen lifts as before.
  2. Place cobblestone and planks, then a chest, and put a few items in it. Smelt raw iron with coal in a placed furnace.
  3. Spawn a cow and a zombie from the eggs, and drop an item on the ground.
  4. Type `/time set 13000` and `/gamemode creative`.
  5. Leave the match. The log says "Minecraft world saved in N ms".
  6. Load the match again: same seed and terrain, your placed blocks, the cow and zombie (saved mobs), the dropped item, the evening time and creative mode are back. The chest and furnace start empty, as before this change: containers, the inventory and weather are not saved yet (docs/MINECRAFT-SURVIVAL.md), and the kit is given again.
  7. Stay 5 minutes: the log says "Minecraft autosave: N chunks queued". Quit the game from the menu, start it again and load the match: your edits are there.
  8. Walk far enough that chunks unload and come back: mobs in them come back too.
- **Kit:** `minecraft:cobblestone` x64, `minecraft:oak_planks` x64, `minecraft:chest` x2, `minecraft:furnace` x1, `minecraft:raw_iron` x16, `minecraft:coal` x16, `minecraft:cow_spawn_egg` x2, `minecraft:zombie_spawn_egg` x2 (all in the regression kit).
- **Not done here:** the plan's checkpoint release build (the orchestrator builds it).
- **Decisions:**
  - The `Server` is stored at the end of the completion block, next to `runtime.world`, instead of where `runtime.entities` was stored: the completion still reads `terrain.stream.player_spawn` for its log line after `Entities::new`. Nothing reads the server in between.
  - The overworld is bound in the same `let ... else` as the world, so the "no world" branch (hide the UI and return) covers both; the server and the world are always set and taken together.
  - `save_and_close` returns early only if the server has no overworld level, which can't happen: every server built today has one.

## B3. Minecraft world-wide state is kept in Server globals

B2 is commit `fd3a993`.

- **What changed:**
  - `minecraft_survival/src/server.rs`: `Server` gains `globals: Globals` (first field, as in the plan's section 2.1), and new `Globals { seed, registries, save, game_rules, difficulty, day, chat, level_writer, autosave_ticks }`. `data` comes in B4. The file stays comment-free and rustfmt-clean.
  - `render_anim/src/minecraft_world.rs`:
    - `Loaded` keeps only the client assets (`packs`, `atlas`, `celestial`, `cloud_mask`, `crack_texture`, `glint_texture`); `registries`, `seed`, `save`, `game_rules`, `difficulty`, `level_writer`, `chat` and `saved_day` are gone. `Runtime` loses `day` and `autosave_ticks`.
    - `load()` now builds the whole `Server` on the loader thread and returns `(Loaded, Server)`: `Globals` with the saved clock read into `day` (`DayCycle::default()` then `set(saved ticks)`, as the completion did) and `autosave_ticks: 0`, and the overworld `Level` with `entities: None` and default containers.
    - The completion binds `globals` and the overworld level from the loaded server, and keeps its steps in today's order: chat join, `Entities::new` (now on `level.terrain.stream`), `set_book` on `level.containers`, the station data and the enchantment names, `IW4L_MINECRAFT_TIME`, `save.started` and `set_host`, the starting game mode, then `level.entities = Some(entities)` and `runtime.server = Some(server)`.
    - `update()` binds `let Server { globals, levels } = server;` and then the overworld level. `world.game_rules`, `.save`, `.chat`, `.seed`, `.difficulty` and `.registries` became `globals.*` (33 sites), the `day` binding became `globals.day` (6 sites plus `day: &mut globals.day` in `Host`), and `*autosave_ticks` became `globals.autosave_ticks` (3). `Host` borrows the disjoint `globals` fields.
    - `save_player_mode(globals, mode)` and `autosave(globals, terrain, mode)` take `&mut Globals`; `autosave` reads the day ticks from `globals.day` itself (passing `globals.day.ticks` next to `globals` would not borrow-check). `stream_around_spawn` takes `&Registries` next to the terrain.
    - `close_world` no longer reads a day; `save_and_close` takes the save, the level writer, the rules, the difficulty and the day ticks from `server.globals`, with the same order as before (entities, loader, writer, chunks, then `level.dat` and the player file).
- **Tests added:** none (plain data). `minecraft_survival` 119, `render_anim` 34, total unchanged (153).
- **Checks:** `check -p launcher --locked` (no new warning), `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `server.rs` passes `rustfmt --check --edition 2024` with empty output.
- **Owner test** (rules, difficulty, time, chat and saving must work exactly as before):
  1. Load a Minecraft match with `IW4L_MINECRAFT_WORLD=split`. The log says "Minecraft world ready: seed N spawn ..." with the same seed as before, and the time of day is where you left it.
  2. Type `/time set 13000`: dusk falls. Type `/gamerule advance_time false`: the sun stops; `/gamerule advance_time true` starts it again.
  3. Type `/difficulty peaceful`: hostile mobs vanish; `/difficulty normal` brings them back at night.
  4. Type `/gamerule keep_inventory true`, fall to your death: you keep your items and XP. Then `/gamerule keep_inventory false`.
  5. Spawn a zombie from the egg and let it kill you: the chat shows "<name> was slain by Zombie". Type `/gamerule show_death_messages false` and die again: no death message.
  6. Type `/summon minecraft:cow`: the chat answers and a cow appears. Open a chest in a village or other structure if one is near: it is filled with loot.
  7. Type `/gamemode creative`, leave the match: the log says "Minecraft world saved". Load again: creative mode and the time of day are back.
  8. Stay 5 minutes: the log says "Minecraft autosave: N chunks queued".
- **Kit:** `minecraft:zombie_spawn_egg` x2, `minecraft:cobblestone` x64 (all in the regression kit).
- **Not done here:** nothing.
- **Decisions:**
  - The completion keeps its steps in the order they run today (`Entities::new` before the time and the game mode). The plan's list puts `Entities::new` last, but none of these steps reads another's result, and keeping the order keeps the diff to path changes.
  - If a loaded server had no overworld level the completion would warn "Minecraft world failed to load: no overworld" and stop, like a failed load; `load()` always builds one, so this can't happen. The same goes for the second `let ... else` in `update()`, which hides the Minecraft UI exactly as the "no world" branch does.

## B4. Minecraft recipes, loot, stations and enchantments load once per world

B3 is commit `96fff5a`.

- **What changed:**
  - New `minecraft_survival/src/data.rs` (no comments, rustfmt-clean): `GameData { jar, recipes: Arc<RecipeBook>, loot: Option<Arc<LootBook>>, stations: Arc<StationBook>, enchantments: Option<Arc<Enchantments>> }` (`Default` for a world without data) and `GameData::load(root, jar)`, which holds the jar reading and its log lines that were in `Entities::new` (item catalog, recipes, block loot, station recipes, enchantments, and the "data JAR not found" warning), unchanged apart from writing into `GameData`.
  - `Globals` gains `data: GameData`. The loader fills it with `Default::default()`, and the load completion replaces it with `GameData::load(asset root, data_jar())` on the main thread, right before `Entities::new`, where the jar was read before.
  - `Entities::new(stream, seed, &GameData)`: it still sends `load_loot` and `set_recipe_book` to its own level server thread when there is a jar, and keeps `Arc` clones of the books (`loot` is now `Option<Arc<LootBook>>`; `roll_drops_named` takes `&self`, so nothing else changed) plus a new `pub recipes` field for B6. The inventory takes `data.recipes`. The kit stays here until B6.
  - The completion reads the station book and the enchantments from `globals.data` for the containers, the inventory screens and the chat's enchantment names.
  - `minecraft_survival/src/lib.rs` gains `pub mod data;`.
- **Tests added** (2, in `data.rs`): `without_a_jar_there_is_no_loot_and_no_recipe` (no jar: no loot, an empty recipe book, no enchantments) and `an_unreadable_jar_is_kept_for_the_level_server` (a jar path that can't be read loads nothing but keeps the path, so the level server still gets its `load_loot` as before). Totals: `minecraft_survival` 121 (119 + 2), `render_anim` 34; the moved total stays 153.
- **Checks:** `check -p launcher --locked` (no new warning), `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `data.rs`, `server.rs` and `lib.rs` pass `rustfmt --check --edition 2024` with empty output.
- **Owner test** (everything read from the jar must work exactly as before):
  1. Load a Minecraft match with the regression kit. The log has "Minecraft recipes: N with the item catalog ..." and no "Minecraft data JAR not found", "recipes unavailable", "block loot unavailable", "station recipes unavailable" or "enchantments unavailable" warning.
  2. Open the inventory and craft sticks from planks in the 2x2 grid; place the crafting table and craft a chest. Sticks stack to 64.
  3. Place the furnace and smelt raw iron with coal: iron ingots come out.
  4. Mine stone with the pickaxe: it drops cobblestone; with silk touch, an ore drops itself. Shoot a block with a gun: it drops its loot too.
  5. Kill a cow from the spawn egg: it drops beef and leather.
  6. Place the enchanting table with the bookshelves around it, drink experience bottles and open it: three enchantment offers show; enchant something.
  7. Place the anvil and open it: it accepts items.
  8. Type `/give @s minecraft:diamond_sword[enchantments={sharpness:5}]`: an enchanted sword (it shimmers) lands in the inventory, or at your feet if it is full. Without the enchantment names the chat would answer "Malformed 'minecraft:enchantments' component" instead.
- **Kit:** `minecraft:oak_planks` x64, `minecraft:crafting_table` x1, `minecraft:furnace` x1, `minecraft:raw_iron` x16, `minecraft:coal` x16, `minecraft:diamond_pickaxe` x1 with silk touch, `minecraft:cow_spawn_egg` x2, `minecraft:enchanting_table` x1, `minecraft:bookshelf` x15, `minecraft:experience_bottle` x64, `minecraft:anvil` x1, `minecraft:diamond_sword` x1 (all in the regression kit).
- **Not done here:** nothing.
- **Decisions:**
  - `Globals.data` is a plain `GameData` (not an `Option`), filled with the empty default by the loader thread and loaded at the completion, as the plan says, so the jar is still read on the main thread at the same point of the load.
  - The jar is now read just before the level server thread starts instead of just after it, so that thread receives `load_loot` a moment later and the "Minecraft weather" log line (when `IW4L_MINECRAFT_WEATHER` is set) comes after the recipe lines. The commands reach the level server in the same order (`load_loot`, then `set_recipe_book`), and the main thread does the same work.
  - `set_recipe_book` is sent whenever there is a jar, where before it was sent only when the recipes parsed. If they fail to parse, the level server now gets an empty book instead of none; the only reader (sheep wool colour mixing) treats both the same (`matching` finds nothing and it falls back to a parent's colour with the same random draw).

## B5. Minecraft players are kept by UUID

B4 is commit `3385f09`.

- **What changed:**
  - New `minecraft_survival/src/player.rs` (no comments, rustfmt-clean): `Player { uuid, id, name, dimension, survival, mode, death_watch }` and `Player::new(profile, mode, survival)`, which builds the player from the chat server's `Profile` (its offline UUID, id and name) in the overworld with a fresh `DeathWatch`.
  - `Server` gains `players: BTreeMap<PlayerUuid, Player>`; the loader builds it empty. `minecraft_survival/src/lib.rs` gains `pub mod player;`.
  - `render_anim/src/minecraft_world.rs`:
    - `Runtime` loses `survival`, `player_mode` and `death_watch`, and gains `local: Option<PlayerUuid>` and `carried_survival: Option<Survival>` (owner decision 2).
    - Load completion: right after `chat.join(HOST, name)` it takes the host's profile; where it chose the starting game mode it now inserts `Player::new(&profile, starting_game_mode(...), carried_survival.take() or a new Survival)`, and it sets `runtime.local` next to `runtime.server`.
    - `update()` looks up the local player, then the level by `player.dimension`, and binds `let Player { survival, mode: player_mode, death_watch, .. } = player;`, so every use in the body is unchanged.
    - `close_world` takes the local player's `Survival` into `runtime.carried_survival` before the server goes to the save thread; `save_and_close` takes the local UUID instead of the mode and writes the player file from that player's mode.
    - `stop()` no longer resets the death watch: it is dropped with the server, and the next load starts a new one.
  - `render_anim/Cargo.toml` gains `minecraft_net.workspace = true`; `Cargo.lock` only gains that dependency line.
- **Tests added** (1, in `player.rs`): `a_joining_player_starts_in_the_overworld_with_the_survival_it_carries` (UUID, id and name from the profile, the overworld, the carried health, the given mode and a fresh death watch). Totals: `minecraft_survival` 122 (121 + 1), `render_anim` 34; the moved total stays 153.
- **Checks:** `check -p launcher --offline` (lock update), `check -p launcher --locked` (no new warning), `test -p minecraft_survival --locked`, `test -p render_anim --locked` (the chat tests); the staged `player.rs`, `server.rs` and `lib.rs` pass `rustfmt --check --edition 2024` with empty output.
- **Owner test** (health, hunger, XP, game mode and deaths must work exactly as before, and survival still carries from one Minecraft match to the next):
  1. Load a Minecraft match with `IW4L_MINECRAFT_WORLD=split` and the regression kit. Hearts, hunger and XP show.
  2. Drink a few experience bottles and fall about 4 blocks, so you have some XP levels and are missing a heart or two.
  3. Leave the match and load it again. The XP level, hearts and hunger are as you left them (they carry over within one game session, as before), and the kit is in the backpack again.
  4. Type `/gamemode creative`, leave the match: the log says "Minecraft world saved". Load again: creative mode is back. Type `/gamemode survival`.
  5. Fall from 6 blocks or more until you die: the death message and the killed-by card show, items (not guns) and XP drop, and you respawn normally.
  6. Leave and load the match again, then die once more (`/kill`): the death is handled the same way as in step 5, with "Killed <name>" in the chat.
  7. Type `/effect give @s minecraft:regeneration 10 1` with a heart missing: the hearts come back.
- **Kit:** `minecraft:experience_bottle` x64, `minecraft:cobblestone` x64 (both in the regression kit).
- **Not done here:** nothing.
- **Decisions:**
  - The player is inserted where the completion chose the game mode (after `Entities::new`) instead of right after `chat.join`, because its mode comes from `starting_game_mode`. The profile is taken right after `chat.join`; a missing profile fails the load like a missing overworld ("no host profile"), which can't happen because `join` always adds it.
  - `render_anim` names `PlayerUuid` for `Runtime.local`, so it depends on `minecraft_net` directly. The plan's section 1.2 lists only `minecraft_survival`, but a re-export would break "`lib.rs` holds only `pub mod` lines".
  - The `Runtime` field is `local`, as in the plan; `update()` binds it as `local_uuid`, because `local` is already its `LocalPresentClient` parameter.
  - `close_world` takes the survival with `std::mem::take` and leaves the player in the server, so the save thread still reads its mode. `save_and_close` writes the player file only if the local player exists, which is always the case once a world has loaded.
  - The carried survival falls back with `unwrap_or_else(Survival::default)`, because `clippy.toml` disallows `unwrap_or_default`.

## B6. Minecraft inventories belong to the player

B5 is commit `b275419`.

- **What changed:**
  - `minecraft_survival/src/player.rs`: `Player` gains `inventory: Inventory` and `selected: usize` (slot 0 at creation); `Player::new` takes the inventory. New `starting_inventory(recipes, kit)`: an inventory on the world's recipe book with the kit given into the backpack, with the same "Minecraft kit: N stacks from IW4L_MINECRAFT_KIT" log line.
  - `minecraft_survival/src/entities.rs`: `Entities` loses `inventory` and `selected`, and `Entities::new` no longer builds an inventory or gives the kit. The methods that used them take the player's part instead: `punch`, `hatch_egg`, `tick` and `server_items_tick` take the inventory (and the selected slot); `award_block_experience`, `drop_blocks` and `mined` take the held stack, and the hand-enchantment helpers read it instead of `self`. `drop_blocks` reads `self.recipes`.
  - `minecraft_survival/src/commands.rs`: `Host` gains `inventory: &'a mut Inventory` and `selected: usize`; `/give` fills that inventory, and the stack sizes come from `entities.recipes`. The test helper `Parts` gets an `Inventory::default()`.
  - `render_anim/src/minecraft_world.rs`:
    - The load completion builds the inventory with `starting_inventory(globals.data.recipes, IW4L_MINECRAFT_KIT)` right after `Entities::new`, where the kit was given before, and hands it to `Player::new`.
    - `update()` binds `inventory` and `selected` from the player. `entities.inventory` became `inventory`, `entities.selected` and `e.selected` became `*selected`, and `entities.inventory.recipes` became `entities.recipes` (60 lines). The four closures that went through `entities.as_ref()` keep that check (`|_|`).
    - `finish_item_use` takes the inventory; `hold_off_death` takes the inventory and the selected slot instead of `Entities`.
  - `render_anim/src/minecraft_chat.rs`: the test helper `Parts` gets an `Inventory::default()`.
- **Tests added** (1, in `player.rs`): `the_kit_fills_the_backpack_and_the_inventory_uses_the_world_recipes` (no kit: an empty inventory; a kit: its stacks split at the stack size into backpack slots 9 onwards; both on the world's recipe book). The B5 test now also checks that the player keeps the inventory it was given and starts on slot 0. Totals: `minecraft_survival` 123 (122 + 1), `render_anim` 34; the moved total stays 153.
- **Checks:** `check -p launcher --locked` (no new warning), `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `player.rs` and `commands.rs` pass `rustfmt --check --edition 2024` with empty output.
- **Owner test** (this is the plan's checkpoint after B6: the full regression, with extra inventory steps; everything must play exactly as before):
  1. Load a Minecraft match with `IW4L_MINECRAFT_WORLD=split` and the regression kit. The log says "Minecraft kit: 27 stacks from IW4L_MINECRAFT_KIT"; the kit is in the backpack, and hearts, hunger and XP show.
  2. Open the inventory and move a few stacks between the backpack and the hotbar. Select slots with the number keys and the mouse wheel: the held item shows in the hand. Select a gun: it is raised.
  3. Mine stone with the pickaxe and shoot a block with a gun. Both drop items, which you pick up with the pop sound; a silk-touch ore drops itself.
  4. Place a crafting table, a furnace and a chest. Craft something, smelt raw iron with coal, and put items in the chest.
  5. Place the ender chest and put an item in it. Break it: the item doesn't drop. Place it again, and the item is still there.
  6. Eat beef. Hold the shield up against a zombie from the spawn egg. Throw a pearl (teleport and fall damage) and an egg: each throw takes one from the stack. Hatch a cow from its egg: one egg is used.
  7. Pour the water bucket and scoop it up again; light a fire with flint and steel; fill a glass bottle at the water.
  8. Fall from 6 blocks or more (fall damage and sound), then swim in water and stand in lava briefly.
  9. Die with the totem in the offhand (it pops). Die again without it: items and XP drop, and guns stay.
  10. Type `/give @s minecraft:diamond 5`: the diamonds land in the inventory with the pickup sound. Type `/gamemode creative`, place some planks (the stack doesn't shrink), then `/tp ~ ~10 ~`, `/time set 13000`, `/weather rain` and `/summon minecraft:cow`. Then `/gamemode survival`.
  11. Leave the match, and check the log for "Minecraft world saved". Start the match again: the same seed and terrain, your placed blocks and your game mode are back, and the kit is in the backpack again (the inventory is not saved yet).
  12. Wait 5 minutes (autosave, "Minecraft autosave: N chunks queued"), then quit the game from the menu. On the next start, the world loads with your edits.
- **Kit:** the regression kit above (27 stacks).
- **Not done here:** the plan's checkpoint release build (the orchestrator builds it).
- **Decisions:**
  - The kit is still given right after `Entities::new` on the main thread, so its log line keeps its place; the adapter reads `IW4L_MINECRAFT_KIT` and `starting_inventory` takes it as an argument, which keeps it testable.
  - Recipe reads that went through `entities.inventory.recipes` use `entities.recipes`, the same `Arc` from `GameData` that B4 added for this, so those lines only lose a token; the player's inventory holds the same book.
  - The `let mut selected = entities.selected; ... entities.selected = selected;` copy in `update()` is gone: the inventory screen and the weapon request now change the player's selected slot directly. Nothing read the slot between those two lines, so the result is the same.
  - The closures that went through `entities.as_ref()` and the first `hold_off_death` call (now `if entities.is_some()`) keep their check on the level's mobs, so nothing changes even for a level without `Entities` (which never happens after the load).
  - `punch` gives `mob_action` the player's inventory by reference instead of a temporary clone, and gets `#[allow(clippy::too_many_arguments)]` like `update`; methods take the player's inventory (or held stack) as their first argument.

## B7. Minecraft hand and fall timers belong to the player

B6 is commit `cb2909c`. (Its entry says "60 lines" for `update()`: the exact count is 54 changed lines in the body of `update()`, and 62 in `minecraft_world.rs` with the helpers, the binding and the completion.)

- **What changed:**
  - `HandState` and `StepState` moved whole from `render_anim/src/minecraft_world.rs` into `minecraft_survival/src/player.rs` as `Hand` and `Steps`: the same fields, types and `Default` derive, with `pub` fields and without their doc comments (new file, no comments).
  - `Player` gains `hand: Hand` and `steps: Steps`, both default in `Player::new`.
  - `Runtime` loses `hand` and `steps`; `update()` takes them from the player (`let Player { survival, mode: player_mode, death_watch, inventory, selected, hand, steps, .. } = player;`), so no other line of `update()` changed.
- **Tests added** (1, in `player.rs`): `a_joining_player_has_no_hand_or_fall_timers_running` (no pearl cooldown, place delay, raised shield, swing, recent placements, last position or fall peak on a new player). Totals: `minecraft_survival` 124 (123 + 1), `render_anim` 34; the moved total stays 153.
- **Checks:** `check -p launcher --locked` (no new warning), `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `player.rs` passes `rustfmt --check --edition 2024` with empty output.
- **Owner test** (hand use, attacks, placing, footsteps and falls must work exactly as before):
  1. Load a Minecraft match with `IW4L_MINECRAFT_WORLD=split` and the regression kit.
  2. Walk on grass, stone and planks: footstep sounds play at the usual pace. Fall from 6 blocks or more: the fall damage and the big fall sound; fall 2 blocks: no damage.
  3. Hold the place button with planks: blocks go down at the usual repeat rate (about 5 per second). Mine stone with the pickaxe: the hand swings and the crack grows.
  4. Hit a zombie from the spawn egg with the sword, waiting between hits: full-strength hits; spam-click: weak hits.
  5. Throw an ender pearl: the hotbar shows the pearl cooldown for a second, and a second pearl can't be thrown until it ends.
  6. Raise the shield with the tactical key (shield in the offhand) against the zombie: the hits are blocked; let go and they hurt again.
  7. Throw a pearl, then leave the match right away and load it again: the new match starts without a pearl cooldown and without fall damage on the first landing.
- **Kit:** `minecraft:oak_planks` x64, `minecraft:diamond_pickaxe` x1, `minecraft:diamond_sword` x1, `minecraft:zombie_spawn_egg` x2, `minecraft:ender_pearl` x16, `minecraft:shield` x1, `minecraft:cobblestone` x64 (all in the regression kit).
- **Not done here:** nothing. C1-C5 and D1 are the next group.
- **Decisions:**
  - B7 is done: it stays a pure move of the two structs (the diff in `minecraft_world.rs` only deletes them and the two `Runtime` fields and adds `hand, steps` to the player binding).
  - As the plan's section 2.2 says, the hand and step timers now start fresh with each loaded world instead of carrying over from the previous match in the same game session. While no world is loaded `update()` returns before touching them, so before this change a pearl cooldown, a raised shield or a fall peak left at the end of one match carried into the first frames of the next; the most visible case was fall damage on the first landing of a new match if the last one ended in mid-air. Vanilla doesn't keep item cooldowns or a raised shield over a rejoin either, so this is the vanilla-like choice.

## C1. Minecraft saving and closing run from minecraft_survival

B7 is commit `9be1914`.

- **What changed:**
  - `minecraft_survival/src/server.rs` (no comments, rustfmt-clean) gains the save procedures from `render_anim/src/minecraft_world.rs`, unchanged apart from their paths:
    - `starting_game_mode(saved, chosen)`: the saved mode, or survival, switched to the mode `IW4L_MINECRAFT_GAMEMODE` names (by name or vanilla id).
    - `save_player_mode(globals, mode)`: the player file on the `minecraft-player-save` thread.
    - `autosave(globals, terrain, mode)`: queues the edited chunks and writes `level.dat` and the player file on the `minecraft-level-save` thread, with the same log lines.
    - `Server::save_and_close(self, local, started)`: the file-writing tail of the adapter's `save_and_close`. It joins the level writer, reads the overworld's world spawn, the rules, the difficulty, the day and the local player's mode, drops the server (the chunks are saved), then writes `level.dat` and the player file and logs "Minecraft world saved in N ms".
  - `render_anim/src/minecraft_world.rs` loses those three functions and the tail. Its `save_and_close` keeps the adapter part in the B2 order: drop every level's mobs, join the loader and drop `loading`, then drop the client assets and hand the server to `Server::save_and_close`. The three call sites point at `minecraft_survival::server::`, and the load completion passes `IW4L_MINECRAFT_GAMEMODE` to `starting_game_mode`.
- **Tests added** (2, in `server.rs`): `a_player_starts_in_survival_or_in_the_mode_their_file_saved` (no save and no choice gives survival, a saved mode comes back whole, an unknown name changes nothing) and `a_chosen_game_mode_by_name_or_id_overrides_the_saved_one` (a padded, capitalised name, an id, the previous mode and the abilities of the chosen mode, and choosing the mode you already have changes nothing). Totals: `minecraft_survival` 126 (124 + 2), `render_anim` 34; the moved total stays 153.
- **Checks:** `check -p launcher --locked` (no new warning), `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `server.rs` passes `rustfmt --check --edition 2024` with empty output.
- **Owner test** (saving, closing, autosave and the starting game mode must work exactly as before):
  1. Load a Minecraft match with `IW4L_MINECRAFT_WORLD=split` and the regression kit. Place some cobblestone and planks, and type `/time set 13000`.
  2. Type `/gamemode creative`. Leave the match: the log says "Minecraft world saved in N ms".
  3. Load the match again: the placed blocks, the evening and creative mode are back.
  4. Add `IW4L_MINECRAFT_GAMEMODE=adventure` to the `.env` and load the match: you start in adventure mode although the world saved creative. Remove the line again.
  5. Stay 5 minutes in the world: the log says "Minecraft autosave: N chunks queued for ... in X ms".
  6. Place a block, then quit the game from the menu without leaving the match first. Start the game and load the match: the block is there and the log of the last run ends with "Minecraft world saved".
- **Kit:** `minecraft:cobblestone` x64, `minecraft:oak_planks` x64 (both in the regression kit).
- **Not done here:** nothing.
- **Decisions:**
  - `starting_game_mode` takes the `IW4L_MINECRAFT_GAMEMODE` value as a second argument, read by the adapter, the same way B6's `starting_inventory` takes the kit: the server crate reads no environment for it, and the choice is testable.
  - The adapter now drops the client assets (`Loaded`) just before the server instead of just after it. Freeing them saves nothing, so the files are written in the same order (entities, then chunks, then `level.dat`, then the player file), and "saved in N ms" still covers the same work.
  - The procedures are free functions taking the parts they need (`&mut Globals`, `&mut Terrain`), so the call sites keep their argument order; only the close is a `Server` method, because it consumes the server.

## C2. Minecraft block edits run from the level

C1 is commit `8ad8272`.

- **What changed:**
  - `minecraft_survival/src/level.rs` (no comments, rustfmt-clean) gains three procedures from `render_anim/src/minecraft_world.rs`, unchanged apart from their paths:
    - `spill(containers, entities, pos)`: a broken container's items drop at its block and its stored furnace XP is found.
    - `write_blocks(terrain, entities, writes)`: the scene takes the blocks, the stream records and marks the edits, and the level server hears of each placed or broken block.
    - `finish_item_use(entities, inventory, used, consumes)`: the held stack is spent, a filled bottle or bucket goes to the hand or is tossed, and the tool wears. It returns the sounds it heard (with the flint-and-steel break sound when the tool breaks) instead of playing them.
  - `minecraft_survival/src/player.rs` gains `hold_off_death(survival, inventory, selected, struck)`, which returns whether a totem popped.
  - `render_anim/src/minecraft_world.rs`:
    - `spill` is gone; its two callers call `minecraft_survival::level::spill`.
    - `write_blocks` keeps its signature. It sets the MW2 collision shape of every write first, then hands the writes to the level.
    - `finish_item_use` and `hold_off_death` keep their signatures. They call the moved rules and play what comes back: the heard sounds, and `TOTEM_SOUND` when a totem popped. No call site changed.
- **Tests added** (2, in `player.rs`): `a_totem_holds_off_death_only_when_the_player_would_die` (alive: the totem stays; at zero health: it pops and leaves one health; with no totem left: nothing) and `a_totem_pop_from_the_game_uses_the_held_totem_at_any_health` (the MW2 pop uses a totem in the selected slot even at full health, and none in another slot). `spill`, `write_blocks` and `finish_item_use` need a level server and a streamed world, so they have no unit test. Totals: `minecraft_survival` 128 (126 + 2), `render_anim` 34; the moved total stays 153.
- **Checks:** `check -p launcher --locked` (no new warning), `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `level.rs` and `player.rs` pass `rustfmt --check --edition 2024` with empty output.
- **Owner test** (placing, item uses, container drops and totems must work exactly as before):
  1. Load a Minecraft match with the regression kit.
  2. Place a few cobblestone and planks blocks, then walk into them: they block you at once, and they make the place sound.
  3. Pour the water bucket on the ground: the water flows and the bucket is empty. Scoop the water up again: the bucket is full. Pour the lava bucket the same way and scoop it up.
  4. Light a fire on the ground with flint and steel: the fire shows and the flint and steel loses durability (its bar shows).
  5. Fill a glass bottle at the water: it becomes a water bottle.
  6. Place a chest, put a few stacks in it and break it: the chest and every stack drop.
  7. Place a furnace and smelt 8 raw iron with coal. Before taking the ingots out, break the furnace: the furnace, the ingots and the coal drop, and the XP bar fills a little.
  8. Put the totem of undying in the offhand and fall from 20 blocks or more: the totem pops with its sound and you keep one heart.
- **Kit:** `minecraft:cobblestone` x64, `minecraft:oak_planks` x64, `minecraft:water_bucket` x1, `minecraft:lava_bucket` x1, `minecraft:flint_and_steel` x1, `minecraft:glass_bottle` x4, `minecraft:chest` x2, `minecraft:furnace` x1, `minecraft:raw_iron` x16, `minecraft:coal` x16, `minecraft:totem_of_undying` x1 (all in the regression kit).
- **Not done here:** nothing.
- **Decisions:**
  - The plan names these `Level::write_blocks` and `Level::spill`. They are free functions in `level.rs` instead, taking the level's parts (`&mut Terrain`, `&mut Containers`, `&mut Entities`). `update()` holds the level split into those parts, so a `&mut Level` method can't be called there, and the call sites keep their argument order.
  - `hold_off_death` goes to `player.rs` rather than `level.rs`: it only reads the player's survival and inventory. `survival.rs` is a moved file, so it stays unedited during the split.
  - The adapter's `write_blocks` now sets every collision shape before the scene takes any of the blocks, where it used to alternate per block. Shapes come from the block states alone, and nothing reads the MW2 collision in between (`sim` reads it in FixedUpdate), so the end state is the same even when a position repeats in one write list.

## C3. Minecraft containers tick in their level

C2 is commit `b6a79ab`.

- **What changed:**
  - `minecraft_survival/src/level.rs` gains `tick_containers(containers, entities, terrain, ticks) -> (writes, brewed)`. This is the container tick moved from `update()` in `render_anim/src/minecraft_world.rs`. Each game tick it ticks the furnaces and brewing stands, drops what they spit out, and cooks on the lit campfires. It then turns their lit and bottle property changes into block writes (private `prop_writes`). It returns those writes and the brewing stands that finished. It also gets a `Writes` alias for the block write list, which `write_blocks` now uses too. The file stays comment-free and rustfmt-clean.
  - `update()` calls it once per frame for the frame's ticks, as before. It still plays the brewing sound at each finished stand and applies the writes through its `write_blocks`, so the MW2 collision is updated as before.
- **Tests added** (3, in `level.rs`, on `prop_writes` with a plain `HandcraftedScene`):
  - `a_furnace_lights_and_goes_out_through_its_lit_property`: a furnace keeps its facing and switches `lit`; lit then unlit in one batch ends unlit.
  - `brewing_stand_bottles_add_up_within_one_tick_batch`: a second bottle builds on the first write, not on the old block.
  - `blocks_without_the_property_and_empty_cells_are_left_alone`.
  
  Totals: `minecraft_survival` 131 (128 + 3), `render_anim` 34; the moved total stays 153.
- **Checks:** `check -p launcher --locked` (no new warning), `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `level.rs` passes `rustfmt --check --edition 2024` with empty output.
- **Owner test** (furnaces, brewing stands and campfires must work exactly as before):
  1. Load a Minecraft match with the regression kit.
  2. Place the furnace and smelt 8 raw iron with coal. While it smelts, its front glows (lit). The ingots come out one by one, and when it runs out of iron the glow goes out.
  3. Type `/give @s minecraft:brewing_stand`, `/give @s minecraft:blaze_powder 2` and `/give @s minecraft:nether_wart 4`. Fill three glass bottles at water (pour the water bucket first).
  4. Place the brewing stand and open it. Put in the blaze powder, then the three water bottles: the stand shows a bottle in each arm. Add the nether wart: about 20 seconds later the brewing sound plays and the bottles are awkward potions.
  5. Type `/give @s minecraft:campfire` and `/give @s minecraft:beef 2`. Place the campfire and use raw beef on it: the beef lies on the fire, and about 30 seconds later cooked beef pops off.
- **Kit:** `minecraft:furnace` x1, `minecraft:raw_iron` x16, `minecraft:coal` x16, `minecraft:glass_bottle` x4, `minecraft:water_bucket` x1 (all in the regression kit). The brewing stand, blaze powder, nether wart, campfire and raw beef come from `/give` in step 3 and 5.
- **Not done here:** nothing.
- **Decisions:**
  - Like C2, `tick_containers` is a free function taking the level's parts (`containers`, `entities`, `terrain`) instead of a `Level` method, so `update()` can call it while it holds the level split into those parts.
  - The property writes are computed in a private `prop_writes(scene, props)`, so they can be tested without a level server; `tick_containers` does the same work in the same order as the old block (all ticks, then the writes from all props).

## C4. Minecraft chunk streaming runs from the level

C3 is commit `e745029`.

- **What changed:**
  - `minecraft_survival/src/level.rs` gains `stream_tick(terrain, entities, block) -> (loaded, forgotten)`. It runs the stream's server tick around a block, hands each loaded chunk to the level server (`load_chunk`) and tells it which chunks were forgotten (`unload_chunk`), then returns both lists. The file stays comment-free and rustfmt-clean.
  - `render_anim/src/minecraft_world.rs`: `update()` (around the player's feet) and `stream_around_spawn` (around the spawn while no player is presented yet) call `stream_tick` instead of `server_tick`. `apply_streamed` loses its `entities` parameter and keeps the MW2 half: the collision shape ids of every loaded chunk, `sim::voxel::set_chunk` and `remove_chunk`.
- **Tests added:** none. `stream_tick` needs a streamed world and a level server, which a unit test can't build without the vanilla data. Totals: `minecraft_survival` 131, `render_anim` 34; the moved total stays 153.
- **Checks:** `check -p launcher --locked` (no new warning), `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `level.rs` passes `rustfmt --check --edition 2024` with empty output.
- **Owner test** (chunks, their collision and their mobs must stream exactly as before):
  1. Load a Minecraft match with the regression kit. The loading screen lifts once the ground around the spawn is there, and you land on solid ground.
  2. Spawn two cows from the eggs next to the spawn.
  3. Type `/gamemode creative` and fly about 300 blocks in a straight line. New terrain streams in ahead. Land now and then: the ground holds you, and water and lava behave as liquids.
  4. Fly back to the spawn: the cows are still where you left them (their chunks unloaded and loaded again).
  5. Type `/gamemode survival`, then die (`/kill`) and respawn: the world around the spawn is there at once and holds you.
- **Kit:** `minecraft:cow_spawn_egg` x2 (in the regression kit).
- **Not done here:** nothing.
- **Decisions:**
  - Like C2 and C3, `stream_tick` is a free function on the level's parts (`terrain` and the level's `Option<Entities>`), so `update()` and `stream_around_spawn` keep their bindings.
  - The level server now gets all `load_chunk` commands of a tick before the adapter fills the MW2 collision of those chunks, where the two used to alternate per chunk. As the plan says, the entity commands and the voxel updates don't depend on each other. Each kind keeps its own order: loads before unloads, chunks in stream order.

## C5. Minecraft chat command handling runs from minecraft_survival

C4 is commit `1703c63`.

- **What changed:**
  - `minecraft_survival/src/commands.rs` gains `run(globals, level, player, sent, pose, alive) -> Handled { delivered, teleports }`. It holds the chat command handling that ran at the top of `update()`, unchanged apart from its paths:
    - It builds the `Host` from the world globals, the level's mobs and the player's survival, inventory, selected slot and game mode.
    - It pumps the sent chat lines through the `CommandServer`.
    - After a rule change it hands the new rules to the level server, and it logs the chat server's warnings.
    - After a game mode change it writes the player file (`server::save_player_mode`).
    - It returns the messages to show and the teleports.
    
    The file stays comment-free and rustfmt-clean.
  - `render_anim/src/minecraft_world.rs`: `update()` computes the host's pose and life state as before (the pose now reads `player.survival`) and calls `run` with `&mut ui.chat.sent`. Then it destructures the player and the level into the same bindings as before, a few lines lower, and shows the delivered messages with `show_chat`. The rest of `update()` is unchanged.
- **Tests added:** none. `run` needs the world globals (loaded registries) and a level, which a unit test can't build without the vanilla data. What it runs is covered by the existing `pump` and `Host` tests (`commands.rs` and `render_anim/src/minecraft_chat.rs`). Totals: `minecraft_survival` 131, `render_anim` 34; the moved total stays 153.
- **Checks:** `check -p launcher --locked` (no new warning), `test -p minecraft_survival --locked`, `test -p render_anim --locked`; the staged `commands.rs` passes `rustfmt --check --edition 2024` with empty output.
- **Owner test** (chat and every command must work exactly as before):
  1. Load a Minecraft match with `IW4L_MINECRAFT_WORLD=split` and the regression kit.
  2. Open the chat and type `hello`: it shows as `<name> hello`. Type `/frob`: the red unknown-command error shows.
  3. Type `/give @s minecraft:diamond 5`: the diamonds land in the inventory. Select an empty hotbar slot and give again: they still land in the inventory.
  4. Type `/tp ~ ~10 ~`: you are lifted 10 blocks and fall back down. Die (`/kill`) and type `/tp ~ ~10 ~` before you respawn: nothing moves.
  5. Type `/gamerule spawn_mobs false`, then `/summon minecraft:zombie`: the zombie still appears, but no new mobs spawn on their own at night. Type `/gamerule spawn_mobs true`.
  6. Type `/gamerule block_drops false` and mine a block: it drops nothing. Type `/gamerule block_drops true`: blocks drop again.
  7. Type `/gamemode creative`, leave the match: creative is back on the next load (the player file was written). Type `/gamemode survival`.
  8. Type `/time set 13000` and `/weather rain`: dusk and rain. Type `/effect give @s minecraft:regeneration 10 1` with a heart missing: the hearts come back.
- **Kit:** `minecraft:cobblestone` x64, `minecraft:diamond_pickaxe` x1 (both in the regression kit). The rest comes from the commands.
- **Not done here:** nothing. The group's optional step is done, because it stayed a clean move.
- **Decisions:**
  - `run` takes `&mut Level` and `&mut Player` as the plan says, so `update()` destructures the player and the level after the call instead of before it. Destructuring has no side effects, and every later use keeps its binding.
  - The return type is a small `Handled { delivered, teleports }` struct, the two outputs `update()` still needs: the messages to show (presentation) and the teleports to apply to the MW2 player.

## Test totals after C5

| Crate | Baseline (`70a01b2`) | After B7 | After C5 |
|---|---|---|---|
| `minecraft_survival` | - | 124 | 131 |
| `render_anim` | 153 | 34 | 34 |
| `minecraft_net` | - | 2 | 2 (not touched) |

The 153 moved tests are all still there (119 in `minecraft_survival` plus 34 in `render_anim`). The 12 new `minecraft_survival` tests are B4 2, B5 1, B6 1, B7 1, C1 2, C2 2 and C3 3. `minecraft_terrain` (464 passed, 3 ignored), `minecraft_commands` (53) and `minecraft_net` (2) were not touched by C1-C5 (`git diff 9be1914` is empty for them).

## D1. Minecraft code map after the crate split

C5 is commit `636e433`.

- **What changed** (docs only, no code):
  - `docs/MINECRAFT-SURVIVAL.md`, "The Minecraft world is single player":
    - The server-thread line names `PLAYER` in `minecraft_survival/src/entities.rs` (it said `PLAYER = 0` in `minecraft_entities.rs`) and adds that what the server thread hands back for a player has no player key yet.
    - The survival-state line says the game state lives in `minecraft_survival`: one `Server`, a `Level` per dimension and a `Player` per UUID, with only the overworld and the host so far. It used to say the render crate.
    - The commands line names `minecraft_survival::commands::Host` as the `CommandWorld` implementation.
  - `docs/MINECRAFT-SURVIVAL.md`, Limitations, "Survival and combat": death drops are spawned by the per-frame update in `render_anim` with `minecraft_survival::death::death_drops`.
  - `docs/MINECRAFT-SURVIVAL.md`, new "Crate layout" section before "Testing workflow": `minecraft_survival` (`Server` with its globals, `Level` per dimension, `Player` per UUID, and the rule modules), `minecraft_net`, `minecraft_terrain`, `minecraft_commands`, and what stays in `render_anim`.
  - `docs/CRATE-SPLIT.md`: a "Done in `5a7a4e4`..`636e433`" note at the top. No step was skipped.
- **Tests added:** none (docs only). Totals unchanged from C5: `minecraft_survival` 131, `render_anim` 34, `minecraft_net` 2, `minecraft_terrain` 464 passed and 3 ignored, `minecraft_commands` 53.
- **Checks:** `check -p launcher --locked` on the final tree, whose code is identical to `636e433`.
- **Owner test** (the plan's end-of-split checkpoint: the full regression on the release build; the whole game must play exactly as before):
  1. Load a Minecraft match with `IW4L_MINECRAFT_WORLD=split` and the regression kit. The log says "Minecraft kit: 27 stacks from IW4L_MINECRAFT_KIT". The kit is in the backpack, and hearts, hunger and XP show.
  2. Mine stone with the pickaxe and shoot a block with a gun. Both drop items, and a silk-touch ore drops itself.
  3. Place a crafting table, a furnace and a chest. Craft something, smelt raw iron with coal (the furnace front glows while it smelts), and put items in the chest.
  4. Place the ender chest and put an item in it. Break it: the item doesn't drop. Place it again, and the item is still there.
  5. Eat beef. Hold the shield up against a zombie from the spawn egg. Throw a pearl (teleport and fall damage) and an egg.
  6. Fall from 6 blocks or more (fall damage and sound), then swim in water and stand in lava briefly.
  7. Die with the totem in the offhand (it pops). Die again without it: items and XP drop, guns stay, and the chat shows the death message.
  8. Type `/gamemode creative`, `/give @s minecraft:diamond 5`, `/tp ~ ~10 ~`, `/time set 13000`, `/weather rain` and `/summon minecraft:cow`. Each one works.
  9. Still in creative, fly about 300 blocks away and back. Terrain streams in ahead, and the cow is still there. Type `/gamemode survival`.
  10. Leave the match, and check the log for "Minecraft world saved". Start the match again: the same seed and terrain, your placed blocks and your game mode are back.
  11. Wait 5 minutes (autosave, "Minecraft autosave: N chunks queued"), then quit the game from the menu. On the next start, the world loads with your edits.
- **Kit:** the regression kit above (27 stacks).
- **Not done here:**
  - The plan's D1 also updates the PROGRESS.md row and log, and the roadmap line in CLAUDE.md if the owner wants it. The orchestrator owns both files, so this commit leaves them alone.
  - The end-of-split release build. The track may not pass `--release`, so the orchestrator builds it.
- **Decisions:**
  - The subject drops the plan's "Docs:" prefix, because CLAUDE.md wants plain-English subjects with no prefix.
  - "Crate layout" sits right before "Testing workflow", not inside "Multiplayer and saving today". That keeps it away from the saving lines that track D will change. The doc's status line at the top ("after round 6") is left for the orchestrator.
  - The "Done in" range in CRATE-SPLIT.md ends at C5, the last code commit, because D1 can't name its own hash.

## Test totals at the end of the split

| Crate | Baseline (`70a01b2`) | After D1 |
|---|---|---|
| `minecraft_survival` | - | 131 |
| `render_anim` | 153 | 34 |
| `minecraft_net` | - | 2 |
| `minecraft_terrain` | 464 passed, 3 ignored | 464 passed, 3 ignored |
| `minecraft_commands` | 53 | 53 |

All 153 `render_anim` tests from the baseline are still there: 119 moved to `minecraft_survival` and 34 stayed. The track added 14 tests, 12 in `minecraft_survival` and 2 in `minecraft_net`.
