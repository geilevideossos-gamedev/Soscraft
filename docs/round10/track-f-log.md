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

### 2. An Elden Ring boss picks, times and lands its moves like Margit

After 6fc4c759.

- **Changed:**
  - New `src/shape.rs`: Minecraft's yaw (`forward`, `left`, `yaw_to`, `wrap`), a `Frame` (his feet and facing when a window starts) and a player's `Body` (feet, width 0.6, height = eye height / 0.9, chest at 0.6 of it). `in_arc` (the near side of the box within reach, inside the arc's width widened by the box's angular size, and overlapping the height band, which for Margit reaches 1 block below his feet), `in_circle`, `under_ring` with `ring_band` (the ring's radius grows over the window in equal bands, one per tick; a player is hit when the band crosses their box, their feet are less than 1 block above the ground under them and inside the ring's height band), `struck` for any `Shape`, `centre` (where a hit comes from), and `segment` against a box (for F4's daggers).
  - New `src/random.rs`: `SplitMix64` (no new dependency) with `unit`, `below`, `between` and `chance`, so a seed replays a fight.
  - New `src/brain.rs`: `Brain::new(def, seed)` and `step(&Sense) -> Act`.
    - `Sense`: the tick, his position, yaw, whether he is on the ground, his anchor, and the eligible players as `Seen` (id, feet, eye height, the height of the ground under their feet, whether a path reaches them).
    - `Act`: where to walk, the yaw to face, the head look (the target's eyes while awake), the root motion this tick, the `Hit`s that landed (player, damage, physical or holy, reaction, shield disable, weapon, where it came from, which strike), the `Dagger`s to launch (from his left hand at the dagger speed, aimed at the target's chest), the cues of this tick, a fade target, the target, and the `Clip` (state, move index, the move's own clock, held, phase, awake).
    - States of design doc 4.4 as `State`: `Asleep` (wakes when an eligible player is within 24 blocks of the anchor), `Wake` (30 ticks with its cues), `Idle` (the phase's idle gap, then a move; meanwhile he faces his target at 30 degrees a tick and walks toward it, stopping 2 blocks short, or circles it in 30-degree steps within 5 blocks, never past the leash), `Move`, `Recover` (a branch that ends a move early), `Kneel`, `Shift`, `Fade` (sends the fade target on its swap tick) and `Dead` (stays at the end of its clip, `gone()`), and `Return` (walks to the anchor and sleeps within 1 block, or fights at once with no rise when a player comes within 24 blocks). `Kneel`, `Shift`, `Fade`, `Dead` and `Return` run their clocks here; what enters them is group `er-rules`.
    - A move: on each new clock tick, in this order: branches (table order, the first whose condition and chance pass wins; a move branch starts the new move on the same tick, an end branch starts the recovery), strikes (the frame is fixed on the strike's first tick before he turns, each strike lands at most once per player, tested against every player on every tick of its window), tracking (turns toward the target only inside a tracking window), root motion, throws (by phase and chance), cues, then the hold. The clock stops for the rolled hold, during which nothing fires and he turns only if the hold sits in a tracking window.
    - Root motion: steps and hops spread evenly over their ticks along his facing (a hop goes back), dropped on a tick that would take him farther out past the leash. A leap's landing is fixed on its first motion tick (the last tracking tick): the target's distance minus how far ahead his landing circle sits, clamped to 6-12 blocks, then pulled onto the leash circle if it is outside it, at the height of the ground under the target (within the apex of his take-off). It is a parabola with the move's apex, given as a displacement per tick.
    - Choosing: the moves of his phase off cooldown whose distance band holds the target's horizontal distance and whose angle band holds the angle to it, rolled by weight. The target is the eligible player nearest to him within 40 blocks of the anchor, picked at the wake, after every move and at once when the target is gone (threat comes with `er-rules`).
  - `check` also refuses a branch at a move's first tick (`data.rs`).
  - Design doc kept in step: the `random` module in 4.1, an arc's angle is its full width (4.3), and `Sense` and `Act` as built (4.4).
- **Tests:** `brain::tests::a_player_at_two_blocks_draws_a_close_move_and_one_at_twenty_a_dagger`, `running_away_at_the_tracking_stop_dodges_the_cane_and_standing_still_does_not`, `a_swing_lands_once_per_player`, `a_jump_clears_the_hammer_shockwave`, `the_dagger_hold_lasts_10_to_30_ticks_and_delays_the_throw`, `the_cane_combo_ends_after_one_two_or_three_swings`, `a_leap_never_lands_outside_the_leash`, `a_move_on_cooldown_is_not_picked`, `the_same_seed_plays_the_same_moves`, and three more: `he_wakes_when_a_player_comes_within_24_blocks_and_rises_for_30_ticks`, `the_backstep_hops_four_blocks_back_and_throws_two_or_three_daggers`, `a_returning_boss_walks_to_his_anchor_and_sleeps_there_or_fights_at_once`. `shape::tests::left_is_plus_x_and_forward_plus_z_at_yaw_zero`, `a_player_box_is_as_tall_as_the_eye_over_nine_tenths`, `an_arc_reaches_a_player_in_a_two_deep_hole_beside_him_and_not_a_three_deep_one`, `an_arc_misses_behind_him_beyond_its_reach_and_outside_its_width`, `a_circle_hits_what_it_touches_in_front_of_him`, `a_ring_hits_standing_feet_as_its_edge_passes_and_never_a_jumper`, `a_segment_hits_a_box_it_crosses_and_not_one_it_passes`. `random::tests::the_same_seed_gives_the_same_numbers`, `rolls_stay_inside_their_bounds_and_reach_both_ends`. The new branch check in `margit::tests::a_branch_to_an_unknown_move_or_poses_out_of_order_are_refused`. `test -p elden_boss --locked`: 29 green; `clippy -p elden_boss --all-targets`: clean.
- **Decisions:**
  - A move's cooldown starts when the move ends (or is cut short), not when it is picked: the cane combo lasts 70 ticks and its cooldown is 20, so counting from the pick would mean nothing. Phase 2 multiplies it by 0.8.
  - The arena's radii (wake, leash, reset) are horizontal distances from the anchor: design doc 8 calls the arena a circle.
  - A strike's frame (his feet and facing) is fixed on its first tick, before he turns that tick, so the cane combo's turn during its first swing aims only the second swing, and the 8-tick tracking stop holds for every strike. Rings and circles are centred where he stood then, so a shockwave does not follow him.
  - A hit's `from` is the arc's origin (his feet) or the circle's and ring's centre, for F4's knockback direction.
  - An end branch becomes its own `Recover` state (the clip keeps the move's index, the clock counts the recovery), then the phase's idle gap follows as after any move. The renderer crossfades from the pose he had.
  - `Sense` carries no health: `Brain::take` (group `er-rules`) decides damage and death, so the brain is the place to keep his health, and the game keeps one copy.
  - He does not start a move while he is off the ground.
  - `Brain::begin(key)` starts a move at once, for tests here and in the game crates later.
- **Owner's steps:** none in game: the crate is not in the game yet. No kit line.
