# Track B, item b2: hoglins and zoglins (design brief)

Paths and line numbers are as of e2313625. Vanilla facts were checked with javap on the 26.3 `client.jar` and with the 26.3 data under `iw4l-artifacts\minecraft-26.3`. No Mojang code is copied; this brief only describes behaviour and numbers.

## 1. The owner's request

> Hoglins (and zoglins, since hoglins zombify in the Overworld): vanilla HoglinAi (attack, knockback throw upward, avoid warped fungus, nether portals and respawn anchors, breeding with crimson fungus, babies, zombification into zoglin after 300 ticks outside the Nether, cooked porkchop and leather drops), zoglin (attacks everything), renderers and sounds.

All state lives on the mob in its `Level`'s `EntityWorld` and is saved in the mob's own tag. Nothing here is global or belongs to player `0`. Section 6 lists where the change meets the per-player boundary that track E is rebuilding.

## 2. What exists today

| Area | State |
|---|---|
| Simulated kinds | Hoglin and zoglin are missing from `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:389-420`). `spawn_saved` (`:424`) falls through to `_ => return None` (`:931`), so every hoglin is dormant: kept, saved and counted, but it never moves, is never drawn and can't be hit (`docs/MINECRAFT-SURVIVAL.md:341`, `docs/NETHER-END.md:249-250`). |
| Entity world | No hoglin storage. The places to wire into are the `EntityWorld` fields (`third_party/minecraftoss/entities/src/world.rs:2267`), `EntityKey` (`:1877`), `MobHit` (`:1929`), `entity_type` (`:3932`), `mob_candidates` (`:3459`, whose `MobCandidate` at `monster_ai.rs:304` has no baby flag), `mob_hits_mob` (`:3525`), `census` (`:4658`) and `check_despawn` (`:4703`). |
| Conversion pattern | `convert_zombie_to_drowned` (`world.rs:3291`) discards the old mob and spawns a new one with copied state. |
| Monster goals | `MonsterKind` (`monster_ai.rs:36`) is the goal framework. Hoglins and zoglins use vanilla's `Brain`, like villagers. `villager_brain` exports `Slot`, `Memories` and `WalkTarget` (`villager_brain/mod.rs:22`), but its behaviours are private and tied to villagers. |
| Iron golem targets | `is_enemy` (`monster_ai.rs:326-347`) lacks hoglin and zoglin. Both are `Enemy` in vanilla, so golems should attack them. |
| Fire immunity | The zoglin is already listed as fire immune (`world/hazards.rs:43`; test at `:327-330`). The hoglin is not fire immune, which is correct. |
| Breeding | `animal::interact` (`animal.rs:12`) feeds an animal and sets `InLove` to 600. `breed.rs` is the cow family's `BreedGoal`. Vanilla's hoglin uses `AnimalMakeLove` instead, which has other timings. `age.rs` has `AgeableMob` aging. |
| Mob use | `mob_actions::interact` (`crates/minecraft_terrain/src/mob_actions.rs:129`): the pig arm (`:174`) feeds from `#pig_food` through the recipe book's item tags. **It is never reached in game:** the host only sends attacks (`minecraft_survival/src/entities.rs:543`, `:778`) and never applies `MobResult.slots`. b3 group 1 ("Minecraft players use items on mobs") wires right-click on mobs and adds the host predicate `mob_actions::would_interact`. Feeding hoglins (commit 6) needs it. |
| Effects | `MobEffect` (`effects.rs:17`) has no `Nausea`, which a new zoglin gets. |
| Block tags | `MobWorld` (`server_mobs.rs:167`) does not override `World::block_in_tag`, so mobs see only the hand-written tags in `authored_block_in_tag` (`third_party/minecraftoss/player/src/lib.rs:1011-1040`). `#hoglin_repellents` is not among them. |
| Level attribute | `gameplay/piglins_zombify` is declared, default `true` (`third_party/minecraftoss/core/src/environment.rs:146`). `the_nether.json` sets it to false. Nothing reads it. The pattern to copy is `Level::monsters_burn` (`world/src/level/time.rs:227`), which `ServerSim::tick_mobs` hands to the entity world every tick (`crates/minecraft_terrain/src/server.rs:660`). |
| Natural spawning | `MobCategory::of_type` already makes both monsters (`world/src/natural_spawner/spawning.rs:100-101`). `type_info` has no hoglin entry (`natural_spawner.rs:264-337`), so the crimson forest's hoglins (weight 9, groups of 3-4) never spawn. |
| Structures | Bastion hoglin stables place `bastion/mobs/hoglin.nbt`. Its hoglins have `PersistenceRequired: 1b` (checked in the NBT). `finalize_spawn` (`generator/src/feature/entities.rs:113-135`) has a piglin step but no hoglin baby roll. |
| Hits on players | `PlayerHitKind::Melee { attacker, hunger_ticks, lift }` (`world.rs:569-584`). `Entities` keeps only player `0`'s hits (`crates/minecraft_survival/src/entities.rs:36`, `:992-1015`) and reads only `attacker`, so the iron golem's `lift` and the husk's `hunger_ticks` are dropped today. Pushes from the server (`players_pushed`, used by the dragon's wings) reach the player through `entities.rs:923` and `render_anim/src/minecraft_world.rs:1850-1854`, using `impulse::launch_velocity`. The shield and hurt loop is at `minecraft_world.rs:1803-1822`. |
| MW2 hits | `Entities` key encoding: `encode` (`entities.rs:1350`), `decode` (`:1374`), and the keys 0-20 in use (`:1399-1401`). `boxes` is at `:1084`, `meshes` at `:1179` (ghast call at `:1231`). Shots and blasts use `PLAYER` (`:764-777`, `:781-798`). |
| Rendering | No hoglin model. Templates: `ghast_render.rs` and `pig_render.rs` (cuboids with `cow_render::cube_*`). Textures are registered at `minecraft_terrain/src/mesh.rs:499-501`, the client pose push is at `client_mobs.rs:1024` (ghast at `:1100`), and shadows at `:1121` (ghast at `:1140`). The pack has `textures/entity/hoglin/{hoglin,hoglin_baby,zoglin,zoglin_baby}.png`. |
| Sounds | `sounds.json` has `entity.hoglin.{ambient,angry,attack,converted_to_zombified,death,hurt,retreat,step}` and `entity.zoglin.{ambient,angry,attack,death,hurt,step}`. |
| Loot | `entities/hoglin` and `entities/zoglin` exist in the data pack. The loot engine already handles the table shape (`furnace_smelt` on `is_on_fire`, `entities/loot.rs:415`; pigs use it). |
| Spawn eggs and summon | `ServerSim::hatch_egg` (`server.rs:757`) and `summon` (`:749`) only need the kind in `SIMULATED_KINDS`. `/summon` passes NBT through. |

## 3. Vanilla 26.3 behaviour (javap)

Classes checked: `Hoglin`, `HoglinAi`, `HoglinBase`, `Zoglin`, `HoglinSpecificSensor`, `AnimalMakeLove`, `BabyFollowAdult`, `BehaviorUtils`, `Sensor`, `PlayerSensor`, `NearestLivingEntitySensor`, `EntityTypes`, `ConversionType`, `LivingEntity`, `Monster`, `HoglinModel`, `BabyHoglinModel`, `AbstractHoglinRenderer`, `HoglinRenderer`, `ZoglinRenderer` and `LivingEntityRenderer`.

### Types and attributes

- **Entity types** (`EntityTypes`):
  - Both are category `MONSTER`, sized 1.3964844 × 1.4, with passenger attachment 1.49375, client tracking range 8, and `notInPeaceful`.
  - Only the zoglin is `fireImmune`.
  - The eye height is the default 0.85 × height, so 1.19.
  - Babies of both use `BABY_DIMENSIONS`: 0.75 × 0.85, eye 0.625, passenger attachment at y 0.875.
- **Attributes** (both): max health 40, movement speed 0.3, knockback resistance 0.6, attack knockback 1, attack damage 6 (0.5 for babies), and follow range 16 (the `Mob` default).
- **Hoglin as an animal:** `Hoglin` extends `Animal` and implements `Enemy`. Its sound source is `HOSTILE`, and `removeWhenFarAway` is true.
- **Hoglin experience and loot:**
  - Experience is 5 for an adult and 3 for a baby (`ageBoundaryReached`). `shouldDropExperience` is always true, so babies give experience too.
  - `shouldDropLoot` is `LivingEntity`'s: adults only.
- **Zoglin:** extends `Monster`, with experience 5 at any age. It drops loot even as a baby (`Monster.shouldDropLoot`).
- **Baby flag:** the hoglin's is `AgeableMob` age, saved as `Age` and `ForcedAge`. The zoglin's is a synced boolean saved as `IsBaby`.
- **Spawning as a baby:** `finalizeSpawn` makes a baby when `nextFloat() < 0.2`, for every spawn reason, before the parent class's finalize. This is `HoglinBase.PROBABILITY_OF_SPAWNING_AS_BABY`.
  - The hoglin then runs `AgeableMob.finalizeSpawn`. With no group data it creates `AgeableMobGroupData(true)` (baby chance 0.05) and shares it through the spawn group. Every member after the first (`getGroupSize() > 0`) draws one more `nextFloat()` and becomes a baby when it is `<= 0.05`; then the group size goes up by one. So in a natural group of 3-4, members 2 to 4 get a second baby roll.
  - The zoglin's `finalizeSpawn` makes a baby when `nextFloat() < 0.2`, then runs `Monster`'s finalize, with no group roll.
- **Spawn rule** (`checkHoglinSpawnRules`): the block below must not be `nether_wart_block`.
- **Walk target value** (`getWalkTargetValue`): -1 within 8 blocks of the remembered repellent, 10 on crimson nylium, otherwise 0.
- **Saved fields:**
  - Hoglin: `IsImmuneToZombification` (bool), `TimeInOverworld` (int) and `CannotBeHunted` (bool), on top of `Animal`'s `Age`, `ForcedAge` and `InLove`.
  - Zoglin: `IsBaby`.

### Attack and the throw (`HoglinBase`)

- **`doHurtTarget`** (both types):
  - It sets `attackAnimationRemainingTicks = 10` and broadcasts entity event 4, on which the client also sets 10 and plays the attack sound.
  - It plays `entity.hoglin.attack` or `entity.zoglin.attack`. The hoglin also runs `HoglinAi.onHitTarget`.
  - Then it runs `hurtAndThrowTarget`.
- **`hurtAndThrowTarget`:**
  - Damage: `f = ATTACK_DAMAGE`. An adult with `(int) f > 0` deals `f / 2 + nextInt((int) f)`, which is 3 to 8 before difficulty scaling. A baby deals a flat `f`, which is 0.5.
  - The damage source is `mob_attack`, so the target's normal hurt knockback (0.4 away from the attacker) applies.
  - When the hurt lands and the attacker is an adult, the attacker runs `throwTarget`.
- **`throwTarget(attacker, target)`:**
  - `k = attacker attack_knockback (1) − target knockback_resistance`. If k ≤ 0 there is no throw, so iron golems (resistance 1.0) are never thrown, and a hoglin hit by another hoglin gets k = 0.4.
  - Horizontal push: `normalize(target − attacker on x, z) × k × (nextFloat × 0.5 + 0.2)`, turned by `Vec3.yRot((float)(nextInt(21) − 10))`.
    - `yRot` takes radians, so the turn is −10 to 10 radians.
    - This is a vanilla quirk: the sideways direction is effectively scattered. Keep it.
  - Vertical push: `k × nextFloat × 0.5`. The push is added with `push(x, y, z)` and the velocity is synced.
  - Draw order: `nextInt(21)`, then `nextFloat` for the horizontal push, then `nextFloat` for the vertical push.
- **Shields:** `blockedByItem` (both types) makes an adult throw the target even when a shield blocked the hit. Babies never throw.
- **Attack animation:** `aiStep` counts `attackAnimationRemainingTicks` down by 1 per tick.

### The hoglin brain (`HoglinAi`, `Brain`)

- **Sensors** (each ticks every 20 ticks after a random first delay; `Sensor` scan rate 20):
  - `NEAREST_LIVING_ENTITIES`: living entities in the box inflated by the follow range (16), visible ones sorted by distance.
  - `NEAREST_PLAYERS`: also sets `NEAREST_VISIBLE_ATTACKABLE_PLAYER`. That is the nearest player within 16 who is visible and attackable (`Sensor.isEntityAttackable`: not creative or spectator).
  - `NEAREST_ADULT`: the nearest visible adult of the same type.
  - `HoglinSpecificSensor`, which sets:
    - `NEAREST_REPELLENT`: the first block in `#hoglin_repellents` from `findBlocksInBoxByManhattanDistance(blockPosition, 8, 4)` (±8 horizontally, ±4 vertically, nearest by Manhattan distance first). The 26.3 tag is `warped_fungus`, `potted_warped_fungus`, `nether_portal` and `respawn_anchor`.
    - `NEAREST_VISIBLE_ADULT_PIGLIN`: the first adult piglin among the visible entities.
    - `NEAREST_VISIBLE_ADULT_HOGLINS`: the list of visible adult hoglins.
    - `VISIBLE_ADULT_PIGLIN_COUNT` and `VISIBLE_ADULT_HOGLIN_COUNT`.
    - Only non-baby `Piglin` (not brutes) and `Hoglin` instances count. The list excludes the hoglin itself.
- **Activities**: `updateActivity` picks the first valid of FIGHT (needs `ATTACK_TARGET`), AVOID (needs `AVOID_TARGET`) and IDLE.
  - When the activity changes, it plays that activity's sound: AVOID, or any activity while converting, plays `entity.hoglin.retreat`; FIGHT plays `angry`; IDLE plays `retreat` near a repellent, else `ambient`.
  - The same mapping is the ambient sound. The ambient interval is `Mob`'s 80 ticks.
  - `setAggressive` is set to whether `ATTACK_TARGET` is present.
- **CORE** (priority 0): `LookAtTargetSink(45, 90)` and `MoveToTargetSink`.
- **IDLE** (priority 10, in order):
  1. `BecomePassiveIfMemoryPresent(NEAREST_REPELLENT, 200)`: with a repellent remembered, set `PACIFIED` for 200 ticks and erase `ATTACK_TARGET`.
  2. `AnimalMakeLove(HOGLIN, 0.6, 2)`.
  3. `SetWalkTargetAwayFrom.pos(NEAREST_REPELLENT, 1.0, 8, true)`.
  4. `StartAttacking(findNearestValidAttackTarget)`: nothing while `PACIFIED` or `BREED_TARGET` is present, otherwise `NEAREST_VISIBLE_ATTACKABLE_PLAYER`. Hoglins only start fights with players.
  5. For adults only: `SetWalkTargetAwayFrom.entity(NEAREST_VISIBLE_ADULT_PIGLIN, 0.4, 8, false)`.
  6. `SetEntityLookTargetSometimes(8, 30-60)`.
  7. `BabyFollowAdult(5-16, 0.6)`: a baby whose nearest visible adult is within 17 blocks but not within 5 walks to it at 0.6 and stops 5 away.
  8. `RunOne`: `RandomStroll.stroll(0.4)` (weight 2), `SetWalkTargetFromLookTarget(0.4, 3)` (weight 2) and `DoNothing(30, 60)` (weight 1).
- **FIGHT** (priority 10, requires `ATTACK_TARGET`):
  1. `BecomePassiveIfMemoryPresent(NEAREST_REPELLENT, 200)`.
  2. `AnimalMakeLove(HOGLIN, 0.6, 2)`.
  3. `SetWalkTargetFromAttackTargetIfTargetOutOfReach(1.0)`.
  4. `MeleeAttack(40)` for adults and `MeleeAttack(15)` for babies: within melee reach and visible, with no attack cooldown memory. It swings, hits and sets the cooldown.
  5. `StopAttackingIfTargetInvalid`.
  6. `EraseMemoryIf(isBreeding, ATTACK_TARGET)`.
- **AVOID** (priority 10, requires `AVOID_TARGET`):
  1. `SetWalkTargetAwayFrom.entity(AVOID_TARGET, 1.3, 15, false)`.
  2. The idle `RunOne`.
  3. `SetEntityLookTargetSometimes(8, 30-60)`.
  4. `EraseMemoryIf(wantsToStopFleeing, AVOID_TARGET)`, where `wantsToStopFleeing` is "adult and piglins don't outnumber hoglins".
- **Being hurt** (`hurtServer`, then `HoglinAi.wasHurtBy`, only when the source has a living entity):
  - It erases `PACIFIED` and `BREED_TARGET`.
  - A baby `retreatFromNearestTarget`: `AVOID_TARGET` becomes the nearer of the current avoid target, the current attack target and the attacker, for `RETREAT_DURATION` (5 to 20 seconds, so 100 to 400 ticks). Setting an avoid target also erases `ATTACK_TARGET` and `WALK_TARGET`.
  - An adult runs `maybeRetaliate`. It returns without doing anything when:
    - it is in AVOID and the attacker is a piglin;
    - the attacker is a hoglin;
    - the attacker is more than 4 blocks farther away than the current target (`otherDistSqr > currentDistSqr + 16`);
    - the attacker is not attackable.
  - Otherwise `setAttackTarget` erases `CANT_REACH_WALK_TARGET_SINCE` and `BREED_TARGET`, sets `ATTACK_TARGET` for 200 ticks, and `broadcastAttackTarget`s it.
- **Broadcasting the target:** `broadcastAttackTarget` gives every visible adult hoglin the target unless it is pacified, keeping the nearer of its current target and the new one.
- **`onHitTarget`** (when an adult hits):
  - If the victim is a piglin and piglins outnumber hoglins, it sets the avoid target and `broadcastRetreat` (each visible adult hoglin retreats from its nearest).
  - Otherwise it runs `broadcastAttackTarget`.
- **`piglinsOutnumberHoglins`:** false for babies, else `VISIBLE_ADULT_PIGLIN_COUNT > VISIBLE_ADULT_HOGLIN_COUNT + 1`.
- **`AnimalMakeLove`:**
  - Its maximum duration is 110 ticks. It starts when a visible hoglin it `canMate` is present.
  - On start it sets `BREED_TARGET` on both, makes them look at and walk to each other (speed 0.6, close enough 2), and picks the child time as start + 60 + `nextInt(50)`.
  - While they are within 3 blocks and that time has passed, it runs `spawnChildFromBreeding`: a baby with age −24000, both parents' age 6000 and love reset, and 1-7 experience with `mob_drops`. Then it erases `BREED_TARGET` on both.
  - `Hoglin.getBreedOffspring` sets the baby's `PersistenceRequired`.
- **Feeding:**
  - `isFood` is `#hoglin_food`, which is `crimson_fungus`.
  - `canFallInLove` requires that the hoglin is not pacified, plus `Animal`'s own rules.
  - `mobInteract` is `Animal`'s (love for 600 ticks, or a baby ages by 10 %) and makes the hoglin persistent when the item is used.
- **Peaceful and leads:** `notInPeaceful` means `Mob.checkDespawn` discards hoglins and zoglins on peaceful. `canBeLeashed` is true for both, but leads are out of scope.

### Zombification (`Hoglin.customServerAiStep`)

- **`isConverting`:** not `IsImmuneToZombification`, not `NoAI`, and the environment attribute `gameplay/piglins_zombify` at its position is true. That holds in the Overworld and the End, and not in the Nether.
- **Each tick while converting:** `timeInOverworld++`. When it is greater than 300 (`CONVERSION_TIME`), the hoglin plays `entity.hoglin.converted_to_zombified` and converts. That happens on the 301st tick, about 15 seconds.
- **When not converting:** the counter resets to 0.
- **Conversion** (`finishConversion`): `convertTo(ZOGLIN, ConversionParams.single(this, keepEquipment true, preserveCanPickUpLoot false))`.
  - `SINGLE` discards the hoglin and adds a new zoglin with its own UUID.
  - It copies the position and rotation, motion, on-ground state, fire, baby flag, persistence, NoAI, silence, invulnerable time, absorption, active effects, portal cooldown, custom name and leash. Health is not copied, so the zoglin starts at 40.
  - The zoglin then gets nausea for 200 ticks, amplifier 0.
- **Rendering:** `HoglinRenderer.isShaking` is true while converting. `LivingEntityRenderer.setupRotations` then adds `cos(ageInTicks × 3.25) × π × 0.4` degrees to the body yaw.

### The zoglin brain (`Zoglin`)

- **Sensors:** `NEAREST_LIVING_ENTITIES` and `NEAREST_PLAYERS`.
- **CORE:** `LookAtTargetSink(45, 90)` and `MoveToTargetSink`.
- **IDLE** (priority 10):
  - `StartAttacking(findNearestValidAttackTarget)`: the closest visible living entity that is not a zoglin or a creeper and is attackable. That includes players, villagers, animals, hoglins and golems.
  - `SetEntityLookTargetSometimes(8, 30-60)`.
  - The same `RunOne` stroll as the hoglin.
- **FIGHT** (priority 10, requires `ATTACK_TARGET`): `SetWalkTargetFromAttackTargetIfTargetOutOfReach(1.0)`, `MeleeAttack(40)` for adults and `MeleeAttack(15)` for babies, and `StopAttackingIfTargetInvalid`.
- **`updateActivity`:** FIGHT, else IDLE. Entering FIGHT plays `entity.zoglin.angry`. It is aggressive while it has a target.
- **Ambient sound:** `angry` while it has a target, else `ambient`.
- **`hurtServer`:** when the attacker is living, `canAttack` holds, and the attacker is not more than 4 blocks farther away than the current target, it sets `ATTACK_TARGET` for 200 ticks.
- **Other sounds:** hurt, death, and step at volume 0.15 (both types).

### Loot (`loot_table/entities/*.json`)

- **Hoglin:**
  - `porkchop` 2-4, plus 0-1 per Looting level. It is smelted into `cooked_porkchop` when the hoglin is on fire or the killer's main hand has a `#smelts_loot` enchantment.
  - `leather` 0-1, plus 0-1 per Looting level.
- **Zoglin:** `rotten_flesh` 1-3, plus 0-1 per Looting level.

### Spawning

- **Crimson forest** (`gameplay/natural_mob_spawns`, monster category): hoglin weight 9 in groups of 3-4, piglin weight 5 in groups of 3-4, and zombified piglin weight 1 in groups of 2-4.
- **Bastions:** hoglin stables place persistent hoglins from `bastion/mobs/hoglin.nbt`.
- **Zoglins** never spawn naturally.

### Model (`HoglinModel`, 128 × 64 texture; part = texture offset, box origin and size, pose)

- **Root parts:**
  - `body`: (1,1), box (−8,−7,−13) size (16,14,26), offset (0,7,0).
    - Child `mane`: (90,33), box (0,0,−9) size (0,10,19) with deformation 0.001, offset (0,−14,−7).
  - `head`: (61,1), box (−7,−3,−19) size (14,6,19), offset (0,2,−12), x rotation 0.87266463 (50°).
- **Head children:**
  - `right_ear`: (1,1), box (−6,−1,−2) size (6,1,4), offset (−6,−2,−3), z rotation −0.6981317.
  - `left_ear`: (1,6), box (0,−1,−2) size (6,1,4), offset (6,−2,−3), z rotation 0.6981317.
  - `right_horn`: (10,13), box (−1,−11,−1) size (2,11,2), offset (−7,2,−12).
  - `left_horn`: (1,13), box (−1,−11,−1) size (2,11,2), offset (7,2,−12).
- **Legs:**
  - `right_front_leg`: (66,42), box (−3,0,−3) size (6,14,6), offset (−4,10,−8.5).
  - `left_front_leg`: (41,42), same box, offset (4,10,−8.5).
  - `right_hind_leg`: (21,45), box (−2.5,0,−2.5) size (5,11,5), offset (−5,13,10).
  - `left_hind_leg`: (0,45), same box, offset (5,13,10).
- **Total:** 11 cubes.

**Baby** (`BabyHoglinModel`, 64 × 64 texture, its own layer):
- **`head`:** three cubes, offset (0,13,−7), x rotation 0.8727.
  - (0,0) box (−5,−2.2605,−10.547) size (10,4,12).
  - (44,29) box (−7,−4.0981,−8.4879) size (2,5,2).
  - (52,29) box (5,−4.0981,−8.4879) size (2,5,2).
- **`body`:** two cubes, both with deformation 0.02, offset (0,24,0).
  - (0,16) box (−4,−14,−7) size (8,8,14).
  - (24,39) box (0,−18,−8) size (0,6,11).
- **Ears** (children of `head`):
  - `right_ear`: (32,5), box (−5.1,−0.5,−2) size (6,1,4), offset (−5,−1,−1.5), z rotation −0.8727.
  - `left_ear`: (32,0), mirrored, box (−0.9,−0.5,−2) size (6,1,4), offset (5,−1,−1.5), z rotation 0.8727.
- **Legs:** box (−1.5,0,−1.5) size (3,6,3) each.
  - `right_hind_leg`: (0,47), offset (−2.5,18,4.5).
  - `left_hind_leg`: (12,47), offset (2.5,18,4.5).
  - `right_front_leg`: (0,38), offset (−2.5,18,−4.5).
  - `left_front_leg`: (12,38), offset (2.5,18,−4.5).
- **Total:** 11 cubes.

**Animation** (`HoglinModel.setupAnim`, shared by the baby):
- **Ears:** `right_ear.zRot = −0.6981317 − speed × sin(pos)` and `left_ear.zRot = 0.6981317 + speed × sin(pos)`. This replaces the baby's posed 0.8727.
- **Head:** `head.yRot` is the head yaw in radians.
- **Headbutt:** `f = 1 − |10 − 2 × attackTicks| / 10`, then `head.xRot = lerp(f, 0.87266463, −0.34906584)`. The baby also adds `f × 2.5` to `head.y`.
- **Legs:** `right_front.xRot = cos(pos) × 1.2 × speed` and `left_front.xRot = cos(pos + π) × 1.2 × speed`. `right_hind` copies `left_front`, and `left_hind` copies `right_front`.

**Renderers:**
- `AbstractHoglinRenderer` is an `AgeableMobRenderer` with shadow radius 0.7.
- `HoglinRenderer` uses `entity/hoglin/hoglin` and `hoglin_baby`, and shakes while converting.
- `ZoglinRenderer` uses `entity/hoglin/zoglin` and `zoglin_baby`.

## 4. Plan: commit groups

New mob code goes into vendored `third_party/minecraftoss/entities` as MinecraftOSS patches, in the style of the ghast and blaze work. The vendored crates are edition 2021, so no let-chains. New files must pass `rustfmt --check`, existing files are edited in their own style, and no comments may be added. Each numbered entry is one commit. `cargo check -p launcher` runs before each commit that touches game code.

### Group H1: hoglins and zoglins exist, are saved and spawn

**1. "Minecraft hoglins and zoglins live in the entity world"**
- **Files:**
  - New `entities/src/hoglin.rs`:
    - `HoglinKind { Hoglin, Zoglin }`.
    - `Hoglin` with body, health (40), `Age` (hoglin) or `baby` (zoglin), `in_love`, `attack_animation`, `time_in_overworld`, `immune_to_zombification`, `cannot_be_hunted`, `persistence_required` and `yaw`.
    - The constants from section 3: sizes, eye heights, speed 0.3, knockback resistance 0.6, damage 6 / 0.5, experience 5 / 3, `CONVERSION_TIME` 300.
    - `damage_roll` (3-8 for adults, 0.5 for babies) and `throw_push(k, random)` (the draw order from section 3).
  - New `entities/src/world/hoglins.rs`: `HoglinEntity` (id, effects, hoglin, `no_ai`, tick count, ambient time, voices, random, previous position) with `hurt`, death placement, physics, fluids and fire through the shared helpers, plus the hurt, death, step (0.15) and ambient (interval 80) voices.
  - `world.rs` wiring:
    - the `hoglins` vector, `EntityKey::Hoglin` and `MobHit::Hoglin` (one variant per storage; the kind lives in the state);
    - `entity_type`, `census`, `mob_candidates` (with a new `baby` field on `MobCandidate`, filled for every kind) and a `mob_hits_mob` victim arm that applies resistance 0.6 to the default knockback;
    - `player_attack` and `mob_blast` arms, so every `match` on `MobHit` compiles;
    - `check_despawn` rules: removable far away and not allowed in peaceful.
  - `death`/`take_deaths`: `entities/hoglin` only for adults, `entities/zoglin` for both; experience 5/3/5, only after a player hit as for other mobs.
  - `effects.rs`: `MobEffect::Nausea` (`minecraft:nausea`, no work on mobs; saved like the others).
- **Tests** (`world`):
  - `a_hoglin_and_a_zoglin_have_forty_health_and_vanilla_sizes` (adult 1.3964844 × 1.4 with eye 1.19, baby 0.75 × 0.85 with eye 0.625).
  - `a_baby_hoglin_drops_experience_but_no_loot_and_a_baby_zoglin_drops_both`.
  - `a_burning_hoglin_drops_cooked_porkchops` (with the real 26.3 tables under `MINECRAFTOSS_ROOT`: 2-4 porkchops, 0-1 leather; on fire they are cooked).
  - `hoglins_and_zoglins_leave_on_peaceful`.
  - `a_zoglin_shrugs_off_lava_and_a_hoglin_burns`.

**2. "Minecraft hoglins and zoglins are saved and spawn in crimson forests and bastions"**
- **Files:**
  - `crates/minecraft_terrain/src/server_mobs.rs`:
    - add both kinds to `SIMULATED_KINDS`;
    - `spawn_saved` arms reading `Age`, `ForcedAge`, `InLove`, `IsImmuneToZombification`, `TimeInOverworld` and `CannotBeHunted` (hoglin) and `IsBaby` (zoglin);
    - `mob_tags` blocks writing the same keys, keeping unknown keys from `originals`.
  - `natural_spawner.rs` `type_info`:
    - hoglin is `(1.396_484_4, 1.4, OnGround, MotionBlockingNoLeaves, false, false)`, with a new `Rules::Hoglin` (`!nether_wart_block` below, no difficulty test, since the monster category is already off on peaceful);
    - a new `Walk::Hoglin` (10 on crimson nylium, else 0);
    - `remove_far` true and cluster 4.
  - The finalize path (`spawning.rs`, inside `finalize_monster`, since the hoglin is in the monster category): for hoglins, `nextFloat() < 0.2` makes a baby, then `AgeableMob`'s group roll through the existing `Self::ageable_monster` with `MonsterGroup::Ageable { should_spawn_baby: true, chance: 0.05, size: 0 }` when the group is still `None`, then `finalize_mob`. For zoglins, the same 20 % roll, then `finalize_mob`.
  - `generator/src/feature/entities.rs` `finalize_spawn`: the same 20 % baby roll for structure hoglins, before `finalize_mob`.
  - `monster_ai.rs` `is_enemy`: add `minecraft:hoglin` and `minecraft:zoglin`, so iron golems target them.
- **Tests:**
  - `server`: `hoglins_and_zoglins_keep_their_age_conversion_clock_and_flags_through_a_save`.
  - `natural_spawner`: `a_crimson_forest_spawns_hoglins_in_groups_but_not_on_nether_wart_blocks` (seeded, real data).
  - `spawning`: `a_hoglin_group_rolls_one_in_five_babies_and_a_further_twentieth_after_the_first` (seeded finalizes of groups of 4: the first member about 20 % babies, the others about 24 %).
  - `terrain` or `server`: `bastion_hoglins_come_alive_and_stay_persistent` (extend `nether_fortresses_bastions_and_every_nether_feature_generate`'s bastion: its hoglins load through `spawn_saved` with `PersistenceRequired`).
  - `world`: `an_iron_golem_goes_for_a_hoglin`.

### Group H2: they look like vanilla's

**3. "Minecraft hoglins and zoglins are drawn like vanilla"**
- **Files:**
  - New `crates/minecraft_terrain/src/hoglin_render.rs`:
    - the adult and baby cuboids, texture sizes 128 × 64 and 64 × 64;
    - `setup_anim`: ears, head yaw, headbutt from `attack_animation`, and the legs from the client walk position and speed;
    - the four textures, and the converting shake from section 3, driven by a `converting` flag the server sets each tick (commit 8 fills it; until then it is false);
    - `SCALE` 1, shadow radius 0.7 (babies like `pig_render`'s babies).
  - `lib.rs` registers the module. `mesh.rs` adds the four textures next to `:499`.
  - `client_mobs.rs`: a push in `server_mobs()` with eye 1.19 or 0.625, and a `shadow_casters` entry with radius 0.7.
  - `crates/minecraft_survival/src/entities.rs`:
    - `boxes()` entries;
    - `encode` and `decode` key 22 (hoglin storage). The zoglin shares the storage, so one key is enough. Keys 0-20 are taken (20 is `DRAGON_KEY`); track B uses 21 for piglins, 22 for hoglins and 23 for striders, and c1 takes 26 and 27;
    - the `meshes()` call next to `ghast_render` (`:1231`).
- **Tests** (`hoglin_render`):
  - `an_adult_and_a_baby_hoglin_have_eleven_cubes_each` (264 vertices each).
  - `the_head_butts_up_and_back_over_ten_ticks` (x rotation 50° at 10 and 0 ticks remaining, −20° at 5; the baby's head y + 2.5 at 5).
  - `the_ears_hang_at_forty_degrees_and_flap_with_the_walk`.
  - `a_converting_hoglin_shakes_and_a_zoglin_does_not`.

### Group H3: the hoglin brain

**4. "Minecraft hoglins charge players and throw them upward"**
- **New `entities/src/hoglin_brain.rs`:**
  - Memories as `villager_brain::Slot`: `attack_target` (with expiry), `avoid_target`, `pacified`, `breed_target`, `walk_target`, `look_target`, `cant_reach_walk_target_since`, `attack_cooling_down`, `nearest_repellent`, `nearest_visible_adult_piglin`, the visible adult hoglins, both counts, the nearest visible attackable player and the nearest adult.
  - Sensors on 20-tick clocks with a random first delay. They read `mob_candidates()`, which gains the `baby` field in commit 1, and the `players` handed to `tick_with_players_where` (`world.rs:4786`): every player in this level, not player 0.
  - Activities CORE, IDLE, FIGHT and AVOID, in section 3's order. Walking uses `navigation::GroundNavigation` like the villager's `MoveToTargetSink`, with the hoglin's speed × multiplier.
  - `update_activity` plays the activity-change sound and sets the aggressive flag.
- **The fight:** `StartAttacking` (players only), the walk to the target, `MeleeAttack` 40/15 with vanilla's attack box, `StopAttackingIfTargetInvalid` (dead, creative, spectator, gone, or unreachable for 200 ticks), and the 200-tick `attack_target` expiry on retaliation.
- **`doHurtTarget`:** animation 10, the attack sound, `onHitTarget`, then `hurt_and_throw`.
- **The throw on a player:** `PlayerHitKind::Melee` gains `throw: DVec3`, which is zero for every other mob.
  - `Entities` (`entities.rs:992-1015`) passes it along with the hit.
  - `render_anim/src/minecraft_world.rs:1803-1822` applies it through `impulse::launch_velocity`, as the dragon's pushes are applied (`:1850-1854`), when the hurt landed or a raised shield blocked it (`blockedByItem`, adults only).
  - The player's knockback resistance comes from worn netherite armour, 0.1 per piece. It is 0 when the fork does not model it.
- **The throw on a mob:** `mob_hits_mob` gains a `throw` push, used for the zoglin's victims and for hoglins retaliating against mobs.
- **Retaliation:** `wasHurtBy`, `maybeRetaliate`, `broadcastAttackTarget` and the baby's retreat for 100-400 ticks at 1.3, from player hits, MW2 shots and blasts (all of them reach `hurt_by_player` or `mob_blast` with an attacker) and mob attacks.
- **Decision on the iron golem:** if track A has not fixed the golem's dropped `lift`, this commit applies it in the same place, since it is the same code. Check A's log first.
- **Tests** (`world` and `hoglin_brain`):
  - `an_adult_hoglin_hits_for_three_to_eight_every_two_seconds_and_a_baby_for_half_a_heart_every_fifteen_ticks`.
  - `a_hoglin_throw_lifts_up_to_half_a_block_per_tick_and_never_lifts_an_iron_golem`.
  - `a_shield_still_gets_thrown_by_an_adult`.
  - `a_hoglin_ignores_a_creative_player_and_one_seventeen_blocks_away`.
  - `hitting_one_hoglin_sets_every_visible_adult_on_the_attacker`.
  - `a_hurt_piglet_runs_away_for_five_to_twenty_seconds`.
  - `a_retaliation_target_is_forgotten_after_two_hundred_ticks`.
  - `minecraft_survival` `entities`: the throw reaches the player's hit list.

**5. "Minecraft hoglins shy away from warped fungus, nether portals and respawn anchors"**
- **Files:**
  - `crates/minecraft_terrain/src/server_mobs.rs`: `MobWorld` overrides `World::block_in_tag` with the registry's block tags (`registries.block_tags.id` plus `Registries::block_in_tag`, as `PathTags` does at `:64-75`). It falls back to `authored_block_in_tag` for tags the registry lacks.
  - The repellent sensor scans ±8/±4 in vanilla's order.
  - `BecomePassiveIfMemoryPresent` (200 ticks), and `SetWalkTargetAwayFrom.pos` at 1.0 for 8 blocks.
  - The walk target value: −1 within 8 blocks of the repellent, 10 on crimson nylium. The stroll's position picking uses it.
  - The retreat ambient sound near a repellent.
  - Avoiding adult piglins (0.4, 8 blocks) and `piglinsOutnumberHoglins`. These are wired by kind name and become visible once piglins are simulated.
- **Tests:**
  - `a_respawn_anchor_or_warped_fungus_within_eight_blocks_calms_a_hoglin_for_ten_seconds`.
  - `a_calm_hoglin_walks_away_from_the_repellent_and_strolls_back_to_crimson_nylium`.
  - `a_portal_block_counts_as_a_repellent` (real data).
  - `enderman_holdable_still_matches_through_the_registry_tags`.
  - `outnumbered_hoglins_retreat_from_a_piglin_they_hit` (synthetic candidates).

**6. "Minecraft hoglins breed with crimson fungus and their piglets grow up"**
- **Files:**
  - `mob_actions.rs`: a hoglin arm using `animal::interact` with `#hoglin_food` through the recipe book, refused while pacified (`canFallInLove`), and setting persistence when the food is used. A zoglin takes nothing. This needs b3 group 1 (right-click on mobs), which lands before this brief.
  - `mob_actions::would_interact` gets the matching hoglin arm (consume only when `interact` would feed), and b3's `would_interact_agrees_with_interact` table gains hoglin and zoglin rows.
  - `hoglin_brain.rs`: `AnimalMakeLove` from section 3. The child is a persistent baby (age −24000), the parents get age 6000, and 1-7 experience orbs drop through the existing breeding orb path.
  - `BabyFollowAdult`.
  - `ageBoundaryReached`: damage and experience switch when a baby grows up.
  - Being in love doesn't stop an attack, but a set `breed_target` does (`EraseMemoryIf`).
- **Tests:**
  - `two_fed_hoglins_make_a_persistent_piglet_within_110_ticks`.
  - `a_pacified_hoglin_will_not_eat`.
  - `feeding_a_piglet_ages_it_by_a_tenth`.
  - `a_piglet_follows_an_adult_from_six_to_sixteen_blocks`.
  - `a_grown_piglet_hits_for_three_to_eight`.

### Group H4: zombification and the zoglin

**7. "Minecraft hoglins turn into zoglins outside the Nether"**
- **Files:**
  - `world/src/level/time.rs`: `Level::piglins_zombify()`, the `gameplay/piglins_zombify` flag (no 26.3 biome overrides it), next to `monsters_burn`.
  - `ServerSim::tick_mobs` (`server.rs:660`) passes it on through a new `EntityWorld::set_piglins_zombify`, so each level, and with E each ticking level, decides for its own hoglins.
  - The conversion clock from section 3. A NoAI or immune hoglin never converts. The `converting` flag is set for the renderer, and the retreat sound plays while converting.
  - Conversion: a new zoglin entity with a new id and UUID. It copies position, yaw, motion, on-ground state, fire, baby, persistence, NoAI and effects, starts at full health, and gets nausea for 200 ticks. The hoglin is discarded, as in `convert_zombie_to_drowned`.
- **Tests:**
  - `a_hoglin_in_the_overworld_becomes_a_zoglin_on_tick_301_and_one_in_the_nether_never_does`.
  - `going_back_to_the_nether_resets_the_clock`.
  - `an_immune_or_no_ai_hoglin_stays`.
  - `a_piglet_becomes_a_baby_zoglin_and_keeps_its_persistence`.
  - `the_clock_survives_a_save` (`server`: save at 200 and reload, then it converts 101 ticks later).
  - `the_end_converts_hoglins_too`.

**8. "Minecraft zoglins attack every mob but creepers and other zoglins"**
- **Files:**
  - `hoglin_brain.rs` zoglin activities: targets from `mob_candidates()` and players, not zoglins or creepers, alive and attackable (players not creative or spectator).
  - `MeleeAttack` 40/15, the walk to the target at 1.0, and the angry sound when entering FIGHT. The ambient sound is angry while it has a target.
  - `hurtServer` retargeting with the 4-block rule.
  - Hits on mobs through `mob_hits_mob` with the throw; hits on players through `PlayerHit`.
- **Tests:**
  - `a_zoglin_goes_for_the_nearest_cow_villager_or_player_but_never_a_creeper_or_zoglin`.
  - `a_zoglin_switches_to_a_closer_attacker_but_not_a_far_one`.
  - `a_zoglin_kills_a_hoglin_which_fights_back` (the hoglin retaliates against a non-hoglin).
  - `a_zoglin_throws_its_victims_but_not_an_iron_golem`.

### Group H5: docs

**9. "Docs: Minecraft hoglins and zoglins"**
- `docs/MINECRAFT-SURVIVAL.md`: the Nether mobs line (`:163`), N6 (`:280`) and the dormant list (`:341`).
- `docs/NETHER-END.md` table rows `:249-250`.
- The round 8 track log entry with the owner's steps and one combined kit line.

## 5. Owner's in-game test steps

Use a fresh world (`IW4L_MINECRAFT_WORLD=b2`), survival, `/difficulty normal`. `/execute in minecraft:the_nether run tp @s ~ ~ ~` moves you into the Nether, and `/execute in minecraft:overworld run tp @s ~ ~ ~` brings you back. Build a 20 × 20 netherrack floor first.

Kit (22 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:hoglin_spawn_egg*16,minecraft:zoglin_spawn_egg*4,minecraft:crimson_fungus*64,minecraft:warped_fungus*16,minecraft:crimson_nylium*16,minecraft:respawn_anchor*2,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying,minecraft:cooked_beef*32,minecraft:netherrack*64,minecraft:lava_bucket,minecraft:flint_and_steel,minecraft:obsidian*14,minecraft:cow_spawn_egg*4,minecraft:creeper_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:cobblestone*64
```

1. In the Nether, put on the armour, use one hoglin egg on the floor in front of you and back off to about 10 blocks. It is a big tusked boar that grunts, charges and headbutts upward. Each hit takes 1.5 to 4 hearts and throws you up and back. **Screenshot 1** (mid-air).
2. Raise the shield as it hits: no damage, but you are still thrown.
3. Use two more eggs close together and hit one: all three charge you. **Screenshot 2.**
4. Place a respawn anchor next to them. Within a second they snort, stop attacking and walk away from it. They stay calm about 10 seconds after you break it, then charge again. **Screenshot 3.** Do the same with warped fungus on crimson nylium, and with a lit nether portal (obsidian and flint and steel).
5. `/gamemode creative`. Right-click two adults with crimson fungus (right-click on mobs comes with b3): hearts, they walk together, and a piglet appears within about 5 seconds. **Screenshot 4.** Feed the piglet: it grows a little. `/gamemode survival`.
6. The piglet nibbles you (half a heart, quickly) and never throws you. Hit it and it runs away for 5 to 20 seconds.
7. Kill an adult: 2-4 raw porkchops, sometimes leather, and experience. Light the netherrack under another (or pour lava) and kill it while it burns: cooked porkchops. **Screenshot 5.** A killed piglet drops only experience.
8. Go to the Overworld and use a hoglin egg. It shakes and snorts for 15 seconds, then turns into a grey, rotting zoglin with a groan. **Screenshot 6** (shaking) and **7** (zoglin). `/summon minecraft:hoglin ~ ~ ~ {IsImmuneToZombification:1b}` stays a hoglin. **Screenshot 8.**
9. Use 4 cow eggs, 2 creeper eggs and a zoglin egg. The zoglin kills the cows and ignores the creepers, then comes for you and throws you. **Screenshot 9.** Its kill drops rotten flesh. In lava it doesn't burn.
10. Use an iron golem egg next to a zoglin: the golem attacks it. The golem isn't thrown.
11. Use a hoglin egg in the Overworld and save and quit after about 10 seconds. After rejoining, it shakes and converts about 5 seconds later.
12. `/locate biome minecraft:crimson_forest` in the Nether and go there: hoglins roam in groups of 3-4 on the nylium. **Screenshot 10.** `/locate structure minecraft:bastion_remnant`: the hoglin stables' hoglins move and attack. **Screenshot 11.**
13. `/difficulty peaceful`: every hoglin and zoglin vanishes.

## 6. Risks, shared files and decisions

**Risks**
- **Size of the brain port.** It is the first `Brain` mob besides the villager, and it lands before b1 in track B. Keep `hoglin_brain.rs` generic over the memories it needs, so b1's piglin brain reuses its sensor clocks, Manhattan repellent scan, `AnimalMakeLove`, `MeleeAttack`, `StartAttacking`, `StopAttackingIfTargetInvalid`, `SetWalkTargetAwayFrom` and `BecomePassive` pieces instead of copying them.
- **Pieces b1 reuses:** `MobEffect::Nausea` and `MobCandidate.baby` (commit 1), the registry-backed `MobWorld::block_in_tag` (commit 5, b1 needs `#piglin_repellents`), `Level::piglins_zombify` and `EntityWorld::set_piglins_zombify` (commit 7), and `PlayerHitKind::Melee.throw` (commit 4, beside which b1 adds `disable_shield_seconds`). Name them so they are not hoglin-specific.
- **Existing worlds wake up.** Bastion hoglins, and any dormant crimson forest hoglins in the owner's world, become hostile and simulated once loaded. That is intended, but say so in the test notes.
- **Repellent scan cost.** The scan reads 17 × 9 × 17 blocks per hoglin every 20 ticks, as vanilla does. Cache the tag id, and read through `MobWorld`'s chunk access, never `load_now`.
- **`MobCandidate.baby`** touches every candidate the world builds. Fill it in one place, the `add` closure at `world.rs:3462`.
- **Untested combinations:**
  - Mobs don't use portals here, so a hoglin never follows you through one. Overworld hoglins come from eggs or `/summon`.
  - `Fire Aspect` is not modelled (no `fire_aspect` in the code), so only burning hoglins drop cooked meat.
- **The registry-backed `block_in_tag`** changes where endermen read `#enderman_holdable`. The test in commit 5 pins it.

**Files shared with other round 8 tracks**
- **Other B items** (nether and overworld mobs, sleep). These run in the same track one after another:
  - entities crate: `world.rs` (storage, `EntityKey`, `MobHit`, `mob_candidates`, `check_despawn`), `monster_ai.rs` (`is_enemy`) and `effects.rs`;
  - natural spawning: `natural_spawner.rs` and `spawning.rs`;
  - server: `server_mobs.rs`, `mob_actions.rs` and `generator/src/feature/entities.rs` (the piglin finalize sits next to the hoglin's);
  - client: `mesh.rs` and `client_mobs.rs`;
  - `minecraft_survival/src/entities.rs`, for the encode keys.

  b1 (after this brief) adds the piglin side of the hoglin relationship: hunting (`StartHuntingHoglin`, `CannotBeHunted`, `RememberIfHoglinWasKilled`) and counting as an adult piglin in the hoglin sensor. b3 group 1 (before this brief) brings right-click on mobs, which commit 6 needs.
- **C (End and visual leftovers):** shulkers will want an encode key and touch `mesh.rs`, `client_mobs.rs` and `Entities::meshes`. Take the next free key, whoever merges second.
- **D (skating):** nothing shared expected.
- **E (multiplayer):**
  - `minecraft_survival/src/entities.rs:923` and `:992-1015` keep only player `0`'s hits and pushes. The new `throw` rides inside `PlayerHit`, which already carries `player_id`, so E's routing covers it with no new state.
  - The application in `render_anim/src/minecraft_world.rs:1803-1822` is in the player-action code E moves out of `render_anim`. It is one block next to the shield code, so tell E.
  - Sensors and targeting use the `players` list each level ticks with, so several players work.
  - MW2 shots and blasts still pass `PLAYER` (`entities.rs:764-798`), as every mob's do.
  - `mob_action` has no player id. Feeding needs none, and vanilla's `LoveCause` (only for statistics and advancements) is left out.
  - `piglins_zombify` is read per `Level` in `ServerSim::tick_mobs`, so with E's "every occupied level ticks" each level converts its own hoglins, and a level that doesn't tick doesn't advance its clocks. That matches vanilla, where only ticking entities count.
- **A (fixes):** the dropped golem `lift` and husk `hunger_ticks` on player hits (`entities.rs:992-1015`). Coordinate with commit 4.

**Decisions (the vanilla-like option each time)**
1. **One storage, two kinds.** Hoglin and zoglin share `HoglinEntity` with a `HoglinKind`, like `ZombieKind`, because they share the model, attack and throw (`HoglinBase`). Conversion still makes a new entity with a new UUID, as `ConversionType.SINGLE` does.
2. **A brain, not goals.** The hoglin and zoglin run a port of their `Brain` (sensor clocks, activities, memory expiry) on `villager_brain::Slot`, not the goal framework, so timings such as 20-tick sensing, 200-tick pacifying and 40-tick attacks match.
3. **Data, not hard-coded lists.** Repellents and food are read from the 26.3 tags at runtime: `#hoglin_repellents` through the registry, and `#hoglin_food` through the recipe book.
4. **Where the throw happens.** It goes out with the hit and lands only if the hit landed or a shield blocked it (`hurtAndThrowTarget` and `blockedByItem`), using the existing launch conversion for MW2 gravity. The radian quirk of `Vec3.yRot` is kept.
5. **How zombification is decided.** It reads `gameplay/piglins_zombify` as a per-level value, since no 26.3 biome changes it. Vanilla samples it at the hoglin's position. If a biome override ever appears, switch to the position sample.
6. **Out of scope:**
   - piglin behaviour (only the hoglin side is wired, by kind name);
   - leads;
   - spawn eggs used on adults to make babies;
   - mobs travelling through portals;
   - Fire Aspect;
   - `LoveCause` statistics.
