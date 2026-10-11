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
| 1. Why the last release build took 49 min | investigating | Machine: 20 threads, 32 GB, both drives NVMe. At the start only ~10 GB RAM was free (a game, browser and Discord open), C: has 7.5 GB free (TEMP lives there), Defender real-time scanning is on (exclusions need admin). Measuring with `--timings` and a memory/paging sampler. |

## Phase 2: tracks
Briefs: `docs/round8/<item>.md`, the multiplayer design `docs/MULTIPLAYER.md`.

| Track | Branch | State |
|---|---|---|
| A. Fixes from the test (gateway, eye of ender, mining speed, spawner mob, overload and /tick) | track-a8 | briefs being written |
| B. Nether and overworld leftovers (piglins, brutes, bartering, hoglins, striders, sleeping) | track-b8 | briefs being written |
| C. End and visual leftovers (shulkers, end cities, item frames, End sky, dragon effects, elytra body, limitations) | track-c8 | briefs being written |
| D. Skating in Minecraft worlds | track-d8 | brief being written |
| E. Multiplayer, first playable | track-e8 | design being written |

## Release builds
| # | When | Commit | State |
|---|---|---|---|
| 1 | tracks A-D merged | - | - |
| 2 | end of round | - | - |

## Decisions

## Log
- 2026-10-10: round 8 started; screenshots read; release build measurement and the brief workflow started.
