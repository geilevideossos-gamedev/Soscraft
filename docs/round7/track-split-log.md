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
