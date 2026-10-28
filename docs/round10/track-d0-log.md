# Track D0 log: the shared mob registry

Branch `track-d0`, worktree `C:\Dev\mc-wt\track-d0`, target dir `D:\iw4l-wt\track-d0`. Brief: `docs/round10/track-d0.md`. Mob list: `docs/round10/mobs.md`.

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Vendored files

Files under `third_party/minecraftoss` this track patched or added (round 8 counted 73 patched and 37 added).

- Patched: none yet.
- Added: none yet.

## Protocol

No commit of this track has changed a `ToClient`/`ToServer` message or the codec yet. `minecraft_net::PROTOCOL` stays 6 (`docs/round10/decisions.md`).

## Group registry-core

### 1. Round 10 mob list with tracks, families and hit keys

After aa942451.

- **Changed:** new `docs/round10/mobs.md`, the D0-8 block of the brief verbatim: the hit-key bytes per group (0-35 taken, 36-39 D0, 40-43 D, 44-47 W, 48-63 M1, 64-79 M2, 80-95 M3, 96-99 F) and one section per round 10 track with its mobs, families, keys and status.
- **Tests:** none (docs only).
- **Decisions:** the file is copied line for line from the brief (`track-d0.md` lines 608-694 at aa942451), so later tracks find exactly the rows the brief describes.
- **Test steps:** none (docs only).
