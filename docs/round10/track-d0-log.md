# Track D0 log: the shared mob registry

Branch `track-d0`, worktree `C:\Dev\mc-wt\track-d0`, target dir `D:\iw4l-wt\track-d0`. Brief: `docs/round10/track-d0.md`. Mob list: `docs/round10/mobs.md`.

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Vendored files

Files under `third_party/minecraftoss` this track patched or added (round 8 counted 73 patched and 37 added).

- Patched (15): `entities/src/world.rs`, `entities/src/world/combat.rs`, `entities/src/world/witches.rs`, `entities/src/world/pushing.rs`, `entities/src/world/fireballs.rs`, `entities/src/world/riding.rs`, `entities/src/world/villagers.rs`, `entities/src/world/leashes.rs`, `entities/src/monster_ai.rs`, `entities/src/bin/check_zombie_player_hits.rs`, `entities/src/world/emissions.rs`, `entities/src/walk_path.rs`, `entities/src/stroll.rs`, `world/src/natural_spawner.rs`, `world/src/natural_spawner/spawning.rs`.
- Added (23): `entities/src/world/families.rs`, `entities/src/world/families/{pests,trial,deep_dark,land,water,night,bosses}.rs`, `entities/src/world/families/goals.rs`, `entities/src/world/families/pests/{silverfish,endermites}.rs`, `entities/src/world/families/pests/probe.rs` (test only), `entities/src/world/families/tests.rs` (test only), `world/src/natural_spawner/families.rs`, `world/src/natural_spawner/families/{pests,trial,deep_dark,land,water,night,bosses}.rs`, `world/src/natural_spawner/families/tests.rs` (test only).

## Protocol

- **"Friends see registered Minecraft mobs"** (group registry-client, commit 2, after 757bfed8) adds `pub family: Option<FamilyShow>` at the end of `MobShow` (`crates/minecraft_terrain/src/mob_tracking.rs`). `FamilyShow` is `{ flags: u32, values: [i32; 8] }` (`crates/minecraft_terrain/src/families.rs`).
  - `MobShow` is part of every `ServerMob` that the state lane carries (`StateFrame.mobs`, postcard). A host and a friend on builds from before and after this commit misread each other's mob states.
  - Postcard writes one byte for `None`, so every other mob costs one byte more per tick.
  - No `ToClient`/`ToServer` variant and no codec function changed.
- `minecraft_net::PROTOCOL` stays 6 on this track (`docs/round10/decisions.md`, "PROTOCOL is bumped only at release time"). The next release build bumps it once, because of this commit.

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

### 2. Registered Minecraft mobs can carry riders

After 37dcfc8d.

- **Changed (vendored entities crate, `world/riding.rs`, D0-2.5):** one generic arm per mount function, each after the existing families:
  - `carries_riders`: `|| self.families.traits(vehicle).is_some_and(|t| t.seats > 0)` on its one line.
  - `passenger_point(vehicle)`: a registered vehicle answers `Family::passenger_point(mob, 0)` (the first seat), before the horse lookup. `seat_point(rider)` asks the family with the rider's index in `passengers_of(vehicle)` (vanilla `getPassengerAttachmentPoint` takes `getPassengers().indexOf(passenger)`), falling back to `passenger_point(vehicle)` for every other vehicle.
  - `vehicle_body`: the core's body, after the ravager.
  - `start_riding`: `passengers_of(vehicle).len() < seats && force_riding(..)`, after the horse and ravager line.
  - `stop_riding`: the spot from `Family::dismount` (`family_spot`, a `None if family_spot.is_some()` arm after the ravager's).
  - `controlling_mob` and `controlling_player`: `Family::controller(mob, passengers_of(vehicle))`, a mob or a player rider each, after the horse block.
  - `order_vehicle`: a registered vehicle passes every `Navigate` and `NavigateToEntity` order to `Family::rider_order` (target, speed), after the horse block.
  - `jumpable_by`: a registered vehicle whose `rider_jumps` holds answers its controlling player, before the horse lookup; `tick_ride_jumps` hands the released power to `Family::rider_jump` before it looks in the cow store.
  - `mob_vehicle_attachment`: its `_ => 0.0` arm becomes `_ => self.families.traits(id).map_or(0.0, |t| t.vehicle_attachment)`.
- **Changed (root file):** `Family::passenger_point`'s default is the offset `(0, height, 0)` above the vehicle's feet, as every vehicle's `passenger_point` in `riding.rs` is an offset (it was a world position in 9eecb2ff, which nothing read yet). `dismount` keeps its default, the world spot on top of the vehicle.
- **Changed (probe):** `MOUNT` true; per-mob `seats`, `saddled` (a player first passenger steers), `mob_steers` (a mob first passenger steers), `jumps` (jump bar), the released powers and the orders it was given, and a rider-side `vehicle_attachment`; seats sit 0.5 apart along -z.
- **Tests added** (`world::families::tests`): `a_registered_mount_seats_a_rider_and_lets_it_off` (`is_vehicle`, `carries_riders` only with seats, two seats and no third rider, the first rider sits at seat 0 less its own attachment and the player at seat 1 less 0.6, `passenger_point`, `stop_riding` sets the rider on top of the mount and frees the seat), `a_player_steers_and_jumps_a_registered_mount` (`controlling_player` only once saddled, no jump bar until `rider_jumps`, `ride_jump_scale` 0.4 after five held ticks and power 40 handed to `rider_jump` on release, a zombie rider controls it through `controlling_mob` and its new navigation target reaches `rider_order` through `end_ride_tick` and `order_vehicle`).
- **Decisions:**
  - Survival's `ride_seat(player, vehicle)` asks `passenger_point(vehicle)` without a seat, so a player on a registered mount is drawn at seat 0 whatever seat it holds. The server-side seat (`seat`, `seat_offset`) is right for every seat. Only the happy ghast (M3, four seats) needs the drawn seat to follow the index; M3 adds a seat-aware call there.
  - The terrain side of mounts (`VEHICLES`/`carries` with `families::is_vehicle`, D0-5.1) belongs to group `registry-client`.
  - `rider_order` gets `NavigateToEntity` orders too (with the entity's position as the target): vanilla passes the controlling mob's navigation target either way.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (413 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new files pass `rustfmt --check --edition 2021`.
- **Test steps** (no registered mount exists yet; the steps check the old mounts and jockeys). Release build, survival, Normal, flat ground in the Overworld, and in the Nether for step 3. Name each screenshot by its label, `D0-4-1.png`.
  1. Use a horse egg, tame it by riding it until hearts show, saddle it and ride: it steers with the mouse, and holding the jump key fills the jump bar; releasing it jumps. **Screenshot D0-4-1** with the jump bar half full.
  2. Get off with the sneak key: you land beside the horse.
  3. In the Nether, use a strider egg, saddle it and ride it with the warped fungus on a stick: it walks where you look. Get off on the lava's shore.
  4. `/summon minecraft:ravager ~ ~ ~5 {Passengers:[{id:"minecraft:pillager"}]}`: the pillager sits on the ravager's back and both come for you. **Screenshot D0-4-4.**
  5. `/summon minecraft:chicken ~ ~ ~5 {Passengers:[{id:"minecraft:zombie",IsBaby:1b}]}`: the baby zombie rides the chicken.

```
IW4L_MINECRAFT_KIT=minecraft:horse_spawn_egg*2,minecraft:saddle*3,minecraft:strider_spawn_egg*2,minecraft:warped_fungus_on_a_stick,minecraft:golden_carrot*16,minecraft:iron_sword,minecraft:shield,minecraft:cooked_beef*32
```

### 3. Minecraft mobs can hurt players with their own damage types

After 7f0cd730.

- **Changed (vendored entities crate):** `PlayerHitKind::Typed { damage_type, message, from, push, bypasses_armor, bypasses_shield, disable_shield_seconds, effect: Option<(EffectKind, u32, u8)> }` at the end of the enum (D0-2.6). `bin/check_zombie_player_hits.rs` takes it as a hit from `from`, beside the melee, magic and firework arm.
- **Changed (game code, `minecraft_survival`):**
  - `death.rs`: `MobAttack::Typed { damage_type, message, bypasses_armor }` at the end of the enum. Its `damage_type` and `message_id` are its own strings, `bypasses_armor` reads the flag, and it has no stand-in attacker when the mob is gone (`DamageCause::mob` gives `Generic` without a source, as for melee).
  - `entities.rs` `mob_hit`: `Typed` hits come from `from` (the hurt direction and the shield's angle), `toward` is dropped when `bypasses_shield` holds (one new line after the magic rule, so no shield blocks it), the attack is `MobAttack::Typed` with the hit's strings and armour flag, the effect is the hit's own, and the difficulty scaling is that of any hit with a source. `melee_push` gives `Typed` its `push` as the throw and its shield cooldown.
- **Tests added:** `world::families::tests::a_typed_hit_reaches_the_player` (every field of a `Typed` strike from `family_strike` reaches the player hit list with its damage and source; the same strike on a mob goes through `mob_hits_mob`; `core.swings` counts both), `entities::tests::a_typed_hit_skips_armour_and_shields_when_told_and_gives_its_effect` (a sonic-boom-like hit: 6, 10 and 15 by difficulty and none in peaceful, no shield angle, `minecraft:sonic_boom` past armour, its push as the throw; a sting-like hit: a raised shield facing it blocks it, armour counts, the poison effect and a 100-tick shield cooldown carried, `Generic` without a source), `death::tests::a_typed_attack_reads_its_own_death_line` (the 26.3 `death.attack.sonic_boom` line, "%1$s was obliterated by a sonically-charged shriek", killer Warden; the sting's own type and message id).
- **Decisions:**
  - A `Typed` hit without a living source is `DamageCause::Generic`, as the other mob attacks are; every planned typed attack (sonic boom, sting, pufferfish) has a mob behind it.
  - `bypasses_shield` works by dropping the shield angle (`toward`), the way evoker fangs already pass shields; the flag mirrors vanilla `#bypasses_shield`, which holds `#bypasses_armor` and so `minecraft:sonic_boom`.
  - The push goes through `MeleePush` (thrown when hurt or shielded, scaled by knockback resistance), so a family passes the vanilla push already scaled for a resistance of 0.
  - `PlayerHitKind` and `MobAttack` are not serialised (survival applies hits on the host), so this commit changes no protocol message.
- **Runs:** `test -p minecraftoss-entities -p minecraft_survival --locked --lib --bins` (entities 414 passed, survival 759 passed, every `check_*` bin builds), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new files pass `rustfmt --check --edition 2021`.
- **Test steps** (no registered mob attacks with a typed hit yet; the steps check that the old attacks on the player still land, are blocked and read their death lines). Release build, survival, Normal, at night, flat ground in the Overworld. Name each screenshot by its label, `D0-5-1.png`.
  1. Use a zombie egg 6 blocks away, raise the shield as it comes: its hits are blocked with the shield sound. Lower the shield: it hurts you and pushes you back.
  2. Use a cave spider egg and let it bite you: you are poisoned (green hearts).
  3. Use an evoker egg: its fangs bite through the raised shield. **Screenshot D0-5-3.**
  4. Let a skeleton shoot you dead (`/summon minecraft:skeleton ~ ~ ~8`): the death screen reads "Player was shot by Skeleton". **Screenshot D0-5-4.**

```
IW4L_MINECRAFT_KIT=minecraft:zombie_spawn_egg*2,minecraft:cave_spider_spawn_egg*2,minecraft:evoker_spawn_egg*2,minecraft:skeleton_spawn_egg*2,minecraft:shield,minecraft:iron_sword,minecraft:cooked_beef*32,minecraft:milk_bucket
```

## Group registry-spawning

### 1. Minecraft mob families bring their own spawn rules

After 724aca35.

- **Changed (vendored world crate, D0-4):**
  - New root `natural_spawner/families.rs`: `FamilySpawn` (`kind`, `info: TypeInfo`, `category`, `follow_range`, `baby_chance: Option<f32>`, `rule: Option<FamilyRule>`, `finalize: Option<FamilyFinalize>`), the two fn types, `rows()` (the seven groups in byte order, then the test rows under `#[cfg(test)]` on the statement), `row(kind)` (with or without `minecraft:`), `rule` (the row's rule with the spawner flag, true for a row without one), `finalize_creature` and `finalize_monster`.
  - Seven group files `natural_spawner/families/{pests,trial,deep_dark,land,water,night,bosses}.rs`, each `pub(super) const ROWS: &[FamilySpawn]`. Only `pests` has rows: `minecraft:silverfish` and `minecraft:endermite`, 0.4 x 0.3, `OnGround`, `MotionBlockingNoLeaves`, not fire immune, `Rules::Family { creature: false }`, monster walk, removed far away, cluster 4, `MobCategory::Monster`, follow range 16, no babies, no finalize, and the rule `any_light_away_from_players` (javap `checkSilverfishSpawnRules` and `checkEndermiteSpawnRules`: `checkAnyLightMonsterSpawnRules`, true for a spawner, else no player within 5 blocks of the block centre).
  - `natural_spawner.rs`: `mod families;`, `Rules::Family { creature: bool }` (after `Patrolling`) and its `creature()` case, the row first in `type_info`, the row first in `finalize_spawn` (it returns before every built-in arm and the catch-all), `Group::Family { data, size }` and its `ageable` arm.
  - `spawning.rs`: the row first in `MobCategory::of_type` and `follow_range_base`; `Rules::Family` arms in `natural_spawn_rules` (spawner false) and `spawner_may_spawn` (spawner true, before `_ =>`); in `finalize_monster`, right after the follow-range line, any row runs `families::finalize_monster` and returns before the catch-all; `MonsterGroup::Family { data, size }`; `dark_enough`, `mob_spawn_rules`, `difficulty_at`, `populate_armor`, `roll_enchantments` and `MonsterGroup` are `pub(super)`.
  - `summon` and the natural spawn need no change: they route by `type_info(kind).rules.creature()`, which is the row's.
- **What changes for the existing game:** silverfish and endermites now have spawn rules, so a silverfish spawner spawns by them (any light for the rule, a monster's walk target, room, peaceful) instead of anywhere (the kind had no `type_info`, so `spawner_may_spawn` said yes), and a summoned or hatched silverfish or endermite gets its follow range and `Mob.finalizeSpawn` without an unsupported note. Endermites now have the monster category (mob cap, despawning, the terrain's dormant-monster check). Neither kind is simulated yet (group `registry-client`), so nothing new is drawn.
- **Tests added** (`natural_spawner::families::tests`, with three `#[cfg(test)]` rows: `minecraft:mooshroom` overriding its built-in row as a creature with a numbering finalize and babies at chance 1, `minecraft:giant` as a monster with the same finalize and follow range 30, `minecraft:trader_llama` as a creature without finalize or babies):
  - `a_family_row_comes_before_the_built_in_tables` (both pests: size, placement, heightmap, rules, walk, removal, cluster, category, `type_width`, the row fields; every row kind unique; a summoned silverfish and endermite get follow range 16, the giant 30).
  - `a_row_overrides_a_built_in_kind` (the mooshroom's size, rules, walk and category come from the row; the giant gets a category it never had; a summoned mooshroom runs the row's finalize without a context and logs nothing).
  - `silverfish_spawn_by_any_light_from_spawners_and_never_near_a_player` (the rule passes in full daylight, refuses a player 4.9 blocks from the block centre and allows one at 5, a spawner ignores players and the block below, bedrock refuses a natural spawn; `spawner_may_spawn` says yes in the dark beside a player, no in peaceful and no in full light (the monster walk target); natural spawning with only silverfish in the biome's monster list spawns them with no unsupported note, and none in peaceful).
  - `a_family_finalize_runs_for_summons_and_natural_spawns_and_shares_group_data` (a summoned giant's finalize gets the context and its difficulty, a summoned mooshroom's gets none; natural giant and mooshroom groups number their members 1, 2, 3 through the shared group data, the monster path with the difficulty and the creature path without, and every member after the first is a baby).
  - `a_row_without_finalize_or_babies_is_supported_and_draws_nothing` (silverfish and endermite summons log nothing; two trader llamas in one group draw exactly what `finalize_mob` draws, get no baby, keep `Group::None` and skip the built-in llama arm; two mooshrooms draw one baby roll more).
- **javap this round:** `Silverfish.checkSilverfishSpawnRules` and `Endermite.checkEndermiteSpawnRules` (as above, `getNearestPlayer(x + 0.5, y + 0.5, z + 0.5, 5.0, true)`), `Monster.checkAnyLightMonsterSpawnRules` (only `checkMobSpawnRules` in 26.3), `Mob.checkMobSpawnRules` (true for a spawner, else `isValidSpawn` below), `EntityGetter.getNearestPlayer(DDDDZ)` (`true` picks `NO_CREATIVE_OR_SPECTATOR`; the distance must be below 5).
- **Decisions:**
  - `FamilyRule` also takes the kind (`&str`, before `&TypeInfo`), unlike the brief's type: vanilla rules get their `EntityType`, and `mob_spawn_rules` (`isValidSpawn`) reads it for leaves, ice and magma, so two kinds can share one rule.
  - The pests rule has no peaceful test: in 26.3 `checkAnyLightMonsterSpawnRules` is only `checkMobSpawnRules` (the brief said "not peaceful"). Peaceful is still refused by the existing `getMobForSpawn` line in `spawn_category_for_position` and the peaceful line of `spawner_may_spawn`, which end the spawn as vanilla does.
  - `SpawnContext.players` holds every non-spectator, while vanilla's 5-block check also skips creative players, so a creative player within 5 blocks also stops a natural silverfish or endermite. Natural spawning already refuses spots within 24 blocks of any player, so this never decides a natural spawn.
  - The monster path honours `baby_chance` too (the brief named only the finalize and `finalize_mob` there): finalize, then the `AgeableMob` baby draw on the group's size, then `finalize_mob`, the order of vanilla's `super.finalizeSpawn` chain. That is what the `size` of `MonsterGroup::Family` is for.
  - Group data: the first member's `Some(data)` makes the group `Family { data, size }` (keeping the size counted so far); a family whose finalize keeps no data uses `Ageable` (with babies) or stays `None` (without), so a `baby_chance: None` row never draws a baby roll.
  - `MonsterGroup` became `pub(super)` (one word) so the root file carries the monster group data; the brief listed only the five helpers.
  - The test rows are the mooshroom, the giant and the trader llama, not the goat the brief suggested: M1 adds a real goat row to `land.rs`, which would shadow a goat test row (or be shadowed by it) and break one side's tests at that merge. None of the three belongs to a round 10 track, and the test rows exist only in this crate's own test build.
- **Left for later:**
  - Chunk-generation spawns (`spawn_for_chunk_generation`, `check_spawn_rules`) have no `SpawnContext`, so a family creature row there still uses the animal rules (`#animals_spawnable_on` below, light above 8) instead of its own rule. The first track with a creature that needs its own rule at world generation (M1's goats, M2's frogs) adds that hook to the root file.
  - `check_spawn_obstruction` refuses any liquid in the box except for striders and drowned, while vanilla `WaterAnimal.checkSpawnObstruction` only asks `isUnobstructed`. M2 adds that exception for its water rows.
- **Runs:** `test -p minecraftoss-world --locked` (107 passed, the five new tests with the 26.3 data), `clippy -p minecraftoss-world --locked --lib --tests` (no warnings), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new files pass `rustfmt --check --edition 2024` (the world crate's edition) on their staged content.
- **Test steps** (silverfish and endermites are not simulated yet, so the steps check that natural spawning, spawners and summons of the old mobs work as before). Release build, survival, Normal, flat open ground in the Overworld. Name each screenshot by its label, `D0-6-1.png`.
  1. `/time set night` and stand still for a minute: zombies, skeletons, creepers and spiders appear in the dark around you and come at you. **Screenshot D0-6-1.**
  2. `/time set day`, then walk or fly 300 blocks into land you have not seen: cows, sheep, pigs and chickens stand about in groups, some of them babies. **Screenshot D0-6-2.**
  3. Build a 7 x 7 x 4 room of stone bricks with no light inside, place the spawner in the middle and use a zombie egg on it: zombies come out within half a minute. Put four torches in the room: no more zombies come. Break the spawner.
  4. `/summon minecraft:zombie ~ ~ ~4` and `/summon minecraft:skeleton ~ ~ ~4`: both come at you, and the skeleton holds its bow.
  5. `/difficulty peaceful`: the monsters vanish and no new ones come. `/difficulty normal`.

```
IW4L_MINECRAFT_KIT=minecraft:spawner*2,minecraft:zombie_spawn_egg*4,minecraft:stone_bricks*64,minecraft:stone_bricks*64,minecraft:torch*16,minecraft:iron_sword,minecraft:shield,minecraft:cooked_beef*32
```

## Group silverfish-live

### 1. Minecraft silverfish and endermites live in the entity world

After 4fdf76e1.

- **Changed (vendored entities crate, D0-7.1):**
  - New `world/families/pests/silverfish.rs` (key 36) and `world/families/pests/endermites.rs` (key 37), registered in `world/families/pests.rs` next to the test probe. Both run on `MonsterKind::Registered` with their own `MonsterSpec` (follow range 16, eye height 0.13) and the registry helpers: `family_base_tick`, `family_goal_step` (movement speed 0.25), `family_strike` (a `PlayerHitKind::Melee` bite, or `mob_hits_mob`), the new `family_alert` and `family_after_move`.
  - Silverfish: 0.4 x 0.3, 8 health, bites for 1. Goals 1 `FloatGoal`, 1 `ClimbOnTopOfPowderSnowGoal`, 3 `SilverfishWakeUpFriendsGoal`, 4 `MeleeAttackGoal(1.0, false)`, 5 `SilverfishMergeWithStoneGoal`; targets 1 `HurtByTargetGoal` with alert, 2 `NearestAttackableTargetGoal(Player, true)`. Its `hurt` starts the wake-friends clock (`SilverfishExtra` in `GoalExtra`) for a hit with an attacker or of a `#always_triggers_silverfish` type (`minecraft:magic`), before the damage. The wake goal runs while the clock is above 0 and counts it down. Its strolls prefer spots above the seven infested host blocks.
  - Endermite: 0.4 x 0.3, 8 health, bites for 2, 3 experience. Goals 1 `FloatGoal`, 1 `ClimbOnTopOfPowderSnowGoal`, 2 `MeleeAttackGoal(1.0, false)`, 3 `WaterAvoidingRandomStrollGoal(1.0)`, 7 `LookAtPlayerGoal(Player, 8)`, 8 `RandomLookAroundGoal`; the same two targets. Its `life` (vanilla `Lifetime`, saved by group registry-client) counts up each tick unless it is persistent; at 2400 it sets `core.removed` (vanilla `discard`: no death, loot or experience).
  - Both answer `enemy` (iron golems hunt them, `is_enemy` is true), take poison, drown and burn, and are hunted by endermen through the enderman's `Prey::Kinds(&["minecraft:endermite"])` goal from registry-hooks (endermites only).
  - New root-owned `world/families/goals.rs` (`pub mod goals;` in `families.rs`): `ClimbOnTopOfPowderSnowGoal`, `RandomStrollGoal { speed, interval }` (`DefaultRandomPos.getPos(10, 7)`), `reduced_tick_delay` and `in_powder_snow`.
  - Root `families.rs`: the `Family::emits_sounds(kind)` hook (default true), `movement_sounds` honouring it, `EntityWorld::family_alert::<S>(id, attacker, ai)` (vanilla `HurtByTargetGoal.alertOthers` over the family's store), and `family_goal_step` now copies `core.yaw` into the AI before the step.
  - `world/emissions.rs`: `MovementSounds.emits_sounds` (true from `monster` and `creature`) and `without_sounds()`; `play_movement` drops the step, chime and swim of a mob that emits no sounds (the splash on entering water still plays).
  - `walk_path.rs` `WalkTarget::MonsterOver(&[&str])` (at the end of the enum) and its arm in `stroll.rs` `walk_target_value`: 10 above one of the blocks, else the monster's darkness value.
- **Tests added:** `world::families::pests::silverfish::tests::a_silverfish_is_a_small_eight_health_monster_that_bites_for_one` (size, eye height, health, key 36, `entity_type`, census, `mob_candidates`, `is_enemy`, the traits and voice, goal and target names, follow range, `can_float`, the walk target; it walks up to a player and bites for 1 as a melee hit with one swing, with no step or swim sound on the way; killed, its loot table is `entities/silverfish` and it drops nothing), `a_hurt_silverfish_alerts_the_others` (a fall starts no clock, magic does, a player's hit sets it to 10; the two silverfish within its follow range turn on the player, the one 40 blocks away and an endermite beside it do not; the clock counts down to 0), `world::families::pests::endermites::tests::an_endermite_is_a_small_eight_health_monster_that_bites_for_two` (the same for key 37 with a bite of 2, and its life counts), `an_endermite_leaves_after_2400_ticks_unless_persistent` (a fresh no-AI mite is there with life 2399 and gone at 2400; a mite with AI goes at 2400 too; a persistent one keeps life 0 and stays; a persistent one loaded with life 2400 goes on its next tick; none leaves a death), `an_enderman_hunts_an_endermite` (the enderman targets the endermite, not the silverfish nearer to it, and hits it), `silverfish_and_endermites_give_five_and_three_experience` (5 and 3 when a player kills them, 0 for an endermite killed by lava, both tables `Some`).
- **javap this round (26.3):**
  - `Silverfish`: `createAttributes` health 8, movement speed 0.25, attack 1; `registerGoals` as above (`HurtByTargetGoal().setAlertOthers()`); `hurtServer`: not invulnerable, then `getEntity() != null || is(#always_triggers_silverfish)` calls `friendsGoal.notifyHurt()` before `Monster.hurtServer`; `tick` sets `yBodyRot = yRot` before `Monster.tick`; `getWalkTargetValue` 10 when the block below `isCompatibleHostBlock`, else `Monster`'s; `getMovementEmission` `EVENTS`; `playStepSound` `entity.silverfish.step` at 0.15.
  - `Silverfish$SilverfishWakeUpFriendsGoal`: no flags, not every tick; `notifyHurt` sets `lookForFriends = adjustedTickDelay(20)` (10 goal ticks) only when it is 0; `canUse` `lookForFriends > 0`; `tick` counts down, and at 0 searches y 0, -1, 1, .. to 5 and x, z 0, -1, 1, .. to 10 for `InfestedBlock`s (destroy with `mob_griefing`, else the host state), stopping on `nextBoolean`.
  - `Silverfish$SilverfishMergeWithStoneGoal`: `RandomStrollGoal(1.0, 10)` with only `MOVE`; `canUse`: no target, navigation done, then with `mob_griefing` and `nextInt(reducedTickDelay(10)) == 0` a `Direction.getRandom` block from `BlockPos.containing(x, y + 0.5, z)` that is a host block sets `doMerge`; else `RandomStrollGoal.canUse`. `start` with `doMerge` turns it infested, `spawnAnim`, `discard`.
  - `Endermite`: `xpReward` 3, `DEFAULT_LIFE` 0, `MAX_LIFE` 2400, health 8, speed 0.25, attack 2; `Lifetime` read and written; `aiStep` after `Monster.aiStep` on the server: `life++` unless `isPersistenceRequired`, then `discard` when `life >= 2400` (persistent or not); on the client two `PORTAL` particles a tick; `tick` `yBodyRot = yRot`; `EVENTS`; `playStepSound` `entity.endermite.step` at 0.15.
  - `Entity$MovementEmission(sounds, events)`: `NONE(false, false)`, `SOUNDS(true, false)`, `EVENTS(false, true)`, `ALL(true, true)`. `Entity.move` plays the step (and amethyst chime) only with `emitsSounds`, and the swim sound only with `emitsSounds`.
  - `ClimbOnTopOfPowderSnowGoal`: `JUMP`, every tick; `canUse` `(wasInPowderSnow || isInPowderSnow)` and `#powder_snow_walkable_mobs`, then the block above `blockPosition` is powder snow or has an empty collision shape; `tick` jumps.
  - `RandomStrollGoal`: `DEFAULT_INTERVAL` 120; `canUse` refuses a mob with a controlling passenger, then `noActionTime >= 100`, then `nextInt(reducedTickDelay(interval)) != 0`; `getPosition` `DefaultRandomPos.getPos(mob, 10, 7)`; `start` `moveTo(wanted, speed)`; `canContinueToUse` navigation not done; `stop` stops it.
  - `HurtByTargetGoal.alertOthers`: `AABB.unitCubeFromLowerCorner(position).inflate(follow, 10, follow)`, `getEntitiesOfClass(mob.getClass(), NO_SPECTATORS)`, skipping itself, mobs with a target, tamables with another owner, allies of the attacker and `toIgnoreAlert`; then `setTarget`.
  - Data: `#always_triggers_silverfish` is `minecraft:magic`; the `entities/silverfish` and `entities/endermite` loot tables have no pools. `Blocks`: the seven `INFESTED_*` blocks have the hosts stone, cobblestone, stone bricks, mossy, cracked and chiseled stone bricks and deepslate.
- **Decisions:**
  - **No step or swim sounds.** The brief said both step at 0.15, but javap shows `EVENTS` (`sounds` false), so vanilla never plays their step, chime or swim. The families answer `emits_sounds` false and keep `step_sound` as the vanilla step for completeness. The splash on entering water still plays (`doWaterSplashEffect` is not gated).
  - **The wake goal only counts down here.** The search for infested blocks at 0 belongs to group silverfish-behaviour (D0-7.3), which also applies the blocks.
  - **The merge goal is its stroll here.** `SilverfishMergeWithStoneGoal` keeps its "no target, navigation done" gate and the `RandomStrollGoal(1.0, 10)` part. Merging (with its `nextInt(5)` and direction draws, the host block and `core.removed`) comes with silverfish-behaviour.
  - **Shared goals file.** `ClimbOnTopOfPowderSnowGoal` and `RandomStrollGoal` live in the root-owned `world/families/goals.rs`, so M1's rabbits and foxes (also in `#powder_snow_walkable_mobs`) reuse them. The tag check is left out because only tag members add the goal. "In powder snow" is the box (less 1e-5) touching powder snow, since bodies don't track `isInPowderSnow`. The goal reads it during the AI step, which is where vanilla's `wasInPowderSnow` comes from.
  - **`family_alert` matches by kind** (vanilla's `getClass()` for these one-kind families). It has no health check (`getEntitiesOfClass` sees dying mobs) and no tamable or ally check (no registered tamable exists yet). It sits in the root so every goal family with `setAlertOthers` can call it.
  - **Yaw comes from the core.** `family_goal_step` copies `core.yaw` into the AI first. This does nothing in normal play, but a yaw set from outside (a loaded `Rotation`) now sticks. Both families also set the body rotation to the yaw before each step (vanilla `tick`: `yBodyRot = yRot`).
  - **The endermite's life counts every tick it lives**, with or without AI and while dying, because vanilla's `aiStep` runs either way. A saved `Lifetime` of 2400 or more ends even a persistent mite.
  - Both loot tables are empty, but `table` stays `Some(kind)` as for any monster (vanilla still rolls the empty table).
  - The `Silverfish::hurt` override calls `hurt_plain` directly because its armour is 0.
  - Two vendored comments in `monster_ai.rs` still say that no endermites live (`Prey::Absent`, `enderman_targets`). They stay as they are, because the fork writes no comments and the code is right.
- **Left for later:**
  - The world crate's silverfish spawn row checks a spawner's spot with the monster walk value (darkness). Vanilla `getWalkTargetValue` gives 10 above a host block, so a silverfish spawner over stone bricks also spawns in light (the lit stronghold portal room, R-13). Group silverfish-behaviour or a world-crate follow-up gives the row the host-block walk value.
  - In game nothing changes yet: the terrain does not simulate silverfish or endermites until group registry-client (D0-5.1), so `mobs.md` keeps both rows `left`.
- **Runs:** `test -p minecraftoss-entities --locked --lib --bins` (420 passed, every bin builds), `clippy -p minecraftoss-entities --locked --lib --tests` (no warnings), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The three new files pass `rustfmt --check --edition 2021` with empty output on their staged content.
- **Test steps** (silverfish and endermites are not live in the game yet; the steps check that the old mobs' sounds, strolls and hunts are unchanged). Release build, survival, Normal, by day, flat open ground in the Overworld. Name each screenshot by its label, `D0-7-1.png`.
  1. Use a cow egg and a zombie egg on stone 4 blocks away (at night, or under a roof for the zombie) and stand still: as they walk you hear their footsteps (the cow's on grass and stone, the zombie's own step). Use a horse egg: its hooves clop as it wanders. **Screenshot D0-7-1** with the three mobs.
  2. Pour water beside the cow and the zombie: both splash going in and swim with the swim sound.
  3. Use an enderman egg and a zombie egg 6 blocks apart, then an iron golem egg beside them: the golem walks to the zombie and throws it up. Look at the enderman's head: it shakes and comes for you. **Screenshot D0-7-3.**
  4. Leave the cow and a sheep for a minute: they wander a few blocks at a time as before.

```
IW4L_MINECRAFT_KIT=minecraft:cow_spawn_egg*2,minecraft:zombie_spawn_egg*4,minecraft:horse_spawn_egg*2,minecraft:sheep_spawn_egg*2,minecraft:enderman_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:water_bucket,minecraft:stone*64,minecraft:iron_sword,minecraft:shield,minecraft:cooked_beef*32
```

## Group registry-client

### 1. Registered Minecraft mobs are saved, loaded and hatched

After bdbc8852.

- **Changed (`minecraft_terrain`, D0-5.1):**
  - New root `families.rs` (`pub mod families;` in `lib.rs`, after `server_mobs`): `Common` (the values `spawn_saved` already parses: position, yaw, health, persistence, `NoAI`, the UUID-seeded random, fire, `Age`), the `Add` callback type of `mob_tags_where`, the `TerrainFamily: Registered` trait with `read` and `fields` (defaults: nothing), the `terrain_group!` macro (per group: `kinds`, `spawn_saved`, `save`; it accepts zero entries and entries with attributes), and the root `kinds`, `spawn_saved` and `save` over the seven groups.
  - Seven group files `families/{pests,trial,deep_dark,land,water,night,bosses}.rs`, each one `terrain_group!` line; only `pests` has entries. The silverfish and endermite glue is in `families/pests/silverfish.rs` and `families/pests/endermites.rs`: each re-exports the entity type and implements `TerrainFamily`. Both `read` set their goals' head turn to the saved yaw. The endermite reads and writes `Lifetime` (vanilla `Endermite.readAdditionalSaveData`/`addAdditionalSaveData`).
  - `server_mobs::spawn_saved`: its `_ => return None` arm becomes the generic arm. It finds the family whose `KINDS` holds the tag's `id`, calls `S::spawn` with that `'static` entry, applies the common values and `TerrainFamily::read`, then `spawn_family`. The existing tail (UUID, leash, effects, motion, ground, fire) then runs for registered mobs too.
  - `mob_tags_where`: one `crate::families::save(world, &mut add)` after the last family (the chickens). It saves every core that has not left (`removed`) through the existing `add`, so the original tag is merged as for every mob and a dying mob is skipped as before.
  - `is_simulated(kind)` and `simulated_kinds()` (sorted, each once): `SIMULATED_KINDS` plus the terrain registry's kinds. `carries` takes registered riders through `is_simulated`, and registered vehicles through `|| families::is_vehicle(id)` (the static `MOUNT` const).
- **Changed (`minecraft_survival`):** `commands::registries` lists `simulated_kinds()`. `/summon` refuses any kind not on that list (`minecraft_commands` `summon`), so this is what lets `/summon minecraft:silverfish` and `/summon minecraft:endermite` work, and tab completion offers them.
- **Changed (terrain tests, `server.rs`):** the eggs survey compares the hatched eggs with `HATCHING_EGGS` plus every registered kind without `minecraft:`, sorted; `HATCHING_EGGS` stays as it was. `command_summons_do_not_reach_the_egg_results` summons every kind of `simulated_kinds()`. `live_shulkers_stay_far_from_players_while_dormant_monsters_go` uses `minecraft:phantom` as its dormant monster instead of the now simulated silverfish (D0-9.4; M3's phantom commit must switch it again).
- **What changes in the game:** silverfish and endermites from eggs, `/summon`, spawners, saved chunks and stronghold spawners now live in the entity world instead of lying dormant: they hunt, bite, are saved with their tags and load again. They are not drawn yet (group silverfish-drawn), and MW2 bullets do not hit them yet (group registry-draw).
- **Tests added:** `server_mobs::tests::a_silverfish_and_an_endermite_save_and_load_with_its_lifetime` (position, yaw and head turn, health capped at the maximum, persistence, `NoAI`, fire, motion, ground, UUID and `Lifetime` load; health, persistence and `Lifetime` save, with an unsimulated field such as `CustomName` kept from the original; both load again the same; a removed mite is not saved; a silverfish saved with a rider stays dormant), `server_mobs::tests::every_registered_kind_is_simulated_and_summonable` (the terrain registry names exactly the entity registry's kinds; `simulated_kinds` sorted, each once, holding `SIMULATED_KINDS` and every registered kind; `is_simulated`; every registered kind loads from a bare tag and rides a ravager as a saved jockey), `commands::tests::summon_knows_the_registered_mobs` (survival: the `/summon` list holds the silverfish and the endermite, sorted). The eggs survey (gated on the 26.3 data) now hatches silverfish and endermites.
- **Decisions:**
  - A registered mob whose tag has `Passengers` stays dormant unless `carries` stripped them (a registered vehicle with simulated riders), exactly as the horse, vex and ravager arms do, so its riders are never lost on the next save.
  - The saved health is capped at the family's maximum (vanilla `setHealth` clamps to `getMaxHealth`). A saved `Age` below 0 makes `core.baby` (vanilla `AgeableMob`); a family whose babies are saved another way (`IsBaby`) reads them in its `read`.
  - `is_simulated` and `simulated_kinds` read the terrain registry (`crate::families::kinds()`), not the entity one, because a kind only loads when it has a `TerrainFamily`. The new test checks that the two registries name the same kinds.
  - The new terrain tests sit next to the vex save test, not at the end of the test module, so other tracks' tests appended at the end never touch them.
  - The family glue files `pub use` the entity type, so each terrain group file names its families as `silverfish::Silverfish`, the same as the entity group file does.
- **Runs:** `test -p minecraft_terrain --locked --lib` (1395 passed, 3 ignored; the known flaky `sections::tests::background_full_update_matches_the_synchronous_one` failed once under load and passed when run alone), `test -p minecraft_survival --locked` (760 + 1 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new files pass `rustfmt --check --edition 2021` with empty output on their staged content.
- **Test steps** (silverfish and endermites are live but not drawn yet, so the steps check what can be seen: the summons, the bites, saving and the old mobs). Release build, survival, Normal, by day, flat open ground in the Overworld. Name each screenshot by its label, `D0-8-1.png`.
  1. Type `/summon minecraft:s` and press Tab: `minecraft:silverfish` is offered. `/summon minecraft:silverfish ~ ~ ~3`: the chat says "Summoned new Silverfish" (before this change it said "Unable to summon entity"). **Screenshot D0-8-1** of the chat line.
  2. Stand still for ten seconds: something you cannot see yet bites you for half a heart about once a second, with the hurt sound. Look down at your feet and swing the sword until the bites stop.
  3. Use the endermite egg on the ground next to you: you take bites of a full heart. Run 30 blocks away and wait two minutes, then come back: nothing bites you any more (the endermite has left).
  4. Use a silverfish egg next to you and save and quit at once. Load the world: the bites start again, so it was saved and loaded.
  5. Use the zombie, cow and villager eggs, save and quit, load: all three are still there and move as before. **Screenshot D0-8-5.**

```
IW4L_MINECRAFT_KIT=minecraft:silverfish_spawn_egg*4,minecraft:endermite_spawn_egg*4,minecraft:zombie_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:villager_spawn_egg*2,minecraft:iron_sword,minecraft:cooked_beef*32
```

### 2. Friends see registered Minecraft mobs

After 757bfed8.

- **Changed (`minecraft_terrain`, D0-5.2):**
  - Root `families.rs`: `FamilyShow { flags: u32, values: [i32; 8] }` (Copy, Default, serde). `TerrainFamily` gains `TRACKING_RANGE` (no default), `VOLATILE` (default empty), `show` (default `FamilyShow::default()`) and `show_on` (default nothing). `terrain_group!` and the root gain `tracking_range`, `volatile`, `shows` and `show_on`. A kind is found with or without `minecraft:`.
  - `mob_tracking`:
    - `tracking_range` asks the registry first.
    - `look` also drops the `VOLATILE` keys of the family whose `KINDS` holds the tag's `id`, and only those.
    - `MobShow.family: Option<FamilyShow>`, at the end of the struct.
    - `shows` makes one `crate::families::shows(world, &mut put)` call after the last family (the piglins), before the raiders' glowing pass. It puts `MobShow { family: Some(S::show(mob)), .. }`.
    - `show_on` makes one `crate::families::show_on(world, mob)` call after the last entity (the llama spit), before the glowing check. It places the core's body with the existing `place`, which is now `pub(crate)`, and calls `S::show_on` when the mob's show has a family part.
  - `client_mobs::server_mobs`: one loop after the shulkers pushes every registered core that has not left (`core.yaw`, `core.look`, `core.body`, `core.baby`, `core.eye_height`), and the swings chain gains `core.swings` after the ravagers. Without this the tracker never sees a registered mob (it walks `server_mobs`), so no friend is ever sent one. The brief listed it under D0-5.3, so group registry-draw only adds the head-turn tweak (`max_head_y_rot`) from that bullet.
  - Glue: silverfish and endermites track at 8 chunks (`clientTrackingRange(8)`). The endermite's `Lifetime` is `VOLATILE`, so its count does not resend its look tag every 10 ticks.
- **Protocol:** `MobShow` is serialised in every `ServerMob` of the state lane, so this commit changes the wire format; see "## Protocol" above. `PROTOCOL` stays 6.
- **What changes in the game:** a friend now gets every registered mob near them: added from its look tag with the generic `spawn_saved`, moved and placed every tick, its bites counted. Nothing is drawn for them yet, on the host or for a friend (group silverfish-drawn).
- **Tests added:**
  - `mob_tracking::tests::a_friend_sees_a_silverfish_and_an_endermite_from_their_look_tags`:
    - tracking range 8 with or without `minecraft:`;
    - both are added to a watcher and spawn on the friend with their ids, the yaw and the endermite's `Lifetime`, and both move every tick;
    - `shows` gives each its family part;
    - a `Lifetime` counting up sends no new look;
    - a moved, burning, biting silverfish reaches the friend through `show_on`: position, previous position, fire and swings.
  - `mob_tracking::tests::a_registered_look_tag_drops_only_its_own_volatile_keys`: the endermite's look loses `Lifetime` but keeps an unrelated `Size`. A silverfish tag and a slime tag keep a `Lifetime` key, because neither lists it.
  - `mob_sender::tests::a_registered_mobs_show_crosses_the_state_lane` (survival): a `ServerMob` with `family: Some(FamilyShow { .. })` and one without round-trip through `minecraft_net::encode`/`decode` on the state lane, and the family part travels with its mob.
- **Decisions:**
  - The `server_mobs` push loop and the swings line move from D0-5.3 into this commit (see above). Both are one generic line each at the end of their lists, as D0-5.3 describes them.
  - `show_on` sets the body only (position, ground, fire) like every other family's arm; the turn comes from the `ServerMob` through `ClientMobs`, as for the rest.
  - Dying registered mobs are pushed like every other mob, so the friend sees the death. Mobs that left (`removed`) are not pushed; they leave `order` at the end of the tick anyway.
  - The `look` lookup uses `.unwrap_or(&[])`, because the workspace's `clippy.toml` forbids `unwrap_or_default`.
- **Left for later (D0-9.4):** `show_on` with family values, `families::interact`, `is_vehicle` in `carries` and the mount inventory get no real-family test in D0, because silverfish and endermites use none of them. The first track whose family uses one adds that test in its own files.
- **Runs:**
  - `test -p minecraft_terrain --locked --lib`: 1397 passed, 3 ignored. The known flaky `sections::tests::background_full_update_matches_the_synchronous_one` failed once under load and passed when run alone. `mob_tracking::` passed again after the clippy fix.
  - `test -p minecraft_survival -p minecraft_net --locked`: survival 761 + 1 passed; net 24 passed, with the codec asserts still at protocol 6.
  - `clippy -p minecraft_terrain -p minecraft_survival --locked --lib --tests -A clippy::approx_constant`: no warning on a line of this group; the remaining warnings are on older lines.
  - `check -p launcher --locked`: green, with only the old render_frame and skate warnings.
  - The changed new files pass `rustfmt --check --edition 2021` with empty output on their staged content.
- **Test steps** (two windows with `scripts\mp-local.ps1`, both on this build. A friend on an older build gets mob states it cannot read, until the release bumps `PROTOCOL`). Survival, Normal, at night, flat open ground in the Overworld, the friend 5 blocks from the host. Name the screenshots `D0-9-<step>-host.png` and `D0-9-<step>-friend.png`.
  1. The host uses a zombie egg and a creeper egg next to the friend. In both windows they walk to the friend, the zombie swings its arms as it hits, and the creeper swells and flashes white before it blows up. **Screenshots D0-9-1-host and D0-9-1-friend** while the creeper is swelling.
  2. The host uses a silverfish egg and an endermite egg next to the friend. Nothing is drawn for them yet, in either window (group silverfish-drawn draws them), but the friend's game keeps running smoothly and shows no error line in the chat while they bite the friend (half a heart from the silverfish, a full heart from the endermite). The host walks to the friend, looks at the friend's feet and swings the sword until the bites stop (a friend's own hits on registered mobs reach the host with group registry-draw, which decodes their hit keys).
  3. The host uses a cow egg and a villager egg, then saves and quits, loads and opens the world to the friend again. The friend joins and sees the cow and the villager where the host left them. **Screenshot D0-9-3-friend.**

```
IW4L_MINECRAFT_KIT=minecraft:zombie_spawn_egg*2,minecraft:creeper_spawn_egg*2,minecraft:silverfish_spawn_egg*4,minecraft:endermite_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:villager_spawn_egg*2,minecraft:iron_sword,minecraft:cooked_beef*32
```

### 3. Minecraft client mob list keeps its spacing

After 2842608b.

- **Changed:** `client_mobs::server_mobs`. 2842608b lost the space after `let shulkers:` on the line after its new loop; this puts it back, so that commit's change to `client_mobs.rs` is only its two generic additions. There is no change in behaviour.
- **Tests:** none (spacing only). **Runs:** `check -p launcher --locked` green.
- **Decision:** a separate commit instead of an amend, because the track's rule is no amending.
- **Test steps:** none (no change in the game).

## Group registry-draw

### 1. Registered Minecraft mobs are drawn, shot and used through one registry

After 2a325d3c.

- **Changed (`minecraft_terrain`, D0-5.3 and D0-6.1):**
  - Root `families.rs`:
    - `FamilyDraw { models, culled, translucent, held }` and `ClientFx { ambient, spell, portal, poof, sounds, random, seen }`.
    - `TerrainFamily` gains `TEXTURES` and `SHADOW_RADIUS` (no default), `MAX_HEAD_Y_ROT` (75) and `CAGE_MARGIN` (0.05), `draw` (no default), `interact` (default: pass) and `client_effects` (default: nothing).
    - `terrain_group!` and the root gain `textures`, `draw`, `draw_glowing`, `shadows`, `max_head_y_rot`, `cage_margin`, `interact` and `client_effects`, plus the root `knows(kind)`.
  - `client_mobs`: the head-turn tweaks ask `families::max_head_y_rot(world, id)` after the ravagers, and `shadow_casters` ends with `families::shadows` (`SHADOW_RADIUS`, halved for babies, none for a mob that left).
  - `mesh.rs` `build_internal`: one `for texture in crate::families::textures()` after the elytra wings.
  - `spawner_display`: `Shown::build` takes `DRAWN_KINDS` or `families::knows(kind)`, and the draw match has one `kind if families::knows(kind)` arm before `_ => {}`.
  - `mob_actions::interact`: a `MobHit::Family(key, id)` arm before `_ => return outcome` hands the click to `families::interact`.
  - `keyframes.rs`: `install(jar) -> bool` (the first call wins) and `shared() -> &'static AnimationLibrary`.
  - Glue: the silverfish (`minecraft:entity/silverfish/silverfish`) and the endermite (`minecraft:entity/endermite/endermite`) give their textures and a shadow of 0.3. Their `draw` adds nothing yet; group silverfish-drawn ports the models.
- **Changed (`minecraft_survival`):**
  - `entities.rs` `decode`: any byte with `families::has_key` gives `MobHit::Family(byte, id)`, so MW2 bullets, blasts and a friend's shots reach registered mobs.
  - `entity_view.rs`:
    - `boxes` has one generic loop of living registered cores (`MobHit::Family(key, id)`).
    - `meshes` makes one `families::draw` call after the felines, with its held items chained after the vexes'.
    - One `families::draw_glowing` pass after the witches' glow loop, which hands each glowing registered mob to `glow_render::append_glowing`.
    - `advance` makes one `families::client_effects` call after `wither_effects`, with a `ClientFx` built from the view's particles, sounds, random and the new `family_seen` map.
- **Changed (`render_anim`):** `minecraft_world::load_look` calls `minecraft_terrain::keyframes::install(data_jar())` first. Both the host's world load and the friend's `prepare` run `load_look`, so host and friend install the same jar.
- **Tests added:**
  - In terrain `families::tests` (new file):
    - `a_registered_mob_is_tracked_textured_and_shadowed_through_its_family`: textures, `knows`, `cage_margin` with or without `minecraft:`, the head turn in `server_mobs`, shadows of 0.3 and 0.15 for a baby, and none once removed.
    - `a_glowing_registered_mob_is_drawn_alone_for_its_outline`: only the glowing ids, in store order, each with exactly its own draw, and none once removed.
    - `a_click_on_a_registered_mob_passes_unless_its_family_takes_it`: the silverfish passes, an unknown key passes, a lead in hand stays, and `would_interact` is false.
    - `client_effects_forget_registered_mobs_that_are_gone`.
  - `spawner_display::tests::every_registered_kind_is_shown_inside_its_cage`: every registered kind gets a display in its own world, inside the cage within its `CAGE_MARGIN`, and turns with the spin once it draws.
  - `keyframes::tests::the_shared_library_reads_after_install_and_says_why_before`: before install it says "no client jar"; the first install takes and a second does not; with `MINECRAFTOSS_ROOT` it reads `WardenAnimation` once and `BatAnimation`.
  - In survival:
    - `entities::tests::every_registered_family_hit_key_encodes_and_decodes`: every `has_key` byte is above 35, round-trips and is no crystal, fireball, bullet, frame or End key. 38 and the probe's 39 decode to nothing.
    - `entity_view::tests::a_registered_mob_is_shot_by_its_family_hit_box`: the silverfish's box is 0.4 x 0.3 under key 36 and the endermite's is under key 37. A ridden mob is left out of `shot_boxes`. The decoded hit hurts it through `mob_actions::attack`. A dead or removed mob has no box.
    - `entity_view::tests::a_glowing_registered_mob_is_outlined`: `meshes` outlines the silverfish with the Glowing effect and not the endermite without it, exactly as `draw_glowing` with `w.glowing` does.
- **javap this round:** `SilverfishRenderer` and `EndermiteRenderer` both call `MobRenderer(context, model, 0.3F)`, use `getFlipDegrees` 180 and take their textures from `textures/entity/silverfish/silverfish.png` and `textures/entity/endermite/endermite.png`.
- **Decisions:**
  - **Where the glow loop lives.** `entity_view` cannot name each family's type, so the generic loop is `families::draw_glowing` in terrain, called once beside the per-kind loops. It draws each glowing registered mob alone (`std::slice::from_ref`) and merges its models, culled and translucent output into one mesh, so the hull is one box around the whole model. Held items get no outline, as in the illager loop. Mobs that left are skipped.
  - **Shadows before models.** The silverfish and endermite shadows show from this commit on, while the models come with group silverfish-drawn. The shadow is the vanilla renderer's, so the owner can see where the mobs are.
  - **Clicks on registered mobs.** The `MobHit::Family` arm sets `outcome.handled` to the family's answer and returns at once. The family sets `inventory_changed`, `mount` and the other fields itself; the shared tail, which marks every handled click as an inventory change, does not run for it.
  - **The keyframe holder.** `shared()` before any install hands out a separate empty library, so an early call never takes the slot that `install` fills. `install` returns whether it took; `load_look` ignores the answer, because a second world opened in the same run keeps the first jar.
  - **Placement.**
    - The `boxes` loop sits after the piglins and before the shulkers, the last user of the `add` closure (the shulker loop pushes to `out` directly).
    - The `meshes` draw sits after the felines (the last mob draw before the leashes), the glow pass after the witches' glow loop, and the held items after the vexes'.
    - `client_effects` comes after `wither_effects`.
  - **No held items in cages.** The spawner display's arm passes a throwaway held list, as its other arms drop their held items.
  - **`ClientFx.seen`** is cleared of mobs that are gone before the families run, so it never grows.
  - The look-tag test named in D0-9.4 (`a_registered_look_tag_drops_only_its_own_volatile_keys`) landed with group registry-client (2842608b).
- **Left for later:**
  - **Geometry checks.** Silverfish and endermites draw nothing until group silverfish-drawn, so the cage test and both glow tests compare against each family's own (empty) draw. Group silverfish-drawn should add non-empty asserts for the two kinds there.
  - **Unused hooks.** `client_effects` (the endermite's portal particles), `interact` and `FamilyDraw.held` have no family that uses them yet; the first family that does adds its test (D0-9.4).
  - Moving the bat to `BatAnimation` (optional, round 9 leftover) is not done.
- **Protocol:** no message or codec change.
- **Runs:**
  - `test -p minecraft_terrain --locked --lib`: 1404 passed, 3 ignored.
  - `test -p minecraft_survival --locked`: 764 + 1 passed.
  - `test -p render_anim --locked --lib`: 212 passed.
  - `check -p launcher --locked`: green, with only the old render_frame and skate warnings.
  - The new and changed new files pass `rustfmt --check --edition 2021` with empty output.
- **Test steps** (the silverfish and endermite models come with group silverfish-drawn; until then each shows as a small round shadow on the ground). Release build, survival, Normal, by day, flat open ground in the Overworld. Step 4 needs two windows (`scripts\mp-local.ps1`). Name each screenshot by its label, `D0-10-1.png`, and for both windows `D0-10-4-host.png` and `D0-10-4-friend.png`.
  1. Use a silverfish egg on the ground 5 blocks away. A small round shadow slides over the ground towards you, and you take bites of half a heart. **Screenshot D0-10-1** with the shadow in view.
  2. Shoot the ground where the shadow is with the M4A1. After a few hits the bites stop, the shadow is gone and experience orbs drop there.
  3. Use an endermite egg and shoot its shadow the same way: it dies and leaves experience.
  4. Two windows: the host uses a silverfish egg next to the friend. Both windows show the shadow. The friend shoots it with the M4A1: in both windows the shadow goes and the orbs drop. **Screenshots D0-10-4-host and D0-10-4-friend** before the shot.
  5. Hold a lead and right-click where a silverfish's shadow is: nothing happens and the lead stays in your hand. Right-click a cow with wheat: it eats and shows hearts as before.
  6. Use a pillager egg and a zombie egg 8 blocks away. Shoot the pillager with a spectral arrow from the bow: it gets the white outline as before. Shoot both with the M4A1: they flash and die as before. **Screenshot D0-10-6** with the outlined pillager.

```
IW4L_MINECRAFT_KIT=minecraft:silverfish_spawn_egg*4,minecraft:endermite_spawn_egg*4,minecraft:pillager_spawn_egg*2,minecraft:zombie_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:wheat*8,minecraft:lead*2,minecraft:bow,minecraft:spectral_arrow*16,minecraft:iron_sword,minecraft:cooked_beef*32
```

### 2. Minecraft players cannot sleep beside pillagers, vexes, ravagers or withers

After 126b07a1.

- **Changed (`minecraft_survival`, D0-5.4):** `entities::resting_monsters` (the mobs the bed check looks at) also lists the living illagers (pillager, vindicator, evoker by their kind), vexes, ravagers and withers, after the piglins. One generic loop after them lists every living registered mob that has not left, by its kind. The bed check still keeps only the kinds on `sleep::MONSTERS`, which already names all of them, the silverfish, the endermite and the round 10 monsters.
- **Before this:** the stores added since round 8 were missing from the list, so a player could sleep with a pillager, vex, ravager or wither next to the bed. Silverfish and endermites, live since group registry-client, were missing as well.
- **Tests added:** `entities::tests::a_player_cannot_sleep_beside_a_pillager_vex_ravager_or_wither`. Each of these, 2 blocks from the bed, keeps the player awake: a pillager, a vindicator, an evoker, a vex, a ravager, a wither, a silverfish and an endermite. A ravager 12 blocks off does not, and neither do a dead wither, a dead vex, a dead silverfish or an endermite that left.
- **javap this round:** `Monster.isPreventingPlayerRest` returns true. `WitherBoss` and `Vex` extend `Monster`, `Ravager` extends `Raider` (a `Monster`), and `Pillager` extends `AbstractIllager`.
- **Decisions:**
  - Dead mobs are skipped, like every other store in the list. Registered mobs that left (`removed`) are skipped too.
  - No mob in the new stores answers "angry": only the zombified piglin's rule depends on anger (`sleep::prevents_rest`).
- **Protocol:** no message or codec change.
- **Runs:** `test -p minecraft_survival --locked` (765 + 1 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). `clippy -p minecraft_terrain -p minecraft_survival --locked --lib --tests -A clippy::approx_constant` shows no warning on a line of this commit.
- **Test steps.** Release build, survival, Normal, in the Overworld. Place a bed on open ground and stand beside it. Name each screenshot by its label, `D0-11-1.png`.
  1. `/time set night`. `/summon minecraft:pillager ~3 ~ ~ {NoAI:1b}` and right-click the bed: the message "You may not rest now; there are monsters nearby" shows and you stay up. **Screenshot D0-11-1.** `/kill @e[type=minecraft:pillager]`.
  2. Do the same with `minecraft:vex`, `minecraft:ravager` and `minecraft:silverfish`, one at a time, each with `{NoAI:1b}`: each one keeps you awake. Kill each before the next.
  3. `/summon minecraft:wither ~6 ~ ~ {NoAI:1b}` and right-click the bed: you stay up. `/kill @e[type=minecraft:wither]`.
  4. With no monster near, right-click the bed: you lie down and the night passes. **Screenshot D0-11-4** while lying down.

```
IW4L_MINECRAFT_KIT=minecraft:white_bed*2,minecraft:silverfish_spawn_egg*2,minecraft:iron_sword,minecraft:cooked_beef*32
```
