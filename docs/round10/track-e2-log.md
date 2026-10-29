# Track E2 log: gameplay and loot

Branch `track-e2`, worktree `C:\Dev\mc-wt\track-e2`, target dir `D:\iw4l-wt\track-e2`. Brief: `docs/round10/track-e.md` (overview, then the E2 sections).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Protocol

No commit of this track has changed a `ToClient` or `ToServer` message or the codec yet.

## Group ti1-targets-and-hurt

### 1. Minecraft thrown things hurt every mob family

After ed23b000.

- **Vanilla (26.3):** a thrown hit is the victim's own `hurtServer` with the projectile's damage source: its armour and damage cooldown, `setLastHurtByMob` (the anger), `resolvePlayerResponsibleForDamage` (the player's kill credit) and, on a full hit, `knockback(0.4F, -motion.x, -motion.z)` along the projectile's flight, scaled by the victim's knockback resistance (`Projectile.calculateHorizontalHurtKnockbackDirection`). An enderman dodges every `#is_projectile` source, and a closed shulker refuses an `AbstractArrow` (tridents too).
- **Changed:**
  - Vendored `entities/src/world/combat.rs`: the body of the private `hurt_by_player` (every store's hurt and blame, the kill credit, the villagers' gossip and the hoglin and piglin anger) is now `hurt_for_player(id, amount, player, kind, source)`. The damage type is passed through instead of the fixed `minecraft:player_attack`, so villagers, wolves, cats, wandering traders and the wither see the real type (the wither's armour turns tridents and wind charges away by it). Melee (`hurt_by_player`) wraps it with the knockback away from the player, as before.
  - The same file: new public `ProjectileHit { amount, kind, player, velocity }` and `EntityWorld::player_projectile_hurt(id, ProjectileHit) -> Option<DamageResult>`: the enderman's dodge, the closed shulker refusing a trident, then `hurt_for_player` with the projectile source and, on a full hit, `knock_back(id, 0.4F, -velocity.x, -velocity.z)` (hoglins, zoglins and ravagers take less; iron golems and shulkers none). `None` when no such mob lives. `ProjectileHit` is re-exported from `world.rs` next to `PlayerAttack`.
  - `minecraft_terrain/src/thrown.rs`: `hurt_by` is one call to `player_projectile_hurt`. Its own store chain and its private knockback are gone, and so is the `random` argument of `hurt` and `hurt_by`.
  - `minecraft_terrain/src/server.rs`: `tick_tridents` no longer draws a seed from `sound_random` for the hit, and `tick_thrown` no longer draws one from the thrown item's own random for a snowball, egg or pearl hit or for a wind charge's hit.
  - What a round 7 mob now gets from a thrown hit that it did not before: a slime's armour, a llama's or horse's memory of the thrower (a llama spits back), a zombie or skeleton without the goal framework turning on the thrower, and the vanilla `0.4F` knockback from the mob's own random.
- **Tests:**
  - Vendored `world::combat::tests` (new): `a_thrown_hit_lands_on_pillagers_vexes_ravagers_and_the_nether_mobs` (a snowball lands on all three illagers, a vex, a ravager, a ghast, a blaze, a strider, a hoglin, a zoglin, a piglin and a brute without harm, and a trident hurts each; an unknown id is `None`), `a_projectile_knocks_a_hoglin_and_a_ravager_back_less` (0.4, 0.16 and 0.1 along the flight), `a_trident_angers_a_hoglin_and_a_piglin_at_the_thrower`, `a_closed_shulker_turns_a_trident_away` (and an open one is hurt, remembers the player and does not move), `an_enderman_dodges_every_thrown_thing`.
  - The existing `thrown::tests` of the wither, the shulker and the trident kill pass unchanged apart from the dropped `random` argument; the snow golem, wind charge, trident and frame tests in `server::tests` pass unchanged.
- **Decisions:**
  - The seed draws for the old knockback are dropped, not kept: vanilla draws a tiny-direction knockback from the victim's own random, which `knock_back` already uses, and no test pinned a later draw.
  - A thrown hit keeps the existing `credit(.., "minecraft:player", true)`, so a mob killed by a thrown thing is still the thrower's kill with its experience and player-only loot.
  - The farm animals keep the `Projectile` source of the old thrown chain (the fixtures measured that it does not make them panic).
- **Test steps (survival, Normal, by day, flat ground):**
  1. Use a llama spawn egg 6 blocks away and throw a snowball at it: it is knocked back, and it spits at you (**ti1-1_llama_spits**).
  2. Use a zombie spawn egg and throw the loyalty trident at it: it flashes red, is knocked back along the throw and comes at you; the trident flies back (**ti1-2_zombie_trident**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:trident+loyalty=3,minecraft:snowball*16,minecraft:llama_spawn_egg*2,minecraft:zombie_spawn_egg*2,minecraft:cobblestone*64
```

### 2. Minecraft thrown things hit pillagers, ravagers, withers and the Nether mobs

After 9b623593.

- **Vanilla (26.3):** a thrown item, trident or potion can hit every living entity that `canBeHitByProjectile`, so every mob family the fork has.
- **Changed:**
  - Vendored `world.rs`: new public `EntityWorld::hit_boxes() -> Vec<PotionTarget>`, every living mob's box (`owner: false`). It walks each store once, like `mob_candidates`. The shulker uses its `bounding_box()`, a dead mob and an exploded creeper are left out. New families (D0) add one loop here.
  - `arrow_targets` is now `hit_boxes` without the shooter, with the players behind it. Its own walk over `self.order` with one `match` arm per store is gone.
  - `minecraft_terrain/src/thrown.rs`: `targets(world)` is one call to `hit_boxes`. Its store-by-store list and the private `target` helper are gone. `thrown.rs` has no per-mob code left.
  - Every user of `thrown::targets` now sees pillagers, vindicators, evokers, vexes, ravagers, withers, ghasts, blazes, striders, hoglins, zoglins, piglins and brutes: snowballs, eggs, pearls, tridents and splash potions in flight, lingering clouds, water bottles, the push of a wind burst, the dragon's wings and fireballs, and the effect swirls over mobs.
- **Tests:**
  - Vendored `world::tests::hit_boxes_cover_every_live_mob`: one mob of each of the 28 stores and a thrown potion; the boxes' ids are `mob_ids()` without the potion; the shulker's box is its `bounding_box`; a dead zombie and an exploded creeper drop out.
  - `server::tests::every_hatching_mob_is_a_thrown_target_and_takes_a_trident`: every `HATCHING_EGGS` mob but the dragon is hatched; each new id (riders included) is in `thrown::targets` and `thrown::hurt_by` with the trident's 8 lands, except on the enderman (dodges) and the closed shulker. A new family added to `HATCHING_EGGS` is covered by it.
  - `server::tests::a_player_snowball_stops_on_a_pillager_and_knocks_it_back`: a real flight through `tick_thrown`; the snowball breaks on the pillager's front, does no harm, pushes it along the throw and turns it on the thrower.
  - `wind::tests::a_burst_pushes_a_pillager_and_a_ravager`.
  - The arrow, potion, cloud, wind, dragon and frame tests pass unchanged.
- **Decisions:**
  - Arrows now take their targets in store order, not spawn order. This only matters when two boxes are entered at exactly the same point, and vanilla's own order there is its entity sections' order, which neither list copies. An exploded creeper is no longer an arrow target (vanilla discards it).
  - Splash, lingering and harming potions now reach ghasts and blazes but still do nothing to them: their effects stores are missing from the potion sites until group `ti2-potions`.
- **Note:** `sections::tests::background_full_update_matches_the_synchronous_one` failed once in the full `minecraft_terrain` run while other tracks were building (it polls a background thread for 1000 ms) and passed when run again alone. It does not touch mobs.
- **Test steps (survival, Normal, by day, flat ground; pen each mob in with cobblestone):**
  1. Use a pillager spawn egg 8 blocks away and throw a snowball at it. The snowball breaks on the pillager (white crumbs), pushes it back without hurting it, and the pillager comes at you (**ti1-3_pillager_snowball**).
  2. Use the vindicator and evoker eggs and throw eggs at both. Each egg breaks on them and pushes them back (sometimes a chick hatches). When the evoker summons vexes, throw a snowball at a vex in the air: it stops on the vex (**ti1-4_vex_snowball**).
  3. Use the ravager egg and throw the loyalty trident at it. The ravager flashes red, loses about 4 hearts and hardly moves back. The trident flies back to you (**ti1-5_ravager_trident**).
  4. Use the hoglin egg and throw the loyalty trident: the hoglin flashes red and charges you. Use the piglin egg and throw a snowball at it (wear no gold): the piglin gets angry and attacks (**ti1-6_hoglin_piglin**).
  5. Use the three other pillager eggs close together and throw a wind charge into the group: they are flung away (**ti1-7_wind_pillagers**).
  6. Use the strider egg and the ghast egg (open sky) and throw a snowball at each: it stops on them instead of flying through (**ti1-8_strider_ghast**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:trident+loyalty=3,minecraft:snowball*16,minecraft:egg*16,minecraft:wind_charge*16,minecraft:pillager_spawn_egg*4,minecraft:vindicator_spawn_egg,minecraft:evoker_spawn_egg,minecraft:ravager_spawn_egg,minecraft:hoglin_spawn_egg,minecraft:piglin_spawn_egg,minecraft:strider_spawn_egg,minecraft:ghast_spawn_egg,minecraft:cobblestone*64,minecraft:golden_apple*16
```

### 3. Minecraft snowballs hurt blazes

After d1ef983f.

- **Vanilla (26.3):** `Snowball.onHitEntity` hurts the entity with `thrown(this, owner)` for 3 when it is a `Blaze` and 0 otherwise. Eggs and ender pearls always deal 0. The same rule holds for a snow golem's snowball.
- **Changed:**
  - Vendored `world/snow_golems.rs`: new public `EntityWorld::snowball_damage(victim) -> f32` (3, `snow_golem::SNOWBALL_BLAZE_DAMAGE`, for a blaze; 0 for anything else). `snowball_hits_mob` uses it instead of its own check.
  - `minecraft_terrain/src/thrown.rs`: `hurt` takes the `Throwable` and passes `snowball_damage` for a snowball, 0 for an egg or a pearl.
  - `minecraft_terrain/src/server.rs` `tick_thrown`: a player's struck mob carries the throwable's kind to `thrown::hurt`.
- **Tests:**
  - `thrown::tests::a_snowball_hurts_a_blaze_by_three_and_nothing_else`: a snowball takes a blaze from 20 to 17; an egg and a pearl leave a blaze at 20; a snowball leaves a ghast and a pig unhurt, and every one of these hits lands (knockback and anger).
  - `server::tests::a_snow_golem_s_snowball_hits_a_blaze`: a snow golem's snowball now reaches a blaze (it was not a thrown target before group ti1) and takes 3 off it. Round 9's step (`docs/round9/track-d-log.md`) works now.
- **Test steps (survival, Normal, by day, flat ground, outside a warm biome):**
  1. Build a cobblestone pen and use a blaze spawn egg inside it. Throw snowballs at it one by one, about a second apart. Every hit flashes the blaze red, and about 7 hits kill it (**ti1-9_blaze_snowballs**).
  2. Use the snow golem egg 6 blocks from a new blaze (the second blaze egg). The golem's snowballs hit the blaze and hurt it (**ti1-10_golem_blaze**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:snowball*32,minecraft:blaze_spawn_egg*2,minecraft:snow_golem_spawn_egg,minecraft:cobblestone*64,minecraft:potion@fire_resistance,minecraft:golden_apple*16
```

### 4. Minecraft wind charges don't anger the mobs vanilla keeps calm

After a1313909.

- **Vanilla (26.3):** `LivingEntity.hurtServer` skips `setLastHurtByMob` when the source is `wind_charge` and the victim is in `#minecraft:no_anger_from_wind_charge`: breeze, skeleton, bogged, stray, zombie, husk, spider, cave_spider and slime. The damage, the knockback and the player's kill credit (`lastHurtByPlayer`) stay. Drowned, zombie villagers, wither skeletons, parched, magma cubes and every other mob still turn on the thrower.
- **Changed:**
  - Vendored `world/combat.rs`: new public `NO_ANGER_FROM_WIND_CHARGE` with the nine ids of the 26.3 tag, re-exported from `world.rs` next to `ProjectileHit`, so track D's breeze wind charges can use the same check.
  - `player_projectile_hurt` works out `angers` (false for a wind charge on a listed type, by `entity_type`) and passes it to `hurt_for_player`. Melee passes `true`.
  - `hurt_for_player` guards the anger of the four stores that hold listed types (zombies, skeletons, spiders, slimes) with it. `credit` still runs, so the kill stays the player's.
- **Tests:** vendored `world::combat::tests::a_wind_charge_leaves_a_zombie_calm_but_angers_a_pillager`: a wind charge hurts and pushes a zombie, a spider and a slime without making them remember the player; a pillager and a drowned turn on the player; a zombie it kills is the player's kill; a trident after it does anger the spider.
- **Decision:** a mob store added later that holds a listed type (track D's breezes) must guard its blame in `hurt_for_player` with `angers` too.
- **Test steps (survival, Normal, by day, open flat ground; a spider is calm in daylight until it is attacked):**
  1. Use a spider spawn egg 6 blocks away. Throw a wind charge at it: it is flung back and flashes red but keeps wandering and does not come at you. Then throw a snowball at it: now it comes at you (**ti1-11_spider_wind**).
  2. Put on the golden helmet and use a piglin spawn egg. Throw a wind charge at it: piglins are not on the calm list, so it gets angry and attacks you although you wear gold (**ti1-12_piglin_wind**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:wind_charge*16,minecraft:snowball*16,minecraft:spider_spawn_egg*2,minecraft:piglin_spawn_egg*2,minecraft:golden_helmet,minecraft:cobblestone*64,minecraft:golden_apple*16
```

## Group ti2-potions

Track D0 is not on `minecraft-survival` yet, so this group adds its own `ghasts` and `blazes` arms at the four potion sites of vendored `world/witches.rs`. D0's generic arm replaces them when it merges.

### 1. Minecraft potions work on ghasts and blazes

After 83a12ec4.

- **Vanilla (26.3):** a ghast and a blaze are `LivingEntity`s like any other: a splash or lingering potion gives them its effects, healing heals them and harming hurts them by `indirect_magic` from the thrower, who becomes their `lastHurtByMob`. `HoglinAi.wasHurtBy` and `PiglinAi.wasHurtBy` run on every hurt whose source has a living entity, so a harming potion, and an evoker's fangs, anger a hoglin or a piglin at the thrower.
- **Changed:** vendored `entities/src/world/witches.rs`:
  - `mob_effects_mut` and `mob_effects` have `ghasts` and `blazes` arms (neither is undead or a spider), next to the chickens.
  - `heal_mob` heals them up to `ghast::MAX_HEALTH` (10) and `blaze::MAX_HEALTH` (20).
  - `hurt_mob_by_magic` hurts them and sets their `ai.state.hurt_by` from the owner, like the vex arm.
  - `hurt_mob_by_magic` calls `note_hoglin_hurt_by_owner` next to `villager_hurt_by` on every applied hit with an owner, so a hoglin or piglin hurt by a harming splash, a harming cloud or an evoker's fangs turns on whoever threw or summoned it.
  - What reaches ghasts and blazes now: a player's splash and lingering potions (group ti1 made them targets), a witch's potions, an ownerless wither skull and an evoker's fangs.
- **Tests:** vendored `world::witches::tests` (new):
  - `a_poison_splash_poisons_a_blaze_and_a_ghast`: both take poison and are neither undead nor spiders; harming takes 6 off each and makes them remember the player; healing gives 4 back.
  - `a_harming_splash_angers_a_hoglin_at_the_thrower`: a harming splash hurts a hoglin and a piglin and both remember the player; a second hit inside the damage cooldown adds nothing; a pillager's potion after the cooldown adds the pillager; a potion with no thrower angers nobody.
- **Test steps (survival, Normal, by day, flat ground; pen each mob in with cobblestone):**
  1. Use a blaze spawn egg and throw the poison splash at it. Green swirls rise from it and it flashes red every second or so until it has 1 heart left; poison never kills (**ti2-1_blaze_poison**).
  2. Use the ghast egg (open sky) and throw a harming splash under it. It flashes red and starts shooting fireballs at you (**ti2-2_ghast_harming**).
  3. Use the hoglin egg and throw the second harming splash at it. It flashes red and charges you (**ti2-3_hoglin_harming**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:splash_potion@poison,minecraft:splash_potion@harming*2,minecraft:blaze_spawn_egg,minecraft:ghast_spawn_egg,minecraft:hoglin_spawn_egg,minecraft:cobblestone*64,minecraft:potion@fire_resistance,minecraft:golden_apple*16
```

### 2. Minecraft harming heals every undead mob

After a8700f4f.

- **Vanilla (26.3):** `HealOrHarmMobEffect` turns round for every mob in `#minecraft:inverted_healing_and_harm`, which is `#minecraft:undead`: `#skeletons`, `#zombies` (with zoglin, zombie horse, camel husk and zombie nautilus), the wither and the phantom. The same tag is `#ignores_poison_and_regen`.
- **Changed:** vendored `entities/src/world/witches.rs`:
  - `splash_instant` decides "undead" by `entity_type(id)` and `wither::is_undead` (the full 26.3 tag), not by the zombie and skeleton stores. Zoglins and withers are now healed by harming and hurt by healing. This covers splash potions, lingering clouds and witches' potions, which all go through `splash_instant`.
  - `mob_effects_mut` reads the same flag once before its borrows and uses it for the `cows` store, so skeleton and zombie horses (track M) will ignore poison and regeneration without another change. The other stores keep their own flags.
- **Tests:** vendored `world::witches::tests::harming_heals_a_zoglin_and_a_wither_and_healing_hurts_them`: a zoglin, a wither and a drowned refuse poison, gain 4 from a harming splash and lose 6 to a healing splash; a hoglin still loses 6 to harming; a horse is not undead.
- **Decision:** the fork has no skeleton or zombie horses yet, so the brief's skeleton-horse case can't be tested. The `cows` flag follows `entity_type`, which returns the horse kind's id, so the test that adds those horses only needs to check the flag.
- **Test steps (survival, Normal, by day, flat ground; pen the zoglin in with cobblestone):**
  1. Use the zoglin spawn egg and throw a harming splash at it. It shows no red flash and loses no health, because harming heals it (**ti2-4_zoglin_harming**).
  2. Throw a healing splash at the same zoglin. Now it flashes red and charges you (**ti2-5_zoglin_healing**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:splash_potion@harming,minecraft:splash_potion@healing,minecraft:zoglin_spawn_egg,minecraft:cobblestone*64,minecraft:golden_apple*16
```

### 3. Minecraft water bottles hurt blazes, striders and snow golems

After 85acbc95.

- **Vanilla (26.3, `AbstractThrownPotion.affectEntitiesAround`):** a water splash (`#hurts_water_sensitive_entities`) looks at every living entity in its box grown by (4, 2, 4) whose feet are less than 4 blocks from the break, and hurts each one with `isSensitiveToWater()` by 1.0 `indirect_magic` from the potion and its thrower, before it puts out fires. Only `Blaze`, `EnderMan`, `SnowGolem` and `Strider` return true. `indirect_magic` is not in `#no_knockback` (plain `magic` is), and a potion's `calculateHorizontalHurtKnockbackDirection` is the victim's position less the potion's, so the hit pushes the mob away from the break, as harming already does.
- **Changed:**
  - Vendored `entities/src/world/witches.rs`: the harm half of `splash_instant` (hurt, the thrower's credit, the push away from the break) is now the private `potion_hurt(id, amount, at, owner)`, unchanged. New public `EntityWorld::splash_water_hurt(id, at, owner) -> Option<DamageResult>` runs it with 1.0 for a blaze, a snow golem or a strider (the private `HURT_BY_SPLASHED_WATER` list, checked by `entity_type`) and returns `None` for any other mob.
  - `minecraft_terrain/src/splash.rs` `wet`: a mob that is not an enderman goes to `splash_water_hurt` with the player as owner. The blaze and the snow golem remember the thrower (the `hurt_mob_by_magic` arms), and the strider takes the hit as it takes harming.
- **Tests:** `splash::tests::a_water_bottle_hurts_a_blaze_a_strider_and_a_snow_golem_by_one`: a water splash takes a blaze and a strider from 20 to 19 and a snow golem from 4 to 3, pushes the blaze and the strider away from the break and makes the blaze remember the thrower; a pig next to them and a blaze 6.5 blocks away are not hurt; `splash_water_hurt` on the pig is `None`. The existing enderman water test in `server::tests` passes unchanged.
- **Decision:** endermen keep their own branch in `wet` (hurt by 1, remember the thrower, then teleport away). They are left out of `HURT_BY_SPLASHED_WATER` because `hurt_mob_by_magic` makes an enderman dodge a potion without taking its damage, which is right for harming but not for water. The list stays in the vendored entities crate, so `splash.rs` holds no per-mob list.
- **Test steps (survival, Normal, by day, flat ground, outside a warm biome; pen the mobs in with cobblestone):**
  1. Use a blaze spawn egg and a strider spawn egg next to each other, and a snow golem spawn egg next to them. Throw a water splash between them. All three flash red once and are pushed a little away from the splash; the blaze then comes at you (**ti2-6_water_bottle**).
  2. Throw the second water splash next to a pig 3 blocks away: the pig does not flash (**ti2-7_water_pig**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:splash_potion@water*2,minecraft:blaze_spawn_egg,minecraft:strider_spawn_egg,minecraft:snow_golem_spawn_egg,minecraft:pig_spawn_egg,minecraft:cobblestone*64,minecraft:potion@fire_resistance,minecraft:golden_apple*16
```

## Group ti3-mounts

### 1. Minecraft thrown things pass the mount their thrower rides

After f64ce4a1.

- **Vanilla (26.3):** `Projectile.canHitEntity` refuses, until the projectile has left its owner, every entity for which `owner.isPassengerOfSameVehicle(entity)` holds: the owner's root vehicle and everything riding it. `isOutsideOwnerCollisionRange` sets `leftOwner` once the projectile's box, swept along its motion and grown by a block, meets none of `owner.getRootVehicle().getSelfAndPassengers()`. So a snowball, egg, pearl, potion, wind charge or trident thrown from a saddle flies through the player's own mount.
- **Changed:**
  - `minecraft_terrain/src/thrown.rs`: new public `ride_group(world, player)`: the player's root vehicle (`vehicle_of(Rider::Player)`, then `root_vehicle`), every mob riding it (`with_mob_riders`) and every player riding any of them (as `PLAYER_TARGET + id`). Empty for a player on foot. New public `skip_own_mount(world, targets, player, bounds)` marks those targets `owner: true` and returns `bounds` grown to cover their boxes.
  - `minecraft_terrain/src/server.rs` `tick_thrown`: a player's throw (no `shooter`) passes its owner box and targets through `skip_own_mount`. `tick_tridents`: a player-owned trident's `TridentOwner.bounds` does the same. Mob throwers keep today's rule.
- **Tests:**
  - `server::tests::a_snowball_thrown_from_a_strider_passes_its_strider_but_hits_another`: a player on a saddled strider throws a snowball down at 72 degrees past the front of its own strider; it breaks on the front of a second strider standing next to it, pushes that one along the throw and leaves the ridden strider still. `ride_group` is the strider and its rider, and empty for a player on foot.
  - `server::tests::a_trident_thrown_from_a_strider_passes_its_strider_but_hits_another`: the same throw with a trident takes 8 off the second strider and nothing off the ridden one.
  - Both tests fail with `skip_own_mount` turned off (checked), because the flight crosses the top of the ridden strider first.
- **Decisions:**
  - The owner box grows to one box around the rider and the whole vehicle group, as the brief says, instead of vanilla's test of each box on its own. The rider sits on top of its mount, so the two only differ at the corners of the grown box, and the test grows it by a block anyway.
  - Other players on the same vehicle (a camel's second seat) and the mobs riding the mount are passed too, as `getSelfAndPassengers` includes them.
  - A dead or spectating thrower keeps today's rule (no owner box, so the throw counts as clear of its owner at once).
- **Test steps (survival, Normal, by day, flat ground; a strider walks on land and shivers but works):**
  1. Use a strider spawn egg, put the saddle on it and get on it. Build a cobblestone ring around it so it can't wander. Look straight down and throw a snowball: it flies through your strider and breaks on the ground at its feet (white crumbs under it). Your strider does not flash or get pushed (**ti3-1_snowball_down**).
  2. Use the second strider egg on the block right in front of your strider, inside the ring. Still riding, look down at the front of the second strider and throw a snowball: it breaks on the second strider, which is pushed away from you; yours stays put (**ti3-2_other_strider**).
  3. Look straight down and throw the loyalty trident: it passes your strider, sticks in the ground under it and flies back to you. Your strider does not flash red (**ti3-3_trident_down**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*2,minecraft:saddle,minecraft:snowball*16,minecraft:trident+loyalty=3,minecraft:cobblestone*64
```

## Group doors-zombies

### 1. Minecraft zombies break wooden doors on Hard like vanilla

After 9647acfb.

- **Vanilla (26.3, checked with javap):**
  - `Zombie.setCanBreakDoors(true)` turns on `navigation.setCanOpenDoors(true)` and adds `BreakDoorGoal(this, difficulty == HARD)` at priority 1, after the other goals. No zombie kind overrides it, so zombies, husks, zombie villagers, drowned and zombified piglins on ground navigation can all break doors.
  - `DoorInteractGoal.canUse`: a horizontal collision and a path not done; for each node up to `nextNodeIndex + 2`, the block above it is the door when the mob's feet are within 1.5 of the node's corner and it is a wooden door (`DoorBlock.isWoodenDoor`: any door but iron, so copper doors too); else the block above its feet.
  - `BreakDoorGoal`: `canUse` adds `mob_griefing`, the difficulty and a closed door; `canContinueToUse` is `breakTime <= 240`, the door still closed, the mob within 2 of the door's centre and the difficulty (not `mob_griefing`). Each tick: on the mob's `nextInt(20) == 0` level event 1019 and a swing when not swinging; `breakTime++`; the stage `(int)(breakTime / 240 * 10)` on a change (`destroyBlockProgress`, -1 in `stop`); at 240 `removeBlock(doorPos)`, 1021 and a 2001 that shows nothing (the block is air by then). `lastBreakProgress` is never reset.
  - The client plays 1019 `entity.zombie.attack_wooden_door` and 1021 `entity.zombie.break_wooden_door` at the block, volume 2.0, pitch `(r - r) * 0.2 + 1.0`.
- **Changed:**
  - Vendored `entities/src/door_goals.rs` (new, `pub mod door_goals` in `lib.rs`):
    - `DoorWork`: what a mob's door goals ask of the world this tick (`toggles`, `events`, `broken`), the crack it shows (`crack`) and its swing clock (`swing_until`). `take_requests` hands the requests over; `door_open` reads this tick's toggles before the world.
    - `is_wooden_door`, the `DoorInteract` core (`can_use`, `is_open`, `start`, `tick` with the passed test) and `BreakDoorGoal::zombie()` (Hard only, `mob_griefing` on start only, inert without `can_break_doors`, every tick, the 1019 knock, its own 6-tick swing clock adding to `melee.swings`, the crack stages, `broken` and 1021 at 240).
  - Vendored `monster_ai.rs`: `MonsterGoalContext.door`; `zombie_goals()` and `zombified_piglin_goals()` add `BreakDoorGoal` last at priority 1, and `"BreakDoorGoal"` closes `ZOMBIE_GOAL_NAMES` (8) and `ZOMBIFIED_PIGLIN_GOAL_NAMES` (7), so goal ids stay stable.
  - Vendored `world.rs`: a zombie with `can_break_doors` also gets `walk.can_open_doors` (`setCanOpenDoors(true)`), so its paths go through wooden and copper doors. After its AI step the store calls `apply_door_work`. New `level_events` queue with `take_level_events()`, next to `take_villager_infections`.
  - Vendored `world/villagers.rs`: `EntityWorld::apply_door_work(id, world, time)`: toggles go through `set_door_open` (its sound at the block's centre, the pitch from the level random, drawn only when the door changes), `broken` is `world.set_block(pos, None)` (the real level then drops the lower half with its particles and sound), events go to the queue, and every changed half recomputes nearby villagers' paths (`recompute_for_block`).
  - `minecraft_terrain/src/server.rs`: the queue is drained into `level.level_events` right after `infect_villagers`, so the host and friends get the events through the existing pipeline (64 blocks).
  - `minecraft_terrain/src/level_events.rs`: `ZOMBIE_ATTACK_WOODEN_DOOR` (1019) and `ZOMBIE_BREAK_WOODEN_DOOR` (1021) with their sounds, played like `ZOMBIE_INFECT`.
  - `docs/MINECRAFT-SURVIVAL.md`: door breaking under World simulation; drowned under the mob limitations.
- **Tests:**
  - Vendored `door_goals::tests` (new): `a_door_breaking_zombie_knocks_cracks_and_breaks_a_wooden_door_after_240_ticks_on_hard` (stages 0 to 9 in order, then none; every knock is 1019 at the upper half; swings rise but never above the knocks; `broken` and 1021 exactly at tick 240), `no_door_breaking_on_normal_without_mob_griefing_at_an_iron_door_or_without_can_break_doors` (and none without a collision; copper, spruce and waxed oxidized copper doors start), `switching_to_normal_stops_the_break_and_clears_the_crack`, `opening_the_door_stops_a_break` (a toggle pending in the same tick, then a door open in the world), `a_mob_walking_through_the_doorway_has_passed_it`.
  - Vendored `world::tests`: `a_zombie_that_can_break_doors_paths_through_a_wooden_door_and_one_that_cannot_goes_around` (a full zombie on Hard sees a villager through a window beside the door: without the tag its paths go round the wall and nothing breaks; with it the path goes through the door, it knocks, the upper half is removed and 1021 ends the events), `door_work_opens_a_door_with_its_sound_queues_the_events_and_breaks_the_upper_half`.
  - `server::tests::a_zombie_breaks_a_door_and_the_door_drops` (both halves gone, the lower half in `take_destroyed`, one `minecraft:oak_door` item, one 1021 and at least one 1019, all at the upper half) and `a_copper_door_breaks_and_an_iron_door_holds`: a saved zombie with `CanBreakDoors` keeping only `BreakDoorGoal`, walked into the door along a set path.
  - `level_events::tests::door_knocks_and_breaks_play_the_zombie_door_sounds`.
  - Entities 415 pass; `minecraft_terrain` 1405 pass (3 ignored). `sections::tests::background_full_update_matches_the_synchronous_one` failed once in the full run while other tracks built and passed alone, as in group ti1.
- **Decisions:**
  - Door changes are applied after the mob's AI step; `door_open` reads the pending toggles, so later goals in the same tick see the door as vanilla would.
  - `apply_door_work` lives in vendored `world/villagers.rs`, not in `door_goals.rs`: `set_door_open` and `recompute_for_block` are private to the `world` module, and this keeps them private.
  - The goal is in every zombie's list but stays inert without `CanBreakDoors`, in place of vanilla's add and remove at slot 1 (it has no controls, so the slot changes nothing). It draws nothing from the random until it runs, so zombies without the tag tick exactly as before.
  - The swing clock is the world's game time (`swing_until`, 6 ticks), standing in for `isSwinging`; it does not see the melee goal's swings.
  - `lastBreakProgress` carries over between break attempts as in vanilla.
  - The last 2001 event is not sent: the particles and the break sound come from the lower half's destroy.
  - Drowned don't break doors yet: in the fork they run their own controller, not `zombie_goals`.
  - A zombie needs sight to pick a target (`TargetingConditions.forCombat`), keeps a villager target out of sight, and drops a player after 3 seconds unseen. The owner's steps therefore use a villager seen through the open door before the door is shut.
- **Test steps (creative, Hard, at night, flat grass; in creative the zombie leaves you alone and goes for the villager):**
  - Setup: `/gamemode creative`, `/difficulty hard`, `/time set night`. Build a closed room of oak planks, 5 by 5 inside and 3 high with a roof, with one oak door in the middle of one wall. Inside, put 8 oak fences in a ring round the middle block and use a villager spawn egg on the middle block (the fences keep it in). Open the door. Stand just outside the door with your back to it, looking straight away from the house.
  1. Run `/summon minecraft:zombie ^ ^ ^12 {CanBreakDoors:1b,PersistenceRequired:1b}` (12 blocks ahead of you). The zombie sees the villager through the doorway and walks towards the house. Turn round and close the door before the zombie gets there (if it walks in first, kill it, open the door and try again).
  2. The zombie walks into the closed door and presses against it. Now and then a loud wooden knock plays and it swings its arms. About 12 seconds after it starts, the door breaks with a loud crunch, wood bits fly, an oak door item drops and the zombie walks in to the villager (**doors-1_zombie_breaks**).
  3. Repeat steps 1 and 2 with a copper door: it breaks the same way and drops a copper door. Repeat them with an iron door: no knocks, and the zombie waits outside (**doors-2_iron_holds**).
  4. Put in an oak door and repeat step 1. Once it knocks, run `/difficulty normal`: the knocking stops and the zombie only presses against the door. Run `/difficulty hard`: after a moment it knocks again, and the door breaks about 12 seconds later.
  5. `/gamerule mob_griefing false` and repeat step 1: the zombie presses against the door and never knocks. `/gamerule mob_griefing true`.
  6. Repeat step 1 with a zombie without the tag (`/summon minecraft:zombie ^ ^ ^12`): it stays outside the shut door and never knocks. Most zombies from eggs and the night are like this.
  7. Summon a door breaker as in step 1 and close the door, then save and quit before it breaks the door and load the world again. Open the door for a moment so it sees the villager, then close it: it knocks and breaks the door again (the tag is saved, the target is not, as in vanilla).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:villager_spawn_egg*4,minecraft:oak_door*8,minecraft:copper_door*4,minecraft:iron_door*2,minecraft:oak_planks*64,minecraft:oak_planks*64,minecraft:oak_fence*16,minecraft:torch*16
```

### 2. Minecraft door cracks from mobs show for the host and friends

After bc57c07f.

- **Vanilla (26.3):** `BreakDoorGoal` sends `destroyBlockProgress(mob.getId(), doorPos, stage)` on each stage change and -1 in `stop`; `ServerLevel` sends it to every player within 32 blocks, and the client's `LevelRenderer.destroyBlockProgress` draws stages 0 to 9 and drops the crack for anything else (the tenth stage at 240).
- **Changed:**
  - Vendored `entities/src/door_goals.rs`: `EntityWorld::door_cracks() -> Vec<(u64, Pos, u8)>`, the cracks of the living zombies and illagers whose door goal shows one (`ai.state.door.crack`). It uses only the stores' public lists, so `world.rs` is not touched.
  - `minecraft_survival/src/session.rs`: `MOB_BREAKER` (`0x4000_0000`) and `mob_breaker(id)`, a breaker key that can't meet `BULLETS` or a player id. `level_cracks` adds each mob crack from the level's mob snapshot (`entities.view.world`); `send_cracks` already sends a stage on each change and -1 when a crack is gone, within `CRACK_RANGE`.
  - `render_anim/src/minecraft_world.rs`: the host's crack list takes the same mob cracks from its snapshot.
  - The friend side needs no change: `ToClient::Crack` already draws, and -1 clears.
  - `docs/MINECRAFT-SURVIVAL.md`: the door breaking line names the cracks.
  - Vendored `world.rs`: puts back the space after `let throw =` that bc57c07f dropped by mistake, so that line is unchanged from before the group again.
- **Tests:**
  - Vendored `door_goals::tests::door_cracks_list_the_living_zombies_and_illagers_at_a_door`: a cracking zombie and vindicator are listed with their stage; a dead zombie and one without a crack are not.
  - `session::tests::a_friend_sees_a_zombies_door_crack_grow_and_clear`: a zombie's crack in a real level reaches a friend's session as stage 0, is not sent again unchanged, then stage 5, then -1, all under `mob_breaker(id)`.
  - Entities 416 pass; `minecraft_survival` 758 pass; `check -p launcher` passes.
- **Decisions:**
  - A mob's crack lasts only as long as the mob and its goal: it is keyed per mob in its own breaker range and vanishes as soon as the mob dies or stops breaking. Vanilla clients keep an orphaned crack for up to 400 ticks.
  - The host and the friends read the cracks from the same mob snapshot, so they show the same stage in the same tick.
- **Test steps (setup as in commit 1: creative, Hard, at night, the plank room with the penned villager, the door open, standing outside with your back to it):**
  1. Do steps 1 and 2 of commit 1. While the zombie knocks, cracks spread over the upper half of the door and grow over the 12 seconds until it breaks (**doors-3_cracks**, about half way).
  2. Put in a new oak door and repeat. Once the cracks show, run `/difficulty normal`: the cracks vanish at once. Run `/difficulty hard`.
  3. Repeat, and once the cracks show, hit the zombie with the diamond sword until it dies: the cracks vanish with it.
  4. **Two windows:** put the friend in creative too (`/gamemode creative <friend's name>`) and have them stand outside the house within 30 blocks of the door. Repeat step 1. The friend sees the zombie swing, the cracks grow on the door and the wood bits when it breaks, and hears the knocks and the break. The friend can pick up the dropped door (**doors-4_friend**, both windows).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:villager_spawn_egg*4,minecraft:oak_door*8,minecraft:copper_door*4,minecraft:iron_door*2,minecraft:oak_planks*64,minecraft:oak_planks*64,minecraft:oak_fence*16,minecraft:torch*16,minecraft:diamond_sword
```

## Group doors-raiders

### 1. Minecraft vindicators open and break doors in raids

After 70809097.

- **Vanilla (26.3, checked with javap):**
  - `Vindicator.registerGoals`: slot 2 `VindicatorBreakDoorGoal`, slot 3 `AbstractIllager.RaiderOpenDoorGoal`.
  - `VindicatorBreakDoorGoal`: `BreakDoorGoal(mob, 6, DOOR_BREAKING_PREDICATE)` (Normal or Hard; the break time is still `max(240, 6)`), `setFlags(MOVE)`. `canUse`: `hasActiveRaid()`, then the vindicator's own `random.nextInt(reducedTickDelay(10)) == 0` (`nextInt(5)`), then `BreakDoorGoal.canUse`. `canContinueToUse`: `hasActiveRaid()` and `BreakDoorGoal.canContinueToUse`. `start`: `BreakDoorGoal.start`, then `setNoActionTime(0)`.
  - `RaiderOpenDoorGoal`: `OpenDoorGoal(raider, false)` whose `canUse` is `DoorInteractGoal.canUse` and then `hasActiveRaid()`. `OpenDoorGoal`: `start` sets `forgetTime = 20` and opens the door (it does not call `DoorInteractGoal.start`); `tick` counts `forgetTime` down; `canContinueToUse` is `closeDoor && forgetTime > 0 && !passed`, so false at once for a raider; `stop` shuts the door. `setOpen` acts only when `hasDoor` and the block is still a `DoorBlock`.
  - `Raider.hasActiveRaid`: a current raid that `isActive()`.
  - `Vindicator.customServerAiStep` (after the goals and the navigation tick): when not NoAI and on ground navigation, `navigation.setCanOpenDoors(level.isRaided(blockPosition()))`.
  - The goal selector stops a goal that cannot continue and then starts the goals that can, in one pass. While the vindicator still bumps into the door, its open goal shuts the door and opens it again in the same pass (two door sounds, the door stays open). Once it no longer bumps into the door, the next full pass shuts it.
- **Changed:**
  - Vendored `entities/src/door_goals.rs`:
    - `BreakDoorGoal` has a `raider` flag. New `BreakDoorGoal::vindicator()`: Normal or Hard, no `CanBreakDoors` needed, `Control::Move`, inert without an active raid, the `nextInt(5)` roll before the door checks, and `start` sets `illager.reset_idle` (the existing `setNoActionTime(0)`).
    - New `OpenDoorGoal::raider()` (`close_door: false`, the active-raid check after the door check, `forget_time` 20, every tick). `start` asks for the door open and `stop` asks for it shut, through `DoorInteract::set_open`, which pushes a toggle only when the goal found a wooden door.
  - Vendored `illager_ai.rs`: the vindicator's slots 2 and 3 are these goals instead of `NeverGoal`s. The goal names were already right.
  - Vendored `world/illagers.rs`: `tick_ai` sets a vindicator's `walk.can_open_doors` to `illager.near_raid` right after the navigation tick, and `tick_illager` calls `apply_door_work` after the illager's tick.
  - Vendored `world/villagers.rs`: `apply_door_work` takes the door work of a zombie or an illager, and gives the door sounds to the illager's voices.
  - `docs/MINECRAFT-SURVIVAL.md`: a new "Vindicators and doors" item after "Door breaking". The doc has no vindicator paragraph, so the brief's sentence went there.
- **Tests:**
  - Vendored `door_goals::tests`:
    - `a_raiding_vindicator_opens_a_door_and_shuts_it_once_it_stops_bumping_into_it`: the toggles are open on the first full tick, nothing on the half tick, then shut and open in one pass while it still bumps into the door, then shut once it no longer does. After that nothing more happens.
    - `a_vindicator_outside_an_active_raid_leaves_doors_alone_and_draws_nothing`: no raid and an inactive raid. No toggles, nothing runs, and the random is untouched.
    - `a_raiding_vindicator_chops_at_a_door_on_one_full_tick_in_five_on_normal_and_hard`: for 40 seeds the chop starts on the first full tick whose `nextInt(5)` is 0, sets `reset_idle` and the first crack, and breaks the door 240 ticks later. The goal holds `Move` and the zombie's holds nothing. It never starts on Easy.
    - `a_vindicator_stops_chopping_when_the_raid_ends`: the crack goes when the raid stops being active.
  - Vendored `world::illagers::tests`, in a village (an acquired bell) with a stone wall, an oak door and a window, and a villager seen through the window:
    - `a_raiding_vindicator_opens_a_wooden_door_and_shuts_it_behind`: its path crosses the door, the door opens, it walks through and the door is shut behind it. Nothing is broken, and the last door sound is `block.wooden_door.close`.
    - `a_vindicator_outside_a_raid_paths_around_doors_and_never_opens_them`.
    - `a_vindicator_near_a_raid_it_is_not_in_paths_through_doors_but_does_not_open_them`: it stops against the shut door.
  - Entities 423 pass. `minecraft_terrain` 1404 pass with 3 ignored. `sections::tests::background_full_update_matches_the_synchronous_one` failed once in the full run while other tracks were building and passed when run alone. `check -p launcher` passes.
- **Decisions:**
  - No optional fourth commit. In the test trace the vindicator bumps into the shut door and opens it, walks through on the next tick and is in the doorway when the door shuts, then walks out of the shut door's panel. The door flapping (shut and open in one pass) leaves the door open and its path unchanged. With `canOpenDoors` set, a wooden door on the path can be walked through whether it is open or shut, so recomputing its own path on the toggle, as vanilla's `sendBlockUpdated` does, would give the same path. As before, only villagers recompute their paths on mob door changes.
  - The door work is applied after the vindicator's tick, as for zombies. In the tick it opens the door it still meets the shut door, and it walks through on the next tick. A door it shuts is shut after that tick's move. Vanilla changes the door during the goal tick, before the move.
  - The roll is drawn on every full goal pass of a raiding vindicator whose break goal could start, before the door checks, as in vanilla. Raiding vindicators therefore draw one more number from their random per full tick than before this commit.
  - `OpenDoorGoal.start` leaves `passed` and the start direction alone, as in vanilla. That only matters with `close_door`, which no fork mob uses.
  - Kept from the brief: `finalizeSpawn`'s `setCanOpenDoors(true)` is not modelled (the first tick overwrites it), and a raid door seldom breaks, because the open goal opens it in the same pass and that ends the chop.
  - Pillagers, evokers, witches and ravagers keep no door goals and no door flag, as in vanilla.
- **Test steps (survival, Hard, in a plains village; put on the kit's armour):**
  - Setup: `/difficulty hard`, `/gamerule keep_inventory true`. Run `/locate structure minecraft:village_plains` and `/tp` there. Pick a village house with a wooden door and a window.
  1. Walk at least 70 blocks out of the village, drink an ominous bottle (Bad Omen), then walk back into the village. About 30 seconds later the raid starts and its bar shows at the top.
  2. Go into the house, shut its door and stand 3 blocks inside, where raiders can see you through the window. Fight the first wave from inside or outside as you like. From wave 2 on, vindicators come with axes. A vindicator that runs at you swings the door open with the door's sound, comes in, and the door swings shut behind it with the shut sound (**doors-5_raid_door**).
  3. Watch the door when a vindicator reaches it. Now and then a crack flashes on the door's upper half for a moment just before it opens. The door never breaks while it can be opened.
  4. Win the raid or leave it running, and go at least 110 blocks from the village, build a 5 by 5 plank room with a roof, one oak door and a glass window next to the door, and stand inside 3 blocks from the door. Use a vindicator spawn egg 6 blocks outside the door. It sees you through the window, walks round the house looking for a way in and never opens the door (**doors-6_no_raid**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*2,minecraft:vindicator_spawn_egg*2,minecraft:oak_door*4,minecraft:oak_planks*64,minecraft:oak_planks*64,minecraft:glass*16,minecraft:torch*16,minecraft:diamond_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32
```
