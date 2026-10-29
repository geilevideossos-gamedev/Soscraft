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
- Every cargo command goes through `scripts\cargo-slot.ps1` (PowerShell). Since the crash on 2026-10-28 the slot file (`%LOCALAPPDATA%\iw4l-cargo-slot.txt`) is `slots=2`, `jobs=4`, `priority=low`, and at most four track runs go at once (decisions: "Lower load after the crash"). Never go back to `jobs=8` with two slots. Gentle (`slots=1`, `jobs=4`, `priority=low`) when the owner says they play.
- The full suite is `scripts\full-suite.ps1`.
- Track worktrees live in `C:\Dev\mc-wt\<name>` on branch `<name>`, each with its own target dir (`D:\iw4l-wt\<name>`, or `C:\iw4l-wt\<name>` when D: is full). Worktrees never share a target dir.
- Merging a track: in the track's worktree `git checkout --detach minecraft-survival` and `git merge --no-ff <track>`, run `scripts\full-suite.ps1 +target=<the track's target dir>` there, then `git merge --ff-only <merge commit>` in `C:\Dev\mashup-crafting` and push to `origin`. Delete the track's target dir once the track is finished.
- Tracks run with the saved workflow `.claude/workflows/track-runner.js` with `round: 10`; logs go to `docs/round10/track-<name>-log.md`. **Every run's exact args are saved in `docs/round10/runs/<segment>.json` before it starts** (round 9 lost its launcher scripts with the session scratchpad). To relaunch a stopped run, pass that file's content as the workflow args, minus the groups already committed.
- If a session stops mid-run: in the worktree read the track log and `git status`, commit or discard a half-done group, then start the same segment again with only the groups not yet committed.
- Owner test steps and kit lines go into files under `docs/round10/`.

## Where it stands
- 2026-10-28: the track-d worktree was fine (links already pointed at `C:\Dev\mc-wt\track-d`; `git worktree repair` found nothing). The track runner was already saved. track-d merged as 6e0e0573 (full suite green, 24.9 min), pushed; release 1 built and copied in. The track-d branch is fast-forwarded to 6e0e0573 and its target dir trimmed of test binaries.
- Briefs written by workflows: E (ed23b000), D and W (382bf959), F and the Elden Ring design doc (c9253dba). D0 (aa942451). M1-M3 (2026-10-29, after the crash) follow D0's design.
- Each track runs in segments of a few groups, one track-runner run each, with the segment's base tagged `r10-<segment>-base` before it starts. The specs are in `docs/round10/runs/`.
- A watchdog (a background loop) alerts when D: or C: falls below 8 GB.
- 2026-10-28 20:27: the PC crashed with d0a, d1, e1a, e2a, f1 and w1 running and the M1-M3 briefs half researched (nothing of them saved). All six segments were in their first group (d0a had its mob list commit 17faa07d, d1 one review commit bad8dd84) and the other worktrees hold their first group's half-done work uncommitted. Every run spec got a RESTARTS note (finish the half-done group from `git status`, clean a corrupt crate). Relaunched at the lower load: d0a, d1, e1a and e2a first; w1 and f1 wait for a free place; the M1-M3 briefs are written again by writers that never build.

## Tracks
| Track | Branch | Brief | Segments | State |
|---|---|---|---|---|
| D0. Shared mob wiring (mob list, spawn tables, render registry) | track-d0 | `docs/round10/track-d0.md` | d0a (registry-core ... silverfish-drawn: the whole registry with silverfish and endermites as its proof), d0b (silverfish-behaviour, flying-navigation) | d0a merged 2026-10-29 as 39654c55 (18 commits; full suite green, 43.5 min, 3917 tests); d0b waits for a free place. D, W, F and M1-M3 merge `minecraft-survival` in before their mob groups |
| D. Trial spawners, vaults, lodestones, breezes | track-d | `docs/round10/track-d.md` | d1 (d5-review ... lodestones, review base `r9-d5-base` so the round 9 d5 commits are reviewed too), d2 (breezes, breeze-render; after D0) | d1 merged 2026-10-29 as 1a893d11 (20 commits; 14 conflict hunks with D0 and E1, both sides kept, decisions; full suite 4080 passed, only the flaky culling test failed and passed alone, now fixed in 274c6dd9); d2 waits for a free place (the track-d branch is fast-forwarded to 605ad21e) |
| W. Darkness, vibrations, sculk, warden | track-w | `docs/round10/track-w.md` | w1 (darkness ... shriekers-catalysts), w2 (warden groups; after D0) | w1 running since 2026-10-29 04:00 (restarted with its half-done darkness group) |
| M1. Land animals (group `land`, bytes 48-63): rabbit, fox, goat, parrot, panda, turtle, armadillo, polar bear, sniffer, bee | track-m1 | `docs/round10/track-m1.md` | m1a-m1i (44 groups; m1h parrots and m1i bees need d0b) | brief written 2026-10-29; m1a running since 2026-10-29 06:20 (worktree from df7258b4) |
| M2. Water and buckets (group `water`, 64-79): P2 water movement, squids, fish, dolphin, axolotl, frog, tadpole, guardians, nautiluses, sulfur cube | track-m2 | `docs/round10/track-m2.md` | m2a-m2i (40 groups; most wait for M1 hooks, decisions) | brief written 2026-10-29; waits for d0a's merge (order M1, M3, M2) |
| M3. Night, built and horses (group `night`, 80-95; horse kinds in the cow store): phantom, P8 lightning, undead horses, camels, allay, copper golem, creaking, happy ghast | track-m3 | `docs/round10/track-m3.md` | m3a-m3g (32 groups; m3f needs d0b, m3g needs W w2) | brief written 2026-10-29; waits for d0a's merge (order M1, M3, M2) |
| E1. Known limits: screens, visuals, sounds | track-e1 | `docs/round10/track-e.md` | e1a (kit-components ... dye-icons), e1b (horse-dye ... bells-outline) | e1a merged 2026-10-29 as 008a5c27 (20 commits; one conflict in the vendored `emissions.rs`, both sides kept; full suite green, 46.3 min, 3990 tests); e1b waits for a free place |
| E2. Known limits: gameplay and loot | track-e2 | `docs/round10/track-e.md` | e2a (ti1 ... fw-launch), e2b (fw-shapes ... ti4-end-targets) | e2a restarted after the crash (2026-10-28 evening) |
| F. First Elden Ring boss: Margit, the Fell Omen | track-f (target `C:\iw4l-wt\track-f`) | `docs/ELDEN-RING-BOSSES.md`, `docs/round10/track-f.md` | f1 (er-core, er-rules: the pure `elden_boss` crate), f2 (er-entity, er-arena; after D0), f3 (er-look, er-fight, er-guns, er-bar-friend, er-altar) | f1 done 2026-10-29 (7 commits, `elden_boss` with 53 tests; not merged alone, merges with f2); track-f took `minecraft-survival` in (cea44a86); f2 running since 06:20 with its target on `D:\iw4l-wt\track-f` |

## Release builds
| # | When | Commit | State |
|---|---|---|---|
| 2 | d0a, e1a and d1 merged, protocol 7 | 605ad21e | done 2026-10-29: release 11m 22s (`-p launcher -p iw4l-master`); copied in as `iw4l-survival.exe`, release 1 kept as `iw4l-survival-6e0e057.exe`, relay copied in (hashes checked); test steps `docs/round10/release-2-tests.md` (78 screenshots, 9 kits, written and checked by two agents) |
| 1 | track-d merged (d6, keyframes, withers, wind charges) | 6e0e0573 | done 2026-10-28: full suite green (24.9 min, 3863 tests); release 12m 57s (`-p launcher -p iw4l-master`); copied in as `iw4l-survival.exe`, old one kept as `iw4l-survival-f743218.exe`, relay copied (same hash); protocol 6; test steps `docs/round10/release-1-tests.md` |

## Log
- 2026-10-29: d1 merged (1a893d11), the flaky culling test made to wait by time (274c6dd9), PROTOCOL 7 (605ad21e), release 2 building; f1 finished (merges with f2), f2 and m1a started; F's target moved to D:.
- 2026-10-29: d0a merged (39654c55) and e1a merged (008a5c27), both pushed; w1 and f1 started in their places; test binaries deleted from both merged targets (D: 18 GB free). Release 2 follows the d1 and e2a merges.
- 2026-10-29: M1-M3 briefs and run specs written and reviewed (32 agents, at most four at once, no cargo); cross-track hook owners and the M run order in decisions.
- 2026-10-28: the PC crashed at 20:27 under six tracks at `-j 8`; load lowered to two slots at `-j 4` with four track runs at once; d0a, d1, e1a and e2a restarted.
- 2026-10-28: `docs/ELDEN-RING-BOSSES.md` written (first boss Margit, the Fell Omen, with a stand-in model); track F (f1) started.
- 2026-10-28: tracks E1 (e1a), E2 (e2a), D (d1) and W (w1) started; disk watchdog on.
- 2026-10-28: release 1 (6e0e0573) built and copied in; test steps `docs/round10/release-1-tests.md`.
- 2026-10-28: round 10 started; track-d merged in its worktree (6e0e0573), full suite running; briefs started.
