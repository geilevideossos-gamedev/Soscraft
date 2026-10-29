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

## Group er-rules

### 3. An Elden Ring boss staggers, changes phase and resets like Margit

After 7c16b429.

- **Changed:**
  - New `src/hurt.rs` (design doc 7.1-7.3 and 9.2):
    - `Source`: `Bullet { mw2 }`, `Blast { mw2 }`, `Melee { damage, from }`, `Projectile { damage }`, `Explosion { damage }` and `Environment { damage }`. `Hurt` is a source, the attacker's player id if any, and the tick. `Taken` is the health and stance dealt, and whether it was a critical, started a kneel or killed him.
    - `uses_hurt_cooldown(&source)` is true only for `Environment`.
    - `Brain::take(&Hurt) -> Taken` uses his resistances from `MARGIT.resist`:

      | Source | Health | Stance |
      |---|---|---|
      | Bullet | MW2 points x 0.2 x 0.5 | points / 60 |
      | Blast | MW2 points x 0.8 x 0.25 | points / 15 |
      | Melee | x 1 | x 1.5 |
      | Projectile | x 1 | x 1 |
      | Explosion | x 0.5 | x 1 |
      | Environment | x 1 | none |

    - While he kneels, every hit does x 1.5. The first melee hit from within 60 degrees of his facing and 4 blocks of him while he kneels is a critical: x 3, the critical cues on his next `Act`, and the kneel clock jumps to its last 20 ticks, so he rises. Stance refills after 120 ticks with no stance damage, and when he rises.
    - Threat is each player's damage over the last 200 ticks. `chosen` picks the eligible player within 40 blocks of the anchor with the most threat, or the nearest when nobody has any: at the wake, after every move and as soon as his target is gone.
    - `hurt_by()` lists everyone who hurt him since the wake. `fighters()` is the wake's fighters plus those, for F4's reward.
    - `health()`, `max_health()`, `stance()`, and `set_health()` for a summon's `Health` (clamped to 1 to his maximum).
    - He takes nothing while asleep, returning, fading home or dead.
  - New `src/fight.rs` (design doc 6, 8 and 9.1):
    - `Brain::awake()`, and `Brain::reset(tick)`, which is public so F2's loader can call it: base health, phase 1, full stance, cooldowns, threat and the hurt list cleared, then `Return`.
    - The wake (`open`): the eligible players within 40 blocks of the anchor are the fighters and set the health factor. Health keeps its share, and stance, threat and the hurt list start fresh.
    - The referee runs every tick before the state's own step: stance refill, the threat window, the reset after 100 ticks with nobody within 40 blocks, the phase shift, a stance break that had to wait, and the stuck fade or daggers.
    - The watch runs after the step: it keeps his frame for the critical's front check and counts how long he has been stuck.
    - `settle` runs where a timed state or a move ends: a home fade ends asleep, a kneel refills his stance, and a waiting shift or a broken stance comes next.
    - The pinned dagger pick and the home fade.
  - `brain.rs`:
    - `Seen.room`: a standing spot with room for his box beside the player, within 3 blocks across and 2 up or down of their feet (design doc 8), which the game finds; `None` when there is none.
    - `Brain` keeps `Wounds` (hurt) and `Rules` (fight). `step` runs the referee and the watch and passes on the queued cues. `wake` opens the fight, and `rest` settles first.
    - Targets are picked by threat. `choose` asks `reaches`: when he is pinned, only moves that throw, at any distance. `awake()` is false during a home fade.
    - The test field is shared with the new modules (`woken`, and fades are applied).
  - Design doc kept in step: 4.4 (`room`, the home fade), 7.1 (a critical replaces the x 1.5), 8 (the home fade).
- **Tests:**
  - `hurt::tests`:
    - `an_m4_body_hit_takes_3_health_and_half_a_stance_point`
    - `a_frag_at_its_centre_takes_30_health_and_10_stance`
    - `two_sword_hits_in_one_tick_both_count`
    - `environment_damage_uses_the_hurt_cooldown_and_player_hits_do_not`
    - `a_broken_stance_kneels_80_ticks_and_refills_after_6_quiet_seconds`
    - `a_melee_hit_from_the_front_on_a_kneeling_boss_is_a_critical_and_one_from_behind_is_not`
    - `an_asleep_or_returning_boss_takes_no_damage`
    - `the_player_who_hurt_him_most_lately_is_his_target`
    - three more: `a_stance_broken_in_a_leap_kneels_when_he_lands`, `a_killing_blow_kills_him_once_and_he_is_gone_after_20_ticks`, `the_fighters_are_the_players_at_the_wake_and_everyone_who_hurt_him`
  - `fight::tests`:
    - `half_health_starts_the_shift_when_the_current_swing_ends` (also at once between moves and during a wind-up)
    - `stance_does_not_break_during_the_shift`
    - `two_fighters_at_the_wake_give_1280_health_and_keep_the_share` (and 2240 with four)
    - `nobody_within_40_blocks_for_5_seconds_resets_the_fight`
    - `an_unreachable_target_for_5_seconds_asks_for_a_fade`
    - `a_target_on_a_pillar_gets_daggers_not_a_fade`
    - three more: `a_shift_in_a_leap_waits_for_the_landing_strike`, `a_reset_boss_that_gets_no_closer_to_home_for_5_seconds_fades_there`, `phase_two_idles_6_to_12_ticks_and_cools_moves_by_four_fifths`
  - `test -p elden_boss --locked`: 49 green. `clippy -p elden_boss --all-targets`: clean.
- **Decisions:**
  - **Hurt cooldown:** the game applies it (the vendored `hurt_absorbed`) before `take`, and only for `Environment`. The brain only says which sources use it and counts every hit it is given, so there is one cooldown, the vendored one.
  - **Criticals:**
    - A critical is x 3 instead of the kneeling x 1.5, not on top of it: 33 for an 11-damage sword hit.
    - It moves the kneel clip to its last 20 ticks, where it goes from `Kneel` to `Rise`, so the renderer draws him getting up. Hits in those 20 ticks are normal: no x 1.5 and no second critical.
    - A melee hit from behind or from farther than 4 blocks does not use up the critical. The first melee hit from the front gets it.
    - The critical's cues go on his next `Act`, which the host plays like the clip cues; `Taken.critical` reports it as well.
  - **Kneeling and phases:**
    - A hit on a kneeling boss takes no stance: his stance is 0 and refills when he rises.
    - A stance break or a shift waits while he is in a leap, between take-off and landing. The break comes on the landing tick, which cancels the landing strike; the shift comes after the landing strike's window. He never stops in mid-air.
    - A shift that comes during `Wake`, `Kneel` or `Fade` waits for that state to end. A stance broken during `Shift`, `Wake` or `Fade` kneels as soon as that state ends.
    - His phase is 2 from the shift's first tick, so the shown clip has phase 2 while the hammer forms.
    - "Health at or below half for the first time" is checked after every hit. A Margit summoned at 50.1% shifts on his first hit (step 9 of the owner's test), and one summoned below half does too.
  - **Stuck:**
    - The stuck count runs from the wake whenever his target is inside the leash, unreachable, and none of his hits landed on it. Daggers do not count as hits here, since the game resolves them.
    - The fade starts at his next idle tick. The spot comes from the game (`Seen.room`).
    - With no room he is pinned: he picks only moves that throw daggers, at any distance and with their cooldowns, until the target is reachable again or a hit lands.
  - **The home fade** after a reset:
    - "No path home found within 100 ticks" is measured as no progress: 100 ticks in `Return` without getting 1 block closer to the anchor than before. This needs no path plan from the game.
    - During the home fade he is not awake (no bar, no target, he takes nothing), and it ends with him asleep at his anchor.
  - **Health:**
    - The reset puts him back at base health (factor 1), and the next wake scales it again.
    - `set_health` keeps at least 1 health.
    - `Taken.died` is set on the killing hit, and the game credits that hit's player.
- **Owner's steps:** none in game: the crate is not in the game yet. No kit line.

### 4. A whole Margit fight plays through without a window

After 77a03ca8.

- **Changed:**
  - New `src/whole_fight.rs`, test only (`#[cfg(test)] mod whole_fight` in `lib.rs`). It plays a headless fight (design doc 4.4) on flat ground with no blocks: Margit's anchor is at the origin, the rifleman (player 1) starts 15 blocks away and the swordsman (player 2) 6 blocks away, so both are fighters at the wake (1280 health). It runs for at most 6000 ticks with a fixed seed.
  - **The boss:** each tick the fight applies his root motion, or walks him toward his walk point at 0.15 blocks a tick and drops him 0.5 a tick when he is above the ground. It also applies fades and his facing.
  - **Daggers:**
    - Each dagger flies its velocity every tick.
    - It hits the first player whose box its segment crosses (`shape::segment`).
    - It ends at the ground or after 40 ticks.
  - **The rifleman:**
    - He backs off at 0.265 blocks a tick (MW2's run) when nearer than 12 blocks and closes in when farther than 18.
    - Between those, he strafes at 0.15 a tick, stops to aim for the first 30 of every 60 ticks and turns his strafe every 60 ticks.
    - While Margit is awake he lands 7 bullets a second of 30 MW2 points (`Source::Bullet`), with a 40-tick reload after every 30.
  - **The swordsman:**
    - He runs straight away at 0.265 a tick when a move with strikes starts within 4 blocks of him (a new move, a branch to another move included).
    - He steps back in when a strike window of that move ends, or when the move ends, and closes to 2 blocks.
    - He hits for 11 (`Source::Melee` from his feet) every 13 ticks when within 3 blocks and not running.
    - During a kneel he walks to 1.5 blocks in front of Margit and hits only from there, so his first hit of the kneel is the critical.
  - Both stay within 38 blocks of the anchor. Players take their hits but are never killed: their health is not modelled.
- **Tests:**
  - `whole_fight::a_whole_margit_fight_shows_every_move_both_phases_a_kneel_and_ends` (seed 24): every one of the eight move keys is seen, phase 2 is reached, he kneels and takes a critical, and both players are hit by a strike or a dagger. He dies within 6000 ticks and is gone 20 ticks later. He never resets and never stands outside the leash. The same seed gives the same record, death tick included, and seed 25 dies on another tick.
  - One more, `every_seed_of_the_whole_fight_reaches_phase_two_kneels_and_dies_inside_his_arena`: seeds 0-29 each reach phase 2, kneel and take a critical, target both players at some point, die within 6000 ticks, never reset and stay inside the leash.
  - `test -p elden_boss --locked`: 51 green. `clippy -p elden_boss --all-targets`: clean.
- **Decisions:**
  - **Wind-up:** the swordsman's "wind-up" is the start of a move that has strikes, not the wind-up of each strike. He steps back in after the first strike window and so meets the delayed swings, which is how he gets hit (the third cane swing, the hammer's ring). If he ran from every strike, he would never be hit.
  - **The rifleman stops to aim** half the time. Strafing without a stop dodged every dagger (it flies about 10 ticks at 12-18 blocks, and the box is 0.6 wide), so he was never hit on any seed.
  - **The seed is 24.** Across seeds 0-29, seeds 15 and 24 show all eight moves and hits on both players in one fight. The others miss one to four moves, because the fight is short. Any change to Margit's numbers changes the fight, so the seed may need to be picked again. The every-seed test keeps what holds for all seeds.
- **Observations for the owner's tuning (no number changed):**
  - The scripted duo never misses and kills him in 797-976 ticks (40-49 s), with 6 or 7 kneels and a critical in almost every kneel.
  - The sword breaks his stance after 5 hits (16.5 stance each against 80), about every 7 s.
  - Design doc 7.5 expected longer fights, with 70% of shots landing and half the time spent dodging.
  - If he kneels too often in game, raise `stance.points` or lower `resist.melee.stance` in `margit.rs`.
- **Owner's steps:** none in game: the crate is not in the game yet. No kit line.

## Review of segment f1

### 5. Margit rises at once after a critical

After a5983466.

- **Defect:** a critical moves the kneel clock to its last 20 ticks (tick 60), so that he rises at once with the 20-tick recovery of design doc 7.3. The kneel clip's poses were Kneel at 6, Kneel at 72 and Rise at 80, so after a critical he stayed fully down for 12 more ticks and rose in the last 8.
- **Changed:** the kneel clip's last Kneel key is at 60 (`kneel.length - critical.recovery`), so the rise takes the last 20 ticks of every kneel, and a critical lands where it starts.
- **Tests:** `margit::tests::a_critical_lands_where_his_twenty_tick_rise_starts`. `test -p elden_boss --locked`: 52 green.
- **Owner's steps:** none in game: the crate is not in the game yet. No kit line.

### 6. Margit's daggers form and his cane slam lands with the sounds of his design

After abe00dff.

- **Defect:** design doc 14.1 gives every golden weapon that forms `block.amethyst_block.chime` and `block.beacon.activate`, and every slam `entity.generic.explode` quiet and low beside the smash. The golden dagger formed with the chime only (`dagger_throw` at 2, `backstep_daggers` at 14), and the cane slam had no explosion under its smash, though the leap slam has one.
- **Changed:** `margit.rs`: the beacon (0.6, pitch 1.6) at both dagger forms, and the quiet low explosion at the cane slam's tick 29. No timing or random roll changed, so the whole fight's seeds play as before.
- **Tests:** `margit::tests::every_golden_weapon_forms_with_chime_and_beacon_and_every_slam_booms`: at each conjure's first tick the chime and the beacon play, and each slam burst comes with the explosion and a smash. `test -p elden_boss --locked`: 53 green.
- **Owner's steps:** none in game: the crate is not in the game yet. No kit line.
