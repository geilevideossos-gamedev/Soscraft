# CLAUDE.md

Fork of chasmlol/2010-rust-rewrite-mashup: a Rust/Bevy rewrite of MW2 (IW4) with a Minecraft 26.3 world. Detailed state, limitations and testing: [docs/MINECRAFT-SURVIVAL.md](docs/MINECRAFT-SURVIVAL.md).

## Goal

Play online with 2-4 friends in one persistent Minecraft world, with MW2 weapons, movement and character models. Later, fight Elden Ring bosses. Skate 3 movement comes after that.

## Roadmap

The Minecraft side for one player is done (round 7). **Multiplayer (1.4) was built in round 8 and waits for the owner's two-window and internet tests** (`docs/round8/track-e-tests.md`).

1. **Multiplayer, saving and dimensions**
   1. Persistent world: **done.** Everything vanilla saves is saved in the vanilla 26.3 format: the world, all three dimensions, block entities and one player file per UUID.
   2. Crate split: **done.** `minecraft_survival` holds `Server` (world-wide state), a `Level` per dimension and a `Player` per UUID; `minecraft_net` holds the shared ids.
   3. Dimension switch and the Nether: **done.** Round 8 (track B) added most of N6: piglins, brutes, bartering, hoglins, zoglins and striders. Only lodestones are left for "Finish survival".
   4. Multiplayer with 2-4 friends: **built, waiting for the owner's test.** The host owns the world and the friends join through our MW2 relay with a join code. Every player, every occupied level and the dragon fight run on the host, and each dimension has its own MW2 collision band. Design and what was left: `docs/MULTIPLAYER.md` (section 20). Owner's test: `docs/round8/track-e-tests.md`. Friend's install: `docs/round8/friend-install.md`.
   5. The End: **done.** Round 8 finished X6: gateway travel to the outer islands (track A), shulkers, levitation, End cities and item frames (track C). It was built before multiplayer, with all state per player and per level.
2. Finish survival: lodestones, trial spawners, the open limitations in the state doc and the multiplayer leftovers (`docs/MULTIPLAYER.md` section 20: skating for friends, trading screens, Minecraft melee between players, the friend's first-person hand). The open non-vanilla items are listed in `docs/PROGRESS.md` (Decisions), such as a ridden strider standing still under `/tick freeze`.
3. One Elden Ring boss as a proof of concept
4. Skate 3. Skating already works in Minecraft worlds for the host (round 8, track D), but not yet for friends.

**Why the crate split came before the dimension switch:** both rewrote the same code, and travel needed the split's seam (keep the player, swap the level). Multiplayer needs several dimensions loaded at once, because friends can be in different dimensions.

**All three dimensions:** everything built for saving and multiplayer must handle all three, with a server that can run several levels at once. The world folder uses the vanilla 26.3 layout. 26.3 no longer uses `DIM-1` and `DIM1` (checked with javap: `DimensionType.getStorageFolder`). Every dimension lives in `dimensions/minecraft/<overworld|the_nether|the_end>/` with `region`, `entities`, `poi` and `data`. `level.dat` and the global `data/` folder sit at the world root, and player files go in `players/data/<uuid>.dat`.

## Rules

- **Branch and remotes**
  - Work on `minecraft-survival`.
  - Push only to `origin` (geilevideossos-gamedev/Soscraft, public). Never push to `upstream` (chasmlol).
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
    - Gentle mode while the owner plays on this PC: `%LOCALAPPDATA%\iw4l-cargo-slot.txt` with `slots=1`, `jobs=4` and `priority=low` runs one cargo at a time with `-j 4` below normal priority. For full speed when the owner is away, set `slots=2`, `jobs=4` and `priority=low`; never `jobs=8` with two slots and never delete the file (the PC crashed under two slots at `-j 8` on 2026-10-28).
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
  - Parallel tracks: one git worktree in `C:\Dev\Soscraft\mc-wt\<name>`, one branch and one target dir (`D:\iw4l-wt\<name>`) each, merged into `minecraft-survival` one at a time with the full suite after each merge.
  - Tracks run with the saved workflow `.claude/workflows/track-runner.js` (the `track-runner` workflow). Args: `{track, worktree, branch, target, base, briefs, groups:[{key,prompt}], avoid?, notes?, skipReview?}`.
    - It runs one agent per commit group in order, each continuing from what is already committed, then an adversarial review that fixes real defects in new commits, then clippy and the tests of the touched crates.
    - Each track keeps a log `docs/round<N>/track-<name>-log.md` with its commits, decisions, the owner's test steps and one combined kit line.
    - The round's hand-over state is in `docs/PROGRESS.md`.
  - Merging a track, one at a time:
    1. In the track's worktree: `git checkout --detach minecraft-survival`, then `git merge --no-ff <track>`.
    2. Run `scripts\full-suite.ps1 +target=<the track's target dir>` there, so the merged code is built in a dir that has only seen this worktree.
    3. In `C:\Dev\Soscraft\mashup-crafting`: `git merge --ff-only <merge commit>`, then push to `origin`.
  - Release builds only when the owner asks for one.
- **Release build:** the `release-build` skill (`.claude/skills/release-build/SKILL.md`) has the command, its timings, the backup and copy into the game folder, the relay and the protocol bump.
- **Test kit:** `IW4L_MINECRAFT_KIT=` in the `.env` next to the exe.
  - At most 27 stacks, because it only fills the 27 backpack slots.
  - Always give the kit line in its own code block so it isn't cut when copied.
- **Leave alone:** the untracked `blocks.txt` and `props.txt`, and the screenshot folders (`screenshots-*/`, `ac0c791/`, `scrennshots/`).
- **Tests:** the fork's test policy is the owner's. Don't trim tests because of upstream's `approved_tests` rule.
- **How the owner tests:** they play the release build and send numbered screenshots. Every change therefore ends with exact in-game test steps and the kit line they need.

## Reference repos (read-only, not integrated)

Cloned into `C:\Dev\Soscraft\references\`, outside this repo:

- **[chasmlol/skycraft](https://github.com/chasmlol/skycraft):** Skyrim x Minecraft.
  - A Fabric mod and an SKSE plugin talk over shared memory.
  - Multiplayer comes from vanilla Open to LAN plus the e4mc tunnel, and saving from vanilla autosave.
  - Licence: MIT.
- **[Funny-Bones/ELDEN-RING-Combat-Rewrite](https://github.com/Funny-Bones/ELDEN-RING-Combat-Rewrite):** Elden Ring player combat in Rust/Bevy 0.19.
  - Loads animations, TAE timings and params from the user's own game install.
  - It has player combat and a sparring dummy, no boss AI.
  - **No licence file:** read it, don't copy code.

## Where upstream docs are wrong for this fork

AGENT.md, CONTEXT.md, CONTRIBUTING.md and docs/ come from upstream and stay unedited so upstream merges stay clean. README.md is the fork's own Soscraft README and is kept current. For this fork:

- **Workflow**
  - There is no `make`, no `make mr`, no `context/` folder and no `externals/`. References live in `C:\Dev\Soscraft\references\`.
  - The branch is `minecraft-survival`, not `master`. Even upstream's default branch is `main`.
  - Ignore the "rustfmt the touched files", "squash onto master" and `make publish-check` landing and public-push recipes in CONTEXT.md and AGENT.md.
  - We keep small descriptive commits and push only to origin.
  - CONTRIBUTING.md's `cargo fmt --all` would reformat upstream files, so don't run it.
- **Tests and comments:** "Tests live only in `crates/approved_tests`" (CONTEXT.md) does not apply. The fork's comment rule is stricter than CONTEXT.md's: no comments at all.
- **README.md**
  - It is the Soscraft README: the goal, what works today, building, running, how friends join, the licences and the credits. Keep it short and true to the current state, and link docs/ for the details.
  - Soscraft is GPLv3 (`LICENSE`). The upstream parts stay Apache-2.0 (`LICENSE-APACHE` with `NOTICE`), and the licences in `third_party/` stay as they are.
- **Upstream's multiplayer docs are MW2 only:** `docs/DUO.md` and `docs/MASTER.md` cover MW2 matches only. The fork's Minecraft multiplayer, through the same relay, is `docs/MULTIPLAYER.md`, two games on one PC run with `scripts\mp-local.ps1`, and a friend's install is `docs/round8/friend-install.md`.
- **Build docs:** `docs/BUILD.md` is Linux-first. This fork builds natively on Windows with MSVC.
- **Vendored code:** `third_party/minecraftoss` is MinecraftOSS `4013a68`, and the fork has patched 73 of its files and added 37 (as of round 8). If upstream ever re-vendors a newer MinecraftOSS, diff our patches first.
