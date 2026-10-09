# Round 7 progress: finish the Minecraft side

Started 2026-10-09 on `minecraft-survival` at 3c2c62b. The owner's brief for this round is the "Big round: finish the Minecraft side" message; this file is the hand-over point if a session stops. Read it top to bottom, then `git log --oneline 3c2c62b..` and `git worktree list`.

## How to continue
- Every cargo command goes through `scripts\cargo-slot.ps1` (PowerShell), e.g. `& scripts\cargo-slot.ps1 test -p render_anim --locked`. It adds `-j 8`, refuses other job counts, and holds one of two machine-wide slots, so at most two cargo processes run at once. `--release`, `--workspace`, `--profile` and `+exclusive` take both slots, so a release build or the full suite runs alone. `+target=D:\dir` picks the target dir (default `D:\iw4l-target`).
- Track worktrees live in `C:\Dev\mc-wt\<track>` on branches `track-<name>`, each with its own target dir `D:\iw4l-wt\<track>` (worktrees must not share a target dir: cargo's freshness check would reuse another worktree's build of a crate whose files are older there).
- Merge tracks into `minecraft-survival` one at a time, run the full suite after each merge, then push to `origin`.
- Owner decisions taken without asking are listed under "Decisions".

## Phase 1
| Item | State | Notes |
|---|---|---|
| 1. Death at a fixed height | investigating | screenshots 1, 2: "Killed By FNG" |
| 2. Faster builds | measuring | before/after table below |
| 3. Upstream | done | `upstream/main` is f608f85, our merge base: nothing new to merge (checked 2026-10-09) |
| 4. Crate split | planning | `crates/minecraft_survival`, `crates/minecraft_net` |
| 5. Nether and End report | researching | `docs/NETHER-END.md` |

## Phase 2 (after the crate split)
| Track | Branch | State |
|---|---|---|
| A. Controls and chat | track-a | not started |
| B. Items, blocks, player model | track-b | not started |
| C. Commands | track-c | not started (C23 partial work in `git stash`) |
| D. Saving everything | track-d | not started |

## Phase 3 (after track D)
| Track | State |
|---|---|
| Dimension switch | not started |
| Nether | not started |
| End | not started |

## Release builds
| # | When | Commit | State |
|---|---|---|---|
| 1 | tracks A-D merged | | |
| 2 | end of round | | |

## Build times
| Measurement | Before | After |
|---|---|---|
| Clean release build (`-p launcher`) | | |
| Release after touching one file | | |
| Clean dev build (`-p launcher`) | | |
| Dev build after touching one file | | |
| Full test suite | | |
| Clippy (workspace) | | |

## Decisions
- (none yet)

## Log
- 2026-10-09: plan written, `scripts/cargo-slot.ps1` added.
