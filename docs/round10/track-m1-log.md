# Track M1 log: land animals on the shared mob registry

Branch `track-m1`, worktree `C:\Dev\mc-wt\track-m1`, target dir `D:\iw4l-wt\track-m1`. Brief: `docs/round10/track-m1.md`. Mob list: `docs/round10/mobs.md` (the `## M1` rows only). Registry: `docs/round10/track-d0.md` (D0-2, D0-3).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Vendored files

Files under `third_party/minecraftoss` this track patched or added.

- Patched: `entities/src/world/families.rs` (hook, rabbits-live 1), `entities/src/world/families/tests.rs` (its test, rabbits-live 1), `entities/src/world/families/land.rs` (registration, rabbits-live 2), `entities/src/monster_ai.rs` (`AvoidEntityGoal::matching`, rabbits-live 2), `world/src/natural_spawner/families/land.rs` (the rabbit spawn test, rabbits-live 2).
- Added: `entities/src/world/families/land/animal_goals.rs`, `entities/src/world/families/land/rabbits.rs`, `entities/src/world/families/land/rabbits/ai.rs`, `entities/src/world/families/land/rabbits/tests.rs` (test only) (rabbits-live 2).

## Hooks

Root hooks this track added (M10, "Hooks this track adds").

- **"Registered Minecraft mobs can run their own AI step"** (rabbits-live 1): `EntityWorld::family_goal_step_with::<S, W>(id, level, players, movement_speed, ai, run)` in `entities/src/world/families.rs`, with the type `AiStep<W>` (`monster_ai_step`'s signature, returning the landed fall distance). `family_goal_step` is now that call with `monster_ai_step`.
- `land/animal_goals.rs` (rabbits-live 2, M1's group module, `pub mod`): goal-framework `PanicGoal<E>`, `TemptGoal<E>` and `BreedGoal<E>` over a family's `GoalExtra` type `E: AnimalExtra` (which hands out its `AnimalGoals`), and `is_monster_class`. Later M1 families reuse them; `MoveToBlockGoal` joins with `rabbits-behaviour`.
- `AvoidEntityGoal::matching(fn(&str) -> bool)` (rabbits-live 2, `monster_ai.rs`): beside `also`; a field set by `new`, so no literal changes.

## Protocol

No commit of this track has changed a `ToClient`/`ToServer` message or the codec yet. `minecraft_net::PROTOCOL` is not bumped here.

## State doc changes

None yet.

## Left for later

- The hop's dust (entity event 1, `spawnSprintParticle`): `ClientFx` has no block particles (M1-1).
- The shared `FloatGoal` uses a fixed 0.4 water height; vanilla's `getFluidJumpThreshold` is 0 for an eye below 0.4 (a baby rabbit, a silverfish). The rabbit's own jump uses the right threshold.

## Segment m1a

### Group rabbits-live

#### 1. Registered Minecraft mobs can run their own AI step

After df7258b4.

- **Checked first:** D0's merged `family_goal_step(id, level, players, movement_speed, ai)` always calls the fixed `monster_ai_step` (`families.rs`), so no family could swap the step; the hook commit was needed.
- **Changed (vendored entities crate, root `world/families.rs`):**
  - New `pub(in crate::world) type AiStep<W>`: `fn(&mut MonsterAi, &mut Body, f32, &mut LegacyRandom, &mut Vec<(Voice, DVec3)>, &mut i32, DVec3, i32, u64, &W, &MonsterStep) -> Option<f64>`, exactly `monster_ai_step`'s arguments.
  - New `EntityWorld::family_goal_step_with::<S, W>(id, level, players, movement_speed, ai, run)`: the same core sync as before around `run` (hurt-by, body, health, eye height, tick count, griefing, daylight and the movement speed with effects in; yaw, look and hurt-by back out), and the fall damage on the distance `run` returns, through `S::hurt` with the monster or creature fall sound.
  - `family_goal_step` is now `family_goal_step_with(.., monster_ai_step)`, so silverfish, endermites and the probe run as before.
  - Both are `pub(in crate::world)` (the hook) and `pub` (the old helper): `MonsterStep` is private to `world`, so only families under `world/families/` can pass their own step, which is where every family lives.
- **Tests added:** `world::families::tests::a_registered_mob_runs_its_own_ai_step_with_its_core_in_sync` (a test step on the probe sees the core's hurt-by, position, health, max health, eye height and movement speed; the yaw, head yaw and hurt-by it sets reach the core; the eight blocks it returns hurt the probe for five with the hostile big-fall sound).
- **Decisions:** the hook takes a plain `fn` pointer of `monster_ai_step`'s signature (M10 names it so), not a closure: a family's own step reads and writes its state through the `MonsterAi` it is given (its `GoalExtra`), because the core and the state are borrowed apart around it.
- **Runs:** `test -p minecraftoss-entities --locked --lib families` (23 passed: the registry, silverfish and endermite tests), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** nothing new to see; the old registered mobs behave as before. Release build, survival, Normal, by night in a stone cave. Name each screenshot by its label.
  1. Use the silverfish egg twice on stone 6 blocks away: they crawl to you and bite, and hitting one wakes the other. **m1-hook-1**
  2. Use the endermite egg: it crawls to you and bites; an enderman egg nearby turns the enderman on it.
- **Kit:** none new (D0's kit line has the silverfish and endermite eggs).

#### 2. Minecraft rabbits hop about, flee and follow carrots in the entity world

After 8ad4c103.

- **Checked first (M1-1, "Check first"):** D0's merged `is_simulated` (`server_mobs.rs:501`), the eggs survey (`server.rs`, `every_spawn_egg_of_a_mob_the_game_has_hatches_that_mob`) and the cage test (`spawner_display.rs`, `every_registered_kind_is_shown_inside_its_cage`) all enumerate the terrain families (`crate::families::kinds()`), not the entity side. But D0's `server_mobs::tests::every_registered_kind_is_simulated_and_summonable` asserts that the terrain and entity registries list the same kinds, so an entity-only registration fails `test -p minecraft_terrain`. This commit touches only the vendored entities and world crates (whose tests are green); that one terrain test is red from this commit until the next, which adds the `TerrainFamily` (save, load, show, `interact`, an empty `draw`) as the brief's fallback says. In-game nothing changes here: the terrain never spawns a kind it does not register, so rabbits stay dormant.
- **javap this round** (`net.minecraft.world.entity.animal.rabbit.Rabbit` and its inner classes, `Mob.serverAiStep`, `MoveControl`, `LivingEntity.getJumpPower(F)`, `jumpFromGround`, `Entity.getFluidJumpThreshold`, `PanicGoal`, `TemptGoal`, `BreedGoal`, `Mob.getMaxHeadYRot`): the M1-1 "Vanilla 26.3" numbers hold. Added on the way:
  - `Mob.serverAiStep` runs sensing, targets, goals, `navigation.tick`, `customServerAiStep`, then the move, look and jump controls; `Rabbit.aiStep` counts the jump ticks after `Animal.aiStep`.
  - The constructor calls `setSpeedModifier(0)`, which sets the move control to `MOVE_TO` towards (0, 0, 0) at speed 0.
  - `RabbitMoveControl.tick`'s `setSpeedModifier` also sets the navigation's speed, so an idle rabbit's navigation speed drops to 0 between hops; `nextJumpSpeed` keeps the last speed above 0, and the landing delay reads the move control's speed that the navigation set that tick.
  - `RabbitJumpControl.tick` in 26.3 starts a hop on any wanted jump (it no longer checks `canJump`, which nothing else reads), so `canJump` is not ported.
  - `getFluidJumpThreshold` is 0.4, or 0 for an eye below 0.4 (a baby rabbit's 0.39).
  - `TemptGoal(1.0, #rabbit_food, false)`: range `TEMPT_RANGE` 10 (sneaking 0.8, at least 2), stop distance 2.5, look limits 95 and 40, calm-down `reducedTickDelay(100)` = 50; the rabbit's is built with `canScare` false, so it never gives up when the player moves.
  - `BreedGoal`: partners within the box grown by 8 and 8 blocks, in love, not panicking, nearest first; it gives up at love time 60 and breeds at `adjustedTickDelay(60)` = 30 within 3 blocks.
  - `PanicGoal`: `#panic_causes` within 40 ticks of the hurt, five-block random spot (or water when burning); it stops with its path.
  - `#rabbit_food` is carrot, golden carrot and dandelion (26.3 data).
- **Changed (vendored entities crate):**
  - New `world/families/land/rabbits.rs`: `Rabbit` (variant, `Age`, love, last damage, `MonsterAi`), `RabbitVariant` (`BROWN` 0 .. `SALT` 5, `EVIL` 99, unknown ids brown), `set_baby` (0.24 x 0.4, eye 0.39), `is_food` (`#rabbit_food`, or the three items without a recipe book), and `impl Family for Rabbit`: key 48, kind `minecraft:rabbit`, 0.49 x 0.6, eye 0.59, health 3, ambient interval 120, category `creature`, leashable (walked by its navigation), panicking while its panic goal runs, its `MonsterSpec`. The tick runs `family_base_tick`, hands the goals the players holding its food (main or off hand, sneaking), the other rabbits as mates, its love and the last damage within 40 ticks, then `family_goal_step_with(.., rabbit_ai_step)` and `family_after_move`. A hurt that lands ends its love.
  - New `world/families/land/rabbits/ai.rs`: the `MonsterSpec` (goals in vanilla order: `FloatGoal`, `ClimbOnTopOfPowderSnowGoal`, `RabbitPanicGoal` 2.2, `BreedGoal` 0.8, `TemptGoal` 1.0, `RabbitAvoidEntityGoal` for players 8, wolves 10 and monsters 4 at 2.2, never for the killer bunny, the water-avoiding stroll 0.6, `LookAtPlayerGoal` 10; no targets yet; the animal walk values and fire maluses back), `RabbitGoals` (the `GoalExtra`: animal goal state, evil, baby, carrot ticks, a speed request from the panic goal, and the jump state) and `rabbit_ai_step`, `monster_ai_step`'s order with the rabbit's parts in it: the speed request, the navigation through the rabbit's `setWantedPosition` (1.5 in water, `nextJumpSpeed`), `customServerAiStep` (jump delay, carrot ticks, landing delay 10 or 3 at 2.2, then face the path's next point, or the wanted point, and hop), `RabbitMoveControl` (speed 0 on the ground between hops), `RabbitJumpControl` (a wanted jump hops), `getJumpPower` (0.3; 0.2 at speed 0.6 or less; 0.5 to a higher next point, against a wall or jumping to a higher wanted spot) and `jumpFromGround` (the push of 0.1 along (0, 1.5, 1), (0, 0.5, 1) for a baby, when slower than 0.1), the jump sound (`entity.rabbit.jump`, pitch 0.64-0.96) and the 15-tick hop clock.
  - New `world/families/land/animal_goals.rs` (`pub mod`): the goal-framework `PanicGoal`, `TemptGoal` and `BreedGoal` over a family's `GoalExtra` through the `AnimalExtra` trait and its `AnimalGoals` state (last damage, panic, tempters, mates, love), and `is_monster_class` (`is_enemy` without ghast, slime, magma cube, hoglin, shulker, phantom and ender dragon).
  - `world/families/land.rs`: `pub mod animal_goals;`, `pub mod rabbits;` and `rabbits: rabbits::Rabbit` in `family_group!`.
  - `monster_ai.rs` `AvoidEntityGoal`: a `matching: Option<fn(&str) -> bool>` field (set in `new`), the builder `matching(..)` beside `also`, and the kind test also takes a kind it matches. No literal changes.
- **Changed (vendored world crate):** `natural_spawner/families/land.rs` gains the test below (no row: the built-in row already matches 26.3).
- **Tests added:**
  - `world::families::land::rabbits::tests`: `a_rabbit_is_small_with_three_health_and_its_kind` (sizes and eyes of adult and baby, census, candidates, traits, goals, the table and 1-3 experience for adults only), `a_rabbit_moves_only_while_it_is_in_the_air` (no forward input on the ground between hops, jump pitches 0.64-0.96, the 15-tick clock, the 10-tick delay, it gets along its path), `a_hop_rises_and_pushes_forward_like_vanilla` (powers 0.2, 0.3 and 0.5, the adult's and the baby's push, no push without speed or when already moving), `a_rabbit_faces_its_next_path_point_before_each_hop` (the path's next point, else the wanted spot; the delay and the landing delay), `a_rabbit_flees_players_wolves_and_monsters_but_not_slimes` (8, 10 and 4 blocks; creative players ignored), `a_hurt_rabbit_panics_at_2_2` (a fall does not panic it, a bite does; the 2.2 jump speed and the 3-tick landing delay), `a_rabbit_follows_a_player_holding_a_carrot_golden_carrot_or_dandelion` (and not wheat), `a_rabbit_in_water_floats_up_and_hops_at_1_5`, `a_wild_wolf_hunts_a_rabbit`.
  - `world::families::land::animal_goals::tests::a_monster_is_an_enemy_but_not_a_slime_ghast_hoglin_or_shulker`.
  - World: `natural_spawner::families::land::tests::rabbits_spawn_on_grass_sand_and_snow_in_daylight_as_one_adult_and_babies_of_its_biome_colour` (pins the built-in row: size, category, rule; snowy plains white or splotched, desert gold, plains brown, salt or black; one colour per group, the rest babies; none on stone or in light 8).
- **Decisions:**
  - `animal_goals` is `pub mod`, not `pub(crate)` as M10 wrote: `RabbitGoals` is public with an `AnimalGoals` field, and a `pub(crate)` module would make that a private type in a public interface; the terrain side may read it too.
  - The rabbit's own state that its step needs (the jump clock, delay, `nextJumpSpeed`, carrot ticks, the panic goal's speed request) lives in `RabbitGoals` inside its `GoalExtra`, because the hook's step only gets the `MonsterAi`.
  - `RabbitPanicGoal.tick`'s `setSpeedModifier` is a request in `RabbitGoals` that the step applies right after the goals, before the navigation ticks, as vanilla's order has it.
  - The monster class to flee is `AvoidEntityGoal::new("", 4, ..).matching(is_monster_class)` (decision 3); registered monsters count through `is_enemy`.
  - The shared `FloatGoal` keeps its fixed 0.4 water height (D0's), so a baby rabbit floats like an adult; its own jump uses the baby's threshold 0.
  - The killer bunny's leap at its target and its goals come with `rabbits-behaviour`; only its "never flees" gate is here.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (448 passed), `test -p minecraftoss-world --locked --lib` (114 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new files pass `rustfmt --check --edition 2021` on their staged content.
- **Test steps:** nothing new in-game yet (the terrain registers rabbits in the next commit); the steps are those of commit 3.
- **Kit:** none.
