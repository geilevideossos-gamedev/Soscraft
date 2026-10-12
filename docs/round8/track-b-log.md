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

### b2.8. Minecraft zoglins attack every mob but creepers and other zoglins

After d5c603a5.

- **Changed:**
  - `hoglin_brain.rs`: the zoglin's `Brain` on the same pieces: the nearest-living and nearest-player sensors only; IDLE `StartAttacking` takes the nearest visible living entity that is not a zoglin or a creeper and is attackable (players not in creative or spectating; cows, villagers, golems, hoglins and the rest), then the look and the idle `RunOne`; FIGHT walks to the target at 1.0 and hits with `MeleeAttack(40)` (15 for a baby) until the target is invalid or out of reach for 200 ticks. `update_zoglin_activity`: FIGHT or IDLE; entering FIGHT plays `entity.zoglin.angry`. Strolls weigh spots by darkness (`Monster.getWalkTargetValue`), and its paths have no fire maluses.
  - `world/hoglins.rs`: zoglins think, steer and walk. Their ambient sound is `angry` while they have a target. `Zoglin.hurtServer`: a living attacker it can attack becomes its target for 200 ticks unless it is more than 4 blocks further than the current one. Its headbutt (`HoglinBase`) deals 3 to 8 and throws mobs (scaled by their knockback resistance, never an iron golem) and players like the hoglin's, without the hoglin's rallying.
- **Tests:** `world::hoglins`: `a_zoglin_goes_for_the_nearest_cow_villager_or_player_but_never_a_creeper_or_zoglin`, `a_zoglin_switches_to_a_closer_attacker_but_not_a_far_one`, `a_zoglin_kills_a_hoglin_which_fights_back`, `a_zoglin_throws_its_victims_but_not_an_iron_golem`.
- **Decisions:**
  - The zoglin shares the hoglin's brain type with its own activity table (decision 1 of the brief), so b1's piglin brain has one set of pieces to reuse.
  - A zoglin keeps no `TimeInOverworld` and never converts (the clock stays 0).
- **Test steps (b2.7 and b2.8; `/difficulty normal`, survival):**
  1. In the Overworld (`/execute in minecraft:overworld run tp @s ~ ~ ~` from the Nether), use a hoglin egg: it shakes and snorts for 15 seconds, then turns into a grey, rotting zoglin with a groan (**6** shaking, **7** zoglin). It has nausea (it may wobble).
  2. `/summon minecraft:hoglin ~ ~ ~ {IsImmuneToZombification:1b}`: it stays a hoglin and never shakes (**8**).
  3. Use a hoglin egg in the Overworld, wait about 10 seconds, save and quit, rejoin: it is still shaking and converts about 5 seconds later.
  4. Use a piglet (a hoglin egg until a small one hatches) in the Overworld: it becomes a baby zoglin.
  5. Use 4 cow eggs, 2 creeper eggs and a zoglin egg: the zoglin kills the cows (throwing them), ignores the creepers, then comes for you and throws you (**9**). Its kill drops rotten flesh; in lava it doesn't burn.
  6. Use an iron golem egg next to a zoglin: they fight; the golem is never thrown.
  7. Use a hoglin egg and a zoglin egg side by side in the Nether: the zoglin attacks the hoglin and the hoglin fights back.
  8. Hit a zoglin that is chasing a cow from close by: it turns on you. Shoot it from far away while it chases: it keeps after the cow.
  9. Go to the End (`/execute in minecraft:the_end run tp @s 0 70 0` on the platform) and use a hoglin egg: it converts there too. In the Nether no hoglin ever shakes.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:hoglin_spawn_egg*16,minecraft:zoglin_spawn_egg*4,minecraft:cow_spawn_egg*4,minecraft:creeper_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:lava_bucket,minecraft:diamond_sword,minecraft:bow,minecraft:arrow*32,minecraft:shield,minecraft:diamond_chestplate,minecraft:cooked_beef*32,minecraft:netherrack*64,minecraft:cobblestone*64
```

### b2.9. Docs: Minecraft hoglins and zoglins

After 2742b33d.

- **Changed:** `docs/MINECRAFT-SURVIVAL.md`: hoglin breeding in the right-click list; a "Hoglins and zoglins" part under the Nether (the brain, the charge and throw, retaliation, repellents, breeding, zombification, the zoglin, drops and spawning, the golem's throw on players); hoglins and zoglins leave N6 and the dormant list; their limitations (leads, eggs on adults, portals, Fire Aspect, fireballs, piglins). `docs/NETHER-END.md`: the hoglin and zoglin rows are present.
- **Decision:** as in entry 10, the vendored MinecraftOSS count in CLAUDE.md is left for the round's hand-over.
- **Test steps:** none (docs only).
- **Combined kit for b2.4 to b2.8** (24 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:hoglin_spawn_egg*16,minecraft:zoglin_spawn_egg*4,minecraft:crimson_fungus*64,minecraft:warped_fungus*16,minecraft:crimson_nylium*16,minecraft:respawn_anchor*2,minecraft:diamond_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:netherite_chestplate,minecraft:cow_spawn_egg*4,minecraft:creeper_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:lava_bucket,minecraft:bow,minecraft:arrow*32,minecraft:cooked_beef*32,minecraft:netherrack*64,minecraft:cobblestone*64
```

## Group b1-piglins-1

Brief `b1-piglins.md`, groups 1, 2 and 2b. Numbered b1.1 and on.

### b1.1. Minecraft piglin bartering rolls from the vanilla loot table

After 708a2da3.

- **Changed:** vendored `entities/src/loot.rs`: `EntityLootBook` also loads `loot_table/gameplay/` (as `minecraft:gameplay/<id>`), the enchantment definitions and the enchantment tags from the jar. `roll_barter()` rolls `minecraft:gameplay/piglin_bartering` on its own random sequence, and only a `minecraft:barter` table; `roll` still takes only `minecraft:entity` tables. The evaluator knows `minecraft:enchant_randomly` (`EnchantRandomlyFunction`): `options` as one id, a list or a `#tag`; with `only_compatible` (default true) an item other than a book keeps only enchantments whose `supported_items` hold it; one `nextInt(size)` draw picks the enchantment, then `Mth.nextInt(1, max_level)` its level (no draw when the maximum is 1); a book becomes `minecraft:enchanted_book` with `minecraft:stored_enchantments`, other items get `minecraft:enchantments`.
- **Tests:** `loot` (with the jar): `the_barter_table_weighs_469_and_every_entry_can_come_out` (20 000 rolls: all 19 entries come out, every count in range, obsidian near 40/469 and ender pearls near 10/469), `a_soul_speed_book_from_bartering_is_an_enchanted_book_of_level_one_to_three`, `bartered_iron_boots_carry_soul_speed`, `fire_resistance_potions_and_splash_potions_keep_their_contents`, `an_entity_roll_never_reads_the_barter_table`.
- **Decisions:**
  - `enchant_randomly` without `options` (the whole registry, in registry order) stays unsupported and gives no drop, because the registry order can't be reproduced from the data files.
  - `include_additional_cost_component` is ignored: it only matters with the trade parameter, which a barter never has.
- **Test steps:** none yet; bartering comes with b1 group 5.

### b1.2. Minecraft piglins and piglin brutes walk the Nether

After 1ebb81a1.

- **Changed:**
  - New vendored `entities/src/brain_behaviors.rs`: the brain pieces the hoglin brain and the piglin brain share, moved out of `hoglin_brain.rs` unchanged (`Living`, `Track`, `WalkGoal`, `Ticker`, `shuffled`, `nearer`, `within_melee_range`, the Manhattan block scan, `MoveSink`). `MoveSink` now takes the walk target, the `cant_reach_walk_target_since` slot and a `Walker` (body, others, world, time, random, navigation, profile, fluid) instead of the hoglin's memories and context, and `targetable_within` takes the follow range (the hoglin's `targetable` keeps 16). `hoglin_brain.rs` re-exports them, so the hoglin behaves the same.
  - New vendored `entities/src/piglin.rs`: `PiglinKind { Piglin, Brute }` and `Piglin` (body, health, baby, armour, both hands and the 8-slot inventory as item stacks, the six drop chances, `CannotHunt`, `IsImmuneToZombification`, `TimeInOverworld`, persistence, `CanPickUpLoot`, left-handedness, the charging, dancing, aggressive and converting flags), the §3.1 attributes (16 or 50 health, speed 0.35 and +0.2 for a baby, 5 or 7 attack, follow range 16 or 12, 0.6 x 1.95 with eyes at 1.79, a baby 0.49 x 0.98 with eyes at 0.78), worn armour points, and `arm_pose` in vanilla's order (dancing, a loved offhand item, aggressive with a melee weapon, charging, a charged crossbow, default).
  - New vendored `entities/src/piglin_brain/`: `PiglinBrain` with the `Activity` set (idle, admire item, fight, celebrate, avoid, ride), `update_activity` (the first valid of admire, fight, avoid, celebrate and idle; the left activity's memory erased; the activity sound on a change; a brute only fights or idles and says `entity.piglin_brute.angry` on entering a fight; dancing erased without a celebrate spot), `activity_sound` (§3.4), the saved memories (`hunted_recently`, `angry_at`, `universal_anger`, `admiring_item`, `admiring_disabled`, `home`) and the slots later groups fill. Sensors on 20-tick clocks: `NearestLivingEntitySensor` and `PlayerSensor` with the follow range. CORE: `LookAtTargetSink(45, 90)`, `MoveToTargetSink` and `InteractWithDoor`. IDLE: the look `RunOne` (player, piglin, anything within 8, for a brute also brutes, or `DoNothing(30, 60)`), the movement `RunOne` (piglin: stroll at 0.6, `InteractWith(piglin, 8)`, walk to the look target unless a player holds a loved item, `DoNothing`, weighted 2, 2, 2, 1; brute: stroll, `InteractWith` piglin and brute, `StrollToPoi(home, 0.6, 2, 100)`, `StrollAroundPoi(home, 0.6, 5)` and `DoNothing`, weighted 2, 2, 2, 2, 2, 1) and `SetLookAndInteract(player, 4)`.
  - New vendored `entities/src/world/piglins.rs`: `PiglinEntity`, `spawn_piglin`, `piglins`, `piglin_mut`, `set_dimension`. The tick: base tick, fire, suffocation, effects, the ambient sound only while idle (a piglin's is its activity sound, a brute's `entity.piglin_brute.ambient`), the brain, ground navigation that opens doors with the fire mali, the doors it opened or shut (with their sound), travel, steps (`entity.piglin.step` or `entity.piglin_brute.step` at 0.15), fall damage and pushing. Hurts go through its worn armour (except magic and other armour-bypassing harm). Death: no loot pools (the 26.3 tables are empty), each equipment slot by its drop chance (guaranteed ones always, others only for a player's kill and with random wear), a piglin's whole inventory, and experience: 5 for a piglin, 20 plus 1 to 3 per item still worn for a brute.
  - `world.rs` and `world/{combat,hazards,pushing,riding,witches}.rs`: `EntityKey::Piglin`, `MobHit::Piglin`, storage, the dimension name, `entity_type`, the tick order, the candidate list with the baby flag, and every per-kind list. `villagers::set_door_open` and `hoglins::hoglin_living` are shared with piglins.
  - `minecraft_terrain`: both ids are simulated, so bastion piglins and brutes no longer wait dormant. New `piglin_save.rs` loads and saves `IsBaby`, `CannotHunt`, `Inventory`, `IsImmuneToZombification`, `TimeInOverworld`, `CanPickUpLoot`, `LeftHanded`, `equipment` (whole stacks, components kept), `drop_chances` and `Brain.memories` with their `ttl`. `ServerSim` names its dimension to the entity world. Client: the mob list, shadows (0.5, half for a baby), the three textures in the atlas, and `piglin_render::append_piglins` with humanoid arms (the item arm lowered, `bobModelPart`), the admiring pose (head down, the off arm raised), adult armour, and both hands' items (`cow_render::left_hand_pose` beside `right_hand_pose`). The brute uses the adult mesh with its own texture.
  - `minecraft_survival`: hit boxes, meshes and held items; MW2 hit key 21.
- **Tests:** `piglin`: `a_piglin_baby_is_049_by_098_and_faster`, `brutes_have_fifty_health_and_twelve_follow_range`, `the_arm_pose_goes_dancing_admiring_melee_charging_then_holding`, `golden_armour_adds_its_defense`. `world::piglins`: `piglins_and_brutes_are_monsters_of_their_own_types`, `idle_piglins_stroll_about`, `a_brute_strolls_back_to_its_home`, `a_piglin_dies_dropping_its_inventory_and_lucky_gear`, `a_brute_earns_twenty_and_more_per_held_item`, `golden_armour_softens_a_blow`. `server_mobs`: `piglins_and_brutes_load_live_and_save_every_field`. `terrain` (with the game data): `a_bastion_s_piglins_wake_up`. `piglin_render`: `a_piglin_and_a_brute_use_their_textures_and_the_piglin_mesh`, `an_admiring_piglin_holds_its_item_in_the_left_hand`. `entities` (survival): `piglin_keys_round_trip`.
- **Decisions:**
  - Armour and hands hold whole item stacks rather than ids, so enchantments and wear survive a save and a drop.
  - Other mobs in the game ignore worn armour; piglins count theirs (golden armour softens hits), as vanilla does.
  - `InteractWithDoor` leaves out `areOtherMobsComingThroughDoor`: a piglin shuts a door behind it even when another piglin is about to walk through. Doors are rare in the Nether, and the other mobs' paths are not at hand in the piglin's tick.
  - Brain durations (`DoNothing`, `LookAtTargetSink`) draw from the level random as vanilla's do; the `RunOne` shuffles and stroll spots use the piglin's own random, like the hoglin brain.
  - The `home` memory keeps its dimension; a brute only strolls home in that dimension, as `StrollToPoi` checks.
  - Death names come from the 26.3 language file at run time, so `death.rs` needs no piglin rows yet; its fixture rows come with group 3's death-message tests, when piglins first hurt players.
  - Golden spears stay plain held items (spears have no use yet).
- **Test steps (start in the Nether, `IW4L_MINECRAFT_DIMENSION=the_nether`, survival, `/difficulty normal`, on a netherrack floor):**
  1. Use a piglin egg and a brute egg. They appear with the vanilla piglin and brute skins, wander, now and then stand still, glance at each other and at you, and grunt (the brute lower). They never attack yet (**1**).
  2. Stand within 4 blocks of a piglin: it turns its head to you.
  3. `/summon minecraft:piglin ~ ~ ~ {IsBaby:1b}`: a small piglin with the baby skin that runs faster.
  4. `/summon minecraft:piglin ~ ~ ~ {equipment:{mainhand:{id:"minecraft:golden_sword",count:1},offhand:{id:"minecraft:gold_ingot",count:1},head:{id:"minecraft:golden_helmet",count:1},chest:{id:"minecraft:golden_chestplate",count:1}}}`: it wears the golden helmet and chestplate, holds the sword in its right hand, and looks down at the ingot raised in its left hand (**2**).
  5. Hit a piglin with the sword: hurt sound and red flash; one wearing gold takes less. Kill the one from step 4: death sound and experience; the sword, ingot and armour only sometimes drop, with wear.
  6. Save and quit, rejoin: the piglins are where they were, with the same gear.
  7. `/locate structure minecraft:bastion_remnant`, `/tp` there: the bastion's piglins and brutes are drawn and walk about instead of being invisible (**3**).
  8. Build a 2-high wall with an oak door in it and stand behind it near a piglin: when it wanders through, it opens the door and shuts it behind it.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*8,minecraft:piglin_brute_spawn_egg*4,minecraft:diamond_sword,minecraft:oak_door*2,minecraft:netherrack*64,minecraft:cooked_beef*16
```

### b1.2b. Minecraft piglins spawn in nether wastes, crimson forests and bastions

After 52536843.

- **Changed:**
  - Vendored `world/src/natural_spawner.rs`: `type_info` knows `piglin` (0.6 x 1.95, on the ground, `MOTION_BLOCKING_NO_LEAVES`, a monster) with the new `Rules::Piglin`, so the nether wastes (weight 15, groups of 4) and crimson forest (weight 5, 3 to 4) entries spawn. `piglin_brute` is known too (no spawn rules: brutes never spawn naturally), so its egg and `/summon` finalize it. `CreatureSpawns` keeps its dimension's name.
  - Vendored `world/src/natural_spawner/spawning.rs`: `Rules::Piglin` (`checkPiglinSpawnRules`: anything but a nether wart block below). The piglin's `finalizeSpawn` (`piglin_spawn`): a baby one time in five (level random); an adult's weapon from its own random (`createSpawnWeapon`: a crossbow half the time, else a golden spear one time in ten, else a golden sword); `hunted_recently` for 600 to 2400 ticks; each golden armour piece at 10% for an adult; the enchantment rolls of the filled slots; then `Mob.finalizeSpawn`. The brute's: `home` at its block in this dimension and a golden axe, then `Mob.finalizeSpawn`; its follow range base is 12. `/summon` keeps `CanPickUpLoot` on for piglins and brutes (`AbstractPiglin.readAdditionalSaveData` reads it as true by default), where other mobs get it off.
  - Vendored `core/src/entity_data.rs`: `set_brain_memory` (a saved brain memory with an optional `ttl`, other memories kept) and `finalize_piglin_brute`, shared by natural spawning, eggs and structures.
  - Vendored `generator`: bastion template brutes get their golden axe and `home` (`Library.dimension`, set with the dimension type); `finalize_piglin` writes its hunt memory through `set_brain_memory`.
- **Tests:** `spawning` (vendored world): `a_piglin_spawn_rolls_a_baby_one_in_five_and_a_crossbow_for_half_the_adults` (10 000 rolls: babies about 20%; of the adults crossbows about 50%, spears about 5%, swords about 45%, each golden piece about 10%), `piglins_do_not_spawn_on_nether_wart_blocks`. `entity_data` (vendored core): `a_bastion_brute_gets_a_golden_axe_and_its_home`. `server` (with the game data): `piglin_eggs_hatch_armed_and_brute_eggs_with_an_axe_and_a_home`. `terrain`: `a_bastion_s_piglins_wake_up` also checks that every bastion brute holds a golden axe and has its home where it stands.
- **Decisions:**
  - The brute's finalize lives in the vendored core, next to `finalize_mob`, because both the generator (bastions) and natural spawning (eggs, `/summon`) need it and the generator cannot use the world crate.
  - Brutes generated in bastions before this commit keep what they were saved with: no axe, no home (as the brief says, vanilla never had such brutes; a bastion not yet visited shows the new ones).
  - The brute's spawn rules are `Rules::Other` (not spawned): no 26.3 biome lists brutes.
- **Test steps (Nether, survival, `/difficulty normal`):**
  1. Use 10 piglin eggs. Roughly two are babies (small, no weapon); the adults hold crossbows or golden swords (rarely a golden spear), and now and then one wears a golden helmet, chestplate, leggings or boots (**4**).
  2. Use a brute egg: it holds a golden axe. Walk 20 blocks away and wait: it keeps strolling back to where it hatched.
  3. Fly over a nether wastes or crimson forest area far from where you were: groups of piglins appear, never standing on nether wart blocks.
  4. `/locate structure minecraft:bastion_remnant` and `/tp` to one you have not visited: its brutes hold golden axes and stay near where they stood (**5**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*16,minecraft:piglin_brute_spawn_egg*4,minecraft:diamond_sword,minecraft:netherrack*64,minecraft:cooked_beef*16
```
- **Combined kit for b1.2 and b1.2b:**

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*16,minecraft:piglin_brute_spawn_egg*4,minecraft:diamond_sword,minecraft:oak_door*2,minecraft:netherrack*64,minecraft:cooked_beef*16
```

## Group b1-piglins-2

Brief `b1-piglins.md`, groups 3, 4 and 5.

### b1.3. Minecraft piglins attack players who wear no gold

After 6920d28f.

- **Changed:**
  - Vendored `entities/src/controls.rs`: `PlayerControls` carries the worn armour (feet, legs, chest, head) next to both hands, and `wears`. `minecraft_survival::entities` fills it from inventory slots 36 to 39 for every player it sends, so each player's gear reaches the mobs of their level by player id (no second per-player list, `PlayerCandidate` unchanged).
  - `EntityWorld`: `set_universal_anger` and `set_forgive_dead_players` (from `ServerSim::apply_game_rules`), `player_wears_gold` (`#minecraft:piglin_safe_armor` on any worn piece) and `player_holds_loved` (`#minecraft:piglin_loved` in either hand), both through the entity world's recipe book.
  - Vendored `piglin_brain/sensors.rs`: `PiglinSpecificSensor` (the nearest visible nemesis, zombified piglin or zoglin, targetable player wearing no gold and player holding a loved item, the visible adult piglins and their count, the nearby adult piglins seen or not) and `PiglinBruteSpecificSensor` (nemesis and nearby adult piglins), on their own 20-tick clock. Hoglin and repellent memories come with groups 8 and 4.
  - New vendored `piglin_brain/fight.rs`: `findNearestValidAttackTarget` for both kinds (piglin: none near a zombified piglin, then `angry_at` while attackable ignoring sight, then the attackable player under universal anger, the nemesis, the ungilded player; brute: `angry_at`, then the attackable player or the nemesis within 12), `StartAttacking` (adults only) in IDLE and CELEBRATE, FIGHT (`StopAttackingIfTargetInvalid` with the nearest-valid check and the 200-tick give-up, `BackUpIfTooClose(5, 0.75)` with a crossbow, the walk to the target at 1.0 unless in reach and seen, `MeleeAttack(20)` without a crossbow, `CrossbowAttack`: 25 ticks of charging, a delay of 20 to 39, then the shot, timeout 1200), CELEBRATE (`GoToTargetLocation` 2 at 1.0 or 4 at 0.6 when dancing, the look/stroll/`DoNothing(10, 20)` `RunOne`), AVOID's walk away to 12 at 1.0, CORE `StartCelebratingIfTargetDead(300)` (dancing one time in ten after a hoglin kill, from `RandomSource.createThreadLocalInstance(gameTime)`) and `StopBeingAngryIfTargetDead`, both honouring `forgive_dead_players`. `update_activity` now erases every other activity's memory on a change, as `Brain.setActiveActivity` does.
  - Vendored `world/piglins.rs`: hurts noted on a piglin (player hits and sweeps, MW2 bullets and blasts, arrows by their owner, mob hits; `note_hoglin_hurt` now hands non-hoglins to `note_piglin_hurt`) run `wasHurtBy` at the piglin's next tick: piglin attackers are ignored, celebrating, dancing and admiring stop, a player sets `admiring_disabled` for 400, an avoid target of another type is dropped, a baby flees its attacker for 100 ticks and broadcasts the anger, an adult runs `maybeRetaliate` (not while avoiding, attackable ignoring sight, not 4 blocks further than its current target) with `setAngerTarget` (`angry_at` = the attacker's UUID for 600, `universal_anger` for players when the rule is on) and `broadcastAngerTarget` to every nearby adult piglin and brute (`setAngerTargetIfCloserThanCurrent`), or the universal variants. Brutes retaliate against anything but piglins and brutes. Melee hits deal the attack damage plus the main hand's attack damage modifier (5 + 3 with a golden sword, 7 + 6 with a golden axe, from the item catalog), swing the arm (`swings` for the client) and carry the weapon's `disable_blocking_for_seconds`. The crossbow is used like `LivingEntity.startUsingItem`: `item.crossbow.loading_start` and `loading_middle` at 0.2 and 0.5 of the charge, `loading_end` on release, the crossbow gets `minecraft:charged_projectiles`, and the shot is a mob arrow from the eyes minus 0.1 at power 1.6 and uncertainty 14 - 4 x difficulty, base damage 2 (no crit), with `item.crossbow.shoot` and the hit sound `item.crossbow.hit` (`ArrowEntity.hit_sound`). Brutes play `entity.piglin_brute.angry` with chance 0.0125 per tick while fighting.
  - `PlayerHitKind::Melee` gains `disable_shield_seconds` (0 for every other mob). `minecraft_survival`: `MeleePush.disable_shield_ticks` (`offhand::disable_ticks`, seconds x 20 rounded); `render_anim`: a blocked hit with it puts the shield on cooldown for that long, lowers it and plays `item.shield.break`; a shield on cooldown can't be raised.
  - Vendored `monster_ai.rs`: wither skeletons' `NearestAttackableTargetGoal<AbstractPiglin>` slot is `Prey::Piglin`; `is_enemy` knows piglins and brutes, so iron golems go for them.
  - `piglin_render.rs`: the weapon arm raised to -1.8 while fighting with a melee weapon, `swingWeaponDown` while swinging, `animateCrossbowHold` and `animateCrossbowCharge` (by the ticks the crossbow has been used); `client_mobs` sends the piglins' swings.
  - Vendored `brain_behaviors.rs`: `walk_away` (`SetWalkTargetAwayFrom`) moved out of the hoglin brain, which now calls it unchanged.
- **Tests:** `piglin_brain::fight`: `the_nearest_valid_target_order_is_zombified_angry_universal_nemesis_then_ungilded`, `only_a_hoglin_kill_can_end_in_a_dance`. `world::piglins`: `one_golden_piece_makes_a_player_safe`, `the_gear_of_each_player_reaches_the_piglins`, `a_creative_or_spectator_player_is_never_a_target`, `a_baby_never_starts_attacking_and_flees_its_attacker_for_100_ticks`, `a_hit_angers_the_piglin_and_its_adult_neighbours_for_600_ticks`, `a_brute_attacks_a_player_in_golden_armour_and_hits_for_13_on_normal` (also checks the 5 seconds of shield disable), `a_crossbow_piglin_charges_25_ticks_waits_20_to_39_and_shoots_a_non_critical_arrow`, `a_crossbow_piglin_backs_off_inside_five_blocks`, `anger_ends_after_its_target_dies_with_forgive_dead_players`, `wither_skeletons_and_piglins_fight`, `iron_golems_attack_piglins`. `piglin_render`: `a_fighting_piglin_raises_its_sword_and_a_crossbow_piglin_pulls_the_string`. `minecraft_survival`: `offhand::an_axe_takes_the_shield_away_for_five_seconds`, `entities::a_brute_s_axe_hit_carries_a_hundred_ticks_of_shield_cooldown`, `entities::the_worn_armour_reaches_the_mobs_feet_first`, `death::a_piglin_shoots_and_a_brute_slays_by_name`.
- **Decisions:**
  - The gear test is an entity-world test with two players' controls (`the_gear_of_each_player_reaches_the_piglins`) rather than a `server.rs` one: `TickInput.controls` already reaches `EntityWorld::set_player_controls` unchanged, so the world is where the per-player check lives.
  - `isEntityAttackableIgnoringLineOfSight` and the sensors use the piglin's own follow range (16, or 12 for a brute): vanilla's shared static conditions take the range of whichever mob's sensor last ticked.
  - A hurt reaches the piglin's brain at the start of its next tick, as for hoglins.
  - A crossbow loses no durability per shot and mob weapons carry no enchantment damage yet (Sharpness on a spawned sword); spears still melee like swords.
  - The crossbow's loading sounds are tracked per piglin (vanilla keeps them on the shared `CrossbowItem`).
  - The dancing pose is drawn with group 8, when hoglin kills (the only dances) arrive; until then a dancing piglin is drawn standing.
- **Test steps (start in the Nether, `IW4L_MINECRAFT_DIMENSION=the_nether`, survival, `/difficulty normal`, on a netherrack floor, no gold on):**
  1. Use 2 piglin eggs in front of you, then back off to about 10 blocks. They turn on you: sword piglins run in with the sword raised and hit (about 4 hearts on normal); crossbow piglins step back when you come within 5 blocks, you hear the crossbow load, they hold it aimed, then shoot (**1**).
  2. Kill them. Put on only the golden boots and use 4 more eggs: they wander and ignore you (**2**).
  3. Hit one adult: it and the adults near it (also a brute) come for you; one 20 blocks away doesn't (**3**). Back off for 30 seconds: they calm down. Die to one with the default `forgive_dead_players`: after respawning they leave you alone.
  4. Hit a baby piglin: it runs away for 5 seconds and the adults near it attack you.
  5. Use 2 brute eggs: they attack even with the gold boots on. Take off all armour: one axe hit takes about 6.5 hearts. Raise the shield as one swings: the shield cracks (break sound), drops and can't be raised for 5 seconds (cooldown shown on the slot) (**4**).
  6. Use a wither skeleton egg near piglins: they fight each other. Use an iron golem egg near piglins: it attacks them (**5**).
  7. `/gamemode creative`: no piglin or brute attacks you; `/gamemode survival`.
  8. Die to a piglin's arrow and to a brute: the death messages read "was shot by Piglin" and "was slain by Piglin Brute".
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*16,minecraft:piglin_brute_spawn_egg*4,minecraft:wither_skeleton_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:golden_boots,minecraft:diamond_sword,minecraft:shield,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:netherrack*64
```

### b1.4. Minecraft piglins avoid soul fire, zombified piglins and wither skeletons

After 0967e73a.

- **Changed:**
  - Vendored `piglin_brain/sensors.rs`: the piglin sensor finds `NEAREST_REPELLENT`, the first block of `#minecraft:piglin_repellents` in vanilla's Manhattan order within 8 across and 4 up or down (b2's shared scan), where a soul campfire counts only while lit; it also counts the visible adult hoglins (`VISIBLE_ADULT_HOGLIN_COUNT`) for `wantsToStopFleeing`.
  - Vendored `piglin_brain/fight.rs` and `mod.rs`: CORE `babyAvoidNemesis` and `avoidZombified` (`CopyMemoryWithExpiry` into `AVOID_TARGET` for 5 to 7 seconds from the level random; the zombified one only within 6 blocks), IDLE and CELEBRATE `avoidRepellent` (`SetWalkTargetAwayFrom.pos` at 1.0 until 8 blocks off), AVOID's `EraseMemoryIf(wantsToStopFleeing)` (stop when the avoided hoglins no longer outnumber, or the avoided zombified mob is no longer the nearest visible one; a player or nemesis runs out its time), and FIGHT's `EraseMemoryIf(isNearZombified, ATTACK_TARGET)`. `isNearRepellent` is now "remembers a repellent", as vanilla's, so a piglin near soul fire says `entity.piglin.retreat`.
  - Vendored `player`: `#piglin_repellents` among the authored block tags, for worlds without the registries; the server answers it from the 26.3 tags (b2.5's `MobWorld::block_in_tag`).
- **Tests:** `world::piglins`: `a_piglin_walks_away_from_a_soul_torch_within_eight_and_ignores_a_normal_torch` (also the retreat sound near a soul lantern), `an_unlit_soul_campfire_repels_nobody`, `a_piglin_runs_from_a_zombified_piglin_within_six_and_drops_its_target`, `a_baby_runs_from_a_wither_skeleton_and_an_adult_attacks_it`. `server` (with the game data): `soul_lights_repel_piglins_through_the_registry_tags`.
- **Decisions:**
  - An avoided mob that is gone from the level ends the flight at once; vanilla keeps fleeing from its last spot until the memory runs out.
  - Hunting, the hoglin retreat and the baby hoglin memories stay with group 8; only the adult hoglin count is sensed now, because `wantsToStopFleeing` needs it.
- **Test steps (Nether, survival, `/difficulty normal`, golden boots on so the piglins stay calm):**
  1. Use 3 piglin eggs, then place a soul torch next to them: they walk away from it until about 8 blocks off and grunt the retreat sound (**6**). A normal torch does nothing. A soul campfire repels them only while lit (put it out with a shovel).
  2. Use a zombified piglin egg next to them: the ones within 6 blocks run from it for 5 to 7 seconds, then come back (**7**).
  3. Take the boots off so they attack you, then use a zombified piglin egg right next to them: they stop attacking and run.
  4. Use a wither skeleton egg near piglins and a baby piglin (`/summon minecraft:piglin ~ ~ ~ {IsBaby:1b}`): the adults attack it, the baby runs away (**8**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*16,minecraft:zombified_piglin_spawn_egg*4,minecraft:wither_skeleton_spawn_egg*2,minecraft:soul_torch*16,minecraft:torch*16,minecraft:soul_campfire*2,minecraft:iron_shovel,minecraft:golden_boots,minecraft:diamond_sword,minecraft:cooked_beef*32,minecraft:netherrack*64
```

### b1.5. Minecraft piglins admire gold and barter

After f9e8e09d.

- **Changed:**
  - New vendored `piglin_brain/admire.rs`: `NearestItemSensor` on its own 20-tick clock (the item list is gathered by the entity world only on the tick the clock fires: items within 32 x 16 x 32 by distance, ones the piglin wants, closer than 32 and in sight), CORE `StopHoldingItemIfNoLongerAdmiring` and `StartAdmiringItemIfSeen(119)` (a `#piglin_loved` item), the ADMIRE_ITEM activity (`GoToWantedItem` at 1.0 within 9 unless the offhand holds a loved item, `StopAdmiringIfItemTooFarAway(9)`, `StopAdmiringIfTiredOfTryingToReachItem(200, 200)`), and the jealous look (`SetEntityLookTarget` at a player holding a loved item within 14) in IDLE and CELEBRATE. New memories: the wanted item, the time spent reaching it, `disable_walk_to_admire_item`, `ate_recently`.
  - Vendored `world/piglins.rs`: `wantsToPickup` (babies ignore `#ignored_by_piglin_babies`, nothing from `#piglin_repellents`, nothing while admiring is disabled during a fight, a gold ingot unless the offhand holds a loved item, nuggets and food only with room in the inventory and food only when not `ate_recently`, other loved items with room, anything else only if it replaces the current gear), with `mob_griefing` and `CanPickUpLoot`; brutes want only a golden axe. `Mob.aiStep`'s pick-up (box grown by 1, 0, 1, no pickup delay): nuggets are taken whole, anything else one at a time; a loved item goes to the offhand (dropping what was there) and is admired for 119 ticks; food is eaten (`ate_recently` 200); the rest is equipped (`equipItemIfPossible`: armour by defence then toughness, weapons by the preferred weapon tag then attack damage, then enchantments, wear and a custom name; the old piece drops by its drop chance) or stored in the 8-slot inventory, any overflow thrown away. `stopHoldingOffHandItem`: an adult's gold ingot after admiring is bartered, any other loved item is equipped or stored; a baby equips it, or keeps it in the main hand and stores or throws what it held there. Items held in a hand drop for sure and make the piglin persistent (but a plain gold ingot in the offhand).
  - Barter: the entity world queues `BarterThrow` (from the eyes minus 0.3, velocity 0.3 toward the nearest visible player plus one block up, else a `LandRandomPos(4, 2)` spot, else straight up); `ServerSim::tick_mobs` rolls `gameplay/piglin_bartering` once per throw (b1.1's `roll_barter`) and spawns each stack with that velocity and a pickup delay of 10.
  - `minecraft_terrain::mob_actions`: right-clicking a piglin with a gold ingot (`canAdmire`: an adult piglin not admiring and not barred from it) uses one ingot (none in creative), puts it in the offhand (what it held drops), starts the 119 ticks of admiring and stops it walking; babies, brutes, an admiring piglin and other items pass. `would_interact` follows.
- **Tests:** `world::piglins`: `a_dropped_gold_ingot_is_admired_for_119_ticks_then_bartered_toward_the_player`, `a_baby_keeps_the_gold_it_admires_and_never_barters`, `a_piglin_eats_a_porkchop_and_takes_the_next_after_200_ticks`, `gold_nuggets_go_whole_into_the_inventory_and_drop_on_death`, `a_hit_while_admiring_ends_it_without_a_barter_and_disables_admiring_for_400_ticks`, `babies_ignore_leather` (also the gear and loved-item rules), `a_piglin_gives_up_on_an_unreachable_ingot_after_200_ticks`, `no_mob_griefing_no_pickup`. `mob_actions`: `a_right_clicked_ingot_is_used_and_admired`, `a_brute_ignores_a_gold_ingot`, and seven piglin rows in `would_interact_agrees_with_interact`. `server` (with the game data): `a_barter_throws_one_roll_of_the_table_at_the_player`.
- **Decisions:**
  - A piglin hit while admiring a gold ingot loses it, as vanilla's `stopHoldingOffHandItem(false)` does (an adult's ingot that is not bartered is neither equipped nor stored). The brief's "keeps the ingot unbartered" test is named after what vanilla does.
  - The item sensor's list is gathered by the entity world on the tick the piglin's sensor clock fires, because only the world knows the item tags, the inventory and the game rules.
  - The barter is rolled after the mob tick on the server, where the loot book lives; the throw keeps the moment's direction.
  - Brutes use the piglin's replacement rules when picking up a golden axe (vanilla brutes use `Mob`'s); only golden axes are ever picked up, so the result is the same unless two golden axes differ in wear or enchantments.
- **Test steps (Nether, survival, `/difficulty normal`, golden boots on, about 10 piglin eggs used on a netherrack floor):**
  1. Hold a gold ingot near them: they turn to look at you and grunt jealously.
  2. Drop one gold ingot (Q) near a piglin: it walks over, picks it up, holds it up in its left hand looking at it (admiring sound) for about 6 seconds, then throws an item at you (**9**).
  3. Right-click a piglin with a gold ingot: one ingot is used and the same happens. Barter about 20 times (**10**, the inventory): obsidian, crying obsidian, quartz, string, gravel, blackstone, soul sand, nether bricks, spectral arrows, leather, fire charges, iron nuggets, ender pearls, dried ghasts, water or fire resistance potions, rarely a soul speed book or soul speed iron boots. In creative the ingot isn't used up.
  4. Right-click a baby piglin or a brute with an ingot: nothing happens. Right-click a piglin that is already admiring: nothing.
  5. Drop a porkchop next to a piglin: it eats it (gone, nothing back); a second porkchop is left for about 10 seconds. Drop a golden carrot: one picks it up, admires it and keeps it. Drop 20 gold nuggets: one piglin takes the whole stack; kill it: the 20 nuggets drop. Drop leather next to a baby: it ignores it.
  6. Hit a piglin while it admires an ingot: it stops at once, throws nothing, and won't take gold for 20 seconds.
  7. `/gamerule mob_griefing false`: dropped gold is ignored; `/gamerule mob_griefing true`.
  8. Put an ingot on top of a 3-block pillar next to a piglin: it tries to reach it for about 10 seconds, then gives up for 10 seconds.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*16,minecraft:piglin_brute_spawn_egg*2,minecraft:gold_ingot*64,minecraft:gold_ingot*64,minecraft:gold_nugget*32,minecraft:golden_carrot*4,minecraft:porkchop*8,minecraft:leather*8,minecraft:golden_boots,minecraft:diamond_sword,minecraft:netherrack*64,minecraft:cooked_beef*32
```

- **Combined kit for b1.3 to b1.5** (21 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*16,minecraft:piglin_brute_spawn_egg*4,minecraft:zombified_piglin_spawn_egg*2,minecraft:wither_skeleton_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:gold_ingot*64,minecraft:gold_ingot*64,minecraft:gold_nugget*32,minecraft:golden_carrot*4,minecraft:porkchop*8,minecraft:leather*8,minecraft:golden_boots,minecraft:diamond_sword,minecraft:shield,minecraft:soul_torch*16,minecraft:torch*16,minecraft:soul_campfire*2,minecraft:iron_shovel,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:netherrack*64
```

## Group b1-piglins-3

Brief `b1-piglins.md`, groups 6, 7, 8 and 9.

### b1.6. Minecraft piglins guard gold blocks and chests

After 6c7ceb71.

- **Changed:**
  - Vendored `world/piglins.rs`: `EntityWorld::anger_nearby_piglins(player, feet, require_sight)` (`PiglinAi.angerNearbyPiglins`). It is queued and resolved at the start of the next entity tick, when the players are known: every `Piglin` (not brutes) whose box meets the player's box (0.6 x 1.8 at the feet) grown by 16, that is in IDLE and, when sight is required, has the player among its visible living entities (`BehaviorUtils.canSee`), gets `setAngerTarget(player)` (600 ticks, only while the player is attackable ignoring sight, so creative and spectating players anger nobody), or under `universal_anger` the nearest visible attackable player it knows (else this one). There is no broadcast.
  - `minecraft_terrain::server`: `Command::AngerPiglins { player, feet, sight }` and `ServerHandle::anger_piglins`.
  - `minecraft_survival::entities`: `anger_piglins(feet, sight)` for the local player, `guard_mined` (any hand-broken block, not a blasted one, in `#minecraft:guarded_by_piglins` from the 26.3 block tags angers without sight, as `Block.playerWillDestroy` does) and `guarded_by_piglins`.
  - `render_anim::minecraft_world`: one call after `entities.mined` (`guard_mined`), and one when a chest, trapped chest, copper chest, barrel, shulker box or ender chest opens (`anger_piglins(feet, true)`, as the containers' `useWithoutItem` does on each open).
- **Tests:** `world::piglins`: `mining_a_gold_block_angers_idle_piglins_within_16_even_unseen` (one behind a wall joins, one 20 blocks away does not, and the angry one then attacks), `opening_a_chest_angers_only_the_piglins_that_see_it`, `brutes_and_busy_piglins_are_not_angered_by_a_chest` (an admiring piglin and a brute stay unangered), `universal_anger_turns_a_chest_s_piglins_on_the_nearest_player`. `minecraft_survival::entities` (with the game data): `only_guarded_blocks_send_anger` (gold block, raw gold block, gilded blackstone, the three gold ores, chests, copper chest, barrel, ender chest and shulker boxes yes, also not when blasted; stone, iron block, netherrack and furnace no).
- **Decisions:**
  - The anger lands at the start of the next entity tick (at most one tick late) rather than inside the command, because only the tick knows the players' positions and game modes. The IDLE and sight checks use the brain as the last tick left it.
  - The player's box is taken standing (1.8 high); a crouching player's is 1.5, which only moves the top of the 16-block reach by 0.3.
  - Chest vehicles do not exist in the game, so `chestVehicleDestroyed` has no caller.
- **Test steps (Nether, survival, `/difficulty normal`, golden boots on so the piglins stay calm, on a netherrack floor):**
  1. Use 4 piglin eggs around you. Place a gold block and mine it: every piglin within about 16 blocks attacks you, also one behind you or behind a wall (**11**). Back off for 30 seconds or kill them.
  2. Use 4 more piglin eggs. Place a chest behind a 3-high netherrack wall so that the piglins can't see you, stand behind the wall and open it: nothing happens. Walk around the wall into their sight and open it again: the ones that see you attack (**12**).
  3. Use 2 brute eggs and a piglin egg, give that piglin a gold ingot (right-click), and open a chest in sight of all of them while it admires: the admiring piglin keeps admiring (the brutes attack you anyway, as brutes do).
  4. Mine a block of iron, stone and netherrack near calm piglins: nothing happens. Mine raw gold, gilded blackstone and nether gold ore: they attack.
  5. `/gamemode creative`, mine a gold block next to calm piglins: nothing happens. `/gamemode survival`.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*16,minecraft:piglin_brute_spawn_egg*2,minecraft:gold_block*8,minecraft:raw_gold_block*4,minecraft:gilded_blackstone*4,minecraft:nether_gold_ore*4,minecraft:iron_block*4,minecraft:chest*4,minecraft:barrel*2,minecraft:gold_ingot*16,minecraft:golden_boots,minecraft:diamond_pickaxe,minecraft:diamond_sword,minecraft:netherrack*64,minecraft:cooked_beef*32
```

### b1.7. Minecraft piglins turn into zombified piglins outside the Nether

After 25c70ca2.

- **Changed:**
  - Vendored `world/piglins.rs`: `AbstractPiglin.customServerAiStep` after the brain. A piglin or brute that is not immune, has AI and stands where piglins zombify (b2.7's per-level `gameplay/piglins_zombify`, true in the Overworld and the End) is converting: the renderer shakes it and a piglin's activity sound is `entity.piglin.retreat`. `TimeInOverworld` counts up, anywhere else it goes back to 0, and past 300 (the 301st tick, 15 seconds) it plays `entity.piglin.converted_to_zombified` or `entity.piglin_brute.converted_to_zombified` (not in peaceful) and converts. A piglin first drops the item it is admiring (`cancelAdmiring`) and its whole inventory. `convert_piglin_to_zombified` (`ConversionParams.single(keepEquipment, preserveCanPickUpLoot)`): a new zombified piglin with its own id and UUID and the full zombified piglin AI, keeping the position, yaw, motion, ground, fall, fire, hurt cooldown, baby, persistence, effects, armour and main hand, with nausea for 200 ticks (amplifier 0). The piglin is gone.
  - The entity world lists each conversion (`PiglinConversion`: the whole equipment with components, drop chances, `CanPickUpLoot`, `LeftHanded`). `ServerSim::tick_mobs` turns it into the new mob's saved tag (`piglin_save::converted_tag`), so the zombified piglin saves its gear whole: enchantments, wear and the offhand item, which a zombified piglin here doesn't hold yet.
  - `piglin_render.rs`: `shake_degrees` (`cos(ageInTicks x 3.25) x pi x 0.4` degrees on the body yaw) while converting; `converting` travels in the tracked mob.
  - Formatting only: four assertions of b1.6's tests and the new code in the track's own new files, so they pass `rustfmt --check`.
- **Tests:** `world::piglins`: `a_piglin_converts_on_the_301st_overworld_tick_with_nausea_and_its_gear`, `an_immune_or_no_ai_piglin_never_converts`, `the_nether_resets_the_count`, `a_brute_converts_too`, `the_inventory_and_admired_item_drop_on_conversion` (an admired ingot drops, a held shield stays in the saved equipment), `no_converted_sound_in_peaceful`. `piglin_render`: `a_converting_piglin_shakes_its_body`. `server` (with the game data): `the_end_zombifies_and_the_nether_does_not` (also checks the saved tag of the zombified piglin: main hand, offhand and `CanPickUpLoot`).
- **Decisions:**
  - The flag is the level's, as for hoglins (b2.7): no 26.3 biome overrides `gameplay/piglins_zombify`.
  - Zombified piglins here hold only a main-hand item and armour as ids, and drop by their own chances; the piglin's offhand item, drop chances and `CanPickUpLoot` live on in the saved tag only (they come back whole when the chunk is saved and reloaded). A custom name and a lead are not carried over: mobs don't have them yet.
  - As in vanilla, a piglin holding a shield or a non-admired item in its offhand keeps it (in the save); a loved item it is no longer admiring was already put away by `StopHoldingItemIfNoLongerAdmiring`.
- **Test steps (`/difficulty normal`, survival):**
  1. In the Nether, build and light a portal (obsidian, flint and steel) and go to the Overworld. Use a piglin egg and a brute egg: both shake from side to side and the piglin grunts the retreat sound (**13**). After 15 seconds each turns into a zombified piglin with a groan, still holding its golden sword or axe and wearing its golden armour, and it wobbles a moment from nausea (**14**).
  2. `/summon minecraft:piglin ~ ~ ~ {IsImmuneToZombification:1b}`: it never shakes or converts.
  3. In the Overworld, drop 20 gold nuggets next to a new piglin, then about 10 seconds after it hatched give it a gold ingot (right-click): when it converts (before it is done admiring) the ingot and the nuggets drop on the ground. (Step corrected in b1.8's commit.)
  4. Use a piglin egg in the Overworld, wait about 10 seconds, save and quit, rejoin: it is still shaking and converts about 5 seconds later.
  5. Go to the End (`/execute in minecraft:the_end run tp @s 0 70 0`) and use a piglin egg: it converts there too. In the Nether no piglin ever shakes.
  6. `/difficulty peaceful` in the Overworld with a piglin egg: it converts without the groan (and the zombified piglin then leaves, as monsters do in peaceful).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*8,minecraft:piglin_brute_spawn_egg*2,minecraft:gold_ingot*16,minecraft:gold_nugget*32,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:golden_boots,minecraft:diamond_sword,minecraft:netherrack*64,minecraft:cooked_beef*32
```

### b1.8. Minecraft piglins hunt hoglins

After 2ce84f2e.

- **Changed:**
  - Vendored `piglin_brain`: the hoglin memories of `PiglinSpecificSensor` (`NEAREST_VISIBLE_HUNTABLE_HOGLIN`: the first visible adult hoglin that can be hunted; `NEAREST_VISIBLE_BABY_HOGLIN`), and IDLE `StartHuntingHoglin` for a piglin that can hunt (`CannotHunt` off, never a brute): an adult with a huntable hoglin in sight, no `angry_at` and no `hunted_recently`.
  - Vendored `world/piglins.rs`: the hunt goes ahead unless one of its visible adult piglins has hunted recently: anger at the hoglin, `dontKillAnyMoreHoglinsForAWhile` (`hunted_recently` for 600 to 2400 ticks from the level random), the anger broadcast, and every visible adult piglin rests too. `setAngerTarget` on a hoglin rests a piglin that can hunt; the anger broadcast skips piglins that can't hunt (brutes, `CannotHunt`) and hoglins that can't be hunted. `RememberIfHoglinWasKilled`: a melee hit that kills the hoglin it fights rests the piglin. `wasHurtBy`: an adult hurt by a hoglin while the hoglins outnumber its piglins (more than the piglins it sees plus one) retreats (`angry_at`, the attack and walk targets erased, the hoglin avoided for 5 to 20 seconds, resting from hunts) and `broadcastRetreat` makes each visible adult piglin retreat from the nearest of its avoided mob, its target and that hoglin. Hoglins that can't be hunted reach the sensor as a list built by the entity world (`cannot_be_hunted`).
  - `piglin_render.rs`: the dancing pose (`PiglinModel.setupAnim` DANCING: the ears flap, the head sways and bobs, both arms swing out and bob, the body bobs), also for the worn armour; dances come only after a hoglin kill (one time in ten, b1.3).
  - The hoglin side was already in place: b2's hoglin sensor counts adult piglins (not brutes) for its own retreat.
- **Tests:** `world::piglins`: `adult_piglins_hunt_a_visible_hoglin_and_rest_30_to_120_seconds_after` (three piglins go for a hoglin, all rest 600 to 2400 ticks from the start of the hunt, kill it and leave a second hoglin alone), `outnumbered_piglins_retreat_from_hoglins` (two piglins, three hoglins: the hurt one and its neighbour flee for 5 to 20 seconds), `a_piglin_with_cannot_hunt_leaves_hoglins_alone` (also a hoglin with `CannotBeHunted`), `about_one_kill_in_ten_ends_in_a_dance` (about 10% of kill times; a kill at a dancing time sets the dancing memory and arm pose, at another time only the celebration). `piglin_render`: `a_dancing_piglin_bobs_its_head_and_body_and_spreads_its_arms`.
- **Decisions:**
  - The hunt is decided in the brain and carried out by the entity world right after the piglin's brain tick, because it touches other piglins' memories; the anger lands in the same tick as in vanilla and the attack starts on the next one, as vanilla's `StartAttacking` (which runs before `StartHuntingHoglin`) does.
  - `RememberIfHoglinWasKilled` only ever fires in vanilla when the piglin's own melee hit kills its target in that tick (`StopAttackingIfTargetInvalid` drops a dead target first), so it lives in the piglin's hit. A crossbow kill doesn't rest the piglin, as in vanilla.
  - Riding stays out (brief decision): `babySometimesRideBabyHoglin` and the RIDE activity are not ported, so the baby hoglin memory is sensed but not used yet.
- **Test steps (Nether, survival, `/difficulty normal`, golden boots on, on a netherrack floor):**
  1. Use 3 piglin eggs close together, then a hoglin egg about 8 blocks away. The piglins (adults with swords) run at the hoglin and kill it (**15**). Now and then (about one kill in ten) they dance afterwards: ears flapping, arms out, bobbing (**16**).
  2. Right after a kill, use another hoglin egg near them: they leave it alone for 30 seconds to 2 minutes.
  3. Use 4 hoglin eggs together, then 2 piglin eggs next to them. The piglins go for a hoglin, the hoglins strike back, and after the first hoglin hit both piglins run away from the hoglins for 5 to 20 seconds (**17**).
  4. `/summon minecraft:piglin ~ ~ ~ {CannotHunt:1b}` next to a hoglin: it never attacks it. `/summon minecraft:hoglin ~ ~ ~ {CannotBeHunted:1b}` next to piglins: they leave it alone.
  5. A brute next to a hoglin ignores it.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*16,minecraft:piglin_brute_spawn_egg*2,minecraft:hoglin_spawn_egg*16,minecraft:golden_boots,minecraft:diamond_sword,minecraft:netherrack*64,minecraft:cooked_beef*32
```

### b1.9. Docs: Minecraft piglins, brutes and bartering

After 5ebaca7c.

- **Changed:** `docs/MINECRAFT-SURVIVAL.md`: a "Piglins, piglin brutes and bartering" part under the Nether (the brains, gold armour, fighting and the axe on shields, fleeing, admiring and bartering, guarding gold and containers, hunting hoglins, zombification, spawning and saving); handing a piglin a gold ingot in the right-click list; the bastion's piglins and brutes live; piglins leave N6 and the dormant list; the shield limitation now says a brute's axe disables a raised shield for 5 seconds; the riding gap (no piglins on hoglins) and the piglin gaps (spears, crossbow wear and model, the door check, the zombified piglin's offhand, old bastion brutes, chest vehicles) are listed; the hoglin line no longer waits for piglins. `docs/NETHER-END.md`: the piglin and piglin brute rows are present.
- **Decision:** as in entries 10 and b2.9, the vendored MinecraftOSS file count in CLAUDE.md is left for the round's hand-over.
- **Test steps:** none (docs only).
- **Combined kit for b1.6 to b1.8** (20 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*16,minecraft:piglin_brute_spawn_egg*2,minecraft:hoglin_spawn_egg*16,minecraft:gold_block*8,minecraft:raw_gold_block*4,minecraft:gilded_blackstone*4,minecraft:nether_gold_ore*4,minecraft:iron_block*4,minecraft:chest*4,minecraft:barrel*2,minecraft:gold_ingot*16,minecraft:gold_nugget*32,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:golden_boots,minecraft:diamond_pickaxe,minecraft:diamond_sword,minecraft:netherrack*64,minecraft:cooked_beef*32,minecraft:golden_apple*8
```

## Group b4-sleep-1

Brief `b4-sleeping.md`, groups 1 to 3.

### b4.1. Minecraft beds check the vanilla sleep rules: the night by sky light, monsters nearby and the sleep percentage

After 11dde7f8.

- **Changed:**
  - New `minecraft_survival/src/sleep.rs`: `dark_outside(sky_light, fixed_time)` (`Level.isDarkOutside`: no fixed time and `(int)(15 - sky_light_level) >= 4`), `monster_box(head)` (the head's bottom centre ± 8, 5, 8), `prevents_rest(kind, angry)` (every class that extends `Monster` in 26.3, checked with javap, also those the fork doesn't have yet; a zombified piglin only when angry at the sleeper), `monster_in` (strict box overlap, as `AABB.intersects`), and `SleepStatus` (`update`, `sleepers_needed`, `enough_sleeping`, `enough_deep_sleeping`, as `SleepStatus` in vanilla).
  - `minecraft_terrain/src/environment.rs`: `DimensionEnvironment::level_sky_light(clock, rain, thunder)` samples `gameplay/sky_light_level` for the whole level (no position), with the weather blend.
  - `minecraft_survival/src/dimension.rs`: `DimensionRules.has_fixed_time` from the dimension type.
  - `minecraft_survival/src/level.rs`: `Level::dark_outside(globals)` from the level's default clock, its own rain and thunder and `has_fixed_time`. It replaces the camera sample for bed use, and the respawn's bed check (`return_to_block`) uses it too (a one-line change).
  - `minecraft_survival/src/entities.rs`: `Entities::monster_near(area, sleeper)` over the mob mirror (zombies of every kind, skeletons of every kind, creepers, spiders, endermen, witches, blazes, slimes, hoglins and zoglins, piglins and brutes).
  - `respawn_block.rs`: `rest_in_bed` takes the monster check and `Sleeper.creative`, and returns `Rest { respawn, outcome }` with `Outcome::Sleep` or `Outcome::Problem`. Vanilla's order: too far, obstructed, the spawn is set, the bed rule's night test, then the monsters (not in creative). `NOT_SAFE` ("You may not rest now; there are monsters nearby"). `BedUse::Occupied` carries the bed head for group 2's villager kick-out.
  - `render_anim`: `minecraft_respawn::use_block` takes a `Night` (the level's darkness, creative, the monster check) and shows the new problem.
- **Tests:**
  - `sleep`: `the_sky_is_dark_at_light_eleven_or_less`, `the_overworld_night_runs_from_tick_12523_to_23477_in_clear_weather` (real data; also the Nether's 4), `a_thunderstorm_darkens_noon_but_rain_alone_does_not` (rain 11.5625), `sleepers_needed_rounds_up_and_needs_at_least_one`, `the_sleep_count_changes_only_when_someone_sleeps`, `monsters_count_in_an_eight_by_five_box_around_the_head`.
  - `respawn_block`: `the_rest_checks_run_in_vanilla_order`; `a_bed_in_reach_sets_the_spawn_by_day_and_says_you_can_sleep_only_at_night` moved to the new result.
  - `entities`: `the_bed_check_sees_zombies_zoglins_and_piglins_and_only_angry_zombified_piglins`.
  - `dimension`: both tests check `has_fixed_time`.
- **Decisions:**
  - The fixed time is the dimension type's `has_fixed_time`, not "no default clock" as the brief had it: the End has a clock but a fixed time (`has_fixed_time: true` in 26.3), so beds there are never "dark" (they explode anyway).
  - A zombified piglin is angry at the sleeper when its persistent anger target (`NeutralMob.isAngryAt`, universal anger off) is that player, not when it merely targets them.
  - The mob-burning `bright_outside` in the adapter still reads the camera sample: it is not a one-line change and it is a different rule (`isBrightOutside`).
  - Nobody lies down yet (group 2): a bed that passes every check just sets the spawn and shows nothing.
- **Test steps (Overworld, survival, `/difficulty normal`):**
  1. By day, place the red bed on open ground and right-click it: "You can sleep only at night or during thunderstorms" above the hotbar, and "Respawn point set" in the chat the first time.
  2. `/time set 12400` and right-click at once: the same message. Wait about 7 seconds (tick 12523) and right-click: no message.
  3. Still at night, use a zombie egg about 6 blocks from the bed behind a cobblestone wall and right-click the bed: "You may not rest now; there are monsters nearby" (**1**). Kill it, use 2 slime eggs next to the bed and right-click: no message.
  4. Use a zombified piglin egg next to the bed and right-click: no message. Hit it once and right-click the bed while it is angry: "there are monsters nearby".
  5. `/gamemode creative`, use a zombie egg beside the bed and right-click: no message. `/gamemode survival`.
  6. Stand 4 blocks from the bed and right-click: "You may not rest now; the bed is too far away". Put cobblestone on top of the pillow block: "This bed is obstructed".
  7. `/time set noon`, `/weather rain`, right-click: "You can sleep only at night or during thunderstorms". `/weather thunder`, wait about 5 seconds and right-click: no message.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:red_bed,minecraft:zombie_spawn_egg*4,minecraft:slime_spawn_egg*2,minecraft:zombified_piglin_spawn_egg*2,minecraft:cobblestone*64,minecraft:diamond_sword
```

### b4.2. Minecraft players lie down in beds and the night passes when enough of them sleep

After f4d14bdc.

- **Changed:**
  - `sleep.rs`: `Sleeping { head, facing, straw, hurts }`; `lie_down` (`LivingEntity.startSleeping`: dismount first, `occupied=true` on both halves, the feet at the head's centre and `sleep height + 0.125` with zero motion, the timer and `time_since_rest` back to 0); `get_up` (`stopSleeping`: `occupied=false`, a straw bed with `destroy_on_leave` breaks with `block.straw_bed.break_leave`, then `findStandUpPosition` or the head + (0.5, 1.1, 0.5), facing the head, pitch 0; the timer to 0 or 100); `tick_sleepers` (`ServerLevel.tick`'s skip, then each player's `Player.tick`: the timer to 100 and the fade to 110, waking when it gets light, when the bed is gone, when hurt or dead, and `time_since_rest`); `start`, `stop`, `tick` and `serve` over the level and the players map; `Server::tick_sleep(dimension)` for track E. The announcements `sleep.players_sleeping` and `sleep.skipping_night` go to every player of the level when the count changes, only while `Server.published` is set and the percentage is at most 100; `sleep.not_possible` always shows above 100.
  - The skip: `ticks_to_marker(total, 0)` through the new `Globals::set_clock` (shared with `/time set` so `save.shift_day` stays in step), every sleeper wakes with the fade, and with `advance_weather` while the rain level is above 0.2 `Entities::reset_weather_cycle` sends rain and thunder times 0 and both flags off (the clear-weather time stays) and mirrors it into `Globals.weather`.
  - `Player.sleeping`, `sleep_timer`, `time_since_rest`; `Level.sleep_status`; `Server.published` (false until track E); `Survival::hurts()` counts landed hurts.
  - An occupied bed: `Entities::wake_villager_in(head)` checks the mirror for a sleeping villager in the head block and sends `Command::WakeVillagerIn`; the vendored `EntityWorld::wake_villager_in` sets its `wake_pending`. Only when none sleeps there does "This bed is occupied" show.
  - `render_anim`: a bed that passes every check becomes `Runtime.bed_request`; at the start of the next frame `sleep::serve` lies the player down (dismounting a strider rider first) and runs one sleep tick for each server tick of the last frame. The written blocks get their MW2 collision shapes back and the local player's overlay lines are shown. The move onto the bed and off it goes through `Player.arrival`, as a respawn does.
- **Tests:** `sleep`: `lying_down_marks_the_bed_occupied_and_puts_the_feet_on_it` (red bed 64.6875, straw bed 64.375), `the_sleep_timer_counts_to_100_and_the_wake_fade_to_110`, `the_night_skips_to_the_next_morning_once_everyone_has_slept_100_ticks` (13000 + 48000 to 72000 on the 101st tick, never before; `advance_time` off keeps the clock but wakes), `a_skip_clears_rain_and_thunder_only_when_it_rains`, `half_the_players_are_enough_at_fifty_percent_and_none_at_101` (also the announcements), `leaving_the_bed_stands_up_beside_it_facing_it`, `a_straw_bed_breaks_when_left`, `a_broken_bed_or_daylight_or_a_hit_wakes_the_sleeper`, `time_since_rest_counts_while_awake_and_resets_on_sleep_and_death`, `an_occupied_bed_with_a_sleeping_villager_wakes_the_villager_instead_of_complaining`.
- **Decisions:**
  - The bed request and the sleep ticks wait for the start of the next frame, where the adapter holds the whole server (the use chain only sees the player and level in pieces). That is one frame late. Track E turns the request into a client action.
  - The skip comes in the level part of the tick and the players' part follows in the same tick, as in vanilla, so a woken sleeper's timer reads 101 after that tick.
  - The sleep count is refreshed at the start of every sleep tick, which also catches players joining, leaving or turning spectator (vanilla refreshes on those events). Several wake-ups in one tick give one announcement.
  - A hurt wakes the sleeper when it lowers health (`Survival::hurts`); vanilla also wakes on a hit that its checks let through but that does no damage.
  - A dead sleeper frees the bed but is not moved, so a stale stand-up place can't follow the respawn.
  - The villager gets up on its own next tick (`wake_pending`), one tick after vanilla's `kickVillagerOutOfBed`.
  - The sleep height is the top of the bed's collision box, which is the same as its outline for beds and straw beds. `occupied` is written on both halves at once, as `BedBlock.updateShape` copies it to the foot.
  - Until group 3 there is no Leave Bed button: waking needs the morning, daylight, a hit or a broken bed.
- **Test steps (Overworld, survival, a cleared flat area, `/difficulty normal`):**
  1. `/time set 12400`, wait about 7 seconds, right-click the red bed: you are put onto the bed (**2**). Stay put. About 5 seconds later the sky jumps to sunrise and you stand beside the bed facing it (**3**). `/time query daytime` gives a few ticks past 0.
  2. `/time set night`, `/weather thunder`, lie down: after the skip the rain and thunder stop (the rain fades over a few seconds).
  3. `/gamerule players_sleeping_percentage 101`, `/time set night`, lie down: "No amount of rest can pass this night" (**4**). Wait 10 seconds: nothing happens. `/time set day`: you get up. `/gamerule players_sleeping_percentage 100`.
  4. `/time set night`, lie down, press T and type `/effect give @s minecraft:poison 5`: you get up at the first poison hit.
  5. `/time set night`, place the straw bed, lie down and wait for the morning: you get up and the straw bed breaks with its rustle. It never said "Respawn point set".
  6. `/time set night`, lie down, press T and type `/setblock ~ ~ ~ minecraft:air` (the pillow block you lie on): you get up at once.
  7. Villager: by day place the blue bed and use a villager egg next to it. `/time set 13000` and wait until the villager lies down in the bed. Right-click the bed: the villager gets up and no message shows.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:red_bed,minecraft:blue_bed,minecraft:straw_bed,minecraft:villager_spawn_egg*2,minecraft:cobblestone*64,minecraft:diamond_sword,minecraft:clock
```
