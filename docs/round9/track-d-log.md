# Track D log: finish survival

Branch `track-d`, worktree `C:\Dev\mc-wt\track-d`, target dir `D:\iw4l-wt\track-d`. Briefs: `docs/round9/track-d.md` and the mob list `docs/round9/mobs.md`.

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

Hit keys for MW2 bullets: track D uses 29 upward, one per new mob store, recorded in `docs/round9/mobs.md`.

## Group mobs-list-drowned-load

### 1. Round 9 mob list: the 52 missing Minecraft mobs

After 24dfdf71.

- **Changed:** new `docs/round9/mobs.md`: one row per missing mob from the D2 table (where it is met, what exists, what is left, size, group key, hit-key byte, status, commits), the hit-key bytes (0-28 taken, 24 and 25 unused but left to the other tracks, track D from 29 up), the batching rule (one `EntityWorld` store per vanilla family), the six-step mob pattern with its `file:line` sites re-checked at 24dfdf71, and the shared prerequisites P1-P10 with the group that builds each.
- **Tests:** none (docs only).
- **Decisions:**
  - The brief's rows 27-30 (fish) and 35-36, 37-38 (guardians, nautiluses) are split into one row per mob, so every mob has its own status.
  - Hit-key bytes 24 and 25 are free in the code but were reserved by round 8's track B; track D keeps to 29 upward as the brief says.
- **Test steps:** none (docs only).

## State doc changes

None yet.

## Vendored files

Patched by track D: none yet. Added: none yet.

## Left for later

Nothing yet.
