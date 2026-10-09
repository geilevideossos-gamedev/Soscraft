# Round 7 progress: finish the Minecraft side

Started 2026-10-09 on `minecraft-survival` at 3c2c62b. The owner's brief for this round is the "Big round: finish the Minecraft side" message; this file is the hand-over point if a session stops. Read it top to bottom, then `git log --oneline 3c2c62b..`, `git worktree list` and the track logs in `docs/round7/`.

## How to continue
- Every cargo command goes through `scripts\cargo-slot.ps1` (PowerShell), e.g. `& scripts\cargo-slot.ps1 test -p render_anim --locked`. It adds `-j 8`, refuses other job counts, and holds one of two machine-wide slots, so at most two cargo processes run at once. `--release`, `--workspace`, `--profile` and `+exclusive` take both slots, so a release build or the full suite runs alone. `+target=D:\dir` picks the target dir (default `D:\iw4l-target`).
- The full suite is `scripts\full-suite.ps1` (workspace tests without doc-tests, then the five minecraftoss crates with their stand-in scenario files).
- Track worktrees live in `C:\Dev\mc-wt\<name>` on branch `<name>`, each with its own target dir `D:\iw4l-wt\<name>`. Worktrees never share a target dir: cargo's freshness check would reuse another worktree's build of a crate whose files are older there.
- D: is short on space (one debug target dir is about 11 GB with the suite and clippy, 6-8 GB without). Delete a track's target dir once the track is merged.
- Merge tracks into `minecraft-survival` one at a time in `C:\Dev\mashup-crafting`, run the full suite after each merge, then push to `origin`.
- Each track agent keeps `docs/round7/track-<name>-log.md` with its commits, decisions and the owner's in-game test steps.
- The workflow scripts used by the orchestrator: a track runner (commit groups in order, then an adversarial review, then clippy). If a session dies, re-create the worktree state from `git worktree list` and continue the track from its log.

## Phase 1
| Item | State | Notes |
|---|---|---|
| 1. Death at a fixed height | done (0550ebb, 65f6402) | Cause: the Minecraft world loads on mp_rust as a proxy map. Its `trigger_hurt` (origin 304 704 -1952, hull 15) starts `_load.gsc` `hurtPlayersThink`, which makes a touching player `suicide()` every 0.5 s: feet at or below spawn Y - 24 (down to spawn Y - 86) within about 100 blocks of the spawn, so with the logged spawn at Y 119 every death came at Y 95 or lower. A second mp_rust `killTrigger` pocket (radius about 1.5 blocks, 4-7 blocks below spawn, 30 blocks east and 46 north) does the same. The suicide records the victim as attacker, hence "Killed By FNG" (the default title) with the player's own name. Fix: both are disarmed on Minecraft loads (trigger_hurt entities emptied in place, the killTrigger line removed from the proxy script); vanilla void damage below min Y - 64 (4 every 10 ticks, through creative, spectator, armour and totems, "fell out of the world"). |
| 2. Faster builds | done | rust-lld linker, `[profile.dev] debug = 0`, dependencies at opt-level 1 in dev and test, suite without doc-tests, `--profile play` for mid-round test exes. Times below. |
| 3. Upstream | done | `upstream/main` is f608f85, our merge base: nothing new to merge (checked 2026-10-09). |
| 4. Crate split | running | `docs/CRATE-SPLIT.md` (22 commits, phases A to D), worktree `C:\Dev\mc-wt\split`, branch `crate-split`, target `D:\iw4l-target`, log `docs/round7/track-split-log.md`. |
| 5. Nether and End report | done | `docs/NETHER-END.md` |

## Phase 2
Design briefs: `docs/round7/a1-grenade-keys.md`, `a2-chat.md`, `b1-throwables.md`, `b2-shulker-lid.md`, `b3-held-armour.md`, `c-commands.md`, `d-saving.md`.

| Track | Branch | State |
|---|---|---|
| A. Controls and chat | track-a | running (worktree `C:\Dev\mc-wt\track-a`, target `D:\iw4l-wt\track-a`) |
| B. Items, blocks, player model | track-b | not started; minecraft_terrain and MW2-body parts can run during the split, the render_anim glue after it |
| C. Commands | track-c | early part running (minecraft_commands, creative tab reader; worktree `C:\Dev\mc-wt\track-c`, target `D:\iw4l-wt\track-c`); chat UI and glue after track A and the split |
| D. Saving everything | track-d | after the split |

## Phase 3 (after track D)
| Track | State |
|---|---|
| Dimension switch | not started |
| Nether | not started |
| End | not started |

## Release builds
| # | When | Commit | State |
|---|---|---|---|
| 1 | tracks A-D merged (`--profile play`) | | |
| 2 | end of round (`--release`) | | |

## Build times
Measured on the owner's PC (i7-13700H, 32 GB), one build at a time, `-j 8`, each clean build in a fresh target dir, "touch" = changing the mtime of `render_anim/src/minecraft_world.rs`. Before: 29f722d with the old environment (`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`). After: 3a1c255.

| Measurement | Before | After |
|---|---|---|
| Clean release build (`-p launcher`, fat LTO) | 19.5 min | 16.6 min |
| Release after touching one file | 12.9 min | 12.7 min |
| Clean `--profile play` build | - | 10.1 min |
| `--profile play` after touching one file | - | 0.1 min |
| Clean dev build (`-p launcher`) | 2.2 min | 6.6 min (dependencies now optimised, once per target dir) |
| Dev build after touching one file | 0.4 min | 0.2 min |
| Building the workspace tests after that | 0.5 min | 1.0 min |
| Running the workspace tests | 3.4 min (with 61 empty doc-test runs) | 1.5 min (`--tests`) |
| Slowest test binary (minecraft_terrain) | 77 s | 56 s |
| Clippy (workspace, all targets) | 1.3 min | 2.0 min |

The 41 minutes of the last round were not reproduced: a clean release build alone took 19.5 min. The slow part of the test run was five server tests that generate terrain with unoptimised dependencies (33 s for the slowest, 9 s now).

## Decisions
- Build: the owner's mid-round test exe is built with the existing `--profile play` (no fat LTO, same code); the last build of the round is `--release`. Tests never used the release profile, so no extra test profile was needed.
- Build: dev and test builds are not incremental (`70a01b2`): incremental caches took 8-11 GB per target dir on a disk with 9 GB left. `C:\Dev\mc-wt\.cargo\config.toml` sets the same for worktrees made before that commit. `D:\iw4l-target` itself is protected from deletion by a permission rule; only its `debug\incremental` was removed.
- Build: the full suite runs at merge time in the main target dir, not in each track's dir, to save disk; clippy runs per track.
- Crate split: a `git mv` rename counts as an existing file (not reformatted, existing comments kept). Survival stats keep carrying over between matches as today until track D saves them. `Entities` moves whole.
- Grenade keys (track A): Ctrl+G and Shift+G clash (the bind system has no chords; Ctrl is MW2 prone and the creative-flight sink key, Shift is sprint). Lethal goes to G and tactical to B, F only swaps hands as in vanilla, and holding F with a gun selected uses the offhand item. Retail MW2 binds G to the lethal too. All rebindable.
- Shulker lids (track B): chests and ender chests get the same vanilla lid animation as shulker boxes, as vanilla has it.

## Log
- 2026-10-09: item 1 merged and pushed (65f6402). Crate split, track A and early track C started.
- 2026-10-09: plan written, `scripts/cargo-slot.ps1` added. Upstream checked (nothing new). Research for items 1, 4, 5 and the Phase 2 briefs done. Build changes measured and committed.
