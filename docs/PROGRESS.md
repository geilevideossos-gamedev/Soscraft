# Round 8 progress: fixes, leftovers, skating and the first playable multiplayer

Started 2026-10-10 on `minecraft-survival` at e2313625. The owner's brief is the "Round 8" message (test results of the final round 7 build e231362, screenshots in `scrennshots/screenshots-e231362/screenshots-e231362/`). This file is the hand-over point if a session stops. Read it top to bottom, then `git log --oneline e2313625..`, `git worktree list` and the track logs in `docs/round8/`. Round 7's hand-over is in git history (`git show e2313625:docs/PROGRESS.md`).

## How to continue
- Every cargo command goes through `scripts\cargo-slot.ps1` (PowerShell), e.g. `& scripts\cargo-slot.ps1 test -p render_anim --locked`. It adds `-j 8`, refuses other job counts, and holds one of two machine-wide slots. `--release`, `--workspace`, `--profile` and `+exclusive` take both slots. `+target=D:\dir` picks the target dir (default `D:\iw4l-target`).
- The full suite is `scripts\full-suite.ps1`.
- Track worktrees live in `C:\Dev\mc-wt\<name>` on branch `<name>`, each with its own target dir `D:\iw4l-wt\<name>`. Worktrees never share a target dir.
- Merging a track: in the track's worktree `git checkout --detach minecraft-survival` and `git merge --no-ff <track>`, run `scripts\full-suite.ps1 +target=<the track's target dir>` there, then `git merge --ff-only <merge commit>` in `C:\Dev\mashup-crafting` and push to `origin`. Delete the track's target dir once merged.
- Tracks run with the saved workflow `.claude/workflows/track-runner.js` (args now take `round: 8`; logs go to `docs/round8/track-<name>-log.md`).
- Owner test steps and kit lines go into files under `docs/round8/` (the owner copies from files, not the terminal).

## Owner's test of e231362
Working: everything in `docs/round7/release-2-tests.md` up to and including the dragon kill.
Broken (screenshot): 1 end gateway does not teleport, 2 eye of ender is not thrown, 3 Efficiency V diamond pickaxe does not insta-mine netherrack, 4 spawners show no mob inside, 5 after `random_tick_speed 20000` and back the world froze (items hang in the air); `/tick` missing.

## Phase 1: release build time
| Item | State | Notes |
|---|---|---|
| 1. Why the last release build took 49 min | done | Machine: i7-13700H laptop (6 P-cores + 8 E-cores, 20 threads), 32 GB, both drives NVMe, Balanced plan. Release after touching `minecraft_net` (12 crates rebuild), measured with `--timings` and a memory/paging sampler: **53 min** with the old fat LTO; the 12 crates took 4.5 min, the single-threaded fat-LTO link of `launcher` (one rustc at ~8 GB) the rest. Paging was low, free disk did not change, so neither swapping nor disk space was the cause; Defender could not be inspected without admin. Cause 1: Windows power throttling (EcoQoS) moves background processes onto the E-cores while another window (a game) is in front; `cargo-slot.ps1` now keeps cargo, rustc and the linker off it (0bcbfb17): **30 min**, link still 26 min. Cause 2: fat LTO is one thread, so anything else running slows it. Fix: `[profile.release]` is `lto = "thin"`, `codegen-units = 16`: **21 min clean, 13.5 min after touching `minecraft_net`** (link 11 min). The exe grows from 122 to 150 MB; upstream measured fat LTO at 5.7 % better p50 frame time than no cross-crate LTO, ThinLTO sits between. The old profile stays as `--profile fat`. |

## Phase 2: tracks
Briefs: `docs/round8/<item>.md`, the multiplayer design `docs/MULTIPLAYER.md`.

| Track | Branch | State |
|---|---|---|
| A. Fixes from the test (gateway, eye of ender, mining speed, spawner mob, overload and /tick) | track-a (`C:\Dev\mc-wt\track-a`, `D:\iw4l-wt\track-a`) | running from 0bcbfb17 (order a5, a2, a3, a4, a1) |
| B. Nether and overworld leftovers (piglins, brutes, bartering, hoglins, striders, sleeping) | track-b | running from 0bcbfb17 (order b3, b2, b1, b4) |
| C. End and visual leftovers (shulkers, end cities, item frames, End sky, dragon effects, elytra body, limitations) | track-c | running from 0bcbfb17 (order c1 G1-3, c5 G1-4, c1 G4-8, c2, c3, c5 G5-11, c4) |
| D. Skating in Minecraft worlds | track-d | waits for disk space (starts when a track merges) |
| E. Multiplayer, first playable | track-e | M1 (E1-E8) done 42262ec2..79ba1549 (review fixed 2 defects, clippy clean; owner steps `docs/round8/e-m1-tests.md`); M2 part 1 (E9 da5b8685, E10a cb4251e9, E10b 4a6c0dc7) done, verified (owner steps `docs/round8/e-m2-tests.md`); the review of all of M2 runs with part 2; E11-E15 run after track A merges and minecraft-survival is merged into track-e (E11 needs A's tick loop) |

## Release builds
| # | When | Commit | State |
|---|---|---|---|
| 1 | tracks A-D merged | - | - |
| 2 | end of round | - | - |

## Decisions
- Gentle builds (2026-10-11, the owner's PC was at 100 % CPU with 2 GB of commit left while gaming): `%LOCALAPPDATA%\iw4l-cargo-slot.txt` holds `slots=1`, `jobs=4`, `priority=low`, so one cargo runs at a time with 4 jobs below normal priority. Delete the file (or set `slots=2`, `jobs=8`) for full speed, e.g. for a release build when the owner is away. The wrapper change is committed on every track branch too.
- Release builds use ThinLTO (faster, robust while the PC is in use); `--profile fat` keeps the old fat-LTO build for an A/B if the owner sees lower frame rates.
- Hit keys: 20 is the dragon; track B uses 21-25, track C 26-28.
- Track D starts after the first merge: four debug target dirs fit on D:, five do not.
- The multiplayer design names the branch `track-e8`; it is `track-e`.

## Log
- 2026-10-11: M2 part 1 done. D: fell to 14 GB free; `D:\iw4l-targetelease` deleted (the first release build of the round starts clean, about 21 min).
- 2026-10-11: track E milestone M1 finished; M2 part 1 started.
- 2026-10-11: briefs and the multiplayer design committed (fe3e2ba9); tracks A, B, C and E (M1) started.
- 2026-10-10: round 8 started; screenshots read; release build measurement and the brief workflow started.
