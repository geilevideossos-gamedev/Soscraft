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

## Hooks

Root hooks this track added (M10, "Hooks this track adds").

- **"Registered Minecraft mobs can run their own AI step"** (rabbits-live 1): `EntityWorld::family_goal_step_with::<S, W>(id, level, players, movement_speed, ai, run)` in `entities/src/world/families.rs`, with the type `AiStep<W>` (`monster_ai_step`'s signature, returning the landed fall distance). `family_goal_step` is now that call with `monster_ai_step`.
- `land/animal_goals.rs` (rabbits-live 2, M1's group module, `pub mod`): goal-framework `PanicGoal<E>`, `TemptGoal<E>` and `BreedGoal<E>` over a family's `GoalExtra` type `E: AnimalExtra` (which hands out its `AnimalGoals`), and `is_monster_class`. Later M1 families reuse them.
- `MoveToBlockGoal` in `land/animal_goals.rs` (rabbits-behaviour 1): vanilla's `MoveToBlockGoal` as a helper that a goal holds. Its fields are public: speed, search range, vertical range and start, `next_start_tick`, `try_ticks`, `max_stay_ticks`, `block` and `reached`. `can_use(ctx, valid)` is the countdown, then `reducedTickDelay(200 + nextInt(200))` and `findNearestBlock` over `feline_ai::find_nearest_block`; then `can_continue(valid)`, `start`, `move_to_target` and `tick` (reached within 1.0 of the centre of the block above, a new walk every 40 ticks). The goal that holds it names its valid block and what it does on arrival, as the rabbit's `RaidGardenGoal` does; foxes' berries and turtles' goals use it the same way.
- `AvoidEntityGoal::matching(fn(&str) -> bool)` (rabbits-live 2, `monster_ai.rs`): beside `also`; a field set by `new`, so no literal changes.
- `World::biome_in_tag(pos, tag)` (rabbits-live 3): `player/src/lib.rs` after `biome`, false by default; the server's `MobWorld` answers it after its `biome` (`server_mobs.rs`) through `Level::biome_in_tag_at`. M2's tadpoles use it under this name.
- **"The registry's texture test checks the pests' textures first, not the whole list"** (rabbits-drawn, before commit 1): not a hook, but a root-file change in its own commit. D0's `families::tests::a_registered_mob_is_tracked_textured_and_shadowed_through_its_family` (terrain root `families/tests.rs`) compared `textures()` with exactly the silverfish and endermite textures, so the first family with a texture of its own (the rabbit) fails it. It now compares `textures()[..2]`: `pests` comes first in the root's order, so the test still proves their two textures and lets every later family add its own. Every other round 10 track with a drawn family needs the same one-line change; whichever merges second keeps the identical line.

## Protocol

No commit of this track has changed a `ToClient`/`ToServer` message or the codec yet. `minecraft_net::PROTOCOL` is not bumped here.

## State doc changes

For the orchestrator to copy at the merge of segment m1a (rabbits live and drawn):

- `docs/MINECRAFT-SURVIVAL.md:441`: drop "rabbit" from the eggs that spawn nothing: "Spawn eggs for unsimulated mobs (fox, goat, the wither and others; ...)".
- `docs/MINECRAFT-SURVIVAL.md:451`: "The others (foxes, goats and the rest) stay dormant: ...", and under World simulation add: "Rabbits (round 10, track M1): hops with vanilla's jump control, fleeing players, wolves and monsters, panic, carrot temptation, breeding with the biome's colour one time in twenty, growth and the golden dandelion lock; saved, hatched from eggs, drawn with vanilla's adult and baby models and their 16 textures (Toast and the killer bunny's among them), hopping and tilting their heads with vanilla's keyframes, and seen by friends at the same moments."

## Left for later

- The hop's dust (entity event 1, `spawnSprintParticle`): `ClientFx` has no block particles (M1-1).
- A rabbit's bite on a carrot: level event 2001 (the crumbs and the break sound) waits for foxes' `World::destroy_effect` (M1-4). The `block_change` game event for W's vibrations is one line where the rabbit's tick applies the bite, added by whichever of W and M1 merges second (rabbits-behaviour 1).
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
