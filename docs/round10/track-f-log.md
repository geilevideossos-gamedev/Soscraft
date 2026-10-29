# Track F log: the first Elden Ring boss, Margit

Branch `track-f`, worktree `C:\Dev\mc-wt\track-f`, target dir `C:\iw4l-wt\track-f`. Briefs: `docs/round10/track-f.md` and the design doc `docs/ELDEN-RING-BOSSES.md`.

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

Segment f1 (groups `er-core` and `er-rules`) builds the pure crate `crates/elden_boss` and touches no shared game file: only the workspace member list in the root `Cargo.toml` and `Cargo.lock`. Nothing of it is in the game yet, so these commits have no owner steps in game and no kit line.

## Protocol

Nothing on the wire changed yet.

## Vendored files

None patched or added yet.

## Group er-core

### 1. Elden Ring boss moves are plain data, starting with Margit's

After c9253dba.

- **Changed:**
  - New crate `crates/elden_boss` (edition 2024 and licence from the workspace, `[lints] workspace = true`, one dependency `glam = "=0.29.3"`, the glam of the vendored entities crate and `minecraft_terrain`, so `DVec3` is the same type on both sides). It is a workspace member in its alphabetical place in the root `Cargo.toml`; `Cargo.lock` gained the package from one `check` without `--locked`.
  - `src/data.rs`: the boss as data (design doc 4.2 and 4.3), all with `'static` slices: `BossDef`, `DataSource` (`Estimate`, `Baked`), `Scaling`, `Stance`, `Resist` (a health and stance `Factor` per source, and the kneeling factor), `Critical`, `Arena`, `Stalk`, `PhaseDef` with its idle `Gap` and shift clip, `MoveDef` with `Pick`, tracking `Window`s, `Hold`, `Motion` (step, hop, leap), `Strike` with `Shape` (arc, circle, ring), `Harm` (physical, holy), `Reaction` (flinch, stagger, knockdown) and the weapon it uses, `Throw`, `Conjure` with `Weapon` (dagger, sword, hammer), `CueKey` with `Cue` (a `Sound` with volume and pitch, or a `Burst`: golden mist, sparks, slam, crit), `PoseKey` with `Pose` (the 24 key poses the renderer will draw), `Branch` with `When` (always, a phase, the target farther than) and `BranchTo` (a move, or end early with a recovery). `ClipDef` and `Clips` for the clips that are not picked (idle, walk, wake, kneel, fade, death; the shift is the phase's). `Daggers`, `Experience`, `Loot`, `Voices` and the shared `Cues`. `def(id)` and `IDS` list the bosses.
  - `check(&BossDef)`: refuses a window (tracking, hold, motion, conjure, strike, throw, cue, branch) outside its move; a hold inside a strike; a strike tracked until less than 8 ticks before it (moves that leap are exempt, their landing is fixed); a branch to an unknown move key; pose keys that do not start at 0, are out of order or do not reach the end; a distance from 0 to 30 blocks (in 0.1 steps) where no move of a phase fits; a strike with a golden weapon that is not shown from 8 ticks before it to its end (design doc 5.4); a throw with no dagger shown; a kneel clip that is not as long as the kneel; phase 1 with a shift or a later phase without one.
  - `src/margit.rs`: `MARGIT` with every number of design doc 4.2, 5.1-5.4, 6, 7.1-7.3, 8, 9.1 and 13, and `source: DataSource::Estimate`: the eight moves with their picks, weights, cooldowns, tracking windows, holds, motion, strikes, throws, conjured weapons, cues and poses; the six branches of 5.2; the clips of 5.3 and the phase 2 shift (gold flash at 20 with `entity.elder_guardian.curse` and `block.beacon.activate`); voices and cue sounds of 14.1.
- **Tests:** `margit::tests::margit_passes_its_checks`, `a_strike_past_the_end_of_its_move_is_refused`, `every_distance_up_to_30_blocks_has_a_move_in_both_phases`, `the_hammer_moves_are_phase_two_only`, and three more: `a_strike_tracked_until_fewer_than_8_ticks_before_is_refused_unless_it_leaps`, `a_branch_to_an_unknown_move_or_poses_out_of_order_are_refused`, `a_golden_weapon_not_shown_during_its_strike_is_refused`, `health_grows_with_the_fighters_at_the_wake`. `test -p elden_boss --locked`: 8 green.
- **Decisions:**
  - An arc's angle in the table is its full width (the cane's 100 degrees is 50 to each side, the sword's 180 is the half circle in front), stored as `spread`. Design doc 4.3 calls it a half-angle; the 180-degree sweep only makes sense as a full width.
  - Each move's hold sits outside its strikes, and the shift is a clip of phase 2 rather than a move, so it is never picked.
  - The dagger hand is `(0.5, 2.2, 0.4)` in his own frame (x to his left, y up, z forward); left is +x when he faces +z, as Minecraft's yaw has it.
  - The idle and walk clips are 40 and 20 ticks; they loop, and the walk is driven by the distance moved (design doc 5.3).
  - A shield disable is 100 ticks (5 s) on both hammer strikes and no other.
- **Owner's steps:** none in game: the crate is not in the game yet. No kit line.
