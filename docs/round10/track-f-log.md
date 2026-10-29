# Track F log: the first Elden Ring boss, Margit

Branch `track-f`, worktree `C:\Dev\mc-wt\track-f`, target dir `C:\iw4l-wt\track-f` for segment f1 and `D:\iw4l-wt\track-f` for segment f2. Briefs: `docs/round10/track-f.md` and the design doc `docs/ELDEN-RING-BOSSES.md`; segment f2 also follows track D0's plug-in checklist (`docs/round10/track-d0.md`, D0-3).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

Segment f1 (groups `er-core` and `er-rules`) builds the pure crate `crates/elden_boss` and touches no shared game file: only the workspace member list in the root `Cargo.toml` and `Cargo.lock`. Nothing of it is in the game yet, so these commits have no owner steps in game and no kit line.

## Combined kit

None for segment f1: no commit of it has owner steps in game, so no kit line is needed.

Segment f2 (commits 7 to 16), 20 stacks:

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:shield,minecraft:cooked_beef*32,minecraft:iron_sword,minecraft:zombie_spawn_egg*4,minecraft:iron_golem_spawn_egg*2,minecraft:wolf_spawn_egg*2,minecraft:bone*16,minecraft:skeleton_spawn_egg*2,minecraft:enderman_spawn_egg*2,minecraft:silverfish_spawn_egg*4,minecraft:cobblestone*256,minecraft:iron_pickaxe
```

Group er-look (commits 17 to 19), 7 stacks:

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:cobblestone*256,minecraft:iron_pickaxe,minecraft:cooked_beef*32
```

Group er-fight (commits 20 and 21), 9 stacks:

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:cobblestone*64,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

Group er-guns (commit 22), 12 stacks:

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:frag_grenade*8,iw4l:barrett,iw4l:sniper_rounds*64,iw4l:rpg,iw4l:rocket*16,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

Group er-bar-friend (commit 23 on), 10 stacks:

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,minecraft:netherite_sword+sharpness=5,minecraft:shield,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:obsidian*14,minecraft:flint_and_steel
```

## Protocol

Nothing on the wire changed yet. Commit 18 fills Margit's `MobShow.family` (D0's `FamilyShow`, already on the wire) with his clip, and commit 19 adds his critical count as its fifth value; the message format is the same, so it needs no bump of its own. Commit 21 tells a friend "GREAT ENEMY FELLED" as an existing `ToClient::Chat` system line with the overlay flag and plays the chime as an existing `ToClient::Sound`, so it adds nothing to the wire either. Commit 22's `Command::EldenHurt` is a command to the host's own level thread, not a network message: friends' shots already reach the host as MW2 events with the friend's id. Commit 23's boss bar rides the existing `ToClient::BossBar` with a new id range (`1 << 41` plus his id), which friends already keep, so the wire format is unchanged.

## Vendored files

Files under `third_party/minecraftoss` this track patched or added. Files that track D0 added count as patched here.

- Patched (6): `entities/Cargo.toml`, `entities/src/world.rs`, `entities/src/world/families.rs`, `entities/src/world/families/bosses.rs`, `entities/src/world/families/pests/probe.rs` (test only), `entities/src/world/families/tests.rs` (test only).
- Added (3): `entities/src/world/families/bosses/elden_bosses.rs`, `entities/src/world/families/bosses/elden_bosses/arena.rs`, `entities/src/world/families/bosses/elden_bosses/blows.rs`.
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

## Review of groups er-entity and er-arena

### 13. A Margit killed while walking stops where he falls

After fe8467c4.

- **Defect:** `steady` puts his last stride back into his velocity at the start of every tick, so pushes never move him (commit 12). It did that while he was dying too. His stride is only updated by his own walking, which stops at death, so a Margit killed mid-stride was set back to full walking speed every tick and slid on through his whole death clip: 1.36 blocks in 15 ticks in the new test, about 2 blocks before he vanished. Vanilla's dying travel (`dying_travel`, which D0's base tick runs) slows a mob to a stop in a few ticks.
- **Changed (vendored entities crate, `world/families/bosses/elden_bosses.rs`):** `steady` restores his stride only while he is alive. Effects and fall distance are still cleared every tick.
- **Tests:** `world::families::bosses::elden_bosses::arena::tests::a_margit_killed_while_walking_stops_where_he_falls` (he walks toward a player 9 blocks away, is killed at full stride, and moves less than 0.5 blocks in the next 15 ticks; it failed at 1.36 before the change). `test -p minecraftoss-entities --locked --lib elden_bosses`: 19 passed. `check -p launcher --locked`: green.
- **Owner's steps.** He is still a round shadow about 2 blocks wide until group `er-look`. Release build, survival, Normal, by day, flat open ground in the Overworld. Name each screenshot by its label, `f2-13-1.png`.
  1. `/summon iw4l:margit ~ ~ ~10 {Health:20f}`: you hear him rise.
  2. Step back to about 8 blocks so the shadow walks toward you, and hit it twice with the netherite sword as it reaches you: the deep evoker death sound plays, and the shadow stops at once where it was hit and is gone a second later. It does not glide on toward you. **Screenshot f2-13-1** right after the death sound.

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:cooked_beef*32
```

### 14. A blast at Margit's feet no longer lifts him

After 1763ffd9.

- **Defect:** commit 8 decided that blasts and pushes never move him, but `steady` dropped only the sideways part of what a blast adds. D0's blast arm adds the whole push (`body.velocity += push`), and its upward part stayed: a TNT 1 block in front of him at ground level threw him into the air, and one right under him threw him higher still. A MW2 frag or rocket at his feet does the same until group `er-guns` routes those blasts.
- **Changed (vendored entities crate):** his stride is now the whole velocity his own travel left, upward part included, and `steady` puts all of it back while he is alive (`elden_bosses.rs`). In `arena::carry_out` a walking tick keeps the whole velocity and a root-motion tick keeps only its upward part, so his falls and leaps go on as before and a push from outside never lasts into his next tick.
- **Tests:** `world::families::bosses::elden_bosses::tests::a_blast_at_his_feet_does_not_lift_him` (a player's TNT-sized blast 1 block in front of him gives him an upward push of more than 0.1 a tick, and he stays exactly where he stood for the next 20 ticks). `a_margit_killed_while_walking_stops_where_he_falls` (commit 13) now measures his stride sideways only, since a mob on the ground keeps a small downward speed. `test -p minecraftoss-entities --locked --lib elden_bosses`: 20 passed. `check -p launcher --locked`: green.
- **Owner's steps.** He is still a round shadow about 2 blocks wide until group `er-look`. Release build, survival, Normal, by day, flat open ground in the Overworld. Name each screenshot by its label, `f2-14-1.png`.
  1. `/summon iw4l:margit ~ ~ ~6`: you hear him rise.
  2. Put a TNT on the ground right beside the shadow, light it and step back 8 blocks: the hurt sound, and the shadow stays as dark as before; it does not fade for a moment as it would if he were thrown up. **Screenshot f2-14-1** right after the blast.

```
IW4L_MINECRAFT_KIT=minecraft:tnt*8,minecraft:flint_and_steel,minecraft:cooked_beef*32
```

### 15. Margit walks all the way home to his anchor

After 274f48e9.

- **Defect:** after a reset he walks home on a path to his anchor's block and falls asleep within 1 block of the anchor (`elden_boss::brain::HOME`). Vanilla's path following counts a node as reached once a mob's feet are within half its width (0.7 for him) of the node's centre on both axes, so the path ended up to about 1.2 blocks from the centre of the anchor's block. An anchor that is not near the middle of its block (a summon at `~ ~ ~` keeps the player's exact position) was then often more than 1 block away. He stood there for 5 seconds and then faded home with the mirror sound, the fade meant for a blocked way home. Tried on a grid of 25 anchor positions within a block and 8 directions 8 blocks out, 18 of the 200 walks home ended in that fade; the existing reset test only used an anchor at a block's centre.
- **Changed (vendored entities crate, `world/families/bosses/elden_bosses/arena.rs`):** when his path is done and the brain's walk point is still more than 0.5 blocks (`ARRIVE`) but at most 2 blocks (`LAST_STEP`) away at about his own height (less than 1 block up or down), he walks straight at it with the same ground travel and speed. Collisions still apply, so a point behind a wall stays out of reach, and the brain's no-progress fade home still covers that. This also brings him onto his stalking point between moves instead of stopping short of it.
- **Decision:** the fix is on the entity side, not a larger `HOME` in the brain: the brain's numbers are the design doc's, and the gap came from how a 1.4-wide mob follows a vanilla path.
- **Tests:** `world::families::bosses::elden_bosses::arena::tests::margit_walks_all_the_way_home_wherever_on_its_block_his_anchor_lies` (the same 200 walks home: none fades, and each ends asleep within 1 block of the anchor). `test -p minecraftoss-entities --locked --lib`: 460 passed. `check -p launcher --locked`: green.
- **Owner's steps.** He is still a round shadow about 2 blocks wide until group `er-look`. Release build, survival, Normal, by day, flat open ground in the Overworld, F3 on. Name each screenshot by its label, `f2-15-1.png`.
  1. Stand near the corner of a block, not its middle (F3's `XYZ` shows fractions near .1 or .9), note the coordinates and type `/summon iw4l:margit ~ ~ ~`: you hear him rise. Walk 10 blocks away: the shadow follows you.
  2. Walk on to 45 blocks from the spot, wait 5 seconds, then come back to 30 blocks: the shadow walks back to where it appeared and goes quiet. There is no mirror sound and no jump of the shadow at the end. **Screenshot f2-15-1** with the shadow on its spot.
  3. Repeat steps 1 and 2 twice from other block corners: each time he walks all the way home.

Kit: none needed for these steps.

### Checks after the review

After 65505460. `test -p minecraftoss-entities --locked --lib`: 460 passed. `test -p minecraft_terrain --locked --lib` with `elden`, `margit` and `summoned_health`: 9 passed. `clippy -p minecraftoss-entities --locked --all-targets -- -A clippy::approx_constant`: nothing in Margit's files. The combined kit at the top covers commits 13 to 15 (their items are already in it). Still left for later, as commit 8 says: `sleep::MONSTERS` does not list `iw4l:margit`, so a player may sleep beside him.

## Final checks of segment f2

### 16. Margit's arena code passes clippy

After 4556eca4.

- **Why:** `third_party/minecraftoss` is excluded from the workspace, so `cargo clippy -p minecraftoss-entities` only checks that crate with rustc: clippy's wrapper runs on workspace members alone. The "nothing in Margit's files" of the checks after the review therefore saw no clippy lints. Run through clippy for real (`check -p minecraftoss-entities --locked --all-targets` with `RUSTC_WRAPPER=clippy-driver`, which reads the root `clippy.toml`), the crate's lib test build has 84 warnings, two of them on this track's lines: `clippy::unnecessary_map_or` in `arena.rs`.
- **Changed (vendored entities crate, `world/families/bosses/elden_bosses/arena.rs`):** `spot_near` and `seen` use `Option::is_none_or` instead of `map_or(true, ...)`. Same behaviour.
- **Tests:** none new; nothing changes in behaviour.
- **Runs** (touched crates from `git diff --stat r10-f2-base..HEAD`: `minecraftoss-entities`, `minecraft_terrain`, `minecraft_survival`):
  - `clippy -p minecraftoss-entities -p minecraft_terrain -p minecraft_survival --locked --all-targets -- -A clippy::approx_constant`: no warning on a line of this track in `minecraft_terrain` or `minecraft_survival` (their older warnings stay), and with `clippy-driver` as above none left in the entities crate either. The other 82 warnings of its lib test build are on older lines (`families.rs` 246-262 `&mut Vec`, `world.rs` `unwrap_or_default`, villagers and others) and stay.
  - `test -p minecraftoss-entities --locked --tests`: 460 passed (the bins have no tests).
  - `test -p minecraft_terrain --locked`: 1440 passed and 3 ignored; doc-tests 0.
  - `test -p minecraft_survival --locked`: 778 passed, `vanilla_world` 1 passed; doc-tests 0.
  - `check -p launcher --locked`: green, with only the old render_frame and skate warnings.
  - The new `.rs` files of segment f2 (`elden_bosses.rs` and `arena.rs` in the entities crate, `elden_save.rs` and `families/bosses/elden_bosses.rs` in `minecraft_terrain`) pass `rustfmt --check --edition 2021` with empty output on their staged content, and no line the segment added holds a comment.
- **Decision:** a `RUSTC_WRAPPER` change does not change cargo's fingerprints, so the check units built through `clippy-driver` replay clippy's warnings on a later plain `check`. Touching `arena.rs` afterwards rebuilt them with rustc; the launcher check above shows no entities warnings again.
- **Owner's steps:** none: no behaviour changes. The combined kit at the top covers every commit of segment f2 (7 to 16).

## Group er-look

Brief F3, commits F3.1-F3.3, with design doc 5.3, 5.4 and 14.1. It starts on 48b1ecf2 (`minecraft-survival` after release 3, with E2's segment e2a merged in 7c017f0e), so arrows and thrown things already reach Margit through D0's generic arms.

### 17. Margit is drawn as a hooded old man with a cane

After 48b1ecf2.

- **Changed (`minecraft_terrain`):**
  - New `elden_skin.rs`: `PATH` and `TEXTURE` (`minecraft:iw4l/margit`) and `paint()`, a 64 x 64 skin painted by code from a small palette with seeded noise (a hash of the pixel, no random crate): a pale face with gold eye pixels and dark brows under a shadow, a dark ragged cloak (the hem's bottom rows have transparent pixels, cut out like every entity skin), fur on the hood's rim and on the shoulders, a grey beard, bone horns with dark tips, cane wood with grain, a bare pale left arm, and one gold area with gleams for the dagger, sword, hammer and eyes. Each part's UV origin and paint area are constants beside it.
  - New `elden_render.rs`: Margit as posed cubes on twelve bones (root, body, head, two horns, right arm, cane, left arm, the left hand that will hold the golden weapons, two legs, and the cloak's skirt): a body leaning 20 degrees forward with the head held up, a hood around the head, two gold eye cubes just in front of the face, a beard, two horns through the hood, a dark right sleeve with the cane planted in front of him, a bare left arm, and a skirt widening to a ragged hem 1.4 blocks wide. The root scales him by 1.4 (`SCALE`), so he stands 3.06 blocks to the tips of his horns, hunched, with his face 0.7 blocks in front of his feet. Idle is the idle clip of his table (Ready and Breathe, built into a looping `keyframes::Definition` from `ClipDef.poses`); walking is a limping one-second loop on `apply_walk` from the client's walk animation (the right leg swings far, the left short, the cane arm plants, the body sways and bobs); the head turns toward his look (head yaw from his facing, at most 75 degrees, and pitch).
  - `families/bosses/elden_bosses.rs`: the family's `draw` calls `elden_render::append_bosses` (D0's render registry line), for the host, every friend and spawner cages alike. The shadow (1.0) and the pose push were already D0's.
  - `mesh.rs`, beside the white tile: the skin is listed with the textures, skipped by the pack read and painted into its atlas cell (the cell is 64 pixels, so the skin fills it at full resolution). No PNG is committed. `lib.rs`: the two modules beside `elden_save`.
- **Tests:** `elden_skin::tests::the_skin_paints_the_cloak_dark_and_the_beard_grey_in_their_uv_boxes` (also the gold eyes, the pale face, the gold and wood areas and the ragged hem). `elden_render::tests::a_standing_margit_is_about_3_blocks_tall_and_1_4_wide_at_his_feet` (fourteen cubes and two eye cubes, feet on the ground, 2.85-3.1 tall, 1.4 wide below half a block, his face ahead of his feet and his cane in front), `every_part_samples_its_own_paint` (every cube's UV box stays inside its paint area), `his_head_follows_his_target_and_his_legs_swing_as_he_walks`. `families::bosses::elden_bosses::tests::margit_is_drawn_from_his_painted_skin_through_his_family` (the atlas cell holds the painted skin, nothing is missing, `families::draw` gives sixteen cubes inside his region and his top 3.06 above his feet).
- **Decisions:**
  - **His body is drawn at his facing**, not at the client's body rotation: `ClientMob.y_rot` interpolated with `rot_lerp`. The client's body rotation lags the head by up to 75 degrees while he stands (vanilla's `BodyRotationControl`), and his strikes come from his facing, so a lagging body would swing his cane where no hit lands. The head turns from that facing toward his look.
  - **The skin is listed in `mesh.rs` beside the white tile, not in the family's `TEXTURES`:** it is a tile made by code like `iw4l/white`, so the pack read must skip it and the cell loop must paint it, which only `mesh.rs` can do; D0's `TEXTURES` lists pack textures. The family's `TEXTURES` stays empty, and D0's test of the registry's texture list is unchanged.
  - **The scale is 1.4,** so the hem is exactly his box's 1.4 blocks and the horns' tips 3.06 blocks up: his box is 3.0.
  - **Spawner cages turn him:** his root takes the client pose's display turn (`MobPose.turn`) before his facing, so a spawner holding `iw4l:margit` spins him like any registered mob (D0's cage test).
  - The hem is a separate skirt cube with its own paint area, so its ragged bottom rows do not cut holes in the other cloth parts, which share one cloth area (torso, sleeve, legs and the skirt's top).
- **Closes from release 3's known limits:** "He is not drawn, only his shadow."
- **Owner's steps.** He is drawn, walks and turns his head, but his moves are not animated yet (the next commit), so he stands still while his sounds play, and he has no golden weapons or particles. Release build, survival, Normal, by day, flat open ground in the Overworld. Name each screenshot by its label, `f3-17-1.png`.
  1. `/summon iw4l:margit ~ ~ ~10`: a hunched old man in a dark hooded cloak stands there, about 3 blocks tall: a pale face with two gold eyes, a grey beard, two small pale horns through the hood, a fur rim on the hood and the shoulders, a cane in his right hand planted in front of him, and a bare pale left arm. His cloak ends in a ragged hem just above his feet. **Screenshot f3-17-1** from the front, about 6 blocks away.
  2. Walk around him at 6-10 blocks: he limps after you (his right leg swings further than his left, his cane arm swings) and his head follows you. **Screenshot f3-17-2** from the side while he walks.
  3. Stand still 5 blocks from him and watch his body slowly rise and fall (breathing) between moves.
  4. Hit him with the sword: he flashes red like any mob.

Kit: none needed for these steps (step 4: any sword).

### 18. Margit's moves and golden weapons are animated from his move table

After 239488a8.

- **Changed (`elden_boss`):** `Clip.from`, the move clock a recovery started from (0 in every other state), so the renderer knows which branch ended the move and how long its recovery lasts. `Brain::clip()`, the clip he shows now (a move's own clock and hold while he plays one), for a copy made from a saved tag. `State::ALL`, the states in a fixed order, whose places are their codes in `FamilyShow`.
- **Changed (vendored entities crate, `elden_bosses.rs`):** `EldenBoss.shown`, the clip of his last brain step (`Act.clip`), set at his spawn from `Brain::clip()`. The host's view world is a copy of the server's, so the host draws from it.
- **Changed (`minecraft_terrain`):**
  - `elden_save::read`: after the reset, `shown` is his walk home, so a loaded Margit and a friend's new copy show it.
  - `families/bosses/elden_bosses.rs`: `show` and `show_on` carry the clip in D0's `FamilyShow`: the state (four bits, its place in `State::ALL`), held and awake as flags, and the move, its own clock, the phase and the recovery's start as values (`show_clip`, `shown_clip`). The friend's copy never steps its brain; it draws from this.
  - `elden_render.rs`:
    - **The pose table:** bone rotations for all 24 `Pose` keys (and the root sinking or rising, and the cane sliding in his grip so it stays planted in low poses). Each pose turns the head back against the body's twist, so his head keeps facing ahead through a swing; the look turn comes on top.
    - **Clips:** one `keyframes::Definition` per move and per clip that is not picked (wake, kneel, fade, death and the phase 2 shift), built once from their `poses` (Catmull-Rom, seconds are ticks / 20). The renderer samples the shown clip's clock plus the partial tick. During a hold the clock stops, and the tick the hold starts on is drawn frozen too, so a delayed swing stays still on screen exactly as long as the brain holds it.
    - Asleep, he kneels (the wake clip's first pose), so his wake rises from one knee. A recovery eases from the move's pose where the branch cut it back to Ready over that branch's recovery. Idle and the walk home keep commit 17's idle and walk.
    - **Golden weapons:** the dagger (3 cubes), the sword (3) and the hammer (2), all in the skin's gold area, on the left hand bone. Each shows only inside its `conjure` window (of a move, or of the shift), grows from nothing to full size over 4 ticks (a scale about the hand, so no translucent pass), and is drawn at full light (`Skin` sky and block 15) without shading. No `append_glowing` and no tint.
    - **The tell (design doc 5.4):** his eye cubes grow to twice their size and go full bright in the 8 ticks before each strike that follows the move's hold: the cane combo's third swing (clocks 36-43), the cane slam (21-27, 7 ticks after its hold) and the hammer slam (32-39).
    - **The fade:** he is not drawn from 2 ticks before to 2 ticks after the fade's swap, so the jump to the new spot is not seen sliding.
- **Tests:**
  - `elden_render::tests`: `every_pose_sets_every_bone_it_names` (each pose names a bone once, a clip of it sets exactly those bones and leaves the rest), `each_moves_clip_lasts_as_long_as_the_move` (every move, the clips and the shift; the cane combo's first swing is drawn on its strike tick and he is back to ready at its end), `a_held_swing_keeps_its_pose_while_the_clock_stops` (the cane stays raised over any partial tick during the hold and on its first tick, and runs on after it), `the_dagger_part_shows_only_while_conjured` (clocks 2-22 of the dagger throw, growing 0, 0.25, 0.5, 0.75, 1 over 4 ticks, three more cubes drawn; the sword in the sweep, the hammer in the shift, nothing in a kneel), `the_golden_weapons_are_full_bright_and_gold_in_the_skin` (every weapon vertex at sky and block 15 inside the gold area, every pixel of their UV boxes gold, the body not full bright, a half-grown sword reaches less far), `his_eyes_flare_before_a_delayed_swing` (only the three windows above, never during the hold, rising through the window; the flaring eyes are full bright and twice as wide), and two more: `asleep_he_kneels_and_a_fade_hides_him_around_its_swap`, `a_recovery_eases_from_where_the_combo_was_cut_back_to_ready`.
  - `families::bosses::elden_bosses::tests::a_friend_sees_margits_clip_from_his_show` (a host Margit held in his dagger throw; the friend's copy from his saved tag shows the walk home, then the host's `server_mobs` and `show_on` give it the same clip; every state round-trips through `FamilyShow`, an unknown state code is ignored). `margit_is_drawn_from_his_painted_skin_through_his_family` now finds him asleep on one knee, 2.56 blocks tall.
  - `elden_boss::brain::tests::a_recovery_shows_the_move_clock_it_was_cut_at` (over 200 seeds: recoveries from clock 16 last under 14 ticks and from 26 under 16; a move shows 0; `clip()` shows a begun move and an asleep brain).
- **Decisions:**
  - **The clip goes through D0's `FamilyShow` now,** not a new `MobShow.elden` field: D0's rule is that every drawn state of a registered family goes through `show` and `show_on`, `FamilyShow` is already on the wire, and so friends draw his moves from this commit on with nothing new on the wire. Group `er-bar-friend` (F6) adds only the daggers in flight. Design doc 9.3 says so now.
  - **A "delayed swing" is a strike that comes after its move's hold.** The flare starts when the hold is over (at most 8 ticks before the strike), never during the hold, so a long hold does not give the timing away. The dagger throw's hold has no strike after it, so it has no flare (the dagger shows in his hand the whole time).
  - **Weapons and flaring eyes are unshaded** at full light, as vanilla's emissive layers (`RenderType.eyes`) are. They grow in over 4 ticks and vanish at the end of their window without shrinking.
  - **Asleep, he kneels.** A Margit walking home after a reset drops to his knee when he reaches his anchor; there is no blend between clips, as the host's view is a copy of the server's each tick and keeps no memory of the last clip.
  - **The poses were checked against the ground:** with a script on the model's own numbers (not committed), no part of any pose goes more than 0.01 blocks below his feet without a weapon; a hammer slammed down goes into the ground, as it should.
- **Owner's steps.** Release build, survival, Normal, by day, flat open ground in the Overworld. His particles come in the next commit, and his strikes and daggers hurt nobody until group `er-fight`, so you can stand next to him. Name each screenshot by its label, `f3-18-1.png`.
  1. `/summon iw4l:margit ~ ~ ~20`: he kneels on one knee, his head bowed, leaning on his cane (asleep). Walk to 15 blocks: he rises from the knee over about a second and a half. **Screenshot f3-18-1** while he rises.
  2. Stand 2-3 blocks in front of him and do not attack. The cane combo: a swing from his right to his left, a backhand from left to right, then the cane goes up overhead and stays there a moment; his two gold eyes grow and glow just before it comes down. Sometimes he stops after one or two swings and settles back. **Screenshot f3-18-2** with the cane up and the eyes bright.
  3. The cane slam: the cane up overhead, held, the eyes flare, and it comes down in front of him.
  4. The golden sword: a glowing gold sword grows in his left hand with a chime, he draws it back to his left and sweeps it across to his right. **Screenshot f3-18-3** mid-sweep.
  5. Step back to 15-20 blocks: a small golden dagger grows in his left hand, he raises it behind his head and holds it for one to one and a half seconds, then throws (no dagger flies yet). **Screenshot f3-18-4** while he holds it.
  6. At 10-14 blocks: he crouches, roars, leaps with his arms up and lands crouched with the cane down.
  7. Stand right next to him: he hops back and throws two or three times.
  8. Hit him with the netherite sword at full charge five times quickly: he drops to one knee, leaning on his cane, for 4 seconds, then rises. Do it again and hit him once from the front while he kneels: he rises at once.
  9. `/summon iw4l:margit ~ ~ ~6 {Health:401f}` and hit him once: he raises his left hand and a gold hammer grows in it. Then the hammer slam: the hammer high behind his head, held, the eyes flare, and he slams it down in front of him. **Screenshot f3-18-5** with the hammer raised.
  10. `/summon iw4l:margit ~ ~ ~6 {Health:20f}` and hit him twice: he sinks to his knees and slumps forward, then is gone.
  11. Optional, two windows (`scripts\mp-local.ps1`): the friend sees the same kneel, swings, golden weapons and eye flare. **Screenshot f3-18-6**, both windows.

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:cooked_beef*32
```

### 19. Golden mist, sparks and dust show Margit's spawn, phase change, slams and death

After 837f9572.

- **Changed (`elden_boss`):** `Clip.criticals`, a wrapping count of his criticals that the brain keeps across resets, so a viewer can tell a critical from the shown state alone.
- **Changed (vendored entities crate, `elden_bosses.rs`):** his steps. After his move each tick (walking or root motion), vanilla's step emission (`play_movement`, one step per block walked, none in the air, a monster's swim and splash) plays his own step, `entity.iron_golem.step` at volume 1.0 and pitch 0.7 (design doc 14.1), as a `Voice::Event`. Every other `Cue::Sound` already played on its tick (commit 8).
- **Changed (`minecraft_terrain`):**
  - New `elden_effects.rs`, what a viewer draws from the shown clip, with nothing sent:
    - Each `Cue::Burst` of the clip's cue table (the move's, or the wake, kneel, fade, death or phase 2 shift clip's) fires once on its tick. The view keeps the last clip it showed in one `i32` per mob (D0's `ClientFx.seen`: state, move, clock and the critical count), so a tick seen twice fires nothing, and ticks the view skipped are caught up, at most 4 ticks back.
    - **Golden mist:** totem sparkles in a sphere around his box with the totem's upward drift, and 16 gold `SpellKind::Effect` motes (gold is 1.0, 0.82, 0.25). It shows his rise, the shift's flash, both halves of a fade and his death.
    - **Conjure sparks:** 10 `EndRod` sparks at his left hand (the def's dagger hand) as a golden weapon forms.
    - **A slam:** 16 `Poof` puffs in a 1-block ring, flying outward, and one `Explosion` cloud, centred where the strike lands: a circle's or ring's centre ahead of him (the leap slams, the hammer's shockwave), or half the arc's reach for the cane slam.
    - **Crits:** 16 `Crit` sparks from his chest when he kneels, and again when the critical count changes while he kneels.
    - **Phase 2:** 2 gold `SpellKind::Effect` motes a tick drip from his body while he is awake and alive in phase 2.
    - Nothing was added to `ambient_particles.rs`: the kinds and the spell particles' colour are the existing ones.
  - `families/bosses/elden_bosses.rs`: the family's `client_effects` (D0's hook, called from every view's tick) runs the above from the client mob's position and facing, the same for the host and every friend. `FamilyShow` carries the critical count as its fifth value.
  - `lib.rs`: the module beside `elden_render`.
- **Tests:**
  - `elden_effects::tests`: `what_he_showed_last_is_remembered_in_one_number`, `each_burst_fires_once_on_its_tick_and_skipped_ticks_catch_up` (the cane slam's dust on clock 29 only, not twice, caught up from 27 to 31, not from a first look at 40; the wake's mist after sleep; the sword's sparks when a branch starts it; a critical's crits during a kneel and none after a reset), `a_slam_rings_dust_where_his_strike_lands` (1, 2 and 1.75 blocks ahead for the leap, the hammer's ring and the cane slam; one explosion and 16 puffs on a 1-block ring).
  - `families::bosses::elden_bosses::tests`: `a_slam_cue_bursts_on_its_tick_for_host_and_friend_views` (a host Margit plays the cane slam; the host's view and a friend's copy fed by `server_mobs` and `show_on` each draw one explosion, on the tick he shows clock 29, and the same 16 puffs), `golden_mist_shows_his_rise_and_phase_two_drips_gold` (nothing asleep; totem sparkles and 16 gold motes the tick he wakes; then 2 gold motes every tick in phase 2, none in phase 1 or dead). `a_friend_sees_margits_clip_from_his_show` also round-trips the critical count.
  - `world::families::bosses::elden_bosses::tests::margit_steps_like_an_iron_golem_low` (entities: no step asleep; walking to a player 10 blocks away, at least 3 steps, all at volume 1.0 and pitch 0.7 in `hostile_volume`).
  - `elden_boss::hurt::tests::a_melee_hit_from_the_front_on_a_kneeling_boss_is_a_critical_and_one_from_behind_is_not` also checks the count on his clip (0, then 1).
- **Decisions:**
  - **The critical count rides on the clip.** A critical jumps the kneel clock to its rise, but a view that skipped a tick sees the same jump, so the clock alone cannot show it. A count kept by the brain can, and a burst fires only while he kneels, so a reset (which starts a fresh count) never shows one.
  - **Catch-up is 4 ticks:** a view that missed a few server ticks still shows a burst it skipped, and a friend who first sees him mid-clip sees no old slam.
  - **Steps follow vanilla's step emission** (a step per block walked), not the walk clip's footfalls, so they match his speed and stop in the air.
- **Runs (the whole group):** `test -p elden_boss -p minecraft_terrain -p minecraftoss-entities --locked --lib`: 54, 1575 (and 3 ignored) and 498 passed. After commit 18 the same run without the entities crate gave 54 and 1570; during commit 17, `minecraft_terrain` gave 1560 passed and one failure, D0's spawner cage test, until his root took the display turn; the spawner and Margit tests then passed (28). `check -p launcher --locked`: green after each commit, with only the old render_frame and skate warnings. `minecraft_survival` is not touched. The new files (`elden_skin.rs`, `elden_render.rs`, `elden_effects.rs`) pass `rustfmt --check --edition 2021` with empty output on their staged content, and no added line holds a comment.
- **Closes from release 3's known limits:** "He has no particles (golden mist, sparks, dust)". The golden ring at the leash is group `er-altar`'s.
- **Owner's steps.** Release build, survival, Normal, by day, flat open ground in the Overworld. His strikes and daggers still hurt nobody until group `er-fight`. Name each screenshot by its label, `f3-19-1.png`.
  1. `/summon iw4l:margit ~ ~ ~20` and walk to 15 blocks: as he rises from his knee, green-gold sparkles and gold motes swirl around him with the mirror sound. **Screenshot f3-19-1** while they swirl.
  2. Walk around him at 6-10 blocks: low, heavy iron golem steps as he limps after you.
  3. Stand 2-3 blocks in front of him and wait for the cane slam: where the cane hits the ground, 1-2 blocks in front of him, a ring of white puffs flies outward with a small grey explosion cloud and the mace's smash. **Screenshot f3-19-2** at the slam.
  4. When the golden sword or a dagger forms in his left hand, white sparks fly from the hand with the chime.
  5. At 10-14 blocks: the leap slam; the ring of puffs and the cloud where he lands.
  6. Hit him with the netherite sword at full charge five times quickly: sparks burst from his chest as he drops to one knee. Hit him once from the front while he kneels: another burst of sparks with the critical sound, and he rises. **Screenshot f3-19-3** at the second burst.
  7. `/summon iw4l:margit ~ ~ ~6 {Health:401f}` and hit him once: green-gold sparkles and gold motes flash around him as the hammer forms, and from then on gold motes keep dripping from him. Wait for the hammer slam: a ring of puffs 2 blocks in front of him. **Screenshot f3-19-4** with the gold motes.
  8. `/summon iw4l:margit ~ ~ ~6 {Health:20f}` and hit him twice: sparkles and gold motes as he falls, and again as he slumps and vanishes.
  9. Optional, the fade: build a ring of cobblestone 6 high, 4 blocks out from your block on every side, with him outside: after about 5 seconds sparkles where he vanishes, and again inside the ring where he appears.
  10. Optional, two windows (`scripts\mp-local.ps1`): the friend sees the same sparkles, sparks, puffs and dripping gold, and hears the steps. **Screenshot f3-19-5**, both windows.

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:cobblestone*256,minecraft:iron_pickaxe,minecraft:cooked_beef*32
```

## Group er-fight

Brief F4, commits F4.1 and F4.2, with design doc sections 5, 7.4 and 13. It starts on 2815efff (group er-look).

### 20. Margit's cane, golden weapons and daggers hurt players

After 2815efff.

- **Changed (`elden_boss`):**
  - `data.rs`: `Push` (throw and lift) and `Reactions` (one `Push` per reaction, `push(reaction)`), and `BossDef.reactions`. `margit.rs`: flinch none, stagger a throw of 0.8, knockdown a throw of 1.2 and a lift of 0.5 (design doc 7.4), so every number stays in `margit.rs`.
  - New `flight.rs`: `Flight` (start, velocity, age; group er-bar-friend sends the same three), `Flight::launch` from the brain's `Dagger`, `at(ticks)` and `path()`, and `fly(def, wall, players) -> Flown`. On each tick the dagger's path is its velocity (1.5 blocks). The first player whose box the path crosses before the wall (`shape::segment`) is struck with the def's dagger hit (4 holy, flinch, the dagger weapon, strike `THROWN`, coming from 1 block behind where it struck); a wall stops it where the path meets it; otherwise it moves on and is spent after 40 moves. `share_of` turns a block hit point into a share of the path.
  - `brain.rs`: `Seen.sight`, whether he sees the player (eye to eye, no block between). A strike skips a player he cannot see on that tick, and may still land later in its window. Throws are unchanged.
- **Changed (vendored entities crate):**
  - New `world/families/bosses/elden_bosses/blows.rs`:
    - `kind(def, hit, feet, facing)`: a physical hit is `PlayerHitKind::Melee` from where it came (his feet for an arc, the centre of a circle or ring), with the reaction's throw away from there (or along his facing when the player stands on that point), its lift, and `disable_shield_seconds` from the strike's shield ticks (100 ticks, 5 s, on both hammer strikes). A holy hit is D0's `PlayerHitKind::Typed` with vanilla's `minecraft:indirect_magic` and its message `indirectMagic`, `bypasses_armor` and `bypasses_shield`, from where it came, with the reaction's throw and lift as its push.
    - `fly(mob, level, players, thrown)`: moves every dagger in flight one tick against the level's blocks (`sight::clip` on collision shapes, so grass and flowers do not stop it) and the eligible players' boxes, plays `item.trident.hit` (the def's `dagger_hit` cue) where one stops or strikes, drops the spent ones, then adds the daggers his brain threw this tick, so a dagger first moves on the tick after the throw.
    - `deal(world, id, players, struck)`: each hit becomes a `PlayerHit` through D0's `family_strike`, with his id as the source.
  - `elden_bosses.rs`: `EldenBoss.daggers`. His tick takes the `Act`'s hits and throws only on a tick his brain stepped (a Margit without AI swings and throws nothing; daggers already in flight fly on), flies the daggers, and deals the hits after his move. `think` passes his eyes (feet + 2.55) to the sense.
  - `arena.rs` `seen`: each eligible player's `sight` from `sight::line_of_sight` (vanilla's `hasLineOfSight`, eye to eye).
- **Changed (`minecraft_terrain`, `elden_render.rs`):** `dagger_root` and `append_daggers`: each dagger in flight is F3's dagger part (three gold cubes) at full light, scaled like Margit, centred on its position with the blade along its flight, and drawn one tick behind like every mob's position (`age - 1 + partial`, never before his hand). `append_bosses` draws them after his model under their own overlay mark, so his red hurt flash does not tint them, and also while a fade hides him.
- **Tests:**
  - `elden_boss`: `flight::tests::a_flight_stops_where_the_wall_or_the_nearer_player_is`, `brain::tests::a_player_he_cannot_see_is_not_struck`.
  - Entities, `world::families::bosses::elden_bosses::blows::tests`:
    - `a_cane_hit_becomes_a_melee_player_hit_from_margit` (4 damage from his id, from his feet, no extra throw or lift, the shield left up)
    - `the_hammer_disables_a_shield_for_5_seconds` (the hammer's arc is melee with 5 s; its shockwave is a holy 4 through shields and armour)
    - `a_knockdown_lifts_and_throws_the_player` (lift 0.5 and a throw of 1.2 away from him; a stagger's 0.8 away, and along his facing when the player stands on the point)
    - `a_dagger_stops_at_a_wall` (a 4-high wall 7.5 blocks out: no hit, one `item.trident.hit` on the wall's face)
    - `a_dagger_hits_the_first_player_on_its_path` (a player stepping into its path after the throw takes it; the target behind does not)
    - `a_dagger_is_gone_after_40_ticks` (in flight for 40 ticks, then gone with no hit and no sound)
    - and `arena::tests::a_player_behind_a_wall_is_out_of_his_sight` (he still sees into a 2-deep hole beside him).
  - Terrain: `elden_render::tests::a_flying_dagger_is_drawn_along_its_flight`.
  - Survival, `death::tests`: `a_holy_hit_is_magic_and_ignores_a_raised_shield` (his sword sweep through `mob_hit`: `indirect_magic`, past armour, no shield direction so a raised shield blocks nothing, knockback from where it came and a stagger of 0.8; his cane slam is `mob_attack`, and a raised shield facing him blocks it), `a_player_killed_by_a_dagger_is_told_killed_using_magic` ("Sos was killed by Margit, the Fell Omen using magic"; the test language gains vanilla's `death.attack.indirectMagic`).
- **Runs:** `test -p elden_boss --locked`: 56 passed. `test -p minecraftoss-entities --locked --lib`: 505 passed. `test -p minecraft_terrain --locked --lib elden`: 26 passed. `test -p minecraft_survival --locked --lib death::tests`: 51 passed. `check -p launcher --locked`: green, with only the old render_frame and skate warnings. The new files (`flight.rs`, `blows.rs`) and the track's own files touched here pass `rustfmt --check` (edition 2024 in `elden_boss`, 2021 in the entities crate and `minecraft_terrain`) with empty output on their content with LF line ends.
- **Decisions:**
  - **Holy hits are `Typed`, not `Magic`.** The brief names `PlayerHitKind::Magic { at }`, but the survival side gives `Magic` no knockback source and no push. `Typed` with `minecraft:indirect_magic` and `indirectMagic` gives the same damage type, death line, armour and shield bypass and difficulty scaling, plus vanilla's knockback (indirect magic is not in 26.3's `#no_knockback`, checked in the jar) and the sword sweep's stagger.
  - **Strikes need sight.** Vanilla's `MeleeAttackGoal` only attacks a target it sees, eye to eye, so the brain gets the answer in `Seen.sight` and skips unseen players. Because the brain decides, its stuck rule ("none of his hits landed") counts only hits that really land, so a player behind a wall he cannot reach still gets the fade. He sees into a 2-deep hole beside him, so commit 12's hole rule holds.
  - **A dagger that strikes or meets a block is gone at once.** It does not stick in the wall. The hit sound plays where it stopped.
  - **Daggers fly on while he dies** and are gone with his body. A Margit without AI never throws one.
  - **Reactions push away from where the hit came from**, not from his feet: a leap's landing circle or a hammer's shockwave throws a player away from its centre.
- **Closes from release 3's known limits:** "His strikes and daggers hurt nobody, and no dagger flies", and "His strikes ignore walls".
- **Left for later:** a friend is hit by daggers but does not see them fly until group er-bar-friend sends them in `MobShow`. The host sees them.
- **Owner's steps.** Release build, survival, Normal, by day, flat open ground in the Overworld. Wear the diamond armour except where a step says otherwise, and keep a golden apple ready. Name each screenshot by its label, `f4-20-1.png`.
  1. Take the armour off. `/summon iw4l:margit ~ ~ ~6` and stand 2-3 blocks in front of him. The cane combo: each of the first two swings takes about 2 hearts and pushes you back a little; the third (after the raised cane and the flaring eyes) takes 3 hearts and throws you back further. **Screenshot f4-20-1** of your health right after a swing.
  2. Put the armour on and hold the shield up facing him: his cane swings and the cane slam are blocked with the shield sound and take nothing.
  3. Keep the shield up for the golden sword: the sweep goes through the shield and the armour, takes about 3.5 hearts and throws you back a block or so. **Screenshot f4-20-2** right after it.
  4. Step back to 15-20 blocks and stand still: the golden dagger leaves his hand as a small glowing gold blade and flies at your chest. It hits through the raised shield and the armour for 2 hearts with the trident's hit sound. Next time, strafe sideways as he throws: it flies past. **Screenshot f4-20-3** with a dagger in the air.
  5. Build a cobblestone wall 3 high and 5 wide between you and him at about 15 blocks and stand right behind it: a dagger thrown at you clinks into the wall with the trident's hit sound and does nothing.
  6. Stay behind that wall with him close on the other side: his swings do not reach through it, and he walks around it to get at you.
  7. At 10-14 blocks in the open, wait for the leap: he lands in front of you, takes 4 hearts and tosses you up and back.
  8. `/summon iw4l:margit ~ ~ ~6 {Health:401f}` and hit him once: the hammer forms. Stand 2-3 blocks in front with the shield raised for the hammer slam: the shield blocks the hammer but goes down with the break sound and stays unusable for 5 seconds, you are thrown back, and the shockwave ring then takes 2 hearts through the shield. Without the shield the hammer tosses you up and back. For the next hammer slam, jump as the ring spreads under you: the ring does nothing.
  9. Take the armour off, drop any totem, and let the cane kill you: "Sos was slain by Margit, the Fell Omen". Respawn, summon another, and let a dagger or the sword finish you: "Sos was killed by Margit, the Fell Omen using magic". **Screenshot f4-20-4** of the death screen.
  10. Optional, two windows (`scripts\mp-local.ps1`): a friend standing in a dagger's path is hurt by it, though the friend does not see it fly yet.

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:cobblestone*64,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

### 21. Margit drops gold and experience when he dies, and every fighter sees GREAT ENEMY FELLED

After 55daa528.

- **Changed (`elden_boss`, `data.rs`):** `Experience::for_fighters` (the base plus the per-fighter amount for each fighter after the first), `Loot::count_for` (the count plus the per-fighter count for each fighter, at least one fighter), and `FELLED`, "GREAT ENEMY FELLED". The numbers stay in `margit.rs` (design doc 13): 1 enchanted golden apple, 2 gold blocks, 3 gold ingots per fighter, and 100 experience plus 50 per fighter after the first.
- **Changed (vendored entities crate, `elden_bosses.rs`):**
  - D0's `experience` hook gives his experience by `Brain::fighters()` (the wake's fighters plus everyone who hurt him), and the `death_drops` hook his items by the same count, in stacks of at most 64. D0's `take_deaths` applies vanilla's rules to both: no drops and no experience with `mob_drops` off, and experience only for a player's kill.
  - `EldenBoss.felled`: on the hit that kills him, `take` keeps the fighters. `EntityWorld::take_elden_felled()` hands each fighter out once, with his def.
- **Changed (`minecraft_terrain`):**
  - New `elden_felled.rs`: `Felled { player, text, sounds }` and `felled(world)`, each fighter with "GREAT ENEMY FELLED" and the def's `felled` cue sounds (`minecraft:ui.toast.challenge_complete`).
  - `server.rs`: `Output.elden_felled`, filled in `run_tick` beside `dragons_killed`, every tick, so a kill by a player's swing between ticks is told on the next one.
- **Changed (`minecraft_survival`):**
  - `entities.rs`: `PlayerOutputs.own_sounds` (sounds only that player hears), and `route` sends each `Felled` to its player's queue beside the wither bars: the line as `Clientbound::System { overlay: true }` in `told`, and the chime in `own_sounds`. A fighter not in the level (gone to another dimension) is skipped.
  - `heard.rs`: `minecraft:ui.toast.challenge_complete` joins `ACTOR_ONLY`, so the chime a friend's recorded sounds retell reaches that friend only, and the host's reaches no friend.
- **Changed (`render_anim`):**
  - `minecraft_players.rs` `struck`: each own sound plays for that player, not at a place (the host hears it, a friend's recorder retells it to the friend). The line goes out with the other `told` lines: to a friend as `ToClient::Chat`, which the friend shows on the action bar (`show_overlays`).
  - `minecraft_world.rs`: the host's struck lines now also go to `minecraft_respawn::show_messages`, which puts an overlay line on the action bar; the chat still takes the others. Until now no struck line was an overlay.
- **Tests:**
  - `elden_boss`: `margit::tests::his_reward_grows_with_the_fighters`.
  - Entities: `world::families::bosses::elden_bosses::tests::margit_drops_gold_and_experience_by_fighters` (two fighters at the wake and a third who hurt him later: an enchanted golden apple, 2 gold blocks, 9 gold ingots and 200 experience; with `mob_drops` off no death drops anything; the three are felled once either way). `a_killing_blow_credits_the_player_and_joins_the_death_list` now expects one fighter's reward (100 experience, three stacks).
  - Terrain: `elden_felled::tests::each_fighter_is_told_once_with_the_challenge_chime`.
  - Survival: `entities::tests::every_fighter_is_told_great_enemy_felled` (the host and a friend each get the white action-bar line and the chime; a player of another level gets nothing; the chime is actor only), and `heard::tests::bubbles_and_portal_noises_stay_with_the_player_who_made_them` also checks the chime.
- **Runs:** `test -p elden_boss --locked`: 57 passed. `test -p minecraftoss-entities --locked --lib elden`: 30 passed (the whole crate gave 505 in commit 20, and only Margit's files changed since). `test -p minecraft_terrain --locked --lib`: 1577 passed and 3 ignored. `test -p minecraft_survival --locked`: 803 passed, `vanilla_world` 1 passed. `test -p render_anim --locked --lib`: 217 passed. `check -p launcher --locked`: green, with only the old render_frame and skate warnings. The new `elden_felled.rs` and the track's own files touched here pass `rustfmt --check` with empty output on their content with LF line ends.
- **Decisions:**
  - **The drops are the death's equipment**, as the wither's nether star is, so they drop where he dies through the existing death drops, after the (empty) loot table.
  - **The line and the chime do not depend on `mob_drops`**: only the items and the experience do, as in vanilla. They come whether a player or the world landed the killing hit, since he only dies in a fight.
  - **The fighters are told from his state, not from the death list:** with `mob_drops` off the death list is empty, and a death's experience needs a player's kill.
  - **The chime is an own sound of the player, not a mob voice:** mob voices are heard by everyone near, and the design wants it for the fighters alone, so it rides `ACTOR_ONLY` like the raid horn.
- **Closes from release 3's known limits:** "he gives no drops, experience or reward" (the boss bar, music and altar are groups er-bar-friend and er-altar).
- **Owner's steps.** Release build, survival, Normal, by day, flat open ground in the Overworld, best in a fresh world so the experience bar starts empty. Name each screenshot by its label, `f4-21-1.png`.
  1. `/summon iw4l:margit ~ ~ ~6 {Health:20f}`, walk up as he rises and hit him twice with the netherite sword: he sinks into golden mist. "GREAT ENEMY FELLED" shows in white above the hotbar with the challenge-complete fanfare, and where he fell lie an enchanted golden apple, 2 gold blocks, 3 gold ingots and experience orbs. **Screenshot f4-21-1** with the line and the drops in view.
  2. Pick it all up: the orbs give 100 points, about level 7 from an empty bar.
  3. `/gamerule mob_drops false`, then summon and kill another the same way: the line and the fanfare come, and nothing drops. `/gamerule mob_drops true`.
  4. Optional, two windows (`scripts\mp-local.ps1`): both stand within 24 blocks as the host runs `/summon iw4l:margit ~ ~ ~6 {Health:20f}`, and the friend lands the last hit. Both windows show "GREAT ENEMY FELLED" and each hears the fanfare once; the drops are 6 gold ingots with the apple and the 2 gold blocks, and 150 points of orbs. **Screenshot f4-21-2**, both windows.
  5. Optional, two windows: the friend stays 60 blocks away and never hits him while the host kills one: only the host sees the line and hears the fanfare.

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:cooked_beef*32
```

## Group er-guns

Brief F5, commit F5.1, with design doc sections 2.2 and 7.1. It starts on 1103ebd2 (group er-fight).

### 22. MW2 guns and explosives hurt Margit through his own resistances

After 1103ebd2.

- **Changed (vendored entities crate, `elden_bosses.rs`):** `EntityWorld::elden_hurt(id, source, player)`: an `elden_boss::Source` straight into his `take` (commit 9: his resistances and stance, no hurt cooldown for anything but `Environment`, the red flash, his hurt voice at most every 10 ticks, and the fighters kept on a killing hit). A hit he takes from a player then gets the same blame and credit as D0's arms give a player's hit: `hurt_by` names the player, and `credit` with `minecraft:player` remembers the hit for the kill (`killed_by_player`, so his experience drops). `PLAYER` is that attacker name.
- **Changed (`minecraft_terrain`, `server.rs`):** `Command::EldenHurt { id, source, player }` beside `Command::EndHit`, sent by `ServerHandle::elden_hurt` beside `ServerHandle::end_hit` and run beside it in the command loop by `ServerSim::elden_hurt`, which applies the hit and then drops the remains of a death (`drop_death_loot`, as `mob_blast` does), so his gold and orbs fall at once.
- **Changed (`minecraft_survival`):**
  - `Cargo.toml`: `elden_boss = { path = "../elden_boss" }`. `Cargo.lock` gained the edge from one `check` without `--locked`.
  - `entities.rs`: `elden_boss_of(key)` beside `end_target_of`: a hit key whose byte is the Elden Ring bosses' family key (96, `elden_bosses::KEY`, D0's allocation in `docs/round10/mobs.md`) names a boss. In `Entities::shoot` such a key goes to `elden_hurt` with `Source::Bullet { mw2 }` and the shooter's id, and in `Entities::blast` with `Source::Blast { mw2 }` and the blast's player, in both cases right after the dragon's `end_hit` arm and instead of `mob_action` and `mob_blast`. `mw2` is the raw MW2 damage `sim::combat` sends, already scaled by distance and hit location (a head hit is x1.4), before any mob scale: his own `Bullet` (x0.2 x0.5, stance /60) and `Blast` (x0.8 x0.25, stance /15) rules apply instead of the generic melee hit at x0.2 and level blast at x0.8.
- **Already there (D0's registry):** his hit key encodes and decodes through D0's generic arm (`MobHit::Family(96, id)`), and his 1.4 x 3.0 box reaches MW2 through `EntityView::boxes`, which `shot_boxes` hands to MW2's trace and aim assist (`sim::voxel::set_mob_boxes`). A friend's boxes come from the friend's mirrored copy with the host's ids, and friends' shots reach the host's `shoot` and `blast` with the friend's id (`render_anim` `minecraft_world.rs` and `minecraft_levels.rs`), so threat, the kill and the reward go to the friend with nothing new.
- **Outside the registry:** `entities.rs` `shoot` and `blast` (survival) are not in D0's never-edit list (that names `encode`, `decode`, the key constants, `resting_monsters` and `mob_hit`, none of which changed); the brief (F5) names these two sites. Each gained one arm beside the dragon's, and the new `elden_boss_of` asks `decode` rather than a key constant of its own.
- **Design doc kept in step:** 2.2 names the route (`ServerHandle::elden_hurt` beside the dragon's `end_hit`). No number changed.
- **Tests:**
  - Entities, `world::families::bosses::elden_bosses::tests`:
    - `a_kneeling_margit_takes_half_again_from_bullets` (two fighters, 1280 health: 160 bullets of 30 points take 480 health and his 80 stance, he kneels, and the next bullet takes 4.5 and no stance; a bullet is never a critical)
    - `a_friends_bullet_credits_the_friend` (player 1 stands nearer and is his target; player 2's bullets, one every 10 ticks, turn him on player 2 after his move; `hurt_by` and the core's blame name player 2; player 2's killing bullet is a player's kill with two fighters' 150 experience, and both fighters are felled)
  - Survival, `entities::tests`:
    - `a_margit_hit_key_encodes_and_decodes` (byte 96 round-trips and names a boss; a wither, a silverfish and the free byte 97 do not; no End, fireball, bullet or frame key)
    - `an_m4_bullet_on_margit_takes_3_health_and_half_a_stance_point` (through `Entities::shoot` with 30 from the friend: 797 health, 79.5 stance, hurt by the friend; the host stands 41 blocks from his anchor, so he is no fighter and Margit keeps 800)
    - `a_frag_on_margit_takes_30` (through `Entities::blast` with 150 by the host: 770 health, 70 stance, hurt by the host)
    - `a_friends_killing_bullet_gives_the_friend_margits_reward` (a Margit summoned at 2 health at the same spot walks up to the friend and dies to one bullet from the friend: the friend gets "GREAT ENEMY FELLED" on the action bar and all 100 points of his orbs; the host, no fighter, gets neither)
  - Survival, `entity_view::tests::margits_gun_box_is_1_4_by_3` (one shot box with his family key, 1.4 wide and 3 tall from his feet, the same box MW2's trace and aim assist read).
- **Runs:** `test -p minecraftoss-entities --locked --lib`: 508 passed. `test -p minecraft_terrain --locked --lib`: 1577 passed and 3 ignored. `test -p minecraft_survival --locked`: 808 passed, `vanilla_world` 1 passed; its 7 Margit tests passed three runs in a row. `check -p launcher --locked`: green, with only the old render_frame and skate warnings. `elden_bosses.rs` passes `rustfmt --check --edition 2021` with empty output on its content with LF line ends, and no added line holds a comment. `elden_boss` is not touched.
- **Decisions:**
  - **The brief's `Hurt::Bullet` is `Source::Bullet`:** group er-rules named the enum of hit kinds `Source`, and `elden_boss::Hurt` is a source with the player and the tick. The command carries the source and the player; the tick is his own (`tick_of`), which only the level knows.
  - **No player attack sound for a bullet on Margit.** `mob_action` played the sword's swoosh (`entity.player.attack.strong`) at the shooter for every bullet on a mob; a bullet is not a swing, and the dragon's `end_hit` plays none either. His hurt voice and the red flash stay.
  - **Blasts neither push nor slow him:** `mob_blast` pushed by the impact and gave a stun's slowness, but nothing moves him but his own legs (commit 8) and no effect lasts on him (commit 8), so `elden_hurt` takes neither. A stun or flash with no damage takes nothing and is refused like any 0 hit.
  - **A blast with no player** (`by_player` is `None`) still hurts him as a `Blast` with no credit, like a level explosion by nobody.
  - **A player's MW2 hit is credited as direct** (`credit(.., true)`), as `mob_action` and `player_blast` credited it before.
  - **The survival tests' spot:** he is summoned on the corner of the friend's stone platform, 41 blocks from the host, so only the friend is a fighter at the wake. For the kill he first walks up to the friend: orbs dropped on the corner rolled off the platform (a first run got 80 of the 100 points).
- **Closes from release 3's known limits:** "MW2 bullets and blasts count as melee hits and level blasts until group er-guns. So a bullet breaks his stance at 1.5 times, and a close shot from the front at a kneeling Margit is a critical."
- **Owner's steps.** Release build, survival, Normal, by day, flat open ground in the Overworld. The class's M4A1 (or the kit's) with Red Dot Sight. His boss bar comes with group er-bar-friend, so these steps count hits. Name each screenshot by its label, `f5-22-1.png`.
  1. `/summon iw4l:margit ~ ~ ~10 {Health:20f}`: he rises. From where you stand, about 10 blocks, shoot his chest with single M4A1 shots and count the hit markers: he dies on the 7th body hit (each takes 3 of his 20 health; release 3 took 4). Each hit shows the hit marker and his red flash, his deep hurt voice comes at most every half second, and you no longer hear a sword swoosh with each hit. **Screenshot f5-22-1** as he sinks into the golden mist.
  2. The same with head shots (the top of his hood): 5 hits.
  3. `/summon iw4l:margit ~ ~ ~10 {Health:20f}` and the Barrett: two or three body hits kill him.
  4. `/summon iw4l:margit ~ ~ ~10 {Health:40f}`: cook a frag so it bursts right at his feet. He lives (it takes about 30; release 3 took about 60). A few M4A1 body hits finish him.
  5. `/summon iw4l:margit ~ ~ ~10`: hold the trigger on his body from about 10 blocks, reloading quickly, and eat a golden apple when you need one. After about 135 body hits (half his health) he raises his left hand and the golden hammer forms. Keep shooting: after about 160 hits in all (five or six magazines) he drops to one knee in sparks with the stunned sound. If you stop shooting for 6 seconds his stance refills, and the count for the kneel starts again. **Screenshot f5-22-2** as he kneels.
  6. While he kneels, walk up and shoot him from the front at close range: no critical sound, and he stays down for the full 4 seconds (a bullet is never a critical; release 3 made it one).
  7. `/summon iw4l:margit ~ ~ ~10 {Health:40f}` and fire an RPG rocket at his feet: it takes at least as much as the frag, and he dies by the second rocket at the latest.
  8. Optional, two windows (`scripts\mp-local.ps1`, with `/op Friend`): the host stands more than 40 blocks away. The friend runs `/summon iw4l:margit ~ ~ ~10 {Health:20f}` and kills him with 7 M4A1 body hits: only the friend sees "GREAT ENEMY FELLED" and hears the fanfare, and the friend picks up the 100 points of orbs. **Screenshot f5-22-3**, the friend's window.

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:frag_grenade*8,iw4l:barrett,iw4l:sniper_rounds*64,iw4l:rpg,iw4l:rocket*16,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

## Group er-bar-friend

Brief F6, commits F6.1 and F6.2, with design doc sections 9 and 10. It starts on 9f730490 (group er-guns).

### 23. Everyone near Margit sees his boss bar and hears the boss music

After 9f730490.

- **Changed (`minecraft_terrain`):**
  - New `elden_bar.rs`, like `wither_bar.rs`: `EldenBar { id, name, progress }`, `bosses(world)` (every Elden Ring boss of the level that is awake: not asleep, not walking home after a reset, not dead), `bars_for(bosses, feet)` (each boss within his def's `arena.bar`, 48 blocks, of the feet, measured in all three directions from his body) and `progress` (health over his scaled maximum, so two fighters' 1280 is a full bar). `COLOR` red and `OVERLAY` progress. `lib.rs`: the module beside `elden_effects`.
  - `server.rs`: `Output.elden_bars` beside `wither_bars`, filled beside it every tick by `elden_bars(sim, input)`: one list per player of the level, empty for a dead player.
- **Changed (`minecraft_survival`):** `PlayerOutputs.elden_bars` and its route beside the wither bars (`entities.rs`), `Player.elden_bars` (`player.rs`). `session.rs`: `ELDEN_BARS = 1 << 41`, `elden_bar_id(id)`, `elden_bar(bar)` (`BossBarOperation::Add` with `Text::literal` of his name, `BossBarColor::Red`, `BossBarOverlay::Progress`, and `play_music` as the only flag), `Session::send_elden_bars` (add, progress and remove, as the wither's) and `Sessions::send_elden_bars`.
- **Changed (`frame`, `hud`):** `MinecraftUi.elden_bars` beside `wither_bars`. `hud` `minecraft_boss.rs`: `shown_bars(ui)`, the dragon's, the raids', the withers' and then the Elden Ring bosses' bars in that order, which `draw_boss_bars` draws.
- **Changed (`render_anim`):**
  - New `minecraft_elden.rs` (like `minecraft_wither.rs`): `show_elden_bars` keeps `ui.elden_bars` in step with the host player's bars: red, progress overlay, asking for music, sliding to a new health like every bar. Called beside `show_wither_bars` in `minecraft_world.rs`.
  - `minecraft_players.rs` `struck`: the player's `elden_bars` from the outputs, beside the wither bars. `minecraft_host.rs`: `send_elden_bars` beside `send_wither_bars`, so friends get `ToClient::BossBar` with the id `1 << 41` plus his id; a friend keeps every bar id already (`remote.rs`).
  - `minecraft_music.rs`: `situational` plays `END_BOSS` (`music.dragon`) whenever a bar asks for music, in any dimension, and `update` stops `END_BOSS` when no bar asks any more and the situation has no music (outside the End). The host's music input counts `ui.elden_bars` too; the friend's already counts every bar.
- **Tests:**
  - Terrain: `elden_bar::tests::a_player_within_48_blocks_of_an_awake_margit_gets_his_bar` (47.5 blocks across or down gets it, 48.5 across or 34 across and 34 up does not; 200 of 800 health is a quarter bar), `an_asleep_margit_shows_no_bar` (asleep, walking home after a reset, and dead with his body still there), `a_player_in_another_level_gets_no_bar` (the same feet in another level's world get nothing). `server::tests::each_living_player_within_48_blocks_of_an_awake_margit_gets_his_bar_every_tick` (through `run_tick`: none asleep; 600 of 800 for the player 10 blocks away and none for the one 52 away; none while dead; none after a reset).
  - Survival: `session::tests::the_elden_bar_is_red_with_his_name_and_asks_for_music` (add, nothing when unchanged, progress, remove), `session::tests::margits_bar_ids_never_meet_wither_raid_or_dragon_ids`, `entities::tests::each_player_of_his_level_gets_margits_bar` (an empty list takes a bar away; a player of another level is not queued).
  - `hud`: `minecraft_boss::tests::the_host_shows_margits_bar_after_the_wither_bars` (with the red sprites). `render_anim`: `minecraft_elden::tests::the_host_sees_a_red_bar_named_margit_that_asks_for_music`, `minecraft_music::tests::boss_music_plays_in_the_overworld_while_a_bar_asks_for_it_and_stops_after` (at once, loops, stops, nothing else starts in 30000 ticks, starts again for the next fight; the Nether too), `minecraft_music::tests::the_end_music_rules_are_unchanged` (the End's situations; after the dragon's bar the End's own music takes over as before; with no bar no boss music). `the_situation_is_the_credits_then_the_boss_then_the_end` now expects the boss music outside the End when a bar asks.
- **Runs:** `test -p minecraft_terrain -p minecraft_survival -p render_anim -p hud -p frame --locked --lib --tests`: 1581 passed and 3 ignored, 811 and `vanilla_world` 1, 220, 200 and 28. `check -p launcher --locked`: green. The new `elden_bar.rs` and `minecraft_elden.rs` pass `rustfmt --check` (editions 2021 and 2024) with empty output on their staged content, and no added line holds a comment.
- **Decisions:**
  - **Only living players get the bar,** as vanilla's dragon and raid bars (`EntitySelector.ENTITY_STILL_ALIVE`): a dead player's list is sent empty, so the bar goes on the death screen and comes back after the respawn while he is near and awake. A dead player still gets an entry each tick, so a bar never sticks.
  - **48 blocks in all three directions from his body,** as the dragon's and the raids' bars measure (`distanceToSqr`), not from his anchor and not by chunk tracking like the wither's: the design doc says "within 48 blocks of him".
  - **The range is his def's `arena.bar`,** already in `margit.rs` (design doc 10), so every number stays there.
  - **The music stops at once** when no bar asks for it outside the End, and starts again at once for the next bar. In the End nothing changed: the dragon's bar is the only one there that asks, and the End's own music replaces the boss music as before. A wither's bar still asks for no music.
  - **The name is plain text** (`Text::literal`), since there is no language key for him (design doc 10).
- **Closes from release 3's known limits:** "There is no boss bar, no music" (the altar is group er-altar).
- **Owner's steps.** Release build, survival, Normal, by day, flat open ground in the Overworld, with the music volume up in the options. Name each screenshot by its label, `f6-23-1.png`.
  1. `/summon iw4l:margit ~ ~ ~10`: as he rises, a full red bar named "Margit, the Fell Omen" shows at the top of the screen, and the Ender Dragon fight music starts at once. **Screenshot f6-23-1** with the bar.
  2. Shoot him with the M4A1 and hit him with the sword: the bar slides down a little with each hit.
  3. With him awake 5-10 blocks in front of you, type `/tp @s ~ ~ ~-55` (55 blocks straight back): the bar is gone and the music stops at once. Within 5 seconds type `/tp @s ~ ~ ~55`: the bar is back, as low as you left it, and the music starts again.
  4. Run 45 blocks away from where you summoned him and wait 6 seconds: his fight resets, and the bar and the music go while he walks home. Walk back within 24 blocks of his spot: he wakes again with a full bar and the music.
  5. `/summon iw4l:margit ~ ~ ~6 {Health:20f}` and hit him twice with the sword: the bar is nearly empty, and it goes with the music as he sinks into the golden mist.
  6. Let a new Margit kill you (no armour, no totem): the bar goes on the death screen. Respawn and walk back to him: the bar returns while he is still awake.
  7. Build a Nether portal from the obsidian, light it and go through. `/summon iw4l:margit ~ ~ ~10` in the Nether: the same red bar and the same music. **Screenshot f6-23-2** in the Nether.
  8. Vanilla check: back in the Overworld, `/summon minecraft:wither ~ ~ ~10`: its purple bar comes, with no music. `/difficulty peaceful` removes it; then `/difficulty normal`.
  9. Optional, two windows (`scripts\mp-local.ps1`): a friend within 48 blocks of an awake Margit has the same red bar and hears the music; a friend 60 blocks away has neither. **Screenshot f6-23-3**, both windows.

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,minecraft:netherite_sword+sharpness=5,minecraft:shield,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:obsidian*14,minecraft:flint_and_steel
```
