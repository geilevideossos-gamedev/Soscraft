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
