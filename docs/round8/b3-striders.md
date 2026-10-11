# Track B, item b3: striders and riding (design brief)

Paths and lines are as of e2313625. Vanilla facts were checked with javap on the 26.3 `client.jar` and with the data in the same jar (tags, loot table, biomes). No code was copied. Riding state is server-owned: the vehicle and its passengers live in the level's entity world, and the `Player` (UUID) keeps only a mirror for the adapter and the save. Every place that still goes through `PLAYER = 0` today is named in section 6.

## 1. The owner's request

> Striders with saddles and the warped fungus on a stick: vanilla striders that walk on lava, go cold and shiver out of it with their cold texture, can be ridden with a saddle and steered with a warped fungus on a stick, with its boost and its durability rule. Their speeds, breeding with warped fungus, baby striders riding adults, jockeys with zombified piglins, and spawning on lava in the Nether.

Scope from the orchestrator also asks how players mount anything today (pigs, boats, minecarts) and how the MW2 player movement would be driven by a mount.

Answer in short: **nothing can be mounted today.** Pigs can be saddled server-side, but no right-click on a mob reaches the server, and there is no passenger relation, no boats and no minecarts. This brief therefore builds three things in order: right-click on mobs, the strider itself, and a riding system that pins the MW2 player to a server-simulated seat.

## 2. What exists today

| Area | State |
|---|---|
| Right-click on mobs | **Not wired on the host.** `ServerSim::mob_action` runs `mob_actions::interact` when `attack` is `None` (`minecraft_terrain/src/server.rs:357-385`), but the host only ever sends attacks (`minecraft_survival/src/entities.rs:543`, `:778`). `MobResult.slots` is never applied; only its sounds are played (`entities.rs:974-976`). So feeding, breeding, saddling pigs, milking and taming never happen in game. The use chain in `render_anim/src/minecraft_world.rs:1157-1215` only looks at blocks. |
| Pig saddle | `Pig::interact_saddle` (`third_party/minecraftoss/entities/src/pig.rs:104-130`), the saddle sound (`minecraft_terrain/src/mob_actions.rs:174-190`), load and save of `equipment.saddle` (`server_mobs.rs:511-512`, `:1220-1223`), and the saddle layer (`pig_render.rs:239-253`, texture `mesh.rs:609`). |
| Item steering | `SteeringState` with `boost` and `tick_boost` matches `ItemBasedSteering` (`third_party/minecraftoss/entities/src/steering.rs:1-33`). It sits on `PigEntity` (`entities/src/world.rs:192`) but nothing calls it outside the check binaries (`entities/src/bin/check_pig_boost.rs`). There is no `boostFactor`. |
| Passengers | None. Natural spawning builds `Passengers` NBT for jockeys (`world/src/natural_spawner/spawning.rs:888`, `:943`, `:1083`, `:1144`), but `spawn_saved` refuses any tag with `Passengers` (`server_mobs.rs:559`, `:596`, `:648`), so jockeys wait dormant. The census marks nobody as riding (`server.rs:612`). |
| Strider | Not simulated (`SIMULATED_KINDS`, `server_mobs.rs:388-419`). `type_info` knows its size and `InLava` placement (`world/src/natural_spawner.rs:286`, check at `:647`), but its rules fall to `Rules::Other` (`:309-327`), which refuses and logs (`natural_spawner/spawning.rs:629-632`), so it never spawns. It is listed as fire immune (`entities/src/world/hazards.rs:37`) and as a creature (`spawning.rs:103`). |
| Lava standing | `WalkProfile.can_stand_on_fluid` exists (`entities/src/walk_path.rs:28`), but the path start only tests `floatable_fluid` (water) (`walk_path.rs:311`), and body collision (`player/src/collision.rs:45-80`) has no fluid shapes at all. |
| Tempting | `PlayerCandidate` has held-item flags (`entities/src/tempt.rs:6-27`), but the host fills all of them with `false` (`minecraft_survival/src/entities.rs:162-183`), so no mob is ever tempted in game. There are 62 struct literals of `PlayerCandidate` in 37 files. |
| Server tick input | `TickInput` (`server.rs:2314-2339`) has one player candidate, built by `Entities::tick` with `PLAYER = 0` (`entities.rs:36`, `:862-890`). |
| Durability | `Inventory::wear_tool` (`player/src/inventory.rs:372-395`) has no Unbreaking; the trident adds it on top (`minecraft_terrain/src/trident.rs:84-94`). Nothing converts a broken item into another. |
| Item-use answers | `Output.item_uses` and the host's `item_use_waiting` (`server.rs:2394`, `entities.rs:601`, `:950`) answer a use the server decides. |
| MW2 movement overrides | Elytra gliding is a pmove mode with its own flags (`playerstate_iw4/src/playerstate.rs:182-184`, `movement_iw4/src/glide.rs`, `sim/src/frame.rs:427-442`), driven by `render_anim/src/minecraft_elytra.rs`. `SimWorld::set_origin` zeroes velocity and moves the player area (`sim/src/frame.rs:367-386`); `teleport` also flips the TELEPORT bit (`:390-398`); `gate_nudge_origin` moves without stopping (`:527-534`). The pm_flags bit `0x0100_0000` is unused (only `usercmd` `MC_FLY_TOGGLE` and the contents mask `CONTENTS_MANTLE` use that value, in other fields). |
| Steps and falls | The adapter counts footsteps, fall peaks and fall damage from the MW2 feet each frame (`minecraft_world.rs:833-908`). A pinned rider would count as falling. |
| Humanoid riding pose | Not drawn. `zombie_render.rs` and `piglin_render.rs` have no passenger pose. |
| Saving | The player file keys are `player_nbt.rs:122` (`OWNED`); an unknown `RootVehicle` from a vanilla file survives the merge (`:264-277`) but is never read. Mobs save through `mob_tags` (`server_mobs.rs:1048`). |
| Spawn eggs | Liquids are valid targets (`minecraft_terrain/src/spawn_egg.rs:10`). `hatch_egg` fails for unsimulated kinds (`server.rs:757-763`). |

## 3. Vanilla 26.3 behaviour (javap)

**Entity type** (`EntityTypes`, static init): strider is `CREATURE`, `fireImmune`, sized 0.9 x 1.7 (eye 0.85 x height = 1.445), tracking range 10. No `ridingOffset`, so a strider rider's own vehicle attachment is 0. `Strider.BABY_DIMENSIONS` is 0.45 x 0.85, eye 0.4375, passenger attachment at 0.65625; `getDefaultDimensions` returns it for a baby without the 0.5 age scale. Adults use the default passenger attachment at their height, 1.7.

**Attributes** (`Strider.createAttributes`): `Animal.createAnimalAttributes` (max health 20, tempt range 10) plus `movement_speed` 0.17499999701976776.

**Synced data:** `DATA_BOOST_TIME` (int) and `DATA_SUFFOCATING` (bool). No extra save data: the saddle is the `equipment.saddle` slot. `canUseSlot(SADDLE)` is alive and adult.

**Warmth** (`Strider.tick`, skipped for `NoAI`):
- Warm when the block at its feet or the block under it (`getBlockStateOnLegacy`) is in `#strider_warm_blocks` (only `minecraft:lava`), or its lava fluid height is above 0, or it rides a strider that is not suffocating.
- `setSuffocating(!warm)`. Suffocating adds the transient modifier `minecraft:suffocating`, -0.34 `ADD_MULTIPLIED_BASE`, to movement speed: 0.175 becomes 0.1155.
- Then `Animal.tick`, then `floatStrider`: in lava, when its box is above the 16x8x16 liquid shape of its block (`CollisionContext.isAbove`) and the block above is not lava, it is on the ground; otherwise its motion becomes `motion * 0.5 + (0, 0.05, 0)` (it rises).
- Fall damage resets while in lava (`checkFallDamage`). `isOnFire` is always false. `isSensitiveToWater`: `LivingEntity.aiStep` hurts it with `drown` for 1.0 every tick it is in water or rain.

**Standing on lava** (`LiquidBlock.getCollisionShape` with `Strider.getLiquidCollisionShape` = `Block.column(16, 0, 8)`): a lava **source** (level 0) collides as a bottom half slab (0 to 0.5) for a strider whose box is above that shape, when the fluid above is not lava (`canStandOnFluid` is lava only). Flowing lava never collides. Block friction under it is the lava block's default 0.6.

**Pathfinding:** `StriderPathNavigation` uses a plain `WalkNodeEvaluator`; `isStableDestination` also accepts lava. Malus: `WATER` -1, `LAVA` 0, `DANGER_FIRE` 0, `DAMAGE_FIRE` 0. `getWalkTargetValue`: 10 at a lava block, minus infinity anywhere else while in lava, 0 otherwise. No `FloatGoal`.

**Goals** (`registerGoals`):

| Priority | Goal | Numbers |
|---|---|---|
| 1 | `PanicGoal` | speed 1.65 |
| 2 | `BreedGoal` | 1.0 |
| 3 | `TemptGoal` | 1.4, `#strider_tempt_items` (`#strider_food` = `warped_fungus`, plus `warped_fungus_on_a_stick`), not scared |
| 4 | `StriderGoToLavaGoal` | `MoveToBlockGoal(1.0, 8, 2)`: only when not in lava; target is `minecraft:lava` with a block above that is pathfindable by land; recalculates every 20 try-ticks; stops once in lava |
| 5 | `FollowParentGoal` | 1.0 |
| 7 | `RandomStrollGoal` | 1.0, interval 60 (not water-avoiding) |
| 8 | `LookAtPlayerGoal` | players, 8 |
| 8 | `RandomLookAroundGoal` | |
| 9 | `LookAtPlayerGoal` | striders, 8 |

`MoveToBlockGoal`: start delay `reducedTickDelay(200 + nextInt(200))`, give up after 1200 try-ticks, accepted distance 1.0, moves to the block centre plus (0.5, 1, 0.5).

**Sounds:** ambient `entity.strider.ambient` (none while panicking or tempted); while tempted, 1 in 140 per tick `entity.strider.happy`; while panicking, 1 in 60 `entity.strider.retreat`; hurt, death; steps `entity.strider.step_lava` in lava, else `entity.strider.step`, every 0.6 of walked distance (`nextStep`); saddle equip `entity.strider.saddle`; eating `entity.strider.eat` at volume 1, pitch `1 + (r1 - r2) * 0.2`.

**Interaction** (`Strider.mobInteract`):
1. Held item **not** strider food, strider saddled, not already a vehicle, player not sneaking: the server calls `player.startRiding(strider)`. Result `SUCCESS`.
2. Otherwise `Animal.mobInteract` (warped fungus feeds: love, baby growth, golden dandelion age lock).
3. If that passed and the held item is equippable in the saddle slot: equip the saddle (`ItemStack.interactLivingEntity`). Otherwise `PASS`.
4. A consumed food interaction plays the eat sound.
- `canAddPassenger`: not already carrying anyone, and its eyes are not in lava. So a strider carries one passenger.
- `getBreedOffspring`: a new strider.

**Who controls it** (`getControllingPassenger`): a saddled strider whose first passenger is a player holding `warped_fungus_on_a_stick` in **either** hand (`Player.isHolding`). Otherwise `Mob.getControllingPassenger`: the first passenger if it is a `Mob`, the strider has AI, and the passenger is not in `#non_controlling_rider` (slime, magma cube, sulfur cube). So a baby strider or a zombified piglin steers the adult under it. `Mob.getNavigation` and `getMoveControl` return the controlled vehicle's, so the rider's goals move the vehicle; `Mob.updateControlFlags` turns off the vehicle's own MOVE, JUMP and LOOK goals while a mob controls it.

**Ridden by a player with the stick** (`LivingEntity.travelRidden`, `Strider.tickRidden`, `getRiddenInput`, `getRiddenSpeed`):
- `tickRidden`: `setRot(player.yRot, player.xRot * 0.5)`; head, body and old yaw all take that yaw; `steering.tickBoost()`.
- Input is always `(0, 0, 1)`: it walks forward wherever the rider looks, without any key.
- Speed = `movement_speed * (suffocating ? 0.35 : 0.55) * boostFactor`.
- `boostFactor` = `1 + 1.15 * sin(boostTime / boostTimeTotal * pi)` while boosting, else 1.
- Then normal `travel`: on lava (friction 0.6) the per-tick push is the speed, and motion keeps `0.6 * 0.91 = 0.546` each tick, so the steady step is `speed / 0.454`:

| State | Speed | Steady blocks per tick | Blocks per second |
|---|---|---|---|
| Warm, no boost | 0.175 x 0.55 = 0.09625 | 0.212 | 4.24 |
| Warm, boost peak | x 2.15 | 0.456 | 9.1 |
| Cold, no boost | 0.1155 x 0.35 = 0.0404 | 0.089 | 1.78 |
| Unridden stroll (AI, for comparison) | 0.175 x 0.175 per tick | 0.067 | 1.35 |

**Boost** (`ItemBasedSteering`): `boost(random)` fails while boosting; otherwise boosting starts, `boostTime` 0, and `boostTimeTotal` = `140 + nextInt(841)` (7 to 49 s). `tickBoost`: `if (boostTime++ > total) boosting = false`.

**The stick** (`FoodOnAStickItem.use`, `Items`): `warped_fungus_on_a_stick` has durability **100** and costs **1** per boost (the carrot on a stick has 25 and costs 7). Server side only: if the player is a passenger whose controlled vehicle is a strider and `boost()` succeeds, `hurtAndConvertOnBreak(1, FISHING_ROD, player, hand)` and `SUCCESS_SERVER` (the hand swings). Otherwise `PASS` (and the `used` stat). `hurtAndBreak` skips players with infinite materials and applies Unbreaking (`EnchantmentHelper.processDurabilityChange`). On break the stack becomes a `fishing_rod` copy with the same components (enchantments, name) and damage 0, and the break sound and particles play. A use while already boosting does nothing and costs nothing.

**Rider placement** (`Entity.positionRider`, `getPassengerRidingPosition`): rider feet = vehicle position + passenger attachment - rider's vehicle attachment.
- Player (`Avatar.DEFAULT_VEHICLE_ATTACHMENT` = 0.6): feet at strider Y + 1.7 - 0.6 = **+1.1**, eyes at +2.72.
- Zombified piglin (`ridingOffset(-0.7)`, so vehicle attachment 0.7, half for a baby): feet at +1.0.
- Baby strider on an adult: feet at +1.7.
- Client only: `Strider.getPassengerAttachmentPoint` adds `0.12 * cos(walkPos * 1.5) * 2 * min(0.25, walkSpeed) * scale` to Y, so riders bob with its stride.
- Strider does not override `clampRotation`: a rider can look all around.

**Getting off:**
- `Player.rideTick` on the server: when `isShiftKeyDown` and a passenger, `stopRiding()`.
- `LivingEntity.stopRiding` → `dismountVehicle` → `Strider.getDismountLocationForPassenger`. It tries five horizontal offsets from `getCollisionHorizontalEscapeVector(0.9, 0.6, yaw)`, with yaw = the rider's yaw, then yaw - 22.5, yaw + 22.5, yaw - 45 and yaw + 45. Each offset is `(-sin, 0, cos)` scaled so its larger component is `(0.9 + 0.6 + 1e-5) / 2`. So the rider steps off **forward**, onto what they look at.
  - For each offset, it walks the column from the strider box's top down to its bottom - 0.5, one block at a time. It skips lava blocks and needs `DismountHelper.isBlockFloorValid` (floor height finite and below 1) and a standing box that `canDismountTo` (no collision, inside the border).
  - If nothing fits: `(x, box top, z)`, on top of the strider.
- `Entity.removePassenger` sets the passenger's `boardingCooldown` to 60, but `Player.removeVehicle` puts it back to 0, so players can mount again at once. `canRide` also refuses while sneaking.
- Dismounting also happens on `setGameMode` (spectator), `teleport`, `startSleeping`, a spear's stab, the vehicle's removal (`setRemoved` → `unRide`), and in `Entity.baseTick` when the vehicle was removed. `LivingEntity.rideTick` resets the rider's fall distance every tick.
- A passenger can't use a nether portal itself (`canUsePortal(false)`).

**Natural spawning:**
- All five Nether biomes list `minecraft:strider` under `creature`, weight 60, group 1-2.
- Placement `IN_LAVA`, heightmap `MOTION_BLOCKING_NO_LEAVES`.
- `checkStriderSpawnRules`: go up from the block above the spawn position while the fluid there is lava; the first non-lava block must be air.
- `checkSpawnObstruction` is overridden to only `level.isUnobstructed(this)` (entities), with no liquid test.
- `PathfinderMob.checkSpawnRules` reads `getWalkTargetValue` (10 in lava).

**`Strider.finalizeSpawn`** (natural, egg and summon alike; a baby goes straight to `Animal.finalizeSpawn`):
1. 1 in 30, and `isSpawningMonsters()` (game rules `spawn_mobs` and `spawn_monsters`): a zombified piglin jockey.
   - `spawnJockey` snaps it to the strider's position and yaw and runs its own `finalizeSpawn` with reason `JOCKEY` and `ZombieGroupData(nextFloat() < 0.05 baby, no chicken jockey)`, so a golden sword first.
   - It then gets `warped_fungus_on_a_stick` in the main hand (replacing the sword) and rides the strider.
   - The strider gets a saddle in the saddle slot with a guaranteed drop.
2. Else 1 in 10: a baby strider (`Age` -24000) is made with reason `JOCKEY` and rides the adult.
3. Else `AgeableMobGroupData(0.5)`, a fresh one each time, so its group size is always 0 and a natural adult never brings a baby.
- Riders are passengers: `Mob.requiresCustomPersistence` is `isPassenger()`, so they never despawn and don't count toward the caps.

**Loot** (`loot_table/entities/strider.json`): 2-5 string, plus 0-1 per Looting level. The saddle drops from the equipment slot.

**Rendering** (`StriderRenderer`, `StriderModel`, `AdultStriderModel`, `BabyStriderModel`):
- Textures `entity/strider/strider`, `strider_cold`, `strider_baby` and `strider_cold_baby`; the cold ones while suffocating. Shadow 0.5, halved for a baby.
- `isShaking` while suffocating: `LivingEntityRenderer.setupRotations` adds `cos(ageInTicks * 3.25) * pi * 0.4` degrees to the body yaw.
- Saddle layer: `SimpleEquipmentLayer` with `STRIDER_SADDLE` (`AdultStriderModel` baked from `ModelLayers.STRIDER_SADDLE`), texture `entity/equipment/strider_saddle/saddle`.
- Adult mesh (64 x 128 texture): legs 4x16x4 at (-4, 8, 0) and (4, 8, 0) (UV 0,32 and 0,55); body 16x14x16 from (-8, -6, -8) at offset (0, 1, 0); six bristle planes 12x0x16 with rest roll ±1.2217305, ±1.134464 and ±0.87266463 (UV 16,65 / 16,49 / 16,33).
- Animation (`StriderModel.setupAnim`, `SPEED` 1.5): legs swing with `sin(pos * 1.5 * 0.5) * 2 * speed` and roll `0.17453292 * cos(...)`; body roll `0.1 * sin(pos * 1.5) * 4 * speed`; walk speed capped at 0.25. Unridden, the body takes the head pitch and yaw; ridden, both are 0. Bristles sway by `animateBristle` (0.6, 1.2, 1.3 offsets; 0.1, 0.4, 0.2, -0.4 and 0.05 terms).
- The implementer copies the exact cuboids and the baby model with javap, like `slime_render.rs`.
- Riding humanoids (`HumanoidModel.setupAnim`, `isPassenger`): arms xRot += -0.62831855; right leg xRot -1.4137167, yRot 0.31415927, zRot 0.07853982; the left leg mirrored.

**Saving a rider** (`ServerPlayer.saveParentVehicle` / `loadAndSpawnParentVehicle`):
- When the player's root vehicle has exactly one player passenger, the player file gets `RootVehicle: {Attach: <player's own vehicle UUID>, Entity: <root vehicle with its passengers>}`.
- On join it loads the entity recursively with `addWithUUID`, which refuses a UUID already in the level, then rides the one whose UUID matches `Attach`. If that fails, it logs "Couldn't reattach entity to player" and discards the vehicle.
- On leave the vehicle is removed with the player (`UNLOADED_WITH_PLAYER`), so the chunk save doesn't keep a copy. Chunk entity loading refuses a second entity with a known UUID.

## 4. The plan: commits in order

New vendored code goes into `third_party/minecraftoss` (edition 2021: no let-chains; count the new and patched files for the state doc's MinecraftOSS line). Fork crates follow the CLAUDE.md rules: no comments, no rustfmt on existing files, new files pass `rustfmt --check`.

**The player identity.** Wherever this brief names "the player" on the server thread, it is the `u64` the tick's `PlayerCandidate.id` carries. Today that is `entities.rs:36` `PLAYER = 0`. Route it through the one constant, never a new literal 0, so track E can swap it for `Player.id`.

### 1. "Minecraft players use items on mobs"

Right-click on a mob, the prerequisite for saddling, mounting and feeding.
- **Host:** before the block chain in the use closure (`minecraft_world.rs:1173`), for each hand in `use_hands` order (main hand, then offhand):
  - If `entities.mob_distance` finds a mob within the entity interaction range (3.0 in survival, 5.0 in creative, as the state doc's reach line says) and nearer than the targeted block, call a new `Entities::use_on_mob(hit, inventory, slot, infinite, player)`.
  - It sends `ServerHandle::mob_action(hit, None, …)` and returns whether the click is consumed.
- **Consume or pass.** The server answers a tick later, so the host decides with a new pure predicate `mob_actions::would_interact(world: &EntityWorld, hit, held: Option<&str>, crouching, is_food: impl Fn(tag, item) -> bool) -> bool` on its copy of the mob world.
  - It mirrors each arm of `mob_actions::interact`: saddle on an unsaddled adult pig; food in the species tag; shears on an unsheared sheep or mooshroom; bucket or bowl on a cow or mooshroom; wolf items; iron ingot on a hurt golem.
  - On `false` the click falls through to the item use, as vanilla's `PASS` does.
- **Results:** `MobResult.slots` (`server.rs:375-388`) is applied to the player's inventory when it arrives (`entities.rs:974`).
  - The server worked on a copy from the moment of the click, so only the changed slots are written back (the same rule the pickup copy follows).
  - Broken tools go into `broken_items`.
- `Command::MobAction` gains `player: u64`. `mob_action` passes it to `villager_interact` and `wolf_interact` (`mob_actions.rs:234`, `:243` hard-code 0) and to `merchant_view` (`server.rs:388`, `:446` use 0).
- **Villagers stay excluded** on the host (no trading screen exists, and an opened server menu would hold the villager): `would_interact` is `false` for `MobHit::Villager`.
- **Files:** `minecraft_terrain/src/mob_actions.rs`, `minecraft_terrain/src/server.rs` (command and `mob_action` signature), `minecraft_survival/src/entities.rs`, `render_anim/src/minecraft_world.rs` (one call), or a new `render_anim/src/minecraft_mob_use.rs` to keep the block chain short.
- **Tests:**
  - `mob_actions`: `would_interact_agrees_with_interact` (a table of mob, held item and crouch cases; for each, `would_interact` equals `interact(...).handled` on a clone).
  - `entities`: `a_saddle_used_on_a_pig_leaves_the_inventory_and_saddles_it` (host path through a real `ServerHandle`, slot emptied after the answer); `a_mob_behind_a_block_is_not_used`; `creative_reaches_mobs_at_five_blocks_and_survival_at_three`.

### 2. "Minecraft striders stand on lava, warm up in it and shiver out of it"

The body, its rules and its save, without AI goals yet (it stands, floats and sounds).
- New vendored `entities/src/strider.rs`: `Strider { body, yaw, age, in_love, persistence_required, health (20), damage, saddled, suffocating, steering: SteeringState, boost_time_total }`.
  - `dimensions(baby)`: 0.9 x 1.7, or 0.45 x 0.85.
  - `movement_speed()`: 0.175, or 0.1155 while suffocating.
  - `boost_factor()`: section 3. Add it to `steering.rs` as `SteeringState::boost_factor`, since `boost_time_total` already lives there.
  - `riding_attachment(baby)`: 1.7 or 0.65625.
- New vendored `entities/src/world/striders.rs` (pattern: `world/blazes.rs`): `spawn_strider`, `striders()`, `strider_mut`, and the tick:
  - the warmth test and the speed modifier, skipped for `NoAI`;
  - `floatStrider`;
  - water and rain `drown` 1.0 (reuse the enderman's `hurt_by_water`, `world/endermen.rs:215-225`);
  - fall reset in lava;
  - step, hurt, death and ambient sounds.
- **Lava collision.** Add a `stands_on_lava: bool` to `collision::Bounds` callers through a new `move_body_on_fluid` (`player/src/collision.rs`). For a lava source whose block above has no lava, it adds the box 0..0.5 to the candidate shapes when the body's bottom is at or above `y + 0.5 - 1e-5` (`isAbove` with its epsilon). `Body::travel_air_jumping` uses it for striders only.
- **Path start.** `walk_path.rs:311` asks a new `World::fluid_is_lava(pos)` when `can_stand_on_fluid` is set, so striders start their path on the lava surface. `WalkProfile::strider` sets the maluses from section 3, no float, and `WalkTarget::Strider` (lava 10, minus infinity in lava, else 0).
- **Wiring** per NETHER-END.md "Wiring in a new mob kind":
  - `MobHit::Strider` (`world.rs:1929-1956`), `entity_type` (`:3932`), `positions` (`:4488`);
  - the `EntityKey` arms in `pushing.rs`, `combat.rs`, `world.rs:2676`, `:3132`, `:3595`, `:4011`, `:4045` and `:4107`;
  - `SIMULATED_KINDS`, and a `spawn_saved` arm (`Age`, `InLove`, `Health`, `equipment.saddle`, `NoAI`, `PersistenceRequired`);
  - a `mob_tags` save block (the pig's at `server_mobs.rs:1220` is the model);
  - the loot table, through the existing `EntityLootBook`.
- **Files:** the vendored `entities/src/{lib.rs, strider.rs, steering.rs, movement.rs, walk_path.rs, world.rs, world/striders.rs, world/pushing.rs, world/combat.rs}`, `player/src/{collision.rs, lib.rs}`, `minecraft_terrain/src/server_mobs.rs`.
- **Tests:**
  - `strider`: `a_strider_on_a_lava_source_stands_half_a_block_up` (stands at y + 0.5 with `on_ground`, and sinks into flowing lava and rises with +0.05);
  - `out_of_lava_it_suffocates_and_slows_to_0_1155`;
  - `water_and_rain_hurt_it`;
  - `a_baby_is_0_45_by_0_85`;
  - `the_boost_factor_peaks_at_2_15_halfway` (with `tick_boost` ending after `total + 1` ticks);
  - `server_mobs`: `a_strider_round_trips_with_its_saddle_age_and_love`;
  - `hazards`: `striders_ignore_lava_and_fire`.

### 3. "Minecraft striders seek lava, follow warped fungus and breed with it"

- New vendored `entities/src/strider_ai.rs` on the pig's selector pattern (`pig_ai.rs`): the goal table from section 3.
  - `StriderGoToLavaGoal` reuses the zombie's `findNearestBlock` ring walk (`monster_ai.rs:1927-1950`) with range 8 and height 2.
  - Ambient, happy and retreat rolls go in the tick.
- **Tempting needs the held items.** Don't add fields to `PlayerCandidate` (62 literals).
  - Add `TickInput.controls: Vec<PlayerControls>`, with `PlayerControls { id, yaw, pitch, main_hand: Option<String>, offhand: Option<String>, sneaking }` in a new vendored `entities/src/controls.rs`. Derive `Default` and build it in one helper, so b1 can add `armor: [Option<String>; 4]` (piglins read worn gold) without touching every caller. This is the only per-player held and worn item channel in track B.
  - Add `EntityWorld::set_player_controls(&[PlayerControls])`, called by `ServerSim::tick` before `tick_mobs`.
  - Strider tempt tests `#minecraft:strider_tempt_items` through the world's item tags (the same lookup `mob_actions` uses through `recipes.item_in_tag`).
  - `Entities::tick` fills one entry from `PlayerView`, which gains `main_hand`, `offhand` and `sneaking` (yaw and pitch are already there).
  - The other species' tempt flags stay `false` (named as a limitation; this brief doesn't change their behaviour).
- **Interaction** in `mob_actions::interact` and `would_interact`, a new `MobHit::Strider` arm in the order from section 3:
  - feeding warped fungus (love, growth, age lock, `entity.strider.eat` sound);
  - the saddle on an adult with `entity.strider.saddle`;
  - the mount arm comes in group 7b.
  - Breeding uses the existing `breed.rs` with a strider offspring.
- **Files:** the vendored `entities/src/{strider_ai.rs, controls.rs, world/striders.rs, lib.rs}`, `minecraft_terrain/src/{mob_actions.rs, server.rs}`, `minecraft_survival/src/entities.rs`, `render_anim/src/minecraft_world.rs` (`PlayerView` fill).
- **Tests:**
  - `strider_ai`: `a_strider_on_netherrack_walks_to_lava_within_eight_blocks`; `a_strider_in_lava_never_strolls_onto_land`; `warped_fungus_and_the_stick_tempt_it_at_1_4`.
  - `mob_actions`: `two_fed_striders_make_a_baby`; `a_saddle_goes_on_an_adult_but_not_on_a_baby`; `warped_fungus_feeds_even_a_saddled_strider`.

### 4. "Minecraft striders look like vanilla"

- New `minecraft_terrain/src/strider_render.rs`:
  - the adult and baby meshes and `setupAnim` from section 3 (cuboids by javap);
  - the four textures chosen by age and `suffocating`;
  - the shake while suffocating;
  - the saddle layer.
- Textures go in `mesh.rs` (next to `:609`). Add a `server_mobs()` push and `shadow_casters` (0.5, baby 0.25) in `client_mobs.rs:1024`, `:1121`, a call in `Entities::meshes` (`entities.rs:1200`), and the hit box and `encode`/`decode` (`entities.rs:1084`, `:1350`, `:1374`) with key 23. Keys 0-20 are taken (20 is `DRAGON_KEY`); track B uses 21 for piglins, 22 for hoglins and 23 for striders, and c1 takes 26 and 27.
- If track A's spawner display lands first, register the strider there too.
- **Tests:**
  - `strider_render`: `the_adult_has_three_cubes_and_six_bristle_planes` (vertex count), `the_cold_texture_shows_while_suffocating`, `the_shake_is_cos_3_25_times_0_4_pi_degrees`, `a_saddled_adult_adds_the_saddle_layer`;
  - `entities`: `a_strider_hit_box_encodes_and_decodes`.

### 5. "Minecraft mobs ride striders"

A passenger relation in the entity world, used for strider vehicles only.
- Vendored `entities/src/world/riding.rs`: `EntityWorld.passengers: BTreeMap<u64, Vec<Rider>>` with `Rider::Mob(u64) | Rider::Player(u64)`, plus `vehicle_of(rider)`, `start_riding(rider, vehicle) -> bool` (`canAddPassenger`: empty, eyes not in lava) and `stop_riding(rider) -> Option<DVec3>` (the dismount search from section 3, over the server `World`).
- **Tick order** (vanilla `tickNonPassenger`, then `tickPassenger`):
  - The vehicle ticks first. A mob controller's goals drive the vehicle's navigation and move control (`Mob.getNavigation` delegation), and the vehicle's own move and look goals pause.
  - Then each mob passenger runs its non-movement tick (goals, attacks, sounds, fire) and is placed at the seat (section 3 numbers).
  - A passenger takes no fall damage, never despawns and is census `riding: true` (`server.rs:612`), so it doesn't count toward the caps.
  - A dead or removed vehicle ejects its riders at their dismount spots.
- **Warmth:** a baby on a strider that isn't suffocating is warm.
- **Save and load:** `spawn_saved` accepts `Passengers` only on a strider (other jockeys keep the dormant guard at `server_mobs.rs:559`, `:596` and `:648`). It spawns each simulated passenger recursively and calls `start_riding`. `mob_tags` writes `Passengers` for mob riders and never for players.
- **Client:** `ClientMobs` places a mob passenger at its vehicle's interpolated pose plus the attachment, every frame (vanilla positions passengers in its client `rideTick`). `zombie_render.rs` and `piglin_render.rs` take the riding pose when `vehicle_of` is set.
- **Files:** the vendored `entities/src/{world.rs, world/riding.rs, world/striders.rs, monster_ai.rs}` (the zombified piglin's navigation through the vehicle), `minecraft_terrain/src/{server_mobs.rs, server.rs, client_mobs.rs, zombie_render.rs, piglin_render.rs}`.
- **Tests:**
  - `riding`: `a_baby_rides_an_adult_at_1_7_and_stays_warm_while_it_does`;
  - `a_zombified_piglin_sits_at_1_0_and_steers_its_strider`;
  - `a_strider_carries_one_passenger_and_none_with_its_eyes_in_lava`;
  - `the_dismount_spot_is_forward_never_lava_and_on_top_as_a_last_resort`;
  - `killing_the_strider_drops_its_rider_beside_it`;
  - `server_mobs`: `a_strider_jockey_round_trips_with_its_passenger`; `a_spider_jockey_still_waits_dormant`;
  - `zombie_render`: `a_riding_zombie_lifts_its_legs_by_1_4137`.

### 6. "Minecraft striders spawn on Nether lava, some with a rider"

- Vendored `natural_spawner.rs`: `Rules::Strider` (the lava climb and the air test), `Walk::Strider` (walk target 10 in lava), and the strider's `check_spawn_obstruction` without the liquid test (`:755-769`, entities only).
- `finalize_spawn` gets a strider arm in the order from section 3:
  - 1 in 30 a zombified piglin jockey when `spawn_mobs` and `spawn_monsters` are on. Run `finalize_monster` for the rider with reason jockey and a 5 % baby roll, then put the stick in its main hand. Give the strider a saddle with a guaranteed drop (`drop_chances` `saddle: 2.0`, as 26.3 writes a guaranteed slot).
  - 1 in 10 a baby strider rider.
  - Else no baby.
- Spawn eggs and `/summon` go through the same `summon` path, so they make jockeys too. `hatch_egg` then finds the strider simulated.
- **Files:** the vendored `world/src/natural_spawner.rs` and `world/src/natural_spawner/spawning.rs`.
- **Tests:**
  - `natural_spawner`: `striders_spawn_in_lava_with_air_above_and_not_under_a_lava_roof`; `a_strider_in_lava_is_not_obstructed_by_the_lava`;
  - `finalize`: `one_in_thirty_get_a_zombified_piglin_with_a_stick_and_a_saddle` (seeded counts over 3000 rolls within vanilla's odds); `no_piglin_rider_when_spawn_monsters_is_off`; `a_natural_adult_never_brings_a_baby`;
  - `server`: `a_nether_level_spawns_striders_on_its_lava_sea` (the round 7 Nether `ServerSim` fixture, some thousand ticks, at least one strider, all with feet in or on lava).

### 7a. "MW2 players can be pinned to a seat"

Split from group 7: the MW2 side alone, with no game behaviour yet, so the large group 7 shrinks and b4 (sleeping) can reuse the pin to hold a player in bed.
- `pm_flags::MC_RIDING = 0x0100_0000` (`playerstate.rs`). Extend the bit test at `sim/src/frame.rs:1245-1250`, and replicate the bit like `MC_GLIDING` (`net/src/transport/netfields.rs:242`).
- `SimWorld::ride(id, Option<[f32; 3]>)` in `sim/src/frame.rs` and `carrier.rs`: with a seat it sets the flag and moves the origin like `gate_nudge_origin`, with zero velocity and no TELEPORT bit; `None` clears the flag.
- `movement_iw4/src/single.rs:70` `pmove`: with `MC_RIDING`, update the view angles, clear SPRINTING, JUMPING and the stance changes, set CROUCH from the command's `CROUCH` or `PRONE` buttons (`input_iw4/src/lib.rs:357-400`, `:467-468`) so the adapter can read the sneak, zero the velocity, and return.
- **Files:** `playerstate_iw4/src/playerstate.rs`, `sim/src/{frame.rs, carrier.rs}`, `movement_iw4/src/single.rs`, `net/src/transport/netfields.rs`.
- **Tests:**
  - `movement_iw4`: `a_riding_player_turns_but_never_moves_jumps_or_sprints`; `crouch_while_riding_sets_the_crouch_flag_only`; `the_next_pmove_after_riding_is_normal`.
  - `sim` `frame`: `ride_moves_without_the_teleport_bit_and_clears_with_none`, plus the bit-uniqueness test.

### 7b. "Minecraft players ride saddled striders"

How the MW2 movement is driven by a mount: **the server simulates the strider, and the MW2 player is pinned to its seat** with 7a's `MC_RIDING`. The MW2 pmove only turns the view.
- **Server:**
  - The mount arm in `mob_actions::interact`: held item not strider food, saddled, no passenger, player not crouching (`PlayerControls.sneaking`), and `start_riding(Rider::Player(id), strider)`. `MobResult` gains `mounted: Option<u64>`.
  - Each tick a player rider with `sneaking` set is dismounted (`Player.rideTick`). `Output.dismounts: Vec<(u64, [f64; 3])>` carries the spot.
  - An uncontrolled ridden strider runs its own AI and carries the player along.
- **Player mirror:** new `minecraft_survival/src/riding.rs`:
  - `Riding { vehicle: u64, uuid: [i32; 4] }` and `Player.riding: Option<Riding>` (`player.rs:17-45`);
  - `seat(vehicle_feet, attachment, bob) -> [f64; 3]` (feet + attachment - 0.6 + bob);
  - `SeatTrack`, which keeps the vehicle's last two server positions and lerps them by the entity clock's partial tick. Use this, not `ClientMobs`' three-step interpolation, so the rider is one tick behind like vanilla's own-vehicle view.
- **MW2:** 7a's `SimWorld::ride` pins the player each frame. Guns, grenades and items still work.
- **Adapter:** new `render_anim/src/minecraft_riding.rs` with one call line next to the elytra step (`minecraft_world.rs:705`).
  - Every frame: apply `mounted` results and `dismounts` (`authority.0.teleport` to the spot and clear `MC_RIDING`).
  - Pin the player to `SeatTrack`'s seat.
  - Feed `sneaking` from CROUCH into `PlayerView`.
  - The steps and falls block (`minecraft_world.rs:839`) skips a rider like noclip and clears `air_peak`.
- **Forced dismounts** (section 3): death, respawn, spectator mode (`commands.rs:342`), `/tp` and pearls (`minecraft_world.rs:1835-1846`, `commands.rs:391`), and any level switch (`player.rs:117-139`) call `Entities::dismount(player)` first. The vehicle's death clears the mirror through `dismounts`. A rider's own portal process doesn't count (`portal.rs` step: "a passenger can't use the portal").
- **Shots:** the MW2 shots and blasts of a rider skip the rider's own vehicle (`entities.rs:1154` `shot_boxes` filter), as vanilla's `Projectile.canHitEntity` skips the owner's vehicle. Melee and the crosshair pick still reach it.
- The MW2 body stays in its standing pose (MW2 has no seated animation). F5 third person works from the pinned eye.
- **Files:** `minecraft_terrain/src/{mob_actions.rs, server.rs}`, the vendored `entities/src/world/riding.rs`, `minecraft_survival/src/{riding.rs, player.rs, entities.rs, lib.rs}`, `render_anim/src/{minecraft_riding.rs, minecraft_world.rs, lib.rs}`.
- **Tests:**
  - `riding` (survival): `the_seat_is_1_1_above_the_strider_and_bobs_with_its_stride`; `the_seat_lerps_between_the_last_two_server_ticks`.
  - `mob_actions`: `right_click_mounts_a_saddled_strider_unless_crouching_or_holding_fungus`.
  - `server`: `crouching_dismounts_forward_onto_land` and `dismounting_over_open_lava_lands_on_top_of_the_strider`.
  - `render_anim`: `a_rider_counts_no_steps_and_no_fall`.

### 8. "Minecraft warped fungus on a stick steers and boosts striders"

- **Server:** the strider's controlling passenger (section 3) is decided each tick from `PlayerControls` (a stick in either hand).
  - With a controller: `tickRidden` (yaw, half pitch, `tick_boost`) and `travel` with input (0, 0, 1) and the ridden speed.
  - Without one: its own AI.
- **Boost:** `Command::Boost { player, hand }` → `ServerSim::boost_ride` → `out.item_uses.push(bool)`. True only when the player rides a strider it controls and `steering.boost(random)` succeeds.
- **Host:**
  - The use chain (before the item uses, after the mob use) sends it when the player rides and the stick is in the hand being tried. It waits with the existing `item_use_waiting` (`entities.rs:601`).
  - On `true`: swing the hand. Unless the player has infinite materials, wear 1 with Unbreaking (move the trident's `wear` from `trident.rs:84-94` into a shared `minecraft_terrain::durability::wear` used by both).
  - On break: replace the stack with a `minecraft:fishing_rod` that keeps the stick's components and has damage 0, and play the break burst.
  - On `false`: nothing, and the click does not fall through (vanilla's `PASS` would try the other hand, which this mirrors through `use_hands`).
- **Files:** the vendored `entities/src/world/striders.rs`, `minecraft_terrain/src/{server.rs, trident.rs, durability.rs, lib.rs}`, `minecraft_survival/src/{entities.rs, riding.rs}`, `render_anim/src/minecraft_riding.rs`.
- **Tests:**
  - `striders`: `a_warm_ridden_strider_reaches_4_24_blocks_a_second_and_a_cold_one_1_78` (on a lava floor and on netherrack, 100 ticks, steady speed within 1 %); `it_turns_to_the_riders_yaw_at_once`; `without_the_stick_the_rider_does_not_steer`;
  - `boost_lasts_140_to_980_ticks_and_a_second_boost_fails_meanwhile`;
  - `durability`: `a_boost_costs_one_of_a_hundred`; `unbreaking_three_spares_three_in_four`; `the_hundredth_boost_leaves_an_enchanted_fishing_rod`; `creative_boosts_cost_nothing`.

### 9. "Minecraft players riding a strider are saved with it"

- `player_nbt.rs`: `RootVehicle` joins `OWNED` (`:122`). It is written when `Player.riding` is set: `Attach` is the strider's UUID, and `Entity` is the strider's full tag with its non-player passengers, taken from the server with a `Job`.
  - On save-and-close (and later on a friend's leave, track E), the strider is removed from the level **before** its chunk saves.
  - An autosave keeps it in both places, as vanilla does.
- **On join:**
  - If the player file has `RootVehicle`, the server spawns that entity unless its UUID is already in the level (vanilla's `addWithUUID`), and the player rides it. A chunk copy with the same UUID loaded later is dropped (vanilla refuses a second entity with a known UUID).
  - If reattaching fails, the vehicle is discarded with the vanilla warning text in the log.
- **Files:** `minecraft_survival/src/{player_nbt.rs, save.rs, riding.rs}`, `minecraft_terrain/src/{server.rs, server_mobs.rs}`.
- **Tests:**
  - `player_nbt`: `a_rider_saves_root_vehicle_with_attach_and_entity`; `a_vanilla_root_vehicle_reads_back`.
  - `server`: `closing_while_riding_keeps_one_strider_in_the_player_file_and_none_in_the_chunk`; `a_chunk_copy_with_the_ridden_uuid_is_dropped`.

### 10. "Docs: Minecraft striders and riding"

- `docs/MINECRAFT-SURVIVAL.md`:
  - add striders and riding to the feature list and the strider to the simulated kinds (`:102`);
  - remove striders from N6 (`:280`) and from the dormant list (`:341`);
  - add the right-click on mobs;
  - add the limitations from section 6;
  - update the MinecraftOSS patch count.
- `docs/NETHER-END.md:241`: the strider row is present.

## 5. Owner's test

Start a fresh world in the Nether: `IW4L_MINECRAFT_WORLD=striders` and `IW4L_MINECRAFT_DIMENSION=the_nether`, in survival. Find the lava sea and build a cobblestone pier a block above it. `/tp ~ ~ ~` prints your position.

Kit (22 stacks; saddles, sticks and buckets don't stack):

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*16,minecraft:saddle,minecraft:saddle,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:warped_fungus_on_a_stick+unbreaking=3,minecraft:warped_fungus*64,minecraft:netherrack*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:lava_bucket,minecraft:lava_bucket,minecraft:lava_bucket,minecraft:lava_bucket,minecraft:water_bucket,minecraft:diamond_sword,minecraft:cooked_beef*32,minecraft:pig_spawn_egg*2,minecraft:carrot*16,minecraft:cow_spawn_egg*2,minecraft:wheat*16,minecraft:bucket
```

1. Use a strider egg on the lava sea. The strider stands on the lava surface, red, and strolls on the lava (**1**).
2. Use an egg on netherrack 5 blocks from the lava. It turns pale purple, shivers, walks slowly, and goes into the lava within about 20 seconds, where it turns red again (**2**).
3. Hold warped fungus: striders near you follow and make happy sounds. Feed two adults: hearts appear, a baby strider appears, and they eat with their sound (**3**).
4. Use a saddle on an adult: the saddle shows and its sound plays. A baby takes no saddle (**4**).
5. With the warped fungus on a stick (not the plain fungus) in hand, right-click the saddled strider: you sit on it, eyes about 2.7 blocks above its feet, and the view bobs with its stride (**5**).
6. While you hold the stick, the strider walks wherever you look, at about 4 blocks a second, without any key.
   - Right-click: it speeds up to about twice that and slows down again over 7 to 49 seconds, and the stick's durability bar drops by one. Right-click again during the boost: nothing happens and the bar stays.
   - Switch to a slot without the stick: the strider stops following your look and wanders on its own with you on it (**6**).
7. Steer onto land: it slows to under 2 blocks a second, turns pale and shivers. Back on lava it warms up (**7**).
8. Press crouch: you step off forward, onto the block you look at. Over open lava you land on top of the strider instead, never in the lava (**8**).
9. Ride again and shoot down at the strider with an MW2 gun: it takes no hit. Shoot another strider: it does.
10. Jockeys:
    - `/summon minecraft:strider ~ ~1 ~ {Passengers:[{id:"minecraft:zombified_piglin",equipment:{mainhand:{id:"minecraft:warped_fungus_on_a_stick",count:1}}}],equipment:{saddle:{id:"minecraft:saddle",count:1}}}`: the zombified piglin sits on it with bent legs and steers it (**9**).
    - `/summon minecraft:strider ~ ~1 ~ {Passengers:[{id:"minecraft:strider",Age:-24000}]}`: a baby rides the adult (**10**).
11. Kill a saddled strider with the sword: 2 to 5 string and the saddle drop (**11**).
12. Ride a strider, then save and exit. Restart: you are still sitting on the same strider, and there is no second one under you (**12**).
13. Ride or fly along the lava sea for a few minutes: wild striders appear in groups of 1 or 2, a few with a baby or a zombified piglin on top (**13**).
14. In the Overworld (start once without the dimension line), pour the four lava buckets into a 2x2 pool and use a strider egg on it: it stands on the lava. Pour water next to a strider on land: it takes damage (**14**).
15. Right-click on mobs in general:
    - Use a pig egg and saddle the pig with the third saddle; feed it carrots.
    - Use both cow eggs, feed them wheat (hearts, a calf), and milk one with the bucket. Each item leaves your inventory (**15**).

## 6. Risks, shared files and decisions

**Risks**
- **Size.** Group 1 makes every existing mob interaction live: feeding, breeding, milking, shearing, taming and golem repair. Its `would_interact` table must cover each arm, or a click is either swallowed or used twice.
- **Async results.** The server answers a mob use and a boost a tick later, and writes back changed slots of a copy. An inventory change in between (a pickup into the same slot) could be overwritten. The pickup copy has the same exposure today.
- **Server-authoritative riding.** Vanilla lets the controlling client simulate its vehicle (`ServerboundMoveVehiclePacket`). Here the server thread does it, so the host rider sees the seat one server tick behind, and a friend over the relay sees it a tick plus the round trip behind. This is the price of one truth on the server; track E may later add client prediction of the ridden vehicle.
- **The pmove riding branch** must not leave CROUCH or stance state that breaks the next normal pmove after a dismount. A test covers the round trip.
- **Lava collision** is new body collision. Restrict it to striders, so no other mob or the player starts standing on lava.
- **Saving order.** Removing the ridden strider before the chunk save depends on the close path in `save.rs`. A crash keeps both copies until the UUID rule drops one.
- **Duplicates.** The UUID-based refusal needs the entity world to know the UUIDs of loaded mobs (`mob_tags` originals keep them). Check that dormant entities are included.
- **Peaceful.** A summoned zombified piglin rider is a monster, and the existing peaceful despawn removes it at once, as vanilla does.

**Files shared with other round 8 tracks**
- **Track B (piglins, brutes, bartering, hoglins, zoglins, sleeping), run in the same worktree one group at a time:**
  - every new mob kind edits the same lists: `SIMULATED_KINDS`, `spawn_saved` and `mob_tags` in `server_mobs.rs`; `MobHit`, `entity_type`, `positions` and the `EntityKey` arms in the vendored `world.rs`, `pushing.rs` and `combat.rs`; `type_info`, rules and finalize in the natural spawner; textures in `mesh.rs`; meshes, boxes and `encode`/`decode` in `minecraft_survival/src/entities.rs`.
  - **Order in track B:** this brief runs first (b3, then b2, then b1, then b4). Group 1 (right-click on mobs) is needed by b2's hoglin feeding and b1's gold-ingot right-click; both add their arms to `interact` and `would_interact`.
  - **Piglins read held and worn gold:** b1 extends `PlayerControls` (group 3) with the armour slots rather than adding `PlayerCandidate` fields or a second per-player vector.
  - `piglin_render.rs` gets the riding pose here, and the piglin brief will rework the file.
  - **Sleeping** (b4) also pins the MW2 player. It reuses 7a's `SimWorld::ride` and `MC_RIDING` rather than adding a second bit, and dismounts a rider before lying down.
- **Track A:** the spawner's displayed mob uses the renderers (group 4 registers the strider). Mining speed and `/tick` don't touch these files.
- **Track C:** shulkers and item frames add to the same mob lists. The elytra body pose and the MW2 third-person body are in the same drawing path as the rider's standing pose. Levitation is a new pmove effect, like `MC_RIDING`.
- **Track D (skating):** `movement_iw4/src/single.rs` and the `pm_flags` bits. Keep `0x0100_0000` for riding and extend the bit test; the skate toggle is refused while riding.
- **Track E (multiplayer):** every per-player touch point here is at the server tick boundary and must take the player's id:
  - `TickInput.controls` (one entry per player);
  - `Command::MobAction.player`, `Command::Boost.player`, `Output.dismounts` (keyed by player);
  - `MobResult.mounted`;
  - `Player.riding` and `RootVehicle` on join and leave;
  - the seat pin for remote players' MW2 bodies;
  - the `PLAYER = 0` constant (`entities.rs:36`), `villager_interact(…, 0, …)` and `wolf_interact(…, 0, …)` (`mob_actions.rs:234`, `:243`) and `merchant_view(0)` (`server.rs:388`, `:446`), which group 1 routes through the player id.

**Decisions taken (vanilla-like where there was a choice)**
- Strider movement is simulated on the server thread, with the vanilla `travelRidden` numbers. The rider is pinned to the seat with one tick of lerp.
- Right-click on mobs comes before the item use, as in vanilla (`startUseItem` tries the entity first). Consume or pass is decided on the host's mob copy with a predicate tested against `interact`.
- Villagers are left out of right-click until a trading screen exists.
- Only striders carry passengers this round. Spider, chicken, zombie horse and camel jockeys keep waiting dormant.
- The MW2 body stands while riding. A seated MW2 animation does not exist.
- A rider cannot use portals, and a strider does not take its rider through one (no mob travels through portals yet).
- Not done, and listed as limitations:
  - shearing saddles off (pigs don't either);
  - the `ride_strider` advancements and the `strider_one_cm` statistic (the fork has neither);
  - tempting for the other species;
  - pig riding with a carrot on a stick, which the same system could carry later with 7 per boost and 25 durability.
