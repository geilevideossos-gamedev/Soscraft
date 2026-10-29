# Track M1 log: land animals on the shared mob registry

Branch `track-m1`, worktree `C:\Dev\mc-wt\track-m1`, target dir `D:\iw4l-wt\track-m1`. Brief: `docs/round10/track-m1.md`. Mob list: `docs/round10/mobs.md` (the `## M1` rows only). Registry: `docs/round10/track-d0.md` (D0-2, D0-3).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Vendored files

Files under `third_party/minecraftoss` this track patched or added.

- Patched: `entities/src/world/families.rs` (hook, rabbits-live 1), `entities/src/world/families/tests.rs` (its test, rabbits-live 1), `entities/src/world/families/land.rs` (registration, rabbits-live 2), `entities/src/monster_ai.rs` (`AvoidEntityGoal::matching`, rabbits-live 2), `world/src/natural_spawner/families/land.rs` (the rabbit spawn test, rabbits-live 2).
- Patched (rabbits-live 3): `player/src/lib.rs` (`World::biome_in_tag`).
- Added: `entities/src/world/families/land/animal_goals.rs`, `entities/src/world/families/land/rabbits.rs`, `entities/src/world/families/land/rabbits/ai.rs`, `entities/src/world/families/land/rabbits/tests.rs` (test only) (rabbits-live 2).
- Changed again, no new file (rabbits-drawn 1): `entities/src/world/families/land/rabbits.rs` (`named`, `toast`, `TOAST`, `KILLER_BUNNY`).
- Changed again, no new file (rabbits-drawn 2): `entities/src/world/families/land/rabbits.rs` (`RabbitAnimations`), `entities/src/world/families/land/rabbits/tests.rs` (its test).
- Changed again, no new file (rabbits-behaviour 1): `entities/src/world/families/land/animal_goals.rs` (`MoveToBlockGoal`), `entities/src/world/families/land/rabbits/ai.rs` (`RaidGardenGoal`), `entities/src/world/families/land/rabbits.rs` (the bite), `entities/src/world/families/land/rabbits/tests.rs`.
- Changed again, no new file (rabbits-behaviour 2): `entities/src/world/families/land/rabbits.rs` (armour, voice, the strike), `entities/src/world/families/land/rabbits/ai.rs` (the evil goals and the leap), `entities/src/world/families/land/rabbits/tests.rs`.
- Patched (rabbits-behaviour 3): `entities/src/feline_ai.rs` (the cat's rabbit target slot, one line); `entities/src/world/families/land/rabbits/tests.rs` changed again (the cat test).
- Patched (polar-bears-live 1): `entities/src/monster_ai.rs` (`MeleeState`'s three accessors, `MeleeCheck`, `MeleeAttackGoal::checked` and `tick_with`, `CheckedMeleeAttackGoal`), `entities/src/world/families/tests.rs` (its test).
- Added (polar-bears-live 2): `entities/src/world/families/land/polar_bears.rs`, `entities/src/world/families/land/polar_bears/ai.rs`, `entities/src/world/families/land/polar_bears/tests.rs` (test only). Changed again: `entities/src/world/families/land.rs` (registration), `entities/src/world/families/land/animal_goals.rs` (`FollowParentGoal`, `panic_environmental_cause`, four `AnimalGoals` fields).
- Patched (polar-bears-live 3): `world/src/natural_spawner/families.rs` (`generation_rule`), `world/src/natural_spawner.rs` (its one call in `spawn_for_chunk_generation`), `world/src/natural_spawner/families/tests.rs` (its test).
- Changed again (polar-bears-live 4): `world/src/natural_spawner/families/land.rs` (the polar bear row, `polar_bear_rule` and their tests).

## Hooks

Root hooks this track added (M10, "Hooks this track adds").

- **"Registered Minecraft mobs can run their own AI step"** (rabbits-live 1): `EntityWorld::family_goal_step_with::<S, W>(id, level, players, movement_speed, ai, run)` in `entities/src/world/families.rs`, with the type `AiStep<W>` (`monster_ai_step`'s signature, returning the landed fall distance). `family_goal_step` is now that call with `monster_ai_step`.
- `land/animal_goals.rs` (rabbits-live 2, M1's group module, `pub mod`): goal-framework `PanicGoal<E>`, `TemptGoal<E>` and `BreedGoal<E>` over a family's `GoalExtra` type `E: AnimalExtra` (which hands out its `AnimalGoals`), and `is_monster_class`. Later M1 families reuse them.
- `MoveToBlockGoal` in `land/animal_goals.rs` (rabbits-behaviour 1): vanilla's `MoveToBlockGoal` as a helper that a goal holds. Its fields are public: speed, search range, vertical range and start, `next_start_tick`, `try_ticks`, `max_stay_ticks`, `block` and `reached`. `can_use(ctx, valid)` is the countdown, then `reducedTickDelay(200 + nextInt(200))` and `findNearestBlock` over `feline_ai::find_nearest_block`; then `can_continue(valid)`, `start`, `move_to_target` and `tick` (reached within 1.0 of the centre of the block above, a new walk every 40 ticks). The goal that holds it names its valid block and what it does on arrival, as the rabbit's `RaidGardenGoal` does; foxes' berries and turtles' goals use it the same way.
- **"Minecraft family spawn rules also decide chunk-generation spawns"** (polar-bears-live 3, world root `natural_spawner/families.rs`): `generation_rule(spawns, level, kind, info, pos)` asks a row with a rule (`spawner` false, a generation context with no players), else `check_spawn_rules`; `spawn_for_chunk_generation` calls it in place of `check_spawn_rules`. M1's turtles and any later family whose creature rule differs from the animal rules get placed with new chunks through it.
- `FollowParentGoal<E>` and `panic_environmental_cause` in `land/animal_goals.rs` (polar-bears-live 2, M1's group module, no commit of their own): the goal-framework `FollowParentGoal(speed)` over `follow_parent::FollowParentState` (vanilla's: no controls, only while a baby, the nearest adult of its family within 8 across plus half the two widths and 4 up or down, none closer than 3, kept between 3 and 16 blocks, a new path every 5 goal ticks); `AnimalGoals` gains `id`, `age`, `kin` (the others of its family as `CowCandidate`s, filled by the family's tick) and `follow_parent`. `PANIC_ENVIRONMENTAL_CAUSES` is the 26.3 `#panic_environmental_causes` (`wolf_ai::environmental` is private to the wolf, so the group module keeps its own list). Pandas, foxes and turtles reuse both.
- `AvoidEntityGoal::matching(fn(&str) -> bool)` (rabbits-live 2, `monster_ai.rs`): beside `also`; a field set by `new`, so no literal changes.
- `World::biome_in_tag(pos, tag)` (rabbits-live 3): `player/src/lib.rs` after `biome`, false by default; the server's `MobWorld` answers it after its `biome` (`server_mobs.rs`) through `Level::biome_in_tag_at`. M2's tadpoles use it under this name.
- **"The registry's texture test checks the pests' textures first, not the whole list"** (rabbits-drawn, before commit 1): not a hook, but a root-file change in its own commit. D0's `families::tests::a_registered_mob_is_tracked_textured_and_shadowed_through_its_family` (terrain root `families/tests.rs`) compared `textures()` with exactly the silverfish and endermite textures, so the first family with a texture of its own (the rabbit) fails it. It now compares `textures()[..2]`: `pests` comes first in the root's order, so the test still proves their two textures and lets every later family add its own. Every other round 10 track with a drawn family needs the same one-line change; whichever merges second keeps the identical line.
- **"Registered Minecraft mobs can land their melee attacks their own way"** (polar-bears-live 1, `monster_ai.rs`): `type MeleeCheck = fn(&mut MonsterGoalContext, &dyn World, TargetInfo)`; `MeleeAttackGoal::tick_with(ctx, world, Option<MeleeCheck>)` is the old `tick_world` body, whose inline attack (cooldown, reach, sight, swing, `attack`, the iron golem's damage roll) runs only with `None`, and `tick_world` calls it with `None`; `MeleeAttackGoal::checked(self, check)` builds `CheckedMeleeAttackGoal { goal, check }`, which hands every `Goal` method to the goal and ticks through `tick_with(.., Some(check))`; `MeleeState` gains `pub(crate)` `ticks_until_attack()`, `reset_attack_cooldown()` (20, `adjustedTickDelay(20)` of an every-tick goal) and `is_time_to_attack()`. No field was added to `MeleeAttackGoal`, so every literal (the ten in the crate, the rabbit's and probe's, M2's dolphins) stays valid.

## Protocol

No commit of this track has changed a `ToClient`/`ToServer` message or the codec yet. `minecraft_net::PROTOCOL` is not bumped here.

## State doc changes

For the orchestrator to copy at the merge of segment m1a (rabbits done):

- `docs/MINECRAFT-SURVIVAL.md:441`: drop "rabbit" from the eggs that spawn nothing: "Spawn eggs for unsimulated mobs (fox, goat, the wither and others; ...)".
- `docs/MINECRAFT-SURVIVAL.md:451`: "The others (foxes, goats and the rest) stay dormant: ...", and under World simulation add: "Rabbits (round 10, track M1): hops with vanilla's jump control, fleeing players, wolves and monsters, panic, carrot temptation, breeding with the biome's colour one time in twenty, growth and the golden dandelion lock; saved, hatched from eggs, drawn with vanilla's adult and baby models and their 16 textures (Toast and the killer bunny's among them), hopping and tilting their heads with vanilla's keyframes, and seen by friends at the same moments. With `mob_griefing` they raid full-grown carrots, a stage a bite. The killer bunny (`RabbitType` 99) has armour 8, never flees, leaps at players and wolves within 4 blocks, bites for 8 before difficulty and calls the rabbits around it when hurt; its sounds are hostile. Wild cats hunt rabbits."
- At the merge of segment m1a (polar bears live; drawn with `polar-bears-drawn`): under World simulation in `docs/MINECRAFT-SURVIVAL.md`: "Polar bears (round 10, track M1): wander, defend their cubs (a hit cub runs and calls the adults within 20 blocks; adults attack players within 10 blocks of a cub), rear up with a growl every two seconds and swipe for 6 before difficulty, hunt foxes, panic only from fire (cubs from any hit), stay angry 20 to 39 seconds (saved with `angry_at`), follow their mothers as cubs; spawn on grass in snowy plains and ice spikes and on sea ice in frozen oceans, an adult with cubs; saved, hatched from eggs and seen by friends." Family creature rows now decide chunk-generation spawns with their own rules (`generation_rule`).

## Left for later

- The hop's dust (entity event 1, `spawnSprintParticle`): `ClientFx` has no block particles (M1-1).
- A rabbit's bite on a carrot: level event 2001 (the crumbs and the break sound) waits for foxes' `World::destroy_effect` (M1-4). The `block_change` game event for W's vibrations is one line where the rabbit's tick applies the bite, added by whichever of W and M1 merges second (rabbits-behaviour 1).
- Polar bears (M1-2): `mustReach` (ported nowhere, so a bear may take a target it cannot path to), the 0.98 water slow-down, the standing bear's taller client box and freeze immunity. The old `AngerTime` key is not read.
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

#### 3. Minecraft rabbits breed and their babies grow up

After 39d9b5bc.

- **javap this round:** `Rabbit.getBreedOffspring` (the biome's colour from `getRandomRabbitVariant` first, a level-random roll of 100 and the `#spawns_white_rabbits` / `#spawns_gold_rabbits` tests; then `nextInt(20) == 0` keeps it, else `nextBoolean` picks the partner's colour or its own), `Animal.spawnChildFromBreeding` and `finalizeSpawnChildFromBreeding` (the baby at the parent's feet with no turn; both parents age 6000 and lose their love; event 18 on the parent; an orb of `nextInt(7) + 1` from the parent's random only with `mob_drops`), `Animal.mobInteract` (food at age 0 out of love: love 600 and event 18; food for a baby: a tenth of its time left; `playEatingSound` does nothing for an `Animal`), `Animal.aiStep` (love ends at any age but 0; it counts down with six random draws every tenth tick), `#cannot_be_age_locked` (zombie horses, skeleton horses, villagers: not rabbits).
- **Changed (vendored player crate):** `World::biome_in_tag(pos, tag)` after `World::biome`, false by default (M10: M1 owns it; M2's tadpoles use it under this name).
- **Changed (vendored entities crate, `land/rabbits.rs`):**
  - `feed(world, id, inventory, hand, infinite)`: `animal::interact` with `#rabbit_food` (the world's recipe book, or the three items), so food makes an adult fall in love (event 18 for E1's hearts) and a baby grow by a tenth, and the golden dandelion locks or unlocks a baby's age; the size follows the age.
  - `breed`: the breed goal's partner is taken after the step; the baby is born as above, with a seed from `born_seeds` or the seed uniquifier (as kittens are).
  - `biome_variant(roll, white, gold)`: `getRandomRabbitVariant` (white 80 / splotched 20, gold, else brown 50 / salt 40 / black 10).
  - The tick grows the rabbit after its move (babies become 0.49 x 0.6 at age 0) and counts its love down with vanilla's random draws.
- **Changed (game code, `minecraft_terrain`):**
  - `server_mobs.rs` `MobWorld::biome_in_tag` after `biome`: `Level::biome_in_tag_at` (the level's biome and its biome tags).
  - New `families/land/rabbits.rs`: `impl TerrainFamily for Rabbit`, with tracking range 8, `VOLATILE` `MoreCarrotTicks` and `ForcedAge`, shadow 0.3, no textures yet and an empty `draw` (the models come with `rabbits-drawn`); `read` (`RabbitType`, unknown ids brown, `EVIL` 99 sets the never-flee gate; `MoreCarrotTicks`; `Age`, `ForcedAge`, `AgeLocked`, `InLove`; a friend's look tag's `Baby`; the saved yaw for its body and head), `fields` (the same keys), `show`/`show_on` (flag bit 0 while a hop runs, `values[0]` its ticks since the hop started; the head tilt joins in `rabbits-drawn`), and `interact` (`feed`, the inventory changed, the golden dandelion's use or unuse sound).
  - `families/land.rs`: `mod rabbits;` and `rabbits::Rabbit` in `terrain_group!`.
- **Check first, as it went:** the terrain registration lands here, in the live group, because D0's `every_registered_kind_is_simulated_and_summonable` compares the two registries (commit 2). From this commit rabbits are live in the game: saved and worldgen rabbits hop, eggs and `/summon` hatch them, spawners show them, friends get them. They are not drawn until `rabbits-drawn` 1, which keeps the models, the 16 textures and the lead line; segment m1a merges only with both groups in.
- **Tests added:**
  - `world::families::land::rabbits::tests`: `two_fed_rabbits_have_a_baby_that_grows_up` (love 600 and event 18 from a carrot each, the baby within 400 ticks, parents aged 6000 out of love, event 18, an orb only with `mob_drops`, the baby's size and colour, a dandelion grows it by a tenth, at age 0 it is an adult), `a_baby_takes_a_parent_s_colour_or_one_time_in_twenty_its_biome_s` (the variant table; 600 litters of a gold and a black parent in a white-rabbit biome: about one in twenty white), `a_golden_dandelion_locks_a_baby_rabbit_s_age` (locked at the baby start, persistent, no growth or feeding, unlocked again).
  - `families::land::rabbits::tests` (terrain): `a_rabbit_saves_and_loads_with_its_type_carrot_ticks_and_age` (types 0-5 and 99, an unknown type brown; the saved keys round trip), `a_friend_sees_a_rabbit_from_its_look_tag` (no volatile keys in the look; the friend's rabbit is a gold baby; the hop reaches the friend through `FamilyShow`), `a_rabbit_eats_a_carrot_from_the_hand_and_falls_in_love` (the first real-family test of D0's `interact` arm: one carrot used, love 600, event 18; not again while in love; wheat does nothing).
- **Decisions:**
  - Decision 2 as recommended: `World::biome_in_tag` keeps vanilla's one-in-twenty biome colour.
  - The family posts its own event 18 on feeding and breeding: E1's generic family call (`track-e.md:284-289`) is not in `minecraft-survival` yet. Whichever lands second keeps exactly one.
  - The terrain `interact` uses the entity world's recipe book (which the server sets) for `#rabbit_food`, as the tempt goal does, not the actor's inventory book, so one book decides.
  - The age lock is saved as `AgeLocked` (a byte, `putBoolean`) like vanilla; the older stores still do not save it (left for later in M1-1).
  - The terrain tests for saving, the look tag and feeding come here with the `TerrainFamily`; `rabbits-drawn` 1 keeps the render, texture and lead tests.
  - A friend is sent the mob's full saved tag (`MobEvent::Add`), and the look tag (without `VOLATILE` and the base volatile keys, with `Baby`) only decides when to send it again, so the friend's rabbit gets its `Age` from the host; `read` still takes `Baby` when no `Age` is there.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (451 passed), `test -p minecraftoss-player --locked --tests` with `full-suite.ps1`'s two stand-in CSV files (198 passed; the files are removed again), `test -p minecraft_terrain --locked` (1432 passed, 2 failed: the first friend test, fixed and rerun, and `sections::tests::background_full_update_matches_the_synchronous_one`, a timing test with 1 ms waits that passed when rerun alone with the rabbit, registry, egg-survey and cage tests, 7 passed; it waits on a background thread, so the laptop's load at -j 4 low priority starved it), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new files pass `rustfmt --check --edition 2021` on their staged content.
- **Test steps** (rabbits are live but not drawn yet, so each one shows as its shadow). Release build, survival, Normal, by day on flat grass. Name each screenshot by its label.
  1. Use the rabbit egg three times on grass 8 blocks away. No rabbit is drawn yet, but three small round shadows appear and move only in short hops, never sliding along the ground, with a soft jump sound for each hop. **m1-rabbits-live-1**
  2. Walk at them: within about 8 blocks the shadows hop away fast. Stand still holding a carrot: they hop up to about 2 blocks from you and stay.
  3. Aim at a shadow from close and right-click with a carrot: one carrot is used. Do the same to a second shadow. Within a few seconds the two meet and a fourth, smaller shadow appears beside them. **m1-rabbits-live-2**
  4. Right-click the small shadow with dandelions a few times: each one is used. Right-click it with the golden dandelion: it is used with its sound; again: used again with the other sound.
  5. Use the zombie egg 3 blocks from a shadow: that shadow hops away.
  6. Hit a shadow with the sword: it dies with the rabbit's death sound and drops raw rabbit, sometimes a hide.
  7. Save, quit and load: the shadows are still there and hop.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*8,minecraft:carrot*16,minecraft:dandelion*8,minecraft:golden_dandelion*4,minecraft:iron_sword,minecraft:zombie_spawn_egg*2
```

### Group rabbits-drawn

#### 0. The registry's texture test checks the pests' textures first, not the whole list

After e862fc31.

- **Why:** `rabbits-drawn` 1 gives the rabbit its 16 textures, and D0's root test `families::tests::a_registered_mob_is_tracked_textured_and_shadowed_through_its_family` asserted that `families::textures()` is exactly the silverfish and endermite textures. It now asserts the first two (`textures()[..2]`), since `pests` is the first group in the root's order. A root-file change, so it is its own commit (M10 rule 2) and listed under "Hooks".
- **Changed:** `crates/minecraft_terrain/src/families/tests.rs`, one line.
- **Tests added:** none (the D0 test keeps its checks).
- **Runs:** `test -p minecraft_terrain --locked --lib families::` (15 passed).
- **Test steps:** nothing to see in game.
- **Kit:** none.

#### 1. Minecraft rabbits are saved, hatch from eggs and are drawn like vanilla

After d7ae60a2.

- **Already in from `rabbits-live` 3** (the brief's fallback, logged there): the `TerrainFamily` with saving, loading, hatching from eggs and `/summon`, the spawner cage, friends' look tags and feeding through `interact` (`#rabbit_food`, event 18). This commit adds the drawing, the textures, the lead row and the killer bunny's and Toast's names.
- **javap this round:**
  - `AdultRabbitModel.createBodyLayer` (64 x 64): `body` (0, 23, 4) turned -0.3927 about x with the 8 x 6 x 10 box at uv (0, 0); its children `tail`, `head` (turned +0.3927, so it sits level), the head's `left_ear` (uv 32, 0) and `right_ear` (uv 26, 0), and `frontlegs` with `right_front_leg` (uv 36, 18) and `left_front_leg` (uv 44, 18), both turned +0.3927; `backlegs` (0, 23, 4) with `right_hind_leg` and `left_hind_leg` and their `right_haunch` (uv 20, 24, turned +0.3927 about y) and `left_haunch` (uv 36, 24, -0.3927). 13 parts, 9 cubes, no mirrored or inflated cube.
  - `BabyRabbitModel.createBodyLayer` (32 x 32): `body` (0, 23, 1.6) holding `body_r1` (turned -0.5236), `tail` and `tail_r1`, `head` (uv 0, 0) with `right_ear` (uv 18, 0) and `left_ear` (uv 24, 0), `frontlegs` with each front leg (turned +0.3927) and its `_r1` (turned -0.3927); `backlegs` (0, 23, 2) with each hind leg turned 3.1416 about y and its haunch (left uv 10, 17 at (1, 0, 0.5) turned -0.7854; right uv 0, 17 at (0.5, 0, -0.9) turned +0.7854). 17 parts, 9 cubes.
  - `LayerDefinitions`: both layers go in as built, with no mesh transform; `AgeableMobRenderer.submit` only swaps the model by `isBaby`, and `LivingEntityRenderer` scales only by the `SCALE` attribute, so neither model is scaled. With its ears the adult stands about 17 pixels tall.
  - `RabbitModel.setupAnim`: after the pose reset, the head's `yRot` and `xRot` are **set** (`putfield`) to the look while the idle tilt is not started, which replaces the adult head's +0.3927: a rabbit looking straight ahead holds its head at the body's tilt. Then `HOP` and `IDLE_HEAD_TILT` are applied (commit 2).
  - `RabbitRenderer`: shadow 0.3; the 16 textures `entity/rabbit/rabbit_{brown,white,black,white_splotched,gold,salt,caerbannog}` and `rabbit_toast`, each with `_baby`; `isToast` wins over the variant, and `EVIL` maps to `caerbannog`. `MobRenderer.checkMagicName(entity, "Toast")` is `getCustomName().getString().equals("Toast")` (case-sensitive). The default render type is `entityCutout` (back faces culled), and `getFlipDegrees` keeps `LivingEntityRenderer`'s 90.
  - `Rabbit.setVariant(EVIL)`: an unnamed rabbit gets `Component.translatable("entity.minecraft.killer_bunny")` as its custom name; `getLeashOffset` is (0, 0.6 x eye, 0.4 x width).
- **Changed (vendored entities crate, `land/rabbits.rs`):** `Rabbit.named` (it has any `CustomName`) and `Rabbit.toast`, the constants `TOAST` and `KILLER_BUNNY`.
- **Changed (game code, `minecraft_terrain`):**
  - New `families/land/rabbits/render.rs`: both models as part tables (name, parent, offset, rotation, cube), `part_poses` (each part's offset and turn plus a keyframe `Moved` offset and its scale, under its parent; the look in place of the head's turn), `model_root` (vanilla's flip of 90 degrees, `scale(-1, -1, 1)` and the 1.501 lift), `texture` (variant, Toast, baby) and `append_rabbits` (the skin lit at the eye, the hurt overlay).
  - `families/land/rabbits.rs`: `mod render;`, `TEXTURES` (the 16), `read` sets `named` and `toast` from `CustomName` (a string or a component's `text`), `fields` writes `CustomName: {translate: "entity.minecraft.killer_bunny"}` for an unnamed killer bunny, and `draw` puts the living rabbits into `out.culled`.
  - `leash_render.rs`: `"rabbit" => (0.6, 0.4)` after the `"iron_golem"` row (M10's anchor), so the rope ties at the neck (the default was 1.0 x eye).
- **Tests added:**
  - `families::land::rabbits::render::tests`: `an_adult_rabbit_has_thirteen_parts_on_64_and_a_baby_seventeen_on_32` (part and cube counts, texture sizes, parents before children, every bone the keyframes name, the head's pivot at (0, 15, -1.48) and level, an ear's and a haunch's pivots, the baby's front leg and its backward hind legs, the look replacing the head's turn), `every_rabbit_type_picks_its_texture_and_toast_wins`, `a_rabbit_s_lead_ties_at_its_neck` (adult and baby), `a_rabbit_is_drawn_with_its_skin_back_faces_culled` (nine cubes into `culled` only, paws at the ground and ears above the head, the gold skin, the light, a Toast baby smaller with its own texture, a removed rabbit not drawn, red while dying).
  - `families::land::rabbits::tests::a_rabbit_saves_and_loads_with_its_type_carrot_ticks_and_age` now also checks the killer bunny's name (written once, kept on the next save) and that a given name is kept and decides Toast (`"Toast"` and `{text:"Toast"}` yes, `"toast"` and `"Bob"` no).
- **Decisions:**
  - The unnamed killer bunny's `CustomName` is written by `fields` when it is saved instead of being set at load: the saved tag is the same as vanilla's, and the fork draws no mob names (M1-1, "Left for later"). `rabbits-behaviour` 2 needs no name work of its own.
  - Toast is read from the loaded `CustomName` only; name tags are not in the fork yet (M1-1, "Left for later"), so `/summon .. {CustomName:"Toast"}` is the way to get one.
  - The models are data tables with a general `part_poses`, so commit 2's keyframes apply by bone name, as `KeyframeAnimation` does.
- **Runs:** `test -p minecraft_terrain --locked` (1438 passed, 3 ignored), `test -p minecraftoss-entities --locked --lib` (451 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new file passes `rustfmt --check --edition 2021` on its staged content.
- **Test steps** (release build, survival, Normal, by day on flat grass; name each screenshot by its label). Rabbits do not play their hop or head-tilt animations yet (commit 2); they hop in a still pose.
  1. Use the rabbit egg three times on grass 8 blocks away: three small rabbits sit up with long ears and a round tail, their front paws on the ground and the body sloping up to the head. They move only in hops. **m1-rabbits-drawn-1**
  2. Walk around one: its head turns to follow you.
  3. `/summon minecraft:rabbit ~ ~ ~3 {RabbitType:1}`, then the same with 2, 3, 4 and 5: white, black, white with black patches, gold, and salt-and-pepper. `/summon minecraft:rabbit ~ ~ ~3 {RabbitType:99}`: white with red eyes. `/summon minecraft:rabbit ~ ~ ~3 {CustomName:"Toast"}`: black and white, Toast's coat. **m1-rabbits-drawn-2**
  4. Right-click two adults with a carrot each: a baby appears beside them, much smaller, with a big head, short ears and a parent's colour. **m1-rabbits-drawn-3**
  5. Right-click a rabbit with a lead: the rope ties at its neck, low at the front. Walk away: it hops after you. **m1-rabbits-drawn-4**
  6. Hit a rabbit with the sword: it flashes red, tips over sideways as it dies and drops raw rabbit.
  7. In creative, place the spawner and use a rabbit egg on it: a small rabbit turns slowly inside the cage. **m1-rabbits-drawn-5**
  8. Save, quit and load: the same rabbits with the same colours, the baby still small.
  9. Two windows (`scripts\mp-local.ps1`): the friend sees the same rabbits in the same colours, the baby and Toast, and their heads turn. **m1-rabbits-drawn-6-host**, **m1-rabbits-drawn-6-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*8,minecraft:carrot*16,minecraft:lead*2,minecraft:iron_sword,minecraft:spawner
```

#### 2. Minecraft rabbits hop and tilt their heads with the vanilla animations, for friends too

After 8afa0650.

- **javap this round:**
  - `Rabbit.<init>`: `idleAnimationTimeout = 180 + random.nextInt(40)`. `baseTick` calls `setupAnimationStates` on the client only: when `shouldPlayIdleAnimation` (the timeout at 0 or below, no leash holder, AI on) it draws a new `180 + nextInt(40)` and `start`s the tilt; else while `jumpTicks > 0` it `startIfStopped`s the hop and stops the tilt; else the timeout counts down and the hop stops. `setLeashData` stops the tilt.
  - The client's jump clock comes from entity event 1 (`handleEntityEvent`: duration 15, ticks 0) and `aiStep` counts it up as on the server.
  - `RabbitAnimation` and `BabyRabbitAnimation`: `HOP` 0.75 s, looping (rotation and position of `body`, `head`, `backlegs`, `frontlegs`, the four legs, both ears and `tail`); `IDLE_HEAD_TILT` 4 s, not looping (also a `body` scale channel). `RabbitModel.setupAnim` applies `HOP` and then `IDLE_HEAD_TILT` at `ageInTicks` (`KeyframeAnimation.apply`, speed 1), so a finished tilt holds its last keyframe until a hop stops it.
- **Changed (vendored entities crate, `land/rabbits.rs`):** `RabbitAnimations` (the hop's and the tilt's start ticks, the idle timeout and its own `LegacyRandom`) as `Rabbit.animations`, with `tick` porting `setupAnimationStates` and the leash stop. `Rabbit::spawn` seeds its random from the spawn position and draws the first timeout. The rabbit's tick runs it right after `family_base_tick` (vanilla's `baseTick`), with the jump ticks the last step left and `leash_holder`.
- **Changed (game code, `minecraft_terrain`):**
  - `families/land/rabbits.rs`: `show` sends flag bit 0 while the hop runs and bit 1 while the tilt is started, with `values[0]` and `values[1]` the ticks since each started; `show_on` sets each start to the friend's own tick count minus that. The friend no longer gets the jump clock, which nothing it draws reads. `draw` reads the clips once per frame.
  - `families/land/rabbits/render.rs`: `Clips` (`HOP` and `IDLE_HEAD_TILT` of `RabbitAnimation` for adults and `BabyRabbitAnimation` for babies, through `keyframes::shared()`), `elapsed` (ticks since the start plus the partial tick), and `append_rabbits` applies both clips by bone name (position, rotation and scale) and the look only while the tilt is not started.
- **Tests added:**
  - Entities: `world::families::land::rabbits::tests::the_tilt_waits_180_to_219_ticks_stops_on_a_hop_and_never_runs_on_a_lead` (first waits 180-219 from 400 spawn spots, both ends seen; the tilt starts the tick the wait runs out, draws a new wait and stays started past its 4 s; a real hop starts the hop clock and stops the tilt, the wait holds during the hop and runs again after it; a lead stops a running tilt and none starts on a lead or without AI; off the lead it tilts at once).
  - Terrain: `families::land::rabbits::render::tests::a_rabbit_hops_with_the_hop_keyframes_and_tilts_its_head_when_idle` (gated on `MINECRAFTOSS_ROOT`: both classes' lengths and looping, every bone they move is a part of the model, the back legs at 125 degrees five ticks into the hop and again a loop later, the head tilts and a finished tilt keeps its last pose, the baby's own hop, and drawn: the hop and the tilt change the mesh, the look moves the head, and while the tilt runs the look does not), `families::land::rabbits::tests::a_friend_sees_a_rabbit_hop_and_tilt_at_the_same_ticks` (flags and ticks since each start, the same clocks on host and friend at every partial tick, the same mesh in both windows with test clips, a stopped rabbit drawn still).
  - `a_friend_sees_a_rabbit_from_its_look_tag` keeps its look-tag checks and leaves the shown clocks to the new test.
- **Decisions:**
  - Decision 4 as recommended: the clock runs on the host on its own `LegacyRandom` (never `core.random`, whose draws stay vanilla's), and `FamilyShow` carries it, so host and friend see one pose; vanilla runs it on each client. Its seed comes from the spawn position, because `Family::spawn` does not know the mob's id yet.
  - The clock runs for every rabbit with a tick, also a dying one, as `baseTick` does; a rabbit in a spawner cage or one not ticked sits still.
  - When the jar's keyframes cannot be read (no shared library), rabbits are drawn still, and while a tilt is started the head keeps its own pose, as vanilla's model does with an empty animation.
- **Protocol:** no message or codec change; the clocks use `FamilyShow`'s existing flags and values.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (452 passed), `test -p minecraft_terrain --locked` (1440 passed, 3 ignored; the gated keyframe test ran against the 26.3 jar that `cargo-slot.ps1` points `MINECRAFTOSS_ROOT` at), `check -p launcher --locked` (green; only the old render_frame and skate warnings). No new file in this commit; the touched files still pass `rustfmt --check --edition 2021`.
- **Test steps** (release build, survival, Normal, by day on flat grass; name each screenshot by its label):
  1. Use the rabbit egg on grass 6 blocks away and watch it hop: each hop tucks the back legs and stretches the body forward with the ears swept back, and it sits up again as it lands. **m1-rabbits-anim-1**
  2. Put the carrot away and stand still a few blocks from a rabbit. Once it has not hopped for about 9 to 11 seconds it tilts its head for 4 seconds; afterwards its head stops turning to you until its next hop. **m1-rabbits-anim-2**
  3. Feed two rabbits a carrot each and watch the baby: it hops and tilts its head with its own, smaller animations. **m1-rabbits-anim-3**
  4. Put a lead on a rabbit and tie it to a fence: it never tilts its head while it is tied. Right-click the fence knot to take the lead back after 15 seconds: the rabbit tilts its head at once.
  5. Two windows (`scripts\mp-local.ps1`), side by side: the host's rabbits hop and tilt their heads at the same moments in the friend's window. **m1-rabbits-anim-4-host**, **m1-rabbits-anim-4-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*8,minecraft:carrot*8,minecraft:lead*2,minecraft:oak_fence*4
```

### Group rabbits-behaviour

#### 1. Minecraft rabbits raid carrot crops

After d008fa7e.

- **javap this round:**
  - `MoveToBlockGoal`: flags `MOVE` and `JUMP`. `canUse` counts `nextStartTick` down and returns false; at 0 it sets it to `reducedTickDelay(200 + nextInt(200))` and runs `findNearestBlock`. `canContinueToUse` is `tryTicks >= -maxStayTicks && tryTicks <= 1200 && isValidTarget`. `start` walks to the block's top (`moveTo(x + 0.5, y + 1, z + 0.5, speed)`, reach 1), sets `tryTicks` 0 and `maxStayTicks = nextInt(nextInt(1200) + 1200) + 1200`. `tick` runs every tick: within `acceptedDistance` 1.0 of the centre of the block above (`closerToCenterThan`) the block is reached and `tryTicks` falls by one, else it rises and every 40 a new walk starts. There is no `stop`. `findNearestBlock` searches the layers `k - 1` for `k` = 0, 1, -1, .. up to the vertical range, ring by ring outwards, and also asks `isWithinHome`.
  - `Rabbit$RaidGardenGoal`: `MoveToBlockGoal(0.699999988079071, 16)`, vertical range 1. `canUse`: with `nextStartTick` at 0 or less, false without `mob_griefing`, then `canRaid = false` and `wantsToRaid = moreCarrotTicks <= 0`; then the parent's. `canContinueToUse` is `canRaid` and the parent's, whose `isValidTarget` refuses once `canRaid` is set. `isValidTarget`: `#supports_crops` below, `wantsToRaid`, not `canRaid`, and a `CarrotBlock` at its `isMaxAge` above; then `canRaid = true`. `tick`: the parent's, a look at the crop (`setLookAt(x + 0.5, y + 1, z + 0.5, 10, maxHeadXRot)`), and once reached, with `canRaid` and a carrot above: age 0 → `setBlock(air, 2)` then `destroyBlock` (nothing left to break), else age − 1 with `setBlock(.., 2)`, game event `block_change` and level event 2001 with the old state; `moreCarrotTicks = 40` in both cases. Then, reached, always `canRaid = false` and `nextStartTick = 10`.
  - `Rabbit.registerGoals` puts it at priority 5, between the avoid goals and the stroll. `wantsMoreFood` is `moreCarrotTicks <= 0`; `customServerAiStep` takes `nextInt(3)` off `moreCarrotTicks` each tick while above 0 (in since `rabbits-live`).
  - `PathNavigation.createPath` makes no path while the mob is off the ground (`canUpdatePath`), so the first look of a rabbit that has just spawned in the air only starts the goal.
- **Changed (vendored entities crate):**
  - `land/animal_goals.rs`: `MoveToBlockGoal` (see "Hooks") and its constants `MOVE_TO_BLOCK_INTERVAL` 200, `MOVE_TO_BLOCK_GIVE_UP_TICKS` 1200, `MOVE_TO_BLOCK_STAY_TICKS` 1200, `MOVE_TO_BLOCK_RECALCULATE` 40 and `MOVE_TO_BLOCK_ACCEPTED_DISTANCE` 1.0.
  - `land/rabbits/ai.rs`: `RaidGardenGoal`, a `MoveToBlockGoal` at 0.7, range 16 and vertical range 1 with `wants_to_raid` and `can_raid` as vanilla keeps them, registered at priority 5 and named in `GOAL_NAMES`. Also `is_garden_target` (`#supports_crops` with a ripe carrot above), `ripe_carrot` and `nibbled` (the carrot one stage younger, or air from age 0), `RabbitGoals.nibble` (the bite the goal asks for) and the constants `RAID_SPEED`, `RAID_RANGE`, `RAID_VERTICAL_RANGE`, `RAID_LOOK_Y` 10, `RAID_NEXT_START` 10, `MORE_CARROT_TICKS` 40, `SUPPORTS_CROPS` and `CARROTS`.
  - `land/rabbits.rs`: the tick takes the bite out of `RabbitGoals` right after the AI step and sets the block (`World::set_block`), as the sheep's grass eating does.
- **Tests added:**
  - `world::families::land::animal_goals::tests::move_to_block_searches_in_vanilla_order_and_stops_within_reach`:
    - the search order: the layer below first, then the feet's layer, then two below; nearer rings first; `+z` before `-z` and `+x` before `-x`; nothing past the range;
    - the 100-200 tick wait between looks (200 seeds) and its countdown;
    - `start` (`try_ticks` 0, a stay of 1200-3599 ticks, a walk), `tick` (counting up, a new walk at 40, reached within a block of the centre above and counting down) and `can_continue`'s bounds.
  - `world::families::land::rabbits::tests::a_rabbit_walks_to_a_ripe_carrot_and_bites_it_down_a_stage`:
    - once on the grass it looks at once; the goal stops at its next full update while its walk goes on up to the carrot, and the first look bites nothing;
    - a rabbit beside a ripe carrot bites it at once (7 → 6, 38-40 carrot ticks left after the tick), leaves the unripe carrot alone for 1000 ticks and bites it again once it is ripe;
    - `nibbled` for ages 7 to 1, air from 0; and on the goal itself, a carrot at age 0 by the bite is eaten up (the crop and air in `nibble`, 40 carrot ticks, and the goal ends).
  - `world::families::land::rabbits::tests::no_rabbit_touches_crops_without_mob_griefing`: 1200 ticks beside a ripe carrot without the goal starting; with `mob_griefing` back it bites within three ticks.
- **Decisions:**
  - Vanilla only aims at full-grown carrots (`isMaxAge`), so a carrot loses one stage each time it ripens. It becomes air only when it is at age 0 by the time the rabbit bites (it changed between the look and the bite). The brief's test name `a_rabbit_walks_to_a_ripe_carrot_and_nibbles_it_down_to_nothing` and its owner step 6 ("each bite shrinks it a stage until it is gone") do not match 26.3. The test is `a_rabbit_walks_to_a_ripe_carrot_and_bites_it_down_a_stage`, the age-0 branch is tested on the goal, and the owner's steps below say what vanilla does.
  - The early stop is ported as is (M1-1): the goal ends at its next full update, the walk it started goes on, and a later look beside the crop bites at once.
  - The bite is a request the goal leaves in `RabbitGoals.nibble`, since goals only see `&dyn World`. The rabbit's tick applies it right after the AI step, before anything else moves. The server's `set_block` updates the neighbours (flag 3) where vanilla uses flag 2, which changes nothing for a crop on farmland.
  - `MoveToBlockGoal` is a helper that a goal holds, not a goal itself, so each user names its own valid block without boxing a closure. `isWithinHome` is not ported: no goal-framework mob in the fork has a home restriction.
  - Level event 2001 and the `block_change` game event: "Left for later".
- **Runs:** `test -p minecraftoss-entities --locked --lib` (455 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). `animal_goals.rs`, `rabbits.rs`, `rabbits/ai.rs` and `rabbits/tests.rs` still pass `rustfmt --check --edition 2021`.
- **Test steps** (release build, survival, Normal, by day on flat grass; `mob_griefing` is on by default; name each screenshot by its label):
  1. Hoe a 5 x 5 patch of grass beside water (pour the water bucket into a hole next to it), plant a carrot on every block and bone-meal them until every carrot is full grown (the orange tops show). Fence the patch in, leaving one block of grass inside the fence on every side.
  2. Use the rabbit egg three times inside the fence, then step out and stand more than 8 blocks away (rabbits flee a closer player). Within one to three minutes a rabbit hops onto a full-grown carrot, looks down at it, and that carrot drops to the stage before (the orange top disappears). **m1-rabbits-raid-1**
  3. Keep watching: the rabbits bite other full-grown carrots, but never a carrot that is not full grown. A bitten carrot is bitten again only after it has grown back (bone-meal it to full and wait). **m1-rabbits-raid-2**
  4. `/gamerule mob_griefing false`, bone-meal every carrot to full and wait three minutes: no carrot is bitten. `/gamerule mob_griefing true`: the bites start again.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*8,minecraft:carrot*32,minecraft:bone_meal*64,minecraft:iron_hoe,minecraft:water_bucket,minecraft:oak_fence*64
```

#### 2. The killer bunny bites players and wolves

After ed19a463.

- **javap this round:**
  - `Rabbit.setVariant(EVIL)`: armour base 8; goal 4 `MeleeAttackGoal(1.4, true)`; targets 1 `HurtByTargetGoal().setAlertOthers()`, 2 `NearestAttackableTargetGoal(Player, true)` and 2 `NearestAttackableTargetGoal(Wolf, true)` (each `randomInterval` 10, so one roll in five after `reducedTickDelay`); the attack modifier `minecraft:evil` +5 (`ADD_VALUE`) on the base 3; `CustomName` = `entity.minecraft.killer_bunny` when it has none. Another variant only drops the modifier: the goals stay once added.
  - `Rabbit.customServerAiStep` on the ground, after the landing check: `EVIL`, no jump delay and a target with `distanceToSqr < 16`: `facePoint(target)`, `moveControl.setWantedPosition(target, moveControl.getSpeedModifier())` (the rabbit's own, so 1.5 in water) and `startJumping`. The hop check after it does not look at the jump control's `startJumping` flag, sees the wanted position just set and starts another hop (a second jump sound) facing the path's next point, or the target.
  - `Mob.doHurtTarget`: the attack damage on the target; when `hurtServer` is true, the knockback (the rabbit has no attack knockback), `setLastHurtMob` and `playAttackSound`. `Rabbit.playAttackSound`: `EVIL` only, `entity.rabbit.attack` at volume 1 and pitch `(r1 - r2) * 0.2 + 1`. `getSoundSource`: `HOSTILE` for `EVIL`, else `NEUTRAL`.
  - `PanicGoal.shouldPanic` (`getLastDamageSource` in `#panic_causes`) has no variant check, so a hurt killer bunny also panics for a moment; `RabbitAvoidEntityGoal` refuses for `EVIL` (in since `rabbits-live`).
  - `Rabbit` is an `Animal`, not an `Enemy`: iron golems leave the killer bunny alone, and it gives a rabbit's experience and loot.
- **Changed (vendored entities crate):**
  - `land/rabbits/ai.rs`: `Evil<G>`, a wrapper whose `can_start` asks `evil` before the goal's own (everything else is the goal's), around `MeleeAttackGoal` (1.4, following an unseen target) added last to the goals at priority 4, and around `HurtByTargetGoal` (alerting others) at 1 and `NearestTargetGoal` for players and for `Prey::Kinds(&["minecraft:wolf"])` at 2 (must see, one roll in five), the rabbit's first target goals. `GOAL_NAMES` gains `MeleeAttackGoal` and `TARGET_NAMES` the three. `evil(ctx)` is shared with the avoid goals. `custom_step` leaps: evil, no delay, a target within 4 blocks → face it, aim the move control at it with its speed and hop. Constants `EVIL_ATTACK_SPEED` 1.4, `LEAP_DISTANCE` 4, `WOLF`.
  - `land/rabbits.rs`: `Family::armor` 8 for the killer bunny and the rabbit's `hurt` takes it off unless the hurt bypasses armour; `Family::voice` in `hostile_volume` for the killer bunny; `Rabbit::evil` and `attack_damage` (3, or 8); the tick strikes what the melee goal hit through `family_strike` (`PlayerHitKind::Melee`, no lift or throw) with the attack sound when it lands, and passes the hurt-by alert to `family_alert`. Constants `ATTACK_DAMAGE` 3, `EVIL_ATTACK_BONUS` 5, `EVIL_ARMOR` 8, `ATTACK_SOUND`.
- **Tests added** (`world::families::land::rabbits::tests`):
  - `a_killer_bunny_has_eight_armour_and_bites_for_eight`: the attack damage of a plain rabbit and of the killer bunny, its hostile voice, still a creature and no enemy; a bite of 2 takes 1.44 through armour 8 and the full 2 from a plain rabbit; next to a player it never flees, targets the player and bites for 8 with its own id, the attack sound in `hostile_volume` at pitch 0.8-1.2 and its jump sounds hostile too; the player crate's `Player::hurt_by` takes 5, 8 and 12 from that bite on Easy, Normal and Hard.
  - `a_killer_bunny_leaps_at_a_target_within_four_blocks`: 3 blocks off it aims the move control at the target, faces it, hops with 1.4 kept as its next speed and plays the jump sound twice; 4.6 blocks off, with a jump delay left, or as a plain rabbit it does not leap.
  - `a_killer_bunny_attacks_a_wolf_and_its_kin_join_in`: a killer bunny bitten by a wolf targets it and the other killer bunny, 20 blocks from the wolf but inside the alert box, takes the wolf as its target; the wolf loses 8 to a bite with the hostile attack sound; a plain rabbit beside them never runs the melee goal.
  - `a_rabbit_is_small_with_three_health_and_its_kind` now checks the three target goal names.
- **Decisions:**
  - Decision 5 as recommended: one spec, with the melee goal and the three target goals registered last in their selectors and gated on `evil`, as vanilla's later `setVariant` adds them. The gate is asked on starting, so a rabbit that stopped being evil would stop attacking and lose its armour, where vanilla keeps both; nothing in the fork changes a living rabbit's variant.
  - The killer bunny keeps vanilla's panic when hurt: after a hit it runs off for a moment before it comes back. It never flees players, wolves or monsters.
  - The leap plays the jump sound twice, as vanilla does.
  - The attack sound plays when a bite on a mob lands and on every bite on a player, since the player side decides later whether the bite hurt (the iron golem does the same).
  - The alert sets the attacker on every rabbit in the box that has no target, killer or not, as vanilla's `alertOthers` asks for the class `Rabbit`; a plain rabbit does nothing with a target.
  - The bite is 8 on the entity side; the player side's difficulty scaling makes it 5, 8 or 12 (the test checks it through the player crate).
  - The killer bunny's `CustomName` was already written by `fields` in `rabbits-drawn` 1, with its test (written only when unnamed, kept on the next save); nothing new here.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (458 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). `rabbits.rs`, `rabbits/ai.rs` and `rabbits/tests.rs` still pass `rustfmt --check --edition 2021`.
- **Test steps** (release build, survival, Normal, by day on flat grass; name each screenshot by its label):
  1. Stand 8 blocks from where you summon: `/summon minecraft:rabbit ~ ~ ~8 {RabbitType:99}`. A white rabbit with red eyes hops straight at you, never away, and within about 4 blocks leaps at you. Each bite takes 4 hearts (2.5 on Easy, 6 on Hard, less in the iron armour). **m1-killer-1**
  2. Take the armour off and let it bite once on Normal: exactly 4 hearts go. Its hops, hurt and bite sounds follow the Hostile Creatures slider in the sound options, not the Friendly Creatures one.
  3. Punch it without jumping, about a second apart: it needs five punches, where a plain rabbit dies on the third. After each punch it runs off for a moment, then comes back. **m1-killer-2**
  4. Summon another killer bunny 8 blocks away and use the wolf egg 3 blocks from it: the bunny goes for the wolf, and its first bite kills the wild wolf. **m1-killer-3**
  5. Summon a killer bunny and a plain rabbit side by side, walk 20 blocks away and shoot the killer bunny with the bow: it comes for you; the plain rabbit never attacks.
  6. Save, quit and load with a killer bunny alive: it is still white with red eyes and still attacks.
  7. Two windows (`scripts\mp-local.ps1`): the host summons a killer bunny next to the friend; it goes for the friend, and the friend hears it. **m1-killer-4-host**, **m1-killer-4-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:wolf_spawn_egg*2,minecraft:bow,minecraft:arrow*32,minecraft:iron_sword,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:cooked_beef*16
```

#### 3. Minecraft cats hunt rabbits

After ee0a08da.

- **javap this round:** `Cat.registerGoals` adds the target goals `1 NonTameRandomTargetGoal(Rabbit, false, null)` and then `1 NonTameRandomTargetGoal(Turtle, false, Turtle.BABY_ON_LAND_SELECTOR)`. `NonTameRandomTargetGoal` is a `NearestAttackableTargetGoal` (random interval 10, so one roll in five after `reducedTickDelay`, no sight needed to keep it) that only starts while the cat is not tame.
- **Changed (vendored entities crate):** `feline_ai.rs` `cat_targets`: the first slot, `Prey::Absent` until now, is `Prey::Kinds(&["minecraft:rabbit"])` with its `wild_only` kept (M10's anchor `feline_ai.rs:196`). The turtle slot after it is left for `turtles-hunted`. The cat's attack goal, leap and bite were already there for the ocelot's chickens.
- **Tests added:** `world::families::land::rabbits::tests::a_wild_cat_hunts_a_rabbit_and_a_tame_cat_does_not` (a wild cat 6 blocks from a rabbit takes it as its target and bites it; a tame one in the same place never targets it in 600 ticks).
- **Decisions:**
  - The test is in the rabbit's own `rabbits/tests.rs`, not in `world/felines.rs` as M1-1 names it: D0-3 and M10 rule 8 put a family's tests in its own files, and this keeps the shared `felines.rs` to the one line in `feline_ai.rs` (M3 also touches the cats).
  - Row 15 of `docs/round10/mobs.md` is `done` with this commit: the rabbit is live, drawn and has its vanilla behaviours, its natural spawns were already vanilla, and the owner's steps are in this log. What is still missing is in its "Left" column and under "Left for later".
- **Runs:** `test -p minecraftoss-entities --locked --lib` (459 passed), `test -p minecraft_terrain --locked` (1440 passed, 3 ignored; nothing of the terrain changed in this group, run once as a check), `check -p launcher --locked` (green; only the old render_frame and skate warnings). `rabbits/tests.rs` still passes `rustfmt --check --edition 2021`.
- **Test steps** (release build, survival, Normal, by day on flat grass; name each screenshot by its label):
  1. Use the rabbit egg twice, then the cat egg 6 blocks from the rabbits, and walk 20 blocks away (a wild cat runs from a player within 16 blocks). Within a minute the cat creeps up on a rabbit, pounces and kills it with a bite. **m1-cats-rabbits-1**
  2. Use another cat egg, tame it with raw cod (feed it until hearts show) and tell it to stand (right-click it so it is not sitting). Use the rabbit egg twice next to it and watch for a minute: the tame cat never goes for the rabbits. **m1-cats-rabbits-2**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*8,minecraft:cat_spawn_egg*4,minecraft:cod*16
```

#### Group rabbits-behaviour: all steps together

The owner's full rabbit test after this group (M1-1, "Owner's steps", with step 6 as vanilla has it), on a release build with all of segment m1a: survival, Normal, by day on flat grass. The earlier entries above have the detail and the screenshot names.

1. Use the rabbit egg three times on grass 8 blocks away: small rabbits that move only in hops, with a soft jump sound. Each hop tucks the back legs and stretches the body. **rabbit1_hop**
2. Walk at them: within about 8 blocks they flee in quick hops. Hold a carrot and stand still: they come to about 2 blocks from you.
3. Feed two adults a carrot each: a baby appears, smaller, with a big head and its own texture. Feed the baby dandelions: it grows faster. **rabbit3_baby**
4. Stand near one for about 10 s: it tilts its head for 4 s, then stops turning its head to you until its next hop.
5. `/summon minecraft:rabbit ~ ~ ~3 {RabbitType:1}`, then 2, 3, 4 and 5: white, black, white with black patches, gold, salt-and-pepper. `/summon minecraft:rabbit ~ ~ ~3 {CustomName:"Toast"}`: Toast. **rabbit5_colours**
6. Hoe a 5 x 5 patch by water, plant carrots, bone-meal them full and fence three rabbits in; stand more than 8 blocks away. Within a few minutes a rabbit hops onto a full-grown carrot and bites it down one stage; it bites only full-grown carrots, so a bitten one waits until it is full grown again. **rabbit6_carrots** Then `/gamerule mob_griefing false` and bone-meal them full: no more bites. `/gamerule mob_griefing true`.
7. `/summon minecraft:rabbit ~ ~ ~6 {RabbitType:99}`: a white rabbit with red eyes leaps at you from 4 blocks and bites for 4 hearts (less in iron armour). It never flees, only runs off for a moment after you hit it. Use a wolf egg 5 blocks away: the two fight. **rabbit7_killer_bunny**
8. A wolf egg near two rabbits: the wolf kills one. A cat egg near two rabbits, with you 20 blocks away: the cat stalks and pounces on one. A zombie egg 3 blocks from a rabbit: the rabbit flees.
9. Kill rabbits with the sword: raw rabbit, often a hide, now and then a rabbit's foot. Set the grass by one on fire: if it dies burning, it drops cooked rabbit. Shoot one with the bow and one with the M4A1: each flashes red and dies.
10. Lead a rabbit: it hops after you, roped at the neck.
11. Leave an adult, a baby and the killer bunny alive, save, quit and load: the same three are there and the bunny still attacks. In a world from an older build, the rabbits that stood frozen now hop.
12. `/locate biome minecraft:snowy_plains`, `/tp` there and walk into new land: groups of white or patched rabbits, one adult with babies of its colour. In a desert they are gold. **rabbit12_snowy**
13. In creative, place the spawner and use a rabbit egg on it: a small rabbit turns in the cage and rabbits spawn around it.
14. Two windows (`scripts\mp-local.ps1`). The host hatches three rabbits and a killer bunny. The friend sees the same rabbits, colours, hops and head tilts at the same moments; the rabbits come to the friend's carrot; the killer bunny goes for the friend too; the friend shoots a rabbit with the M4A1 and it dies in both windows. **rabbit14-host**, **rabbit14-friend**

The kit for all of it (M1-1's, 25 stacks; the friend's `.env` needs the same line):

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*16,minecraft:carrot*64,minecraft:golden_carrot*16,minecraft:dandelion*16,minecraft:iron_hoe,minecraft:bone_meal*64,minecraft:water_bucket,minecraft:oak_fence*64,minecraft:wolf_spawn_egg*2,minecraft:cat_spawn_egg*2,minecraft:zombie_spawn_egg*2,minecraft:lead*4,minecraft:iron_sword,minecraft:shield,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:spawner,minecraft:cooked_beef*32,minecraft:bow,minecraft:arrow*64,minecraft:grass_block*64,minecraft:flint_and_steel,minecraft:lava_bucket
```

### Group polar-bears-live

#### 1. Registered Minecraft mobs can land their melee attacks their own way

After 9bfbc35e.

- **javap this round** (`MeleeAttackGoal`, `PolarBear$PolarBearMeleeAttackGoal`): `MeleeAttackGoal.tick` counts `ticksUntilNextAttack` down and calls `checkAndPerformAttack(target)` every tick it has a target; the default is `canPerformAttack` (`isTimeToAttack`, `isWithinMeleeAttackRange`, `hasLineOfSight`), then `resetAttackCooldown` (`adjustedTickDelay(20)` = 20), `swingForAttack` and `doHurtTarget`. The polar bear overrides `checkAndPerformAttack` and `stop` only, and reads `getTicksUntilNextAttack`, `isTimeToAttack` and `resetAttackCooldown`; its check never calls `swingForAttack`.
- **Changed (vendored entities crate, `monster_ai.rs`):** the hook (see "Hooks"). The body of the old `tick_world` keeps its lines; it moved under `impl MeleeAttackGoal` as `tick_with`, and the attack's `if` became the `else` of `if let Some(check) = check`.
- **Tests added:** `world::families::tests::a_melee_goal_with_its_own_check_lands_hits_through_it` (goal level, on the probe's context with a player in reach: a checked goal has the plain goal's controls, ticks every tick and is interruptible; its check gets the target every tick and the cooldown 0, 19, 18, .. and lands hits on ticks 0, 20 and 40 through `attack`, the goal's own swing never runs; `stop` still stands it down; a check that does nothing never hits; the plain goal still hits on ticks 0, 20 and 40 and swings three times). The melee tests of D0 (`a_registered_monster_runs_its_goals_and_strikes_a_player`) are unchanged and pass.
- **Decisions:** decision 1 as recommended (the wrapper over `tick_with`). Until commit 2 uses them, `checked`, `CheckedMeleeAttackGoal` and the three accessors are only used by the test, so this commit alone builds with three dead-code warnings in `minecraftoss-entities`; commit 2 clears them.
- **Runs:** `test -p minecraftoss-entities --locked --lib families::tests` (18 passed), `check -p launcher --locked` (green; the old render_frame and skate warnings and the three dead-code warnings above).
- **Test steps:** nothing new to see; registered mobs bite as before. Release build, survival, Normal, by night in a stone cave: use the silverfish egg 6 blocks away; it crawls to you and bites about once a second. **m1-melee-hook-1**
- **Kit:** none new (D0's kit line has the silverfish egg).

#### 2. Minecraft polar bears live, defend their cubs and stand up to strike

After 4d6f6a32.

- **javap this round** (`PolarBear` and its three inner goals, `NeutralMob`, `FollowParentGoal`, `Animal.checkAnimalSpawnRules`; the 26.3 tags `#panic_causes`, `#panic_environmental_causes`, `#polar_bear_immune_to`, `#freeze_immune_entity_types`): the M1-2 "Vanilla 26.3" numbers hold. Added on the way:
  - `registerGoals`: 0 `FloatGoal`, 1 `PolarBearMeleeAttackGoal` (`MeleeAttackGoal(1.25, true)`), 1 `PanicGoal(2.0, isBaby ? #panic_causes : #panic_environmental_causes)`, 4 `FollowParentGoal(1.25)`, 5 `RandomStrollGoal(1.0)` (interval 120), 6 `LookAtPlayerGoal(Player, 6)`, 7 `RandomLookAroundGoal`; targets 1 `PolarBearHurtByTargetGoal` (no ignored classes, no `setAlertOthers`), 2 `PolarBearAttackPlayersGoal` (`NearestAttackableTargetGoal(Player, 20, true, true, null)`), 3 `NearestAttackableTargetGoal(Player, 10, true, false, this::isAngryAt)`, 4 `NearestAttackableTargetGoal(Fox, 10, true, true, (t, l) -> !isBaby())`, 5 `ResetUniversalAngerTargetGoal(false)`. The intervals become one roll in 10, 5 and 5 (`reducedTickDelay`); the cub test of goal 2 runs before its roll, the fox selector after it.
  - `PolarBearMeleeAttackGoal.checkAndPerformAttack`: `canPerformAttack` → `resetAttackCooldown`, `doHurtTarget`, stand down (no `swingForAttack`); else within `(targetWidth + 3)²` of the target's feet: when `isTimeToAttack`, stand down and reset; then with `getTicksUntilNextAttack() <= 10`, stand up and `playWarningSound`; else reset and stand down. `stop` stands down first.
  - `playWarningSound`: `makeSound(entity.polar_bear.warning)` (volume 1, `getVoicePitch`) when `warningSoundTicks <= 0`, then 40. `tick`: after `super.tick()` the client steps the stand clock, then `warningSoundTicks--` while above 0 (both sides), then `updatePersistentAnger(level, true)` on the server.
  - `PolarBearHurtByTargetGoal.start`: `super.start()`, then for a cub `alertOthers()` and `stop()`; `alertOther` only reaches an adult `PolarBear`. `alertOthers` searches `unitCubeFromLowerCorner(position).inflate(followRange, 10, followRange)` for its own class with no target.
  - `PolarBearAttackPlayersGoal.canUse`: false for a cub (no roll); else `super.canUse()` and any `PolarBear` cub in its box inflated by (8, 4, 8). `getFollowDistance` is half the follow range (10), for the search and for keeping the target.
  - `NeutralMob`: saves `anger_end_time` (long, game time) and `angry_at` (`EntityReference`, the UUID as four ints, left out when null); reading takes `anger_end_time` (or the old `AngerTime`) and `angry_at`, and on a server level sets the target to the entity if it is there. `isAngryAt`: `canAttack`, then universal anger (off), then the saved target's UUID. `stopBeingAngry` clears the last hurt-by mob, the anger target, the target and the end time (-1). `PERSISTENT_ANGER_TIME = TimeUtil.rangeOfSeconds(20, 39)` (400-780 ticks).
  - `BABY_DIMENSIONS = scalable(0.7, 0.7).withEyeHeight(0.34375)`; the adult's eye is the default `1.4 * 0.85`. `getAmbientSound`: `.ambient_baby` for a cub; `playStepSound`: `entity.polar_bear.step` at 0.15. `isFood` is false, so nothing makes one fall in love; a cub still takes the golden dandelion (`Animal.mobInteract`).
- **Changed (vendored entities crate):**
  - New `world/families/land/polar_bears.rs`: `PolarBear` (age, love, last damage, `stand` and `stand_old` (the client's stand clock, 0 to 6, stepped on the host), `angry_at` (the saved UUID), `MonsterAi`), `set_baby` (0.7 x 0.7, eye 0.34375), `interact` (`animal::interact` with no food: only the golden dandelion lock), and `impl Family`: key 55, kind `minecraft:polar_bear`, 1.4 x 1.4, eye `1.4 * 0.85`, health 30, speed 0.25, follow range 20, ambient interval 120, category `creature`, leashable (walked by its navigation), panicking while its panic goal runs, step sound `entity.polar_bear.step`. Its `hurt` ends its love and keeps the damage type for the 40-tick panic window.
  - The tick: `family_base_tick`; a cub's `Voice::Ambient` becomes `entity.polar_bear.ambient_baby` (vanilla's roll and pitch kept); a saved `angry_at` becomes the anger target once that player (or mob) is in the level; the goals get its age, its id, the last damage and its family as `kin`; `family_goal_step`; the swipe through `family_strike` (`PlayerHitKind::Melee`, 6); a hit cub's alert reaches every adult polar bear with no target within 20 across and 10 up or down; the warning voice; `family_after_move`; growing up; the stand clock (old = now, then one up while standing or one down, 0 to 6); the warning countdown; then `update_enderman_anger` (`updatePersistentAnger`) with its own random and the hurt-by it clears, and `angry_at` follows the anger target's UUID (`uuid_of`): kept while a saved target not yet found is still angry by time, dropped when the anger ends.
  - New `world/families/land/polar_bears/ai.rs`: `SPEC` and `PolarBearGoals` (animal goal state, `baby`, `standing`, `warning_ticks`, the warning's pitch). `PolarBearMeleeAttackGoal` is `MeleeAttackGoal { 1.25, follows unseen }.checked(check_and_perform_attack)` (the hook of commit 1) with the stand-down on `stop`; `check_and_perform_attack` is vanilla's check and draws the warning's voice pitch from the bear's random at the moment it growls. `PolarBearPanicGoal` takes `feline_ai::panic_cause` (`#panic_causes`) for a cub and `panic_environmental_cause` for an adult when it starts. Then `PolarBearHurtByTargetGoal`, `PolarBearAttackPlayersGoal` (the cub test over `ctx.mobs`, the follow range halved around its calls), `NearestTargetGoal` for `Prey::AngryPlayer`, `Adult(NearestTargetGoal(Prey::Kinds([fox])))` and a `NeverGoal` for the universal-anger reset. The walk keeps the animal fire maluses and `WalkTarget::Animal`.
  - `land/animal_goals.rs`: `FollowParentGoal` and `panic_environmental_cause` (see "Hooks"). `land.rs`: `pub mod polar_bears;` and `polar_bears: polar_bears::PolarBear` in `family_group!`.
- **Changed (game code, `minecraft_terrain`):** new `families/land/polar_bears.rs`, `impl TerrainFamily for PolarBear`: tracking range 10, `VOLATILE` `angry_at` and `ForcedAge`, shadow 0.9, `TEXTURES` `entity/bear/polarbear` and `polarbear_baby`, an empty `draw` (the models come with `polar-bears-drawn`); `read` (`Age`, `ForcedAge`, `AgeLocked`, `InLove`, a friend's `Baby`, `anger_end_time` (-1 when missing), `angry_at`, the saved yaw), `fields` (the same keys, `angry_at` only when there is one), `show` (flag bit 0 while standing, `values[0]` the stand clock and `values[1]` its last value), `show_on` (the same back) and `interact` (the golden dandelion's use and unuse sounds). `families/land.rs`: `mod polar_bears;` and `polar_bears::PolarBear` in `terrain_group!`.
- **Tests added:**
  - Entities (`world::families::land::polar_bears::tests`):
    - `a_polar_bear_is_large_with_thirty_health_and_its_kind`: sizes and eyes of adult and cub, health, interval, census, candidates, traits, voice, step sound, goals and targets, follow range, floating, walk target, the table and 1-3 experience for adults only.
    - `a_hit_bear_stands_warns_every_two_seconds_and_strikes_for_six`: at goal level with the bear held still, in reach it swipes every 20 ticks, stands for the 10 ticks before each swipe and growls once as it first stands; 3 blocks off it never swipes, stands half the time and growls once; 5 blocks off it does nothing. In the world, punched, it comes and swipes for 6 as `Melee` at least a second apart, always from standing; it growls in `neutral_volume` at pitch 0.8-1.2, at least 40 ticks apart and exactly 40 apart while it keeps striking; its stand clock steps 0-6 one at a time.
    - `a_hit_cub_runs_and_nearby_adults_attack`: the cub panics, runs and drops its own target; the adult 6 blocks off takes the player and gets angry; the adult 30 blocks off and the other cub do not.
    - `an_adult_with_a_cub_near_attacks_players_within_ten`: with a cub 4 blocks off, a player 8 blocks off is hunted through `PolarBearAttackPlayersGoal`; not at 11 blocks, not with the cub 10 blocks off, not without a cub; cubs never go for players.
    - `an_adult_hunts_foxes_and_a_cub_does_not`: at goal level with a fox and a rabbit candidate, the adult takes the fox and starts its attack; the cub takes nothing.
    - `its_anger_lasts_twenty_to_thirty_nine_seconds_and_ends_for_creative_players`: over 60 seeds the anger runs 400-780 ticks past the tick it took the target (both ends seen) and `angry_at` is the player's UUID; creative ends it with the target and the UUID; with the player 40 blocks away it lasts to its end time, then the UUID goes too.
    - `an_adult_panics_only_from_fire_and_a_cub_from_any_hit`: adults not from a player, a mob or an arrow, but from fire, burning and lava; cubs from a player, a mob, an arrow and fire, not from a fall or drowning; at speed 2.0.
    - `a_cub_follows_an_adult_and_sounds_like_a_cub`: the cub 7 blocks from a still adult runs `FollowParentGoal` and ends within 5 blocks of it; its ambient sounds are `ambient_baby` at pitch 1.3-1.7, the adult's the plain `ambient`.
  - Terrain (`families::land::polar_bears::tests`):
    - `a_polar_bear_saves_and_loads_with_its_anger_and_age`: the keys round trip, a cub's size, the yaw; a calm bear saves `anger_end_time` -1 and no `angry_at`.
    - `a_friend_sees_a_polar_bear_stand_up_from_its_look_tag`: no volatile key in the look, `Baby` in it, the friend's cub, tracking range 10, the standing flag and both clock values through `FamilyShow` and back to still.
    - `a_golden_dandelion_locks_a_cub_s_age_and_nothing_feeds_a_polar_bear`: cod, salmon and wheat do nothing; the dandelion locks the cub's age with its sound, makes it persistent and posts no hearts.
- **Decisions:**
  - Decision 1 (the wrapper) and decision 2 (`stand` and `stand_old` stepped on the host and sent in `FamilyShow`) as recommended.
  - `angry_at` is kept as a UUID in the bear's state, because `TerrainFamily::fields` sees only the mob. The tick writes the anger target's UUID (`EntityWorld::uuid_of`, a player's through `PLAYER_TARGET`) and turns a loaded UUID back into the anger target once that player or mob is in the level, as vanilla's `isAngryAt` matches the saved reference. A loaded target is not made the bear's target at load (the player may not be in the level yet); goal 3 finds the player again. The old `AngerTime` key is not read: no saved polar bear of the fork has it.
  - The stand clock is the host's, 0 to 6 in whole steps (vanilla's floats only ever take whole values), so `FamilyShow` carries it as two `i32`.
  - The warning's pitch is drawn from the bear's own random inside the goal, where vanilla's `makeSound` draws it; the voice is pushed after the step, at the bear's new position.
  - `VOLATILE` also has `ForcedAge`, which counts down while a fed cub grows, as the rabbit's does.
  - The swipe goes through `family_strike`, which counts a swing; the bear's model has no swing, so nothing shows it.
  - The cub's `ambient_baby` is a rewrite of D0's ambient voice in the bear's own tick (M10 rule 6); hurt and death keep the cub's higher pitch through `hurt_plain`.
  - Commit 1's three dead-code warnings are gone with this commit.
  - Not ported (M1-2 "Left for later"): `mustReach`, the 0.98 water slow-down, the standing bear's client box, freeze immunity.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (468 passed), `test -p minecraft_terrain --locked` (1443 passed, 3 ignored), `check -p launcher --locked` (green; only the old render_frame and skate warnings). Both runs and the launcher check had the world-crate changes of the next two commits in the tree as well (the slots were busy, so each crate was built once for the three commits). The new files pass `rustfmt --check --edition 2021` on their staged content.
- **Test steps** (polar bears are live but not drawn yet, so each shows as its round shadow). Release build, survival, Normal, by day on flat grass. Name each screenshot by its label.
  1. Use the polar bear egg 8 blocks away: a big round shadow wanders about and leaves you alone. **m1-polar-live-1**
  2. Punch it once: the shadow comes at you. Close up you hear a growl about every 2 seconds, and it hits you for 3 hearts about once a second. Raise the shield: the hits land on the shield. Kill it with the iron sword: 0 to 2 raw cod or salmon, and experience. **m1-polar-live-2**
  3. `/summon minecraft:polar_bear ~ ~ ~12` and `/summon minecraft:polar_bear ~2 ~ ~12 {Age:-24000}`: a big shadow and a small one that stays near it and growls higher. Walk to within 10 blocks of the big one: it comes at you without being hit. **m1-polar-live-3**
  4. Summon another pair 20 blocks away and shoot only the small one with an arrow: the small shadow runs off fast, and every big one within 20 blocks of it comes for you.
  5. Make a bear angry, run 30 blocks away and wait 40 seconds, then walk back to 15 blocks: it stays calm. Make another angry, save and quit at once, load and walk to within 15 blocks of it: it comes at you again. Make one angry and switch to creative: it stops.
  6. Set the grass under a big bear alight with flint and steel: it runs while it burns. Punching a big bear never makes it run; it fights.
  7. Right-click a small shadow with the golden dandelion: it is used, with its sound.
  8. Two windows (`scripts\mp-local.ps1`): the friend sees the same shadows; the friend punches a bear and it goes for the friend. **m1-polar-live-4-host**, **m1-polar-live-4-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:polar_bear_spawn_egg*8,minecraft:iron_sword,minecraft:shield,minecraft:bow,minecraft:arrow*32,minecraft:flint_and_steel,minecraft:golden_dandelion*2,minecraft:cooked_beef*16
```

#### 3. Minecraft family spawn rules also decide chunk-generation spawns

After feed5dab.

- **Why:** a hook commit (M10 rule 2, D0-3), found while porting the polar bear's row. D0's `Rules::Family` rows reach their own rule from the two spawning paths that have a `SpawnContext` (`natural_spawn_rules` and `spawner_may_spawn`), but chunk generation (`spawn_for_chunk_generation`, vanilla's `spawnMobsForChunkGeneration` with `EntitySpawnReason.CHUNK_GENERATION`) called `check_spawn_rules` directly, which knows only the animal rules and the `SpawnableOn` tags. So a family creature whose vanilla rule differs (the polar bear on sea ice; later turtles on beaches) would never be placed with a new chunk, which is how most creatures appear.
- **Changed (vendored world crate):**
  - Root `natural_spawner/families.rs`: `generation_rule(spawns, level, kind, info, pos)`. A row with a rule decides with that rule, `spawner` false, and a generation context (no players, no respawn, difficulty 0, no weather, no structures, monsters not spawning); vanilla's chunk generation only places creatures, whose rules read no context. Any other kind gets `check_spawn_rules` as before.
  - `natural_spawner.rs` `spawn_for_chunk_generation`: its one `check_spawn_rules(level, &info, at)` call is `families::generation_rule(self, level, &data.kind, &info, at)`.
- **Tests added:** `natural_spawner::families::tests::chunk_generation_asks_a_family_row_for_its_spawn_rule` (the silverfish row's own rule allows stone in the dark where the animal rules do not; the probe row without a rule and the cow keep the animal rules, refusing stone in the dark and allowing grass in daylight).
- **Decisions:** the generation context is built in the root rather than changing D0's `FamilyRule` to take an `Option<&SpawnContext>`, so no group file changes. Its difficulty is 0, the only value that keeps a monster rule from ever passing there, though no monster row reaches chunk generation.
- **Runs:** `test -p minecraftoss-world --locked --lib` (117 passed, with the polar bear row of the next commit in the tree), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The touched files keep `rustfmt --check --edition 2024` clean.
- **Test steps:** nothing new to see until the next commit gives a family row a rule of its own (the polar bear's).
- **Kit:** none.

#### 4. Minecraft polar bears spawn in snowy biomes and on sea ice

After fa44b301.

- **javap this round:** `PolarBear.checkPolarBearSpawnRules`: in a biome of `#polar_bears_spawn_on_alternate_blocks` (frozen and deep frozen ocean in 26.3), raw light above 8 and a block of `#polar_bears_spawnable_on_alternate` (ice) below, with no spawner exception; elsewhere `Animal.checkAnimalSpawnRules` (`#animals_spawnable_on`, grass, below, and raw light above 8 unless `EntitySpawnReason.ignoresLightRequirements`, as for spawners). `finalizeSpawn` uses `AgeableMobGroupData(1.0)` when there is no group data. `SpawnPlacements` registers it `ON_GROUND`, `MOTION_BLOCKING_NO_LEAVES`.
- **Changed (vendored world crate, `natural_spawner/families/land.rs`):** the row `minecraft:polar_bear`: 1.4 x 1.4, `OnGround`, `MotionBlockingNoLeaves`, not fire immune, `Rules::Family { creature: true }`, `Walk::Animal`, kept when far, cluster 4, `MobCategory::Creature`, follow range 20, `baby_chance: Some(1.0)` (the first of a group an adult, every other a cub), no finalize; and `polar_bear_rule`, which looks both tags up by name as `check_spawn_rules` does. The built-in size row (`natural_spawner.rs`, rules `Other`, so it never spawned) is overridden by the row; ice as a valid floor and powder snow as no danger were already there.
- **Tests added:**
  - `natural_spawner::families::land::tests::polar_bears_spawn_on_grass_in_the_light_and_on_ice_in_frozen_oceans`: the row (size, placement, heightmap, rules, walk, cluster, category, follow range, baby chance); the rule allows grass in daylight in snowy plains and sea ice in frozen and deep frozen oceans, and a spawner on grass in the dark; it refuses grass in the dark, snow, ice on land, sea ice in the dark even from a spawner, packed ice and grass in a frozen ocean. Natural spawns and chunk generation both put bears on sea ice and none on packed ice.
  - `natural_spawner::families::land::tests::a_polar_bear_group_is_an_adult_with_cubs`: natural spawns (with a `SpawnContext`) and chunk generation (without one) in snowy plains start every group with an adult, the rest cubs; a summoned bear is an adult.
- **Decisions:**
  - A spawner ignores the light on grass but not on sea ice, as vanilla's two branches do.
  - The row's walk is `Walk::Animal` (`Animal.getWalkTargetValue`: grass is best, else the light), so a bear on sea ice needs daylight there too, as in vanilla.
- **Runs:** `test -p minecraftoss-world --locked --lib` (117 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The first run failed both new tests: chunk generation drew its seeds as 0..39, whose first `nextFloat` is about 0.73 for every small seed, so with the 0.5 probability no group ever started; the test now spreads its seeds. Natural spawning puts each of its three packs in a group of its own, often of one bear, so the group test counts cubs instead of asking for one per adult. `land.rs` keeps `rustfmt --check --edition 2024` clean.
- **Test steps** (bears are not drawn yet, so look for their round shadows). Release build, survival, Normal, by day. Name each screenshot by its label.
  1. `/locate biome minecraft:frozen_ocean`, `/tp` there and fly over new sea ice (chunks you have not loaded before): here and there a big shadow on the ice, sometimes with one or two small ones beside it. **m1-polar-spawn-1**
  2. `/locate biome minecraft:snowy_plains`, `/tp` there and walk into new land: now and then a big shadow, alone or with small ones, among the rabbits. **m1-polar-spawn-2**
  3. Stay a few minutes on a frozen ocean by day in loaded chunks after killing the bears there: new ones turn up on the ice over time.
  4. In creative, place the spawner on grass in a dark room and use the polar bear egg on it: bears spawn around it (none in a dark room on ice).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:polar_bear_spawn_egg*2,minecraft:spawner,minecraft:grass_block*64,minecraft:ice*32,minecraft:torch*16
```
