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
| A. Fixes from the test (gateway, eye of ender, mining speed, spawner mob, overload and /tick) | track-a | **merged** 080b5c1e (full suite green, 8.5 min); review fixed 1 defect (eye of ender tests timing out under load); trial spawner mob left for "Finish survival"; owner steps and kit in `docs/round8/track-a-log.md` |
| B. Nether and overworld leftovers (piglins, brutes, bartering, hoglins, striders, sleeping) | track-b | **merged** 75dd9d5e (25 conflict hunks with A and D resolved; full suite green, 7.6 min), plus 6ae244dc (no board while riding or asleep, D's open item) and 23798eab (B's mobs spin in spawners); review fixed 3 defects (rustfmt on new files, teleport wakes a sleeper, the ridden strider can't be hit or fed); owner steps and kit in `docs/round8/track-b-log.md` |
| C. End and visual leftovers (shulkers, end cities, item frames, End sky, dragon effects, elytra body, limitations) | track-c | **merged** f294bfe5 (about 124 conflict hunks with A, B and D resolved), plus a1e689b5 (B's mobs float with levitation), 4392dc37 (shulkers in spawners), 57b42ea1 (no board in powder snow), d87b9def (rail and powder snow bits checked); full suite green apart from the known flaky `background_full_update_matches_the_synchronous_one`, which passed alone; review fixed 2 defects (rustfmt on a new file, levitation in powder snow and cobwebs); owner steps and kit in `docs/round8/track-c-log.md` |
| D. Skating in Minecraft worlds | track-d | **merged** 9d4fbe3a (full suite green, 9.9 min); review fixed 1 defect (a bail in the air counted as a landing); guns while skating: put away, as the brief decides; the board is refused while riding or asleep since the B merge (6ae244dc); owner steps and kit in `docs/round8/track-d-log.md` |
| E. Multiplayer, first playable | track-e | M1 (E1-E8) done 42262ec2..79ba1549 (review fixed 2 defects, clippy clean; owner steps `docs/round8/e-m1-tests.md`); M2 part 1 (E9 da5b8685, E10a cb4251e9, E10b 4a6c0dc7) done, verified (owner steps `docs/round8/e-m2-tests.md`); M2 done (track A merged into track-e as 8230328a; E11 3caf3ef0, E12-E15 up to 6457abca; review fixed 2 defects: furnace progress made friends' clicks stale, a friend leaving left a chest lid open; clippy clean 03f1bc8d; owner steps `docs/round8/e-m2-tests.md`). Running: the merge of minecraft-survival (B, C, D; 72 conflict hunks) into track-e, then M3 (E16-E17) and M4 (E18), from 03f1bc8d |

## Release builds
| # | When | Commit | State |
|---|---|---|---|
| 1 | tracks A-D merged | 4d45e4fe | done 2026-10-12: 16.4 min clean in gentle mode; copied in as `iw4l-survival.exe`, old one kept as `iw4l-survival-e231362.exe`; test steps `docs/round8/release-1-tests.md` |
| 2 | end of round | - | - |

## Decisions
- Gentle builds (2026-10-11, the owner's PC was at 100 % CPU with 2 GB of commit left while gaming): `%LOCALAPPDATA%\iw4l-cargo-slot.txt` holds `slots=1`, `jobs=4`, `priority=low`, so one cargo runs at a time with 4 jobs below normal priority. Delete the file (or set `slots=2`, `jobs=8`) for full speed, e.g. for a release build when the owner is away. The wrapper change is committed on every track branch too.
- Release builds use ThinLTO (faster, robust while the PC is in use); `--profile fat` keeps the old fat-LTO build for an A/B if the owner sees lower frame rates.
- Hit keys: 20 is the dragon; track B uses 21-25, track C 26-28.
- Track D starts after the first merge: four debug target dirs fit on D:, five do not.
- The multiplayer design names the branch `track-e8`; it is `track-e`.
- Open, not vanilla: under `/tick freeze` a strider carrying a player stands still; vanilla 26.3 (`TickRateManager.isEntityFrozen`) keeps an entity carrying a player ticking. The mob tick runs for all mobs or none on both sides, so matching needs a filtered tick. Left for "Finish survival".

## Log
- 2026-10-12: track C merged (4d45e4fe); release build 1 done; M3 and M4 started.
- 2026-10-12: track C finished, its merge (about 124 conflict hunks with A, B and D) is being resolved in `C:\Dev\mc-wt\track-c`; multiplayer M2 finished.
- 2026-10-12: track B merged (23798eab).
- 2026-10-12: track D merged (9d4fbe3a).
- 2026-10-12: track A merged (080b5c1e); track D and M2 part 2 started.
- 2026-10-11: M2 part 1 done. D: fell to 14 GB free; `D:\iw4l-target\release` deleted (the first release build of the round starts clean, about 21 min).
- 2026-10-11: track E milestone M1 finished; M2 part 1 started.
- 2026-10-11: briefs and the multiplayer design committed (fe3e2ba9); tracks A, B, C and E (M1) started.
- 2026-10-10: round 8 started; screenshots read; release build measurement and the brief workflow started.
