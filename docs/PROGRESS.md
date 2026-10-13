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
- Every cargo command goes through `scripts\cargo-slot.ps1` (PowerShell). Gentle mode is on (the owner plays): `%LOCALAPPDATA%\iw4l-cargo-slot.txt` holds `slots=1`, `jobs=4`, `priority=low`.
- The full suite is `scripts\full-suite.ps1`.
- Track worktrees live in `C:\Dev\mc-wt\<name>` on branch `<name>`, each with its own target dir `D:\iw4l-wt\<name>`. Worktrees never share a target dir.
- Merging a track: in the track's worktree `git checkout --detach minecraft-survival` and `git merge --no-ff <track>`, run `scripts\full-suite.ps1 +target=<the track's target dir>` there, then `git merge --ff-only <merge commit>` in `C:\Dev\mashup-crafting` and push to `origin`. Delete the track's target dir once merged.
- Tracks run with the saved workflow `.claude/workflows/track-runner.js` with `round: 9`; logs go to `docs/round9/track-<name>-log.md`.
- Owner test steps and kit lines go into files under `docs/round9/`.

## Where it stands
- 2026-10-13: briefs being researched (workflow `round9-briefs`: one researcher per item, three weapon designs judged into `docs/WEAPONS.md`, one brief per track in `docs/round9/track-<x>.md`).

## Tracks
| Track | Branch | State |
|---|---|---|
| A. Multiplayer fixes | track-a | brief |
| B. Items and the screen | track-b | brief |
| C. Guns and grenades as real items | track-c | design and brief |
| D. Finish survival | track-d | brief |

## Release builds
| # | When | Commit | State |
|---|---|---|---|
| 1 | tracks A and B merged | | |
| 2 | end of the round | | |

## Decisions
- Gentle mode from the start of round 9: the owner had two `iw4l-survival` windows open (their multiplayer test).

## Log
- 2026-10-13: round 9 started; brief research started.
