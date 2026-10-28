# Round 10 progress: survival in parallel tracks, known limits, the first Elden Ring boss

Started 2026-10-28 on `minecraft-survival` at 532474d6. The owner's brief is the "Round 10" message (summary below). This file is the hand-over point if a session stops. Read it top to bottom, then `git log --oneline 532474d6..`, `git worktree list`, the run specs in `docs/round10/runs/` and the track logs in `docs/round10/`. Round 9's hand-over is in git history (`git show 532474d6:docs/PROGRESS.md`). Decisions go to `docs/round10/decisions.md`.

## The owner's brief
- Plenty of usage this week: run as many tracks in parallel as the PC allows; the cargo-slot rules still decide how many builds run at once.
- Step 0: the folders were moved back from `C:\Dev\soscraft` (a leftover, ignored); check the track-d worktree; run the full suite on track-d (28 commits: d6 friend fixes and part of d5: shared keyframes, withers, wind charges), merge it, release build at once; make sure `iw4l-master.exe` matches the multiplayer protocol; the track runner saved in `.claude/workflows/`.
- Track D: finish track D: the rest of d5 (trial chambers and trial spawners, lodestones), d3 (deep dark, warden, sculk) and d4 (the remaining mobs) split into two or three tracks by mob group. Land the shared changes (mob list, spawn tables, render registry) first so the later merges stay small.
- Track E: the known limits of `docs/round9/release-2-tests.md`: particles (taming hearts, bucking smoke, trust hearts, raid sweat, llama spit), banner patterns and the crossbow pull with its arrow, vindicators and zombies breaking and opening doors, swinging bells, fireworks bursting on mobs with knockback, thrown items hitting the new mobs, the trading screen's keys and drags and effects beside the inventory, horse fall damage, the gallop sound, vehicle hearts and dyed leather horse armour, punching off lead knots and cats keeping chests shut, phantoms, shipwreck and ocean ruin loot with explorer maps.
- Track F: Elden Ring bosses, first step: read the reference, write `docs/ELDEN-RING-BOSSES.md` (fit with the Minecraft world, MW2 guns, multiplayer and saving; which boss first; assets; boss bar; phases; summoning or finding it), then build that boss as a playable proof of concept in a Minecraft world.
- Release builds: one right after the track-d merge, then at least one every day. Each time back up the old exe, copy the new exe and `iw4l-master.exe` into the game folder, push, and write test steps and kits into `docs/round10/`. At the end update CLAUDE.md and `docs/MINECRAFT-SURVIVAL.md`.
- Usual rules: this file current, decisions in `docs/round10/decisions.md`, no questions to the owner, the CLAUDE.md workflow. Gentle mode when the owner says they are playing.

## How to continue
- Every cargo command goes through `scripts\cargo-slot.ps1` (PowerShell). Full speed (`slots=2`, `jobs=8`, `priority=normal` in `%LOCALAPPDATA%\iw4l-cargo-slot.txt`) since the start of round 10; gentle (`slots=1`, `jobs=4`, `priority=low`) when the owner says they play.
- The full suite is `scripts\full-suite.ps1`.
- Track worktrees live in `C:\Dev\mc-wt\<name>` on branch `<name>`, each with its own target dir (`D:\iw4l-wt\<name>`, or `C:\iw4l-wt\<name>` when D: is full). Worktrees never share a target dir.
- Merging a track: in the track's worktree `git checkout --detach minecraft-survival` and `git merge --no-ff <track>`, run `scripts\full-suite.ps1 +target=<the track's target dir>` there, then `git merge --ff-only <merge commit>` in `C:\Dev\mashup-crafting` and push to `origin`. Delete the track's target dir once the track is finished.
- Tracks run with the saved workflow `.claude/workflows/track-runner.js` with `round: 10`; logs go to `docs/round10/track-<name>-log.md`. **Every run's exact args are saved in `docs/round10/runs/<segment>.json` before it starts** (round 9 lost its launcher scripts with the session scratchpad). To relaunch a stopped run, pass that file's content as the workflow args, minus the groups already committed.
- If a session stops mid-run: in the worktree read the track log and `git status`, commit or discard a half-done group, then start the same segment again with only the groups not yet committed.
- Owner test steps and kit lines go into files under `docs/round10/`.

## Where it stands
- 2026-10-28: the track-d worktree was fine (links already pointed at `C:\Dev\mc-wt\track-d`; `git worktree repair` found nothing). The track runner was already saved. track-d merged as 6e0e0573 (full suite green, 24.9 min), pushed; release 1 built and copied in. The track-d branch is fast-forwarded to 6e0e0573 and its target dir trimmed of test binaries.
- Briefs being written by four workflows: D0 (shared mob registry), E (known limits, split E1/E2), F (Elden Ring design doc and proof of concept), D and W (refreshed from round 9).

## Tracks
| Track | Branch | Brief | State |
|---|---|---|---|
| D0. Shared mob wiring (mob list, spawn tables, render registry) | track-d0 | `docs/round10/track-d0.md` | brief |
| D. Trial spawners, vaults, lodestones, breezes | track-d | `docs/round10/track-d.md` | brief |
| W. Darkness, vibrations, sculk, warden | track-w | `docs/round10/track-w.md` | brief |
| M1-M3. The remaining mobs | track-m1..m3 | after D0's design | waiting for D0 |
| E1/E2. Known limits | track-e1, track-e2 | `docs/round10/track-e.md` | brief |
| F. First Elden Ring boss | track-f | `docs/ELDEN-RING-BOSSES.md`, `docs/round10/track-f.md` | brief |

## Release builds
| # | When | Commit | State |
|---|---|---|---|
| 1 | track-d merged (d6, keyframes, withers, wind charges) | 6e0e0573 | done 2026-10-28: full suite green (24.9 min, 3863 tests); release 12m 57s (`-p launcher -p iw4l-master`); copied in as `iw4l-survival.exe`, old one kept as `iw4l-survival-f743218.exe`, relay copied (same hash); protocol 6; test steps `docs/round10/release-1-tests.md` |

## Log
- 2026-10-28: release 1 (6e0e0573) built and copied in; test steps `docs/round10/release-1-tests.md`.
- 2026-10-28: round 10 started; track-d merged in its worktree (6e0e0573), full suite running; briefs started.
