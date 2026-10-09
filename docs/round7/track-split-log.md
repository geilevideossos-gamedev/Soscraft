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
