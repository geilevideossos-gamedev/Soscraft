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

## State doc changes

- `docs/MINECRAFT-SURVIVAL.md`: drowned are now saved, loaded, hatched from their egg and seen by friends; a zombie that drowns keeps its equipment and name as a drowned. Drowned spawn naturally in rivers (one in 15 rolls), deep oceans (one in 40, below sea level minus 5) and dripstone cave water in the dark, with a trident (6.25 %) or a fishing rod (3.75 %) and a nautilus shell in the offhand (3 %), which they always drop; a player's kill sometimes drops the worn trident. By day drowned on land walk back into the water; at night they swim up and come out onto the nearest beach, and they stroll in the water. A drowned with a trident raises it over its head and throws it every 2 seconds at its target within sight (8 damage, "was impaled by Drowned"); the thrown tridents cannot be picked up and despawn after a minute. Friends see the raised arm and the tridents. Zombie nautilus jockeys and the trident roll for ocean ruin drowned are missing.
- `docs/MINECRAFT-SURVIVAL.md`: right-clicking an adult villager with a job opens its vanilla trading screen for the host: the offer buttons with their payments, arrows and results, the struck-through old price on a changed price, a crossed arrow on a sold-out offer, the scroller beyond 7 offers (wheel and drag), the experience bar with the coming experience, and the title "<Profession or name> - <Level>". Picking an offer fills the payments from the inventory, the result trades once or, shift-clicked, as often as the items last, and a shift-click moves stacks between inventory and hotbar. Trades give the villager experience, level it up with new offers and Regeneration, and drop experience orbs (3 to 6, 5 more on a level-up). Prices follow reputation and Hero of the Village. The screen closes when the villager is out of reach (8 blocks), dies or sleeps, and the payments return to the inventory (or drop when it is full); dying or leaving the level closes it the same way. Babies shake their heads. Friends cannot trade yet.
- `docs/MINECRAFT-SURVIVAL.md`: friends trade with villagers like the host: the same trading screen, prices, experience, level-ups and closing rules. A villager trades with one player at a time. This replaces "Friends cannot trade yet" above.
- `docs/MULTIPLAYER.md`:
  - `minecraft_net::PROTOCOL` is 4 (track D, round 9: `MenuKind::Merchant`, `MenuExtras::Merchant` and `NetOffer`). The code block at line 162 and the note at line 1415 read 4.
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

## Vendored files

Patched by track D: 21 (`entities/src/effects.rs`, `entities/src/lib.rs`, `entities/src/potion.rs`, `entities/src/stroll.rs`, `entities/src/world.rs`, `entities/src/world/villagers.rs`, `entities/src/zombie.rs`, `player/src/survival.rs`, `world/src/level/spawning.rs`, `world/src/natural_spawner.rs`, `world/src/natural_spawner/spawning.rs`, `entities/src/monster_ai.rs`, `entities/src/world/combat.rs`, `entities/src/world/hazards.rs`, `entities/src/world/pushing.rs`, `entities/src/world/riding.rs`, `entities/src/world/witches.rs`, `generator/src/feature/entities.rs`, `entities/src/controls.rs`, `entities/src/loot.rs`, `world/src/level/time.rs`). Added: 4 (`entities/src/drowned_ai.rs`; `entities/src/feline.rs`, `entities/src/feline_ai.rs` and `entities/src/world/felines.rs` from group cats). Group zombie-villagers added `entities/src/world/villagers.rs` and `world/src/level/spawning.rs` (commit 1) and `entities/src/effects.rs` and `entities/src/potion.rs` (commit 2) to the patched list. Group effects-omens added `player/src/survival.rs` (commit 1). Group cats added `entities/src/monster_ai.rs`, `entities/src/world/combat.rs`, `entities/src/world/hazards.rs`, `entities/src/world/pushing.rs`, `entities/src/world/riding.rs`, `entities/src/world/witches.rs` and `generator/src/feature/entities.rs` (commit 1), and `entities/src/controls.rs`, `entities/src/loot.rs` and `world/src/level/time.rs` (commit 2).

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
