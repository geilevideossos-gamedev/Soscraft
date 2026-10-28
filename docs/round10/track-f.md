# Track F: the first Elden Ring boss, Margit

The owner's words for round 10 (`docs/PROGRESS.md`, round 10 brief):

> F. Elden Ring bosses, first step (the next roadmap phase). Read the reference in C:\Dev\references\ELDEN-RING-Combat-Rewrite. If it's missing, clone Funny-Bones/ELDEN-RING-Combat-Rewrite there. Write docs/ELDEN-RING-BOSSES.md: how a boss fight fits into our game (Minecraft world, MW2 guns, multiplayer, saving), which boss to start with, assets, the boss bar, phases, and how players summon or find it. Then build that first boss as a playable proof of concept in a Minecraft world.

The reference was read (HEAD 606e6ec) and the design is written: **`docs/ELDEN-RING-BOSSES.md`** (called "the design doc" below). It picks Margit, the Fell Omen. This track builds him as a playable proof of concept. Every number Margit uses is in the design doc; the build agents read both files.

**The proof of concept is done when** the owner can summon Margit (an altar or `/summon`), fight him with MW2 guns and Minecraft weapons through both phases, see his boss bar with every player in range, play the fight with a friend who sees and hurts him too, save and load (the fight resets cleanly), and kill him for a reward.

ELDEN RING is not installed on this PC (`docs/round10/decisions.md`), so nothing comes from the game. Margit is our own model, our own numbers and vanilla Minecraft sounds read at runtime. Never commit any file from ELDEN RING or Minecraft, and never copy code from the reference: it has no licence file.

Track F has seven parts and nine commit groups, in this order (F1 and F2 are the two biggest parts, so each is split into two groups of two commits):

- **F1, groups `er-core` and `er-rules`:** the new pure crate `crates/elden_boss`: the boss as data, Margit's table and the brain (`er-core`); damage, stance, phases, arena rules and a headless whole fight (`er-rules`).
- **F2, groups `er-entity` and `er-arena`:** Margit lives in the entity world, is saved as a vanilla marker and takes Minecraft hits (`er-entity`); he is summoned with `/summon iw4l:margit` and walks and leaps in his arena (`er-arena`).
- **F3, group `er-look`:** his model, code-painted skin, animations, golden weapons, particles and sounds.
- **F4, group `er-fight`:** his strikes and daggers hurt players; his death drops the reward and tells every fighter.
- **F5, group `er-guns`:** MW2 bullets and explosives hurt him through his own resistances.
- **F6, group `er-bar-friend`:** the boss bar and music for every player in range, and friends see and fight him.
- **F7, group `er-altar`:** the altar (a lantern on two gold blocks) and the golden fog ring.

**Segments.** F1 (`er-core`, `er-rules`) touches no shared game file and starts from `minecraft-survival` now (segment `f1`). F2 to F7 add a mob family and must use track D0's shared mob registry, so they run as segment `f2` after D0 is merged into `minecraft-survival` and `minecraft-survival` is merged into `track-f`.

Paths and line numbers are as of 6e0e0573 (the track-d merge, checked out in `C:\Dev\mc-wt\track-d`). Re-check them before editing: tracks D0, D, W, M1-M3, E1 and E2 run at the same time, and D0 will move many of them. Vanilla facts were checked with javap on the 26.3 `client.jar` and in the 26.3 data inside it. Screenshots of this track are named `f01_...` and up.

**Multiplayer rule for every item.** The host owns every level, every mob and every rule. Friends run no AI: they mirror mobs from look tags and per-tick `MobShow`, so every drawn Margit state goes through `MobShow`. Particles are drawn by each viewer from the shown state, so they need no message. Everything a player earns is keyed by player id on the host and goes to that player, friends included. Nothing keys on player 0.

---

## F0. Shared points for the track

### Coordination with the other round 10 tracks
| Track | Shared with F |
|---|---|
| D0 | The mob registry. F2-F7 plug Margit in by D0's plug-in checklist (`docs/round10/track-d0.md`). The hit-key byte comes from D0's allocation. If D0 sets a per-family rule for `MobShow` or render dispatch, F follows it |
| E1 | `frame/src/minecraft_ui.rs` (E1 adds fields; F adds `elden_bars`). F does not touch `ambient_particles.rs`: no new kinds and no tint. Gold particles are `spell_particles.rs` `SpellKind::Effect` with a colour, as the wither's charge (`entity_view.rs:291-297`) |
| E2 | Arrows, tridents, thrown items and fireworks reach mobs through D0's generic arms after E2's `ti1` (no per-mob code in `thrown.rs`, `splash.rs` or `wind.rs`). Margit gets them from his D0 lines. F does not touch `entities.rs` `punch` |
| W | `MobShow` gets warden fields too. W or D creates a jar `AnimationLibrary` holder for vanilla keyframes; F builds its clips in code and does not need it |
| D | Breezes add `MobShow` fields and `mesh.rs` textures |
| M1-M3 | New families in the same registries; `death.rs` names |
| Everyone | `server.rs`, `server_mobs.rs`, `entities.rs`, `entity_view.rs`, `mob_tracking.rs`, `session.rs`, `render_anim` `minecraft_world.rs` and `minecraft_remote.rs` |

Rules for shared files:
- Every edit is additive: new modules, new match arms, new fields. Never restructure or reformat a shared function.
- New enum variants and match arms go next to a related one (Margit's next to the wither's), never at the end of a shared enum such as `Command`, `ToClient` or `Output`, so two tracks never add adjacent lines.

### The D0 registry
Margit is a new mob family. His wiring goes **through the D0 registry**: his own files plus one line per registry. Before F2 starts, read D0's checklist and its log and follow it exactly. Use Appendix A (where the wither is wired today) only for what D0 does not cover, and record each such site in the log.

The design doc (section 2.1) says why Margit is an entity world family and not a list of our own like the dragon. If D0's registry turns out not to fit a boss (for example, if it assumes one model per entity without extra parts), keep Margit's extra parts (golden weapons, daggers) inside his own family and renderer and say so in the log; do not change D0's registry for him.

### Protocol
`minecraft_net::PROTOCOL` is 6 (`crates/minecraft_net/src/messages.rs:7`). F6 adds `MobShow.elden`, which changes the wire. **No track-F commit bumps `PROTOCOL`** (`docs/round10/decisions.md`, as in track E): the release step bumps it once when anything on the wire changed since the last release (`messages.rs:7` and the asserts in `codec.rs`). Record under "## Protocol" in the log that `MobShow` changed. If `docs/round10/decisions.md` sets another rule by the time F6 runs, follow it.

### The new crate and vendored files
- `crates/elden_boss` is new (F1): edition 2024 from the workspace, `glam = "=0.29.3"`. New `.rs` files there pass `rustfmt --check --edition 2024`; new files in `minecraft_terrain` and the minecraftoss crates use edition 2021.
- If D0 keeps family code in the vendored entities crate, that crate gets a path dependency on `elden_boss` (F2). It is the first vendored crate that depends on a fork crate; record it. The minecraftoss crates have no workspace of their own (the root `Cargo.toml` excludes the folder and builds them as path dependencies), so the one root `Cargo.lock` covers it.
- `crates/minecraft_terrain` (F2: `elden_save.rs`, `Command::EldenHurt`) and `crates/minecraft_survival` (F2: `death.rs` names; F5: `Hurt::Bullet` in `entities.rs`) also need `elden_boss = { path = "../elden_boss" }`, added by the first commit that uses it, with `Cargo.lock` updated by one `check` without `--locked`.
- Count the vendored minecraftoss files the track patches and adds in the log ("Vendored files").

### Numbers
Every Margit number lives in one place, `crates/elden_boss/src/margit.rs`, and comes from the design doc sections named in each part. If a number has to change during the work, change it in `margit.rs` and in the design doc in the same commit, and say why in the log.

### Files and docs
- The track log is `docs/round10/track-f-log.md`: commits, decisions, vendored file counts, the wire change, "State doc changes", "Left for later", and each commit's owner steps and kit line.
- **Decisions go in the track log** while the track runs. The merge copies them into `docs/round10/decisions.md` under a "Track F" heading.
- Do not edit `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md`, `docs/PROGRESS.md` or `docs/round10/decisions.md`. Write their new wording in the log under "State doc changes": roadmap step 3 has its proof of concept; a "Bosses" paragraph for the state doc; the `MobShow.elden` line for the multiplayer doc.
- `docs/ELDEN-RING-BOSSES.md` may be edited by this track, only to keep its numbers and file names in step with the code.

---

## F1. The boss as data, the brain and a headless fight (groups `er-core` and `er-rules`, segment f1)

### What the owner asked
"Build that first boss as a playable proof of concept." This part builds the fight logic, which needs no game code and no game files.

### Today
- Nothing of Elden Ring exists in the fork.
- The reference's `src/sim` (read only, ideas only) is the model: a pure 60 Hz state machine with no engine types, stepped headless in tests, with actions as tables of windows. Design doc sections 4 and 15.
- No shared game file is touched in this part, only the workspace member list and `Cargo.lock`, so it runs while D0 is in progress.

### Plan
1. **"Elden Ring boss moves are plain data, starting with Margit's"**
   - New crate `crates/elden_boss`. Its `Cargo.toml` follows `crates/minecraft_net/Cargo.toml` (`edition.workspace`, `license.workspace`, `[lints] workspace = true`) with one dependency, `glam = "=0.29.3"`: not the workspace's glam 0.32, because the vendored entities crate and `minecraft_terrain` use 0.29 and Margit hands them `DVec3`. Add `"crates/elden_boss"` to the workspace `members` in the root `Cargo.toml`, in its alphabetical place. Run the first `check -p elden_boss` without `--locked` once so `Cargo.lock` gains the package, then always with `--locked`. Commit `Cargo.lock`.
   - `src/data.rs`: the types of design doc 4.2 and 4.3, with `'static` slices: `BossDef`, `DataSource` (`Estimate`, `Baked`), `Resist`, `Arena`, `Scaling`, `PhaseDef`, `MoveDef`, `Pick`, `Hold`, `Motion` (step, hop back, leap), `Strike`, `Shape` (arc, circle, ring), `Harm` (physical, holy), `Reaction` (flinch, stagger, knockdown), `Throw`, `Conjure` with `Weapon` (dagger, sword, hammer), `Cue` (a sound with volume and pitch, or a burst kind: golden mist, sparks, slam, crit), `Pose` (an enum of key poses shared with the renderer), `Branch` and `BranchTo` (a move, or end early with a recovery). `def(id)` and `IDS` list the bosses.
   - `src/margit.rs`: `MARGIT`, with every number of design doc 4.2, 5 (moves, tracking, branches, clips, tells, daggers), 6, 7.1-7.3, 8, 9.1 and 13, and `source: DataSource::Estimate`. Voices and cue sounds as in design doc 14.1. Pose keys for each move (ready, cane back, swing through, cane high, slam, throw wind-up, release, hop, crouch, airborne, landing, sword out, sweep through, hammer raised, hammer down, hand raised, kneel, and so on).
   - `check(&BossDef) -> Result<(), String>`, used by the tests and later by baked data. It refuses: a window outside its move; a melee strike whose last tracking tick is less than 8 ticks before it (leaps are exempt, their landing is fixed); a branch to an unknown key; pose keys out of order or not reaching the end; a distance from 0 to 30 blocks where no move of a phase fits; a conjured weapon that is not shown during the strikes that use it.
2. **"An Elden Ring boss picks, times and lands its moves like Margit"**
   - `src/shape.rs`: arc, circle and ring tests against a player's box (feet, width 0.6, height = eye height / 0.9), and a segment against a box (F4's daggers). An arc's height band reaches 1 block below his feet, so a player in a 2-deep hole next to him is still in reach.
   - `src/brain.rs`: `Brain::new(def, seed)` and `step(&Sense) -> Act`, with the states of design doc 4.4. Its own small random (SplitMix64, no new dependency), so a seed replays a fight.
     - The move clock stops during a hold. He turns toward his target at `turn` per tick only inside tracking windows (and through a hold that sits inside one).
     - Root motion: steps and hops spread over their ticks; a leap is a parabola from take-off to a landing point fixed when tracking ends, clamped to 6-12 blocks and to the leash, given as a displacement per tick.
     - Each strike is tested on every tick of its window against every eligible player and lands at most once per player. Rings hit when their edge passes under the feet of a player standing less than 1 block above the ground (each player in `Sense` carries the height of the ground under their feet).
     - Throws become `Act.throws`: from his left hand, aimed at the target's chest, at the dagger speed.
     - Choosing a move, cooldowns, idle gaps and branches as design doc 4.4, 5.1 and 5.2.
   - `Act.clip` is what F3 draws and F6 sends: the state, the move index, the move clock, whether it is held, the phase, whether he is awake.
3. **"An Elden Ring boss staggers, changes phase and resets like Margit"** (group `er-rules` starts here; commits 1 and 2 are `er-core`)
   - `src/hurt.rs`: `Brain::take(&Hurt) -> Taken`. `Hurt` has a source (`Bullet { mw2 }`, `Blast { mw2 }`, `Melee { damage, from }`, `Projectile { damage }`, `Explosion { damage }`, `Environment { damage }`), the attacker's player id if any, and the tick. `Taken` has the health dealt, the stance dealt, whether it was a critical, whether a kneel started, and whether he died. Design doc 7.1-7.3: resistances per source, x1.5 while kneeling, the critical (from within 60 degrees of his facing and 4 blocks, the first melee hit of a kneel, x3, he rises with a 20-tick recovery), stance refill after 120 quiet ticks, and threat per attacker over 200 ticks. `uses_hurt_cooldown(&source)` is true only for `Environment`. The brain also keeps every player who hurt him since the wake (F4's reward). While `Asleep` or in `Return` he takes nothing.
   - `src/fight.rs`: design doc 6, 8 and 9. The wake (from `Asleep`, or from `Return` without the 30-tick rise since he is already standing; the fighters within 40 blocks set the health factor; health keeps its share), the leash clamp, the reset after 100 ticks with nobody eligible within 40 blocks (full health, phase 1, full stance, cooldowns and threat cleared, `Return`, and a fade home when no path home is found within 100 ticks), being stuck for 100 ticks (a fade request beside the target, within 3 blocks across and 2 up or down of its feet; if the target's spot has none, no fade and he keeps throwing daggers), the phase shift at half health once the current strike window ends (stance cannot break during it), phase 2's cooldown factor, idle gaps and third dagger.
4. **"A whole Margit fight plays through without a window"** (group `er-rules`, with commit 3)
   - `src/whole_fight.rs` (test only): flat ground with no blocks, two scripted players, a fixed seed, at most 6000 ticks (design doc 4.4).
     - The rifleman strafes at 12-18 blocks and lands 7 bullets a second of 30 MW2 points, with a 40-tick reload after every 30.
     - The swordsman steps in when a strike window ends, hits for 11 at full charge every 13 ticks (one hit a kneel from the front), and runs straight away at 0.265 blocks a tick (MW2's run) when a wind-up starts within 4 blocks of him.
     - Players take their hits but are never killed in the script (their health is not modelled).

### Tests
- Data: `margit_passes_its_checks`, `a_strike_past_the_end_of_its_move_is_refused`, `every_distance_up_to_30_blocks_has_a_move_in_both_phases`, `the_hammer_moves_are_phase_two_only`.
- Brain: `a_player_at_two_blocks_draws_a_close_move_and_one_at_twenty_a_dagger`, `running_away_at_the_tracking_stop_dodges_the_cane_and_standing_still_does_not` (2.5 blocks in front, running straight back at 0.265 a tick from the last tracking tick), `a_swing_lands_once_per_player`, `a_jump_clears_the_hammer_shockwave`, `the_dagger_hold_lasts_10_to_30_ticks_and_delays_the_throw`, `the_cane_combo_ends_after_one_two_or_three_swings` (over many seeds), `a_leap_never_lands_outside_the_leash`, `a_move_on_cooldown_is_not_picked`, `the_same_seed_plays_the_same_moves`.
- Hurt and fight: `an_m4_body_hit_takes_3_health_and_half_a_stance_point`, `a_frag_at_its_centre_takes_30_health_and_10_stance`, `two_sword_hits_in_one_tick_both_count`, `environment_damage_uses_the_hurt_cooldown_and_player_hits_do_not`, `a_broken_stance_kneels_80_ticks_and_refills_after_6_quiet_seconds`, `a_melee_hit_from_the_front_on_a_kneeling_boss_is_a_critical_and_one_from_behind_is_not`, `half_health_starts_the_shift_when_the_current_swing_ends`, `stance_does_not_break_during_the_shift`, `two_fighters_at_the_wake_give_1280_health_and_keep_the_share`, `nobody_within_40_blocks_for_5_seconds_resets_the_fight`, `an_asleep_or_returning_boss_takes_no_damage`, `the_player_who_hurt_him_most_lately_is_his_target`, `an_unreachable_target_for_5_seconds_asks_for_a_fade`, `a_target_on_a_pillar_gets_daggers_not_a_fade`.
- Whole fight: `a_whole_margit_fight_shows_every_move_both_phases_a_kneel_and_ends` (every move key seen, phase 2 reached, at least one kneel and one critical, both players hit, he dies, and the same seed dies on the same tick).

### Owner's steps
None in game: the crate is not in the game yet. The log says so for each commit, with no kit line.

---

## F2. Margit lives in Minecraft worlds (groups `er-entity` and `er-arena`, segment f2, needs D0)

### What the owner asked
"Summon or find it", "it saves and loads mid-fight or resets cleanly", and fighting it with Minecraft weapons.

### Today
- `/summon` refuses an `iw4l:` id twice: the command registry is `SIMULATED_KINDS` (`crates/minecraft_survival/src/commands.rs:147`, from `crates/minecraft_terrain/src/server_mobs.rs:433`), and `ServerSim::summon` (`crates/minecraft_terrain/src/server.rs:1549`) calls `level.summon_mob` (`third_party/minecraftoss/world/src/level/spawning.rs:206`), which needs the jar's default tag for the kind and refuses monsters on Peaceful by `MobCategory`, which knows no `iw4l:` id.
- A saved entity the loader does not know goes to `ServerSim::dormant` (`server.rs:931-934`) and is saved again as it came. Markers are dormant today.
- Vanilla 26.3 (javap): `EntityType.create` drops an entity with an unknown id and logs "Skipping Entity with id {}"; every entity keeps its custom data as `data` (`Entity.customData`), markers included. Design doc 12.1.
- Every mob has a hurt cooldown (`third_party/minecraftoss/entities/src/health.rs:83-110` `hurt_absorbed`).
- The wither is the pattern for a boss with its own AI in the entity world: `third_party/minecraftoss/entities/src/wither.rs` (state), `world/withers.rs` (`WitherEntity`, `spawn_wither` at `:411`, `tick_wither` at `:451`, `hurt_typed` at `:181`), `crates/minecraft_terrain/src/wither_save.rs` (load and save fields).
- Player candidates (`third_party/minecraftoss/entities/src/tempt.rs:6-27` `PlayerCandidate`) carry `alive`, `spectator` and `attackable`.

### Plan
1. **"Margit, an Elden Ring boss, lives in Minecraft worlds and is saved like vanilla"**
   - The family by D0's checklist. An `EldenBossEntity` store: id, `&'static BossDef`, `Brain`, `Body` (1.4 x 3.0, step height 1.0), health and `DamageState`, anchor, yaw and body yaw, look control, ground navigation, voices, daggers (filled in F4), random, previous position, tick count, `no_ai`. `spawn_elden_boss(def, position, yaw, anchor, no_ai) -> id`, `elden_bosses()`, `elden_boss_mut(id)`.
   - `entity_type(id)` is the def's id (`iw4l:margit`). He is persistent and never despawns by distance (D0's flag, or `crates/minecraft_terrain/src/dormant.rs:14-19` `NEVER_FAR_AWAY`). Peaceful removes him like a monster (the wither's line, `world.rs:5902`). Knockback resistance 1. No fall damage. No potion effects apply.
   - Voices through `resolve_voices` (`world.rs:742`) with `Voice::Event` only (there is no `entity.margit.*` sound, so `Voice::Hurt` and `Voice::Death` would name missing events; ambient, hurt and death come from design doc 14.1), category `"hostile_volume"` as the wither's (`world.rs:5185`).
   - Mobs do not target him (iron golems, wolves and the like skip boss ids) and he ignores them (design doc 16). A mob-made hit that reaches him anyway (a skeleton's stray arrow, a creeper's blast) is `Projectile` or `Explosion` with no player id.
   - If D0 keeps family code in the vendored entities crate: `third_party/minecraftoss/entities/Cargo.toml` gets `elden_boss = { path = "../../../crates/elden_boss" }`.
   - New `crates/minecraft_terrain/src/elden_save.rs`, the carrier of design doc 12.1:
     - `claims(tag)`: a `minecraft:marker` whose `data.iw4l.boss` is a known boss id.
     - `spawn(world, tag)`: Margit at his saved position in the `Return` state (full base health, phase 1; he walks back to his anchor and sleeps there), with his saved anchor (his position if it is missing). It never reads `Health`.
     - `fields(entity)`: `id` `minecraft:marker` and `data: {iw4l: {boss, anchor, yaw}}` for `mob_tags_where`'s `add` (`server_mobs.rs:1497`; its `fields` are inserted last, so they replace the `id` it writes). `add` also writes `Health`, `Motion` and the like; vanilla's marker ignores them and our loader never reads `Health` from a saved tag.
     - `spawn_saved` (`server_mobs.rs:580`) asks `elden_save` first; every other marker stays dormant. A friend's copy comes through the same call (`mob_tracking.rs:787` `spawn_tracked`), so the friend gets the anchor from the same tag.
2. **"Minecraft weapons hurt Margit with no hurt cooldown, and his stance breaks"**
   - The family's hurt arms (D0's generic ones, or the wither's sites in Appendix A) turn each hit into an `elden_boss` `Hurt` and call `Brain::take`:
     - a player attack (`third_party/minecraftoss/entities/src/world/combat.rs:71-110` `player_attack_with`, after vanilla charge, crit, sweep and enchantments) is `Melee` with the attacker's position for the critical's front check;
     - arrows, tridents and thrown items are `Projectile`;
     - level explosions (TNT, creepers, crystals, beds, anchors; the blaster path at `world.rs:2995-3020`) are `Explosion`;
     - fire, lava, cactus, drowning and suffocation are `Environment`.
   - Player-made hits skip `hurt_absorbed`; `Environment` keeps it (design doc 7.2). `Taken` sets health, death and the kill credit (the attacker's player id, as `player_attack_with` credits today).
   - His death joins the death list with no drops yet (F4 adds them). A kneel and a critical play their sounds (`entity.ravager.stunned`, `entity.player.attack.crit`) as voices; F3 adds their particles.
3. **"An Elden Ring boss can be summoned in a Minecraft world"** (group `er-arena` starts here; commits 1 and 2 are `er-entity`)
   - The command registry lists the boss ids beside `SIMULATED_KINDS` (`commands.rs:147`), or in D0's list if it replaced it, so `/summon iw4l:margit` is accepted and suggested.
   - `ServerSim::summon` (`server.rs:1549`): for a boss id, `elden_save::summon(world, def, position, y_rot, nbt)` runs before `level.summon_mob`. On Peaceful it refuses with vanilla's "You can not summon a monster in peaceful mode" (`level.difficulty == 0`). It gives a fresh UUID (`SpawnLevel::next_uuid`), puts the anchor at the position, uses a `Health` from the NBT if there is one (tests start him near half health), and stores his tag in `mob_tags` like other summons, so saving and `MobAdd` see him.
   - Death lines name him: `crates/minecraft_survival/src/death.rs:686` `entity_name` returns `elden_boss::def(id).name` for a boss id ("Sos was slain by Margit, the Fell Omen").
4. **"Margit walks, turns and leaps on blocks and stays in his arena"**
   - Each tick (`tick_elden_boss`, dispatched like `tick_wither` at `world.rs:6646`):
     - build the `Sense` from the level's `PlayerCandidate`s (eligible: `alive`, not `spectator`, `attackable`), with each player's ground height from the blocks under their feet (the candidate has no on-ground flag) and `reachable` from a path plan (`third_party/minecraftoss/entities/src/navigation.rs`, `GroundNavigation` and `plan_walk_path`), planned at most every 10 ticks;
     - step the brain;
     - apply the `Act`: walking through ground navigation at speed 0.3, facing and head look, root motion and leaps as collision moves on `Body` (`third_party/minecraftoss/entities/src/movement.rs`; a leap that meets a wall drops with gravity), and a fade as a move to a standing spot (a solid block with room for his 1.4 x 3.0 box) beside the requested point;
     - `Act.hits` and `Act.throws` are left for F4.
   - The arena of design doc 8 comes from the brain: wake, leash, reset, `Return` and stuck.

### Tests
- `elden_save`: `a_hurt_margit_loads_at_full_health_and_walks_home` (hurt to 300, saved with `mob_tags_where`, loaded into a fresh world: full health, phase 1, `Return`, the same anchor), `a_plain_marker_stays_dormant`, `margits_saved_tag_is_a_vanilla_marker` (`id` is `minecraft:marker`, `data.iw4l.boss` is `iw4l:margit`, no `iw4l:` id where vanilla reads ids).
- Server: `summon_makes_one_asleep_margit_with_his_anchor_there`, `summoning_margit_on_peaceful_is_refused`, `a_summoned_health_is_used_and_a_saved_one_is_not`, `the_summon_command_knows_iw4l_margit`, `peaceful_removes_margit`, `margit_never_despawns_far_from_players`, `a_death_by_margit_names_him`.
- Hurt: `two_players_sword_hits_in_one_tick_both_count` (through `mob_action`), `an_arrow_and_tnt_hurt_margit`, `standing_in_fire_hurts_him_once_per_10_ticks`, `a_sword_hit_from_the_front_on_a_kneeling_margit_is_a_critical`.
- Movement (a flat test world, as the wither's tests use): `margit_walks_to_a_player_10_blocks_away`, `margit_stops_at_his_leash`, `a_leap_over_a_one_block_wall_lands_beyond_it`, `margit_resets_when_everyone_leaves_for_5_seconds`, `margit_fades_inside_a_wall_ring_he_cannot_reach` (a player in a hole inside a ring of blocks 6 high, 3 out, too high for his leaps; a plain 1 x 1 hole 2 deep is not enough, since his cane reaches 1 block below his feet), `a_player_in_a_two_deep_hole_beside_him_is_still_hit`.

### Owner's steps (idea; he is drawn from F3 on)
1. `/summon iw4l:margit ~ ~ ~10`: he appears and comes after you; walk away 30 blocks from where he appeared: he stops at about 24 and turns to face you. **f01_margit_summoned**
2. Hit him with the netherite sword: each full-charge hit counts, crits with a jump; a hit while he is busy still counts.
3. Bow and trident hurt him. A TNT next to him hurts him (half as much as a mob of his size would take).
4. Run 45 blocks away and wait 5 s, come back: he walks back to his spot, asleep, at full health.
5. Save and quit while he is hurt, load: he is where he was, at full health, and walks home to sleep. **f02_after_load**
6. `/difficulty peaceful`: he vanishes. `/difficulty normal`.

```
IW4L_MINECRAFT_KIT=minecraft:netherite_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:trident,minecraft:shield,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:cobblestone*64,minecraft:iron_shovel,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:totem_of_undying
```

---

## F3. Margit is drawn (group `er-look`)

### What the owner asked
"Build that first boss as a playable proof of concept", with stand-in assets (the game is not installed).

### Today
- `crates/minecraft_terrain/src/posed_cube.rs`: `Cube`, `Skin`, `part_pose`, `append`; units are 1/16 block.
- `crates/minecraft_terrain/src/keyframes.rs`: `Definition::new(length, looping)`, `add(bone, Channel::new(target, keyframes))`, `Keyframe::new`, `degree_vec`, `pos_vec`, Catmull-Rom, `apply(millis, scale, &mut Moved)`, `apply_walk`. Definitions can be built in code.
- `crates/minecraft_terrain/src/wither_render.rs`: a boss model scaled by its root matrix (`model_root(feet, scale)` at `:198`), drawn from `crates/minecraft_survival/src/entity_view.rs:1099`; its textures are listed at `mesh.rs:544`.
- `crates/minecraft_terrain/src/glow_render.rs`: `FULL_BRIGHT`, `append_glowing`.
- The code-made white tile: `mesh.rs:108-109` (`WHITE_PATH`, `WHITE_TEXTURE`), listed at `:530`, skipped by the pack read at `:1672`, made at `:1760`.
- Particle kinds: `ambient_particles.rs:42-59` (`Totem`, `EndRod`, `Crit`, `Poof`, `Explosion` and others); `ambient_particles::add(kind, at, motion)` takes no colour. Coloured particles: `spell_particles.rs` `SpellParticles::spawn(SpellKind::Effect, at, motion, color, ...)`, used for the wither's charge at `entity_view.rs:291-297`.
- `posed_cube::Skin` has no tint; `glow_render::append_glowing` is the Glowing effect's white hull drawn through walls, not a weapon glow.
- Client-side effects from mirrored state: `entity_view.rs:278` `wither_effects`. Pose push `client_mobs.rs:1291`, shadow `:1370`.

### Plan
1. **"Margit is drawn as a hooded old man with a cane"**
   - New `crates/minecraft_terrain/src/elden_skin.rs`: `PATH` and `TEXTURE` (`minecraft:iw4l/margit`), and `paint() -> RgbaImage`, a 64 x 64 skin painted from a small palette with seeded noise per UV box: dark ragged cloak, fur trim, grey beard, pale skin, gold eyes, bone horns, cane wood, and the gold of the dagger, sword and hammer. In `mesh.rs`, beside the white tile: listed with the textures, skipped by the pack read, painted in the cell loop (the atlas tile is 64 pixels, `mesh.rs:1665`, so the skin fills one cell at full resolution). No PNG is committed.
   - New `crates/minecraft_terrain/src/elden_render.rs`: Margit's parts (body leaning forward, cloak front and back with a ragged hem, hood, head, two eye pixels, beard, two horns, arms, legs, cane; and the golden dagger, sword and hammer as parts) at a humanoid layout, scaled by the root so he stands about 3 blocks tall, hunched. Idle and walk (`apply_walk` from the distance moved), head look toward his target, shadow radius 1.0.
   - Drawn through D0's render registry line, or a `meshes` call beside the wither's (`entity_view.rs:1099`), and the pose push and shadow beside the wither's (`client_mobs.rs:1291`, `:1370`), for host and friend views alike.
2. **"Margit's moves and golden weapons are animated from his move table"**
   - A pose table in `elden_render.rs`: bone rotations (and offsets) for every `elden_boss::Pose`.
   - One clip per move, built once into a `keyframes::Definition` from `MoveDef.poses` (Catmull-Rom; seconds are ticks / 20). The clip samples the move clock from `Act.clip`, which stops during a hold, so a delayed swing freezes on screen exactly as long as the brain holds it. Clips for wake, shift, kneel, fade and death (design doc 5.3).
   - Golden weapons show only inside their `conjure` window: gold pixels of his skin, drawn at full light (`Skin` sky and block `FULL_BRIGHT`), growing from nothing to full size over 4 ticks (a scale, so no translucent pass is needed). No `append_glowing` (see Today).
   - The tell: his eyes grow and brighten in the 8 ticks before each strike of a delayed swing (design doc 5.4).
3. **"Golden mist, sparks and dust show Margit's spawn, phase change, slams and death"**
   - `EntityView` effects for each boss from the shown clip, clock and phase, the same on host and friend, with nothing sent: each `Cue::Burst` on its tick (golden mist as `Totem` plus gold `SpellKind::Effect`, conjure sparks as `EndRod` at the left hand, a slam as a ring of `Poof` and one `Explosion`, a kneel and a critical as `Crit`), a gold shimmer of 2 gold `SpellKind::Effect` a tick in phase 2, and mist on wake, fade and death.
   - Each `Cue::Sound` on its tick becomes a `Voice::Event` on the host (friends hear mob voices already), with the steps as `entity.iron_golem.step` low.

### Tests
- `the_skin_paints_the_cloak_dark_and_the_beard_grey_in_their_uv_boxes`.
- `every_pose_sets_every_bone_it_names`, `each_moves_clip_lasts_as_long_as_the_move`, `a_held_swing_keeps_its_pose_while_the_clock_stops`.
- `the_dagger_part_shows_only_while_conjured`, `the_golden_weapons_are_full_bright_and_gold_in_the_skin`, `his_eyes_flare_before_a_delayed_swing`.
- `a_slam_cue_bursts_on_its_tick_for_host_and_friend_views`, `phase_two_drips_gold`.
- A mesh test: a standing Margit is about 3 blocks tall and 1.4 wide at his feet.

### Owner's steps (idea)
1. `/summon iw4l:margit ~ ~ ~10`: golden mist, and he rises: a hunched old man in a dark hooded cloak, grey beard, gold eyes, small horns, a cane, about 3 blocks tall. **f03_margit_rises**
2. Walk around him at 6-10 blocks: he limps after you and circles, his head following you.
3. Stand 2-3 blocks in front: the cane combo; the third swing rises, holds, then falls, and his eyes flare just before. **f04_cane_combo**
4. A golden sword forms in his left hand with a chime and sweeps wide. **f05_golden_sword**
5. Step back 15-20 blocks: a golden dagger forms, he holds it, throws it. **f06_dagger**
6. At 10-14 blocks: he crouches, roars and leaps, and dust rings out where he lands. **f07_leap_slam**
7. Break his stance (F2's sword hits): he drops to one knee in sparks.

Kit: F2's.

---

## F4. Margit fights back and pays out (group `er-fight`)

### What the owner asked
"Fight it with guns and Minecraft weapons... death and reward."

### Today
- `PlayerHit` and `PlayerHitKind` (`third_party/minecraftoss/entities/src/world.rs:595-627`): `Melee { attacker, hunger_ticks, lift, throw, disable_shield_seconds }` and `Magic { at }` cover physical and holy hits.
- `crates/minecraft_survival/src/entities.rs:1882` `mob_hit` applies difficulty (`difficulty_scaled`), knockback and death causes; `Magic` with a mob source is `indirect_magic` ("was killed by ... using magic", `death.rs:186`, `:245`). Vanilla 26.3 data: `indirect_magic` is in `#bypasses_armor` and so in `#bypasses_shield`.
- `MobDeath` (`world.rs:633-650`): loot table, equipment, experience; the wither's nether star at `world.rs:4646-4653`.
- Action-bar lines to one player: `Clientbound::System { overlay: true }` (`crates/minecraft_survival/src/sleep.rs:385`, `crates/render_anim/src/minecraft_players.rs:1854`); friends show them (`render_anim/src/minecraft_remote.rs:701-718`).

### Plan
1. **"Margit's cane, golden weapons and daggers hurt players"**
   - Each `Act.hits` entry becomes a `PlayerHit` with `source: Some(id)`. Physical: `PlayerHitKind::Melee` from his position, with the reaction's throw and lift (flinch: vanilla knockback; stagger: throw 0.8; knockdown: lift 0.5 and throw 1.2) and `disable_shield_seconds` 5 for hammer hits. Holy: `PlayerHitKind::Magic { at }`. Design doc 7.4.
   - Daggers: each `Act.throws` entry starts a dagger in the entity (start, velocity, age). Each tick a dagger moves 1.5 blocks, stops at the first solid block on its segment (a block ray as in `third_party/minecraftoss/entities/src/sight.rs` `line_of_sight`) with `item.trident.hit`, or hits the first eligible player whose box the segment crosses (`elden_boss::shape`) as a holy hit, and is gone after 40 ticks. Daggers are drawn with F3's dagger part at their position and heading.
2. **"Margit drops gold and experience when he dies, and every fighter sees GREAT ENEMY FELLED"**
   - His death (the wither's pattern at `world.rs:4646`): `MobDeath { table: None, equipment, experience }` with design doc 13: 1 enchanted golden apple, 2 gold blocks, 3 gold ingots per fighter, and 100 experience plus 50 per fighter after the first. The fighters are the wake's fighters plus every player who hurt him since the wake (the set the brain keeps, F1).
   - The fighters' ids leave the level in an `Output` list, routed per player beside the wither bars (`entities.rs:399-403` into the player's outputs, read in `render_anim/src/minecraft_players.rs` beside `outputs.wither_bars` at `:941` as of 6e0e0573). Each gets "GREAT ENEMY FELLED" on the action bar (an overlay system line, as the sleep messages; it is white, because `McOverlayMessage` has no colour) and `ui.toast.challenge_complete` for that player only (heard by the actor only, like `heard.rs` `ACTOR_ONLY`).

### Tests
- `a_cane_hit_becomes_a_melee_player_hit_from_margit`, `a_holy_hit_is_magic_and_ignores_a_raised_shield`, `the_hammer_disables_a_shield_for_5_seconds`, `a_knockdown_lifts_and_throws_the_player`.
- `a_dagger_stops_at_a_wall`, `a_dagger_hits_the_first_player_on_its_path`, `a_dagger_is_gone_after_40_ticks`.
- `margit_drops_gold_and_experience_by_fighters`, `every_fighter_is_told_great_enemy_felled` (host and friend lines), `a_player_killed_by_a_dagger_is_told_killed_using_magic`.

### Owner's steps (idea)
1. Let the cane hit you: knocked back a little, about 2 hearts on Normal without armour. Raise the shield: cane hits are blocked.
2. A dagger hits you through the raised shield and armour. **f08_dagger_hit**
3. A hammer slam on your raised shield (phase 2, F2's `Health` summon): the shield is down for 5 s. The shockwave ring: jump over it and nothing happens.
4. A leap slam tosses you up and back.
5. Let him kill you (take the armour off, no totem): "Sos was slain by Margit, the Fell Omen", or "... using magic" for a dagger.
6. Kill him: he sinks into golden mist; "GREAT ENEMY FELLED" (white) on the action bar with the challenge sound; an enchanted golden apple, 2 gold blocks, 3 gold ingots and experience orbs. **f09_felled**

Kit: F2's.

---

## F5. MW2 guns and explosives (group `er-guns`)

### What the owner asked
"Fight it with guns."

### Today
- Bullets: `sim::combat` traces mob boxes (`crates/sim/src/combat.rs:1250-1256`) with the hit location from the height on the box (`crates/sim/src/voxel.rs:276-285`), then `Entities::shoot` (`crates/minecraft_survival/src/entities.rs:1387`) turns the key into a `MobHit` and runs `mob_action` with the damage x0.2 (`HEALTH_SCALE`, `:33`). Blasts: `Entities::blast` (`:1426`) runs `mob_blast` with x0.8.
- Both lose what kind of hit it was. The dragon is routed before `decode` (`end_target_of`, `:2079`) to `ServerHandle::end_hit` (`server.rs:3964`, `Command::EndHit` at `:3456`, run at `:4140`).
- Keys: kind in the top byte (`entities.rs:1987-2085`); bytes up to 35 are taken.
- The host and friends' levels call `shoot` with the shooter's id (`crates/render_anim/src/minecraft_world.rs:993-1003`, `minecraft_levels.rs:299`).

### Plan
1. **"MW2 guns and explosives hurt Margit through his own resistances"**
   - His hit key from D0's allocation, in `encode` and `decode` (or D0's table).
   - In `shoot` and `blast`, a boss key goes to `ServerHandle::elden_hurt(id, hurt)` (beside `ServerHandle::end_hit`, `server.rs:3964`) with `Hurt::Bullet { mw2 }` or `Hurt::Blast { mw2 }`, the raw MW2 points (before any mob scale; for bullets already distance- and hit-location-scaled by `sim::combat`), and the player id, instead of `mob_action` and `mob_blast`. New `Command::EldenHurt` beside `Command::EndHit` (run beside it at `server.rs:4140`), run by `ServerSim::elden_hurt` into `Brain::take`, with the same credit and death path as F2's hurt arms.
   - His box (1.4 x 3.0) reaches MW2 through `EntityView::boxes` by D0's registry, so aim assist sees him too. A friend's own boxes come from the friend's mirrored copy, so the friend's key matches the host's.

### Tests
- `a_margit_hit_key_encodes_and_decodes`.
- `an_m4_bullet_on_margit_takes_3_health_and_half_a_stance_point` (through `Entities::shoot` with 30), `a_frag_on_margit_takes_30` (`blast` with 150), `a_kneeling_margit_takes_half_again_from_bullets`.
- `a_friends_bullet_credits_the_friend` (threat and the kill go to player id 2).
- `margits_gun_box_is_1_4_by_3`.

### Owner's steps (idea)
1. M4A1 body shots: hit markers, and the bar (F6) drops a little each hit, about 270 hits for a full bar alone. Head hits take a little more. **f10_m4_hits**
2. A frag at his feet takes a clear chunk (about 4%). A Barrett chest hit about 1.3%. An RPG rocket a big chunk.
3. Keep shooting: his stance breaks after a while and he kneels.

```
IW4L_MINECRAFT_KIT=iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:barrett,iw4l:sniper_rounds*64,iw4l:rpg,iw4l:rocket*16,minecraft:netherite_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:totem_of_undying
```

---

## F6. The boss bar, music and friends (group `er-bar-friend`)

### What the owner asked
"The boss bar", for "every player in range", and "multiplayer": friends see and fight it.

### Today
- The wither's bar is the template: `crates/minecraft_terrain/src/wither_bar.rs` (`bars_for` at `:33`), computed per player each tick (`server.rs:4293`, `:4323-4333`, `Output.wither_bars` at `:3683`), routed per player (`crates/minecraft_survival/src/entities.rs:186`, `:399-403`; `player.rs:58`), sent to friends (`session.rs:282-296`, `Sessions::send_wither_bars` `:1299`, from `render_anim/src/minecraft_host.rs:644`), shown on the host (`render_anim/src/minecraft_wither.rs`, called at `minecraft_world.rs:1263`) in `ui.wither_bars` (`frame/src/minecraft_ui.rs:271`), drawn after the dragon and raid bars (`hud/src/minecraft_boss.rs:37-58`).
- Bar ids: dragon 1 (`session.rs:38`), raids `0x100` and up, withers `1 << 40` plus the id (`session.rs:831-836`). Friends keep any bar id (`minecraft_survival/src/remote.rs:521-560`).
- Music: `render_anim/src/minecraft_music.rs:37` `situational` plays `END_BOSS` (`music.dragon`) only in the End; the host asks `ui.boss_bars` for `music` (`minecraft_world.rs:1586`), the friend asks every bar (`minecraft_remote.rs:1773`). With no situational music the manager lets the current track play on (`tick`, `:117`).
- Friends: `MobShow` (`crates/minecraft_terrain/src/mob_tracking.rs:65-97`), filled for withers at `:463`, applied at `:705`; `tracking_range` at `:285-306` (unknown kinds get 5 chunks).

### Plan
1. **"Everyone near Margit sees his boss bar and hears the boss music"**
   - New `crates/minecraft_terrain/src/elden_bar.rs` like `wither_bar.rs`: `EldenBar { id, name, progress }`, `bosses(world)`, `bars_for(bosses, feet)`: awake, same level, within 48 blocks. `Output.elden_bars` beside `wither_bars`, filled beside it.
   - `crates/minecraft_survival`: `Player.elden_bars`, routing beside the wither bars in `entities.rs` (and in `render_anim/src/minecraft_players.rs` beside `wither_bars` at `:756` and `:941` as of 6e0e0573), `Session::send_elden_bars` and `Sessions::send_elden_bars` beside the wither ones, `ELDEN_BARS = 1 << 41` and `elden_bar_id(id)`, and `elden_bar(bar)`: `BossBarOperation::Add` with `Text::literal(name)`, `BossBarColor::Red`, `BossBarOverlay::Progress`, flags `play_music` only (design doc 10).
   - Host UI: `frame::MinecraftUi.elden_bars`, filled by new `crates/render_anim/src/minecraft_elden.rs` (like `minecraft_wither.rs`) called beside it, drawn after the wither bars in `hud/src/minecraft_boss.rs`.
   - Music: `situational` plays `END_BOSS` whenever a bar asks for music, in any dimension; `update` stops `END_BOSS` outside the End when no bar asks any more. The host's input counts `elden_bars` too; the friend's already counts every bar. Vanilla bosses are unchanged (only the dragon's bar asks for music, and only in the End).
2. **"A friend sees Margit's moves and fights him"**
   - `MobShow.elden: Option<EldenShow>` (or D0's per-family show): the clip state, the move index, the move clock, held, phase, awake, and up to three daggers (start, velocity, age). Filled in `shows` beside the wither's and applied in `show_on` to the friend's copy, which never steps its brain and draws from the shown state. `None` for every other mob, so they pay one byte.
   - `tracking_range` 10 for boss ids (or D0's table). The friend gets the anchor from the look tag (the carrier).
   - Friends' bullets and swings already reach the host with their ids (F2, F5); nothing to add.
   - No `PROTOCOL` bump in the commit (F0).

### Tests
- `a_player_within_48_blocks_of_an_awake_margit_gets_his_bar`, `an_asleep_margit_shows_no_bar`, `a_player_in_another_level_gets_no_bar`.
- `the_elden_bar_is_red_with_his_name_and_asks_for_music` (session), `margits_bar_ids_never_meet_wither_raid_or_dragon_ids`.
- `the_host_shows_margits_bar_after_the_wither_bars`.
- `boss_music_plays_in_the_overworld_while_a_bar_asks_for_it_and_stops_after`, `the_end_music_rules_are_unchanged`.
- `a_friend_sees_margit_swing_and_his_daggers_fly` (`spawn_tracked` from his look tag, then `show_on`), `a_friend_draws_the_same_clip_time_as_the_host`.

### Owner's steps (idea)
1. As he wakes: a red bar "Margit, the Fell Omen" at the top, and the dragon fight music starts. **f11_bar_and_music**
2. Walk 50 blocks away: the bar goes. Come back: it returns while he is awake. When he dies or resets, the bar and the music stop.
3. Friend window: the friend sees him rise, every move, the golden weapons and daggers, hears him, has the bar and the music. **f12_friend_view**
4. Wake him with both of you near: the bar takes longer to empty than alone (x1.6).
5. The friend shoots and slashes him: the bar drops for both. The friend hurts him most for 10 s: he turns on the friend.
6. He kills the friend: the death line names him; the friend respawns and the fight goes on.
7. The friend lands the last hit: both see GREAT ENEMY FELLED.

Kit: F5's.

---

## F7. The altar and the fog ring (group `er-altar`)

### What the owner asked
"How players summon or find it."

### Today
- The wither's build: `third_party/minecraftoss/world/src/level/wither_build.rs:130-170` `wither_skull_placed` finds the pattern, clears its blocks (no drops) and pushes a summon tag; it does nothing on Peaceful. It runs on every player placement in `ServerSim::player_edit_block` (`server.rs:2357`, the call at `:2375`).
- Particles drawn by each viewer: `entity_view.rs:278` `wither_effects`.

### Plan
1. **"Margit rises from golden mist when a lantern is set on two gold blocks"**
   - New `crates/minecraft_terrain/src/elden_altar.rs`: `find(placed, block_at) -> Option<Altar>`, pure and tested without a level: the placed block is a standing `minecraft:lantern` (`hanging=false`) and the two blocks under it are `minecraft:gold_block`.
   - Called in `player_edit_block` (`server.rs:2357`) after `wither_skull_placed` (`:2375`). Not on Peaceful. It clears the three blocks without drops (as the wither build does) and summons through `elden_save::summon` at the pillar's base, facing the nearest eligible player in the level. `Command::PlayerEdit` carries no player id, and adding one would change a shared command, so the placer is found as the nearest player (they stand within reach of the lantern). That player is within 24 blocks, so he wakes at once, rising from golden mist.
2. **"Margit's arena shows a golden fog ring while he fights"**
   - `EntityView` effects: while a boss is awake, every 4 ticks a few gold `SpellKind::Effect` spell particles on the 24-block leash circle around his anchor, only on the part within 20 blocks of the viewer, at the ground's height there. The same for host and friend (the friend has the anchor from the look tag).

### Tests
- `a_standing_lantern_on_two_gold_blocks_is_an_altar`, `a_hanging_lantern_or_one_gold_block_is_not`.
- `the_altar_summons_margit_and_clears_its_blocks`, `the_altar_does_nothing_on_peaceful`.
- `the_fog_ring_shows_only_while_he_is_awake_and_near_the_viewer`.

### Owner's steps (idea)
1. Stack two gold blocks and put a lantern on top: the three blocks vanish into golden mist and Margit rises facing you (the nearest player). **f13_altar**
2. A lantern hanging under a block beside two gold blocks: nothing. One gold block and a lantern: nothing. On Peaceful the full altar does nothing.
3. While he fights, a golden ring of sparks shows on the ground about 24 blocks around his spot. **f14_fog_ring**

```
IW4L_MINECRAFT_KIT=minecraft:gold_block*16,minecraft:lantern*8,minecraft:oak_planks*64,minecraft:cobblestone*64,minecraft:netherite_sword+sharpness=5,minecraft:shield,minecraft:golden_apple*8,minecraft:cooked_beef*32,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64
```

---

## Owner's test for the release

What the owner runs on the release build after segment f2 merges. The final group (`er-altar`) copies this section into the track log, adjusted to what was built.

### Setup
- Survival, a new world, open flat ground (a plains). In every new world the host types `/difficulty normal` and `/gamerule keep_inventory true`; in two windows also `/op Friend`.
- Class: an **M4A1** (the first join gives it on hotbar slot 1 with frags and flashes in the grenade slots).
- Two windows: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r10f -Fresh -HostSound`. Both windows read the kit line.

### Host
1. `/summon iw4l:margit ~ ~ ~10`: golden mist, he rises. A hunched, hooded old man with a cane, about 3 blocks tall. The red bar "Margit, the Fell Omen" appears and the boss music starts. **f03_margit_rises**
2. Do not attack. Stand 2-3 blocks in front of him and watch: the cane combo (one, two or three swings; the third is raised, held and falls after his eyes flare), the delayed overhead slam, and the golden sword sweep. Run straight back as a swing winds up: it misses. **f04_cane_combo**, **f05_golden_sword**
3. Back off to 15-20 blocks: a golden dagger forms and is thrown after a pause. Strafe: it misses. Let one hit you with the shield raised: it goes through, and through the armour. **f06_dagger**
4. Stand close and wait: he hops back and throws two or three daggers.
5. At 10-14 blocks: he roars and leaps; the landing tosses you up and back. **f07_leap_slam**
6. M4A1 body shots: the bar drops a little each hit (about 270 hits alone). A frag at his feet takes about 4%; a Barrett chest hit about 1.3%; an RPG rocket much more. **f10_m4_hits**
7. Netherite sword: every full-charge hit counts; jump for crits. Keep hitting and shooting until he drops to one knee in sparks with a stunned sound. Hit him from the front with the sword: a big critical, and he gets up. **f15_kneel_critical**
8. Bow and trident hurt him; a TNT beside him hurts him.
9. Keep fighting. At half health he stops, raises his left hand, and a golden hammer forms with a flash and a deep sound. **f16_phase_two** (In a later run, alone, `/summon iw4l:margit ~ ~ ~10 {Health:401f}` and one hit of any kind reach this at once.)
10. Phase 2: the hammer slam (a held wind-up, then the slam and a shockwave ring along the ground; jump over the ring), the jumping hammer slam from range, and two cane swings into the hammer. A hammer hit on the raised shield disables it for 5 s. **f17_hammer_slam**
11. Kill him: he sinks into golden mist; "GREAT ENEMY FELLED" (white) on the action bar with the challenge sound; an enchanted golden apple, 2 gold blocks, 3 gold ingots and experience orbs. The bar and the music stop. **f09_felled**
12. Summon another. Take the armour off, drop the totem, and let him kill you: "Sos was slain by Margit, the Fell Omen" (or "... using magic" for a dagger). Respawn, put the armour on.
13. Go back to him, then run 45 blocks away and wait 5 s: the bar goes. Come back: he is walking home at full health, phase 1, and wakes again when you come within 24 blocks.
14. Pillar up 5 blocks near him: he throws daggers. Come down, dig a 1 x 1 hole 2 deep and stand in it: his cane still reaches you. Climb out, build a ring of cobblestone 2 high, 3 blocks out from the hole all round, and stand in the hole again: he gets in, by a leap over the ring or, within about 5 s, by vanishing in golden mist and appearing inside the ring beside the hole, and keeps swinging.
15. Hurt him, then Save and Quit, and load: he stands where he was, at full health, and walks back to his spot to sleep. **f02_after_load**
16. `/difficulty peaceful`: he vanishes. `/difficulty normal`.
17. Stack two gold blocks and put a lantern on top: the blocks vanish into golden mist and Margit rises facing you. **f13_altar**
18. While he fights: a golden ring of sparks on the ground about 24 blocks around his spot. **f14_fog_ring**
19. **Optional, vanilla check:** with Margit asleep, quit, copy the world and open the copy in vanilla 26.3: it loads with no error, and `/data get entity @e[type=minecraft:marker,sort=nearest,limit=1]` near his spot shows `data.iw4l.boss`. Open the copy in our game again: Margit is there.

### Friend (two windows)
20. The friend sees him rise, every move, the golden weapons, the daggers and the fog ring, hears him, and has the bar and the music. **f12_friend_view**
21. `/summon iw4l:margit ~ ~ ~10` with both of you within 24 blocks: the bar empties more slowly than alone (x1.6).
22. The friend shoots and slashes him: the bar drops for both. If the friend hurts him most for 10 s, he turns on the friend.
23. He kills the friend: the death line names him; the friend respawns and the fight goes on.
24. The friend lands the last hit: both see GREAT ENEMY FELLED and the drops fall.
25. Both run 45 blocks away for 5 s: the fight resets and both bars go.

---

## Commit groups

| Order | Key | Segment | Commits | Brief |
|---|---|---|---|---|
| 1 | `er-core` | f1 (now) | 1. Elden Ring boss moves are plain data, starting with Margit's. 2. An Elden Ring boss picks, times and lands its moves like Margit. | F1 (commits 1-2) |
| 2 | `er-rules` | f1 | 3. An Elden Ring boss staggers, changes phase and resets like Margit. 4. A whole Margit fight plays through without a window. | F1 (commits 3-4) |
| 3 | `er-entity` | f2 (after D0) | 1. Margit, an Elden Ring boss, lives in Minecraft worlds and is saved like vanilla. 2. Minecraft weapons hurt Margit with no hurt cooldown, and his stance breaks. | F2 (commits 1-2) |
| 4 | `er-arena` | f2 | 3. An Elden Ring boss can be summoned in a Minecraft world. 4. Margit walks, turns and leaps on blocks and stays in his arena. | F2 (commits 3-4) |
| 5 | `er-look` | f2 | 1. Margit is drawn as a hooded old man with a cane. 2. Margit's moves and golden weapons are animated from his move table. 3. Golden mist, sparks and dust show Margit's spawn, phase change, slams and death. | F3 |
| 6 | `er-fight` | f2 | 1. Margit's cane, golden weapons and daggers hurt players. 2. Margit drops gold and experience when he dies, and every fighter sees GREAT ENEMY FELLED. | F4 |
| 7 | `er-guns` | f2 | 1. MW2 guns and explosives hurt Margit through his own resistances. | F5 |
| 8 | `er-bar-friend` | f2 | 1. Everyone near Margit sees his boss bar and hears the boss music. 2. A friend sees Margit's moves and fights him. | F6 |
| 9 | `er-altar` | f2 | 1. Margit rises from golden mist when a lantern is set on two gold blocks. 2. Margit's arena shows a golden fog ring while he fights. | F7 |

## Combined kit
One kit for the whole owner's test (27 stacks). The first join adds the class loadout (the M4A1, frags and flashes, and rifle rounds on hotbar slots 3 to 5 because the kit fills the backpack).

```
IW4L_MINECRAFT_KIT=iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:rifle_ammo*64,iw4l:barrett,iw4l:sniper_rounds*64,iw4l:rpg,iw4l:rocket*16,minecraft:netherite_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:trident,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:totem_of_undying,minecraft:gold_block*8,minecraft:lantern*4,minecraft:cobblestone*64,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:iron_shovel
```

---

## Appendix A. Where the wither is wired today
For anything D0's registry does not cover. Lines as of 6e0e0573; D0 will have folded many of them.

- **Vendored `third_party/minecraftoss/entities/src/world.rs`:** the module and re-export (`:41-42`); `EntityKey::Wither` (`:2092`, `id()` `:2131`); `MobHit::Wither` (`:2166`, `:2173`); the store (`:2504-2505`, default `:2652-2653`); body lookup (`:2941`); blasts (`:2995-3020`); player attacks (`:3134`); targets (`:3347`); projectile knockback (`:3642`); the mob list for AI (`:4131`); deaths and loot (`:4646-4653`); `entity_type` (`:4850`); bodies (`:4934`, `:4982`); damage states (`:5029`, `:5077`); voices (`:5185`); positions (`:5537`, `:5582`); `clone_where` (`:5623`, `:5658`); saved entries (`:5803`); Peaceful (`:5902`); `remove_where` (`:5941`); the tick dispatch (`:6646`); removing the dead (`:8076`); live ids (`:8118`, `:8171`); the shot list (`:8353`). `grep -n -i wither` finds them all.
- **Vendored `world/`:** `pushing.rs`, `combat.rs`, `hazards.rs`, `riding.rs`, `villagers.rs` and `witches.rs` each gained wither lines in d4f932de and 94bad055.
- **`crates/minecraft_terrain`:** `server_mobs.rs` (`SIMULATED_KINDS` `:433`, the wither at `:480`; `spawn_saved` `:728`; `mob_tags_where` `:1819`); `mob_tracking.rs` (`MobShow` `:65-97`, `tracking_range` `:285-306`, `shows` `:463`, `show_on` `:705`); `client_mobs.rs` (pose `:1291`, shadow `:1370`); `mesh.rs:544` (textures); `dormant.rs:14-19` (never far away); `lib.rs` (modules).
- **`crates/minecraft_survival`:** `entities.rs` (keys `:1987-2085`, bars `:186`, `:399`); `entity_view.rs` (effects `:151`, `:278`; boxes `:685`; meshes `:1099`); `death.rs` (names `:686`, `:876-890`); `session.rs` and `player.rs` (bars).
- **`crates/render_anim`:** `minecraft_wither.rs`, `minecraft_world.rs:1263`, `minecraft_players.rs:756`, `:941`, `minecraft_host.rs:644`.

## Appendix B. Not in this track
Design doc section 16: real game data and the baker, an Elden Ring style bottom bar, the gate arena, Margit's Shackle, a spawn egg, lock-on, a dodge roll, hit-stop, status buildup, the MW2 knife against mobs, several hurtboxes, punishing heals, and mobs fighting him. Write any of these that come up under "Left for later" in the log.

## Rules
One commit per item, with its tests. Clippy on the touched crates once per segment, `rustfmt --check` on new files' staged content (edition 2024 in `elden_boss`, 2021 in `minecraft_terrain` and the minecraftoss crates), no code comments of any kind, minimal diffs to existing files, cargo only through `scripts\cargo-slot.ps1` (gentle mode when the owner plays). No file from ELDEN RING or Minecraft is committed, and no code is copied from the reference. The log is `docs/round10/track-f-log.md`, with each commit's owner steps and kit line (at most 27 stacks, in its own code block).
