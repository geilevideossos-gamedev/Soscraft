# Track B, item b1: piglins, piglin brutes, bartering and gold armour (design brief)

Paths and line numbers are as of e2313625. Vanilla facts were checked with javap on the obfuscation-free 26.3 `client.jar` and against the 26.3 data under `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3`. No Mojang code is copied; this brief describes behaviour and numbers only.

## 1. The owner's request

> "Piglins and piglin brutes with bartering and the gold armour rules (from the Nether leftovers N6)."

Scope as given for this item: full vanilla. That means the `PiglinAi` brain:
- admiring gold for 6 seconds, then bartering from `gameplay/piglin_bartering`;
- anger at players who wear no gold, who open chests or who mine gold blocks nearby;
- hunting hoglins, and avoiding soul fire and zombified piglins;
- babies, crossbow and golden sword attacks;
- turning into zombified piglins after 300 ticks outside the Nether.

It also covers:
- the piglin brute (bastions only, golden axe, no bartering);
- the gold armour that makes piglins neutral: any piece in the 26.3 item tag `#minecraft:piglin_safe_armor`;
- renderers and sounds.

The state doc lists this as open: "N6, the rest of the Nether: piglins (the bastion's wait dormant), piglin brutes, hoglins and zoglins, ... piglin bartering" (`docs/MINECRAFT-SURVIVAL.md:280`). Hoglins, zoglins and striders are other track B items. This brief only touches hoglins where piglins hunt them (group 8).

## 2. What exists today

| Area | State |
|---|---|
| Bastion piglins and brutes | Generated with `PersistenceRequired`, then dormant: saved, never ticked, drawn or hit (`crates/minecraft_terrain/src/dormant.rs:4-12`; the test `crates/minecraft_terrain/src/terrain.rs:2473`). |
| Structure finalize | `finalize_piglin` (`third_party/minecraftoss/generator/src/feature/entities.rs:223-260`, called at `:134`) sets `hunted_recently` (600 to 2400 ticks) and rolls golden armour at 10% a piece. Brutes get no finalize, so the template brute has no golden axe and no `home` memory. The bastion templates `bastion/mobs/melee_piglin(_always).nbt` hold a bare `minecraft:piglin_brute`; `sword_piglin` and `crossbow_piglin` hold a piglin with its weapon. |
| Peaceful and despawn | `ALLOWED_IN_PEACEFUL` already lists `minecraft:piglin` (`dormant.rs:4-12`). `MobCategory::of_type` knows piglins and brutes as monsters (`third_party/minecraftoss/world/src/natural_spawner/spawning.rs:97-103`). |
| Natural spawning | `type_info` has no `piglin` (`third_party/minecraftoss/world/src/natural_spawner.rs:264-306`), so the nether_wastes entry (weight 15, count 4) and the crimson_forest entry (weight 5, count 3-4) never spawn. `finalize` has no piglin arm (`spawning.rs:959-963`, which falls through to "unsupported"). |
| Model | `piglin_render.rs` already has `AdultPiglinModel` and `BabyPiglinModel` cuboids, ear flaps and armour. It is only used for zombified piglins with zombie arms (`crates/minecraft_terrain/src/piglin_render.rs:49-131`, `:161-245`, `:247-329`). Only the `zombified_piglin*` textures are in the atlas (`crates/minecraft_terrain/src/mesh.rs:533-535`); the pack has `piglin.png`, `piglin_baby.png` and `piglin_brute.png`. |
| Zombified piglins | Live (round 7): `ZombieKind::ZombifiedPiglin`, group anger and portal piglins (`third_party/minecraftoss/entities/src/world.rs:4893-5085`). A golden spear is held, but the spear goal is a `NeverGoal` (`third_party/minecraftoss/entities/src/monster_ai.rs:1670-1671`). |
| Mob conversion | Zombie to drowned and husk to zombie: `convert_drowning_zombie` and `convert_zombie_to_drowned` (`world.rs:3259-3312`). These are the pattern for piglin zombification. |
| Wither skeleton targets | The `NearestAttackableTargetGoal<AbstractPiglin>` slot is a placeholder `Prey::Absent` (`monster_ai.rs:1186`). `is_enemy` has no piglins (`monster_ai.rs:326`). |
| Brain framework | Villager-only: `villager_brain/{mod,memory,behaviors}.rs`. Reusable parts: the expiring `Slot<T>` (`villager_brain/memory.rs:14-75`), `Seen`/`seen_item` (`mod.rs:92`, `:162`), `MoveToTargetSink` (`behaviors.rs:854`), `GoToWantedItem` (`:1330`), `InteractWithDoor` and `door_halves` (`:3349`, `:3290`), and the throw math in `throw_half_stack` (`:1565-1581`). `Ctx`/`Me` are villager-shaped (`mod.rs:191-330`). |
| Item pickup by mobs | `villager_pick_up` (`third_party/minecraftoss/entities/src/world/villagers.rs:191-218`) over `World::items_in`/`take_item`. `spawn_item` throws items (`third_party/minecraftoss/player/src/lib.rs:95-117`). |
| Item tags | `RecipeBook::item_in_tag` (used by `mob_actions.rs:139`). `EntityWorld` holds the recipe book (`world.rs:2304`, `set_recipe_book` at `:4216`). |
| Loot | `EntityLootBook` loads `loot_table/entities/` and `charged_creeper/` only, rolls only `type: minecraft:entity`, and knows `set_count`, `enchanted_count_increase` (looting 0), `furnace_smelt` and `set_potion` (`third_party/minecraftoss/entities/src/loot.rs:144-191`, `:228-245`, `:395-432`). The server owns one per level (`crates/minecraft_terrain/src/server.rs:56`, `:332`) and spawns death loot after the mob tick (`server.rs:691`, `:700-711`). |
| Players as mobs see them | `TickInput` carries `mob_players: Vec<PlayerCandidate>`, `mob_views` and `mob_vitals` keyed by player id (`server.rs:2314-2339`), applied at `server.rs:2758-2785`. The client builds them for player `PLAYER = 0` (`crates/minecraft_survival/src/entities.rs:36`, `:151-181`, `:861-893`). `PlayerCandidate` is built as a struct literal in about 60 places across 37 files, including `third_party/minecraftoss/entities/src/bin/*`. Nothing tells mobs about armour or the offhand. |
| Right-click on mobs | **Not wired on the host.** `ServerSim::mob_action` runs `mob_actions::interact` only when `attack` is `None` (`server.rs:357-385`), but the host only sends attacks (`minecraft_survival/src/entities.rs:543`, `:778`) and never applies `MobResult.slots`. b3 group 1 ("Minecraft players use items on mobs") wires it, with a host predicate `mob_actions::would_interact`. |
| Shared pieces from b2 | `MobEffect` (`entities/src/effects.rs:17`) has no `Nausea`. `MobCandidate` (`monster_ai.rs:304`) has no baby flag. `MobWorld` (`server_mobs.rs:167`) doesn't override `World::block_in_tag`, so mobs only see the hand-written tags of `authored_block_in_tag` (`third_party/minecraftoss/player/src/lib.rs:1011-1040`), which lack `#piglin_repellents`. b2 adds all three (commits 1 and 5), plus `Level::piglins_zombify` (commit 7) and a reusable brain module (commits 4-6). |
| Shields and axes | Mob axe hits don't disable a raised shield (`docs/MINECRAFT-SURVIVAL.md:315`). The shield and hurt loop is at `render_anim/src/minecraft_world.rs:1803-1822`. |
| Player hits, blasts and use | `hurt_by_player` sets `hurt_by` per kind (`third_party/minecraftoss/entities/src/world/combat.rs:234-290`). `player_blast` handles MW2 bullets and grenades (`world.rs:2689-2715`). `mob_actions::interact` (`crates/minecraft_terrain/src/mob_actions.rs:129`) handles held-item use. |
| Mining and containers | `ServerSim::player_mines` (`server.rs:1225-1239`) gets the broken blocks and how they broke. Containers open on the client side: `toggle_openers` (`crates/minecraft_survival/src/level.rs:245-264`), called from `crates/render_anim/src/minecraft_world.rs:2027`. Mining reaches the server from `minecraft_world.rs:1613` through `Entities::mined` (`entities.rs:731`). |
| Dimension rule | `gameplay/piglins_zombify` is declared, default true (`third_party/minecraftoss/core/src/environment.rs:146`), and false only in `dimension_type/the_nether.json`. Nothing reads it. `Level::monsters_burn` (`third_party/minecraftoss/world/src/level/time.rs:227-232`) shows how to read such a flag. |
| Game rules | `universal_anger` (off) and `forgive_dead_players` (on) are declared (`third_party/minecraftoss/world/src/game_rules.rs:127`, `:169`). `apply_game_rules` passes only `mob_griefing` to the mobs (`server.rs:2013-2026`). |
| Wiring points for a new kind | `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:388-419`), `spawn_saved` (`:424`), `mob_tags` (`:1048`), `client_mobs::server_mobs` (`crates/minecraft_terrain/src/client_mobs.rs:1024`) and `shadow_casters` (`:1121`). Survival `boxes`/`meshes`/`encode`/`decode` are at `crates/minecraft_survival/src/entities.rs:1084`, `:1179`, `:1350` and `:1374`. Keys 0-16 and 18 are mob kinds, and 17 (fireballs), 19 (crystals) and 20 (the dragon's parts, `DRAGON_KEY`) are taken (`entities.rs:1398-1401`). Track C's c1 takes 26 and 27 and leaves 21-25 to track B. `EntityKey` is at `world.rs:1877`, `MobHit` at `:1929`, the `EntityWorld` storage at `:2267` and `entity_type` at `:3932`. |
| Death messages | `death.rs:656-660` names skeleton, zombie, creeper, ghast and blaze only. |
| Crossbows, spears and riding | There is no crossbow item use or charged-crossbow model state. Spears have no use (`docs/MINECRAFT-SURVIVAL.md:50`). There is no mount or passenger system. |

## 3. Vanilla 26.3 behaviour (javap)

Classes read: `AbstractPiglin`, `Piglin`, `PiglinBrute`, `PiglinAi`, `PiglinBruteAi`, `PiglinSpecificSensor`, `PiglinBruteSpecificSensor`, `StartAdmiringItemIfSeen`, `StopAdmiringIfItemTooFarAway`, `StopAdmiringIfTiredOfTryingToReachItem`, `StopHoldingItemIfNoLongerAdmiring`, `StartHuntingHoglin`, `RememberIfHoglinWasKilled`, `StartCelebratingIfTargetDead`, `StopBeingAngryIfTargetDead`, `BackUpIfTooClose`, `CrossbowAttack`, `CrossbowAttackMob`, `RangedAttackMob`, `CrossbowItem`, `EnchantRandomlyFunction`, `NearestItemSensor`, `NearestLivingEntitySensor`, `PlayerSensor`, `MemoryModuleType`, `EntityTypes`, `Block`, `ChestBlock`, `BarrelBlock`, `ShulkerBoxBlock`, `EnderChestBlock`, `ContainerEntity`, `PiglinModel`, `PiglinRenderer` and `LivingEntityRenderer`.

### 3.1 Types, attributes and saving
- **`EntityTypes`:**
  - Piglin: `MONSTER`, sized 0.6 x 1.95, eye height 1.79, tracking range 8. It is **not** `notInPeaceful`.
  - Piglin brute: the same size, but `notInPeaceful`.
  - Neither is fire-immune.
- **`Piglin.createAttributes`:** health 16, speed 0.35, attack damage 5. The follow range stays at the mob default of 16.
  - A baby gets the `minecraft:baby` speed modifier +0.2 (`ADD_MULTIPLIED_BASE`).
  - `BABY_DIMENSIONS`: scalable 0.49 x 0.98, eye height 0.78.
  - `xpReward` is 5, and `getBaseExperienceReward` returns it flat, with no equipment bonus.
- **`PiglinBrute.createAttributes`:** health 50, speed 0.35, attack damage 7, follow range 12.
  - `xpReward` is 20, plus `Mob`'s 1-3 per equipped item.
- **`AbstractPiglin`:**
  - Its constructor turns on `setCanPickUpLoot(true)` and opening doors.
  - Path malus: `FIRE_IN_NEIGHBOR` 16, `FIRE` -1.
  - It saves `IsImmuneToZombification` and `TimeInOverworld`, and reads `CanPickUpLoot` with a default of true.
- **`Piglin` saves** `IsBaby`, `CannotHunt` and `Inventory`: a `SimpleContainer(8)`, exposed as slots 300-307.
- **Saved brain memories** (`MemoryModuleType` entries registered with a codec):
  - `angry_at`: a UUID;
  - `universal_anger`, `admiring_item`, `admiring_disabled` and `hunted_recently`: booleans with a ttl;
  - `home`: a `GlobalPos`, for brutes.
  - `ate_recently` and the others are not saved.
- **Damage:**
  - A piglin with a golden sword hits for 5 + 3 = 8 on normal.
  - A brute with a golden axe hits for 7 + 6 = 13 on normal (the golden axe's tool damage is 6).
  - Every axe carries the `minecraft:weapon` component with `disable_blocking_for_seconds` 5 (`Item.Properties.axe`). `Player.blockUsingItem` calls `BlocksAttacks.disable` when the attacker's `getSecondsToDisableBlocking()` (its main-hand weapon) is above 0, so a brute hit on a raised shield puts the shield on a 5-second cooldown with `item.shield.break`.

### 3.2 Spawning and finalize
- **`checkPiglinSpawnRules`:** the block below is not a nether wart block. Nothing else: no light test.
  - Placement is on the ground, with `MOTION_BLOCKING_NO_LEAVES`.
  - Biome entries: nether_wastes (weight 15, count 4) and crimson_forest (weight 5, count 3-4).
- **`Piglin.finalizeSpawn`**, for any reason but `STRUCTURE`:
  - with `nextFloat() < 0.2` it becomes a baby;
  - otherwise an adult gets `createSpawnWeapon`: `nextFloat() < 0.5` gives a crossbow, else `nextInt(10) == 0` gives a golden spear, else a golden sword.
  - Then, for every reason:
    - `PiglinAi.initMemories`: `hunted_recently` for `rangeOfSeconds(30, 120)`, i.e. 600..2400 ticks;
    - `populateDefaultEquipmentSlots`: adults roll golden helmet, chestplate, leggings and boots, each with `nextFloat() < 0.1`;
    - `populateDefaultEquipmentEnchantments`;
    - then `Monster` and `Mob` finalize.
- **`PiglinBrute.finalizeSpawn`:**
  - `initMemories` sets `home` to `GlobalPos(level dimension, blockPosition())`;
  - it puts a golden axe in the main hand;
  - then `AbstractPiglin`/`Mob` finalize. There is no enchantment pass of its own.
- **Brutes only come from bastions** (the `melee_piglin` templates), from spawn eggs and from `/summon`. There is no biome entry.

### 3.3 Gold armour and the sensors
- **`PiglinAi.isWearingSafeArmor`:** any of the four armour slots holds an item in `#minecraft:piglin_safe_armor`. In 26.3 that is the golden helmet, chestplate, leggings and boots, so one piece is enough.
- **`NearestLivingEntitySensor` and `PlayerSensor`:**
  - range = the follow range (16 for piglins, 12 for brutes), in a box inflated by that range on all three axes;
  - "attackable" and "targetable" use `TargetingConditions`;
  - players in creative or spectator, or invulnerable ones, are never attackable (`Player.canBeSeenAsEnemy`).
- **`PiglinSpecificSensor.doTick`** walks the nearest visible living entities in order and fills:
  - `NEAREST_VISIBLE_NEMESIS`: the first wither skeleton or wither;
  - `NEAREST_TARGETABLE_PLAYER_NOT_WEARING_GOLD`: the first player who is not wearing safe armour and whom the piglin `canAttack`;
  - `NEAREST_PLAYER_HOLDING_WANTED_ITEM`: the first non-spectator player holding a `#piglin_loved` item in either hand;
  - `NEAREST_VISIBLE_ZOMBIFIED`: the first zombified piglin or zoglin;
  - for hoglins: `NEAREST_VISIBLE_BABY_HOGLIN`, `NEAREST_VISIBLE_HUNTABLE_HOGLIN` (adult and `canBeHunted`) and `VISIBLE_ADULT_HOGLIN_COUNT`;
  - from visible adult piglins and all brutes: `NEAREST_VISIBLE_ADULT_PIGLINS` and `VISIBLE_ADULT_PIGLIN_COUNT`;
  - `NEARBY_ADULT_PIGLINS`: every adult `AbstractPiglin` (brutes included) among `NEAREST_LIVING_ENTITIES`, seen or not;
  - `NEAREST_REPELLENT`: the first block in `findBlocksInBoxByManhattanDistance(pos, 8, 4)` that is in `#piglin_repellents` (soul fire, soul torch, soul wall torch, soul lantern, soul campfire). A soul campfire counts only while lit.
- **`PiglinBruteSpecificSensor`:** `NEAREST_VISIBLE_NEMESIS` (wither skeleton or wither) and `NEARBY_ADULT_PIGLINS`.
- **`NearestItemSensor`:**
  - items within 32 / 16 / 32 that the mob `wantsToPickUp`, that it can see, and that are closer than 32;
  - the closest goes into `NEAREST_VISIBLE_WANTED_ITEM`.
- **Sensors tick** every `Sensor.DEFAULT_SCAN_RATE` (20 ticks) from a random offset, as the villager brain port already does.

### 3.4 The piglin brain (`PiglinAi`)
Activities, the first valid one wins (`updateActivity`): `ADMIRE_ITEM`, `FIGHT`, `AVOID`, `CELEBRATE`, `RIDE`, `IDLE`. Then the brain:
- plays the new activity's sound when the activity changes (`getSoundForActivity`);
- sets aggressive while `ATTACK_TARGET` is set;
- stops a baby riding a baby (`isBabyRidingBaby`);
- erases `DANCING` once `CELEBRATE_LOCATION` is gone, and syncs `isDancing`.

**CORE** (priority 0):
- `LookAtTargetSink(45, 90)`, `MoveToTargetSink` and `InteractWithDoor`.
- `babyAvoidNemesis`: a baby copies `NEAREST_VISIBLE_NEMESIS` into `AVOID_TARGET` for 5-7 s.
- `avoidZombified`: when `isNearZombified` (the zombified piglin is within 6), it copies `NEAREST_VISIBLE_ZOMBIFIED` into `AVOID_TARGET` for 5-7 s.
- `StopHoldingItemIfNoLongerAdmiring`.
- `StartAdmiringItemIfSeen(119)`.
- `StartCelebratingIfTargetDead(300, wantsToDance)`.
- `StopBeingAngryIfTargetDead`.

**IDLE** (priority 10):
- `SetEntityLookTarget(isPlayerHoldingLovedItem, 14)`.
- `StartAttacking`: only adults, using the target finder `findNearestValidAttackTarget`.
- `StartHuntingHoglin`, triggered while `canHunt`.
- `avoidRepellent`: `SetWalkTargetAwayFrom.pos(NEAREST_REPELLENT, 1.0, 8, false)`.
- `babySometimesRideBabyHoglin`.
- Idle look: a `RunOne` of looking at a player, a piglin or anything within 8, or `DoNothing(30, 60)`.
- Idle movement: a `RunOne` of `RandomStroll.stroll(0.6)` (weight 2), `InteractWith(PIGLIN, 8, 0.6, 2)` (2), `SetWalkTargetFromLookTarget(0.6, 3)` while no player holds a loved item (2), and `DoNothing(30, 60)` (1).
- `SetLookAndInteract(PLAYER, 4)`.

**`findNearestValidAttackTarget`**, in order:
1. Near a zombified piglin: no target.
2. `ANGRY_AT`, when it is attackable ignoring line of sight.
3. With `UNIVERSAL_ANGER`: the visible attackable player.
4. `NEAREST_VISIBLE_NEMESIS`.
5. `NEAREST_TARGETABLE_PLAYER_NOT_WEARING_GOLD`, when attackable.
6. Otherwise nothing.

**FIGHT** (priority 10, needs `ATTACK_TARGET`):
- `StopAttackingIfTargetInvalid`: it also stops when the target is no longer the nearest valid one.
- `BackUpIfTooClose(5, 0.75)`, only while holding a crossbow.
- `SetWalkTargetFromAttackTargetIfTargetOutOfReach(1.0)`.
- The three spear behaviours (`SpearApproach(1.0, 10)`, `SpearAttack`, `SpearRetreat`).
- `MeleeAttack(20)`.
- `CrossbowAttack`:
  - uncharged: start using the crossbow and set `charging`;
  - charging: after `getChargeDuration` (1.25 s × 20 = 25 ticks, no quick charge) release, and wait `attackDelay = 20 + nextInt(20)`;
  - then `performRangedAttack`: `CrossbowItem.performShooting` with power 1.6 (`MOB_ARROW_POWER`) and uncertainty `14 - difficulty * 4`;
  - the arrow is not critical, because only a player shooter crits. Its hit sound is `item.crossbow.hit`;
  - sounds `item.crossbow.loading_start/middle/end` and `item.crossbow.shoot`; `TIMEOUT` is 1200.
- `RememberIfHoglinWasKilled`.
- `EraseMemoryIf(isNearZombified, ATTACK_TARGET)`.

**CELEBRATE** (priority 10, needs `CELEBRATE_LOCATION`):
- `avoidRepellent` and the loved-item look.
- `StartAttacking` for adults.
- Walk to the celebrate location: `GoToTargetLocation(2, 1.0)` when not dancing, `(4, 0.6)` when dancing.
- A `RunOne`, each with weight 1, of looking at piglins within 8, `RandomStroll.stroll(0.6, 2, 1)` (2 blocks out, 1 up or down) or `DoNothing(10, 20)`.
- `StartCelebratingIfTargetDead`: when the attack target is dead, set `CELEBRATE_LOCATION` = its position for 300 ticks, plus `DANCING` for 300 ticks if `wantsToDance`. That is the case when the victim is a hoglin and `RandomSource.createThreadLocalInstance(gameTime).nextFloat() < 0.1`. With `forgive_dead_players` a dead player's anger is erased too.

**ADMIRE_ITEM** (priority 10, needs `ADMIRING_ITEM`):
- `GoToWantedItem(isNotHoldingLovedItemInOffHand, 1.0, true, 9)`.
- `StopAdmiringIfItemTooFarAway(9)`.
- `StopAdmiringIfTiredOfTryingToReachItem(200, 200)`: with an empty offhand, it counts `TIME_TRYING_TO_REACH_ADMIRE_ITEM`. Past 200 it erases `ADMIRING_ITEM` and sets `DISABLE_WALK_TO_ADMIRE_ITEM` for 200 ticks.

**AVOID** (priority 10, needs `AVOID_TARGET`):
- `SetWalkTargetAwayFrom.entity(AVOID_TARGET, 1.0, 12, true)`, plus the idle look and movement.
- `EraseMemoryIf(wantsToStopFleeing)`: no avoid target, or a hoglin target once the piglins equal or outnumber the hoglins, or a zombified one that is no longer the nearest visible zombified.

**RIDE:** `Mount(0.8)` and dismounting. This needs a passenger system, which the repo lacks (see Decisions).

**Admiring and bartering:**
- **`wantsToPickup(item)`**, in order:
  - a baby never picks up `#ignored_by_piglin_babies` (leather);
  - nothing in `#piglin_repellents` (items);
  - nothing while admiring is disabled and it has an attack target;
  - a gold ingot when the offhand holds no loved item;
  - gold nuggets when they fit the inventory;
  - `#piglin_food` (porkchop, cooked porkchop) when it has not eaten recently and the offhand holds no loved item;
  - a `#piglin_loved` item when the offhand holds no loved item;
  - otherwise `canReplaceCurrentItem`.
- **`Piglin.wantsToPickUp`** also needs `mob_griefing` and `canPickUpLoot`. Pick-up reach is the box grown by (1, 0, 1).
- **`PiglinAi.pickUpItem`:**
  - it stops walking;
  - gold nuggets take the whole stack into the inventory; anything else takes one item;
  - a loved item erases `TIME_TRYING...`, goes to the offhand (dropping the old offhand item) and starts admiring (`ADMIRING_ITEM` for 119 ticks);
  - food, when it has not eaten recently, is eaten (`ATE_RECENTLY` for 200 ticks);
  - otherwise it equips the item if it can, else puts it in the inventory.
  - Holding a non-gold-ingot item in a hand makes it persistent (`holdInMainHand`/`holdInOffHand` call `setPersistenceRequired`).
- **`canAdmire`:** admiring is not disabled, it is not already admiring, it is an adult, and the item is a gold ingot.
  - **`mobInteract`** (right-click with a gold ingot): one is consumed (`consumeAndReturn(1)`, which keeps it in creative), held in the offhand and admired, and the piglin stops walking. The result is `SUCCESS`.
- **`stopHoldingOffHandItem(barter)`**, run by `StopHoldingItemIfNoLongerAdmiring` when `ADMIRING_ITEM` expires, with `barter = true`:
  - An adult holding a gold ingot with `barter` throws `getBarterResponseItems`, rolled from `gameplay/piglin_bartering` with `THIS_ENTITY`.
  - Otherwise the item is equipped, or put in the inventory.
  - A baby equips it, or (if its main hand holds a loved item) stores it, or throws back the main-hand item and holds the new one.
- **`throwItems`:**
  - toward `NEAREST_VISIBLE_PLAYER`, else toward `LandRandomPos.getPos(piglin, 4, 2)`, else its own position;
  - the target is plus (0, 1, 0), and it swings the offhand;
  - `BehaviorUtils.throwItem`: from the eye height minus 0.3, at 0.3 along the normalised way.
- **The barter table** (`loot_table/gameplay/piglin_bartering.json`, `type: minecraft:barter`, one roll, total weight 469, random sequence `minecraft:gameplay/piglin_bartering`):

  | Entry | Weight | Count or function |
  |---|---|---|
  | book | 5 | `enchant_randomly` with options `minecraft:soul_speed`, so an enchanted book with stored soul speed |
  | iron_boots | 8 | `enchant_randomly` soul_speed |
  | potion (fire_resistance) | 8 | |
  | splash_potion (fire_resistance) | 8 | |
  | potion (water) | 10 | |
  | iron_nugget | 10 | 10-36 |
  | ender_pearl | 10 | 2-4 |
  | dried_ghast | 10 | 1 |
  | string | 20 | 3-9 |
  | quartz | 20 | 5-12 |
  | obsidian | 40 | |
  | crying_obsidian | 40 | 1-3 |
  | fire_charge | 40 | |
  | leather | 40 | 2-4 |
  | soul_sand | 40 | 2-8 |
  | nether_brick | 40 | 2-8 |
  | spectral_arrow | 40 | 6-12 |
  | gravel | 40 | 8-16 |
  | blackstone | 40 | 8-16 |

  - `EnchantRandomlyFunction.run`: a book skips the compatibility filter. It picks one option with `Util.getRandomSafe` (one `nextInt(size)` draw), then `enchantItem` draws `Mth.nextInt(min, max)`: 1..3 for soul speed. A book becomes `minecraft:enchanted_book` with stored enchantments; other items are enchanted in place.

**Anger:**
- **`wasHurtBy(attacker)`:**
  - An attacker that is a `Piglin` is ignored.
  - If the offhand is held, `stopHoldingOffHandItem(false)` runs: no barter, the ingot goes to the inventory.
  - It erases `CELEBRATE_LOCATION`, `DANCING` and `ADMIRING_ITEM`.
  - A player attacker sets `ADMIRING_DISABLED` for 400 ticks.
  - An avoid target of another type is erased.
  - A baby sets `AVOID_TARGET` = the attacker for 100 ticks and, if the attacker is attackable, `broadcastAngerTarget`.
  - A hoglin attacker while the hoglins outnumber the piglins (`VISIBLE_ADULT_PIGLIN_COUNT + 1 < VISIBLE_ADULT_HOGLIN_COUNT`) makes it retreat (`AVOID_TARGET` for 5-20 s, no hunting for a while) and broadcast the retreat.
  - Otherwise `maybeRetaliate`.
- **`maybeRetaliate`:**
  - Not while `AVOID` is active.
  - Only when the attacker is attackable ignoring line of sight.
  - Not when the current target is much closer (`isOtherTargetMuchFurtherAwayThanCurrentAttackTarget(4.0)`).
  - A player attacker with `universal_anger` gets the nearest visible targetable player plus `broadcastUniversalAnger`. Otherwise it gets `setAngerTarget` plus `broadcastAngerTarget`.
- **`setAngerTarget`:**
  - Only for attackable targets.
  - It erases `CANT_REACH_WALK_TARGET_SINCE` and sets `ANGRY_AT` = UUID for 600 ticks.
  - A hoglin target while it can hunt also sets `dontKillAnyMoreHoglinsForAWhile`.
  - A player target with `universal_anger` also sets `UNIVERSAL_ANGER` for 600.
- **`broadcastAngerTarget`** goes to every `NEARBY_ADULT_PIGLINS` (brutes included) through `setAngerTargetIfCloserThanCurrent`. Hoglin targets are skipped for piglins that cannot hunt it.
- **`angerNearbyPiglins(player, requireSight)`:**
  - every `Piglin` (not brutes) in the player's box inflated by 16;
  - that is in `IDLE`;
  - and, if `requireSight`, that `BehaviorUtils.canSee` the player;
  - gets `setAngerTarget(player)`. With `universal_anger` it gets the nearest targetable player instead.
  - There is no broadcast.
  - Callers:
    - `Block.playerWillDestroy` for `#guarded_by_piglins` (copper chests, gold block, barrel, chest, ender chest, gilded blackstone, trapped chest, raw gold block, shulker boxes, gold ores), with `requireSight = false`;
    - `ChestBlock`, `BarrelBlock`, `ShulkerBoxBlock` and `EnderChestBlock` `useWithoutItem` when the menu opens, with `requireSight = true`;
    - `ContainerEntity.chestVehicleDestroyed`, with `true`.

**Sounds:**
- `getSoundForActivity`:
  - `FIGHT`: `entity.piglin.angry`;
  - converting: `entity.piglin.retreat`;
  - `AVOID` within 12 of the avoid target: `retreat`;
  - `ADMIRE_ITEM`: `admiring_item`;
  - `CELEBRATE`: `celebrate`;
  - seeing a player hold a loved item: `jealous`;
  - near a repellent: `retreat`;
  - otherwise `ambient`.
- `AbstractPiglin.playAmbientSound` plays only while `IDLE` (`Mob`'s 80-tick ambient interval).
- Hurt and death: `entity.piglin.hurt/death`. Step: `entity.piglin.step` at volume 0.15.
- Brute:
  - ambient, hurt, death and step (0.15) under `entity.piglin_brute.*`;
  - `maybePlayActivitySound` plays `entity.piglin_brute.angry` in `FIGHT` with `nextFloat() < 0.0125` each tick, and on entering `FIGHT`.

### 3.5 The brute brain (`PiglinBruteAi`)
- **CORE** (0): `LookAtTargetSink(45, 90)`, `MoveToTargetSink`, `InteractWithDoor` and `StopBeingAngryIfTargetDead`.
- **IDLE** (10):
  - `StartAttacking(findNearestValidAttackTarget)`: `ANGRY_AT` if attackable, else the visible attackable player (any armour), else the nemesis.
  - Look: a `RunOne` of player, piglin, brute or anything at 8, or `DoNothing(30, 60)`.
  - Movement: a `RunOne` of `stroll(0.6)` (2), `InteractWith` piglin and brute at 8, 0.6, 2 (2 each), `StrollToPoi(HOME, 0.6, 2, 100)` (2), `StrollAroundPoi(HOME, 0.6, 5)` (2) and `DoNothing(30, 60)` (1).
  - `SetLookAndInteract(PLAYER, 4)`.
- **FIGHT** (10): `StopAttackingIfTargetInvalid` (not the nearest valid one), `SetWalkTargetFromAttackTargetIfTargetOutOfReach(1.0)` and `MeleeAttack(20)`.
- **`wasHurtBy`:** `maybeRetaliate` with the `PiglinAi` rules, unless the attacker is an `AbstractPiglin`.
- **Other:**
  - `canHunt` is false, and it never barters or admires.
  - `wantsToPickUp` accepts only a golden axe.
  - Arm pose: melee when aggressive and holding a weapon, else the default.

### 3.6 Zombification (`AbstractPiglin.customServerAiStep`)
- `isConverting`: not immune to zombification, not no-AI, and the environment attribute `gameplay/piglins_zombify` at its position is true. In 26.3 that is the Overworld and the End; the Nether sets it false.
- Converting: `timeInOverworld++`. Not converting: it resets to 0.
- When `timeInOverworld > 300` (`CONVERSION_TIME`, so on the 301st tick):
  - the converted sound plays, unless peaceful;
  - `Piglin.finishConversion` runs: `cancelAdmiring` drops the offhand item, the whole inventory drops, then `convertTo(ZOMBIFIED_PIGLIN, ConversionParams.single(mob, keepEquipment = true, preserveCanPickUpLoot = true))`;
  - the new zombified piglin gets `NAUSEA` for 200 ticks, amplifier 0.
- Brutes do the same with `entity.piglin_brute.converted_to_zombified`.
- While converting, the renderer shakes it: `isShaking` adds `cos(ageInTicks × 3.25) × π × 0.4` degrees to the body yaw.

### 3.7 Model and renderer
- **`PiglinRenderer` textures:**
  - `entity/piglin/piglin_brute` for brutes;
  - `piglin_baby` for babies;
  - `piglin` otherwise.
- **`PiglinModel.setupAnim`** runs the humanoid walk, then by `PiglinArmPose`:
  - `DANCING`: ears, head bob, arms swing and body bob over `ageInTicks / 60`.
  - `ATTACKING_WITH_MELEE_WEAPON`, not swinging: weapon arm `xRot` -1.8.
  - `CROSSBOW_HOLD`: `animateCrossbowHold`.
  - `CROSSBOW_CHARGE`: `animateCrossbowCharge` over `ticksUsingItem` of 25.
  - `ADMIRING_ITEM`: head `xRot` 0.5 and `yRot` 0, off arm `yRot` ±0.5 and `xRot` -0.9.
  - `setupAttackAnimation` uses `swingWeaponDown` when the pose is melee.
- **`Piglin.getArmPose` order:** dancing, then a loved item in the offhand (admiring), then aggressive with a melee weapon, then charging, then holding a charged crossbow, then default.

## 4. The plan (one commit per group, in order)

New logic goes into vendored `third_party/minecraftoss/entities` as new modules (MinecraftOSS patches, listed in the track log). New files must pass `rustfmt --check`. No comments anywhere. Edits to existing files change only the lines they need.

State lives in two places:
- the `EntityWorld` of the `Level` the piglin is in (one per `ServerSim`);
- per-player inputs keyed by player id, like the existing `mob_views`.

There is no new global or "player 0" state. The existing `PLAYER` constant in `minecraft_survival/src/entities.rs` is used only where that file already uses it.

**Prerequisites.** This brief runs after b3 and b2 in track B. It builds on what they land and must not add a second copy:
- b3 group 1: right-click on mobs (`would_interact`, `Command::MobAction.player`, applied `MobResult.slots`). Group 5 needs it.
- b3 group 3: `PlayerControls` in `entities/src/controls.rs`, `TickInput.controls` and `EntityWorld::set_player_controls`. Group 3 extends it.
- b3 group 5: the passenger relation (striders only).
- b2 commit 1: `MobEffect::Nausea` and `MobCandidate.baby`. b2 commit 5: the registry-backed `MobWorld::block_in_tag`. b2 commit 7: `Level::piglins_zombify` and `EntityWorld::set_piglins_zombify`.
- b2's brain pieces (`hoglin_brain.rs`: sensor clocks, `MeleeAttack`, `StartAttacking`, `StopAttackingIfTargetInvalid`, `SetWalkTargetAwayFrom`, the walk to the attack target, the Manhattan repellent scan). Move what both brains use into a shared module (for example `entities/src/brain_behaviors.rs`) in the group that first needs it here, and leave the hoglin's behaviour unchanged.
- The hoglin kind itself, so group 8 lands with the rest.

### Group 1. "Minecraft piglin bartering rolls from the vanilla loot table"
- **Files:**
  - `third_party/minecraftoss/entities/src/loot.rs`:
    - load `data/minecraft/loot_table/gameplay/` into `tables` as `minecraft:gameplay/<id>`;
    - add `EntityLootBook::roll_barter(&mut self) -> Option<Vec<ItemStack>>`, which accepts only `type: minecraft:barter` and uses the table's `random_sequence` stream like `roll`;
    - support `minecraft:enchant_randomly` in `create`: `options` as one id, a list or a `#tag`. A book gets no compatibility filter; other items keep only enchantments that support them. One `next_int(len)` draw, then `next_int(max - min + 1) + min`. A book becomes `minecraft:enchanted_book` with `minecraft:stored_enchantments`; other items get `minecraft:enchantments`. The soul speed levels (min 1, max 3) come from `data/minecraft/enchantment/soul_speed.json`, read from the jar like the tables.
- **Tests** (in `loot.rs`, gated on the jar like the other loot tests):
  - `the_barter_table_weighs_469_and_every_entry_can_come_out` (20 000 seeded rolls; obsidian at about 8.5%, ender pearls at about 2.1%; every count stays in range);
  - `a_soul_speed_book_from_bartering_is_an_enchanted_book_of_level_one_to_three`;
  - `bartered_iron_boots_carry_soul_speed`;
  - `fire_resistance_potions_and_splash_potions_keep_their_contents`;
  - `an_entity_roll_never_reads_the_barter_table`.

### Group 2. "Minecraft piglins and piglin brutes walk the Nether"
- **New `third_party/minecraftoss/entities/src/piglin.rs`:**
  - `PiglinKind { Piglin, Brute }` and `Piglin` state: `body`, health, `baby`, `kind`, `armor: [Option<String>; 4]`, `main_hand`, `offhand`, `inventory: [Option<ItemStack>; 8]`, `cannot_hunt`, `immune_to_zombification`, `time_in_overworld`, `persistence_required`, `can_pick_up_loot`, `left_handed`, `charging_crossbow`, `dancing`, `aggressive`, `tick_count` and the drop chances;
  - the attributes and dimensions of §3.1;
  - `arm_pose()` per §3.7.
- **New `third_party/minecraftoss/entities/src/piglin_brain/` (`mod.rs`, `memory.rs`, `sensors.rs`, `behaviors.rs`):**
  - It reuses `villager_brain::Slot` (already exported at `villager_brain/mod.rs:22`), `seen_item` and b2's shared brain pieces.
  - Write a lean `PiglinCtx` (the piglin, the `World`, a seen list, the level random, game time and an outbox of world changes, like `Remote`). Do not reuse the villager-shaped `Ctx`/`Me`.
  - This group brings in:
    - `Activity` {Core, Idle, AdmireItem, Fight, Celebrate, Avoid, Ride};
    - `update_activity`;
    - the CORE look, move and door behaviours: port `InteractWithDoor`/`door_halves` as free functions, or call them if they generalise without touching villager behaviour;
    - the IDLE look and stroll `RunOne`s;
    - the brute's `StrollToPoi`/`StrollAroundPoi(HOME)`;
    - `NearestLivingEntitySensor` and `PlayerSensor` with the follow range, reading `mob_candidates()` with b2's `baby` flag (fill it for piglins in the same `add` closure).
  - The other behaviours come in later groups.
- **New `third_party/minecraftoss/entities/src/world/piglins.rs`:**
  - `PiglinEntity`, `spawn_piglin`, `piglins()` and `piglin_mut()`;
  - the tick: brain, then movement through `GroundNavigation` with `can_open_doors` and the fire malus;
  - hurt, death drops: no loot-table pools, equipment by drop chance (picked-up items always drop), the inventory, and experience per §3.1;
  - hurt, death, step and ambient sounds.
- **`third_party/minecraftoss/entities/src/world.rs`:** `EntityKey::Piglin`, `MobHit::Piglin`, storage, `entity_type` (`minecraft:piglin`/`minecraft:piglin_brute`), the tick order, `pushing.rs`, `combat.rs` (`mob_body`, knockback, `hurt_by_player` without anger yet) and `hazards.rs` (`exposed!`, not fire-immune). Also `lib.rs`.
- **`crates/minecraft_terrain/src/server_mobs.rs`:**
  - both ids go into `SIMULATED_KINDS`;
  - a `spawn_saved` arm reads `IsBaby`, `CannotHunt`, `Inventory`, `IsImmuneToZombification`, `TimeInOverworld`, `CanPickUpLoot`, `equipment`, `drop_chances` and `Brain.memories` (`hunted_recently`, `angry_at`, `universal_anger`, `admiring_item`, `admiring_disabled`, `home`, each with its `ttl`);
  - a matching `mob_tags` block writes the same back.
- **Client:**
  - `client_mobs.rs`: a push in `server_mobs()`, and shadows at radius 0.5, scaled for babies.
  - `crates/minecraft_survival/src/entities.rs`: `boxes`, `meshes`, `encode`/`decode` with key 21 (20 is `DRAGON_KEY`).
  - `death.rs`: `entity.minecraft.piglin` "Piglin" and `entity.minecraft.piglin_brute` "Piglin Brute".
  - `mesh.rs`: the three textures.
- **`crates/minecraft_terrain/src/piglin_render.rs`:**
  - `append_piglins` with humanoid (not zombie) arms, the default and melee arm poses, the main-hand item, and the offhand item through a new `cow_render::left_hand_pose` that mirrors `right_hand_pose`;
  - adult armour as for zombified piglins;
  - the brute uses the adult mesh and its own texture.
- **Tests:**
  - `piglin.rs`: `a_piglin_baby_is_049_by_098_and_faster`; `brutes_have_fifty_health_and_twelve_follow_range`.
  - `server.rs`: `piglins_and_brutes_load_live_and_save_every_field` (a round trip of all the fields above, the brain memories with their ttl); `a_bastion_s_piglins_wake_up` (the bastion test's piglins are no longer dormant).
  - `piglin_render.rs`: `a_piglin_and_a_brute_use_their_textures_and_the_piglin_mesh` (vertex counts as the zombified test); `an_admiring_piglin_holds_its_item_in_the_left_hand`.
  - `entities.rs`: `piglin_keys_round_trip`.

### Group 2b. "Minecraft piglins spawn in nether wastes, crimson forests and bastions"
Split from group 2 so that commit stays a size one agent can finish.
- `third_party/minecraftoss/world/src/natural_spawner.rs`: `type_info` gets `"piglin" => (0.6, 1.95, OnGround, MotionBlockingNoLeaves, false, false)` with a new `Rules::Piglin` (the block below is not `minecraft:nether_wart_block`) and the monster walk.
- `natural_spawner/spawning.rs`: a piglin arm in `finalize` per §3.2 (weapon roll from the entity's own random, as `createSpawnWeapon` uses `this.random`; baby and armour rolls from the level random), and a brute arm (golden axe, `home` at its block, dimension from the level).
- `third_party/minecraftoss/generator/src/feature/entities.rs`: `finalize_piglin_brute` beside `finalize_piglin`, for the bastion template brutes.
- **Tests:**
  - `spawning.rs`: `a_piglin_spawn_rolls_a_baby_one_in_five_and_a_crossbow_for_half_the_adults` (10 000 seeded finalizes: babies about 20%, of the adults crossbows about 50%, spears about 5%, swords about 45%, each golden armour piece about 10%); `piglins_do_not_spawn_on_nether_wart_blocks`.
  - `generator entities.rs`: `a_bastion_brute_gets_a_golden_axe_and_its_home`.

### Group 3. "Minecraft piglins attack players who wear no gold"
- **Per-player gear:** extend b3's `PlayerControls` (`entities/src/controls.rs`, carried in `TickInput.controls` and stored by `EntityWorld::set_player_controls`), do not add a second per-player vector.
  - `PlayerControls` gains `armor: [Option<String>; 4]` (feet, legs, chest, head), filled by `Entities::tick` from the inventory slots 36-39 next to b3's `main_hand` and `offhand`.
  - The piglin brain tests `#minecraft:piglin_safe_armor` on the armour and `#minecraft:piglin_loved` on both hands through the entity world's recipe book (`RecipeBook::item_in_tag`, `world.rs:2304`), so the tags are read once, on the server.
  - `PlayerCandidate` is **not** extended, because it is built as a struct literal in about 60 places.
- **Game rules:** `EntityWorld::set_universal_anger`/`set_forgive_dead_players`, from `apply_game_rules` (`server.rs:2013`).
- **Brain:**
  - `PiglinSpecificSensor` and `PiglinBruteSpecificSensor`, except the hoglin and repellent memories;
  - `findNearestValidAttackTarget` for both kinds;
  - `StartAttacking` (adults only);
  - FIGHT: `StopAttackingIfTargetInvalid`, `BackUpIfTooClose(5, 0.75)` with a crossbow, walk-to-target at 1.0, `MeleeAttack(20)` and `CrossbowAttack` (§3.4: 25 ticks of charge, a delay of 20-39, `spawn_arrow` with a mob shooter, power 1.6, uncertainty `14 - 4 × difficulty`, no crit, the crossbow sounds);
  - CELEBRATE with `StartCelebratingIfTargetDead(300)` and dancing;
  - `StopBeingAngryIfTargetDead` with `forgive_dead_players`;
  - `wasHurtBy`, `maybeRetaliate`, `setAngerTarget` (600), `broadcastAngerTarget` and the universal anger variants;
  - babies flee their attacker for 100 ticks;
  - the activity sounds and the brute's 0.0125 angry sound.
- **Hurt hooks:**
  - `combat.rs:234` (`hurt_by_player`) and `world.rs:2689` (`player_blast`, MW2 bullets and grenades) call `piglin_was_hurt_by(id, Target::Player(player))`;
  - mob melee and arrow hits on a piglin call it with `Target::Mob`.
- **Axes disable shields** (§3.1): `PlayerHitKind::Melee` (`world.rs:569-584`, which b2 already extends with `throw`) gains `disable_shield_seconds: f32`, taken from the attacker's main-hand item (5 for any axe, from the item's `minecraft:weapon` component, else 0). When it is above 0 and the hit lands on a raised shield, the adapter's shield code (`minecraft_world.rs:1803-1822`) puts the shield on a 5-second cooldown and plays `item.shield.break`. This covers brutes and any other mob holding an axe.
- **Nemesis:**
  - `monster_ai.rs:1186`: the `Prey::Absent` slot becomes `Prey::Piglin`, so wither skeletons hunt piglins and brutes.
  - `is_enemy` (`monster_ai.rs:326`) gains both ids, so iron golems attack them.
- **Death messages:** "was slain by Piglin" and "was shot by Piglin", using the existing mob attack kinds.
- **Renderer:** the crossbow charge and hold poses, the melee raised arm and `swingWeaponDown`, the dancing pose and aggressive ears.
- **Tests:**
  - `piglin_brain` unit tests:
    - `one_golden_piece_makes_a_player_safe`, with each of the four pieces alone, and a chainmail helmet not;
    - `a_creative_or_spectator_player_is_never_a_target`;
    - `the_nearest_valid_target_order_is_zombified_angry_universal_nemesis_then_ungilded`;
    - `a_baby_never_starts_attacking_and_flees_its_attacker_for_100_ticks`.
  - `world.rs`:
    - `a_hit_angers_the_piglin_and_its_adult_neighbours_for_600_ticks`: one 10 blocks away joins, one 20 blocks away does not; a brute nearby joins;
    - `a_brute_attacks_a_player_in_golden_armour_and_hits_for_13_on_normal`;
    - `a_brute_s_axe_disables_a_raised_shield_for_five_seconds`;
    - `a_crossbow_piglin_charges_25_ticks_waits_20_to_39_and_shoots_a_non_critical_arrow`;
    - `a_crossbow_piglin_backs_off_inside_five_blocks`;
    - `anger_ends_after_its_target_dies_with_forgive_dead_players`;
    - `wither_skeletons_and_piglins_fight`;
    - `iron_golems_attack_piglins`.
  - `server.rs`: `the_gear_of_each_player_reaches_the_piglins` (two `PlayerControls` entries, one gilded and one not: only the second is targeted).

### Group 4. "Minecraft piglins avoid soul fire, zombified piglins and wither skeletons"
- **Brain:**
  - `NEAREST_REPELLENT` from `findBlocksInBoxByManhattanDistance(8, 4)`, with lit soul campfires only. It reads `#minecraft:piglin_repellents` through b2's registry-backed `MobWorld::block_in_tag` and reuses b2's Manhattan scan; without that override the tag is always false;
  - `avoidRepellent` in IDLE and CELEBRATE;
  - `NEAREST_VISIBLE_ZOMBIFIED`, `isNearZombified` (6), `avoidZombified` (5-7 s);
  - `babyAvoidNemesis` (5-7 s);
  - the AVOID activity: walk away to 12 at 1.0, with the idle look and stroll;
  - `wantsToStopFleeing`;
  - `EraseMemoryIf(isNearZombified, ATTACK_TARGET)`;
  - the retreat sounds.
- **Tests:**
  - `a_piglin_walks_away_from_a_soul_torch_within_eight_and_ignores_a_normal_torch`;
  - `an_unlit_soul_campfire_repels_nobody`;
  - `a_piglin_runs_from_a_zombified_piglin_within_six_and_drops_its_target`;
  - `a_baby_runs_from_a_wither_skeleton_and_an_adult_attacks_it`.

### Group 5. "Minecraft piglins admire gold and barter"
- **Brain:**
  - `NearestItemSensor` (32/16/32, sight, `wantsToPickup`);
  - `StartAdmiringItemIfSeen(119)`, `GoToWantedItem(1.0, 9)`, `StopAdmiringIfItemTooFarAway(9)`, `StopAdmiringIfTiredOfTryingToReachItem(200, 200)` and `StopHoldingItemIfNoLongerAdmiring`;
  - `pickUpItem`, `stopHoldingOffHandItem`, `wantsToPickup` and `canReplaceCurrentItem`. The `Mob.canReplaceCurrentItem` and `canReplaceEqualItem` rules come from javap: armour by defense and toughness, weapons by attack damage, then durability and enchantments. The piglin's loved and preferred-weapon override comes first;
  - eating with `ATE_RECENTLY` (200);
  - nuggets into the inventory;
  - `ADMIRING_DISABLED` (400) on a player hit;
  - the jealous look (`SetEntityLookTarget(isPlayerHoldingLovedItem, 14)`) and sound;
  - persistence when holding non-ingot items.
- **Pick-up:** `piglin_pick_up` in `world/piglins.rs`, following `villager_pick_up`. It needs `mob_griefing` and `can_pick_up_loot`, with reach (1, 0, 1).
- **Barter:**
  - When an adult's admired gold ingot runs out, the entity world queues `BarterThrow { piglin, from: eye - 0.3, toward: Option<DVec3> }`. `toward` is the nearest visible player's feet plus (0, 1, 0), else a `LandRandomPos(4, 2)` spot, else none, which throws at its own position.
  - `ServerSim::tick_mobs` (`server.rs:691`, beside `drop_death_loot`) rolls `entity_loot.roll_barter()` for each throw and spawns each stack with `level.spawn_item`: velocity 0.3 along the normalised way, pickup delay 10.
- **Right-click** (needs b3 group 1, which makes the host send mob uses at all):
  - `mob_actions::interact` (`mob_actions.rs:129`) gets a `MobHit::Piglin` arm: `canAdmire` with a gold ingot in the hand being tried uses one (none in creative), sets the offhand and `ADMIRING_ITEM` 119, and stops walking. The outcome is handled.
  - `mob_actions::would_interact` gets the same arm, so the host consumes the click only when `canAdmire` holds and lets it fall through otherwise. b3's `would_interact_agrees_with_interact` table gains the piglin rows.
  - Brutes pass.
- **Renderer:** `ADMIRING_ITEM` (head and off arm), and the offhand item drawn in the left hand.
- **Tests:**
  - `world.rs`:
    - `a_dropped_gold_ingot_is_admired_for_119_ticks_then_bartered_toward_the_player`;
    - `a_right_clicked_ingot_is_used_and_admired`;
    - `a_baby_keeps_the_gold_it_admires_and_never_barters`;
    - `a_piglin_eats_a_porkchop_and_takes_the_next_after_200_ticks`;
    - `gold_nuggets_go_whole_into_the_inventory_and_drop_on_death`;
    - `a_hit_while_admiring_keeps_the_ingot_unbartered_and_disables_admiring_for_400_ticks`;
    - `babies_ignore_leather`;
    - `a_piglin_gives_up_on_an_unreachable_ingot_after_200_ticks`;
    - `no_mob_griefing_no_pickup`.
  - `server.rs`: `a_barter_throws_one_roll_of_the_table_at_the_player` (with the loot book loaded from the jar).
  - `mob_actions.rs`: `a_brute_ignores_a_gold_ingot`.

### Group 6. "Minecraft piglins guard gold blocks and chests"
- **`EntityWorld::anger_nearby_piglins(player: u64, player_box, require_sight: bool)`:** `Piglin` kind only, in `IDLE`, within the player's box inflated by 16, with line of sight when asked (`sight.rs`), `setAngerTarget` or its universal variant, and no broadcast.
- **Server command:** `Command::AngerPiglins { player: u64, feet: [f64; 3], sight: bool }` (`server.rs:2189`), sent by `Entities::anger_piglins`.
- **Callers:**
  - `crates/render_anim/src/minecraft_world.rs:1613`, after `entities.mined`: any hand-mined block (`HowBroken::Mined`/`SilkTouched`) in `#minecraft:guarded_by_piglins` (block tags from the registries), with `sight = false`.
  - `minecraft_world.rs:2027`: an `OpenerEvent` with `Change::Opened`, or any open of a chest, trapped chest, copper chest, barrel, shulker box or ender chest, with `sight = true`. Opening an already-open chest still angers, as vanilla's `useWithoutItem` does each time.
  - Both pass the local player's id and feet.
  - Chest boats and chest minecarts do not exist in the game, so `chestVehicleDestroyed` has no caller.
- **Tests:**
  - `world.rs`: `mining_a_gold_block_angers_idle_piglins_within_16_even_unseen`; `opening_a_chest_angers_only_the_piglins_that_see_it`; `brutes_and_busy_piglins_are_not_angered_by_a_chest`.
  - `minecraft_survival`: `only_guarded_blocks_send_anger` (gold block, raw gold block, gilded blackstone, gold ores and containers yes; stone and iron block no).

### Group 7. "Minecraft piglins turn into zombified piglins outside the Nether"
- **The rule per level:** reuse b2 commit 7's `Level::piglins_zombify()` and `EntityWorld::set_piglins_zombify` (fed each tick by `ServerSim::tick_mobs`, `server.rs:660`). Do not add a second flag.
- **Nausea:** use b2 commit 1's `MobEffect::Nausea`; `MobEffect` (`effects.rs:17`) has no nausea before it.
- **Conversion:**
  - `time_in_overworld` counts per §3.6 and resets when not converting.
  - Past 300: the converted sound unless peaceful; for a piglin, `cancelAdmiring` and inventory drops.
  - `convert_piglin_to_zombified` follows `convert_zombie_to_drowned` (`world.rs:3291`): a `ZombieKind::ZombifiedPiglin` with the same position, motion, baby flag, armour, main hand, persistence and `can_pick_up_loot`, plus `MobEffect` nausea 200 ticks, amplifier 0.
- **Activity sound:** retreat while converting.
- **Renderer:** the converting shake per §3.7 (`converting` travels in the snapshot).
- **Tests:**
  - `world.rs`:
    - `a_piglin_converts_on_the_301st_overworld_tick_with_nausea_and_its_gear`;
    - `an_immune_or_no_ai_piglin_never_converts`;
    - `the_nether_resets_the_count`;
    - `a_brute_converts_too`;
    - `the_inventory_and_admired_item_drop_on_conversion`;
    - `no_converted_sound_in_peaceful`.
  - `server.rs`: `the_end_zombifies_and_the_nether_does_not` (from the real 26.3 dimension types under `MINECRAFTOSS_ROOT`).

### Group 8. "Minecraft piglins hunt hoglins" (needs b2's hoglin kind)
b2 lands before this brief, so `HoglinEntity` (adult or baby, `cannot_be_hunted`) and its sensor's piglin side already exist. This group adds the piglin side and makes piglins count in b2's `HoglinSpecificSensor` (adult `Piglin` only, not brutes).
- **Brain:**
  - the hoglin memories of `PiglinSpecificSensor` (`NEAREST_VISIBLE_HUNTABLE_HOGLIN`, `NEAREST_VISIBLE_BABY_HOGLIN`, `VISIBLE_ADULT_HOGLIN_COUNT`);
  - `StartHuntingHoglin`: an adult with no `hunted_recently` and no `ANGRY_AT`, while none of its `NEAREST_VISIBLE_ADULT_PIGLINS` has `hunted_recently`. It sets anger at the hoglin, `dontKillAnyMoreHoglinsForAWhile` (30-120 s) and the broadcast. Every visible adult piglin also gets `hunted_recently`;
  - `RememberIfHoglinWasKilled`;
  - the hoglin branches of `wasHurtBy` and `wantsToStopFleeing` (outnumbered: retreat 5-20 s and `broadcastRetreat`);
  - `wantsToDance` after a hoglin kill;
  - `CannotHunt`.
- **Tests:**
  - `adult_piglins_hunt_a_visible_hoglin_and_rest_30_to_120_seconds_after`;
  - `outnumbered_piglins_retreat_from_hoglins`;
  - `a_piglin_with_cannot_hunt_leaves_hoglins_alone`;
  - `about_one_kill_in_ten_ends_in_a_dance`.

### Group 9. "Docs: Minecraft piglins, brutes and bartering"
- `docs/MINECRAFT-SURVIVAL.md`:
  - Nether mobs gains piglins, brutes, bartering, gold armour and zombification;
  - the N6 limitation (`:280`) and the dormant list (`:341`) shrink;
  - the riding and spear gaps are listed;
  - the shield line (`:315`) now says axes disable shields for 5 seconds.
- `docs/NETHER-END.md:247-248`: the piglin and brute rows.

## 5. Owner's in-game test

Start in the Nether: `IW4L_MINECRAFT_DIMENSION=the_nether` in the `.env`, survival, `/difficulty normal`. Put on nothing golden first. Kit (24 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:piglin_spawn_egg*16,minecraft:piglin_brute_spawn_egg*4,minecraft:zombified_piglin_spawn_egg*2,minecraft:wither_skeleton_spawn_egg*2,minecraft:hoglin_spawn_egg*4,minecraft:gold_ingot*64,minecraft:gold_ingot*64,minecraft:gold_nugget*32,minecraft:golden_carrot*4,minecraft:porkchop*8,minecraft:leather*8,minecraft:golden_helmet,minecraft:golden_boots,minecraft:diamond_chestplate,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:gold_block*8,minecraft:chest*2,minecraft:soul_torch*16,minecraft:cobblestone*64,minecraft:obsidian*14,minecraft:flint_and_steel
```

1. With no gold on, use 2 piglin eggs on the ground in front of you, then back off to about 10 blocks. They turn on you:
   - sword piglins run in and hit;
   - crossbow piglins back off, you hear the crossbow load, and they shoot.

   **Screenshot 1.**
2. Kill them. Put on only the golden boots and the diamond chestplate. Use 4 more eggs: they wander and ignore you. Some may be babies, which are smaller and faster. **Screenshot 2.**
3. Hold a gold ingot and stand near them: they stare at you and make the jealous sound.
4. Drop one gold ingot (Q) near a piglin. It walks over, picks it up, holds it in its left hand looking at it for about 6 seconds, then throws an item at you. **Screenshot 3.**
5. Right-click a piglin with a gold ingot (right-click on mobs comes with b3): one ingot is used and the same happens. Barter about 20 times. You get items from the barter list: obsidian, crying obsidian, quartz, string, gravel, blackstone, soul sand, nether bricks, spectral arrows, leather, fire charges, iron nuggets, ender pearls, dried ghasts, water or fire resistance potions, and rarely a soul speed book or soul speed iron boots. **Screenshot 4** (inventory).
6. Drop a porkchop: a piglin eats it and gives nothing back. Drop a golden carrot: it admires it and keeps it. Drop the gold nuggets: one piglin takes the whole stack. Drop leather near a baby: it ignores it.
7. Hit one adult piglin while wearing the golden boots. It and the adult piglins near it attack you, and the hit piglin drops nothing it was admiring. **Screenshot 5.** Back off for 30 seconds or kill them.
8. Hit a baby piglin: it runs away and the adults attack you.
9. With fresh piglins near (still wearing gold), place a gold block and mine it: the nearby idle piglins attack, even ones behind you. **Screenshot 6.**
10. Place a chest out of their sight and open it: nothing happens. Place one in their sight and open it: they attack.
11. Place a soul torch next to calm piglins: they back away from it. **Screenshot 7.** Use a zombified piglin egg near them: they run from it.
12. Use a wither skeleton egg near piglins: they fight each other. **Screenshot 8.**
13. Use 2 brute eggs. Brutes have golden axes and attack you even with gold on. Take off all armour: one hit takes about 6.5 hearts on normal. Raise the shield as one swings: the shield breaks with its sound and can't be raised again for 5 seconds. **Screenshot 9.**
14. `/locate structure minecraft:bastion_remnant`, then `/tp` there. The bastion's piglins and brutes are awake. Brutes stay near where they stood. Open a bastion chest while piglins can see you: they attack. **Screenshot 10.**
15. Use a hoglin egg near 3 adult piglins (hoglins land with b2, before this brief). They attack it, and after the kill they may dance. **Screenshot 11.**
16. Build and light a portal (obsidian, flint and steel) and go to the Overworld. Use a piglin egg and a brute egg. Both shake, and after 15 seconds they turn into zombified piglins with a groan, still holding their weapons. **Screenshot 12.**
17. Back in the Nether, anger a piglin, then save and quit within 30 seconds and restart. It is still angry at you. Bartering piglins you left holding gold keep it. **Screenshot 13.**

## 6. Risks, shared files and decisions

**Risks**
- **Size.** `PiglinAi` is the largest brain port. That is why it is split into eight code groups (2 and 2b to 8). Each group must leave piglins working; group 2 piglins only wander.
- **Hoglins (b2).** b2 lands first in track B, so group 8 has the hoglin kind. Its shared pieces (nausea, the baby flag, registry block tags, `piglins_zombify`, the brain behaviours) are reused, never copied.
- **The player and server boundary (track E):**
  - The armour rides in b3's `PlayerControls` (`TickInput.controls`), one entry per player id, like `mob_views` and `mob_vitals`. E's multi-player tick must fill it for every player in the level.
  - `Command::AngerPiglins` carries the player id. Its two callers are in `render_anim/src/minecraft_world.rs` (`:1613`, `:2027`), which E is moving out of `render_anim` (`docs/CRATE-SPLIT.md` section 6). Keep the calls to one line each so they move easily.
  - `angry_at` saves a UUID through `set_player_uuid` (`world.rs:3406`), so it works per UUID.
  - The barter throws go to the nearest visible player in the piglin's own level.
  - The new client-visible piglin state (arm pose, offhand item, dancing, converting) travels in the `tracked_mobs` snapshot. E's network messages must carry it.
- **Item tags in the entity world** need the recipe book (`set_recipe_book`). Unit tests must set one (or a stub with the piglin tags) or the gear checks are always false.
- **Older bastions.** Brutes generated in round 7 bastions have no golden axe or `home`. Vanilla never had such brutes, so they stay as saved: bare-handed, 7 damage, no home strolling. Step 14 should use a bastion that has not been visited yet.
- **Determinism.** The brains draw from the level random as the villager brain does. The barter roll uses the table's own random sequence. The barter throw now lands after the mob tick in the same server tick, not mid-tick.

**Files shared with other round 8 tracks**
- `third_party/minecraftoss/entities/src/world.rs` (`EntityKey`, `MobHit`, storage, tick order, conversions), `world/combat.rs`, `world/pushing.rs`, `world/hazards.rs` and `monster_ai.rs` (`is_enemy`, `Prey`): b2 (hoglins, zoglins, striders) and C (shulkers).
- `crates/minecraft_terrain/src/server_mobs.rs`, `client_mobs.rs`, `mesh.rs` (textures) and `crates/minecraft_survival/src/entities.rs` (boxes, meshes, keys): b2 and C.
  - **Key numbers:** 20 is `DRAGON_KEY`. Track B uses 21 (piglins and brutes), 22 (hoglins and zoglins) and 23 (striders); c1 takes 26 and 27. The merges are then a plain union.
- `third_party/minecraftoss/world/src/natural_spawner.rs` and `natural_spawner/spawning.rs` (`type_info`, `Rules`, `finalize`): b2 (hoglin, strider spawn rules).
- `third_party/minecraftoss/generator/src/feature/entities.rs`: b2 if it finalizes bastion hoglins.
- `third_party/minecraftoss/entities/src/loot.rs`: anything else that rolls loot this round (A, C).
- `crates/minecraft_terrain/src/server.rs`: `TickInput`, the `Command` enum and `tick_mobs`. These are also touched by E (multiplayer tick), A (`/tick`, the overload freeze) and C.
- `crates/render_anim/src/minecraft_world.rs`: E (player actions move out) and D (skating input).
- `crates/minecraft_terrain/src/piglin_render.rs`: b3 group 5 (the riding pose), and A's "spawners show no mob inside" may draw piglins in a cage later.
- `crates/minecraft_terrain/src/mob_actions.rs` (`interact` and `would_interact`): b3 group 1 and b2 commit 6.
- `docs/MINECRAFT-SURVIVAL.md` and `docs/NETHER-END.md`: every track.

**Decisions taken (the vanilla-like option wherever one exists)**
- **One storage.** Piglins and brutes share one storage with `PiglinKind`, as `ZombieKind` does. They share `AbstractPiglin`, the model, conversion and saving. The brain branches on the kind.
- **A piglin brain module of its own.** It reuses `Slot`, `seen_item`, the door and throw helpers. The villager brain is not touched, so villagers cannot regress.
- **Armour in b3's `PlayerControls`**, not in `PlayerCandidate` and not in a second per-player vector. This avoids editing about 60 struct literals, and fits E's per-player inputs.
- **Golden spears.** Spears have no use in the game yet, so a spear piglin melees like a sword piglin. The zombified piglin port did the same. The 1-in-10 spear roll is kept, so the item and its drop are right.
- **No riding.** b3 group 5 brings a passenger relation for striders only. Extending it to piglins on hoglins (the RIDE activity, `babySometimesRideBabyHoglin`, stacks of up to 3) is left out and listed as a gap. `isBabyRidingBaby` is always false. b3 group 5 also gives `piglin_render.rs` a riding pose for zombified piglins on striders; keep it when this brief reworks the file.
- **Crossbow model.** A held crossbow draws its plain model; the arm poses show charging and aiming. If the item-model `charge_type` select gets evaluated later, the charged model follows for free.
- **Containers anger every time they open,** as `useWithoutItem` does. The check runs where the client opens the container, with the server doing the line-of-sight test.
- **Chest vehicles** are not in the game, so they have no hook.
- **Anger saving.** `angry_at`, `universal_anger`, `admiring_item`, `admiring_disabled`, `hunted_recently` and `home` are saved with their ttl, as vanilla's codecs do. That is unlike round 7's zombified piglins, which keep only the anger time.
- **Peaceful.** Piglins exist in peaceful (they are already in `ALLOWED_IN_PEACEFUL`) and brutes do not. Their hits on players scale with difficulty through the existing path.
