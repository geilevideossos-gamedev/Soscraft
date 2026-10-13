# Round 9 progress: multiplayer fixes, items and the screen, guns as items, finishing survival

Started 2026-10-13 on `minecraft-survival` at f1dddf78. The owner's brief is the "Round 9" message (test results of round 8's release 2, 03e0c9be; screenshots in `scrennshots/screenshots-03e0c9b/screenshots-03e0c9b/`). This file is the hand-over point if a session stops. Read it top to bottom, then `git log --oneline f1dddf78..`, `git worktree list` and the track logs in `docs/round9/`. Round 8's hand-over is in git history (`git show f1dddf78:docs/PROGRESS.md`).

## The owner's brief
- Working in release 2: part F (model lighting, spyglass, spawn eggs, grenade keys), joining with two windows, seeing each other in the Overworld and the End, chat, shared chests, damage between players, both players travelling to the Nether and the End.
- Track A, multiplayer fixes: 1 players invisible to each other in the Nether; 2 held items float and overlap on other players and in the inventory preview; 3 the dragon stutters for the friend; 4 worn mob heads and carved pumpkins not drawn (and the pumpkin overlay); 5 dragon respawn only with crystals on the vanilla spots; 6 creative inventory for a friend in creative; 7 players stuck inside each other after spawn or teleport.
- Track B, items and the screen: 8 readable written books, books and quills, lecterns; 9 vanilla potion colours everywhere; 10 a much weaker, shorter damage overlay, still switchable.
- Track C, guns and grenades as real items (design in `docs/WEAPONS.md` first): 11 guns are normal items, class loadout once on first join, recipes for every gun, ammo, attachments, recipe book and creative tabs; 12 lethal and tactical grenade slots next to the hotbar, 64 per stack, 32 of each at start, recipes; 13 guns do half damage to players and armour reduces gun damage twice as much; 14 a totem also gives 4 s immunity to bullets and explosions.
- Track D, finish survival: the 52 missing mobs (survival order first: villagers, trading and villages, pillagers and raids, the warden, foxes...), trial chambers and trial spawners, lodestones and compasses, the friend limits (skating, first-person hand, music, Minecraft melee between players, portal overlay), and the open not-vanilla items (ridden strider under `/tick freeze`, TNT lit by another player with pvp off).
- Release builds: one when A and B are merged, one at the end; back up the old exe as `iw4l-survival-<old commit>.exe`, push, test steps and kit lines in `docs/round9/`. At the end update CLAUDE.md and `docs/MINECRAFT-SURVIVAL.md`. Decisions go to `docs/round9/decisions.md`.

## How to continue
- Every cargo command goes through `scripts\cargo-slot.ps1` (PowerShell). Full speed since 2026-10-13 afternoon (the owner closed the multiplayer test, no game running): `%LOCALAPPDATA%\iw4l-cargo-slot.txt` holds `slots=2`, `jobs=8`. Set it back to `slots=1`, `jobs=4`, `priority=low` when the owner plays.
- The full suite is `scripts\full-suite.ps1`.
- Track worktrees live in `C:\Dev\mc-wt\<name>` on branch `<name>`, each with its own target dir `D:\iw4l-wt\<name>`. Worktrees never share a target dir.
- Merging a track: in the track's worktree `git checkout --detach minecraft-survival` and `git merge --no-ff <track>`, run `scripts\full-suite.ps1 +target=<the track's target dir>` there, then `git merge --ff-only <merge commit>` in `C:\Dev\mashup-crafting` and push to `origin`. Delete the track's target dir once merged.
- Tracks run with the saved workflow `.claude/workflows/track-runner.js` with `round: 9`; logs go to `docs/round9/track-<name>-log.md`.
- Owner test steps and kit lines go into files under `docs/round9/`.

## Where it stands
- 2026-10-13: briefs and `docs/WEAPONS.md` committed (24dfdf71). Worktrees `C:\Dev\mc-wt\track-{a,b,c,d}` on branches `track-{a,b,c,d}` from 24dfdf71 (round 8's merged track branches were deleted first), target dirs `D:\iw4l-wt\track-{a,b,c,d}`. All four tracks run with the track-runner (base 24dfdf71, round 9).
- Track runs are split into segments; each segment is one track-runner run with its own review and verify. The launcher scripts (`run-<segment>.js`, made by `mkruns.py` from the brief workflow's groups) are in the session scratchpad; if they are lost, the groups are listed in each brief's plan.
  - A: one run, 5 groups (`a1-a2-a5-bodies-hands-crystals`, `a4-worn-heads`, `a3-dragon-sync`, `a6-friend-creative`, `a7-player-collision`).
  - B: one run, 7 groups (`b10-overlay` ... `b9-swirls`).
  - C: c1 = 8 groups (`fork-items` ... `book-place-creative`), then c2 = 5 groups (`ammo` ... `damage-totem`).
  - D: 70 groups in six runs: d1 = groups 1-13 (`mobs-list-drowned-load` ... `loot-functions`), d2 = 14-19 (pillagers, raids), d3 = 20-27 (darkness ... shriekers), d4 = 28-52 (`rabbits` ... `happy-ghasts`), d5 = 53-61 (`wither-live` ... `lodestones`), d6 = 62-70 (`friend-hand` ... `mobs-wrapup`).
- If a session stops mid-run: in the worktree read the track log and `git status`, commit or discard a half-done group, then start the same segment again with only the groups not yet committed.

## Tracks
| Track | Branch | State |
|---|---|---|
| A. Multiplayer fixes | track-a | **merged** 2db4cf47 + 57302e99 (5 conflicts with B, all both-sides; the message round-trip test counted one new message too few). 17 commits from the run (its own review stopped early with a progress note), then a second review (5 areas, 2 skeptics per finding) confirmed 7 defects, fixed e578f44f..d990a882 (rider head swing, frozen dragon for friends, sneak edge, ladders, adventure respawn, two log steps). Full suite green apart from that test, which passed after the fix; worktree and target removed; owner steps `docs/round9/track-a-log.md` and `docs/round9/release-1-tests.md` |
| B. Items and the screen | track-b | **merged** 2e13b78c (full suite green, 10.2 min); 12 commits f7cfddd4..fa5d1bf0: faint 0.5 s hit flash and no pain vision on Minecraft maps (B10), potion colours in icons, hands, drops, frames, other players and effect swirls (B9), books read from the hand, edit and sign screens, friends through the host, tooltips (B8); review fixed 1 defect (an owner step asked for a friend's first-person potion); worktree and target removed; owner steps `docs/round9/track-b-log.md` (not in this branch any more: in minecraft-survival) |
| C. Guns and grenades as real items | track-c | c1 running |
| D. Finish survival | track-d | d1 running |

## Release builds
| # | When | Commit | State |
|---|---|---|---|
| 1 | tracks A and B merged | 57302e99 | done 2026-10-13: 10.1 min at full speed (`-p launcher -p iw4l-master`); copied in as `iw4l-survival.exe`, old one kept as `iw4l-survival-03e0c9b.exe`, relay copied too (unchanged); test steps `docs/round9/release-1-tests.md` |
| 2 | end of the round | | |

## Decisions
- Gentle mode from the start of round 9: the owner had two `iw4l-survival` windows open (their multiplayer test).
- A5 (dragon respawn): no defect. Our check is vanilla's `EnderDragonFight.tryRespawn`: a crystal's 2x2x2 box must touch the cell 3 blocks out on each side at portal height + 1, so any of the three straight rim blocks on a side counts and only the four corner blocks don't. The crystals in screenshot 9 sit on straight rim blocks, so vanilla respawns too. Track A pins it with tests; exact centres only would be one line and not vanilla.
- TNT with pvp off (track D): the brief found that TNT hurts nobody today, so the owner's premise needs checking in game; D10 handles it.
- minecraft_net `PROTOCOL` stays 4 after both A and B bumped it: no protocol 4 build was released before release 1.
- Four tracks at once in gentle mode: D: had 45 GB free; a watchdog warns below 10 GB.

## Log
- 2026-10-13: track A merged (57302e99); release 1 built and copied in; test steps `docs/round9/release-1-tests.md`.
- 2026-10-13: track B merged (2e13b78c), full suite green.
- 2026-10-13: briefs done (28 agents); tracks A, B, C (c1) and D (d1) started.
- 2026-10-13: round 9 started; brief research started.
