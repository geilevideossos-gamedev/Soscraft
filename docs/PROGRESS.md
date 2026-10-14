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
- Every cargo command goes through `scripts\cargo-slot.ps1` (PowerShell). Gentle mode since 2026-10-13 evening (the owner tests release 1): `%LOCALAPPDATA%\iw4l-cargo-slot.txt` holds `slots=1`, `jobs=4`, `priority=low`. Full speed (`slots=2`, `jobs=8`, `priority=normal`) since 2026-10-13 late, when the owner said they stopped playing; back to gentle when they say "gentle mode" again.
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
- **New order (owner, 2026-10-14, weekly usage at 80 %):** after d2, merge track D into minecraft-survival (full suite), build release 2 (A, B, C, d1, d2), write `docs/round9/release-2-tests.md`. Then track-d continues from that merge in this order, each a track-runner run whose base is a tag set on track-d's HEAD just before it starts:
  1. `run-n1-d6.js` (base `r9-d6-base`): d6 without `mobs-wrapup` (friend hand, portal and music, pvp melee, friend skating, skater poses, strider freeze, TNT, small not-vanilla items).
  2. `run-n2-d5.js` (`r9-d5-base`): `keyframes` (moved from d3, breeze drawing needs it), then d5 (wither, breezes, trial spawners, vaults, lodestones).
  3. `run-n3-d3.js` (`r9-d3-base`): d3 without `keyframes` (darkness, vibrations, warden, shriekers).
  4. `run-n4-d4a.js` (`r9-d4a-base`): the common survival mobs (rabbits, foxes, phantoms, squids, fish, goats, silverfish and endermites, parrots, bees, guardians, turtles, dolphins, axolotls).
  5. `run-n5-d4b.js` (`r9-d4b-base`): the less important mobs (frogs ... happy ghasts).
  6. `run-n6-wrap.js` (`r9-dw-base`): `mobs-wrapup` last.
  The scripts are made by `mkdnext.py` in the session scratchpad. PROTOCOL is 5 in release 2; the first later group that changes a message sets 6, once. Then the final review, merge, release 3, decisions.md, CLAUDE.md and MINECRAFT-SURVIVAL.md.
- **If the usage limit hits:** stop cleanly. Commit only finished, tested groups (discard a half-done one), update this file with where it stopped, push.
- If a session stops mid-run: in the worktree read the track log and `git status`, commit or discard a half-done group, then start the same segment again with only the groups not yet committed.

## Tracks
| Track | Branch | State |
|---|---|---|
| A. Multiplayer fixes | track-a | **merged** 2db4cf47 + 57302e99 (5 conflicts with B, all both-sides; the message round-trip test counted one new message too few). 17 commits from the run (its own review stopped early with a progress note), then a second review (5 areas, 2 skeptics per finding) confirmed 7 defects, fixed e578f44f..d990a882 (rider head swing, frozen dragon for friends, sneak edge, ladders, adventure respawn, two log steps). Full suite green apart from that test, which passed after the fix; worktree and target removed; owner steps `docs/round9/track-a-log.md` and `docs/round9/release-1-tests.md` |
| B. Items and the screen | track-b | **merged** 2e13b78c (full suite green, 10.2 min); 12 commits f7cfddd4..fa5d1bf0: faint 0.5 s hit flash and no pain vision on Minecraft maps (B10), potion colours in icons, hands, drops, frames, other players and effect swirls (B9), books read from the hand, edit and sign screens, friends through the host, tooltips (B8); review fixed 1 defect (an owner step asked for a friend's first-person potion); worktree and target removed; owner steps `docs/round9/track-b-log.md` (not in this branch any more: in minecraft-survival) |
| C. Guns and grenades as real items | track-c | **merged** 763466a2 (full suite green, 9.3 min). c1: 21 commits to d69f2140 (fork item table, vanilla carriers with custom_data, arsenal, drops, first-join loadout, recipes, attachments, recipe book, creative). minecraft-survival (A, B) merged in at c27f9bc9 with PROTOCOL 5. c2: 15 commits to 114a788b (ammo, clip bar, grenade slots 43/44, G and Shift+G from the slots, 32 each for new players, tactical insertion respawn, class menu gate, half gun damage and doubled armour share, pierce, totem grace); its review fixed 2 defects. A second review (5 areas, 2 skeptics per finding) confirmed 10, fixed be508862..6b7e4e71 (clip written onto a second copy, vanilla recipe unlock on craft, akimbo reload beyond the stock, insertion loadout, rustfmt, x.5 rounding, owner steps). Worktree and target removed; owner steps `docs/round9/track-c-log.md` |
| D. Finish survival | track-d | d1 done (30 commits up to 22278b12): mob list, drowned, villager trading for host and friends, zombie villagers and cures, omens and effect icons, cats and ocelots, mules and llamas, riding and the horse inventory, leads, wandering traders, iron and snow golems, loot functions and ancient city chests. The stop during horse-family commit 2 was redone from scratch (6bb46886). Its review fixed 4 defects (rustfmt, baby drowned experience, one viewer per horse inventory, a clock-dependent test). Then minecraft-survival (A, B, C) is merged into track-d (20 conflicting files, PROTOCOL stays 5), checked by a second agent, tag `r9-d2-base`, then d2 (pillagers ... raid-villagers) runs with base `r9-d2-base` (`run-d2m.js`, made by `mkd2.py`). Then d3..d6. Merge done (tag `r9-d2-base` = 92ef4231, then b429636a for the log's protocol line). d2 got through pillagers, patrols-vindicators, evokers-vexes and ravagers (up to c14c26aa) before a second stop on 2026-10-14; the raids group's uncommitted half was discarded (diff kept as `track-d-raids-halfdone.patch`, reference only) and d2 relaunched for raids and raid-villagers (`run-d2r.js`, made by `mkd2r.py`). |

## Release builds
| # | When | Commit | State |
|---|---|---|---|
| 1 | tracks A and B merged | 57302e99 | done 2026-10-13: 10.1 min at full speed (`-p launcher -p iw4l-master`); copied in as `iw4l-survival.exe`, old one kept as `iw4l-survival-03e0c9b.exe`, relay copied too (unchanged); test steps `docs/round9/release-1-tests.md` |
| 2 | after track D's d2 (owner's request, before the limit) | | planned |
| 3 | end of the round | | |

## Decisions
- Gentle mode from the start of round 9: the owner had two `iw4l-survival` windows open (their multiplayer test).
- A5 (dragon respawn): no defect. Our check is vanilla's `EnderDragonFight.tryRespawn`: a crystal's 2x2x2 box must touch the cell 3 blocks out on each side at portal height + 1, so any of the three straight rim blocks on a side counts and only the four corner blocks don't. The crystals in screenshot 9 sit on straight rim blocks, so vanilla respawns too. Track A pins it with tests; exact centres only would be one line and not vanilla.
- TNT with pvp off (track D): the brief found that TNT hurts nobody today, so the owner's premise needs checking in game; D10 handles it.
- minecraft_net `PROTOCOL` stays 4 after both A and B bumped it: no protocol 4 build was released before release 1.
- Four tracks at once in gentle mode: D: had 45 GB free; a watchdog warns below 10 GB.

## Log
- 2026-10-14: the owner reordered track D for the usage limit: release 2 after d2, then d6, d5, d3, d4 (important mobs first), mobs-wrapup last.
- 2026-10-14: the session stopped again mid-run (VS Code closed); track D checked, the half-done raids group discarded, d2 relaunched for raids and raid-villagers.
- 2026-10-14: track D d1 done; A, B and C being merged into track D, then d2.
- 2026-10-13: track C merged (763466a2), full suite green.
- 2026-10-13: the session stopped mid-run (VS Code closed); tracks C and D checked and relaunched with their remaining groups, full speed.
- 2026-10-13: the owner stopped playing; cargo back to full speed.
- 2026-10-13: track C c1 done; A and B being merged into track C, then c2.
- 2026-10-13: track A merged (57302e99); release 1 built and copied in; test steps `docs/round9/release-1-tests.md`.
- 2026-10-13: track B merged (2e13b78c), full suite green.
- 2026-10-13: briefs done (28 agents); tracks A, B, C (c1) and D (d1) started.
- 2026-10-13: round 9 started; brief research started.
