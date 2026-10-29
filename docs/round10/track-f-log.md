# Track F log: the first Elden Ring boss, Margit

Branch `track-f`, worktree `C:\Dev\mc-wt\track-f`, target dir `C:\iw4l-wt\track-f` for segment f1 and `D:\iw4l-wt\track-f` for segment f2. Briefs: `docs/round10/track-f.md` and the design doc `docs/ELDEN-RING-BOSSES.md`; segment f2 also follows track D0's plug-in checklist (`docs/round10/track-d0.md`, D0-3).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

Segment f1 (groups `er-core` and `er-rules`) builds the pure crate `crates/elden_boss` and touches no shared game file: only the workspace member list in the root `Cargo.toml` and `Cargo.lock`. Nothing of it is in the game yet, so these commits have no owner steps in game and no kit line.

## Combined kit

None for segment f1: no commit of it has owner steps in game, so no kit line is needed.

Segment f2 so far (commits 7 to 12), 20 stacks:

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:shield,minecraft:cooked_beef*32,minecraft:iron_sword,minecraft:zombie_spawn_egg*4,minecraft:iron_golem_spawn_egg*2,minecraft:wolf_spawn_egg*2,minecraft:bone*16,minecraft:skeleton_spawn_egg*2,minecraft:enderman_spawn_egg*2,minecraft:silverfish_spawn_egg*4,minecraft:cobblestone*256,minecraft:iron_pickaxe
```

## Protocol

Nothing on the wire changed yet.

## Vendored files

Files under `third_party/minecraftoss` this track patched or added. Files that track D0 added count as patched here.

- Patched (6): `entities/Cargo.toml`, `entities/src/world.rs`, `entities/src/world/families.rs`, `entities/src/world/families/bosses.rs`, `entities/src/world/families/pests/probe.rs` (test only), `entities/src/world/families/tests.rs` (test only).
- Added (2): `entities/src/world/families/bosses/elden_bosses.rs`, `entities/src/world/families/bosses/elden_bosses/arena.rs`.
- `entities/Cargo.toml` depends on `elden_boss` (a path dependency on `crates/elden_boss`): the first vendored crate that depends on a fork crate. The root `Cargo.lock` covers it, as the minecraftoss crates have no workspace of their own.

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

## Final checks of segment f1

After fac9df13.

- Touched crates (`git diff --stat r10-f1-base..HEAD`): only `elden_boss`, plus its line in the root `Cargo.toml` and `Cargo.lock`.
- `clippy -p elden_boss --locked --all-targets -- -A clippy::approx_constant`: clean.
- `test -p elden_boss --locked`: 53 green.
- Every new `.rs` file passes `rustfmt --check --edition 2024` on its committed content, and none has a comment.
- **Owner's steps:** none in game. No kit line.

## Group er-entity

Segment f2 starts here, on cea44a86 (minecraft-survival with D0's registry and E1 merged into `track-f`). Margit is a registered family in group `bosses` (hit-key byte 96, `docs/round10/mobs.md`), by D0's checklist. The sites D0's checklist does not cover are listed per commit under "Outside the registry".

### 7. Registered Minecraft mobs can stay out of other mobs' targets

After cea44a86.

- **Why:** design doc 16 and the brief (F2, commit 1): mobs ignore Margit, so iron golems, tame wolves, withers and the like never pick him. Every mob AI picks its targets from `EntityWorld::mob_candidates`, and D0's generic arm there lists every registered mob. D0's checklist says a missing hook goes into the root file in a commit of its own.
- **Changed (vendored entities crate):**
  - Root `world/families.rs`: the hook `Family::targetable(kind)` (default true) next to `enemy`; `family_group!` generates `untargetable(kind)` next to `enemy`; the root `targetable(kind)` next to `enemy`.
  - `world.rs` `mob_candidates`: D0's generic arm adds only the registered mobs whose kind is targetable.
  - The test probe gets a third kind, `minecraft:registry_probe_shy`: an enemy that is not targetable.
- **Tests:** `world::families::tests::a_registered_mob_left_out_of_targets_is_never_hunted`: the shy probe is not targetable but is an enemy, is not among `mob_candidates`, and an iron golem 8 blocks away never targets it in 40 ticks; it is still in `mob_ids`, and a player's hit still hurts it. `test -p minecraftoss-entities --locked --lib`: 438 passed.
- **Decisions:**
  - A static hook per kind, like `enemy`, not a per-mob trait: `mob_candidates` runs on every AI step of every mob, and for a boss the answer depends on the kind only.
  - Only other mobs' targeting changes. Players, arrows, explosions and MW2 shots find mobs through their own walks (`arrow_targets`, `blast_target`, `mob_on_ray_except`, the hit boxes), so they still reach a mob that is not targetable.
- **Owner's steps** (nothing new is visible: no family answers false until the next commit, and Margit cannot be summoned until group `er-arena`; the steps check that mobs still pick their targets). Release build, survival, Normal, at night or under a roof, flat ground in the Overworld.
  1. Use a zombie egg, then an iron golem egg 8 blocks from it: the golem walks to the zombie and throws it up.
  2. Tame a wolf with bones, then hit a new zombie once with the sword: the wolf attacks the zombie.
  3. Use a skeleton egg 10 blocks away: it shoots you. Use an enderman egg and look at its head: it comes for you.

```
IW4L_MINECRAFT_KIT=minecraft:zombie_spawn_egg*4,minecraft:iron_golem_spawn_egg*2,minecraft:wolf_spawn_egg*2,minecraft:bone*16,minecraft:skeleton_spawn_egg*2,minecraft:enderman_spawn_egg*2,minecraft:iron_sword,minecraft:shield,minecraft:cooked_beef*32
```

### 8. Margit, an Elden Ring boss, lives in Minecraft worlds and is saved like vanilla

After 522362cb.

- **Changed (vendored entities crate):**
  - `Cargo.toml`: `elden_boss = { path = "../../../crates/elden_boss" }` (see "Vendored files"). `Cargo.lock` gained the edge from one `check` without `--locked`.
  - New `world/families/bosses/elden_bosses.rs`, registered in `world/families/bosses.rs` (`elden_bosses: elden_bosses::EldenBoss`, key 96, kinds `elden_boss::IDS`, so `iw4l:margit`):
    - `EldenBoss`, the state beside D0's `MobCore`: his `BossDef`, his `Brain`, his anchor, his rest yaw and the brain's last walk target. `boss(def, position)` builds him: 1.4 x 3.0 with step height 1.0 (the def's numbers), eye height 0.85 of his height (2.55, the vanilla default), the def's health, persistent, and a brain seeded from his position.
    - `EntityWorld::spawn_elden_boss(def, position, yaw, anchor, no_ai)`, `elden_bosses()` and `elden_boss_mut(id)`.
    - Hooks: category `monster` (the default, so Peaceful and the census treat him as the wither), despawn kept and not allowed in Peaceful, `targetable` false (commit 7), knockback resistance 1, experience 0 (group `er-fight` gives his reward), and a `hurt` that refuses every hit until the next commit.
    - His tick: first `steady` (potion effects cleared, horizontal velocity and fall distance dropped), then D0's `family_base_tick` (fluids, burning, suffocation, drowning, the death clock). With AI, the brain steps on a `Sense` of the eligible players (alive, not spectators, attackable). His yaw and head look come from the `Act`, the walk target is kept for group `er-arena`, and each `Cue::Sound` of the `Act` becomes a `Voice::Event`. Alive, he only falls (no walk yet, no fall damage). Every voice is turned into a `Voice::Event` from his def (an ambient roll only while he is awake), his core health and maximum follow the brain's, and D0's `family_after_move` ends the tick.
- **Changed (`minecraft_terrain`):**
  - `Cargo.toml`: `elden_boss = { path = "../elden_boss" }`.
  - New `elden_save.rs`, the carrier of design doc 12.1:
    - `claims(tag)`: a `minecraft:marker` whose `data.iw4l.boss` names a known boss.
    - `spawn(world, tag, common)`: the boss at his saved position, facing his saved `Rotation`, with the UUID's random, his fire and `NoAI`; then `read`.
    - `read(mob, tag)`: the anchor from `data.iw4l.anchor` (his position if it is missing), the rest yaw from `data.iw4l.yaw` (his facing if missing), then `Brain::reset`: `Return`, full base health, phase 1. It never reads `Health`.
    - `fields(mob)`: `id` `minecraft:marker` and `data: {iw4l: {boss, anchor: [x, y, z], yaw}}`, which replace the `id` that `mob_tags_where`'s `add` writes.
    - `summon(world, def, position, y_rot, nbt, uuid)`: a fresh, asleep boss with his anchor and rest yaw where and as he is summoned, the given UUID, the NBT's `NoAI`, and the NBT's `Health` if there is one (`Brain::set_health`, so the wake keeps its share). It returns his id and his carrier tag (`Pos`, `Rotation`, `UUID` and `fields`).
  - New `families/bosses/elden_bosses.rs` (`mod elden_bosses;` and the type in `families/bosses.rs`): `TerrainFamily` with tracking range 10 (F6's number), no textures and nothing drawn yet (group `er-look`), a shadow of radius 1.0 (design doc 14.1, so he shows where he stands until then), and `read` and `fields` through `elden_save`.
- **Outside the registry** (D0's checklist does not cover these; brief F2, commit 1):
  - `crates/minecraft_terrain/src/lib.rs`: `pub mod elden_save;` beside `wither_save`.
  - `crates/minecraft_terrain/src/server_mobs.rs` `spawn_saved`: a `minecraft:marker` arm beside the wither's asks `elden_save::spawn`. Every other marker gives `None` there and stays dormant, saved as it came. A friend's copy comes through the same call (`spawn_tracked`), so the friend gets the anchor from the same tag.
  - `crates/minecraft_terrain/src/server.rs` `ServerSim::summon`: a boss id goes to `elden_save::summon` before `level.summon_mob` (which knows no `iw4l:` id), with a fresh UUID from the level (`SpawnLevel::next_uuid`), and his tag goes into `mob_tags` like every summon's, so saving and `MobAdd` see him. On Peaceful it refuses with vanilla's message ("You can not summon a monster in peaceful mode", which the command shows as "Monsters cannot be summoned in Peaceful difficulty").
  - `/summon` already accepts and suggests `iw4l:margit`: D0 made the command's list `simulated_kinds()`, which holds every registered kind.
- **Moved from group `er-arena` (brief F2, commit 3, its first bullet):** the summon. D0's registry makes every registered kind summonable, and the gated terrain test `command_summons_do_not_reach_the_egg_results` summons each of them, so Margit's kind fails it without the summon. Group `er-arena` keeps the death names, the rest of that commit's tests and his walking.
- **`docs/round10/mobs.md`:** a section for track F (group `bosses`) with Margit's row, between the hit-key table and D0's section, so no other track's rows are next to it; the hit-key row names byte 96.
- **Tests:**
  - `world::families::bosses::elden_bosses::tests` (entities):
    - `margit_is_a_persistent_boss_of_his_size_and_kind` (kind, key, size, step, eye, 800 health, persistent, the traits, the census, not targetable, not an enemy, not among `mob_candidates`)
    - `peaceful_removes_margit`
    - `margit_never_despawns_far_from_players` (500 blocks away and idle for 10000 ticks, persistent or not)
    - `margit_sleeps_until_a_player_comes_within_24_blocks_and_then_watches_him` (asleep and silent with the player at 30 blocks, awake at 10, his target and head look on the player's eyes, standing on the ground)
    - `margit_speaks_in_named_events_and_ignores_potions` (the golden mist sound of his rise and the evoker's ambient as events, no `margit` event name, all in `hostile_volume`; poison is cleared)
  - `elden_save::tests` (terrain):
    - `margits_saved_tag_is_a_vanilla_marker` (`id` is `minecraft:marker`, `data.iw4l` holds the boss, the anchor and the rest yaw, every `id` in the tag is a `minecraft:` one, the look tag keeps `data`)
    - `a_hurt_margit_loads_at_full_health_and_walks_home` (awake, at 300 health and 8 blocks from his anchor, saved with `mob_tags_where` and loaded into a fresh world: where he was, 800 health, phase 1, `Return`, the same anchor, and his brain walks him home; saved again, the same carrier)
    - `a_plain_marker_stays_dormant` (a bare marker, one with other data, one with an unknown boss, and an armour stand with the carrier's data)
    - `a_saved_health_is_never_read` (a marker with `Health:5f` and no anchor loads at 800 with his anchor where he stands and his facing as the rest yaw; a tag with his own id loads through D0's generic arm and the same `read`)
    - `a_summoned_margit_sleeps_with_his_anchor_there_and_a_given_health` (asleep, anchor and rest yaw at the summon, 800 health, the carrier tag with his UUID; `{Health:401.0f,NoAI:1b}` gives 401 and no AI)
  - `server::tests` (terrain, with the 26.3 data): `summon_makes_one_asleep_margit_with_his_anchor_there` (one asleep Margit, nothing dormant, his anchor and facing, his marker in `mob_tags` with his UUID; a second summon with `Health:401.0f` has 401), `summoning_margit_on_peaceful_is_refused`.
- **Decisions:**
  - **Brain ticks** are his `tick_count`, so a hurt between two ticks has the tick of the last step.
  - **`data.iw4l.yaw` is his rest yaw**, the facing he was summoned with, fixed for his life. D0's family `VOLATILE` keys are looked up by the tag's `id`, which is the marker's, so a field of `data` that changed while he lives would resend his look tag every 10 ticks. His current facing is in `Rotation`.
  - **The carrier's `data` is written whole:** other keys a vanilla command put in a marker's `data` are not kept. The entity crate cannot hold an NBT tag, and only our summons make these markers.
  - **He moves only by his own will:** knockback (resistance 1), blasts and pushes never move him, because `steady` drops his horizontal velocity at the start of each tick. Group `er-arena` moves him with collision moves (brief F2, commit 4), not by velocity.
  - **Ambient voices only while he is awake:** an asleep boss is silent.
  - **Every `Cue::Sound` of the `Act` plays already** (the kneel's stunned sound, the critical, his rise's mist and each move's sounds). Brief F3, commit 3 lists this; group `er-look` only adds the bursts and the steps.
  - **Until group `er-arena`:** every eligible player is reachable, the ground under a player is the player's feet, and no fade spot is found (`Seen.room` is `None`).
  - **A tag with his own id** (`iw4l:margit`, never written by the game) still loads through D0's generic arm, with the same `read`.
  - **His death's loot table** is `iw4l:margit` (D0's rule for monsters), which does not exist, so `EntityLootBook::roll` gives nothing. Group `er-fight` gives the drops.
- **Runs:** `test -p minecraftoss-entities --locked --lib`: 443 passed. `test -p minecraft_terrain --locked --lib`: 1437 passed and 3 ignored; the known flaky `sections::tests::background_full_update_matches_the_synchronous_one` failed once under load and passed when run alone. `check -p launcher --locked`: green, with only the old render_frame and skate warnings. The three new files pass `rustfmt --check --edition 2021` with empty output on their staged content.
- **Left for later:** `sleep::MONSTERS` (survival) does not list `iw4l:margit`, so a player may sleep beside him. Vanilla's `Monster.isPreventingPlayerRest` is true for every monster.
- **Owner's steps.** He is not drawn until group `er-look` and does not walk until group `er-arena`: until then he is a round shadow about 2 blocks wide, and you hear him. In this commit nothing hurts him yet (the next commit). Release build, survival, Normal, by day, flat open ground in the Overworld. Name each screenshot by its label, `f2-8-1.png`.
  1. Stand still and type `/summon iw4l:margit ~ ~ ~30`: the chat confirms the summon, and a round shadow about 2 blocks wide lies on the ground 30 blocks away. He sleeps: no sound. **Screenshot f2-8-1** with the shadow in view.
  2. Walk toward the shadow: once you are within about 24 blocks you hear a deep illusioner mirror sound (golden mist) as he rises, and then every few seconds a deep evoker mutter.
  3. Save and quit, and load: the shadow is where it was. Walk off 40 blocks and come back: it is still there.
  4. `/difficulty peaceful`: the shadow goes. `/summon iw4l:margit ~ ~ ~5`: the chat says "Monsters cannot be summoned in Peaceful difficulty". `/difficulty normal`.
  5. Use a silverfish egg and a zombie egg (under a roof), save and quit, load: both are there and come at you, as before.

```
IW4L_MINECRAFT_KIT=minecraft:silverfish_spawn_egg*4,minecraft:zombie_spawn_egg*4,minecraft:iron_sword,minecraft:shield,minecraft:cooked_beef*32
```

### 9. Minecraft weapons hurt Margit with no hurt cooldown, and his stance breaks

After 03534171.

- **Changed (vendored entities crate, `world/families/bosses/elden_bosses.rs`):** his `hurt` hook turns each hit that D0's generic arms hand him into an `elden_boss::Source` and calls `take`:
  - `player_attack` (D0's arm in `hurt_by_player`, after vanilla charge, crit, sweep and enchantments): `Melee`, from where that player stood at his last tick (`EldenBoss.seen`, kept from the level's player candidates), for the critical's front check;
  - a mob's blow (`mob_attack`): `Melee` from nowhere near, so never a critical;
  - arrows and other projectiles (D0's `apply_arrow_hit` arm, and E2's thrown things once merged): `Projectile`;
  - level explosions (TNT, creepers, crystals, beds, anchors, through D0's `hurt_by_blast` arm): `Explosion`;
  - fire, lava, cactus, drowning, suffocation and the other generic and hazard types: `Environment`;
  - falls and magic (harming, wither and poison damage): refused, as design doc 7.1 has no fall damage and no potion effects on him.
  - The player id is the attacker's when the hit names a player (melee, a player's arrow, a player's blast), so threat, `hurt_by` and the reward follow it.
- **`take(mob, source, player)`** (public, for group `er-guns`'s bullets and blasts):
  - Only `Environment` passes the vanilla hurt cooldown first: `DamageState::hurt_absorbed` on a copy decides what passes (all of it, or the increase over the last hit), and only the cooldown fields are kept. Every other hit skips it.
  - `Brain::take` deals the damage. A hit he does not take (asleep, returning, fading home or dead) is refused: no red flash, no credit.
  - The damage state then records the hit with the cooldown held open and put back after, so player hits never touch the cooldown that fire uses. This keeps vanilla's red flash (`hurt_ticks`, `hurts`), the death record (`killed_by_player`, the place, burning) and the kill credit that the callers give (`credit`).
  - His core health and maximum are the brain's after every hit. A killing hit takes all his core health, so the death is recorded exactly when the brain says he died.
  - The hurt voice (`entity.illusioner.hurt`, pitched down, design doc 14.1) plays at most once every 10 ticks, as a `Voice::Event`.
- **His death** joins the death list through D0's `take_deaths` with no drops and no experience yet (group `er-fight`). The death clip's cue (`entity.evoker.death`) and the kneel's and the critical's sounds (`entity.ravager.stunned`, `entity.player.attack.crit`) play through the `Act`'s cues (commit 8). Group `er-look` adds their particles.
- **Tests:**
  - `world::families::bosses::elden_bosses::tests` (entities):
    - `an_asleep_margit_takes_no_hits` (no hurt, no red flash)
    - `an_arrow_and_tnt_hurt_margit` (a player's arrow takes its 6 and credits player 1, with no knockback; a player's TNT-sized blast takes half its damage in health and all of it in stance, and does not move him)
    - `standing_in_fire_hurts_him_once_per_10_ticks` (4 hits of 1 in 40 ticks of fire; a sword hit right after still takes 11 and 16.5 stance)
    - `a_sword_hit_from_the_front_on_a_kneeling_margit_is_a_critical` (5 full sword hits break his stance; from behind x1.5 is 16.5, then from the front a critical of 33; the stunned and crit sounds play)
    - `a_killing_blow_credits_the_player_and_joins_the_death_list` (killed by player 1, table `iw4l:margit`, no experience or drops, the death sound, gone after 20 ticks)
    - `falls_and_potions_do_not_hurt_margit` (a 20-block fall, a fall hit, magic and a harming splash take nothing; lava takes 4)
  - `families::bosses::elden_bosses::tests` (terrain): `two_players_sword_hits_in_one_tick_both_count` (through `mob_actions::attack`, as `ServerSim::mob_action` runs it: two fighters give 1280 health, both hits take 11 in the same tick, both players are on his hurt list, and one hurt voice plays for both).
- **Runs:** `test -p minecraftoss-entities --locked --lib`: 449 passed. `test -p minecraft_terrain --locked --lib`: 1439 passed and 3 ignored. `test -p minecraft_survival --locked`: 776 + 1 passed (survival is not touched; its hit keys, boxes and `/summon` list read the registry). `check -p launcher --locked`: green, with only the old render_frame and skate warnings. Both changed new files pass `rustfmt --check --edition 2021` with empty output on their staged content.
- **Decisions:**
  - **The melee front check uses where the player stood at his last tick:** D0's `Hurt` carries the attacker but not where it stands, and his own state can keep the players' positions, so no shared struct changes. A tick is at most 0.3 blocks of running against the 4-block, 60-degree check.
  - **A mob's blow counts as melee from nowhere**, so it breaks stance at x1.5 but is never a critical. Mobs do not target him (commit 7), so only a stray hit can do this.
  - **Magic is refused** (design doc 7.1, "Potion effects: none apply to him"): harming splashes, the wither effect and poison. His potion effects are also cleared every tick (commit 8).
  - **The red flash** shows on every hit he takes (bullets included, which reach him as melee until group `er-guns`), but his hurt voice plays at most once every 10 ticks, as vanilla's cooldown would allow.
  - **Until group `er-guns`**, MW2 bullets and blasts reach him through the survival crate's generic mob path, as player melee hits at the mob scale (x 0.2 for bullets) and level blasts (x 0.8). So a bullet breaks stance at x 1.5, and a close shot from the front at a kneeling Margit is a critical. Group `er-guns` routes them to `Bullet` and `Blast`.
- **Owner's steps.** He is still a round shadow about 2 blocks wide until group `er-look`, and he does not walk until group `er-arena`, so you hear the hits. Release build, survival, Normal, by day, flat open ground in the Overworld. Name each screenshot by its label, `f2-9-1.png`.
  1. `/summon iw4l:margit ~ ~ ~6`: you hear him rise. Stand 2-3 blocks from the shadow and hit the space above it with the netherite sword at full charge: each hit lands, with a deep illusioner hurt sound at most every half second. **Screenshot f2-9-1** while hitting.
  2. Keep hitting at full charge without a pause: after the fifth hit a ravager's stunned sound plays (his stance broke, he kneels for 4 seconds). Hit him once more from where you stand (he faces you, his target): a critical sound.
  3. Step back 10 blocks and shoot him with the bow: the hurt sound. Put a TNT 3 blocks from the shadow and light it: the hurt sound, and the shadow does not move.
  4. `/summon iw4l:margit ~ ~ ~6 {Health:20f}`, walk to it and hit it twice: a deep evoker death sound, and the shadow is gone a second later. Nothing drops yet.

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:shield,minecraft:cooked_beef*32
```

## Group er-arena

Brief F2, commits 3 and 4. The summon itself landed in commit 8 (see "Moved from group `er-arena`" there), so this group adds the registry hook a boss's name needs (commit 10), the death names and the summon's remaining tests (commit 11), and his walking, leaping and fading on blocks (commit 12).

### 10. Registered Minecraft mobs can carry their own name

After 439e2172.

- **Why:** brief F2, commit 3 wants death lines to name Margit ("Sos was slain by Margit, the Fell Omen"). Vanilla names a mob through its `entity.minecraft.<kind>` language key, and `iw4l:margit` has none, so `death.rs` would print the bare id. D0's checklist lists `death.rs` among the files never edited for one family, and says a missing hook goes into the registry's root file in a commit of its own. This is that hook.
- **Changed (vendored entities crate):**
  - Root `world/families.rs`: the hook `Family::display_name(kind) -> Option<&'static str>` (default `None`, so every vanilla kind keeps its language key) next to `targetable`; `family_group!` generates `display_name(kind)` next to `untargetable`; the root `display_name(kind)` (the first group that knows the kind answers) next to `targetable`.
  - The test probe names its shy kind "Shy Probe".
- **Tests:** `world::families::tests::a_registered_mob_may_carry_its_own_name` (the shy probe has its name; the other probe kinds, a vanilla kind and an unknown kind have none). `test -p minecraftoss-entities --locked --lib families::tests`: 18 passed.
- **Decision:** a static hook per kind, like `enemy` and `targetable`: a death line or a summon line knows only the kind, not the mob.
- **Owner's steps:** nothing changes in game until the next commit uses the hook. No kit line.

### 11. Minecraft deaths by Margit name him, and /summon knows him

After bcb20591. Brief F2, commit 3 (its summon part landed in commit 8).

- **Changed:**
  - Vendored entities crate, `world/families/bosses/elden_bosses.rs`: his `display_name` (commit 10's hook) is his def's name, "Margit, the Fell Omen".
  - `crates/minecraft_survival/src/death.rs`: `entity_name` (the death line and the death screen's killer) takes a registered mob's own name before the `entity.minecraft.<kind>` key, and the new `entity_text(kind)` gives the chat text: the name as a literal for a mob that has one, else vanilla's entity name text (resolved from each viewer's language). `death_text` (what friends receive) uses it.
  - `crates/minecraft_survival/src/commands.rs` `summon_answers`: the summon line uses `entity_text`, so it reads "Summoned new Margit, the Fell Omen" instead of the bare id.
  - Neither line names a family: both ask the registry (D0's rule for `death.rs`).
- **Already there:** `/summon` accepts and suggests `iw4l:margit` (D0's `simulated_kinds()`), Peaceful refuses him, a summon's `Health` is used and his tag goes into `mob_tags` (commit 8). This commit adds the tests the brief lists for them.
- **Tests:**
  - `death::tests::a_death_by_margit_names_him` (survival): his blow gives "%1$s was slain by Margit, the Fell Omen" with him as the killer, the friend's text resolves to "Sos was slain by Margit, the Fell Omen", and a zombie keeps its language key.
  - `commands::tests::the_summon_command_knows_iw4l_margit` (survival): the registry lists him, `/summon iw4l:m` suggests `iw4l:margit`, and `/summon iw4l:margit 0 30000000 0` gets past the registry to the position check, where `/summon iw4l:nothing` stops at the registry.
  - `server::tests::a_summoned_health_is_used_and_a_saved_one_is_not` (terrain, with the 26.3 data): `{Health:401.0f}` gives 401; saved with `mob_tags_where`, his marker carries `Health:401` (vanilla's marker ignores it), and loaded into a fresh world he has 800 and the same anchor.
  - `margit_is_a_persistent_boss_of_his_size_and_kind` (entities) also checks his name.
  - `peaceful_removes_margit` and `margit_never_despawns_far_from_players` (the brief's server list) are commit 8's entity tests.
- **Runs:** `test -p minecraft_survival --locked --lib margit`: 2 passed. `test -p minecraft_terrain --locked --lib` with `margit` and `summoned_health`: 6 passed. `test -p minecraftoss-entities --locked --lib elden`: 11 passed. `check -p launcher --locked`: green.
- **Decisions:**
  - **The summon line names him too:** vanilla's "Summoned new %s" shows the entity's name, and his id has no language key. One generic call, as in `death.rs`.
  - **Holy hits** will die as "%1$s was killed by %2$s using magic" (`indirectMagic`) through the same name once group `er-fight` lands his hits.
- **Owner's steps.** He is still a round shadow and cannot hurt you until groups `er-look` and `er-fight`, so a death by him can't be tested yet. Release build, survival, Normal, by day, flat open ground in the Overworld. Name each screenshot by its label, `f2-11-1.png`.
  1. Type `/summon iw4l:` and press Tab: `iw4l:margit` is offered.
  2. Run `/summon iw4l:margit ~ ~ ~30`: the chat says "Summoned new Margit, the Fell Omen". **Screenshot f2-11-1** of the chat.
  3. `/summon minecraft:zombie ~ ~ ~5` under a roof: the chat says "Summoned new Zombie", as before.

Kit: none needed for these steps.

### 12. Margit walks, turns and leaps on blocks and stays in his arena

After 86d53bb1. Brief F2, commit 4.

- **Changed (vendored entities crate):**
  - New `world/families/bosses/elden_bosses/arena.rs` (`pub mod arena;` in `elden_bosses.rs`), between the brain and the blocks:
    - **The `Sense`** (`seen`): every eligible player (alive, not a spectator, attackable) with the ground under their feet from the blocks (`ground_under`: the highest collision top under the player's 0.6-wide footprint, at most 8 blocks down; 8 below the feet when there is none, so a ring never reaches a player in the air), and `reachable` and `room` from his last survey.
    - **The survey** (`survey`), at most every 10 ticks and only while he is awake: for each eligible player within 40 blocks of his anchor, a walk path (`plan_walk_path` on a scratch `GroundNavigation`, reach 1) decides `reachable`; while he is in the air, the last answer stays. For a player no path reaches, `room` is the nearest standing spot within 3 blocks across and 2 up or down of their feet (`fade_across`, `fade_rise` of his def) whose box does not overlap the player's.
    - **A standing spot** (`stands`): his 1.4 x 3.0 box meets no block, and the block under his centre has its top at his feet.
    - **Walking** (`act.walk`): a `GroundNavigation` path to the brain's point (reach 0), replanned when the point moves to another block, at most every 10 ticks while a path is under way. Each tick he follows it with vanilla's ground travel at speed 0.3 (his def's), heading along the path while his body keeps the brain's facing, so he circles his target facing it. His walk profile: a monster's (no fire maluses), step 1.0, a fall of at most 3, follow range 40 (640 nodes), no floating.
    - **Root motion** (`act.motion`): a step or a hop is that exact sideways move through the collision move, with gravity; a leap is the brain's displacement, up and down, through the same move. A leap that meets a wall drops: from that tick on, the leap's motion is ignored and he falls. Moves stop his path.
    - **The fade** (`act.fade`): he appears on a standing spot near the point (the brain's `room`, which is one already, or his anchor when he fades home), or on the point itself when there is none.
  - `elden_bosses.rs`: his state gains the path, the survey, his stride and the leap's drop flag; `think` takes the level, keeps the `Act`'s hits and daggers on his state (`hits`, `throws`, replaced on every tick his brain steps, so a dying Margit holds none from before his death) for group `er-fight`, and returns the `Act`; his tick hands the `Act` to `arena::carry_out` (he only fell before). `steady` now keeps his own stride from the last tick and still drops anything else, so pushes and blasts never move him.
- **Tests** (`world::families::bosses::elden_bosses::arena::tests`, flat test worlds as the wither's tests use):
  - `margit_walks_to_a_player_10_blocks_away` (between moves he walks more than 3 blocks, never faster than 0.25 a tick, and comes within 5.5 blocks of the player)
  - `margit_stops_at_his_leash` (with the player 34 blocks from his anchor, he goes at least 21 and never more than 25 from it, keeps the fight and faces the player)
  - `a_leap_over_a_one_block_wall_lands_beyond_it` (a forced leap rises above 66 and lands on the ground beyond the wall; a 6-high wall stops him and he drops to its foot)
  - `margit_resets_when_everyone_leaves_for_5_seconds` (hurt and moved, the player 60 blocks away: awake for 99 ticks, then `Return` at 800 health in phase 1, and he walks home and sleeps within a block of his anchor, with no fade)
  - `margit_fades_inside_a_wall_ring_he_cannot_reach` (a player inside a 6-high ring 4 blocks out: no hit lands, after at least 100 stuck ticks he fades with the player unreachable and a room found, and appears on a standing spot inside; then a path reaches the player)
  - `a_player_in_a_two_deep_hole_beside_him_is_still_hit` (ground 62 in the hole, never in the hole himself, a hit for the player, and no path ends in the hole)
  - `a_standing_spot_needs_a_block_under_him_and_room_for_his_box` (ground, air, inside the ground, a block in his box; no room beside a player on a 1 x 1 pillar; room on flat ground is beside him)
- **Runs:** `test -p minecraftoss-entities --locked --lib`: 457 passed, the older Margit tests unchanged. `test -p minecraft_terrain --locked --lib`: 1440 passed and 3 ignored. `test -p minecraft_survival --locked`: 778 + 1 passed. `check -p launcher --locked`: green, with only the old render_frame and skate warnings. The hits and daggers then moved from `carry_out` into `think` (so a dying Margit holds none): `test -p minecraftoss-entities --locked --lib` 457 passed again, `test -p minecraft_terrain --locked --lib` with `elden` and `margit` 11 passed, `check -p launcher --locked` green. `arena.rs` and `elden_bosses.rs` pass `rustfmt --check --edition 2021` with empty output on their staged content.
- **Decisions:**
  - **He walks facing his target:** vanilla's `MoveControl` turns a mob to where it walks. Margit's facing belongs to the brain (it turns 30 degrees a tick inside tracking windows), so the path gives only the heading of his steps. The speed and the travel physics are vanilla's ground travel, about 0.2 blocks a tick at full stride.
  - **His stride survives the tick:** commit 8 dropped all sideways velocity at the start of each tick, which would have held him to one tick's acceleration (0.09 a tick). He now keeps the velocity his own walking left and drops anything else.
  - **Reach 1 for "a path reaches them", reach 0 for walking:** vanilla's `moveTo(entity)` uses 1. Walking aims at the exact block, because a 1.4-wide mob stands at a node's corner (`getEntityPosAtNode` adds 1.0) and his home check is 1 block.
  - **A standing spot needs the block under his centre:** an edge would do for vanilla's on-ground test, but then a player on a 1 x 1 pillar would always have room beside them, and the design wants daggers there.
  - **The ring in the fade test is 4 blocks out, not 3:** at 3 out his sword sweep (4.5 blocks) reaches the player through the wall, so his hits land and the stuck clock never runs. His strikes have no line of sight (see "Left for later").
  - **The stuck clock counts from the wake:** the brain counts while he is awake with an unreachable target, the 30 wake ticks included, so the fade can come 100 ticks after the wake starts.
- **Left for later:**
  - His strikes ignore walls: an arc or a ring hits a player behind a block when the shape reaches. Group `er-fight` applies the hits and can check a line from his chest to the player first; the brain would then need that answer for its stuck rule.
  - He walks in water and lava with the dry ground travel.
- **Owner's steps.** He is still a round shadow about 2 blocks wide until group `er-look`, and his swings and daggers hurt nobody until group `er-fight`: watch the shadow and listen. Release build, survival, Normal, by day, flat open ground in the Overworld, F3 on for distances. Name each screenshot by its label, `f2-12-1.png`.
  1. `/summon iw4l:margit ~ ~ ~10`: you hear him rise, and about a second and a half later the shadow comes toward you. Walk around it at 6-10 blocks: it follows and circles you at about walking pace. **Screenshot f2-12-1** with the shadow near you.
  2. Step back to about 12 blocks and stand still: within a few seconds a ravager roar, the shadow jumps, and it lands 1-2 blocks in front of you with a mace smash and a low boom.
  3. Walk straight away until you are 30 blocks from where he appeared: the shadow stops about 24 blocks from its spot and stays there while you stand. **Screenshot f2-12-2** with the shadow and your coordinates.
  4. Go on to 45 blocks, wait 5 seconds, then come back to 30 blocks: the shadow walks back to where it appeared and goes quiet (asleep). Come within 24: it rises again.
  5. Build a ring of cobblestone 6 high around yourself, 4 blocks out from your block on every side (a 9 x 9 square), with the shadow outside: it cannot get in, and after about 5 seconds you hear the mirror sound and the shadow appears inside the ring beside you. **Screenshot f2-12-3** inside the ring. Break out with the pickaxe.
  6. Pillar up 5 blocks on a single column of cobblestone: the shadow stays below and does not fade up to you.

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*256,minecraft:iron_pickaxe,minecraft:cooked_beef*32
```
