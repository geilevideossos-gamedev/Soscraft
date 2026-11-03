# Track E3 log: known gaps from the release test files

Branch `track-e3`, worktree `C:\Dev\Soscraft\mc-wt\track-e3`, target dir `D:\iw4l-wt\track-e3`. Brief: `docs/round10/track-e3.md` (E3-0 first, then the section of each group).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Protocol

Nothing yet.

## State doc changes

Nothing yet. E3-1's tooling groups edit `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md` and `.claude/workflows/track-runner.js` directly (E3-0 "Docs"); their changes are listed in their entries below.

## Segment e3a

Base `r10-e3a-base` (3a70396d). The orchestrator held M3's m3d until e3a and e3b are merged (2026-11-02), so rider-falls stays in this segment.

## Group tool-clippy-vendored

### 1. A script lints the vendored minecraftoss crates with clippy

After 3a70396d. Gap code-3.

- **Why:** `Cargo.toml` excludes `third_party/minecraftoss` from the workspace, so its five crates are path dependencies, and `cargo clippy` wraps only members with `clippy-driver`: `clippy -p minecraftoss-entities` builds the crate with plain rustc and reports nothing. Until now the crates were linted by hand (touch `lib.rs`, `check` with `RUSTC_WRAPPER=clippy-driver`, touch again), and the clippy-built units replayed their warnings on later plain checks in the same target dir.
- **Changed:**
  - New `scripts/clippy-vendored.ps1`, shaped like `full-suite.ps1`, no comments. `+target=`, `+wait=` and `+exclusive` are passed to `cargo-slot.ps1`; every other argument is a crate name, short (`entities`) or full (`minecraftoss-entities`), any case; no name lints all five; an unknown name prints the usage and exits 2 before anything runs.
  - It lints in its own folder `<target>\clippy-vendored` (`<target>` is the `+target=` dir, else `$env:CARGO_TARGET_DIR`, else `D:\iw4l-target`) with `RUSTC_WRAPPER=clippy-driver`: `check -p minecraftoss-<crate>... --locked --all-targets`. Every unit there is built through `clippy-driver`, so cargo's replay of a fresh unit shows clippy's warnings again, and the track's own target dir never gets clippy-built units. Dependencies from crates.io get `--cap-lints allow` from cargo, so `clippy-driver` builds them as plain rustc; path packages (the five crates and `elden_boss`) are linted. No lint flags: the root `clippy.toml` reaches the crates through `clippy-driver`.
  - It saves `RUSTC_WRAPPER` and `CARGO_TARGET_DIR` and restores both in `finally` (an unset variable is removed again), because `cargo-slot.ps1` sets `CARGO_TARGET_DIR` in the caller's session. It ends with one line, `clippy-vendored: exit <code>, <crates>, lint dir <dir>, <minutes> min`, and exits with cargo's code.
  - `docs/MINECRAFT-SURVIVAL.md` "Per track" step 1: a sub-bullet on the script. `CLAUDE.md` "Build workflow": "Once per track: clippy on the touched crates (the vendored minecraftoss crates through `scripts\clippy-vendored.ps1`)". `.claude/workflows/track-runner.js` verify prompt, after step 2: a track that touched `third_party/minecraftoss` also runs the script and fixes its warnings on lines the track added (only that template string changed; `${A.target}` and the doubled backslashes kept; the file still parses and the rendered step reads `& .\scripts\clippy-vendored.ps1 +target=<target> <crate> <crate>`).
- **Decisions:**
  - `minecraftoss-player`'s lib test `include_str!`s two files under `third_party/scenarios`, so `check --all-targets` fails without them. The script makes the same empty stand-in files as `full-suite.ps1` when the player crate is linted and `third_party/scenarios` does not exist, and deletes them in `finally`. It dates them 2000-01-01, so cargo's mtime check keeps the player's lib test unit fresh and a second run replays its warnings instead of rebuilding it.
  - Crate names are lowered before the check, because cargo's package specs are case sensitive.
- **Tests:** no Rust tests: tooling only. The script is PowerShell, so rustfmt does not apply.
- **Runs** (2026-11-02, slots `slots=2`, `jobs=4`; the waits for a slot are other tracks' builds):
  1. `& scripts\clippy-vendored.ps1 +target=D:\iw4l-wt\track-e3 entities` (first build of the lint folder): cargo 93 s after 751 s waiting, exit 0. A crate's path dependencies are linted with it, so entities brings the player lib and `elden_boss`. Cargo's counts: `minecraftoss-entities` lib 69 warnings, lib test 76 (67 duplicates), ten bins 1-5 each; `minecraftoss-player` lib 20; `elden_boss` none. None is in Margit's files (`world/families/bosses/elden_bosses.rs` and `elden_bosses/`). Most are `disallowed_methods` (`unwrap_or_default`), then `ptr_arg` (`&Vec` arguments), `type_complexity`, `manual_*` and `neg_cmp_op_on_partial_ord`. Before and after: `RUSTC_WRAPPER=[]`, `CARGO_TARGET_DIR=[]` (unset, and unset again).
  2. The same command again: `Finished ... in 1.46s`, cargo exit 0 after 2 s, with the same 116 warning locations replayed (a sorted diff of the `-->` lines of both runs is empty). The same lint with `CARGO_TARGET_DIR=D:\iw4l-wt\track-e3` set and no `+target=`, named `minecraftoss-Entities`: lint dir `D:\iw4l-wt\track-e3\clippy-vendored`, the same counts in 1.01 s, and `CARGO_TARGET_DIR` afterwards `D:\iw4l-wt\track-e3` again.
  3. `& scripts\cargo-slot.ps1 +target=D:\iw4l-wt\track-e3 check -p minecraftoss-entities --locked --all-targets`: exit 0 in 85 s, no warning at all and no `clippy::` name in the output: plain rustc in the track's own folder, which has no clippy-built unit.
  4. `& scripts\clippy-vendored.ps1 +target=D:\iw4l-wt\track-e3 nosuch` and `... entities minecraftoss-bogus`: the usage line, exit 2, no cargo run and no lint folder made; both variables unchanged.
  5. `& scripts\clippy-vendored.ps1 +target=D:\iw4l-wt\track-e3` (all five): exit 0, cargo 16 s after 87 s waiting (entities replayed, the other four checked). Cargo's counts: core lib 25 and lib test 26 (25 duplicates); world lib 59 and lib test 61 (59 duplicates); generator lib 71 and lib test 72 (71 duplicates); player lib 20, lib test 31 (20 duplicates) and four bins 1 each; entities as in run 1. Distinct warning locations per crate: core 26, world 62, generator 76, player 46, entities 96 (306 in all; 85 of them `unwrap_or_default`, 60 `collapsible_if`). No `approx_constant` anywhere. The player lib test built with the stand-in files the script made; `third_party/scenarios` is gone afterwards, both variables are restored, and the lint folder holds 0.15 GB.
- **Not run:** `check -p launcher` and Rust tests: the commit changes no Rust code.
- **Owner's steps:** none in game (tooling only). No kit line.

## Group tool-lints-terrain

Before the group (at 17a138f2): `test -p minecraft_terrain --locked` passed 1818, failed 0, ignored 3 (lib; doc-tests 0), in 406 s.

### 2. minecraft_terrain takes the workspace lints, so clippy needs no extra flag

After 17a138f2. Gap code-2, part 1.

- **Why:** `crates/minecraft_terrain/Cargo.toml` was the only workspace member without `[lints] workspace = true`, so it missed the root `approx_constant = "allow"` and `excessive_precision = "allow"`, and every clippy call carried `'--' -A clippy::approx_constant`.
- **Changed:**
  - `crates/minecraft_terrain/Cargo.toml`: `[lints]` with `workspace = true` at the end, as in `crates/minecraft_survival/Cargo.toml`.
  - `docs/MINECRAFT-SURVIVAL.md` "Per track" step 1 (now `:505`): the clippy command loses ` '--' -A clippy::approx_constant`, and a new sub-bullet after the rule says: "Since round 10 E3, `minecraft_terrain`, `minecraft_survival` and the fork's `minecraft_*` files in `render_anim` and `hud` have no clippy warnings, so any warning there is new." The rule itself stays (no warnings on lines the track touched; older ones stay).
  - `.claude/workflows/track-runner.js:37`, the verify prompt's clippy step: ` '--' -A clippy::approx_constant` dropped; nothing else in the template string changed, and the file still parses (its body compiled as an async function).
  - `CLAUDE.md:61` keeps the point (quote the `--`) with an example that still needs it: `& scripts\cargo-slot.ps1 test -p minecraft_terrain --locked '--' darkness`.
- **Decisions:**
  - Inheriting `assertions_on_constants = "deny"` is safe: the crate has no constant assertions, and the clippy run below passes with the deny in place.
  - The new sentence also names `minecraft_survival`, `render_anim` and `hud`, which tool-lints-fork cleans later in this segment. Segment e3a merges as a whole, so the sentence holds when it reaches `minecraft-survival`. A branch made before E3 still carries the old warnings.
  - No other file named the flag outside the round logs and briefs (`git grep approx_constant`), which stay as written.
- **Tests:** no new tests (nothing changes in behaviour). `clippy -p minecraft_terrain --locked --all-targets` with no extra flag: exit 0, lib 33 warnings, lib test 34 (33 duplicates), 34 distinct locations, none `approx_constant`, `excessive_precision` or `assertions_on_constants`. These are the older warnings the next commit fixes. `check -p launcher --locked`: green.
- **Owner's steps:** none in game (lints and docs only). No kit line.

### 3. minecraft_terrain's older clippy warnings are fixed with no change in behaviour

After c1b6ecd6. Gap code-2, parts 2 and 3.

- **Why:** with the workspace lints in place, `clippy -p minecraft_terrain --locked --all-targets` still reported the crate's older warnings: lib 33, lib test 34 (33 duplicates), 34 distinct locations (the run with c1b6ecd6's lints in place, before this commit). They were exactly the brief's table (taken at 9c6efebd), with lines moved by the merges since: `pickup` is now `server.rs:4072` and its alias `PickedStack` `:4519`, `pack.rs:110` and `:113` are the two `map_or(true, ..)` at `:112` and `:113`, `model.rs:1002`, `mesh.rs:2625` for the late-initialised corners. No new warning had appeared.
- **Changed** (14 files, one line per site where the fix allows it, no comments, no other line touched):
  - `block_particles.rs:455`: `is_empty` after `len` (`len_without_is_empty`).
  - `server.rs:695` `merchant_view` offers: `.map_or_else(Vec::new, <[_]>::to_vec)`. `server.rs:4072` `pickup` returns `Vec<PickedStack>`, the alias with the same tuple (`type_complexity`).
  - `mob_actions.rs:68` `death_remains`: `loot.map_or_else(Vec::new, |book| book.death_drops(deaths))`.
  - `item_icons.rs:36` tints: `.transpose()?.into_iter().flatten().collect::<Vec<_>>()`. `item_icons.rs:94` language: the `map` becomes `map_or_else(HashMap::new, ..)` and the `unwrap_or_default` line goes.
  - `mesh.rs`: `:417` item tints as `item_icons.rs:36`; `:974` `.transpose()?.into_iter().flatten().map(..)`; `:677` block models through `std::collections::hash_map::Entry::Vacant` and `slot.insert(variants)` (`map_entry`); `:1643` `length <= 0.0 || !length.is_finite()` (a NaN length still returns `"up"` through `!is_finite()`); `:1975` `!sheet.width().is_multiple_of(width)` and the same for the height, after the zero test that keeps the zero case; `:2236` `full_cube.then_some(match dir { .. })` (the match only picks constant arrays); `:2528` `#[allow(clippy::too_many_arguments)]` on `fluid_quad`, as on `append_elements`; `:2621` `let (nw, ne, sw, se) = if h >= 1.0 { (1.0, 1.0, 1.0, 1.0) } else { ..; (fluid_corner_height(..), ..) };` with the same four calls in the same order.
  - `model.rs:687` textures: `.map_or_else(serde_json::Map::new, Clone::clone)` in place of `.cloned().unwrap_or_default()`. `model.rs:1002`, `pack.rs:112`, `:113`: `is_none_or(..)` in place of `map_or(true, ..)`.
  - `pack.rs:204` description: the `map` becomes `map_or_else(String::new, ..)` (the value is a `String`, not a `Vec` as the brief's table said).
  - `item_icon.rs:443` `raster_triangle`: `#[allow(clippy::too_many_arguments, clippy::needless_range_loop)]` (`x` and `y` are pixel coordinates as well as indices).
  - Type aliases next to each `type_complexity` site: `scene.rs` `ChunkEdits<'a>` between `HandcraftedScene` and its `impl`, so `chunk_edits` returns `ChunkEdits<'_>`; `cow_render.rs` `CowBox` and `HornBox` inside `append_cows`, each on the line above its `let`; `skeleton_render.rs` `Mushroom` above `MUSHROOMS`'s doc lines.
  - `server_mobs.rs`: `:437` `seed`; `:1313` a held trade item's components `.map(tag_json).as_ref().and_then(serde_json::Value::as_object).map_or_else(serde_json::Map::new, Clone::clone)`; `:2158` `.map_or_else(Vec::new, <[Tag]>::to_vec)`; `:2220` the `map` becomes `map_or_else(Vec::new, ..)` and the `unwrap_or_default` line goes.
  - `walk_animation.rs:67`, `:76`: `self.0.get(&id).map_or_else(WalkAnimation::default, |walk| *walk)`. M3's walk target (`:21`-`:22`) and the `mob_actions.rs` horse arm are untouched.
  - `fluid.rs:830` (test only): `.map(move |z| (x, z))` in place of `.filter_map(move |z| Some((x, z)))`.
- **Decisions:**
  - `server_mobs.rs:437`: `Tag::as_ints` returns an owned `Option<Vec<i32>>`, not a slice, so the brief's `unwrap_or(&[])` does not apply. `seed` folds `tag.get("UUID").and_then(Tag::as_ints).into_iter().flatten()` instead, over the same ints in the same order (two lines: the `let` and the fold's `iter()` and `&w`). A missing UUID still folds nothing and gives the start value.
  - `item_icons.rs:36` and `mesh.rs:417` keep a `Vec` for the slice callers, so they collect the flattened option; the turbofish is needed because the callers only borrow it as `&[[u8; 3]]`.
  - The aliases are private, like W's `DeathRemains` in `mob_actions.rs`; `cow_render.rs` keeps its two aliases inside the function next to their `let`s, as the boxes are local to it.
  - No `unwrap_or_else(Vec::new)` or `unwrap_or(Vec::new())` anywhere (both trip `clippy::unwrap_or_default`).
- **Tests:** no new tests (nothing changes in behaviour).
  - `clippy -p minecraft_terrain --locked --all-targets`, no extra flag: exit 0, no warnings at all (the output has no `warning` line).
  - `test -p minecraft_terrain --locked`: 1818 passed, 0 failed, 3 ignored (lib; doc-tests 0), in 793 s: the same counts as before the group (17a138f2: 1818, 0 and 3, in 406 s).
  - `check -p launcher --locked`: green; its only warnings are the old `render_frame` and skate ones that tool-lints-fork removes.
  - `git diff -U0 c1b6ecd6` on the crate: no comment added and no line changed beyond the warning sites.
- **Owner's steps.** Release build, survival, Normal, a new world in a plains with water nearby, by day. Nothing should look or behave differently from the release before; these steps cover every place the commit edits. Name the screenshots `e3-9-1.png` and so on.
  1. Look round: grass and leaves are green, water and lava animate, fences connect, and block corners are shaded. Place the water bucket on a slope and the lava bucket in a stone pit: the flowing water runs down its slopes and both surfaces animate as before. **e3-9-1**
  2. Open the inventory: the cobblestone, grass block, fence and leaves icons are 3D, and the grass and leaves icons are green. Hover over them: the names show. **e3-9-2**
  3. Use both cow eggs, then `/summon minecraft:cow ~ ~ ~4 {variant:"minecraft:warm"}`: the warm cow has its horns and its orange coat. Hold wheat: the cows follow you and their legs swing as they walk. Then `/summon minecraft:bogged ~ ~ ~6 {NoAI:1b}`: it has red and brown mushrooms on its head. **e3-9-3**
  4. Kill a cow with the sword: it drops beef, maybe leather, and experience orbs. Walk over them: they go into your inventory.
  5. Place the composter, use the villager egg beside it and wait until the villager becomes a farmer (green sparkles, a straw hat). Right-click it: the trading screen lists its offers. **e3-9-4**
  6. Save and Quit, then load the world: the cows, the bogged and the farmer are where they were, and the farmer offers the same trades.

```
IW4L_MINECRAFT_KIT=minecraft:water_bucket,minecraft:lava_bucket,minecraft:cow_spawn_egg*2,minecraft:wheat*16,minecraft:villager_spawn_egg,minecraft:composter,minecraft:iron_sword,minecraft:grass_block*16,minecraft:oak_leaves*16,minecraft:oak_fence*16,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

## Group tool-lints-fork

Before the group (at aa83c8d2):
- `clippy -p minecraft_survival -p render_anim -p hud --locked --all-targets`: exit 0. `minecraft_survival` lib 1 warning, lib test 1 (1 duplicate): `large_enum_variant` on `menus.rs:45` `Open`. `hud` lib 62, lib test 62 (62 duplicates): 1 in `minecraft_screens.rs:887` `SlotDraw`, 61 in the upstream MW2 files. `render_anim` lib 82, lib test 82 (82 duplicates): 5 in `minecraft_*` files (`minecraft_world.rs:149` and `:2306` `too_many_arguments`, `minecraft_host.rs:516` `unwrap_or_default`, `minecraft_inventory.rs:448` and `minecraft_sounds.rs:75` the disallowed `unwrap_or_default`), 77 in the other files.
- `test -p minecraft_survival -p render_anim -p hud --locked`: `hud` 206 passed; `minecraft_survival` 843 passed and `tests/vanilla_world.rs` 1; `render_anim` 226; no failure, none ignored, doc-tests 0 for each crate. The build printed the four old rustc warnings: `skate-core` `fetch_update`, `skate-host` `Settings` and `Runtime` (`private_interfaces`) and `render_frame` `must_use` on a static.

### 4. The fork's Minecraft files in survival, render_anim and hud pass clippy

After aa83c8d2. Gap code-2, part 4.

- **Why:** the fork's Minecraft files in `minecraft_survival`, `hud` and `render_anim` still had the seven older clippy warnings listed above, so a track could not tell a new warning there from an old one.
- **Changed** (one line per site, no comments, no other line touched):
  - `minecraft_survival/src/menus.rs:45` `enum Open` and `hud/src/minecraft_screens.rs:887` `enum SlotDraw`: `#[allow(clippy::large_enum_variant)]` (under the derive for `Open`).
  - `render_anim/src/minecraft_world.rs:149` `place_inventory_puppet` and `:2306` `draw_body_gear` (Bevy systems with 8 and 9 parameters): `#[allow(clippy::too_many_arguments)]` directly above the `fn`, as `update` (`:470`) and `stream_around_spawn` (`:1934`) carry. The doc lines above `place_inventory_puppet` stay.
  - `render_anim/src/minecraft_host.rs:516`: `host.edit_books.remove(&session.client).into_iter().flatten().collect()`, as `clicks` does above it (`edits` is `BookEdits`, a `Vec`).
  - `render_anim/src/minecraft_inventory.rs:448` item language: `unwrap_or_else(|_| HashMap::new())`, as the same file does at `:432`.
  - `render_anim/src/minecraft_sounds.rs:51`: the `.map(|sounds| ..)` becomes `.map_or_else(Vec::new, |sounds| ..)`, and the `.unwrap_or_default()` line (`:75`) goes.
- **Decisions:**
  - Allow, not box: there is one `Open` per player and one `SlotDraw` per slot per frame. Boxing the merchant or mount variant of `Open` would change 33 match and build sites across survival, render_anim and hud, next to M2's nautilus lines.
  - Left out, outside this group's crates: `assets/src/minecraft_setup.rs:69` has one disallowed `Result::unwrap_or_default` (an upstream file the fork extended). The `minecraft_*` sentence in `docs/MINECRAFT-SURVIVAL.md` names only `render_anim` and `hud`, so it stays true. `render_anim`'s 77 remaining warnings include 5 in `skate.rs` and `skate/rails.rs` (upstream files the fork extended): they count with the upstream ones, as the brief's "about 77" does.
- **Tests:** no new tests (nothing changes in behaviour).
  - `clippy -p minecraft_survival -p render_anim -p hud --locked --all-targets`: exit 0. `minecraft_survival` has no warning at all. `hud` lib 61, lib test 61 (61 duplicates); `render_anim` lib 77, lib test 77 (77 duplicates); none of them in a `minecraft_*` file.
  - `test -p minecraft_survival -p render_anim -p hud --locked`: `hud` 206, `minecraft_survival` 843 and 1, `render_anim` 226, no failure: the same counts as before the group.
  - `check -p launcher --locked`: green; its only warnings are the four old `render_frame` and skate ones the next commit removes.
- **Owner's steps.** Release build, survival, Normal, a new world in a plains, by day. Nothing should look or behave differently from the release before. Name the screenshots `e3-9-5.png` and so on.
  1. Put on the iron helmet and chestplate and open the inventory: the character in the inventory wears them and turns towards the mouse. In third person (F5) the MW2 body wears them too. **e3-9-5**
  2. Place the chest and open it. With the left button held, drag the stack of 64 cobblestone over four empty slots: while you drag, each slot shows 16, and the counts stay when you let go. Hover over the cobblestone: its name shows. Open the crafting table: it works as before. **e3-9-6**
  3. Place the composter and use the villager egg beside it. Once it is a farmer, right-click it: the trading screen opens and closes as before. You hear steps, the chest opening and the villager's voice.
  4. Write two pages in a book and quill, press Done and open it again: the pages are kept. Optional, with two windows (`scripts\mp-local.ps1`): the friend does the same with the second book and quill, and their pages are kept too (this is the host path the commit edits). **e3-9-7**

```
IW4L_MINECRAFT_KIT=minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:chest,minecraft:crafting_table,minecraft:cobblestone*64,minecraft:writable_book*2,minecraft:villager_spawn_egg,minecraft:composter,minecraft:cooked_beef*16
```
