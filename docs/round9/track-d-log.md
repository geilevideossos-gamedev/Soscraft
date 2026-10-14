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

## Group ravagers

### 1. Minecraft ravagers roar, stun and carry illagers

After ee573ae0.

- **Vanilla (javap, 26.3, and the 26.3 data pack):**
  - `EntityTypes.RAVAGER`: `MobCategory.MONSTER`, `sized(1.95, 2.2)` (eye height the default 0.85 of the height, 1.87), `passengerAttachments((0, 2.2625, -0.0625))`, `clientTrackingRange(10)`, `notInPeaceful`. `SpawnPlacements`: `ON_GROUND`, `MOTION_BLOCKING_NO_LEAVES`, `Monster.checkMonsterSpawnRules`; no biome lists it (raids bring it).
  - `Ravager` (a `Raider`, so a `PatrollingMonster`): `xpReward` 20, `setPathfindingMalus(LEAVES, 0)`; `createAttributes`: `Monster`'s with max health 100, movement speed 0.3, knockback resistance 0.75, attack damage 12, attack knockback 1.5, follow range 32, step height 1. `getMaxHeadYRot` 45. `canBeLeader` false; `applyRaidBuffs` does nothing; `getCelebrateSound` `entity.ravager.celebrate`; ambient, hurt, death `entity.ravager.*`, step `entity.ravager.step` at volume 0.15. Saves `AttackTick`, `StunTick` and `RoarTick` (default 0) beside the raider and patrol fields.
  - Goals by registration (after `PatrollingMonster`'s 4 `LongDistancePatrolGoal` and `Raider`'s 1, 3, 4, 5): 0 `FloatGoal`, 4 `MeleeAttackGoal(1.0, true)`, 5 `WaterAvoidingRandomStrollGoal(0.4)`, 6 `LookAtPlayerGoal(Player, 6)`, 10 `LookAtPlayerGoal(Mob, 8)` (both at the default probability 0.02). Targets: 2 `HurtByTargetGoal(Raider ignored).setAlertOthers()`, 3 `NearestAttackableTargetGoal(Player, true)`, 4 `(AbstractVillager, true, not a baby)`, 4 `(IronGolem, true)`. `getAttackBoundingBox` deflates the default box by 0.05 across.
  - `updateControlFlags`: the MOVE, LOOK and TARGET flags of the goal selector stay on while the controlling passenger is no mob or a raider (`#raiders`), JUMP also needs no boat. `Mob.getControllingPassenger` is the first passenger when it is a mob that can control and the ravager has AI.
  - `aiStep` after `Raider.aiStep` (which skips `serverAiStep` and zeroes the inputs while `isImmobile`: dead, `attackTick`, `stunnedTick` or `roarTick` above 0): if alive, the movement speed base becomes 0 while immobile, else `lerp(0.1, base, target ? 0.35 : 0.3)`; on the server, with `horizontalCollision` and `mob_griefing`, every `LeavesBlock` in the box grown 0.2 (`betweenClosed` of the floored corners) is `destroyBlock`ed with drops, and if none broke while on the ground it `jumpFromGround`s; then `roarTick` counts down and at 10 calls `roar()`; `attackTick` counts down; `stunnedTick` counts down with `stunEffect()` (one in `nextInt(6)`, an `ENTITY_EFFECT` particle (0.49803922, 0.5137255, 0.57254905) at `x - width * sin(yBodyRot) + rand * 0.6 - 0.3`, `y + height - 0.3`, `z + width * cos(yBodyRot) + ...`) and at 0 plays `entity.ravager.roar` and sets `roarTick` 20. `hasLineOfSight` is false while stunned or roaring.
  - `doHurtTarget`: `attackTick` 10, entity event 4, `entity.ravager.attack`, then `Mob.doHurtTarget` (12 damage; on a hit, knockback `1.5 * 0.5` along its facing and its own motion times (0.6, 1, 0.6)).
  - `blockedByItem(defender, ...)` (called through the defender's `blockUsingItem` for a blocked non-projectile hit whose direct attacker it is): only while `roarTick` is 0; `nextDouble() < 0.5` stuns it (`stunnedTick` 40, `entity.ravager.stunned`, entity event 39, `defender.push(ravager)`), otherwise `strongKnockback(defender)`; then the defender's velocity syncs.
  - `roar()`: every living entity in the box grown 4 that is not a ravager and is alive (and no armor stand without `mob_griefing`): all but `AbstractIllager`s take 6 `mobAttack` from it, all but players get `strongKnockback`; then the `ENTITY_ACTION` game event and entity event 69, which on the client throws 40 `POOF` particles from the box centre (gaussian x 0.2) and gives the local player the same `strongKnockback`. `strongKnockback(e)`: `d = max(dx^2 + dz^2, 0.001)`, `push(dx / d * 4, 0.2, dz / d * 4)`.
  - `checkSpawnObstruction`: no liquid in its box. `loot_table/entities/ravager.json`: one saddle.
- **Changed:**
  - New vendored `entities/src/ravager.rs` (`Ravager`: size, health, attributes, the raider state, the three clocks and the eased speed; `is_immobile`, `update_speed`, `blocked_by_item`, `tick_timers`, `leaf_cells`, `attack_throw`, `stun_particle`, `strong_knockback`, `passenger_point`), `entities/src/ravager_ai.rs` (the goal and target lists in vanilla's slots on the shared raider goals) and `entities/src/world/ravagers.rs` (the `ravagers` store: `RavagerEntity`, `spawn_ravager`, its tick with the immobile skip, the bite with its knockback, leaf breaking through the new `World::destroy_block`, the clocks, the roar on mobs and players, `blocked_by_shield`, `take_player_pushes`, the alert to other ravagers, the hold-still under a non-raider rider).
  - Vendored `entities`: `world.rs` and `world/{combat,hazards,pushing,riding,villagers,witches}.rs` (the store in every walk over the mobs, `EntityKey::Ravager` and `MobHit::Ravager`, knockback resistance 0.75 on knockback and arrows, 20 experience, hostile to villagers, `player_pushes`); `world/riding.rs` (a ravager carries one rider at its turned passenger point and lets it off on its top; illagers sit 0.6 lower, vexes 0.04 higher); `monster_ai.rs` (`MonsterKind::Ravager`, enemy, spares baby villagers, floats, follow range 32, leaves malus 0 and a one-block step, the shrunk attack box, `MonsterAi::set_steered`); `look.rs` (`tick_limited` for a head limit other than 75); `illager_ai.rs` (`raider_goals` shared); `lib.rs`. Vendored `player/src/lib.rs`: `World::destroy_block` (default: set air).
  - Vendored `world/src/natural_spawner.rs` (`type_info` for the ravager: 1.95 x 2.2 on the ground, monster rules) and `natural_spawner/spawning.rs` (the monster category, follow range 32, the finalize: `CanJoinRaid`, the leader roll drawn, `Mob`'s part).
  - `minecraft_terrain`: new `ravager_save.rs` (a ravager from its tag and its fields); `server_mobs.rs` (`SIMULATED_KINDS`, `VEHICLES` (illager jockeys load mounted), the `spawn_saved` and `mob_tags_where` arms, `MobWorld::destroy_block` through the level's `destroy_block_drops`); `mob_tracking.rs` (tracked at 10 chunks, the clocks in the new `MobShow.ravager`, `AttackTick`, `StunTick` and `RoarTick` volatile); `client_mobs.rs` (ravagers in `server_mobs` with the 45-degree head and their swings); `server.rs` (`Command::ShieldBlock` and `ServerHandle::shield_block`, the entity world's player pushes into `players_pushed`, `HATCHING_EGGS` grows to 49); `lib.rs`.
  - `minecraft_survival`: `entities.rs` (hit key 34 for `MobHit::Ravager`, `Entities::shield_blocked`), `entity_view.rs` (ravagers hit by MW2 bullets). `render_anim/src/minecraft_players.rs`: a mob's melee hit that a raised shield blocks is reported to the host's entity world (`shield_blocked`), for the host and every friend.
- **Tests:**
  - Vendored entities: `ravager::tests` (`a_ravager_is_a_big_hundred_heart_raider`, `a_blocked_ravager_is_stunned_for_forty_ticks_then_roars_ten_ticks_in` (stunned for 40 ticks, the roar sound at tick 40, the roar at tick 50, still for 60 ticks), `a_roaring_ravager_ignores_a_block`, `an_attack_holds_it_still_for_nine_more_ticks_and_slows_it`, `the_roar_flings_sideways_harder_the_closer_it_is`, `its_rider_sits_two_and_a_quarter_blocks_up_a_touch_behind`, `its_hit_throws_forward_and_up`), `world::ravagers::tests` (`a_ravager_is_a_live_monster_of_its_size_and_health` (20 experience, its loot table), `a_ravager_runs_down_a_player_bites_for_twelve_and_stands_still_after`, `a_blocked_ravager_is_stunned_for_forty_ticks_then_roars_and_flings_what_is_near` (the nudge, event 39, the stun sound, about one particle in six ticks, the roar at tick 50: 6 damage to the player and its fling, a cow hurt and flung, a pillager flung but unhurt), `a_blocked_ravager_that_is_not_stunned_throws_the_shield_bearer_back`, `a_ravager_breaks_leaves_it_walks_into_only_with_mob_griefing`, `a_pillager_rides_a_ravager_two_blocks_up_and_the_ravager_still_hunts`, `a_zombie_on_a_ravager_holds_it_still`, `iron_golems_hunt_ravagers_villagers_flee_them_and_babies_are_spared`).
  - Vendored world: `natural_spawner::spawning::tests::a_summoned_ravager_can_join_raids_but_never_leads`.
  - `minecraft_terrain`: `server_mobs::tests::a_ravager_saves_and_loads_mid_stun_with_its_raider_state`, `a_pillager_on_a_ravager_loads_rides_and_saves_inside_it`; `mob_tracking::tests::a_friend_sees_a_ravager_and_its_rider_and_watches_it_bite_stagger_and_roar`; `spawn_egg::tests::a_ravager_egg_hatches_as_wide_as_the_ravager_and_lands_on_a_ladder_edge`; `every_spawn_egg_of_a_mob_the_game_has_hatches_that_mob` now hatches 49 eggs.
  - `minecraft_survival`: `entities::tests::a_ravager_hit_box_encodes_and_decodes`.
  - Runs: `minecraftoss-entities` lib 357, `minecraftoss-world` lib 96, `minecraft_terrain` lib 1314 of 1317 (3 ignored), `minecraft_survival` lib 719, `render_anim` lib 177, all green. `check -p launcher` green. The four new files pass `rustfmt --check --edition 2021`.
- **Decisions:**
  - The ravager has its own `ravagers` store (hit key 34): vanilla's `Ravager` is a `Raider` of its own, not an illager.
  - The shield block is decided where the player's shield is (the player step that applies mob hits, host and friends alike), so the stun reaches the entity world one tick after the bite as a `ShieldBlock` command; the stun then runs its 40 ticks from there. The blocker's push (the nudge or the fling) comes back through `players_pushed`.
  - The ravager's attack knockback rides in the hit's `throw`, which the fork applies on a shielded hit too (as the hoglin's); vanilla's `causeExtraKnockback` runs only on a hurt.
  - The roar's fling of players is the server's: vanilla does it on each client for its own player; the fork's host sends it to every player in the box like the wind charge's.
  - A ravager carrying a mob that is no raider (only from commands; raids put illagers on it) turns its MOVE, JUMP, LOOK and TARGET goals off as vanilla does and stands; the fork does not pass that rider's path to it. Illager riders leave it its own goals, so it hunts while its rider shoots or strikes from its back.
  - A dismounting rider lands on the ravager's top (`Entity.getDismountLocationForPassenger`).
  - Patrol companions stay illagers only; a patrolling ravager (none spawn yet) finds none and stops patrolling.
  - Entity events 39 and 69 are recorded as vanilla sends them; the client draws the stun and roar particles from the shown clocks (commit 2).
- **Test steps:** none on their own; the ravager is drawn with commit 2, whose steps cover this commit (bite, stun and roar, leaves, riders).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ravager_spawn_egg*4,minecraft:pillager_spawn_egg*4,minecraft:iron_golem_spawn_egg*2,minecraft:villager_spawn_egg*2,minecraft:shield,minecraft:diamond_sword,minecraft:oak_leaves*64,minecraft:golden_apple*8,minecraft:cooked_beef*32
```

### 2. Minecraft ravagers are drawn like vanilla

After 953bfece.

- **Vanilla (javap, 26.3):**
  - `RavagerModel.createBodyLayer` (128 x 128): `neck` (68, 73) box (-5, -1, -18) 10 x 10 x 18 at (0, -7, 5.5); on it `head` (0, 0) box (-8, -20, -14) 16 x 20 x 16 and the nose (0, 0) box (-2, -6, -18) 4 x 8 x 4 at (0, 16, -17); on the head `right_horn` (74, 55) box (0, -14, -2) 2 x 14 x 4 at (-10, -14, -8) turned 1.0995574 about X, `left_horn` the same mirrored at (8, -14, -8), and `mouth` (0, 36) box (-8, 0, -16) 16 x 3 x 16 at (0, -2, 2); `body` (0, 55) box (-7, -10, -7) 14 x 16 x 20 and (0, 91) box (-6, 6, -7) 12 x 13 x 18 at (0, 1, 2) turned pi/2 about X; `right_hind_leg` (96, 0) box (-4, 0, -4) 8 x 37 x 8 at (-8, -13, 18), `left_hind_leg` mirrored at (8, -13, 18), `right_front_leg` (64, 0) at (-8, -13, -5), `left_front_leg` mirrored at (8, -13, -5).
  - `RavagerModel.setupAnim` (after the pose reset): biting (`attackTicksRemaining` above 0), `f = (1 + triangleWave(attack, 10)) * 0.5`, the neck moves to z `-6.5 + f^3 * 12` (and y `-7 - that * sin(neck xRot)`, which is 0), and the jaw opens `sin((attack - 4) / 4) * pi * 0.4` above 5 ticks, else `0.15707964 * sin(pi * attack / 10)`; otherwise the neck sits at (0, -7, 5.5), stunned it pitches 0.21991149 with the jaw at `pi * 0.05` (else `pi * 0.01`) and sways sideways `sin(stunned / 40 * 10) * 3`, and while roaring the jaw opens `pi/2 * sin(roar * pi * 0.25)`; the head turns and pitches; the legs swing `cos(walkPos * 0.6662 (+ pi)) * 0.4 * walkSpeed`, right hind with left front.
  - `RavagerRenderer`: shadow 1.1, `textures/entity/illager/ravager.png`; `extractRenderState` gives the stun and attack ticks less the partial tick (0 when none) and `roarAnimation = (20 - roarTick + partial) / 20` while roaring.
  - Entity event 69 (the roar) on the client: 40 `POOF` particles from the box centre with gaussian x 0.2 speeds; `stunEffect` runs in `aiStep` on the client too, so a stunned ravager shows its swirls there.
- **Changed:**
  - New `crates/minecraft_terrain/src/ravager_render.rs` (`RavagerModel` ported from `createBodyLayer` and `setupAnim` on `posed_cube`, `RavagerLook` from `extractRenderState`, shadow 1.1, its texture); `minecraft_terrain`: `lib.rs`, `mesh.rs` (the texture), `client_mobs.rs` (the shadow), `spawner_display.rs` (ravagers turn in spawner cages), `server_mobs.rs` (a rider tag without a position, as `/summon` writes one, sits where its vehicle stands, for every jockey).
  - `minecraft_survival/src/entity_view.rs`: ravagers drawn; the stun swirls (one tick in six, at the ravager's head) and the roar's 40 puffs once per roar, from the shown clocks, for the host and every friend.
- **Tests:**
  - `minecraft_terrain`: `ravager_render::tests` (`a_ravager_stands_on_its_legs_about_two_blocks_tall`, `its_neck_lunges_and_its_jaw_snaps_while_it_bites`, `a_stunned_ravager_hangs_its_head_and_sways_then_roars_wide`, `its_legs_swing_in_diagonal_pairs`), `server_mobs::tests::a_summoned_rider_without_a_position_sits_on_its_ravager`, the spawner cage test with the ravager.
  - `minecraft_survival`: `entity_view::tests::a_stunned_ravager_swirls_now_and_then_and_its_roar_puffs_once`.
  - Runs: `minecraft_terrain` lib 1319 of 1322 (3 ignored), `minecraft_survival` lib 720, all green. `check -p launcher` green. The new `ravager_render.rs` passes `rustfmt --check --edition 2021`.
- **Decisions:**
  - The swirls and puffs are drawn by each viewer from the ravager's shown clocks (vanilla runs `stunEffect` and entity event 69 on the client), so friends see them without a new message; the viewer's own random places them.
  - The roar's puffs come when the viewer first sees the roar clock at 10 or below, so a viewer that skips the exact tick still shows them once.
  - `/summon` riders with no `Pos` take their vehicle's position (vanilla positions passengers on their vehicle as they load); before this, such a rider did not load.
- **Test steps (survival, Normal, by day, open flat ground; diamond armour on):**
  1. (D4 step 2, the ravager.) In creative, use a ravager egg: a big grey horned beast stands there, turns its head (only so far) and walks with its legs swinging in pairs (**146_ravager**).
  2. `/gamemode survival` and walk to about 10 blocks: it runs at you, its neck lunges and the jaw snaps, a bite takes about 2 hearts through diamond armour (6 without armour) and throws you forward, and it stands still for half a second after each bite (**147_ravager_bite**).
  3. Hold up the shield facing it. A blocked bite either stuns it (the stunned sound, its head hangs and sways from side to side under grey swirls for 2 seconds, then it roars with its jaw wide open and a ring of puffs, throwing you and every mob near it away and hurting you for almost a heart through diamond armour, 3 hearts without), or, half the time, throws you far away without a stun (**148_ravager_stunned**, **149_ravager_roar**). Put a cow and a pillager beside it before the roar: the cow is hurt and thrown, the pillager only thrown.
  4. Build a wall of oak leaves three high between you and it and stand behind: it walks through, breaking the leaves (some drop saplings or sticks) (**150_ravager_leaves**). `/gamerule mob_griefing false`, build a new wall: it stays behind the leaves. `/gamerule mob_griefing true`.
  5. `/summon minecraft:ravager ~ ~ ~6 {Passengers:[{id:"minecraft:pillager",equipment:{mainhand:{id:"minecraft:crossbow",count:1}}}]}`: the pillager sits on its back with its legs bent forward and shoots its crossbow while the ravager runs at you (**151_ravager_rider**). Kill the ravager: the pillager drops to the ground and keeps fighting. `/summon minecraft:ravager ~ ~ ~6 {Passengers:[{id:"minecraft:zombie"}]}`: that ravager stands still.
  6. In creative, an iron golem egg beside a ravager: they fight; a villager egg 10 blocks away: the villager runs from it. `/gamemode survival` and kill a ravager with the sword: it drops a saddle and 20 experience.
  7. Save and quit while a ravager is stunned and while a pillager rides one, then load: the stunned one finishes its stun and roars, the rider is still on its ravager.
  8. Friend window: the friend sees the ravager, its rider, the bite, the stun swirls and the roar puffs; a bite the friend blocks with a shield stuns it or throws the friend; the roar throws the friend; the friend shoots ravagers with a gun (**152_friend_ravager**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ravager_spawn_egg*8,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg*2,minecraft:evoker_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:villager_spawn_egg*2,minecraft:cow_spawn_egg*4,minecraft:shield,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:oak_leaves*64,minecraft:birch_leaves*64,minecraft:cobblestone*64,minecraft:spyglass
```

### 3. Track D's ravager renderer passes clippy

After 7e008469.

- **Changed:** `crates/minecraft_terrain/src/ravager_render.rs`: the jaw's pitch is one `if` expression (`clippy::needless_late_init`); no change in what is drawn.
- **Tests:** `ravager_render::tests` (4) green. Clippy on `minecraftoss-entities`, `minecraftoss-world`, `minecraft_terrain`, `minecraft_survival` and `render_anim` (`--all-targets`, `-A clippy::approx_constant`) finds nothing in group ravagers' code. The file passes `rustfmt --check --edition 2021`.
- **Test steps:** none; commit 2's steps hold.
- **Kit:** commit 2's.

```
IW4L_MINECRAFT_KIT=minecraft:ravager_spawn_egg*8,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg*2,minecraft:evoker_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:villager_spawn_egg*2,minecraft:cow_spawn_egg*4,minecraft:shield,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:oak_leaves*64,minecraft:birch_leaves*64,minecraft:cobblestone*64,minecraft:spyglass
```

## Group raids

### 1. Minecraft raids come in waves with a boss bar and a horn

After c14c26aa.

- **Vanilla (javap, 26.3, and the 26.3 data pack):**
  - `BadOmenMobEffect.applyEffectTick` (every tick): a non-spectator `ServerPlayer`, not on Peaceful, standing where `ServerLevel.isVillage(blockPosition)` (`isCloseToVillage(pos, 1)`: `sectionsToVillage` at most 1, from the sections holding an occupied `#village` POI), and either no raid at `getRaidAt` (`Raids.getNearbyRaid(pos, 9216)`: the nearest active raid centre within 96 blocks) or one below raid omen level 5: gets `RAID_OMEN` for 600 ticks at the bad omen's amplifier and `setRaidOmenPosition(blockPosition)`, and the bad omen ends (the tick returns false). `RAID_OMEN` is neutral and plays `event.mob_effect.raid_omen` when added.
  - `RaidOmenMobEffect`: ticks only at duration 1; then, for a non-spectator player with a raid omen position, `Raids.createOrExtendRaid(player, position)` and `clearRaidOmenPosition`; the effect ends. `ServerPlayer` saves `raid_omen_position` (`BlockPos.CODEC`, an int array) in the player file.
  - `Raids.createOrExtendRaid`: nothing for a spectator, with the game rule `raids` off or where the environment attribute `gameplay/can_start_raid` is false; the centre is the floored mean of the occupied `#village` POIs within 64 blocks, else the omen position; `getOrCreateRaid` takes the raid at that centre or a new `Raid(centre, difficulty)` registered under `++nextId` (`nextId` starts at 1, so the first raid is 2); if it has not started or its omen level is below 5, `absorbRaidOmen`: `raidOmenLevel = clamp(raidOmenLevel + amplifier + 1, 0, 5)`.
  - `Raids.tick` (in `ServerLevel.tick`, before the entities): `tick` counts up; each raid is stopped while the `raids` rule is off, a stopped raid is removed, the others tick. Saved data `minecraft:raids` per dimension (`data/minecraft/raids.dat` in the dimension folder): `raids` (each with `id` and the raid's `started`, `active`, `ticks_active` (long), `raid_omen_level`, `groups_spawned`, `cooldown_ticks`, `post_raid_ticks`, `total_health` (float), `group_count`, `status` (`ongoing`, `victory`, `loss`, `stopped`), `center` (int array), `heroes_of_the_village` (UUID int arrays)), `next_id`, `tick`. A new `Raids` marks itself dirty, so the file is written even empty.
  - `Raid` constants: `getNumGroups` Peaceful 0, Easy 3, Normal 5, Hard 7; pre-raid cooldown 300 ticks; `RAID_TIMEOUT_TICKS` 48000; `MAX_NO_ACTION_TIME` 2400; `MAX_CELEBRATION_TICKS` 600; `POST_RAID_TICK_LIMIT` 40; `OUTSIDE_RAID_BOUNDS_TIMEOUT` 30; `NUM_SPAWN_ATTEMPTS` 5; `LOW_MOB_THRESHOLD` 2; `VALID_RAID_RADIUS` 96 (9216 squared); `RAID_REMOVAL_THRESHOLD_SQR` 12544 (112 blocks); `HERO_OF_THE_VILLAGE_DURATION` 48000; max raid omen level 5. The bar is `ServerBossEvent(RAID_NAME_COMPONENT, RED, NOTCHED_10)` named `event.minecraft.raid`, `event.minecraft.raid` + " - " + `event.minecraft.raid.raiders_remaining` (with the count), `event.minecraft.raid.victory.full` or `event.minecraft.raid.defeat.full`.
  - `Raid.tick` while ongoing: `active = hasChunkAt(centre)`; Peaceful stops it; the bar's visibility follows `active`; an inactive raid waits; a centre no longer in a village moves to the nearest village section centre within 2 sections (`SectionPos.cube`), and if none is a village the raid is lost (once a wave spawned) or stopped; `ticksActive` counts to 48000 (then stopped). With no raider alive and more waves to come, during the cooldown the spawn point is looked for every 5 ticks (`findRandomSpawnPos(level, 8)`, again when it stops ticking), the bar's players are refreshed at 300 and every 20 ticks, the cooldown counts down and the bar fills `(300 - cooldown) / 300`; at cooldown 0 after a wave it restarts at 300 named "Raid". Every 20 ticks `updatePlayers` (alive players whose `getRaidAt` is this raid), `updateRaiders` and the name ("Raiders Remaining" at 1 or 2 alive, by the count from the tick's start). Then `spawnGroup` at the spawn point (or `findRandomSpawnPos(level, 20)`) while `shouldSpawnGroup` (cooldown 0, a wave or the bonus wave due, nobody alive), the horn with the first group, stopped after more than 5 failures. Once started, with no more waves and nobody alive at the tick's start, 40 ticks later the raid is won and every hero (by UUID, living, not a spectator) gets `HERO_OF_THE_VILLAGE` for 48000 ticks at `raidOmenLevel - 1`. Over: 600 ticks of celebration, every 20 refreshing the players and showing "Raid - Victory" at progress 0 or "Raid - Defeat", then stopped.
  - Waves: `hasBonusWave` is raid omen level above 1; `getDefaultNumSpawns(type, wave, bonus)` reads `spawnsPerWaveBeforeBonus[bonus ? numGroups : wave]` (`Raid$RaiderType` in this order: vindicator 0,0,2,0,1,4,2,5; evoker 0,0,0,0,0,1,1,2; pillager 0,4,3,3,4,4,4,2; witch 0,0,0,0,3,0,0,1; ravager 0,0,0,1,0,1,0,2), plus `getPotentialBonusSpawns`: vindicators and pillagers up to `nextInt(2)` on Easy, 1 on Normal, 2 on Hard (then `nextInt(n + 1)`), witches up to 1 off Easy on waves above 2 but not 4, ravagers up to 1 on the bonus wave off Easy, evokers none. The first raider that can lead (`canBeLeader`: not witches, not ravagers) leads its wave and wears the ominous banner (drop chance 2). A ravager carries a pillager on wave 5 (Normal's last) and from wave 7 an evoker, then vindicators. Each member: `joinRaid` (registered with its health in the total, placed at `pos + (0.5, 1, 0.5)`), `finalizeSpawn(EVENT)` (no leader roll), `applyRaidBuffs`, on the ground. `getEnchantOdds`: raid omen level 2: 0.1, 3: 0.25, 4: 0.5, 5: 0.75. Buffs: within the odds (`nextFloat()` of the raider) a pillager's crossbow gets `enchantment_provider/raid/pillager_post_wave_3` (quick charge I) after wave 3 and `pillager_post_wave_5` (quick charge II) after wave 5; a vindicator gets a new iron axe, within the odds with `raid/vindicator` (sharpness I) or after wave 5 `raid/vindicator_post_wave_5` (sharpness II); evokers, witches and ravagers get nothing.
  - `findRandomSpawnPos(level, attempts)`: `i = cooldown / 20`, `f = 0.22 * i - 0.24`, a start angle `nextFloat() * 2 pi`, each attempt `+ pi * j / 8`: `x = centre.x + floor(cos * 32 * f) + nextInt(3) * floor(f)`, the same for z, y the `WORLD_SURFACE` height; skipped more than 96 blocks above or below the centre, in a village after 7 seconds of cooldown, without the chunks 10 blocks around, outside entity ticking, or where a ravager cannot stand (`RAVAGER_SPAWN_PLACEMENT_TYPE`) unless it is air on snow.
  - `playSound`: one `nextLong` seed; to every player of the level within 64 blocks across (or on the bar), `event.raid.horn` (`NEUTRAL`, volume 64, pitch 1) 13 blocks from the player toward the spawn point, at the player's height.
  - `updateRaiders`: a raider removed, in another dimension or 112 blocks from the centre leaves (its health out of the total); one older than 600 ticks outside a village with `noActionTime` above 2400 counts `ticksOutsideRaid` and leaves at 30. `removeFromRaid` drops it from its wave and clears its raid.
  - `Raider`: `aiStep` joins a raider that can (`Raids.canJoinRaid`: alive, `CanJoinRaid`, `noActionTime` at most 2400) to the active raid within 96 blocks every 20 game ticks (at the raid's current wave, with its health); one in a raid hunting a player or an iron golem keeps `noActionTime` at 0. `die` drops its wave's leader and adds a killing player to the heroes. In a raid it never despawns (`removeWhenFarAway`, `requiresCustomPersistence`). Goals 1 `ObtainRaidLeaderBannerGoal`, 3 `PathfindToRaidGoal` (no target, no controlling passenger, an active raid not over, outside a village: walks toward the centre's bottom centre by `DefaultRandomPos.getPosTowards(15, 4, pi/2)` at speed 1 whenever its path is done, and every 20 ticks recruits the raiders within its box grown 16 that have no active raid and can join), 4 `RaiderMoveThroughVillageGoal(1.05, 1)` (a random unvisited `HOME` POI within 48 (`Util.toShuffledList`), the last two visits remembered, `getPosTowards(16, 7, pi/10)` then `(8, 7, pi/2)`), 5 `RaiderCelebration` (alive, no target, a lost raid: `celebrating`, the celebrate sound one tick in 50, a jump one in 25 when not riding). `Witch.canBeLeader` and `Ravager.canBeLeader` are false; `Raider.isCaptain` is the ominous banner on a patrol leader's head, so a wave's captain drops an ominous bottle (the raider loot tables).
- **Changed:**
  - New `crates/minecraft_terrain/src/raids.rs`: `Raids` and `Raid` ported from `Raids` and `Raid` (the wave table, bonus spawns, enchant odds, spawn points, horns, the bar's players and names, the raiders' upkeep, joining and recruiting, deaths and heroes, victory, defeat and the celebration, `raids.dat` read and write) on a `RaidLevel` seam.
  - `minecraft_terrain/src/server.rs`: the level's `Raids` beside the custom spawners, read from and saved to `dimensions/minecraft/<dimension>/data/minecraft/raids.dat`, ticked before the entities (`tick_raids`): a player's bad omen turns into raid omen in a village, a raid omen that ran out starts or grows a raid, the raids tick, and each player's bars, horns and hero effect go back in `Output`; `RaidSim` gives the raids the level (chunks, entity ticking, the world surface, the ravager's spawn check, `gameplay/can_start_raid`, the village POIs, player UUIDs) and spawns raid members with their riders through `spawn_saved_all`; the `raids` game rule; `TickInput.omens`, carried across a batch of ticks so a fired raid omen is never lost.
  - `minecraft_terrain`: `illager_save.rs` (`read_raider` and `raider_fields`, now shared by illagers, ravagers and witches), `ravager_save.rs` (uses them), `server_mobs.rs` (witches load and save their raider state; a raider out of its raid saves no `RaidId`), `lib.rs`, `eye_of_ender.rs` (test input).
  - New vendored `entities/src/raid.rs` (`RaidView`, the raid-in-range lookup, `PathfindToRaidGoal`, `MoveThroughVillageGoal`, `CelebrationGoal`) and `entities/src/world/raiders.rs` (`RaiderView` and `RaiderMut` over illagers, ravagers and witches, the celebrate sounds). Vendored `entities`: `illager_ai.rs` (the raider goals in their vanilla slots, the raid in `IllagerState`), `monster_ai.rs` (witches take the raider goals), `witch.rs` (the raider state), `world.rs` (the level's raid views; raiders in a raid never despawn), `world/{illagers,ravagers,witches}.rs` (the raid each tick, the POIs lent to the goals, celebrating and its sound), `lib.rs`. Vendored `world`: `level/spawning.rs` (`raid_member`, `raid_spawn_ok`), `natural_spawner/spawning.rs` (a raid member's tag: `finalizeSpawn(EVENT)`, wave, raid, leader with the banner, `applyRaidBuffs`), `level/time.rs` (`can_start_raid_at`), `storage.rs` (`dimension_dir`).
  - `minecraft_survival`: `survival.rs` (`turn_bad_omen`, the raid omen position and its firing on the omen's last tick, `become_hero` for 48000 ticks), `player.rs` and `player_nbt.rs` (`raid_omen_position` saved and read like vanilla), `server.rs` (read at join), `entities.rs` (`PlayerView.omens` into the level's tick input; raid omens, heroes, horns and bars routed to each player), `session.rs` (a friend's raid bars as `ToClient::BossBar` with ids `0x100 + raid id`: added red and notched 10, renamed, progress, removed), `heard.rs` (only its player hears the horn); test inputs in `actions.rs`, `level.rs`, `remote.rs`, `the_end/dragon_fight.rs`.
  - `render_anim`: new `minecraft_raids.rs` (the host's raid bars as `McBossBar`s with vanilla's names); `minecraft_players.rs` (raid omen with its sound, hero of the village, the horn at volume 64, each player's bars), `minecraft_world.rs`, `minecraft_host.rs` (bars to friends), `minecraft_remote.rs` (a friend's bars keep their colour and overlay), `minecraft_gui.rs` (all `boss_bar/` sprites), `lib.rs`.
  - `frame/src/minecraft_ui.rs` (`MinecraftUi.raid_bars`; `McBossBar` colour, overlay and key), `hud/src/minecraft_boss.rs` (a bar draws its colour and its notches, in vanilla's order: background, notches, progress, notch progress), `minecraft_net/src/messages.rs` (`BossBarColor` and `BossBarOverlay` sprite names; no new message, protocol 5 kept).
- **Tests:**
  - `minecraft_terrain`: `raids::tests` (`waves_per_difficulty_follow_the_raider_table` (the table, Easy 3, Normal 5, Hard 7, the bonus wave reading the last wave's column, bonus spawns, enchant odds, leaders), `an_easy_raid_has_three_waves_and_a_raid_omen_above_one_adds_a_bonus_wave` (one captain a wave, never a ravager), `raids_dat_reads_back_what_it_wrote` (every field, vanilla's first id 2), `bad_omen_turns_into_raid_omen_only_inside_a_village` (not outside, not for spectators, not on Peaceful, not at a raid already at level 5, not with `raids` off), `a_won_raid_makes_every_player_who_killed_raiders_a_hero` (host and friend heroes at the omen level less one, the friend sees the bar, a far player neither bar nor horn, "Raid - Victory" at 0, gone after the celebration), `a_village_left_without_its_centre_is_a_lost_raid` ("Raid - Defeat"; a raid with no wave yet stops), `a_raid_is_lost_once_the_villagers_holding_the_village_are_dead`, `raiders_that_wander_into_a_raid_join_its_current_wave` (and a killer becomes a hero)); `server::tests::a_raid_omen_in_a_village_brings_waves_of_raiders_with_a_bar_and_a_horn` (generated terrain: the omen turns at the player's block, a fired raid omen starts a raid, the first wave of 4 or 5 crossbow pillagers with one captain, the horn and the bar, saved with `RaidId`, `raids` off stops it), `server::tests::raid_members_carry_their_wave_raid_and_raid_buffs` (sharpness II after wave 5, quick charge, the captain's banner, `pos + (0.5, 1, 0.5)`), `server::tests::raids_are_saved_in_the_dimension_s_data_folder_and_read_back`; `server_mobs::tests::a_witch_saves_and_loads_its_raid_and_forgets_a_raid_it_left`.
  - Vendored entities: `raid::tests` (`the_nearest_active_raid_within_96_blocks_is_the_one_here`, `shuffling_swaps_from_the_back_as_util_shuffle_does`), `world::raiders::tests` (`every_raider_kind_celebrates_with_its_own_sound`, `pillagers_ravagers_and_witches_are_listed_with_their_raid`, `raiders_walk_from_outside_the_village_towards_the_raid_centre` (recruiting on the way), `raiders_with_no_raid_stay_put`, `raiders_celebrate_a_lost_raid_with_their_own_sound`, `a_raider_in_a_raid_never_despawns_far_from_players`).
  - `minecraft_survival`: `survival::tests::bad_omen_turns_into_a_raid_omen_that_fires_once_where_it_was_given` (600 ticks, fired once, nothing from a raid omen given by `/effect`, hero for 48000), `player_nbt::tests::the_raid_omen_position_is_saved_and_read_back`, `session::tests::a_friend_gets_the_raid_bar_red_and_notched_with_its_name_and_progress`.
  - `render_anim`: `minecraft_raids::tests::the_host_sees_a_red_notched_raid_bar_named_like_vanilla`. `hud`: `minecraft_boss::tests::a_bar_draws_its_colour_and_its_notches_over_it`.
  - Runs: `minecraftoss-entities` lib 365, `minecraftoss-world` lib 96, `minecraft_terrain` lib 1331 of 1334 (3 ignored), `minecraft_survival` lib 723, `render_anim` lib 178, `hud` lib 186, all green. `check -p launcher` green. The new `raids.rs`, `entities/src/raid.rs` and `entities/src/world/raiders.rs` pass `rustfmt --check --edition 2021`, `render_anim/src/minecraft_raids.rs` passes it with edition 2024.
- **Decisions:**
  - Each level's raids live on its level thread in `ServerSim` (`minecraft_terrain/src/raids.rs`), keyed by raid id and saved in that dimension's `data/minecraft/raids.dat`: vanilla's saved data id is `minecraft:raids`, so in the 26.3 layout the file sits under `data/minecraft/` like `wandering_trader.dat`, not in `data/` itself. It is written on every save, empty too, as vanilla's new `Raids` is dirty. Heroes are kept by player UUID; nothing keys on player 0.
  - Bad omen turns into raid omen in two steps: each tick the player thread hands its bad omen level to the level (`TickInput.omens`), the level answers when the player stands in a village with room in the raid there, and the player thread swaps the effects (a tick later than vanilla's single step). A raid omen that runs out hands its level and position to the level the same way; a raid omen from `/effect` has no position and starts nothing, as in vanilla.
  - A raider's killer is the player in its `hurt_by` (the last player to hurt it, within 100 ticks) when it dies; vanilla credits the killing blow's player, so a raider a player hurt and fire then killed makes that player a hero here.
  - Raid members are written as tags (vanilla's `finalizeSpawn(EVENT)` and `applyRaidBuffs`, with `Wave`, `RaidId`, `CanJoinRaid` and the leader's banner) and loaded through `spawn_saved_all` like eggs and summons, so a ravager's rider loads mounted; the buff's `nextFloat` comes from a random seeded by the new UUID, as the live entity's random only starts once it loads. The raid enchantment providers' levels (quick charge I and II, sharpness I and II) were read from the 26.3 data pack and are used as numbers.
  - A ravager in a raid walks to the village itself even with an illager on its back. Vanilla's rider steers it through the vehicle's navigation and the ravager's own `PathfindToRaidGoal` waits while it has a controlling passenger; the fork keeps group ravagers' choice that an illager rider leaves the ravager its own goals. A celebrating raider on a ravager may jump with it (vanilla's celebration skips the jump while riding; the goals do not know about riding).
  - The friend's bar id is `0x100 + raid id` (the dragon's lies below), one per raid; the host draws raid bars after the dragon's, as vanilla's overlay lists bars in the order they were added.
  - The horn goes to each player alone (`heard.rs` actor-only), at vanilla's spot 13 blocks toward the raiders, volume 64.
  - The last seen health of each raider is kept (not saved), so a raider that vanishes without dying (unloaded) takes its health out of the bar's total as vanilla's `removeFromRaid` does.
- **Test steps (survival, Normal, a village with villagers in their beds and at their bell, by day; diamond armour on):**
  1. (D4 step 4.) `/locate structure minecraft:village_plains` and `/tp` there. Outside the village drink an ominous bottle (level I): the Bad Omen icon shows. Walk among the houses: Bad Omen turns into Raid Omen with its sound (**153_raid_omen**).
  2. Wait 30 seconds: Raid Omen ends and a red bar notched in ten, "Raid", appears at the top and fills over 15 seconds. The raid horn sounds from the side the raiders come from and the first wave arrives: 4 or 5 pillagers with crossbows, one wearing the ominous banner (**154_raid_bar**).
  3. The raiders walk into the village and go from house to house, shooting villagers, iron golems and you. The bar empties as you hurt them; with two left it reads "Raid - Raiders Remaining: 2" (**155_raiders_remaining**). Kill the captain: it drops the banner and an ominous bottle.
  4. Each later wave comes 15 seconds after the last raider falls, with the horn and a refilled bar: wave 2 vindicators and pillagers, wave 3 pillagers and a ravager, wave 4 pillagers, a vindicator and witches, wave 5 vindicators, pillagers, an evoker and a ravager with a pillager on its back (Normal has 5 waves; `/difficulty easy` gives 3, Hard 7).
  5. Win: two seconds after the last raider falls the bar reads "Raid - Victory" for 30 seconds; the Hero of the Village icon (level I) shows for 40 minutes and the villagers' prices drop (**156_raid_victory**).
  6. In another village drink a level III bottle (`minecraft:ominous_bottle[ominous_bottle_amplifier=2]`): the raid has a sixth, bonus wave like the fifth, some vindicator axes and pillager crossbows are enchanted, and the victory gives Hero of the Village III.
  7. Defeat: start a raid in a third village and `/kill @e[type=minecraft:villager]` after the first wave arrives: the bar reads "Raid - Defeat" and the raiders celebrate, arms up, cheering and jumping (**157_raid_defeat**).
  8. Save and quit between two waves, then load: the bar comes back and the raid goes on with its next wave; raiders still alive stay in it and never despawn.
  9. `/gamerule raids false` during a raid: the bar goes away and the raiders stay as plain mobs. `/gamerule raids true`. Drinking a bottle far from any village keeps Bad Omen until you walk into one.
  10. (D4 step 5.) Friend window: the friend in the village sees the same bar with its name and progress, hears the horn, kills raiders and after the victory has Hero of the Village too; a friend far away sees no bar (**158_friend_raid**). Villagers hiding and ringing the bell come with group raid-villagers.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:ominous_bottle[ominous_bottle_amplifier=2]*2,minecraft:milk_bucket,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:crossbow,minecraft:shield,minecraft:totem_of_undying,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32,minecraft:villager_spawn_egg*8,minecraft:bell,minecraft:spyglass
```

### 2. Witches heal raiders in Minecraft raids

After 8f46df5c.

- **Vanilla (javap, 26.3):**
  - `Witch.registerGoals` targets: 1 `HurtByTargetGoal(Raider ignored)`, 2 `NearestHealableRaiderTargetGoal(Raider, mustSee, selector: the witch has an active raid and the target is no witch)`, 3 `NearestAttackableWitchTargetGoal(Player, 10, mustSee)`.
  - `NearestHealableRaiderTargetGoal`: `cooldown` starts at 0; `canUse` is false while the cooldown is above 0 or on a failed `nextBoolean()` (drawn only off the cooldown) or without an active raid, else `findTarget` (the nearest raider in sight within follow range, by the selector); `start` sets the cooldown to `reducedTickDelay(200)` (100) and the target.
  - `Witch.aiStep` (before the goals): `decrementCooldown`, then the players goal may attack only while the cooldown is at most 0 (`NearestAttackableWitchTargetGoal.canUse`: `canAttack && super.canUse()`, no roll otherwise).
  - `Witch.performRangedAttack` at a `Raider`: healing at 4 health or less, else regeneration, the target cleared (`setTarget(null)`), aimed and thrown like the other potions (where the target will be next tick).
- **Changed:** vendored `entities`: `monster_ai.rs` (`HealRaidersGoal` finds the nearest healable raider in sight off its cooldown with a raid and sets the 100-tick cooldown, `WitchPlayersGoal` holds the players goal while the cooldown runs, `Prey::HealableRaider`, the heal throw for a raider target, `WitchState.heal_cooldown` and `raiders`), `witch.rs` (`heal`, `HEAL_COOLDOWN`), `world/witches.rs` (the cooldown in and out of the goals; the raiders' health and motion while the witch's raid is active), `world/raiders.rs` (`RaiderView.velocity`).
- **Tests:** vendored entities `world::raiders::tests::a_raid_witch_heals_a_badly_hurt_raider_and_gives_regeneration_to_a_scratched_one`, `a_witch_with_no_raid_leaves_raiders_alone`. Runs: `minecraftoss-entities` lib 367, `minecraft_terrain` lib 1331 of 1334 (3 ignored), `minecraft_survival` lib 723, all green; `check -p launcher` green; `world/raiders.rs` passes `rustfmt --check --edition 2021`.
- **Decisions:**
  - The heal goal reads the witch's own raid (`hasActiveRaid`), and only raiders other than witches are candidates, as vanilla's selector says; a raider target from any goal gets the healing throw, as vanilla's `instanceof Raider` does.
- **Test steps (in a raid of group raids' steps, Normal):**
  1. In wave 4, hurt a pillager or vindicator near a witch without killing it and step back: the witch throws a pink splash potion at it (healing when it has 2 hearts or less, else regeneration), and for 5 seconds after each heal it picks no new player to attack (**159_raid_witch_heals**).
  2. A witch from a spawn egg (no raid) beside a hurt pillager never throws at it.
- **Kit:** commit 1's, plus witch and pillager eggs for step 2:

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:ominous_bottle[ominous_bottle_amplifier=2]*2,minecraft:milk_bucket,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:crossbow,minecraft:shield,minecraft:totem_of_undying,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32,minecraft:villager_spawn_egg*8,minecraft:bell,minecraft:spyglass,minecraft:witch_spawn_egg*2,minecraft:pillager_spawn_egg*2,minecraft:wooden_sword
```

### 3. Track D's raid test passes clippy

After fb7bb5df.

- **Changed:** `crates/minecraft_terrain/src/raids.rs`: a test picks the killer with `is_multiple_of` (`clippy::manual_is_multiple_of`); no change in the game.
- **Tests:** `raids::tests` (8) green. Clippy on `minecraftoss-entities`, `minecraftoss-world`, `minecraft_terrain`, `minecraft_survival`, `render_anim` and `hud` (`--all-targets`, `-A clippy::approx_constant`) finds nothing else in group raids' code. The file passes `rustfmt --check --edition 2021`.
- **Test steps:** none; commits 1 and 2 hold.
- **Kit:** commit 2's.

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:ominous_bottle[ominous_bottle_amplifier=2]*2,minecraft:milk_bucket,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:crossbow,minecraft:shield,minecraft:totem_of_undying,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32,minecraft:villager_spawn_egg*8,minecraft:bell,minecraft:spyglass,minecraft:witch_spawn_egg*2,minecraft:pillager_spawn_egg*2,minecraft:wooden_sword
```

## Group raid-villagers

### 1. Villagers hide and ring bells in Minecraft raids, and heroes get gifts

After 47c4dcbc.

- **Vanilla (javap, 26.3, and the 26.3 data pack):**
  - `Villager.BRAIN_PROVIDER` adds, after panic, `PRE_RAID` (`getPreRaidPackage`), `RAID` (`getRaidPackage`) and `HIDE` (`getHidePackage`), none with memory requirements.
  - `getCorePackage`: `ReactToBell` and `SetRaidStatus` at priority 0 (after `WakeUp`). `ReactToBell` (one-shot, `HEARD_BELL_TIME` present): with no raid at the villager's block (`ServerLevel.getRaidAt`), `setActiveActivityIfPossible(HIDE)`. `SetRaidStatus`: `level.random.nextInt(20) == 0`, then with a raid at its block: `RAID` once the first wave has spawned and it is not between waves, else `PRE_RAID`, as the default activity and the active one.
  - `getPreRaidPackage(0.5)`: 0 `RingBell`; 0 `TriggerGate.triggerOneShuffled` of `SetWalkTargetFromBlockMemory(MEETING_POINT, 0.75, 2, 150, 200)` weight 6 and `VillageBoundRandomStroll(0.75)` weight 2; 5 the minimal look; 99 `ResetRaidStatus`.
  - `getRaidPackage(0.5)`: 0 `sequence(triggerIf(raidExistsAndNotVictory), triggerOneShuffled(MoveToSkySeeingSpot(0.5) weight 5, VillageBoundRandomStroll(0.55) weight 2))` (despite its name, `raidExistsAndNotVictory` answers raid present and won); 0 `CelebrateVillagersSurvivedRaid(600, 600)`; 2 `sequence(triggerIf(raidExistsAndActive), LocateHidingPlace(24, 0.7, 1))` (active, neither won nor lost); 5 the minimal look; 99 `ResetRaidStatus`.
  - `getHidePackage(0.5)`: 0 `SetHiddenState(15, 3)`; 1 `LocateHidingPlace(32, 0.625, 2)`; 5 the minimal look.
  - `ResetRaidStatus`: `level.random.nextInt(20) == 0`, then with no raid at its block, or one stopped or lost: the default activity `IDLE` and `updateActivityFromSchedule`.
  - `RingBell` (`MEETING_POINT` present): `level.random.nextFloat() > 0.95`, the meeting point closer than 3 to its block (`BlockPos.closerThan`, block distance squared under 9), and a bell there: `BellBlock.attemptToRing(null, level, pos, null)`.
  - `LocateHidingPlace(radius, speed, closeEnough)` (`WALK_TARGET` absent): the first `HOME` point of interest (any occupancy) within `closeEnough + 1` whose centre is closer than `closeEnough`, else `PoiManager.getRandom` of the homes within `radius` (`Util.toShuffledList` with the villager's random), else its `HOME` memory; found: `PATH`, `LOOK_TARGET`, `BREED_TARGET` and `INTERACTION_TARGET` erased, `HIDING_PLACE` set, and a walk target there unless its centre is already closer than `closeEnough`.
  - `SetHiddenState(15, 3)` (`HIDING_PLACE` and `HEARD_BELL_TIME` present): `HIDE_TIMEOUT` 300; while it has counted at most 300 hidden ticks and the bell is no more than 300 ticks old, a tick within 3 of the hiding place counts; else both memories are erased, `updateActivityFromSchedule`, and the count restarts. `HIDING_PLACE` and `HEARD_BELL_TIME` have no codec (never saved).
  - `MoveToSkySeeingSpot(speed)` (`WALK_TARGET` absent): nothing when the villager's block sees the sky; else ten tries (its random) at its block plus `nextInt(20) - 10`, `nextInt(6) - 3`, `nextInt(20) - 10`; the first where `hasNoBlocksAbove` (`canSeeSky` and the `MOTION_BLOCKING` height at most the villager's Y) becomes the walk target (its bottom centre, close enough 0).
  - `CelebrateVillagersSurvivedRaid(600, 600)`: starts with a won raid at its block and no blocks above it; runs while that raid is not stopped; each tick (its random) one in 100 `playCelebrateSound` (`entity.villager.celebrate` at the voice pitch) and one in 200, with no blocks above, a firework rocket at its eyes (`DyeColor` by `Util.getRandom`, flight `nextInt(3)`, one `BURST` explosion of the dye's firework colour); stopping runs `updateActivityFromSchedule`.
  - `GiveGiftToHero(100)` at priority 3 in the work, meet and idle packages: `THROW_GIFT_AT_DISTANCE` 5, the first wait 600 ticks, then `600 + level.random.nextInt(6001)`; it needs `NEAREST_VISIBLE_PLAYER` with `HERO_OF_THE_VILLAGE`, counts its wait down only while such a hero is seen, then (`Behavior` duration 100) looks at the hero, walks to within 5 (`setWalkAndLookTargetMemories(0.5, 5)`) and, more than 20 ticks after starting, within 5 blocks (`BlockPos.closerThan`), throws the gift (`dropFromGiftLootTable`: `gameplay/hero_of_the_village/<profession>_gift`, babies `baby_gift`, unemployed and nitwits `unemployed_gift`; `BehaviorUtils.throwItem` at 0.3 towards the hero's feet from eye height less 0.3, pickup delay 10); stopping erases `INTERACTION_TARGET`, `WALK_TARGET` and `LOOK_TARGET`.
  - `Villager.customServerAiStep`: with AI, one tick in 100 (its random) in an active raid not over, entity event 42 (the client's splash particles, the villagers' sweat).
  - `BellBlock`: `useWithoutItem` (main hand only, through `ServerPlayerGameMode.useItemOn`) and `onProjectileHit` call `onHit`; `isProperHit`: not the top or bottom face and the hit at most 0.8124 above the block; a floor bell from the faces on its facing axis, a wall bell (single or double) from the other axis, a ceiling bell from any side. `attemptToRing`: `BellBlockEntity.onHit` (block event 1 with the direction, the state's facing when none), `block.bell.use` (`BLOCKS`, volume 2, pitch 1) and `BLOCK_CHANGE`. `neighborChanged`: a new signal rings it; `powered` follows the signal.
  - `BellBlockEntity`: `DURATION` 50, `GLOW_DURATION` 60, `MIN_TICKS_BETWEEN_SEARCHES` 60, `MAX_RESONATION_TICKS` 40, `TICKS_BEFORE_RESONATION` 5, `SEARCH_RADIUS` 48, `HEAR_BELL_RADIUS` 32, `HIGHLIGHT_RAIDERS_RADIUS` 48. `triggerEvent(1, dir)`: `updateEntities` (the living entities in the bell's box inflated by 48, searched again only when 60 ticks have passed; each alive one within 32 of the bell's centre gets `HEARD_BELL_TIME`), then resonation ticks 0, ticks 0, shaking. The ticker: shaking counts ticks, at 50 it stops; from 5 ticks with no resonance yet and a raider (`#raiders`) of the list within 32, it resonates with `block.bell.resonate` (`BLOCKS`, 1, 1); after 40 resonation ticks the server gives every raider of the list within 48 `GLOWING` for 60 ticks (the client draws `ENTITY_EFFECT` particles from the bell to each one instead). It saves nothing.
  - `DyeColor` firework colours, in order: 15790320, 15435844, 12801229, 6719955, 14602026, 4312372, 14188952, 4408131, 11250603, 2651799, 8073150, 2437522, 5320730, 3887386, 11743532, 1973019. `MobEffects.GLOWING` colour 9740385.
- **Changed:**
  - Vendored `entities`: `villager_brain/behaviors.rs` (the pre-raid, raid and hide packages; `ReactToBell`, `SetRaidStatus` reading the raid, `ResetRaidStatus`, `RingBell`, `LocateHidingPlace`, `SetHiddenState`, `MoveToSkySeeingSpot`, `CelebrateVillagersSurvivedRaid`, `GiveGiftToHero` in the work, meet and idle packages, the raid checks as a sequence; the unused `Never` stand-in removed), `villager_brain/mod.rs` (`Ctx.raids`, `Seen.hero`, `Remote::RingBell`, `HeroGift` and `Firework`), `villager_brain/memory.rs` (`hiding_place`), `world/villagers.rs` (the level's raids to the brain, players' Hero of the Village, the remotes into `RaidVillagerEvents`, entity event 42), `world/raiders.rs` (`RaidVillagerEvents`, bell listeners, hearing and the raiders near a bell, `glowing`), `raid.rs` (`RaidView.victory`, `stopped` and `between_waves`), `effects.rs` and `potion.rs` (`MobEffect::Glowing`), `loot.rs` (`roll_hero_gift`), `world.rs`. Vendored `player`: `lib.rs` (`World::motion_blocking_height`). Vendored `world`: `level/redstone.rs` (`Kind::Bell`: a new signal rings it; `ring_bell` with its block event and sound), `level/piston.rs` (the bell's block event into `bell_rings`), `level/mod.rs`.
  - New `crates/minecraft_terrain/src/bells.rs`: `Bells` per level (`BellBlockEntity`'s ring, listeners, shaking, resonance and glow), `proper_hit`, and the entity world as the bell's listeners.
  - `minecraft_terrain`: `server.rs` (the level's bells: rung bells heard after the level's block events, ticked after the mobs with the resonance sound; villagers' rings, hero gifts thrown from the loot tables; `Command::RingBell`), `server_mobs.rs` (the `MOTION_BLOCKING` height), `mob_tracking.rs` (`MobShow.glowing`, for friends), `raids.rs` (the raid's new view fields), `potions.rs` (a test, now that glowing exists on mobs), `lib.rs`.
  - `minecraft_survival`: `actions.rs` (a main-hand use on a bell's proper side rings it, for the host and friends alike), `entities.rs` (`ring_bell`).
- **Tests:**
  - Vendored entities: `world::raiders::tests` (`villagers_who_hear_a_bell_hide_at_home_until_it_has_been_quiet_for_15_seconds`, `before_a_raid_villagers_ring_their_bell_and_once_it_starts_they_hide` (and event 42), `villagers_celebrate_a_won_raid_with_fireworks_and_never_a_lost_one`, `a_farmer_throws_a_hero_of_the_village_a_gift_and_nobody_else`, `a_bell_finds_villagers_and_raiders_and_makes_the_raiders_glow`), `loot::tests::every_hero_gift_table_rolls_vanillas_gifts` (26.3 jar).
  - `minecraft_terrain`: `bells::tests` (`a_ring_wakes_the_villagers_within_32_blocks`, `the_listeners_are_searched_again_only_after_three_seconds`, `raiders_near_a_rung_bell_resonate_and_glow_after_two_seconds`, `a_bell_without_raiders_only_shakes_and_a_broken_one_is_forgotten`, `a_bell_rings_from_its_sides_below_its_rim`), `server::tests::a_rung_bell_wakes_the_villagers_and_lights_up_the_raiders` (generated terrain: a player's ring and its sound, the villager hearing it, one resonance, the near pillager glowing for friends too, a redstone block ringing it, the glow gone after 60 ticks).
  - Runs: `minecraftoss-entities` lib 373, `minecraftoss-player` lib 198, `minecraftoss-world` lib 96, `minecraft_terrain` lib 1337 of 1340 (3 ignored), `minecraft_survival` lib 723, all green; all targets of the touched crates check; `check -p launcher` green. `bells.rs` passes `rustfmt --check --edition 2021`, as does `world/raiders.rs`.
- **Decisions:**
  - Each level's bells live on its level thread (`ServerSim.bells`), keyed by position: vanilla's `BellBlockEntity` saves nothing, so neither does the fork. A bell's entry goes once it is quiet and its listener list is older than 60 ticks (vanilla would search again then anyway), or when it is no longer a bell.
  - A ring is heard after the level's block events (vanilla's `triggerEvent`), so a villager's ring is heard on the next tick, as in vanilla. The listener list holds villagers and raiders only, the only living things a bell reaches (`HEARD_BELL_TIME` is a villager memory, the glow is for `#raiders`).
  - The villagers see their level's raids as `RaidView`s each tick; vanilla's `CelebrateVillagersSurvivedRaid` keeps the `Raid` object, the fork keeps its id and looks it up (gone counts as stopped).
  - The hero is a player with Hero of the Village in the level's player list (`set_player_hero`, per player as the effect stands).
  - Glowing is a mob effect in the entity world (`MobEffect::Glowing`), with vanilla's effect colour for its swirls, and reaches friends as `MobShow.glowing` (no new message, protocol 5). The outline around a glowing mob is drawn by a later commit of this group.
  - The bell's sounds go through the level's sounds (the `ambient` category the fork gives level sounds; vanilla plays them as `BLOCKS`).
  - Entity event 42 (the sweat) is sent; the client draws no entity-event particles yet.
  - A celebration's firework launches are recorded but not launched yet (the rocket comes with the next commit of this group); the draws match vanilla's.
- **Test steps (survival, Normal, by day, a plains village with beds, a bell and villagers):**
  1. (D4 step 4.) `/locate structure minecraft:village_plains` and `/tp` there. Right-click the bell from a side (below its rim): it rings with the bell sound, and every villager within 32 blocks runs into a nearby house with a bed and stays there; about 15 seconds after the last ring they come out again (**160_bell_hide**).
  2. Right-click the top of the bell: it does not ring (a block in hand is placed instead). Crouch with an item in hand: it does not ring.
  3. Place a lever next to the bell and switch it on: the bell rings; off and on again rings it again.
  4. Hatch two pillagers about 20 blocks from the bell and ring it: about a quarter second later a second, humming sound plays (the resonance), and two seconds later both pillagers show pale olive swirls (Glowing) for 3 seconds (**161_bell_glow**). A pillager 60 blocks away gets none.
  5. Drink an ominous bottle and walk into the village: while the raid's bar fills before the first wave, villagers gather at the bell and now and then ring it; when the wave comes they run into houses and stay (**162_raid_hiding**).
  6. Win the raid: villagers step out under the open sky and cheer now and then.
  7. With Hero of the Village, stay near a farmer: within about 30 seconds it walks up and throws you bread, pumpkin pie or a cookie; children throw poppies, unemployed villagers wheat seeds; each villager gives again only after 30 seconds to 5 minutes (**163_hero_gift**).
  8. Friend window: the friend rings the bell the same way (the villagers hide), sees the glow swirls on the pillagers and, as a hero, gets gifts too (**164_friend_gift**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:milk_bucket,minecraft:bell*2,minecraft:villager_spawn_egg*8,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg*2,minecraft:redstone_block*2,minecraft:lever*2,minecraft:white_bed*4,minecraft:composter*2,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32,minecraft:spyglass,minecraft:oak_planks*64,minecraft:oak_door*2
```

### 2. Villagers launch fireworks after a won Minecraft raid

After 0d36426a.

- **Vanilla (javap, 26.3):**
  - `FireworkRocketEntity(level, owner, x, y, z, stack)` (the celebration's): `life` 0; flight `1 + Fireworks.flightDuration`; `setDeltaMovement(random.triangle(0, 0.002297), 0.05, random.triangle(0, 0.002297))`; `lifetime = 10 * flight + random.nextInt(6) + random.nextInt(7)`; the entity's size 0.25.
  - `tick` (not attached, not shot at an angle): the motion times 1.15 across (1.0 after a horizontal collision) plus 0.04 up; the hit along the move (`ProjectileUtil.getHitResultOnMoveVector`), the move, then a block or entity hit explodes it (`onHitBlock` only with explosions); at `life == 0` `entity.firework_rocket.launch` (`AMBIENT`, volume 3, pitch 1); `life++`; on the client a `FIREWORK` spark every tick at the rocket with `(gaussian * 0.05, -motionY * 0.5, gaussian * 0.05)`; past `lifetime` the server explodes it.
  - `explode`: entity event 17, `EXPLODE`, `dealExplosionDamage`, discard. `dealExplosionDamage`: `5 + 2 * explosions` (7 for one); every living entity in the box grown by 5 whose feet are within 5 of the rocket and that one of two rays (to its feet and to half its height, `ClipContext.Block.COLLIDER`) reaches takes `7 * sqrt((5 - distance) / 5)` of `fireworks` (the rocket direct, its owner causing). `fireworks` is in `#is_explosion` and `#panic_causes`, scales with difficulty when a living non-player causes it, and its knockback runs from the rocket (`calculateHorizontalHurtKnockbackDirection`). Its death message is `death.attack.fireworks`.
  - Client (`FireworkParticles.Starter`, event 17 through `ClientLevel.createFireworks`): the blast at once, `entity.firework_rocket.blast` or `blast_far` beyond 16 blocks of the camera, `AMBIENT`, volume 20, pitch `0.95 + random * 0.1`; a `BURST`: two base gaussians times 0.05, then 70 sparks at the rocket with `(motionX * 0.5 + gaussian * 0.15 + baseX, motionY * 0.5 + random * 0.5, motionZ * 0.5 + gaussian * 0.15 + baseZ)`, each a random colour of the explosion's; then a `FLASH` of the first colour. `SparkParticle` (`SimpleAnimatedParticle`, gravity 0.1, friction 0.91, full bright): size times 0.75, lifetime `48 + nextInt(12)`, alpha 0.99, sprite by age (`spark_7` to `spark_0`), fading after half its life. `OverlayParticle` (the flash): lifetime 4, size `7.1 * sin((age + partial - 1) * 0.25 * pi)`, alpha `0.6 - (age + partial - 1) * 0.25 * 0.5`, translucent.
- **Changed:**
  - New `crates/minecraft_terrain/src/firework_rocket.rs` (the rocket: launch, flight, block hits, its burst, the blast's damage, its view as the firework rocket item) and `crates/minecraft_terrain/src/firework_particles.rs` (sparks, trails, the flash and the blast's sound).
  - `minecraft_terrain`: `server.rs` (the level's rockets launched from the celebrations, ticked after the bells with the launch sound; a burst hurts mobs as a blast and players as a `Fireworks` hit by the launching villager; rockets in the snapshot's thrown views and bursts in `EntitySnapshot.firework_bursts`), `mesh.rs` (the spark and flash sprites in the atlas), `lib.rs`.
  - `minecraft_survival`: `entity_view.rs` (bursts with their bang, trails behind rockets, sparks and flashes drawn translucent), `mob_sender.rs` (bursts reach friends with the other bursts), `entities.rs` (a `Fireworks` hit knocks the player away from the rocket and can be blocked by a shield), `death.rs` (`MobAttack::Fireworks` and `DamageCause::Fireworks`: `minecraft:fireworks`, "went off with a bang").
  - Vendored `entities`: `world.rs` (`PlayerHitKind::Fireworks`), `bin/check_zombie_player_hits.rs`, and the launching villager in `Remote::Firework` and `FireworkLaunch` (`villager_brain/{behaviors,mod}.rs`, `world/{villagers,raiders}.rs`).
- **Tests:**
  - `minecraft_terrain`: `firework_rocket::tests` (`a_rocket_lives_ten_ticks_a_flight_level_and_up_to_eleven_more`, `a_rocket_speeds_up_its_climb_and_explodes_once_its_life_is_over`, `a_rocket_that_meets_a_roof_explodes_there`, `its_blast_hurts_what_is_in_reach_and_in_the_open`), `firework_particles::tests` (`a_burst_throws_seventy_sparks_of_its_colour_with_a_flash_and_a_bang`, `sparks_fall_fade_and_go_while_the_flash_lasts_four_ticks`, `a_rocket_leaves_a_white_trail_that_stops_on_the_ground`, `spark_sprites_run_from_the_largest_to_the_smallest`), `server::tests::a_villager_s_rocket_climbs_bursts_under_a_roof_and_hurts_who_is_near` (generated terrain: the launch sound, the burst under the roof, the player's hit by the villager, the pillager below hurt).
  - `minecraft_survival`: `death::tests::a_villager_s_firework_makes_the_player_go_off_with_a_bang`, `entity_view::tests::a_firework_burst_bangs_and_sparkles_for_whoever_sees_it`.
  - Runs: `minecraftoss-entities` lib 373, `minecraft_terrain` lib 1346 of 1349 (3 ignored), `minecraft_survival` lib 725, all green; `check -p launcher` green; the entities bins check. The two new files pass `rustfmt --check --edition 2021`.
- **Decisions:**
  - A rocket lives on its level's thread (`ServerSim.rockets`) and is not saved: vanilla saves `Life`, `LifeTime` and its item, but a celebration's rocket lives two seconds at most.
  - A rocket explodes on a block in its path; it does not yet explode on an entity in its path (vanilla's `onHitEntity`), which a rocket launched straight up rarely meets.
  - Each rocket draws from its own random, seeded from the level's sound random (vanilla's entity randoms are unseeded).
  - The burst hurts mobs through the explosion route (`player_blast` with no push, so no knockback for mobs) and players as a `Fireworks` hit caused by the launching villager (knocked away from the rocket, scaled by difficulty, blockable by a shield); a villager gone since gives the sourceless cause.
  - Rockets reach the clients as thrown views of the firework rocket item, drawn facing the camera like thrown items (vanilla's `FireworkEntityRenderer` draws the item the same way), so friends see them with no new message; bursts ride `EntitySnapshot.firework_bursts` like the other bursts. Protocol 5.
  - A spark stops when it would enter a block with collision boxes (vanilla collides with the shapes axis by axis); sparks are full bright, the flash takes the light where it is.
  - The bang is chosen on each client by its own camera distance (16 blocks), as vanilla's client does.
- **Test steps (in a raid of commit 1's steps, Normal, by day):**
  1. Win the raid and watch the villagers out in the open: now and then one launches a firework rocket from its head with a whoosh; it climbs with a white trail and bursts into sparks of one colour with a flash and a bang (the bang muffled from further than 16 blocks) (**165_raid_fireworks**).
  2. Stand close to a celebrating villager: a low burst can hurt you a little, knocking you away from it; dying to it reads "<name> went off with a bang".
  3. Friend window: the friend sees and hears the same rockets and bursts (**166_friend_fireworks**).
- **Kit:** commit 1's.

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:milk_bucket,minecraft:bell*2,minecraft:villager_spawn_egg*8,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg*2,minecraft:redstone_block*2,minecraft:lever*2,minecraft:white_bed*4,minecraft:composter*2,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32,minecraft:spyglass,minecraft:oak_planks*64,minecraft:oak_door*2
```

### 3. Glowing Minecraft raiders show through walls

After 67f0254e.

- **Vanilla (26.3):** a mob with `GLOWING` (`Entity.isCurrentlyGlowing`, synced as the shared flag 6) is drawn by `LevelRenderer` into the outline buffer in its team colour (white without a team) and outlined by the `entity_outline` post effect, through every block in front of it; the effect's own swirls show as for any visible effect. The bell's glow lasts 60 ticks.
- **Changed:**
  - New `crates/minecraft_terrain/src/glow_render.rs`: a glowing mob's model copied as a white, full-bright hull a sixteenth of a block bigger on each side, and the model itself again to draw over the hull.
  - `minecraft_survival`: `entity_view.rs` (glowing pillagers, vindicators, evokers, ravagers and witches into the glow and cover meshes), `entities.rs` (`MobMeshes.glow` and `glow_cover`).
  - `render_anim`: `minecraft_world.rs` and `minecraft_remote.rs` (the two meshes to the frame, for the host and friends).
  - `render_gpu`: `drawsurf/minecraft_world.rs` (`glow_fragment`, white where the texture is not cut out; a pipeline with no depth test or depth write; after the weather and before the hand: the hull over everything, then the cover with the mobs' own depth test, so a seen raider keeps its look inside a white rim and a hidden one shows as a white shape).
- **Tests:** `minecraft_terrain`: `glow_render::tests::a_glowing_mob_gets_a_white_hull_a_sixteenth_bigger_and_a_copy_drawn_over_it`; `render_gpu`: `drawsurf::minecraft_world::tests::glowing_mobs_draw_through_walls_in_white_with_their_cover_after`. Runs: `minecraft_terrain` glow tests, `render_gpu` lib 39, `render_anim` lib 178, `minecraft_survival` lib 725, all green; `check -p launcher` green; `glow_render.rs` passes `rustfmt --check --edition 2021`.
- **Decisions:**
  - Vanilla's outline post effect is not ported (the renderer has no outline buffer or stencil); the stand-in is the white hull drawn without a depth test and the model drawn again over it: in the open a white rim around the raider, behind blocks a white shape of it. Only raiders glow in the fork (the bell is the only source), so only their models get the hull; their held items get none.
- **Test steps (by day, a village with a bell):**
  1. Hatch a pillager and a ravager about 20 blocks from the bell and hide one behind a house; ring the bell: two seconds later both glow for 3 seconds, the one in the open with a white rim, the hidden one as a white shape seen through the house (**167_glow_outline**).
  2. Friend window: the friend sees the same rims and shapes (**168_friend_glow**).
- **Kit:** commit 1's with two ravager eggs.

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:milk_bucket,minecraft:bell*2,minecraft:villager_spawn_egg*8,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg*2,minecraft:redstone_block*2,minecraft:lever*2,minecraft:white_bed*4,minecraft:composter*2,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32,minecraft:spyglass,minecraft:oak_planks*64,minecraft:oak_door*2,minecraft:ravager_spawn_egg*2
```

### 4. Track D's bells and rockets pass clippy

After 15926d5f.

- **Changed:** `crates/minecraft_terrain/src/bells.rs` (the listener list's empty fallback named, `unwrap_or_else(Vec::new)`, as the workspace's `clippy.toml` asks) and `crates/minecraft_terrain/src/server.rs` (a rocket's seed without the cast to its own type); no change in the game.
- **Tests:** `bells::tests` and the firework tests (17) green. Clippy on `minecraftoss-entities`, `minecraftoss-world`, `minecraft_terrain`, `minecraft_survival` and `render_gpu` (`--all-targets`, `-A clippy::approx_constant`) finds nothing else in group raid-villagers' code. `bells.rs` passes `rustfmt --check --edition 2021`.
- **Test steps:** none; commits 1 to 3 hold.
- **Kit:** commit 3's.

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:milk_bucket,minecraft:bell*2,minecraft:villager_spawn_egg*8,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg*2,minecraft:redstone_block*2,minecraft:lever*2,minecraft:white_bed*4,minecraft:composter*2,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32,minecraft:spyglass,minecraft:oak_planks*64,minecraft:oak_door*2,minecraft:ravager_spawn_egg*2
```

## Review (D2)

Adversarial review of everything since `r9-d2-base` (groups pillagers to raid-villagers), after 3f359895.

### 1. Track D's new illager files pass rustfmt

- **Found:** `entities/src/illager.rs`, `entities/src/illager_ai.rs` and `entities/src/world/illagers.rs` (added by groups pillagers to evokers-vexes, edited by group raids) did not pass `rustfmt --check --edition 2021` on their committed content: long calls and asserts were left on one line.
- **Changed:** the three files formatted with `rustfmt --edition 2021` (they are new in this track); no change in the game.
- **Tests:** none new; the three files now pass `rustfmt --check --edition 2021`, as do all 26 `.rs` files track D added since `r9-d2-base`.
- **Test steps:** none.
- **Kit:** group raid-villagers' last kit.

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:milk_bucket,minecraft:bell*2,minecraft:villager_spawn_egg*8,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg*2,minecraft:redstone_block*2,minecraft:lever*2,minecraft:white_bed*4,minecraft:composter*2,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32,minecraft:spyglass,minecraft:oak_planks*64,minecraft:oak_door*2,minecraft:ravager_spawn_egg*2
```

### 2. Patrolling pillagers near a raid they are not in hold their ground

After 80af480a.

- **Found:** group raids made an illager's `has_raid` true for an illager in a raid or merely within 96 blocks of an active raid (vanilla's unused `Raider.hasRaid`). The flag feeds `Raider.HoldGroundAttackGoal`, whose `canUse` (javap, 26.3) asks `getCurrentRaid() == null` only, so a patrolling pillager or vindicator that is not in the raid next door stopped holding its ground and walked up to its target.
- **Changed:** vendored `entities/src/world/illagers.rs`: `has_raid` is the illager's own raid (`getCurrentRaid() != null`). `near_raid` (vanilla's `ServerLevel.isRaided`) stays in the goal state for the vindicator's door opening, which is not built yet.
- **Tests:** vendored entities `world::illagers::tests::a_patrolling_pillager_near_a_raid_it_is_not_in_still_holds_its_ground` (fails before the fix: the pillager walked to the player). `minecraftoss-entities` lib 374, green.
- **Test steps (survival, Normal, a village, a raid running):**
  1. Hatch two pillagers about 60 blocks from the village centre (inside 96 blocks, outside the village) and make them a patrol: `/summon minecraft:pillager ~ ~ ~ {Patrolling:1b,CanJoinRaid:0b}` twice (so they cannot join the raid). Stand 14 blocks from them: they stay where they are, turn to you and one calls the other, then they come at you once you are within 10 blocks or hit one (**169_patrol_near_raid**).
- **Kit:** commit 1's.

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:milk_bucket,minecraft:bell*2,minecraft:villager_spawn_egg*8,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg*2,minecraft:redstone_block*2,minecraft:lever*2,minecraft:white_bed*4,minecraft:composter*2,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32,minecraft:spyglass,minecraft:oak_planks*64,minecraft:oak_door*2,minecraft:ravager_spawn_egg*2
```

### 3. A raider that joins a raid twice in one tick counts its health once

After 8f623fdf.

- **Found:** `Raids::tick` (`minecraft_terrain/src/raids.rs`) joined wandering raiders (vanilla's `Raider.aiStep`, every 20 game ticks) and then let recruiters call raiders near them (`PathfindToRaidGoal.recruitNearby`) from the same snapshot taken at the tick's start. A raider that walked into range on a tick that was both a multiple of 20 and a recruiter's call still looked raid-less to the recruiter and joined again: `addWaveMob` kept one copy of it but added its health to the raid's total twice, so the bar never filled to the raiders' real health. In vanilla the recruiter sees the live raider, whose `hasActiveRaid` is then true.
- **Changed:** `crates/minecraft_terrain/src/raids.rs`: the raider snapshot is read again after the wanderers join, before the recruiters call.
- **Tests:** `minecraft_terrain` `raids::tests::a_raider_that_wanders_in_and_is_recruited_on_the_same_tick_counts_once` (48 before the fix, 24 after). `raids::tests` (9) green; `check -p launcher` green.
- **Test steps:** none in game (one tick in twenty, and the bar's fill is the only sign); group raids' steps 2 and 3 hold.
- **Kit:** commit 1's.

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:milk_bucket,minecraft:bell*2,minecraft:villager_spawn_egg*8,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg*2,minecraft:redstone_block*2,minecraft:lever*2,minecraft:white_bed*4,minecraft:composter*2,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32,minecraft:spyglass,minecraft:oak_planks*64,minecraft:oak_door*2,minecraft:ravager_spawn_egg*2
```

### 4. A Minecraft ravager's roar does not fling players

After 131b07f0.

- **Found:** group ravagers' roar (`world/ravagers.rs`, `ravager_roars`) gave every player within reach the strong knockback as well as the 6 damage. `Ravager.roar` (javap, 26.3) calls `strongKnockback` only when the entity is not a `Player` (`instanceof Player; ifne`): a player takes the 6 `mob_attack` damage with its ordinary knockback and is not flung.
- **Changed:** vendored `entities/src/world/ravagers.rs`: the roar no longer queues a player push; the player's hit (a plain melee blow from the ravager) is unchanged. Mobs are still flung.
- **Tests:** vendored entities `world::ravagers::tests::a_blocked_ravager_is_stunned_for_forty_ticks_then_roars_and_flings_what_is_near` now checks that the roar hurts the player with a plain blow and queues no push (the cow and the pillager are still flung). Ravager tests (17) green; `check -p launcher` green.
- **Test steps (survival, Normal, shield in the offhand):**
  1. Hatch a ravager and block its bite with the shield until it is stunned (grey swirls). Stay within 4 blocks: when it roars you take a hit with the usual small knockback, not a long throw; a cow beside it is still thrown away (**170_ravager_roar_player**).
- **Kit:** commit 1's.

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:milk_bucket,minecraft:bell*2,minecraft:villager_spawn_egg*8,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg*2,minecraft:redstone_block*2,minecraft:lever*2,minecraft:white_bed*4,minecraft:composter*2,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32,minecraft:spyglass,minecraft:oak_planks*64,minecraft:oak_door*2,minecraft:ravager_spawn_egg*2,minecraft:cow_spawn_egg*2
```

### 5. Track D's mob list names the ravager roar fix

After 30a2d1cb.

- **Changed:** `docs/round9/mobs.md` row 13 (ravager): the roar flings every mob but itself, and players only take the blow (commit 4 left the row unchanged). The review commits touched only vendored files already listed under "Vendored files" (`entities/src/illager.rs`, `illager_ai.rs`, `world/illagers.rs`, `world/ravagers.rs`), so the counts there stand.
- **Tests:** after commits 1 to 4: `minecraftoss-entities` lib 374 and `minecraft_terrain` lib 1348 of 1351 (3 ignored), all green; `check -p launcher` green; clippy on both crates (`--all-targets`, `-A clippy::approx_constant`) finds nothing in track D's files.
- **Test steps:** none; commit 4's step holds.
- **Kit:** commit 4's.

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:milk_bucket,minecraft:bell*2,minecraft:villager_spawn_egg*8,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg*2,minecraft:redstone_block*2,minecraft:lever*2,minecraft:white_bed*4,minecraft:composter*2,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32,minecraft:spyglass,minecraft:oak_planks*64,minecraft:oak_door*2,minecraft:ravager_spawn_egg*2,minecraft:cow_spawn_egg*2
```

## Final verification (D2)

### 1. Track D's log gets a combined kit for the raid groups

After 1fa0ce90.

- **Checked:** clippy on the crates touched since `r9-d2-base` (`frame`, `hud`, `minecraft_net`, `minecraft_survival`, `minecraft_terrain`, `render_anim`, `render_gpu` and `minecraftoss-entities`, `-generator`, `-player`, `-world`; `--all-targets`, `-A clippy::approx_constant`, with the stand-in `third_party/scenarios` files of `scripts\full-suite.ps1`, removed afterwards) warns on no line added since `r9-d2-base`; every warning is older. The 26 `.rs` files added since `r9-d2-base` pass `rustfmt --check` (edition 2021) on their committed content, and no added line has a comment.
- **Tests:** none new. `test -p` on the eleven crates all green: `frame` 28, `hud` 186, `minecraft_net` 20, `minecraft_survival` 725 and `vanilla_world` 1, `minecraft_terrain` 1348 (3 ignored), `minecraftoss-entities` 374, `-generator` 30, `-player` 198, `-world` 96, `render_anim` 178, `render_gpu` 39; doc-tests green. `check -p launcher` green.
- **Changed:** this log only: the d2 combined kit below, so groups pillagers to raid-villagers are tested with one kit.
- **Test steps:** none; the steps of each group's commits hold.

## Group friend-hand

Segment d6 (friend fixes), brief D9. Continues from the merge of track D's d1 and d2 into minecraft-survival (f7432188, released as round 9 release 2 with protocol 5).

### 1. A friend sees their held items, eating, the shield and the trident charge in first person

After f7432188.

- **Vanilla (26.3):** the client draws its own first-person hands from its own use state (`ItemInHandRenderer.renderArmWithItem`: `applyEatTransform` for eat and drink, the raised shield for block, `applyItemArmTransform` plus the trident pull for spear, the spyglass hiding the hands while scoping); `GameRenderer.renderItemInHand` skips the hands in third person, while sleeping, with the GUI hidden and in spectator. The swing is commit 2.
- **Changed:**
  - `render_anim/src/minecraft_hand.rs`: the host's held-item drawing (`minecraft_world.rs`, the block after the totem guard) moved into `held_hand_mesh(held, view, (packs, atlas, light, dials), light_at, aspect) -> Option<(vertices, indices, clip)>`, with `Held { main: (slot, stack), offhand, swing, swing_offhand, using: Option<HeldUse { slot, kind }>, partial, activation }` and `UseShown { Eat { remaining, duration }, Block, Charge { ticks }, Scope }`. `host_use(survival, hand)` gives the host's use; `UseClock` gives the friend's from its player view. The host calls `held_hand_mesh` with the same stacks, filters and partial as before.
  - `minecraft_net/src/messages.rs`: `UsingView.kind: UseKind { Eat { duration }, Block, Charge, Scope }`; **`PROTOCOL` 5 to 6** (track D's one bump after release 2).
  - `minecraft_survival/src/views.rs`: `using()` fills the kind (eating with the food's `consume_ticks`).
  - `render_anim/src/minecraft_remote.rs`: the friend's hand left `present()` for a new `show_hand()`, run after the friend's hand input each frame. It draws the selected stack while `ui.holding_item`, the offhand stack while holding an item or using one, the use from `UseClock` (re-based on every new player view, then counted on the friend's own tick: eating down, the trident charge up) and the totem rise as before. It hides the items in F5, in bed, while skating, dead or with the UI off (`HandGate`), hides them while scoping (`held_hand_mesh`), and sets `ui.offhand_busy` (the MW2 hands step aside while a use runs, as on the host) and `ui.hand_swing` (0 until commit 2).
- **Tests:** `minecraft_net`: `codec::tests::a_player_view_says_what_kind_of_use_its_hand_is_in` (and the protocol test now asks for 6); `minecraft_survival`: `views::tests::a_friend_is_told_what_kind_of_use_their_hand_is_in` (Eat 32 for a steak, Eat 16 for dried kelp in the offhand, Block, Charge, Scope; the ticks stay out of the sent key); `render_anim`: `minecraft_hand::tests::a_friends_use_counter_counts_eating_down_between_views`, `a_friends_trident_charge_counts_up_and_a_shield_and_spyglass_hold`, `scoping_hides_both_held_items`, `the_host_hand_uses_eating_before_a_shield_a_trident_or_a_spyglass`; `minecraft_remote::tests::the_friends_held_items_hide_in_f5_in_bed_while_skating_dead_or_with_the_ui_off`. Runs: `minecraft_net` 21, `minecraft_survival` 726, `render_anim` 183, all green; `check -p launcher` green.
- **Decisions:**
  - One use at a time per player, as vanilla's single `useItem`: eating before the shield, the trident and the spyglass, the order `views.rs` already sent. The host drew an eating hand and a blocking hand at once if both were set; the fork never sets both.
  - The friend's use clock re-bases on every new player view (the host sends one only when something shown changes, the use ticks are left out of that key), then counts on the friend's own level tick. A finished meal waits at the mouth until the host's next view, which comes at once because the stack count and food change.
  - `UseKind` defaults to `Block` (the derive needs a unit variant); the host never sends a default.
  - With the UI off (`ui_draw 0`) the friend's held items and the totem rise hide, as vanilla's hidden GUI hides the hands. The host's held items still show with the UI off (unchanged in this commit, listed under Left for later). In F5 the totem rise still shows, as on the host.
  - `PROTOCOL` is 6: the first message change after release 2 (protocol 5). Commit 2's `ToClient::Animate` rides in the same 6.
- **Test steps** (same new build in both windows: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r9-friend -Fresh -HostSound`; the friend gets the kit when joining; in the friend window move the items to the hotbar first):
  1. Friend: select the diamond sword: it shows in the right hand like on the host (**171_friend_sword**). Select an empty slot: the MW2 hands show.
  2. Friend: put the shield in the offhand (swap key) and hold right-click with the sword selected: the shield rises (**172_friend_shield**); let go: it lowers.
  3. Friend: select the cooked beef and hold right-click: the beef rises to the mouth and bobs every 4 ticks, then the hand comes back down after 1.6 s (**173_friend_eat_beef**). Dried kelp in the offhand: the same in half the time. The swiftness potion: the same drinking pose.
  4. Friend: hold right-click with the trident: it is raised and pulled back over half a second and shakes (**174_friend_trident**); let go: it is thrown and the hand is empty.
  5. Friend: right-click with the spyglass: the scope shows and no item is in view; let go: the items come back.
  6. Friend: F5 hides the first-person items; F5 back shows them. Lie in a bed: no items in view.
  7. Host window: its own sword, shield, eating and trident look as before (**175_host_hand_unchanged**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword,minecraft:shield,minecraft:cooked_beef*32,minecraft:dried_kelp*16,minecraft:trident,minecraft:spyglass,minecraft:potion@swiftness,minecraft:totem_of_undying,minecraft:white_bed,minecraft:cobblestone*64
```

### 2. A friend's swing plays at once and follows the host's

After 29cafd82.

- **Vanilla (26.3, javap):** `Minecraft.startAttack` returns at once while `missTime > 0`; a miss (or a null hit result) sets `missTime = 10` when `MultiPlayerGameMode.hasMissTime()` (not creative), and every attack that gets past the early returns ends in `LocalPlayer.swing(MAIN_HAND, animation, false)` on the client itself. `continueAttack(false)` clears `missTime`; `Minecraft.tick` counts it down. `LivingEntity.swing(hand, animation, updateSelf)` starts a swing only through `SwingState.startIfAble`: when none runs, when its `ticks >= durationTicks / 2` or when `ticks <= 0` (the 6-tick swing restarts from tick 3 on). The server sends `ClientboundSwingAnimationPacket` to the tracking players, and to the swinger too only for swings it started itself (`sendToTrackingPlayersAndSelf` when `updateSelf`).
- **Changed:**
  - `minecraft_net`: `ToClient::Animate { client, offhand }` (codec index 33, protocol 6 from commit 1).
  - `minecraft_survival`: `Hand.swing_predicted` (`player.rs`), set by the attack-press swing in `actions.rs` only.
  - `render_anim/src/minecraft_players.rs`: `movement()` clears `swing_predicted` where it advances the swing, before the player acts.
  - `render_anim/src/minecraft_host.rs`: after `step_friend`, `told_swing` puts `Animate` into that friend's own outbox for a swing that began this frame (`swing == Some(0.0)`) and was not predicted, and marks it told so a step that returns before `movement()` does not send it again.
  - `render_anim/src/minecraft_hand.rs`: `FriendSwing` (the swing, its hand, the miss time and a tick clock), `may_restart` (vanilla's rule) and `swing_shown`.
  - `render_anim/src/minecraft_remote.rs`: the friend advances `FriendSwing` each frame; on its own attack press (sent this frame, while holding an item, not using one, not skating and not throwing an MW2 grenade) it swings at once, and a press at a miss outside creative starts the 10-tick miss time; `Animate` for the friend's own client restarts the swing by the vanilla rule. `show_hand()` draws the swing on the held item and sets `ui.hand_swing` for the MW2 hands of an empty hand.
- **Tests:** `minecraft_net`: `codec::tests::a_swing_takes_the_next_free_index_on_protocol_6` (and `Animate` in the every-message round trip); `minecraft_survival`: `actions::tests::only_the_swing_of_an_attack_press_is_predicted_by_a_friend`; `render_anim`: `minecraft_hand::tests::a_friends_swing_plays_at_once_and_an_echo_restarts_it_only_after_half_way`, `a_missed_swing_holds_the_next_press_for_ten_ticks_outside_creative`; `minecraft_host::tests::a_friend_is_told_only_of_its_own_swings_it_did_not_predict_once`. Runs: `minecraft_net` 22, `minecraft_survival` 727, `render_anim` 186, all green; `check -p launcher` green.
- **Decisions:**
  - `Animate` goes only to the swinging friend: other players do not draw Minecraft arm swings on bodies yet (the MW2 body has its own animations), so there is no one else to tell.
  - The host keeps restarting its own attack-press swing at once (unchanged); the friend's prediction uses vanilla's restart rule, so a fast second click in the first half of a swing does not restart it on the friend.
  - Swings that start outside the friend's step (none known) would be missed, because the next frame's `movement()` advances them before `told_swing` looks.
- **Test steps** (same build and setup as commit 1):
  1. Friend: select the diamond sword and left-click into the air: the sword swings at once, with no wait for the host (**176_friend_swing_air**). Click fast: each click swings; a click early in a swing does not restart it.
  2. Friend: select an empty slot and left-click: the MW2 hands swing (**177_friend_empty_swing**).
  3. Friend: hit a cow with the sword: one swing and the cow is hurt.
  4. Friend: place cobblestone: the hand swings once (the host's swing reaches the friend) (**178_friend_place_swing**). Hold left-click on stone with the pickaxe: the hand keeps swinging while it digs.
  5. Friend: `/gamemode creative` (the host runs it), left-click into the air twice fast: both swing.
  6. Host window: the host's own swings are as before (**179_host_swing_unchanged**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword,minecraft:iron_pickaxe,minecraft:cobblestone*64,minecraft:cow_spawn_egg*4,minecraft:shield,minecraft:cooked_beef*32,minecraft:trident,minecraft:spyglass,minecraft:dried_kelp*16,minecraft:white_bed
```

## Group friend-portal-music

Segment d6 (friend fixes), brief D9.

### 1. A friend sees the purple portal overlay while standing in a portal

After cc6ff02d.

- **Vanilla (26.3, javap):** `LocalPlayer.handlePortalTransitionEffect` runs on the client's own tick from its own portal state: while the player is inside a portal this tick it closes any screen not allowed in portals (`closeContainer` for a container screen), plays `block.portal.trigger` when the intensity is 0, and adds 0.0125; outside it takes 0.05 off while above 0; the intensity is clamped to 0..1 and drawn lerped between the last two ticks. `NetherPortalBlock.entityInside` marks the player inside only when `canUsePortal(false)`: alive, not a passenger and not sleeping. A dimension change makes a new `LocalPlayer`, so the intensity starts again at 0.
- **Changed:**
  - `render_anim/src/minecraft_portal.rs`: `FriendPortal` (the effect, a 20 Hz clock and the frame age), `friend_inside(feet, spectator, axis_at)` (the host's `portal::touched_portal` with `portal_axis_in`) and the pure `friend_portal_tick(portal, seconds, inside) -> (inside this tick, overlay)`, with the host's `PortalEffect` curve and `overlay_alpha`.
  - `render_anim/src/minecraft_remote.rs`: the friend loads `ui.portal_frames` with its other overlays; a new `show_portal()` runs after `present()` each frame from the real frame time: inside while alive, not riding and not a spectator, on the friend's own streamed terrain. A tick inside closes the friend's open screen (`show_menus` then tells the host), and `ui.portal_overlay` follows the effect. The effect resets on a level rebuild (`present()`, a new level from the host) and `stop()` clears the overlay.
  - The trigger and travel sounds stay as the host sends them (the host's `step_friend` runs the friend's real portal and its sounds).
- **Tests:** `render_anim`: `minecraft_portal::tests::a_friends_portal_overlay_rises_inside_falls_outside_and_restarts_on_a_new_level` (0.0125 a tick inside, drawn between ticks, full after 80 ticks, gone within 20 ticks outside, the first tick inside closes the screen, a reset starts again from 0), `a_friend_is_inside_a_portal_only_alive_and_not_a_spectator`. Runs: `render_anim` 188, all green; `check -p launcher` green.
- **Decisions:**
  - The friend draws its own overlay from its own terrain and position, as vanilla's client does, instead of a new message; the host still decides the travel.
  - A riding friend sees no overlay (vanilla `canUsePortal(false)` and the host's `may_use_portals`); a spectator sees none (the brief's rule; the host's own Nether portal check does not test the game mode, unchanged).
  - The screen closes on every tick inside, not only the first, as vanilla closes it each tick and the host does each step.
- **Test steps** (same new build in both windows: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r9-friend -Fresh -HostSound`; the friend gets the kit when joining):
  1. Friend: build a 4×5 obsidian frame and light it with flint and steel.
  2. Friend: open the inventory (E) and walk into the portal: the inventory closes, the purple swirl fades in with the trigger sound (**180_friend_portal_overlay**), and after 4 seconds the friend goes to the Nether.
  3. Friend: walk in and step out at once: the swirl fades out within a second (**181_friend_portal_fade**).
  4. Friend: back in the Overworld through the Nether portal: the swirl starts again from nothing when the friend steps into the portal once more.
  5. Host window: stepping into the same portal shows the host's swirl as before (**182_host_portal_unchanged**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_pickaxe,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

### 2. A friend hears the End, dragon and credits music

After 6b9d6085.

- **Vanilla (26.3):** `MusicManager` is client-side (`Minecraft.tick` calls `musicManager.tick()` with `Minecraft.getSituationalMusic()`), so every client picks and plays its own music from its own level, its own boss overlay (`BossHealthOverlay.shouldPlayMusic`) and its own credits screen. The fork's `minecraft_music` already follows it for the host (round 8).
- **Changed:** `render_anim/src/minecraft_remote.rs`:
  - `Friend.music`, a `MusicManager` of its own; `ResMut<audio::McMusic>` joins the `(body_gear, sound_queue, sleepers, weapons, strings)` tuple, as `friend()` is at Bevy's 16-parameter limit.
  - `present()` now returns whether the friend's environment ticked (`SkyTicked.ticked`, the same tick the host's music follows); on such a frame `play_music()` builds the `MusicInput` from the friend's level dimension, its credits, its boss bars (`McBossBar.music`, from the host's `play_music` flag) and its loading state (`ui.loading_world`, set just before), and hands the action to `minecraft_music::apply` with the friend's own sounds and packs.
  - `stop()` and the inactive early return clear `McMusic.wanted`, as the host's `update` does when its view is inactive.
- **Tests:** `render_anim`: `minecraft_remote::tests::a_friend_hears_the_dragon_music_with_its_bar_and_stop_clears_it` (a dragon bar in the End starts `music.dragon` at once and it plays on, the Overworld stops it, nothing starts while loading, `stop()` clears the wanted track). Runs: `render_anim` 189, all green; `check -p launcher` green.
- **Decisions:**
  - The friend's music runs only on frames where its environment ticked, like the host's, so its 20 Hz delays match vanilla's.
  - No message change: the boss bars, credits and level the music needs were already on the friend.
- **Test steps** (same build and setup as commit 1, with host sound: `-HostSound`; use headphones on the friend window or mute the host to tell the two apart):
  1. Host: `/execute in minecraft:the_end run tp @a 0 70 0`. When the dragon's bar shows at the top of the friend's screen, the friend hears the dragon music at once (**183_friend_dragon_music**, the friend window with the bar).
  2. Host: `/kill Friend`; the friend respawns in the Overworld: the music stops at once and no new music starts there (**184_friend_music_stopped**).
  3. Friend: leave the game while in the End (return to the menu): the music stops with it.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword,minecraft:bow,minecraft:arrow*64,minecraft:ender_pearl*16,minecraft:water_bucket,minecraft:cooked_beef*32,minecraft:obsidian*14,minecraft:flint_and_steel
```

## Group pvp-melee

Segment d6 (friend fixes), brief D9.

### 1. Minecraft melee hits other players, with sweeps, crits, knockback and the pvp rule

After 91dd9245.

- **Vanilla (26.3, javap of `Player.attack`, `isSweepAttack`, `canCriticalAttack`, `causeExtraKnockback`, `doSweepAttack` and `LivingEntity.getKnockback`; enchantment data `sharpness.json` and `knockback.json` from the jar):**
  - `Player.attack` treats a player like any `LivingEntity`. Base = attack damage × (0.2 + s² × 0.8), magic = (`getEnchantedDamage` − damage) × s. With neither above 0 nothing happens.
  - Full strength is s > 0.9. A sprinting full hit plays `entity.player.attack.knockback` before the hurt. A crit (falling, not on ground, climbing, in water, riding or sprinting) multiplies only the base by 1.5, so Sharpness is not multiplied. The sweep needs full strength, no crit, no sprint knockback, on ground, slow movement and a sword.
  - `hurtOrSimulate` false (pvp off: `ServerPlayer.hurtServer` refuses a player source when `canHarmPlayer` is false; a creative or spectator victim is invulnerable) plays `entity.player.attack.nodamage` and nothing else. `onAttack` resets the attack strength in both cases.
  - On a hurt: `causeExtraKnockback(getKnockback + 0.5 if sprinting)`, where `getKnockback` = (attribute + the Knockback enchantment's `add 1 per level`) / 2, so Knockback 2 adds 1.0. `LivingEntity.knockback(strength, sin(yaw), −cos(yaw))`.
  - `doSweepAttack`: every living entity in the target's box grown by 1, 0.25, 1, other than the attacker and the target, within 3 blocks (distance² < 9) of the attacker, takes `getEnchantedDamage(1 + 0 × base) × s` (so Sharpness adds to the sweep too) and knockback 0.4.
  - Then `itemAttackInteraction` (the weapon's `item_damage_per_attack` wear) and `causeFoodExhaustion(0.1)`, only after a hurt. Sharpness: `add 1 + 0.5 per level above the first` (Sharpness 5 adds 3.0).
- **Changed:**
  - New `minecraft_survival/src/pvp.rs`: `PlayerBox` (id, 0.6-wide box, invulnerable), the pure `melee(attack, victim) -> PvpHit` (the vanilla damage, crit, knockback, sweep flags and sounds above), `swept(attack, around, target, players)` (the sweep round a player or a mob), `player_on_ray` (never the attacker), `shield_disable_ticks` (the weapon component's `disable_blocking_for_seconds`, 100 ticks for an axe), `wear_weapon` and `hit()` (a `Hit` with `DamageCause::Player { client }`).
  - New `minecraft_terrain/src/melee.rs`: `weapon_bonus(stack)` (Sharpness damage and Knockback strength from the stack's enchantments), the one helper both player hits and mob hits use.
  - `minecraft_survival/src/entities.rs`: `Entities.player_boxes` (`set_player_boxes`, `player_box`) and `punch_player`, run before `punch`. It hits the nearest player only when no frame, mob, End part, shulker bullet, fireball or block is nearer. Each blow goes into the victim's `queue.out.hits`, where `struck()` already applies the shield, the axe's shield disable, armour, Protection, the totem and the MW2 knockback. A sword's sweep round a player target reaches the other players and, through the new `ServerHandle::mob_sweep` (`Command::MobSweep` on the entity world's thread, with the mobs' hurt sounds, loot and experience), the mobs beside them; a sweep round a mob target now reaches players beside it too. `mob_distance` counts players. A mob hit by a creative player no longer passes `infinite: false`.
  - `minecraft_survival/src/actions.rs`: `act()` tries `punch_player` first. A player hit resets the attack strength and counts as a punch even with pvp off (only the no-damage sound). On a hurt it adds the attack exhaustion and wears the weapon (not in creative). `allows()` takes a player-box lookup and passes `Target::Player` only within entity reach.
  - `minecraft_survival/src/death.rs`: `DamageCause::Player { client }` with the damage type `minecraft:player_attack`. Its death line is commit 2.
  - `render_anim/src/minecraft_players.rs`: `set_player_boxes` fills each level's boxes every frame before any `act()`, from `sim::voxel::player_hull`. Only alive players count, and not ones who are switching or arriving, not spectators, and only in their own dimension's band. `step_friend`'s `allows` sees the other players' boxes, never the friend's own. `render_anim/src/minecraft_world.rs` calls it after the queues are drained.
  - `render_anim/src/minecraft_remote.rs`: the friend's `aim()` tests the other players' bodies in the same band (alive) against the mobs and the block, and sends `Target::Player { client }` for the nearest.
  - Vendored `entities/src/world/combat.rs` (already in the patched list): `player_attack_with(attack, target, enchanted, extra_knockback)` adds Sharpness to the hit and the sweep and Knockback to the push, and the sweep loop moved into `player_sweep(attack, around, target, enchanted)`, which a player target's sweep uses too. `player_attack` keeps its old behaviour through it. `minecraft_terrain/src/mob_actions.rs` gains `sweep()` and `server.rs` the `MobSweep` command (additive). `minecraft_terrain/src/mob_actions.rs` passes `weapon_bonus` and wears the weapon after a hurt. Mob hits never wore the weapon before.
- **Tests:**
  - `minecraft_survival`:
    - `pvp::tests`: a full hit lands once with the weapon's damage and the strong sound; half strength scales like vanilla (6 × 0.4, and 1 → 0.4); pvp off or a creative victim gives only the no-damage sound and no blow; a falling full hit is critical (×1.5) and Sharpness is not multiplied; Sharpness 5 adds 3 and scales with strength; sprint knockback (0.5) and Knockback 2 (1.0) throw along the attacker's facing and add up; an axe disables a shield for 100 ticks (also from the real item catalog); a sweep reaches a second player in range and skips one 4 blocks away, also round a cow, never with pvp off and never past 3 blocks from the attacker; the ray finds the nearest other player and never the attacker; a hit wears the weapon by its damage per attack.
    - `actions::tests::a_player_target_is_allowed_only_in_reach`.
    - `entities::tests::a_punch_queues_one_hit_on_the_player_in_front_and_none_without_pvp`: one hit with `DamageCause::Player`, none on the attacker; the sweep reaches the player beside the target; a nearer block or a look away hits nobody; pvp off queues nothing and plays the no-damage sound.
  - `minecraft_terrain`: `melee::tests` (Sharpness 1 → 1.0, 5 → 3.0; Knockback 2 → 1.0); `mob_actions::tests::a_sweep_round_a_player_hurts_the_mobs_beside_them_and_not_one_far_away` (two cows beside the box lose 1 + Sharpness 3, one 4 blocks off keeps its health).
  - `render_anim`: `minecraft_players::tests::only_a_living_player_in_its_own_band_gets_a_melee_box` (spectator, dead and out of band skipped, creative marked invulnerable).
  - Runs: `minecraft_survival` 739, `minecraft_terrain` 1354 (one run had `sections::tests::background_full_update_matches_the_synchronous_one` fail under load; it passes alone and touches nothing of this commit), `render_anim` 190 and `minecraftoss-entities` 374, all green. `check -p launcher` green.
- **Decisions:**
  - The victim side is `struck()` as it was. A player blow is a `Hit` in the victim's queue, so the shield, armour, Protection, totem, knockback resistance and the death all follow the paths mob hits already use, for host and friends alike.
  - The sweep round a mob target reaches players at once. The mob's own hit runs on the entity world's thread, so the host does not wait to learn whether that hit landed (vanilla sweeps only after a landed hit).
  - A creative attacker's weapon is not worn, on players or mobs.
  - No message change: `Target::Player` already existed. `PROTOCOL` stays 6.
- **Test steps** (same new build in both windows: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r9-friend -Fresh -HostSound`; both get the kit; host: `/op Friend`, `/gamerule pvp true`):
  1. Both put on the iron armour. Friend: hit the host 3 times with the diamond sword, waiting for the full charge each time. The host loses armour-reduced hearts (about 2 hearts per hit), is pushed back a little and hears the strong-hit sound (**185_pvp_full_hits**, the host window with the hearts).
  2. Friend: jump and hit the host while falling: the crit sound and a bigger hit (**186_pvp_crit**).
  3. Friend: sprint into the host and hit: the host flies further, with the knockback sound. Friend: hit with the iron sword with Knockback II: the host flies further still (**187_pvp_knockback**).
  4. Host: raise the shield (right click) facing the friend. Friend: hit with the sword: blocked, no hearts lost. Friend: hit with the iron axe: the shield drops and stays down for 5 seconds (**188_pvp_shield_axe**).
  5. Host: hatch two cows right beside you. Friend: stand still and hit the host at full charge with the diamond sword (not sprinting, not falling): the sweep sound plays and both cows are hurt too (**189_pvp_sweep**). Friend: hit one of the cows the same way while the host stands beside it: the host is hurt by the sweep.
  6. Host: hit the friend the same way: the same results on the friend's side (**190_pvp_host_hits_friend**).
  7. Host: `/gamerule pvp false`. Both hit each other: only the no-damage sound, no hearts lost, no push (**191_pvp_off**). Host: `/gamerule pvp true` again.
  8. Host: `/gamemode creative Friend`. Host hits the friend: the no-damage sound, no hearts lost. `/gamemode survival Friend`.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword,minecraft:iron_axe,minecraft:shield,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:iron_sword+knockback=2,minecraft:cow_spawn_egg*4,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:totem_of_undying
```

### 2. Players killed by Minecraft melee get vanilla death lines

After 2f0c9dee.

- **Vanilla (26.3, javap of `DamageSource.getLocalizedDeathMessage`; `en_us.json`):**
  - A `player_attack` source (message id `player`) reads "death.attack.player" ("%1$s was slain by %2$s") with the attacker's display name.
  - The `.item` form ("%1$s was slain by %2$s using %3$s", the weapon as `[name]`) is used only when the attacker's main-hand item at the time of the message has `minecraft:custom_name`.
  - `CombatTracker.getFallMessage` does the same for "death.fell.assist" and "death.fell.finish" (both have `.item` forms).
  - A death from the world shortly after a player's hit names the kill credit: `getKillCredit` prefers `lastHurtByPlayer` (100 ticks) over the mob, as in "death.attack.lava.player".
- **Changed:**
  - `minecraft_survival/src/death.rs`:
    - `DamageCause::Player` now has the message id `player`.
    - New `Killer { name, weapon }`. `death_line` and `death_text` take a player lookup (`impl Fn(u32) -> Option<Killer>`).
    - A player's last blow gives "death.attack.player". With a custom-named weapon it becomes "death.attack.player.item" with `[name]`.
    - A player's hit before a long fall gives "death.fell.assist" (or `.item`).
    - `CombatTracker` keeps `hurt_by_player` for 100 ticks, for the `.player` escape lines. A mob that strikes last is still the one named.
    - A killer who is no longer in the game falls back to "death.attack.generic" or "death.fell.killer".
    - The kill card's killer is the attacker's name.
  - `minecraft_survival/src/survival.rs`: the `death_line` and `death_text` wrappers pass the lookup through.
  - `minecraft_survival/src/pvp.rs`: `custom_name(stack)` reads the plain-string or `{"text": ...}` name.
  - `minecraft_survival/src/entities.rs`: `set_held_names` and `held_name` hold each player's held custom name for the frame.
  - `render_anim/src/minecraft_players.rs`:
    - `set_player_boxes` also fills every level's held names.
    - `settle()` builds the lookup from the chat's player names (`globals.chat.name`) and the held names, for the death line, the chat text and the kill card, for host and friends alike.
  - No MW2 kill credit: the death still goes to the sim as a `MinecraftDeath` with no attacker.
- **Tests:** `minecraft_survival`:
  - `death::tests::a_players_blow_slays_by_name_and_names_only_a_custom_named_weapon` ("Friend was slain by Host", "... using [Excalibur]" only with a custom name, a killer who left gives "died", the damage type).
  - `death::tests::a_player_dooms_a_fall_and_is_escaped_into_lava_for_five_seconds` (the fall assist with and without the named weapon, the lava escape and its 100-tick end, a zombie's last bite still named).
  - `pvp::tests::a_weapon_is_named_only_by_its_custom_name`.
  - Runs: `minecraft_survival` 742 and `render_anim` 190, all green. `check -p launcher` green.
- **Decisions:**
  - The weapon name is the attacker's held item when the death line is made, as vanilla reads `getMainHandItem()` at message time. Held names are refreshed every frame from all players.
  - A killer in another dimension still has its name (from the chat roster), but not its weapon name.
  - No message change. `PROTOCOL` stays 6.
- **Test steps** (same build and setup as commit 1, `/gamerule pvp true`):
  1. Host: place the anvil, drink experience bottles until level 5 or more, and rename the iron sword "Excalibur" in the anvil.
  2. Host: kill the friend with Excalibur. Both chats show "Friend was slain by Host using [Excalibur]" once, with the host's real name (**192_pvp_named_kill**, both windows). The friend's kill card names the host.
  3. Host: kill the friend with the plain diamond sword: "Friend was slain by Host" (**193_pvp_plain_kill**).
  4. Friend: hit the host off a ledge at least 6 blocks high with Excalibur passed to the friend (drop it): "Host was doomed to fall by Friend using [Excalibur]" (**194_pvp_doomed_fall**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:iron_sword,minecraft:diamond_sword,minecraft:anvil,minecraft:experience_bottle*32,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

## Group friend-skating

Segment d6 (friend fixes), brief D9 and `docs/round8/d-skate.md`.

### 1. A friend can skate: the host moves them from the friend's board

After b5a04ad1.

- **Vanilla (26.3, javap of `ServerGamePacketListenerImpl.handleMoveVehicle`):** skating has no vanilla twin; the closest is a vehicle, which the client moves and the server checks. The server refuses a vehicle move that is "too quickly" (the squared distance from the vehicle's place minus its squared speed above `100.0`, so 10 blocks a packet beyond its speed) or "wrongly" (ending more than `0.0625` from where the server's own collision puts it), and puts the vehicle back. A rider who dies, changes dimension or is teleported leaves the vehicle.
- **Changed:**
  - `minecraft_net/src/messages.rs`: `SkateReport { tick, on, airborne, origin, yaw }`; the state lane now carries `StateUp { Hand(HandState), Skate(SkateReport) }` instead of a bare `HandState`; `ToServer::Skate(SkateReport)` (index 17) is the reliable off report; `ToClient::SkateRefused { tick }` (index 34). `PROTOCOL` stays 6 (set by group friend-hand, unreleased).
  - `net/src/client/predict.rs`: `ClientPrediction::set_external_motion(enabled)` and `set_origin(origin)` forward to its `SimWorld`. The origin set while skating is held through every adopted snapshot (`on_authority` puts it back after the replay), so the host's older place never pulls the friend's skater back; turning external motion off drops it.
  - `sim/src/voxel.rs`: `inside_blocks(origin, mins, maxs)`, whether a box overlaps any block's collision in that level.
  - `render_anim/src/skate.rs`: the hard `AuthorityWorld` requirement is gone. A `SkateBody` trait (`set_external_motion`, `set_origin`, `e_flags`) is implemented for the host's `AuthorityWorld` and the friend's `ClientPredictionState`; the teleport bit comes from the authority on the host and from `presented.player(local)` on a friend. A new `SkateMode.refused` (frame) picks the board up when the host refused it.
  - `render_anim/src/minecraft_world.rs`: `board_allowed` and `board_gear` are `pub(crate)`.
  - `render_anim/src/minecraft_remote.rs`: the friend computes `ui.board_allowed` from its own view (alive, spectator, flying, gliding, riding, eyes under water from its own terrain, asleep). Every new skate pose, and every 0.25 s while the pose stands still, goes to the host as `StateUp::Skate` on the state lane; picking the board up sends one `ToServer::Skate` with `on: false` on the reliable lane. A `SkateRefused` for the current ride sets `SkateMode.refused`.
  - New `render_anim/src/minecraft_skaters.rs`:
    - The host's `Skater` per friend keeps the newest report (older ticks are dropped).
    - `judge()` accepts a report only while the friend is alive, not switching dimensions, the board is allowed (from the host's own step of that friend: the authoritative `ps`, its game mode, its eyes in water, its bed and powder snow), the origin is finite, not inside blocks and within `40 blocks/s x the time since the last accepted report (at most 1 s) + 10 blocks` of the host's place for the friend.
    - An accepted report sets external motion, the origin and the view yaw (so the body faces the board). A refused one turns external motion off, sends `SkateRefused { tick }`, and ignores the friend's reports until its off report.
    - While riding, death, a dimension switch or change, a teleport bit flip (`/tp`, pearls, gateways, respawn) and 2 s without a report refuse the board the same way. An off report stops it.
    - The friend's `Reporter` numbers the reports and knows which refusal belongs to the current ride.
  - `render_anim/src/minecraft_host.rs`: reports from both lanes reach the friend's `Skater` (`HostSessions.skaters`, dropped when the friend leaves). Each frame the host judges the newest report before `step_friend`, which now takes the friend's `Skating { active, airborne }`. The skating friend's held items are cleared from the body gear the host draws and from the `Equipment` it sends, so nobody sees an item in a skater's hands.
  - `render_anim/src/minecraft_players.rs`: `step_friend` passes the `Skating` to `movement()` (no footsteps, no movement food, vanilla fall damage on landing, as the host's own board), sets `actor.skating` (no mining, punching, placing, eating or blocking), and returns `Stepped.board_allowed`.
- **Tests:**
  - `minecraft_net`: `codec::tests::a_skate_report_rides_the_state_lane_beside_the_hand_on_protocol_6` (the `StateUp` tags, `ToServer::Skate` at 17, `SkateRefused` at 34 with its varint tick, round trips); the every-message tests cover both new messages.
  - `net`: `client::predict::tests::an_external_origin_holds_through_snapshots_until_it_is_released`.
  - `sim`: `voxel::tests::a_body_is_inside_blocks_only_where_it_overlaps_one`.
  - `render_anim`: `minecraft_skaters::tests::a_valid_report_moves_the_friend`, `a_report_too_far_is_refused_until_the_friend_picks_the_board_up`, `a_report_inside_a_wall_or_in_a_bad_state_is_refused` (wall, dead, board not allowed, NaN), `death_a_dimension_change_a_teleport_or_silence_clear_the_board`, `an_off_report_stops_and_older_reports_are_ignored`, `a_skater_shows_no_held_item`, `the_friend_reports_new_poses_keeps_the_ride_alive_and_sends_one_reliable_off`, `a_refusal_stops_only_the_ride_it_was_for`, `the_board_yaw_follows_the_skater_root`. Leaving is the `Skater`'s removal with the session (no separate test).
  - Runs: `minecraft_net` 23, `net` (lib) 17, `sim` 157, `render_anim` 199 and `minecraft_survival` 742, all green. `check -p launcher` green. The new file passes `rustfmt --check --edition 2024`.
- **Decisions:**
  - The host checks the friend's skater like a vanilla vehicle move, with a fixed top speed: the skate engine reads its speed limits from the Skate 3 data, which the host may not have, so the host uses 40 blocks/s (above any rolling speed and a 4-second free fall) plus vanilla's 10-block allowance per move. The wall test uses a 0.4-wide body from half a block to 1.5 blocks above the skater root, since the root and the wheels sit in the surface they roll on.
  - The friend's `SkateReport.tick` is the friend's own report number, not the engine's pose tick, so the host can order the two lanes and drop a stale state report that arrives after the reliable off.
  - The host keeps each friend's skate state in `HostSessions.skaters` (keyed by client, dropped on leave) beside the other per-client host state, not in `minecraft_survival::session::Session`, so the shared crate does not change.
  - The friend does not test powder snow itself (it has no survival state); the host does and refuses the board there.
  - Others see the friend's soldier move with the board and turned to its heading, in the idle pose and without the board: the pose and board for everyone are group skater-poses. The host's own skating still sends its held items to friends until then.
  - No vendored file changed.
- **Test steps** (same new build in both windows: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r9-friend -Fresh -HostSound`; Skate 3 must be converted in the game folder, so both role folders get `IW4L_SKATE_ASSETS`; host: `/op Friend`):
  1. In both windows: Options > Controls > Minecraft > Skating: Enabled.
  2. Friend: on flat ground, hold the diamond sword and press J. The board goes down and the skate camera takes over. In the host window the friend's soldier rolls along with no sword in its hands (**195_friend_skates**, both windows).
  3. Friend: build a 5-step staircase of oak stairs on cobblestone columns and skate up and down it. The host sees the friend go up and down smoothly, without jumps back (**196_friend_skates_stairs**, host window).
  4. Friend: build a 4-block cobblestone tower with a 6-block runway and roll off the edge without an ollie: half a heart is lost on landing (**197_friend_skate_fall**). With a hay bale set into the ground at the landing spot: nothing.
  5. Friend: dig a 3x3 pool 2 deep, fill it with the water bucket (in creative, then `/gamemode survival`) and ride in: the board is picked up when the eyes go under, and the friend swims (**198_friend_skate_water**).
  6. Friend on the board: open chat and run `/tp ~ ~ ~20`. The friend stands at the new place with the board picked up (**199_friend_skate_tp**). J puts it down again there.
  7. Friend on the board: the host runs `/kill Friend`. After the respawn J puts the board down again.
  8. Friend on the board: press E. The inventory opens and the skater coasts; the host still sees the friend moving.
  9. Host skates (J): the friend's window shows the host's soldier moving, as before.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:oak_stairs*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:hay_block*16,minecraft:water_bucket,minecraft:diamond_sword,minecraft:cooked_beef*32
```

## Group skater-poses

Segment d6 (friend fixes), brief D9 group skater-poses.

### 1. Everyone sees every skater's pose and board

After 0a36201a.

- **Vanilla (26.3):** skating has no vanilla twin. The nearest is how a client sees another entity: the server sends its place at most once a tick (`ServerEntity.sendChanges`, from `ChunkMap.TrackedEntity`), only to players tracking it in the same level, and each client moves the entity smoothly between the last places it heard (`Entity.lerpTo`, `InterpolationHandler`). The skater pose follows that: one pose a tick, relayed only within the level, interpolated by every viewer.
- **Changed:**
  - `frame/src/skate.rs`: `SKATE_BONES`, the 22 mapped skater bones of `skate/rig.rs` (`HIPS` .. `RIGHTTOEBASE`) plus `SKATEBOARD_ROOT`, in a fixed order; `SkatePose { root, bones }` with `from_named(root, names, bones)` and `of(&SkateMode)`; the resource `SkatePoses(BTreeMap<u32, SkatePose>)`, keyed by client.
  - `render_anim/src/skate/rig.rs`: `pose(dobj, names, bones)` and `board(names, bones, geom)` take names and bones instead of `&SkateMode`. `pose` runs the unchanged `pose_with(reference, ...)`. The board's joints come from `board_skin`: a joint the bones do not carry (trucks, wheels) follows `SKATEBOARD_ROOT`.
  - `render_anim/src/skate.rs`: a new `publish` system after `update` puts the local skater's `SkatePose` into `SkatePoses` under its client and takes it out when the board is picked up.
  - `render_anim/src/occupancy/remote_body.rs`: `sync_remote_bodies` and `pose_remote_bodies` look the skater up in `SkatePoses` by client (`minecraft_skaters::drawn`), for the local body as for everyone else: the body root comes from the pose, the soldier is posed by `rig::pose` with `SKATE_BONES`, and the board is drawn with no gun. The local skater's board keeps every Skate joint (`board_bones`); others' boards use the shared bones. A game without `rig.json` draws no skater (`drawn` is `None`): the idle soldier at the place and heading the host sends, with no board.
  - `minecraft_net/src/messages.rs` (protocol 6, no bump): `SkatePose { root: [f32; 12], bones: [[f32; 12]; 23] }` (3x4 column-major, the bottom row is `0 0 0 1`); `SkateReport.pose: Option<Box<SkatePose>>` (boxed so `StateUp` and `ToServer` stay small; `SkateReport`, `StateUp` and the friend's `Report` are no longer `Copy`); `Skaters(Vec<(u32, SkatePose)>)`; `StateFrame.skaters: Blob` (the encoded `Skaters`, empty when nobody else skates in that level).
  - `minecraft_survival/src/mob_sender.rs`: the state frame starts with no skaters (the host fills them in).
  - `render_anim/src/minecraft_skaters.rs`:
    - `net_pose` and `from_net` turn a `SkatePose` into its 3x4 floats and back.
    - The host's `Skater` keeps the pose of the last accepted report while the ride lasts (`shown`) and hands each new one over once (`take_fresh`); a refusal, an off report or the end of the ride drops it.
    - `relay(skaters, to, level)` gives a player every skater in its level but itself; `relayed` encodes it into the frame's blob, `heard_skaters` decodes it.
    - `PoseTrack` keeps the last two poses heard and shows the skater between them, one interval behind (the span is capped at 0.2 s): root and bones are split into scale, rotation and place, the rotations slerped and the rest lerped. A track not heard for 1 s is dropped.
    - `SkaterTracks` holds the tracks of the other skaters, writes them into `SkatePoses` each frame and removes exactly the entries it wrote when a skater leaves the list, goes silent or the game stops.
    - The friend's `Reporter` sends at most one state report per tick (50 ms), as vanilla's client sends one move a tick; the keep-alive stays at 0.25 s.
  - `render_anim/src/minecraft_remote.rs` (friend): every state report carries the skater's pose. A state frame the friend accepts gives its skaters to the friend's `SkaterTracks` (its own client left out), which fill `SkatePoses` each frame. `stop()` and a role change clear them.
  - `render_anim/src/minecraft_host.rs` (host): an accepted report's pose goes into the host's `SkaterTracks` (drawn on the host with interpolation); a friend that stops skating or leaves is removed. Each state frame gets the skaters of the receiver's level: the host's own (from its `SkateMode`, under the host's client) and every friend riding with a pose. The host's own held items are now cleared from the `Equipment` it sends while it skates, as the friends' already were.
- **Bandwidth per skater (from the codec tests):** one pose is 1152 bytes (12 + 23 x 12 floats). A friend's state report is 1173 bytes (1 tag + 19 report + 1 option + 1152), at most 20 a second while the pose changes (one every 0.25 s while it stands still): about 23.5 KB/s up per skating friend. In each state frame (one per server tick, 20 a second) each other skater in the receiver's level adds 1153 bytes (1 client + 1152): about 23 KB/s down per skater for every other player in that level, so with four players all skating each game receives about 69 KB/s more. The off report on the reliable lane is 21 bytes. Both lanes are zstd-packed when that is smaller; float poses rarely pack much.
- **Tests:**
  - `minecraft_net`: `codec::tests::skater_poses_ride_the_state_frame_at_1152_bytes_a_skater` (the pose size, `Skaters` round trip and size, the frame's blob, the report sizes with and without a pose); `state_lane_messages_round_trip` covers `StateFrame.skaters`; `skate_report(true)` now carries a pose in the every-message tests.
  - `render_anim`: `skate::rig::tests::the_pose_from_the_mapped_bones_equals_the_pose_from_every_skate_bone` (a 23-joint soldier and a synthetic rig: `pose` with the 23 shared bones equals `pose` with every engine bone in another order, an unmapped finger follows its wrist), `board_joints_beyond_the_shared_bones_ride_the_deck`; `minecraft_skaters::tests::the_shared_bones_are_the_ones_the_wire_carries`, `the_local_pose_keeps_only_the_mapped_bones_in_order`, `the_host_relays_skaters_only_to_players_in_the_same_level`, `others_see_a_skater_between_its_last_two_poses`, `a_game_shows_the_skaters_it_hears_and_drops_the_ones_that_left`, `the_own_board_keeps_every_skate_bone_and_others_ride_the_shared_ones`, `a_viewer_without_the_rig_draws_the_idle_soldier`, `the_host_keeps_the_pose_of_an_accepted_report_until_the_ride_ends`, `the_friend_reports_at_most_once_a_tick`.
  - Runs: `minecraft_net` 24, `frame` 28, `render_anim` 210 and `minecraft_survival` 742 (+1 world test), all green. `check -p launcher` green. Clippy on `render_anim`, `minecraft_net` and `frame` (with `-A clippy::approx_constant`) shows no new warning in the changed code besides the existing too-many-arguments note on `sync_remote_bodies`. No new `.rs` file; `minecraft_skaters.rs` (track D's own file) still passes `rustfmt --check --edition 2024`.
- **Decisions:**
  - The pose carries 23 bones, not 22: the 22 body bones of `rig.rs` plus `SKATEBOARD_ROOT`, without which no viewer can place the board. Other board joints (Skate 3's trucks and wheels, if `board.json` weights any vertex to them) ride the deck rigidly on other players' boards; the skater's own board in third person still uses every joint. `SPINE3`, `NECK` and `HEAD` are sent though `rig::pose` copies the parent for `j_spine4`, `j_neck` and `j_head`, as the brief asks for all 22.
  - Bones are sent in Skate's own space and converted by each viewer, so the bytes on the wire are the engine's; the root is sent in map space, as `SkateMode.root` holds it.
  - Each game shows other skaters one report interval behind, between the last two poses (vanilla interpolates entity moves the same way); the local skater is never delayed.
  - The host relays only the newest accepted pose of each friend, so a refused or stopped skater disappears from everyone's view at once.
  - The friend's reports are throttled to one per 50 ms (vanilla's one move per client tick) to keep the upload near 23 KB/s; the host's movement check already allows for any gap.
  - The host's own held items are cleared from its `Equipment` while it skates (the friend-skating leftover), so friends see no item in the skating host's hands.
  - No vendored file changed.
- **Test steps** (same new build in both windows: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r9-friend -Fresh -HostSound`; both role folders need Skate 3 converted, so both get `IW4L_SKATE_ASSETS`; host: `/op Friend`):
  1. In both windows: Options > Controls > Minecraft > Skating: Enabled.
  2. Friend: on flat ground press J and roll towards the host. In the host window the friend's soldier stands on a board in the skate pose (knees bent, arms out), the board under its feet, and it leans and turns with the friend's board without stutter (**200_host_sees_friend_skate**, host window, and the friend's own third-person view with F5 for comparison).
  3. Friend: ollie and kickflip next to the host. The host sees the jump and the flip of the board (**201_host_sees_friend_trick**).
  4. Friend: press J again. In the host window the board is gone at once and the friend's soldier stands in its idle pose.
  5. Host: press J and roll past the friend. In the friend's window the host's soldier rides a board in the skate pose with no item in its hands (**202_friend_sees_host_skate**, friend window).
  6. Both skate at the same time next to each other: each window shows the other on its board (**203_both_skate**, both windows).
  7. Host, standing on its board in the Overworld: `/execute in minecraft:the_nether run tp Friend 0 80 0`. The friend in the Nether sees no skater. Host: `/execute in minecraft:overworld run tp Friend ~ ~ ~` (the friend lands at the host): the friend sees the host on its board again.
  8. A friend whose `skate-data` has no `rig.json` (rename it) sees the host as the standing soldier moving along with no board, and does not crash.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:oak_stairs*64,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:diamond_sword,minecraft:cooked_beef*32
```

## Group strider-freeze

Segment d6, brief D10 group strider-freeze.

### 1. A Minecraft strider carrying a player keeps walking under /tick freeze

After 3476d774.

- **Vanilla (26.3, javap):**
  - `TickRateManager.isEntityFrozen(e)` is `!runsNormally() && !(e instanceof Player) && e.countPlayerPassengers() <= 0`; `countPlayerPassengers` counts indirect passengers too.
  - `ServerLevel.lambda$tick$0` (the entity tick list): a removed entity returns, a frozen one returns, then `checkDespawn`, then a non-player outside `inEntityTickingRange` returns, a passenger returns (its vehicle ticks it), else `tickNonPassenger`.
  - `ServerLevel.tickPassenger` checks only `isRemoved`, the vehicle and the tick list, never the frozen state, so the whole stack under a player rider ticks: the root vehicle and every passenger on it.
  - A frozen tick does not advance the game time; the client uses the same `isEntityFrozen` test, so a player-carrying vehicle is drawn on its own running clock.
- **Changed:**
  - Vendored `entities/src/world/riding.rs`: `EntityWorld::player_carriers()`, every mob in the stack under a player rider (`with_mob_riders(root_vehicle(v))` for each vehicle with a `Rider::Player`), sorted and deduplicated.
  - Vendored `entities/src/world.rs`: `tick_with_players_where` became a wrapper over a private `tick_selected(.., frozen)`; the new `tick_frozen_with_players_where` ticks only the carriers inside the ticking range, does not bump `game_time`, skips `resolve_piglin_angers`, and runs the same leash, ride, death and pruning epilogue. Audit of `game_time`: the tick body passes the frozen time to burn, suffocate, hurt and goal timers (`last_can_use_check`), as vanilla's frozen level time does; `hurt_by_blast`, `player_hurt`, `apply_arrow_hit` and `mob_hits_mob` read the field and need no change.
  - `minecraft_terrain/src/server.rs`: `tick_mobs` became a wrapper over `tick_mobs_with(.., frozen)`; the new `ServerSim::tick_ridden_mobs` (nothing to do without a carrier) runs it frozen, and `run_tick`'s frozen branch calls it. The frozen run skips `push_from_players` and `explode_blasts`; the mob event drains (death loot, barters, conversions, creeper and fireball blasts) still run.
  - `minecraft_terrain/src/client_mobs.rs`: `ClientMobs::tick_only(ids)` ticks only the listed mobs; the rider snapping of `tick()` moved into `seat_riders`, shared by both. `set_ride_partial(carriers, partial)` and `partial_of(id, partial)`: `ClientMobs::pose` and `shadow_casters` draw a carrier with the ride partial.
  - `minecraft_survival/src/entity_view.rs`: `EntityView.ride_clock` and `ride_partial()`. While the game runs normally the ride clock is the level clock; while frozen it runs on real time and every tick length ticks the client mobs of `world.player_carriers()` (the host's copy of the server's entity world, so the friends' vehicles too). `meshes()` hands the carriers and the ride partial to `ClientMobs`, so every renderer, the rider mobs and the shadows use it.
  - `minecraft_survival/src/entities.rs`: `Entities::ride_seat` (the host's and every friend's seat on the host) uses the ride clock.
  - The friend needs no change: its `EntityView` always runs normally (`render_anim/src/minecraft_remote.rs`, `ClientTiming::default()`), and the strider's moves reach it from the host's trackers, which now see it move.
- **Tests:**
  - `minecraftoss-entities`: `world::riding::tests::player_carriers_is_the_stack_under_a_player` (a player on a baby strider riding an adult with a zombified piglin also on it: all three; a strider jockey with no player: none; a second player's strider; dismounting drops the stack) and `a_frozen_tick_moves_only_what_carries_a_player` (20 frozen ticks: the steered strider walks over 2 blocks and ticks 20 times, the other strider neither moves nor ticks, game time unchanged; outside the ticking range the carrier waits; a normal tick runs everything again).
  - `minecraft_terrain`: `server::tests::a_strider_carrying_a_player_walks_while_frozen` (`run_tick` with `normal = false`: the ridden strider walks where the rider looks, a cow keeps its place and tick count, the level's and the mobs' game time stay; after dismounting the strider freezes too; a normal tick runs the cow) and `client_mobs::tests::tick_only_moves_only_the_listed_vehicle_and_seats_its_rider` (also the ride partial in `pose`).
  - `minecraft_survival`: `entity_view::tests::a_ridden_strider_keeps_its_own_clock_while_the_host_is_frozen` (the level partial stays, the ride partial runs and the carrier ticks once a tick length, the empty strider does not; they agree after unfreezing).
  - Runs: `minecraftoss-entities` 376, `minecraft_terrain` 1353 (3 ignored), `minecraft_survival` 743 (+1 world test), all green. `check -p launcher` green. Clippy on the three crates (with `-A clippy::approx_constant`, tests included) shows no warning in the changed code. No new `.rs` file.
- **Decisions:**
  - The whole stack under a player ticks (vanilla's `tickPassenger` ticks every passenger of a ticking vehicle), still only inside the entity-ticking range.
  - A frozen tick neither bumps the entity world's game time nor resolves piglin anger (both belong to a normal tick); leashes tick only for carriers (`tick_leashes` gets the ticking set).
  - In frozen ticks players do not push mobs (`push_from_players`) and queued block blasts (respawn anchors, crystals) wait for the next normal tick, as before this commit; only mob work runs.
  - The host's carriers use their own clock only while frozen; on unfreezing the ride clock snaps back to the level clock, so the carrier's partial tick may jump once (by less than one tick, 50 ms).
- **Test steps** (single player for 1 and 2; step 3 with the same new build in both windows: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r9-friend -Fresh -HostSound`, host: `/op Friend`):
  1. Lay netherrack 10 by 10, pour the two lava buckets into a small pool on it, hatch two striders in the lava and saddle both, and hatch a cow 5 blocks away. Hold the warped fungus on a stick and right-click one strider to mount it. Type `/tick freeze`. Hold W: the strider walks smoothly where you look; the cow and the other strider stay still (**210_strider_freeze_start**, then **211_strider_freeze_5s** after 5 seconds of walking).
  2. Crouch to get off: the strider stops at once. Mount it again: it walks. `/tick step 20`: the cow and the other strider move for one second, then stop. `/tick unfreeze`: everything runs again (**212_strider_unfreeze**).
  3. Two windows: the friend mounts the second strider holding its own fungus on a stick, then the host types `/tick freeze` (the host stays off its strider). The friend walks: both screens show the friend's strider moving smoothly; the host's empty strider and the cow stay frozen (**213_friend_strider_host**, host window, and **214_friend_strider_friend**, friend window).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*4,minecraft:saddle,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:warped_fungus_on_a_stick,minecraft:lava_bucket,minecraft:lava_bucket,minecraft:netherrack*64,minecraft:cow_spawn_egg*8
```

## Group tnt

Segment d6, brief D10 group tnt. **Notice for the owner: TNT now hurts players and mobs everywhere, in single player too.** Until this group a TNT blast broke blocks and hurt dropped items only; no mob and no player lost health.

### 1. Minecraft TNT hurts mobs and players like vanilla, before its blocks break

After 4e171e63.

- **Vanilla (26.3, javap of `ServerExplosion.explode` and `hurtEntities`, `TntBlock.prime` and `wasExploded`):**
  - `explode()` runs `calculateExplodedPositions`, then `hurtEntities`, then `interactWithBlocks` and `createFire`: every entity's exposure (`getSeenPercent`) is taken with the blocks still standing.
  - `hurtEntities` takes every entity in the box grown by twice the radius plus one, skips those whose distance over twice the radius is above 1, and gives each `hurtServer` with `getEntityDamageAmount` ((p² + p) / 2 × 7 × 2r + 1, p = (1 − distance) × exposure). The entity list is taken before the first hurt, so the loot of a mob it kills is not hurt by the same blast.
  - A primed TNT explodes with power 4 at a sixteenth of its height; its damage source is `explosion(tnt, owner)` (commit 2 adds the owner).
- **Changed:**
  - Vendored `world/src/level/mod.rs`: `Level.defer_tnt_blasts` (false by default, so the vendored level and its tests explode at once as before) and `Level.tnt_blasts`.
  - Vendored `world/src/level/explosion.rs`: `tick_tnt` pushes its `Explosion` into `tnt_blasts` instead of exploding when the level defers them.
  - `minecraft_terrain/src/server.rs`:
    - `ServerSim::new` sets `defer_tnt_blasts`.
    - New `ServerSim.blast_players`, filled by `run_tick` from the tick's `mob_players` right before `ServerSim::tick`.
    - `ServerSim::tick` drains the deferred blasts right after `level.tick()` in the new `explode_tnt`: the entity world's `blast` hurts and pushes the mobs and hands the players their `PlayerHit`s, then `level.explode` breaks the blocks and hurts the items, then the dead mobs drop their loot, then the `CreeperExplosion` goes into `explosions` for the sound, the crystals and the dragon as before.
    - The old scan for TNT about to burst (and `TNT_BLAST_RADIUS`) is gone; the radius is now the TNT's own power.
- **Tests:** `minecraft_terrain`: `server::tests::tnt_hurts_a_mob_and_a_player_before_its_blocks_break` (through `run_tick`: nothing for 79 ticks; then one explosion of radius 4; the open player 3 blocks away gets one ownerless explosion hit over 10; the player behind a 3 by 3 cobblestone wall gets one hit under 2, although the blast breaks that wall; the cow dies and, with the jar's loot, its beef lies on the ground). The other TNT tests (`flint_primes_tnt_which_blows_up_with_a_bang`, `primed_tnt_explodes_and_drops_items`, `primed_tnt_waits_while_frozen`, `an_mw2_blast_primes_tnt_it_reaches`, `tnt_beside_a_frame_breaks_it`) pass unchanged. Runs: `minecraft_terrain` 1354 (3 ignored), `minecraftoss-world` 96, all green. `check -p launcher` green.
- **Decisions:**
  - The loot of the mobs a TNT blast kills drops after `level.explode`, not before as in `explode_blasts`: vanilla takes its entity list before the first hurt, so the drops must never be hurt by their own blast. (In the test the order made no difference, but dropping after the blocks can never destroy them.)
  - Only `ServerSim::tick` drains the blasts, so the frozen tick leaves primed TNT alone as before. Tests that call `ServerSim::tick` without `run_tick` blast the mobs but no players.
  - The players hurt are the tick's `mob_players` (alive, not spectators), the same list creepers use. A creative player gets the hit, which `struck()` drops as for every hit.
- **Test steps** (single player, new world `r9-tnt`, survival):
  1. Lay a 9 by 9 floor of cobblestone. Hatch a cow, place TNT one block from it, step 3 blocks away and light the TNT with flint and steel. You lose hearts (about 7 on Normal without armour) and the cow dies and drops beef and experience (**220_tnt_hurts**).
  2. Build a 3 wide, 3 high cobblestone wall, place TNT 2 blocks in front of it, light it and run behind the wall, standing right against it. The wall breaks but you lose little or nothing (**221_tnt_behind_wall**).
  3. Place TNT with a dropped stack of cobblestone right beside it (throw a few with Q) and light it: the dropped items are blown away as before; then put a cow beside another TNT and light it: the beef lies in the crater afterwards (**222_tnt_loot_stays**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:tnt*16,minecraft:flint_and_steel,minecraft:cow_spawn_egg*8,minecraft:cobblestone*64,minecraft:cobblestone*64,minecraft:cooked_beef*32,minecraft:golden_apple*8
```

### 2. Minecraft explosions push players like vanilla

After 1ed1d4f7. Not in the brief's list of three: the brief took the pushes for granted (its owner steps say "you lose hearts and are pushed"), but no explosion pushed a player in the fork (track C's second review found the same, `docs/WEAPONS.md:621`). Without it a player spared by pvp (commit 3) would get no push at all.

- **Vanilla (26.3, javap of `ServerExplosion.hurtEntities`; the jar's `#minecraft:no_knockback` damage type tag):**
  - After `hurtServer`, every entity is pushed by direction × (1 − distance) × exposure (× (1 − `explosion_knockback_resistance`) for a living one) through `pushFromExplosion`, whether the hurt landed or not. A player that is neither a spectator nor a flying creative player also gets it in `hitPlayers`, which the explode packet hands its client.
  - `explosion`, `player_explosion` and `bad_respawn_point` are in `#minecraft:no_knockback`, so the hurt itself adds no 0.4 knockback: the blast's push is the only one.
- **Changed:** `minecraft_survival/src/entities.rs`: `route` turns the knockback of every `PlayerHitKind::Explosion` and `BadRespawnPoint` hit into that player's `out.pushes` (the same blocks-a-tick motion `players_pushed` already carries for wind bursts and the dragon), before the hit is converted, so a hit that does no damage (peaceful, a totem's grace, a creative player) still pushes. `render_anim/src/minecraft_players.rs` applies it through `launch_velocity` and `push_velocity` as before, and skips dead, spectating and creative-flying players. Creepers, ghast fireballs, beds and respawn anchors, end crystals and TNT all push now.
- **Tests:** `minecraft_survival`: `entities::tests::an_explosion_pushes_the_player_it_hit_even_when_it_does_no_damage` (the friend's TNT hit gives one push and one hit; a bad respawn point pushes the host; an arrow adds no push here; on peaceful the hit is gone but the push stays). Runs: `minecraft_survival` 744 (+1 world test), green. `check -p launcher` green.
- **Decisions:**
  - Blast Protection's `explosion_knockback_resistance` is still not applied to the push (`docs/MINECRAFT-SURVIVAL.md:423` already lists it), and MW2 blasts still push no player.
- **Test steps** (single player, `r9-tnt`, survival):
  1. Stand 3 blocks from TNT and light it: you lose hearts and are thrown away from it, a little up (**223_tnt_push**).
  2. Hatch a creeper and let it blow up beside you: you are pushed too (**224_creeper_push**).
  3. `/gamemode creative`, fly (double jump) next to lit TNT: no push. Land beside another one: pushed, no hearts lost. `/gamemode survival`.
  4. Hold a totem of undying in the offhand, take a TNT blast with half a heart left so the totem pops, then stand by a second TNT within 4 seconds: no damage, but the push (**225_totem_push**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:tnt*16,minecraft:flint_and_steel,minecraft:creeper_spawn_egg*4,minecraft:cobblestone*64,minecraft:totem_of_undying,minecraft:totem_of_undying,minecraft:cooked_beef*32,minecraft:golden_apple*8
```

### 3. Minecraft primed TNT remembers who lit it, and with pvp off a player's TNT or end crystal only pushes players

After 21862eeb. (Commit 2's log said `minecraft_survival` 745; the run there was 744, corrected above.)

- **Vanilla (26.3, javap of `TntBlock.prime` and `wasExploded`, `Explosion.getIndirectSourceEntity`, `EndCrystal.hurtServer`, `ServerPlayer.hurtServer`):**
  - `TntBlock.prime(level, pos, igniter, stack)` makes `PrimedTnt(level, x, y, z, igniter)`: flint and steel and fire charges pass the player; `prime(level, pos)` (redstone, fire, neighbours) passes null.
  - `wasExploded` primes a chained TNT with `explosion.getIndirectSourceEntity()`, which for a TNT blast is `PrimedTnt.getOwner()` (a `typeSwitch` over PrimedTnt, LivingEntity and Projectile), so a chain keeps its igniter.
  - The blast's damage source is `explosion(tnt, owner)`: `player_explosion` with the player as the causing entity. `EndCrystal.hurtServer` explodes with `explosion(crystal, source.getEntity())`, so a crystal popped by a player names them too, and its chained TNT gets no owner (the crystal is not a living source).
  - `ServerPlayer.hurtServer` refuses a hurt whose causing entity is a player when `canHarmPlayer` is false (pvp off), the igniter included. `ServerExplosion.hurtEntities` still pushes the spared player.
- **Changed:**
  - Vendored `world/src/level/entity.rs`: `TntData.owner: Option<u64>`.
  - Vendored `world/src/level/explosion.rs`: `Explosion.owner` (every literal sets it); `ignite_tnt_by(pos, owner)` and the private `prime_tnt_by`, with `ignite_tnt` and `prime_tnt` passing `None`; `new_primed_tnt` takes the owner; `tick_tnt` hands the TNT's owner to its explosion; a TNT hit by a blast is primed with the blast's owner.
  - Vendored `entities/src/world.rs`: `PlayerHitKind::Explosion { knockback, by }`, `by` set from `Blaster::Player` in `apply_explosion_by` (TNT and crystals); the bad-respawn-point conversion ignores `by`. Vendored `entities/src/world/fireballs.rs`: a fireball a player sent back names that player in `by`.
  - `minecraft_terrain/src/server.rs`: `ServerSim::item_on_block_by(player, ..)` (`item_on_block` passes `None`); `Command::ItemOnBlock` passes its player, so `PrimeTnt` primes in the player's name. `explode_tnt` blasts a TNT with an owner through `blast_by_player`, which credits the owner with the kills (experience drops, as for a player's hit) and names them in the players' hits. The other `Explosion` literals pass `owner: None`.
  - `minecraft_survival/src/entities.rs`: on the host, `spared_by_blasts_without_pvp` (next to `unharmed_without_pvp`) takes every explosion hit whose `by` is a player in the game out of `player_hits` when pvp is off and puts its knockback into `players_pushed`, so the player is pushed (the same blocks-a-tick motion commit 2 gives every blast) and loses no health. A blast by someone who left the game hurts, as vanilla's owner reference no longer resolves.
- **Tests:**
  - `minecraft_terrain`: `server::tests::chained_tnt_inherits_the_owner` (two TNT reached by a player's blast are primed in that player's name; one primed by redstone has none); `tnt_lit_by_a_player_names_them_and_credits_their_kills` (both players' hits name the igniter, the igniter included, no mob source; the cow dies and drops experience); `a_crystal_popped_by_a_player_names_them` (both hits name the player who popped it). Commit 1's test now also checks that an ownerless TNT kill drops no experience.
  - `minecraft_survival`: `entities::tests::with_pvp_off_a_players_blast_only_pushes_players` (pvp on keeps every hit; pvp off removes the hits by a player, the igniter's own included, and pushes each victim once; an ownerless blast and a blast by someone gone still hurt; through `route` both players get one push and no hit).
  - The existing TNT tests pass unchanged. Runs: `minecraft_terrain` 1357 (3 ignored), `minecraftoss-entities` 376, `minecraftoss-world` 96, `minecraft_survival` 745 (+1 world test), all green. `check -p launcher` green.
- **Decisions:**
  - The owner is a player only (the server's player id). Vanilla also keeps a mob as owner (a creeper's blast priming TNT, `getIndirectSourceEntity` of a living source); that would only change a chained TNT's death line to "blown up by Creeper", so it is left for later.
  - Primed TNT is not saved by the fork, so the owner needs no save field (vanilla saves `owner`).
  - A crystal destroyed by a player's TNT blast does not name the player for its own blast yet: the level's crystal pass (`tick_end`, `crystals.blast(.., None)`) gets no owner. Left for later.
  - The sparing runs on the host only, where every player's hit is routed, so it covers friends and the host alike.
- **Test steps** (two windows: `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r9-tnt -Fresh -HostSound`; host: `/op Friend`):
  1. Host: `/gamerule pvp false`. Both stand 2 blocks from a TNT with a cow beside it. Friend lights it with flint and steel: neither loses hearts, both are pushed, the cow dies and drops beef and experience (**226_tnt_pvp_off**, host window).
  2. Lay 4 TNT in a row; the friend lights the end one while both stand 3 blocks from the row: the whole chain spares both of you (**227_tnt_chain_pvp_off**).
  3. Place TNT with a redstone block beside it (nobody lit it): both lose hearts (**228_tnt_redstone_hurts**).
  4. Host: `/gamerule pvp true`. The friend lights TNT 2 blocks from both: both lose hearts, the friend too (**229_tnt_pvp_on**).
  5. Host: `/gamerule pvp false`. Place obsidian and an end crystal 3 blocks from both; the friend punches the crystal: both are pushed but lose no hearts (**230_crystal_pvp_off**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:tnt*32,minecraft:flint_and_steel,minecraft:flint_and_steel,minecraft:redstone_block*4,minecraft:cow_spawn_egg*8,minecraft:end_crystal*4,minecraft:obsidian*8,minecraft:cobblestone*64,minecraft:cooked_beef*32,minecraft:golden_apple*8
```

### 4. An explosion with no attacker is told as "blew up"

After fa83bb5e.

- **Vanilla (26.3, `DamageSource.getLocalizedDeathMessage`; `en_us.json`):** only a source with neither a causing nor a direct entity falls back to the kill credit (`getKillCredit`, the `.player` form such as "was blown up by Zombie"). A TNT or end crystal blast with no owner is `explosion(tnt, null)`: its direct entity is the TNT or the crystal, so the line is `death.attack.explosion` with that entity's name, whose template ignores it: "%1$s blew up", even right after a mob's or a player's hit.
- **Found:** the brief's cause ("an explosion with no source is told as a generic death") was already fixed by track C (7660486d, `DamageCause::Explosion` with the message id `explosion`). What was left: a player or mob that hit the victim in the 5 seconds before the blast turned it into "<victim> was blown up by <them>" through the kill-credit escape.
- **Changed:** `minecraft_survival/src/death.rs`: `DamageCause::has_direct_source()` (true for `Explosion`, the fork's ownerless TNT and crystal blast); `death_key` gives such a cause its own key before the `hurt_by_player` and `hurt_by` escapes.
- **Tests:** `minecraft_survival`: `death::tests::an_explosion_with_no_attacker_blew_up_even_right_after_a_hit` (a zombie's bite and the host's blow, then an ownerless blast: "%1$s blew up", killer "blew up", resolved "Friend blew up"; lava after the host's blow still says "to escape Host"). Runs: `minecraft_survival` 746 (+1 world test), green. `check -p launcher` green.
- **Decisions:** fireworks with no shooter keep the escape for now (their direct entity is the rocket too, but the fork's `DamageCause::Fireworks` also covers a rocket whose shooter is gone); left for later.
- **Test steps** (two windows as in commit 3; host: `/gamerule pvp true`, `/gamerule show_death_messages true`):
  1. Host: put a TNT and a redstone block in the inventory hotbar. With 2 hearts left, the friend hits the host once with a fist, and within 5 seconds the host places the TNT and the redstone block beside it and stays: the chat reads "<host> blew up", not "was blown up by <friend>" (**231_tnt_blew_up**, both windows).
  2. A zombie hits the friend, then the friend dies to a redstone-primed TNT: "<friend> blew up" (**232_tnt_blew_up_zombie**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:tnt*16,minecraft:redstone_block*8,minecraft:zombie_spawn_egg*2,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

### 5. A player's Minecraft TNT is told as "was blown up by <player>"

After 8a834f02. Not in the brief's list: commit 3 names the igniter in the hit, and this commit gives that name to the death line and the kill credit.

- **Vanilla (26.3, `DamageSource.getLocalizedDeathMessage`, `CombatTracker.getFallMessage`; the jar's `damage_type/player_explosion.json` and `en_us.json`):** `player_explosion` has the message id `explosion.player` and scales with difficulty always. With a causing player the line is `death.attack.explosion.player` ("%1$s was blown up by %2$s"), or its `.item` form ("... using %3$s") when the player's main-hand item has a custom name. The player becomes the victim's `lastHurtByPlayer`, so a fall right after is "doomed to fall by", and lava "to escape", that player.
- **Changed:**
  - `minecraft_survival/src/death.rs`: `DamageCause::PlayerExplosion { client }` (damage type `minecraft:player_explosion`, in `IS_EXPLOSION`, so a totem's grace refuses it too). It enters combat and sets the player kill credit like `Player`; its death line goes through `by_player` with the key `death.attack.explosion.player` (its `.item` form is added to `ITEM_FORMS`), and an igniter who left falls back to "blew up". A fall after it is "death.fell.assist" naming the player.
  - `minecraft_survival/src/entities.rs`: `mob_hit` gives an explosion hit with `by` the cause `PlayerExplosion`.
- **Tests:** `minecraft_survival`: `death::tests::a_players_blast_blows_up_by_name_and_counts_as_their_hit` (the damage type and message id; "%1$s was blown up by Host", with "using [Excalibur]"; "blew up" after the host left; "doomed to fall by Host"; lava "to escape Host"); `entities::tests::an_explosion_pushes_the_player_it_hit_even_when_it_does_no_damage` also checks that a hit with `by` routes as `PlayerExplosion`. Runs: `minecraft_survival` 747 (+1 world test), green. `check -p launcher` green.
- **Decisions:** the client id is the server's player id (`u32::try_from`, as `pvp::hit` does), so the chat roster names it for host and friends alike.
- **Test steps** (two windows as in commit 3; host: `/gamerule pvp true`):
  1. The friend lights TNT next to the host, who stays: "<host> was blown up by <friend>" in both chats, and the host's kill card names the friend (**233_tnt_blown_up_by**).
  2. The friend renames the flint and steel "Boom" in an anvil, holds it and lights TNT next to the host: "<host> was blown up by <friend> using [Boom]" (**234_tnt_blown_up_using**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:tnt*16,minecraft:flint_and_steel,minecraft:flint_and_steel,minecraft:anvil,minecraft:experience_bottle*16,minecraft:cobblestone*64,minecraft:cooked_beef*16
```

### 6. Track D's TNT pvp test passes clippy

After 96d39045. Clippy on `minecraft_terrain` and `minecraft_survival` (tests included, `-A clippy::approx_constant`) flagged one line of this group: `field_reassign_with_default` in `entities::tests::with_pvp_off_a_players_blast_only_pushes_players` (commit 3), now a struct literal. The other warnings in the touched files (`server.rs:649`, `:3263`) are upstream's. No game change, no test steps, no kit.

## Group small-not-vanilla

Segment d6, brief D10 group small-not-vanilla: four small not-vanilla items, one commit each.

### 1. A Minecraft spawner that refuses an egg says so in chat

After 2886e3fa.

- **Vanilla (26.3, javap of `SpawnEggItem.useOn`):** on a spawner block entity, when `ServerLevel.isSpawnerBlockEnabled()` is false, the server calls `ServerPlayer.sendSystemMessage(Component.translatable("advMode.notEnabled.spawner"))` for the player who used the egg (a chat line, not the action bar, no colour) and returns `FAIL`, so the egg is kept. Nobody else sees it.
- **Changed:**
  - `minecraft_terrain/src/spawn_egg.rs`: `SPAWNER_DISABLED` (`advMode.notEnabled.spawner`), which `ServerSim::spawner_egg` now returns instead of the literal.
  - `minecraft_survival/src/entities.rs`: `PlayerOutputs.told` (chat lines for that one player). The egg answers are settled in the new `settle_eggs`; an answer refused with `SPAWNER_DISABLED` adds the translatable system line through `egg_refusal`. The other refusals (no spawner there, a monster in peaceful, an unsimulated mob) stay silent and logged, as vanilla's `canSpawn` failure is.
  - `render_anim/src/minecraft_players.rs`: `struck` returns that player's `told` lines. The host shows them in its own chat (`minecraft_world.rs`); a friend's go into `Stepped.lines`, which the host sends to that friend only as `ToClient::Chat`.
- **Tests:** `minecraft_survival`: `entities::tests::a_spawner_that_refuses_an_egg_tells_only_the_player_who_used_it` (the friend's refused egg gives one system line that resolves to the language's text, not an overlay; the friend's hatched egg and the host's "no spawner" answer give none; the refused egg is kept, the hatched one spent). Runs: `minecraft_survival` 748 (+1 world test), `minecraft_terrain` spawner tests 29, `render_anim` 210, all green. `check -p launcher` green.
- **Decisions:** the line rides the existing per-player chat paths (the host's chat box and the friend's relayed `ToClient::Chat`), so no message or protocol change; `PROTOCOL` stays 6.
- **Test steps** (two windows, host and friend in the same overworld spot, both in survival):
  1. Host: `/gamerule spawner_blocks_work false`. Host places a spawner and right-clicks it with the pig egg: the host's chat shows "Spawner blocks are not enabled" (in the game's language) and the egg count stays the same. The friend's chat shows nothing (**235_spawner_refused**, both windows).
  2. Friend right-clicks the same spawner with the zombie egg: only the friend's chat shows the line, and the friend keeps the egg (**236_spawner_refused_friend**, both windows).
  3. `/gamerule spawner_blocks_work true`: the pig egg on the spawner now sets a pig in the cage, the egg is spent, and no line appears (**237_spawner_accepts**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:spawner*2,minecraft:pig_spawn_egg*4,minecraft:zombie_spawn_egg*4,minecraft:cooked_beef*16
```

### 2. The Minecraft enchanting seed is saved with the player

After 5f6b7017.

- **Vanilla (26.3, javap of `Player.addAdditionalSaveData` and `readAdditionalSaveData`):** the player file holds `XpSeed` as an int (`putInt("XpSeed", enchantmentSeed)`), read back with `getIntOr("XpSeed", 0)`. The enchanting table's three offers come from that seed and the bookshelves; the seed changes only when the player enchants.
- **Changed:**
  - `minecraft_survival/src/player_nbt.rs`: `PlayerData.xp_seed`, read from and written as `XpSeed` (`Tag::Int`). Like `PortalCooldown` it is not in `OWNED`, so a `PlayerData` without a seed keeps the one already in the file.
  - `minecraft_survival/src/player.rs`: `Player::data` saves `enchantment_seed`.
  - `minecraft_survival/src/server.rs`: `join_from` restores it when the file has one. Every player (host and friends) has their own file under `players/data/<uuid>.dat`, so the seed is per UUID.
- **Tests:** `minecraft_survival`: `player_nbt::tests::the_enchanting_seed_is_an_int_named_xp_seed_like_vanilla` (an int, read back, missing reads as none, an unset seed keeps the file's, a set one replaces it); `server::tests::a_joining_player_keeps_the_enchanting_seed_their_file_saved` (a file without `XpSeed` gives the UUID-derived seed, a saved seed comes back, another player keeps their own); the world test `a_saved_world_comes_back_whole_with_vanilla_keys_and_tag_types` now expects `XpSeed` as an int in the player file and round-trips a seed. Runs: `minecraft_survival` 750 plus the world test, green. `check -p launcher` green.
- **Decisions:** a file without `XpSeed` (every player file the fork wrote before this commit) gets the seed derived from the UUID, as `Player::new` already gave, instead of vanilla's 0. So a player's table offers stay what they were before the update, and two players of an old world do not share seed 0. A vanilla 26.3 file always has `XpSeed`, so for those nothing differs from vanilla.
- **Test steps** (single player, then a friend):
  1. Place the enchanting table, drink experience bottles to level 3 or more, put the iron sword and lapis in the table and note the three offers (**238_enchant_offers_before**).
  2. Take the sword out, save and quit to the menu, load the world again and put the sword back: the same three offers (**239_enchant_offers_after**).
  3. Enchant once: the offers change. Save, quit and load again: the new offers stay (**240_enchant_offers_kept**).
  4. Friend window: the friend does steps 1 and 2 on the same table, leaving and rejoining instead of the host's restart: the friend sees their own offers, the same before and after, and the host's do not change (**241_enchant_offers_friend**, both windows).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:enchanting_table,minecraft:lapis_lazuli*32,minecraft:experience_bottle*32,minecraft:iron_sword,minecraft:iron_sword,minecraft:book*4
```

### 3. An MW2 gun hit wakes a sleeping Minecraft player

After 83e369cf.

- **Vanilla (26.3, javap of `LivingEntity.hurtServer`):** right after the invulnerability and `isDeadOrDying` checks, a sleeping entity calls `stopSleeping()`, before armour, absorption and the damage amount. So any accepted hit wakes a sleeper at once.
- **Cause:** a sleeper wakes when `Survival::hurts()` changes (`sleep.rs`, the check next to the broken bed). Only Minecraft damage counted it (`Survival::record`). MW2 damage (bullets, grenades, knives) lowers the MW2 health, which `Survival::observe` follows each frame for every player on the host (`minecraft_players::life`, run for the host and for each friend), but it never counted as a hurt.
- **Changed:** `minecraft_survival/src/survival.rs`: `observe` counts a hurt when the MW2 health drops (also when absorption takes it all, as vanilla wakes before absorption). Nothing in `sim/src/damage.rs` (track C's file) changed: the hook is on the Minecraft side, per player.
- **Tests:** `minecraft_survival`: `sleep::night_tests::a_gun_hit_wakes_the_sleeper_like_a_minecraft_hurt` (two sleepers; an unchanged MW2 health wakes nobody; a 30-point MW2 drop takes 6 Minecraft health from the one hit and wakes them at once with the timer at 0, while the other stays in bed). Runs: `minecraft_survival` 751 plus the world test, green. `check -p launcher` green.
- **Decisions:** `hurts` is read only by the sleep check, so the extra count changes nothing else.
- **Test steps** (two windows, overworld, `/time set night`, `/gamerule pvp true`):
  1. Host and friend each place a bed and the host lies down. The friend shoots the host once with the M4: the host gets up at once beside the bed, without the slow fade (**242_gun_wakes_host**, both windows).
  2. The friend lies down and the host shoots the friend once: the friend gets up at once (**243_gun_wakes_friend**, both windows).
  3. Both lie down again and nobody shoots: the night is skipped as before (**244_sleep_still_skips**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=iw4l:m4,iw4l:rifle_ammo*64,minecraft:white_bed,minecraft:red_bed,minecraft:cooked_beef*16
```

### 4. A ghast's or blaze's fireball makes a hoglin retaliate

After cedee1fd.

- **Vanilla (26.3, `Hoglin.hurtServer` and `HoglinAi.wasHurtBy`; `LargeFireball.onHit`, `ServerExplosion.hurtEntities`):** a hoglin that takes damage calls `wasHurtBy` with `source.getEntity()`. A ghast's large fireball explodes with `fireball(fireball, ghast)`, whose entity is the ghast, so a hoglin hurt by the blast goes for the ghast (and calls the visible adults); a blaze's small fireball hits with `fireball(fireball, blaze)` the same way. Piglins do the same through `PiglinAi.wasHurtBy`.
- **Found:** the direct hit already noted the attacker (`apply_arrow_hit` calls `note_hoglin_hurt_by_owner`), so a blaze's small fireball and a large fireball that struck the hoglin itself worked. What was missing was the large fireball's blast: `fireball_explosion` hurt the hoglin through `hurt_by_blast` and never noted who sent it.
- **Changed:** vendored `entities/src/world/fireballs.rs`: `fireball_explosion` notes the blaster (the ghast, or the player who sent the fireball back) on every mob its blast hurts, through `note_hoglin_hurt`, which falls back to `note_piglin_hurt` for piglins and does nothing for other mobs.
- **Tests:** `minecraftoss-entities`: `world::hoglins::tests::a_ghasts_fireball_blast_and_a_blazes_fireball_make_a_hoglin_fight_back` (a ghast's fireball landing 1.6 blocks from a hoglin hurts it and sets its attack target on the ghast; it fails without the fix; a blaze's small fireball sets it on the blaze). Runs: `minecraftoss-entities` 377, `minecraft_terrain` 1357 (3 ignored), green. `check -p launcher` green.
- **Decisions:** piglins hurt by a ghast's blast now also turn on the ghast, as vanilla's `PiglinAi.wasHurtBy` does. A hit by both the fireball and its blast notes the ghast twice in the same tick; the second note repeats the same target and changes nothing.
- **Test steps** (single player, Nether, a 9 x 9 netherrack platform in an open area, `/gamemode creative` so the mobs leave you alone):
  1. Hoglin egg in the middle of the platform. Ghast egg 12 blocks away and a few blocks up, then `/gamemode survival` and stand so the ghast shoots at you with the hoglin between you and the ghast; step aside so the fireball lands next to the hoglin. The hurt hoglin turns and charges the ghast, jumping at it under it (**245_hoglin_vs_ghast**).
  2. `/gamemode creative`. Blaze egg 8 blocks from a new hoglin, then `/gamemode survival` with the hoglin between you and the blaze: when a blaze fireball hits the hoglin, it goes for the blaze (**246_hoglin_vs_blaze**).
  3. Repeat step 1 with a piglin instead of the hoglin: the piglin shoots or runs at the ghast (**247_piglin_vs_ghast**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:hoglin_spawn_egg*4,minecraft:ghast_spawn_egg*2,minecraft:blaze_spawn_egg*2,minecraft:piglin_spawn_egg*2,minecraft:netherrack*64,minecraft:shield,minecraft:cooked_beef*16
```

### Group notes (small-not-vanilla)

- All four items were small enough to do; none was skipped.
- `PROTOCOL` stays 6: no `minecraft_net` message or encoding changed in this group.
- Owner's D10 step 5 is covered by commit 1 step 1 (the spawner line) and commit 2 steps 1 to 3 (the table's offers after a restart).
- Group kit (all four commits, 19 stacks):

```
IW4L_MINECRAFT_KIT=minecraft:spawner*2,minecraft:pig_spawn_egg*4,minecraft:zombie_spawn_egg*4,minecraft:enchanting_table,minecraft:lapis_lazuli*32,minecraft:experience_bottle*32,minecraft:iron_sword,minecraft:book*4,iw4l:m4,iw4l:rifle_ammo*64,minecraft:white_bed,minecraft:red_bed,minecraft:hoglin_spawn_egg*4,minecraft:ghast_spawn_egg*2,minecraft:blaze_spawn_egg*2,minecraft:piglin_spawn_egg*2,minecraft:netherrack*64,minecraft:shield,minecraft:cooked_beef*16
```

## Review (d6)

Adversarial review of the groups friend-hand to small-not-vanilla (`r9-d6-base..99e46d92`).

### 1. A client in an MW2 match cannot start a board its host never moves

After 99e46d92.

- **Found:** group friend-skating (`0a36201a`) let `skate::update` run on any client with a `ClientPredictionState`, so it no longer returned early without an `AuthorityWorld`. Skating also works on MW2 maps, so a client in an upstream MW2 match (two players over the relay, no Minecraft world) with skating on could press J: its own prediction took the board's place, while the MW2 host, which has no skater handling, kept the client where it stood. The two disagreed until the board was picked up. Before the group the key did nothing on such a client.
- **Changed:** `render_anim/src/skate.rs`: a client skates on its own prediction only while a Minecraft world is loaded (`MinecraftWorldView.active`, the friend's streamed world). Anywhere else a client picks up a board it still has (left over from a Minecraft session) and returns. The host and single player still skate on their `AuthorityWorld` as before.
- **Tests:** `render_anim`: `skate::tests::only_a_minecraft_friend_skates_on_its_own_prediction`. `check -p launcher` green.
- **Test steps:**
  1. MW2 match through the relay (not a Minecraft world), skating on in Options > Controls > Minecraft on the client. Client: press J. No board appears, the client keeps walking normally and the host sees it where the client sees itself (**248_mw2_client_no_board**, both windows).
  2. Minecraft world with a friend: friend presses J on flat ground. The friend skates as in group friend-skating (**249_friend_still_skates**, both windows).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cooked_beef*16
```

### 2. A rocket with no shooter is told as "went off with a bang" after a fight

After 2b9614a3.

- **Vanilla (26.3, `DamageSource.getLocalizedDeathMessage`):** the `.player` form ("... while fighting X", the kill credit of `getKillCredit`) is used only when the damage source has neither a causing nor a direct entity. A firework rocket is always the direct entity of its `fireworks` damage, so a rocket with no shooter reads "went off with a bang" even right after a player's or a mob's hit.
- **Found:** group tnt (`8a834f02`) gave `DamageCause::Explosion` (TNT with no owner) its direct source, but `DamageCause::Fireworks` (a rocket with no shooter) still fell through to the kill credit, which group pvp-melee had just extended to players: "went off with a bang while fighting Host". The log listed it under the TNT leftovers.
- **Changed:** `minecraft_survival/src/death.rs`: `has_direct_source` covers `Fireworks` as well as `Explosion`. The TNT leftover line drops the fireworks item.
- **Tests:** `minecraft_survival`: `death::tests::a_villager_s_firework_makes_the_player_go_off_with_a_bang` now also bites, hits by a player and then kills by a sourceless rocket, and expects "went off with a bang". `check -p launcher` green.
- **Test steps:** the fork launches no rocket without a shooter yet (raid villagers' rockets have one), so the case itself is covered by the unit test. To check that the kill credit still works (`/gamerule pvp true`):
  1. Friend hits the host once with the sword; within 5 seconds the host walks into lava and dies. The chat reads "Host tried to swim in lava to escape Friend" (**250_lava_escape_still_named**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:iron_sword,minecraft:lava_bucket,minecraft:cooked_beef*16
```

## Combined kit

One kit for the owner's test of the whole track (27 stacks). The ancient city steps of group loot-functions keep their own kit (a pickaxe, torches and night vision). Groups pillagers to raid-villagers (d2, from `r9-d2-base`) have their own combined kit, the second line: swap the `.env` line between the two parts of the test. The village steps of group raid-villagers use a plains village's own beds and doors; group pillagers' crossbow step uses that commit's own kit (it adds `minecraft:crossbow`).

```
IW4L_MINECRAFT_KIT=minecraft:drowned_spawn_egg*16,minecraft:villager_spawn_egg*8,minecraft:zombie_spawn_egg*8,minecraft:zombie_villager_spawn_egg*4,minecraft:composter,minecraft:emerald*64,minecraft:wheat*64,minecraft:splash_potion@weakness,minecraft:golden_apple*16,minecraft:ominous_bottle[ominous_bottle_amplifier=4]*2,minecraft:milk_bucket,minecraft:cat_spawn_egg*8,minecraft:cod*64,minecraft:horse_spawn_egg*4,minecraft:donkey_spawn_egg*2,minecraft:llama_spawn_egg*4,minecraft:saddle*4,minecraft:golden_carrot*16,minecraft:chest*4,minecraft:lead*16,minecraft:oak_fence*16,minecraft:wandering_trader_spawn_egg*2,minecraft:iron_block*8,minecraft:carved_pumpkin*4,minecraft:snow_block*4,minecraft:shears,minecraft:cooked_beef*32
```

d2 groups (pillagers, patrols-vindicators, evokers-vexes, ravagers, raids, raid-villagers), 27 stacks:

```
IW4L_MINECRAFT_KIT=minecraft:pillager_spawn_egg*16,minecraft:vindicator_spawn_egg*8,minecraft:evoker_spawn_egg*8,minecraft:vex_spawn_egg*4,minecraft:ravager_spawn_egg*4,minecraft:witch_spawn_egg*2,minecraft:villager_spawn_egg*8,minecraft:iron_golem_spawn_egg*2,minecraft:sheep_spawn_egg*4,minecraft:cow_spawn_egg*2,minecraft:ominous_bottle*4,minecraft:milk_bucket,minecraft:bell,minecraft:lever*2,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:totem_of_undying,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:spyglass,minecraft:oak_leaves*64
```

## Protocol

Before the merge of minecraft-survival (tracks A, B and C) into track D, `minecraft_net::PROTOCOL` was 4 (from 3) on this track, bumped once in group trading-friend. Since that merge (`r9-d2-base`) it is 5, the number tracks A, B and C took together: protocol 5 has not been released (release 1 is protocol 4), so no released build shares the number with a different message set. `ToServer::OpenMountInventory` moved to codec index 16, after track A's `CreativeSlot` (13) and track C's `RecipeBookSettings` (14) and `RecipeSeen` (15). Later groups of this track must not bump it again.

Protocol 5 shipped in round 9 release 2. Group friend-hand (commit 1, `UsingView.kind`) set it to 6, the one bump allowed after release 2; every later message change of round 9 rides in 6 and never goes above it.

Group skater-poses changed `SkateReport` (new `pose`) and `StateFrame` (new `skaters`) and added `SkatePose` and `Skaters`, all inside protocol 6 (still unreleased), with no bump.

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
- `docs/MINECRAFT-SURVIVAL.md`: new paragraph: "Ravagers are live mobs. They hatch from their egg and are saved, mid-stun or mid-roar too, with their raid and patrol state. A ravager hunts players, adult villagers, wandering traders and iron golems within 32 blocks, runs them down (faster with a target) and bites for 12 on Normal, throwing its victim forward, then stands still for half a second. A bite blocked by a shield either stuns it for 2 seconds (dizzy grey swirls, then a roar) or flings the shield bearer away, half and half. Its roar hurts everything within 4 blocks but illagers for 6 and flings it all away, players too. With `mob_griefing` on it breaks the leaves it walks into, and it steps up whole blocks. A pillager, vindicator or evoker can ride it (from saves and commands; raids come later) and fights from its back while it hunts; another mob on its back holds it still. Iron golems hunt ravagers and villagers flee them within 12 blocks. It drops a saddle and 20 experience."
- `docs/MINECRAFT-SURVIVAL.md`: in the ravager paragraph, add: "Ravagers are drawn like vanilla: the neck lunges and the jaw snaps as it bites, the head hangs and sways under grey swirls while it is stunned, and the jaw opens wide with a ring of puffs when it roars. They turn in spawner cages. `/summon` riders sit on their mount."
- `docs/MULTIPLAYER.md`: a friend's shield block on a ravager's bite is reported to the host by the player step (`Entities::shield_blocked`, the server's `ShieldBlock`), which answers with the friend's push in `players_pushed`; friends see the bite, stun and roar clocks through the new `MobShow.ravager` (no new message); MW2 bullets hit ravagers through hit key 34.
- `docs/MINECRAFT-SURVIVAL.md`: in the player effects paragraph, "Bad Omen does not yet turn into Raid Omen in a village, Raid Omen does not yet start a raid" becomes "Bad Omen turns into Raid Omen in a village and Raid Omen starts a raid (see Raids)". New paragraph "Raids": "Raids come like vanilla. A player with Bad Omen who walks into a village (not on Peaceful, not as a spectator) gets Raid Omen for 30 seconds at the same level; when it ends, a raid starts at the middle of the village's claimed points of interest, or grows by the omen's level when one is already there (the `raids` game rule; never in the Nether, whose `gameplay/can_start_raid` is false). After 15 seconds the raid horn sounds and the waves come from outside the village (Easy 3, Normal 5, Hard 7, and a bonus wave from omen level II): pillagers, vindicators, ravagers (wave 5 with a pillager on its back, from wave 7 with an evoker or vindicators), witches and evokers by vanilla's table, each wave led by a captain with the ominous banner who drops an ominous bottle. From omen level II some pillager crossbows get Quick Charge and some vindicator axes Sharpness. Raiders walk to the village and from house to house, call raiders near them into the raid, never despawn while in it and leave it if they stray 112 blocks away. Every player within 96 blocks of the raid sees the red bar notched in ten: "Raid" filling between waves and emptying with the raiders' health, "Raid - Raiders Remaining: N" at the last two, "Raid - Victory" or "Raid - Defeat" for 30 seconds at the end. When the last wave falls, every player who killed a raider gets Hero of the Village (the omen level, 40 minutes); when the village loses its last claimed point of interest (its villagers dead) the raid is lost and the raiders celebrate. Raids are saved in each dimension's `data/minecraft/raids.dat`, and the player's raid omen position in the player file. Villagers do not hide or ring bells yet, vindicators do not break doors yet, and raiders do not pick up banners." In the witch paragraph (or the raids one): "Witches in a raid heal the other raiders: off a 5-second cooldown a witch throws a splash potion of healing at a raider in sight with 2 hearts or less, else regeneration, and picks no new player to attack while the cooldown runs."
- `docs/MINECRAFT-SURVIVAL.md`: in the pillager, vindicator, evoker and ravager paragraphs, "Raids are not there yet", "raids come later" and "Door breaking waits for raids" become "They join raids (see Raids)"; the vindicator keeps "Door breaking in raids is not there yet".
- `docs/MULTIPLAYER.md`: raid bars reach friends as `ToClient::BossBar` (id `0x100 + raid id`, red, notched 10; `Name` and `Progress` updates, `Remove` when the player leaves the raid's range or it ends); the horn is a sound to that player alone; the friend's bad omen, raid omen and hero of the village run like the host's (the omen hand-over rides in the host's tick input as `PlayerOmens`). No new message; protocol 5.
- `docs/MINECRAFT-SURVIVAL.md`: in the raids paragraph, "Villagers do not hide or ring bells yet, vindicators do not break doors yet, and raiders do not pick up banners." becomes "Vindicators do not break doors yet, and raiders do not pick up banners." New paragraph "Bells and villagers in raids": "Bells ring like vanilla: right-clicked with the main hand on one of their sides below the rim (a floor bell from the front or back, a wall bell from its sides, a hanging bell from any side) or given a new redstone signal. Every villager within 32 blocks hears it and, outside a raid, runs to a home (a bed) nearby and hides until 15 seconds after the ring. When raiders are within 32 blocks the bell resonates a quarter second later, and two seconds after that every raider within 48 blocks glows for 3 seconds: a white rim around it in the open, a white shape of it seen through blocks. In a raid, villagers gather at their bell before the first wave and now and then ring it, hide at home while the waves come, and after a victory come out under the open sky, cheer and launch firework rockets that burst into coloured sparks with a flash and a bang (a low burst hurts whoever stands within 5 blocks: "went off with a bang"). A player with Hero of the Village gets gifts: every 30 seconds to 5 minutes each villager that sees the hero walks up and throws a gift from vanilla's hero of the village loot (a farmer bread, pumpkin pie or a cookie, a child a poppy, and so on). Friends ring bells, see the glow and get gifts the same way. The bell does not swing yet, arrows and wind charges do not ring it yet, and frightened villagers do not sweat yet."
- `docs/MULTIPLAYER.md`: a friend's bell use is a hand use on the host (no new message); glowing mobs reach friends as `MobShow.glowing`; hero gifts are item entities thrown on the host; firework rockets reach friends as thrown views and their bursts as `EntitySnapshot.firework_bursts`. Protocol 5.
- `docs/MINECRAFT-SURVIVAL.md` (review fix 4): in the ravager paragraph, "Its roar hurts everything within 4 blocks but illagers for 6 and flings it all away, players too." becomes "Its roar hurts everything within 4 blocks but illagers for 6 and flings every mob away; players only take the blow."
- `docs/MULTIPLAYER.md` (group friend-hand, commit 1): `minecraft_net::PROTOCOL` is 6 (the code block at line 162 and the note at line 1415). `PlayerView.using` (`UsingView`) now carries `kind: UseKind { Eat { duration }, Block, Charge, Scope }`. In section 20, "the friend's first-person hand" becomes: "A friend draws their own first-person held items like the host: the selected item, the offhand item while holding an item or using one, eating and drinking at the mouth, the raised shield, the trident pulled back and the spyglass hiding the items. The use comes from the friend's `PlayerView.using` and is counted on between views on the friend's own tick. The items hide in F5, in bed, while skating, dead or with the UI off. The swing is not shown yet." No vendored file changed.
- `docs/MULTIPLAYER.md` (group friend-hand, commit 2): new message `ToClient::Animate { client, offhand }` (protocol 6). In the section 20 text of commit 1, "The swing is not shown yet." becomes: "A friend's own attack press swings at once on the friend, with vanilla's restart rule and the 10-tick miss time; every other swing the host starts for that friend (placing, using, digging) reaches only that friend as `ToClient::Animate`." Line 1303 drops "the friend's first-person hand", and the leftover item at line 1326 ("**The friend's first-person hand:** ...") is removed.
- `CLAUDE.md` roadmap item 2: drop "the friend's first-person hand" from the multiplayer leftovers.
- `docs/MINECRAFT-SURVIVAL.md` (group friend-portal-music, commit 1): in "What a friend's game still lacks" (line 369) the item "the purple overlay while standing in a portal (the trigger sound plays);" is removed.
- `docs/MULTIPLAYER.md` (group friend-portal-music, commit 1): add to section 20's list of what friends have: "A friend draws its own purple portal overlay from its own position on its streamed terrain (vanilla's client-side portal effect), closes its open screen inside a portal, and starts the effect again on a new level; the host still decides the travel and sends the trigger and travel sounds." No message change.
- `docs/MULTIPLAYER.md` (group friend-portal-music, commit 2): the leftover item at line 1327 ("**Music on the friend's game** ...") is removed, and line 1433 drops ", and music on the friend's game" (ending "... (section 18.3)."). New wording in section 20: "Each game plays its own Minecraft music, as vanilla's client-side `MusicManager` does: a friend hears the End music, the dragon music while a boss bar asks for music, and the credits music, from its own level, boss bars and credits." No message change.
- `docs/MINECRAFT-SURVIVAL.md` (group friend-portal-music, commit 2): in "What a friend's game still lacks" (line 367) the item "music of any kind;" is removed. `CLAUDE.md` names neither the friend's music nor its portal overlay, so it is unchanged.
- `docs/MINECRAFT-SURVIVAL.md` (group pvp-melee, commit 1): line 374 drops "Minecraft melee against another player (swords and axes reach only mobs),"; line 425's shield note becomes "a mob's or a player's axe hit disables a raised shield for 5 seconds; MW2 bullets ignore it." New wording for the multiplayer section: "Minecraft melee hits other players like vanilla `Player.attack`: the attack strength scale, crits, sprint knockback, Sharpness and Knockback, the sword's sweep (players and mobs beside the target, and players beside a mob target), the shield and the axe's 5-second shield disable, armour, Protection and the totem. With the `pvp` game rule off a hit makes only the no-damage sound. Creative and spectator players cannot be hurt; spectators cannot be hit at all. Swords and axes wear by their `item_damage_per_attack` on mobs and players."
- `docs/MULTIPLAYER.md` (group pvp-melee, commit 1): line 783 ("Not covered") drops its first two sentences about melee, line 1303 drops "and Minecraft melee between players", line 1321's leftover item is removed, and section 20 gains: "Minecraft melee between players runs on the host: each frame the host gives every level the boxes of its living, non-spectator players in that dimension's band; the attacker's `act()` hits the nearest of a frame, mob, End part, bullet, fireball or player before the block, and a hit is a `Hit` with `DamageCause::Player` in the victim's queue, which `struck()` applies with the shield, armour and totem. A friend aims at the other bodies it draws and sends `Target::Player`; the host checks reach. No message change."
- `CLAUDE.md` roadmap item 2: drop "Minecraft melee between players" from the multiplayer leftovers.
- `docs/MINECRAFT-SURVIVAL.md` (group pvp-melee, commit 2): line 374's "A gun hit followed by a fall or lava is told as a plain fall or lava death" stays for guns. New wording: "A player killed by another player's Minecraft melee gets vanilla's line: '<victim> was slain by <killer>', with 'using [<name>]' only for a custom-named weapon; a melee hit before a long fall gives 'was doomed to fall by <killer>', and lava, fire and the like within 5 seconds of a melee hit name the player escaped. Melee kills give no MW2 kill credit."
- `docs/MULTIPLAYER.md` (group pvp-melee, commit 2): line 1427 ("There is no `DamageCause::Player`") becomes "`DamageCause::Player { client }` is the Minecraft melee cause; its death lines take the killer's name from the chat roster and the weapon's custom name from the killer's held item when the line is made. A gun kill's line is still built from the sim's death (E18)."

- `docs/round8/friend-install.md` (group friend-skating): line 104 (`IW4L_SKATE=off`) becomes: "`IW4L_SKATE=off`: the zip has no Skate 3 converter, and without this line the first start stops to ask for Skate 3. A friend who wants to skate needs their own extracted Skate 3 (`default.xex` with its `data` folder) and the converter: drop this line, put `skate\iw4l-skate-convert.exe` (with `skate\licenses\`) from the host's game folder beside the exe, and set Skating to Enabled in Options > Controls > Minecraft; the next start asks for `default.xex` and converts it into `skate-data`." Line 162 drops "skating," from the list of what friends still miss.
- `docs/MULTIPLAYER.md` (group friend-skating): the leftover item at line 1325 ("**Skating for friends:** ...") becomes: "**Skating for friends:** a friend skates on its own Skate 3 data. Its skate worker moves its own predicted player (`ClientPrediction::set_external_motion` and `set_origin`, held through each snapshot), and it reports the skater to the host: every new pose as `StateUp::Skate(SkateReport)` on the state lane, and `ToServer::Skate` with `on: false` on the reliable lane when the board is picked up. The host accepts a report only while the friend is alive, not switching, allowed the board (the host's own step of that friend) and within 40 blocks/s plus 10 blocks of its last place, outside blocks; then it moves the friend with external motion and runs the friend's Minecraft rules with `Skating` (no movement food or footsteps, fall damage on landing, no item use). Otherwise, and on death, dimension changes, teleports or 2 s of silence, it sends `ToClient::SkateRefused` and the friend picks the board up. Others see the friend move in the idle pose without a board until the skater pose is sent." Line 1303 (the leftover list) drops "skating for friends". The state lane note gains `StateUp`; protocol stays 6.
- `docs/MINECRAFT-SURVIVAL.md` (group friend-skating): line 364 ("skating (the skate key does nothing for a friend, and the host's board shows only on the host's screen);") becomes "the skate pose and board of other players (each game sees only its own board);". Line 418 drops "a friend can't skate and"; the skating paragraph (line 140) gains: "Friends skate the same way with their own Skate 3 data; the host checks each move and picks the board up when it is too far, inside blocks or no longer allowed."
- `CLAUDE.md` roadmap item 2: drop "skating for friends" from the multiplayer leftovers; item 4 becomes "Skating already works in Minecraft worlds for the host and friends (round 8 track D, round 9 track D), but nobody sees another player's skate pose yet."
- `docs/MULTIPLAYER.md` (group skater-poses): in the friend-skating item above, "Others see the friend move in the idle pose without a board until the skater pose is sent." becomes: "Every skater's pose reaches the others: the friend's state report carries the root and 23 bones (the 22 mapped skater bones and the board root) as 3x4 floats, and each state frame carries `Skaters`, every other skater in the receiver's level with its pose, the host's own included. Each game shows another skater between the last two poses it heard, one interval behind, and draws the soldier in the skate pose on a board; a game without `rig.json` draws the standing soldier with no board. About 23 KB/s per skater in each direction." The state lane note gains `StateFrame.skaters` and `SkateReport.pose`; protocol stays 6.
- `docs/MINECRAFT-SURVIVAL.md` (group skater-poses): the item "the skate pose and board of other players (each game sees only its own board);" in "What a friend's game still lacks" is removed. The skating paragraph gains: "Every player sees every skater in the same dimension on a board in the skate pose, the host and friends alike. A skater holds no item in its hands for the others."
- `CLAUDE.md` (group skater-poses): roadmap item 4 becomes "Skate 3. Skating works in Minecraft worlds for the host and friends, and everyone sees every skater's pose and board (round 8 track D, round 9 track D)."
- `docs/MINECRAFT-SURVIVAL.md` (group strider-freeze): in the **Frozen ticks** item (line 129), after "... mobs, projectiles and the End." add: "A mob carrying a player keeps ticking, as in vanilla: the whole stack under a player rider (its vehicle, that vehicle's own vehicle and their other mob riders) walks, steers and is drawn on its own clock, for the host's and the friends' rides alike; the game time stays frozen."
- `CLAUDE.md` (group strider-freeze): roadmap item 2's last sentence drops ", such as a ridden strider standing still under `/tick freeze`".
- `docs/PROGRESS.md` (group strider-freeze): the open not-vanilla item "ridden strider under `/tick freeze`" (line 10) is done by track D, group strider-freeze; it answers `docs/round8/decisions.md:23`.
- `docs/MINECRAFT-SURVIVAL.md` (group tnt, commit 1): line 430 ("TNT explosions don't hurt mobs or players; only creeper explosions do.") is removed. The TNT item at line 126 ("TNT explodes.") becomes: "TNT explodes and hurts the mobs and players around it like vanilla, with their exposure taken before its blocks break; the mobs it kills drop their loot after the blast."
- `docs/WEAPONS.md` (group tnt, commit 2; track C's file): line 621 becomes "- Explosion knockback still pushes the player: a Minecraft blast's push reaches the player as `out.pushes` whether the hurt lands or not, as vanilla pushes an entity whose hurt is refused. MW2 blasts push no player."; line 748's step 3 ends "no damage (and no push: MW2 blasts push no player)"; line 796's row reads "Bullets and explosions, MW2 and Minecraft, 80 ticks; a Minecraft blast still pushes".
- `docs/MINECRAFT-SURVIVAL.md` (group tnt, commit 2): the explosions item at line 33 gains: "Minecraft explosions (TNT, creepers, fireballs, beds, anchors, crystals) push every player they reach away from the centre like vanilla, even when the hit does no damage; a flying creative player is not pushed."
- `docs/MINECRAFT-SURVIVAL.md` (group tnt, commit 3): line 374 drops ", and TNT lit by a player with `pvp` off (the vendored level's primed TNT keeps no igniter, so its blast hurts players anyway)". The pvp paragraph gains: "Primed TNT remembers the player who lit it with flint and steel or a fire charge, and TNT a blast primes keeps that player; redstone and fire prime it for nobody. With `pvp` off a blast caused by a player (their TNT, an end crystal they popped, a ghast fireball they sent back) pushes every player but hurts none, the igniter included; mobs are still hurt and the player gets the credit for kills."
- `docs/MULTIPLAYER.md` (group tnt, commit 3): line 783 drops its last sentence ("TNT lit by a player still hurts other players with pvp off: ..."), and the leftover at line 1323 ("TNT lit by a player honouring `pvp`: ...") is removed. Section 20 gains: "With `pvp` off the host takes every explosion hit caused by a player (`PlayerHitKind::Explosion.by`) out of the routed hits and pushes the victim instead."
- `docs/round8/decisions.md:71` is answered: TNT lit by a player honours `pvp` (track D, group tnt).
- `docs/PROGRESS.md` (group tnt, commit 3): the open not-vanilla item "TNT lit by another player with pvp off" (line 10) is done by track D, group tnt.
- `docs/MINECRAFT-SURVIVAL.md` (group tnt, commit 4): the death lines paragraph gains: "A blast with nobody behind it (TNT nobody lit, an end crystal nobody popped) reads '<victim> blew up', even right after another player's or a mob's hit, as vanilla."
- `docs/MINECRAFT-SURVIVAL.md` (group tnt, commit 5): the death lines paragraph gains: "A blast a player caused (their TNT, a crystal they popped, a fireball they sent back) reads '<victim> was blown up by <player>' (with 'using [<name>]' for a custom-named held item), and counts as that player's hit for 'doomed to fall' and 'to escape' lines."
- `docs/MULTIPLAYER.md` (group tnt, commit 5): next to the `DamageCause::Player` note (line 1427) add: "`DamageCause::PlayerExplosion { client }` is a player's blast (`minecraft:player_explosion`), named the same way."
- `docs/PROGRESS.md` (group tnt, commit 1): line 57 ("TNT with pvp off (track D): the brief found that TNT hurts nobody today...") becomes: "TNT (track D, group tnt): TNT now hurts players and mobs everywhere, single player too; tell the owner."
- `docs/MINECRAFT-SURVIVAL.md` (group small-not-vanilla, commit 1): line 441's last sentence ("An egg refused by a spawner because `spawner_blocks_work` is off gives no chat message ...") is removed; line 120 gains: "An egg refused because `spawner_blocks_work` is off tells that player 'Spawner blocks are not enabled' in the chat, like vanilla."
- `docs/MINECRAFT-SURVIVAL.md` (group small-not-vanilla, commit 2): line 380's last sentence ("A player's enchanting seed is not saved ...") is removed; the player file paragraph gains: "Each player's enchanting seed is saved as vanilla's `XpSeed`, so a table offers the same enchantments after a restart; a player file from before round 9 keeps the seed the player's UUID gives."
- `docs/MULTIPLAYER.md` (group small-not-vanilla, commit 2): the leftover at line 1352 ("The enchanting seed is per player since E14 but not saved ...") is removed.
- `docs/round8/decisions.md:73` (enchanting seed) is answered by track D, group small-not-vanilla, commit 2.
- `docs/MINECRAFT-SURVIVAL.md` (group small-not-vanilla, commit 3): line 374 drops "A gun hit does not wake a sleeping player, the host included."; line 426 drops "; a gun hit does not wake anyone in bed (only Minecraft damage does)"; line 78's waking list reads "a hit that hurts (Minecraft damage or an MW2 gun, grenade or knife hit)".
- `docs/MULTIPLAYER.md` (group small-not-vanilla, commit 3): the leftover at line 1324 ("a gun hit waking a sleeping player ...") is removed.
- `docs/round8/decisions.md:72` (a gun hit doesn't wake a sleeping player) is answered by track D, group small-not-vanilla, commit 3.
- `docs/round8/decisions.md:29` (a ghast's or blaze's fireball doesn't make a hoglin retaliate) is answered by track D, group small-not-vanilla, commit 4: the large fireball's blast now notes the ghast, and the direct hits already noted the shooter. "Looting and Fire Aspect don't affect their drops" stays.
- `docs/MINECRAFT-SURVIVAL.md` (group small-not-vanilla, commit 4): the hoglin paragraph gains: "A hoglin or piglin hurt by a ghast's fireball blast or a blaze's fireball turns on the ghast or blaze."

## Vendored files

Patched by track D: 44 (`entities/src/movement.rs`, `entities/src/effects.rs`, `entities/src/lib.rs`, `entities/src/potion.rs`, `entities/src/stroll.rs`, `entities/src/world.rs`, `entities/src/world/villagers.rs`, `entities/src/zombie.rs`, `player/src/survival.rs`, `world/src/level/spawning.rs`, `world/src/natural_spawner.rs`, `world/src/natural_spawner/spawning.rs`, `entities/src/monster_ai.rs`, `entities/src/world/combat.rs`, `entities/src/world/hazards.rs`, `entities/src/world/pushing.rs`, `entities/src/world/riding.rs`, `entities/src/world/witches.rs`, `generator/src/feature/entities.rs`, `entities/src/controls.rs`, `entities/src/loot.rs`, `world/src/level/time.rs`, `entities/src/cow_ai.rs`, `entities/src/horse.rs`, `entities/src/world/shulker_bullets.rs`, `entities/src/bin/check_zombie_player_hits.rs`, `entities/src/world/merchants.rs`, `world/src/level_storage.rs`, `world/src/storage.rs`, `world/src/level/mod.rs`, `player/src/lib.rs`, `core/src/item.rs`, `core/src/loot.rs`, `entities/src/look.rs`, `entities/src/witch.rs`, `entities/src/villager_brain/behaviors.rs`, `entities/src/villager_brain/memory.rs`, `entities/src/villager_brain/mod.rs`, `world/src/level/piston.rs`, `world/src/level/redstone.rs`, `world/src/level/explosion.rs`, `world/src/level/entity.rs`, `entities/src/world/fireballs.rs`, `entities/src/world/hoglins.rs`). Added: 28 (`entities/src/drowned_ai.rs`; `entities/src/feline.rs`, `entities/src/feline_ai.rs` and `entities/src/world/felines.rs` from group cats; `entities/src/llama_ai.rs`, `entities/src/llama_spit.rs` and `entities/src/world/equines.rs` from group horse-family; `entities/src/world/leashes.rs`, `entities/src/wandering_trader.rs`, `entities/src/trader_ai.rs` and `entities/src/world/traders.rs` from group wandering-trader; `world/src/level/golem_build.rs`, `entities/src/snow_golem.rs`, `entities/src/snow_golem_ai.rs` and `entities/src/world/snow_golems.rs` from group built-golems; `entities/src/illager.rs`, `entities/src/illager_ai.rs` and `entities/src/world/illagers.rs` from group pillagers; `entities/src/evoker_fangs.rs`, `entities/src/world/evoker_fangs.rs`, `entities/src/vex.rs`, `entities/src/vex_ai.rs` and `entities/src/world/vexes.rs` from group evokers-vexes; `entities/src/ravager.rs`, `entities/src/ravager_ai.rs` and `entities/src/world/ravagers.rs` from group ravagers; `entities/src/raid.rs` and `entities/src/world/raiders.rs` from group raids). Group zombie-villagers added `entities/src/world/villagers.rs` and `world/src/level/spawning.rs` (commit 1) and `entities/src/effects.rs` and `entities/src/potion.rs` (commit 2) to the patched list. Group effects-omens added `player/src/survival.rs` (commit 1). Group riding added `entities/src/movement.rs` (commit 1). Group horse-family added `entities/src/cow_ai.rs`, `entities/src/horse.rs`, `entities/src/world/shulker_bullets.rs` and `entities/src/bin/check_zombie_player_hits.rs` (commit 1). Group cats added `entities/src/monster_ai.rs`, `entities/src/world/combat.rs`, `entities/src/world/hazards.rs`, `entities/src/world/pushing.rs`, `entities/src/world/riding.rs`, `entities/src/world/witches.rs` and `generator/src/feature/entities.rs` (commit 1), and `entities/src/controls.rs`, `entities/src/loot.rs` and `world/src/level/time.rs` (commit 2). Group wandering-trader added `entities/src/world/merchants.rs`, `world/src/level_storage.rs` and `world/src/storage.rs` (commit 2). Group built-golems added `world/src/level/mod.rs` (commit 1) and `player/src/lib.rs` (commit 2). Group loot-functions added `core/src/item.rs` and `core/src/loot.rs` (commit 1). Group patrols-vindicators touched only files already listed (commit 1: `player/src/lib.rs`, `world/src/natural_spawner/spawning.rs`, `world/src/level/spawning.rs`, `world/src/level/time.rs`, `entities/src/world.rs` and the added illager files; commit 2: `entities/src/monster_ai.rs`, `world/src/natural_spawner.rs`, `world/src/natural_spawner/spawning.rs`, `generator/src/feature/entities.rs` and the added illager files). Group evokers-vexes touched only files already listed besides its added ones (commit 1: `entities/src/lib.rs`, `entities/src/loot.rs`, `entities/src/monster_ai.rs`, `entities/src/world.rs`, `entities/src/world/{equines,pushing,riding,shulker_bullets,witches}.rs`, `entities/src/bin/check_zombie_player_hits.rs`, `world/src/natural_spawner.rs`, `world/src/natural_spawner/spawning.rs`, `generator/src/feature/entities.rs` and the illager files; commit 2: `entities/src/lib.rs`, `entities/src/monster_ai.rs`, `entities/src/illager_ai.rs`, `entities/src/world.rs`, `entities/src/world/{combat,hazards,pushing,riding,villagers,witches,illagers,evoker_fangs}.rs`, `world/src/natural_spawner.rs` and `world/src/natural_spawner/spawning.rs`). Group ravagers added `entities/src/look.rs` (commit 1) and otherwise touched files already listed (commit 1: `entities/src/lib.rs`, `entities/src/monster_ai.rs`, `entities/src/illager_ai.rs`, `entities/src/world.rs`, `entities/src/world/{combat,hazards,pushing,riding,villagers,witches}.rs`, `player/src/lib.rs`, `world/src/natural_spawner.rs` and `world/src/natural_spawner/spawning.rs`). Group raids added `entities/src/witch.rs` (commit 1) and otherwise touched files already listed (commit 1: `entities/src/lib.rs`, `entities/src/monster_ai.rs`, `entities/src/illager_ai.rs`, `entities/src/world.rs`, `entities/src/world/{illagers,ravagers,witches}.rs`, `world/src/level/spawning.rs`, `world/src/level/time.rs`, `world/src/natural_spawner/spawning.rs` and `world/src/storage.rs`; commit 2: `entities/src/monster_ai.rs`, `entities/src/witch.rs`, `entities/src/world/witches.rs` and the added `entities/src/world/raiders.rs`). Group raid-villagers added `entities/src/villager_brain/{behaviors,memory,mod}.rs`, `world/src/level/piston.rs` and `world/src/level/redstone.rs` (commit 1) and otherwise touched files already listed (commit 1: `entities/src/effects.rs`, `entities/src/potion.rs`, `entities/src/loot.rs`, `entities/src/world.rs`, `entities/src/world/villagers.rs`, `player/src/lib.rs`, `world/src/level/mod.rs` and the added `entities/src/raid.rs` and `entities/src/world/raiders.rs`).
Group pvp-melee touched only `entities/src/world/combat.rs`, already listed (commit 1: `player_attack_with` and `player_sweep`).

Group strider-freeze touched only `entities/src/world.rs` and `entities/src/world/riding.rs`, already listed (`tick_selected`, `tick_frozen_with_players_where`, `player_carriers`).

Group tnt added `world/src/level/explosion.rs` (commit 1: `tick_tnt` defers its blast; commit 3: the owner), `world/src/level/entity.rs` and `entities/src/world/fireballs.rs` (commit 3: `TntData.owner`, the fireball's `by`) to the patched list, and touched `world/src/level/mod.rs` and `entities/src/world.rs`, already listed (commit 1: `defer_tnt_blasts`, `tnt_blasts`; commit 3: `PlayerHitKind::Explosion.by`).

Group small-not-vanilla added `entities/src/world/hoglins.rs` (commit 4: a test) to the patched list and touched `entities/src/world/fireballs.rs`, already listed (commit 4: the hurt-by note in `fireball_explosion`).

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
- Pillager leftovers: witches answering a pillager's hold-ground call, witches and ravagers as patrol companions (vanilla's `LongDistancePatrolGoal` counts every `PatrollingMonster`), and picking up items (`Mob.pickUpItem` equipment and the raid banner). (The patrol goal, the patrolling despawn rule and the captain's banner and bottle landed with group `patrols-vindicators`.)
- Vindicators: `VindicatorBreakDoorGoal` and `RaiderOpenDoorGoal` (raids only): the fork has no `DoorInteractGoal`, `BreakDoorGoal` or `OpenDoorGoal` for any mob yet (zombies on Hard do not break doors either), so both should be built once for zombies and vindicators, with the door crack progress and level events 1019, 1021 and 2001 for clients; naming a vindicator Johnny with a name tag (the fork has no name tags), and enchanted spawn equipment (not ported for any mob).
- Banner patterns are not drawn (placed banners, held banners and the captain's ominous banner show their base colour only); skulls worn by illagers are not drawn (`CustomHeadLayer`'s skull branch).
- The piglin's crossbow shot draws a pitch (`1 / (0.8..1.2)`) from its random; vanilla's single crossbow arrow plays at pitch 1 with no draw (`CrossbowItem.getShotPitch` index 0). The pillager does it the vanilla way; the piglin (round 8, `world/piglins.rs`) is unchanged.
- The crossbow's loading start and middle sounds play in the shooter's category; vanilla plays them as `PLAYERS` sounds.
- A friend never loads `ui.freeze_outline` (`minecraft_remote.rs`, next to the portal frames), so a friend in powder snow sees no frost outline; found in group friend-portal-music, outside its two commits.
- Vexes: flight through water and lava (vanilla's `travelInFluid` drag) and drowning; the follow range bonus roll of a summoned vex; a summoned vex joining its evoker's team.
- Evokers: the fangs' knockback (`LivingEntity.hurtServer` pushes 0.4 away from the fangs; the fork's indirect magic pushes nothing), a summoned vex joining its evoker's scoreboard team (the fork has no teams) (the celebrate sound landed with group `raids`).
- A mob's held crossbow is drawn with its plain item model: a loaded crossbow shows no arrow and a charging one no pull (`mesh::HeldItem` carries no components); pillagers in spawner cages are drawn without their crossbow (the cage draws mob models only).
- Raids: `ObtainRaidLeaderBannerGoal` (a raider picks up a dropped ominous banner and becomes its wave's captain; illagers pick up nothing yet); the raid statistics and advancements (`raid_trigger`, `raid_win`, the `voluntary_exile` and `hero_of_the_village` criteria); the raid omen particles around the player; a celebrating raider skipping its jump while riding. (Villagers hiding, ringing the bell, raiders glowing, celebrating and gifts landed with group `raid-villagers`.)
- Bells: the bell's swing (`BellRenderer` from the block event's direction), the client's `ENTITY_EFFECT` particles from a resonating bell to each raider, arrows and other projectiles ringing it (`onProjectileHit`), wind charges ringing it (`onExplosionHit` with `canTriggerBlocks`; the fork's wind charges trigger no blocks yet), the `bell_ring` statistic and the `BLOCK_CHANGE` game event (group `vibrations`).
- Villagers in raids: the sweat particles of entity event 42 (the client draws no entity-event particles yet).
- Fireworks: a rocket exploding on an entity in its path (`onHitEntity`), knockback for mobs hurt by a burst, the other explosion shapes (small and large balls, stars, creepers), trails and twinkles, saving rockets in flight, players launching rockets from the ground and dispensers and crossbows firing them.
- Thrown things and illagers: `thrown::targets` and `thrown::hurt_by` (players' tridents, snowballs, eggs and splash potions) list no pillagers, vindicators, evokers, vexes, ravagers, hoglins, piglins, striders, ghasts or blazes, so those cannot be struck by them; found while building the firework blast, which uses every tracked mob instead.
- Ravagers: a raid rider steering its ravager (the fork's ravager walks to the village itself; vanilla moves it by the rider's navigation); patrol companions beyond illagers (vanilla's `LongDistancePatrolGoal` counts every `PatrollingMonster`); a non-raider rider steering it (vanilla leaves such a ravager to its rider; the fork holds it still); the attack knockback landing on a fully blocked hit (the fork's melee `throw` lands on shielded hits); armor stands (excluded from the roar without `mob_griefing`) are not mobs in the fork; `checkSpawnObstruction` (no liquid in its box) is not asked by eggs and summons in the fork either; a vindicator rider's melee reach does not grow by its ravager's box (`Mob.getAttackBoundingBox` with a vehicle).
- The host's own first-person held items still show with the UI off (`ui_draw 0`); vanilla's hidden GUI hides the hands, as the friend's now do (group friend-hand, commit 1).
- Skating leftovers (group friend-skating): done in group skater-poses (the pose and board for everyone, the host's held items cleared while it skates).
- Skater-pose leftovers (group skater-poses): other players' boards keep their trucks and wheels rigid on the deck (only `SKATEBOARD_ROOT` is sent); the poses ride the state frames, which the host sends with each server output of the receiver's level, so a gap of more than 1 s between frames (a host stall, a level without an entity world) shows other skaters as standing soldiers until the next frame.
- Melee leftovers (group pvp-melee): the attacker's own slowdown after a sprint knockback hit (`causeExtraKnockback` multiplies the attacker's motion by 0.6 and stops the sprint), for mob and player hits alike; `EnchantmentHelper.doPostAttackEffects` (Fire Aspect and other post-attack enchantments) on players; a sweep round a mob reaches players before the entity world says whether the mob was hurt.
- Strider-freeze leftovers (group strider-freeze): under `/tick freeze` a player on foot does not push a ridden strider or horse (vanilla's `pushEntities` still pushes a ticking carrier), and queued respawn-anchor and crystal blasts wait for the next normal tick (as before the group).
- TNT leftovers (group tnt): a mob as TNT owner (a creeper's blast priming TNT; vanilla's `getIndirectSourceEntity` keeps any living source, so a chain would read "blown up by Creeper"); an end crystal destroyed by a player's TNT blast does not name that player for its own blast (`tick_end` passes no owner to `crystals.blast`); the direct hit of a fireball a player sent back (`PlayerHitKind::Fireball`) still hurts players with pvp off; Blast Protection's `explosion_knockback_resistance` on explosion pushes.
