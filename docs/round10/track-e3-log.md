# Track E3 log: known gaps from the release test files

Branch `track-e3`, worktree `C:\Dev\Soscraft\mc-wt\track-e3`, target dir `D:\iw4l-wt\track-e3`. Brief: `docs/round10/track-e3.md` (E3-0 first, then the section of each group).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Protocol

Nothing yet.

## State doc changes

E3-1's tooling groups edit `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md` and `.claude/workflows/track-runner.js` directly (E3-0 "Docs"); their changes are listed in their entries below. For the orchestrator to copy at round end:

- `docs/round10/release-2-tests.md:565` (and any state doc that repeats it): "A rider hears no block fall sound, and a mob riding a horse takes no share of its fall." becomes "A rider hears no block fall sound (vanilla)." (rider-falls, E3-2). `docs/round10/track-e1-log.md:477`'s note that a wind charge's impulse does not shorten a rider's share is resolved by the same group.

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

### 5. The launcher check builds without the old render_frame and skate warnings

After 70a8016a. Gap code-2, parts 5 and 6.

- **Why:** every `check -p launcher` printed four old rustc warnings: `render_frame` `unused_attributes` (`#[must_use]` on a static, which will become a hard error), `skate-core` the deprecated `AtomicUsize::fetch_update` (renamed `try_update` in toolchain 1.99), and `skate-host` two `private_interfaces`.
- **Changed** (one line each; patches to files that were identical to `upstream/main`):
  - `crates/render_frame/src/retained.rs` (upstream IW4L): `#[must_use]` moves from `:156`, where upstream 4979ef31 left it above the new `DEPTH_HACK_SCENE_ENTNUM` static, back above `pub const fn host_viewmodel_render_fx_flags` (now `:165`), where 0f387098 had it. The doc lines above the static stay. Every caller uses the result (`render_gpu/src/drawsurf/depth_range.rs:22`, `render_backend/src/material_exec.rs:175`, `:359`), so no new warning appears.
  - `skate/crates/skate-core/src/animation/commands/buffers.rs:9` (vendored SK8-ENGINE cb79689): `fetch_update` becomes `try_update`, with the same arguments and result (stable since 1.95; in 1.99 `fetch_update` only calls it).
  - `skate/crates/skate-host/src/physics/footplant/settings.rs:4` (SK8-ENGINE): `pub(super) struct Settings` becomes `pub(in crate::physics) struct Settings`, the visibility of the `Footplant::settings` field that holds it.
  - `skate/crates/skate-host/src/physics/respawn.rs:16` (SK8-ENGINE): `pub(super) struct Runtime` becomes `pub(crate) struct Runtime`, the visibility of `SkaterRuntime` whose `pub respawn` field holds it.
  - The types are raised, not the fields, so no field's access changes. **Patches to keep or drop knowingly** at an upstream merge or a skate re-vendor: these four files, one line each.
- **Decisions:**
  - The brief's `check -p skate-core -p skate-host --locked --all-targets` cannot run from the root workspace: with `--all-targets` cargo answers "package ID specification `skate-core` did not match any packages" (the same for `skate-host`), because both belong to their own skate workspaces (`skate/Cargo.toml`, and `skate-host`'s own `[workspace]`), which have no lock file. Without `--all-targets` the specs match. So the libs are checked from the root, and `skate-core`'s test target through its own workspace with a copy of the root `Cargo.lock` that is deleted afterwards (`--offline`, so cargo only prunes the copy and picks no new versions). `skate-host` has no tests.
- **Tests:** no new tests (nothing changes in behaviour).
  - `check -p skate-core -p skate-host --locked`: exit 0, no warnings.
  - `check --manifest-path skate\Cargo.toml -p skate-core --offline --all-targets` (lib and lib test, temporary lock copy): exit 0, no warnings; `skate\Cargo.lock` is gone afterwards.
  - `check -p render_frame --locked --all-targets`: exit 0, no warnings.
  - `check -p launcher --locked`: exit 0 and no warning at all (no line of the output contains "warning"); its tail is `Checking launcher v0.1.0 ...`, `Finished dev profile [unoptimized] target(s) in 37.73s`.
  - `test -p minecraft_survival -p render_anim -p hud --locked`: `hud` 206, `minecraft_survival` 843 and 1, `render_anim` 226, no failure: the same counts as before the group, and the build printed no warning.
- **Owner's steps.** The same release build and world as the steps above; nothing should change. The kit is the one above (skating needs no item).
  5. Press **J**: you skate as before. Bail off a ledge: you get back on the board. **e3-9-8**

## Group rider-falls

Gaps horses-4 and horses-7 (E3-2). Anchors re-checked at ce609a5c: `take_rider_falls` is `riding.rs:683`, the tick's last `end_ride_tick` `world.rs:8200-8202`, `player_dismounts` `world.rs:2660`, `:2777` and `:5874`, `exposed_mut` `pushing.rs:194`, the strider landing `striders.rs:442-449`, `rider_fall` `survival.rs:540` and its caller `minecraft_players.rs:864-865`.

### 6. Minecraft mobs riding a horse share its fall

After ce609a5c. Gap horses-4, horse half.

- **Why:** `take_rider_falls` drained each horse's `pending_fall` for its player riders only and mapped mob riders to nothing, so a husk riding a horse off a cliff landed unhurt.
- **Changed:**
  - `entities/src/inside_blocks.rs`: `Hazard::Fall` between `Cramming` and `Freeze`. `bypasses_armor` includes it and `damage_type` is `minecraft:fall`; `is_fire` and `panics` do not (26.3's `#bypasses_armor` has `minecraft:fall`; `#panic_causes` and `#panic_environmental_causes` do not).
  - `crates/minecraft_survival/src/death.rs`: `damage_type` and `message_id` get the `Hazard::Fall` arm beside `Cramming` (`minecraft:fall`, `fall`, the words of `Self::Fall`). Players never take it: they fall through `DamageCause::Fall`.
  - `entities/src/world/pushing.rs:194`: `exposed_mut` is `pub(super)`, so `riding.rs` can hurt any mob by key.
  - `entities/src/world.rs`: `EntityWorld.rider_falls: Vec<(u64, f64, f32)>` beside `player_dismounts` (declaration, `Default` and `clone_where`'s copy, which starts empty), and one line after the tick's last `end_ride_tick`: `self.share_vehicle_falls(&*world, game_time);`.
  - `entities/src/world/riding.rs`:
    - `share_vehicle_falls(world, game_time)` takes every horse's `pending_fall` (set by `horse_fall` only when the horse itself is hurt, as `AbstractHorse.causeFallDamage`) and walks its riders: a player rider is queued in `rider_falls` as `(player, distance, multiplier)`; a mob rider takes its share in `share_fall` and then hands the same fall to its own riders (`Entity.propagateFallToPassengers` runs inside every passenger's `causeFallDamage`).
    - `share_fall`: a rider whose type is in 26.3's `#fall_damage_immune` takes nothing and passes nothing on (`Entity.causeFallDamage` returns before propagating, and `LivingEntity.calculateFallDamage` gives 0). Otherwise `floor((distance + 1e-6 - 3) * multiplier)`; above 0 it pushes `entity.hostile.small_fall`/`big_fall` (big above 4) at the rider for the stores whose own landings pass `hostile` (zombies, skeletons, creepers, spiders, endermen, witches, illagers, ravagers, blazes, piglins), zoglins, and registry families whose `families::category` is `monster`, `entity.generic.*` for the rest, then `exposed_mut(key).hurt_hazard(Hazard::Fall, damage, world, game_time)` while the rider lives. No block fall sound (vanilla: 0.2 below the rider's feet is the air above its mount).
    - `take_rider_falls` is now `std::mem::take(&mut self.rider_falls)`, so `server.rs:5237` is unchanged.
  - `entities/src/world/equines.rs` tests: `a_ridden_horse_falling_ten_blocks_takes_two_and_hands_its_rider_the_fall` and `a_llama_landing_makes_no_land_sound` asserted that `pending_fall` still waited after the tick; the tick now hands it on, so they assert it is gone (their rider-fall checks are unchanged).
  - Vendored files: 5 patched (`inside_blocks.rs`, `world.rs`, `world/pushing.rs`, `world/riding.rs`, `world/equines.rs` tests), none new.
- **Decisions:**
  - `#fall_damage_immune` is a names list in `riding.rs` (`fall_damage_immune`, the 18 ids of the 26.3 tag), like `hazards::fire_immune`: `EntityWorld` holds no entity tags. The brief did not name it; without it a chicken or a golem riding a horse would take the fall.
  - A mob rider uses the default safe distance 3 and multiplier 1 (the brief), not its own type's attributes; no mob that can ride today has others.
  - A dying rider still voices the fall and passes it on, but is not hurt (vanilla plays the sound before `hurt`, which a dying mob refuses).
  - The share happens at the end of the tick, after every rider has ticked, not inside the horse's own tick as in vanilla; the damage, sound and order of riders are the same.
- **Tests:**
  - `world::riding::tests`: `a_husk_riding_a_horse_fifteen_blocks_down_takes_twelve_with_the_big_fall_sound` (horse 20 to 16, husk 20 to 8 though its armour is 2, one `entity.hostile.big_fall` and no other fall sound, the husk rides on, no player fall), `a_mob_rider_takes_nothing_when_its_horse_takes_nothing` (7 blocks), `a_player_rider_s_share_is_queued_once` (10 blocks: one `(0, 10, 1)`, then none, and none after standing 60 more ticks), `a_registered_mob_riding_a_horse_takes_the_fall_through_its_family` (D0's probe on a horse falling 8: horse 19, probe 10 to 5, one `entity.hostile.big_fall`), `a_chicken_riding_a_horse_is_spared_and_a_rider_on_a_rider_shares_the_fall` (horse, strider, zombified piglin 15 blocks: strider and piglin 20 to 8 with one generic and one hostile big fall; a chicken on a horse keeps its 4 health and no fall sound plays).
  - `inside_blocks::tests::a_shared_fall_is_minecraft_fall_bypasses_armour_and_panics_nothing`.
  - `minecraftoss-entities`: 618 passed, 0 failed (lib; bins and doc-tests 0), no warning.
  - `minecraft_survival`: 843 passed and `tests/vanilla_world.rs` 1, no failure. `check -p launcher --locked`: green, no warning.
- **Owner's steps** (release build, survival, a new world, by day on flat grass, the group's kit at the end of this group; full health before each drop). Name the screenshots as the brief does: `e3h01.png` to `e3h04.png`.
  1. Build a 3-high pillar and stand on it: `/fill ~2 ~ ~ ~2 ~2 ~ cobblestone`, then `/tp @s ~2 ~3 ~`.
  2. `/summon minecraft:horse ~4 ~12 ~ {Passengers:[{id:"minecraft:husk"}]}`: the horse falls 15 blocks with the husk on it.
     - On landing you hear the horse's thud and a heavy fall crunch from the husk, and both flash red.
     - The husk rides on (it lost 12 of its 20).
     - **Screenshot e3h01**, just after the landing.
  3. `/summon minecraft:horse ~4 ~22 ~ {Passengers:[{id:"minecraft:husk"}]}` (25 blocks): the horse flashes red and lives, and the husk dies on landing. **Screenshot e3h02.** Before this build both husks rode on unhurt. Kill the survivors with the sword.

### 7. Minecraft strider riders share its fall

After 310cd4bf. Gap horses-4, strider half.

- **Why:** a strider landed through `living::fall_damage` and recorded nothing for its riders, and the share walked only the cow store's horses. Vanilla's `Strider` does not override `causeFallDamage`, so `LivingEntity.causeFallDamage` hands every landing on to its passengers (`Entity.causeFallDamage`, offset 67) before and whatever its own damage.
- **Changed:**
  - `entities/src/world/striders.rs`: `StriderEntity.pending_fall: Option<(f64, f32)>` beside `last_damage_tick`, `None` in `spawn_strider` (the only constructor, so on load too). A landing out of lava with `fallen > 0` records `(fallen * reduction, multiplier)` from `living::landed_block`, as `Block.fallOn` passes it (a bed halves the distance, hay gives 0.2, slime 0), before the strider's own damage. A landing in lava records nothing (`Strider.checkFallDamage` resets the fall in lava).
  - `entities/src/world/riding.rs` `share_vehicle_falls`: takes the striders' `pending_fall` after the horses', so a player rider is queued in `rider_falls` (the player's `rider_fall` gives 0 up to 3 blocks) and a strider jockey takes its share as a horse's mob rider does.
  - Vendored files: 2 patched (`world/striders.rs`, `world/riding.rs`), none new.
- **Decisions:**
  - A dying strider's landing (`dying_travel`) passes nothing on, as a dying horse's does not before mount-odds; a dead strider's rider is let down within a second anyway.
  - A strider that steps down a block hands its rider a fall under 3 blocks, which hurts nobody; vanilla does the same, and the share stays on the host (no message).
- **Tests:**
  - `world::riding::tests::a_strider_dropping_six_blocks_hands_its_rider_six`: a saddled strider with the player and a strider with a zombified piglin each drop 6 blocks onto netherrack. Both striders 20 to 17 with `entity.generic.small_fall`; one `(0, 6, 1)` for the player, then none after 60 more ticks; the piglin 20 to 17 with `entity.hostile.small_fall`.
  - `world::riding::tests::a_strider_landing_in_lava_hands_nothing_on`: 6 blocks into a lava sea with the player on: the strider keeps its 20, no rider fall and no fall sound.
  - `minecraftoss-entities` lib: 620 passed, 0 failed. `minecraft_terrain`: 1818 passed, 0 failed, 3 ignored (lib; doc-tests 0), the counts before the group, so no server test changed with the shared falls of commits 6 and 7. `check -p launcher --locked`: green, no warning.
- **Owner's steps** (as above, in the Nether or the Overworld; the steps go on from step 3):
  4. Strider: at a fresh spot, run `/fill ~3 ~ ~-3 ~9 ~5 ~3 netherrack` (6 high), then `/tp @s ~6 ~6 ~`.
     - Use a strider egg on the top, saddle the strider and get on.
     - Hold the warped fungus on a stick and ride off the edge.
     - You lose 1.5 hearts and hear your fall; the strider loses 1.5 of its 10 mount hearts.
     - **Screenshot e3h03**, right after landing.
  5. Repeat from 4 high (`/fill ~3 ~ ~-3 ~9 ~3 ~3 netherrack`, `/tp @s ~6 ~4 ~`): you lose half a heart. From 3 high you lose nothing.
  7. Friend (two windows, `scripts\mp-local.ps1`): F does step 4 on F's own strider. F loses 1.5 hearts, and the host hears F land. **Screenshot e3h04**, F's window.
