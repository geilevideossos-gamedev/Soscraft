# Track D0 log: the shared mob registry

Branch `track-d0`, worktree `C:\Dev\mc-wt\track-d0`, target dir `D:\iw4l-wt\track-d0`. Brief: `docs/round10/track-d0.md`. Mob list: `docs/round10/mobs.md`.

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Vendored files

Files under `third_party/minecraftoss` this track patched or added (round 8 counted 73 patched and 37 added).

- Patched (9): `entities/src/world.rs`, `entities/src/world/combat.rs`, `entities/src/world/witches.rs`, `entities/src/world/pushing.rs`, `entities/src/world/fireballs.rs`, `entities/src/world/riding.rs`, `entities/src/world/villagers.rs`, `entities/src/world/leashes.rs`, `entities/src/monster_ai.rs`.
- Added (11): `entities/src/world/families.rs`, `entities/src/world/families/{pests,trial,deep_dark,land,water,night,bosses}.rs`, `entities/src/world/families/pests/probe.rs` (test only), `entities/src/world/families/tests.rs` (test only).

## Protocol

No commit of this track has changed a `ToClient`/`ToServer` message or the codec yet. `minecraft_net::PROTOCOL` stays 6 (`docs/round10/decisions.md`).

## Group registry-core

### 1. Round 10 mob list with tracks, families and hit keys

After aa942451.

- **Changed:** new `docs/round10/mobs.md`, the D0-8 block of the brief verbatim: the hit-key bytes per group (0-35 taken, 36-39 D0, 40-43 D, 44-47 W, 48-63 M1, 64-79 M2, 80-95 M3, 96-99 F) and one section per round 10 track with its mobs, families, keys and status.
- **Tests:** none (docs only).
- **Decisions:** the file is copied line for line from the brief (`track-d0.md` lines 608-694 at aa942451), so later tracks find exactly the rows the brief describes.
- **Test steps:** none (docs only).

### 2. Minecraft mob families plug into the entity world through one registry

After 17faa07d.

- **Changed (vendored entities crate):**
  - New root `world/families.rs` (D0-2.2): `MobCore` (with `removed`, plus `ambient_interval`), `FamilyMob<S>`, `FamilyStore<S>`, `Hurt` (with `new`, `by`, `bypassing_armor`), `Despawn` (`of_category`), `MonsterSpec` (the D0-2.4 struct, so the `monster` hook exists now; group registry-hooks wires `MonsterKind::Registered` to it), `MobTraits`, `BaseTick`, the `Family` trait with every D0-2.2 hook and its default, `Registered`, the `family_group!` macro, the root `Families` (seven groups, delegating), the free lookups `kinds`, `knows`, `key_of_kind`, `has_key`, `enemy`, `is_vehicle`, `monster_spec`, `category`, `key_ranges`, and the generic `impl<S: Family> Exposed for FamilyMob<S>` (fire resistance honoured, every hazard through `S::hurt` with its damage type and `bypasses_armor`).
  - `EntityWorld` API in `families.rs`: `spawn_family`, `spawn_family_kind`, `family`, `family_mob_mut`, `family_core`, `family_core_mut`, `family_cores`, `family_traits`, `hurt_family`, and the tick helpers `family_base_tick` (previous position, tick count, `base_tick_fluid` with the family's `MovementSounds` from `step_sound` and the category, burning, suffocation, air and drowning, the death clock, the 100-tick `hurt_by` expiry, effects with heal and magic hurt, `float_from`, the ambient voice roll, `dying_travel` for a dead mob with AI, the idle clock for a mob without AI), `family_goal_step` (syncs `hurt_by`, health, eye height, tick count, griefing and daylight into the `MonsterAi` before `monster_ai_step`, and yaw, look and `hurt_by` back after it; lands fall damage through `S::hurt`), `family_after_move` (`blocks_act`, `push_entities`, which files the mob) and `family_strike` (a player hit or `mob_hits_mob`, then `swings += 1`).
  - Seven group files `world/families/{pests,trial,deep_dark,land,water,night,bosses}.rs`, each one `family_group!` line with its byte range (36-39, 40-43, 44-47, 48-63, 64-79, 80-95, 96-99). Only `pests` holds an entry: the test-only probe (key 39) in `world/families/pests/probe.rs`.
  - `EntityKey::Family(u8, u64)` and `MobHit::Family(u8, u64)`, `EntityWorld.families`, and one generic arm at the end of each D0-2.3 site: `blast_target`, `hurt_by_blast` (`Explosion`, the blaster's damage type, attacker and its kind), `blasted_by_player`, `arrow_targets`, `apply_arrow_hit` (`Projectile`, knockback through `knock_back`), `mob_candidates`, `mob_hits_mob` (`MobAttack`, the attacker blamed), `take_deaths`, `entity_type`, `bodies_mut`, `body_mut`, `damage_state`, `damage_state_mut`, `take_sounds`, `positions`, `burning`, `clone_where`, `census`, `check_despawn`, `remove_where`, the `tick_selected` dispatch, the death retain (`death_ticks < 20 && !removed`), the alive ids and the `order.retain` arm, `mob_on_ray_except`; `combat` `mob_body`, `knock_back` (power times `1 - knockback_resistance`), `hurt_by_player` (before the blazes' final `else`); `witches` `mob_effects_mut`, `mob_effects`, `heal_mob`, `hurt_mob_by_magic`, and the undead flag of `splash_instant`; `fireballs::fireball_damages_mob` (per-mob fire immunity); `pushing` `push_body`, `exposed_mut`, `pushes_on_local_player`; `riding::remove_mobs`; `villagers::seen_all`; `leashes` `body_of`, `leash_yaw`, `leash_no_ai`, `leash_panicking`, `mob_baby`, `can_be_leashed`, `leash_walk` (its final `else`); `monster_ai::is_enemy` (`|| families::enemy(kind)`).
- **Changed (game code):** `crates/minecraft_survival/src/entities.rs` `encode` gains `MobHit::Family(key, _) => key`, which its exhaustive match needs to compile. `decode` and the hit-key test come with group registry-draw (D0-5.3).
- **Tests added** (`world::families::tests`, on the probe): `every_family_key_is_unique_and_inside_its_group_range`, `a_registered_mob_is_seen_by_every_world_walk`, `a_registered_mob_takes_melee_arrows_blasts_magic_and_fire`, `a_registered_mob_dies_with_its_table_experience_and_voices`, `a_registered_mob_despawns_by_its_rule_leaves_with_its_chunk_and_is_copied_where_kept`, `a_removed_registered_mob_leaves_without_a_death`, `a_lead_pulls_a_leashable_registered_mob`, `a_fireball_spares_a_fire_immune_registered_mob`, `two_registered_mobs_push_apart_and_cram`, `a_hurt_registered_monster_turns_on_its_attacker`, `an_iron_golem_hunts_an_enemy_registered_mob`. The whole `minecraftoss-entities` lib suite stays green unchanged.
- **javap this round:** `MobCategory` (despawn distance 128 for every category but `water_ambient`, 64), `Mob.checkDespawn` (peaceful first, then persistence, then far and idle rolls, as `check_despawn` does), `Animal.removeWhenFarAway` (false) and `Animal.getAmbientSoundInterval` (120).
- **Decisions:**
  - `Despawn::of_category` gives monster (128, removable, not in peaceful), creature and misc (128, kept, peaceful ok: `Animal.removeWhenFarAway` is false), water_ambient (64, removable), every other category (128, removable). `kept` is added to the persistence flag (vanilla `requiresCustomPersistence`).
  - `MobCore.ambient_interval` (default 80, `Mob.getAmbientSoundInterval`) is a field, not a hook: a family whose class overrides it (an `Animal`: 120) sets it in `spawn`.
  - `breathes_water` defaults to the kind being `#undead` instead of false: vanilla `#can_breathe_under_water` holds `#undead`, and the old undead families do not drown either. A family that breathes water for another reason overrides it. Water breathing as an effect counts too.
  - The family helpers and the generated group methods are `pub`, not `pub(crate)`: no non-test family exists yet, so `pub(crate)` ones would warn as dead code. Only the two that name the crate-private `Exposed` trait stay `pub(crate)` (and are used by `pushing.rs`).
  - `family_goal_step(id, level, players, movement_speed, ai)` takes the family's base `MOVEMENT_SPEED` (effects are applied inside) and an accessor `fn(&mut S) -> &mut MonsterAi` for the AI kept in the state, because the AI and the core are two fields of one `FamilyMob` that the helper borrows apart. It also lands fall damage (`minecraft:fall`, past armour) through `S::hurt`, as the spider's `tick_ai` does.
  - Registered mobs are blamed vanilla-style: an arrow or fireball from a player (`PLAYER_TARGET + id`) is `Target::Player`, not `Target::Mob` as the old families take it; a creeper's blast blames the creeper (`hurtServer` remembers the source's entity).
  - The arrow arm knocks back through `knock_back` (power `0.4 * (1 - knockback_resistance)`, nothing and no random draw at 1), because `projectile_knockback` normalises the direction and so cannot scale it. `mob_hits_mob` counts a registered mob with resistance 1 as resisting, and an iron golem's lift is scaled by `1 - knockback_resistance` on a new line (times 1.0 for every old family, so no change there).
  - `splash_instant` also flips healing and harming for registered undead mobs (the old line only knew zombies and skeletons; it stays as it was for them).
  - `villagers::seen_all` and `leashes::leash_yaw` became `pub(super)` (one word each) so the registry's tests can reach them; nothing else changes.
  - The test probe keeps a `MonsterKind::Spider` goal set in its state to exercise `family_goal_step` now; group registry-hooks moves it to `MonsterKind::Registered` with its own `MonsterSpec`.
  - `MobHit` is not serialised (the wire carries the `encode`d `u64`), so this commit changes no protocol message.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (409 passed), `test -p minecraft_survival --locked` (757 + 1 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new files pass `rustfmt --check --edition 2021`.
- **Test steps** (nothing new is visible: this commit only adds the registry; the steps check that the old mobs behave as before). Release build, survival, Normal, by day, flat ground in the Overworld. Name each screenshot by its label, `D0-2-1.png`.
  1. Use the zombie, creeper, cow, villager and iron golem eggs about 8 blocks apart. The zombie comes at you, the creeper hisses, the cow wanders, the villager walks about and the golem turns on the zombie. **Screenshot D0-2-1.**
  2. Hit the cow with the sword: it flashes red, is knocked back and runs. Shoot the zombie with the bow and with the M4A1: it flashes and dies and leaves experience.
  3. Light TNT next to a cow and a villager: both flash, are thrown and die.
  4. Throw the harming splash potion at a zombie: it heals (no damage). Throw it at a cow: it is hurt.
  5. Put a lead on a cow and walk 8 blocks: the cow follows. Save and quit with a zombie and the cow alive, load: both are there and move. **Screenshot D0-2-5.**

```
IW4L_MINECRAFT_KIT=minecraft:zombie_spawn_egg*4,minecraft:creeper_spawn_egg*2,minecraft:cow_spawn_egg*4,minecraft:villager_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:tnt*8,minecraft:flint_and_steel,minecraft:splash_potion@harming,minecraft:lead*2,minecraft:cooked_beef*32
```
## Group registry-hooks

### 1. Registered Minecraft mobs use the shared monster goals and targets

After 9eecb2ff.

- **Changed (vendored entities crate):**
  - `monster_ai.rs` (D0-2.4): `MonsterKind::Registered(&'static str)` at the end of the enum. `goal_names` and `target_goal_names` answer it from `families::monster_spec(kind)` (an empty list without a spec), so `running_goals`, `running_targets` and `retain_goals` work for registered mobs. `MonsterKind::Registered(_)` joins the fall-sacrifice arm of `profile`. `of_kind` looks the spec up once (`let spec`), takes the follow range from it (16 without one), keeps the monster defaults (fire maluses cleared, `WalkTarget::Monster`), then sets `can_float` from the spec's goal names and runs `spec.walk`; the goals tuple arm builds `(spec.goals)()`, `(spec.targets)()` and `spec.eye_height`.
  - `MonsterGoalContext.extra: GoalExtra` after `shulker`, `extra: Default::default()` after `shulker: ShulkerState::default()`.
  - `Prey::Kinds(&'static [&'static str])` at the end of `Prey`, and its arm at the end of `NearestTargetGoal::can_start`'s match: the `ctx.mobs` of those kinds. `enderman_targets` uses it for endermites.
  - `world/families.rs`: `ExtraState` (`clone_box`, `as_any`, `as_any_mut`, blanket impl for `T: Any + Clone + Send`) and `GoalExtra` (`get::<T>`, `get_mut::<T>`, `set(T)`, `Clone` through `clone_box`, `Default` empty).
  - The test probe now runs on `MonsterKind::Registered(KIND)` with its own `MonsterSpec` (follow range 24, eye height 0.7, `walk` sets a one-block step): goals `FloatGoal`, `MeleeAttackGoal` (hits), `ProbeGoal` (test only: starts while the family put `allowed` into `GoalExtra`, counts its ticks there), `LookAtPlayerGoal`, `RandomLookGoal`; targets `HurtByTargetGoal` and `NearestAttackableTargetGoal` over `Prey::Kinds(&[ANIMAL])`. Its tick writes `ProbeExtra` before `family_goal_step` and reads it back after.
- **Tests added** (`world::families::tests`): `a_registered_monster_runs_its_goals_and_strikes_a_player` (the spec's names through `goal_names`/`target_goal_names`, an unknown registered kind has none, `of_kind` takes follow range, eye height, `can_float` and the step from the spec, `running_goals` shows `MeleeAttackGoal` and `ProbeGoal`, `running_targets` `HurtByTargetGoal`, the bite is a `PlayerHitKind::Melee` from the probe at its damage with `core.swings` counting it, `GoalExtra` read and written by the goal, `retain_goals(&["ProbeGoal"])` keeps only that goal and its targets), `prey_kinds_targets_the_named_kinds_only` (no target with a zombie and another probe near, the probe animal becomes the target and is bitten). `a_hurt_registered_monster_turns_on_its_attacker` now runs on the registered goals instead of the spider's.
- **Decisions:**
  - `can_float` follows the spec: true when its goal names hold `FloatGoal` (vanilla `FloatGoal`'s constructor calls `getNavigation().setCanFloat(true)`, which is why the old list names exactly the kinds with that goal). `spec.walk` runs after it and may still change it.
  - A registered kind without a spec gets empty selectors, follow range 16 and eye height 0.85 of the body (the creeper's rule), so `of_kind` never panics on a family that has no goals.
  - `GoalExtra` lives in the root file; `monster_ai.rs` only gains the field. `ExtraState` asks `Send` (not `Sync`): `MonsterAi` is `Send` already and boxed goals are not `Sync` either.
  - The enderman's endermite target goal switches from `Prey::Absent` to `Prey::Kinds(&["minecraft:endermite"])` here (D0-2.4 names it), not in group `silverfish-live`: until endermites live there is no candidate of that kind, so the goal rolls its interval and finds nothing exactly as before, and the variant is used outside tests (without a user `Prey::Kinds` warns as never constructed).
  - `HurtByTargetGoal` starts only when the hurt timestamp differs from its last one, which begins at 0 (vanilla does the same), so a mob hurt on the tick it spawned does not turn on its attacker. The new goal test ticks once before the attacks for that reason; no code change.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (411 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new files pass `rustfmt --check --edition 2021`.
- **Test steps** (registered mobs have no family yet, so nothing new is visible; the steps check that monsters on the shared goals behave as before). Release build, survival, Normal, at night or underground, flat ground in the Overworld. Name each screenshot by its label, `D0-3-1.png`.
  1. Use the zombie, skeleton, spider and creeper eggs 10 blocks away. The zombie walks at you and hits, the skeleton shoots, the spider runs and leaps at you, the creeper hisses next to you. **Screenshot D0-3-1.**
  2. Use an enderman egg and look at its head: it shakes, then comes for you. Hit it once: it teleports.
  3. Use an iron golem egg next to a new zombie: the golem walks to the zombie and hits it into the air.
  4. Use a wolf egg, tame it with bones, then hit a zombie: the wolf attacks the zombie. **Screenshot D0-3-4.**
  5. Pour water next to a spider and a creeper: both float in it instead of sinking.

```
IW4L_MINECRAFT_KIT=minecraft:zombie_spawn_egg*4,minecraft:skeleton_spawn_egg*2,minecraft:spider_spawn_egg*2,minecraft:creeper_spawn_egg*2,minecraft:enderman_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:wolf_spawn_egg*2,minecraft:bone*16,minecraft:water_bucket,minecraft:iron_sword,minecraft:shield,minecraft:cooked_beef*32,minecraft:torch*64
```
