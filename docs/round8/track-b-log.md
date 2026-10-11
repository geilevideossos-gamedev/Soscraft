# Track B log: striders, hoglins, piglins and sleeping

Branch `track-b`, worktree `C:\Dev\mc-wt\track-b`, target dir `D:\iw4l-wt\track-b`. Briefs, in the order they run: `b3-striders.md`, `b2-hoglins.md`, `b1-piglins.md`, `b4-sleeping.md`.

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

Hit keys for MW2 bullets: track B uses 21 to 25 (20 is the dragon's `DRAGON_KEY`), track C uses 26 to 28.

## Group b3-striders-1

### 1. Minecraft players use items on mobs

After 0bcbfb17.

- **Changed:**
  - `render_anim/src/minecraft_world.rs`: the use chain tries the mob under the crosshair first, for each hand in `use_hands` order. A mob counts when it is within the entity reach (3 in survival, 5 in creative), nearer than the targeted block, and no fireball, crystal or dragon part is in front of it. A consumed click swings the hand.
  - `minecraft_survival/src/entities.rs`: `Entities::use_on_mob` picks the mob (`mob_to_use`), asks `would_interact`, and only then sends `ServerHandle::mob_action(hit, None, …)`. The answer's changed slots (`MobResult.slots`) are written back into the inventory when it arrives (`apply_mob_result`), and broken shears go to `broken_items` (the break sound and burst).
  - `minecraft_terrain/src/mob_actions.rs`: new `would_interact(world, hit, inventory, slot, infinite, player)`. `Actor` carries the acting player's id, which `villager_interact` and `wolf_interact` now get instead of a literal 0. `MobOutcome.broken` names a tool the action broke.
  - `minecraft_terrain/src/server.rs`: `Command::MobAction`, `ServerHandle::mob_action` and `ServerSim::mob_action` take the player id; `merchant_view` reads it. `MobResult.broken`.
  - This makes every existing mob interaction live in game: feeding and breeding (cows, sheep, pigs, chickens, wolves, mooshrooms), saddling pigs, milking cows, mushroom stew from mooshrooms, shearing sheep and mooshrooms, taming wolves with bones, wolf collars and sitting orders, and repairing iron golems with iron ingots.
- **Tests:** `mob_actions`: `would_interact_agrees_with_interact` (a table of 25 cases over cows, calves, pigs, saddled pigs, piglets, sheep, chickens, mooshrooms, wolves, golems and villagers, in survival and creative), `a_saddle_on_a_pig_is_used_up_with_its_sound`. `entities`: `a_mob_behind_a_block_is_not_used`, `creative_reaches_mobs_at_five_blocks_and_survival_at_three`, `a_saddle_used_on_a_pig_leaves_the_inventory_and_saddles_it` (through a real server thread; skips without the game data).
- **Decisions:**
  - `would_interact` runs `interact` on a clone of the host's copy of the mob world and of the inventory, instead of mirroring each arm by hand. It can't drift from `interact`, and the arms the later briefs add (striders, hoglins, piglins) are covered without touching it. The one exception is shears on a sheep: the host has no shearing loot table, so it answers true for any live sheep (vanilla's `Sheep.mobInteract` consumes the click whether or not the sheep is ready).
  - Villagers stay out: `would_interact` is false for them, so a right-click on a villager falls through to the item and opens nothing (no trading screen exists yet, and an opened server menu would hold the villager).
  - A click that the mob passes (`would_interact` false) goes on down the old chain, blocks behind the mob included. Vanilla would only try the item, not the block behind the mob; the old chain already did this, so nothing new breaks.
  - The `merchant` operations (`server.rs`, `MerchantOp`) still use player 0: nothing on the host opens a trading screen, so they are left for track E.
- **Test steps (survival):**
  1. Use a pig spawn egg, right-click the pig with a saddle: the saddle appears on the pig with its sound, and leaves your hand (**1**). Right-click it again with the second saddle: nothing happens, the saddle stays in your hand.
  2. Right-click the pig with carrots: hearts, one carrot used. Use the second pig egg, feed that pig too: a piglet appears (**2**).
  3. Use both cow eggs, feed them wheat: hearts, a calf (**3**). Right-click an adult cow with the empty bucket: you get a milk bucket and hear the milking sound. Right-click the calf with a bucket: nothing.
  4. Use a sheep egg and right-click the sheep with shears: wool drops, the shears lose durability (**4**).
  5. Use a wolf egg and right-click the wolf with bones until it tames (hearts, a red collar, it sits). Right-click it with an empty hand: it stands up; again: it sits (**5**).
  6. Stand a sheep behind a glass pane and right-click the pane with wheat: the sheep is not fed.
  7. `/gamemode creative`: feed a cow from 4.5 blocks away; nothing is used up. In survival the same distance does nothing.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:pig_spawn_egg*2,minecraft:saddle,minecraft:saddle,minecraft:carrot*16,minecraft:cow_spawn_egg*2,minecraft:wheat*16,minecraft:bucket,minecraft:sheep_spawn_egg,minecraft:shears,minecraft:wolf_spawn_egg,minecraft:bone*16,minecraft:glass_pane*4
```

### 1a. The mob use predicate test leaves villagers out of the comparison

After c8ccb784.

- **Changed:** `mob_actions::would_interact_agrees_with_interact` no longer compares `would_interact` with `interact` for villagers. `interact` handles a villager click (it would open trading), while `would_interact` turns it away on purpose (entry 1's decision), so only the expected answer (false) is checked for them.
- **Tests:** the same test, now green.
- **Test steps:** none (test only).

### 2. Minecraft striders stand on lava, warm up in it and shiver out of it

After 28e87e7c.

- **Changed:**
  - Vendored `entities`: new `strider.rs` (`Strider`: body 0.9 x 1.7, baby 0.45 x 0.85 with eye 0.4375, health 20, `movement_speed` 0.175 or 0.1155 while suffocating, `riding_attachment` 1.7 or 0.65625, the warmth test and `float_in_lava`) and `world/striders.rs` (`StriderEntity`, `spawn_strider`, `striders`, `strider_mut`, `tick_strider`). The tick: warmth (lava at the feet, under them or overlapping, or `#strider_warm_blocks`) sets `suffocating` unless it has no AI; fluid, fire (immune), wall and air checks; effects; ambient, hurt and death sounds (`entity.strider.*`, neutral volume, interval 120); travel (in water `travel_water`, in lava whose block is not at its feet `travel_lava`, otherwise ground travel with the lava floor); `entity.strider.step` or `step_lava` at volume 1 every 0.6 walked; fall damage except in lava; water and rain hurt 1 (`isSensitiveToWater`); age and love; then `floatStrider` (on the lava surface it is on the ground, otherwise its motion becomes `motion * 0.5 + 0.05` up). A saddled strider drops its saddle on death, besides its loot table (2 to 5 string).
  - `steering.rs`: `SteeringState::boost_factor` (`1 + 1.15 * sin(time / total * pi)` while boosting).
  - `movement.rs`: `Body.lava_floor` (ground travel stands on lava sources) and `Body.step_after` (the strider's `nextStep` of 0.6).
  - Vendored `player`: `collision::move_body_on_fluid`: with `lava_floor`, a lava source (level 0) with no lava above collides as a half slab (0 to 0.5) when the body's bottom is above `y + 0.5 - 1e-5` (`LiquidBlock.getCollisionShape` with `isAbove`). `move_body` is that without the floor, so nothing else changes. `World::fluid_is_lava`, and `#strider_warm_blocks` (lava) in the authored block tags.
  - `walk_path.rs`: the path start climbs lava rather than water for a mob that stands on fluid; `WalkProfile::strider` (water -1, lava 0, fire 0, no float) and `WalkTarget::Strider` (10 at lava, minus infinity elsewhere while in lava, else 0) for the AI of entry 3.
  - Wiring: `EntityKey::Strider`, `MobHit::Strider` and every per-kind list in `world.rs`, `combat.rs`, `pushing.rs`, `hazards.rs` and `witches.rs` (effects, healing, magic); the census counts striders as creatures.
  - `minecraft_terrain/src/server_mobs.rs`: `minecraft:strider` is simulated; `spawn_saved` reads `Age`, `InLove`, `Health`, `PersistenceRequired`, `NoAI` and `equipment.saddle`; `mob_tags` saves `Age`, `ForcedAge`, `InLove` and the saddle slot, keeping the other equipment slots as loaded. A strider with `Passengers` (a jockey) still waits dormant until entry 5.
  - `minecraft_survival/src/entities.rs`: MW2 hit key 23 for striders (`STRIDER_KEY`); the hit box comes with entry 4.
- **Tests:** `strider`: `a_strider_on_a_lava_source_stands_half_a_block_up`, `only_striders_stand_on_lava`, `out_of_lava_it_suffocates_and_slows_to_0_1155`, `water_and_rain_hurt_it`, `a_baby_is_0_45_by_0_85`, `the_boost_factor_peaks_at_2_15_halfway`, `striders_ignore_lava_and_fire`. `server_mobs`: `a_strider_round_trips_with_its_saddle_age_and_love`, `a_strider_with_a_rider_waits_dormant`.
- **Decisions:**
  - Strider movement uses ground travel even inside lava, as vanilla's `shouldTravelInFluid` does for a mob that can stand on the fluid at its block; only a strider whose block is not lava while it overlaps lava swims with `travel_lava`.
  - The suffocating speed modifier (-0.34 of the base) is applied before the potion effects' speed changes, a close match to vanilla's attribute order for the effects a strider can get.
  - Sounds use the neutral volume slider (`Entity.getSoundSource` is `NEUTRAL`; striders don't override it).
- **Test steps:** striders are drawn from entry 4 on; until then these steps show only sounds. Start in the Nether (`IW4L_MINECRAFT_DIMENSION=the_nether`), survival:
  1. Use a strider egg on the lava sea: the strider stands on the surface, half a block up (**1**).
  2. Use an egg on netherrack 5 blocks from the lava: it stands there, and turns pale and shivers (entry 4) (**2**).
  3. Pour a water bucket next to a strider on land so the water reaches it: it takes damage every half second.
  4. Kill a strider with the sword: 2 to 5 string drop.
  5. Save and exit with a strider on lava and one on land; restart: both are where they were.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*8,minecraft:water_bucket,minecraft:diamond_sword,minecraft:netherrack*64
```

### 3. Minecraft striders seek lava, follow warped fungus and breed with it

After e9e31c3e.

- **Changed:**
  - Vendored `entities`: new `strider_ai.rs`, the goal table of `Strider.registerGoals`: panic (1.65), breed (1.0), tempt (1.4, `#strider_tempt_items`), `StriderGoToLavaGoal` (`MoveToBlockGoal(1.0, 8, 2)`: only out of lava, a start delay of 100 to 200 goal ticks, the ring search over 8 blocks and 3 layers for lava with a pathfindable block above, a fresh path every 20 tries, and it keeps on until the strider is in lava or the lava is gone), follow parent (1.0), random stroll (1.0, interval 60, `DefaultRandomPos`, not water-avoiding), look at players (8), random look, look at striders (8). `world/striders.rs` runs them with navigation, move and look controls, plays `entity.strider.happy` (1 in 140 while tempted) and `entity.strider.retreat` (1 in 60 while panicking), keeps the ambient sound quiet in both, and breeds two striders into a baby.
  - New vendored `controls.rs`: `PlayerControls { id, yaw, pitch, main_hand, offhand, sneaking }`, the per-player held-item channel of track B (b1 adds the armour). `EntityWorld::set_player_controls` and `player_controls`. Striders are tempted through the world's item tags, so only `#strider_tempt_items` counts.
  - `stroll.rs`: a mob that stands on fluid takes a lava block as a stable destination (`StriderPathNavigation.isStableDestination`).
  - `strider.rs`: navigation, move control, `walk_profile`, `interact_food` (`Animal.mobInteract` with `#strider_food`) and `interact_saddle` (an adult, alive, unsaddled strider and a saddle in the hand).
  - `minecraft_terrain`: `mob_actions::interact` has a strider arm: warped fungus feeds (love, a baby's growth, the golden dandelion's age lock) with `entity.strider.eat` (volume 1, pitch `1 + (r1 - r2) * 0.2` from the strider's random), otherwise a saddle goes on with `entity.strider.saddle`. `TickInput.controls`, handed to the entity world before the mobs tick.
  - `minecraft_survival`: `PlayerView` carries `main_hand`, `offhand` and `sneaking`; `Entities::tick` sends one `PlayerControls` per living player. `render_anim` fills them (the main hand only while a Minecraft item is held, sneaking from crouch or prone).
  - Also drops a needless `mut` in entry 1's `a_saddle_used_on_a_pig_leaves_the_inventory_and_saddles_it` (a compiler warning).
- **Tests:** `strider_ai`: `a_strider_on_netherrack_walks_to_lava_within_eight_blocks`, `a_strider_in_lava_never_strolls_onto_land`, `warped_fungus_and_the_stick_tempt_it_at_1_4`. `mob_actions`: `two_fed_striders_make_a_baby`, `a_saddle_goes_on_an_adult_but_not_on_a_baby`, `warped_fungus_feeds_even_a_saddled_strider`. `controls`: `either_hand_counts`.
- **Decisions:**
  - `StriderGoToLavaGoal.canContinueToUse` doesn't call `MoveToBlockGoal`'s, so the strider never gives up after 1200 tries; checked with javap and kept that way (the brief said 1200).
  - Breeding gives no experience orbs, as pig breeding in this repo doesn't either.
  - The other species' tempt flags in `PlayerCandidate` stay false, as before (a known limitation).
- **Test steps (Nether, survival):**
  1. Use a strider egg on netherrack 5 blocks from the lava sea: within about 20 seconds it walks into the lava (**2**).
  2. Hold warped fungus near a few striders: they come to you (on land slowly), sometimes making a happy sound. Hold the warped fungus on a stick: the same. A carrot on a stick: nothing (**3**).
  3. Feed two adult striders warped fungus: each eats with its sound and shows hearts, and a baby strider appears between them (**3**).
  4. Feed the baby: it grows a little (green particles are not drawn yet).
  5. Use a saddle on an adult: the saddle is used up with the strider saddle sound (the saddle is drawn from entry 4). Try a saddle on the baby: nothing happens and you keep the saddle (**4**).
  6. Feed a saddled strider: it still eats.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*8,minecraft:warped_fungus*64,minecraft:warped_fungus_on_a_stick,minecraft:carrot_on_a_stick,minecraft:saddle,minecraft:saddle,minecraft:netherrack*64
```

### 4. Minecraft striders look like vanilla

After 0eead303.

- **Changed:**
  - New `minecraft_terrain/src/strider_render.rs`: `AdultStriderModel` (64 x 128: two 4 x 16 x 4 legs at (-4, 8, 0) and (4, 8, 0), the 16 x 14 x 16 body, six 12 x 0 x 16 bristle planes with rest rolls 0.8727, 1.1345 and 1.2217, the right ones mirrored) and `BabyStriderModel` (32 x 32: 2 x 4 x 2 legs at (-1.5, 20, 0) and (1.5, 20, 0), the 7 x 7 x 8 body at 17.25, three 7 x 3 x 0 bristles), cuboids by javap on `createBodyLayer`. `StriderModel.setupAnim` and both `customAnimations`: the legs swing and roll with the walk (speed capped at 0.25), the body rocks and bobs, the bristles sway with the walk and the age, and an unridden body takes the head's pitch and yaw. The texture is `strider`, `strider_cold`, `strider_baby` or `strider_cold_baby` by age and `suffocating`; while suffocating the body shakes by `cos(ageInTicks * 3.25) * pi * 0.4` degrees (`isShaking`). A saddled adult draws the model again with `entity/equipment/strider_saddle/saddle` (the saddle layer bakes the same adult layer).
  - `mesh.rs` loads the five textures; `client_mobs.rs` tracks striders and casts their shadows (0.5, a baby 0.25); `minecraft_survival/src/entities.rs` draws them and gives MW2 bullets their hit boxes (key 23).
- **Tests:** `strider_render`: `the_adult_has_three_cubes_and_six_bristle_planes`, `the_cold_texture_shows_while_suffocating`, `the_shake_is_cos_3_25_times_0_4_pi_degrees`, `a_saddled_adult_adds_the_saddle_layer`, `walking_swings_the_legs_against_each_other`. `entities`: `a_strider_hit_box_encodes_and_decodes`.
- **Decisions:** the spawner's displayed mob (track A) is not in this worktree, so the strider is not registered there; whoever merges second adds the one line.
- **Test steps (Nether, survival):**
  1. Use strider eggs on the lava sea: red striders with six swaying bristles stand on the lava; walking, their legs swing (**1**).
  2. Use an egg on netherrack away from the lava: it is pale purple and shivers until it reaches lava, where it turns red (**2**).
  3. Breed two (warped fungus): the baby is a small strider with three bristles, red on lava, purple on land.
  4. Saddle an adult: the saddle shows on its back (**4**).
  5. Shoot a strider with an MW2 gun: it takes damage and dies; kill one with the sword: string drops, and a saddled one also drops its saddle (**11**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*8,minecraft:warped_fungus*64,minecraft:saddle,minecraft:saddle,minecraft:diamond_sword,minecraft:netherrack*64
```

## Group b3-striders-2

### 5. Minecraft mobs ride striders

After c32a75e5.

- **Changed:**
  - New vendored `entities/src/world/riding.rs`: `Rider::Mob | Rider::Player`, `EntityWorld.passengers` (vehicle to riders), `passengers_of`, `vehicle_of`, `riding_mobs`, `rides_together`, `controlling_mob`, `seat` and `seat_offset`, `start_riding` (`canAddPassenger`: nobody on it yet, its eyes not in lava, no riding in circles), `force_riding` (loading, as `startRiding(force)`), and `stop_riding` with `Strider.getDismountLocationForPassenger` (the rider's yaw, then -22.5, +22.5, -45 and +45; each column from the strider's top down to half a block under its feet; lava skipped; a floor below 1 and a free box; on top of the strider as the last resort).
  - The seat is the strider's passenger attachment (its height, 1.7, or 0.65625 for a baby) less the rider's vehicle attachment (`ridingOffset`: 0.7 for the zombie family, skeletons and piglins, half for babies; 0.875 for wither skeletons; -0.5 for ghasts; 0.6 for players; 0 otherwise). A zombified piglin sits at +1.0, a baby strider at +1.7.
  - The tick (`tickNonPassenger`, then `tickPassenger`): riders tick right after their vehicle with their motion zeroed first (`rideTick`), then sit at the seat with their fall distance reset. A strider controlled by a mob turns off its own move, jump and look goals (`updateControlFlags`). A baby strider on top steers: its goals' navigation orders go to the strider under it, and they read that strider's navigation (`Mob.getNavigation`). A zombified piglin steers when its own navigation picks a new target: the strider paths there with its own walk profile. A strider carried by a warm strider is warm.
  - Riders and their vehicle don't push each other. Riders never despawn and count as persistent; vehicles and riders count as riding in the census (`requiresCustomPersistence`).
  - A removed vehicle (dead for 20 ticks, despawned or unloaded) lets its riders go where they sat.
  - `minecraft_terrain/src/server_mobs.rs`: `spawn_saved_all` loads a strider with its `Passengers`, recursively, each rider with its own saved tag. Any other jockey, or a strider with a rider the entity world doesn't simulate, still waits dormant as a whole. `mob_tags` saves riders inside their vehicle's `Passengers` and never on their own. `server.rs`: chunk loading, `/summon` and spawn eggs go through it, and the census passes the riding flag on.
  - Client: `ServerMob.vehicle` (the vehicle and the seat height). `ClientMobs` puts a rider on its vehicle's position every client tick, so it never lags behind. A rider's pose has no walk swing and takes its body yaw from the vehicle (`solveBodyRot`, within 85 degrees of the head). `MobPose.riding`; zombies and zombified piglins sit with their legs forward (`HumanoidModel`: -1.4137167, yaw ±0.31415927, roll ±0.07853982); their arms stay held out, as `animateZombieArms` overrides the riding arm offset.
- **Tests:** `riding`: `a_baby_rides_an_adult_at_1_7_and_stays_warm_while_it_does`, `a_zombified_piglin_sits_at_1_0_and_steers_its_strider`, `a_strider_carries_one_passenger_and_none_with_its_eyes_in_lava`, `the_dismount_spot_is_forward_never_lava_and_on_top_as_a_last_resort`, `killing_the_strider_lets_its_rider_down_where_it_sat`. `server_mobs`: `a_strider_jockey_round_trips_with_its_passenger`, `a_spider_jockey_still_waits_dormant` (they replace entry 2's `a_strider_with_a_rider_waits_dormant`). `client_mobs`: `a_rider_sits_on_its_vehicle_every_tick_without_walking`. `zombie_render`: `a_riding_zombie_lifts_its_legs_by_1_4137`. `piglin_render`: `a_riding_zombified_piglin_bends_its_legs_forward`.
- **Decisions:**
  - Javap of `LivingEntity.dismountVehicle`: a rider whose vehicle was removed stays at its own x and z, at the higher of both heights. So the brief's `killing_the_strider_drops_its_rider_beside_it` became `killing_the_strider_lets_its_rider_down_where_it_sat`: it rides the dying strider for its 20 death ticks, then falls from the seat.
  - Only riders that can drive a navigation steer a strider: zombies with their goals, and striders. Vanilla lets any mob rider steer (`Mob.getControllingPassenger`); here a skeleton or ghast on a strider rides along while the strider walks on its own, rather than freezing it.
  - A zombified piglin's goals keep reading its own navigation; only the targets it picks are handed to the strider. Vanilla gives the rider the strider's navigation object directly.
  - A mob-controlled strider runs none of its own goals: all of them use move, look or jump except `FollowParentGoal`, which never starts for an adult.
  - A rider counts as standing on the ground at its seat, so its goals can plan paths (planning needs ground or liquid).
  - Players as riders exist in the relation, but nothing mounts them yet (entry 7b).
- **Test steps (Nether, survival; `IW4L_MINECRAFT_DIMENSION=the_nether`):** stand on netherrack by the lava sea.
  1. `/summon minecraft:strider ~ ~1 ~ {Passengers:[{id:"minecraft:zombified_piglin",equipment:{mainhand:{id:"minecraft:warped_fungus_on_a_stick",count:1}}}],equipment:{saddle:{id:"minecraft:saddle",count:1}}}`: the zombified piglin sits on the saddled strider with its legs forward and the stick in its hand, and moves with it without lagging behind (**9**).
  2. Watch it for a minute: the strider walks where the piglin wanders. Hit the piglin: it turns on you and the strider carries it towards you.
  3. `/summon minecraft:strider ~ ~1 ~ {Passengers:[{id:"minecraft:strider",Age:-24000}]}`: a baby strider rides the adult, red while the adult is warm, pale and shaking together with it on land (**10**).
  4. Kill a strider that carries a rider with the sword: the rider stays on top while the strider dies, then drops down where it sat.
  5. Save and exit next to a jockey, restart: the jockey is still one strider with its rider (**12**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*8,minecraft:diamond_sword,minecraft:netherrack*64
```

### 6. Minecraft striders spawn on Nether lava, some with a rider

After 225276bd.

- **Changed:**
  - Vendored `world/src/natural_spawner.rs`: `Rules::Strider` (`checkStriderSpawnRules`: up from the block above the spawn through lava; the first block that isn't lava must be air), used by natural spawning, spawners and chunk generation. The strider's walk target check passes as `PathfinderMob`'s 0 does (`Walk::Neutral`; its value is 10 in lava and 0 elsewhere when it spawns), and its obstruction test is entities only (`Strider.checkSpawnObstruction` has no liquid test). A `finalizeSpawn` arm for `minecraft:strider` (the ageable part).
  - `natural_spawner/spawning.rs`: `finalize_strider` (`Strider.finalizeSpawn`): one in 30, when monsters may spawn, a zombified piglin jockey (its baby roll at 5 %, its own zombified piglin finalize without a chicken jockey, then `warped_fungus_on_a_stick` in its main hand), and the strider gets a saddle with a guaranteed drop (`drop_chances:{saddle:2.0f}`); otherwise one in 10 a baby strider rider (`Age` -24000). A fresh group every time, so a natural adult never brings a baby. Natural spawning, `/summon` without NBT, spawn eggs and spawners all go through it. `SpawnContext.spawning_monsters` carries the game rules `spawn_mobs` and `spawn_monsters` (`ServerLevel.isSpawningMonsters`).
  - `level/spawning.rs` hands the rule to the context in the three places that make one.
- **Tests (`minecraft_terrain` `server`, with the game data):** `striders_spawn_in_lava_with_air_above_and_not_under_a_lava_roof`, `a_strider_in_lava_is_not_obstructed_by_the_lava`, `one_in_thirty_get_a_zombified_piglin_with_a_stick_and_a_saddle` (3000 summons; it also checks that a natural adult never brings a baby, and that a summoned jockey tag arrives with its rider), `no_piglin_rider_when_spawn_monsters_is_off`, `a_nether_level_spawns_striders_on_its_lava_sea` (seed 0, 121 chunks, six creature rounds).
- **Decisions:**
  - The brief's `natural_spawner` and `finalize` tests need the real registries, so they live with the Nether `ServerSim` tests in `server.rs`; `a_natural_adult_never_brings_a_baby` is an assertion inside the 3000-summon test.
  - Striders placed by chunk generation come without riders: generation has no spawn context (difficulty, game rules) to finalize a zombified piglin with.
- **Test steps (Nether, survival):**
  1. Fly along the lava sea for a few minutes: wild striders appear in groups of 1 or 2, standing on the lava; a few carry a baby, or a zombified piglin with a warped fungus on a stick on a saddle (**13**).
  2. Use about 30 strider eggs on the lava: roughly one in ten hatches with a baby on top, and now and then one with a zombified piglin.
  3. `/gamerule spawn_monsters false`, then use 30 more eggs: babies still come, zombified piglins never.
  4. Kill a jockey's strider: its saddle always drops.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*64,minecraft:diamond_sword,minecraft:netherrack*64
```

### 6a. The Nether structure test leaves the lava sea's striders out

After f3c92b83.

- **Changed:** `terrain::nether_fortresses_bastions_and_every_nether_feature_generate` checks that a fortress's and a bastion's mobs never despawn. Since entry 6, chunk generation also places striders on the lava around them (biome creatures, not structure mobs, and saved without `PersistenceRequired`, as vanilla saves them), so the check leaves `minecraft:strider` out.
- **Tests:** the same test, now green.
- **Test steps:** none (test only).

### 7a. MW2 players can be pinned to a seat

After 642029f2.

- **Changed:**
  - `playerstate_iw4`: `pm_flags::MC_RIDING = 0x0100_0000`. `pm_flags` is replicated whole (`net/src/transport/netfields.rs`), so the bit needs no network change.
  - `movement_iw4`: new `ride.rs`. While `MC_RIDING` is set, `pmove` turns the view, then drops sprinting, jumping, mantling, ladders, gliding and the stance, sets `CROUCH` while the crouch or prone button is held (the sneak the adapter will read), zeroes the velocity and returns before any move.
  - `sim`: `SimWorld::ride(id, Some(seat))` sets the bit and moves the player there like `gate_nudge_origin` (velocity zeroed, no `TELEPORT` bit, the player area follows); `ride(id, None)` clears the bit. The carrier forwards it.
  - No game behaviour yet: nothing calls `ride` until entry 7b (and b4's beds).
- **Tests:** `movement_iw4` `ride`: `crouch_while_riding_sets_the_crouch_flag_only`. `sim` `voxel` (the full pmove with its real context): `a_riding_player_turns_but_never_moves_jumps_or_sprints`, `the_next_pmove_after_riding_is_normal`. `sim` `frame`: `ride_moves_without_the_teleport_bit_and_clears_with_none`, and the bit test in `an_elytra_lets_a_player_glide_and_a_rocket_lights_only_while_gliding` now covers `MC_RIDING`.
- **Decisions:** the two full-pmove tests sit in `sim`, which builds the real pmove context (`step::pmove_context`) and a voxel world; `movement_iw4` keeps the unit test of the seat itself.
- **Test steps:** none in game (the pin has no caller yet); entry 7b's steps cover it.

## Group b3-striders-3

### 7b. Minecraft players ride saddled striders

After a0bc99d1.

- **Changed:**
  - `minecraft_terrain/src/mob_actions.rs`: the strider arm mounts first (`Strider.mobInteract`): held item not `#strider_food`, saddled, alive, nobody on it and the player not crouching (its `PlayerControls.sneaking`). `MobOutcome.mount` names the strider; the click is consumed and nothing is used up.
  - `minecraft_terrain/src/server.rs`: `ServerSim::mob_action` seats the player with `start_riding(Rider::Player(id), …)` (eyes not in lava, one passenger) and answers `MobResult.mounted`. Each tick, after the controls arrive, a riding player who crouches is let off (`Player.rideTick`) at the strider's dismount spot (`ride_tick`); a strider that is removed (dead for 20 ticks, despawned, unloaded) lets its player go where it sat (`take_vehicle_losses`). Both arrive as `Output.dismounts` (player, vehicle, spot or none). `Command::Dismount { player }` unseats a player without an answer (forced dismounts).
  - Vendored `entities`: `prune_riding` keeps the players whose vehicle went (`take_player_dismounts`), `dismount_player` and `drop_player`; a player never pushes its own vehicle (`push_from_players`; the rider's box overlaps the strider's).
  - `minecraft_terrain/src/client_mobs.rs`: `ClientMobs::set_own_vehicle`: the player's own vehicle takes each server tick's position directly instead of the three-step interpolation, so the drawn strider and the seat move together, one tick behind the server like vanilla's own-vehicle view.
  - New `minecraft_survival/src/riding.rs`: `Riding { vehicle, uuid }` (`Player.riding`), `seat` (the vehicle's feet lerped between its last two ticks, plus 1.7 - 0.6) and `bob` (`Strider.getPassengerAttachmentPoint`'s client bob, `0.12 * cos(walkPos * 1.5) * 2 * min(0.25, walkSpeed)`). `Entities::ride_seat`, `take_mounts`, `take_dismounts`, `dismount`; `shot_boxes` leaves out the rider's own vehicle (`Projectile.canHitEntity`).
  - New `render_anim/src/minecraft_riding.rs`: each frame it takes mounts and dismounts (a dismount teleports the MW2 player to the spot and clears `MC_RIDING`), unseats a dead or spectating player, and pins the MW2 player to the seat with 7a's `SimWorld::ride`. `/tp`, ender pearls and leaving the level unseat first. A rider counts no steps and no fall.
- **Tests:** `mob_actions`: `right_click_mounts_a_saddled_strider_unless_crouching_or_holding_fungus`. `server` (with the game data): `crouching_dismounts_forward_onto_land`, `dismounting_over_open_lava_lands_on_top_of_the_strider`, `a_removed_strider_lets_its_player_go_where_it_sits`. `client_mobs`: `the_players_own_vehicle_moves_with_each_server_tick`. `riding` (survival): `the_seat_is_1_1_above_the_strider_and_bobs_with_its_stride`, `the_seat_lerps_between_the_last_two_server_ticks`. `minecraft_riding` (render_anim): `a_rider_counts_no_steps_and_no_fall`. Entry 3's `a_saddle_goes_on_an_adult_but_not_on_a_baby` now expects a second saddle, or any other item but warped fungus, to ride the saddled adult, as vanilla does.
- **Decisions:**
  - The seat follows the client's copy of the strider, not a separate track of server positions: the own vehicle is taken off interpolation in `ClientMobs` instead, so the strider drawn under the player and the player's seat can't drift apart.
  - A strider whose vehicle was removed gives no dismount spot: the player stays where it sat and falls from there, as `dismountVehicle` leaves a rider of a removed vehicle in place.
  - Forced dismounts (death, spectator mode, `/tp`, pearls, a level switch) unseat on the host at once and tell the server without an answer, so a late answer can't move the player after a teleport. A dismount answer for a vehicle the player no longer rides is ignored.
  - MW2 bullets and blasts skip the rider's own strider. The MW2 knife uses the same boxes, so it skips it too; the Minecraft hand still reaches it.
  - The player is the server's player 0 (`PLAYER`) as everywhere in `Entities`; every new message carries the player id for track E.
- **Test steps (Nether, survival; `IW4L_MINECRAFT_DIMENSION=the_nether`):** stand on netherrack by the lava sea.
  1. Use a strider egg on the lava and a saddle on the strider. Right-click it with an empty hand (or the stick): you sit on it, your eyes about 2.7 blocks above its feet, and the view bobs gently as it walks (**5**).
  2. Without the stick it wanders on its own and carries you; you can look all around and still shoot and use items.
  3. Right-click a saddled strider with warped fungus: it eats instead. Crouch and right-click: nothing.
  4. Crouch while riding next to land: you step off forward onto the block you look at (**8**). Ride out over open lava and crouch: you land on top of the strider, never in the lava.
  5. Ride and shoot straight down with an MW2 gun: your strider takes no hit; another strider does.
  6. Ride, then `/tp ~ ~5 ~`: you leave the strider and stand where the command sent you. Ride again and throw an ender pearl: you land at the pearl, off the strider.
  7. Ride a strider and kill it with the sword from its back: you stay seated during its death, then drop down where you sat, without fall damage from the ride.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*8,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:warped_fungus*16,minecraft:warped_fungus_on_a_stick,minecraft:diamond_sword,minecraft:ender_pearl*4,minecraft:netherrack*64
```

### 8. Minecraft warped fungus on a stick steers and boosts striders

After 65532cc9.

- **Changed:**
  - Vendored `entities`: `controlling_player` (`Strider.getControllingPassenger`): a saddled strider whose first passenger is a player holding `warped_fungus_on_a_stick` in either hand. That strider runs `tickRidden` each tick (its yaw, body and head take the rider's yaw, its pitch half the rider's, `tickBoost`) and travels with input (0, 0, 1) at `getRiddenSpeed`: `movement_speed` (with effects) times 0.55, or 0.35 while suffocating, times the boost factor. Its goals keep running as vanilla's do (a player controller is no `Mob`), but their moves don't reach the travel. Without the stick the strider walks on its own with the player on it. `boost_ride(player)`: only the controlling rider, and only while no boost runs (`ItemBasedSteering.boost`, 140 to 980 ticks from the strider's random).
  - `minecraft_terrain/src/server.rs`: `Command::Boost { player }` answers `Output.boosts` (player, boosted).
  - New `minecraft_terrain/src/durability.rs`: `wear` (`hurtAndBreak` with Unbreaking's `remove_binomial` for non-armour, `level / (level + 1)` per point), `wear_and_convert` (`hurtAndConvertOnBreak`: a broken stack becomes the given item with its components and no damage) and `spend_boost` (1 point, a `fishing_rod` when it breaks, nothing in creative). The trident's wear now goes through `wear`.
  - Host: the use chain, after the block uses and before the item uses, sends the boost when the player rides a strider it controls that is not boosting yet (`Entities::can_boost`, on the host's copy); otherwise the stick passes on. A `true` answer swings that hand and spends 1 durability (unless the game mode has infinite materials); a broken stick turns into a fishing rod with the break sound and burst.
- **Tests:** `riding` (vendored): `a_warm_ridden_strider_reaches_4_24_blocks_a_second_and_a_cold_one_1_78` (on the lava sea and on netherrack, steady speed within 1 % over the last 10 of 100 ticks), `it_turns_to_the_riders_yaw_at_once`, `without_the_stick_the_rider_does_not_steer` (the off hand steers too, an unsaddled strider never), `boost_lasts_140_to_980_ticks_and_a_second_boost_fails_meanwhile` (the peak is 2.15 times the speed). `durability`: `a_boost_costs_one_of_a_hundred`, `unbreaking_three_spares_three_in_four`, `the_hundredth_boost_leaves_an_enchanted_fishing_rod`, `creative_boosts_cost_nothing`.
- **Decisions:**
  - Boost answers have their own channel (`Output.boosts`, keyed by player) rather than `item_uses`, so a boost and a bucket or eye use waiting at the same time can't take each other's answer.
  - Whether a boost can start is checked on the host's copy of the strider before sending (controlled, not boosting), so a use that vanilla would `PASS` goes on down the use chain instead of being swallowed while the answer is on its way.
  - The boost sits after the block uses, as vanilla tries `useItemOn` on the targeted block before `FoodOnAStickItem.use`: right-clicking a chest while riding still opens it.
- **Test steps (Nether, survival):** saddle a strider on the lava sea.
  1. Hold the warped fungus on a stick and right-click the strider: you sit on it, and it walks wherever you look at about 4 blocks a second, without any key (**6**).
  2. Right-click with the stick: it speeds up to about twice that and slows again over 7 to 49 seconds; the stick's durability bar drops by one. Right-click again during the boost: nothing happens and the bar stays.
  3. Switch to a slot without the stick: the strider stops following your look and wanders on its own. Put the stick in the off hand instead: it steers again.
  4. Steer onto netherrack: it slows to under 2 blocks a second, turns pale and shivers; back on lava it warms up and speeds up (**7**).
  5. With the Unbreaking III stick, boost about 20 times: the bar drops only about 5 times.
  6. `/gamemode creative` and boost: the bar never drops.
  7. Take the nearly broken stick (the kit's third, damage 99) and boost once: it breaks with the break sound and leaves a fishing rod in the hand.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*8,minecraft:saddle,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:warped_fungus_on_a_stick+unbreaking=3,minecraft:warped_fungus_on_a_stick+damage=99,minecraft:netherrack*64
```

### 9. Minecraft players riding a strider are saved with it

After 0338a62f.

- **Changed:**
  - `minecraft_survival/src/player_nbt.rs`: `RootVehicle` joins the keys the player file owns. `PlayerData.root_vehicle` reads it (only with an `Entity`) and writes it back whole: `{Attach: <the player's own vehicle's UUID>, Entity: <the root vehicle with its mob passengers>}` (`ServerPlayer.saveParentVehicle`). A player who got off writes none, so a stale one is dropped from the file.
  - `minecraft_terrain/src/server.rs`: `root_vehicle(player, remove)` builds that compound from the entity world (the strider's saved tag, its riders nested in `Passengers`, never the player). With `remove` it also takes the strider and its mob riders out of the level, so the chunk saves no copy. `reattach(player, root)`: unless an entity with the root's UUID is already in the level (`addWithUUID`), it spawns the saved vehicle with its riders and seats the player on the one whose UUID is `Attach`; otherwise it logs "Couldn't reattach entity to player" and discards the vehicle. Chunk entity loading now refuses an entity whose UUID is already in the level. `Command::RootVehicle` and `Command::Reattach` answer `Output.root_vehicles` and `Output.mounts`.
  - Vendored `entities`: `root_vehicle`, `with_mob_riders`, `mob_with_uuid`, `remove_mobs`.
  - `minecraft_survival`: `Player.saved_vehicle` (the last `RootVehicle` taken from the server) and `Player.reattach` (the one read at join, not sent yet). Every autosave refreshes the riding players' vehicles and keeps them in the level too, as vanilla's autosave does; save-and-close (`Server::take_vehicles`, before any level saves its entities) takes them out of the level into the player file. At join a living player's `RootVehicle` waits in `reattach`, and the riding step sends it once the player is in the level; the seat comes back through the usual mount answer.
- **Tests:** `player_nbt`: `a_rider_saves_root_vehicle_with_attach_and_entity`, `a_vanilla_root_vehicle_reads_back`. `server` (with the game data): `closing_while_riding_keeps_one_strider_in_the_player_file_and_none_in_the_chunk` (the autosave copy keeps the strider in the level, closing takes it out, reattaching brings it back with its UUID and saddle), `a_chunk_copy_with_the_ridden_uuid_is_dropped`, `a_root_vehicle_without_its_attached_entity_is_discarded`.
- **Decisions:**
  - The server builds the `RootVehicle` when it is needed (autosave, closing) with a short wait for the server thread, rather than streaming a copy every tick.
  - A player file whose player died keeps no vehicle to reattach (a dead player rode nothing).
  - Track E: a friend's leave should call the same `root_vehicle(player, true)` before the friend's file is written.
- **Test steps (Nether, survival):**
  1. Saddle a strider on the lava sea, ride it with the stick, then save and exit to the menu. Restart the world: you sit on the same strider, it still has its saddle, and there is no second strider under you (**12**).
  2. Ride for more than five minutes (an autosave passes), then close the game from the task manager. Restart: you ride the strider, and still only one is there (the chunk's copy is refused).
  3. Get off, save and exit, restart: you stand where you got off and the strider is on the lava where it was.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*4,minecraft:saddle,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:netherrack*64
```

### 9a. A rejoining rider sits on the chunk's copy of its strider when that loaded first

After 75e8b766.

- **Changed:** `ServerSim::reattach`: when an entity with the saved vehicle's UUID is already in the level, the player now rides the loaded entity whose UUID is `Attach` instead of riding nothing. After a crash that skipped the close (the strider is both in its chunk, from the autosave, and in the player file), the chunk can load before the player's vehicle is sent; vanilla attaches the player before chunk entities load, so the player still rides.
- **Tests:** `server`: `a_rider_whose_strider_loaded_with_its_chunk_first_sits_on_that_one`; `a_chunk_copy_with_the_ridden_uuid_is_dropped` no longer expects a second reattach to fail.
- **Decision:** loading the vehicle twice still makes one strider; only which copy the player sits on depends on the order, and both are the same saved strider.
- **Test steps:** entry 9's step 2 covers it: after the forced close the player rides the one strider whichever loads first.

### 9b. Minecraft riders don't travel through portals

After d0871eb2.

- **Changed:** `minecraft_survival/src/portal.rs`: `may_use_portals` (`Player.canUsePortal(false)`: alive, not a passenger), used by the portal step. A player riding a strider never counts as inside a Nether or End portal: no swirl, no trip. Section 6 of the brief listed it with 7b; it was left out there.
- **Tests:** `player`: `a_rider_never_uses_a_portal`.
- **Test steps (Nether, survival):**
  1. Build a Nether portal frame next to the lava sea and light it. Ride a strider (with the stick) into the portal: nothing happens, no purple swirl. Crouch to get off inside the portal: the swirl starts and you travel after 4 seconds.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*2,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:netherrack*64
```

### 10. Docs: Minecraft striders and riding

After 2f83e57d.

- **Changed:** `docs/MINECRAFT-SURVIVAL.md`: right-click on mobs in the item list; a "Striders and riding" part under the Nether (spawning, lava, warmth, jockeys, a player's ride, steering, the boost, getting off, saving); `RootVehicle` in the player file; `riding.rs` and `minecraft_riding.rs` in the crate layout; striders leave N6 and the dormant list; the riding limitations from the brief's section 6 (only striders carry passengers, pigs can't be ridden, the standing MW2 body, the one-tick-behind server seat, non-steering riders, no portal travel for mounts, no saddle shearing, tempting only for striders). `docs/NETHER-END.md`: the strider row is present.
- **Decision:** the vendored MinecraftOSS count (CLAUDE.md, "patched 53 of its files and added 14 (as of round 7)") is not edited in this track, because tracks A, C and E change it too and would conflict at the merges. As of this commit track B alone makes it 57 patched and 19 added (`git diff --name-status 114c18e1 HEAD -- third_party/minecraftoss`); the round's hand-over recounts it after the last merge.
- **Test steps:** none (docs only).

## Group b2-hoglins-1

Brief `b2-hoglins.md`, groups H1 (commits 1 and 2) and H2 (commit 3). Numbered b2.1 and on, after the brief's commits.

### b2.1. Minecraft hoglins and zoglins live in the entity world

After f58c8593.

- **Changed:**
  - New vendored `entities/src/hoglin.rs`: `HoglinKind { Hoglin, Zoglin }` and `Hoglin` (body, health 40, `Age` for the hoglin and a `baby` flag for the zoglin, `in_love`, `attack_animation`, `time_in_overworld`, `immune_to_zombification`, `cannot_be_hunted`, `persistence_required`, `converting`). The 26.3 numbers: 1.3964844 x 1.4 with eye 1.19, a baby 0.75 x 0.85 with eye 0.625, speed 0.3, knockback resistance 0.6, attack knockback 1, damage 6 (0.5 for a baby), experience 5 (3 for a baby hoglin, 5 for any zoglin), `CONVERSION_TIME` 300. `damage_roll` (`hurtAndThrowTarget`: 3 to 8 for an adult, a flat 0.5 for a baby) and `throw_push` (`throwTarget`: `nextInt(21)` first, then the two `nextFloat`s; the `Vec3.yRot` turn in radians with `Mth`'s table, as vanilla).
  - New vendored `entities/src/world/hoglins.rs`: `HoglinEntity`, `spawn_hoglin`, `hoglins`, `hoglin_mut`, `tick_hoglin`. The tick: fluids, fire (the zoglin is immune), suffocation, drowning (the zoglin is undead and breathes under water), effects (inverted for the zoglin), the ambient sound every 80 ticks on average, hurt and death sounds (`entity.hoglin.*` or `entity.zoglin.*`, hostile volume), the step at 0.15, travel with no input yet (the brain comes with H3), fall damage (hostile fall sounds for the zoglin), the attack animation countdown, and the hoglin's age and love.
  - `world.rs` and its parts: `EntityKey::Hoglin` and `MobHit::Hoglin` (one storage for both kinds), and every per-kind list: entity type, census, despawn (removable far away, gone in peaceful), positions, bodies, damage states, voices, burning, blasts, the host copy, the bullet boxes, the death sweep, pushing, potions and effects, player hits, mob hits and arrows.
  - Knockback resistance: `knock_back` scales a hoglin's knockback by 1 - 0.6 (`LivingEntity.knockback`), so player hits, sprint hits, mob hits and arrows all push it less; an iron golem's lift on it is scaled the same way (`IronGolem.doHurtTarget`). Blasts push it fully (its explosion knockback resistance is 0).
  - Deaths: a hoglin drops `entities/hoglin` only as an adult, a zoglin `entities/zoglin` at any age; the experience (5, 3 for a baby hoglin) only for a player's kill, as for the other monsters. A burning hoglin's death is marked on fire, so its porkchops are cooked.
  - `MobCandidate.baby`, filled for every kind (the zombies' villager list included), for the brain of H3 and b1.
  - `effects.rs`: `MobEffect::Nausea` (`minecraft:nausea`, colour 5578058, no work on mobs), at the end of the enum.
  - `minecraft_survival/src/entities.rs`: MW2 hit key 22 for hoglins and zoglins (`HOGLIN_KEY`); the boxes come with b2.3.
- **Tests:** `hoglin`: `an_adult_rolls_three_to_eight_and_a_baby_half_a_heart`, `the_throw_lifts_at_most_half_a_block_and_spares_full_resistance`, `a_hoglin_baby_is_aged_and_a_zoglin_baby_is_flagged`. `world::hoglins`: `a_hoglin_and_a_zoglin_have_forty_health_and_vanilla_sizes`, `a_baby_hoglin_drops_experience_but_no_loot_and_a_baby_zoglin_drops_both`, `a_burning_hoglin_drops_cooked_porkchops` (the 26.3 tables from `MINECRAFTOSS_ROOT\client.jar`: 2 to 4 porkchops, at most one leather, cooked when it burns), `hoglins_and_zoglins_leave_on_peaceful`, `a_zoglin_shrugs_off_lava_and_a_hoglin_burns`, `a_hoglin_takes_four_tenths_of_the_knockback`.
- **Decisions:**
  - The throw draws from the hoglin's own random, not a level random (the entity world has none for this); the order of the draws is vanilla's.
  - A hoglin is an `Animal`: its idle clock runs one a tick and its swim, splash and fall sounds are the generic ones. A zoglin is a `Monster`: two more a tick in bright light, and the hostile sounds.
  - A mob with `NoAI` stands still and only keeps its idle clock, as the other mobs here do.
- **Test steps:** none in game yet: hoglins and zoglins are simulated from b2.2 on.

### b2.2. Minecraft hoglins and zoglins are saved and spawn in crimson forests and bastions

After 8f545881.

- **Changed:**
  - `minecraft_terrain/src/server_mobs.rs`: `minecraft:hoglin` and `minecraft:zoglin` are simulated. `spawn_saved` reads `Age`, `ForcedAge`, `InLove`, `IsImmuneToZombification`, `TimeInOverworld` and `CannotBeHunted` for the hoglin and `IsBaby` for the zoglin; `mob_tags` writes them back (always, as `Hoglin.addAdditionalSaveData` does) and keeps the other keys as loaded. Hoglins that waited dormant in a saved world (bastion stables, crimson forests) come alive when their chunk loads.
  - Vendored `world/src/natural_spawner.rs`: the hoglin's spawn data (1.3964844 x 1.4, on the ground, motion blocking without leaves, not fire immune, removed far away, groups of up to 4) with a new `Rules::Hoglin` (`checkHoglinSpawnRules`: anything but a nether wart block below; no light test). The zoglin's (fire immune, `checkAnyLightMonsterSpawnRules`, the monster walk value) for spawners and `/summon`; it never spawns naturally (no biome lists it).
  - `natural_spawner/spawning.rs`: the hoglin's `finalizeSpawn`: one in five a baby, then `AgeableMob`'s group roll (one in twenty for every member after the first; a natural group of 3 or 4 shares it), then `Mob`'s part. The zoglin's: one in five a baby (`IsBaby`), then `Mob`'s part. Natural spawning, spawn eggs, spawners and `/summon` without NBT go through it.
  - Vendored `generator/src/feature/entities.rs`: a hoglin or zoglin placed by a structure (the bastion stables) is a baby one time in five, from the level random, before `Mob`'s part.
  - Vendored `entities/src/monster_ai.rs`: `is_enemy` lists the hoglin and the zoglin (both `Enemy`), so iron golems hunt them.
- **Tests:** `server_mobs`: `hoglins_and_zoglins_keep_their_age_conversion_clock_and_flags_through_a_save`. `server` (with the game data): `a_crimson_forest_spawns_hoglins_in_groups_but_not_on_nether_wart_blocks` (seed 0, the nearest crimson forest, 120 ticks of monster spawning around a player). `spawning`: `a_hoglin_group_rolls_one_in_five_babies_and_a_further_twentieth_after_the_first` (20 000 groups of 4: the first member 20 %, the others 24 %). `terrain` (with the game data): `bastion_hoglins_come_alive_and_stay_persistent` (the first hoglin stables among seed 77's bastions: every stable hoglin loads as a simulated, persistent hoglin). `world::hoglins`: `an_iron_golem_goes_for_a_hoglin`.
- **Decisions:**
  - The hoglin's walk target value at spawn is 10 on crimson nylium and 0 elsewhere (`-1` needs a remembered repellent, which a mob being created has none of), so it never fails the spawn check: the spawner uses the neutral walk check instead of a new `Walk::Hoglin` (the brief's), with the same outcome. The value for strolling comes with the brain (b2.5).
  - The brief's bastion test became its own test: the bastion nearest seed 77's origin (the one `nether_fortresses_bastions_and_every_nether_feature_generate` uses) has no stables, so the new test walks out to the first bastion that has.
  - The group baby roll is a small function (`hoglin_spawns_as_baby`) so it can be tested without a level.
- **Test steps:** together with b2.3's (hoglins are drawn from b2.3 on).

### b2.3. Minecraft hoglins and zoglins are drawn like vanilla

After 61ad7269.

- **Changed:**
  - New `minecraft_terrain/src/hoglin_render.rs`: the adult model (`HoglinModel`, 128 x 64: body with its mane plane, head with two ears and two horns, two front and two hind legs; 11 cubes) and the baby model (`BabyHoglinModel`, 64 x 64: a three-cube head with its ears, a two-cube body, four legs; 11 cubes), cuboids, texture offsets and poses by javap on `createBodyLayer`. `hoglin_pose` is `setupAnim`: the ears hang at 40 degrees and flap with the walk, the head turns with the head yaw, the headbutt (`1 - |10 - 2 * attackTicks| / 10`) lifts the head from 50 degrees down to 20 degrees up and back over the 10 attack ticks (a piglet's head also drops 2.5 pixels at the peak), and the legs swing (`cos(walkPos) * 1.2 * walkSpeed`, the hind legs copying the opposite front legs). The four textures (`hoglin`, `hoglin_baby`, `zoglin`, `zoglin_baby`). A converting hoglin shakes (`HoglinRenderer.isShaking`: `cos(ageInTicks * 3.25) * pi * 0.4` degrees on the body yaw, driven by `Hoglin.converting`, which b2.7 sets); a zoglin never does. The dying tip-over is `LivingEntityRenderer`'s 90 degrees. Shadow 0.7, 0.35 for a baby.
  - `mesh.rs` loads the four textures; `client_mobs.rs` tracks hoglins and zoglins (eye 1.19 or 0.625) and casts their shadows; `minecraft_survival/src/entities.rs` draws them and gives MW2 bullets their hit boxes (key 22, `HOGLIN_KEY`, from b2.1).
- **Tests:** `hoglin_render`: `an_adult_and_a_baby_hoglin_have_eleven_cubes_each` (264 vertices each, feet 24 pixels down), `the_head_butts_up_and_back_over_ten_ticks`, `the_ears_hang_at_forty_degrees_and_flap_with_the_walk`, `a_converting_hoglin_shakes_and_a_zoglin_does_not`. `entities` (survival): `a_hoglin_hit_box_encodes_and_decodes`.
- **Decisions:**
  - The model's two cube deformations (0.001 pixel on the mane, 0.02 pixel on the piglet's body) are left out: they only grow the boxes by a fraction of a texel.
  - Until the brain (b2.4 on), hoglins and zoglins stand where they are: they fall, swim, burn, take hits and knockback, but don't walk, look around or attack.
- **Test steps (b2.1 to b2.3; start in the Nether, `IW4L_MINECRAFT_DIMENSION=the_nether`, survival, `/difficulty normal`, on a netherrack floor):**
  1. Use a hoglin spawn egg: a big pink tusked boar with a mane and two horns stands there and grunts now and then (**1**). It doesn't move yet.
  2. Use 10 more hoglin eggs: about two of them are piglets (small, with a round head and tiny tusks) (**2**).
  3. Use a zoglin egg: a grey, rotting zoglin with the same shape (**3**).
  4. Hit an adult hoglin with the sword: it is pushed back much less than a pig would be. Kill it: 2 to 4 raw porkchops, sometimes a leather, and experience orbs. Kill a piglet: only experience.
  5. Set the netherrack under a hoglin alight with flint and steel and kill it while it burns: the porkchops come out cooked (**4**).
  6. Kill a zoglin: 1 to 3 rotten flesh and experience. Pour lava under another zoglin: it stands in the lava without burning; a hoglin in lava burns.
  7. Use an iron golem egg next to a hoglin: the golem walks over and hits it (**5**).
  8. Shoot a hoglin with an MW2 gun: it takes damage and dies.
  9. Save and exit, restart: the hoglins, piglets and zoglins are back where they were, piglets still small.
  10. `/locate biome minecraft:crimson_forest` and go there: hoglins stand in groups of up to four on the red nylium (**6**). `/locate structure minecraft:bastion_remnant`: a hoglin stable has its hoglins (**7**). Hoglins already saved in the world wake up as these.
  11. `/difficulty peaceful`: every hoglin and zoglin vanishes.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:hoglin_spawn_egg*16,minecraft:zoglin_spawn_egg*4,minecraft:iron_golem_spawn_egg,minecraft:diamond_sword,minecraft:flint_and_steel,minecraft:lava_bucket,minecraft:netherrack*64
```

## Group b2-hoglins-2

Brief `b2-hoglins.md`, groups H3 (commits 4 to 6), H4 (commits 7 and 8) and H5 (commit 9).

### b2.4. Minecraft hoglins charge players and throw them upward

After 2c1b64c3.

- **Changed:**
  - New vendored `entities/src/hoglin_brain.rs`: the hoglin's `Brain` (`HoglinAi`). Memories on `villager_brain::Slot` (attack, avoid and breed targets, pacified, walk and look targets, `cant_reach_walk_target_since`, the attack cooldown, the nearest repellent, the visible adult piglin and hoglins with their counts, the nearest visible attackable player and adult). Four sensors on 20-tick clocks with a random first delay: nearest living entities (visible ones need line of sight within 16), nearest players (the first visible, attackable one: not creative, not spectating), nearest adult, and the hoglin sensor (adult piglins and hoglins in sight). The core sinks `LookAtTargetSink(45, 90)` and `MoveToTargetSink(150, 250)` (paths to the walk target, the `DefaultRandomPos` fallback towards it, `cant_reach_walk_target_since`, the stuck cooldown). IDLE: `StartAttacking` (players only, not while pacified or breeding), `SetEntityLookTargetSometimes(8, 30-60)` and the `RunOne` of stroll (0.4), walk to the look target (0.4, 3) and `DoNothing(30, 60)` weighted 2, 2, 1. FIGHT: the walk to the target (1.0) unless in reach and seen, `MeleeAttack` with a cooldown of 40 ticks (15 for a piglet), `StopAttackingIfTargetInvalid` (dead, gone, creative, spectating, or out of reach for more than 200 ticks). AVOID: walk away (1.3, within 15 blocks), the `RunOne`, the look, and an adult stops fleeing unless piglins outnumber hoglins. `updateActivity` picks FIGHT, AVOID or IDLE, erases the other activity's target and plays `angry` or `retreat` on a change; the ambient sound follows the activity too. The shared pieces (`Living`, `Track`, `MoveSink`, `Ticker`, `shuffled`, `targetable`, `within_melee_range`) are generic for b1's piglin brain.
  - `world/hoglins.rs`: hoglins think (zoglins still stand until b2.8): brain, navigation and move control before the look control and travel. A headbutt sets the 10-tick attack animation, plays `entity.hoglin.attack`, runs `onHitTarget` (an adult rallies every visible adult hoglin on the victim, or retreats with them from a piglin when piglins outnumber them) and `hurtAndThrowTarget`: 3 to 8 damage (a piglet 0.5); an adult's throw goes out with a player hit, or lands on a mob whose hit took (scaled by its knockback resistance: an iron golem is never thrown, a hoglin takes 0.4 of it).
  - Retaliation: player hits and sweeps, MW2 shots and blasts, arrows (by their owner) and mob hits note the attacker on the hoglin; at its next tick `wasHurtBy` forgets pacified and breeding, a piglet retreats from the nearer of its avoid target, attack target and the attacker for 100 to 400 ticks, an adult retaliates for 200 ticks (not against hoglins, not a piglin while retreating, not one more than 4 blocks further than its target, and only an attackable one in sight within 16) and rallies every visible adult that is not pacified.
  - `PlayerHitKind::Melee` gains `throw`. `minecraft_survival`: the hit list carries `impulse::MeleePush { throw, lift }`; `render_anim` lands the throw when the hurt took or a raised shield blocked it, and the iron golem's `lift` (until now dropped) when the hurt took, both scaled by `1 - knockback resistance` (netherite armour, 0.1 a piece) and launched like the dragon's pushes.
  - `walk_path::WalkTarget::Hoglin`: strolls weigh crimson nylium 10, a spot within 8 of the remembered repellent -1 (filled from b2.5).
- **Tests:** `world::hoglins`: `an_adult_hoglin_hits_for_three_to_eight_every_two_seconds_and_a_baby_for_half_a_heart_every_fifteen_ticks`, `a_hoglin_throw_lifts_up_to_half_a_block_per_tick_and_never_lifts_an_iron_golem`, `a_hoglin_ignores_a_creative_player_and_one_seventeen_blocks_away`, `hitting_one_hoglin_sets_every_visible_adult_on_the_attacker`, `a_hurt_piglet_runs_away_for_five_to_twenty_seconds`, `a_retaliation_target_is_forgotten_after_two_hundred_ticks`. `hoglin_brain`: `the_look_ticker_fires_after_its_interval`, `a_heavier_choice_comes_first_more_often`. `impulse` (survival): `a_shield_still_gets_thrown_by_an_adult`, `an_iron_golem_lifts_only_a_hit_that_lands`. `entities` (survival): `a_hoglin_throw_reaches_the_player_hit_list`.
- **Decisions:**
  - Hits land 41 and 16 ticks apart, not 40 and 15: the cooldown memory set with a 40-tick expiry is erased on the 41st brain tick after the hit (`MemorySlot.tick`), as in vanilla.
  - A hurt reaches the hoglin's brain at the start of its next tick (where the level's players and line of sight are at hand), not inside `hurtServer`; a player's hit is handled within the same server tick, as vanilla handles its packets before the entity ticks.
  - The throw on a player is drawn on the server with the hit; the host applies it only when the hurt landed or a shield blocked it. Vanilla draws it only then, so the hoglin's random advances slightly differently.
  - Track A has not fixed the iron golem's dropped `lift`; it is applied here in the same place (b1 adds `disable_shield_seconds` beside `throw`).
  - A hoglin's navigation does not float (no `FloatGoal`), as vanilla's brain hoglin.
  - Blaze and ghast fireballs that hurt a hoglin don't make it retaliate (they don't reach the hurt note); every other attacker does.
- **Test steps:** together with b2.5 and b2.6.

### b2.5. Minecraft hoglins shy away from warped fungus, nether portals and respawn anchors

After c638f7df.

- **Changed:**
  - `minecraft_terrain/src/server_mobs.rs`: `MobWorld` answers `World::block_in_tag` from the 26.3 block tags (`Registries::block_in_tag`); each tag's id is looked up once and kept in `MobTables.block_tags`. A tag the data lacks (the authored `blocks_motion`) falls back to `authored_block_in_tag`. Endermen's `#enderman_holdable` now comes from the data too.
  - Vendored `player`: `#hoglin_repellents` (warped fungus, potted warped fungus, nether portal, respawn anchor) among the authored tags, for worlds without the registries.
  - `hoglin_brain.rs`: the hoglin sensor finds the nearest `#hoglin_repellents` block in vanilla's order (`BlockPos.withinManhattan` over 8 across and 4 up or down, by Manhattan distance; the offsets are worked out once). IDLE and FIGHT start with `BecomePassiveIfMemoryPresent` (pacified for 200 ticks, the attack target erased); IDLE walks away from the repellent at 1.0 until 8 blocks off (`SetWalkTargetAwayFrom.pos`, rerun while walking), and an adult walks away from the nearest visible adult piglin at 0.4 (8 blocks, not while walking). Strolls weigh spots within 8 of the repellent at -1 (b2.4's walk target value), and near one the ambient and activity sounds are `entity.hoglin.retreat`.
- **Tests:** `world::hoglins`: `a_respawn_anchor_or_warped_fungus_within_eight_blocks_calms_a_hoglin_for_ten_seconds`, `a_calm_hoglin_walks_away_from_the_repellent_and_strolls_back_to_crimson_nylium`, `outnumbered_hoglins_retreat_from_a_piglin_they_hit` (a synthetic piglin). `hoglin_brain`: `the_repellent_scan_runs_by_manhattan_distance_within_eight_by_four`. `server` (with the game data): `a_portal_block_counts_as_a_repellent`, `enderman_holdable_still_matches_through_the_registry_tags`.
- **Decisions:**
  - The tag cache is a mutex-guarded map in `MobTables` rather than a fixed list of tags, so any tag a later mob asks for (b1's `#piglin_repellents`) comes from the data without new code.
  - The calm lasts 200 ticks from when it was set, renewed only once it ran out while a repellent is still remembered: after the repellent goes, a hoglin stays calm for what is left of those ten seconds, as in vanilla.
- **Test steps:** together with b2.6.

### b2.6. Minecraft hoglins breed with crimson fungus and their piglets grow up

After dbb52c23.

- **Changed:**
  - Vendored `hoglin.rs`: `Hoglin::interact_food` (`Animal.mobInteract` with `#hoglin_food`): an adult falls in love for 600 ticks unless pacified (`canFallInLove`), a piglet grows by a tenth of what it has left, the golden dandelion locks its age; a used item makes it persistent (`Hoglin.mobInteract`). A zoglin takes nothing.
  - `minecraft_terrain/src/mob_actions.rs`: the hoglin arm through the recipe book's `#hoglin_food`; `would_interact` follows it (it runs `interact` on a copy).
  - `hoglin_brain.rs`: `AnimalMakeLove(HOGLIN, 0.6, 2)` in IDLE and FIGHT: a hoglin in love picks the nearest visible hoglin in love, both remember each other as `breed_target`, look at and walk to each other (0.6, within 2), and the child is due 60 to 109 ticks after the start; it comes only if they are within 3 blocks at that tick, while both are still in love and in sight (at most 110 ticks). `BabyFollowAdult(5-16, 0.6)`: a piglet whose nearest visible adult hoglin is 5 to 17 blocks off walks to within 4. FIGHT ends with `EraseMemoryIf(isBreeding, ATTACK_TARGET)`.
  - `world/hoglins.rs`: the partner's brain takes the courting; a birth makes a persistent piglet (age -24000) where the first parent stands, both parents get age 6000 and no love, and one experience orb of 1 to 7 drops there (with `mob_drops` on).
- **Tests:** `mob_actions`: `two_fed_hoglins_make_a_persistent_piglet_within_110_ticks`, `a_pacified_hoglin_will_not_eat`, `feeding_a_piglet_ages_it_by_a_tenth`, and five hoglin and zoglin rows in `would_interact_agrees_with_interact`. `world::hoglins`: `a_piglet_follows_an_adult_from_six_to_sixteen_blocks`, `a_grown_piglet_hits_for_three_to_eight`.
- **Decisions:**
  - The breeding orb goes out through the same per-tick orb list as trade experience (`take_trade_experience`), whole, as vanilla's single `ExperienceOrb` of 1 to 7.
  - `ageBoundaryReached` needs no state of its own here: damage and experience are read from the age every time, so a grown piglet hits for 3 to 8 and gives 5 experience at once.
  - No eating sound: `Animal.playEatingSound` is silent for hoglins.
- **Test steps (b2.4 to b2.6; start in the Nether, `IW4L_MINECRAFT_DIMENSION=the_nether`, survival, `/difficulty normal`, on a 20 x 20 netherrack floor):**
  1. Put on the armour and use one hoglin egg in front of you, then back off about 10 blocks. It grunts angrily, charges and headbutts: each hit takes 1.5 to 4 hearts and throws you up and away (**1**, mid-air). It hits about every two seconds.
  2. Raise the shield as it hits: no damage, but you are still thrown.
  3. Use two more eggs close together and hit one with the sword: all three come for you (**2**).
  4. Place a respawn anchor next to them: within a second they snort, stop attacking and walk away from it. Break it: they stay calm for up to ten seconds, then charge again (**3**). Do the same with warped fungus on crimson nylium, and with a lit nether portal (obsidian and flint and steel).
  5. `/gamemode creative`: the hoglins stop attacking. Right-click two adults with crimson fungus: hearts, they walk to each other, and a piglet appears between them within about five seconds (**4**). Feed the piglet: it grows a little. Right-click a calmed hoglin next to the anchor with crimson fungus: it doesn't eat. `/gamemode survival`.
  6. The piglet nibbles you (half a heart, quickly) and never throws you. Hit it: it runs away for 5 to 20 seconds, then comes back.
  7. Stand 17 blocks or more from a hoglin, or in creative: it ignores you.
  8. Use an iron golem egg next to a hoglin: they fight; the golem is never thrown, the hoglin is lifted a little by the golem's blows.
  9. Wear four pieces of netherite armour (if you have them) and let a hoglin hit you: the throw is clearly smaller.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:hoglin_spawn_egg*16,minecraft:crimson_fungus*64,minecraft:warped_fungus*16,minecraft:crimson_nylium*16,minecraft:respawn_anchor*2,minecraft:diamond_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:netherite_helmet,minecraft:netherite_chestplate,minecraft:netherite_leggings,minecraft:netherite_boots,minecraft:iron_golem_spawn_egg*2,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:cooked_beef*32,minecraft:netherrack*64
```

### b2.7. Minecraft hoglins turn into zoglins outside the Nether

After 5ff86a48.

- **Changed:**
  - Vendored `world/src/level/time.rs`: `Level::piglins_zombify`, the `gameplay/piglins_zombify` attribute (true in the Overworld and the End, false in the Nether; no 26.3 biome changes it), next to `monsters_burn`.
  - `ServerSim::tick_mobs` hands it to its entity world each tick (`EntityWorld::set_piglins_zombify`), so every level decides for its own hoglins, and a level that doesn't tick doesn't move their clocks.
  - `world/hoglins.rs`: `Hoglin.customServerAiStep`'s clock after the brain: a hoglin that is not immune, has AI and stands where hoglins zombify is converting (the renderer's shake and the `retreat` sounds); `TimeInOverworld` counts up and, past 300 (the 301st tick, 15 seconds), the hoglin plays `entity.hoglin.converted_to_zombified` and becomes a zoglin; anywhere else the clock goes back to 0. `convert_hoglin_to_zoglin` (`ConversionType.SINGLE`): a new entity with its own id and UUID that keeps the position, yaw, motion, ground, fall, fire, hurt cooldown, baby, persistence, NoAI and effects, starts at 40 health and gets nausea for 200 ticks; the hoglin is gone.
- **Tests:** `world::hoglins`: `a_hoglin_in_the_overworld_becomes_a_zoglin_on_tick_301_and_one_in_the_nether_never_does`, `going_back_to_the_nether_resets_the_clock`, `an_immune_or_no_ai_hoglin_stays`, `a_piglet_becomes_a_baby_zoglin_and_keeps_its_persistence`. `server_mobs`: `the_clock_survives_a_save` (saved at 200, it converts on the 101st tick after loading). `server` (with the game data): `the_end_converts_hoglins_too`.
- **Decisions:**
  - The flag is read per level, not sampled at each hoglin's position (decision 5 of the brief): no 26.3 biome overrides it.
  - A custom name and a lead are not carried over: neither exists on mobs here yet.
- **Test steps:** together with b2.8.
