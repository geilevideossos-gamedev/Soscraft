# Track D log: finish survival

Branch `track-d`, worktree `C:\Dev\mc-wt\track-d`, target dir `D:\iw4l-wt\track-d`. Briefs: `docs/round9/track-d.md` and the mob list `docs/round9/mobs.md`.

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

Hit keys for MW2 bullets: track D uses 29 upward, one per new mob store, recorded in `docs/round9/mobs.md`.

## Group mobs-list-drowned-load

### 1. Round 9 mob list: the 52 missing Minecraft mobs

After 24dfdf71.

- **Changed:** new `docs/round9/mobs.md`: one row per missing mob from the D2 table (where it is met, what exists, what is left, size, group key, hit-key byte, status, commits), the hit-key bytes (0-28 taken, 24 and 25 unused but left to the other tracks, track D from 29 up), the batching rule (one `EntityWorld` store per vanilla family), the six-step mob pattern with its `file:line` sites re-checked at 24dfdf71, and the shared prerequisites P1-P10 with the group that builds each.
- **Tests:** none (docs only).
- **Decisions:**
  - The brief's rows 27-30 (fish) and 35-36, 37-38 (guardians, nautiluses) are split into one row per mob, so every mob has its own status.
  - Hit-key bytes 24 and 25 are free in the code but were reserved by round 8's track B; track D keeps to 29 upward as the brief says.
- **Test steps:** none (docs only).

### 2. Minecraft drowned are saved, loaded and seen by friends

After 80caed9e.

- **Vanilla (javap, 26.3):**
  - `ConversionTracker` (the zombie's drowning tracker) converts with `ConversionParams.single(mob, true, true)`: keep the equipment, keep `CanPickUpLoot`.
  - `ConversionType.SINGLE` and `convertCommon` copy to the new mob: position and motion, riding, every equipment slot and its drop chance, fall distance, on-ground, leash, absorption, effects, baby, `CanPickUpLoot`, left-handedness, `NoAI`, persistence, custom name visibility, invulnerability, no-gravity, portal cooldown, silence, the entity tags and the custom name component. The new mob gets a new UUID and its own attributes and health.
  - `Drowned.createAttributes`: the zombie's attributes plus `step_height` 1.0; navigation `AmphibiousPathNavigation`.
- **Changed:**
  - `minecraft_terrain/src/server_mobs.rs`: `minecraft:drowned` is in `SIMULATED_KINDS`, and the zombie arm of `spawn_saved` loads it as `ZombieKind::Drowned` with step height 1. A drowned with AI loads on `spawn_drowned_pursuit` (its `DrownedAttackGoal` and target goals with amphibious navigation, the fork's only drowned AI); with `NoAI` it loads inert like any zombie. New `converted_zombie_tag`: what a converted zombie keeps of the old zombie's saved tag (equipment, drop chances, `CanPickUpLoot`, `LeftHanded`, `NoAI`, `PersistenceRequired`, `CustomName`, `CustomNameVisible`, `Invulnerable`, `NoGravity`, `PortalCooldown`, `Silent`, `Tags`, `leash`), never its UUID, attributes or drowning clocks.
  - Vendored `entities/src/world.rs`: `EntityWorld.zombie_conversions` records each `(old id, new id)` of a drowning conversion (zombie to drowned, husk to zombie), taken with the new `take_zombie_conversions`. Both conversions now also keep the armor slots (`keepEquipment`). `convert_zombie_to_drowned` is public, and a zombie on the goal framework (every zombie in the game) becomes a drowned on the drowned attack profile instead of an inert one, so a fresh drowned and a reloaded one behave the same.
  - `minecraft_terrain/src/server.rs`: after the entity tick, `carry_zombie_conversions` moves the old zombie's saved tag to the new mob through `converted_zombie_tag`, as the piglin conversions already did. `mob_tags_where` writes no equipment for zombies; it keeps what the loaded tag had, so before this a converted drowned was saved with empty hands.
  - `HATCHING_EGGS` grows to 37 with `drowned`: the drowned spawn egg hatches a drowned.
  - `mob_tracking.rs` already gives drowned the monster range of 8, and the drowned's drawn state (arms raised, baby, hand item) already goes through the zombie's `MobShow`.
- **Tests:**
  - `server_mobs::tests::a_drowned_saves_and_loads_as_a_drowned`: a loaded zombie with a damaged trident, an iron helmet, a name and a UUID converts; the conversion is reported once; the saved drowned keeps the trident with its damage, the name, a new UUID, no attributes and no drowning clock; it loads into a fresh world as a drowned with its health (11), trident, helmet, step height 1 and the attack profile.
  - `mob_tracking::tests::a_friend_sees_a_drowned`: the tracker adds a baby drowned with a trident to a watcher, and `spawn_tracked` on the added tag makes it on the friend under the host's id, a baby with its trident and step height 1. (The brief said `spawn_tracked` on `look(tag)`, but `look` strips `Pos` and is only what the tracker compares; the friend spawns from the full added tag, as every other kind does.)
  - `server::tests::a_zombie_that_drowns_keeps_its_equipment_and_name_as_a_drowned`: a summoned zombie with a trident and a name converts on the server; after `carry_zombie_conversions` the drowned's saved tag keeps both and the zombie's tag is gone.
  - `server::tests::every_spawn_egg_of_a_mob_the_game_has_hatches_that_mob` with `drowned` (37 eggs).
  - Runs: `minecraft_terrain` lib 1119 of 1120 green in the full run; the one failure, `sections::tests::background_full_update_matches_the_synchronous_one`, is a timing test of the background section updates (no mob code), and it passed twice when run alone. `minecraftoss-entities` lib 230 green (`drowned_conversion_replaces_id_and_copies_common_mob_state` among them). `check -p launcher` green.
- **Decisions:**
  - A loaded drowned runs the existing drowned attack profile (`spawn_drowned_pursuit`) rather than the zombie's goals: the zombie goals would float it to the surface and walk it like a zombie. It has no wandering, water or beach goals yet; those come with the `drowned` group (D1 commit 3).
  - The friend loads the drowned through the same `spawn_saved` and runs no AI, as for every mob.
  - A converted mob keeps the vanilla `convertCommon` fields that the fork saves in the tag; effects and absorption are not kept because the zombie's effects are not saved by `mob_tags_where` yet (they are not today either).
- **Test steps (survival):**
  1. At a river edge use a zombie spawn egg on the floor of water two blocks deep, so the zombie stands under water. Wait about 45 s: it shakes and turns into a drowned (**1_drowned_converted**).
  2. Use a drowned spawn egg on the shore: a drowned appears and the egg is used. Use another on the water floor: a second one (**2_drowned_eggs**). Stand in the water with them at night (`/time set night`) or in the water by day: they come at you and hit; block with the shield.
  3. Save and quit, then load the world again: every drowned is still there, moves and attacks; one killed drops its loot (**3_drowned_after_reload**).
  4. Friend window: the friend sees the same drowned in the same places, baby drowned small, and can hit them with the sword and with a gun (**4_drowned_friend**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:drowned_spawn_egg*8,minecraft:zombie_spawn_egg*4,minecraft:shield,minecraft:iron_sword,minecraft:water_bucket,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

## Group drowned

### 1. Minecraft drowned spawn in rivers and oceans with tridents and nautilus shells

After 8fa0de3a.

- **Vanilla (javap, 26.3):**
  - `EntityTypes.DROWNED`: monster, 0.6 x 1.95, eye 1.74, client tracking range 8, not in peaceful. Baby dimensions 0.49 x 0.98, eye 0.775.
  - `SpawnPlacements`: `IN_WATER`, `MOTION_BLOCKING_NO_LEAVES`, `Drowned.checkDrownedSpawnRules`.
  - `checkDrownedSpawnRules`: false unless the block below the feet holds water (spawners skip this). `canSpawn` = not peaceful, `Monster.isDarkEnoughToSpawn` (trial spawners skip the light) and water at the feet (spawners skip this). A spawner or reinforcement spawn is allowed when `canSpawn`. In `#minecraft:more_frequent_drowned_spawns` (`#is_river`): `nextInt(15) == 0 && canSpawn`. Elsewhere: `nextInt(40) == 0 && isDeepEnoughToSpawn && canSpawn`, with `isDeepEnoughToSpawn` = `y < seaLevel - 5`. `checkSpawnObstruction` is only `isUnobstructed` (no liquid test).
  - `populateDefaultEquipmentSlots` (replaces the zombie's armour and iron weapon): `nextFloat() > 0.9`, then `nextInt(16) < 10` a trident, else a fishing rod.
  - `finalizeSpawn` after `Zombie.finalizeSpawn`: an empty offhand gets a nautilus shell when `nextFloat() < 0.03` (`NAUTILUS_SHELL_CHANCE`), with `setGuaranteedDrop`. Then, for a natural or structure spawn holding a trident: `nextFloat() < 0.5`, not a baby and a biome outside `#more_frequent_drowned_spawns` make a zombie nautilus jockey.
  - Biome spawn lists (26.3 data pack): river drowned weight 100 (count 1), frozen river 1, ocean 5, dripstone caves 95 (count 4).
  - `Mob.dropCustomDeathLoot`: per slot in `EquipmentSlot.VALUES` order with a drop chance other than 0, when the chance is above 1 (preserved) or a player hit it, `nextFloat() < chance` drops it; an unpreserved damageable item keeps `max - nextInt(1 + nextInt(max(max - 3, 1)))` damage. `Mob.getBaseExperienceReward` adds `1 + nextInt(3)` per item still worn with a drop chance of at most 1; `Zombie` multiplies by 2.5 for a baby.
- **Changed:**
  - Vendored `world/src/natural_spawner.rs`: `type_info` has `drowned` (0.6 x 1.95, `InWater`, a monster: monster walk value, removed when far, cluster 4) with the new `Rules::Drowned`. `check_spawn_obstruction` skips the liquid test for drowned, as for striders. `CreatureSpawns` reads `#minecraft:more_frequent_drowned_spawns`.
  - Vendored `world/src/natural_spawner/spawning.rs`: `drowned_spawn_rules` (the natural rules above) and the spawner rule (dark enough). `finalize_monster` finalizes a drowned as a zombie with the drowned's equipment roll instead of armour and iron weapons, then `finalize_drowned` rolls the offhand nautilus shell (saved with `drop_chances:{offhand:2.0f}`) and draws the jockey roll. Spawn eggs and `/summon` without NBT finalize the same way.
  - Vendored `entities/src/zombie.rs` and `entities/src/world.rs`: a zombie has `off_hand` and `hand_drop_chances` (0.085 each by default), kept through both drowning conversions. A dead drowned drops its hands' items as above (the shell always, a player's kill sometimes the trident or rod, worn) and earns 1 to 3 experience per item it still wears.
  - `minecraft_terrain/src/server_mobs.rs`: a zombie loads its offhand and the hands' `drop_chances`.
  - `minecraft_terrain/src/zombie_render.rs`: a zombie's offhand item is drawn in its left hand (`ItemInHandLayer`), for the host and the friend, who loads the drowned from the same tag.
- **Tests:**
  - `natural_spawner::spawning::tests::a_drowned_spawns_in_a_river_column_at_night_and_never_on_dry_land`: 2000 monster spawn rounds in a river column (water from y 50 to 62, night) add only drowned, all in the water; 2000 rounds on a sand bank add other monsters but no drowned; the drowned's `type_info`; one block of water over sand never passes the rules.
  - `natural_spawner::spawning::tests::a_drowned_rolls_a_trident_or_fishing_rod_one_time_in_ten_and_a_nautilus_shell_three_in_a_hundred`: the equipment rolls with a seeded random (6.25 % tridents, 3.75 % rods, 3 % shells).
  - `world::tests::a_drowned_always_drops_its_nautilus_shell_and_a_player_s_kill_sometimes_its_worn_trident`: 400 deaths; the shell always drops, the trident only on a player's kill and worn, and a kept trident adds 1 to 3 experience.
  - `server_mobs::tests::a_drowned_loads_its_nautilus_shell_with_its_guaranteed_drop`.
  - Runs: `minecraftoss-world` lib 77, `minecraftoss-entities` lib 231, `minecraft_terrain` lib 1121 (3 ignored), all green. `check -p launcher` green.
- **Decisions:**
  - A zombie nautilus jockey is not built: the zombie nautilus is mob 38 (group `nautiluses`). The roll is still drawn, so the level random stays in step, and it is noted as unsupported. Those drowned spawn on their own.
  - Ocean ruin drowned (generation) keep the generator's plain `finalizeSpawn` (no trident roll): porting `Zombie.finalizeSpawn` into the generator would change its random draws. They now load as live drowned instead of dormant ones. Listed under Left for later.
  - Only drowned drop their hands' items so far; other zombies' armour and weapon drops stay as they were.
- **Test steps (survival, difficulty normal):**
  1. Stand on a river bank 30 blocks from the water and `/time set night`. Within a few minutes drowned come up out of the river, and none appear on the dry bank (**1_drowned_river_night**).
  2. Use the drowned spawn egg on the water floor 16 times. About one in ten holds a trident or a fishing rod in its right hand, and now and then one holds a nautilus shell in its left hand (**2_drowned_hands**).
  3. Summon one with a shell: `/summon minecraft:drowned ~ ~ ~2 {equipment:{offhand:{id:"minecraft:nautilus_shell",count:1}},drop_chances:{offhand:2.0f}}`. Kill it: it always drops the nautilus shell with its rotten flesh (**3_drowned_shell_drop**). Kill trident drowned with the sword: now and then one drops a worn trident.
  4. Save and quit, load again: the drowned still hold their tridents, rods and shells (**4_drowned_hands_reload**).
  5. Friend window: the friend sees the same items in the same hands (**5_drowned_hands_friend**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:drowned_spawn_egg*16,minecraft:shield,minecraft:iron_sword,minecraft:water_bucket,minecraft:cobblestone*64,minecraft:cooked_beef*32,minecraft:torch*32
```

### 2. Minecraft drowned go to water by day, to the beach at night and throw tridents

After 93fe442f.

- **Vanilla (javap, 26.3):**
  - `Drowned.addBehaviourGoals`: 1 `DrownedGoToWaterGoal(1.0)`, 2 `DrownedTridentAttackGoal(1.0, 40, 10.0)`, 2 `DrownedAttackGoal(1.0, false)`, 5 `DrownedGoToBeachGoal(1.0)`, 6 `DrownedSwimUpGoal(1.0, seaLevel)`, 7 `RandomStrollGoal(1.0)`.
  - `DrownedGoToWaterGoal` (MOVE): by day (`isBrightOutside`) and out of water, ten tries of `offset(nextInt(20) - 10, 2 - nextInt(8), nextInt(20) - 10)` for a water block, then `moveTo` its bottom centre; it runs until the path is done.
  - `DrownedTridentAttackGoal` extends `RangedAttackGoal` (MOVE, LOOK, every tick): it needs a living target and a trident in the main hand; start sets aggressive and `startUsingItem`, stop clears both. Tick: `seeTime` counts ticks of line of sight; within 10 blocks with 5 ticks seen it stops walking, else `moveTo(target)`; it looks at the target (30, 30); `attackTime` counts down from 40 and at 0, if it sees the target, throws with power `clamp(dist / 10, 0.1, 1)`.
  - `performRangedAttack`: a `ThrownTrident` at the eye less 0.1, aimed at a third of the target's height with `horizontal * 0.2` added, power 1.6, uncertainty `14 - difficulty * 4`, the `entity.drowned.shoot` sound at pitch `1 / (nextFloat * 0.4 + 0.8)`. A mob's arrow has `pickup` DISALLOWED. `ThrownTrident.onHitEntity` deals 8 with the `minecraft:trident` damage type ("death.attack.trident": "%1$s was impaled by %2$s").
  - `DrownedGoToBeachGoal` extends `MoveToBlockGoal(drowned, 1.0, 8, 2)` (MOVE, JUMP, every tick): `nextStartTick` = `reducedTickDelay(200 + nextInt(200))`, then `findNearestBlock` (8 around, 2 up and down, ring order) for a block with two empty blocks above and a full top face; it starts at night, in water, at `y >= seaLevel - 3`; start clears `searchingForLand` and sets `maxStayTicks = nextInt(nextInt(1200) + 1200) + 1200`; it gives up after 1200 tries; it re-paths every 40 ticks until within 1 block of the spot above.
  - `DrownedSwimUpGoal` (no flags): at night, in water, below `seaLevel - 2`; start sets `searchingForLand`; on its ticks below `seaLevel - 1`, when the path is done or near its end, `DefaultRandomPos.getPosTowards(4, 8, (x, seaLevel - 1, z), pi / 2)`, stuck when there is none.
  - `wantsToSwim` = `searchingForLand` or a target in water; `DrownedMoveControl` adds 0.002 upward while searching or below its target.
  - `DrownedRenderer.getArmPose`: the main arm holding a trident while aggressive is `THROW_TRIDENT`; `HumanoidModel.poseRightArm` turns it to `xRot * 0.5 - pi`, `yRot 0`, and `AnimationUtils.animateZombieArms` leaves a `THROW_TRIDENT` arm alone; `bobModelPart` still bobs it. Vanilla syncs only the aggressive flag for this.
  - `AmphibiousPathNavigation.isStableDestination`: any block below that is not air.
- **Changed:**
  - New vendored `entities/src/drowned_ai.rs`: the drowned's goal state (`DrownedGoals`: the running move goal, swim-up, `searchingForLand`, the beach search and its timers, the ranged attack's `attackTime` and `seeTime`, sea level and difficulty) with the water search, the beach search and its targets, the ranged attack tick and the throw's uncertainty and pitch.
  - Vendored `entities/src/world.rs`: a drowned loaded, hatched, spawned or converted from a zombie on the goal framework (`spawn_drowned`, `convert_zombie_to_drowned`) runs the six goals inside its pursuit profile, in vanilla's priority order, on the goal ticks (every second tick) with the trident and beach goals ticking every tick. The water, beach, swim-up and stroll goals walk with amphibious navigation; `wantsToSwim` drives the swim travel and move control. A throw becomes an `EntityWorld` trident throw (`take_trident_throws`), and `PlayerHitKind::Trident` is a trident's hit on a player. `set_sea_level` gives the level's sea level; `mob_shot_mob` lets a mob's projectile hurt another mob. `spawn_drowned_pursuit` (the harness's isolated attack goal) is unchanged.
  - Vendored `entities/src/stroll.rs`: `amphibious_random_position` and `amphibious_random_position_towards` (the default random positions with the amphibious stable destination).
  - `minecraft_terrain/src/trident.rs`: `Pickup::Disallowed` (saved as `pickup:0b`, loaded back) and `ThrownTrident::thrown_by_mob`: a trident with a mob owner, no loyalty, that nobody can pick up and that despawns after 1200 ticks in the ground.
  - `minecraft_terrain/src/server.rs`: after the mobs tick, the server turns each throw into a thrown trident; the thrower's box is the trident's owner until it leaves it; a hit on a player is a `PlayerHitKind::Trident` hit of 8 from the drowned, a hit on a mob goes through `mob_shot_mob`; the level's sea level goes to the entity world.
  - `minecraft_terrain/src/zombie_render.rs`: a drowned holding a trident while aggressive raises its right arm over its head (`trident_arm`), on the host and on the friend, whose mirror gets `aggressive` through `MobShow`.
  - `minecraft_survival`: `MobAttack::Trident` (damage type `minecraft:trident`, message "trident") for the new hit kind.
  - `minecraft_terrain/src/server_mobs.rs`: a loaded drowned uses `spawn_drowned`.
- **Tests:**
  - `world::tests::a_drowned_with_a_trident_throws_it_from_range`: a trident drowned 8 blocks from a player at night throws every 40 ticks from its eye, at about 1.6 blocks a tick towards the player, with the shoot sound each time, and is aggressive while it aims; a bare drowned never throws.
  - `world::tests::a_drowned_heads_for_water_by_day_and_the_beach_by_night`: on a shore, a drowned on land by day runs the water goal into the water; at night the beach goal takes it back onto the land.
  - `drowned_ai::tests::a_drowned_finds_water_by_day_and_the_shore_from_the_water` and `the_trident_goal_throws_every_two_seconds_while_it_sees_its_target`.
  - `trident::tests::a_trident_a_drowned_threw_flies_like_a_thrown_one_and_nobody_picks_it_up`.
  - `server::tests::a_drowned_throws_its_trident_at_a_player_and_it_hits_them_for_the_drowned`: a summoned trident drowned at night on a platform throws at a player 8 blocks away; every hit is a trident hit of 8 from the drowned, its own tridents never hurt it, nobody picks them up and they save with `pickup:0b`.
  - `mob_tracking::tests::a_friend_sees_a_drowned_raise_its_trident` (the `MobShow` round trip) and `zombie_render::tests::a_drowned_raises_its_trident_over_its_head_while_it_aims`.
  - `death::tests::a_drowned_s_trident_impales_the_player`.
- Runs: `minecraftoss-entities` lib 235 and `minecraft_survival` lib 594 green; `minecraft_terrain` lib 1124 of 1125 in the full run (3 ignored), the one failure again `sections::tests::background_full_update_matches_the_synchronous_one` (the background section timing test, no mob code), which passed when run alone; `check -p launcher` green. The new `drowned_ai.rs` passes `rustfmt --check --edition 2021` on its staged content.
- **Decisions:**
  - No new `MobShow` field: the raised arm comes from the `aggressive` flag `MobShow` already carries plus the main hand in the look tag, exactly what vanilla syncs (`DrownedRenderer.getArmPose`). No protocol change.
  - The goals run inside the fork's hand-written drowned profile, not the `MonsterAi` goal framework; their order, flags and timers follow vanilla. The path to a ranged target is made again when the target moves a block or the path ends (vanilla's `moveTo(entity)` keeps the path while the target block stays).
  - A mob's thrown trident draws its spread from a random seeded by its position (vanilla's new projectile has its own unseeded random); the drowned's own random draws only the sound pitch, as in vanilla.
  - A drowned's trident saves its owner as the drowned's UUID; after a reload it is an ownerless, unpickable trident that despawns, as in vanilla when the owner is gone.
  - The trident in the drowned's raised hand uses the plain held model; vanilla switches to the `trident_throwing` model while the item is in use. Listed under Left for later.
- **Test steps (survival, difficulty normal):**
  1. At night stand on the shore of a sea or river and summon a trident drowned in the water 10 blocks away: `/summon minecraft:drowned ~ ~ ~10 {equipment:{mainhand:{id:"minecraft:trident",count:1}}}`. It raises the trident over its head and throws it about every 2 seconds (**6_drowned_trident_raised**).
  2. Raise the shield: the tridents bounce off. Without it a hit takes about 4 hearts on normal. Thrown tridents stick in the ground; you cannot pick them up and they vanish after a minute (**7_drowned_trident_shield**).
  3. `/time set day`, hatch a few drowned eggs on the beach two blocks from the water: within a few seconds they walk into the water (**8_drowned_day_water**).
  4. `/time set night` while standing far from the water: drowned in the shallows come out onto the beach; deep ones swim up towards the surface first (**9_drowned_night_beach**).
  5. Friend window: the friend sees the raised arm and the flying tridents, and a trident thrown at the friend hits them (**10_drowned_trident_friend**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:drowned_spawn_egg*16,minecraft:shield,minecraft:iron_sword,minecraft:iron_chestplate,minecraft:water_bucket,minecraft:cobblestone*64,minecraft:cooked_beef*32,minecraft:torch*32
```

## State doc changes

- `docs/MINECRAFT-SURVIVAL.md`: drowned are now saved, loaded, hatched from their egg and seen by friends; a zombie that drowns keeps its equipment and name as a drowned. Drowned spawn naturally in rivers (one in 15 rolls), deep oceans (one in 40, below sea level minus 5) and dripstone cave water in the dark, with a trident (6.25 %) or a fishing rod (3.75 %) and a nautilus shell in the offhand (3 %), which they always drop; a player's kill sometimes drops the worn trident. By day drowned on land walk back into the water; at night they swim up and come out onto the nearest beach, and they stroll in the water. A drowned with a trident raises it over its head and throws it every 2 seconds at its target within sight (8 damage, "was impaled by Drowned"); the thrown tridents cannot be picked up and despawn after a minute. Friends see the raised arm and the tridents. Zombie nautilus jockeys and the trident roll for ocean ruin drowned are missing.

## Vendored files

Patched by track D: 6 (`entities/src/lib.rs`, `entities/src/stroll.rs`, `entities/src/world.rs`, `entities/src/zombie.rs`, `world/src/natural_spawner.rs`, `world/src/natural_spawner/spawning.rs`). Added: 1 (`entities/src/drowned_ai.rs`).

## Left for later

- Ocean ruin drowned: the generator's `finalize_spawn` (`generator/src/feature/entities.rs`) gives them only `Mob.finalizeSpawn`; vanilla runs `Drowned.finalizeSpawn` with the `STRUCTURE` reason (trident roll, shell, zombie nautilus jockey).
- Zombie nautilus jockeys for natural drowned with tridents (needs mob 38, group `nautiluses`).
- The drowned's raised trident uses the plain held model; vanilla draws `item/trident_throwing` while the item is in use (`minecraft:using_item`), which needs a using flag on `mesh::HeldItem`.
