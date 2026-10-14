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

## Group trading-host

### 1. Minecraft villagers open their trading screen

After 616204a0.

- **Vanilla (26.3):** `Villager.mobInteract` opens the trading screen for an adult villager with offers that is awake and trades with nobody else; a baby, or one without offers, shakes its head; the villager spawn egg passes. `AbstractVillager.stillValid` closes the screen once the villager is beyond the player's reach plus four, dead or done trading. `MerchantMenu.removed` puts the carried stack and both payments back into the inventory of a living player (`clearContainer`); a dead or disconnected player drops them. The screen's title is the villager's display name, `entity.minecraft.villager.<profession>` ("Farmer").
- **Changed:**
  - `minecraft_terrain/src/mob_actions.rs`: `would_interact` no longer refuses villagers: a right-click on one goes to the level whenever `villager_interact` on a copy of the world would not pass (an adult or a baby, with or without offers), and never for a sleeping villager, one trading with someone else, or with the villager spawn egg in hand.
  - `minecraft_terrain/src/server.rs`: `MerchantOp::QuickMove(slot)` runs `merchant_quick_move_inventory` (a shift-click on an inventory or hotbar slot while trading).
  - `minecraft_survival/src/entities.rs`: each player's queue takes the trading news of the level thread, in the level's order: the answers to the player's own operations, a screen the tick opened (`MobResult.merchant`), then the end-of-tick "close it" (`check_merchant`). An answer's inventory slots and cursor are set in the tick's actor loop next to the mob slots, before picked items arrive, and only then is the news handed out (`take_merchant`). `merchant` sends an operation and counts it as waiting (`awaits_merchant`); `close_merchant_now` closes a player's screen at once (waits for the level thread, sets the answers' slots), for death and for leaving a level.
  - `minecraft_survival/src/menus.rs`: `Open::Merchant(MerchantScreen)` (the level's `MerchantView`, the picked offer, the held clicks). Cells 0, 1 and 2 are payment A, payment B and the result. A click on a payment is `Payment`, on the result `Result` (shift: trade as often as it goes), a shift-click on an inventory slot `QuickMove`, an offer picked `Select`; plain clicks, drags, drops and number keys on inventory slots stay on the player's own inventory. Only one operation is out at a time: further clicks are held and replayed in order once its answer came, so an answer never overwrites a newer change. Closing sends `Close` (the escape key, `update.closing` from the level, death and leaving a level through `close_merchant_now`); the cursor is left to the level's close, which puts it back with the payments. `menus::merchant` (each frame) opens, updates and closes the screen from the news and sends the waiting operations.
  - Callers: the host's `apply_input` path in `render_anim/src/minecraft_world.rs` (opens the screen in the HUD, closes it on the level's word, throws what the replayed clicks threw), the friend's menu clicks on the host (`friend_menus`), the friend's and host's level leaving and death, and a disconnect (`minecraft_survival/src/server.rs`).
  - A plain placeholder screen: `McScreen::Merchant` in `frame`, drawn by the HUD on the stonecutter's background: payment A in the stonecutter's input slot (20, 33), payment B below it (20, 52, no frame), the result at (143, 33), and the offers as the stonecutter's grid of the items they sell, with its scroller, hover and controller focus. Group `trading-screen` replaces it with the vanilla villager screen.
- **Tests:**
  - `mob_actions::tests::would_interact_agrees_with_interact`: a trading villager (with an emerald, empty-handed, not with the villager egg), one without offers, a baby, a sleeping one and one trading with someone else; `would_interact` now agrees with `interact` for villagers too, and leaves the world alone (no trade starts).
  - `server::tests::players_trade_with_villagers` (extended): one orb a trade, 3 to 6 experience for a plain trade and 5 more for the one that levels the farmer up, and Regeneration on the new level.
  - `server::tests::a_shift_click_in_the_trading_screen_moves_between_inventory_and_hotbar`, `closing_the_trading_screen_gives_the_payments_back_and_drops_what_does_not_fit` (a full inventory: the payments and the cursor drop at the feet), `the_trading_screen_closes_once_its_villager_is_out_of_reach` (asked once beyond 8 blocks, asked again for a new screen).
  - `entities::tests::trading_screens_reach_only_their_player_and_set_the_slots_before_anything_else`: the friend's news reaches only the friend, in the order answer, opened, closing; the answer's slots and cursor are set before any news is handed out, and the waiting count drops by one.
  - `menus::tests::a_villager_s_trading_screen_turns_clicks_into_trades` (the click to operation table, the title, plain inventory clicks stay local, closing sends `Close`), `clicks_wait_for_the_answer_to_the_click_before_them_and_follow_in_order` (held clicks replay in order after each answer, the cursor waits for the level's close), `the_server_closes_the_trading_screen_and_friends_turn_it_down_for_now` (the level's close, a vanished villager, a friend and a player with another screen open answer `Close`).
  - `hud` layout test: the placeholder's payment B at (20, 52).
  - Runs: `minecraft_terrain` lib 1128 green (3 ignored), `minecraft_survival` lib 598, `render_anim` 149, `hud` 151, `frame` 24, all green; `check -p launcher` green.
- **Decisions:**
  - The level thread keeps the trading menu (vanilla-traced and tested); the client side keeps one operation in flight and holds the rest, rather than predicting the menu locally.
  - A dead or leaving player's screen closes at once with the payments back in the inventory, and the death drop or the saved player file takes them from there. Vanilla drops them at the feet for a dead or disconnected player; the result differs only with `keepInventory` (the payments are kept) and on a disconnect (they stay in the inventory).
  - Friends do not trade yet: the host turns a friend's trading screen down at once (`Close`, nothing was paid in), because showing it needs `MenuKind::Merchant`, which comes with the protocol bump in group `trading-friend`. Until then `kind_of` gives the never-sent `MenuKind::Inventory` for a merchant menu.
  - The title is the profession's name; a villager's custom name is not used yet.
  - Number keys, Q and double-click gathering on the payment and result slots, and drags across them, do nothing (there is no level operation for them); vanilla allows them. Listed under Left for later.
- **Test steps (survival, host):**
  1. `/locate structure minecraft:village_plains`, then `/tp` to it. The villagers, the iron golem and the animals are there (**11_village**).
  2. Right-click a villager that has a job (a farmer by a composter, a librarian at a lectern; or hatch a villager egg next to the composter from the kit and wait until it takes the job). The trading screen opens with the profession as its title ("Farmer"), the offers as item icons in the grid, payment slots on the left and the result on the right. It still uses the stonecutter's background; the vanilla look comes next (**12_trading_open**).
  3. Click an offer: its payments move from your inventory into the payment slots and the result shows. Take the result; then shift-click the result to trade it as often as your items last. Shift-click wheat in the inventory: it moves to the hotbar and back. After two or three trades the villager levels up (green particles) and more offers appear in the grid (**13_trading_level_up**).
  4. Keep using one offer until the result no longer appears for it (sold out). The next morning, after the villager has worked at its job block, the offer works again.
  5. Put payments in the slots, then walk away about 8 blocks with the screen open: it closes and the payments are back in the inventory. Do it again and press Escape instead: the same (**14_trading_walk_away**).
  6. Hit a villager twice and open its screen again: the payments it asks for are higher. Right-click a baby villager: it shakes its head and no screen opens (**15_baby_villager**).
  7. Friend window: right-clicking a villager opens nothing yet (friends trade with group `trading-friend`), and the villager can still be traded with by the host straight after.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:emerald*64,minecraft:emerald*64,minecraft:wheat*64,minecraft:paper*64,minecraft:stick*64,minecraft:coal*64,minecraft:string*64,minecraft:rotten_flesh*64,minecraft:iron_ingot*32,minecraft:villager_spawn_egg*4,minecraft:composter,minecraft:lectern,minecraft:fletching_table,minecraft:smithing_table,minecraft:red_bed,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

## Group trading-screen

### 1. The Minecraft trading screen looks like vanilla

After 0535f063.

- **Vanilla (javap, 26.3):**
  - `MerchantMenu`: payment slots at (136, 37) and (162, 37), the result at (220, 37), the inventory from (108, 84) and the hotbar at (108, 142).
  - `MerchantScreen`: 276 x 166 from the 512 x 256 `textures/gui/container/villager.png`; inventory label at x 107.
    - **Offer buttons:** seven `TradeOfferButton`s (`Button.Plain`, 88 x 20) at (5, 18 + 20i).
    - **Offer rows:** each row's items sit at y 19 + 20k: the first cost at x 10, the second cost at 40, the arrow (10 x 9) at 60, y + 3, and the result at 73.
    - **Discount:** when the first cost's count differs from its base count, the base count is drawn at the item and the new count 14 to the right (a count of 1 is written out), with `discount_strikethrough` (9 x 2) at (+7, +12).
    - **Sold out:** a sold-out offer draws `trade_arrow_out_of_stock`. The picked offer (`shopItem`, 0 until one is picked) draws `out_of_stock` (28 x 21) at (182, 35) when it is sold out. Hovering there (22 x 21 from (186, 35), one pixel of slack) on a merchant that restocks shows `merchant.deprecated`.
    - **Scroller:** 6 x 27 at (94, 18) on a 139-pixel bar, only with more than 7 offers. Its y is `min(113, scrollOff * (1 + rest / n + 139 / n))`, or 113 on the last step, with `n = offers - 6` and `rest = 139 - (27 + (n - 1) * 139 / n)`. Otherwise `scroller_disabled`.
    - **Scrolling:** the wheel scrolls `scrollOff - scrollY`, anywhere on the screen. A press with x strictly between 94 and 100 and y in (18, 158] starts a drag, and dragging sets `(int)(((y - 18 - 13.5) / 112) * (offers - 7) + 0.5)`, clamped.
    - **Clicking a button:** picks offer `index + scrollOff`, with the left button only, and the button keeps the focus highlight.
    - **Button tooltips:** x below the button's x + 20 shows the first cost (the discounted stack), x strictly between +30 and +50 the second cost, and x above +65 the result.
    - **Experience bar:** shown only when `showProgressBar` and the level is below 5. The background is 102 x 5 at (136, 16). The current part is `min(floor(102 / (max - min) * (xp - min)), 102)` wide, and only drawn when xp >= the level's minimum and the level can rise. The future part is `min(floor(futureXp * ratio), 102 - current)` wide and starts where the current part ends.
    - **Labels:** the title is `merchant.title` ("%s - %s") with `merchant.level.<n>` when the level is 1 to 5 and the progress bar shows, otherwise the plain title. It is centred on 49 + 138, y 6. "Trades" (`merchant.trades`) is centred on 53, y 6. All labels use colour 0xFF404040 without shadow.
- **Changed:**
  - `frame/src/minecraft_ui.rs`:
    - New `McOffer`: the first cost at its modified count, its base count, the second cost, the result and whether it is sold out.
    - New `McMerchant`: the offers, level, xp, future xp, whether the progress bar shows and whether the merchant restocks.
    - New field `MinecraftUi.merchant`.
  - `render_anim/src/minecraft_gui.rs`: the `villager` sheet, the `container/villager/` sprites and the `merchant.` texts load into the GUI atlas.
  - `render_anim/src/minecraft_inventory.rs`: the trading screen fills `ui.merchant` from the `MerchantScreen`. The first cost comes from `MerchantOffer::cost_a_count` with the item's stack size the level sent (`max_stacks`), and the costs keep their component predicates for the icons. The offers no longer go through `ui.choices`.
  - New `hud/src/minecraft_merchant.rs`: the screen's numbers above, the offer-row hit test, the scroller, the wheel and drag scrolling, the experience bar widths, the title, the tooltips and the drawing.
  - `hud/src/minecraft_screens.rs`: the `Merchant` layout uses the vanilla slot positions, the `villager` sheet, and the merchant background, labels and tooltip. The placeholder's stonecutter look is gone.
  - `hud/src/minecraft_inventory.rs`: the trading input.
    - A left click on an offer row picks it (`Choose(index)`) and keeps that row highlighted.
    - The scroller drags and the wheel scrolls, both reset when the screen opens.
    - Hovering an offer's columns shows the item tooltips.
    - With a controller, LB and RB move the highlight through the offers and scroll with it, and RT picks the highlighted offer (like the enchanting table, because picking moves the payments).
  - `minecraft_terrain/src/server.rs`: `MerchantView.name` carries the villager's custom name from its saved tag (`CustomName`, plain text or the component's `text`).
  - `minecraft_survival/src/menus.rs`: the screen's title is that name when there is one, otherwise `entity.minecraft.villager.<profession>`.
- **Tests:**
  - `minecraft_screens::tests::station_slots_sit_where_vanilla_puts_them`: the merchant's size (276 x 166), sheet, payment and result slots, inventory and hotbar positions, and its 39 slots.
  - `minecraft_merchant::tests::offer_rows_take_clicks_on_the_vanilla_buttons`: the button edges, rows 0, 1 and 6, rows past the offers, and a row with the list scrolled.
  - `minecraft_merchant::tests::nine_offers_scroll_two_steps_by_wheel_and_drag`: 9 offers scroll 0 to 2, the shown range, wheel clamping, drag positions at the top, middle and bottom, the scroller at 0, 53 and 113, and the scroller's grab area.
  - `minecraft_merchant::tests::a_discount_strikes_the_old_price_and_a_sold_out_offer_crosses_its_arrow`.
  - `minecraft_merchant::tests::the_experience_bar_fills_toward_the_next_level`.
  - `minecraft_merchant::tests::the_title_names_the_level_and_tooltips_follow_the_columns`: "Farmer - Apprentice", the plain title without the bar, the tooltip columns, the hovered stack with the list scrolled, and the restock hint only on a sold-out picked offer.
  - `minecraft_inventory::tests::a_trading_offer_shows_its_discounted_first_cost_its_second_cost_and_what_it_sells` (render_anim): a special price of -3 on 10 emeralds shows 7, the book as the second cost, the enchanted book keeps its components, and a price above a stack shows 64.
  - `minecraft_gui::tests::the_real_gui_textures_and_fonts_load_into_one_atlas`: the villager sheet and the scroller (6 x 27) and strikethrough (9 x 2) sprites load.
  - `server::tests::a_named_villager_s_trading_screen_carries_its_name`.
  - `menus::tests::a_named_villager_s_trading_screen_takes_its_name_as_the_title`.
  - Runs: `hud` 156, `render_anim` lib 150, `minecraft_survival` lib 599, `frame` 24, `minecraft_terrain` lib 1129 (3 ignored), all green. `check -p launcher` green. The new `minecraft_merchant.rs` passes `rustfmt --check --edition 2024` on its staged content.
- **Decisions:**
  - Every merchant today is a villager, so `show_progress` and `can_restock` are both true. The wandering trader (group `wandering-trader`) sets both to false.
  - The clicked row keeps its highlight while the list scrolls, as vanilla's focused button does. With a controller, the highlighted row follows the offer the pad points at.
  - With a controller, moving through the offers does not pick them. Only RT picks, because picking moves payments out of the inventory. The stonecutter picks as it moves.
  - The villager's custom name rides on `MerchantView`, read from the mob's saved tag. The fork has no name tags on mobs yet, so a name comes from `/summon` or a saved world.
  - The hud keeps its own copy of the villager level thresholds (0, 10, 70, 150, 250) for the bar. It has no dependency on the vendored entities crate.
- **Test steps (survival, host):**
  1. Put a composter down and hatch a villager egg next to it, or go to a village. Wait until the villager is a farmer, then right-click it. The wide vanilla trading screen opens. On the left, "Trades" sits over the offer buttons, each showing the payment, an arrow and the result. On the right, "Farmer - Novice" is centred over an empty experience bar, with the two payment slots and the result, and the inventory sits below (**16_trading_screen**).
  2. Hover over an offer's payment, second payment and result: each shows its item tooltip.
  3. Click an offer: its button stays highlighted, the payments move into the slots and a light green part on the bar shows the experience the trade gives. Take the result: the bar fills, and after a few trades the title becomes "Farmer - Apprentice" with two more offers (**17_trading_xp_bar**).
  4. Summon a librarian that is about to level: `/summon minecraft:villager ~ ~ ~2 {CustomName:"Bob",Xp:249,VillagerData:{type:"minecraft:plains",profession:"minecraft:librarian",level:1}}`. Fence it in. Make four trades, closing and reopening the screen between them if it does not level up straight away. It goes to "Bob - Master", the bar is gone, and there are 9 offers.
  5. With the 9 offers, the scroller on the right of the buttons is lit. The mouse wheel scrolls two steps, and dragging the scroller does the same (**18_trading_scroll**).
  6. Buy one cheap offer until it is sold out: its arrow gets a red cross. While it is picked, a big red cross sits between the payments and the result, and hovering over it says "Villagers restock up to two times per day." (**19_trading_sold_out**).
  7. Hit a villager once and right-click it again. The first payment of each offer shows the old price struck through with the new, higher price next to it (**20_trading_price_up**).
  8. Controller: LB and RB move the highlighted offer, scrolling with it, and RT picks it.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:emerald*64,minecraft:emerald*64,minecraft:emerald*64,minecraft:paper*64,minecraft:paper*64,minecraft:book*64,minecraft:wheat*64,minecraft:string*64,minecraft:stick*64,minecraft:coal*64,minecraft:rotten_flesh*64,minecraft:iron_ingot*32,minecraft:villager_spawn_egg*4,minecraft:composter,minecraft:lectern,minecraft:fletching_table,minecraft:red_bed,minecraft:oak_fence*64,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

## Group trading-friend

### 1. Friends trade with Minecraft villagers

After ec8dd240.

- **Vanilla (26.3):** `ClientboundMerchantOffersPacket` carries the container id, the offers, the villager level, its xp, `showProgress` and `canRestock`. `MerchantOffer.STREAM_CODEC` sends per offer the first cost (`ItemCost`: item, count, component predicate), the result stack, the optional second cost, out of stock, uses, max uses, xp, special price, price multiplier and demand. The client works out the discounted first cost itself (`getModifiedCostCount` with the item's max stack size) and keeps the picked offer (`shopItem`) locally, telling the server with `ServerboundSelectTradePacket`. The server runs every click on the real `MerchantMenu`.
- **Changed:**
  - `minecraft_net/src/messages.rs`:
    - **`PROTOCOL` goes from 3 to 4.** This is track D's only bump.
    - New `MenuKind::Merchant`.
    - New `MenuExtras::Merchant { villager, profession, level, xp, future_xp, show_progress, selected, offers: Vec<NetOffer> }`.
    - New `NetOffer`: the first cost, the second cost and the result as item NBT (`id`, `count` as an int, `components`), uses, max uses, xp, special price, demand, price multiplier and the reward flag.
    - `MenuExtras` is no longer `Eq`, because the price multiplier is a float. Nothing needed it.
  - `minecraft_survival/src/menus.rs`:
    - `kind_of` gives `MenuKind::Merchant` for a trading screen (it gave the never-sent `Inventory`).
    - `snapshot` builds the merchant extras from the player's `MerchantScreen`: `net_offer` for each offer, and the picked offer as `selected`.
    - `sync` already sends a fresh `MenuOpen` when the extras change. So every trade, level-up, payment (future xp) and picked offer reaches the friend. The payment and result cells go as the menu's three slots, and the cursor as `carried`.
    - `mirror_open` builds `Open::Merchant` on the friend (`mirror_merchant`):
      - the offers come back through `offer_of`;
      - the profession is matched to `Profession::ALL` (`minecraft:none` when unknown);
      - the payments and result come from the slots, `selected` from the extras, and there are no held clicks;
      - `max_stacks` stays empty: the friend's screen takes each item's stack size from its own recipe book, as the vanilla client takes it from its item registry.
    - New public `net_offer` and `offer_of`.
  - `render_anim/src/minecraft_players.rs`: `friend_menus` lets the trading news open a friend's screen (`shows` is now true).
    - A friend's right-click on a villager opens the same level-thread menu the host uses, keyed by the friend's player id.
    - The friend's clicks already came to `menus::click` on the host and become the same `MerchantOp`s.
    - Death, leaving the level and a disconnect already closed a friend's screen through `close_merchant_now`.
  - The friend's side needed no new code: `show_menus` and the inventory publishing in `render_anim` already turn `mirror` into the HUD's `McScreen::Merchant` and `ui.merchant`.
  - `minecraft_terrain/src/mob_tracking.rs`: `Offers` and `Xp` are in `VOLATILE`, so a trade no longer resends the villager's look tag to every watcher. A level change still does, for the badge.
- **Tests:**
  - `codec::tests::every_message_to_the_client_round_trips` now holds a merchant menu with two offers:
    - a discounted one (special price -3, demand 4, a second cost);
    - a sold-out one (16 of 16 uses, no second cost, no reward).
  - `codec::tests::a_trading_screen_keeps_its_discounted_and_sold_out_offers`: the decoded extras keep the villager, the profession, the picked offer, and both offers' prices, uses and flags. An empty merchant round-trips too.
  - `menus::tests::an_offer_crosses_the_network_with_its_costs_components_and_prices`: an enchanted book offer comes back equal, with the same discounted first cost. It has a component cost, a second cost, a special price, demand and no reward. A sold-out offer stays sold out.
  - `menus::tests::a_friend_s_trading_screen_is_sent_and_mirrored_with_its_offers`:
    - The host opens a friend's screen and sends `MenuKind::Merchant` with three slots and the extras.
    - The mirror gives `Open::Merchant` with equal offers, the discounted cost (20 - 4 = 16) and the sold-out flag.
    - The friend's pick goes out as `Select`. After the answer, a fresh `MenuOpen` with the same menu id carries the pick, the future xp, the payment and the result.
    - A friend's shift-click on the result becomes `Result { shift: true }`.
    - The friend's own close sends `Close` and no `MenuClose` back. The level's close sends `MenuClose`.
  - `menus::tests::the_server_closes_the_trading_screen_and_a_player_who_cannot_show_it_turns_it_down`: renamed from `..._and_friends_turn_it_down_for_now`, with the same checks. A player whose screen cannot show still answers `Close`.
  - `mob_tracking::tests::a_trade_changes_no_look_but_a_new_level_does`.
  - Runs: `minecraft_net` 16, `minecraft_survival` lib 601, `render_anim` lib 150 and `minecraft_terrain` lib 1130 (3 ignored), all green. `check -p launcher` is green.
- **Decisions:**
  - **The picked offer travels in the extras (`selected`)** instead of living on the friend.
    - The host already keeps it in `MerchantScreen.selected`, and the HUD reads it as `ui.chosen`. So the friend's highlight and the big sold-out cross follow the host after one round trip.
    - Vanilla keeps it on the client. The outcome on screen is the same.
  - `NetOffer` also carries the offer's xp, as vanilla's codec does, besides the reward flag the brief named. So the friend's offer equals the host's.
  - Costs and results go as NBT built from the offer's id, count and components. The count is an int, so counts above a stack survive. They don't go through `slot_nbt`, which caps counts at 99 and would need an `ItemStack`.
  - `show_progress` is always true until the wandering trader (group `wandering-trader`) exists. Like the host's, the friend's screen still shows the bar for every merchant.
  - Each change of the extras resends the whole menu (three slots plus the offers), as the anvil and enchanting extras already do. A friend's click made before the resend arrives is stale: it still acts and asks for a resend, as for every menu.
- **Test steps:** two windows, survival, difficulty normal. Start them with `scripts\mp-local.ps1 -Exe iw4l-survival.exe -World mp-r9-trading -Fresh -FriendKit`.
  1. Host: put a composter down and hatch a villager egg next to it, wait until it is a farmer, and fence it in. Friend: walk up and right-click it. The friend gets the same vanilla trading screen: "Farmer - Novice", the offer buttons, the payment and result slots and the experience bar (**21_friend_trading_screen**).
  2. Friend: hover the offers to see the tooltips, then click one.
     - It stays highlighted, the payments move from the friend's inventory into the slots, and the result shows.
     - Take the result, then shift-click it to trade as often as the items last.
     - The friend's emeralds or wheat change in the hotbar and the bar fills. Green particles show over the villager when it levels up (**22_friend_trade_done**).
  3. Host, while the friend's screen is open: right-click the same villager. Nothing opens, because it trades with the friend. Friend: press Escape. Host: right-click again. The host's screen opens and shows the uses the friend's trades used up (**23_host_after_friend**).
  4. Friend: buy one cheap offer until it is sold out. Its arrow gets the red cross. While it is picked, the big cross between the payments and the result shows (**24_friend_sold_out**).
  5. Friend: put payments in, then walk 8 blocks away with the screen open. It closes and the payments are back in the friend's inventory.
  6. Friend: hit the villager once and right-click it again. The friend's screen shows the old price struck through with the higher one next to it. Reputation is per player, so the host's prices stay as they were (**25_friend_price_up**).
  7. Friend: open the screen with payments in the slots, then the host runs `/kill <friend's name>`. The screen closes, and the payments are in the friend's death drop.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:emerald*64,minecraft:emerald*64,minecraft:emerald*64,minecraft:paper*64,minecraft:book*64,minecraft:wheat*64,minecraft:wheat*64,minecraft:string*64,minecraft:stick*64,minecraft:coal*64,minecraft:rotten_flesh*64,minecraft:iron_ingot*32,minecraft:villager_spawn_egg*4,minecraft:composter,minecraft:lectern,minecraft:fletching_table,minecraft:red_bed,minecraft:oak_fence*64,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

## Group zombie-villagers

### 1. Zombies turn the villagers they kill into zombie villagers

After d8de1bcb.

- **Vanilla (javap, 26.3):**
  - `Zombie.killedEntity` covers every zombie kind: zombie, husk, drowned, zombie villager and zombified piglin.
    - It only acts on Normal or Hard, and only when the victim is a `Villager`.
    - On Normal it returns early when the zombie's own `random.nextBoolean()` is true. Hard never returns early.
    - Then it calls `convertVillagerToZombieVillager`. A conversion makes `killedEntity` answer false, so `LivingEntity.die` drops no loot and sends no death event for the villager. Its death sound has already played, and `Villager.die` has already told the witnesses and released its points of interest.
  - `convertVillagerToZombieVillager` calls `villager.convertTo(ZOMBIE_VILLAGER, ConversionParams.single(villager, true, true))`.
    - The two flags keep the equipment and `CanPickUpLoot`.
    - `ConversionType.SINGLE` copies the common fields listed under group mobs-list-drowned-load and gives a new UUID.
    - Then, in order: `setVillagerDataFinalized(villager's)`, `finalizeSpawn(CONVERSION, new ZombieGroupData(false, true))`, `setVillagerData`, a copy of the gossips, a copy of the offers and `setVillagerXp`.
    - Last, level event 1026 at the zombie's block, unless the zombie is silent.
  - `ZombieVillager.finalizeSpawn` first runs `finalizeVillagerType`. It sets the biome's type only when the data is not finalized, and the villager's data overwrites it anyway. Then it runs `Zombie.finalizeSpawn`:
    - `Mob.finalizeSpawn`: the follow range's random bonus and left-handedness, from the level random.
    - No `CanPickUpLoot` roll for `CONVERSION`.
    - Not a baby from the group data.
    - `setCanBreakDoors(nextFloat < special * 0.1)`.
    - No equipment or enchantments for `CONVERSION`.
    - The Halloween head.
    - `handleAttributes` from the zombie's own random. The reason makes no difference there.
  - `LevelEventHandler` 1026 plays `entity.zombie.infect` (HOSTILE) at the block, volume 2.0, pitch `(nextFloat - nextFloat) * 0.2 + 1.0`.
  - `ZombieVillager.removeWhenFarAway` is false while it converts or while its `Xp` is not 0.
- **Changed:**
  - Vendored `entities/src/world.rs`:
    - A zombie's killing blow on a villager goes through `zombie_killed_villager`. That covers its melee hit and any hit through `mob_hits_mob`, including a drowned's trident.
    - It applies the difficulty gate above. Then it drops the villager's death record (no loot, no experience), releases its points of interest and adds `(zombie, villager)` to the new `villager_infections` list (`take_villager_infections`).
    - `check_despawn` never removes a zombie villager with villager experience for being far away.
  - Vendored `entities/src/zombie.rs`: new `Zombie.villager_xp`, loaded from a zombie villager's `Xp`.
  - Vendored `entities/src/world/villagers.rs`: new `release_villager_pois` releases one villager's points of interest at once.
  - Vendored `world/src/natural_spawner/spawning.rs`:
    - New `CreatureSpawns::finalize_conversion`: the zombie villager's `CONVERSION` finalize above, with a new UUID from the level.
    - New `zombie_attributes`: the zombie's `handleAttributes`, moved out of `finalize_zombie` unchanged so both use it.
  - Vendored `world/src/level/spawning.rs`: new `Level::finalize_conversion`.
  - `minecraft_terrain/src/server_mobs.rs`: new `infected_villager_tag` builds the zombie villager's tag from the villager.
    - It starts from the kept fields of the villager's saved tag, as `converted_zombie_tag` does.
    - It adds the villager's position, motion, yaw, baby, persistence, `CanPickUpLoot`, `VillagerData`, `VillagerDataFinalized`, `Xp`, offers, gossips, held item and effects.
    - It has no health and no attributes, because the new mob has its own.
  - `minecraft_terrain/src/server.rs`:
    - After the mobs tick, `infect_villagers` finalizes that tag and removes the villager.
    - It adds the zombie villager like a loaded mob, so the friend gets it through the tracker like any new mob.
    - It sends level event 1026 unless the zombie is `Silent`.
    - **The entity world now gets the level's difficulty every tick** (`set_difficulty`). Until now it stayed at Normal, so every mob rule that depends on the difficulty ran as on Normal: this gate, skeleton fire rates, trident and arrow spread, and husk hunger.
  - `minecraft_terrain/src/level_events.rs`: event 1026 plays `entity.zombie.infect` for the host and the friend. The friend already plays the host's level events.
- **Tests:**
  - `world::tests::a_zombie_s_killing_blow_infects_a_villager_always_on_hard_half_the_time_on_normal_and_never_below`:
    - 200 kills on each difficulty infect 200 on Hard, about 100 on Normal, and none on Easy or Peaceful.
    - An infected villager leaves no death to drop loot. A villager that stays dead does.
    - A skeleton's kill never infects.
  - `world::tests::a_zombie_villager_that_was_a_traded_villager_never_despawns_far_away`.
  - `server::tests::a_zombie_turns_the_villager_it_kills_into_a_zombie_villager_with_its_trades`:
    - On Hard, a zombie kills a named desert farmer of level 2 with 12 xp, an offer, gossip and `NoAI`. The villager is gone.
    - The zombie villager stands where the villager stood, with 20 health. It has the same type, profession, level, xp, offer, gossip, name and `NoAI`, `VillagerDataFinalized` and a new UUID.
    - Level event 1026 plays at the zombie's block.
    - Its saved tag loads back as a zombie villager with the same data.
    - On Easy a second kill infects nobody and sends no event.
  - `level_events::tests::a_zombie_infecting_a_villager_groans_loudly_where_it_stands`.
  - Runs: `minecraftoss-entities` lib 237, `minecraftoss-world` lib 77 and `minecraft_terrain` lib 1132 (3 ignored), all green. `check -p launcher` is green.
- **Decisions:**
  - The villager's witnesses only hear of the murder when the villager's own tick comes before the server removes it. Vanilla always tells them. The gossip is about a zombie, which no villager trades with, so nothing a player sees changes.
  - A drowned's trident kill infects too, one tick later, because vanilla's `killedEntity` goes to the trident's owner. The trident's hit lands after the mobs' tick.
  - The zombie villager is built as a tag and loaded like a saved mob, not made inside the entity world. That way it carries every saved field the entity world does not hold (offers, gossips, name), and a new one behaves exactly like one loaded from a save.
  - Giving the entity world the level's difficulty is a fix beyond the brief. The gate needs it, and `docs/MINECRAFT-SURVIVAL.md` already says that mob behaviour follows the difficulty.
- **Test steps (survival):**
  1. `/difficulty hard` and `/time set night`. Build a closed 3 x 3 box of planks with a roof, so no sun reaches inside.
     - Inside, summon a farmer that has traded: `/summon minecraft:villager ~ ~ ~2 {VillagerData:{type:"minecraft:plains",profession:"minecraft:farmer",level:2},Xp:12}`.
     - Hatch a zombie egg next to it. The zombie kills the villager and a loud groan plays.
     - A zombie villager in the farmer's straw hat stands where the villager died, and nothing drops (**26_villager_infected**).
  2. Look at the zombie villager closely: green skin, the plains overlay and the farmer's hat and clothes (**27_infected_outfit**).
  3. Save and quit, then load again: it is still a zombie villager in the farmer's clothes. Fly 130 blocks away and come back: it is still there, because a villager that traded never despawns.
  4. `/difficulty normal`: repeat step 1 four times with plain villager eggs. About half of the villagers turn, and the rest simply die. Then `/difficulty easy`: villagers never turn (**28_infection_normal_easy**).
  5. Friend window: the friend sees the villager vanish and the zombie villager appear in its place, and hears the groan (**29_infected_friend**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:villager_spawn_egg*8,minecraft:zombie_spawn_egg*8,minecraft:oak_planks*64,minecraft:oak_planks*64,minecraft:torch*32,minecraft:diamond_sword,minecraft:cooked_beef*32
```

### 2. A golden apple cures a weakened Minecraft zombie villager

After c61f2757.

- **Vanilla (javap, 26.3):**
  - `ZombieVillager.mobInteract` with a golden apple:
    - With Weakness: `ItemStack.consume(1, player)` (nothing is used up in creative), then `startConverting(player UUID, random.nextInt(2401) + 3600)`, answering `SUCCESS_SERVER`.
    - Without Weakness: `CONSUME`. The click is taken, the apple is kept and the player does not eat it.
    - Any other item goes to `Zombie.mobInteract`.
  - `startConverting(uuid, time)`: sets the starter and the time and the synced `DATA_CONVERTING_ID`. Removes Weakness, adds Strength for `time` ticks at amplifier `min(difficulty - 1, 0)`, and broadcasts entity event 16.
  - Entity event 16 (`handleEntityEvent`): `entity.zombie_villager.cure` at the eye, volume `1 + nextFloat`, pitch `nextFloat * 0.7 + 0.3`, unless silent.
  - `tick`, before `Zombie.tick`: while alive and converting, the time drops by `getConversionProgress()`, and at 0 or below `finishConversion` runs.
  - `getConversionProgress`: 1. On a `nextFloat() < 0.01`, it walks x, y and z from the truncated `(int) getX() - 4` up to `+ 4` (exclusive), stopping after 14 blocks of `#speeds_up_zombie_villager_curing` (iron bars and `#beds` in the 26.3 data pack). Each block found adds 1 when `nextFloat() < 0.3`.
  - `finishConversion`: `convertTo(VILLAGER, ConversionParams.single(this, false, false))`, so no equipment and no `CanPickUpLoot` carry over. Then:
    - `dropPreservedEquipment` drops items with a drop chance above 1. Binding-cursed items move onto the villager.
    - `setVillagerDataFinalized`, `setVillagerData`, the gossips (when there are any), a copy of the offers (when there are any) and `setVillagerXp`.
    - `finalizeSpawn(CONVERSION, null)`: `finalizeVillagerType` (the biome's type only when not finalized), `AbstractVillager` and `AgeableMob` with no roll for the first of a group, then `Mob.finalizeSpawn`.
    - `refreshBrain`.
    - When the starter is a player in this level (`ServerLevel.getPlayerByUUID`): the advancement trigger and `ZOMBIE_VILLAGER_CURED`. In `Villager.onReputationEventFrom` that is +20 `major_positive` and +25 `minor_positive` gossip about the player.
    - Nausea for 200 ticks, and level event 1027 at the block unless silent.
  - Saved data: `ConversionTime` (always written, -1 when not converting) and `ConversionPlayer` (a UUID, written when known). Loading with a time other than -1 runs `startConverting`.
  - `LevelEventHandler` 1027: `entity.zombie_villager.converted` (HOSTILE), volume 2.0, pitch `(nextFloat - nextFloat) * 0.2 + 1.0`.
  - `AbstractZombieRenderer.isShaking` is true while a zombie converts under water, and `ZombieVillagerRenderer` adds `isConverting`. A shaking body turns by `cos(ageInTicks * 3.25) * PI * 0.4` degrees.
  - `MobEffects.STRENGTH`: beneficial, colour 16762624, +3 `attack_damage` a level.
- **Changed:**
  - Vendored `entities/src/zombie.rs`:
    - New fields `villager_conversion_time` (-1 when not curing) and `conversion_starter`.
    - New `curing()` and `shaking()` (curing, or drowning into a drowned).
    - New `cure_progress`, the progress above.
  - Vendored `entities/src/world.rs`:
    - New `start_curing(zombie, player)`: `None` for any zombie but a living zombie villager, `Some(false)` without Weakness. Otherwise it rolls the time from the zombie's random and records the player's UUID. It swaps Weakness for Strength and makes the cure sound at the eye.
    - A curing zombie villager counts down first thing in its tick, with iron bars and beds speeding it up.
    - When the time runs out, the zombie goes on the new `zombie_cures` list (`take_zombie_cures`), stops curing and skips the rest of that tick.
    - `check_despawn` keeps a curing zombie villager like one with villager experience.
  - Vendored `entities/src/effects.rs`: new `MobEffect::Strength` (`minecraft:strength`), its +3 a level in `attack_damage_modifier`, and `MobEffects::remove`. Vendored `entities/src/potion.rs`: Strength's colour.
  - Vendored `world/src/natural_spawner/spawning.rs`: `finalize_conversion` has a villager branch. It gives the biome's type when the data is not finalized, then runs `Mob.finalizeSpawn`'s part.
  - `minecraft_terrain/src/mob_actions.rs`: a golden apple on a zombie calls `start_curing`. A started cure uses one apple (not in creative), a zombie villager without Weakness takes the click and keeps the apple, and any other zombie lets the player eat.
  - `minecraft_terrain/src/server_mobs.rs`:
    - A zombie villager saves `ConversionTime` and, while curing, `ConversionPlayer`, and loads both back.
    - New `cured_villager_tag` builds the villager from the zombie villager's saved tag. It keeps the common fields, effects, `VillagerData`, gossips, offers and `Xp`, sets `VillagerDataFinalized` as vanilla reads it and an `Age` for a baby, and lists the preserved equipment to drop.
  - `minecraft_terrain/src/server.rs`: after the mobs tick, `cure_zombie_villagers` turns each cured zombie villager into the villager.
    - It drops the preserved equipment, finalizes the tag and replaces the mob.
    - When the starter is among the players of this level, it gives the cured villager +20 `major_positive` and +25 `minor_positive` gossip about them.
    - It adds Nausea for 200 ticks and sends level event 1027 unless the zombie was `Silent`.
  - `minecraft_terrain/src/level_events.rs`: event 1027 plays `entity.zombie_villager.converted` for the host and the friend.
  - `minecraft_terrain/src/mob_tracking.rs`:
    - `MobShow.converting` carries a zombie's shaking (curing or drowning), and `show_on` sets it on the friend's copy.
    - `ConversionTime` is in `VOLATILE`, so the countdown does not resend the look tag every tick.
  - `minecraft_terrain/src/zombie_render.rs`: a shaking zombie or zombie villager turns its body like a converting piglin, for the host and the friend.
  - `minecraft_terrain/src/potions.rs`: its test now uses glowing as the effect mobs do not have, since strength exists now.
  - The cure sound is a mob sound on the host, so it reaches friends through `heard.rs` like every other mob sound. No protocol change.
- **Tests:**
  - `zombie::tests::a_cure_counts_one_a_tick_and_a_hundredth_of_ticks_look_for_up_to_fourteen_bars_and_beds`:
    - Most ticks look at no blocks.
    - The cap is 14 blocks (progress 15).
    - Three in ten found blocks add one.
    - The box runs from 4 below to 3 above the truncated position.
    - `curing` and `shaking`.
  - `world::tests::a_golden_apple_starts_curing_only_a_weakened_zombie_villager`:
    - It does nothing for a plain zombie, and gives `Some(false)` without weakness.
    - Once started: a time of 3600 to 6000 with the player's UUID, weakness gone, strength for the time, and the cure sound once.
    - A second apple does nothing.
    - 500 seeded starts spread over 3600 to 6000.
  - `world::tests::a_curing_zombie_villager_counts_down_faster_beside_iron_bars_and_beds_and_is_handed_over`:
    - 3000 ticks count exactly 3000 without bars, and more than 3040 beside 28 iron bars and beds.
    - The cure is handed over exactly once, at 0.
    - A curing zombie villager 200 blocks away does not despawn.
  - `mob_actions::tests::a_golden_apple_cures_only_a_weakened_zombie_villager_and_is_used_up_then`: a plain zombie lets the player eat; without weakness the click is taken and the apple kept; `would_interact` agrees and changes nothing; with weakness one apple is used and the cure starts; other food does nothing.
  - `mob_tracking::tests::a_friend_sees_a_zombie_villager_shake_while_it_is_cured_and_a_zombie_while_it_drowns`: shaking turns on and off on the friend's copy, and the countdown changes no look.
  - `server::tests::a_golden_apple_cures_a_weakened_zombie_villager_into_the_same_villager_that_trades_cheaper_with_its_curer`:
    - No apple is used without weakness. With weakness one is.
    - The saved tag has `ConversionTime` and `ConversionPlayer`, a curing one is not removed when far away (`dormant.rs`), and it reloads with the same time and starter.
    - At the end of the countdown the zombie villager becomes a desert farmer of level 2 with 12 xp, its offer, its name and Strength, standing in the same place.
    - The villager has 200 ticks of Nausea, +20 `major_positive` and +25 `minor_positive` gossip about the curer, and level event 1027 plays.
    - The wheat offer costs the other player 20 and the curer 14.
  - `level_events::tests::a_cured_zombie_villager_rings_out_where_it_stood`.
  - `potions::tests::effects_cross_to_mobs_and_players_where_they_exist` (updated).
  - Runs: `minecraftoss-entities` lib 240, `minecraftoss-world` lib 77, `minecraft_terrain` lib 1136 (3 ignored) and `minecraft_survival` lib 601, all green. `check -p launcher` is green.
- **Decisions:**
  - The cure sound's volume and pitch come from the zombie's random on the host, so host and friend hear the same sound. Vanilla draws them on each client.
  - Preserved equipment (drop chance above 1) drops at the cure. Binding-cursed items do not move onto the villager, because the fork's villagers wear no armour.
  - A curing zombie villager loaded from a save keeps its saved effects. It does not run `startConverting` again, which in vanilla removes Weakness and gives Strength for the remaining time once more on load.
  - Drowning zombies now shake too, as in vanilla. `MobShow.converting` carries both kinds of shaking.
  - Strength is a mob effect now, so a splash potion of strength also lands on mobs. Mob melee does not read effects yet, so Strength does not make a curing zombie villager hit harder. Vanilla adds 3 damage.
  - The starter counts as present when they are among the players ticking this level, which is `getPlayerByUUID` on that level.
- **Test steps (survival, difficulty normal):**
  1. Build a closed planks box of about 5 x 5 with a roof, so the sun cannot burn what is inside. Inside, summon a farmer zombie villager: `/summon minecraft:zombie_villager ~ ~ ~2 {PersistenceRequired:1b,VillagerData:{type:"minecraft:plains",profession:"minecraft:farmer",level:1}}`.
  2. Right-click it with a golden apple: nothing happens and the apple stays in your hand.
  3. Throw a splash potion of weakness at it, then right-click it with a golden apple. One apple is used, a cure sound plays and the zombie villager starts shaking from side to side (**30_curing_shakes**).
  4. Place iron bars and a red bed within 4 blocks of it. Save and quit after a minute, load again: it is still shaking.
  5. Wait 3 to 5 minutes in total. It turns into a plains farmer with a loud sound (**31_cured_villager**).
  6. Right-click the farmer. Its offers are much cheaper than a normal farmer's: the struck-through old price shows next to the new one (**32_cured_prices**).
  7. Friend window: the friend sees the shaking and hears the cure and converted sounds. The friend's own trading screen with that farmer shows the normal prices (**33_friend_prices**). The friend can cure a second zombie villager the same way and gets the discount there.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:zombie_villager_spawn_egg*4,minecraft:splash_potion@weakness,minecraft:splash_potion@weakness,minecraft:splash_potion@weakness,minecraft:splash_potion@weakness,minecraft:golden_apple*16,minecraft:iron_bars*16,minecraft:red_bed,minecraft:oak_planks*64,minecraft:oak_planks*64,minecraft:wheat*64,minecraft:emerald*32,minecraft:torch*32,minecraft:cooked_beef*32
```

## Group effects-omens

### 1. Minecraft players carry darkness, bad omen, raid omen, trial omen and hero of the village

After 4cd55bf8.

- **Vanilla (javap, 26.3):**
  - `MobEffects` registrations (category, colour):
    - `darkness`: HARMFUL, 2696993, `setBlendDuration(22)`.
    - `bad_omen`: NEUTRAL, 745784, a `BadOmenMobEffect` with `withSoundOnAdded(event.mob_effect.bad_omen)`.
    - `hero_of_the_village`: BENEFICIAL, 4521796.
    - `trial_omen`: NEUTRAL, 1484454, the `trial_omen` particle and `withSoundOnAdded(event.mob_effect.trial_omen)`.
    - `raid_omen`: NEUTRAL, 14565464, a `RaidOmenMobEffect` with the `raid_omen` particle and `withSoundOnAdded(event.mob_effect.raid_omen)`.
  - `BadOmenMobEffect.applyEffectTick` (every tick): a non-spectator `ServerPlayer` outside Peaceful standing in a village (`ServerLevel.isVillage`), whose raid there is missing or below its maximum omen level, gets Raid Omen for 600 ticks at the same amplifier and `setRaidOmenPosition` there, and the Bad Omen ends.
  - `Villager.updateSpecialPrices`: Hero of the Village takes `floor((0.3 + 0.0625 x amplifier) x first cost)`, at least 1, off each offer. The fork's `EntityWorld::set_player_hero` already held this (`world/merchants.rs:76`, `:134`), but nothing called it.
- **Changed:**
  - Vendored `player/src/survival.rs`: `EffectKind` gains `Darkness`, `BadOmen`, `RaidOmen`, `TrialOmen` and `HeroOfTheVillage` at the end, and `ALL` has 20 kinds. None of them does anything on its own tick here.
  - `minecraft_survival/src/player_nbt.rs`: `EFFECTS` maps them to `minecraft:darkness`, `minecraft:bad_omen`, `minecraft:raid_omen`, `minecraft:trial_omen` and `minecraft:hero_of_the_village`, so they are saved in the player file's `active_effects` and read back. Before this, a vanilla file's omens were kept untouched as unknown effects.
  - `minecraft_survival/src/commands.rs`: `effect_kind` knows the five ids, so `/effect give` and `/effect clear` work on them.
  - `minecraft_terrain/src/potions.rs`: `player_effect` has the five arms (added arms only, for track B9's colour edits).
  - `minecraft_survival/src/entities.rs`: the mobs' `PlayerView` has `hero` (the Hero of the Village amplifier), and `Entities::tick` puts every actor's into the new `TickInput.heroes`, beside `mob_players`.
  - `minecraft_terrain/src/server.rs`: `TickInput.heroes` (dropped by `remove_player`). Each level tick hands them to `EntityWorld::set_player_hero` before the mobs tick, so the next trading screen a hero opens is cheaper, for that player only.
  - `render_anim/src/minecraft_world.rs` (the host) and `render_anim/src/minecraft_players.rs` (friends on the host) fill `hero` from the player's own effects. The other `PlayerView` builders (tests, the level loader, the dragon fight and the remote tests) pass `None`.
- **Tests:**
  - `player_nbt::tests::darkness_the_omens_and_hero_of_the_village_are_saved_and_read_back`: the five kinds are written with their vanilla ids, durations and amplifiers and read back the same.
  - `commands::tests::darkness_the_omens_and_hero_of_the_village_are_given_and_cleared_by_command`: `/effect give` of each (seconds times 20, the amplifier), `/effect clear` of one and then of all; `effect_kind` maps each id.
  - `server::tests::hero_of_the_village_from_the_tick_input_lowers_that_player_s_prices`: through `run_tick`, a 20-wheat offer costs 20 without the effect, 14 at level I and 9 at level V, and 20 for another player and after the effect ends.
  - `survival::tests::every_effect_kind_is_listed_once_at_its_own_index` (existing) covers the 20 kinds.
  - Runs: `minecraftoss-player` lib 190, `minecraft_survival` lib 603 and `minecraft_terrain` lib 1137 (3 ignored), all green. `check -p launcher` green.
- **Decisions:**
  - Bad Omen does not turn into Raid Omen in a village yet, and Raid Omen does not start a raid when it ends: both need villages and raids (group `raids`). Trial Omen waits for trial spawners. Darkness is only carried and saved; its look (the pulsing dark screen and the shorter view) is group `darkness`.
  - The hero amplifier is sent every tick for every player in the level, also when it is `None`, so an effect that ends or is drunk away with milk stops the discount at the next screen.
  - The fork saves every player effect with `ambient:0b`, `show_particles:1b` and `show_icon:1b`, as before. Vanilla's ominous bottle gives Bad Omen without particles; the fork draws no player effect particles yet, so nothing shows the difference.
- **Test steps (survival, difficulty normal):**
  1. In a closed room summon a farmer: `/summon minecraft:villager ~ ~ ~2 {NoAI:1b,PersistenceRequired:1b,VillagerData:{type:"minecraft:plains",profession:"minecraft:farmer",level:1}}`. Right-click it and note the wheat price (**40_price_before**). Close the screen.
  2. `/effect give @s minecraft:hero_of_the_village 600 0`, then right-click the farmer again: every price is lower, with the old price struck through (**41_hero_prices**).
  3. `/effect give @s minecraft:hero_of_the_village 600 4`: the prices drop further (level V). Then `/effect clear @s minecraft:hero_of_the_village` and right-click again: the prices are back (**42_hero_cleared**).
  4. `/effect give @s minecraft:bad_omen 6000 2`, `/effect give @s minecraft:trial_omen 900`, `/effect give @s minecraft:raid_omen 30` and `/effect give @s minecraft:darkness 60`: each is answered with the vanilla "Applied effect" line and none is refused (**43_omens_given**).
  5. Save and quit, load again, then `/effect clear @s minecraft:bad_omen`: the answer says it was removed, so it came back with the save (**44_omen_after_reload**).
  6. Friend window: the friend runs step 2 with their own `/effect`. The friend's prices drop; the host's prices at the same farmer stay as they were (**45_friend_hero**).
  7. Give Hero of the Village again and drink a milk bucket: the next trading screen has the normal prices.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:wheat*64,minecraft:wheat*64,minecraft:emerald*32,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:oak_planks*64,minecraft:torch*32,minecraft:cooked_beef*32
```

### 2. Minecraft ominous bottles give bad omen

After 0dd063c0.

- **Vanilla (javap, 26.3):**
  - `Items.OMINOUS_BOTTLE`: rarity UNCOMMON, the `Consumables.OMINOUS_BOTTLE` consumable and `OminousBottleAmplifier(0)` as its default `minecraft:ominous_bottle_amplifier`. It stacks to 64 and has no remainder.
  - `Consumables.OMINOUS_BOTTLE` = `defaultDrink()` (1.6 s = 32 ticks, the DRINK animation, `entity.generic.drink`, no particles) plus `soundAfterConsume(item.ominous_bottle.dispose)`, which is a `PlaySoundConsumeEffect` at the drinker's block, volume 1, pitch 1.
  - `Consumable.onConsume`: the drink sound (`emitParticlesAndSounds`, volume 0.5, pitch 0.9 to 1), then the `ConsumableListener` components (here `OminousBottleAmplifier.onConsume`), then the consume effects (the dispose sound), then `ItemStack.consume(1)` (nothing is used in creative).
  - `OminousBottleAmplifier.onConsume`: `addEffect(new MobEffectInstance(BAD_OMEN, 120000, value, false, false, true))`. Its codec is `ExtraCodecs.intRange(0, 4)`.
  - `MobEffect.onEffectAdded` plays the effect's `soundOnAdded` at the entity, volume 1, pitch 1, only when the effect is new (`LivingEntity.addEffect` calls it for a fresh effect, not for an update). Bad Omen's is `event.mob_effect.bad_omen`.
- **Changed:**
  - `minecraft_terrain/src/bottle.rs`: `OMINOUS_BOTTLE`, `OMINOUS_AMPLIFIER`, `OMINOUS_DISPOSE_SOUND`, `BAD_OMEN_TICKS` (120000), `drink_ominous` (the drink with no remainder) and `ominous_amplifier` (the component, 0 when missing, held to 0 to 4).
  - `minecraft_survival/src/actions.rs`: right-clicking with an ominous bottle in either hand starts drinking it. When it is drunk, the new sounds in `Bite::Done.heard` play at the player, after the drink sound.
  - `minecraft_survival/src/survival.rs`: `Bite::Done` has `heard`. A finished ominous bottle runs `drink_ominous_bottle`: Bad Omen for 120000 ticks at the bottle's amplifier, then `event.mob_effect.bad_omen` when the omen is new and `item.ominous_bottle.dispose`.
  - `minecraft_survival/src/offhand.rs`: the ominous bottle is an item the offhand uses.
  - `minecraft_commands/src/args.rs`: `/give` and `/kit` take `[minecraft:ominous_bottle_amplifier=N]` (0 to 4; outside that, vanilla's "malformed" error with "Value N outside of range [0:4]").
  - `minecraft_terrain/src/crafting.rs`: the env kit takes the same component (`minecraft:ominous_bottle[ominous_bottle_amplifier=4]`). A value outside 0 to 4 is left out.
- **Tests:**
  - `survival::tests::an_ominous_bottle_is_drunk_away_and_gives_bad_omen_at_its_amplifier`: 32 ticks of drinking; amplifier 3 gives Bad Omen 120000 ticks at 3 with the drink sound, the omen sound and the dispose sound; one bottle of two is used and no glass bottle comes back; a second bottle plays only the dispose sound; a bottle without the component gives amplifier 0, and a value of 9 is held to 4.
  - `args::tests::an_ominous_bottle_takes_its_amplifier_from_zero_to_four`: `=4` gives the component, `=5` is malformed at the 5, `=x` wants an integer.
  - `crafting::tests::kit_ominous_bottles_carry_their_amplifier`.
  - Runs: `minecraft_survival` lib 604, `minecraft_commands` lib 135 and `minecraft_terrain` lib 1138 (3 ignored), all green. `check -p launcher` green.
- **Decisions:**
  - The dispose sound plays at the player like the omen sound. Vanilla plays it at the centre of the player's block, less than a block away.
  - `/effect give` of an omen plays no omen sound. Vanilla plays it there too; the fork's commands have no sound path to the player yet. It is listed under Left for later.
  - The bottle's tooltip line ("Bad Omen V (100:00)") is not drawn: the fork's tooltips show potion contents only. It is listed under Left for later.
- **Test steps (survival, difficulty normal):**
  1. Hold the plain ominous bottle and keep right-click pressed: the drinking animation and sound run for 1.6 seconds, then an eerie omen sound and the bottle-break sound play, and one bottle is gone with no glass bottle back (**46_ominous_drunk**).
  2. `/effect clear @s minecraft:bad_omen` answers that Bad Omen was removed (it was level I). Drink the bottle with amplifier 4, then give yourself the same omen again with `/effect give @s minecraft:bad_omen 10 0`: the answer says it could not be applied, because a stronger one (level V) is running (**47_omen_level_five**).
  3. Drink an ominous bottle from the offhand (F swaps it there): it works the same.
  4. In creative mode (`/gamemode creative`), drink one: the bottle stays.
  5. Back in survival, drink a milk bucket: the Bad Omen is gone (`/effect clear @s minecraft:bad_omen` now says there was nothing to remove).
  6. Friend window: the friend drinks a bottle and hears the omen sound; the host standing next to them hears it too (**48_friend_omen**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:ominous_bottle[ominous_bottle_amplifier=4]*2,minecraft:ominous_bottle[ominous_bottle_amplifier=2],minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cooked_beef*16
```

### 3. Minecraft effect icons show in the top right like vanilla

After c013310f.

- **Vanilla (javap, 26.3):**
  - `Hud.extractRenderState` draws `extractEffects` after the hotbar and before the boss bars, only while the HUD is not hidden (F1), in every game mode.
  - `Hud.extractEffects`:
    - Nothing when there are no effects, or when the open screen `showsActiveEffects` (the survival and creative inventories do when `EffectsInInventory.canSeeEffects`, and then draw the effects beside themselves).
    - The effects are sorted with `Ordering.natural().reverse()` and drawn in that order when `showIcon`.
    - A beneficial effect (`MobEffect.isBeneficial`: category BENEFICIAL only) goes to x = gui width - 25 x (its count so far), y = 1. Any other (HARMFUL or NEUTRAL, so the omens) goes to the same x by its own count, at y = 1 + 26. Demo mode adds 15 to y.
    - The frame is `hud/effect_background` 24 x 24, or `hud/effect_background_ambient` for an ambient effect. The icon is the sprite `mob_effect/<id>` (`getMobEffectSprite`, from the `mob_effect` folder in the gui atlas, `atlases/gui.json`) at x + 3, y + 3, 18 x 18, drawn in `ARGB.white(alpha)`.
    - The alpha is 1, except for a non-ambient effect that `endsWithin(200)` (not infinite and at most 200 ticks left): with d ticks left and m = 10 - d / 20 (integer division), alpha = clamp(d / 10 / 5 x 0.5, 0, 0.5) + cos(d x pi / 5) x clamp(m / 10 x 0.25, 0, 0.25), held to 0 to 1.
  - `MobEffectInstance.compareTo`: when both have more than 32147 ticks, or both are ambient, it compares ambient (false first), then the colour. Otherwise it compares ambient (false first), infinite (false first), the duration (an infinite one is -1), then the colour. The categories and colours of all 40 effects are from `MobEffects.<clinit>` (see commit 1 for the five new ones).
  - `hud/effect_background` and `hud/effect_background_ambient` are 24 x 24, every `mob_effect/*.png` is 18 x 18.
- **Changed:**
  - New `hud/src/minecraft_effects.rs`:
    - The table of the 40 vanilla effects: path, beneficial and colour.
    - `compare` (vanilla's ordering), `alpha` (the blink) and `layout` (the rows, the frame and the sprite of each effect).
    - `draw_effects` draws the frames and the icons through the GUI painter.
    - `minecraft_hotbar::draw_hud` calls it before the boss bars and before the spectator check, so the icons show in every game mode wherever the Minecraft HUD is drawn.
  - `frame/src/minecraft_ui.rs`: new `McEffect` (id, amplifier, duration with -1 for unending, ambient) and `MinecraftUi.effects`.
  - `render_anim/src/minecraft_gui.rs`: the GUI atlas also holds every `textures/mob_effect/*.png` as the sprite `mob_effect/<name>`, as vanilla's gui atlas does. The `hud/` folder already brought the two frames.
  - `minecraft_survival/src/views.rs`:
    - New `effect_views`, the effects of a `SurvivalStatus` as the protocol's `EffectView`s. `player_view` uses it.
    - `HudClock::effects` counts a friend's effects down on the friend between views. The host sends the view only when something other than a duration changes, so the friend counts from the view it last got and takes a new view as it is.
  - `render_anim/src/minecraft_world.rs`: new `mc_effects`. The host fills `ui.effects` from its own status each frame, and nothing while dead.
  - `render_anim/src/minecraft_remote.rs`: the friend fills `ui.effects` from `HudClock::effects`, and nothing while dead.
- **Tests:**
  - `minecraft_effects::tests::beneficial_effects_fill_the_top_row_and_harmful_ones_the_second_from_the_right`: six effects in the vanilla order and places, with Bad Omen (neutral) in the second row.
  - `minecraft_effects::tests::long_effects_sort_by_colour_and_short_ones_by_time_left`: two effects over 32147 ticks by colour, an unending one before the rest, then the most time left, and an ambient one first.
  - `minecraft_effects::tests::an_ambient_effect_gets_its_frame_and_never_blinks`.
  - `minecraft_effects::tests::the_icon_blinks_in_the_last_ten_seconds_and_fades_out`: full above 200 ticks and when unending, 0.5 at 200, 0.475 at 195, 0.625 at 100, 0.375 at 105, 0.35 at 10, 0 at 5, a pulse every 10 ticks, and `ARGB.white`.
  - `views::tests::a_friend_s_effects_count_down_between_views_and_restart_from_a_new_one`.
  - `minecraft_gui::tests::every_mob_effect_icon_and_the_effect_frames_load`: the name filter, then with the 26.3 pack every `mob_effect/` sprite 18 x 18 (at least 40), both frames 24 x 24 and the atlas still at most 2048 high.
  - Runs: `hud` lib 160, `frame` lib 24, `minecraft_survival` lib 605 and `render_anim` lib 151, all green. `check -p launcher` green. Clippy (touched crates, with `-A clippy::approx_constant` as for the whole fork) shows nothing on this group's lines. The new file passes `rustfmt --check --edition 2024`.
- **Decisions:**
  - The survival and creative inventories do not draw vanilla's effect list beside them yet (`EffectsInInventory`), so the HUD icons stay on screen while they are open. Vanilla hides the HUD icons then and shows the list instead. It is listed under Left for later.
  - The blink's cosine is the float `cos`; vanilla's `Mth.cos` reads a 65536-step sine table. The difference is far below one alpha step.
  - Every player effect is sent and drawn as not ambient, because the fork's `EffectInstance` has no ambient flag (vanilla's conduit and beacon effects are ambient). The ambient frame and order are in place for when it has one.
  - The friend's durations count on the friend's own HUD clock, so a friend's blink can run a few ticks apart from the host's. The host's next view (any change that shows) sets it right again.
- **Test steps (survival, difficulty normal):**
  1. Eat a golden apple: Regeneration and Absorption icons appear at the top right in one row, each in a grey frame. Regeneration lasts only 5 seconds, so it blinks at once and fades out (**49_effect_icons**).
  2. Throw a splash potion of weakness at your feet: the Weakness icon appears in a second row under the first (**50_harmful_row**).
  3. Drink an ominous bottle: the Bad Omen icon joins the second row (the omens are neutral, and vanilla puts every effect that is not beneficial there) (**51_bad_omen_icon**).
  4. `/effect give @s minecraft:hero_of_the_village 15`: the Hero of the Village icon joins the top row. Watch it for 15 seconds: for its last 10 seconds it blinks and fades, then it is gone (**52_icon_blinks**).
  5. `/effect give @s minecraft:darkness 30` and `/effect give @s minecraft:trial_omen 30`, `/effect give @s minecraft:raid_omen 30`: each has its own icon in the second row (**53_all_new_icons**).
  6. Open the inventory: the icons stay at the top right.
  7. Drink a milk bucket: every icon is gone.
  8. Friend window: the friend eats a golden apple and drinks an ominous bottle; the friend's own icons show the same way, and the Regeneration icon blinks in its last 10 seconds (**54_friend_icons**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*16,minecraft:splash_potion@weakness,minecraft:splash_potion@weakness,minecraft:ominous_bottle*4,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cooked_beef*16
```

## Group cats

### 1. Minecraft cats and ocelots live in the entity world and are drawn like vanilla

After c983b378.

- **Vanilla (javap, 26.3):**
  - `EntityTypes.CAT`: `CREATURE`, `sized(0.6, 0.7)`, `eyeHeight(0.35)`, passenger attachment 0.5125, `clientTrackingRange(8)`. `EntityTypes.OCELOT`: `CREATURE`, `sized(0.6, 0.7)` (eyes at the default 0.85 x 0.7 = 0.595), passenger attachment 0.6375, `clientTrackingRange(10)`. Both `BABY_DIMENSIONS`: `scalable(0.3, 0.35)`, eyes 0.34375.
  - `Cat.createAttributes` and `Ocelot.createAttributes`: 10 health, 0.30000001192092896 speed, 3 attack damage (an animal's 16 follow range and 10 tempt range).
  - `Cat` is a `TamableAnimal`, `Ocelot` an `Animal`. Speeds: tempt 0.6, walk 0.8, sprint 1.33. `customServerAiStep`: with a wanted move at 0.6 the pose is `CROUCHING`, at 1.33 standing and sprinting (the sprint adds the +30% speed modifier), else standing.
  - `Cat.registerGoals`: 1 `FloatGoal`, 1 `TamableAnimalPanicGoal(1.5)` (`#panic_causes`), 2 `SitWhenOrderedToGoal`, 3 `CatRelaxOnOwnerGoal`, 4 `CatTemptGoal(0.6, #cat_food, canScare)`, 5 `CatLieOnBlockGoal(1.1, 8)`, 6 `FollowOwnerGoal(1.0, 10, 5)`, 7 `CatSitOnBlockGoal(0.8)`, 8 `LeapAtTargetGoal(0.3)`, 9 `OcelotAttackGoal`, 10 `BreedGoal(0.8)`, 11 `WaterAvoidingRandomStrollGoal(0.8, 1.0000001E-5)`, 12 `LookAtPlayerGoal(Player, 10)`. Targets: 1 `NonTameRandomTargetGoal(Rabbit)` and 1 `NonTameRandomTargetGoal(Turtle, BABY_ON_LAND)`. `reassessTameGoals` adds `CatAvoidEntityGoal(Player, 16, 0.8, 1.33)` at 4 while wild, after the others.
  - `Ocelot.registerGoals`: 1 `FloatGoal`, 3 `OcelotTemptGoal(0.6, #ocelot_food, canScare)`, 7 `LeapAtTargetGoal(0.3)`, 8 `OcelotAttackGoal`, 9 `BreedGoal(0.8)`, 10 the stroll, 11 `LookAtPlayerGoal(Player, 10)`. Targets: 1 `NearestAttackableTargetGoal(Chicken, false)` and 1 the baby turtles on land (interval 10). `reassessTrustingGoals` adds `OcelotAvoidEntityGoal(Player, 16, 0.8, 1.33)` at 4 while it does not trust.
  - `TemptGoal`: the nearest player within the tempt range (non-combat, sight ignored) holding the item in either hand. With `canScare`, a player within 6 blocks who moved more than 0.1 blocks or turned more than 5 degrees scares it off. It looks at the player (max head turn + 20, 40) and walks to it until 2.5 blocks; it calms 50 ticks after it stops. `CatTemptGoal`: never while tame, and a picked favourite player (one roll in 300 per goal tick, dropped one in 250) cannot scare it. `OcelotTemptGoal`: scares only while it does not trust.
  - `OcelotAttackGoal`: every tick it looks at the target (30, 30), runs at 1.33 between its reach ((2 x width)^2) and 4 blocks, else 0.6 within 15 blocks (0.8 beyond), bites (`doHurtTarget`) every 20 ticks within reach, and gives up beyond 15 blocks.
  - Sounds: `Cat.getAmbientSound` (stray ambient while wild; purr in love, purreow one in four, else ambient while tame), interval 120. The `cat_sound_variant` registry has `classic` and `royal`; babies use `entity.baby_cat.*`. `Cat.tick` begs every 100 ticks while tempted and wild, and purrs every 5 ticks while lying or relaxed at volume 0.6 + 0.4 x (r1 - r2). `Ocelot`: `entity.ocelot.ambient`, `.hurt`, `.death`, interval 900.
  - Saved: `TamableAnimal` `Owner`, `Sitting`; `Cat` `variant`, `sound_variant`, `CollarColor` (default red, 14); `Ocelot` `Trusting`. `removeWhenFarAway`: wild (cat) or not trusting (ocelot) and more than 2400 ticks old.
  - `Cat.finalizeSpawn`: `AgeableMob`'s first (the baby roll, then `Mob.finalizeSpawn`), then `VariantUtils.selectVariantToSpawn` (`cat_variant` data: `all_black` at priority 1 in `#cats_spawn_as_black` structures, the swamp hut, and at priority 0 with the moon at 0.9 or brighter; the others at 0), then a random sound variant. `Ocelot.finalizeSpawn`: `AgeableMobGroupData(1.0)` (all after the first of a group are babies). `checkOcelotSpawnRules`: `nextInt(3) != 0`. `SpawnPlacements`: `ON_GROUND`, `MOTION_BLOCKING`. `checkSpawnObstruction`: no liquid, at or above sea level, grass block or leaves below. Cats have no spawn placement.
  - Models: `AdultFelineModel.createBodyMesh` (64 x 32): the head at (0, 15, -9) with `main` (-2.5, -2, -3; 5 x 4 x 5), `nose` (-1.5, -0.001, -4; 3 x 2 x 2 at 0, 24), `ear1` (-2, -3, 0; 1 x 1 x 2 at 0, 10) and `ear2` (1, -3, 0 at 6, 10); the body at (0, 12, -10) turned pi/2 (-2, 3, -8; 4 x 16 x 6 at 20, 0); `tail1` at (0, 15, 8) turned 0.9 and `tail2` at (0, 20, 14) (-0.5, 0, 0; 1 x 8 x 1 at 0, 15 and 4, 15; `tail2` deformed -0.02); hind legs at (+-1.1, 18, 5) (-1, 0, 1; 2 x 6 x 2 at 8, 13); front legs at (+-1.2, 14.1, -5) (-1, 0, 0; 2 x 10 x 2 at 40, 0). `BabyFelineModel.createBabyLayer` (32 x 32): the head at (0, 20, -3.125) with four cubes, legs at (+-1, 22, -1.5 and 2.5), the body at (0, 20.5, 0.5) (4 x 3 x 7 at 0, 8), `tail1` at (0, 19.107, 3.9151) turned -0.567232 (1 x 1 x 5 at 0, 18), an empty `tail2`. Both `setupAnim`s (crouching, sprinting, walking, sitting, lying and relaxing offsets and turns) are ported in `cat_render.rs`.
  - `LayerDefinitions`: `CAT` and `CAT_COLLAR` take `AdultCatModel.CAT_TRANSFORMER` (`MeshTransformer.scaling(0.8)`); the collar mesh is `createBodyMesh(CubeDeformation(0.01))`; `CAT_BABY_COLLAR` is the baby layer scaled by 1.01; `OCELOT` and `OCELOT_BABY` are the plain layers. `CatRenderer` and `OcelotRenderer`: shadow 0.4. `CatRenderer.setupRotations` while lying: translate (0.4, 0.15, 0.1) x amount, turn Z by 90 x amount, and 0.15 x amount further on a sleeping player. `CatCollarLayer`: a tame cat's collar in its dye's diffuse colour. Textures `entity/cat/cat_<variant>` and `_baby`, `entity/cat/ocelot` and `ocelot_baby`, `cat_collar` and `cat_collar_baby`.
- **Changed:**
  - Vendored `entities`:
    - New `feline.rs`: `FelineKind { Cat, Ocelot }` and `Feline` with its sizes, voices, textures and the lying and relaxing amounts.
    - New `feline_ai.rs`: the goals and targets above on the monster goal framework (panic, the cat and ocelot tempt with scaring, the ocelot attack, avoiding players while wild). Sitting, relaxing, lying, following and breeding hold their slots as idle goals until the next commit.
    - New `world/felines.rs`: `FelineEntity`, `spawn_feline`, `felines`, `feline_mut`, its hurt, ambient sound, AI step with the crouch and sprint poses, and the purr and beg tail.
    - `MonsterKind::Cat` and `::Ocelot`, `Prey::Chicken`, `MonsterGoalContext.feline`; a tame cat never targets its owner.
    - Every `EntityWorld` site: `EntityKey::Feline`, `MobHit::Feline`, bodies, damage, deaths with the `entities/cat` and `entities/ocelot` loot and 1 to 3 experience, projectiles, blasts, potions, pushing, hazards, despawning, census and the villagers' sight.
  - Vendored `world`: `type_info` for `ocelot` (its own rules, placement and obstruction check) and `cat` (no rules); `finalize_cat` (variant by the moon and the swamp hut, then the sound variant) for `/summon`, spawn eggs and the cat spawner to come; the ocelot group's babies. Vendored `generator`: `CatVariants::candidates` and `sounds` are public for it.
  - `minecraft_terrain`:
    - `SIMULATED_KINDS`, `spawn_saved` and `mob_tags_where` arms.
    - New `cat_render.rs`; textures in `mesh.rs`; the pose push and shadow in `client_mobs.rs`; spawner displays.
    - Tracking ranges (cat 8, ocelot 10) and `MobShow.sitting`, `lying`, `relaxed`, `crouching` and `sprinting` with `shows` and `show_on`.
    - Thrown items hit them; `HATCHING_EGGS` grows to 39 with `cat` and `ocelot`; `wolf_render::DYE_DIFFUSE` is public for the collar.
  - `minecraft_survival`: hit-key byte 29 (`FELINE_KEY`), the shot boxes and the render call.
- **Tests:**
  - `feline::tests::a_cat_and_an_ocelot_take_their_sizes_textures_and_voices`, `feline::tests::lying_down_eases_in_and_out_at_the_vanilla_rates`, `feline_ai::tests::the_panic_causes_follow_the_damage_type_tag`.
  - `world::felines::tests::a_cat_and_an_ocelot_are_live_mobs_of_their_kind` (type, census, candidates, hurt) and `a_wild_ocelot_runs_from_a_player_and_a_tempted_one_crouches_after_fish`.
  - `natural_spawner::spawning::tests::ocelots_spawn_by_their_own_rules_and_cats_have_none`.
  - `server_mobs::tests::a_cat_and_an_ocelot_save_and_load_with_variant_collar_owner_sitting_and_trust`, `mob_tracking::tests::a_friend_sees_a_cat_and_an_ocelot_from_their_look_tags`, `cat_render::tests` (cube counts, standing on the ground, the 0.8 cat scale, sitting, lying and crouching poses, textures), `server::tests::summoned_cats_draw_their_variants_and_voices_and_summoned_ocelots_live`, the egg survey (39 eggs) and the spawner cage test with a cat and an ocelot.
  - `entities::tests::a_cat_and_an_ocelot_hit_box_encodes_and_decodes`.
  - Runs: `minecraftoss-entities` lib 245, `minecraftoss-world` lib 78, `minecraftoss-generator` lib 30, `minecraft_terrain` lib 1144, `minecraft_survival` lib 606, all green. `check -p launcher` green. The four new files pass `rustfmt --check --edition 2021`.
- **Decisions:**
  - Cats and ocelots share one store (`felines`, hit key 29), as the batching rule says; `FelineKind` picks the goals, model, texture and voice.
  - The fork never fills the players' tempt flags (the `PlayerCandidate` food flags are always false), so felines read the held items from the players' controls (`PlayerControls`, which the host already sends to the level each tick) against `#cat_food` and `#ocelot_food`.
  - The black cat structure test (`#cats_spawn_as_black`) names the swamp hut, as the generator already does.
  - The baby model's crouch offsets use the baby age scale of 0.5.
  - The cat's targets (rabbits, baby turtles) and the ocelot's baby turtles have no mob in the fork yet, so those goals only roll their interval. Ocelots hunt chickens.
  - Vanilla cats in spawners stick out of the cage (the spawner scales by the box, 0.6 x 0.7); the cage test allows them 0.1 and 0.25 blocks.
  - `MobShow` grows by five fields. `minecraft_net::PROTOCOL` stays 4: track D bumped it once in group trading-friend, and the mob moves ride inside it.
- **Test steps (survival, normal difficulty, by day):**
  1. Use the cat egg on grass a few times: cats appear in different coats (tabby, black, calico and others) and walk about. Come closer and they run away from you (**55_cats_drawn**).
  2. Hold raw cod and stand still about 8 blocks from a cat: it creeps towards you crouched. Turn your view or step and it runs off (**56_cat_tempted**).
  3. Use the ocelot egg on grass: a spotted ocelot appears and runs from you. Use the chicken egg near it: the ocelot crouches, sprints and bites the chicken (**57_ocelot_hunts**).
  4. Hit a cat with the sword: it hurts, flees and dies after a few hits, dropping string and experience.
  5. `/locate structure minecraft:village_plains` and `/tp` there: the village's cats walk about now instead of standing still (**58_village_cats**). `/locate structure minecraft:swamp_hut`: the hut's black cat moves too.
  6. Place the spawner and use the cat egg on it: a small cat turns inside the cage (**59_cat_spawner**).
  7. Save and quit, then load again: every cat keeps its coat.
  8. Friend window: the friend sees the same cats and ocelots, crouching and sprinting with them, and can hit them with the sword and a gun (**60_friend_cats**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cat_spawn_egg*8,minecraft:ocelot_spawn_egg*4,minecraft:chicken_spawn_egg*4,minecraft:cod*32,minecraft:salmon*16,minecraft:iron_sword,minecraft:spawner,minecraft:cooked_beef*16
```

### 2. Minecraft cats are tamed with fish, sit, and scare creepers; ocelots learn to trust

After 31f6a995.

- **Vanilla (javap, 26.3):**
  - `Cat.mobInteract`, wild: raw cod or salmon (`#cat_food`) is eaten (one used unless creative), `tryToTame` tames one time in three (`tame(player)`, `setOrderedToSit(true)`, entity event 7) or shows smoke (event 6); either way it becomes persistent and plays its eat sound. Tame and owned: a dye (`#cat_collar_dyes`) of another colour recolours the collar (one used, persistent); its food heals it while hurt (`TamableAnimal.feed(1, 1)`: the food's nutrition); otherwise `Animal.mobInteract` (food: love or a baby's growth), and when that does nothing the owner's click toggles `setOrderedToSit`. Tame but someone else's: only `Animal.mobInteract`. `Cat` has no `applyTamingSideEffects` (10 health stays).
  - `Ocelot.mobInteract`: while its tempt goal runs, it does not trust yet, the item is `#ocelot_food` and the player is within 3 blocks (`distanceToSqr < 9`), the fish is used and it trusts one time in three (entity event 41, hearts) or shows smoke (event 40); otherwise `Animal.mobInteract`.
  - `SitWhenOrderedToGoal` (2): a tame cat on the ground and dry sits when told, or whenever its owner is away; not beside an owner who was just hurt.
  - `CatRelaxOnOwnerGoal` (3, no flags): a tame cat not told to sit, within 10 blocks of its sleeping owner whose bed (`#cats_can_lie_on`) is under the owner's block, goes (at 1.1) to the block past the bed's foot (the bed's facing, reversed) unless another cat lies or relaxes within 2 blocks of it. Within `sqrt(2.5)` of the owner it relaxes (looks at the owner, 45 and 45) for 8 goal ticks (`adjustedTickDelay(16)`) and then lies down. `stop`: if the owner's sleep timer is 100 or more and the level random's float is below `gameplay/cat_waking_up_gift_chance`, the morning gift: `randomTeleport` to its block plus (-5..5, -2..2, -5..5) avoiding `#cat_does_not_teleport_to`, then the `gameplay/cat_morning_gift` loot (`minecraft:gift`) dropped at its block, offset by its body yaw (x - sin, z + cos).
  - The gift chance comes from the `minecraft:day` timeline: 0.7 from tick 23667 to tick 362 of the day, else 0 (constant ease, `maximum` modifier over the default 0).
  - `CatLieOnBlockGoal(1.1, 8)` (5): `MoveToBlockGoal` with search range 8, vertical range 6 starting at -2 (so the y offsets tried are -3, 2, -4, 3, -5, 4, -6, 5, -7), a new search every 40 ticks; a `#cats_can_lie_on` block with air above; it lies down once within 1 block of the block above's centre. `MoveToBlockGoal`: a path to the block's top centre at the speed, a new one every 40 ticks away from it, staying `nextInt(nextInt(1200) + 1200) + 1200` ticks, giving up after 1200 ticks away.
  - `FollowOwnerGoal(1.0, 10, 5)` (6): from 10 blocks, until 5; a new path every 5 goal ticks; 12 blocks or more away it teleports next to its owner. `TamableAnimalPanicGoal` teleports the same way.
  - `CatSitOnBlockGoal(0.8)` (7): search range 8, vertical range 1; `#cats_can_sit_on` with air above: a chest nobody has open, a lit furnace, the foot of a bed; it sits while on it.
  - `BreedGoal(0.8)` (10 for cats, 9 for ocelots): `Cat.canMate` needs both tame; the kitten takes either parent's coat, and of a tame parent its owner, tameness and the parents' collars mixed (`DyeColor.getMixedColor`).
  - `Creeper.registerGoals`: `AvoidEntityGoal(Ocelot, 6, 1.0, 1.2)` then `AvoidEntityGoal(Cat, 6, 1.0, 1.2)`, both at 3.
  - `Cat.handleLieDown`: a lying cat with a sleeping player within 2 blocks lies on top of them (drawn 0.15 further over).
- **Changed:**
  - Vendored `entities`:
    - `feline_ai.rs`: `SitGoal`, `RelaxOnOwnerGoal`, `BlockGoal` (lying and sitting on blocks), `FollowOwnerGoal`, `BreedGoal`, the panic goal's teleport, `random_teleport`, `find_nearest_block`, `can_lie_on`, `can_sit_on`; `FelineState` gains the owner's sleep, the resting cats, the gift chance, the mates and the follow and breed state.
    - `world/felines.rs`: `feline_interact` (taming, sitting, collars, healing, love, ocelot trust), `feline_breed`, the surroundings each tick (owner, owner's sleep, resting cats, mates, sleepers nearby), the morning gift after the goals, `set_cat_gift_chance`, `take_cat_gifts`.
    - `controls.rs`: `PlayerControls.sleeping` and `sleep_timer`. `monster_ai.rs`: creepers avoid ocelots and cats; `look_at` is `pub(crate)`. `loot.rs`: `roll_cat_gift`. `feline.rs`: the teleport tag and the gift table.
  - Vendored `world`: `Level::cat_gift_chance` reads the attribute.
  - `minecraft_terrain`: the right-click (`mob_actions.rs`); the server sets the gift chance each tick and drops the gifts (`drop_cat_gifts`); `MobShow.on_sleeper`.
  - `minecraft_survival` and `render_anim`: `PlayerView.sleeping` and `sleep_timer` (the bed's head and the sleep timer) for the host and each friend, passed on in the players' controls.
- **Tests:**
  - `world::felines::tests::a_wild_cat_takes_raw_fish_and_is_tamed_one_time_in_three`, `its_owner_sits_a_tame_cat_down_dyes_its_collar_and_others_cannot`, `a_tame_cat_follows_the_player_with_its_owner_s_uuid`, `a_creeper_runs_from_a_cat_and_from_an_ocelot`, `an_ocelot_learns_to_trust_only_while_tempted_and_close`, `a_tame_cat_lies_down_on_a_bed`, `a_cat_curls_up_on_its_sleeping_owner_s_bed_and_brings_a_morning_gift`, `two_tame_cats_in_love_have_a_tame_kitten`.
  - `mob_actions::tests::would_interact_agrees_with_interact` with a stray cat, a tame cat (its owner and another player), collar dyes and an ocelot.
  - `mob_tracking::tests::a_friend_sees_a_cat_and_an_ocelot_from_their_look_tags` also checks the cat lying on a sleeper.
  - Runs: `minecraftoss-entities` feline tests 13, `minecraft_terrain` lib tests for mob actions, tracking, cats and ocelots 53, `minecraft_survival` hit-key tests 4, all green. `check -p launcher` green. The full suites of the touched crates run at the end of the group (commit 3).
- **Decisions:**
  - The players' sleep reaches the level through `PlayerControls` (sent every tick already), so a friend's sleep counts for that friend's cats like the host's.
  - The lying search keeps vanilla's vertical order from -2 (offsets -3, 2, -4, ...), so a cat does not search for a bed at its own level; it still sits on the foot of beds (`CatSitOnBlockGoal`) and lies on its sleeping owner's bed.
  - A chest's open count is not known to the mobs; every chest counts as closed for sitting. Cats on a chest do not yet stop it opening (`ChestBlock.isChestBlockedAt`); listed under Left for later.
  - The gift chance is read at the level's first biome; no biome sets the attribute.
  - Entity events 6, 7, 18, 40 and 41 are recorded but the client draws no particles for entity events yet (as for wolves).
  - Phantoms do not exist yet, so cats do not scare them (group `phantoms`).
- **Test steps (survival, normal difficulty):**
  1. Use the cat egg on grass. Hold raw cod, stand still and let it come to you, then right-click it with cod until it takes (some tries fail): it sits down and wears a red collar (**61_cat_tamed**).
  2. Right-click it with an empty hand: it stands up and follows you. Walk 15 blocks away fast: it appears next to you. Right-click again: it sits and stays.
  3. Right-click it with the blue dye: its collar turns blue.
  4. Use the creeper egg 3 blocks from the cat: the creeper runs away from the cat (**62_creeper_flees**).
  5. Place a chest next to the standing cat: it walks onto the chest and sits on it (**63_cat_on_chest**).
  6. At night place the bed and sleep in it with the tame cat standing near: the cat comes and lies at the foot of the bed. In the morning, when you wake up, it may have left a gift on the floor (string, a feather, rabbit hide, a rabbit's foot, raw chicken or rotten flesh) (**64_morning_gift**).
  7. Use the ocelot egg, hold cod and stand still: when it creeps within 3 blocks, right-click it with cod until hearts show: it no longer runs from you (**65_ocelot_trusts**).
  8. Tame a second cat, feed both cod at full health: hearts, and a kitten with a collar appears.
  9. Friend window: the friend tames their own cat with cod; it follows the friend, not the host; only the friend can sit it down; both see the same cats sitting and lying (**66_friend_cat**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cat_spawn_egg*6,minecraft:ocelot_spawn_egg*3,minecraft:creeper_spawn_egg*2,minecraft:cod*64,minecraft:salmon*32,minecraft:blue_dye*4,minecraft:red_bed,minecraft:chest*2,minecraft:iron_sword,minecraft:cooked_beef*16
```

### 3. Minecraft villages and swamp huts get their cats

After 75b10b22.

- **Vanilla (javap, 26.3):**
  - `MinecraftServer` gives only the Overworld its custom spawners (`PhantomSpawner`, `PatrolSpawner`, `CatSpawner`, `VillageSiege`, `WanderingTraderSpawner`). `ServerChunkCache.tickChunks` runs `ServerLevel.tickCustomSpawners(spawnEnemies)` after the block-ticking chunks, only while the `spawn_mobs` game rule is on.
  - `CatSpawner.tick`: `nextTick` counts down (it starts at 0 and is not saved); at 0 or below it is reset to 1200 and the spawner looks once. `getRandomPlayer` (the living players, one drawn with the level random); x and z offsets of `(8 + nextInt(24)) * (nextBoolean ? -1 : 1)` each from the player's block, at the player's y. It gives up unless every chunk from 10 blocks around is loaded (`hasChunksAt`). Cats have no spawn placement, so any spot is fine (`isSpawnPositionOk` with `NO_RESTRICTIONS`).
  - Near a village (`isCloseToVillage(pos, 2)`: at most 2 sections from a village section): more than 4 occupied `HOME` points of interest within 48 blocks and fewer than 5 cats in the box of the spot grown by 48, 8, 48 → a cat. Otherwise, in a piece of a `#cats_spawn_in` structure (the swamp hut): no cat in the box grown by 16, 8, 16 → a persistent cat.
  - `spawnCat`: `EntityType.create(NATURAL)`, then `finalizeSpawn(NATURAL)`, then `setPersistenceRequired` for the hut, then `snapTo(pos, 0, 0)` (the block's bottom centre), then `addFreshEntityWithPassengers`. `finalizeSpawn` runs before the cat is placed, so its variant's structure test looks at (0, 0, 0): a spawner cat in a swamp hut gets an ordinary coat (black only at full moon), unlike the hut's generated cat.
- **Changed:**
  - New `minecraft_terrain/src/custom_spawners.rs`: `CustomSpawners` (the hook later spawners join: wandering traders, patrols, phantoms), built per dimension with a `CatSpawner` in the Overworld only; `CatSpawner::next_spot` (the timer, the player and the offsets from the level random); the chunk, box and rule helpers.
  - `minecraft_terrain/src/server.rs`: `ServerSim.custom_spawners` and `tick_custom_spawners`, run right after the level tick (natural spawning) and before the mobs, while `spawn_mobs` is on. It reads the village distance and the occupied homes from the entity world's points of interest, counts the cats in the box, tests the swamp hut through the level's structure finder, and adds the cat.
  - Vendored `world/src/level/spawning.rs`: `Level::spawner_cat` (the natural cat's tag finalized with `finalize_cat` at the vanilla moment, then placed, persistent for the hut) and `Level::in_structure_piece`.
- **Tests:**
  - `custom_spawners::tests::the_cat_spawner_looks_every_minute_between_eight_and_thirty_one_blocks_away` (the 1200-tick timing, the offsets, no player) and `villages_take_up_to_five_cats_with_more_than_four_homes_and_huts_one` (the rules, the boxes, the chunks, Overworld only).
  - `server::tests::the_cat_spawner_brings_cats_to_a_village_with_more_than_four_occupied_homes`: no village, no cat; with 25 occupied homes the next cat comes exactly 1200 ticks later, wild and in a vanilla coat; never more than five; nothing while `spawn_mobs` is off.
  - Runs (with the whole group in place): `minecraftoss-entities` lib 253, `minecraftoss-world` lib 78, `minecraftoss-generator` lib 30, `minecraft_terrain` lib 1147, `render_anim` lib 151, all green. `minecraft_survival` lib 605 of 606: `the_end::tests::an_arrival_rebuilds_the_platform_once_its_chunks_are_in` (End chunk streaming, no cat code) failed once under load and passed twice alone. `check -p launcher` green. `custom_spawners.rs` passes `rustfmt --check --edition 2021`.
- **Decisions:**
  - The spawner state is not saved, as in vanilla (`CatSpawner.nextTick` is a plain field), so the first look comes on the first tick after loading.
  - The structure test keeps vanilla's order (finalize before placing), so spawner cats in swamp huts are not forced black.
  - The hook runs once per server tick of the Overworld level, after natural spawning, which is where `tickChunks` runs it.
- **Test steps (survival, normal difficulty, `spawn_mobs` on):**
  1. `/locate structure minecraft:village_plains`, `/tp` there by day and count the cats (**67_village_cats_before**).
  2. Wait in the middle of the village for about five minutes (or until night, when the villagers have claimed their beds): new cats appear in the village, up to five near you (**68_village_cats_after**). A village with four or fewer claimed beds gets none.
  3. `/locate structure minecraft:swamp_hut`, `/tp` into the hut. If the hut's cat is still there, kill it with the sword. Wait about a minute: a new cat appears in or near the hut and stays there, it does not despawn (**69_hut_cat**).
  4. `/gamerule spawn_mobs false`: no more cats come (nor other mobs). Set it back to true.
  5. Friend window: the friend sees the new village and hut cats.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:iron_sword,minecraft:cod*32,minecraft:red_bed*4,minecraft:villager_spawn_egg*6,minecraft:cooked_beef*16,minecraft:torch*32
```

## Group horse-family

### 1. Minecraft mules, llamas and trader llamas join the horse family

After 58a9ee21.

- **Vanilla (javap, 26.3):**
  - `EntityTypes.MULE`: `CREATURE`, `sized(1.3964844, 1.6)`, `eyeHeight(1.52)`, passenger attachment 1.2125, `clientTrackingRange(8)`. `EntityTypes.LLAMA` and `TRADER_LLAMA`: `CREATURE`, `sized(0.9, 1.87)`, `eyeHeight(1.7765)`, passenger attachment (0, 1.37, -0.3), `clientTrackingRange(10)`. `LLAMA_SPIT`: `MISC`, no loot table, `sized(0.25, 0.25)`, `clientTrackingRange(4)`, `updateInterval(10)`. Babies: `AbstractChestedHorse.babyDimensions` and `Llama.BABY_DIMENSIONS` both scale 0.5.
  - `AbstractChestedHorse.createBaseChestedHorseAttributes`: speed 0.17499999701976776, jump 0.5 (on the horse's 53 health; `randomizeAttributes` draws health 15 + `nextInt(8)` + `nextInt(9)`). `getInventoryColumns`: 5 with a chest; `Llama`: its strength. Saved: `ChestedHorse` and, with a chest, `Items` (`ItemStackWithSlot`). `dropEquipment`: the inventory, then the chest block. `mobInteract`: a tamed adult with sneak opens the inventory; a baby with a golden dandelion goes to `Animal`; food goes to `fedFood`; any other item on a wild one `makeMad` (rear and the angry sound); a chest on a tame one without one `equipChest` (the chest sound at pitch (r1 - r2) x 0.2 + 1); then `AbstractHorse.mobInteract`: a body item that fits (`isEquippableInSlot(BODY)`, carpets on llamas) is equipped, else `doPlayerRide`.
  - `AbstractHorse.handleEating`: wheat (heal 2, grow 20 s, temper 3), sugar (1, 30, 3), hay bale (20, 180, 0), apple and carrot (3, 60, 3), red mushroom (3, 0, 3), golden carrot (4, 60, 5, love when tame, adult and not in love), golden apples (10, 240, 10, love likewise); then heal when hurt, grow a baby (not age-locked), temper (when eaten or wild, below the max: 100); eating opens the mouth and plays the eat sound. `Llama.handleEating`: wheat (2, 10, 3), hay bale (10, 90, 6, love), max temper 30, no mouth.
  - `Horse.canMate` and `Donkey.canMate`: the other is a horse or donkey and both `canParent` (not ridden, tame, adult, full health, in love); `Mule` cannot mate. `Donkey.getBreedOffspring`: a mule with a horse, else a donkey; `Horse.getBreedOffspring`: a mule with a donkey, else a horse with the coat `nextInt(9)` (<4 this, <8 the partner's, else random of 7) and the markings `nextInt(5)` (<2, <4, else random of 5). `setOffspringAttributes`: health, jump and speed by `createOffspringAttribute(a, b, min, max)` (clamped parents, spread `|a - b| + 0.3 (max - min)`, mean plus spread times the mean of three doubles minus 0.5, reflected at the bounds); bounds health 15-30, jump 0.4-1.0, speed 0.1125-0.3375 (floats).
  - `Llama`: `Strength` (1-5) and `Variant` (creamy, white, brown, gray) saved. `finalizeSpawn`: strength `1 + nextInt(nextFloat() < 0.04 ? 5 : 3)`, the group's variant or a random one. `canMate` only with another llama (trader llamas too) when both can parent; `getBreedOffspring`: its own kind (`makeNewLlama`; a trader llama's child is persistent), strength `nextInt(max(s1, s2)) + 1` plus one when `nextFloat() < 0.03`, the variant of either parent by `nextBoolean`. `canEatGrass` false, `canPerformRearing` false, `isImmobile` only while dead or eating.
  - `Llama.registerGoals` (no `addBehaviourGoals`): 0 `FloatGoal`, 1 `RunAroundLikeCrazyGoal(1.2)`, 2 `LlamaFollowCaravanGoal(2.1)`, 3 `RangedAttackGoal(1.25, 40, 20)`, 3 `PanicGoal(1.2)`, 4 `BreedGoal(1.0)`, 5 `TemptGoal(1.25, #llama_tempt_items, false)`, 6 `FollowParentGoal(1.0)`, 7 `WaterAvoidingRandomStrollGoal(0.7)`, 8 `LookAtPlayerGoal(Player, 6)`, 9 `RandomLookAroundGoal`. Targets: 1 `LlamaHurtByTargetGoal` (stops once it has spat: `didSpit`), 2 `LlamaAttackWolfGoal` (`NearestAttackableTargetGoal<Wolf>`, interval 16, must reach, wild wolves, follow distance x 0.25). `TraderLlama` adds 1 `PanicGoal(2.0)`, targets 1 `TraderLlamaDefendWanderingTraderGoal`, 2 zombies (not zombified piglins, in sight) and 2 illagers.
  - `Llama.spit`: the spit at x - (width + 1) x 0.5 x sin(body yaw), eye - 0.1, z + (width + 1) x 0.5 x cos(body yaw); aimed at the target's x, y + height / 3 and z with an added rise of 0.2 x the horizontal distance, `shoot` at 1.5 with uncertainty 10; the spit sound at pitch 1 + (r1 - r2) x 0.2; `didSpit` set.
  - `LlamaSpit.tick`: `Projectile.tick` (left-owner check), the hit along the move (a block clip, then the nearest entity box grown by the margin), position + motion, the rotation eased (`lerpRotation` 0.2); gone when no block in its box is air or it is in water; motion x 0.99 and gravity 0.06. `onHitEntity`: only with a living owner, `spit` damage 1 (no discard); `onHitBlock`: discard. `spit` damage type: message `mob`, scaling `when_caused_by_living_non_player`.
  - `LlamaFollowCaravanGoal`: not leashed and not in a caravan; the nearest llama (box grown 9, 4, 9) in a caravan without a tail, else the nearest leashed one without a tail; not within 2 blocks; the head or its chain (at most 8) must be on a lead; it follows to 2 blocks short of the head, speeding up x 1.2 (to 3) beyond 26 blocks. `TraderLlama`: `DespawnDelay` 47999, counting down unless tame, leashed to another holder, ridden, age-locked or persistent.
  - Sounds: `entity.mule.*` and `entity.llama.*` (ambient, hurt, death, eat, angry, chest; `entity.llama.step` at 0.15, `spit`, `swag`). Loot tables `entities/mule`, `entities/llama`, `entities/trader_llama` (leather).
- **Changed:**
  - Vendored `entities`:
    - `horse.rs`: `HorseKind` += `Mule`, `Llama`, `TraderLlama` with sizes, eye heights, attributes, sound families, mating and offspring kinds, food and tempt tags; `HorseState` gains strength, the body item (carpet), the chest's items, a lead flag, the trader's despawn delay and `LlamaFight` (target, attacker, spit and caravan state); `Feeding` (the `handleEating` table) and `offspring_attribute`.
    - New `llama_ai.rs`: the llama goals on the cow goal framework (`HurtByGoal`, `NearestFoeGoal` for wolves and zombies, `SpitGoal`, `CaravanGoal`) and `LlamaContext`.
    - New `llama_spit.rs`: the projectile. New `world/equines.rs`: `LlamaSpitEntity` and its store and tick, the inputs of the horse goals (mates by `canMate`, tempting players, the llama's surroundings), spitting and caravans, `horse_interact` (feeding, rearing, chests, carpets), `breed_horses` (the offspring of every pair), the trader llama's despawn, the death drops.
    - `cow_ai.rs`: `registered_llama_goals`, the goal names, `TemptItems::LlamaTempt`, the context's mates, tempters and llama, the spit and caravan effects. `world.rs`: `EntityKey::LlamaSpit`, `PlayerHitKind::Spit`, the spit store everywhere the shulker bullets are, the llama goals on spawn, the horses' own loot tables and drops (they dropped cow loot before), attackers noted on hurts. `combat.rs`, `pushing.rs`, `riding.rs`, `shulker_bullets.rs`: the new key. `bin/check_zombie_player_hits.rs`: the trident and spit hit kinds (it no longer built with `--tests` since the drowned group).
  - Vendored `world`: trader llamas finalize like llamas.
  - `minecraft_terrain`:
    - `SIMULATED_KINDS`, `spawn_saved` and `mob_tags_where` for the three kinds (variant, strength, `equipment.body` kept with the other slots, `ChestedHorse`, `Items` read for the drops, `DespawnDelay`) and `minecraft:llama_spit` (owner, motion, rotation).
    - `mob_tracking.rs`: tracking ranges (mule 8, llamas 10, spit 4), the spit as a tracked projectile, `DespawnDelay` volatile.
    - `mob_actions.rs`: right-clicks on the horse family go to `horse_interact` (horses no longer take the cow's wheat-and-bucket path).
    - `horse_render.rs`: the mule as the donkey model scaled 0.92 in `horse/mule`; `llama_texture`. `spawner_display.rs`: mules in cages. `HATCHING_EGGS` grows to 42.
  - `minecraft_survival`: `PlayerHitKind::Spit` hurts the player as `MobAttack::Spit` (`minecraft:spit`, "was slain by Llama").
- **Tests:**
  - `world::equines::tests::mules_and_llamas_take_their_sizes_health_and_voices`, `a_tame_horse_and_a_tame_donkey_in_love_have_a_mule`, `llamas_breed_llamas_with_strength_from_their_parents`, `a_tame_llama_takes_a_chest_and_a_carpet_and_a_wild_one_gets_angry`, `a_llama_spits_at_the_player_who_hit_it_once`, `a_trader_llama_leaves_when_its_delay_runs_out`; `llama_spit::tests` (flight, hits and the wall, water); `llama_ai::tests::a_caravan_counts_only_when_its_first_llama_is_on_a_lead`.
  - `server_mobs::tests::a_mule_a_llama_and_a_trader_llama_save_and_load_with_variant_strength_carpet_and_chest`, `llama_spit_in_flight_saves_and_loads_with_its_owner`; `mob_tracking::tests::a_friend_sees_mules_and_llamas_from_their_look_tags_and_their_new_chests_and_carpets`, `a_friend_sees_llama_spit_fly`; `mob_actions::tests::would_interact_agrees_with_interact` with llamas, a mule and a horse; `horse_render::tests::a_mule_is_a_donkey_scaled_to_0_92_in_its_own_coat`; `server::tests::summoned_llamas_draw_their_strength_and_coat_and_mules_and_trader_llamas_live`; the egg survey (42 eggs).
  - `death::tests::a_llama_s_spit_slays_the_player_by_the_llama`.
  - Runs: `minecraftoss-entities` lib 263, `minecraftoss-world` lib 78, `minecraft_terrain` lib 1152 (the timing test `sections::tests::background_full_update_matches_the_synchronous_one` skipped, see the drowned group), `minecraft_survival` lib 607, all green. `check -p launcher` green. The three new files pass `rustfmt --check --edition 2021`.
- **Decisions:**
  - Mules, llamas and trader llamas share the cow store with horses and donkeys (hit key 11), as the batching rule says; no new store and no new hit-key byte.
  - The llama's target goals run in the same goal selector as its other goals (registered first, with only the target flag), which gives vanilla's order: targets, then goals.
  - Carpets, chests and the coat reach friends through the look tag (`equipment.body`, `ChestedHorse`, `Variant`), not through new `MobShow` fields: they change rarely and the tag already carries them.
  - There are no leads yet, so no llama is ever on a lead: caravans form only once group `wandering-trader` adds leads, and a loaded `leash` stays in the saved tag untouched.
  - Taming by riding is group `riding`'s; a tame horse, donkey, mule or llama can be summoned with `{Tame:1b}`. An empty hand on a horse-family mob does nothing until then (vanilla mounts it).
  - Llama spit keeps flying after it hits a mob or player, as in 26.3 (`onHitEntity` does not discard it); the hurt cooldown keeps it to one hit.
  - The horse family drops its own loot now (leather for horses, donkeys, mules and llamas) instead of the cow's beef, and the body item, the chest's items and the chest.
  - The mule is drawn already in this commit (the donkey model and its own texture), so no hatched mob stays invisible.
  - A loaded horse, donkey, mule or llama's health is capped at its maximum, and without a saved `Health` it is the maximum (`LivingEntity.readAdditionalSaveData` reads the attributes first and `setHealth` clamps). Summoned and hatched ones carried the catalog's 53 over a drawn maximum of 15 to 30 before. Breeding needs full health, so this matters for step 2.
- **Test steps (survival, normal difficulty, by day):**
  1. Use the mule egg on grass: a brown mule with long ears, a little bigger than a donkey, walks about (**70_mule**).
  2. `/summon minecraft:horse ~ ~ ~ {Tame:1b}` and `/summon minecraft:donkey ~2 ~ ~ {Tame:1b}`. Feed each a golden carrot: hearts, and soon a baby mule stands between them (**71_mule_foal**). Feeding a mule a golden carrot gives it hearts but never a foal.
  3. `/summon minecraft:mule ~ ~ ~ {Tame:1b}` and right-click it with a chest: the chest sound plays and two chests hang on its sides (**72_mule_chest**). Kill it with the sword: it drops the chest (and sometimes leather).
  4. Right-click a wild mule from the egg with a stick: it rears and neighs angrily.
  5. Save and quit, then load again: the mule foal is still a foal and the chested mule still has its chests.
  6. Friend window: the friend sees the mules, the foal and the chests.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:mule_spawn_egg*4,minecraft:golden_carrot*16,minecraft:chest*4,minecraft:stick,minecraft:iron_sword,minecraft:cooked_beef*16
```

### 2. Minecraft llamas and trader llamas are drawn like vanilla and spit

After c0e12ab3.

- **Vanilla (javap, 26.3):**
  - `LlamaModel.createBodyLayer(CubeDeformation)` (128 x 64): `head` at (0, 7, -6) with the muzzle (-2, -14, -10; 4 x 4 x 9 at 0, 0), `neck` (-4, -16, -6; 8 x 18 x 6 at 0, 14) and two `ear`s (-4 and 1, -19, -4; 3 x 3 x 2 at 17, 0); `body` at (0, 5, 2) turned pi/2 about X (-6, -10, -7; 12 x 18 x 10 at 29, 0); `right_chest` at (-8.5, 3, 3) and `left_chest` at (5.5, 3, 3), both turned pi/2 about Y (-3, 0, 0; 8 x 8 x 3 at 45, 28 and 45, 41); the four legs at (+-3.5, 10, 6 and -5) (-2, 0, -2; 4 x 14 x 4 at 29, 29).
  - `BabyLlamaModel.createBodyLayer` (64 x 64): `head` at (0, 12, -4) with (-3, -9, -4; 6 x 11 x 4 at 0, 0), (-1.5, -7, -7; 3 x 3 x 3 at 0, 15) and the ears (0.5 and -2.5, -11, -3; 2 x 2 x 2 at 20, 4 and 20, 0); legs at (+-2.5, 16.5, 4.5 and -3.5) (-1.4 or -1.6, -0.5, -1.5; 3 x 8 x 3 at 0 or 12, 45 and 34); `body` at (0, 14, 2.5) (-4, -3, -8.5; 8 x 6 x 13 at 0, 15); the chests at (-8.5 and 5.5, 4, 3) as on the adult. It keeps `LlamaModel.setupAnim`.
  - `LlamaModel.setupAnim`: the head's pitch and yaw from the look (x 0.017453292); right hind and left front legs `cos(pos x 0.6662) x 1.4 x speed`, the other two with + pi; both chests visible only with `hasChest`.
  - `LayerDefinitions`: `LLAMA` and `TRADER_LLAMA` = `LlamaModel.createBodyLayer(NONE)`, `LLAMA_BABY` and `TRADER_LLAMA_BABY` = the baby layer, `LLAMA_DECOR` with `CubeDeformation(0.5)`, `LLAMA_BABY_DECOR` with `CubeDeformation(0.2)`; no mesh scaling.
  - `LlamaRenderer`: `AgeableMobRenderer` with shadow 0.7; textures `entity/llama/llama_<creamy|white|brown|gray>` and `_baby`; `hasChest` = not a baby and `hasChest()`.
  - `LlamaDecorLayer.submit`: an adult (`!isBaby`) whose body item has an `Equippable` asset draws that asset's `llama_body` layer on the decor model; otherwise a trader llama draws `trader_llama_baby` or `trader_llama`. The carpets' assets (`equipment/<dye>_carpet.json`) name the `llama_body` textures `<dye>`, the trader's `trader_llama` and `trader_llama_baby`.
  - `LlamaSpitModel.createBodyLayer` (64 x 32): one part `main` of seven 2 x 2 x 2 cubes at (-4, 0, 0), (0, -4, 0), (0, 0, -4), (0, 0, 0), (2, 0, 0), (0, 2, 0), (0, 0, 2), all at 0, 0. `LlamaSpitRenderer.submit`: translate (0, 0.15, 0), turn Y by `yRot - 90`, then Z by `xRot` (both lerped), texture `entity/llama/llama_spit`, no overlay.
  - `LlamaSpit.recreateFromPacket` adds seven `spit` particles on the client; not drawn here (Left for later).
- **Changed:**
  - New `minecraft_terrain/src/llama_render.rs`: the adult and baby models and their pose, the coat by variant, the chests, the decor layer (carpets on adults, the trader's blanket), and the spit model drawn between its last two places with its turn eased.
  - `minecraft_terrain`: `mesh.rs` loads the 27 llama textures; `client_mobs.rs` gives llamas the 0.7 shadow (horses, donkeys and mules keep 0.75); `spawner_display.rs` draws llamas and trader llamas in cages.
  - `minecraft_survival/src/entity_view.rs`: draws the llamas after the horses and the spit beside the shulker bullets, for the host and friends alike (the friend's copies come from the look tag and the tracked moves of commit 1).
  - No vendored file changed in this commit.
- **Tests:**
  - `llama_render::tests::a_llama_has_nine_cubes_and_two_more_with_a_chest_and_stands_on_the_ground`, `the_legs_swing_in_pairs_and_the_head_follows_the_look`, `the_decor_is_an_adult_s_carpet_or_the_trader_s_blanket_and_grows_half_a_pixel`, `a_saved_llama_draws_its_coat_chests_and_carpet_and_a_baby_trader_its_blanket` (loaded from a saved tag through `spawn_saved` and drawn through `ClientMobs`, the friend's path too), `spit_is_seven_little_cubes_turned_along_its_flight`, `flying_spit_is_drawn_between_its_last_two_places`.
  - `spawner_display::tests::a_zombie_a_blaze_the_nether_mobs_and_a_shulker_sit_inside_their_cage` with the mule, the llama and the trader llama (all within 0.05 of the cage).
  - The brief's family tests (a mule from horse x donkey, a llama spits at its attacker, the carpet and chest save round trip) came with commit 1: `world::equines::tests::a_tame_horse_and_a_tame_donkey_in_love_have_a_mule`, `a_llama_spits_at_the_player_who_hit_it_once`, `server_mobs::tests::a_mule_a_llama_and_a_trader_llama_save_and_load_with_variant_strength_carpet_and_chest`; they still pass.
  - Runs: `minecraft_terrain` lib 1159 (3 ignored), `minecraft_survival` lib 607, all green. `check -p launcher` green. `llama_render.rs` passes `rustfmt --check --edition 2021`.
- **Decisions:**
  - The carpet texture comes from the carpet's id (`<dye>_carpet` to `entity/equipment/llama_body/<dye>`), as `armor_render.rs` maps armour items to their assets; it is what the 16 `equipment/<dye>_carpet.json` files say. Any other body item draws no carpet.
  - The decor layer's `CubeDeformation` is drawn by scaling each cube about its centre, as `cat_render.rs` does for the collar, so the texture coordinates stay those of the plain cube.
  - Llamas get no new `MobShow` field: the chest, carpet and coat come from the look tag (commit 1), and the pose (head, legs) from the tracked moves.
  - The spit's seven particles when it appears are not drawn; listed under Left for later.
- **Test steps (survival, normal difficulty, by day):**
  1. Use the llama egg on grass a few times: llamas in creamy, white, brown and gray coats walk about, with their long necks and ears; a baby is small with a short neck (**73_llamas_drawn**).
  2. `/summon minecraft:llama ~ ~ ~ {Tame:1b}`. Right-click it with a chest: two chests hang on its sides. Right-click it with the white carpet: a white blanket covers its back, neck and chests. Summon a second tame llama and right-click it with the red carpet: a red one (**74_llama_chest_carpet**).
  3. Hit a llama from the egg with the sword: it turns and spits once; the little white ball of spit flies at you and hurts a little (half a heart) (**75_llama_spits**).
  4. Use the trader llama egg: it wears the blue trader's blanket. `/summon minecraft:trader_llama ~ ~ ~ {Age:-24000}`: the foal wears the small blanket (**76_trader_llamas**). Use the zombie egg 5 blocks from a trader llama: it spits at the zombie.
  5. `/summon minecraft:horse ~ ~ ~ {Tame:1b}` and `/summon minecraft:donkey ~2 ~ ~ {Tame:1b}`, feed both a golden carrot: hearts, and a mule foal stands between them. Or use the mule egg (as in step 2 of commit 1).
  6. Place the two spawners and use the llama egg on one and the trader llama egg on the other: a small llama turns inside each cage (**77_llama_spawner**).
  7. Save and quit, then load again: every llama keeps its coat, chests and carpet.
  8. Friend window: the friend sees the same llamas with their coats, chests, carpets and the trader's blanket, and sees the spit fly when the host hits a llama (**78_friend_llamas**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:llama_spawn_egg*4,minecraft:trader_llama_spawn_egg*2,minecraft:horse_spawn_egg*2,minecraft:donkey_spawn_egg*2,minecraft:mule_spawn_egg*2,minecraft:golden_carrot*16,minecraft:chest*2,minecraft:white_carpet*2,minecraft:red_carpet*2,minecraft:hay_block*8,minecraft:zombie_spawn_egg*2,minecraft:spawner*2,minecraft:iron_sword,minecraft:cooked_beef*16
```

## Group riding

### 1. Minecraft players tame and ride horses, donkeys, mules and llamas

After 6bb46886.

- **Vanilla (javap, 26.3):**
  - `EntityTypes` passenger attachments: horse 1.44375, donkey 1.1125, mule 1.2125, llama and trader llama (0, 1.37, -0.3); a baby's are scaled with its dimensions. `AbstractHorse.getPassengerAttachmentPoint` adds (0, 0.15 x `standAnimO` x scale, -0.7 x `standAnimO` x scale) turned by `-yRot`; the llama's point is turned by `-yRot` too. The player's vehicle attachment is 0.6.
  - `AbstractHorse.mobInteract`: a vehicle or a baby goes to `Animal.mobInteract`; a tame adult with sneak opens the inventory; a held item tries `interactLivingEntity` (the saddle's `Equippable`, `canUseSlot(SADDLE)`: alive, adult, tame), then a body item that fits and is not worn yet (`equipBodyArmor`); otherwise `doPlayerRide` (stop eating and standing, then `startRiding`). `Horse.mobInteract` and `AbstractChestedHorse.mobInteract` first feed food and `makeMad` on a wild one for any other item (and the chest on a tame chested one). `getEquipSound(SADDLE)` is `entity.horse.saddle`; horse armour's `Equippable` sound is `entity.horse.armor`. Entity type tags: `can_equip_saddle` (horse, skeleton and zombie horse, donkey, mule, pig, strider, camels, nautiluses; no llamas), `can_wear_horse_armor` (horse, zombie horse).
  - `RunAroundLikeCrazyGoal(1.2)` (flag MOVE): `canUse` when not mob-controlled (`isMobControlled`: the first passenger is a `Mob`), not tame and carrying, with `DefaultRandomPos.getPos(5, 4)`; `canContinueToUse` while not tame, the path runs and it carries. `tick` (not every tick, so `adjustedTickDelay(50)` = 25): one in 25, the first passenger a player: `nextInt(maxTemper) < temper` tames (`tameWithName`: owner, tame, entity event 7) and returns; else temper + 5; then `ejectPassengers`, `makeMad` (rear and the angry sound) and entity event 6.
  - `getControllingPassenger`: a saddled horse's first passenger when it is a player, else `Mob.getControllingPassenger` (a non-NoAI horse's first passenger when it is a mob). `Mob.updateControlFlags` turns off the MOVE, JUMP and LOOK goals while a mob controls it.
  - `LivingEntity.travelRidden`: `getRiddenInput` (zero on the ground while rearing with no pending jump and no `allowStandSliding`; else the rider's `xxa` x 0.5 and `zza`, x 0.25 backwards), `tickRidden` (yaw from the rider, pitch x 0.5, head, body and old yaw snap to it; on the ground a pending jump runs `executeRidersJump` and the pending scale clears), `setSpeed(getRiddenSpeed)` (the movement speed attribute) and `travel`. `getFlyingSpeed` is `speed x 0.1` with a player in control (0.02 otherwise).
  - `executeRidersJump(scale, input)`: motion Y = `getJumpPower(scale)` (jump strength x scale x block jump factor), `needsSync`; forward input adds (-0.4 sin yaw, 0, 0.4 cos yaw) x scale. `onPlayerJump(power)`: only saddled; a negative power is 0, else `allowStandSliding` and rear; the pending scale is `getPlayerJumpPendingScale`: 1.0 from 90, else 0.4 + 0.4 x power / 90. `canJump` = saddled; `handleStartJump`: rear and `entity.horse.jump` (0.4, 1).
  - `LocalPlayer.aiStep` charges the jump on the client while `jumpableVehicle()` (the controlled vehicle that `canJump`) has no cooldown: a press zeroes the ticks; held, ticks + 1 and the scale is ticks x 0.1 below 10, else 0.8 + 2 / (ticks - 9) x 0.1; on release the ticks go to -10, `onPlayerJump(floor(scale x 100))` and `START_RIDING_JUMP` are sent; negative ticks count back up and zero the scale at 0. `JumpableVehicleBar`: `hud/jump_bar_background` 182 x 5 at the experience bar's place, `hud/jump_bar_progress` to `Mth.lerpDiscrete(scale, 0, 182)`, `hud/jump_bar_cooldown` while cooling (never for horses).
  - `AbstractHorse.getDismountLocationForPassenger`: the escape vector at `yRot + 90` (right-handed), then `- 90`; in each column from the horse's feet up to its top + 0.75, the first valid floor where a standing, crouching or swimming player fits; else the horse's position. `aiStep` grazes only when it carries nobody.
  - `EquineSaddleModel.createSaddleLayer`: the body mesh plus `saddle` on the body (26, 0; -5, -8, -9; 10 x 9 x 9, deformation 0.5), and on `head_parts` the two bit rings (29, 5; +-2/-3, -9, -6; 1 x 2 x 2), the reins (32, 2; +-3.1, -6, -8; 0 x 3 x 16, turned -0.5235988 about X, shown only while ridden), `head_saddle` (1, 1; -3, -11, -1.9; 6 x 5 x 6, 0.22) and `mouth_saddle_wrap` (19, 0; -2, -11, -4; 4 x 5 x 2, 0.2); 64 x 64. `LayerDefinitions`: `HORSE_ARMOR` = `createBodyMesh(0.1)` (head, mane, mouth, legs and tail grow 0.1, the body keeps 0.05), saddles scaled like their horse (1.1, 0.87, 0.92). Textures: `entity/equipment/horse_saddle|donkey_saddle|mule_saddle/saddle`, `entity/equipment/horse_body/<copper|iron|gold|diamond|netherite|leather>`.
- **Changed:**
  - `sim`: the authority keeps each client's last `forwardmove` and `rightmove` beside its old buttons (`old_moves`, set in the finalize step), so the host reads every rider's keys, its own and each friend's.
  - Vendored `entities`:
    - `controls.rs`: `PlayerControls` gains `forward`, `strafe` and `jumping`.
    - `horse.rs`: `HorseKind::saddleable`, `wears_armor` and `passenger_attachment`; `HorseState` gains `saddled`, `owner`, `jump_pending` and `ridden`, with `jump_scale`, `on_player_jump` and `ridden_input`; new `RideJump` (the client's jump charge).
    - `world/riding.rs`: a vehicle is a strider or any of the horse family. `passenger_point`, `seat`, `seat_offset`, `controlling_mob` (zombies and skeletons steer a horse), `controlling_player` (a saddled horse's first rider), `can_ride`, `start_riding` (one rider), `stop_riding` with the horse's own dismount search (`horse_dismount_spot`), `order_vehicle` for a mob-steered horse, `eject_passengers`, the per-player jump charge (`tick_ride_jumps`, `ride_jump_scale`); `take_player_dismounts` carries the spot a bucked rider lands on.
    - `world.rs`: the ride charge ticks before the mobs; a horse controlled by a player runs `ride_tick` instead of its AI, a mob-steered horse turns its MOVE, JUMP and LOOK goals off, grazing waits while it carries someone, and `ridden` follows its passengers.
    - `world/equines.rs`: `ride_tick` (travelRidden), `horse_ride` (the rider, the mob control and the wild-rider state for the goal), `make_mad`, taming and bucking from the goal's effects, saddling and horse armour by right-click, `horse_rides_on_use` (when a click mounts), the saddle in the death drops.
    - `cow_ai.rs`: `RunAroundLikeCrazyGoal` as above (`Crazy` context, `Tame`, `Temper`, `Buck` effects). `movement.rs`: `travel_air_flying` (the air acceleration as a parameter) and `Body::jump_power`.
  - `minecraft_terrain`:
    - `server_mobs.rs`: `carries` takes any vehicle the entity world simulates (strider and the horse family), so a skeleton or zombie on a horse or llama loads with its rider; a horse with riders no longer loads alone. Horses, donkeys and mules load and save `equipment.saddle`, `equipment.body` (horse armour) and `Owner`.
    - `mob_actions.rs`: right-click mounts a horse, donkey, mule or llama like vanilla (empty hand on any adult, or any other item on a tame one). `server.rs`: a bucked rider is put down on the dismount spot.
    - `mob_tracking.rs`: `MobShow.ridden` shows the reins on the friend's copy; the saddle and armour reach friends in the look tag.
    - `horse_render.rs`: the saddle layer (seat, bridle, reins while ridden) on horses, donkeys and mules and the horse armour layer (leather in its undyed colour).
  - `minecraft_survival`: `PlayerView` carries the ride keys; `ride_seat` sits any rider on its vehicle's passenger point (the strider keeps its stride bob); `Player.jump_bar`, sent to friends in `PlayerView.jump_bar`.
  - `render_anim`: `minecraft_riding::ride_input` turns the authority's last keys into Minecraft's `zza` and `xxa` (normalised, x 0.98) and the jump key, for the host and every friend; the host and each friend get their jump bar. `hud`: the jump bar replaces the experience bar while it shows. `frame`: `MinecraftUi.jump_bar`. `minecraft_net`: `PlayerView.jump_bar` (no protocol bump, track D's 4 holds).
- **Tests:**
  - `horse::tests::a_held_jump_charges_to_full_at_ten_ticks_and_eases_off_after`, `a_saddled_horse_takes_the_released_power_and_rears`.
  - `world::equines::tests::a_wild_horse_is_tamed_by_its_rider_when_its_temper_allows`, `a_wild_horse_bucks_its_rider_off_and_grows_tamer`, `a_tamed_horse_takes_a_saddle_and_its_rider_steers_it`, `a_full_charge_jumps_as_high_as_the_horse_s_jump_strength_allows` (jump strength 0.5 peaks between 1.4 and 2 blocks, 1.0 between 5 and 6.5; a llama has no jump bar and a passenger point 0.3 behind).
  - `server_mobs::tests::a_jockey_on_a_horse_loads_and_a_saddled_horse_keeps_its_saddle_armour_and_owner` (a skeleton on a horse and a zombie on a llama load mounted, the skeleton steers).
  - `mob_actions::tests::right_click_mounts_a_horse_and_a_saddle_goes_on_a_tame_one`; `would_interact_agrees_with_interact` with the new mounts.
  - `mob_tracking::tests::a_friend_sees_a_horse_get_its_saddle_and_armour_and_a_rider`; `horse_render::tests::the_saddle_adds_its_seat_and_bridle_and_reins_only_while_ridden`; `minecraft_riding::tests::the_keys_steer_like_a_minecraft_player_s_input`; `minecraft_hotbar::tests::the_jump_bar_fills_like_mth_lerp_discrete`; the codec round trip carries `jump_bar`.
  - Runs: `minecraftoss-entities` lib 269, `minecraft_terrain` lib 1162 (the timing test skipped as before), `minecraft_survival` lib 607, `minecraft_net` 16, `hud` 161, the `render_anim` riding tests, all green. `check -p launcher` green.
- **Decisions:**
  - Riding is server-authoritative: the host's entity world runs the horse from each rider's keys, and the jump charge runs there too (vanilla charges on the client and sends the power). The bar is read from the host's copy of the world and sent to friends in `PlayerView`, so a friend's bar lags by the round trip.
  - The keys come from the sim's last command per client: `forwardmove` and `rightmove` (-127 to 127, IW4's right is Minecraft's minus `xxa`) normalised like `KeyboardInput` and times 0.98 like `LivingEntity.aiStep`.
  - While a player controls a horse its goals do not tick (vanilla still ticks them, but `travelRidden` ignores the moves); its path is stopped so it does not walk off after the rider leaves.
  - A horse steered by a mob (zombies and skeletons, as on striders) passes the rider's path to its own navigation; there are no such jockeys in natural spawning yet (zombie horse and skeleton trap are group `undead-horses`).
  - `doPlayerRide` turns the player to the horse's facing on the server in vanilla; the fork leaves the player's view alone.
  - The rider's fall damage on a horse's fall, the gallop and breathe sounds and the taming hearts and smoke particles (entity events 7 and 6 are sent but the client draws no entity-event particles yet) are listed under Left for later. A rider right-clicking a carried horse's food is not diverted to `Animal.mobInteract`.
  - Leather horse armour is drawn in the undyed leather colour (`dyed_color` is not read).
- **Test steps (survival, by day, on grass):**
  1. Use the horse egg, then right-click the horse with an empty hand: you sit on it while it runs about. After a few seconds it rears, neighs angrily and throws you off beside it, with smoke (no smoke drawn yet) (**80_horse_bucks**). Try again until it shows hearts and stays calm: it is tamed (**81_horse_tamed**). Feeding it wheat, sugar or apples first makes taming quicker.
  2. Right-click the tamed horse with a saddle: the saddle and bridle appear and the saddle sound plays (**82_horse_saddled**). Right-click it again with an empty hand: you sit on it and the reins show.
  3. Steer with W, A, S, D and the mouse: it turns where you look and walks back slowly. Hold space: the jump bar fills over the experience bar; let go: the horse rears and jumps, higher for a fuller bar (**83_horse_jump_bar**). Jump a one-block fence with the bar full. Crouch to get off: you land beside it.
  4. Right-click it with iron horse armour: the armour shows on it (**84_horse_armour**).
  5. Donkey and mule eggs: tame each by riding, saddle them and ride. They cannot wear armour. Llama egg: ride it to tame it; it cannot take a saddle and wanders where it likes while you sit on it.
  6. Save and quit, then load: the horse is still tame, saddled and armoured. Ride it, save and quit while riding, load: you are still on it.
  7. Friend window: the friend tames and saddles their own horse, rides it, sees their own jump bar and jumps; the host sees the friend on the horse with the reins showing (**85_friend_horse**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:horse_spawn_egg*4,minecraft:donkey_spawn_egg*2,minecraft:mule_spawn_egg*2,minecraft:llama_spawn_egg*2,minecraft:saddle*4,minecraft:iron_horse_armor,minecraft:golden_horse_armor,minecraft:diamond_horse_armor,minecraft:wheat*32,minecraft:apple*16,minecraft:golden_carrot*16,minecraft:oak_fence*16,minecraft:cooked_beef*16
```

### 2. The Minecraft horse inventory holds the saddle, armour and chest

After 77d958ce.

- **Vanilla (javap, 26.3):**
  - `HorseInventoryMenu`: slot 0 the saddle (8, 18; empty icon `container/slot/saddle`; active when `canUseSlot(SADDLE)` and the type is in `can_equip_saddle`), slot 1 the body (8, 36; `container/slot/llama_armor` on a llama, else `container/slot/horse_armor`; active for `can_wear_horse_armor` or a llama; an `ArmorSlot`, so only what the mount can wear and one at a time), then the chest: columns x 3 slots at (80 + 18 col, 18 + 18 row) (`getInventoryColumns`: 5 on a chested donkey or mule, the llama's strength), then the player's inventory at y 84.
  - `AbstractMountInventoryMenu.quickMoveStack`: from the mount's slots to the player's (backwards); from the player's: the body slot when it takes the stack and is empty, else the saddle slot likewise, else the chest; when the chest takes nothing (or there is none), between the inventory and the hotbar. `stillValid`: the inventory has not changed size (`hasInventoryChanged`), the mount is alive and within the player's entity interaction range + 4.
  - `AbstractHorse.openCustomInventoryScreen`: only a tame one, and only when nobody or the opener rides it. `mobInteract` opens it on a tame adult with sneak; the inventory key opens it while riding one (`HasCustomInventoryScreen`). `ClientboundMountScreenOpenPacket` carries the container id, the columns and the entity.
  - `HorseInventoryScreen` (`AbstractMountInventoryScreen`): `textures/gui/container/horse.png`, 176 x 166; `container/horse/chest_slots` (90 x 54) at (79, 17), cut to columns x 18 wide; the `container/slot` frame at (7, 17) for an active saddle slot and (7, 35) for an active body slot; the mount itself follows the mouse in the box (26, 18)-(78, 70); title the mount's display name.
  - `AbstractChestedHorse` saves `Items` as `ItemStackWithSlot` with slots 0 to columns x 3 - 1; `equipChest` creates the new inventory.
- **Changed:**
  - Vendored `entities`: `HorseState.inventory` keeps its slots (`Vec<Option<ItemStack>>`, sized by `resize_inventory` when the chest goes on); `horse_opens_inventory` (the rule above) and `set_mount_inventory` (the screen's saddle, body item and chest written back, with the saddle, armour or swag sound when one goes on; a screen of another size is ignored).
  - `minecraft_terrain`:
    - New `mount_inventory.rs`: `MountInventory` (the vehicle, columns, which slots are active, the cells), built from the entity world, with the vanilla slot rules, clicks, shift-clicks, number keys, throws and `still_valid` (alive, same size, eye within 7 blocks of its box).
    - `server_mobs.rs`: the chest's `Items` load into their slots and are saved from the horse's state (they were kept from the loaded tag).
    - `mob_actions.rs`: a crouching click on a tame adult with nobody on it opens its inventory (`MobOutcome.mount_inventory`, `MobResult.mount_inventory`); `server.rs`: `Command::MountInventory` hands the screen's slots to the level thread.
  - `minecraft_net`: `MenuKind::Mount`, `MenuExtras::Mount { mount, columns, saddle, armor, llama }` and `ToServer::OpenMountInventory` (no protocol bump; track D's 4 holds).
  - `minecraft_survival`:
    - `menus.rs`: `Open::Mount(MountInventory)`. Clicks run on the main thread on the screen's copy, like a chest's, and each click that changed it queues the copy for the level thread. `menus::mount` (each frame, host and friends) opens the screen from the level's answer or from the inventory key while riding, closes it when `still_valid` fails, and sends the queued copies. The menu goes to friends as `MenuKind::Mount` with its extras and is mirrored back to `Open::Mount`. The title is `entity.minecraft.<kind>`.
    - `entities.rs`: the mount menus routed per player (`take_mount_menus`) and `set_mount_inventory`.
  - `frame`: `McScreen::Mount { columns, saddle, armor, llama }` and `MinecraftUi.inventory_key_opened` (set by the HUD when the inventory key opens the screen). `hud`: the layout, the `horse` sheet, the chest-slot sprite and slot frames. `render_anim`: `screen_for`, the drag targets, the `horse` sheet, `container/horse/` and `container/slot` in the GUI atlas, the host's inventory key while riding, and a friend's (`ToServer::OpenMountInventory` when its own inventory opens without a menu; the host opens the horse's when the friend rides a tame one).
- **Tests:**
  - `mount_inventory::tests::a_horse_takes_a_saddle_and_armour_a_llama_a_carpet_and_chests_hold_anything`, `clicks_and_shift_clicks_fill_the_saddle_armour_and_chest_slots_like_vanilla`, `the_screen_closes_when_the_mount_dies_moves_away_or_gets_a_chest`.
  - `world::equines::tests::a_tame_horse_s_inventory_opens_on_a_crouching_click_or_from_its_saddle_and_the_screen_sets_its_slots`.
  - `server_mobs::tests::a_mule_a_llama_and_a_trader_llama_save_and_load_with_variant_strength_carpet_and_chest`: the chest's items load into their slots and save from the state (slot 2 diamonds, slot 4 apples).
  - `mob_actions::tests::right_click_mounts_a_horse_and_a_saddle_goes_on_a_tame_one`: a crouching click opens the inventory.
  - `menus::tests::a_horse_inventory_turns_clicks_into_level_updates_and_a_friend_mirrors_it`.
  - `codec::tests::a_horse_inventory_keeps_its_columns_and_slots`; `every_message_to_the_client_round_trips` with a llama's menu, `every_message_to_the_server_round_trips` with `OpenMountInventory`.
  - `minecraft_screens::tests::a_horse_inventory_puts_saddle_armour_and_chest_slots_where_vanilla_does`; `minecraft_gui::tests::the_real_gui_textures_and_fonts_load_into_one_atlas` loads the horse sheet, `container/horse/chest_slots` (90 x 54) and `container/slot` (18 x 18).
  - Runs: `minecraftoss-entities` lib 270, `minecraft_terrain` (the touched modules), `minecraft_survival` lib 608, `minecraft_net` 17, `hud` 162, `render_anim` lib 152, all green. `check -p launcher` green. `mount_inventory.rs` passes `rustfmt --check --edition 2021`.
- **Decisions:**
  - The open screen's copy is what counts while it is open, as a chest's cells are (clicks answer at once); each change is sent to the level thread, which owns the horse. Another player's saddle or chest put on by right-click while the screen is open would be overwritten by the next click on the screen; a chest closes the screen (the size changes).
  - The saddle slot holds the saddle as an item without components (the horse keeps only "saddled"), the body slot keeps the item id; components on a saddle or horse armour are lost through the screen.
  - The mount is not drawn in the screen's box yet (Left for later). The title is the kind's name; a custom name is not read.
  - A friend sends `OpenMountInventory` each time its own inventory opens with no menu; the host ignores it unless that friend rides a tame horse, donkey, mule or llama.
- **Test steps (survival, by day):**
  1. `/summon minecraft:donkey ~2 ~ ~ {Tame:1b}`. Crouch and right-click it: the donkey's screen opens with the saddle slot (saddle icon) and no armour slot (**86_donkey_inventory**). Shift-click a saddle from the hotbar: it goes on the donkey and you hear the saddle. Close the screen: the donkey wears it.
  2. Right-click the donkey with a chest, then crouch and right-click it again: 15 chest slots (5 columns). Shift-click cobblestone and wheat in (**87_donkey_chest**). Save and quit, load: the items are still in the same slots.
  3. Ride the donkey and press E: its inventory opens instead of yours. Walk 8 blocks away from a donkey with its screen open: it closes.
  4. `/summon minecraft:horse ~ ~ ~2 {Tame:1b}`, crouch and right-click: saddle and armour slots, no chest. Put iron horse armour in the armour slot: the horse wears it; take it out again: it is gone from the horse (**88_horse_armour_slot**). A chest does not fit the armour slot.
  5. `/summon minecraft:llama ~ ~ ~-2 {Tame:1b,Strength:3}`, give it a chest, crouch and right-click: 9 chest slots (3 columns), a carpet slot with the llama icon and no saddle slot. Put a white carpet in: the llama wears it (**89_llama_inventory**).
  6. Kill the donkey with the sword: it drops the saddle, the chest's items and the chest.
  7. Friend window: the friend tames or summons their own donkey, gives it a chest and opens it by crouching and right-clicking, and by pressing E while riding it; the friend puts items in, closes and reopens: they are still there; the host opening the same donkey sees the friend's items (**90_friend_donkey**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:horse_spawn_egg*2,minecraft:donkey_spawn_egg*2,minecraft:llama_spawn_egg*2,minecraft:saddle*4,minecraft:iron_horse_armor,minecraft:golden_horse_armor,minecraft:chest*4,minecraft:white_carpet*2,minecraft:red_carpet*2,minecraft:cobblestone*64,minecraft:wheat*32,minecraft:golden_carrot*16,minecraft:iron_sword,minecraft:cooked_beef*16
```

## Group wandering-trader

### 1. Minecraft leads tie mobs to players and fences

After 6650cdce.

- **Vanilla (javap, 26.3):**
  - `Leashable`: `LEASH_TOO_FAR_DIST` and `leashSnapDistance` 12.0 (not 10), `LEASH_ELASTIC_DIST` and `leashElasticDistance` 6.0, `AXIS_SPECIFIC_ELASTICITY` (0.8, 0.2, 0.8), `STIFFNESS` 0.11, `TORSIONAL_ELASTICITY` 10, `ENTITY_ATTACHMENT_POINT` (0, 0.5, 0.5) and `LEASHER_ATTACHMENT_POINT` (0, 0.5, 0) times the boxes' width, height, width. `canBeLeashed` true; `canHaveALeashAttachedTo(holder)`: not itself, box centres at most 12 apart, and `canBeLeashed`.
  - `Leashable.tickLeash` (end of `Entity.baseTick`, server only): a saved holder is restored first (a UUID to a loaded entity, a block position to `getOrCreateKnot`); unresolved after `tickCount > 100` a lead is dropped and the leash forgotten. Then: mob or holder unable to act (`canInteractWithLevel`: dead, removed or spectator) drops the lead (`entity_drops` on) or just lets go; box centres more than 12 apart play `item.lead.break` at the holder's feet and `dropLeash` (the lead item at the mob); more than 6 minus both widths runs `checkElasticInteractions`: the spring from the mob's point (its yaw turning (0, h/2, w/2)) to the holder's centre, force `normalize(holder - mob) * (distance - 6)`, times 0.3 when the mob's own movement (none with `NoAI`) points along it, and the mob's motion grows by `force * (0.8, 0.2, 0.8) + (holder movement - mob movement) * 0.11`; otherwise `PathfinderMob.closeRangeLeashBehaviour`: unless panicking, `moveTo` towards the holder to 2 blocks short at `followLeashSpeed` (1.0, `Llama` 2.0). `whenLeashedTo` sets its home to the holder with radius 5.
  - `Mob.canBeLeashed`: not an `Enemy`. Overrides: `AbstractVillager`, `AmbientCreature`, `WaterAnimal`, `AgeableWaterCreature`, `Panda` and `Turtle` false; `TamableAnimal`, `Hoglin`, `Zoglin`, `Axolotl`, `Dolphin`, `Squid` true; `Wolf` not angry; the zombie horse, camel husk and nautiluses by their own rules. `Mob.requiresCustomPersistence`: a passenger or leashed. `Mob.startRiding` drops the lead; `removeAfterChangingDimensions` lets go.
  - `Entity.interact` (from `Mob.interact` after name tags and spawn eggs, before `mobInteract`): crouching on a leashable, living, grown mob ties the player's own leashed mobs within the 32-block box around it to it (`item.lead.tied`); shears cut every leash of the mob and to it (`item.shears.snip` from the player, 1 durability); a click with anything on a mob the player holds lets it go (the lead drops unless creative, `item.lead.untied`); a lead on a mob not held by a player (a knot's lead drops first) ties it to the player, one lead used.
  - `LeadItem.useOn` and `FenceBlock.useWithoutItem` → `bindPlayerMobs`: the player's leashed mobs in the 32-block box round the fence go to its knot (`getOrCreateKnot`, `item.lead.tied`); none, and a new knot is discarded (pass).
  - `LeashFenceKnotEntity`: `EntityTypes.LEASH_KNOT` 0.375 x 0.5, eye 0.0625, tracking range 10, `noSave` (knots are not saved: the mobs' saved block position recreates them). Its position is (x + 0.5, y + 0.375, z + 0.5); `interact`: the player's mobs to the knot, else (not crouching) the knot's mobs to the player, `item.lead.tied`; `survives` is a block in `#fences`, checked every 100 ticks (`BlockAttachedEntity.tick`), gone with `item.lead.untied`; `getRopeHoldPosition` is 0.2 above it; it goes when its last mob leaves (`notifyLeasheeRemoved`).
  - `Leashable.LeashData.CODEC`: `Codec.xor(UUID field "UUID", BlockPos)`, so `leash:{UUID:[I;...]}` or `leash:[I;x,y,z]`.
  - Drawing: `EntityRenderer.extractRenderState` starts at the mob's position plus `getLeashOffset` turned by its body yaw and ends at the holder's `getRopeHoldPosition` (`Entity`: 0.7 of its eye height; `Player`: the main arm's side, -0.22 for the right, box height - 1 up, and 0.07 forward or -0.2 crouching, turned by its body yaw). `getLeashOffset`: (0, eye height, width x 0.4); `Llama` (0, 0.75 eye, 0.5 width), `Ocelot` 0.5 eye, `IronGolem` 0.875 eye, `Pig`, `Wolf` and `Strider` 0.6 eye. `LeashFeatureRenderer`: 24 steps, width 0.05, two triangle strips (fudge 0.05 up, then 0 back), sideways offset `(dz, dx) * invSqrt(dx² + dz²) * 0.025`, a slack curve (`dy * p²` rising, `dy - dy (1 - p)²` falling), colour (0.5, 0.4, 0.3) times 0.7 on every other step, block and sky light lerped between the two eye blocks. `LeashKnotModel`: one cube (-3, -8, -3) 6 x 8 x 6 on a 32 x 32 `entity/lead_knot/lead_knot`, drawn with `scale(-1, -1, 1)`.
- **Changed:**
  - New vendored `entities/src/world/leashes.rs`: the leash store (`Leash`: holder, saved holder, ticks waited), `LeashHolder` (a player, a mob or a knot's fence), `SavedLeash`, the knots (fence positions with their check clocks), `can_be_leashed`, `leash_interact` (the four `Entity.interact` steps), `leash_fence` (`bindPlayerMobs` and the knot's interact), `set_leash`, `drop_leash`, `load_leash` and `saved_leash`, and `tick_leashes` (restore, letting go, snapping, the spring, the walk to the holder, the knots' checks). Lead drops and the sounds wait in the world for the server.
  - Vendored `entities/src/world.rs`: the store's fields, copied into a tracked copy; the leash pass runs before the mobs tick; leashed mobs count as persistent in the despawn check and the census. `world/riding.rs`: a mob that starts riding drops its lead; removed mobs leave the store; `unlink` is `pub(super)`. `world/pushing.rs`: `player_size` is `pub(super)`.
  - `minecraft_terrain`: `server_mobs.rs` loads `leash` for every mob and saves it from the store (removed when untied); `mob_actions.rs` runs the leash steps before each mob's own use (a lead used, shears worn, the dropped leads as drops); `server.rs` drops the snapped and freed leads as items after the mobs tick and takes `Command::LeashFence`. New `leash_render.rs`: the rope (`rope_strips`, `append_rope`), the knot model, `rope_ends` (a resolved or saved holder: mob, player UUID or knot), `leash_offset`, `player_hold`; `mesh.rs` loads the knot texture.
  - `minecraft_survival`: `actions.rs` sends a click on a fence (`ends_with("_fence")`, the `#fences` members) to the level when it ties or hands back mobs, before the block's own use; `entities.rs` `leash_fence` (tried on a copy first, like mob uses), `has_knot`, `hold_rope`; `entity_view.rs` keeps each player's rope hold (`RopeHolder` by UUID) and draws the knots and ropes after the mobs.
  - `render_anim`: the host passes every player's feet, yaw and height to `hold_rope` with its body (`track_body`), and the friend passes every player's from its snapshot with the player's UUID.
  - No protocol change: the leash rides in the mob's look tag (`leash`), so friends load it with the mob and draw it to the mob, player or knot it names.
- **Tests:**
  - `world::leashes::tests` (vendored): `a_lead_ties_a_cow_to_its_player_and_a_second_click_unties_it`, `monsters_and_villagers_take_no_lead_and_golems_do`, `a_lead_breaks_beyond_twelve_blocks_with_its_sound_and_drops_the_lead`, `a_leashed_cow_is_pulled_after_its_player_and_walks_up_behind_it` (and the spring's damping), `a_fence_ties_the_players_mobs_to_a_knot_and_hands_them_back`, `a_knot_lets_its_mobs_go_at_its_check_once_its_fence_is_gone`, `a_saved_leash_finds_its_player_by_uuid_or_drops_after_a_hundred_ticks`, `shears_cut_a_cow_loose_and_sneaking_ties_the_players_mobs_to_another`, `a_leashed_mob_stays_and_a_lead_to_a_gone_mob_drops`.
  - `leash_render::tests`: `a_rope_is_two_strips_of_twenty_five_pairs_hanging_slack_between_its_ends`, `the_rope_leaves_the_mob_at_its_leash_offset_and_the_players_right_hand`, `a_knot_is_a_six_by_eight_cube_on_top_of_its_fence_post`, `a_friend_draws_a_saved_leash_to_its_player_mob_or_knot`.
  - `server_mobs::tests::a_leashed_cow_saves_and_loads_its_leash_to_a_player_a_mob_or_a_knot`, `mob_tracking::tests::a_friend_sees_a_cow_tied_to_the_host_and_then_to_a_fence_knot`, `mob_actions::tests::a_lead_ties_a_sheep_to_the_player_and_any_click_lets_it_go_with_its_lead`, `server::tests::a_cow_on_a_lead_ties_to_a_fence_saves_its_knot_and_drops_the_lead_when_it_snaps`.
  - Runs: `minecraftoss-entities` lib 279, `minecraft_terrain` lib 1174 of 1177 run (3 ignored; the background section timing test `sections::tests::background_full_update_matches_the_synchronous_one` failed once in the full run and passed alone, as in earlier groups), `minecraft_survival` lib 608, `render_anim` lib 152, all green. `check -p launcher` green. The new `world/leashes.rs` and `leash_render.rs` pass `rustfmt --check --edition 2021`.
- **Decisions:**
  - The lead snaps beyond 12 blocks, as 26.3 does (`leashSnapDistance`); the brief's 10 is older.
  - Knots are not entities in the entity world: a knot is a fence position the store keeps while a mob is tied to it. Vanilla does not save knots either (`noSave`); the mob's saved `leash:[I;x,y,z]` brings it back, so worlds stay vanilla-compatible.
  - A click on a fence that has a knot acts as a click on the knot (the knot's box sits on the post, where the click lands). Punching a knot off is not possible yet (a left click digs the fence); breaking the fence frees the mobs, with their leads, at the knot's next 100-tick check.
  - Not ported: the leash's turning of the mob (`angularMomentum`), the home radius round the holder (`setHomeTo`), the fall distance on an elastic pull, and the `entity_drops` game rule (leads always drop). A player's movement for the spring is the change of position since the last tick.
  - The rope runs to the third-person hold position for the local player in first person too.
  - Friends get no new `MobShow` field: the leash is part of the mob's saved tag, which the tracker resends when it changes (every 10 ticks at most, like other look changes).
  - Llamas now know they are leashed, so caravans form behind a llama on a lead.
- **Test steps (survival, by day, on grass):**
  1. Use the cow egg twice. Hold a lead and right-click a cow: the tie sound plays, a brown rope runs from the cow's head to your right hand, and one lead is used (**91_lead_on_cow**). Walk away: the cow follows and stays about 2 blocks behind; run, and it is pulled along.
  2. Place an oak fence and right-click it while you hold the cow: a knot appears on top of the post and the rope runs to it (**92_cow_at_fence**). Walk 15 blocks away: the cow stays at the fence.
  3. Right-click the fence again with an empty hand: the cow is back on your lead. Tie it to the fence again, save and quit, and load: the cow is still tied, knot and rope drawn (**93_knot_after_reload**).
  4. Take the cow on your lead again and run more than 12 blocks away from it quickly (sprint-jump down a slope, or `/tp ~ ~ ~14`): the lead snaps with its sound and a lead lies by the cow (**94_lead_snapped**).
  5. Right-click a cow you hold, with any item: it is let go and its lead drops. Lead it again and use shears on it: snip, and the lead drops.
  6. A lead on a zombie or a villager does nothing. Lead a llama and walk: a second llama from the egg falls in behind it in a caravan (**95_llama_caravan**).
  7. With a cow on your lead, crouch and right-click the other cow: the first cow is now tied to the second one.
  8. Tie a cow to a fence and break the fence: within 5 seconds the knot is gone and a lead lies by the cow.
  9. Friend window: the friend sees the rope from the host's hand to the cow and the knot on the fence; the friend leads their own cow and the host sees the rope run to the friend's hand (**96_friend_leads**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:lead*16,minecraft:cow_spawn_egg*4,minecraft:llama_spawn_egg*3,minecraft:oak_fence*16,minecraft:shears,minecraft:zombie_spawn_egg,minecraft:villager_spawn_egg,minecraft:wheat*16,minecraft:cooked_beef*16
```

### 2. Minecraft wandering traders visit with their own trades and two trader llamas

After ca00eb5d.

- **Vanilla (javap, 26.3):**
  - `WanderingTrader` (an `AbstractVillager`): `EntityTypes.WANDERING_TRADER` 0.6 x 1.95, eye 1.62, tracking range 10; `Mob.createMobAttributes` with health 20 and movement speed 0.7 (`Villager`'s 0.5 is not used). Goals: 0 `FloatGoal`, 0 `UseItemGoal` (an invisibility potion, `entity.wandering_trader.drink_potion`, when `level.isDarkOutside()` and not invisible), 0 `UseItemGoal` (a milk bucket, `drink_milk`, when `isBrightOutside()` and invisible), 1 `TradeWithPlayerGoal`, 1 `AvoidEntityGoal` for zombies (the `Zombie` class: husks, drowned, zombie villagers, zombified piglins) 8 blocks, evokers 12, vindicators 8, vexes 8, pillagers 15, illusioners 12 and zoglins 10, all at 0.5 / 0.5, 1 `PanicGoal` 0.5, 1 `LookAtTradingPlayerGoal`, 2 `WanderToPositionGoal` (2.0 stop distance, 0.35), 4 `MoveTowardsRestrictionGoal` 0.35, 8 `WaterAvoidingRandomStrollGoal` 0.35, 9 `InteractGoal` (players, 3), 10 `LookAtPlayerGoal` (mobs, 8).
  - `UseItemGoal`: holds the item in the main hand and uses it (32 ticks for both, `LivingEntity.startUsingItem`), then `finishUsingItem` applies the potion (Invisibility 3600 ticks) or the milk (clears every effect), with `disappeared` and `reappeared` after the drink.
  - `TradeWithPlayerGoal`: trading, not in water, on the ground, no `hurtMarked`, the player within 4 blocks; it stops the navigation and its `stop` calls `stopTrading`.
  - `updateTrades`: `trade_set/wandering_trader/buying` (2), then `uncommon` (2), then `common` (5), each through `TradeSet`'s random sequence. `showProgressBar` false, `canRestock` false, `rewardTradeXp` drops 3 to 6 experience (`3 + nextInt(4)`) at its feet plus 0.5 when the offer rewards it, no xp to the trader. `mobInteract`: not with a villager egg, alive, not trading, not a baby; no offers means no screen.
  - `maybeDespawn`: `despawnDelay > 0`, not trading and `--despawnDelay == 0` discards it (no death). It saves `DespawnDelay`, `wander_target` (`BlockPos`) and `home_pos` / `home_radius` (`Mob.addAdditionalSaveData`).
  - `WanderingTraderSpawner` (Overworld only): `tickDelay` 1200; each 1200 ticks `spawnDelay` -= 1200; at 0 or less it is 24000 again, then (with `spawn_mobs` on) `chance = spawnChance`, `spawnChance = clamp(spawnChance + 25, 25, 75)`, and `random.nextInt(100) > chance` stops it. `spawn`: a random player (`level.getRandomPlayer`; none counts as a spawn), `random.nextInt(10) != 0` stops it, the centre is a `MEETING` point of interest within 48 blocks (any occupancy) or the player's feet; `findSpawnPositionNear`: 10 tries at ±48 (`nextInt(96) - 48`) on `MOTION_BLOCKING_NO_LEAVES`, `SpawnPlacements.isSpawnPositionOk` as `ON_GROUND`; `hasEnoughSpace` over the 2 x 3 x 2 box; not in `#without_wandering_trader_spawns`. The trader (reason `EVENT`) gets `despawnDelay` 48000, its wander target and its home (radius 16) at the centre; two trader llamas within 4 blocks, `EVENT` (`TraderLlama.finalizeSpawn` sets age 0), `setLeashedTo(trader, true)`. A spawn sets `spawnChance` back to 25. `WanderingTraderData` is `data/minecraft/wandering_trader.dat` with `spawn_delay` and `spawn_chance`; the game rule `spawn_wandering_traders` turns the spawner off.
  - `TraderLlama`: `canDespawn` is not tame, not leashed to anything but a wandering trader, no single player passenger; `maybeDespawn` takes the trader's delay minus 1 while leashed to one, else its own minus 1, and at 0 or less lets go and is discarded. `TraderLlamaDefendWanderingTraderGoal`: leashed to a trader, it targets the trader's last attacker once per `lastHurtByMobTimestamp`.
  - `WanderingTraderRenderer`: the `VillagerModel` (`LayerDefinitions.VILLAGER`, scale 0.9375), `entity/wandering_trader/wandering_trader`, shadow 0.5, `CrossedArmsItemLayer` (the held bottle or bucket in front of the folded arms). An invisible trader draws no body, but the item layer still draws.
- **Changed:**
  - New vendored `entities/src/wandering_trader.rs`: `WanderingTrader` (body, health, despawn delay, wander target, home, the drink in hand and its clock), `Drink` (invisibility potion or milk), the sounds and the trade sets. New `entities/src/trader_ai.rs`: the goal list above on the monster goal framework (`MonsterKind::WanderingTrader`), with `UseItemGoal`, `TradeWithPlayerGoal`, `PanicGoal`, `LookAtTradingPlayerGoal`, `WanderToPositionGoal` and `MoveTowardsRestrictionGoal`. New `entities/src/world/traders.rs`: the `wandering_traders` store (`TraderEntity`, `EntityKey::Trader`, `MobHit::WanderingTrader`), its tick (AI, drinking, despawn), offers from `TradeBook::offers_from` over the three sets, the trading screen (`wandering_trader_interact`), its trade sounds and experience, and `take_departed_traders`.
  - Vendored `monster_ai.rs`: `MonsterKind::WanderingTrader`, its `TraderState` in the goal context, and `AvoidEntityGoal::also` for the other kinds of the `Zombie` class. `effects.rs`: `MobEffect::Invisibility`; `potion.rs` its colour. `world.rs` and its submodules (`combat`, `hazards`, `pushing`, `riding`, `villagers`, `witches`, `merchants`, `leashes`): the store in every walk over the mobs, as a zombie's villager victim, in the merchant menu (`villager_offers`, `trade_updated`, `notify_trade`, `still_valid`, closing) and as a leash holder. `horse.rs`, `world/equines.rs`, `llama_ai.rs`, `cow_ai.rs`: a trader llama knows its trader and that trader's delay, despawns with it, and `DefendTraderGoal` targets the trader's attacker. `world/src/natural_spawner.rs` `on_ground_spawn_ok`; `world/src/level/spawning.rs` `wandering_trader_spot`, `has_room_for_trader`, `biome_in_tag_at`; `world/src/level_storage.rs` `read_saved_at` and `write_saved_at` are public; `world/src/storage.rs` `world_dir`.
  - `minecraft_terrain`: `custom_spawners.rs` `TraderSpawner` (`due`, `tries`, `spawned`) and `TraderData`; `server.rs` `tick_trader_spawner` after the cat spawner, the trader and its two llamas added as saved entities (`leash:{UUID:...}` on each llama), the rule from `apply_game_rules`, `wandering_trader.dat` read with the entity storage and written with every entity save, departed traders dropped from the saved tags, `MerchantView.trader`; `server_mobs.rs` loads and saves the trader (`DespawnDelay`, `wander_target`, `home_pos`, `home_radius`, `Offers`, the drink in `equipment.mainhand`; its effects through the shared `active_effects`); `mob_tracking.rs` shows friends the drink and the invisibility (`MobShow.drink`, `MobShow.invisible`); `villager_render.rs` `append_wandering_traders`; `client_mobs.rs`, `mob_actions.rs`, `thrown.rs`, `spawner_display.rs`, `mesh.rs` the rest of the wiring.
  - `minecraft_survival`: hit key 30; the trader's boxes and model in `entity_view.rs`; `menus.rs` titles the screen "Wandering Trader" and sends `show_progress: false`. `render_anim` `minecraft_inventory.rs`: no experience bar and no restock for a trader. No protocol change: `MenuExtras::Merchant` already carries `show_progress`, which the friend reads back as the trader flag.
- **Tests:**
  - `wandering_trader::tests` (vendored): `a_trader_counts_its_stay_down_only_while_nobody_trades`, `home_and_wander_target_use_block_distances`, `drinking_takes_32_ticks_with_a_gulp_every_fourth_from_25`.
  - `world::traders::tests` (vendored): `a_wandering_trader_is_a_live_mob_of_its_size_and_health`, `a_trader_drinks_invisibility_at_night_and_milk_by_day`, `a_trader_leaves_once_its_delay_runs_out_but_not_while_trading`, `walking_away_from_a_trader_ends_the_trade`, `a_trader_heads_for_its_wander_target_and_forgets_it_there`, `trader_llamas_on_its_lead_leave_with_the_trader_and_drop_no_lead`, `trader_llamas_turn_on_whoever_hurts_their_trader`.
  - `custom_spawners::tests`: `the_trader_spawner_rolls_every_twenty_minutes_at_25_then_50_then_75_percent` (also the rule, `spawn_mobs` and the 1 in 10), `the_trader_data_reads_and_writes_the_vanilla_fields`.
  - `server::tests`: `a_wandering_trader_sells_two_buying_two_uncommon_and_five_common_trades_with_no_experience_bar` (real trade sets), `a_trade_with_a_wandering_trader_drops_three_to_six_experience_and_teaches_it_nothing`, `the_trader_spawner_brings_a_trader_within_48_blocks_with_two_trader_llamas_on_leads` (real terrain), `the_trader_spawner_waits_for_its_rule_and_its_saved_day` (the rule and `wandering_trader.dat` read and written through the entity storage); `every_spawn_egg_of_a_mob_the_game_has_hatches_that_mob` now hatches 43 eggs.
  - `server_mobs::tests::a_wandering_trader_saves_and_loads_its_stay_wander_target_home_offers_and_potion`, `mob_tracking::tests::a_friend_sees_the_wandering_trader_drink_and_vanish`, `villager_render::tests::a_wandering_trader_is_a_villager_in_its_own_coat_and_vanishes_but_its_bottle_does_not`, `minecraft_survival` `entities::tests::a_wandering_trader_hit_box_encodes_and_decodes` and the trader title and bar in `menus::tests::a_named_villager_s_trading_screen_takes_its_name_as_the_title`. The lead tests of commit 1 (`a_lead_breaks_beyond_twelve_blocks_with_its_sound_and_drops_the_lead`, `a_leashed_cow_saves_and_loads_its_leash_to_a_player_a_mob_or_a_knot`) cover a lead breaking and the leash save.
  - Runs: `minecraftoss-entities` lib 289, `minecraftoss-world` lib 78, `minecraft_terrain` lib 1182 of 1185 run (3 ignored), `minecraft_survival` lib 609, `render_anim` lib 152, all green. `check -p launcher` green. Clippy on `minecraftoss-entities` and `minecraft_terrain` (with `-A clippy::approx_constant`) finds nothing in the new code. The three new vendored files pass `rustfmt --check --edition 2021`.
- **Decisions:**
  - The trader is its own store with hit key 30, not a villager: vanilla's `WanderingTrader` is no `Villager` (no brain, no profession, no gossip). Its merchant runs through the villagers' menu code, which asks the trader store when the id is a trader.
  - The trader's spawn egg brings no llamas, as in vanilla (`EntityType.spawn` from the egg runs no spawner). The brief's step 6 promised llamas with the egg; only the spawner's visit brings them.
  - A llama on a trader's lead keeps the trader's delay as it was before the trader's tick (the leash pass runs before the mobs), so it leaves on the same tick as its trader rather than one tick earlier. Neither drops a lead.
  - Like vanilla, a trader llama tied to a trader whose delay is 0 (a trader from the egg or `/summon`) leaves at once; players cannot tie anything to a trader (`canBeLeashed` is false), so only a hand-edited save can do that.
  - Invisibility is a real `MobEffect` on the trader (saved in `active_effects`, cleared by the milk). Friends learn the drink and the invisibility from `MobShow` and run no trader AI.
  - The illager avoid goals are left out until illagers exist (group `pillagers`); the zombie and zoglin ones are in.
  - `LookAtPlayerGoal` looks at players only, as for the other ported mobs.
  - `wandering_trader.dat` is read and written by the Overworld's level thread next to its entity storage (`ChunkStorage::world_dir`), in vanilla's saved-data format, so a vanilla world keeps its spawner clock.
- **Test steps (survival, flat ground near a village if you have one):**
  1. (D3 step 5, from commit 1.) Lead a cow to an oak fence and tie it: knot and rope.
  2. Use the wandering trader egg by day: a trader in its blue robe stands there, and no llamas come with it (vanilla's egg brings none). Right-click it: the "Wandering Trader" screen with 9 offers, the first two buying items for emeralds, and no experience bar above the arrow (**97_trader_screen**). Buy something with emeralds: the item comes out and a small experience orb drops at its feet.
  3. `/time set night`: within a few seconds it raises a potion to its mouth, gulps, and vanishes with a sound; only the floating bottle shows while it drinks (**98_trader_drinks_potion**). `/time set day`: it drinks milk from a bucket and reappears (**99_trader_milk**).
  4. Hit it with a sword: it runs away from you. A zombie egg near it: it keeps away from the zombie.
  5. Natural visit with llamas: stand near a village bell (or anywhere in the Overworld) and run `/tick sprint 30d`. A trader arrives within 48 blocks with two trader llamas in blue and gold blankets, each on a lead to the trader (**100_trader_with_llamas**). Hit the trader: the llamas spit at you. It is random (about 90 % for 30 days), so sprint again if none came.
  6. Save and quit and load: the trader is still there, invisible if it was, and its offers are unchanged.
  7. (D3 step 8.) Friend window: the friend sees the trader, its potion and milk, its vanishing at night and its llamas on their leads, and can trade with it while the host is not (one customer at a time) (**101_friend_sees_trader**).
- **Kit (the villages kit plus a bell):**

```
IW4L_MINECRAFT_KIT=minecraft:cat_spawn_egg*4,minecraft:creeper_spawn_egg*2,minecraft:horse_spawn_egg*2,minecraft:donkey_spawn_egg*2,minecraft:mule_spawn_egg*2,minecraft:llama_spawn_egg*2,minecraft:wandering_trader_spawn_egg*2,minecraft:cod*32,minecraft:golden_carrot*16,minecraft:saddle*2,minecraft:iron_horse_armor,minecraft:chest*2,minecraft:white_carpet*2,minecraft:lead*4,minecraft:oak_fence*8,minecraft:emerald*64,minecraft:iron_block*8,minecraft:snow_block*4,minecraft:carved_pumpkin*4,minecraft:zombie_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:shield,minecraft:iron_sword,minecraft:bell
```

## Group built-golems

### 1. Minecraft iron golems can be built from iron blocks and a carved pumpkin

After 19d84a55.

- **Vanilla (javap, 26.3):**
  - `CarvedPumpkinBlock.onPlace`: unless the old state is already the same block, `trySpawnGolem`. It tries `snowGolemFull`, then `ironGolemFull`, then `copperGolemFull` with `BlockPattern.find(level, pos)` and builds the first that matches. Every `setBlock` of a carved pumpkin or jack o'lantern runs it (a player's placement, a piston), not only the player's.
  - `ironGolemFull`: one aisle `"~^~", "###", "~#~"`: `^` is `PUMPKINS_PREDICATE` (`carved_pumpkin` or `jack_o_lantern`, any facing), `#` is `iron_block`, `~` is `BlockStateBase::isAir`. `snowGolemFull`: `"^", "#", "#"` with `snow_block`; `copperGolemFull`: `"^", "#"` with `#copper`. The base patterns (a space in place of `^`, for `canSpawnGolem`) are the same without the head.
  - `BlockPattern.find`: every corner in the cube from `pos` to `pos + (max(width, height, depth) - 1)` (`BlockPos.betweenClosed`, x fastest, then y, then z), every `forwards` in `Direction.values()` (down, up, north, south, west, east) and every `up` off its axis: 24 orientations, so a golem can be built facing any way and lying down. `translateAndRotate(origin, forwards, up, right, down, depth)` is `origin + up * -down + (forwards x up) * right + forwards * depth`. Cells are read through the level cache without loading chunks: an unloaded cell is `null`, which fails every `hasState` cell (a space is `alwaysTrue` and passes).
  - `spawnGolemInWorld`: `clearPatternBlocks` (every cell of the match set to air with flags 2, then level event 2001 with the cell's old state: break particles and sound, nothing for air), the golem `snapTo(x + 0.5, y + 0.05, z + 0.5, 0, 0)` at `getBlock(1, 2, 0)` for the iron golem (`(0, 2, 0)` for the snow golem), `addFreshEntity`, the `summoned_entity` trigger for players within 5 blocks (advancements), then `updatePatternBlocks` (`updateNeighborsAt(cell, AIR)` for every cell).
  - The iron golem is `EntityType.create(level, TRIGGERED)` (no `finalizeSpawn`: no follow-range bonus, no left-handed roll) with `setPlayerCreated(true)`; `IronGolem.canAttackType` keeps a player-made golem off players.
- **Changed:**
  - New vendored `world/src/level/golem_build.rs`: `GolemPattern` (`IRON_GOLEM_FULL`, the try order `GOLEMS`), `find` (vanilla's search order and orientations), `translate_and_rotate`, `GolemMatch` (`block`, `cells`), `spawn_position`, `golem_tag` (`PlayerCreated:1b` for the iron golem), and `Level::carved_pumpkin_on_place`: the cells cleared (flags 2), each non-air cell put in `Level.destroyed` (the client's break particles and sound, as level event 2001), the golem's tag from the entity defaults, placed with a fresh UUID and yaw 0, put in `Level.spawned`, then the neighbour updates.
  - Vendored `world/src/level/mod.rs`: the module, and `on_place` calls it for a carved pumpkin or jack o'lantern that replaced another block.
  - `minecraft_terrain`: no game code changed; the server takes the golem's tag with the other spawned mobs at the next tick (`take_spawned`), and `spawn_saved` already reads `PlayerCreated`. Friends see it as any iron golem.
- **Tests:**
  - `level::golem_build::tests` (vendored world): `an_iron_t_with_a_carved_pumpkin_on_top_is_found_facing_either_way` (carved pumpkin and jack o'lantern, the nine cells, the spawn spot), `the_four_rotations_of_the_iron_t_all_build_a_golem`, `the_arms_need_air_under_them_and_every_iron_block` (and a plain pumpkin does nothing), `the_corners_beside_the_head_must_be_air`, `a_golem_built_lying_down_is_found_too`, `an_unloaded_block_matches_only_the_open_cells`, `a_built_iron_golem_is_player_made`, `rotating_the_pattern_follows_vec3i_cross`.
  - `server::tests::a_carved_pumpkin_on_an_iron_t_builds_a_player_made_iron_golem` (real level: the five blocks cleared and broken, the golem on the bottom block, player-made, yaw 0, joining at `take_spawned`), `server::tests::a_carved_pumpkin_on_an_incomplete_t_stays_a_pumpkin`.
  - Runs: `minecraftoss-world` lib 86, `minecraft_terrain` lib 1185 of 1188 run (3 ignored), all green. `check -p launcher` green. The new `golem_build.rs` passes `rustfmt --check --edition 2021`.
- **Decisions:**
  - The check runs in the level's `on_place`, as vanilla's does, so a player, a friend and a piston all build golems. The golem joins the entity world as a spawned tag at the next tick (with the other spawned mobs), not in the same tick as vanilla's `addFreshEntity`.
  - Copper golems (`copperGolemFull` and the copper chest it leaves) wait for mob 47 (group `copper-golems`); the snow golem pattern comes with the snow golem in the next commit. The dispenser's carved pumpkin behaviour (`canSpawnGolem`, placing the pumpkin to finish a golem) is not ported: the dispenser lists it among the behaviours it does not have yet.
  - No `summoned_entity` advancement trigger: the fork has no advancements.
- **Test steps (survival, flat open ground):**
  1. Place one iron block, three iron blocks across on top of it (a T), and a carved pumpkin on the middle top block. The five blocks break with their particles and sounds, and an iron golem stands where the bottom block was (**102_iron_golem_built**).
  2. Hit it with the sword: it does not fight back (built by a player). Use a zombie egg near it: the golem goes for the zombie.
  3. Build a second T across the other axis, with a jack o'lantern: a golem. Build a T with a gold block as one arm: the pumpkin stays and no golem comes (**103_wrong_t_stays**).
  4. Build a T lying on the ground (the bottom block, the three across and the pumpkin all on one layer, pumpkin last): a golem stands up from it (**104_golem_lying_down**).
  5. Save and quit, then load: the built golems are still there and still do not attack you.
  6. Friend window: the friend builds a golem the same way; the host sees the blocks break and the golem appear (**105_friend_builds_golem**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:iron_block*32,minecraft:carved_pumpkin*4,minecraft:jack_o_lantern*2,minecraft:gold_block*2,minecraft:zombie_spawn_egg*2,minecraft:iron_sword,minecraft:cooked_beef*16
```

### 2. Minecraft snow golems are built, shoot snowballs and leave snow

After e9cf2504.

- **Vanilla (javap, 26.3):**
  - `EntityTypes.SNOW_GOLEM`: `MobCategory.MISC`, `immuneTo(#snow_golem_immune_to)`, `sized(0.7, 1.9)`, `eyeHeight(1.7)`, `clientTrackingRange(8)`. `SnowGolem.createAttributes`: `Mob.createMobAttributes` (follow range 16), max health 4, movement speed 0.2. `AbstractGolem`: never despawns far away, ambient interval 120.
  - `SnowGolem.registerGoals`: 1 `RangedAttackGoal(1.25, 20, 10.0)` (one throw every 20 ticks within 10 blocks, walking closer until it has seen the target for 5 ticks), 2 `WaterAvoidingRandomStrollGoal(1.0, 1.0000001E-5)`, 3 `LookAtPlayerGoal(Player, 6)`, 4 `RandomLookAroundGoal`; targets: 1 `NearestAttackableTargetGoal(Mob, 10, true, false, e -> e instanceof Enemy)` (creepers too, unlike the iron golem's).
  - `performRangedAttack`: a `Snowball(level, golem, snowball)` from (x, eyeY - 0.1, z), `shoot(dx, target.eyeY - 1.1 + 0.2 * horizontal distance - snowball.y, dz, 1.6, rangedAttackUncertainty 12)`, then `entity.snow_golem.shoot` at volume 1 and pitch `0.4 / (random.nextFloat() * 0.4 + 0.8)` from the golem's random. `Snowball.onHitEntity`: 3 damage to a blaze, 0 to anything else (`thrown` damage, owner credited: a mob turns on the golem; a player ignores a 0 hit).
  - `isSensitiveToWater` true: `LivingEntity.aiStep` hurts it 1 (`drown`) in water or rain. `SnowGolem.aiStep` after it: the `SNOW_GOLEM_MELTS` environment attribute at its position hurts it 1 (`onFire`), then, with `mob_griefing`, for i in 0..4 the block at `floor(x + (i % 2 * 2 - 1) * 0.25), floor(y), floor(z + (i / 2 % 2 * 2 - 1) * 0.25)` gets `snow` when it is air and `snow.canSurvive` there (and a `BLOCK_PLACE` game event). There is no temperature check for the trail in 26.3.
  - `mobInteract` with shears while `readyForShearing` (it wears its pumpkin): `shear` plays `entity.snow_golem.shear` (players source), clears the pumpkin flag (`DATA_PUMPKIN_ID` bit 16), drops `shearing/snow_golem` (a carved pumpkin) at its eye height (`spawnAtLocation`), the `SHEAR` game event and one point of shear wear; success. It saves `Pumpkin` (default true). Leash offset (0, 0.75 eye height, 0.4 width).
  - `CarvedPumpkinBlock`: `snowGolemFull` (`"^", "#", "#"`) is tried before the iron golem; the golem stands on `getBlock(0, 2, 0)`, created with `TRIGGERED` (no finalize, yaw 0).
  - `SnowGolemModel` (64 x 64, every cube `CubeDeformation(-0.5)`): `head` (0, 0) box (-4, -8, -4) 8 x 8 x 8 at (0, 4, 0); `left_arm` (32, 0) box (-1, 0, -1) 12 x 2 x 2 at (5, 6, 1) turned (0, 0, 1.0); `right_arm` the same box at (-5, 6, -1) turned (0, pi, -1.0); `upper_body` (0, 16) box (-5, -10, -5) 10 x 10 x 10 at (0, 13, 0); `lower_body` (0, 36) box (-6, -12, -6) 12 x 12 x 12 at (0, 24, 0). `setupAnim`: head yaw and pitch; `upperBody.yRot` a quarter of the head yaw; the arms take that yaw (the right one plus pi) and sit at (cos * 5, 6, -sin * 5) and (-cos * 5, 6, sin * 5).
  - `SnowGolemRenderer`: shadow 0.5, `entity/snow_golem/snow_golem`. `SnowGolemHeadLayer` with the pumpkin: the head's pose, then `translate(0, -0.34375, 0)`, `rotateY(180)`, `scale(0.625, -0.625, -0.625)`, `translate(-0.5, -0.5, -0.5)` and the default `carved_pumpkin` block model (skipped while invisible and not glowing).
- **Changed:**
  - New vendored `entities/src/snow_golem.rs` (sizes, attributes, the goal numbers, sounds, `SnowGolem` with its `pumpkin`, the snowball start and aim, the shoot pitch, the four trail cells), `entities/src/snow_golem_ai.rs` (the goal and target lists on the monster goal framework, `SnowGolemState.shot`) and `entities/src/world/snow_golems.rs` (the `snow_golems` store: `SnowGolemEntity`, `spawn_snow_golem`, its tick with fire, walls, air, effects, the ambient roll, AI, water and rain, melting, the snow trail, the shot as a `SnowballShot`; `shear_snow_golem`; `snowball_hits_mob`; `take_snowball_shots`).
  - Vendored `monster_ai.rs`: `MonsterKind::SnowGolem` (walk target zero, no float, follow range 16), `Prey::Monster` (every `Enemy`), `RangedAttackGoal` usable from other modules, and its attack for a snow golem records the target and the pitch drawn from the golem's random. `world.rs` and `world/{combat,hazards,leashes,pushing,riding,villagers,witches}.rs`: the store in every walk over the mobs, `EntityKey::SnowGolem` and `MobHit::SnowGolem`, arrows, mob hits, splash potions and blasts on it, its deaths and loot (`entities/snow_golem`, no experience), its sounds, the census and the despawn check (it never despawns), leads. `loot.rs`: `ShearingLootBook::roll_items` for a plain shearing table. `player/src/lib.rs`: the `World` trait gains `snow_golem_melts` and `block_survives` (defaults false and true). `world/src/level/time.rs`: `snow_golem_melts_at` (the attribute in the biome at the block). `world/src/level/golem_build.rs`: `SNOW_GOLEM_FULL`, tried first.
  - `minecraft_terrain`: `server_mobs.rs` loads and saves the golem (`Pumpkin`) and answers `snow_golem_melts` and `block_survives` from the level (`can_survive_at`, so snow lies where the level's own snow-layer rule allows); `thrown.rs` gains `Thrown::shot_by_mob` (a snowball with its shooter, owned by no player), snow golems as targets and as victims of player throws; `server.rs` turns each shot into such a snowball after the mobs tick, keeps the shooter's own box out of its first ticks and lets its hit run `snowball_hits_mob`; `mob_actions.rs` shears the pumpkin off; `mob_tracking.rs` tracks it at 8 chunks and shows friends the pumpkin (`MobShow.pumpkin`); new `snow_golem_render.rs` draws the model and returns the pumpkin as a posed block; `client_mobs.rs` (pose, shadow 0.5), `mesh.rs` (texture), `spawner_display.rs` (cages), `leash_render.rs` (leash offset).
  - `minecraft_survival`: hit key 31 (`encode`, `decode`, the boxes); `entity_view.rs` draws the golems and their pumpkins with the other posed blocks. Friends see the snowballs with the other thrown items (`EntitySnapshot.thrown_views`), so no new message.
- **Tests:**
  - Vendored entities: `snow_golem::tests` (`a_snow_golem_is_small_and_frail_and_wears_its_pumpkin`, `the_trail_covers_the_four_blocks_under_its_corners`, `a_snowball_leaves_under_its_eyes_and_aims_above_the_target_with_distance`), `world::snow_golems::tests` (`a_snow_golem_is_a_live_mob_of_its_size_and_health`, `a_snow_golem_leaves_snow_where_it_can_lie_unless_mobs_may_not_grief`, `a_snow_golem_melts_where_the_biome_is_too_warm_and_rain_hurts_it`, `a_snow_golem_throws_snowballs_at_a_monster_in_range`, `a_snow_golem_leaves_players_and_animals_alone`, `shears_take_its_pumpkin_once`, `its_snowball_turns_a_zombie_on_it_and_burns_a_blaze`).
  - Vendored world: `level::golem_build::tests::two_snow_blocks_under_a_pumpkin_build_a_snow_golem_facing_any_way` (upright with a carved pumpkin or jack o'lantern, and lying along all four sides), `a_snow_golem_needs_both_snow_blocks_and_is_not_player_made`, `the_snow_golem_is_tried_before_the_iron_golem`.
  - `minecraft_terrain`: `server::tests::two_snow_blocks_and_a_carved_pumpkin_build_a_snow_golem_that_leaves_snow` (real level), `server::tests::a_snow_golem_s_snowball_flies_with_the_thrown_items_and_turns_a_zombie_on_it` (a husk: no harm, it turns on the golem), `server_mobs::tests::a_snow_golem_saves_and_loads_with_its_pumpkin_and_health`, `mob_tracking::tests::a_friend_sees_a_snow_golem_from_its_look_tag_and_loses_its_pumpkin_to_shears`, `mob_actions::tests::shears_take_a_snow_golem_s_pumpkin_once_and_drop_it_at_its_eyes` (the real shearing table when the jar is there), `snow_golem_render::tests` (`a_snow_golem_is_five_shrunk_cubes_standing_on_its_feet`, `the_arms_follow_a_quarter_of_the_head_turn`, `the_pumpkin_sits_on_its_head_at_ten_sixteenths_of_a_block`), the cage test with the snow golem, and `every_spawn_egg_of_a_mob_the_game_has_hatches_that_mob` now hatches 44 eggs.
  - `minecraft_survival`: `entities::tests::a_snow_golem_hit_box_encodes_and_decodes`.
  - Runs: `minecraftoss-entities` lib 299, `minecraftoss-world` lib 89, `minecraft_terrain` lib 1193 of 1196 run (3 ignored), `minecraft_survival` lib 610, all green; `minecraftoss-player` lib tests need the full suite's stand-in files (only two default `World` methods changed there). `check -p launcher` green. Clippy on `minecraftoss-entities`, `minecraftoss-world` and `minecraft_terrain` (with `-A clippy::approx_constant`) finds nothing in the new code. The new `snow_golem.rs`, `snow_golem_ai.rs`, `world/snow_golems.rs` and `snow_golem_render.rs` pass `rustfmt --check --edition 2021`.
- **Decisions:**
  - The snow golem has its own store (`snow_golems`, hit key 31) instead of joining the iron golem's: that store is built round the village guard (villagers about it, the poppy, crackiness, melee, anger) and the two share only `AbstractGolem`'s despawn rule and ambient interval. The copper golem (mob 47) will decide its own home.
  - Its snowballs are the server's thrown snowballs with the golem as shooter (owner `MOB_THROWER`), so the host and friends draw them and their hit crumbs like a player's. A hit on a mob runs as the golem's hit (`mob_hits_mob`: 0 damage, 3 to a blaze, knockback along the flight, the victim turns on the golem); a hit on a player does nothing, as vanilla's `Player.hurtServer` ignores a 0 hit.
  - The snow trail follows 26.3: every biome where it lies (`SnowLayerBlock` survival through the level), and none where the golem melts away first; the brief's "only in cold biomes" is older behaviour. Melting reads the `gameplay/snow_golem_melts` attribute of the biome at its feet.
  - The `#snow_golem_immune_to` tag (powder snow) is not used: mobs do not freeze here yet.
  - The pumpkin is not drawn in spawner cages (the cage draws mob models only; the iron golem's poppy is left out the same way).
  - The `BLOCK_PLACE` and `SHEAR` game events wait for vibrations (group `vibrations`).
- **Test steps (survival, plains or another temperate biome, by day, clear weather):**
  1. Stack two snow blocks and put a carved pumpkin on top. The three blocks break and a snow golem with its pumpkin head stands there (**106_snow_golem_built**). Build a second one lying on the ground (pumpkin last) with a jack o'lantern: it stands up too.
  2. Watch it walk: it leaves thin snow layers behind on grass and stone (**107_snow_trail**).
  3. Use a zombie egg about 6 blocks away: the golem turns, throws snowballs with a puffing sound (some miss), the zombie is knocked back without losing health and comes after the golem (**108_snowballs_at_zombie**). Kill the zombie with the sword.
  4. Use a blaze egg near it: the snowballs hurt the blaze (red flash).
  5. Right-click the golem with shears: snip, a carved pumpkin pops out at its head height and its face shows (**109_sheared_snow_golem**). Shears again do nothing. A lead ties it like an animal.
  6. `/weather rain`: the golems flash red in the rain. `/weather clear`. Use the snow golem egg in a desert (`/locate biome minecraft:desert`, `/tp` there): it melts away within a few seconds.
  7. `/gamerule mob_griefing false`: no more snow trail; `/gamerule mob_griefing true` again.
  8. Save and quit, then load: the sheared golem is still bare, the other still wears its pumpkin.
  9. Friend window: the friend sees the golems, their pumpkins, the flying snowballs and the snow; shears one (the host sees its face) and shoots one with a gun (**110_friend_snow_golem**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:snow_block*16,minecraft:carved_pumpkin*8,minecraft:jack_o_lantern*2,minecraft:shears,minecraft:snow_golem_spawn_egg*4,minecraft:zombie_spawn_egg*4,minecraft:blaze_spawn_egg*2,minecraft:lead*2,minecraft:iron_sword,minecraft:cooked_beef*16
```

## Group loot-functions

### 1. Minecraft loot sets damage, potions, stew effects, enchantments, components and ominous bottle levels

After bd2521c7.

- **Vanilla (javap, 26.3, `net.minecraft.world.level.storage.loot`):**
  - `SetItemDamageFunction.run`: only for `isDamageableItem` (a max damage, not unbreakable). With `add`, the kept part is `1 - damage / max`, else 0. It sets `floor((1 - clamp(damage + kept, 0, 1)) * max)`, and `setDamageValue` clamps that to `0..max`. A damage of 0 is the default, so it leaves no component. An iron sword (max 250) at 0.5 gets damage 125.
  - `SetPotionFunction`: updates `potion_contents` with the potion and keeps any colour and custom effects.
  - `SetStewEffectFunction`: only on `suspicious_stew` with a non-empty list. `Util.getRandom` picks an entry (`nextInt(size)`), then the duration provider is rolled. The duration is times 20 unless the effect is instantaneous (instant health, instant damage, saturation). `SuspiciousStewEffects.withEffectAdded` appends `{id, duration}`, and the codec leaves out a duration of 160.
  - `SetOminousBottleAmplifierFunction`: the int provider is clamped to 0..4 and set as `ominous_bottle_amplifier`. 0 is the bottle's default.
  - `SetEnchantmentsFunction`: a book becomes an enchanted book with `transmuteCopy`, which keeps the count and components. `EnchantmentHelper.updateEnchantments` writes `stored_enchantments` on an enchanted book and `enchantments` on anything else. Each listed enchantment is set to the provider's value, or with `add` raised by it, clamped to 0..255. Level 0 removes it.
  - `SetComponentsFunction`: `applyComponentsAndValidate(patch)`. A `!key` entry removes that component.
  - The chest tables also need these:
    - `SetNameFunction`: the name goes to `custom_name`, or to `item_name` with `target: item_name`.
    - `SetInstrumentFunction`: `HolderSet.getRandomElement` takes `nextInt(size)` of the tag's members, here `#minecraft:regular_goat_horns` for the pillager outpost.
    - `FilteredFunction`: `item_filter` decides between `on_pass` and `on_fail`.
    - `DiscardItem`: gives `ItemStack.EMPTY`.
    - `LocationCheck`: false without `ORIGIN`, otherwise `LocationPredicate.matches` at the origin plus the offset, here only `biomes` (an id, a list or a tag).
  - The map filter `{"minecraft:map_id": {}}` is the component's `AnyValue` predicate: it holds when the stack carries `map_id`.
- **Which tables use what (26.3 jar, outside `blocks/`):**
  - `set_damage`: 24 times in chests (ancient city, bastions, trial chambers) and 4 in fishing.
  - `set_potion`: 12 times in chests, 7 in dispensers, 13 in spawners and 3 in entity tables.
  - `set_stew_effect`: the shipwreck supply chest, the ancient city ice box and the desert well.
  - `set_ominous_bottle_amplifier`: the trial chamber reward chests and `entities/pillager` (raid captains).
  - `set_enchantments` and `set_components`: the trial chamber equipment tables and the wind burst book in the ominous rare reward.
  - `set_name`, `filtered` and `exploration_map`: the abandoned camp, shipwreck map and ocean ruin chests.
  - `set_instrument`: the pillager outpost.
  - `location_check`: the abandoned camp chests, the mineshaft's `music_disc_bounce` in `sulfur_caves`, and fishing junk.
- **Changed:**
  - `third_party/minecraftoss/core/src/item.rs`:
    - `ItemCatalog` reads `max_damage` from the 26.3 item catalog.
    - New `ItemCatalog::max_damage`.
  - `third_party/minecraftoss/core/src/loot.rs`:
    - `apply_function` ports `set_damage`, `set_potion`, `set_stew_effect`, `set_ominous_bottle_amplifier`, `set_enchantments`, `set_components`, `set_name`, `set_instrument`, `filtered` and `discard`, next to `enchant_randomly`. Instrument tags are read from the data pack's `tags/instrument`.
    - `condition` gains `location_check` from the new `LootParams.biome`.
    - `item_predicate` gains the `map_id` predicate.
    - `exploration_map` is the one function in these tables that still fails. It needs a structure search and a filled map.
  - `third_party/minecraftoss/entities/src/loot.rs`, the entity loot engine:
    - `set_ominous_bottle_amplifier`.
    - The `type_specific/raider` `is_captain` predicate, from a new `EntityLootContext.captain`. No pillager sets it yet; group `pillagers` will.
    - With these, `entities/pillager` (a captain's ominous bottle) rolls. The other entity tables need nothing new.
  - `crates/minecraft_terrain/src/containers.rs`: new `structure_loot_in` with the chest's biome. `structure_loot` calls it without one.
  - `crates/minecraft_terrain/src/terrain.rs`: new `BlockStates::biome_name`, the zoomed biome at a block, as `Level.getBiome` reads it.
  - `crates/minecraft_survival/src/menus.rs`: a structure chest opened for the first time rolls with the biome it stands in.
- **Tests:**
  - `minecraftoss-core` `loot::tests`:
    - `set_damage_wears_an_iron_sword_by_its_max_damage`: 0.5 gives 125, `add` 0.25 on top gives 62, and 1.0 removes the component. A uniform range stays within the max, and a stick is left alone.
    - `set_ominous_bottle_amplifier_rolls_zero_to_four`: uniform 0-4 gives every level and no component at 0. A constant 9 gives 4.
    - `set_enchantments_sets_or_adds_levels_and_stores_them_on_books`: set, add, partial reset, clear to no component, and a book of 2 becoming 2 enchanted books with `stored_enchantments`.
    - `set_components_sets_and_removes_components`: a copper trim, and `!minecraft:damage`.
    - `potions_stews_names_horns_and_filters_write_vanilla_components`: covers set_potion, both stew durations, the default 160, a plain bowl left alone, `item_name` and `custom_name`, all four regular goat horns, and the map filter discarding a map without `map_id`.
    - `location_checks_read_the_biome_at_the_origin`: an id, a list, a tag, no origin, and an unsupported `structures` key.
    - `every_chest_spawner_equipment_and_gameplay_table_rolls_without_an_unsupported_function`, gated on `MINECRAFTOSS_ROOT`:
      - It covers every table under `chests`, `spawners`, `equipment` and `gameplay` (over 90).
      - Every function found in a table is applied to four sample items, and only `exploration_map` may report itself unsupported.
      - Each table rolls 8 times. Chests, spawners and equipment must succeed unless the table has an exploration map. Gameplay rolls may fail only on conditions the core engine leaves to the entity engine, such as fishing's entity predicates.
    - `unknown_functions_still_fail` now uses `exploration_map`.
  - `minecraftoss-entities` `loot::tests::a_pillager_captain_drops_an_ominous_bottle_of_level_one_to_five`: a plain pillager drops nothing, and a captain drops one bottle, every amplifier 0-4 over 200 rolls.
  - `minecraft_terrain` `containers::tests::ancient_city_pyramid_and_trial_chamber_chests_roll_with_their_components`, over 100 seeds:
    - Ancient city chests are never empty, hold worn gear and a strong regeneration potion.
    - Desert pyramid chests are never empty.
    - Ominous trial reward bottles have amplifier 2-4.
    - The mineshaft's bounce disc comes only in `sulfur_caves`.
- **Decisions:**
  - Beyond the brief's six functions, `set_name`, `set_instrument`, `filtered`, `discard` and `location_check` are ported too. The survey test asks every chest table to roll except through `exploration_map`, and without them the pillager outpost (`set_instrument`) and the abandoned mineshaft (`location_check`) still failed.
  - `exploration_map` stays unported, as the brief says. Its five tables still roll nothing: the abandoned camp chests, `shipwreck_map` and both ocean ruin chests.
  - No biome (or no origin) makes a `location_check` false. A loot table rolled without a world position therefore never gets a biome-only item.
  - A component set to its default value (damage 0, amplifier 0, an empty enchantment map) is removed, as vanilla's component patch does, so the stack stacks with a plain one.
  - `chests/desert_pyramid` uses only `set_count` and `enchant_randomly` in 26.3, so it already rolled. The state doc's line 435 is out of date there, and the test keeps it as a check.
  - The mineshaft table belongs to chest minecarts, which the fork does not have. Its `location_check` is exercised only by the test.
- **Test steps (survival; use `/locate` and `/tp`, take night vision for the deep dark):**
  1. `/locate structure minecraft:ancient_city`, `/tp` there and find three chests. Each holds loot: worn diamond hoes or leggings (their durability bars part used), enchanted books, a strong regeneration potion, sculk, candles and so on (**111_ancient_city_chests**).
  2. `/locate structure minecraft:desert_pyramid`, `/tp` there and dig to the hidden room. Its four chests still hold loot (**112_desert_pyramid_chests**).
  3. `/locate structure minecraft:shipwreck` and open the supply chest. It may hold a suspicious stew; eating it gives night vision, jump boost, weakness, blindness, poison or saturation (**113_shipwreck_supply**).
  4. `/locate structure minecraft:trial_chambers` and open a corridor chest or barrel. The tools in it are worn (**114_trial_chamber_chest**).
  5. In an ancient city, the friend opens a chest the host has not opened. It holds loot for the friend too (**115_friend_ancient_city_chest**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword,minecraft:torch*64,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:ladder*64,minecraft:water_bucket,minecraft:potion@night_vision,minecraft:potion@night_vision
```

### 2. Minecraft ancient cities generate with sculk shriekers and loot chests

After 7f709c62.

- **What it checks:** this commit is a test only.
  - `/locate` finds the nearest `minecraft:ancient_city` start from (0, -51, 0) on seed 77, and the test generates every chunk of its bounding box.
  - The start piece (the city centre) stands in `minecraft:deep_dark`, read with the zoomed biome from commit 1.
  - The chunks hold `reinforced_deepslate` (the portal frame), `sculk_shrieker` (at least one with `can_summon=true`), `sculk_catalyst`, `sculk_sensor`, `deepslate_bricks` and chests, and no `structure_block` data markers.
  - At least 10 chests inside the city's pieces carry a loot table, every one `minecraft:chests/ancient_city` or `minecraft:chests/ancient_city_ice_box`, and each rolls non-empty loot through the core engine.
  - The city near (-2500, -50, -1050) on seed 77 also has a chest whose template holds a fixed golden apple and no loot table. Simple dungeon chests inside its bounding box are left out, because they are not in a city piece.
- **Changed:** `crates/minecraft_terrain/src/terrain.rs`, the test only.
- **Tests:** `terrain::tests::ancient_cities_generate_in_the_deep_dark_with_sculk_shriekers_and_loot_chests` (gated on `MINECRAFTOSS_ROOT` and the block entity catalog; about 8 s).
- **Decisions:**
  - Commit 1's functions change what new chests roll. Older chests behave as follows:
    - A chest that rolled nothing before (a function was missing) was never filled and keeps its `LootTable`, so it fills the first time it is opened in this build. The exception is a chest the player put an item into: it was saved with `Items` and no loot table.
    - A chest already filled stays as it was saved.
  - Shipwreck supply and treasure chests, buried treasure, ancient cities, bastions, trial chambers and pillager outposts now roll.
  - Desert pyramids already rolled; their table needs only `set_count` and `enchant_randomly`.
  - Shipwreck map chests and ocean ruins still roll nothing until explorer maps land.
- **Test steps (survival, night vision on):**
  1. `/locate structure minecraft:ancient_city`, `/tp` there, and open chests in different rooms. They hold loot (commit 1's step 1, **111_ancient_city_chests**).
  2. The city's centre has the reinforced deepslate portal frame, and sculk shriekers, sensors and catalysts stand around the city. Do not trigger the shriekers yet; their behaviour comes with groups `vibrations` and `shriekers-catalysts` (**116_ancient_city_sculk**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword,minecraft:torch*64,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:ladder*64,minecraft:water_bucket,minecraft:potion@night_vision,minecraft:potion@night_vision
```

## Review

The adversarial review of the whole track (24dfdf71..2b1160db). One commit per fix.

### 1. Track D's new custom spawner and horse files pass rustfmt

After 2b1160db.

- **Defect:** two new files of this track failed `rustfmt --check` on their committed content, against the rule for new files: `crates/minecraft_terrain/src/custom_spawners.rs` (group wandering-trader, two `assert!` calls in a test) and `third_party/minecraftoss/entities/src/world/equines.rs` (groups horse-family and riding: `ride_tick`, `horse_ride` and the riding tests).
- **Changed:** both files formatted with `rustfmt --edition 2021` (they are new in this track, so no upstream lines move). No code changes.
- **Tests:** none new; the formatting changes no behaviour.
- **Test steps:** none.

### 2. A baby Minecraft drowned gives vanilla's experience

After 3fb8ee1f.

- **Defect:** a baby drowned killed by a player multiplied its whole reward by 2.5, equipment bonus included, so a baby drowned that kept its trident gave 15 to 20 experience. javap of 26.3 `Zombie.getBaseExperienceReward`: a baby first sets `xpReward` to `(int) (5 * 2.5)` = 12, and only then `Mob.getBaseExperienceReward` adds `1 + nextInt(3)` for each held or worn item with a drop chance of at most 1. So the vanilla reward is 13 to 15.
- **Changed:** `third_party/minecraftoss/entities/src/world.rs` (`take_deaths`, the drowned branch): the base is 12 for a baby, 5 for an adult, and the equipment bonus is added after.
- **Tests:** `world::tests::a_baby_drowned_s_kill_gives_twelve_experience_and_its_kept_trident_adds_one_to_three`.
- **Test steps (survival):**
  1. `/summon minecraft:drowned ~ ~ ~3 {IsBaby:1b,equipment:{mainhand:{id:"minecraft:trident",count:1}}}` and kill it with the sword. Count the orbs: 13 to 15 experience in all (vanilla's split, often 10 + 3 or so), never 16 or more; exactly 12 when it dropped the trident (**117_baby_drowned_experience**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword
```

### 3. A Minecraft horse inventory opens for one player at a time

After 31d0d242. It also corrects step 1 of the entry above (a baby drowned that drops its trident gives exactly 12).

- **Defect (multiplayer item duplication):** each player's horse inventory screen is a copy of the mount's saddle, armour and chest taken when it opens, and every click writes the whole copy back to the level (`Command::MountInventory`). Two players could open the same tame donkey at once (the host by crouch-click, a friend by crouch-click or from its saddle). Then player A takes the diamonds and A's copy is written back without them. Player B's copy still shows them, so B takes them again, and both players end up with the diamonds.
- **Changed:**
  - `crates/minecraft_survival/src/entities.rs`: the level's `Entities` keeps which player has each mount's inventory open (`mount_viewer`, `set_mount_viewer`), and `leave` lets a leaving player's go.
  - `crates/minecraft_survival/src/menus.rs`: `mount` refuses to open a mount's inventory that another player has open, and records each player's open mount screen at the end of every call (none once it closes, dies or goes out of reach).
- **Decision:** vanilla shares one `SimpleContainer` between all viewers, so two players can have it open at once. The fork's screen is a per-player copy written back as a whole, so the vanilla-like safe option is one viewer at a time. Villager trading already works this way, and vanilla allows one trader per villager too. A second player's crouch-click does nothing while the screen is open, and works once it is closed.
- **Left open:** another player's right-click that saddles, armours or chests the mount while its screen is open is not merged into the open copy. The next click in the screen writes the copy back and can remove that saddle or armour. The window between a mount's death and the screen closing (one server tick) is also open. Both are listed under "Left for later".
- **Tests:** `menus::tests::a_horse_inventory_opens_for_one_player_at_a_time` (gated on the 26.3 data). It covers the first opener, the refused second player, the release on close and the release on leaving the level. It fails without the fix.
- **Test steps (host and a friend, survival):**
  1. Host: `/summon minecraft:donkey ~ ~ ~2 {Tame:1b,ChestedHorse:1b}`, crouch and right-click it, and put the 64 diamonds in its chest. Leave the screen open (**118_donkey_open_host**).
  2. Friend: crouch and right-click the same donkey. No screen opens (**119_donkey_refused_friend**).
  3. Host: close the screen. Friend: crouch and right-click again. The donkey's screen opens with the 64 diamonds. Take them (**120_donkey_friend_takes**).
  4. Host: crouch and right-click the donkey once the friend has closed it. The chest is empty, and the diamonds exist only once, in the friend's inventory (**121_donkey_no_copy**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond*64
```

### 4. The wandering trader spawner test no longer depends on the clock

After 2ac3b93b.

- **Defect (flaky test):** `server::tests::the_trader_spawner_brings_a_trader_within_48_blocks_with_two_trader_llamas_on_leads` failed in about 1 run in 12. The loot-functions group had listed it under "Left for later". The spawner's own random is seeded from the clock, as vanilla's `RandomSource.create()` is, so every run picked other spots on seed 0's hilly terrain.
  - Debug output from the failing runs showed the trader at y 56 with a trader llama at y 38 (and once the other way round). Their box centres were 18 blocks apart, so the first leash pass snapped the lead, as vanilla's `Leashable.tickLeash` does beyond 12 blocks.
  - One run found only one llama spot in vanilla's 10 tries.
  - Both are vanilla behaviour (`findSpawnPositionNear` takes any surface spot within 4 blocks), not a code defect.
- **Changed:** `crates/minecraft_terrain/src/server.rs`, the test only: it seeds the trader spawner's random with 5, so the trader and both llamas land on the same level ground every run.
- **Tests:** the test itself, run 20 times in a row with no failure. The line under "Left for later" is removed.
- **Test steps:** none (a test-only change).

## Final verification

### 5. Track D's new code passes clippy

After 1cf92f76.

- **Defect:** clippy on the touched crates (`frame`, `hud`, `minecraft_commands`, `minecraft_net`, `minecraft_survival`, `minecraft_terrain`, `render_anim`, `sim` and the five minecraftoss crates, `--all-targets`) warned on three lines this track added; every other warning is older than the track.
  - `crates/minecraft_terrain/src/mob_actions.rs` (group built-golems, bd2521c7): the snow golem's shearing loot used `unwrap_or_else(Vec::new)` (`clippy::unwrap_or_default`, while `unwrap_or_default` itself is disallowed by `clippy.toml`). It now flattens the optional loot.
  - `crates/minecraft_terrain/src/server.rs` (group drowned): the struck mob trident used the disallowed `Option::unwrap_or_default` on an owner the match guard already checked. It now names its fallback with `unwrap_or(0)`.
  - `crates/minecraft_survival/src/entities.rs` (group trading-friend, a test): `field_reassign_with_default` on the test's `Output`, now built with the field in its literal.
- **Tests:** none new; behaviour is unchanged. The tests of all touched crates run green.
- **Test steps:** none.

## Group pillagers

### 1. Minecraft pillagers live in the entity world and shoot crossbows

After b429636a.

- **Vanilla (javap, 26.3):**
  - `EntityTypes.PILLAGER`: `MobCategory.MONSTER`, `canSpawnFarFromPlayer`, `sized(0.6, 1.95)` (eye height the default 0.85 of the height, 1.6575), `passengerAttachments(2.0)`, `ridingOffset(-0.6)`, `clientTrackingRange(8)`, `notInPeaceful`. Vindicators and evokers have the same size and range.
  - `Pillager.createAttributes`: `Monster.createMonsterAttributes`, movement speed 0.35 (`0.3499999940395355`), max health 24, attack damage 5, follow range 32. `Vindicator`: 0.35, follow range 12, health 24, damage 5. `Evoker`: 0.5, follow range 12, health 24.
  - Class chain `Pillager` → `AbstractIllager` → `Raider` → `PatrollingMonster` → `Monster`. `Raider` saves `Wave`, `CanJoinRaid` and `RaidId`, syncs `IS_CELEBRATING`, runs its idle clock double (`updateNoActionTime` +2), never despawns while in a raid and joins a raid at its block every 20 ticks when it can. `PatrollingMonster` saves `patrol_target`, `PatrolLeader` and `Patrolling`; `removeWhenFarAway` is plain unless patrolling (then only beyond 128 blocks). `AbstractIllager.canAttack` spares baby villagers; its default arm pose is `CROSSED`.
  - Goals by registration: `PatrollingMonster` 4 `LongDistancePatrolGoal(0.7, 0.595)`; `Raider` 1 `ObtainRaidLeaderBannerGoal`, 3 `PathfindToRaidGoal`, 4 `RaiderMoveThroughVillageGoal(1.05)`, 5 `RaiderCelebration`; `Pillager` 0 `FloatGoal`, 1 `AvoidEntityGoal(Creaking, 8, 1.0, 1.2)`, 2 `Raider.HoldGroundAttackGoal(10)`, 3 `RangedCrossbowAttackGoal(1.0, 8)`, 8 `RandomStrollGoal(0.6)`, 9 `LookAtPlayerGoal(Player, 15, 1.0)`, 10 `LookAtPlayerGoal(Mob, 15)`. Targets: 1 `HurtByTargetGoal(Raider ignored).setAlertOthers()`, 2 `NearestAttackableTargetGoal(Player, true)`, 3 `(AbstractVillager, false)`, 3 `(IronGolem, true)`.
  - `RangedCrossbowAttackGoal` (MOVE and LOOK, every tick): usable with a living target and a crossbow in either hand. Each tick the sight counter resets when sight changes, then counts up or down; it walks at the target (speed 1.0 while uncharged, 0.5 otherwise, a new path every 20 to 40 ticks, `TimeUtil.rangeOfSeconds(1, 2)`) while the target is beyond 8 blocks or seen for under 5 ticks and no shot is waiting, else stops; it looks at the target (30, 30). UNCHARGED: when standing, start using the crossbow and set charging. CHARGING: when no longer using, back to UNCHARGED; at 25 ticks of use (`CrossbowItem.getChargeDuration`, 1.25 s) release, CHARGED, an attack delay of `20 + nextInt(20)` and charging off. CHARGED: the delay counts down to READY_TO_ATTACK. READY_TO_ATTACK: with the target in sight, `performRangedAttack` and UNCHARGED. `stop` clears aggression and the target and stops using (its `CHARGED_PROJECTILES` reset lands on the emptied use item, so it does nothing).
  - `CrossbowItem.onUseTick` (before the AI, with the ticks used so far): `loading_start` at 20 % and `loading_middle` at 50 % (volume 0.5, pitch 1), and at 100 % the arrow loads with `loading_end` (volume 1, pitch `1 / (level random * 0.5 + 1) + 0.2`). `releaseUsingItem` runs one more use tick for a crossbow (`useOnRelease`), which loads it at exactly 25 ticks.
  - `Pillager.performRangedAttack(target, 1.6)` → `performCrossbowAttack`: shoots only the charged arrow (power 1.6, spread `14 - 4 * difficulty`), aimed a third of the way up the target plus 0.2 per block of distance, `item.crossbow.shoot` at pitch 1 for the single arrow (`getShotPitch` index 0, no draw), then `onCrossbowAttackPerformed` resets the idle clock.
  - `Raider.HoldGroundAttackGoal(10)` (MOVE and LOOK, every tick): only outside a raid, while patrolling, with a target, not aggressive and not last hurt by a player. Start stops the path and sets its target on raiders within 8 blocks; while the target is beyond 10 blocks it looks at it and plays its ambient sound one tick in 50, closer it becomes aggressive; stop sets the target and aggression on raiders within 8 and itself.
  - `Pillager.getArmPose`: `CROSSBOW_CHARGE` while charging, `CROSSBOW_HOLD` while holding a crossbow, `ATTACKING` when aggressive, else `NEUTRAL`.
  - `WanderingTrader.registerGoals` flees zombies (8), evokers (12), vindicators (8), vexes (8), pillagers (15), illusioners (12) and zoglins (10), all at speed 0.5. `VillagerHostilesSensor` counts a pillager within 15 blocks.
  - Equipment drops (`Mob.dropCustomDeathLoot`): each slot in `EquipmentSlot.VALUES` order at its drop chance (0.085) on a player's kill, a dropped crossbow worn to `465 - nextInt(1 + nextInt(462))`. Experience: `Monster`'s 5 plus 1 to 3 per kept item whose chance is at most 1.
- **Changed:**
  - New vendored `entities/src/illager.rs` (the `IllagerKind` family `Pillager`, `Vindicator`, `Evoker` with their sizes, speeds, follow ranges and sounds; `Illager` with equipment, drop chances, inventory, `Raider` state (wave, can join raid, celebrating, raid id, patrol target, leader, patrolling), charging and aggression; the crossbow charge kept as vanilla's `charged_projectiles` component; `IllagerArmPose` and its index for friends), `entities/src/illager_ai.rs` (the pillager's goal and target lists on the monster goal framework: the five raider goals, which cannot start outside a patrol or raid; `RangedCrossbowAttackGoal`, `HoldGroundAttackGoal`, a plain `RandomStrollGoal`, `LookAtPlayerGoal` at probability 1 and for mobs; the hurt-by target goal ignoring raiders and alerting other pillagers) and `entities/src/world/illagers.rs` (the `illagers` store: `IllagerEntity`, `spawn_illager`, its tick with the raider idle clock, the crossbow's use ticks and sounds, the shot as an arrow with the crossbow hit sound, the hold-ground call to nearby illagers, the alert to other pillagers; `is_ominous_banner` for the captain).
  - Vendored `monster_ai.rs`: `MonsterKind::Pillager` (walk target zero, floats, follow range 32), `IllagerState` in the goal context, pillagers spare baby villagers, `is_enemy` counts pillagers, vindicators and evokers. `world.rs` and `world/{combat,hazards,pushing,riding,villagers,witches}.rs`: the store in every walk over the mobs, `EntityKey::Illager` and `MobHit::Illager`, arrows, mob hits, splash potions and blasts on it, its deaths with the captain's loot context, the crossbow drop and experience, its sounds (`hostile_volume`), the census and the monster despawn rule, and villagers see it as a hostile. `trader_ai.rs` and `wandering_trader.rs`: the trader's five illager `AvoidEntityGoal`s in their vanilla slots.
  - `minecraft_survival/src/entities.rs`: hit key 32 for `MobHit::Illager` in `encode` and `decode` (the compiler needs the arm now; the hit boxes come with commit 2).
- **Tests:**
  - Vendored entities: `illager::tests` (`a_pillager_is_a_crossbow_raider_of_vanilla_size`, `its_arm_pose_charges_holds_attacks_or_hangs`, `a_charged_crossbow_carries_one_arrow_until_it_shoots`, `raiders_are_the_illagers_the_witch_and_the_ravager`), `world::illagers::tests` (`a_pillager_is_a_live_monster_of_its_size_and_health`, `a_pillager_charges_its_crossbow_for_25_ticks_then_shoots_20_to_39_ticks_later` (six seeds: exactly 25 ticks of charge, 21 to 40 ticks to the shot, the four crossbow sounds, the arrow owned by it with the crossbow hit sound), `a_pillager_walks_closer_when_its_target_is_beyond_eight_blocks`, `a_pillager_targets_villagers_and_iron_golems_but_not_baby_villagers`, `a_patrolling_pillager_holds_its_ground_and_calls_the_others`, `iron_golems_hunt_pillagers_and_wandering_traders_flee_them`, `a_villager_senses_a_pillager_within_fifteen_blocks_as_a_hostile`, `a_pillager_shot_by_another_raider_does_not_turn_on_it`, `a_dead_pillager_leaves_five_experience_and_sometimes_its_crossbow`, `a_patrol_leader_wearing_the_ominous_banner_dies_as_a_captain`).
  - `minecraft_survival`: `entities::tests::a_pillager_hit_box_encodes_and_decodes`.
  - Runs: `minecraftoss-entities` lib 316, `minecraft_survival` lib, all green. `check -p launcher` green. The three new vendored files pass `rustfmt --check --edition 2021`.
- **Decisions:**
  - One `illagers` store for the family (hit key 32), as the batching rule asks; only the pillager has its behaviour and spawns. Vindicators and evokers get their goals in groups `patrols-vindicators` and `evokers-vexes`.
  - The crossbow's item use lives in the goal context's item-use counters (the ones the skeleton's bow uses), so the charge counts up before the AI like `LivingEntity.updatingUsingItem`.
  - The goal's `stop` does not unload a charged crossbow: vanilla's reset lands on the already emptied use item.
  - The crossbow shot sound has pitch 1 with no random draw, as vanilla's single arrow does (the piglin port draws a pitch; see Left for later).
  - The crossbow loading sounds play in the pillager's own (hostile) category; vanilla plays start and middle as `PLAYERS` sounds.
  - The raider goals (`LongDistancePatrolGoal`, raid banner, pathfinding to the raid, moving through the village, celebrating) hold their slots but cannot start: there are no patrols or raids yet (groups `patrols-vindicators` and `raids`). Hold-ground works for a pillager loaded as patrolling. The patrolling despawn rule (only beyond 128 blocks) waits for patrols too.
  - The creaking `AvoidEntityGoal` is the real goal; it finds nothing until creakings exist.
  - Hold-ground and the hurt alert reach other illagers; witches, which are raiders too, are not called yet.
  - Pillagers pick up nothing yet (`Mob.pickUpItem` equipment and the raid banner); `CanPickUpLoot` is kept for saving.
- **Test steps:** none on their own. A pillager can be hatched from commit 3 on, whose steps cover this commit (crossbow fight, iron golems, fleeing traders and villagers).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:pillager_spawn_egg*8,minecraft:iron_golem_spawn_egg*2,minecraft:villager_spawn_egg*4,minecraft:wandering_trader_spawn_egg*2,minecraft:shield,minecraft:diamond_sword,minecraft:cooked_beef*32
```

### 2. Minecraft pillagers are drawn like vanilla

After 836bb203.

- **Vanilla (javap, 26.3):**
  - `LayerDefinitions`: `ModelLayers.PILLAGER` (and `EVOKER`, `VINDICATOR`, `ILLUSIONER`) is `IllagerModel.createBodyLayer()` with `MeshTransformer.scaling(0.9375)` (the root scaled by 0.9375 and moved down `24.016 * (1 - 0.9375)` pixels).
  - `IllagerModel.createBodyLayer` (64 x 64): `head` (0, 0) box (-4, -10, -4) 8 x 10 x 8 at (0, 0, 0) with the `hat` (32, 0) box (-4, -10, -4) 8 x 12 x 8 grown 0.45 (hidden: `hat.visible = false`) and the `nose` (24, 0) box (-1, -1, -6) 2 x 4 x 2 at (0, -2, 0); `body` (16, 20) box (-4, 0, -3) 8 x 12 x 6 and the robe (0, 38) box (-4, 0, -3) 8 x 20 x 6 grown 0.5; `arms` at (0, 3, -1) turned -0.75 about X: (44, 22) box (-8, -2, -2) 4 x 8 x 4 and (40, 38) box (-4, 2, -2) 8 x 4 x 4, with `left_shoulder` (44, 22) mirrored box (4, -2, -2) 4 x 8 x 4; `right_leg` (0, 22) box (-2, 0, -2) 4 x 12 x 4 at (-2, 12, 0), `left_leg` the same mirrored at (2, 12, 0); `right_arm` (40, 46) box (-3, -2, -2) 4 x 12 x 4 at (-5, 2, 0), `left_arm` the same mirrored, box (-1, -2, -2), at (5, 2, 0).
  - `IllagerModel.setupAnim`: the head turns and pitches; riding, the arms are at -0.628 and the legs at -1.414 spread 0.314 and rolled 0.0785; walking, the arms swing `cos(pos * 0.6662 (+ pi)) * 2 * speed * 0.5` and the legs `* 1.4 * speed * 0.5`. By arm pose: `ATTACKING` (zombie arms and a second bob with an empty main hand, else `swingWeaponDown`), `SPELLCASTING` (arms raised to 2.356 rolled, swaying 0.25), `BOW_AND_ARROW`, `CROSSBOW_HOLD` (`AnimationUtils.animateCrossbowHold`, always right-handed: the right arm -0.3 and the left 0.6 plus the head's turn, the right arm -pi/2 + 0.1 and the left -1.5 plus the head's pitch), `CROSSBOW_CHARGE` (`animateCrossbowCharge`, right-handed: the right arm at -0.8 and -0.971, the left drawing from 0.4 to 0.85 and from -0.971 to -pi/2 over the 25 charge ticks), `CELEBRATING`. The crossed `arms` show only for `CROSSED`, the two free arms otherwise.
  - `IllagerRenderer`: shadow 0.5 (`PillagerRenderer`), a `CustomHeadLayer`; `extractRenderState` takes the riding flag, the main arm, the arm pose, the crossbow's charge duration while charging and the ticks used with the partial tick. `PillagerRenderer` adds `ItemInHandLayer` (both hands, always) and draws `textures/entity/illager/pillager.png`.
  - `ItemInHandLayer`: the arm's pose, then -90 degrees about X, 180 about Y and out to the hand (`±1/16, 2/16, -10/16`), in the third-person hand display.
- **Changed:**
  - New `crates/minecraft_terrain/src/illager_render.rs`: the model ported from `createBodyLayer` and `setupAnim` on `posed_cube` (all eight arm poses, riding, the 0.9375 scale, the grown robe, the hidden hat), the textures of the three illagers, the crossbow (or any held item) in the hand by `ItemInHandLayer`'s pose, shadow 0.5.
  - `minecraft_terrain`: `lib.rs` (the module), `mesh.rs` (the three illager textures), `client_mobs.rs` (the pose push and the shadow), `mob_tracking.rs` (pillagers, vindicators and evokers tracked at 8 chunks; friends see the arm pose, the charge ticks, charging and aggression through `MobShow.arm_pose`, `using_item`, `charging` and `aggressive`, no new field).
  - `minecraft_survival/src/entity_view.rs`: the illagers drawn with their held items, and their hit boxes for MW2 bullets (hit key 32 from commit 1).
- **Tests:**
  - `minecraft_terrain`: `illager_render::tests` (`a_pillager_stands_two_blocks_tall_at_fifteen_sixteenths_with_free_arms`, `crossed_arms_replace_the_two_free_arms`, `a_crossbow_is_held_up_to_the_eye_and_the_string_pulled_while_charging`, `the_crossbow_sits_in_the_right_hand_ten_pixels_down_the_arm`, `a_riding_illager_bends_its_legs_forward`), `mob_tracking::tests::a_friend_sees_a_pillager_charge_and_then_hold_its_crossbow`.
  - Runs: `minecraft_terrain` illager and tracking tests, `minecraft_survival` lib 715, all green. `check -p launcher` green. The new `illager_render.rs` passes `rustfmt --check --edition 2021`.
- **Decisions:**
  - The held crossbow is drawn with its plain item model: the held-item renderer takes no components, so a loaded crossbow does not show its arrow and a charging one does not show its pull (the piglin's crossbow is drawn the same way).
  - The hat cube is left out, as vanilla hides it; no layer draws it for illagers.
  - The custom head layer (a block or skull worn on the head) is not drawn: the fork's mobs draw armour heads elsewhere and pillagers wear none (the captain's banner comes with group `patrols-vindicators`).
  - Pillagers are not drawn in spawner cages yet: the cage builds its mob from the saved tag, which commit 3 brings.
- **Test steps:** none on their own; commit 3's steps check the model (a pillager can be hatched from commit 3 on).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:pillager_spawn_egg*8,minecraft:iron_golem_spawn_egg*2,minecraft:villager_spawn_egg*4,minecraft:wandering_trader_spawn_egg*2,minecraft:shield,minecraft:diamond_sword,minecraft:cooked_beef*32
```

### 3. Minecraft pillagers are saved and spawn at pillager outposts

After b569efa9.

- **Vanilla (javap, 26.3, and the 26.3 data pack):**
  - `worldgen/structure/pillager_outpost.json`: `spawn_overrides.monster` with `bounding_box: full` and the single entry pillager (weight 1, count 1), so inside the outpost's whole box the monster list is pillagers only.
  - `SpawnPlacements` for the pillager: `ON_GROUND`, `MOTION_BLOCKING_NO_LEAVES`, `PatrollingMonster.checkPatrollingMonsterSpawnRules`: no spawn where the block light is above 8, else `checkAnyLightMonsterSpawnRules` (not peaceful and a valid block below), in any sky light. `Pillager.getWalkTargetValue` is 0 and `getMaxSpawnClusterSize` 1; `EntityType.canSpawnFarFromPlayer`.
  - `Pillager.finalizeSpawn`: `populateDefaultEquipmentSlots` puts a crossbow in the main hand; `populateDefaultEquipmentEnchantments` draws the main hand's roll (`0.25 * special multiplier`) and then `Pillager.enchantSpawnedWeapon` draws `nextInt(300)`, which at 0 enchants the crossbow from `enchantment_provider/pillager_spawn_crossbow` (piercing I); then `Raider.finalizeSpawn` sets `CanJoinRaid` (true for every raider but a natural witch), `PatrollingMonster.finalizeSpawn` makes a leader for reasons other than patrol, event and structure on `nextFloat() < 0.06` and gives a leader the ominous banner (drop chance 2), and `Mob.finalizeSpawn` rolls the follow range bonus and left-handedness.
  - `Pillager` saves its `Inventory` (5 slots) through `InventoryCarrier`; `Raider` saves `Wave`, `CanJoinRaid` and `RaidId`; `PatrollingMonster` saves `patrol_target` (a block position), `PatrolLeader` and `Patrolling`. `Pillager.readAdditionalSaveData` always sets `CanPickUpLoot`.
- **Changed:**
  - New `crates/minecraft_terrain/src/illager_save.rs`: an illager from its tag (health, yaw, left hand, equipment and drop chances, the pillager's inventory, wave, raid join, raid id, patrol target, leader, patrolling; a pillager always picks up loot) and its fields for saving.
  - `minecraft_terrain`: `server_mobs.rs` (`SIMULATED_KINDS`, the `spawn_saved` arm and the `mob_tags_where` arm), `lib.rs`, `spawner_display.rs` (pillagers turn in spawner cages), `server.rs` (`HATCHING_EGGS` grows to 45; the dormant-monster test now uses a silverfish, as pillagers live).
  - Vendored `world/src/natural_spawner.rs`: `type_info` for the pillager (0.6 x 1.95 on the ground, its own `Rules::Patrolling`, walk value 0, clusters of 1, removed far away like monsters) and `natural_spawner/spawning.rs`: the patrolling rule, its follow range base of 32, and its finalize (crossbow, the enchantment rolls with piercing I one time in 300, `CanJoinRaid`, the 6 % leader roll, then `Mob`'s part). Summons and eggs finalize the same way.
- **Tests:**
  - Vendored world: `natural_spawner::spawning::tests::pillagers_spawn_alone_in_a_pillager_outpost_by_day_with_a_crossbow` (the type info and follow range; 3000 rounds by day in an outpost column spawn over 100 pillagers, one at a time, each with a crossbow and `CanJoinRaid`, a few leaders, and nothing outside the outpost by day).
  - `minecraft_terrain`: `server::tests::pillager_outposts_spawn_pillagers_with_crossbows_by_day` (seed 1234: the nearest outpost is found, its box answers `kinds_at` with pillagers only and the land far away does not; spawning inside the box by day makes live pillagers, only pillagers, inside the box with crossbows), `server_mobs::tests::a_pillager_saves_and_loads_with_its_crossbow_and_raider_state` (a loaded crossbow stays loaded through a save), `mob_tracking::tests::a_friend_sees_a_pillager_from_its_look_tag_with_its_crossbow`, the spawner cage test with the pillager, and `every_spawn_egg_of_a_mob_the_game_has_hatches_that_mob` now hatches 45 eggs.
  - Runs: `minecraftoss-world` lib 91, `minecraft_terrain` lib 1292 of 1295 run (3 ignored), all green. `check -p launcher` green. Clippy on `minecraftoss-entities`, `minecraftoss-world`, `minecraft_terrain` and `minecraft_survival` (`--all-targets`, `-A clippy::approx_constant`) finds nothing in this group's code. The new `illager_save.rs` passes `rustfmt --check --edition 2021`.
- **Decisions:**
  - A patrol leader rolled at spawn gets `PatrolLeader` but no ominous banner yet (the level notes it as unsupported); the banner, and with it the captain's bottle, comes with group `patrols-vindicators`.
  - Outpost pillagers come only from natural spawning, as in vanilla: the outpost templates place no pillagers (its allay and iron golem cages are the generator's).
  - Vindicators and evokers still wait dormant: the `illagers` store holds them only once their groups give them behaviour.
- **Test steps (survival, a world with a pillager outpost; by day is fine, pillagers ignore the sky light):**
  1. `/locate structure minecraft:pillager_outpost` and `/tp` to about 40 blocks from the tower. Within a minute pillagers stand about the tower and its base, each holding a crossbow up in both hands (**122_pillager_outpost**).
  2. Walk up to about 15 blocks: a pillager turns to you, pulls the string back with two clicks, waits one to two seconds and shoots; the arrow flies with the crossbow sound and hurts. Hold up the shield: it blocks the arrows (**123_pillager_crossbow**).
  3. Back away beyond 8 blocks: they follow you and stop again within 8 to shoot. Kill a few with the sword: each leaves 6 to 8 experience, now and then a worn crossbow (then 5).
  4. `/gamemode creative` (pillagers then leave you alone). On flat ground, use a villager egg and a wandering trader egg about 10 blocks from a pillager egg: the pillager shoots the villager; the villager and the trader run away from it (**124_pillager_villagers**).
  5. Use an iron golem egg near a pillager: the golem walks over and strikes it; the pillager shoots at the golem (**125_golem_pillager**). `/gamemode survival`.
  6. Save and quit, then load: the pillagers are still there with their crossbows and fight again.
  7. Friend window: the friend sees the pillagers, their crossbows and the string pulled back while they charge, gets shot at, and shoots them with a gun (**126_friend_pillagers**).
  8. `/difficulty peaceful`: the pillagers vanish; a pillager egg hatches nothing. `/difficulty normal` again.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:pillager_spawn_egg*16,minecraft:iron_golem_spawn_egg*2,minecraft:villager_spawn_egg*8,minecraft:wandering_trader_spawn_egg*2,minecraft:shield,minecraft:diamond_sword,minecraft:crossbow,minecraft:arrow*64,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:spyglass,minecraft:cooked_beef*32
```

## Group patrols-vindicators

### 1. Minecraft pillager patrols roam with a banner captain

After 08fcef00.

- **Vanilla (javap, 26.3, and the 26.3 data pack):**
  - `PatrolSpawner.tick(level, spawnEnemies)` (a custom spawner of the Overworld only, run by `ServerChunkCache.tickChunks` while `spawn_mobs` is on, with `spawnEnemies` from `ServerLevel.isSpawningMonsters`: `spawn_mobs` and `spawn_monsters`): nothing without `spawnEnemies` or the game rule `spawn_patrols`; then `nextTick` counts down and at 0 or less gains `12000 + nextInt(1200)`; then `isBrightOutside`, `nextInt(5) == 0`, at least one player, a random one of `level.players()` that is not a spectator, not `isCloseToVillage(its block, 2)` (`sectionsToVillage` at most 2); the start is the player's block moved `(24 + nextInt(24)) * (nextBoolean ? -1 : 1)` on X, then the same on Z; `hasChunksAt` 10 blocks around it; the environment attribute `gameplay/can_pillager_patrol_spawn` there; group size `ceil(getCurrentDifficultyAt(start).getEffectiveDifficulty()) + 1`. For each member the Y is the `MOTION_BLOCKING_NO_LEAVES` height, the first member is the leader and a failed first member ends the patrol; after each member X and Z move by `nextInt(5) - nextInt(5)`. Every draw comes from the level random. `nextTick` is a plain field: it is not saved and starts at 0, so a loaded world checks on its first tick.
  - **No "day 5" check in the code any more:** the rule lives in the data. `timeline/early_game.json` (on the Overworld clock) holds `gameplay/can_pillager_patrol_spawn` false until 120000 ticks (day 5) with the modifier `and`, and `worldgen/biome/mushroom_fields.json` sets it false. The attribute's default is true (`EnvironmentAttributes`).
  - `spawnPatrolMember`: `NaturalSpawner.isValidEmptySpawnBlock` for the pillager, `PatrollingMonster.checkPatrollingMonsterSpawnRules` (block light at most 8, not peaceful, a valid spawn block below), `EntityType.create(PATROL)`; a leader gets `setPatrolLeader(true)` and `findPatrolTarget()` **before** `setPos`, so its target is the origin `(0, 0, 0)` moved by `-500 + nextInt(1000)` on X and Z from the mob's own random (Y 0); then `setPos(x, y, z)` at the block corner and `finalizeSpawn(PATROL)`.
  - `PatrollingMonster.finalizeSpawn`: the 6 % leader roll only for reasons other than `PATROL`, `EVENT` and `STRUCTURE`; a leader gets `Raid.getOminousBannerInstance` on its head with drop chance 2; reason `PATROL` sets `patrolling`. `removeWhenFarAway(d)` is `!patrolling || d > 16384`. `setPatrolTarget` and `setPatrolLeader` also set `patrolling`.
  - `Raid.getBannerComponentPatch`: a white banner with `banner_patterns` rhombus cyan, stripe_bottom light gray, stripe_center gray, border light gray, stripe_middle black, half_horizontal light gray, circle light gray, border black; `tooltip_display` hiding the patterns; `item_name` translatable `block.minecraft.ominous_banner`; `rarity` uncommon.
  - `PatrollingMonster.LongDistancePatrolGoal(0.7, 0.595)` (MOVE): usable while patrolling, without a target, rider or cooldown, with a patrol target. Each tick with its navigation done: the `PatrollingMonster`s whose boxes meet its box grown by 16 that `canJoinPatrol` (a raider without an active raid) are its companions; patrolling with none, it stops patrolling; a leader within 10 (3-D, `closerToCenterThan`) of its target finds a new one; otherwise the target's bottom centre plus `(position - target).yRot(90) * 0.4` (90 **radians**, as the code passes it), 10 blocks from its position towards that, at the `MOTION_BLOCKING_NO_LEAVES` height, is walked to at 0.595 (leader) or 0.7; when no path is made it walks to a random spot within 8 blocks (0.7) and waits 200 ticks, else a leader gives every companion that spot as its patrol target.
  - `Raider.die`: only a raider in a raid does more (removes its leader, makes a player killer a hero, leaves the raid). The captain's ominous bottle is `loot_table/entities/pillager.json`: one bottle with `set_ominous_bottle_amplifier` uniform 0 to 4 when `type_specific/raider.is_captain`.
  - `CustomHeadLayer` for a head item that is no skull: `translateToHead`, then `translate(0, -0.25, 0)`, 180 degrees about Y and `scale(0.625, -0.625, -0.625)`, drawn in the `HEAD` display; `item/template_banner.json` has its own `head` display.
- **Changed:**
  - Vendored `player/src/lib.rs`: `World::motion_blocking_no_leaves_height` (the entity world's `MOTION_BLOCKING_NO_LEAVES` height; authored worlds scan down for a block with collision or a fluid that is no leaves). `minecraft_terrain` `server_mobs.rs` `MobWorld` answers it from the level's heightmap.
  - Vendored `world/src/natural_spawner/spawning.rs`: `ominous_banner()` (the banner's NBT stack), `finalize_pillager` (the pillager arm of `finalize_monster`, now also for patrols: the leader roll only where vanilla rolls it, the banner with drop chance 2 on every leader, `Patrolling` for patrol members), `spawn_patrol` and `patrol_member`. `world/src/level/spawning.rs` `Level::spawn_patrol`; `world/src/level/time.rs` `can_pillager_patrol_spawn_at` (the attribute with the biome and the timelines).
  - Vendored `entities/src/illager.rs` (the patrol numbers), `illager_ai.rs` (`LongDistancePatrolGoal` in the pillager's slot 4, `find_patrol_target`; the goal context carries the leader flag, the patrol target, the companions and the leader's order), `world/illagers.rs` (companions within the grown box before the goals, the leader's spot handed to them after, the patrol state written back), `world.rs` (`check_despawn`: a patrolling illager is kept while a player is within 128 blocks).
  - `minecraft_terrain`: `custom_spawners.rs` `PatrolSpawner`, `patrol_offset`, `near_village` and the `spawn_patrols` flag; `server.rs` `tick_patrol_spawner` on the level tick before the cat spawner (vanilla's order: phantoms, patrols, cats, sieges, traders) and the rule from `apply_game_rules`; `illager_render.rs` draws a head item that is no skull or helmet (the captain's banner) in the `HEAD` display on the illager's head (`head_item_pose`, `worn_on_head`).
- **Tests:**
  - Vendored entities `world::illagers::tests`: `a_patrol_leader_heads_for_its_target_and_its_companions_follow`, `a_patroller_without_companions_stops_patrolling` (and `find_patrol_target`'s range), `a_patrolling_illager_stays_while_a_player_is_within_128_blocks`.
  - Vendored world `natural_spawner::spawning::tests`: `a_patrol_is_a_captain_with_the_ominous_banner_and_pillagers_on_patrol` (three members on a new Normal world, the captain's target near the origin with Y 0, the banner's eight patterns, name, rarity and drop chance 2, block corners on the ground, none in peaceful, none when the captain cannot stand); `pillagers_spawn_alone_in_a_pillager_outpost_by_day_with_a_crossbow` now checks that a natural leader carries the banner and no outpost pillager patrols.
  - `minecraft_terrain`: `custom_spawners::tests::the_patrol_spawner_looks_every_ten_to_eleven_minutes_and_only_by_day`, `patrols_start_24_to_47_blocks_away_on_both_axes_and_not_near_a_village`; `server::tests::patrols_come_by_day_away_from_villages_with_a_banner_captain_who_drops_an_ominous_bottle` (real terrain: none before day five, none at night, none beside a claimed bell, none with the rule off, then a patrol 24 to 47 blocks away whose captain carries the banner; killed, it drops the banner and an ominous bottle from the 26.3 loot); `mob_tracking::tests::a_friend_sees_a_patrol_captain_with_its_ominous_banner`; `illager_render::tests::a_captain_wears_its_banner_above_its_head_but_no_helmet_or_skull`.
  - Runs: `minecraftoss-entities` lib 319, `minecraftoss-world` lib 92, `minecraft_terrain` lib 1297 of 1300 (3 ignored), all green. `check -p launcher` green. The `minecraftoss-player` lib tests need the full suite's stand-in files (`scenarios/survival/*.csv`), so they run there. No new files.
- **Decisions:**
  - The "day 5" rule is not code: it comes from the 26.3 `early_game` timeline through the environment attributes, read at runtime with the mushroom fields' false.
  - The spawner's clock is not saved, as in vanilla (a plain field): it lives with the Overworld's custom spawners and starts at 0, so loading a world checks for a patrol on the first tick.
  - Vanilla's quirks are kept: the leader's target is drawn before it is placed (within 500 blocks of the world origin, Y 0, so a leader never "arrives" and patrols drift towards the origin), the members stand on block corners, the swerve turns by 90 radians, and a patrol whose first path fails because it has not touched the ground yet waits 200 ticks.
  - Patrol companions are illagers only. Witches and ravagers are `PatrollingMonster`s too; witches have no patrol state yet and ravagers do not exist (groups `ravagers` and `raids`).
  - The banner is drawn on the captain's head as the plain white banner item: the fork draws no banner patterns yet (neither placed nor held). The patterns are saved with the stack. Helmets are not drawn on illagers, as in vanilla (no armour layer); skulls on an illager's head are left out.
  - Group pillagers' decision "a patrol leader rolled at spawn gets `PatrolLeader` but no ominous banner yet" is replaced: every leader, rolled or patrolling, carries the banner.
- **Test steps (survival, Normal):**
  1. In creative, on open ground, use pillager eggs one after another: about one pillager in 17 hatches as a captain with a white banner standing up from its head (the banner shows plain white; the fork draws no banner patterns yet) (**127_pillager_captain**). `/gamemode survival`.
  2. (D4 step 3.) Kill the captain: it drops the banner ("Ominous Banner") and an ominous bottle; the plain pillagers drop no bottle (**128_captain_bottle**).
  3. Natural patrols come from the fifth day on: in a new world run `/time add 120000`. Stand in open land at least 60 blocks from any village, `/time set day`, save and quit and load (the spawner looks once right after loading), then run `/tick sprint 1d` a few times. A patrol of two to four pillagers appears 24 to 47 blocks away with the captain in front; they walk off together (**129_patrol**). Walk up to them: they stop and call each other at about 10 blocks, then shoot.
  4. `/gamerule spawn_patrols false`: no more patrols come. `/gamerule spawn_patrols true` again.
  5. Friend window: the friend sees the captain's banner and the patrol walking (**130_friend_patrol**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:pillager_spawn_egg*32,minecraft:shield,minecraft:diamond_sword,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:spyglass
```

### 2. Minecraft vindicators

After d876d0b3.

- **Vanilla (javap, 26.3, and the 26.3 data pack):**
  - `EntityTypes.VINDICATOR`: `MobCategory.MONSTER`, `sized(0.6, 1.95)`, `passengerAttachments(2.0)`, `ridingOffset(-0.6)`, `clientTrackingRange(8)`, `notInPeaceful`. `SpawnPlacements`: `NO_RESTRICTIONS`, `MOTION_BLOCKING_NO_LEAVES`, `Monster.checkMonsterSpawnRules`; no biome lists it, and `PatrolSpawner` makes only pillagers (`EntityTypes.PILLAGER`).
  - `Vindicator.createAttributes`: movement speed 0.35 (`0.3499999940395355`), follow range 12, max health 24, attack damage 5. `getWalkTargetValue` is `Monster`'s (no override).
  - Goals by registration (after the raider goals of group pillagers' slots: 4 `LongDistancePatrolGoal`, 1 `ObtainRaidLeaderBannerGoal`, 3 `PathfindToRaidGoal`, 4 `RaiderMoveThroughVillageGoal`, 5 `RaiderCelebration`): 0 `FloatGoal`, 1 `AvoidEntityGoal(Creaking, 8, 1.0, 1.2)`, 2 `VindicatorBreakDoorGoal` (`BreakDoorGoal(6, Normal or Hard)`, MOVE, only with an active raid and a `nextInt(reducedTickDelay(10))` roll), 3 `AbstractIllager.RaiderOpenDoorGoal` (`OpenDoorGoal(false)`, only with an active raid), 4 `Raider.HoldGroundAttackGoal(10)`, 5 `MeleeAttackGoal(1.0, false)`, 8 `RandomStrollGoal(0.6)`, 9 `LookAtPlayerGoal(Player, 3, 1.0)`, 10 `LookAtPlayerGoal(Mob, 8)`. Targets: 1 `HurtByTargetGoal(Raider ignored).setAlertOthers()`, 2 `NearestAttackableTargetGoal(Player, true)`, 3 `(AbstractVillager, true)`, 3 `(IronGolem, true)`, 4 `VindicatorJohnnyAttackGoal` (`NearestAttackableTargetGoal(LivingEntity, 0, true, true, attackable)`, only while `isJohnny`, its `start` resets the idle clock).
  - `customServerAiStep`: with ground navigation, `setCanOpenDoors(level.isRaided(blockPosition()))` every tick. `finalizeSpawn`: `super` (Raider: `CanJoinRaid` for every raider but a natural witch; `PatrollingMonster`: the 6 % leader roll and the banner; `Mob`: follow range bonus, left hand), then `setCanOpenDoors(true)`, `populateDefaultEquipmentSlots` (an iron axe when not in a raid) and `populateDefaultEquipmentEnchantments`. `applyRaidBuffs` (raids only): an iron axe enchanted from `raid/vindicator` or `raid/vindicator_post_wave_5`.
  - `setCustomName`: a name whose string is exactly "Johnny" makes it Johnny for good; `Johnny` is saved only when true. `Entity.load` reads `CustomName` through `setCustomName`, so a saved or summoned "Johnny" counts too.
  - `getArmPose`: `ATTACKING` while aggressive, `CELEBRATING` while celebrating, else `CROSSED`. `AbstractIllager.canAttack` spares baby villagers; `considersEntityAsAlly` makes every `#illager_friends` (`#illager`: evoker, illusioner, pillager, vindicator) without a team an ally, which no combat targeting picks.
  - `Mob.doHurtTarget`: the attack damage attribute (5 plus the iron axe's 8, so 13 before difficulty), the weapon's blocking disable (an axe: 5 seconds), no attack sound. Sounds `entity.vindicator.ambient`, `.hurt`, `.death`, `.celebrate`.
  - `VindicatorRenderer`: `IllagerModel` (`ModelLayers.VINDICATOR`, the same 0.9375 mesh as the pillager), `textures/entity/illager/vindicator.png`, shadow 0.5, and an `ItemInHandLayer` that draws only while aggressive.
  - `WoodlandMansionPieces` data markers "Warrior": a vindicator, `setPersistenceRequired`, placed at the marker's block centre and `finalizeSpawn(STRUCTURE)` (no leader roll).
  - `loot_table/entities/vindicator.json`: an emerald (0 to 1, looting) on a player's kill.
- **Changed:**
  - Vendored `entities/src/illager.rs`: `Illager::vindicator` with the iron axe, the `johnny` flag and `set_custom_name`, the vindicator's arm poses, `shows_held_items` (only an aggressive vindicator shows its axe), `is_illager`. `illager_ai.rs`: `vindicator_goals` and `vindicator_targets` in vanilla's slots on the shared `raider_goals`, `JohnnyAttackGoal`; the two door goals hold their slots but cannot start without a raid (group `raids`). `monster_ai.rs`: `MonsterKind::Vindicator` (floats, follow range 12, the monster walk value), `Prey::Living` (every player, villager and mob but itself), `NearestTargetGoal` with interval 0 draws no roll, illagers spare baby villagers and never pick an illager. `world/illagers.rs`: the vindicator in the store with its own goals, its melee strike (attack damage plus the weapon's, the weapon's shield disable) on players, villagers and mobs, melee aggression as the one aggressive flag, Johnny's idle reset.
  - Vendored `world/src/natural_spawner.rs` (`type_info` for the vindicator: no restrictions, monster rules, no biome spawns), `natural_spawner/spawning.rs` (`finalize_vindicator` for summons, eggs and spawners on the shared `finalize_raider`, the monster category, follow range 12), `generator/src/feature/entities.rs` (mansion vindicators: `CanJoinRaid`, the iron axe and the enchantment draw).
  - `minecraft_terrain`: `server_mobs.rs` (`SIMULATED_KINDS`, the `spawn_saved` arm), `illager_save.rs` (`Johnny`, and a `CustomName` of "Johnny"), `illager_render.rs` (hand items only when shown), `client_mobs.rs` (the illagers' melee swings), `spawner_display.rs` (vindicators turn in spawner cages), `server.rs` (`HATCHING_EGGS` grows to 46). Friends see the crossed arms, the raised axe and the swing through the existing `MobShow.arm_pose`, `aggressive` and `ServerMob.swings`; MW2 bullets hit vindicators through hit key 32.
- **Tests:**
  - Vendored entities: `illager::tests::a_vindicator_holds_an_iron_axe_crosses_its_arms_and_raises_them_to_attack`, `only_a_vindicator_named_johnny_becomes_johnny`; `world::illagers::tests::a_vindicator_is_a_live_monster_with_an_iron_axe_that_golems_hunt`, `a_vindicator_raises_its_axe_and_strikes_a_player_through_a_shield`, `a_vindicator_named_johnny_attacks_a_cow_but_spares_other_illagers`.
  - Vendored world: `natural_spawner::spawning::tests::a_summoned_vindicator_carries_an_iron_axe_and_sometimes_leads_with_the_banner`.
  - `minecraft_terrain`: `terrain::tests::woodland_mansions_hold_vindicators_with_iron_axes` (seed 77: the nearest mansion's 14 warriors are persistent, carry iron axes, can join raids, never lead, and come alive), `server_mobs::tests::a_vindicator_saves_and_loads_with_its_axe_and_stays_johnny`, `mob_tracking::tests::a_friend_sees_a_vindicator_cross_its_arms_and_then_raise_its_axe`, the spawner cage test with the vindicator, and `every_spawn_egg_of_a_mob_the_game_has_hatches_that_mob` now hatches 46 eggs.
  - Runs: `minecraftoss-entities` lib 324, `minecraftoss-world` lib 93, `minecraftoss-generator` lib 30, `minecraft_terrain` lib 1300 of 1303 (3 ignored), `minecraft_survival` lib 715, all green. `check -p launcher` green. Clippy on `minecraftoss-entities`, `minecraftoss-world`, `minecraftoss-generator`, `minecraft_terrain` (`--all-targets`) and `minecraftoss-player` (lib), with `-A clippy::approx_constant`, finds nothing in this group's lines. No new files.
- **Decisions:**
  - The vindicator joins the `illagers` store (hit key 32) with its own `MonsterKind`; evokers still wait dormant (group `evokers-vexes`).
  - Vindicators never spawn on their own and patrols stay pillagers only, as in 26.3 (the brief's "patrols" for vindicators does not hold); they come from mansions, eggs, spawners and, later, raids.
  - Door breaking and door opening only act in raids in vanilla: their goal slots wait for group `raids` (the `has_raid` flag the goals read is always false until then), and outside raids vindicators path without doors, as `customServerAiStep` makes them.
  - Johnny comes only from a saved or summoned name or the `Johnny` tag: the fork has no name tags yet. `/summon` with NBT runs no `finalizeSpawn`, so a summoned Johnny needs its axe in the NBT, as in vanilla.
  - The iron axe's damage comes from the item catalog's attack attributes in the game (13 on Normal before difficulty); the unit tests have no catalog and see the base 5. The axe disables a shield for 5 seconds.
  - Melee aggression and hold-ground aggression are one flag, as vanilla's `Mob.setAggressive`: the melee goal sets and clears it when it starts and stops.
  - Enchanted spawn equipment stays unported (the roll is drawn and noted), like every other mob's; the raid buffs come with raids.
- **Test steps (survival, Normal):**
  1. (D4 step 2.) Use vindicator eggs on open ground: each vindicator stands with its arms crossed and no axe in sight (**131_vindicator_calm**). Stay close: it runs at you, raises both arms with the iron axe and strikes hard; hold up the shield and the axe knocks it out of use for five seconds (**132_vindicator_axe**). A kill leaves experience and now and then an emerald.
  2. `/gamemode creative`. A villager egg and an iron golem egg about 10 blocks from a vindicator egg: the vindicator goes for the adult villager (never a baby), the golem goes for the vindicator, and a wandering trader from its egg runs away from it. `/gamemode survival`.
  3. Johnny: put three cow eggs and a pillager egg down, then `/summon minecraft:vindicator ~ ~ ~4 {CustomName:"Johnny",equipment:{mainhand:{id:"minecraft:iron_axe",count:1}}}`. Johnny attacks the cows one after another and leaves the pillager alone; a vindicator from the egg ignores the cows (**133_johnny**).
  4. (D4 step 3.) About one vindicator egg in 17 hatches as a captain with the white banner on its head; kill it: an ominous bottle drops.
  5. `/locate structure minecraft:mansion`, `/tp` there and go in: vindicators with axes stand in the rooms and stay when you walk away and back (**134_mansion_vindicators**). Evokers there still do nothing (group `evokers-vexes`).
  6. Save and quit, then load: the vindicators are still there, and Johnny still hunts the cows.
  7. Friend window: the friend sees the crossed arms, the raised axe and the swing, gets hit, and shoots vindicators with a gun (**135_friend_vindicator**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:vindicator_spawn_egg*16,minecraft:pillager_spawn_egg*16,minecraft:cow_spawn_egg*8,minecraft:villager_spawn_egg*4,minecraft:iron_golem_spawn_egg*2,minecraft:wandering_trader_spawn_egg*2,minecraft:shield,minecraft:diamond_sword+sharpness=5,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:totem_of_undying,minecraft:ominous_bottle*4,minecraft:spyglass
```

## Group evokers-vexes

### 1. Minecraft evokers cast fangs and summon vexes

After 60ff4ce4.

- **Vanilla (javap, 26.3, and the 26.3 data pack):**
  - `EntityTypes.EVOKER`: `MobCategory.MONSTER`, `sized(0.6, 1.95)`, `passengerAttachments(2.0)`, `ridingOffset(-0.6)`, `clientTrackingRange(8)`, `notInPeaceful`. `SpawnPlacements`: `NO_RESTRICTIONS`, `MOTION_BLOCKING_NO_LEAVES`, the monster rules (as the vindicator); no biome lists it. `EntityTypes.EVOKER_FANGS`: `MobCategory.MISC`, `noLootTable`, `sized(0.5, 0.8)`, `clientTrackingRange(6)`, `updateInterval(2)`.
  - `Evoker`: `xpReward` 10; `createAttributes`: movement speed 0.5, follow range 12, max health 24. Goals by registration (after the raider goals): 0 `FloatGoal`, 1 `EvokerCastingSpellGoal`, 2 `AvoidEntityGoal(Player, 8, 0.6, 1.0)`, 3 `AvoidEntityGoal(Creaking, 8, 0.6, 1.0)`, 4 `EvokerSummonSpellGoal`, 5 `EvokerAttackSpellGoal`, 6 `EvokerWololoSpellGoal`, 8 `RandomStrollGoal(0.6)`, 9 `LookAtPlayerGoal(Player, 3, 1.0)`, 10 `LookAtPlayerGoal(Mob, 8)`. Targets: 1 `HurtByTargetGoal(Raider ignored).setAlertOthers()`, 2 `NearestAttackableTargetGoal(Player, true).setUnseenMemoryTicks(300)`, 3 `(AbstractVillager, false).setUnseenMemoryTicks(300)`, 3 `(IronGolem, false)`. `considersEntityAsAlly`: itself, `#illager_friends` (the four illagers) and any vex whose root owner is it or its ally. Sounds `entity.evoker.ambient`, `.hurt`, `.death`, `.cast_spell`, `.celebrate`; `applyRaidBuffs` does nothing.
  - `SpellcasterIllager`: saves `SpellTicks` (`spellCastingTickCount`), syncs the spell id as a byte (`DATA_SPELL_CASTING_ID`), counts the ticks down in `customServerAiStep` (after the goals); `isCastingSpell` is ticks above 0 on the server and the synced id above 0 on clients; `getArmPose` is `SPELLCASTING` while casting, `CELEBRATING` while celebrating, else `CROSSED`. Each client tick while casting two `ENTITY_EFFECT` particles of the spell's colour rise at the shoulders: `f = yBodyRot * 0.017453292 + cos(tickCount * 0.6662) * 0.25`, at `x ± cos(f) * 0.6`, `y + 1.8`, `z ± sin(f) * 0.6`. `IllagerSpell` ids and colours: none 0, summon vex 1 (0.7, 0.7, 0.8), fangs 2 (0.4, 0.3, 0.35), wololo 3 (0.7, 0.5, 0.2), disappear 4, blindness 5.
  - `SpellcasterCastingSpellGoal` (MOVE and LOOK): runs while casting, stops the navigation, looks at the target (the wololo sheep for the evoker) and sets the spell back to none when it stops. `SpellcasterUseSpellGoal` (no flags, so it runs beside the others): usable with a living target, not casting and `tickCount` past its next tick; on start the warmup is `adjustedTickDelay(castWarmupTime)` (20, so 10 goal ticks), the casting time and the next tick (`tickCount + interval`) are set, the prepare sound plays and the spell is set; it ticks the warmup down and at 0 casts and plays `entity.evoker.cast_spell`; it continues while the target lives and the warmup is above 0. Fangs: casting 40, interval 100, `entity.evoker.prepare_attack`. Summon: casting 100, interval 340, `entity.evoker.prepare_summon`, usable when `nextInt(8) + 1` beats the vexes within 16 (`forNonCombat().range(16).ignoreLineOfSight().ignoreInvisibilityTesting()`), three vexes at the evoker's block plus `(-2 + nextInt(5), 1, -2 + nextInt(5))`, each `MOB_SUMMONED` with its owner, bound origin and `setLimitedLife(20 * (30 + nextInt(90)))`. Wololo: warmup 40, casting 60, interval 140, `entity.evoker.prepare_wololo`, only without a target and with `mob_griefing`, a random sheep of `getNearbyEntities(Sheep, forNonCombat().range(16)` with blue wool, box grown 16, 4, 16), which turns red.
  - `EvokerAttackSpellGoal.performSpellCasting`: `minY = min(target.y, y)`, `maxY = max(target.y, y) + 1`, `f = atan2(target.z - z, target.x - x)`; within 3 blocks (distance squared under 9) five fangs at radius 1.5 at `f + i * π * 0.4` with warmup 0 and eight at radius 2.5 at `f + i * π * 2 / 8 + 1.2566371` with warmup 3, else sixteen in a line at `1.25 * (i + 1)` with warmup `i`. `createSpellEntity` searches down from `(x, maxY, z)` to `floor(minY) - 1` for a block whose top face is sturdy below a spot, standing on the spot's collision top (0 for air), and spawns `EvokerFangs(x, y, z, f, warmup, evoker)` (yaw `f * 57.295776`).
  - `EvokerFangs.tick` (server): the warmup counts down; below 0, at -8 it hurts every living entity in its box grown 0.2 across (`dealDamageTo`: alive, not invulnerable, not its owner nor an ally of it; 6 `indirectMagic` from the owner, or plain `magic` without one); the first time it sends entity event 4; then `lifeTicks` (22) counts down and below 0 it is gone. Event 4 on the client starts the bite and plays `entity.evoker_fangs.attack` (volume 1, pitch `0.85 + nextFloat * 0.2`); the client counts its own `lifeTicks` and at 14 throws 12 `CRIT` particles; `getAnimationProgress` is `1 - (lifeTicks - 2 - partial) / 20` (1 at 2 and below). Saves `Warmup` and `Owner`.
  - `EvokerFangsModel` (64 x 32): `base` (0, 0) box (0, 0, 0) 10 x 12 x 10 at (-5, 24, -5), with `upper_jaw` and `lower_jaw` (40, 0) boxes (0, 0, 0) 4 x 14 x 8 at (6.5, 0, 1) rolled 2.042035 and at (3.5, 0, 9) turned π and rolled 4.2411504. `setupAnim(bite)`: `f = 1 - min(bite * 2, 1)³`, the upper jaw rolls `π - f * 0.35π`, the lower `π + f * 0.35π`, the base rises `(bite + sin(bite * 2.7)) * 7.2` pixels, and above 0.9 the whole model shrinks to the ground (`root.y = 24 - 20s`, scale `s = (1 - bite) / 0.1`). `EvokerFangsRenderer`: nothing at 0, else turned `90 - yRot`, flipped and lowered 1.501, `textures/entity/illager/evoker_fangs.png`, no shadow.
  - `EvokerRenderer`: the illager model (`ModelLayers.EVOKER`, 0.9375), `textures/entity/illager/evoker.png`, shadow 0.5, and an `ItemInHandLayer` drawn only while casting.
  - `loot_table/entities/evoker.json`: always a totem of undying, and on a player's kill 0 to 1 emerald (looting). `WoodlandMansionPieces` data markers "Mage": an evoker, persistent, `finalizeSpawn(STRUCTURE)`.
- **Changed:**
  - New vendored `entities/src/evoker_fangs.rs` (`EvokerFangs`: warmup, life, the spike, owner by id or UUID, the bite at -8, the bite progress for drawing; `spots` for the line and the two rings; `floor_at` for `createSpellEntity`'s ground search) and `entities/src/world/evoker_fangs.rs` (the `evoker_fangs` store with its tick: the bite on players as a new `PlayerHitKind::Magic` and on mobs through the potions' indirect magic, sparing the owner and, for an illager owner, every illager; the attack sound; `cast_fangs`).
  - Vendored `entities/src/illager.rs`: `IllagerSpell` with its ids and colours, `Illager::evoker`, the spell and its ticks, the evoker's arm poses (spellcasting while a spell shows), its hand items only while casting, `experience` (10 for the evoker). `illager_ai.rs`: `evoker_goals` and `evoker_targets` in vanilla's slots, `CastingSpellGoal`, one `SpellGoal` for the three spells (warmups, casting times, intervals, prepare and cast sounds), the player target's 150-tick unseen memory, `SpellCast`. `monster_ai.rs`: `MonsterKind::Evoker`, the evoker spares baby villagers and illagers, `AvoidEntityGoal` also flees players. `world/illagers.rs`: the evoker in the store with its own goals, the spell ticks counted down after the goals, the blue sheep around it for wololo, the casts (fangs, wololo; the summon spell casts with its sounds and particles but its vexes come with commit 2). `world.rs` and `world/{pushing,riding,shulker_bullets,equines,witches}.rs`: the fangs in every walk over the entities (`EntityKey::EvokerFangs`), the evoker's 10 experience. `bin/check_zombie_player_hits.rs` takes the new hit kind.
  - Vendored `world/src/natural_spawner.rs` (`type_info` for the evoker: no restrictions, monster rules, no biome spawns), `natural_spawner/spawning.rs` (summons, eggs and spawners: `CanJoinRaid`, the 6 % leader with the banner, follow range 12) and `generator/src/feature/entities.rs` (mansion evokers can join raids).
  - `minecraft_terrain`: new `evoker_fangs_render.rs` (`EvokerFangsModel` and its renderer); `server_mobs.rs` (`SIMULATED_KINDS`, the evoker's and the fangs' `spawn_saved` arms, the fangs saved with `Warmup`, `Owner` and `UUID`), `illager_save.rs` (`SpellTicks`), `mob_tracking.rs` (fangs tracked at 6 chunks as a projectile with their warmup and life in the new `MobShow.bite`, the spell id in the new `MobShow.spell`, `SpellTicks` and `Warmup` volatile), `mesh.rs` (the fangs texture), `spawner_display.rs` (evokers turn in spawner cages), `server.rs` (`HATCHING_EGGS` grows to 47), `lib.rs`.
  - `minecraft_survival`: `entity_view.rs` (the fangs drawn, the spell particles of every casting evoker and the fangs' crit burst, for host and friend alike), `entities.rs` (a fangs bite is the evoker's indirect magic, no shield blocks it; without an owner it is plain magic), `death.rs` (`MobAttack::Magic`: `minecraft:indirect_magic`, "was killed by ... using magic"; magic goes past armor), `survival.rs` (armor is skipped for damage that bypasses it).
- **Tests:**
  - Vendored entities: `evoker_fangs::tests` (`fangs_bite_eight_ticks_after_their_warmup_and_go_23_ticks_after_it`, `the_jaws_rise_over_twenty_ticks_once_the_spike_is_sent`, `far_off_a_line_of_sixteen_runs_at_the_target_close_in_two_rings`, `a_fang_stands_on_the_ground_found_below_its_top`), `illager::tests::an_evoker_crosses_its_arms_and_raises_them_while_it_casts`, `world::illagers::tests` (`an_evoker_is_a_live_monster_of_its_size_and_health` (with 10 experience), `an_evoker_casts_a_line_of_fangs_at_a_player_in_range` (the summon and attack sounds, the spell pose, 16 or 13 fangs on the ground owned by it), `fangs_bite_a_player_once_and_a_cow_but_spare_illagers` (one 6-damage magic hit, one attack sound, a pillager untouched, the fangs gone), `wololo_turns_only_a_blue_sheep_red_and_only_with_mob_griefing`), `loot::tests::an_evoker_always_drops_a_totem_of_undying`.
  - Vendored world: `natural_spawner::spawning::tests::a_summoned_evoker_holds_nothing_can_join_raids_and_sometimes_leads`.
  - `minecraft_terrain`: `evoker_fangs_render::tests` (`the_fangs_rise_out_of_the_ground_and_sink_back`, `the_jaws_open_wide_then_snap_shut`), `server_mobs::tests::an_evoker_saves_and_loads_mid_spell`, `evoker_fangs_save_and_load_with_their_warmup_and_owner`, `mob_tracking::tests::a_friend_sees_an_evoker_cast_and_its_fangs_bite`, `terrain::tests::woodland_mansions_hold_vindicators_with_iron_axes_and_evokers` (seed 77: the mansion's evokers are persistent, can join raids and come alive), the spawner cage test with the evoker, and the egg survey now hatches 47 eggs.
  - `minecraft_survival`: `entities::tests::evoker_fangs_bite_past_shields_as_the_evokers_indirect_magic`, `survival::tests::magic_from_evoker_fangs_goes_past_armor`.
  - Runs: `minecraftoss-entities` lib 334, `minecraftoss-world` lib 94, `minecraftoss-generator` lib 30, `minecraft_terrain` lib 1305 of 1308 (3 ignored), `minecraft_survival` lib 717, all green. `check -p launcher` green. The three new files pass `rustfmt --check --edition 2021`.
- **Decisions:**
  - The evoker joins the `illagers` store (hit key 32) with its own `MonsterKind`; the fangs are a level entity in their own `evoker_fangs` store, drawn and tracked like a projectile (no hit key: vanilla fangs cannot be hit).
  - The three spell goals are one `SpellGoal` with the spell's numbers; they take no controls, as vanilla's, so they run beside the avoid and stroll goals, and they tick on the full goal ticks only (the warmups are halved as `adjustedTickDelay` halves them).
  - Commit 1 casts the summon spell (pose, particles and sounds) without vexes: the vex comes with commit 2, which makes the cast spawn them.
  - Friends see the spell through `MobShow.spell` (vanilla's synced byte) and the bite through `MobShow.bite` (warmup and life), so their fangs open and close with the host's; the fangs' sound comes from the host, as every other mob sound does, at the pitch the host draws.
  - The fangs' bite on a player is the new `PlayerHitKind::Magic`: 6 indirect magic from the evoker, scaled by difficulty as vanilla scales a living source's, past shields and armor (`#bypasses_shield` and `#bypasses_armor` hold `indirect_magic`). Fangs whose evoker is gone do plain magic to everyone.
  - Wololo picks among the blue sheep in vanilla's box that the evoker sees within 16 blocks, from the level's sheep in their store order. A wololo'd sheep shows red to friends through its look tag (`Color`).
  - The fangs give no knockback: the fork's indirect magic (splash potions too) pushes nothing yet.
- **Test steps (survival, Normal):**
  1. (D4 step 2.) In creative, put an evoker egg on open ground in daylight: the evoker stands with crossed arms (**136_evoker**). `/gamemode survival` and walk to about 10 blocks: it backs away from you, raises both arms with grey-blue swirls and the summon sound (its vexes come with the next commit), and five seconds later raises them again with dark swirls: a line of fangs bursts out of the ground towards you, the jaws snap shut and sink back (**137_evoker_fangs_line**). A bite takes 3 hearts even through diamond armour, and a raised shield does not stop it.
  2. Get within 3 blocks: the next cast puts a ring of fangs around the evoker (**138_evoker_fang_ring**).
  3. In creative, far from the evoker (or with no player in reach), `/summon minecraft:sheep ~5 ~ ~ {Color:11b}` and `/summon minecraft:sheep ~-5 ~ ~`: the evoker looks at the blue sheep, raises its arms with orange swirls and the "wololo" sound, and the blue sheep turns red; the white one stays white (**139_wololo**). `/gamerule mob_griefing false` and summon another blue sheep: it stays blue. `/gamerule mob_griefing true`.
  4. Kill an evoker with the sword: it always drops a totem of undying, now and then an emerald, and 10 experience (**140_evoker_totem**).
  5. An iron golem from its egg walks to an evoker and strikes it; a villager runs away from it.
  6. `/locate structure minecraft:mansion` and go in: evokers stand in some rooms and come alive when you are near.
  7. Save and quit in the middle of a cast, then load: the evoker is still there and casts again.
  8. Friend window: the friend sees the raised arms, the swirls, the fangs coming out of the ground and closing, gets bitten, and shoots evokers with a gun (**141_friend_evoker**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:evoker_spawn_egg*16,minecraft:sheep_spawn_egg*4,minecraft:iron_golem_spawn_egg*2,minecraft:villager_spawn_egg*4,minecraft:shield,minecraft:diamond_sword+sharpness=5,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:spyglass
```

### 2. Minecraft vexes fly through walls and charge

After 03c34c19.

- **Vanilla (javap, 26.3, and the 26.3 data pack):**
  - `EntityTypes.VEX`: `MobCategory.MONSTER`, `fireImmune`, `sized(0.4, 0.8)`, `eyeHeight(0.51875)`, `passengerAttachments(0.7375)`, `ridingOffset(0.04)`, `clientTrackingRange(8)`, `notInPeaceful`. `SpawnPlacements`: `NO_RESTRICTIONS`, `MOTION_BLOCKING_NO_LEAVES`, the monster rules; no biome lists it.
  - `Vex`: `xpReward` 3; `createAttributes`: `Monster`'s with max health 14 and attack damage 4 (follow range 16). Each tick it sets `noPhysics` around `Monster.tick` (it moves through blocks, pushes and is pushed by nothing, is never in a wall) and `setNoGravity(true)`; with a limited life `limitedLifeTicks` counts down and at 0 or below it is set to 20 and the vex takes 1 `starve` damage. Saves `bound_pos`, `life_ticks` (only with a limited life) and `owner`. `finalizeSpawn`: an iron sword in the main hand with drop chance 0, the enchantment roll, then `Monster`'s part. Sounds `entity.vex.ambient`, `.hurt`, `.death`, `.charge`. `getLightLevelDependentMagicValue` 1. Loot: `loot_table/entities/vex.json` has no pools.
  - Goals: 0 `FloatGoal`, 4 `VexChargeAttackGoal` (MOVE, every tick: with a living target, the move control idle, `nextInt(reducedTickDelay(7)) == 0` and the target beyond 2 blocks, it flies at the target's eyes at speed 1, sets charging and plays `entity.vex.charge`; while the boxes meet it attacks (`doHurtTarget`) and stops charging, within 3 blocks it re-aims at the eyes; it continues while the move control has a target, it charges and the target lives), 8 `VexRandomMoveGoal` (MOVE, one tick: with the move control idle and `nextInt(4) == 0`, up to three spots `bound origin (or its block) + (nextInt(15) - 7, nextInt(11) - 5, nextInt(15) - 7)`, the first empty one's centre at speed 0.25, looked at (180, 20) without a target), 9 `LookAtPlayerGoal(Player, 3, 1.0)`, 10 `LookAtPlayerGoal(Mob, 8)`. Targets: 1 `HurtByTargetGoal(Raider ignored).setAlertOthers()`, 2 `VexCopyOwnerTargetGoal` (the owner's target while the vex may attack it, no sight needed), 3 `NearestAttackableTargetGoal(Player, true)`.
  - `VexMoveControl.tick`: only for `MOVE_TO`; with `d` the distance to the wanted spot, below the box's size (`AABB.getSize`, (0.4 + 0.8 + 0.4) / 3) it waits and halves its velocity, else it adds `delta * (speedModifier * 0.05 / d)` to the velocity and faces it (`-atan2(dx, dz)` in degrees, body too), or faces its target when it has one. `LivingEntity.travelInAir` with no input and no gravity then moves it by its velocity and keeps 0.91 of it across and 0.98 up and down.
  - `VexModel` (32 x 32): `root` at (0, -2.5, 0) holding `head` (0, 0) box (-2.5, -5, -2.5) 5 x 5 x 5 at (0, 20, 0) and `body` at (0, 20, 0) with (0, 10) box (-1.5, 0, -1) 3 x 4 x 2 and (0, 16) box (-1.5, 1, -1) 3 x 5 x 2 shrunk 0.2; on the body `right_arm` (23, 0) box (-1.25, -0.5, -1) 2 x 4 x 2 shrunk 0.1 at (-1.75, 0.25, 0), `left_arm` (23, 6) box (-0.75, -0.5, -1) at (1.75, 0.25, 0), `left_wing` (16, 14) mirrored flat box (0, 0, 0) 0 x 5 x 8 at (0.5, 1, 1) and `right_wing` the same unmirrored at (-0.5, 1, 1). `setupAnim`: the head turns and pitches; `f = cos(age * 5.5°) * 0.1`, the arms roll `±(0.62831855 + f)`; charging, the body stands straight and `setArmsCharging` raises the arm holding an item (x 3.6651914, y ±0.2617994, roll `∓0.47123888 ∓ f`) or with empty hands holds both at x -1.2217305; otherwise the body leans 0.15707964; the wings turn `±(1.0995574 + cos(age * 45.836624°) * 16.2°)` and tilt 0.47123888. `translateToHand`: root, body and arm, scale 0.55, then `(±0.046875, -0.15625, 0.078125)`.
  - `VexRenderer`: shadow 0.3, block light always 15, `textures/entity/illager/vex.png` or `vex_charging.png` while charging, and an `ItemInHandLayer`.
- **Changed:**
  - New vendored `entities/src/vex.rs` (`Vex`: size, health, the iron sword it never drops, owner by id or UUID, bound origin, limited life and its once-a-second hurt, charging; `steer` and `yaw_towards` for the move control), `entities/src/vex_ai.rs` (`VexState` and the goal and target lists: `ChargeAttackGoal`, `RandomMoveGoal`, `CopyOwnerTargetGoal`, the raider-sparing hurt-by goal with its alert and the look goals of the illagers) and `entities/src/world/vexes.rs` (the `vexes` store: `VexEntity`, `spawn_vex`, its tick with the move control and the drag but no collisions, gravity, pushing, suffocation or fall damage, the limited life, the strike with its sword, the alert to other vexes; `summon_vexes` for the evoker's spell: three vexes at its block plus `(-2..2, 1, -2..2)` with a life of 30 to 119 seconds from the evoker's random; `vexes_near` for the spell's count).
  - Vendored `entities/src/world/illagers.rs` (the summon spell counts the vexes within 16 and spawns three), `world/evoker_fangs.rs` (fangs spare vexes whose owner is the fangs' owner or an illager), `monster_ai.rs` (`MonsterKind::Vex`, vexes are enemies, the `vex` state), `illager_ai.rs` (the raider hurt-by and look goals are shared), `world.rs` and `world/{combat,hazards,pushing,riding,villagers,witches}.rs` (the store in every walk over the mobs, `EntityKey::Vex` and `MobHit::Vex`, its 3 experience plus the sword's, fire immunity), `lib.rs`.
  - Vendored `world/src/natural_spawner.rs` (`type_info` for the vex: 0.4 x 0.8, no restrictions, fire immune, monster rules) and `natural_spawner/spawning.rs` (summons, eggs and spawners: the iron sword with drop chance 0, the enchantment roll, `Mob`'s part).
  - `minecraft_terrain`: new `vex_save.rs` (a vex from its tag and its fields: `bound_pos`, `life_ticks`, `owner`, equipment and drop chances) and `vex_render.rs` (`VexModel` with both textures, full block light, the sword in hand by `translateToHand`, shadow 0.3); `server_mobs.rs` (`SIMULATED_KINDS`, `spawn_saved`, `mob_tags_where`), `mob_tracking.rs` (vexes tracked at 8 chunks, charging through `MobShow.charging`, `life_ticks` volatile), `client_mobs.rs` (the pose push, the swing and the shadow), `mesh.rs` (the two textures), `spawner_display.rs` (vexes in spawner cages), `server.rs` (`HATCHING_EGGS` grows to 48), `lib.rs`.
  - `minecraft_survival`: `entities.rs` (hit key 33 for `MobHit::Vex`), `entity_view.rs` (vexes drawn with their swords and hit by MW2 bullets).
- **Tests:**
  - Vendored entities: `vex::tests` (`a_vex_is_small_floats_and_carries_an_iron_sword_it_never_drops`, `a_summoned_vex_starts_hurting_itself_once_a_second_when_its_life_runs_out`, `it_steers_towards_where_it_wants_until_it_is_there`), `world::vexes::tests` (`a_vex_is_a_live_monster_of_its_size_and_health` (no sword drop, 4 to 6 experience), `a_vex_flies_through_stone_hovers_and_charges_a_player` (charging, the charge sound, a hit owned by it), `an_evoker_summons_three_vexes_that_take_its_target` (the summon sound, three vexes owned by the evoker one block up within 2 of it, lives of 600 to 2380 ticks, iron swords, the evoker's target taken, counted by `vexes_near`), `a_vex_wears_out_a_heart_a_second_once_its_life_is_over`, `evoker_fangs_spare_the_vexes_of_illagers`).
  - Vendored world: `natural_spawner::spawning::tests::a_summoned_vex_carries_an_iron_sword_it_never_drops`.
  - `minecraft_terrain`: `vex_render::tests` (`a_vex_floats_above_its_feet_within_its_box`, `charging_raises_the_sword_arm_and_straightens_the_body`, `the_sword_sits_in_the_right_hand`), `server_mobs::tests::a_summoned_vex_saves_and_loads_with_its_owner_origin_and_life`, `mob_tracking::tests::a_friend_sees_a_vex_charge`, the spawner cage test with the vex, and the egg survey now hatches 48 eggs.
  - `minecraft_survival`: `entities::tests::a_vex_hit_box_encodes_and_decodes`.
  - Runs: `minecraftoss-entities` lib 342, `minecraftoss-world` lib 95, `minecraft_terrain` lib 1310 of 1313 (3 ignored), `minecraft_survival` lib 718, all green. `check -p launcher` green. Clippy on `minecraftoss-entities`, `minecraftoss-world`, `minecraft_terrain` and `minecraft_survival` (`--all-targets`, `-A clippy::approx_constant`) finds nothing in this group's code. The five new files pass `rustfmt --check --edition 2021`.
- **Decisions:**
  - The vex has its own `vexes` store (hit key 33): vanilla's `Vex` is a `Monster` of its own, not an illager.
  - The vex keeps no collisions at all, as `noPhysics` makes it: it flies through blocks, pushes nothing and takes no suffocation or fall damage. Its flight ignores water and lava (vanilla would use `travelInFluid` there); it does not drown in the fork.
  - The summon spell draws the vexes' spots and lives from the evoker's own random and their left hand and enchantment rolls from the level random, as `EntityType.create` and `finalizeSpawn` do; the follow range bonus roll is left out (as for every mob made in the entity world).
  - A vex follows its owner's target by the owner's entity id, looked up again from its saved `owner` UUID after a load; a vex whose evoker is gone keeps its own player target goal.
  - Vexes from eggs, spawners and summons have no limited life, as in vanilla.
  - Commit 1's decision on the summon spell is replaced: the cast now brings its three vexes.
- **Test steps (survival, Normal):**
  1. (D4 step 2, the vexes.) Use an evoker egg on open ground and walk to about 10 blocks: the evoker raises its arms with grey-blue swirls and three vexes appear just above it, each with an iron sword (**142_vexes**). They fly straight at you, turn to their charging look as they dive in and strike with the sword; the shield blocks a strike from the front.
  2. Stand behind a wall of blocks: the vexes fly through it and keep attacking (**143_vex_through_wall**).
  3. Kill the evoker: its vexes keep fighting, and after 30 seconds to 2 minutes they lose half a heart every second until they die.
  4. In creative, use a vex egg: that vex flies about its spot for good (it never wears out) (**144_vex_egg**). `/gamemode survival`: it comes for you.
  5. An iron golem from its egg swats vexes; a villager runs away from them.
  6. Save and quit while summoned vexes are alive, then load: they are still there, still follow their evoker's target and still wear out on time.
  7. Friend window: the friend sees the vexes flying with their swords and their charging look, gets struck, and shoots them with a gun (**145_friend_vexes**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:evoker_spawn_egg*16,minecraft:vex_spawn_egg*8,minecraft:iron_golem_spawn_egg*2,minecraft:villager_spawn_egg*4,minecraft:shield,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

## Combined kit

One kit for the owner's test of the whole track (27 stacks). The ancient city steps of group loot-functions keep their own kit (a pickaxe, torches and night vision), and groups pillagers and patrols-vindicators keep their own kits (their last commits).

```
IW4L_MINECRAFT_KIT=minecraft:drowned_spawn_egg*16,minecraft:villager_spawn_egg*8,minecraft:zombie_spawn_egg*8,minecraft:zombie_villager_spawn_egg*4,minecraft:composter,minecraft:emerald*64,minecraft:wheat*64,minecraft:splash_potion@weakness,minecraft:golden_apple*16,minecraft:ominous_bottle[ominous_bottle_amplifier=4]*2,minecraft:milk_bucket,minecraft:cat_spawn_egg*8,minecraft:cod*64,minecraft:horse_spawn_egg*4,minecraft:donkey_spawn_egg*2,minecraft:llama_spawn_egg*4,minecraft:saddle*4,minecraft:golden_carrot*16,minecraft:chest*4,minecraft:lead*16,minecraft:oak_fence*16,minecraft:wandering_trader_spawn_egg*2,minecraft:iron_block*8,minecraft:carved_pumpkin*4,minecraft:snow_block*4,minecraft:shears,minecraft:cooked_beef*32
```

## Protocol

Before the merge of minecraft-survival (tracks A, B and C) into track D, `minecraft_net::PROTOCOL` was 4 (from 3) on this track, bumped once in group trading-friend. Since that merge (`r9-d2-base`) it is 5, the number tracks A, B and C took together: protocol 5 has not been released (release 1 is protocol 4), so no released build shares the number with a different message set. `ToServer::OpenMountInventory` moved to codec index 16, after track A's `CreativeSlot` (13) and track C's `RecipeBookSettings` (14) and `RecipeSeen` (15). Later groups of this track must not bump it again.

## State doc changes

- `docs/MINECRAFT-SURVIVAL.md`: drowned are now saved, loaded, hatched from their egg and seen by friends; a zombie that drowns keeps its equipment and name as a drowned. Drowned spawn naturally in rivers (one in 15 rolls), deep oceans (one in 40, below sea level minus 5) and dripstone cave water in the dark, with a trident (6.25 %) or a fishing rod (3.75 %) and a nautilus shell in the offhand (3 %), which they always drop; a player's kill sometimes drops the worn trident. By day drowned on land walk back into the water; at night they swim up and come out onto the nearest beach, and they stroll in the water. A drowned with a trident raises it over its head and throws it every 2 seconds at its target within sight (8 damage, "was impaled by Drowned"); the thrown tridents cannot be picked up and despawn after a minute. Friends see the raised arm and the tridents. Zombie nautilus jockeys and the trident roll for ocean ruin drowned are missing.
- `docs/MINECRAFT-SURVIVAL.md`: right-clicking an adult villager with a job opens its vanilla trading screen for the host: the offer buttons with their payments, arrows and results, the struck-through old price on a changed price, a crossed arrow on a sold-out offer, the scroller beyond 7 offers (wheel and drag), the experience bar with the coming experience, and the title "<Profession or name> - <Level>". Picking an offer fills the payments from the inventory, the result trades once or, shift-clicked, as often as the items last, and a shift-click moves stacks between inventory and hotbar. Trades give the villager experience, level it up with new offers and Regeneration, and drop experience orbs (3 to 6, 5 more on a level-up). Prices follow reputation and Hero of the Village. The screen closes when the villager is out of reach (8 blocks), dies or sleeps, and the payments return to the inventory (or drop when it is full); dying or leaving the level closes it the same way. Babies shake their heads. Friends cannot trade yet.
- `docs/MINECRAFT-SURVIVAL.md`: friends trade with villagers like the host: the same trading screen, prices, experience, level-ups and closing rules. A villager trades with one player at a time. This replaces "Friends cannot trade yet" above.
- `docs/MULTIPLAYER.md`:
  - `minecraft_net::PROTOCOL` is 5 (round 9: tracks A, B and C, and track D's `MenuKind::Merchant`, `MenuExtras::Merchant`, `NetOffer`, `MenuKind::Mount`, `MenuExtras::Mount` and `ToServer::OpenMountInventory`). The code block at line 162 and the note at line 1415 read 5.
  - Section 20 and the leftover lists at lines 1303 and 1331 drop "trading screens". New wording: "A friend opens a villager's trading screen on the host's level thread. The offers, level, xp, coming xp and picked offer ride in `MenuExtras::Merchant`, and the friend's clicks run on the host."
- `CLAUDE.md` roadmap item 2: drop "trading screens" from the multiplayer leftovers.
- `docs/MINECRAFT-SURVIVAL.md`: a zombie of any kind that kills a villager turns it into a zombie villager: always on Hard, half the time on Normal, never on Easy or Peaceful. The zombie villager keeps the villager's type, profession, level, experience, trades, gossip, name and held item. The villager drops nothing, and a loud groan plays (level event 1026). A zombie villager that was a traded villager never despawns.
- `docs/MINECRAFT-SURVIVAL.md`: the server's mobs now follow the world difficulty in their own rules too. Until round 9 the entity world always ran as Normal. This covers skeleton fire rates and spread, drowned trident spread, husk hunger and the zombie villager conversion.
- `docs/MINECRAFT-SURVIVAL.md`: a zombie villager with Weakness (splash weakness on it) is cured by right-clicking it with a golden apple, which is used up. It shakes for 3 to 5 minutes, faster with iron bars and beds within 4 blocks, keeps curing through a save, and never despawns while curing. Then it becomes the same villager again (type, profession, level, experience, trades, gossip and name), with nausea and a loud sound. The player who cured it gets much lower prices from it. Without Weakness the apple is kept. Friends see the shaking and hear the sounds. Zombies drowning into drowned shake too.
- `docs/MINECRAFT-SURVIVAL.md`: players can carry Darkness, Bad Omen, Raid Omen, Trial Omen and Hero of the Village. They are saved in the player file, `/effect` gives and clears them, and milk and totems remove them. Hero of the Village lowers that player's prices at every villager (30 % of the first cost at level I, 6.25 % more a level). Bad Omen does not yet turn into Raid Omen in a village, Raid Omen does not yet start a raid, Trial Omen is not yet read by trial spawners, and Darkness does not yet darken the screen.
- `docs/MINECRAFT-SURVIVAL.md`: ominous bottles are drunk like potions (1.6 seconds, either hand) and give Bad Omen for 100 minutes at the bottle's level (`minecraft:ominous_bottle_amplifier`, 0 to 4), with the omen sound when it is new and the bottle-break sound. The bottle is used up without a glass bottle. `/give`, `/kit` and the test kit take `minecraft:ominous_bottle[ominous_bottle_amplifier=N]`.
- `docs/MINECRAFT-SURVIVAL.md`: the player's effects show as vanilla icons at the top right of the screen, for the host and for friends: beneficial effects in the top row, harmful and neutral ones (the omens) in the row below, sorted as in vanilla, each blinking and fading in its last 10 seconds. The inventory screens do not yet list the effects beside them.
- `docs/MINECRAFT-SURVIVAL.md`: cats and ocelots are live mobs. Cats come in the 11 vanilla coats (black ones in swamp huts and, at full moon, anywhere) with the two vanilla voices, and village and swamp hut cats move. Ocelots spawn in jungles (on grass or leaves at or above sea level, babies after the first of a group). Both are saved, hatch from their eggs, run from players while wild, creep after a player holding raw cod or salmon (crouching, scared off by quick moves or turns) and hunt (ocelots chase and bite chickens). They are drawn like vanilla with their walking, crouching and sprinting poses. Friends see them and can hit them.

- `docs/MINECRAFT-SURVIVAL.md`: cats are tamed with raw cod or salmon (one try in three) and then belong to that player by UUID: they sit and stand on their owner's click, follow and teleport to their owner, wear a collar their owner can dye, heal from fish, sit on chests, lit furnaces and the foot of beds, relax and lie on their sleeping owner's bed and may leave a morning gift (string, a feather, rabbit hide, a rabbit's foot, raw chicken or rotten flesh) when the owner wakes up at dawn, and breed into tame kittens. Ocelots learn to trust a player who feeds them fish while they creep close. Creepers run from cats and ocelots. Phantoms do not exist yet, and cats sitting on chests do not keep them shut yet.
- `docs/MINECRAFT-SURVIVAL.md`: the Overworld has vanilla's cat spawner: once a minute, while `spawn_mobs` is on, a spot 8 to 31 blocks from a random player gets a cat when it is near a village with more than four claimed beds within 48 blocks and fewer than five cats around (48 by 8), or inside a swamp hut with no cat within 16 blocks (that cat stays for good).
- `docs/MINECRAFT-SURVIVAL.md`: mules, llamas and trader llamas are live mobs: saved, hatched from their eggs and seen by friends; natural llamas in windswept hills and savanna plateaus come alive. A tame horse and a tame donkey in love have a mule (horses and donkeys have their own kind; mules never breed), with health, speed and jump from both parents. Horses, donkeys, mules and llamas eat like vanilla (heal, grow, temper; golden carrots and apples, or hay for llamas, bring love when tame), rear angrily at other items while wild, take a chest when tame (donkeys, mules, llamas) and llamas a carpet, and drop their own loot, the carpet, the chest and its items. A llama hit by a player or mob spits at it once (1 damage, "was slain by Llama"), spits at wild wolves within 4 blocks, and trader llamas spit at zombies and leave after 40 minutes unless tamed or kept. Taming by riding, riding and the horse inventory are not there yet; caravans form once leads exist.
- `docs/MINECRAFT-SURVIVAL.md`: players ride horses, donkeys, mules and llamas. An empty hand mounts any adult one (any other item a tame one); a wild one runs about and either throws the rider off (rearing, angry, its temper up by 5) or is tamed by that player (the chance grows with temper; food raises it). A tame horse, donkey or mule takes a saddle by right-click and is then steered by its rider with the movement keys and the mouse (slower backwards and sideways); holding jump fills the jump bar in place of the experience bar, and letting go rears and jumps the horse as high as the charge and its jump strength allow. Horses take horse armour by right-click. Llamas can be ridden and tamed but never steered. Crouching gets off beside the horse. The saddle, bridle and reins and the horse armour are drawn; saddle, armour and owner are saved; a rider stays mounted through a save. Friends tame and ride their own horses with their own jump bar. Skeleton and zombie jockeys on horses and llamas in saved worlds load with their riders. The horse inventory, the rider's fall damage, the gallop sound and the taming hearts and smoke are not there yet.
- `docs/MINECRAFT-SURVIVAL.md`: tame horses, donkeys, mules and llamas have the vanilla inventory screen: crouch and right-click one, or press the inventory key while riding it. It has the saddle slot (not on llamas), the armour slot (horses: horse armour; llamas: a carpet) and the chest (5 columns on a chested donkey or mule, the llama's strength in columns), with vanilla's shift-click rules; it closes when the mount dies, gets a chest or is more than about 7 blocks away. The chest's items are saved in their slots. Friends open and use it the same way. The mount is not drawn in the screen yet. This replaces "The horse inventory ... not there yet" above.
- `docs/MULTIPLAYER.md`: a friend's horse inventory is a host menu sent as `MenuKind::Mount` with `MenuExtras::Mount`; a friend opening its own inventory sends `ToServer::OpenMountInventory`, and the host opens the horse's when the friend rides a tame one.
- `docs/MULTIPLAYER.md`: a friend's ride keys reach the host through the friend's MW2 commands (the authority keeps each client's last `forwardmove`, `rightmove` and buttons); the host runs the horse and sends the friend's jump bar in `PlayerView.jump_bar`.
- `docs/MINECRAFT-SURVIVAL.md`: llamas and trader llamas are drawn like vanilla: the four coats, babies with their own model, the chests on a chested llama, a carpet of any of the 16 colours on an adult, and the trader's blanket on trader llamas (the small one on foals). Llama spit is drawn in flight. Llamas and trader llamas turn in spawner cages. Friends see all of it. The puff of spit particles is not drawn yet.
- `docs/MINECRAFT-SURVIVAL.md`: leads work like vanilla. A lead on an animal, golem, strider or hoglin (not monsters or villagers, nor angry wolves) ties it to the player, who leads it: within 6 blocks it walks up to 2 blocks behind, beyond that the lead pulls it, and beyond 12 blocks the lead snaps with its sound and drops. Clicking a held mob with anything lets it go (the lead drops), shears cut a mob's leads, and crouching on another mob ties the held mobs to it. Clicking a fence ties the held mobs to a knot on it, and clicking it again takes them back; a broken fence frees them. Leashed mobs never despawn, keep their leash through a save (vanilla's `leash` tag), and llamas on a lead lead caravans. Friends see the ropes and knots and use leads the same way. Knots cannot be punched off yet.
- `docs/MINECRAFT-SURVIVAL.md`: wandering traders are live mobs. The Overworld's wandering trader spawner runs like vanilla: every 20 minutes (a Minecraft day) while `spawn_mobs` and `spawn_wandering_traders` are on, a 25 % chance (50 % and 75 % after misses) and one in ten picks a random player, and a trader appears within 48 blocks of a village bell near that player, or of the player, with two trader llamas tied to it by leads; it stays 40 minutes (longer while someone trades with it) and leaves with its llamas. Its clock is saved in `data/minecraft/wandering_trader.dat`. The trader and its spawn egg (which brings no llamas, as in vanilla) are saved, hatched and seen by friends. It trades two buying, two uncommon and five common offers from vanilla's trade sets in the trading screen without the experience bar, and drops 3 to 6 experience per trade. At night it drinks a potion of invisibility and vanishes (the bottle and bucket stay visible while it drinks), by day it drinks milk and reappears. It flees zombies, zoglins and its attackers, walks to its target and stays near its home, and its llamas spit at whoever hurts it. It does not yet flee illagers (none exist).
- `docs/MINECRAFT-SURVIVAL.md`: in the leads paragraph, "llamas on a lead lead caravans" gains: "and trader llamas follow their wandering trader on leads".
- `docs/MULTIPLAYER.md`: friends see a wandering trader's drink and invisibility through `MobShow.drink` and `MobShow.invisible` (no new message), and a trader's trading screen is the villagers' `MenuExtras::Merchant` with `show_progress` false.
- `docs/MINECRAFT-SURVIVAL.md`: players build iron golems like vanilla: a carved pumpkin or jack o'lantern set on a T of four iron blocks (with air under the arms and beside the head) turns the five blocks into an iron golem, in any of the 24 orientations (upright facing any way, or lying down). The blocks break with their particles and sounds. A built golem is player-made and never attacks players. A piston pushing the pumpkin into place builds one too; a dispenser does not place the pumpkin yet.
- `docs/MINECRAFT-SURVIVAL.md`: snow golems are live mobs. Two snow blocks under a carved pumpkin or jack o'lantern (upright or lying down, any facing) turn into a snow golem; the snow golem egg hatches one. It strolls, looks at players, and throws a snowball every second at the nearest monster in sight within 10 blocks (creepers too): snowballs knock monsters back without harm (3 damage to blazes) and make them turn on the golem; players are not affected. Water and rain hurt it, it melts in warm biomes (the `gameplay/snow_golem_melts` attribute: deserts, savannas, badlands, the Nether), and while `mob_griefing` is on it leaves snow layers wherever snow can lie. Shears take off its pumpkin (it drops as an item), a lead ties it, it never despawns, and its pumpkin and health are saved. Friends see it, its pumpkin and its snowballs, and shear and shoot it the same way. A dispenser cannot finish a golem with a pumpkin yet.
- `docs/MINECRAFT-SURVIVAL.md` line 435 ("Structure chests whose loot tables use a function not ported yet roll nothing ...") becomes: "Structure chests roll every 26.3 loot function except `exploration_map`: worn gear (`set_damage`), potions, suspicious stews, ominous bottle levels, fixed enchantments, armour trims, names, goat horns and biome-only items (`location_check`). The chests with explorer maps (abandoned camps, shipwreck map chests and ocean ruins) still roll nothing." The structure loot sentence at line 108 gains: "ancient city, trial chamber, bastion, shipwreck supply and pillager outpost chests roll too".
- `docs/MINECRAFT-SURVIVAL.md`: ancient cities generate in the deep dark with their reinforced deepslate portal frame, sculk shriekers (`can_summon=true`), sensors and catalysts, and their chests (and the kitchen's ice box) roll vanilla loot. The sculk blocks do not react yet (groups `vibrations` and `shriekers-catalysts`).
- `docs/MINECRAFT-SURVIVAL.md`: pillagers are live mobs. They spawn at pillager outposts (anywhere in the outpost's box, by day and night, one at a time, never where block light is above 8), hatch from their egg and are saved with their crossbow (loaded or not), raid and patrol state. A pillager holds its crossbow up in both hands, targets players, adult villagers, wandering traders and iron golems within 32 blocks, walks up to 8 blocks of its target, pulls the string for 1.25 seconds with the loading sounds, waits one to two seconds and shoots one arrow (spread by difficulty), and repeats; one in 300 spawns with piercing. Hit by a player or mob it fights back and calls other pillagers, but never turns on another raider. A patrolling pillager (from older saves) holds its ground and calls the pillagers within 8 blocks. Iron golems hunt pillagers, villagers flee them within 15 blocks and wandering traders within 15. A kill by a player leaves 5 experience plus 1 to 3 for a kept crossbow, which drops worn 8.5 % of the time. Friends see pillagers and their crossbow poses (the arm drawing back while charging), and can shoot them. Patrols, captains with the ominous banner, raids, vindicators and evokers are not there yet; pillagers pick up nothing.
- `docs/MINECRAFT-SURVIVAL.md`: in the pillager paragraph, "Patrols, captains with the ominous banner, raids, vindicators and evokers are not there yet" becomes "Raids, vindicators and evokers are not there yet". New paragraph: "Pillager patrols come like vanilla. From the fifth day on (the overworld clock past 120000), by day, every 10 to 11 minutes the Overworld picks a random player and, one time in five and when that player is not near a village, sends a patrol of `ceil(effective difficulty) + 1` pillagers (three on a new Normal world) 24 to 47 blocks away, never in mushroom fields, peaceful or with `spawn_patrols`, `spawn_mobs` or `spawn_monsters` off. The first is the captain, who wears the ominous banner (it always drops) and leads the others; every pillager that spawns as a leader elsewhere (6 %) wears it too. A captain drops an ominous bottle (level I to V). Patrolling pillagers walk together, and they stay while a player is within 128 blocks. The banner is drawn plain white (no banner patterns yet)."
- `docs/MULTIPLAYER.md`: friends see a captain's banner from the pillager's look tag (its saved equipment); no new message.
- `docs/MINECRAFT-SURVIVAL.md`: in the pillager paragraph, "Raids, vindicators and evokers are not there yet" becomes "Raids and evokers are not there yet". New paragraph: "Vindicators are live mobs. They stand guard in woodland mansions (persistent, with iron axes), hatch from their egg (one in 17 as a captain with the ominous banner) and are saved. A vindicator keeps its arms crossed until it attacks: it hunts players, adult villagers, wandering traders and iron golems within 12 blocks, raises its axe and strikes for 13 damage on Normal, and an axe hit knocks a raised shield out of use for 5 seconds. Hit, it calls the other vindicators. A vindicator named "Johnny" (from a saved or summoned name) attacks every living thing near it but illagers. Iron golems hunt vindicators, villagers flee them within 10 blocks and wandering traders within 8. Friends see the crossed arms, the raised axe and the swing. Door breaking waits for raids."
- `docs/MULTIPLAYER.md`: friends see a vindicator's arm pose, its axe and its swing through `MobShow.arm_pose`, `MobShow.aggressive` and `ServerMob.swings` (no new message); MW2 bullets hit vindicators through hit key 32.
- `docs/MINECRAFT-SURVIVAL.md`: in the wandering trader paragraph, "It does not yet flee illagers (none exist)." becomes "It flees evokers, vindicators, vexes, pillagers and illusioners (only pillagers exist yet)."
- `docs/MULTIPLAYER.md`: friends see a pillager's arm pose and crossbow charge through `MobShow.arm_pose` (the illager pose index), `using_item`, `charging` and `aggressive` (no new message); MW2 bullets hit illagers through hit key 32.
- `docs/MINECRAFT-SURVIVAL.md`: in the pillager paragraph, "Raids and evokers are not there yet" becomes "Raids are not there yet". New paragraph: "Evokers are live mobs. They stand in woodland mansions (persistent), hatch from their egg (one in 17 as a captain with the ominous banner) and are saved, mid-spell too. An evoker hunts players within 12 blocks (it remembers one out of sight for 15 seconds), adult villagers, wandering traders and iron golems, backs away from players within 8 blocks, and casts with its arms raised and coloured swirls at its shoulders: every 5 seconds fangs (a line of 16 running at its target from up to 20 blocks, or two rings around itself when the target is within 3), each biting once for 6 magic damage that armour and shields do not stop, sparing illagers; every 17 seconds it summons vexes; and without a target, while `mob_griefing` is on, it turns a blue sheep within 16 blocks red (wololo). It always drops a totem of undying and gives 10 experience. Iron golems hunt evokers and villagers flee them. Friends see the spells, the swirls and the fangs."
- `docs/MULTIPLAYER.md`: friends see an evoker's spell through the new `MobShow.spell` (the synced spell id) and evoker fangs as a tracked projectile whose warmup and life ride in the new `MobShow.bite` (no new message); a fangs bite on a friend is a host `PlayerHit` of the new kind `Magic`.
- `docs/MINECRAFT-SURVIVAL.md`: new paragraph: "Vexes are live mobs. An evoker with a target summons three (when fewer than one to eight are within 16 blocks) just above it, each with an iron sword it never drops; they copy the evoker's target, fly through blocks without gravity, roam within 7 blocks of where they were summoned, and charge with their charging look to strike (9 damage on Normal). Summoned vexes live 30 seconds to 2 minutes and then lose half a heart a second until they die; vexes from eggs and spawners live on. Evoker fangs spare vexes. Vexes are saved with their owner, spot and remaining life. Iron golems hunt vexes and villagers flee them. Friends see the vexes, their swords and their charging look and can shoot them."
- `docs/MULTIPLAYER.md`: friends see a vex's charging through `MobShow.charging` (no new message); MW2 bullets hit vexes through hit key 33.

## Vendored files

Patched by track D: 33 (`entities/src/movement.rs`, `entities/src/effects.rs`, `entities/src/lib.rs`, `entities/src/potion.rs`, `entities/src/stroll.rs`, `entities/src/world.rs`, `entities/src/world/villagers.rs`, `entities/src/zombie.rs`, `player/src/survival.rs`, `world/src/level/spawning.rs`, `world/src/natural_spawner.rs`, `world/src/natural_spawner/spawning.rs`, `entities/src/monster_ai.rs`, `entities/src/world/combat.rs`, `entities/src/world/hazards.rs`, `entities/src/world/pushing.rs`, `entities/src/world/riding.rs`, `entities/src/world/witches.rs`, `generator/src/feature/entities.rs`, `entities/src/controls.rs`, `entities/src/loot.rs`, `world/src/level/time.rs`, `entities/src/cow_ai.rs`, `entities/src/horse.rs`, `entities/src/world/shulker_bullets.rs`, `entities/src/bin/check_zombie_player_hits.rs`, `entities/src/world/merchants.rs`, `world/src/level_storage.rs`, `world/src/storage.rs`, `world/src/level/mod.rs`, `player/src/lib.rs`, `core/src/item.rs`, `core/src/loot.rs`). Added: 23 (`entities/src/drowned_ai.rs`; `entities/src/feline.rs`, `entities/src/feline_ai.rs` and `entities/src/world/felines.rs` from group cats; `entities/src/llama_ai.rs`, `entities/src/llama_spit.rs` and `entities/src/world/equines.rs` from group horse-family; `entities/src/world/leashes.rs`, `entities/src/wandering_trader.rs`, `entities/src/trader_ai.rs` and `entities/src/world/traders.rs` from group wandering-trader; `world/src/level/golem_build.rs`, `entities/src/snow_golem.rs`, `entities/src/snow_golem_ai.rs` and `entities/src/world/snow_golems.rs` from group built-golems; `entities/src/illager.rs`, `entities/src/illager_ai.rs` and `entities/src/world/illagers.rs` from group pillagers; `entities/src/evoker_fangs.rs`, `entities/src/world/evoker_fangs.rs`, `entities/src/vex.rs`, `entities/src/vex_ai.rs` and `entities/src/world/vexes.rs` from group evokers-vexes). Group zombie-villagers added `entities/src/world/villagers.rs` and `world/src/level/spawning.rs` (commit 1) and `entities/src/effects.rs` and `entities/src/potion.rs` (commit 2) to the patched list. Group effects-omens added `player/src/survival.rs` (commit 1). Group riding added `entities/src/movement.rs` (commit 1). Group horse-family added `entities/src/cow_ai.rs`, `entities/src/horse.rs`, `entities/src/world/shulker_bullets.rs` and `entities/src/bin/check_zombie_player_hits.rs` (commit 1). Group cats added `entities/src/monster_ai.rs`, `entities/src/world/combat.rs`, `entities/src/world/hazards.rs`, `entities/src/world/pushing.rs`, `entities/src/world/riding.rs`, `entities/src/world/witches.rs` and `generator/src/feature/entities.rs` (commit 1), and `entities/src/controls.rs`, `entities/src/loot.rs` and `world/src/level/time.rs` (commit 2). Group wandering-trader added `entities/src/world/merchants.rs`, `world/src/level_storage.rs` and `world/src/storage.rs` (commit 2). Group built-golems added `world/src/level/mod.rs` (commit 1) and `player/src/lib.rs` (commit 2). Group loot-functions added `core/src/item.rs` and `core/src/loot.rs` (commit 1). Group patrols-vindicators touched only files already listed (commit 1: `player/src/lib.rs`, `world/src/natural_spawner/spawning.rs`, `world/src/level/spawning.rs`, `world/src/level/time.rs`, `entities/src/world.rs` and the added illager files; commit 2: `entities/src/monster_ai.rs`, `world/src/natural_spawner.rs`, `world/src/natural_spawner/spawning.rs`, `generator/src/feature/entities.rs` and the added illager files). Group evokers-vexes touched only files already listed besides its added ones (commit 1: `entities/src/lib.rs`, `entities/src/loot.rs`, `entities/src/monster_ai.rs`, `entities/src/world.rs`, `entities/src/world/{equines,pushing,riding,shulker_bullets,witches}.rs`, `entities/src/bin/check_zombie_player_hits.rs`, `world/src/natural_spawner.rs`, `world/src/natural_spawner/spawning.rs`, `generator/src/feature/entities.rs` and the illager files; commit 2: `entities/src/lib.rs`, `entities/src/monster_ai.rs`, `entities/src/illager_ai.rs`, `entities/src/world.rs`, `entities/src/world/{combat,hazards,pushing,riding,villagers,witches,illagers,evoker_fangs}.rs`, `world/src/natural_spawner.rs` and `world/src/natural_spawner/spawning.rs`).

## Left for later

- Cats sitting on a chest keep it shut (`ChestBlock.isChestBlockedAt`, `isCatSittingOnChest`): the level's containers cannot see the entity world's cats yet; it needs glue in `minecraft_terrain`.
- Cats scare phantoms (group `phantoms`), and cats hiss at them.
- Ocean ruin drowned: the generator's `finalize_spawn` (`generator/src/feature/entities.rs`) gives them only `Mob.finalizeSpawn`; vanilla runs `Drowned.finalizeSpawn` with the `STRUCTURE` reason (trident roll, shell, zombie nautilus jockey).
- Zombie nautilus jockeys for natural drowned with tridents (needs mob 38, group `nautiluses`).
- The drowned's raised trident uses the plain held model; vanilla draws `item/trident_throwing` while the item is in use (`minecraft:using_item`), which needs a using flag on `mesh::HeldItem`.
- Trading: number keys, Q, double-click gathering and drags on the payment and result slots.
- `/effect give` of Bad Omen, Raid Omen or Trial Omen plays no omen sound (vanilla plays `event.mob_effect.<omen>` for a new omen from any source); the commands need a sound path to the player.
- The ominous bottle's tooltip line ("Bad Omen V (100:00)" from `OminousBottleAmplifier.addToTooltip`).
- The inventory screens' effect list (`EffectsInInventory`: the effect names, levels and times beside the survival and creative inventories when there is room, which hides the HUD icons while it shows).
- Ambient player effects (conduit power, later beacons): `EffectInstance` has no ambient flag, so the ambient frame and order never show.
- Wolves flee llamas (`Wolf.WolfAvoidEntityGoal`: a wild wolf avoids a llama within 24 blocks when the llama's strength beats `nextInt(5)`): the wolf's goal context has no llama strength yet; the slot stays the idle goal in `wolf_ai.rs`.
- Leads: punching a knot off (a left click on the fence digs it; vanilla's knot entity takes the hit), the leash turning its mob (`Leashable.angularMomentum`), the home radius round the holder (`PathfinderMob.whenLeashedTo`), the fall distance an elastic pull keeps (`onElasticLeashPull`), the `entity_drops` game rule for dropped leads, and the first-person rope end (vanilla draws it from the hand in view).
- Mount inventories: another player saddling, armouring or chesting a mount while its inventory screen is open (the open copy should be refreshed or closed), and a click in the one tick between the mount's death and its screen closing. Review fix 3 lets only one player open a mount's inventory at a time.
- The mount drawn in its inventory screen's box (`InventoryScreen.extractEntityInInventoryFollowsMouse`); a mount's custom name as the screen title; components on a saddle or horse armour put in through the screen.
- Riding leftovers: the rider takes fall damage with its horse (`AbstractHorse.causeFallDamage` hurts the passengers), the gallop and breathe sounds of a ridden horse (`playStepSound` with `gallopSoundCounter`), the taming hearts and the bucking smoke (entity events 7 and 6 are sent, the client draws no entity-event particles yet), the vehicle's hearts in place of the food bar (`Gui.renderVehicleHealth`, in `hud/minecraft_vitals.rs`, track B's file), dyed leather horse armour.
- The seven `spit` particles when llama spit appears (`LlamaSpit.recreateFromPacket`): the client has no particle for a newly tracked projectile yet.
- Wandering traders: a trader llama leaves on the same tick as its trader, not one tick before it (the leash pass runs before the mobs tick). (Fleeing illagers landed with group pillagers.)
- Built golems: the dispenser's carved pumpkin (`canSpawnGolem` with the base patterns, placing the pumpkin to finish a golem, else putting it on an entity's head); copper golems from a carved pumpkin on a copper block (group `copper-golems`); the snow golem's pumpkin in spawner cages; the snow golem's `BLOCK_PLACE` and `SHEAR` game events (group `vibrations`).
- Explorer maps (`exploration_map`: the structure search, the filled map and its decorations): the abandoned camp, shipwreck map and ocean ruin chests roll nothing until it lands.
- Chest minecarts (the abandoned mineshaft's loot with its `sulfur_caves` disc): the fork has no minecarts that carry loot.
- Pillager leftovers: witches answering a pillager's hold-ground call, witches and ravagers as patrol companions (groups `ravagers` and `raids`), and picking up items (`Mob.pickUpItem` equipment and the raid banner). (The patrol goal, the patrolling despawn rule and the captain's banner and bottle landed with group `patrols-vindicators`.)
- Vindicators: `VindicatorBreakDoorGoal` and `RaiderOpenDoorGoal` (raids only; group `raids`), `applyRaidBuffs` and the celebrate sound (group `raids`), naming a vindicator Johnny with a name tag (the fork has no name tags), and enchanted spawn equipment (not ported for any mob).
- Banner patterns are not drawn (placed banners, held banners and the captain's ominous banner show their base colour only); skulls worn by illagers are not drawn (`CustomHeadLayer`'s skull branch).
- The piglin's crossbow shot draws a pitch (`1 / (0.8..1.2)`) from its random; vanilla's single crossbow arrow plays at pitch 1 with no draw (`CrossbowItem.getShotPitch` index 0). The pillager does it the vanilla way; the piglin (round 8, `world/piglins.rs`) is unchanged.
- The crossbow's loading start and middle sounds play in the shooter's category; vanilla plays them as `PLAYERS` sounds.
- Vexes: flight through water and lava (vanilla's `travelInFluid` drag) and drowning; the follow range bonus roll of a summoned vex; a summoned vex joining its evoker's team.
- Evokers: the fangs' knockback (`LivingEntity.hurtServer` pushes 0.4 away from the fangs; the fork's indirect magic pushes nothing), a summoned vex joining its evoker's scoreboard team (the fork has no teams), the celebrate sound in raids (group `raids`).
- A mob's held crossbow is drawn with its plain item model: a loaded crossbow shows no arrow and a charging one no pull (`mesh::HeldItem` carries no components); pillagers in spawner cages are drawn without their crossbow (the cage draws mob models only).
