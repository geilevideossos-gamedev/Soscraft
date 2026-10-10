# Round 7 progress: finish the Minecraft side

Started 2026-10-09 on `minecraft-survival` at 3c2c62b. The owner's brief for this round is the "Big round: finish the Minecraft side" message; this file is the hand-over point if a session stops. Read it top to bottom, then `git log --oneline 3c2c62b..`, `git worktree list` and the track logs in `docs/round7/`.

## How to continue
- Every cargo command goes through `scripts\cargo-slot.ps1` (PowerShell), e.g. `& scripts\cargo-slot.ps1 test -p render_anim --locked`. It adds `-j 8`, refuses other job counts, and holds one of two machine-wide slots, so at most two cargo processes run at once. `--release`, `--workspace`, `--profile` and `+exclusive` take both slots, so a release build or the full suite runs alone. `+target=D:\dir` picks the target dir (default `D:\iw4l-target`).
- The full suite is `scripts\full-suite.ps1` (workspace tests without doc-tests, then the five minecraftoss crates with their stand-in scenario files).
- Track worktrees live in `C:\Dev\mc-wt\<name>` on branch `<name>`, each with its own target dir `D:\iw4l-wt\<name>`. Worktrees never share a target dir: cargo's freshness check would reuse another worktree's build of a crate whose files are older there.
- D: is short on space (one debug target dir is about 11 GB with the suite and clippy, 6-8 GB without). Delete a track's target dir once the track is merged.
- Merging a track: in the track's worktree run `git checkout --detach minecraft-survival` and `git merge --no-ff <track>`, run `scripts\full-suite.ps1 +target=<the track's target dir>` there (so the merged code is built in a dir that has only seen this worktree), then `git merge --ff-only <merge commit>` in `C:\Dev\mashup-crafting` and push to `origin`.
- PowerShell drops a bare `--` when calling a script, so pass it quoted: `& scripts\cargo-slot.ps1 clippy -p x --locked '--' -A clippy::approx_constant`.
- Each track agent keeps `docs/round7/track-<name>-log.md` with its commits, decisions and the owner's in-game test steps.
- The workflow scripts used by the orchestrator: a track runner (commit groups in order, then an adversarial review, then clippy). If a session dies, re-create the worktree state from `git worktree list` and continue the track from its log.

## Where everything stands (stopped 2026-10-09 for a PC shutdown)

All agents and builds were stopped on the owner's request. Every branch is committed and pushed to `origin`. Nothing is running. Resume from this section.

- `minecraft-survival` (main, `C:\Dev\mashup-crafting`): Phase 1 done, track A and the crate split merged. Last code merge 6315039.
- **Track B** (`track-b`, worktree `C:\Dev\mc-wt\track-b`, target `D:\iw4l-wt\track-b`), head c1c9c4b:
  - Early half done and reviewed (throwables, lids and body gear behind APIs, up to ba18264 plus review fixes). Main merged in as 984bd51.
  - Second half, group `throw-glue`, was in progress: done 4749db4, 243bc13, cc5a400, 643656a, e1228a3, c1c9c4b (throw sources, pearls per thrower, crumbs and specks, swirl and glass, potion colours in flight, cloud specks).
  - Next: finish `throw-glue` (potion effects on the player from splashes, clouds and witch potions; trident hold-and-release, the 3D trident in hand and in flight). Then group `lids-glue` (log items 15-17: terrain.rs hook, per-frame open-lid draw, opener block events), then `body-glue` (publish held items and armour per player to the MW2 body, death cam, other players, inventory character box). Then review and clippy, then merge.
  - Restart with the track runner (see "How to continue") with `base` 984bd51 and only the unfinished groups; the agent reads `docs/round7/track-b-log.md` ("After the split" list) to see what is done.
- **Track C** (`track-c`, worktree `C:\Dev\mc-wt\track-c`, target `D:\iw4l-wt\track-c`), head 2a3d60e:
  - Early half done and reviewed (suggestion engine, /clear /xp /enchant /setblock /fill /spawnpoint /locate /execute /kit, creative tab reader). Main merged in as 44759b7.
  - Second half, group `glue`, is finished: eefc73b..2a3d60e (commands act on the named player, /clear, /xp, /enchant, /setblock and /fill and /locate on the level server thread, /kit re-reads .env, /execute in looks up loaded levels, command hints kept up to date, and track A's leftover: hands stay still while an MW2 grenade is up). /spawnpoint glue waits for track D (`Player.respawn`).
  - Next: group `chat-ui` (suggestion list, grey inline text, Tab cycling, usage lines and input colours in `hud/src/minecraft_chat.rs`), then `creative` (creative inventory screen with the tabs, search, saved hotbars), then review and clippy, then merge after track B or D, whichever is first.
- **Track D** (`track-d`, worktree `C:\Dev\mc-wt\track-d`, target `D:\iw4l-wt\track-d`), head b681b6d:
  - Groups `world-basics` (33a04dd, a8ed2a4, 2c5e8d3) and `player-file` (382dcc5, ce55298, 1397259, f484535, 4bffa75, f025a7a) are done.
  - Group `block-entities` was in progress: done 50f39ce (block entities saved with their chunk) and f6822d8 (chests, barrels, shulker boxes). b681b6d is a WIP commit of the furnace and campfire step (brief commit 12): it touches `minecraft_terrain/src/campfire.rs` and the vendored `minecraftoss/player/src/crafting.rs`, is unfinished and was not built or tested. Check it, finish it or reset it before going on.
  - Next: finish brief commit 12 (furnaces, smokers, blast furnaces, brewing stands, campfires keep their progress), commit 13 (hoppers, dispensers, droppers), then group `compat` (brief commit 14: end-to-end save test, Saving section of the docs, owner test), then review and clippy, then merge. The phase 3 plan also asks track D to add the `PortalCooldown` and `seenCredits` player keys.
- **Merge order:** D first (phase 3 waits for it), then B and C. Expect conflicts in `render_anim/src/minecraft_world.rs` and `minecraft_survival/src/{entities,player,commands}.rs` between B, C and D. Merge each in its own worktree as described in "How to continue", full suite there, then fast-forward main and push.
- **After A-D are merged:** release build 1 with `--profile play` (back up the current `iw4l-survival.exe` as `iw4l-survival-3c2c62b.exe`), then phase 3 per `docs/round7/e0-phase3-plan.md`, starting with the dimension switch groups S1-S4.
- **Disk:** after the restart D: had 7.8 GB free; deleting the stale `D:\iw4l-target\debug` cache (25.4 GB, rebuilt on demand) and the old incremental caches in the track dirs left 32.6 GB free. The merged branches `track-a` and `crate-split` were deleted locally. The target dirs are `D:\iw4l-target` (main; protected from deletion, only subfolders may be removed), `D:\iw4l-wt\track-b`, `track-c` and `track-d`. Delete a track's dir after it is merged.
- **Workflow script:** the track runner is saved as `.claude/workflows/track-runner.js` (run it as the workflow `track-runner` or by that path). Args: `{track, worktree, branch, target, base, briefs, groups:[{key,prompt}], avoid, notes, skipReview}`. Per track it runs one agent per commit group in order (each continues from what is already committed), then an adversarial review agent that fixes real defects in new commits, then a verify agent (clippy and tests of the touched crates, combined kit line in the track log). The full suite runs at merge time, not in the runner.
## Phase 1
| Item | State | Notes |
|---|---|---|
| 1. Death at a fixed height | done (0550ebb, 65f6402) | Cause: the Minecraft world loads on mp_rust as a proxy map. Its `trigger_hurt` (origin 304 704 -1952, hull 15) starts `_load.gsc` `hurtPlayersThink`, which makes a touching player `suicide()` every 0.5 s: feet at or below spawn Y - 24 (down to spawn Y - 86) within about 100 blocks of the spawn, so with the logged spawn at Y 119 every death came at Y 95 or lower. A second mp_rust `killTrigger` pocket (radius about 1.5 blocks, 4-7 blocks below spawn, 30 blocks east and 46 north) does the same. The suicide records the victim as attacker, hence "Killed By FNG" (the default title) with the player's own name. Fix: both are disarmed on Minecraft loads (trigger_hurt entities emptied in place, the killTrigger line removed from the proxy script); vanilla void damage below min Y - 64 (4 every 10 ticks, through creative, spectator, armour and totems, "fell out of the world"). |
| 2. Faster builds | done | rust-lld linker, `[profile.dev] debug = 0`, dependencies at opt-level 1 in dev and test, suite without doc-tests, `--profile play` for mid-round test exes. Times below. |
| 3. Upstream | done | `upstream/main` is f608f85, our merge base: nothing new to merge (checked 2026-10-09). |
| 4. Crate split | done (merged as 6315039, full suite 1328 passed; the known flaky poof_particles test failed once and passed on re-run) | 24 commits 5a7a4e4..ee818cd, all from `docs/CRATE-SPLIT.md` including the optional B7 and C5. `minecraft_survival` holds `Server` (globals), `Level` per dimension and `Player` per UUID; `minecraft_net` holds `Dimension` and `PlayerUuid`. Log: `docs/round7/track-split-log.md`. |
| 5. Nether and End report | done | `docs/NETHER-END.md` |

## Phase 2
Design briefs: `docs/round7/a1-grenade-keys.md`, `a2-chat.md`, `b1-throwables.md`, `b2-shulker-lid.md`, `b3-held-armour.md`, `c-commands.md`, `d-saving.md`.

| Track | Branch | State |
|---|---|---|
| A. Controls and chat | track-a | merged (80cacc3, full suite 1315 passed). Left for after the split: mouse buttons still mine/place while a grenade is up with a Minecraft item selected (see `docs/round7/track-a-log.md`) |
| B. Items, blocks, player model | track-b | merged (e9ba6ad, full suite 1555 passed). Log `docs/round7/track-b-log.md` |
| C. Commands | track-c | merged (e166289, plus 375b5b4 /spawnpoint glue and 4ac8b27 rustfmt; full suite 1695 passed). Open: `/kit` skips `@` entries that the start kit accepts. Log `docs/round7/track-c-log.md` |
| D. Saving everything | track-d | merged (fc6d4bd, full suite 1398 passed). Review fixed 4 defects (rustfmt, broken containers coming back full, hoppers losing their last moves on quit, and one more). Log `docs/round7/track-d-log.md` |

## Phase 3 (after track D)
Briefs: `docs/round7/e0-phase3-plan.md` (order, groups, ownership, estimate), `e1-dimension-switch.md`, `e2-nether.md`, `e3-end.md`. Planned for this round: the switch (S1-S4), Nether N1-N5, End X1-X4 (a dragon that can be killed, exit portal, credits). Left for "finish survival": N6 (piglin/hoglin brains, striders, bartering), X5 (dragon strafing, breath, perching, egg teleport), X6 (gateways, item frames, shulkers, levitation).
| Track | State |
|---|---|
| Dimension switch | merged (9dbec02a + /execute in f0c30259, /time per dimension 2d9fe216, /kit @-entries 89f07c23, test race fix 3aef2ebe; full suite 1755 passed). Log `docs/round7/track-switch-log.md` |
| Nether | merged (d7d0d780 + af3cc119, full suite 1936 passed): N1-N5 (level rules, Nether lighting and lava, portal travel with timer, overlay, sounds, 8:1, search and build; zombified piglins, magma cubes, wither skeletons, ghasts with deflectable fireballs, blazes, spawners, fortress spawns; water evaporates, beds explode, respawn anchors, bed respawn, compasses and clocks). N6 left for finish survival. Log `docs/round7/track-nether-log.md` |
| End | parts 1-2 merged (d49f75d2, d9eabf74). Running on branch `end` (base af3cc119): X3 (crystals, the flying dragon, boss bar, multipart hits, healing beams) and X4 (death, exit portal, egg, gateway, credits, respawning the dragon). Log `docs/round7/track-end-log.md` |

## Phase 4 (known limitations)
Running beside the End dragon work on branch `limits` (worktree `C:\Dev\mc-wt\limits`, target `D:\iw4l-wt\limits`), on limitations from `docs/MINECRAFT-SURVIVAL.md` in files the End track does not touch. Log `docs/round7/track-limits-log.md`.

## Release builds
| # | When | Commit | State |
|---|---|---|---|
| 1 | tracks A-D merged (`--profile play`) | 4ac8b27 | installed as `iw4l-survival.exe` (old one is `iw4l-survival-3c2c62b.exe`), smoke-run on a temp world clean; owner steps in `docs/round7/release-1-tests.md` |
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

- Killstreaks: air support (airstrike, stealth bomber, harrier, AC-130, chopper, predator) is refused in a dimension with a ceiling (the Nether); the streak is kept and a line says it is unavailable. It works in the End and the Overworld. MW2 guns, grenades and craters work in every dimension with vanilla blast resistance.
- End credits: as vanilla, once per player (`seenCredits`), Esc skips, the trip home loads behind them.
- Ghast fireballs are deflected by MW2 bullets as well as melee (vanilla `AIM_DEFLECT` for redirectable projectiles on any hit).
- Elytra: a new 20 Hz glide mode in MW2 movement, guns still usable.
- Throwables kit syntax: `id@potion` for potion contents (`#` starts a comment in `.env`).
## Log
- 2026-10-10: Nether merged (af3cc119) and pushed; End X3-X4 and phase 4 started.
- 2026-10-10: End part 2 (eye throw, elytra, rockets) merged (d9eabf74) and pushed.
- 2026-10-09: End part 1 merged (d49f75d2) and pushed; End part 2 (eye throw, elytra) started.
- 2026-10-09: dimension switch merged (3aef2ebe) and pushed; Nether and End tracks started.
- 2026-10-09: dimension switch finished on its branch; merge onto main running.
- 2026-10-09: tracks B and C merged; release build 1 (play profile, 4ac8b27) installed and pushed.
- 2026-10-09: track D merged (fc6d4bd) and pushed; the dimension switch started.
- 2026-10-09: resumed after the restart: disk freed, track runner saved in the repo, tracks B, C and D restarted from their heads.
- 2026-10-09: stopped for a PC shutdown; all branches committed (track D with one WIP commit) and pushed. See "Where everything stands".
- 2026-10-09: crate split merged (6315039) and pushed; track D started; the split worktree was removed, so `D:\iw4l-target` belongs to the main checkout again.
- 2026-10-09: track A merged (80cacc3) and pushed.
- 2026-10-09: item 1 merged and pushed (65f6402). Crate split, track A and early track C started.
- 2026-10-09: plan written, `scripts/cargo-slot.ps1` added. Upstream checked (nothing new). Research for items 1, 4, 5 and the Phase 2 briefs done. Build changes measured and committed.
