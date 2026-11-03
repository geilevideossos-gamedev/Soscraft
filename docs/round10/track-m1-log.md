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
- polar-bears-drawn: no vendored file (game code only).
- Changed again, no new file (review fix 1): `entities/src/world/families/land/rabbits.rs` (the jump clock while the step does not run), `entities/src/world/families/land/rabbits/ai.rs` (`count_jump`), `entities/src/world/families/land/rabbits/tests.rs` (its test).
- Added (pandas-live 1): `entities/src/world/families/land/pandas.rs`, `entities/src/world/families/land/pandas/ai.rs`, `entities/src/world/families/land/pandas/tests.rs` (test only). Changed again: `entities/src/world/families/land.rs` (registration).
- Changed again (pandas-live 2): `world/src/natural_spawner/families/land.rs` (the panda row, `finalize_panda`, its test; the rows are named consts).
- Changed again, no new file (pandas-drawn 1): `entities/src/world/families/land/pandas.rs` (`Amount`, the sit, back and roll amounts, `step_amounts`, `animate`), `entities/src/world/families/land/pandas/ai.rs` (the sneeze, eat and roll counters, `eating()` in place of the `eating` flag), `entities/src/world/families/land/pandas/tests.rs` (its test).
- Patched (pandas-eating 1, hook H1): `entities/src/monster_ai.rs` (`MonsterAi.still` and its `None` in `of_kind`), `entities/src/world.rs` (the check in `monster_ai_step`), `entities/src/world/families/tests.rs` (its test).
- pandas-eating 2 (hook H2): no vendored file (game code only).
- Changed again, no new file (pandas-eating 3): `entities/src/world/families/land/pandas.rs` (the pickup, `settle`, `worry`, `face_ahead`, `handle_eating`, `death_drops`, eating from the hand), `entities/src/world/families/land/pandas/ai.rs` (`PandaSitGoal`, `held_still`, `within`, four `PandaGoals` fields), `entities/src/world/families/land/pandas/tests.rs` (its tests).
- Patched (pandas-moods 1, hook H3): `entities/src/world/families.rs` (`FamilyGift`, `Families.gifts`, `family_gift`, `take_family_gifts`), `entities/src/loot.rs` (`EntityLootBook::roll_table` and its test), `entities/src/world/families/tests.rs` (its test).
- Changed again, no new file (pandas-moods 2): `entities/src/world/families/land/pandas.rs` (`roll_delta`, `sneeze`, `after_sneeze`, `handle_roll`, the tail's order), `entities/src/world/families/land/pandas/ai.rs` (the lie, sneeze and roll goals, `stays_down`, `roll_ahead`), `entities/src/world/families/land/pandas/tests.rs` (its tests, `Grove.ledge`).
- Patched (pandas-thunder 1, hook H4): `player/src/lib.rs` (`World::raining`, `World::thundering`), `world/src/level/fire.rs` (`Level::is_raining` made `pub` with vanilla's `canHaveWeather`, `Level::is_thundering`).
- Changed again, no new file (pandas-thunder 2): `entities/src/world/families/land/pandas.rs` (`scare`, the thunder half of `worry`), `entities/src/world/families/land/pandas/tests.rs` (its test, `Grove.thunder` and `Grove.pool`).
- Changed again, no new file (m1b review fix 1): `entities/src/world/families/land/pandas/tests.rs` (two statements wrapped as rustfmt wants).
- Patched (foxes-live 1, hook): `entities/src/world/families.rs` (`Family::spared_blocks` and the generic `Exposed` impl's `spared_blocks`), `entities/src/world/hazards.rs` (`Exposed::spared_blocks`, passed on by `blocks_act`), `entities/src/inside_blocks.rs` (`block_effects` takes the spared blocks and skips their `entityInside`; its test), `entities/src/world/families/pests/probe.rs` (the probe's `spares`), `entities/src/world/families/tests.rs` (its test). Game code: `crates/minecraft_survival/src/survival.rs` (the player spares nothing).
- Added (foxes-live 2): `entities/src/world/families/land/foxes.rs`, `entities/src/world/families/land/foxes/ai.rs`, `entities/src/world/families/land/foxes/tests.rs` (test only). Changed again: `entities/src/world/families/land.rs` (registration). Patched: `entities/src/look.rs` (`LookControl::tick_from`).
- Changed again (foxes-live 3): `world/src/natural_spawner/families/land.rs` (the fox spawn test and its `creatures` helper; no row).
- foxes-drawn 1: no vendored file (game code only).
- foxes-drawn 2: no vendored file (game code only).
- Patched (foxes-hunt 1, hook): `player/src/lib.rs` (`World::destroy_effect`). Game code: `crates/minecraft_terrain/src/server_mobs.rs` (`MobWorld::destroy_effect` and its test).
- Changed again, no new file (foxes-hunt 2): `entities/src/world/families/land/foxes.rs` (`set_target_goals`, the strike, the bite sound, the crouch reset, the faceplant's break effect), `entities/src/world/families/land/foxes/ai.rs` (the stalk, pounce, faceplant, melee and leap goals, the prey target goals, `is_path_clear`), `entities/src/world/families/land/foxes/tests.rs` (its tests and the den's snow, stone, tag and effects).

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
- **H1 "Registered Minecraft mobs can hold still while their goals run"** (pandas-eating 1): `MonsterAi.still: Option<fn(&MonsterGoalContext) -> bool>` (`monster_ai.rs`, after `jumping`; `None` in `of_kind`, the struct's only literal). `monster_ai_step` (`world.rs`) asks it after `tick_goals`, the navigation and the input fade; when it answers true the move control does not tick, so the yaw, the speed and the fading inputs stay as they are and the move control keeps its operation (vanilla `PandaMoveControl.tick`: `if (canPerformAction()) super.tick()`). It is not a `MonsterSpec` field, so no other track's spec literal changes. A family sets it right after it builds its `MonsterAi`; pandas use it now, foxes (`FoxMoveControl`) later.
- **H2 "Registered Minecraft mobs can scatter item crumbs"** (pandas-eating 2): `ClientFx.item_crumbs: &mut Vec<(String, DVec3, Vec<Vec3>)>` (terrain root `families.rs`, after `poof`), the type of `EntityView.item_crumbs`. Survival's `advance` builds the `ClientFx` with `item_crumbs: &mut self.item_crumbs` (the hook's one line in `entity_view.rs`, a never-edit file), so a family's `client_effects` pushes `(item id, position, velocities)` into the list that `present_crumbs` (`render_anim/src/minecraft_world.rs` for the host, `minecraft_remote.rs` for a friend) already drains into `spawn_item_crumbs`, the eye of ender's and thrown eggs' path. Every crumb is one item particle at its own position with one velocity per entry (`spawn_item` adds a tenth of a random push, as vanilla's `BreakingItemParticle` does). Pandas use it now, foxes (event 45) later; M2's sulfur cube should use this field rather than a `crumbs` field of its own.
- **H3 "Living Minecraft mobs can drop gifts from loot tables"** (pandas-moods 1): `EntityWorld::family_gift(table, this_type, at)` queues a `FamilyGift` on the root `Families.gifts` and `take_family_gifts()` drains them (entities root `families.rs`; a view made by `clone_where` carries none). `EntityLootBook::roll_table(table, loot_type, this_type)` (`loot.rs`, beside `roll_hero_gift`) rolls any loaded table of that type from its named random sequence over the private `roll_of_type`, with `this_type` as the `THIS_ENTITY` type. The server drains the queue right after `drop_cat_gifts` with one line in `server.rs`, `crate::families::drop_family_gifts(&mut self.mobs, self.entity_loot.as_mut(), &mut self.level)`; the body lives in the terrain root `families.rs`: each gift rolls its table as `minecraft:gift` and every stack goes through `Level::spawn_at_location` at the queued spot (vanilla `dropFromGiftLootTable` with `spawnAtLocation`: the default pickup delay of 10 and the random push of a new `ItemEntity`). A gift without a loot book, of an unknown table or of a table of another type drops nothing. The family decides when to give (vanilla gates the panda's sneeze on `mob_drops`, the armadillo's shedding on nothing). Pandas use it now; armadillos (`gameplay/armadillo_shed`), sniffers (`gameplay/sniffer_digging`) and turtles (`gameplay/turtle_grow`) later.
- **H4 "Minecraft mobs know when it rains and thunders"** (pandas-thunder 1): `World::raining()` and `World::thundering()` (`player/src/lib.rs`, right after `rain_at`, false by default) answer vanilla's `Level.isRaining` and `Level.isThundering`. The server's `MobWorld` answers them right after its `rain_at` (`server_mobs.rs`) through the level's `is_raining` (now `pub`) and the new `is_thundering` beside it (`level/fire.rs`): a level that can have weather, with the rain level above 0.2, or the thunder level times the rain level above 0.9 (`getThunderLevel(1)` is thunder times rain, so thunder needs rain). There is no `EntityWorld` field and no `world.rs` or `server.rs` line: goals and family ticks see the `World` and ask it where they need it. Pandas use it now (`isScared`), foxes and bees later. The spawner's own `thundering` (`level/spawning.rs:162`) stays M3's to correct; neither touches the other's line.
- **"Registered Minecraft mobs can be spared by the blocks they walk through"** (foxes-live 1): `Family::spared_blocks(mob) -> &'static [&'static str]` (root `families.rs`, beside `fire_immune`, `&[]` by default) and a defaulted `Exposed::spared_blocks` beside `fire_immune` (`hazards.rs`), which the generic `impl Exposed for FamilyMob<S>` answers from the family. `blocks_act` passes it to `inside_blocks::block_effects`, which now takes `spared: &[&str]` and skips the `entityInside` of a spared block (vanilla `SweetBerryBushBlock.entityInside` returns at once for `EntityTypes.FOX` and `EntityTypes.BEE`: no `makeStuckInBlock`, no prick); the block still counts as visited and still advances the effect step, as vanilla's walk does. The player's own walk (`survival.rs`) passes `&[]`. Foxes use it now, bees later.
- `LookControl::tick_from(.., reset_pitch)` (foxes-live 2, in the fox commit, as M10 names it): `look.rs`, beside `tick_limited`, which now calls it with `true`, so every other mob looks as before. With `false` the tick does not set the pitch to 0 first (vanilla `LookControl.resetXRotOnTick`), which `FoxLookControl` turns off while the fox pounces, crouches, is interested or lies faceplanted. M2's frogs can use it.
- **"Mobs can show a block's break effect without breaking it"** (foxes-hunt 1): `World::destroy_effect(pos)` (`player/src/lib.rs`, right after `destroy_block`, nothing by default) is vanilla's `levelEvent(2001, pos, Block.getId(state))`. The server's `MobWorld` answers it right after its `destroy_block` (`server_mobs.rs`): the state at `pos`, unless it is air, goes into `level.destroyed`, the list a real break fills, so host and friends get the same burst of block bits and the break sound at `(volume + 1) / 2` and `pitch * 0.8` (vanilla `LevelEventHandler` 2001) while the block stays. A level that keeps no break list (`destroyed: None`) shows nothing. Foxes use it now (the faceplant); the rabbit's carrot nibble and the turtle's digging can use it with one line each.

## Protocol

No commit of this track has changed a `ToClient`/`ToServer` message or the codec yet. `minecraft_net::PROTOCOL` is not bumped here.

## State doc changes

For the orchestrator to copy at the merge of segment m1a (rabbits done):

- `docs/MINECRAFT-SURVIVAL.md:441`: drop "rabbit" from the eggs that spawn nothing: "Spawn eggs for unsimulated mobs (fox, goat, the wither and others; ...)".
- `docs/MINECRAFT-SURVIVAL.md:451`: "The others (foxes, goats and the rest) stay dormant: ...", and under World simulation add: "Rabbits (round 10, track M1): hops with vanilla's jump control, fleeing players, wolves and monsters, panic, carrot temptation, breeding with the biome's colour one time in twenty, growth and the golden dandelion lock; saved, hatched from eggs, drawn with vanilla's adult and baby models and their 16 textures (Toast and the killer bunny's among them), hopping and tilting their heads with vanilla's keyframes, and seen by friends at the same moments. With `mob_griefing` they raid full-grown carrots, a stage a bite. The killer bunny (`RabbitType` 99) has armour 8, never flees, leaps at players and wolves within 4 blocks, bites for 8 before difficulty and calls the rabbits around it when hurt; its sounds are hostile. Wild cats hunt rabbits."
- At the merge of segment m1a (polar bears done): under World simulation in `docs/MINECRAFT-SURVIVAL.md`: "Polar bears (round 10, track M1): wander, defend their cubs (a hit cub runs and calls the adults within 20 blocks; adults attack players within 10 blocks of a cub), rear up with a growl every two seconds and swipe for 6 before difficulty, hunt foxes, panic only from fire (cubs from any hit), stay angry 20 to 39 seconds (saved with `angry_at`), follow their mothers as cubs; spawn on grass in snowy plains and ice spikes and on sea ice in frozen oceans, an adult with cubs; saved, hatched from eggs, drawn with vanilla's adult model (1.2 times its size) and the cub's own model, rearing onto their hind legs before each swipe, and seen by friends in the same pose." Family creature rows now decide chunk-generation spawns with their own rules (`generation_rule`).

- At the merge of segment m1b (pandas): `docs/MINECRAFT-SURVIVAL.md:134` gains pandas among the simulated mobs, and under World simulation a pandas paragraph; `pandas-thunder` writes its final wording here when the family is done. So far (pandas-live): "Pandas (round 10, track M1): genes and the seven variants (lazy, worried, playful, brown, weak, aggressive and normal, brown and weak only when both genes match), breeding next to a bamboo stalk with genes from the parents and one in 32 mutating, sulking with a sad sound without bamboo, biting back once (aggressive pandas keep biting and join a fight), worried pandas fleeing players and monsters, panic from fire and cactus, bamboo temptation and feeding, growth and the golden dandelion lock; weak pandas have 10 health and lazy ones walk at 0.07; spawn in bamboo jungles, a cub one time in five after the first; saved with their genes, held item and attributes, hatched from eggs and seen by friends." pandas-drawn adds to it: "drawn with vanilla's adult and cub models and their 14 textures, in every pose (sitting, eating, scared, on the back, rolling, sneezing and sulking, eased in and out like vanilla) and with the held item in their paws, the same for friends." pandas-eating adds: "they sit down to eat bamboo and cake: fetched from the ground (a whole stack, with `mob_griefing`) or given by hand when they cannot breed, munching with the eat sound and crumbs (for friends too) for 5 to 15 seconds, dropping what they hold when they get up, and dropping a picked-up stack when they die; worried pandas never settle down to eat." pandas-moods adds: "lazy pandas lie on their backs until a click gets them up, playful pandas and cubs somersault (and roll off ledges), and cubs sneeze (weak ones often) with a green puff that makes the adults around them hop and drops a slime ball one time in 700; the same for friends." pandas-thunder adds: "while it thunders, worried pandas out of water sit with their faces covered and tremble, stop eating and take no bamboo, and stand up when the storm passes; the same for friends."
- **Final wording for the merge of segment m1b (pandas done):** in `docs/MINECRAFT-SURVIVAL.md`, the "Simulated mobs:" line (`:134`) gains "panda" after "wolf", and under World simulation, after the polar bears' paragraph: "Pandas (round 10, track M1): genes and the seven variants (lazy, worried, playful, brown, weak, aggressive and normal; brown and weak show only when both genes match), breeding next to a bamboo stalk with genes from the parents and one in 32 mutating, sulking with a sad sound without bamboo, biting back once (aggressive pandas keep biting and join a fight), worried pandas fleeing players and monsters, panic from fire and cactus, bamboo temptation and feeding, growth and the golden dandelion lock; weak pandas have 10 health and lazy ones walk at 0.07. They sit down to eat bamboo and cake, fetched from the ground (a whole stack, with `mob_griefing`) or given by hand when they cannot breed, munching with the eat sound and crumbs for 5 to 15 seconds, dropping what they hold when they get up and a picked-up stack when they die; worried pandas never settle down to eat. Lazy pandas lie on their backs until a click gets them up, playful pandas and cubs somersault (and roll off ledges), and cubs sneeze (weak ones often) with a green puff that makes the adults around them hop and drops a slime ball one time in 700. While it thunders, worried pandas out of water sit with their faces covered and tremble, stop eating and take no bamboo, and stand up when the storm passes. They spawn in bamboo jungles, a cub one time in five after the first; they are saved with their genes, held item and attributes, hatched from eggs, drawn with vanilla's adult and cub models and their 14 textures in every pose with the held item in their paws, and seen by friends in the same poses at the same moments. Egg pandas pick up nothing (see Limitations)." Under Limitations: "Spawn eggs give mobs `CanPickUpLoot` false, as `/summon` does, so egg pandas (and villagers, piglins and later foxes) never pick up items; vanilla eggs keep the mob's own default." Mobs read the level's rain and thunder like vanilla (`World::raining`, `World::thundering`), with no thunder outside rain and none in the Nether or the End.
- At the merge of segment m1c, from foxes-live (the fox paragraph grows with each fox group; `foxes-trust` writes the final wording): `docs/MINECRAFT-SURVIVAL.md`'s "Simulated mobs:" line gains "fox", the fox leaves the eggs that spawn nothing (`:441`) and the dormant list (`:451`), and under World simulation: "Foxes (round 10, track M1): red and snow foxes float, climb out of powder snow, panic when hurt, run from walking players (not sneaking, creative or trusted ones), wild wolves and polar bears, head for shade on sunny days away from villages and for cover in thunderstorms, sleep by day under cover until something walks near (a sneaking or trusted player does not wake them; thunder and water do), perch and look about, stroll through villages at night, follow their parents as cubs, snore asleep and screech at night when no player is near, take a lead, keep and drop what they hold in their mouths, walk through berry bushes unhurt and take no fall damage up to five blocks; saved with their type, trust, sleep, sitting, love and mouth item, hatched from eggs and seen by friends." foxes-drawn adds: "drawn with vanilla's adult and cub models and their eight textures (shut eyes while asleep), trotting with swinging legs or, as cubs, with vanilla's walk keyframes, curled up asleep, sitting, crouching with a cocked head, tilted in a pounce and paddling when faceplanted, carrying what they hold in their mouths (laid aside while asleep), the same for friends." foxes-hunt adds: "they hunt chickens and rabbits (snow foxes put fish first once fish swim): a fox walks towards its prey, crouches 6 blocks from it with its head cocked, leaps at it in a high arc once fully crouched and bites it as it comes down; a fox whose leap is blocked, or that missed, runs at its prey and bites it; a leap that lands in snow leaves the fox stuck nose-first for 2 seconds with snow bits flying, the same for friends. Bred cubs hunt only once the world has been loaded again, as in vanilla."

## Left for later

- The hop's dust (entity event 1, `spawnSprintParticle`): `ClientFx` has no block particles (M1-1).
- A rabbit's bite on a carrot: level event 2001 (the crumbs and the break sound) is now one `World::destroy_effect` line in the rabbit's tick (the hook landed with foxes-hunt 1, after 6ef1a2d9); it was left out of foxes-hunt, whose commits are the foxes'. The `block_change` game event for W's vibrations is one line where the rabbit's tick applies the bite, added by whichever of W and M1 merges second (rabbits-behaviour 1).
- Polar bears (M1-2): `mustReach` (ported nowhere, so a bear may take a target it cannot path to), the 0.98 water slow-down, the standing bear's taller client box and freeze immunity. The old `AngerTime` key is not read.
- The shared `FloatGoal` uses a fixed 0.4 water height; vanilla's `getFluidJumpThreshold` is 0 for an eye below 0.4 (a baby rabbit, a silverfish). The rabbit's own jump uses the right threshold.
- Pandas (M1-3): eggs read `CanPickUpLoot` as false (`spawning.rs` serves eggs and `/summon` alike), so egg pandas pick up nothing; vanilla eggs keep the constructor's true (fork-wide, the steps use `{CanPickUpLoot:1b}`). Dispensers filling a panda's paw and the pickup's flight animation. D0's `monster_ai_step` adds `Monster`'s +2 idle in light for every goal family, creatures included (D0's fix). Sneaking players are not seen at 0.8 of the range by the breed goal's look search (the fork's look goals ignore it too). From pandas-eating: a damageable item a panda drops on death keeps its wear (pandas only ever pick up bamboo and cake); the 2% input drift on the tick the sit goal sits a panda down (decision in pandas-eating 3); friends see crumbs only for bamboo and cake. From pandas-moods: the sneeze cloud does not drift down towards the feet of a player within 2 blocks (`PlayerCloudParticle.tick`; the ambient particles have no players), and its motion is the client mob's last tick of movement rather than the synced motion.
- Foxes (M1-4), from foxes-live: no equip or unequip game event when a fox picks up, spits out or eats an item (W's w3 helpers such as `Body::equipment_changed` are not in this branch; left for the orchestrator to add one line each where `foxes-items` changes the mouth item, once w3 is merged). Invisibility (`getVisibilityPercent`) does not shorten the avoid and alert ranges, and walking on powder snow (`#powder_snow_walkable_mobs`) is not ported (a shared collision flag, M1-4 "Left for later"). A fox's paths take falls of 3 like vanilla's (`getComfortableFallDistance` adds 3 whatever the safe fall distance); only its fall damage counts from 5.
- Foxes (M1-4), from foxes-hunt: the turtle prey slot stays `Prey::Absent` until `turtles-hunted` (M1-8); fish prey wait for M2's fish. `Fox.setTarget(null)` ends defending only for the faceplant so far; `foxes-trust` makes every dropped target end it, with the defend goal.

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

### Group polar-bears-drawn

#### 1. Minecraft polar bears are drawn like vanilla

After 84d52a57.

- **javap this round** (`PolarBearModel`, `BabyPolarBearModel`, `QuadrupedModel`, `MeshTransformer`, `PartPose.scaled`, `MeshDefinition.transformed`, `LayerDefinitions`, `PolarBearRenderer`, `PolarBearRenderState`, `PolarBear.getStandingAnimationScale`, `LivingEntity.getAgeScale`):
  - `PolarBearModel.createBodyLayer` (128 x 64): `head` at (0, 10, -16) with the head box (uv 0, 0; from -3.5, -3, -3; 7 x 7 x 7), `mouth` (uv 0, 44; -2.5, 1, -6; 5 x 3 x 3), `right_ear` (uv 26, 0; -4.5, -4, -1; 2 x 2 x 1) and `left_ear` (uv 26, 0, **mirrored**; 2.5, -4, -1); `body` at (-2, 9, 12) turned π/2 about x with two boxes (uv 0, 19; -5, -13, -7; 14 x 14 x 11 and uv 39, 0; -4, -25, -7; 12 x 12 x 10); hind legs (uv 50, 22; -2, 0, -2; 4 x 10 x 8) at (∓4.5, 14, 6) and front legs (uv 50, 40; 4 x 10 x 6) at (∓3.5, 14, -8), none mirrored. 6 parts, 10 cubes. Then `MeshTransformer.scaling(1.2)`: only the root's `PartPose` changes (`MeshDefinition.transformed` maps the root and keeps its children), to scale 1.2 and y -4.8032 (`24.016 × (1 - 1.2)`), so the model grows 1.2 times from the floor line at y 24.016.
  - `BabyPolarBearModel.createBodyLayer` (64 x 64, no transform; `LayerDefinitions` puts both layers in as built): `body` at (0, 17.5, 0), not turned (uv 0, 9; -4, -3.5, -6; 8 x 7 x 12); `head` at (0, 18.625, -5.75) with the head (uv 0, 0; -3, -2.625, -4.25; 6 x 5 x 4), the nose (uv 20, 3; -2, 0.375, -6.25; 4 x 2 x 2) and two ears (uv 20, 0 and 26, 0; 2 x 2 x 1 at x -4 and 2, y -3.625, z -2.75, neither mirrored); four 3 x 3 x 3 legs (from -1.5, -0.5, -1.5) at (∓2.5, 21.5, ±4.5) with uv (0, 34), (12, 34), (0, 28) and (12, 28). 6 parts, 9 cubes. It extends `PolarBearModel`, so it shares its `setupAnim`.
  - `QuadrupedModel.setupAnim`: after the pose reset the head's `xRot` and `yRot` are set to the look, and the legs swing `cos(pos × 0.6662 [+ π]) × 1.4 × speed` (the fork's `quadruped_legs`). `PolarBearModel.setupAnim` then, with `f = standScale²` and `a = ageScale`: `body.xRot -= f·π·0.35`, `body.y += f·a·2`; the right front leg `y -= f·a·20`, `z += f·a·4`, `xRot -= f·π·0.45`, and the left front leg takes the right one's `y` and `z` (the same values, as both legs start at the same height and depth) and the same `xRot` change; `head.y -= f·24`, `head.z += f·13`, `head.xRot += f·π·0.15` (the head's move does not use `ageScale`).
  - `PolarBearRenderer`: `AgeableMobRenderer(adult, cub, 0.9)`, textures `entity/bear/polarbear` and `polarbear_baby` by `isBaby`; `extractRenderState` sets `standScale = getStandingAnimationScale(partial) = lerp(partial, O, now) / 6`; `ageScale` is `LivingEntity.getAgeScale` (0.5 for a baby, else 1). `QuadrupedModel`'s default render type is `RenderTypes.entityCutout` (back faces culled); `getFlipDegrees` stays 90. `PolarBear` has no `getLeashOffset` of its own.
- **Changed (game code, `minecraft_terrain`):**
  - New `families/land/polar_bears/render.rs`: both models as part tables (name, offset, turn, cubes) in `QuadrupedModel`'s order (head, body, right and left hind leg, right and left front leg); `Stance` (look, leg swings, stand scale, age scale); `part_poses` (vanilla's `setupAnim` on the offsets and turns, then `part_pose` per part); `stand_scale` (the bear's `stand_old` and `stand` lerped by the partial tick, over 6); `scaled_root` (copied from `cat_render.rs`, which keeps it private, as the brief says) and `model_root` (vanilla's flip of 90 degrees, `scale(-1, -1, 1)` and the 1.501 lift, then the model's own scale, 1.2 for the adult and 1 for the cub); `texture`, `age_scale` and `append_polar_bears` (the skin lit at the eye, the hurt overlay).
  - `families/land/polar_bears.rs`: `mod render;`, `TEXTURES` now from `render`, `CAGE_MARGIN` 0.13, and `draw` puts the living bears into `out.culled`.
- **Tests added** (`families::land::polar_bears::render::tests`):
  - `the_adult_is_scaled_1_2_and_the_cub_has_its_own_model`: 10 and 9 cubes, texture sizes 128 x 64 and 64 x 64, scales 1.2 and 1, the part names, the mirrored left ear, the textures and shadow, the scaling's floor line kept at 24.016, the body lying along the bear. Drawn: 60 faces into `culled` only, the paws on the ground, the top 1.426 above them and 35 × 1.2 pixels from snout to rump, every uv inside the adult skin, the light. A cub drawn alone (a removed adult is not drawn) has 54 faces, its own skin, and its back on top at 0.626.
  - `a_standing_bear_lifts_its_body_front_legs_and_head`: standing fully, the body's pivot and its turn of 0.15π, the front legs' pivots and turns, the head's pivot and nod, and the hind legs unchanged; half standing moves by the square; a standing cub with `ageScale` 0.5 and the head's full move; the look and the walk swing add to the standing turns. Drawn, a standing bear stays on its hind paws and is more than 1.4 blocks taller.
  - `the_stand_clock_lerps_with_the_partial_tick`: `stand_scale` at partial 0, 0.5 and 1, going up and down; the drawn mesh is last tick's pose at partial 0, this tick's at partial 1 and neither in between; the clock is squared after the lerp; a friend given the host's `FamilyShow` draws the same mesh as the host at partial 0, 0.25, 0.5 and 1.
- **Decisions:**
  - Each screen lerps the stand clock from the two values the host sends (M1-2 decision 2, as `polar-bears-live` took it), so the host and every friend draw the same pose at every partial tick.
  - The model tables follow vanilla's part names and `setupAnim` line by line. `pig_render.rs`'s private, index-based `part_pose` was not needed; only `cat_render.rs`'s `scaled_root` was copied.
  - `CAGE_MARGIN` 0.13. At the cage's scale (0.53125 / 1.4) and its 30-degree tilt, the top edge of the bear's rump is 0.62 blocks from the cage's centre, so as the bear turns it pokes up to 0.124 out of the cell. Vanilla's spawner draws it the same way. D0's cage test (`every_registered_kind_is_shown_inside_its_cage`) failed with the default 0.05 in the first full run.
  - The lead keeps the fork's default offset, because vanilla's `PolarBear` does not override `getLeashOffset`.
  - The standing bear's taller box stays left for later (it is client-only in vanilla; see M1-2 "Left for later").
- **Protocol:** no message or codec change. The pose uses `FamilyShow`'s existing flag and the two values from `polar-bears-live`.
- **Runs:** `test -p minecraft_terrain --locked` (1446 passed, 3 ignored; the first run failed only the cage test, fixed by the margin above), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new file passes `rustfmt --check --edition 2021` on its staged content.
- **Test steps** (release build, survival, **Normal**, by day on flat grass; name each screenshot by its label):
  1. Use the polar bear egg 8 blocks away: a big white bear on all fours, about 2.6 blocks from snout to rump and 1.4 tall, with a snout and two small ears, and a wide round shadow. Its legs swing as it walks, and its head turns to look at you. **m1-polar-drawn-1**
  2. `/summon minecraft:polar_bear ~2 ~ ~8 {Age:-24000}`: a cub with its own rounder model (big head, short snout, stubby legs, about half a block tall) and its own skin, not a shrunken adult. It follows the adult. **m1-polar-drawn-2**
  3. Punch the adult and let it come at you. Just before each swipe it rears onto its hind legs within a third of a second: body tilted up, front paws reaching forward, head high, about 3 blocks tall. It drops back onto all fours as it swipes. Take the screenshot while it is standing. **m1-polar-drawn-3**
  4. Step back to about 3 blocks from it while it is angry: it rears up and drops down again about once a second without hitting you.
  5. Kill it with the iron sword: it flashes red on each hit and tips over sideways as it dies.
  6. In creative, place the spawner and use a polar bear egg on it: a small bear turns slowly inside the cage, and at times its rump pokes a little out of the cage, as in vanilla. **m1-polar-drawn-4**
  7. Save, quit and load: the same bears, the cub still a cub.
  8. Two windows (`scripts\mp-local.ps1`), side by side: the friend sees the same bear and cub. The friend punches the bear: both windows show it rear up and drop down at the same moments. **m1-polar-drawn-5-host**, **m1-polar-drawn-5-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:polar_bear_spawn_egg*8,minecraft:iron_sword,minecraft:shield,minecraft:cooked_beef*16,minecraft:spawner
```

#### Polar bears (M1-2) as a whole

Row 22 of `docs/round10/mobs.md` is now `done`. The owner can test the whole section with M1-2's steps in `docs/round10/track-m1.md` and this kit. The fox egg spawns nothing until M1-4 lands.

```
IW4L_MINECRAFT_KIT=minecraft:polar_bear_spawn_egg*8,minecraft:fox_spawn_egg*4,minecraft:iron_sword,minecraft:flint_and_steel,minecraft:bow,minecraft:arrow*64,minecraft:shield,minecraft:cooked_beef*32,minecraft:spawner,minecraft:lead*4,minecraft:golden_dandelion*4,minecraft:grass_block*64,minecraft:ice*32,minecraft:snow_block*32,minecraft:torch*64
```

### Review fixes

#### 1. Dying Minecraft rabbits stop hopping when their jump ends

After 1c672729.

- **Defect:** the rabbit's jump clock (`jumpTicks` and `jumpDuration`) only counted inside `rabbit_ai_step`, which the tick skips for a dying rabbit and for one without AI. A rabbit killed mid-hop (a fleeing rabbit almost always is) kept its clock at the tick it died, so `RabbitAnimations` kept the hop started and the hop keyframes looped through the whole death, for the host and every friend. In vanilla `Rabbit.aiStep` counts the clock after `LivingEntity.aiStep` on every tick, dead or without AI (`isImmobile` and `isEffectiveAi` only skip `serverAiStep`), so the hop ends with the jump, at most 15 ticks later.
- **javap this round:** `Rabbit.aiStep` (`super.aiStep()`, then `jumpTicks != jumpDuration` counts up, else a non-zero duration resets both and `setJumping(false)`), `LivingEntity.aiStep` (`isImmobile` clears the inputs, otherwise `serverAiStep` only with effective AI).
- **Changed (vendored entities crate):** `land/rabbits/ai.rs`: the jump clock's count is `count_jump(jump, jumping)`, which `rabbit_ai_step` calls where it counted before. `land/rabbits.rs`: when the tick runs no AI step (dead or `NoAI`), it counts the clock with `count_jump`, at the place of the step.
- **Tests added:** `world::families::land::rabbits::tests::a_dying_or_still_rabbit_ends_its_hop_with_its_jump` (a rabbit killed four ticks into its jump and one with `NoAI` keep the hop for the 12 ticks left of the jump while the clock counts on, then the clock is reset, `jumping` is off and the hop has ended).
- **Docs:** row 22 of `docs/round10/mobs.md` names `1c672729` in place of the polar bears' drawing subject.
- **Runs:** `test -p minecraftoss-entities --locked --lib rabbits` (20 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The three rabbit files still pass `rustfmt --check --edition 2021`.
- **Test steps** (release build, survival, Normal, by day on flat grass; name each screenshot by its label):
  1. Use the rabbit egg 8 blocks away and walk at the rabbit so it flees in quick hops. Shoot it with the bow while it is in the air. As it flashes red and tips over sideways, its legs finish the hop and then stay still; they do not keep kicking for the whole second it lies dying. **m1-fix-rabbit-death-1**
  2. Two windows (`scripts\mp-local.ps1`): the friend kills a fleeing rabbit with the M4A1; both windows show it stop hopping as it dies. **m1-fix-rabbit-death-2-host**, **m1-fix-rabbit-death-2-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*8,minecraft:bow,minecraft:arrow*32,minecraft:iron_sword
```

### Final verification

After e80f5930, on the whole of segment m1a (`r10-m1a-base..HEAD`).

- **Crates touched:** `minecraft_terrain`, `minecraftoss-entities`, `minecraftoss-player`, `minecraftoss-world`.
- **Clippy** (`clippy -p minecraft_terrain -p minecraftoss-entities -p minecraftoss-player -p minecraftoss-world --locked --all-targets -- -A clippy::approx_constant`, with `full-suite.ps1`'s two empty stand-in scenario files): green. `minecraft_terrain` has 36 old warnings, none on a line this track added (the four in `server_mobs.rs` are on lines from before `r10-m1a-base`); the three vendored crates have none.
- **Tests:**
  - `test -p minecraftoss-entities --locked`: 469 passed.
  - `test -p minecraftoss-player --locked`: 198 passed.
  - `test -p minecraftoss-world --locked`: 117 passed.
  - `test -p minecraft_terrain --locked`: 1446 passed, 3 ignored. The first run failed only the known flaky `sections::tests::background_full_update_matches_the_synchronous_one`; the re-run passed.
- **Fixes:** none needed.

### The whole track's kit

One line for every step above (rabbits, polar bears and the review fix), 27 stacks. It leaves out the golden carrots (carrots and dandelions tempt and breed rabbits too), the lava bucket (flint and steel sets the fire), the snow blocks (no step uses them) and the fox egg (it spawns nothing until M1-4 lands). The friend's `.env` needs the same line.

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*16,minecraft:polar_bear_spawn_egg*8,minecraft:carrot*64,minecraft:dandelion*16,minecraft:golden_dandelion*4,minecraft:iron_hoe,minecraft:bone_meal*64,minecraft:water_bucket,minecraft:oak_fence*64,minecraft:wolf_spawn_egg*2,minecraft:cat_spawn_egg*2,minecraft:zombie_spawn_egg*2,minecraft:lead*4,minecraft:iron_sword,minecraft:shield,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:spawner,minecraft:cooked_beef*32,minecraft:bow,minecraft:arrow*64,minecraft:grass_block*64,minecraft:flint_and_steel,minecraft:ice*32,minecraft:torch*16
```

## Segment m1b

### Group pandas-live

#### 1. Minecraft pandas live in the entity world with their genes

After 7a1afdf7 (`r10-m1b-base`).

- **javap this round** (`Panda` and its inner classes `Gene`, `PandaAttackGoal`, `PandaAvoidGoal`, `PandaBreedGoal`, `PandaHurtByTargetGoal`, `PandaLookAtPlayerGoal`, `PandaMoveControl`, `PandaPanicGoal`; `EntityTypes.PANDA`; `Mob.doHurtTarget`): the M1-3 "Vanilla 26.3" numbers hold. Added on the way:
  - `EntityTypes.PANDA`: `CREATURE`, `sized(1.3, 1.25)` (eye 1.0625), `clientTrackingRange(10)`. `BABY_DIMENSIONS` is the type's size scaled 0.5 with eye 0.28125.
  - The constructor calls `setCanPickUpLoot(true)` only when `!isBaby()`, which is always true while it is built.
  - `Gene.getRandom`: `nextInt(16)` with 0 lazy, 1 worried, 2 playful, 4 aggressive, 3 and 5-8 weak, 9-10 brown, 11-15 normal. `getVariantFromGenes`: a recessive main gene (brown, weak) shows only when the hidden gene is the same, else `NORMAL`. `CODEC` by name; an unknown name reads as `NORMAL`.
  - `setGeneFromParents(a, b)` runs on the cub: its `nextBoolean` and both `nextInt(32)` mutations come from the cub's random, while `getOneOfGenesRandomly` runs on each parent with that parent's random. `getBreedOffspring(level, partner)` passes `this` (the parent whose breed goal bred) as `a`.
  - `PandaBreedGoal.canUse`: `BreedGoal.canUse()` and an unhappy counter of 0, then `canFindBamboo` (any `minecraft:bamboo` at `blockPosition()` plus x and z -7..7, y 0..2). Otherwise, once `unhappyCooldown <= tickCount`: unhappy 32, cooldown `tickCount + 600`, and `lookAtPlayerGoal.setTarget(getNearestPlayer(forNonCombat().range(8), panda))`.
  - `PandaLookAtPlayerGoal.canUse`: the 0.02 roll, a search only while `lookAt` is null, then `canPerformAction() && lookAt != null`. `canContinueToUse` adds `lookAt != null`; `tick` runs only with a target.
  - `PandaHurtByTargetGoal.canContinueToUse`: `gotBamboo` or `didBite` clears the target and ends the goal. `alertOther` sets the target only on a `Panda` whose `isAggressive()` holds; the panda's override of `isAggressive` is its aggressive variant. `HurtByTargetGoal.alertOthers` searches its own class in `unitCubeFromLowerCorner(position).inflate(followRange, 10, followRange)` for mobs with no target.
  - `PandaAttackGoal` and `PandaAvoidGoal` test `canPerformAction` (the avoid goals `isWorried` first) before the parent's `canUse`. `PandaPanicGoal(2.0, #panic_environmental_causes)` ends at once while the panda sits, stopping its path. `PandaMoveControl.tick` skips the move while `!canPerformAction` (H1, `pandas-eating`).
  - `Panda.tick` after `super.tick()`: the worried panda's thunder sit (`pandas-thunder`); `getTarget() == null` clears `gotBamboo` and `didBite`; the unhappy tail (`lookAt(target, 90, 90)` while it has a target, `cant_breed` at 29 and 14, then one less); then the sneeze, the roll, sitting, eating and the amounts (later groups).
  - `doHurtTarget` sets `didBite` on a panda that is not aggressive, before `Animal.doHurtTarget`. `Mob.doHurtTarget` plays `playAttackSound` (the panda's `entity.panda.bite` at 1, 1) only after the hurt landed.
  - `mobInteract`: scared → pass; on its back → it gets up; `#panda_food` → `gotBamboo` when it has a target, then a cub that `canAgeUp` grows (`ageUp((-age / 20) * 0.1, true)`), any other cub passes, an adult at age 0 that `canFallInLove` falls in love, otherwise the eat-from-hand branch (`pandas-eating`); a cub held a golden dandelion → `Animal.mobInteract`.
- **Changed (vendored entities crate):**
  - New `world/families/land/pandas.rs`: `Gene` (ids, names, recessive, the 16-slot table, `variant`); `Panda` (the two genes, `Age`, love, the last damage, its own movement speed, the held stack with its drop chance, `can_pick_up_loot`, `bred`, `MonsterAi`); `set_baby` (0.65 x 0.625, eye 0.28125); `set_attributes` (weak: max health 10, health clamped to it; lazy: speed 0.07); `is_food` (`#panda_food`, or bamboo without a recipe book); `genes_from_parents`; `interact`; and `impl Family`: key 52, kind `minecraft:panda`, 1.3 x 1.25, eye 1.0625, health 20, speed 0.15, ambient interval 120, category `creature`, no lead, panicking while its panic goal runs, step `entity.panda.step`, its `MonsterSpec`.
  - The tick: `family_base_tick`; an aggressive or worried panda's ambient voice becomes `entity.panda.aggressive_ambient` or `entity.panda.worried_ambient` (vanilla's roll and pitch kept); the goals get its variant, age, the damage of the last 40 ticks, the players holding bamboo, and the other pandas as mates and kin; `family_goal_step` at its own speed; the bite (`family_strike`, 6, `Melee`; `did_bite` unless aggressive; the bite sound unless the hit failed); the alert to aggressive pandas; breeding; `family_after_move`; growing up and the love countdown; the tail.
  - New `world/families/land/pandas/ai.rs`: `SPEC` (follow range 16, eye 1.0625, the animal fire maluses, `WalkTarget::Animal`), `PandaGoals` (the animal goal state, variant, cub, the sit, back, roll, sneeze, eat and scared flags the later groups set, the unhappy counter, `got_bamboo`, `did_bite`, `can_perform_action`), `can_find_bamboo`, and the goals in vanilla's order: `FloatGoal` 0, `PandaPanicGoal` 2 (2.0), `PandaBreedGoal` 2 (1.0), `PandaAttackGoal` 3 (melee 1.2, follows unseen), `TemptGoal` 4 (1.0), two `PandaAvoidGoal`s 6 (players within 8, `is_monster_class` within 4, both 2.0), `PandaLookAtPlayerGoal` 9 (6 blocks), `RandomLookAroundGoal` 10, `FollowParentGoal` 13 (1.25), `WaterAvoidingRandomStrollGoal` 14 (1.0); the target `PandaHurtByTargetGoal` 1, alerting others.
  - New `world/families/land/pandas/tests.rs` (test only).
  - `world/families/land.rs`: `pub mod pandas;` and `pandas: pandas::Panda` in `family_group!`, between the rabbits and the polar bears (key order).
- **Tests added** (`world::families::land::pandas::tests`):
  - `a_panda_is_a_big_twenty_health_animal_with_its_kind` (sizes and eyes of adult and cub, health, speed, attack, interval, genes, loot pickup, census, candidates, traits, voice, step sound, goal and target names, follow range, floating, walk target, the table and 1-3 experience for adults only).
  - `a_cub_is_half_size_with_its_own_eye_height` (the walk profile too; it grows to 1.3 x 1.25 at age 0).
  - `random_genes_follow_the_sixteen_slot_table` (each roll, 16000 draws, ids, names, an unknown name normal, the two recessive genes), `a_recessive_gene_shows_only_when_both_match` (the goals see the variant).
  - `a_cub_takes_a_gene_from_each_parent_and_mutates_one_time_in_thirty_two` (6400 litters: 3-9% mutated, either parent gives the main gene about half the time, all four genes seen; two weak parents make weak cubs).
  - `weak_pandas_have_ten_health_and_lazy_ones_walk_at_0_07` (only the shown variant counts; a tempted lazy panda covers under 0.6 of a normal one's way in 100 ticks).
  - `bamboo_within_seven_blocks_across_and_two_up_counts`, `pandas_breed_only_with_bamboo_within_seven_blocks_and_sulk_otherwise` (goal level: unhappy 32, the look target the nearest visible player within 8 blocks, no sulk while unhappy, the goal runs with bamboo 7 across and 2 up; in a walled pen: bamboo 5 blocks off gives a cub with hearts, parents at 6000, an orb of 1-7, a cub with `bred` and 10 health when it is weak; bamboo 12 blocks off or none gives no cub, each parent sulks once in its love with two `cant_breed` sounds 15 ticks apart).
  - `a_hit_panda_bites_back_once_unless_aggressive` (one bite for 6 as `Melee` with the bite sound, then no target and `did_bite` cleared; an aggressive panda keeps biting), `only_aggressive_pandas_join_a_fight` (within 16 blocks), `bamboo_calms_an_angry_panda`.
  - `a_panda_panics_from_fire_and_cactus_but_not_from_a_punch` (fire, burning, lava, cactus and a hot floor at 2.0; not a punch, a bite, an arrow or a fall).
  - `a_worried_panda_runs_from_players_within_eight_and_monsters_within_four` (not from creative players or slimes; other genes never run).
  - `bamboo_grows_a_cub_and_brings_an_adult_into_love` (love 600 and event 18; not again in love; a cub grows a tenth; a locked cub, a parent, wheat and an adult's golden dandelion pass), `a_tempted_panda_follows_bamboo` (in the off hand; wheat does not), `a_panda_refuses_a_lead`, `aggressive_and_worried_pandas_have_their_own_ambient_sounds`.
- **Decisions:**
  - Goals 7, 8 and 12 (sit, lie on back, sneeze, roll) come with their groups and go in at their vanilla places in the list. The flags they set are in `PandaGoals` already, so `can_perform_action` and the goals that read it are final now.
  - `PandaAttackGoal` and both `PandaAvoidGoal`s are one generic wrapper (`Gated`) that tests its gate before the wrapped goal, as vanilla's `&&` does, so the melee goal's 20-tick check and the avoid search only run when the gate holds.
  - The breed goal's look target goes into the shared `look_at_player` state, which only `PandaLookAtPlayerGoal` uses.
  - Genes from the parents only in the two-parent form: a panda breeds only with a panda, so vanilla's one-parent branch (`b == null`) is never reached.
  - The bred cub's random is seeded as the rabbit's (`born_seeds`, else the seed uniquifier); its gene draws and mutations come from it, the parents' picks from their own randoms, and the orb from the breeding parent's random after that, in vanilla's order.
  - `bred` marks a cub whose weak or lazy attributes came from breeding, so the terrain saves `attributes` for it (commit 2). Loaded and spawned pandas keep their tag's `attributes` (decision 3).
  - Event 18 goes through E1's `broadcast_entity_event` (merged with e1b), once per feeding and once per breeding. D0's generic interact arm posts none for registered families, so there is no double post.
  - The held stack, its drop chance (0.085; a pickup's guaranteed 2.0) and `can_pick_up_loot` (the constructor's true) are in the state now so commit 2 saves and loads them; the pickup, eating, the drop when it stops sitting and the death drop come with `pandas-eating`.
  - The unhappy tail's `lookAt(target, 90, 90)` turns the body and pitch through `monster_ai::look_at` after the move, as vanilla's tail runs after `super.tick()`, and copies the yaw and pitch to the core.
  - The worried panda's monsters are `animal_goals::is_monster_class` (M1-1).
  - The breeding test keeps its pandas in a walled pen, since a panda that strolls within 7 blocks of the bamboo would breed (vanilla asks again at every full goal update).
  - Until commit 2 registers the `TerrainFamily`, D0's terrain test `server_mobs::tests::every_registered_kind_is_simulated_and_summonable` sees the two registries differ, as with rabbits-live 2. The vendored crates are green; in-game nothing changes (the terrain spawns no kind it does not register).
- **Runs:** `test -p minecraftoss-entities --locked --lib` (572 passed; the first panda run failed two statistical tests that reseeded `LegacyRandom` with 0, 1, 2, ..., whose first draws are close together; they now draw from one stream), `check -p launcher --locked` (green; only the old render_frame and skate warnings; the tree also held commit 2's world-crate row, which nothing else uses). The new files pass `rustfmt --check --edition 2021` on their staged content.
- **Test steps:** nothing new in-game yet (the terrain registers pandas in the next commit); the steps are those of commit 2.
- **Kit:** none.

#### 2. Minecraft pandas spawn in bamboo jungles and are saved, hatched and seen by friends

After 58f52911.

- **javap this round:** `Panda.finalizeSpawn` (main, then hidden gene from the level random; `setAttributes`; `AgeableMobGroupData(0.2)` when there is no group data; then `Animal.finalizeSpawn`), `setAttributes` (weak: `max_health` base 10, which clamps the health through `onAttributeUpdated`; lazy: `movement_speed` base `0.07000000029802322`), `addAdditionalSaveData`/`readAdditionalSaveData` (`MainGene` and `HiddenGene` by name, unknown or missing read as `normal`), the constructor (`CanPickUpLoot` true), `EntityTypes.PANDA` (`clientTrackingRange(10)`). The built-in size row (1.3 x 1.25, `NO_RESTRICTIONS`, `MOTION_BLOCKING_NO_LEAVES`, the animal rules) and the creature category match 26.3.
- **Changed (vendored world crate, `natural_spawner/families/land.rs`):**
  - The row `minecraft:panda`: today's size row (1.3 x 1.25, `NoRestrictions`, `MotionBlockingNoLeaves`, not fire immune, `Rules::Animal`, `Walk::Animal`, kept when far, cluster 4), `MobCategory::Creature`, follow range 16, `baby_chance: Some(0.2)`, no rule, `finalize: Some(panda_finalize)`. It overrides the built-in row, so pandas no longer reach the catch-all `note_unsupported("finalizeSpawn of minecraft:panda")` with a 0.05 baby chance.
  - `finalize_panda`: two genes from the level random through `PANDA_GENES` (the 16-slot table); a weak variant gets a `max_health` base of 10 and `Health` 10, a lazy one a `movement_speed` base of 0.07 (`set_attribute_base`).
  - The rows are named consts now (`ROWS = &[PANDA, POLAR_BEAR]`), so each later family adds one const and one name without moving the others' lines.
- **Changed (game code, `minecraft_terrain`):**
  - New `families/land/pandas.rs`: `impl TerrainFamily for Panda` with tracking range 10, `VOLATILE` `equipment`, `drop_chances` and `ForcedAge`, shadow 0.9, no textures yet and an empty `draw` (the models come with `pandas-drawn`).
  - `read`: the saved yaw for its body and head; `MainGene` and `HiddenGene` (unknown or missing: normal); `Age`, `ForcedAge`, `AgeLocked`, `InLove` and a friend's `Baby`; `CanPickUpLoot` (true when missing, the constructor's); `equipment.mainhand` and `drop_chances.mainhand` (0.085 when missing); the `max_health` and `movement_speed` bases from `attributes` (20 and 0.15 when missing; the health clamped to the maximum).
  - `fields`: the same keys; `equipment` and `drop_chances` always (empty when it holds nothing or its chance is the default, so a stack it no longer holds is not kept from the loaded tag); `attributes` (both bases) only for a bred cub.
  - `show`/`show_on`: flag bits 0 sitting, 1 on its back, 2 rolling, 3 sneezing, 4 scared; `values[2]` the unhappy counter; `values[4]` the held item (0 none, 1 bamboo, 2 cake, 3 anything else, which friends do not draw). The sneeze, eat and roll counters (`values[0]`, `[1]`, `[3]`) and the amounts join with their groups.
  - `interact`: the entity `pandas::interact`; a pass leaves the click unhandled, anything else changes the inventory, and the golden dandelion plays its use or unuse sound.
  - `families/land.rs`: `mod pandas;` and `pandas::Panda` in `terrain_group!`, between the rabbits and the polar bears.
- **Tests added:**
  - World: `natural_spawner::families::land::tests::a_panda_spawns_with_two_genes_and_later_group_members_are_cubs_one_time_in_five` (the row; 3200 finalizes: the gene table, weak with `max_health` 10 and `Health` 10, lazy with speed 0.07, others with no `attributes`; 80 natural spawns in a bamboo jungle with no `note_unsupported`, every panda with two known genes, the first of each group an adult and 10-30% of the later ones cubs; a summoned panda an adult with genes).
  - Terrain (`families::land::pandas::tests`): `a_panda_saves_and_loads_with_its_genes_held_item_love_and_attributes` (a weak cub with 10 health, speed 0.07, no loot pickup, three bamboo held and a guaranteed drop round trips with its `attributes` kept as loaded; missing and unknown genes; an empty paw writes no `mainhand`; a bred weak cub writes both bases and loads with 10 health), `a_summoned_weak_panda_keeps_twenty_health` (NBT skips the finalize), `a_friend_sees_a_panda_from_its_look_tag` (no volatile key in the look, its genes and `Baby` in it, range 10, every flag, the unhappy counter and the held item through `FamilyShow` and back), `bamboo_from_the_hand_brings_a_panda_into_love` (through D0's interact arm: wheat passes, one bamboo used, love 600, one event 18).
- **Decisions:**
  - Decision 3 as recommended: weak and lazy live in the saved `attributes`, set only by the finalize and by breeding, never derived from the genes at load; a `/summon` with genes keeps 20 health and 0.15 speed, as in vanilla.
  - `fields` writes `attributes` only for a bred cub (`Panda.bred`), so a loaded panda keeps its whole list, including the follow-range spawn bonus.
  - `VOLATILE` also holds `ForcedAge`, which counts down while a fed cub grows, as for rabbits and polar bears.
  - The friend's mirror draws only bamboo and cake in the paw; anything else from a loaded tag is shown as nothing (`values[4]` 3).
  - The row keeps `Rules::Animal` (the shared animal rules, which the built-in row used) rather than a family rule: vanilla's panda has no rule of its own.
- **Protocol:** no message or codec change; the panda's state travels in `FamilyShow`'s existing flags and values.
- **Runs:** `test -p minecraftoss-world --locked --lib` (136 passed; the first run reseeded `LegacyRandom` per draw like commit 1's first test run and missed a gene, so it now draws from one stream), `test -p minecraft_terrain --locked` (1700 passed, 3 ignored; D0's registry, eggs-survey and cage tests see the panda), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new terrain file passes `rustfmt --check --edition 2021` on its staged content; the world group file keeps `rustfmt --check` clean.
- **Test steps** (pandas are live but not drawn yet, so each shows as its round shadow: a big one about a block and a half wide, a small one for a cub). Release build, survival, **Normal**, by day on flat grass. Name each screenshot by its label.
  1. Use the panda egg three times on grass 8 blocks away: three big round shadows amble slowly and never run from you. **m1-pandas-live-1**
  2. `/summon minecraft:panda ~ ~ ~3 {MainGene:"worried",HiddenGene:"worried"}`. Walk at its shadow: within about 8 blocks it runs away fast. Use the zombie egg 3 blocks from it: it runs from the zombie too. An egg panda never runs from you.
  3. `/summon minecraft:panda ~2 ~ ~6 {MainGene:"aggressive",HiddenGene:"aggressive"}` twice, next to an egg panda. Punch the egg panda's shadow: it comes at you and bites once (about 3 hearts), then wanders off; the two aggressive shadows keep attacking. Right-click one attacker with bamboo: it stops attacking. **m1-pandas-live-2**
  4. Put a bamboo shoot on grass 4 blocks from two egg pandas and bone-meal it until it is a stalk. Right-click both pandas with bamboo: they meet, experience orbs pop out and a small shadow appears beside them. **m1-pandas-live-3**
  5. Right-click the small shadow with bamboo: one is used. Right-click it with the golden dandelion: it is used, with its sound.
  6. Break the stalk, walk 20 blocks away, and right-click two other egg pandas with bamboo: each makes a sad sound twice, under a second apart, and no cub comes.
  7. Set the grass next to a panda alight with flint and steel: its shadow runs. Punching a panda never makes it run.
  8. Use the panda egg ten times more and hit each with the iron sword, not jumping: about one in ten dies after two hits, the rest take four. Each adult drops bamboo and experience; the small shadow drops nothing.
  9. `/locate biome minecraft:bamboo_jungle`, `/tp` there and walk into new chunks: big shadows in ones and twos among the bamboo, now and then a small one. **m1-pandas-live-4**
  10. Save, quit and load: the shadows are where they were, the small one still small.
  11. Two windows (`scripts\mp-local.ps1`): the friend sees the same shadows. The friend feeds two pandas bamboo next to a bamboo stalk: the host sees them meet and a small shadow appear. The friend punches an aggressive panda: it goes for the friend in both windows. **m1-pandas-live-5-host**, **m1-pandas-live-5-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:panda_spawn_egg*16,minecraft:bamboo*64,minecraft:bone_meal*16,minecraft:golden_dandelion*2,minecraft:iron_sword,minecraft:flint_and_steel,minecraft:zombie_spawn_egg*2,minecraft:cooked_beef*16
```

### Group pandas-drawn

#### 1. Minecraft pandas are drawn like vanilla

After 85551190.

- **javap this round** (`PandaModel`, `BabyPandaModel`, `PandaRenderer`, `PandaRenderState`, `PandaHoldsItemLayer`, `HoldingEntityRenderState`, `Panda.tick`, `updateSitAmount`, `updateOnBackAnimation`, `updateRollAmount`, `getSitAmount`, `isEating`, `eat`, `sneeze`, `Mth.rotLerpRad`, `PoseStack.rotateDegrees`, `LivingEntityRenderer.render`): the M1-3 "Client" numbers hold. Added or corrected on the way:
  - `PandaModel.createBodyLayer` (64 x 64): `head` at (0, 11.5, -17) with four cubes in one part (the head, the nose and both ears share uv 52, 25, none mirrored), `body` at (0, 10, 0) turned π/2 about x, four legs (uv 40, 0; 6 x 9 x 6). 6 parts, 9 cubes.
  - `BabyPandaModel.createBodyLayer` (64 x 64): the legs are **3 x 2 x 3** (from -1.5, 0, -1.5), not 3 x 3 x 3; the body is not turned. Its `animateSitting` replaces the adult's whole (no head turn): body xRot towards 0.17453292, then the lerps of the body z, head z and y and the leg z.
  - `PandaRenderer.extractRenderState`: `isUnhappy` is the unhappy counter above 0; `sneezeTime` the sneeze counter; `isEating` is the eat counter above 0 (`Panda.isEating`, a synched counter, no flag); `rollAmount` is 0 for a cub; `rollTime` is `rollCounter + partial` only while the counter is above 0. Textures by `getVariant`, the baby map for cubs.
  - `setupRotations`, after the body yaw and the death tilt: the roll (`translate` then `rotateDegrees(XP, -angle)`), then the sit (with the scared tremble and the cub's shift inside it), then the back, all before `scale(-1, -1, 1)` and the 1.501 drop. `xRot` there is the entity's lerped pitch.
  - `PandaHoldsItemLayer`: the main-hand stack (`HoldingEntityRenderState`, display `GROUND`) only when it is not empty, the panda sits (the flag, not the amount) and is not scared; `translate(0.1, y, z)` from (1.4, -0.6), less `0.09 sin(0.6 age)` in y and `0.2 sin(0.6 age) + 0.2` in z while eating; drawn on no part.
  - `Panda.tick`, after the sneeze and roll counters: sitting sets `xRot` 0; then `updateSitAmount`, `handleEating`, `updateOnBackAnimation`, `updateRollAmount`, each `old = now`, then +0.15 up to 1 while its flag is set, else -0.19 down to 0; the getters lerp `old` to `now` by the partial tick.
- **Changed (vendored entities crate):**
  - `land/pandas.rs`: `Amount` (`old`, `now`, `step(on)`, `at(partial)`), the panda's `sit_amount`, `back_amount` and `roll_amount`, `Panda::step_amounts` (steps all three from the sit, back and roll flags), and `animate`, the tick's last step: a sitting panda's pitch is 0 (its core look and its look control), then `step_amounts`.
  - `land/pandas/ai.rs`: `PandaGoals` gains `sneeze_counter`, `eat_counter` and `roll_counter` beside `unhappy`; the `eating` flag becomes `eating()`, the eat counter above 0, as vanilla's `isEating` (`can_perform_action` reads it).
- **Changed (game code, `minecraft_terrain`):**
  - New `families/land/pandas/render.rs`: both models as part tables in `QuadrupedModel`'s order; the 14 textures (the seven genes in `Gene` id order, adults then cubs) and `texture(variant, baby)`; `Stance`; `setup_anim` (the quadruped head look and leg swing, then vanilla's unhappy shake, sneeze, sit with the cub's own `animateSitting`, eating, scared, back and roll lines, with `rot_lerp_rad` as `Mth.rotLerpRad` and the table `Mth.sin`); `roll_turn` (the four 8-tick quarters, their lifts and angles); `body_turns` (roll, sit with the tremble and the cub's shift, back); `turned_root` and `model_root` (the 90-degree flip, the turns, the 1.501 lift and the mirror); `held_pose`; `stance` (the amounts and the roll time lerped by the partial tick, the roll amount 0 for cubs); `append_pandas` (the skin lit at the eye, the hurt overlay, the held item into `held` with `HeldDisplay::Ground` lit at the eye).
  - `families/land/pandas.rs`: `mod render;`, `TEXTURES` from `render`, `CAGE_MARGIN` 0.11; `FamilyShow` values [0] sneeze counter, [1] eat counter and [3] roll counter join [2] and [4]; `show_on` sets them and steps the amounts once per host tick, as vanilla's client does; `draw` puts the living pandas into `out.culled` and their held items into `out.held`.
- **Tests added:**
  - Entities: `world::families::land::pandas::tests::the_sit_back_and_roll_amounts_ease_in_and_out_like_vanilla` (through real ticks: 0.15 a tick up to 1 and 0.19 down to 0, each amount only from its own flag, `old` the last tick's value and `at` the lerp; a sitting panda's pitch 0 in both looks; `eating()` follows the eat counter and blocks `can_perform_action`).
  - Terrain (`families::land::pandas::render::tests`): `an_adult_and_a_cub_have_nine_cubes_each_on_their_own_models` (cubes, names, no mirror, pivots, the body lying along the panda; drawn: 54 faces into `culled` only, the paws on the ground, the top 1.282 up, 2.25 from nose to rump and 1.19 wide, the skin and light; the cub 54 faces, 0.56 tall, 1.06 long, its own skin), `each_gene_and_age_draws_its_texture` (every name, 14 distinct, and drawn skins for weak, a hidden weak shown plain, lazy, worried, and brown, aggressive and playful cubs), `a_sitting_panda_sits_upright_and_an_eating_one_works_its_paws` (the sit turns and leg z turns, half a sit, the cub's moves; the back upright within 10 degrees and the head still looking ahead; chewing head and paws only while sitting; drawn: partial 0 of the first tick still standing, a full sit with the ears 2.36 up and the rump 0.07 into the ground as in vanilla; a friend given the host's `FamilyShow` each tick draws the same mesh and held item as the host at partial 0, 0.25, 0.5 and 1), `a_rolling_panda_turns_a_full_circle_in_thirty_two_ticks` (the angle and lift at nine times, none at 0 and from 32 on, never turning back, a full circle, the cub's lower lift; the roll amount's head and leg kicks; drawn upside down 1.6 up at 15.5 ticks, a cub 0.6 up with no head roll), `a_panda_on_its_back_and_a_scared_panda_pose_like_vanilla` (the kicking legs and head, half on its back, the paws 1.3 up and the spine on the ground, a cub 0.5 up; drawn; the scared face and paws even while eating, the tremble's angle, the cub's shift), `a_sneezing_panda_holds_its_head_back_from_tick_fifteen` (0, 7, 14, 15-19 held by the integer division, 20 and not sneezing), `an_unhappy_panda_shakes_its_head_and_paws`, `a_sitting_panda_holds_its_item_on_the_ground_display_unless_scared` (the layer's offsets and chewing; drawn: none standing, the bamboo 1.4 up and 0.9 in front of the feet on the ground display lit at the eye, the same for the friend, none when scared on both, a cake for the friend, a stick for the host only, none with an empty paw).
  - Terrain `families::land::pandas::tests::a_friend_sees_a_panda_from_its_look_tag` now also carries the three counters and checks that `show_on` steps the amounts once.
- **Decisions:**
  - The amounts live on the entity `Panda` and are stepped at the very end of the host's tick (`animate`) from the tick's final flags, and by `show_on` from the shown flags on a friend. A vanilla client steps its own amounts from the flags the server synced at the end of its tick, so host and friend draw the same pose as a vanilla client would. `pandas-eating` keeps `animate` last, after its `handleEating`.
  - The sneeze, eat and roll counters sit in `PandaGoals` beside the unhappy counter, so `show`/`show_on` read and write one place; the later groups count them (`pandas-eating`, `pandas-moods`). `eating` became the eat counter, as in vanilla, rather than a flag kept beside a counter.
  - A sitting panda's pitch is set to 0 in `animate`, as vanilla's `tick` does: the sit turn adds `xRot`, and the pitch reaches friends through the tracked look.
  - `CAGE_MARGIN` 0.11: at the cage's scale (0.53125 / 1.3) and its 30-degree tilt, the corner of the panda's rump pokes up to 0.099 out of the cell at a 200-degree spin, as in vanilla's spawner. D0's cage test failed with the default 0.05.
  - The lead keeps the fork's default offset: pandas cannot be leashed.
  - The scared tremble uses f64 `cos`, as vanilla's `Math.cos`; every model sine uses the table `Mth.sin` (`minecraftoss_player::mth::sin`).
- **Protocol:** no message or codec change; the three counters use `FamilyShow`'s existing values.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (573 passed), `test -p minecraft_terrain --locked` (1708 passed, 3 ignored; the first targeted run failed only D0's cage test, fixed by the margin above), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new `render.rs` passes `rustfmt --check --edition 2021` on its staged content.
- **Test steps** (no behaviour sets sitting, rolling, sneezing or lying on the back yet; those come with `pandas-eating`, `pandas-moods` and `pandas-thunder`, so these steps check the models, the genes' faces, the cubs, the sulk and friends). Release build, survival, **Normal**, by day on flat grass. Name each screenshot by its label.
  1. Summon seven pandas 3 blocks apart: `/summon minecraft:panda ~ ~ ~3 {MainGene:"normal",HiddenGene:"normal"}`, then the same with `lazy`, `worried`, `playful`, `brown`, `weak` and `aggressive`. You see seven black-and-white pandas about 2.25 blocks long and 1.3 tall, each with its own face: plain, sleepy, frowning, tongue out, brown fur, runny nose, angry brows. **m1-pandas-drawn-1**
  2. `/summon minecraft:panda ~ ~ ~3 {MainGene:"weak",HiddenGene:"normal"}`: a plain face, since weak is recessive.
  3. `/summon minecraft:panda ~2 ~ ~5 {Age:-24000,MainGene:"brown",HiddenGene:"brown"}` and `/summon minecraft:panda ~4 ~ ~5 {Age:-24000,MainGene:"playful",HiddenGene:"playful"}`: two cubs with their own rounder model (a big head, short stubby legs, about half a block tall), a brown cub and a playful cub with its tongue out, not shrunken adults. **m1-pandas-drawn-2**
  4. Walk around them: their legs swing as they amble, and their heads turn to look at you within 6 blocks.
  5. Use the panda egg twice side by side, 8 blocks away, with no bamboo stalk around, and right-click both new pandas with bamboo: each turns to you, shakes its head side to side and paws the air for about a second and a half, with the sad sound twice. **m1-pandas-drawn-3**
  6. Hit a panda with the iron sword: it flashes red; kill it and it tips over sideways.
  7. In creative, place the spawner and use a panda egg on it: a small panda turns inside the cage, its rump poking a little out of the cage at times, as in vanilla. **m1-pandas-drawn-4** Switch back to survival.
  8. Save, quit and load: the same faces, the cubs still cubs.
  9. Two windows (`scripts\mp-local.ps1`), side by side: the friend sees the same faces and cubs. The friend feeds two adults bamboo far from a stalk: both windows show the head shake at the same moment. **m1-pandas-drawn-5-host**, **m1-pandas-drawn-5-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:panda_spawn_egg*16,minecraft:bamboo*64,minecraft:iron_sword,minecraft:spawner,minecraft:cooked_beef*16
```

### Group pandas-eating

#### 1. Registered Minecraft mobs can hold still while their goals run

After b6ec8731.

- **javap this round:** `Panda$PandaMoveControl.tick` is `if (panda.canPerformAction()) super.tick();` and nothing else, so a panda that cannot act keeps its yaw, its speed and its inputs (which `LivingEntity` fades by 0.98 a tick), and its `MoveControl` keeps its operation and wanted position for when it can act again.
- **Changed (vendored entities crate):**
  - `monster_ai.rs`: `MonsterAi.still: Option<fn(&MonsterGoalContext) -> bool>` after `jumping`, `None` in `of_kind`.
  - `world.rs` (`monster_ai_step`): after the goals, the navigation and the 0.98 fade, a mob whose `still` answers true gets a `MoveFrame` of its own yaw, speed and inputs instead of a `move_control.tick`. Every other mob runs exactly as before.
- **Tests added:** `world::families::tests::a_registered_mob_held_still_keeps_its_heading_and_drifts_to_a_stop` (on the probe with its goals removed, a wanted position to the east and an input of 0.3: free, it turns to 270 and stops the next tick; held, it keeps yaw 0 for 300 ticks, its input fades by exactly 0.98 a tick, it moves only straight ahead, speeds up, then slows every tick to under 0.001 a tick, and its move control still holds `MoveTo`; let go, it turns east and walks at 0.3 at once).
- **Decisions:** as M1-3 decision 1: the hook is a field on `MonsterAi` set by the family, checked after the goals, not a `MonsterSpec` field (which would break every other track's spec literal) and not a panda AI step of its own (which would copy `monster_ai_step` and D0's sync). The check sits after the input fade, as vanilla's `applyInput` fades the inputs whether or not the move control ticks.
- **Runs:** `test -p minecraftoss-entities --locked --lib families::tests` (22 passed), `check -p launcher --locked` on this commit alone (green; only the old render_frame and skate warnings). The new test keeps the root test file's `rustfmt --check` output to the two diffs it already had.
- **Test steps:** nothing new to see; no mob holds still until the pandas sit down (commit 3). The silverfish and endermite steps of rabbits-live 1 still hold.
- **Kit:** none.

#### 2. Registered Minecraft mobs can scatter item crumbs

After f1972742.

- **Checked first:** a host and a friend each run `EntityView::advance` (survival `entity_view.rs`), which builds D0's `ClientFx` once per client tick and calls `families::client_effects`; both views' `item_crumbs` lists are drained by `present_crumbs` (`render_anim/src/minecraft_world.rs:1241` for the host's window, `minecraft_remote.rs:2002` for a friend's), so a field in `ClientFx` that lends the view's own list reaches every viewer with no other change.
- **Changed (game code):**
  - `crates/minecraft_terrain/src/families.rs`: `ClientFx.item_crumbs` after `poof`; `glam::Vec3` imported.
  - `crates/minecraft_survival/src/entity_view.rs`: the `ClientFx` literal in `advance` lends `&mut self.item_crumbs` (one line, logged as a hook).
  - The two other `ClientFx` literals on this branch gain `item_crumbs`: D0's `families::tests::client_effects_forget_registered_mobs_that_are_gone` and track D's breeze test in `families/trial/breezes.rs` (`a_breeze_whirls_now_and_then_for_every_viewer`). No behaviour changes.
- **Tests added:** terrain `families::tests::client_effects_lend_the_viewers_item_crumbs_to_every_family` (a viewer's crumbs survive twenty client ticks over silverfish and endermites, which scatter none). The end-to-end check that a family's crumbs reach the host and a friend comes with the first family that scatters them: `an_eating_panda_scatters_six_crumbs_every_five_ticks` in commit 3 (decision below).
- **Decisions:**
  - As M1-3 decision 5: one field into the existing item-crumb path, not a new particle kind and not a second list.
  - The brief's test name for this hook, `a_registered_family_can_scatter_item_crumbs_for_every_viewer`, needs a family that scatters crumbs, and none does before commit 3 (the terrain crate has no test family; the entity crate's probe is test-only). The hook commit carries the pass-through test above, and the panda's terrain test in commit 3 checks the host's and a friend's crumbs at the same ticks.
- **Merge note:** `minecraft-survival` holds two more `ClientFx` literals that this branch does not: W's warden test helper (`families/deep_dark/wardens.rs`, `effects`) and F's boss test (`families/bosses/elden_bosses.rs`, `effects`). Whichever merges second adds `item_crumbs: &mut <a new Vec>` to them (one line each, beside `poof`); without it only the terrain crate's tests fail to compile.
- **Runs:** see commit 3 (`test -p minecraft_terrain --locked` covers this commit's test; `check -p launcher --locked` on this commit alone: green, only the old render_frame and skate warnings). The edited terrain files keep `rustfmt --check` clean.
- **Test steps:** nothing new to see until the pandas eat (commit 3): eggs thrown at a wall and an eye of ender breaking still scatter their crumbs, for the host and a friend.
- **Kit:** none.

#### 3. Minecraft pandas sit down to eat bamboo and cake

After 8f047f0d.

- **javap this round** (`Panda$PandaSitGoal`, `Panda.tryToSit`, `handleEating`, `addEatingParticles`, `pickUpItem`, `canPickUpAndEat`, `mobInteract`, `hurtServer`, `tick`; `Mob.aiStep`, `wantsToPickUp`, `canHoldItem`, `getPickupReach`, `setGuaranteedDrop`, `dropCustomDeathLoot`, `isEffectiveAi`; `LivingEntity.dropAllDeathLoot`, `shouldDropLoot`; `Vec3.xRot`, `yRot`): the M1-3 numbers hold. Added or corrected on the way:
  - **Sit goal** (priority 7, `MOVE`). `canUse`: its cooldown not above `tickCount`, adult, not in water, `canPerformAction`, unhappy counter 0; then a held item, or any item entity in its box grown by 6 that `canPickUpAndEat` (`#panda_eats_from_ground`, alive, no pickup delay). `canContinueToUse`: not in water, then (lazy, or `nextInt(300) != 1`) and `nextInt(1000) != 1` (the `reducedTickDelay` of 600 and 2000). `start`: with an empty paw it walks to the first such item in its box grown by 8 (`moveTo(entity, 1.2000000476837158)`, reach 1), else `tryToSit`; the cooldown becomes 0 either way. `tick`: holding and not sitting → `tryToSit`. `stop`: a held stack is dropped (`spawnAtLocation`), the paw emptied and the cooldown set to `tickCount + (10 + nextInt(lazy ? 50 : 150)) * 20`; with nothing held there is no cooldown; then `sit(false)`.
  - **`tryToSit`**: not in water → `setZza(0)`, the navigation stops, `sit(true)`.
  - **`handleEating`** (after `updateSitAmount`): not eating, sitting, not scared, holding and `nextInt(80) == 1` → `eat(true)` (counter 1); else an empty paw or not sitting → `eat(false)`. While eating: `addEatingParticles`, then on the server, with the counter above 80 and `nextInt(20) == 1`: above 100 with a `#panda_eats_from_ground` item the paw empties, game event `eat`, `sit(false)`; then `eat(false)` and return. Otherwise the counter counts up.
  - **`addEatingParticles`**, every counter that is a multiple of 5: `entity.panda.eat` at volume `0.5 + 0.5 * nextInt(2)` and pitch `(r1 - r2) * 0.2 + 1`; then, holding, six `ITEM` particles of the held stack, each drawing six floats from the mob's random, on the server too (where `addParticle` does nothing): motion `((r - 0.5) * 0.1, r * 0.1 + 0.1, (r - 0.5) * 0.1)` turned by `-xRot` then `-yRot`; position `((r - 0.5) * 0.8, -r * 0.6 - 0.3, 1 + (r - 0.5) * 0.4)` turned by `-yBodyRot`, plus `(x, eyeY + 1, z)`; added with the motion's y plus 0.05. `Vec3.xRot` is `(x, y cos + z sin, z cos - y sin)` and `yRot` `(x cos + z sin, y, z cos - x sin)`, with `Mth.sin`/`cos`.
  - **Pickup**: `Mob.aiStep`, after `LivingEntity.aiStep`: `canPickUpLoot`, alive and `mob_griefing`; item entities in its box grown by (1, 0, 1), not removed, not empty, no pickup delay, `wantsToPickUp` (`canHoldItem`, true). `Panda.pickUpItem`: an empty paw and `canPickUpAndEat` → the whole stack into the main hand, `setGuaranteedDrop` (2.0), `take`, `discard`. There is no age check, and NoAI does not stop it (`isEffectiveAi` is the only NoAI test).
  - **`mobInteract`**, a `#panda_food` item that neither grows a cub nor brings love (an adult in love, or a parent): on the server, not sitting and not in water → `tryToSit`, `eat(true)`, the old held stack dropped unless the player has infinite materials, `new ItemStack(item, 1)` in the paw (the drop chance unchanged), one used; `SUCCESS_SERVER`. Sitting or in water → `PASS`.
  - **Death**: `dropAllDeathLoot` calls `dropCustomDeathLoot` only when `shouldDropLoot` (not a baby, `mob_drops`); a chance of 0 skips; a guaranteed slot (chance above 1) always drops; otherwise only after a player's hit and `nextFloat() < 0.085`.
  - **`Panda.tick`**: the worried panda's `else if (!isEating()) sit(false)` runs on every tick it is not thundering, so a worried panda never stays sat unless it is eating.
- **Changed (vendored entities crate):**
  - `land/pandas/ai.rs`: `PandaSitGoal` at priority 7, between the avoid goals and the look goal (`GOAL_NAMES` now 12); `try_to_sit`; `within` (an item's box within reach of the panda's); `held_still`, H1's answer (`!can_perform_action`); `PandaGoals` gains `holding`, `food` (the edible item entities within 8 that the tick hands the goal), `sat_down` and `dropped` (where the stop let go).
  - `land/pandas.rs`: `spawn` sets `ai.still`; the tick hands the goals `holding` and `food`, then after the step `settle` (zero the forward input after a goal sat the panda down; drop the held stack where the goal let go, with the level random's push and pickup delay 10), after `family_after_move` the pickup (`pick_up`), then `grow`, `worry` (the non-thunder half of the worried rule), the unhappy tail, `face_ahead` (pitch 0 while sitting), `handle_eating`, the amounts and the `eat` game event; `death_drops`; `interact`'s eat-from-hand branch, which now returns the old item as a fourth value; `eats_from_ground`, `can_pick_up_and_eat`.
- **Changed (game code, `minecraft_terrain`):**
  - `families/land/pandas.rs`: `interact` puts the old item into `outcome.drops` at the panda's feet (the server spawns it with `spawn_at_location`); `client_effects` scatters six crumbs of the held item into H2's `item_crumbs` when the shown eat counter reaches a multiple of 5 (`fx.seen` keeps the last counter each viewer saw, so each burst fires once per viewer); `eating_crumbs` with `x_rot` and `y_rot`.
- **Tests added:**
  - Entities (`world::families::land::pandas::tests`):
    - `a_panda_picks_up_a_whole_stack_of_bamboo_or_a_cake_only_with_mob_griefing_and_an_empty_paw` (NoAI pandas: nothing without `mob_griefing`; five bamboo taken whole with a guaranteed drop; a cake; not wheat, not out of reach, not before the pickup delay ends; not with a full paw or without `CanPickUpLoot`).
    - `a_sitting_panda_eats_after_eighty_ticks_and_finishes_after_a_hundred` (24 hand-fed parents: a munch only at multiples of 5, volume 0.5 or 1, pitch 0.8-1.2, neutral; it sits still; a pause only past 80, the end only past 100, with the paw empty, standing and one `eat` game event; some pause and start again).
    - `feeding_a_panda_in_love_seats_it_with_one_bamboo_and_returns_its_old_item` (the cake comes back, one bamboo used and one in its paws, the guaranteed chance kept, still in love, no hearts, the forward input 0 and the path stopped; no second feed while it sits; a tempting player does not move it; a creative player's bamboo replaces the cake; in water it passes).
    - `a_panda_that_stops_sitting_drops_its_item_and_waits` (goal level, 40 seeds each: in water it gets up and lets go where it stood; it sits again after 200-3180 ticks, a lazy one after 200-1180; with nothing held it waits not; through the tick: a panda sat with a cake gets up when bamboo tempts it and the cake lies at its feet with pickup delay 10 and vanilla's push).
    - `a_held_item_drops_on_death_always_when_picked_up` (a picked-up stack drops whoever kills it; a cub's and one without `mob_drops` do not; a hand-fed bamboo only after a player's kill, 40-100 times in 800).
    - `a_worried_panda_does_not_settle_on_picked_up_bamboo` (600 ticks: a worried panda never ends a tick sitting and eats none of its three bamboo; a normal one eats).
  - Changed: `bamboo_grows_a_cub_and_brings_an_adult_into_love` (a panda in love and a parent now sit down to eat the bamboo instead of passing) and every `interact` call (the fourth value).
  - Terrain (`families::land::pandas::tests`): `feeding_a_panda_in_love_drops_its_old_item_where_it_sits` (through D0's interact arm: the cake in `outcome.drops` at its feet, one bamboo used, the shown sit flag, eat counter and bamboo; a creative player gets no drop and keeps the bamboo), `an_eating_panda_scatters_six_crumbs_every_five_ticks` (host and friend, the friend through `show_on`: six bamboo crumbs at eat counters 5, 10, 15 and again 5 after a stop, none twice for one counter, none for an empty paw; each 0.8-1.27 blocks out from the panda, 0.3-0.9 below its eyes plus one, moving up 0.15-0.25 and sideways at most 0.0708). The second is the end-to-end check of H2 (commit 2).
- **Decisions:**
  - `tryToSit`'s `setZza(0)` from inside the sit goal: goals cannot reach `MonsterAi.forward`, so the goal flags `sat_down` and the tick zeroes the input right after the step. On the tick a panda sits down its input fades by 2% instead of dropping to 0, about a tenth of a block of drift at most; feeding from the hand zeroes it at once. The brief's H1 keeps the move control's faded inputs, as vanilla does, so the zero belongs to the panda, not to the hook.
  - The goals cannot read item tags or item entities' tag membership, so the tick hands the sit goal `holding` and `food` (the item entities within 8 that `canPickUpAndEat`), only while the goal could start (adult, empty paw, able to act), and clears `food` after the step so the host's per-tick world copy stays small.
  - The sit goal's stop records where the panda stood; the tick spawns the dropped stack there with the entity world's level random push and pickup delay 10 (as piglins' `spawn_at_location`). The old item from feeding goes through `MobOutcome.drops` and the server level's `spawn_at_location`.
  - The six particles' 36 random floats are drawn from the panda's random on the host as on a vanilla server, so its random stays in step; each viewer's crumbs come from its own random.
  - The pickup also runs for NoAI pandas, as vanilla's `Mob.aiStep` does.
  - The death drop's "killed by a player" reads `damage.player_memory > 0`: D0's `take_deaths` takes the death record before `death_drops` runs, and the memory is what vanilla's `dropAllDeathLoot` reads.
  - A friend draws only bamboo and cake in a panda's paws (`FamilyShow` value 4), so another item from a loaded tag scatters crumbs for the host only.
  - W's `vibrations` is merged, so the finished meal posts the `eat` game event here (no line left for a later merge).
  - The worried rule's non-thunder half (`else if (!isEating()) sit(false)`) lands here, because it is what keeps a worried panda from settling on picked-up bamboo; `pandas-thunder` adds the thunder half in front of it.
- **Protocol:** no message or codec change; sitting, the eat counter and the held item already travel in `FamilyShow`.
- **Runs:** `test -p minecraftoss-entities --locked --lib pandas` (24 passed on the first run), then on the group's final tree `test -p minecraftoss-entities -p minecraft_survival --locked --lib` (580 and 817 passed), `test -p minecraft_terrain --locked` (1711 passed, 3 ignored; it also covers commit 2's test), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The panda files keep `rustfmt --check --edition 2021` clean (the entity tests file keeps the two diffs pandas-drawn left in the amounts test); no comments were added.
- **Test steps.** Release build, survival, **Normal**, by day on flat grass, `/gamerule mob_griefing true`, the kit below. Eggs never let a panda pick up items (see "Left for later"), so the pick-up steps use `/summon` with `CanPickUpLoot:1b`. Name each screenshot by its label.
  1. Use the panda egg on grass 4 blocks away and right-click it with bamboo: hearts. Right-click it with bamboo again: it sits up, holds one bamboo stalk in its paws and munches, with a crunching sound and little bamboo crumbs falling every quarter second. Walk around it holding bamboo: it stays sitting where it is. **m1-pandas-eating-1**
  2. Keep watching: within about 5 to 15 seconds the bamboo is gone and it stands up. Now and then one stops munching for a few seconds before it carries on. Repeat with another egg panda if the first one finishes too quickly to see.
  3. `/summon minecraft:panda ~ ~ ~3 {CanPickUpLoot:1b}`, step back 6 blocks and throw a whole stack of bamboo 4 blocks from it (Ctrl+Q): it walks over, picks up the whole stack, sits down and eats. When it is done the whole stack is gone, as in vanilla. **m1-pandas-eating-2**
  4. Throw a cake (Q) near it: it fetches the cake, sits and eats it the same way.
  5. `/gamerule mob_griefing false` and throw a bamboo next to it: it walks over the bamboo and leaves it. `/gamerule mob_griefing true`.
  6. While a `CanPickUpLoot` panda sits eating a thrown stack, stand 5 blocks away holding bamboo: it gets up, the rest of its stack pops out at its feet, and it follows you.
  7. `/summon minecraft:panda ~ ~ ~3 {InLove:600,equipment:{mainhand:{id:"minecraft:cake",count:1}}}` and right-click it with bamboo: the cake pops out onto the grass, and it sits down with one bamboo and munches. **m1-pandas-eating-3**
  8. `/summon minecraft:panda ~ ~ ~3 {MainGene:"worried",HiddenGene:"worried",CanPickUpLoot:1b}` and throw a bamboo at its feet: it picks it up but never sits down to eat it.
  9. `/summon minecraft:panda ~ ~ ~3 {CanPickUpLoot:1b}`, let it pick up a thrown stack of bamboo, and kill it with the iron sword while it eats: the stack falls out with its own bamboo drop and experience.
  10. Two windows (`scripts\mp-local.ps1`), side by side: the friend sees a fed panda sit with bamboo in its paws, the crumbs falling and the munching sound at the same moments as the host. The friend feeds an egg panda bamboo twice: the host sees it sit and eat. **m1-pandas-eating-4-host**, **m1-pandas-eating-4-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:panda_spawn_egg*8,minecraft:bamboo*64,minecraft:bamboo*64,minecraft:cake,minecraft:cake,minecraft:cake,minecraft:iron_sword,minecraft:cooked_beef*16
```

### Group pandas-moods

#### 1. Living Minecraft mobs can drop gifts from loot tables

After e7926cf6.

- **javap this round** (`LivingEntity.dropFromGiftLootTable`, `dropFromLootTable(ServerLevel, ResourceKey, Function, BiConsumer)`, `Entity.spawnAtLocation(ServerLevel, ItemStack)` and its `(.., float)` and `(.., Vec3)` forms; `Panda.afterSneeze`): a gift rolls its table with the gift parameters (the mob as `THIS_ENTITY`, its position as `ORIGIN`) from the table's random sequence, and hands every stack to the consumer; the panda's consumer is `this::spawnAtLocation`, a new `ItemEntity` at the mob's feet (offset 0) with `setDefaultPickUpDelay` (10) and the constructor's random push `(nextDouble * 0.2 - 0.1, 0.2, nextDouble * 0.2 - 0.1)`. Nothing drops for an empty roll; `dropFromGiftLootTable` itself checks no game rule. The 26.3 tables of type `minecraft:gift`: `gameplay/panda_sneeze` (a slime ball against an empty entry of weight 699), `armadillo_shed` (one scute), `sniffer_digging`, `turtle_grow`, `cat_morning_gift` and the hero of the village gifts.
- **Changed (vendored entities crate):**
  - `world/families.rs`: `FamilyGift { table, this_type, at }` before `Families`; `Families.gifts` (empty in `clone_where`); `EntityWorld::family_gift` and `take_family_gifts` after `family_strike`.
  - `loot.rs`: `EntityLootBook::roll_table(table, loot_type, this_type)` after `roll_hero_gift`.
- **Changed (game code, `minecraft_terrain`):**
  - `families.rs`: `GIFT` (`minecraft:gift`) and `drop_family_gifts(mobs, loot, level)` after `block_dust`.
  - `server.rs`: one line right after `self.drop_cat_gifts();` in the mob tick (M10's anchor; the turtle egg pass and `bees_to_level` come after it).
- **Tests added:**
  - Entities: `world::families::tests::a_queued_family_gift_waits_until_the_server_takes_it` (two gifts kept in order, none in a `clone_where` view, each taken once); `loot::tests::a_named_table_rolls_only_as_its_own_type` (with the 26.3 jar: the armadillo's shed always one scute; 7000 sneeze rolls give 3 to 20 slime balls and nothing else; the sneeze table asked as an entity table, an entity table asked as a gift and an unknown table give `None`; `roll_table` on the cat's gift table follows the same named stream as `roll_cat_gift`).
  - Terrain: `families::tests::a_queued_family_gift_rolls_its_table_and_drops_at_its_feet` (with the 26.3 jar and data: a shed gift drops one scute exactly at the queued feet with pickup delay 10, an upward push of 0.2 and a sideways one within 0.1; an entity table and an unknown table drop nothing; without a loot book the queue is drained and nothing drops; 7000 sneeze gifts drop 3 to 20 slime balls).
- **Decisions:**
  - The server's line calls the terrain root's `drop_family_gifts` with the server's mob world, loot book and level rather than a `ServerSim` method: `ServerSim`'s fields are private to `server.rs`, so a method's body could not live in `families.rs`, and the call keeps `server.rs` to the one line M10 names.
  - The queue lives on the root `Families`, not on `EntityWorld` (which would need a `world.rs` field and `Default` line, a never-edit file): `Families` is `Default`, so no literal outside the root changes.
  - The family gates the gift (the sneeze on `mob_drops`, as vanilla), not the hook: vanilla's `dropFromGiftLootTable` checks no rule and the armadillo's shedding ignores `mob_drops`.
  - The roll uses the loot book's named random sequence, as M1-3 decision 4 recommends, so gifts follow the world seed like the cat's and the hero's gifts.
- **Protocol:** no message or codec change; the dropped items reach friends as item entities, as every other drop does.
- **Runs:** `test -p minecraftoss-entities --locked --lib -- gift named_table` (5 passed, the cat's and the hero's gifts among them), `test -p minecraft_terrain --locked -- family_gift client_effects` (3 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new tests keep `rustfmt --check` of the root test files where it was (the entity root's two old diffs, the terrain root's none); `loot.rs` keeps its own long-line style. No comments were added.
- **Test steps:** nothing new to see until a panda cub sneezes (commit 2). Cats' morning gifts and the hero of the village's gifts drop as before.
- **Kit:** none.

#### 2. Minecraft panda cubs sneeze, lazy pandas lie on their backs and playful pandas roll

After 87275971.

- **javap this round** (`Panda$PandaLieOnBackGoal`, `Panda$PandaSneezeGoal`, `Panda$PandaRollGoal`, `Panda.registerGoals`, `tick`, `handleRoll`, `afterSneeze`, `sneeze`, `roll`, `setOnBack`, `isSneezing`, `isRolling`, `isOnBack`, `canPerformAction`, `mobInteract`; `LivingEntity.jumpFromGround`, `dropFromGiftLootTable`; `PlayerCloudParticle` and its `SneezeProvider`; `assets/minecraft/particles/sneeze.json`): the M1-3 numbers hold. Added or corrected on the way:
  - **Lie on back** (priority 8, no flags). `canUse`: its cooldown below `tickCount`, lazy, `canPerformAction`, `nextInt(reducedTickDelay(400)) == 1`. `canContinueToUse` is the sit goal's (dry, then lazy or `nextInt(300) != 1`, then `nextInt(1000) != 1`). `start`: on its back, cooldown 0. `stop`: off its back, cooldown `tickCount + 200`.
  - **Sneeze** (priority 8, no flags). `canUse`: a cub that `canPerformAction`; a weak one first rolls `nextInt(reducedTickDelay(500)) == 1`, and every cub that got that far (a weak one that missed too) then rolls `nextInt(reducedTickDelay(6000)) == 1`. `canContinueToUse` false; `start` sets the sneezing flag and keeps the counter.
  - **Roll** (priority 12, `MOVE`, `LOOK`, `JUMP`, not interruptable). `canUse`: a cub or a playful panda on the ground that `canPerformAction`; true at once, with no random draw, when the block at `blockPosition()` plus (`Mth.sign(-sin yRot)` if its size is over 0.5, -1, `Mth.sign(cos yRot)` if over 0.5) is air; then a playful one rolls `nextInt(reducedTickDelay(60)) == 1`, and every panda that got that far (a playful one that missed too) `nextInt(reducedTickDelay(500)) == 1`. `canContinueToUse` false; `start` sets the rolling flag.
  - **`Panda.tick`**, after the unhappy tail: sneezing → the counter counts up; above 20 → `sneeze(false)` (the counter back to 0) and `afterSneeze`; at 1 → `entity.panda.pre_sneeze` (1, 1). Rolling → `handleRoll`, else the roll counter is 0. Then sitting's pitch, the amounts and eating.
  - **`handleRoll`**: the counter counts up; above 32 → `roll(false)` and nothing else. On the server, counter 1: `rollDelta = (dx + (-sin(yRot) * s), 0, dz + cos(yRot) * s)` with `s` 0.2 (a cub 0.1), the sine and product in floats; the motion becomes `rollDelta + (0, 0.27, 0)`. Counters 7, 15 and 23: `(0, onGround ? 0.27 : dy, 0)`. Otherwise `(rollDelta.x, dy, rollDelta.z)`. No impulse flag.
  - **`afterSneeze`**: the `SNEEZE` particle at `(x - (w + 1) * 0.5 * sin(yBodyRot), eyeY - 0.1, z + (w + 1) * 0.5 * cos(yBodyRot))` with the motion `(dx, 0, dz)` (it draws only on a client, which runs the same tick); `entity.panda.sneeze` (1, 1); every `Panda` in its box grown by 10 that is not a baby, on the ground, not in water and `canPerformAction` calls `jumpFromGround` (`deltaMovement.y = max(getJumpPower(), dy)` when the power is above 1e-5, `needsSync`; the panda's jump strength is the default 0.42 times the block's jump factor); then, with `mob_drops`, `dropFromGiftLootTable(PANDA_SNEEZE, spawnAtLocation)`.
  - **`mobInteract`** on its back: `setOnBack(false)` and `SUCCESS` before any item check. The lie goal keeps running; its later stop starts the 200-tick cooldown.
  - **`PlayerCloudParticle`** through `SneezeProvider`: the base particle with no motion, friction 0.96, its velocity times 0.1 plus the given motion, one `nextFloat` for a grey that the provider replaces with (0.22, 1.0, 0.53) and alpha 0.4, the quad size times 1.875, lifetime `max((int)(8 / (r * 0.8 + 0.3)) * 2.5, 1)` (17 to 65 ticks), no physics, the translucent layer, a quad size that grows over the first 1/32 of its life, and the sprites `generic_7` to `generic_0` by age. Its tick also pulls it towards the feet of the nearest player within 2 blocks when it is above them ("Left for later").
- **Changed (vendored entities crate):**
  - `land/pandas/ai.rs`: `PandaLieOnBackGoal` and `PandaSneezeGoal` at priority 8 after the sit goal, `PandaRollGoal` at 12 after `RandomLookAroundGoal`, in vanilla's order (`GOAL_NAMES` now 15); `stays_down`, the sit goal's continue rule, now shared with the lie goal; `roll_ahead` (the block ahead's x and z steps).
  - `land/pandas.rs`: `Panda.roll_delta`; `sneeze` (the counter, the pre-sneeze sound), `after_sneeze` (the sound, the neighbours' jump through `Body::jump_power(level, 0.42, 1.0)`, H3's `family_gift(SNEEZE_GIFT, KIND, feet)` with `mob_drops`), `handle_roll`; the tail now runs the sneeze, `after_sneeze`, then the roll before the pitch, eating and the amounts, in vanilla's order.
- **Changed (game code, `minecraft_terrain`):**
  - `ambient_particles.rs`: `Kind::Sneeze` right after `Poof`; `Particle::sneeze` after `poof`; its arms next to `Poof`'s in `add` and the sprite list, in the translucent list and the growing size arm; it coasts with no physics.
  - `families/land/pandas.rs`: `sneeze_cloud` (the vanilla spot and motion); `client_effects` adds one `Kind::Sneeze` cloud when the shown sneeze counter drops; the viewer's `fx.seen` entry holds the eat counter in its low 16 bits and the sneeze counter above them.
- **Tests added:**
  - Entities (`world::families::land::pandas::tests`):
    - `a_weak_cub_sneezes_and_adults_within_ten_jump` (goal level over 20000 full updates: a weak cub 55-125 sneezes, a plain cub 1-18, an adult and a sitting cub none; through ticks: the counter 1 to 20 then 0, `pre_sneeze` on the first tick and `sneeze` on the 21st (neutral, 1, 1); adults 5 and 9.5 blocks off jump at 0.42 with `needs_sync`, while one 12 off, one 11 above, a sitting one, one in water, another cub and the sneezer do not; one `gameplay/panda_sneeze` gift at the cub's feet with `mob_drops`, none without).
    - `a_lazy_panda_lies_on_its_back_until_clicked` (only lazy pandas able to act; in water it gets up and waits over 200 ticks; through ticks a lazy panda lies down, keeps its heading for 40 ticks with the back amount at 1, and a click with an empty hand gets it up with `SuccessServer`, no events, no drop).
    - `a_playful_panda_rolls_with_three_hops_in_thirty_two_ticks` (goal level over 6000 updates: a playful adult 170-280 rolls, a plain cub 10-40, a playful cub over 150, a plain adult and a playful one in the air none; `roll_ahead`; through ticks: counters 1 to 32 then 33 off and 0, the heading kept, the roll delta 0.2 ahead, the hop at 1 and on landing at 15 and 23, no sideways motion at 7, 15 and 23, the roll delta in between, over 3 blocks covered, the roll amount easing out).
    - `a_playful_panda_rolls_off_a_ledge` (at a drop ahead a playful adult and a cub roll at every full update without a random draw, a plain adult never; through ticks a playful panda tumbles off the ledge).
    - The test world `Grove` gains a `ledge` (the ground drops to y 60 beyond z 1); `a_panda_is_a_big_twenty_health_animal_with_its_kind` sees the 15 goal names.
  - Terrain:
    - `ambient_particles::tests::a_sneeze_is_a_green_translucent_cloud` (200 seeds: colour, alpha, friction 0.96, no gravity, translucent, quad size 0.1875-0.375, the given motion plus at most a tenth of the base push, lifetimes 17-65, `generic_7` to `generic_0`, swelling from 0, passing through stone, gone after its lifetime).
    - `families::land::pandas::tests::a_sneeze_ends_in_one_green_cloud_for_every_viewer` (host and friend through `show_on`: one cloud as each sneeze ends and one when a friend missed the end tick, none twice; 0.825 blocks out from the cub at its eyes less 0.1; `sneeze_cloud`'s spot and motion).
    - `families::land::pandas::tests::a_sneeze_drops_a_slime_ball_about_once_in_seven_hundred` (with the 26.3 jar: 7000 sneezes queue 7000 gifts at the cub's feet that roll 3-20 slime balls through `roll_table`; none without `mob_drops`).
- **Decisions:**
  - The sneeze cloud is drawn by each viewer's `client_effects` from the shown sneeze counter, as vanilla's client draws it from its own tick, so the host and friends see it at the same tick. It fires when the shown counter drops below the last one the viewer saw: the end of a sneeze, or a new sneeze shown before the end was seen. D0's `fx.seen` is one `i32` per mob, so the panda keeps the eat counter in its low 16 bits and the sneeze counter above them (an eat counter never comes near 65535: it stops one tick in 20 after 80).
  - The cloud's motion is the client mob's last tick of movement (`position - old_position`), which is what vanilla's synced motion carries.
  - `after_sneeze` runs right after the sneezer's own sneeze step and before its roll, eating and amounts, as in vanilla; the jumping pandas read their own last fluid frame for `isInWater`. The sneezer itself is only skipped for being a baby, as in vanilla.
  - The lie goal's continue rule is the sit goal's, so both share `stays_down`.
  - A panda clicked off its back keeps its lie goal running, as in vanilla; it cannot lie down again until the goal stops and its 200-tick cooldown passes.
  - The roll's push and hops set the velocity without `needs_sync`, as vanilla's `setDeltaMovement` sets no impulse; friends follow the position. The neighbours' jump sets it, as `jumpFromGround` does.
- **Protocol:** no message or codec change; the sneeze and roll counters and flags already travel in `FamilyShow`, and `Kind` is not serialized.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (586 passed on the first run), `test -p minecraft_terrain --locked` (1715 passed, 3 ignored), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The panda files keep `rustfmt --check --edition 2021` where they were (the entity tests file keeps the two diffs pandas-drawn left; the rest clean); `ambient_particles.rs` keeps its own style. No comments were added.
- **Test steps.** Release build, survival, **Normal**, by day on flat grass, `/gamerule mob_griefing true`, `/gamerule mob_drops true`, the kit below. Name each screenshot by its label.
  1. `/summon minecraft:panda ~ ~ ~4 {MainGene:"lazy",HiddenGene:"lazy"}` and wait within a minute: it flops onto its back and kicks its legs in the air. Walk around it holding bamboo: it stays on its back. **m1-pandas-moods-1**
  2. Right-click it with an empty hand: it rolls back onto its feet. It does not lie down again for at least ten seconds.
  3. `/summon minecraft:panda ~ ~ ~4 {MainGene:"playful",HiddenGene:"playful"}`: every few seconds it does a forward somersault, about five blocks along, with one hop at the start and two small bounces as it lands. **m1-pandas-moods-2**
  4. Build a 2-high cobblestone platform, 5 by 5, and `/summon` the playful panda on it: when it faces the edge it rolls straight off and tumbles down.
  5. `/summon minecraft:panda ~ ~ ~3 {Age:-24000,MainGene:"weak",HiddenGene:"weak"}` and two plain adults 4 blocks from it. Within a minute or two the cub tips its head back with a "huh" sound, then sneezes: a small green puff in front of its nose and the sneeze sound, and both adults hop. **m1-pandas-moods-3**
  6. A plain cub (`/summon minecraft:panda ~ ~ ~3 {Age:-24000}`) sneezes too, but rarely (about once in five minutes); it also somersaults now and then.
  7. Leave the weak cub near you for a while: about one sneeze in 700 drops a slime ball at its feet. With `/gamerule mob_drops false` it never drops one. Set it back to true.
  8. Right-click the lazy panda with bamboo while it lies on its back: it only gets up, and the bamboo stays in your hand.
  9. Two windows (`scripts\mp-local.ps1`), side by side: the friend sees the lazy panda on its back, the somersaults and the cub's green puff at the same moments as the host, and hears the sneezes. The friend right-clicks the lazy panda on its back: it gets up in both windows. **m1-pandas-moods-4-host**, **m1-pandas-moods-4-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:panda_spawn_egg*8,minecraft:bamboo*64,minecraft:cobblestone*64,minecraft:iron_sword,minecraft:cooked_beef*16
```

### Group pandas-thunder

#### 1. Minecraft mobs know when it rains and thunders

After da88e0e0.

- **javap this round** (`Level.isRaining`, `isThundering`, `getRainLevel`, `getThunderLevel`, `canHaveWeather`; `Panda.isScared`, `tick`): `isRaining` is `canHaveWeather() && getRainLevel(1) > 0.2`, `isThundering` is `canHaveWeather() && getThunderLevel(1) > 0.9`, both compared as doubles. `getRainLevel(1)` is the rain level and `getThunderLevel(1)` is the thunder level times `getRainLevel(1)`, so there is no thunder without rain. `canHaveWeather` is a sky light, no ceiling and not the End. `Panda.isScared` is `isWorried() && level().isThundering()`, asked live.
- **Changed (vendored):**
  - `player/src/lib.rs`: `World::raining` and `World::thundering` right after `rain_at`, false by default.
  - `world/src/level/fire.rs`: `is_raining` is `pub` and asks `can_have_weather` as vanilla does; `is_thundering` right after it.
- **Changed (game code, `minecraft_terrain`):** `server_mobs.rs`: `MobWorld` answers both right after its `rain_at`.
- **Tests added:** terrain `server_mobs::tests::mob_worlds_know_when_it_rains_and_thunders` (with the 26.3 data: an overworld level under `/weather thunder` rains from tick 20 or 21 and thunders from tick 95, and once it starts each keeps on; a thunder level of 1 with a rain level of 0.5 rains but does not thunder, and with no rain neither; rain alone and a cleared sky; the Nether and the End with both levels forced to 1 have neither). Placed at the top of the test module, off the end where other tracks add theirs.
- **Decisions:**
  - As M1-3 decision 2: the `World` methods read through `MobWorld` from the level, not an `EntityWorld.thundering` field set beside `bright_outside`, so there is no `world.rs` line and goals can read it.
  - `is_raining` now checks `can_have_weather` and compares the rain level as a double, as vanilla does. Nothing changes in play: a level without weather never raises its rain level (`advance_weather_cycle` returns first, and loading sets the levels only where `can_have_weather`), and the double compare only differs at a rain level of exactly `0.2f32`.
  - The `Overlay` that villagers' pathfinding reads through does not forward the two methods: no goal behind it asks.
- **Protocol:** no message or codec change.
- **Runs:** `test -p minecraft_terrain --locked --lib mob_worlds_know` (passed; run once with a wrong thunder tick to see it fail, so it does not return early for want of data), `test -p minecraftoss-world --locked --lib` (136 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). `minecraftoss-player`'s own tests need the full suite's stand-in files; the change there is two defaulted methods.
- **Test steps:** nothing new to see until the worried pandas hide from thunder (commit 2). Rain still puts out fires and snow still piles up as before.
- **Kit:** none.

#### 2. Minecraft worried pandas hide from thunder

After f2fda2e4.

- **javap this round** (`Panda.isScared`, `isWorried`, `tick`, `canPerformAction`, `handleEating`, `mobInteract`, `eat`; `Level.isThundering`): `isScared` is `isWorried() && level().isThundering()`, asked live by `canPerformAction`, `handleEating`, `mobInteract` and the client's model; `Panda.tick` starts, right after `super.tick()`, with `if (isWorried()) { if (level().isThundering() && !isInWater()) { sit(true); eat(false); } else if (!isEating()) sit(false); }`. `eat(false)` sets the eat counter to 0. `isInWater` is `wasTouchingWater`, set in `baseTick`.
- **Changed (vendored entities crate):** `land/pandas.rs`: `scare` sets `PandaGoals.scared` to worried and `level.thundering()` (hook H4) right after the base tick, before the goals run, for every panda (a NoAI one too, as vanilla's `tick` runs without AI); `worry` gains the thunder half in front of the half pandas-eating landed: scared and not in water (`body.touching_water`, vanilla's `wasTouchingWater`) sits it down and stops its eating.
- **What follows from the scared flag (all landed earlier):** `can_perform_action` is false, so the attack, avoid, sit, lie, sneeze, roll and look goals do not start and H1 holds it still; `handle_eating` never starts a meal; `interact` passes, so bamboo does nothing; `FamilyShow` bit 4 carries the flag, and the renderer (host and friends) draws the scared sit: the head down at 2.17, the front paws over the face at -0.9, the body trembling by `cos(1.25 age) * 0.05 * pi` degrees, and no held item.
- **Tests added:** entities `world::families::land::pandas::tests::a_worried_panda_sits_and_trembles_in_thunder_and_refuses_bamboo` (`Grove` gains `thunder` and a `pool` of shallow water beyond x 20; for 200 thundery ticks a worried panda, an eating worried panda holding bamboo and a NoAI worried panda all sit scared from the first tick, never eat and cannot act; the first holds still and never runs from a player 6 blocks off, the second keeps its bamboo uneaten, a worried panda in the pool never sits while in the water, a plain panda is never scared; the sit amount reaches 1; bamboo on the worried panda passes and is kept, while a plain panda falls in love; once the sky clears all three stand up on the next tick, the first runs from the player again within 40 ticks, and bamboo brings it into love).
- **Decisions:**
  - `scared` stays a `PandaGoals` field (it is already shown to friends, and `held_still` and `can_perform_action` see no `World`), set once a tick from `level.thundering()` before the goals run. The weather changes only in the level tick, so the value is vanilla's live `isScared` for the whole panda tick; `interact` between ticks reads the last tick's value, which is the weather the player sees.
  - Vanilla's thunder `sit(true)` does not stop the navigation or zero the input (only `tryToSit` does), so neither does `worry`; H1 holds the panda still while it is scared.
  - `isInWater` is `core.body.touching_water` (set in the base tick for every panda), not the AI's fluid frame, which a NoAI panda never refreshes.
- **Protocol:** no message or codec change; the scared flag already travels in `FamilyShow` (bit 4).
- **Runs:** `test -p minecraftoss-entities --locked --lib pandas` (29 passed on the first run), then `test -p minecraftoss-entities --locked --lib` (587 passed), `test -p minecraft_terrain --locked` (1716 passed, 3 ignored; it also covers commit 1's test), `check -p launcher --locked` (green; only the old render_frame and skate warnings). `pandas.rs` keeps `rustfmt --check --edition 2021` clean and the panda entity tests file keeps only the two diffs pandas-drawn left; no comments were added.
- **Test steps.** Release build, survival, **Normal**, by day on flat grass, the kit below. Name each screenshot by its label.
  1. `/summon minecraft:panda ~ ~ ~4 {MainGene:"worried",HiddenGene:"worried"}` and a plain one with `/summon minecraft:panda ~3 ~ ~6`. Stand 6 blocks from the worried panda: it walks away from you.
  2. `/weather thunder` and wait about 5 seconds: as the storm builds, the worried panda sits down, covers its face with its front paws and trembles. The plain panda carries on as before. **m1-pandas-thunder-1**
  3. Walk right up to the worried panda: it stays where it sits. Right-click it with bamboo: nothing happens and the bamboo stays in your hand. Right-click the plain panda with bamboo: hearts.
  4. `/summon minecraft:panda ~ ~ ~3 {MainGene:"worried",HiddenGene:"worried",CanPickUpLoot:1b}` and throw a bamboo (Q) at its feet: it picks it up and sits trembling, with no bamboo in its paws and no munching.
  5. Dig a 3 by 3 pit one block deep, fill it with the two water buckets and `/summon` another worried panda into the water: it bobs about and never sits while it is in the water.
  6. `/weather clear`: within about half a second every worried panda stands up, the first walks away from you again, and bamboo now makes it fall in love. **m1-pandas-thunder-2**
  7. Two windows (`scripts\mp-local.ps1`), side by side, `/weather thunder` on the host: the friend sees the worried pandas sit and tremble at the same moment as the host and stand up when it clears. The friend's bamboo does nothing on a trembling panda. **m1-pandas-thunder-3-host**, **m1-pandas-thunder-3-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:bamboo*64,minecraft:water_bucket,minecraft:water_bucket,minecraft:iron_shovel,minecraft:cooked_beef*16
```

#### Pandas (M1-3) as a whole

Row 19 of `docs/round10/mobs.md` is now `done`. The owner can test the whole section with M1-3's steps in `docs/round10/track-m1.md` and this kit (17 stacks). Two notes on those steps: egg pandas pick up nothing, so step 6 keeps its `/summon ... {CanPickUpLoot:1b}`; and in step 10 the worried panda sits once the storm has built up, about 5 seconds after `/weather thunder`.

```
IW4L_MINECRAFT_KIT=minecraft:panda_spawn_egg*16,minecraft:bamboo*64,minecraft:bone_meal*32,minecraft:cake,minecraft:cake,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:shield,minecraft:flint_and_steel,minecraft:cactus*4,minecraft:sand*8,minecraft:cobblestone*64,minecraft:zombie_spawn_egg*2,minecraft:spawner,minecraft:golden_dandelion*2,minecraft:cooked_beef*32
```

### Review fixes (m1b)

#### 1. The Minecraft panda entity tests pass rustfmt

After c1ee9e3c.

- **Defect:** `entities/src/world/families/land/pandas/tests.rs` is a new file of this segment, so its staged content must pass `rustfmt --check --edition 2021` (CLAUDE.md, M10 "Rules"). pandas-drawn left two lines rustfmt would wrap in `the_sit_back_and_roll_amounts_ease_in_and_out_like_vanilla` (the `fell.push` tuple and the back-and-roll `assert!`), and the three later groups kept them.
- **Changed (vendored entities crate):** those two statements are wrapped the way rustfmt wants. No test logic changed, and no other file was touched.
- **Tests:** none added; the file now has no `rustfmt --check --edition 2021` diff.
- **Runs:** `test -p minecraftoss-entities --locked --lib pandas` (29 passed).
- **Test steps:** none, a test file only.
- **Kit:** none.

#### 2. Minecraft panda genes are read without unwrap_or_default

After 269b277f.

- **Defect:** the segment's clippy run (`clippy -p minecraft_survival -p minecraft_terrain -p minecraftoss-entities -p minecraftoss-player -p minecraftoss-world --locked --all-targets -- -A clippy::approx_constant`) warned once on a line this segment added: `gene` in the terrain's `families/land/pandas.rs` read a missing `MainGene` or `HiddenGene` with `unwrap_or_default()`, a method the workspace's `clippy.toml` disallows because it hides which fallback a site takes.
- **Changed (game code):** `crates/minecraft_terrain/src/families/land/pandas.rs`: `gene` falls back with `unwrap_or("")`. The behaviour is the same: an empty name matches no gene, so `Gene::from_name` still gives `normal`, as vanilla's `Gene.CODEC` does for a missing or unknown name. The file still passes `rustfmt --check --edition 2021`.
- **Tests:** none added; `families::land::pandas::tests::a_panda_saves_and_loads_with_its_genes_held_item_love_and_attributes` already loads a panda without genes (both `normal`) and one with the unknown `sleepy` (main `normal`, hidden `brown`).
- **Runs:** `test -p minecraft_terrain --locked` (1716 passed, 3 ignored), `check -p launcher --locked` (green; only the old `must_use` warning).
- **Test steps** (written down after the commit, in the final verification's commit; release build, survival, Normal, by day on flat grass; name each screenshot by its label):
  1. `/summon minecraft:panda ~ ~ ~4 {MainGene:"lazy",HiddenGene:"normal"}`, `/summon minecraft:panda ~2 ~ ~4 {MainGene:"brown",HiddenGene:"brown"}` and `/summon minecraft:panda ~4 ~ ~4 {MainGene:"sleepy"}`: a lazy-faced panda, a brown panda and a plain black-and-white one (an unknown gene reads as normal). **m1-fix-panda-genes-1**
  2. Save, quit and load: the three pandas look exactly as before. **m1-fix-panda-genes-2**
- **Kit:** none (the steps use `/summon`).

### Final verification (m1b)

After d152d507, on the whole of segment m1b (`r10-m1b-base..HEAD`).

- **Crates touched:** `minecraft_survival` (the H2 line in `entity_view.rs`), `minecraft_terrain`, `minecraftoss-entities`, `minecraftoss-player`, `minecraftoss-world`.
- **Clippy** (`clippy -p minecraft_survival -p minecraft_terrain -p minecraftoss-entities -p minecraftoss-player -p minecraftoss-world --locked --all-targets -- -A clippy::approx_constant`, with `full-suite.ps1`'s two empty stand-in scenario files, removed again afterwards): green. The first run warned once on a line this segment added (review fix 2, `d152d507`); the re-run has none. `minecraft_terrain` keeps 33 old warnings and `minecraft_survival` one (`menus.rs`), all on lines from before `r10-m1b-base` (the four in `server_mobs.rs` among them); the three vendored crates have none.
- **Tests:**
  - `test -p minecraft_terrain --locked`: 1716 passed, 3 ignored.
  - `test -p minecraftoss-entities --locked`: 587 passed.
  - `test -p minecraftoss-player --locked`: 200 passed.
  - `test -p minecraftoss-world --locked`: 136 passed.
  - `test -p minecraft_survival --locked`: 817 and 1 passed.
- **rustfmt:** the five new `.rs` files of the segment (the terrain's `families/land/pandas.rs` and `pandas/render.rs`, the entities crate's `land/pandas.rs`, `pandas/ai.rs` and `pandas/tests.rs`) have no `rustfmt --check --edition 2021` diff.
- **Fixes:** review fix 2 (`d152d507`, the clippy warning). No test failed.

### The segment's kit

One line for every step of segment m1b (pandas-live, pandas-drawn, pandas-eating, pandas-moods, pandas-thunder, M1-3 as a whole and the review fixes), 22 stacks. The friend's `.env` needs the same line.

```
IW4L_MINECRAFT_KIT=minecraft:panda_spawn_egg*16,minecraft:bamboo*64,minecraft:bamboo*64,minecraft:bone_meal*32,minecraft:cake,minecraft:cake,minecraft:cake,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:shield,minecraft:flint_and_steel,minecraft:cactus*4,minecraft:sand*8,minecraft:cobblestone*64,minecraft:zombie_spawn_egg*2,minecraft:spawner,minecraft:golden_dandelion*2,minecraft:water_bucket,minecraft:water_bucket,minecraft:iron_shovel,minecraft:cooked_beef*32
```

## Segment m1c

### Group foxes-live

#### 1. Registered Minecraft mobs can be spared by the blocks they walk through

After 1a2f03c1 (`r10-m1c-base`).

- **javap this round:** `SweetBerryBushBlock.entityInside` starts with `if (!(entity instanceof LivingEntity) || entity.is(EntityTypes.FOX) || entity.is(EntityTypes.BEE)) return;`, so a fox or a bee is neither held (`makeStuckInBlock` 0.8, 0.75, 0.8) nor pricked by a bush. It is the only block of 26.3 that spares mobs by type.
- **Changed (vendored entities crate):**
  - `world/families.rs`: `Family::spared_blocks(mob) -> &'static [&'static str]`, `&[]` by default, beside `fire_immune`; the generic `impl Exposed for FamilyMob<S>` answers `Exposed::spared_blocks` from it.
  - `world/hazards.rs`: `Exposed::spared_blocks`, `&[]` by default, beside `fire_immune`; `blocks_act` passes it to `block_effects`.
  - `inside_blocks.rs`: `block_effects(world, body, old_position, moves, spared)` skips the `entityInside` (`inside_block`) of a block whose id is spared. The block is still visited and still advances the collector's step, as vanilla's `checkInsideBlocks` does before it calls `entityInside`.
  - `world/families/pests/probe.rs`: the test probe's `spares` (empty), so the root test can spare a block.
- **Changed (game code):** `crates/minecraft_survival/src/survival.rs`: the player's own `block_effects` call passes `&[]`.
- **Tests added:** `inside_blocks::tests::a_spared_block_neither_holds_nor_pricks` (a body moving through a grown bush is held and pricked; sparing the bush gives no effect; sparing another block changes nothing), `world::families::tests::a_registered_mob_spared_by_berry_bushes_walks_through_unhurt` (two probes pushed through grown bushes: the plain one loses a heart and is held, the one that spares bushes keeps its health, is not held and goes on through).
- **Decisions:**
  - Decision 1 as recommended: one `spared_blocks` hook rather than a fox-only workaround; bees (M1-10) reuse it.
  - The hook takes the mob (`spared_blocks(mob)`, as `fire_immune(mob)` does) rather than nothing, as M1-4 wrote it, so the root test's probe can spare per mob. A family that always spares the same blocks ignores the argument.
  - The skip sits in `block_effects` around the one `inside_block` call (a separate line from E3's `Hazard::Fall` beside `InWall` and `Cramming`), so `inside_block` and its tests keep their signature.
- **Runs:** `test -p minecraftoss-entities --locked --lib spared` (2 new tests passed), `check -p launcher --locked` on this commit alone (green; only the old warnings).
- **Test steps:** nothing new to see; no family spares a block yet (foxes do from the next commit). Release build, survival, Normal.
  1. Plant sweet berries on grass, bone-meal them until the bush is red, then walk into it: you are slowed and pricked as before. **m1-hook-spared-1**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:sweet_berries*16,minecraft:bone_meal*16
```

#### 2. Minecraft foxes live in the entity world

After 39a52395.

- **javap this round** (`Fox` and its inner classes `FoxFloatGoal`, `FoxPanicGoal`, `FoxBehaviorGoal`, `FoxAlertableEntitiesSelector`, `SleepGoal`, `SeekShelterGoal`, `FoxFollowParentGoal`, `FoxStrollThroughVillageGoal`, `FoxLookAtPlayerGoal`, `PerchAndSearchGoal`, `FoxLookControl`, `FoxMoveControl`; `FloatGoal`, `FollowParentGoal`, `StrollThroughVillageGoal`, `Animal.getWalkTargetValue`, `Mob.getMaxFallDistance`, `Mob.getComfortableFallDistance`, `LivingEntity.calculateFallDamage`, `SweetBerryBushBlock.entityInside`): the M1-4 "Vanilla 26.3" numbers hold. Added or corrected on the way:
  - `BABY_DIMENSIONS` is the type's size scaled by `0.6f` (0.36 x 0.42 in float) with the eye set to 0.34375 and the passenger point (0, 0.375, 0).
  - `FoxFloatGoal.canUse` is `isInFluidDeeperThan(0.25, #entity_floatable) || isInLava()` (the plain `FloatGoal` uses the fluid jump threshold); its `start` clears the states. `FloatGoal`'s constructor sets the navigation's `canFloat`.
  - `FoxBehaviorGoal.alertable` is `getNearbyEntities(LivingEntity, forCombat().range(12).ignoreLineOfSight().selector(FoxAlertableEntitiesSelector), box.inflate(12, 6, 12))`. The selector: never a fox; always a chicken, rabbit or `Monster`; a `TamableAnimal` only when it is not tame (so a wild wolf or cat counts); never a creative or spectating player; never a trusted entity; anything else unless it `isSleeping` or `isDiscrete`. `forCombat` also refuses players in peaceful (`canAttack`).
  - `SleepGoal`: the countdown is `nextInt(reducedTickDelay(140))` = `nextInt(70)`, drawn once when the goal is made and again on `stop`; `canSleep` takes one off it per call while it is above 0. `start` also sets the move control's wanted position to the fox's own at speed 0.
  - `SeekShelterGoal` extends `FleeSunGoal`: its first interval is `reducedTickDelay(100)` = 50 calls, every later one 100; the thunder branch (thundering and open sky at its block) skips the interval, the daylight and the village. `getHidePos` is ten tries at `blockPosition + (nextInt(20) - 10, nextInt(6) - 3, nextInt(20) - 10)`, out of the sky with `getWalkTargetValue < 0`. `Animal.getWalkTargetValue` is 10 on a grass block, else `getPathfindingCostFromLightLevels`.
  - `FoxStrollThroughVillageGoal(32, 200)` passes only the interval on: `StrollThroughVillageGoal` with `reducedTickDelay(200)`; `canUse` is night, `nextInt(100) == 0`, `isCloseToVillage(pos, 6)` and `LandRandomPos.getPos(15, 7, -sectionsToVillage)`; `canContinueToUse` needs the navigation's target to be that spot; `tick` walks 10 blocks of the way (pulled 0.4 towards the fox) to the `MOTION_BLOCKING_NO_LEAVES` height, else a random spot within 8. The fox adds `canFoxMove` (not sleeping, sitting or defending, no target) to both checks and clears its states on `start`.
  - `PerchAndSearchGoal`'s look time is `adjustedTickDelay(80 + nextInt(20))` and it ticks every other tick (it is not an every-tick goal); it looks at `(x + cos, eyeY, z + sin)` with the 75 and 40 degree limits.
  - `Mob.getComfortableFallDistance(allowed)` is `floor(allowed + 3)` whatever the `SAFE_FALL_DISTANCE`, so a fox's paths still take falls of 3 (plus its sacrifice with a target), not 5; only `calculateFallDamage` reads the attribute.
  - `getAmbientSound` draws its screech roll before `playAmbientSound` draws the pitch.
- **Changed (vendored entities crate):**
  - New `world/families/land/foxes.rs`: `FoxVariant` (`red` 0, `snow` 1, unknown names red), `Fox` (variant, `trusted` UUIDs, the mouth item with its drop chance, `can_pick_up_loot`, `ticks_since_eaten`, `Age`, love and its cause, the last damage, the interested angle and crouch amount with their old values, `MonsterAi`), `trust` (the first two slots, a third replacing the second as `addTrustedEntity` does), `step_angles` (vanilla `tick`'s 0.4 ease of the interested angle and the 0.2 crouch step capped at 5), `set_baby` (0.36 x 0.42, eye 0.34375) and `impl Family`: key 49, kind `minecraft:fox`, 0.6 x 0.7, eye 0.4, health 10, speed 0.30000001192092896, follow range 32, ambient interval 120, category `creature`, leashable (walked by its navigation), panicking while its panic goal runs, spared by sweet berry bushes, the mouth item as its death drop, its `MonsterSpec`.
  - The tick: `family_base_tick`; the ambient voice becomes `entity.fox.sleep` while it sleeps, or at night `entity.fox.screech` at volume 2 one time in ten when no player who is not spectating stands within its box grown by 16 (vanilla's roll, the base tick's pitch); `ticksSinceEaten` and, with no living target, crouching and interest off (`aiStep`); the goals get the alertable test, the players it does not avoid (sneaking or trusted), the tame wolves and cats, the damage of the last 40 ticks and the other foxes as kin; the level's points of interest are lent to its goals for the step (as `tick_selected` lends them to zombies); `family_goal_step_with(.., fox_ai_step)`; the defending aggro (5 %); `family_after_move`; growing up and the love countdown; then vanilla `tick`'s server part: water, a target or thunder (`World::thundering`, H4) wake it, water or sleep make it stand; and the interested angle and crouch amount step.
  - New `world/families/land/foxes/ai.rs`: the flags (vanilla's byte: 1 sitting, 4 crouching, 8 interested, 16 pouncing, 32 sleeping, 64 faceplanted, 128 defending), `FoxGoals` (the animal goal state, the flags, `clearStates`, `canMove`, the pitch the look keeps, alertable, `moving`, the halt request, tame pets and unwary players), `held_still` (H1: `FoxMoveControl` ticks only when the fox can move), `walk_value`, `has_shelter`, the `MonsterSpec` (the animal fire maluses, `DAMAGING` and `DAMAGING_IN_NEIGHBOR` 0, floating, `WalkTarget::Animal`, 512 path nodes from `setRequiredPathLength(32)`) and the goals in vanilla's order: `FoxFloatGoal` 0, `ClimbOnTopOfPowderSnowGoal` 0 (D0's), `FoxPanicGoal` 2 (2.2, not while defending), three `AvoidEntityGoal`s 4 (players 16, wild wolves 8, polar bears 8; walk 1.6, 1.4 within 7 blocks; none while defending), `SeekShelterGoal` 6 (1.25), `SleepGoal` 7, `FoxFollowParentGoal` 8 (1.25), `FoxStrollThroughVillageGoal` 9, `WaterAvoidingRandomStrollGoal` 11 (1.0), `FoxLookAtPlayerGoal` 12 (24 blocks, not while faceplanted or interested), `PerchAndSearchGoal` 13. No target goals yet.
  - `fox_ai_step`: `monster_ai_step`'s order with the fox's parts: a sleeping fox's inputs and jump are zeroed first (`Fox.aiStep`), the sleep goal sees whether its inputs are zero, the halt request sets the move control's wanted position to its own at speed 0 after the goals, `MonsterAi.still` holds the move control, the look ticks through `LookControl::tick_from` only while it is awake and keeps its pitch while pouncing, crouching, interested or faceplanted (`FoxLookControl`), and the landed fall comes back shortened by `2 / reduction` of the block it lands on (`living::landed_block`, E1's helper), so `family_goal_step_with`'s fall damage counts from 5 blocks.
  - `look.rs`: `LookControl::tick_from(.., reset_pitch)` beside `tick_limited`, which now calls it with `true` (every other mob as before).
  - `world/families/land.rs`: `pub mod foxes;` and `foxes: foxes::Fox` in `family_group!`, between the rabbits and the pandas (key order).
  - New `world/families/land/foxes/tests.rs` (test only).
- **Tests added** (`world::families::land::foxes::tests`):
  - `a_fox_is_small_with_ten_health_and_its_kind` (sizes and eyes of adult and cub, health, speed, interval, variant names, census, candidates, traits, voice, spared blocks, goal and target names, follow range, the walk profile, the empty table, 1-3 experience for adults only and the mouth item dropped).
  - `a_fox_flees_a_walking_player_but_not_a_sneaking_or_creative_one` (12 blocks yes, 20 no, sneaking, creative and trusted players no, not while defending; 1.4 within 7 blocks).
  - `a_fox_flees_wild_wolves_and_polar_bears_but_not_tame_wolves` (8 blocks; a cow no).
  - `a_fox_sleeps_by_day_under_cover_and_wakes_when_something_walks_near` (asleep within 142 ticks and still; a sneaking or trusted player leaves it asleep, a stranger wakes it; a cow within 12 wakes it, a tame wolf nearby does not stop it sleeping; no sleep at night or under open sky).
  - `a_fox_seeks_shelter_from_the_sun_but_not_in_a_village` (by day under open sky; not in a village, not at night; `walk_value` and `has_shelter`).
  - `a_fox_wakes_in_a_thunderstorm_and_seeks_cover_under_open_sky` (thunder wakes it and keeps it awake; it heads for cover at once even at night in a village, but not under a roof).
  - `a_fox_wakes_and_stands_up_in_water` (and a sleeping fox does not sit).
  - `a_fox_perches_and_looks_around_two_to_four_times` (sits still, turns its head, gets up after 150-410 ticks; never when something alerts it).
  - `a_hurt_fox_panics_and_falls_out_of_love` (not while defending; panicking for the lead's rule).
  - `a_fox_screeches_at_night_only_with_no_player_within_sixteen` (about one in ten at night, none by day or with a player within 16, spectators do not count, `entity.fox.sleep` while asleep).
  - `a_fox_walks_through_sweet_berry_bushes_unhurt_and_unslowed` (the hook of commit 1).
  - `a_fox_takes_no_damage_from_a_five_block_fall` (5 blocks none, 6 one, 8 three).
  - `a_fox_cub_follows_its_parent`, `a_leashed_fox_is_pulled`.
  - `a_sleeping_fox_holds_its_head_and_a_faceplanted_one_its_pitch` (`tick_from` keeps the pitch, `tick_limited` resets it; asleep the head does not turn).
  - `a_fox_strolls_through_a_village_only_at_night`.
- **Decisions:**
  - Decision 5 as recommended for the fall: the fox's own step shortens the landed distance by `2 / reduction` (E1's `landed_block` is in, so `fall_multiplier` and `fall_reduction` stay private). Paths keep the fork's fall of 3 (`getComfortableFallDistance` adds 3, see javap), so `FOX_SPEC.walk` sets no `max_fall_distance`.
  - The avoid goals are the fox's own `AvoidEntityGoal` port (`FoxAvoidGoal`), filtering by `FoxGoals.tame` and `FoxGoals.unwary`, because `MobCandidate` has no tame flag and `PlayerCandidate` no sneaking; its goal name stays `AvoidEntityGoal` as vanilla's class is.
  - `alertable` is worked out once per tick in the family tick (players with their controls and trust, awake villagers from the villager list, every other mob candidate but foxes and tame wolves and cats) and handed to both goals that ask, as the goals run before the fox moves.
  - The sleep goal's first countdown is drawn on its first use rather than when the goal is made (the selector is built without the mob's random); later ones on `stop`, as vanilla.
  - The sleep goal's `setWantedPosition(own, 0)` is a halt request in `FoxGoals` that `fox_ai_step` applies after the goals, as the goal context has no move control.
  - The level's points of interest are lent to the fox's goals for its step only (`isVillage` for the shelter, `sectionsToVillage` for the village stroll); the family tick takes them back right after.
  - The trusted list holds UUIDs: a player is trusted when its UUID (`uuid_of(PLAYER_TARGET + id)`) is in the list, as the wolf's owner works.
  - `ambient` keeps the base tick's pitch and draws the screech roll after it (vanilla draws it before the pitch); the odds are vanilla's.
  - The faceplant's level event 2001, stalking, pouncing, the bite, berries, items, breeding and trust come with their groups; the flags they set are in `FoxGoals` now, so `canMove`, the look and the gates above are final.
  - Until commit 3 registers the `TerrainFamily`, D0's terrain test `server_mobs::tests::every_registered_kind_is_simulated_and_summonable` sees the two registries differ, as with rabbits-live 2 and pandas-live 1. In-game nothing changes (the terrain spawns no kind it does not register).
  - Not handled: invisibility (`getVisibilityPercent`) shortening the avoid and alert ranges, and armour stands counting as alert mobs (the fork's armour stands are no mobs).
- **Runs:** `test -p minecraftoss-entities --locked --lib` (662 passed, the 16 fox tests and the hook's among them; the first fox run failed four tests whose set-ups were wrong: a fox that strolled within reach of a wolf placed out of reach, a sleeping flag set by hand that the sleep goal clears at night as vanilla's does, a cub that perched for longer than the test waited, and a bound of exactly 1.5 blocks; the tests now keep only the avoid goals, put the fox to sleep by day, wait 600 ticks and allow 1.2), `check -p launcher --locked` (green; only the old warnings). The new files pass `rustfmt --check --edition 2021` on their staged content.
- **Test steps:** nothing new in-game yet (the terrain registers foxes in the next commit); the steps are those of commit 3.
- **Kit:** none.

#### 3. Minecraft foxes are saved, hatched and seen by friends

After e41f0c0a.

- **javap this round:** `Fox.addAdditionalSaveData` writes `Trusted` (always, a list of the present trust slots as UUID int arrays), `Sleeping`, `Type` (by name), `Sitting` and `Crouching` (booleans as bytes); `readAdditionalSaveData` clears the trust, adds each listed UUID through `addTrustedEntity` (slot 0 if empty, else slot 1, so a third replaces the second), reads the flags (false when missing) and `Type` (red when missing or unknown), then sets the prey goals (`foxes-hunt`). `Animal` writes `InLove` and, when set, `LoveCause`; `Mob` writes `CanPickUpLoot`, `equipment` and `drop_chances`. `EntityTypes.FOX` has `clientTrackingRange(8)`, and `FoxRenderer` a shadow of 0.4. The built-in spawn row (0.6 x 0.7, `NO_RESTRICTIONS`, `MOTION_BLOCKING_NO_LEAVES`, `#foxes_spawnable_on` with a raw brightness above 8, the creature category) and the finalize (the group's variant, `#spawns_snow_foxes`, cubs from the third fox, the 20 % mouth item, follow range 32) match 26.3, so foxes get no `families/land.rs` row.
- **Changed (game code, `minecraft_terrain`):**
  - New `families/land/foxes.rs`: `impl TerrainFamily for Fox` with tracking range 8, `VOLATILE` `Sleeping`, `Sitting`, `Crouching` and `ForcedAge`, shadow 0.4, no textures yet and an empty `draw` (the models come with `foxes-drawn`).
  - `read`: the saved yaw for its body and head; `Type`; `Trusted` through `Fox::trust`; the three flags; `Age`, `ForcedAge`, `AgeLocked`, `InLove`, `LoveCause` and a friend's `Baby`; `CanPickUpLoot` (true when missing, the constructor's); `equipment.mainhand` and `drop_chances.mainhand` (0.085 when missing).
  - `fields`: the same keys; `equipment` and `drop_chances` always (empty when its mouth is empty or the chance is the default); `LoveCause` only when set.
  - `show`/`show_on`: `FamilyShow.flags` is vanilla's flag byte; a friend sets it and steps the interested angle and crouch amount once per host tick, as a vanilla client does from the synced flags (no `values` used yet).
  - `families/land.rs`: `mod foxes;` and `foxes::Fox` in `terrain_group!`, between the rabbits and the pandas.
  - `server.rs`, the egg test `a_spawn_egg_hatches_its_mob_on_the_clicked_slab_and_each_egg_is_answered`: the kind that cannot hatch is now `minecraft:illusioner` (no row, never simulated, in no round 10 track) in both places, since foxes hatch.
- **Changed (vendored world crate):** `natural_spawner/families/land.rs` gains the spawn test below and its `creatures` helper (no row).
- **Tests added:**
  - Terrain (`families::land::foxes::tests`): `a_fox_saves_and_loads_with_its_type_trust_sleep_sit_crouch_love_and_mouth_item` (a snow cub with two trusted players, asleep, sitting and crouching, in love with its cause, no loot pickup and an emerald with a guaranteed drop round trips; a tag with three trusted UUIDs keeps the first and the last, an unknown type is red, an empty mouth writes no `mainhand`, no `LoveCause` when there is none), `a_friend_sees_a_fox_from_its_look_tag` (no volatile key in the look, its type, `Baby` and mouth item in it, range 8, the flag byte through `FamilyShow`, the friend's crouch amount and head tilt eased tick by tick and dropped again), `a_dormant_fox_from_an_old_save_wakes` (simulated now; a full vanilla tag loads, ticks and runs from a player).
  - World: `natural_spawner::families::land::tests::foxes_spawn_on_podzol_in_light_in_groups_with_a_snowy_variant_in_snowy_biomes` (the built-in row; red foxes on podzol and on grass in a taiga, snow foxes on snow blocks in a snowy taiga, two adults and then cubs in each group of four, follow range 32 on each, one in five or so holding one of vanilla's seven mouth items; none on stone or at light 8), with a `creatures(kind, biome, floor, sky)` helper beside the rabbits' one.
- **Decisions:**
  - Decision 5's `minecraft:illusioner` for the egg test.
  - Show and drawing: `FamilyShow` carries the flag byte only, and the friend steps the interested angle and crouch amount from it in `show_on`, as pandas do with their amounts and as a vanilla client does, rather than M1-4's start clocks; host and friend run the same steps from the same flags. The eat counter for event 45's crumbs joins with `foxes-items`.
  - The mouth item stays in the look tag (M1-4): it seldom changes, so a friend sees a new one when the look tag is sent again.
  - Spawn eggs still give foxes `CanPickUpLoot` false, fork-wide (see pandas, "Left for later"), so egg foxes will not pick things up once `foxes-items` lands; natural and `/summon .. {CanPickUpLoot:1b}` foxes will.
- **Protocol:** no message or codec change; the fox's state uses `FamilyShow`'s existing flags.
- **Runs:** `test -p minecraft_terrain --locked --lib` (1841 passed, 3 ignored; D0's registry, eggs-survey and cage tests see the fox), `test -p minecraftoss-world --locked --lib` (138 passed; the first run failed the new spawn test, which took a call's foxes for one group: a call spawns up to three groups, each with its own group data, so the test now checks each run of adults and cubs), `check -p launcher --locked` (green; only the old warnings). The new terrain file passes `rustfmt --check --edition 2021` on its staged content, and the world group file keeps `rustfmt --check` clean.
- **Test steps** (foxes are live but not drawn yet, so each shows as its small round shadow). Release build, survival, Normal, flat grass in a plains or taiga biome. Name each screenshot by its label.
  1. `/time set 14000` (night). Use the fox egg three times on grass 6 blocks away: three small shadows appear and wander about. **m1-foxes-live-1**
  2. Walk at them without sneaking: from about 16 blocks off the shadows run away, quickly at first and a little slower once you are within 7 blocks. Sneak towards one: it lets you come close.
  3. Use the wolf egg 6 blocks from a fox: the fox's shadow runs from the wolf. Use the polar bear egg near another: it runs from the bear too.
  4. Build a roof of oak planks 3 blocks above the grass (7 by 7), stand far off, `/time set 1000` and use a fox egg under the roof, then walk at least 13 blocks away and wait. Within about 7 seconds the shadow stops for good, and you hear soft snoring sounds now and then. **m1-foxes-live-2** Sneak up to it: it stays put. Walk up to it without sneaking: once you are within 12 blocks it gets up and runs.
  5. Use a fox egg in the open by day, away from any village, and wait: within half a minute it heads for the shade under your roof. `/weather thunder` (wait until the storm is loud): sleeping foxes wake, and foxes in the open run for cover. `/weather clear`.
  6. `/time set 14000` and stand still 20 blocks from a fox: now and then you hear a fox screech.
  7. Plant a line of sweet berries across the grass and bone-meal them until the bushes are red. Stand so that a fox is between you and the bushes and walk at it: it runs straight through the bushes without slowing down and never flashes red. Walk into the bushes yourself: you are slowed and pricked. **m1-foxes-live-bushes**
  8. `/summon minecraft:fox ~3 ~5 ~`: it drops five blocks and lands without flashing red. `/summon minecraft:fox ~3 ~8 ~`: it flashes red as it lands.
  9. Right-click a fox with a lead: the rope ties. Walk off: it follows.
  10. Hit a fox with the sword: it flashes red and runs off fast. Kill one that holds something in its mouth (summoned with `/summon minecraft:fox ~ ~ ~3 {equipment:{mainhand:{id:"minecraft:emerald",count:1}}}`): the emerald drops.
  11. In creative, place the spawner and use a fox egg on it: the spawner takes it (nothing is drawn inside yet). Switch back to survival.
  12. Save, quit and load: the shadows are where they were. A fox that was asleep may get up for a moment and lies down again within a few seconds, as in vanilla (its sleep goal counts down again after a load).
  13. `/locate biome minecraft:taiga`, `/tp` there and walk into new chunks: small shadows in groups of two to four. Foxes in old saves now move.
  14. Two windows (`scripts\mp-local.ps1`): the friend sees the same shadows. The friend walking up to a sleeping fox wakes it; the friend's M4A1 hit makes a fox run in both windows. **m1-foxes-live-3-host**, **m1-foxes-live-3-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:fox_spawn_egg*16,minecraft:wolf_spawn_egg*2,minecraft:polar_bear_spawn_egg*2,minecraft:oak_planks*64,minecraft:oak_log*16,minecraft:sweet_berries*16,minecraft:bone_meal*32,minecraft:lead*2,minecraft:spawner,minecraft:iron_sword,minecraft:cooked_beef*16
```

### Group foxes-drawn

#### 1. Minecraft foxes are drawn like vanilla

After 7459c74e.

- **javap this round** (`FoxModel`, `AdultFoxModel`, `BabyFoxModel`, `FoxRenderer` with `FoxTexturesByState` and `FoxTexturesByAge`, `FoxRenderState`, `FoxBabyAnimation`, `LayerDefinitions`, `AgeableMobRenderer`, `LivingEntityRenderer.submit`, `LivingEntity.getAgeScale`, `Fox.getHeadRollAngle`, `getCrouchAmount`, `getLeashOffset`): the M1-4 "Render" numbers hold. Ported from the bytecode:
  - `AdultFoxModel.createBodyLayer` (48 x 32): `head` at (-1, 16.5, -3), box (-3, -2, -5) 8 x 6 x 6 at uv (1, 5), with the children `right_ear` (uv 8, 1; (-3, -4, -4) 2 x 2 x 1), `left_ear` (uv 15, 1; (3, -4, -4)) and `nose` (uv 6, 18; (-1, 2.01, -8) 4 x 2 x 3); `body` at (0, 16, -6) turned π/2 about x, box (-3, 3.999, -3.5) 6 x 11 x 6 at uv (24, 15), with its child `tail` at (-4, 15, -1) turned -0.05235988, box (2, 0, -1) 4 x 9 x 5 at uv (30, 0); four legs, box (2, 0.5, -1) 2 x 6 x 2 grown by `CubeDeformation(0.001)`, the right ones at uv (13, 24), the left ones at uv (4, 24), at (-5, 17.5, 7), (-1, 17.5, 7), (-5, 17.5, 0) and (-1, 17.5, 0). 10 parts, 10 cubes. `LayerDefinitions` adds no mesh transform to either layer.
  - `BabyFoxModel.createBodyLayer` (32 x 32): `head` at (0, 18.125, 0.125) with four cubes in one part (the skull at uv 0, 0, the nose at uv 18, 20, the ears at uv 22, 8 and 22, 11); `body` at (0, 20, 2), box (-2.5, -2, -3) 5 x 4 x 6 at uv (0, 10), with `tail` at (0, -0.5, 3), box (-1.5, -1.48, -1) 3 x 3 x 6 at uv (0, 20); legs 2 x 2 x 2 at (±1.5, 22, 4) and (±1.5, 22, 0), the right ones at uv (22, 4), the left ones at uv (22, 0). 7 parts, 10 cubes.
  - `FoxModel.setupAnim` after the pose reset: `setWalkingPose` (head `zRot` = `headRollAngle`, legs visible; the adult swings its legs as `QuadrupedModel` does, `cos(pos * 0.6662 [+ π]) * 1.4 * speed`; the cub adds `FOX_BABY_WALK` through `applyWalk(pos, speed, 1, 2.5)`), then crouching, else sleeping, else sitting; then pouncing; then the look (`xRot`, `yRot` times 0.017453292) unless sleeping, faceplanted or crouching; then, asleep, the head at (0, -2.0943952, `Mth.cos(age * 0.027) / 22`); then, faceplanted, `legMotionPos += 0.67` and the legs paddle at `Mth.cos(legMotionPos * 0.4662 [+ π]) * 0.1`.
  - Crouching: body `xRot` + 0.10471976, head `y` + `crouchAmount * ageScale` (`getAgeScale` is 0.5 for a baby), `cos(age) * 0.05` as the body's `yRot`, the hind legs' `zRot` and half of it on the front legs; the adult's body `y` + `crouchAmount`, the cub's + `crouchAmount / 6`.
  - Sleeping: legs hidden; the adult's body `zRot` -π/2 and `y` + 5, tail `xRot` -2.6179938, head `x` + 2 and `y` + 2.99; the cub's body `zRot` -π/2, `xRot` -0.17453292 and (-1.5, +1.5, -1.5), tail `xRot` -2.1816616 (written right after -2.6179938) and (-0.7, +0.9, +0.6), head (-2, +2.8, -4); both heads `yRot` -2.0943952 and `zRot` 0.
  - Sitting: head `xRot` and `yRot` 0 (the look sets them again); the adult's body `xRot` π/6, `y` - 7, `z` + 3, tail `xRot` π/4 and `z` - 1, head `y` - 6.5 and `z` + 2.75, front legs `xRot` -0.2617994, hind legs `xRot` -1.3089969, `y` + 4, `z` - 0.25; the cub's body `xRot` -0.959931, `z` - 4.5 x age scale, `y` + 3 x age scale, tail `y` - 0.6, `z` - 2 x age scale, `xRot` 0.95993114, head `y` - 0.75, front legs `xRot` -0.2617994 and `z` - 1.5, hind legs `z` - 3.75, the right legs' `x` + 0.01 and the left legs' - 0.01.
  - Pouncing: the adult's body and head `y` - `crouchAmount / 2`; the cub does not override `setPouncingPose`.
  - `FoxRenderer`: an `AgeableMobRenderer` (adult or cub model by `isBaby`) with shadow 0.4; `setupRotations` turns X by `-xRot` degrees after the body yaw and the death tilt while pouncing or faceplanted; `getTextureLocation` picks `fox`, `fox_sleep`, `fox_snow` or `fox_snow_sleep`, each with `_baby` for cubs, by variant and `isSleeping`. `extractRenderState`: `headRollAngle` = `lerp(partial, interestedAngleO, interestedAngle) * 0.11 * π`, `crouchAmount` = `lerp(partial, crouchAmountO, crouchAmount)`, the flags from the entity. The default render type is `entityCutout` (back faces culled). `Fox.getLeashOffset` is (0, 0.55 x eye, 0.4 x width).
- **Changed (game code, `minecraft_terrain`):**
  - New `families/land/foxes/render.rs`: both models as part tables with the shared parts first (head, body, tail, the four legs; the adult's ears and nose after them as the head's children), `PartState` (offset, rotation, scale, visible: a `ModelPart`'s fields), `Stance` (look, head roll, crouch amount, age, walk, flags), `setup_anim` in `FoxModel.setupAnim`'s order with the walking, crouching, sleeping, sitting and pouncing poses, `part_poses`, `shown` (a hidden part hides its children), `grown` (the legs' 0.001), `model_root` (the 90-degree flip, the pounce and faceplant tilt, the 1.501 lift and the mirror), `stance` (the head roll and crouch amount lerped by the partial tick), `texture`, `baby_walk` (`FOX_BABY_WALK` through `keyframes::shared()`) and `append_foxes` (the skin lit at the eye, the hurt overlay, into `out.culled`).
  - `families/land/foxes.rs`: `mod render;`, the eight `TEXTURES`, `CAGE_MARGIN` 0.21 and `draw` (the living foxes with the cub walk read once per frame).
  - `leash_render.rs`: `"fox" => (0.55, 0.4)` after the `"ocelot"` row (M10's anchor), so the rope ties at the neck.
- **Tests added** (`families::land::foxes::render::tests`):
  - `an_adult_and_a_cub_have_their_parts_and_textures` (parts, cubes, texture sizes, names, parents, the tail under the body, the ears and nose under the head, the legs' growth, the eight textures; drawn: 60 faces into `culled` only, the paws on the ground, the ears 0.72 up, the nose 0.69 in front of the feet, the red skin and light; a snow cub 60 faces, 0.63 tall, its own skin).
  - `a_sleeping_fox_hides_its_legs_and_closes_its_eyes` (legs hidden, the head turned back with its bob whatever the look, the curled body and tail of adult and cub; drawn: 36 faces on the ground, lower than a standing fox, the sleep skins).
  - `a_crouching_fox_lowers_its_head_by_its_crouch_clock` (head and body down by the amount, the cub by half and a sixth, the 6-degree tilt, the wiggle, the head cocked but not following the look; the clock and the tilt lerped by the partial tick; the drawn mesh is the set-up one).
  - `a_sitting_pouncing_or_faceplanted_fox_takes_vanilla_s_pose` (the sit of adult and cub, the adult's pounce drop and none for the cub, the paddling legs, the root tilted by the pitch only while pouncing or faceplanted).
  - `a_fox_s_lead_ties_at_its_neck`, `a_friend_draws_a_fox_as_the_host_does` (crouching and interested, asleep, sitting: the same mesh and skin at four partial ticks from `FamilyShow`).
  - `the_cub_walk_matches_the_jar` (gated on `MINECRAFTOSS_ROOT`: 0.5 s, looping, every bone a part of the cub; no speed no step, the offsets added to the walking pose, a loop every 10 ticks of walk, topping out at speed 0.4; adults swing their legs).
- **Decisions:**
  - Vanilla's `legMotionPos` is a model field that grows by 0.67 on every `setupAnim` call of a faceplanted fox, so its paddle speed follows the frame rate. The port derives it from the age: 2.01 a tick (one call a frame at 60 frames a second), so host and friend paddle alike.
  - The legs' `CubeDeformation(0.001)` is drawn by scaling the cube about its centre, so the corners grow and the UVs stay those of the 2 x 6 x 2 box, as vanilla's.
  - `CAGE_MARGIN` 0.21: the spawner scales a fox by 0.53125 (its box is under a block, so vanilla does not shrink it) and its 29-pixel length pokes out of the cage by up to 0.2 at a spin of 0, as in vanilla's spawner. D0's cage test failed with the default 0.05.
  - The poses are set up as vanilla's `ModelPart` fields in `setupAnim`'s order, so the cub's keyframes add to the walking pose and the crouch, sleep and sit poses then overwrite the fields vanilla overwrites.
  - The crouch, the head tilt, the pounce tilt and the faceplant are drawn from the flags already; the goals that set them come with `foxes-hunt`.
  - The mouth item is drawn with the next commit.
- **Protocol:** no message or codec change; the drawing reads the flag byte and the stepped clocks foxes-live already sends.
- **Runs:** `test -p minecraft_terrain --locked --lib` (1847 passed, 3 ignored, one failed: D0's cage test `every_registered_kind_is_shown_inside_its_cage` with the default margin, fixed by the margin above), then `test -p minecraft_terrain --locked --lib -- inside_its_cage fox families::` (90 passed: the cage test, every fox test with the gated walk test against the 26.3 jar that `cargo-slot.ps1` points `MINECRAFTOSS_ROOT` at, and D0's registry tests), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new `render.rs` passes `rustfmt --check --edition 2021` on its staged content.
- **Test steps** (release build, survival, Normal, by day on flat grass in a plains or taiga biome; name each screenshot by its label). The crouch, head tilt, pounce and faceplant come with `foxes-hunt`; these steps check the models, the sleep, the sit, the cubs, the lead and friends.
  1. Use the fox egg on grass 6 blocks away: an orange fox with a white chest and throat, black legs and a long bushy tail with a white tip, about 1.8 blocks from nose to tail tip and 0.7 tall at the ears. `/summon minecraft:fox ~3 ~ ~ {Type:"snow"}`: the same fox in white. **m1-foxes-drawn-1**
  2. Walk around a fox: its legs swing as it trots, and its head turns to follow you.
  3. `/summon minecraft:fox ~ ~ ~4 {Age:-24000}` and `/summon minecraft:fox ~2 ~ ~4 {Age:-24000,Type:"snow"}`: two cubs with their own rounder model (a big head, short stubby legs, a short tail, about 0.6 tall), not shrunken adults. Watch one walk: its body bobs and its little legs step with the walk animation. **m1-foxes-drawn-2**
  4. Sneak and stay 6 to 10 blocks from a grown fox for a minute: now and then it sits down, rump on the ground, front legs straight, tail curled forward, and looks about. **m1-foxes-drawn-3**
  5. Under the oak plank roof (7 by 7, 3 blocks up), by day, use a fox egg and walk 13 blocks away: within a few seconds it lies down curled on its side, legs out of sight, tail wrapped round, head turned back, eyes shut, its head bobbing slowly. A snow fox and a cub do the same with their own sleeping skins. **m1-foxes-drawn-4**
  6. Right-click a fox with a lead: the rope ties low at its neck. Walk away: it follows. **m1-foxes-drawn-5**
  7. Hit a fox with the sword: it flashes red; kill it and it tips over sideways.
  8. In creative, place the spawner and use a fox egg on it: a small fox turns inside the cage, its tail poking out a little as in vanilla. **m1-foxes-drawn-6** Switch back to survival.
  9. Save, quit and load: the same foxes and cubs, a sleeping one lying down again within a few seconds.
  10. Two windows (`scripts\mp-local.ps1`), side by side: the friend sees the same foxes, cubs, sitting foxes and sleeping foxes in the same poses at the same moments. **m1-foxes-drawn-7-host**, **m1-foxes-drawn-7-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:fox_spawn_egg*16,minecraft:oak_planks*64,minecraft:oak_log*16,minecraft:lead*2,minecraft:spawner,minecraft:iron_sword,minecraft:cooked_beef*16
```

#### 2. Minecraft foxes show what they carry in their mouths

After a374c030 (its subject starts with an invisible UTF-8 byte order mark, written by PowerShell 5.1's `Set-Content -Encoding utf8` into the message file; left as it is, since the track does not amend).

- **javap this round** (`FoxHeldItemLayer`, `HoldingEntityRenderState`, `LivingEntityRenderer.submit`): `FoxRenderer.extractRenderState` fills `heldItem` from the main hand with `ItemDisplayContext.GROUND`. `LivingEntityRenderer.submit` runs `setupAnim` again and hands each layer the pose stack after `scale(-1, -1, 1)` and `translate(0, -1.501, 0)`, with the render state's `yRot` and `xRot`. `FoxHeldItemLayer.submit`: nothing for an empty stack; else `translate(head.x / 16, head.y / 16, head.z / 16)` (the head part's fields after `setupAnim`), `scale(0.75)` for a cub, rotate Z by `headRollAngle` (radians), Y by `yRot` and X by `xRot` (degrees), then translate (0.06, 0.27, -0.5) awake or (0.46, 0.26, 0.22) asleep for an adult, (0.06, 0.26, -0.5) or (0.4, 0.26, 0.15) for a cub, rotate X by 90 degrees and, asleep, Z by 90 degrees, and submit the item with no overlay.
- **Changed (game code, `minecraft_terrain`, `families/land/foxes/render.rs`):** `BABY_ITEM_SCALE`, `HELD_AT` and `held_at` (the four offsets), `held_pose` (the layer's transforms from the head's set-up position under the model root), and `append_foxes` pushes the mouth item into `out.held` with `HeldDisplay::Ground`, lit at the eye.
- **Tests added:** `families::land::foxes::render::tests::a_held_item_sits_at_the_mouth_turned_sideways_when_asleep` (the four offsets; at the nose tip and laid flat awake, beside the curled head and turned on its side asleep, a cub's item at three quarters and its own spots, turning with the head's look and tilt; drawn: one emerald on the ground display at the eye's light, 0.2 up at the tip of the nose; a friend sees the same item at the same spot awake and asleep; nothing with an empty mouth; a cub's feather at three quarters).
- **Decisions:**
  - The layer uses the head's look (`yRot`, `xRot`) and tilt, not the head part's turn, as vanilla does: a sleeping fox's item stays where the layer puts it, beside the curled-back head.
  - The item pose goes to the shared held-item pass (`HeldDisplay::Ground`), as the panda's and the witch's do; the pass applies the item's ground display.
  - A friend's mouth item comes from the look tag (foxes-live 3), so a new item shows for a friend when the look tag is sent again.
- **Protocol:** no message or codec change.
- **Runs:** `test -p minecraft_terrain --locked --lib -- fox inside_its_cage` (12 passed), then `test -p minecraft_terrain --locked --lib` (1849 passed, 3 ignored: every test of both foxes-drawn commits, D0's registry, cage and texture tests, the gated walk test against the 26.3 jar), `check -p launcher --locked` (green; only the old render_frame and skate warnings). `render.rs` still passes `rustfmt --check --edition 2021` on its staged content.
- **Test steps** (release build, survival, Normal, by day on flat grass; name each screenshot by its label). Foxes pick up items with `foxes-items`; these steps give them items with `/summon`.
  1. `/summon minecraft:fox ~ ~ ~4 {equipment:{mainhand:{id:"minecraft:emerald",count:1}}}`: the fox carries the emerald lying flat in its jaws, at the tip of its nose, and it moves with the head as the fox looks about. **m1-foxes-mouth-1**
  2. `/summon minecraft:fox ~2 ~ ~4 {equipment:{mainhand:{id:"minecraft:feather",count:1}}}` and `/summon minecraft:fox ~4 ~ ~4 {Age:-24000,equipment:{mainhand:{id:"minecraft:rabbit_foot",count:1}}}`: a feather in the adult's mouth, and a smaller rabbit's foot in the cub's. **m1-foxes-mouth-2**
  3. Under the oak plank roof, by day, `/summon minecraft:fox ~ ~ ~2 {equipment:{mainhand:{id:"minecraft:emerald",count:1}}}` and walk 13 blocks away: once it curls up asleep, the emerald turns on its edge beside its curled-back head. **m1-foxes-mouth-3**
  4. `/locate biome minecraft:taiga`, `/tp` there and walk into new chunks: about one fox in five carries something in its mouth (an emerald, an egg, a rabbit's foot or hide, wheat, leather or a feather).
  5. Kill the fox with the emerald: the emerald drops and nothing is left in its mouth.
  6. Two windows (`scripts\mp-local.ps1`): the friend sees the same items in the same mouths, awake and asleep. **m1-foxes-mouth-4-host**, **m1-foxes-mouth-4-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:fox_spawn_egg*8,minecraft:oak_planks*64,minecraft:oak_log*16,minecraft:iron_sword,minecraft:cooked_beef*16
```

#### Group foxes-drawn: all steps together

One line for both commits' steps (`foxes-drawn` 1 and 2), 7 stacks. The friend's `.env` needs the same line.

```
IW4L_MINECRAFT_KIT=minecraft:fox_spawn_egg*16,minecraft:oak_planks*64,minecraft:oak_log*16,minecraft:lead*2,minecraft:spawner,minecraft:iron_sword,minecraft:cooked_beef*16
```

### Group foxes-hunt

#### 1. Mobs can show a block's break effect without breaking it

After 6ef1a2d9.

- **javap this round:** `Fox.tick`'s server part (`isEffectiveAi`): while faceplanted, `level().getRandom().nextFloat() < 0.2F` posts `level().levelEvent(2001, blockPosition(), Block.getId(getBlockState(blockPosition())))`. `LevelEventHandler` on 2001: for a state that is not air, the state's break sound (`SoundSource.BLOCKS`, volume `(volume + 1) / 2`, pitch `pitch * 0.8`), then `ClientLevel.addDestroyBlockEffect(pos, state)` (no bits for air, whose shape is empty). Nothing changes in the level. `Level.destroyBlock` posts the same 2001 for a real break (`destroy_block_drops` already pushes into `level.destroyed`, which the host and friends turn into that burst and sound: `minecraft_world.rs` and `minecraft_levels.rs`, `BREAK_EVENT` for friends).
- **Changed (vendored player crate):** `player/src/lib.rs`: `World::destroy_effect(pos)` right after `destroy_block`, doing nothing by default (authored worlds and tests show no effects).
- **Changed (game code, `minecraft_terrain`):** `server_mobs.rs`: `MobWorld::destroy_effect` right after its `destroy_block`: the state at `pos` goes into `level.destroyed` unless it is air or the level keeps no list; the block is not touched.
- **Tests added:** `server_mobs::tests::a_destroy_effect_shows_the_break_burst_and_keeps_the_block` (the default changes nothing; on a real level, a snow layer and the stone under it are pushed with their states, the air above is not, the snow stays; a level with no break list keeps none). It needs the 26.3 data (`MINECRAFTOSS_ROOT`, which `cargo-slot.ps1` sets) and returns early without it, as the weather test beside it does.
- **Decisions:**
  - Decision 3 as recommended: the faceplant bursts go through `World::destroy_effect` into `level.destroyed`, so they reach friends the way real breaks do, with no new message.
  - The hook skips air on the server, as the client skips the sound for air and draws no bits for it, so no empty event travels to friends.
- **Protocol:** no message or codec change; `level.destroyed` already travels as `BREAK_EVENT`.
- **Runs:** `test -p minecraft_terrain --locked --lib a_destroy_effect` (passed, with the 26.3 data), `test -p minecraftoss-player --locked --tests` with `full-suite.ps1`'s two stand-in scenario files (208 passed; the files are removed again), `test -p minecraft_terrain --locked --lib` (1850 passed, 3 ignored), `check -p launcher --locked` (green; only the old warnings), all on this commit alone.
- **Test steps:** nothing new to see in-game; no mob shows break effects until the next commit, whose steps cover it.
- **Kit:** none.

#### 2. Minecraft foxes stalk, pounce on and bite chickens and rabbits

After 593e11ee.

- **javap this round** (`Fox.registerGoals`, `setTargetGoals`, `tick`, `aiStep`, `isFullyCrouched`, `isPathClear`, `setTarget`, the selectors `lambda$registerGoals$0`/`$1` and `lambda$static$2`; `Fox$StalkPreyGoal`, `Fox$FoxPounceGoal`, `Fox$FaceplantGoal`, `Fox$FoxMeleeAttackGoal`; `JumpGoal`, `PathNavigation.moveTo(Entity, double)`, `Mth.rotLerp`, `Mth.wrapDegrees`): the M1-4 "Vanilla 26.3" numbers hold. Added or corrected on the way:
  - `registerGoals` builds the three prey goals as fields: `landTargetGoal` = `NearestAttackableTargetGoal(Animal, 10, false, false, chicken or rabbit)`, `turtleEggTargetGoal` = `(Turtle, 10, false, false, Turtle.BABY_ON_LAND_SELECTOR)`, `fishTargetGoal` = `(AbstractFish, 20, false, false, AbstractSchoolingFish)` (cod, salmon, tropical fish; not pufferfish). Their intervals are `reducedTickDelay(10)` = 5 and `reducedTickDelay(20)` = 10; `mustSee` and `mustReach` are false. `setTargetGoals` adds them to the target selector: red at 4 (land), 4 (turtle), 6 (fish); snow at 4 (fish), 6 (land), 6 (turtle). The selector's set of `WrappedGoal`s compares the goals, so a second call adds nothing: the first call fixes the priorities.
  - Goals added in vanilla's order: `FaceplantGoal` 1 (after the powder snow climb), `StalkPreyGoal` 5, `FoxPounceGoal` 6 (before `SeekShelterGoal` 6), `FoxMeleeAttackGoal(1.2000000476837158, true)` 7 (before `SleepGoal` 7), `LeapAtTargetGoal(0.4)` 10 (after the village stroll; `FoxEatBerriesGoal` 10 goes before it with `foxes-items`).
  - `StalkPreyGoal` (MOVE, LOOK; not every tick; `canContinueToUse` is `canUse`): not asleep, a live chicken or rabbit target more than 6 blocks off (`distanceToSqr > 36`), not crouching, not interested, not `jumping`. `start`: sitting and faceplanted off. `tick`: look at the target (75, 40); within 6 crouch and cock the head and stop, else `moveTo(target, 1.5)` (reach 1). `stop`: with a target and a clear path crouch, cock the head, stop and look at it, else neither.
  - `isPathClear(fox, target)`: six steps along the line (`i / 6.0F`, the x step from the z step through the slope, which a zero slope or a zero x turns round), the blocks 1 to 3 above each step `canBeReplaced`.
  - `FoxPounceGoal` (a `JumpGoal`: MOVE, JUMP; not interruptable; not every tick): `canUse` needs `crouchAmount == 5.0` (`isFullyCrouched`), a live target and a clear path; a blocked path only computes a path it throws away (`createPath(target, 0)`) and ends the crouch and the interest. `canContinueToUse`: a live target, and not (landed: `vy * vy < 0.05`, `|xRot| < 15`, on the ground) and not faceplanted. `start`: `setJumping(true)`, pouncing on, interest off, look at the target (60, 30), velocity plus (x * 0.8, 0.9, z * 0.8) of the unit vector towards it, navigation stopped. `tick`: look at the target; unless faceplanted, `vy * vy < 0.03` with a pitch eases the pitch to 0 (`rotLerp(0.2)`), else the pitch follows the flight (`-signum(vy * f) * acos(h / len) * 57.2957763671875` with `f` 6.5 while `jumping` and rising, the length above 1.0E-5F); then within 2 blocks (`distanceTo`, in floats) `doHurtTarget`, else with a positive pitch, on the ground, a vertical speed that is not 0 as a float and a `minecraft:snow` layer at its block: pitch 60, `setTarget(null)`, faceplanted. `stop`: crouching off, `crouchAmount` and `crouchAmountO` 0, interest and pouncing off.
  - `FaceplantGoal` (LOOK, JUMP, MOVE): while faceplanted, `adjustedTickDelay(40)` = 20 goal ticks (40 game ticks); `stop` ends the faceplant.
  - `FoxMeleeAttackGoal`: `canUse` is not sitting, asleep, crouching or faceplanted and then `MeleeAttackGoal.canUse`; `start` ends the interest first; `checkAndPerformAttack` is `canPerformAttack` (cooldown, reach, sight), the cooldown reset, `doHurtTarget` and `entity.fox.bite` at (1, 1), with no swing. A pounce's hit plays no bite.
  - `Fox.setTarget(null)` also ends `defending`. `jumping` is `LivingEntity.jumping`: the jump control sets it at the end of every step, so the pounce's `setJumping(true)` only counts for the goals that run after its `start` in the same step (its own first tick: `f` 6.5), and `StalkPreyGoal` sees the last step's jump.
  - `Fox.tick` (after `aiStep`, `isEffectiveAi`): the faceplant's 2001 (commit 1) before the angles step.
  - `JumpGoal`'s flags are MOVE and JUMP; `PathNavigation.moveTo(Entity, double)` is `moveTo(entity, 1, speed)`.
- **Changed (vendored entities crate):**
  - `world/families/land/foxes/ai.rs`: the constants above; `FoxGoals` gains `hunts` (the variant whose prey goals are set, none until set), `jumping`, `crouch_amount` (the fox's, for the pounce's `isFullyCrouched`), `uncrouched` (the pounce's reset of the crouch amount) and `bit` (a melee bite to sound); the goals `FaceplantGoal`, `StalkPreyGoal`, `FoxPounceGoal`, `FoxMeleeAttackGoal` (on polar-bears-live's `MeleeAttackGoal::checked` with the fox's `bite` check) and D0's `LeapGoal` at vanilla's places, with their names (18 goals); the target selector `FoxPreyGoal`, a `NearestTargetGoal::every(.., false, interval)` that only starts for a fox whose `hunts` is its variant, six of them at both variants' priorities and in vanilla's order (`TARGET_NAMES`); `is_path_clear` (the `#replaceable` block tag for `canBeReplaced`), `rot_lerp`, Java's `signum`, `distanceTo` in floats and `Vec3.normalize`; `fox_ai_step` hands the last step's jump to the goals.
  - `world/families/land/foxes.rs`: `BITE_SOUND`, `FACEPLANT_BURST_CHANCE`; `Fox::set_target_goals` (vanilla `setTargetGoals`: the first call fixes the variant's priorities); the tick hands the crouch amount to the goals, lands the step's strike as a `PlayerHitKind::Melee` hit for 2 (`family_strike`), sounds the bite, resets the crouch amount the pounce asked to reset, and after waking rolls the faceplant's level event on the level random (`World::destroy_effect` at its block, 1 in 5 a tick, not for a fox without AI).
  - `world/families/land/foxes/tests.rs`: the den gains snow layers (no collision at one layer), stone blocks, the `#replaceable` tag and a record of break effects; the tests below.
- **Changed (game code, `minecraft_terrain`):** `families/land/foxes.rs`: `read` ends with `set_target_goals`, so eggs, `/summon`, natural spawns and loaded foxes hunt with their type's priorities; a fox made in the entity world (a bred cub, with `foxes-trust`) has none until it is loaded again. Its test.
- **Tests added** (`world::families::land::foxes::tests`):
  - `a_red_fox_stalks_a_chicken_crouches_within_six_and_pounces_when_fully_crouched` (a red fox 14 blocks from a chicken stalks it, crouches with its head cocked within 6, pounces 25 to 30 ticks later up and towards it, fully crouched; a fox with no prey goals beside it takes no target).
  - `a_pounce_that_lands_within_two_bites` (a fully crouched fox 6 blocks from a chicken bites it once as it comes down, within 2 blocks, with no bite sound).
  - `a_pounce_into_snow_faceplants_for_forty_ticks_with_break_effects` (on snow layers a pounce that falls short lands faceplanted, drops its target, lies still for 40 to 42 ticks under the faceplant goal, shows 2 to 20 break effects at its own snow block, then gets up uncrouched; the chicken is unhurt).
  - `a_snow_fox_puts_fish_first` (at the goal level with a chicken, a cod and a pufferfish: a red fox ends on the chicken, a snow fox on the cod, a fox with no prey goals on nothing; a second `set_target_goals` keeps the first variant's priorities).
  - `a_blocked_pounce_paths_instead` (`is_path_clear`: blocked 1 to 3 above the line, not at 0 or 4, not by snow, diagonally and straight along z; a fully crouched fox with a block in the way stands up, never pounces, runs at the chicken with its melee goal and kills it with two bites, each with the bite sound).
  - Terrain (`families::land::foxes::tests`): `a_loaded_fox_hunts_with_the_target_goals_of_its_type` (a loaded fox with no type, a red one, a snow one and a snow cub get their type's prey goals; a fox spawned in the entity world has none).
- **Decisions:**
  - The prey goals by variant are set where the fox is read (`TerrainFamily::read`), as M1-4 plans, through `FoxGoals.hunts` and six gated target goals rather than a selector that grows at run time: `MonsterAi`'s selectors are private to `monster_ai.rs`, and gated goals of the other variant never start and never roll, so a red fox's selector acts as vanilla's three red goals and a snow fox's as its three. `foxes-trust` puts `DefendTrustedTargetGoal` (3) first in `targets()`, as vanilla registers it before `setTargetGoals` runs.
  - "The first call fixes the priorities" (vanilla's set of goals) is kept: `set_target_goals` fills `hunts` once. Vanilla's `/summon` without a tag loads the fox (red) before `finalizeSpawn` picks a snow variant, so a summoned snow fox hunts with red's priorities; the fork's `/summon` finalizes the variant into the tag before `read`, so it hunts with snow's. Only the order of fish and land prey differs, and fish are not simulated yet.
  - `isPathClear`'s `BlockState.canBeReplaced()` is asked as the `#minecraft:replaceable` block tag: the mob world's `World::replaceable` answers from authored shapes, and the 26.3 tag lists the blocks the `replaceable` property marks (air, water, lava, grass and ferns, snow layers, vines, fire and the rest).
  - The blocked pounce's `createPath(target, 0)` is not computed: vanilla throws the path away, and it only records the navigation's target and stuck timer.
  - The stalk goal works out `isPathClear` in `canContinueToUse` for its `stop`, which gets no world here; the selector stops a goal only on a full tick right after it asked every running goal to continue, so the answer is this tick's.
  - The pounce sets the context body's velocity and `leap` (D0's direct velocity, applied after the goals), so its own first tick reads the new velocity as vanilla's does.
  - The pounce writes the pitch (`look.pitch`, the entity's `xRot`), and the look control then turns it towards the target's eyes by up to 30 degrees a tick while its look cooldown lasts, as vanilla's does after the goals; the faceplant's 60 degrees is turned the same way for two ticks.
  - The faceplant's `setTarget(null)` also ends defending (vanilla `Fox.setTarget`); a target dropped by a target goal does not yet, which `foxes-trust` adds with the defend goal.
  - The leap carries the fox about 4 blocks up and 5 ahead, not more: the launch tick still starts on the ground, so vanilla's `travelInAir` takes the ground's friction (0.6 x 0.91) for that tick's drag, as the fork's travel does. A fox that crouched 6 blocks off comes down within 2 blocks of a prey that stood still.
  - While a fox lies faceplanted its prey goals may take the prey again (the target selector keeps running), as in vanilla; the faceplant goal holds the move, look and jump controls, so it stays down for its 40 ticks.
  - The pounce's `acos` is Rust's, not Java's `StrictMath.acos`; they can differ in the last bit of the tilt only.
  - `LivingEntity.getMotionDirection() == getDirection()` is always true for a living target, so the pounce skips it (M1-4).
  - The turtle slot stays `Prey::Absent` (rolling its interval) until `turtles-hunted` (M1-8) switches it to `Prey::BabiesOnLand`; the fish slot targets cod, salmon and tropical fish once M2 simulates them.
- **Protocol:** no message or codec change; the crouch, interest, pounce and faceplant flags already travel in `FamilyShow`, the pitch in the mob pose, the bursts as `BREAK_EVENT`, the bite as a sound.
- **Runs:** `test -p minecraftoss-entities --locked --lib foxes` (22 passed; the first run failed three new tests whose expectations were wrong: a trace showed the leap covers about 5 blocks, so the bite test's chicken moved from 8 to 6 blocks, the launch tick's horizontal speed after travel is 0.437, and the faceplanted fox's prey goals take the prey again while it lies, so only the faceplant tick drops it; a debug trace test was removed again), `test -p minecraftoss-entities --locked --lib` (667 passed), `test -p minecraft_terrain --locked --lib` (1851 passed, 3 ignored), `check -p launcher --locked` (green; only the old warnings). The changed fox files keep `rustfmt --check --edition 2021` clean on their staged content.
- **Test steps** (release build, survival, Normal, by day on wide flat open grass in a plains or taiga biome, away from trees and roofs; name each screenshot by its label). A fox runs from a player who walks within 16 blocks, so watch from about 20 blocks off, or sneak.
  1. Use three chicken eggs close together, walk 14 blocks away, use a fox egg on the grass, then sneak 20 blocks to one side and watch. Within a few seconds the fox trots briskly towards a chicken and stops about 6 blocks from it, crouches low with its head cocked and its rear wiggling for about a second and a half **m1-foxes-hunt-1**, then leaps at it in a high arc (about 4 blocks up and 5 ahead), its body tilted along its flight **m1-foxes-hunt-2**.
  2. When it lands next to the chicken, the chicken flashes red. When it misses, it stalks the chicken again or runs at it and bites it, with a snapping bite sound each time, until the chicken dies (two bites), then goes after the next chicken.
  3. Use a rabbit egg 14 blocks from a fox: it hunts the rabbit the same way.
  4. `/summon minecraft:fox ~ ~ ~ {Type:"snow"}` 14 blocks from chickens: the snow fox hunts them too (fish would come first, but fish swim with track M2).
  5. Cover a patch of grass about 10 by 10 with snow layers (right-click the snow item once on each block) and repeat step 1 with the chickens at the far end of it and the fox at the near end. Pounces that land in the snow away from the chicken (it wandered off while the fox crouched, or the bite knocked it away) leave the fox stuck in the snow for 2 seconds, legs paddling, with bits of snow flying up now and then and a soft snow sound **m1-foxes-hunt-3**; then it gets up and hunts again.
  6. Save, quit and load while a fox hunts: after the load it hunts on.
  7. Two windows (`scripts\mp-local.ps1`): the friend sees the same crouch with the cocked head, the tilted leap, the faceplant with its snow bits and sound, and hears the bites. **m1-foxes-hunt-4-host**, **m1-foxes-hunt-4-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:fox_spawn_egg*8,minecraft:chicken_spawn_egg*16,minecraft:rabbit_spawn_egg*4,minecraft:snow*64,minecraft:snow*64,minecraft:cooked_beef*16
```

#### Group foxes-hunt: all steps together

One line for both commits' steps (`foxes-hunt` 1 has none of its own), 6 stacks. The friend's `.env` needs the same line.

```
IW4L_MINECRAFT_KIT=minecraft:fox_spawn_egg*8,minecraft:chicken_spawn_egg*16,minecraft:rabbit_spawn_egg*4,minecraft:snow*64,minecraft:snow*64,minecraft:cooked_beef*16
```
