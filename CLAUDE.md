# CLAUDE.md

Fork of chasmlol/2010-rust-rewrite-mashup: a Rust/Bevy rewrite of MW2 (IW4) with a Minecraft 26.3 world. Detailed state, limitations and testing: [docs/MINECRAFT-SURVIVAL.md](docs/MINECRAFT-SURVIVAL.md).

## Goal

Play online with 2-4 friends in one persistent Minecraft world, with MW2 weapons, movement and character models. Later, fight Elden Ring bosses. Skate 3 movement comes after that.

## Roadmap

The Minecraft side for one player is done (round 7). **Multiplayer (1.4) is the next step.**

1. **Multiplayer, saving and dimensions**
   1. Persistent world: **done.** Everything vanilla saves is saved in the vanilla 26.3 format: the world, all three dimensions, block entities and one player file per UUID.
   2. Crate split: **done.** `minecraft_survival` holds `Server` (world-wide state), a `Level` per dimension and a `Player` per UUID; `minecraft_net` holds the shared ids.
   3. Dimension switch and the Nether: **done.** Left for "Finish survival": N6 (piglin and hoglin brains, piglin brutes, zoglins, striders, bartering, lodestones).
   4. **Next: multiplayer with 2-4 friends.** The host owns the world and the friends join through our MW2 relay. Still to build: a server tick with several players (the level thread still talks to player `0`), every occupied level ticking, chunk streaming per player, an MW2 collision world per dimension, the network messages in `minecraft_net`, and the player actions and menus moved out of `render_anim` (`docs/CRATE-SPLIT.md` section 6).
   5. The End: **done apart from X6** (gateway travel to the outer islands, item frames, shulkers, levitation), which is left for "Finish survival". It was built before multiplayer, with all state per player and per level.
2. Finish survival: N6, X6 and the open limitations in the state doc
3. One Elden Ring boss as a proof of concept
4. Skate 3

**Why the crate split came before the dimension switch:** both rewrote the same code, and travel needed the split's seam (keep the player, swap the level). Multiplayer needs several dimensions loaded at once, because friends can be in different dimensions.

**All three dimensions:** everything built for saving and multiplayer must handle all three, with a server that can run several levels at once. The world folder uses the vanilla 26.3 layout. 26.3 no longer uses `DIM-1` and `DIM1` (checked with javap: `DimensionType.getStorageFolder`). Every dimension lives in `dimensions/minecraft/<overworld|the_nether|the_end>/` with `region`, `entities`, `poi` and `data`. `level.dat` and the global `data/` folder sit at the world root, and player files go in `players/data/<uuid>.dat`.

## Rules

- **Branch and remotes**
  - Work on `minecraft-survival`.
  - Push only to `origin` (geilevideossos-gamedev/mashup-crafting). Never push to `upstream` (chasmlol).
  - One commit per fix or feature. Subjects are plain English with no prefix, e.g. "Minecraft campfires cook food like vanilla".
- **Code style**
  - No code comments of any kind: no `//`, `/* */`, `///` or `//!`.
  - Never run `rustfmt` or `cargo fmt` on existing files. Edits change only the lines they need, in the surrounding style.
  - New files must pass `rustfmt --check` on their staged content.
- **Mojang files:** never commit any. Vanilla assets and data are read at runtime from the downloaded 26.3 packs and jar.
- **Cargo**
  - Run cargo only through `scripts\cargo-slot.ps1` from PowerShell, e.g. `& scripts\cargo-slot.ps1 test -p render_anim --locked`.
    - It adds `-j 8`, refuses other job counts and `cargo fmt`, and sets `CARGO_TARGET_DIR` (default `D:\iw4l-target`, or `+target=D:\dir`).
    - It also puts cargo on the `Path` and sets `MINECRAFTOSS_ROOT` to the downloaded 26.3 data when it is not set, so the tests that need the real game data run.
    - It holds one of two machine-wide slots, so at most two cargo processes run at once. More have run the PC out of memory.
    - `--release`, `--workspace`, `--profile` and `+exclusive` take both slots, so a release build or the full suite runs alone.
    - If its shell is killed, cargo and rustc die with it.
    - PowerShell drops a bare `--` when calling a script, so quote it: `& scripts\cargo-slot.ps1 clippy -p x --locked '--' -A clippy::approx_constant`.
  - One target dir per worktree. Two worktrees must never share one: cargo's mtime check would reuse the other worktree's build of a crate whose files are older there.
  - Builds link with rust-lld (`.cargo/config.toml`). Dev and test builds optimise dependencies at opt-level 1, skip debug info and are not incremental (`Cargo.toml`), so no `CARGO_INCREMENTAL` or `CARGO_PROFILE_DEV_DEBUG` variables are needed.
  - Use PowerShell, not Git Bash, for cargo.
- **Disk:** D: is short on space. One debug target dir is about 6-8 GB, or 11 GB with the full suite and clippy.
  - Delete a track's target dir (`D:\iw4l-wt\<name>`) once the track is merged.
  - `D:\iw4l-target` itself is protected from deletion by a permission rule. Only its subfolders may be removed, such as a stale `debug`.
- **Build workflow**
  - While implementing: `check -p` and `test -p` on only the crates that changed, and `check -p launcher` before a commit that touches game code.
  - Once per track: clippy on the touched crates, and the full suite `scripts\full-suite.ps1` (workspace tests without doc-tests, plus the five minecraftoss crates with their stand-in files) after the track is merged.
  - Parallel tracks: one git worktree in `C:\Dev\mc-wt\<name>`, one branch and one target dir (`D:\iw4l-wt\<name>`) each, merged into `minecraft-survival` one at a time with the full suite after each merge.
  - Tracks run with the saved workflow `.claude/workflows/track-runner.js` (the `track-runner` workflow). Args: `{track, worktree, branch, target, base, briefs, groups:[{key,prompt}], avoid?, notes?, skipReview?}`.
    - It runs one agent per commit group in order, each continuing from what is already committed, then an adversarial review that fixes real defects in new commits, then clippy and the tests of the touched crates.
    - Each track keeps a log `docs/round<N>/track-<name>-log.md` with its commits, decisions, the owner's test steps and one combined kit line.
    - The round's hand-over state is in `docs/PROGRESS.md`.
  - Merging a track, one at a time:
    1. In the track's worktree: `git checkout --detach minecraft-survival`, then `git merge --no-ff <track>`.
    2. Run `scripts\full-suite.ps1 +target=<the track's target dir>` there, so the merged code is built in a dir that has only seen this worktree.
    3. In `C:\Dev\mashup-crafting`: `git merge --ff-only <merge commit>`, then push to `origin`.
  - Release builds only when the owner asks for one.
- **Release build:** `& scripts\cargo-slot.ps1 build --release --locked -p launcher` (fat LTO: about 17 min clean, 13 min after an edit).
  - Output: `D:\iw4l-target\release\iw4l.exe`.
  - Copy it to `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-survival.exe`, after keeping the old one as `iw4l-survival-<old commit>.exe`.
  - A mid-round test build may use `--profile play` instead (the same code at opt-level 3 without fat LTO: about 10 min clean, seconds after an edit; output `D:\iw4l-target\play\iw4l.exe`). The last build of a round is always `--release`.
- **Test kit:** `IW4L_MINECRAFT_KIT=` in the `.env` next to the exe.
  - At most 27 stacks, because it only fills the 27 backpack slots.
  - Always give the kit line in its own code block so it isn't cut when copied.
- **Leave alone:** the untracked `blocks.txt` and `props.txt`, and the screenshot folders (`screenshots-*/`, `ac0c791/`, `scrennshots/`).
- **Tests:** the fork's test policy is the owner's. Don't trim tests because of upstream's `approved_tests` rule.
- **How the owner tests:** they play the release build and send numbered screenshots. Every change therefore ends with exact in-game test steps and the kit line they need.

## Reference repos (read-only, not integrated)

Cloned into `C:\Dev\references\`, outside this repo:

- **[chasmlol/skycraft](https://github.com/chasmlol/skycraft):** Skyrim x Minecraft.
  - A Fabric mod and an SKSE plugin talk over shared memory.
  - Multiplayer comes from vanilla Open to LAN plus the e4mc tunnel, and saving from vanilla autosave.
  - Licence: MIT.
- **[Funny-Bones/ELDEN-RING-Combat-Rewrite](https://github.com/Funny-Bones/ELDEN-RING-Combat-Rewrite):** Elden Ring player combat in Rust/Bevy 0.19.
  - Loads animations, TAE timings and params from the user's own game install.
  - It has player combat and a sparring dummy, no boss AI.
  - **No licence file:** read it, don't copy code.

## Where upstream docs are wrong for this fork

AGENT.md, CONTEXT.md, README.md, CONTRIBUTING.md and docs/ come from upstream and stay unedited so upstream merges stay clean. For this fork:

- **Workflow**
  - There is no `make`, no `make mr`, no `context/` folder and no `externals/`. References live in `C:\Dev\references\`.
  - The branch is `minecraft-survival`, not `master`. Even upstream's default branch is `main`.
  - Ignore the "rustfmt the touched files", "squash onto master" and `make publish-check` landing and public-push recipes in CONTEXT.md and AGENT.md.
  - We keep small descriptive commits and push only to origin.
  - CONTRIBUTING.md's `cargo fmt --all` would reformat upstream files, so don't run it.
- **Tests and comments:** "Tests live only in `crates/approved_tests`" (CONTEXT.md) does not apply. The fork's comment rule is stricter than CONTEXT.md's: no comments at all.
- **README.md**
  - The README was already edited by the fork up to ac0c791. It has no feature notes from round 4 onward, and none of the fork goal.
  - Its "Minecraft World.bat" starts upstream's `iw4l.exe`, not `iw4l-survival.exe`.
- **Multiplayer is MW2 only:** the MW2 "multiplayer", `docs/DUO.md` and `docs/MASTER.md` cover MW2 matches only. The Minecraft world is saved (all three dimensions, vanilla 26.3 format) but not shared yet (see the state doc).
- **Build docs:** `docs/BUILD.md` is Linux-first. This fork builds natively on Windows with MSVC.
- **Vendored code:** `third_party/minecraftoss` is MinecraftOSS `4013a68`, and the fork has patched 53 of its files and added 14 (as of round 7). If upstream ever re-vendors a newer MinecraftOSS, diff our patches first.
