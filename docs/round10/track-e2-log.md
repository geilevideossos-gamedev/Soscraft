# Track E2 log: gameplay and loot

Branch `track-e2`, worktree `C:\Dev\mc-wt\track-e2`, target dir `D:\iw4l-wt\track-e2`. Brief: `docs/round10/track-e.md` (overview, then the E2 sections).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Protocol

- Group `fw-burst`, commit 1 ("Minecraft firework rockets carry their stars"): `firework_rocket::Burst` (inside `EntitySnapshot.firework_bursts`, sent to friends in the `ToClient::Bursts` blob) carries `explosions: Vec<firework::Explosion>` instead of `colors: Vec<i32>`. The release step bumps `PROTOCOL`.

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

## Group maps-loot

### 1. Minecraft keeps map data in the vanilla 26.3 map files

After 701ee0df.

- **Vanilla (26.3, checked with javap):**
  - `MapItemSavedData.CODEC`: `dimension` (level key), `xCenter`, `zCenter` (ints), `scale` (byte, default 0), `colors` (16384 bytes, required; any other length leaves the map blank), `trackingPosition` (default true), `unlimitedTracking` (default false), `locked` (default false), `banners` and `frames` (lists, default empty). `optionalFieldOf` leaves out a value equal to its default. The codec clamps `scale` to 0-4.
  - `MapId.key()` is `maps/<id>` in the server's own data storage, so a map is `data/minecraft/maps/<id>.dat`. `MapIndex` is `maps/last_id` with one field `map` (default -1); `getNextMapId` adds one first, so the first map is 0.
  - `createFresh(x, z, scale)`: `i = 128 << scale`, `centre = floor((x + 64) / i) * i + i / 2 - 64`.
  - `addDecoration`: `f = (worldX - centreX) / (1 << scale)` as a float. `clampMapCoordinate`: f <= -63 is -128, f >= 63 is 127, else `(byte)(int)(f * 2 + 0.5)`. `isInsideMap`: -63 <= f <= 63 on both axes. `calculateRotation`: `(byte)(int)((rot +- 8) * 16 / 360)`, and in the Nether `(t * t * 34187121 + t * 121) >> 15 & 15` with `t = gameTime / 10`. Other markers show only inside the map or with `unlimitedTracking`; the holder outside it is `player_off_map` within 320 map units, `player_off_limits` with `unlimitedTracking`, else nothing.
- **Changed:**
  - `minecraft_terrain/src/maps.rs` (new):
    - `MapData` with the codec's fields (`colors: Box<[u8; 16384]>`, banners and frames kept as raw NBT so they round trip), `create_fresh`, `map_units`, `decoration(kind, x, z, rot, game_time)` (vanilla's `calculateDecorationLocationAndType`), `to_tag` and `from_tag`.
    - The pure helpers `inside`, `clamp_coordinate`, `rotation_byte`, `nether_rotation`, `player_marker`.
    - `MapStore`: the loaded maps, `last_id` (-1 in a new world), the dirty maps and whether the index changed. `load(storage)` reads `last_id.dat` and every `<id>.dat` in the folder; `free_id` is `getFreeMapId`; `insert`, `get`, `get_mut` (marks the map dirty); `take_writes` hands the dirty maps and the index to a `MapWrites`, which the save thread writes with `LevelStorage::write_saved`.
  - `minecraft_survival/src/server.rs`: `Globals.maps: Mutex<MapStore>` next to `weather`. `WorldFiles` takes the map writes with the rest of the world files, so the autosave and the closing save write new and changed maps next to `weather.dat`.
  - `minecraft_survival/src/save.rs`: `OpenedWorld.maps`, read when the world opens (an unreadable map is a warning and is left out); `WorldSave::write_maps`.
  - `render_anim/src/minecraft_world.rs`: the host's `Globals` takes `opened.maps`.
- **Tests:**
  - `maps::tests`: `a_fresh_map_snaps_its_centre_to_the_vanilla_grid` ((1000, -300) at scale 1 is centred on (1088, -192), (0, 0) on (0, 0), and the grid edges and scale 4), `a_target_is_placed_clamped_and_turned_like_vanilla` (the target (1000, -300) is (-87, -107) with rotation 8; f = +-63 clamps to -128 and 127; far targets stay on the edge of an unlimited map and vanish from a plain one), `the_holder_marker_leaves_the_map_like_vanilla` (off the map at 300 map units, off limits at 400 only with unlimited tracking, the Nether spin), `map_data_keeps_the_vanilla_keys_and_skips_defaults`, `maps_are_saved_in_the_vanilla_files_and_come_back` (the first id is 0, `last_id.dat` holds `map: 0`, an unchanged map is not written again), `a_map_file_beyond_the_index_is_never_overwritten`.
  - `server::tests::autosaves_and_closing_write_new_maps_into_the_vanilla_map_files`: an autosave writes `0.dat` and `last_id.dat`; the closing save writes only the map that changed after it; the reopened world has both ids counted and the new map back.
- **Decisions:**
  - `Globals.maps` is a `Mutex`, because the host and the friends open chests with a shared `&Globals` (`menus::open_station`, `FriendWorld.globals`). It is only locked on the main thread and for the save snapshot.
  - All map files are read when the world opens, so later groups can draw and tick maps without reading files in a frame. The next id is past the highest map file even when `last_id.dat` says less, so a lost index never overwrites a map. Vanilla trusts the index.
  - The map decorations of a held map (`maps-held`) are not saved, as in vanilla; only banners and frames are, and those stay as they were read.
- **Test steps:** none in game for this commit alone; the next commit makes the first maps. The check is step 6 of commit 2.
- **Kit:** none.

### 2. Minecraft shipwreck, ocean ruin and camp chests roll explorer maps

After 6d6c26ca.

- **Vanilla (26.3, `ExplorationMapFunction.run`, checked with javap):** an empty stack or a roll with no `ORIGIN` comes back unchanged, and the item is not checked. Otherwise `findNearestMapStructure(destination, BlockPos.containing(origin), search_radius, skip_existing_chunks)`; when nothing is found the stack comes back unchanged and the table's `filtered{map_id}` step discards it. When found, `applyNewSavedData(x, z, zoom, tracking true, unlimited true)` sets `minecraft:map_id`, and `addTargetDecoration` adds `minecraft:map_decorations` entry `"+"` = `{type, x, z, rotation: 180.0f}` (`x` and `z` doubles). Defaults: decoration `minecraft:mansion`, zoom 2, radius 50, skip true. In 26.3 the explorer maps are their own items (`minecraft:buried_treasure_map` and so on), so the stack keeps its id.
- **Changed:**
  - Vendored `core/src/loot.rs`: new `pub trait Exploring` (`explore(destination, origin, radius, skip_known, zoom) -> Option<(map id, target)>`) next to `Enchanting`, `LootParams.exploring`, and an `exploration_map` arm next to `filtered`. It resolves `destination` through `tags/worldgen/structure`, reads the defaults, floors the origin, and writes `map_id` (Int) and merges the `"+"` mark into any marks already there. Without a stack, an origin, a hook or a result it leaves the stack alone.
  - The same file's tests: `unknown_functions_still_fail` now uses `minecraft:furnace_smelt`; the survey has no exception left and runs every table with a stub explorer.
  - `minecraft_terrain/src/maps.rs`: `LootExplorer` (the level's `WorldGen`, its dimension and the first free map id) implements `Exploring` with `locate::nearest_structure` and a 2 s deadline (`SEARCH_TIME`). Each find makes a `MapData::create_fresh(target, zoom, tracking, unlimited)` with the next id; a timeout counts as nothing found and is kept for the log. `take()` hands over the new maps and the timeouts.
  - `minecraft_terrain/src/containers.rs`: `structure_loot_in` takes the explorer and puts it in `LootParams`.
  - `minecraft_survival/src/menus.rs` `open_station`: a chest or barrel opened for the first time gets an explorer for the player's level, with the store's next id. When the roll is used, its new maps go into `globals.maps` (which moves `last_id` on) and timeouts are logged. The host and the friends share this path.
- **Tests:**
  - Vendored `loot::tests`: `exploration_maps_ask_the_explorer_and_mark_its_target` (map id, the red X at the target with rotation 180.0, the treasure map's radius, zoom and skip, the defaults `minecraft:mansion`, zoom 2, radius 50, skip true, and other marks kept), `an_exploration_map_with_nothing_found_or_no_origin_is_filtered_out` (nothing found, no origin or no hook discards the map; no origin and an empty stack never search; a seeded `chests/shipwreck_map` roll has the map only when one is found, and the rest of the chest rolls either way), `unknown_functions_still_fail` (furnace_smelt), and the survey of every chest, spawner, equipment and gameplay table with no unsupported function and every destination resolved.
  - `maps::tests::a_shipwreck_map_chest_rolls_a_treasure_map_to_the_nearest_buried_treasure` (seed 77, gated on `MINECRAFTOSS_ROOT`): the nearest shipwreck's map chest rolls a `minecraft:buried_treasure_map` with map id 5; its X is `nearest_structure(["minecraft:buried_treasure"], origin, 50)` at block 9, 9 of its chunk; the explorer made exactly that map (scale 1, unlimited); and the generated chunk holds the `chests/buried_treasure` chest in that column.
  - `maps::tests::a_chest_without_maps_or_with_nothing_to_find_makes_no_map`: a dungeon chest makes no map; a search for a Nether fortress finds nothing; a far search gives up within about 2 s and is kept as a timeout.
  - The existing `containers::tests` loot tests pass unchanged apart from the new argument.
- **Decisions:**
  - `unknown_functions_still_fail` uses `minecraft:furnace_smelt`: of the 26.3 functions in the loot tables only it and `enchanted_count_increase` are left unported in this file, both in entity tables.
  - The search runs on the main thread when the chest is opened, as in vanilla, and stops after 2 s. A woodland mansion map in a camp's secret chest can therefore pause the game for up to 2 s.
  - New maps get their ids only when the roll is kept. A roll that fails frees its ids again; vanilla would have used them up.
  - A map's zoom is clamped to 4, as the vanilla codec does when it loads a map.
  - The `skip_existing_chunks` flag reaches the explorer but is not used until commit 4, so a camp map can still point at its own camp until then.
  - Cartographer trades still skip explorer maps (left for later).
- **Test steps (survival, Normal; holding a map shows nothing yet, that is group `maps-held`):**
  1. Load a world with the kit. Run `/locate structure minecraft:shipwreck` and `/tp` to it. Drink a water breathing potion.
  2. Open the shipwreck's chests. The map chest is usually in the cabin at the back. It now holds paper, feathers, a compass, map or clock, and a **Buried Treasure Map** (**maps-1_shipwreck**). If the wreck has no map chest (half wrecks), locate the next one.
  3. Run `/locate structure minecraft:ocean_ruin_warm` (or `ocean_ruin_cold`) and `/tp` there. The ruin chests roll coal, wheat, gold nuggets and the like again instead of nothing, and about 4 in 10 also hold a Buried Treasure Map (**maps-2_ruin**).
  4. Save and quit. In the world folder, `data/minecraft/maps/0.dat` and `data/minecraft/maps/last_id.dat` are there, and there is one numbered file for each treasure map found (**maps-3_files**, a screenshot of the folder).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:potion@long_water_breathing,minecraft:potion@long_water_breathing,minecraft:potion@long_water_breathing,minecraft:potion@night_vision,minecraft:potion@night_vision,minecraft:oak_boat,minecraft:diamond_pickaxe,minecraft:iron_shovel,minecraft:torch*64,minecraft:cobblestone*64,minecraft:cooked_beef*32,minecraft:turtle_helmet
```

### 3. Minecraft explorer maps show the coastline around their target

After 79d69c23.

- **Vanilla (26.3, `MapItem.renderBiomePreviewMap`, checked with javap):** with `s = 1 << scale`, it reads `getBiome(((centreX / s - 64) + x) * s, seaLevel, ((centreZ / s - 64) + z) * s)` for each of the 128 x 128 map pixels and asks whether the biome is in `#minecraft:water_on_map_outlines` (`#is_ocean`, `#is_river`, `swamp`, `mangrove_swamp`). For the inner pixels 1-126 it counts the watery neighbours n out of 8 and sets `colors[x + z * 128]` to `id << 2 | brightness`:
  - a watery pixel is `COLOR_ORANGE` (15): with n > 7 on an even z its brightness is `((x + (int)(Mth.sin(z) * 7)) / 8) % 5` (0 and 4 LOW, 1 and 3 NORMAL, 2 HIGH); with n > 7 on an odd z it stays clear; n > 5 NORMAL; n > 1 LOW; else LOWEST;
  - a dry pixel with n > 0 is `COLOR_BROWN` (26), NORMAL when n > 3, else LOWEST; other pixels stay clear.
  - Brightness ids: LOW 0, NORMAL 1, HIGH 2, LOWEST 3. `Mth.sin` is the 65536-entry table, which the generator crate already ports (`providers::sin`).
- **Changed:** `minecraft_terrain/src/maps.rs`:
  - `biome_preview(map, watery)` ports the function over any `watery(x, z)` test, with `packed(colour, brightness)` and the colour and brightness ids as constants.
  - `coastline(gen, map)` runs it on the level's generator: the zoomed biome at sea level (`zoom::quart_for_block` with the world's zoom seed, then `biome_at_quart`), each quart worked out once, and the 26.3 tag through the registries' biome tags.
  - `LootExplorer::explore` draws the coastline on each new map before it is handed over, so the map file holds it from the start.
- **Tests:**
  - `maps::tests::the_coastline_preview_outlines_water_like_vanilla`: on a hand-made shore, a dry spot in the water, a lone pond and open water, the shore's water and land, the stripe (including a negative `sin`), the clear odd rows, the 6-7 neighbour NORMAL, the lone pond's LOWEST and the untouched border match the formula; a scale 1 map centred on (1088, -192) samples blocks (960, -320) to (1214, -66).
  - `maps::tests::a_shipwreck_map_chest_rolls_a_treasure_map_to_the_nearest_buried_treasure` now also checks that the explorer's map equals a fresh map run through `coastline`, and that it holds only clear, orange and brown pixels, with both orange and brown (the treasure lies on a coast).
- **Decisions:**
  - The quarts are cached per map only (about 66 x 66 at scale 1); the biomes come from the noise, which is what the chunks would hold.
  - The coastline is drawn on the main thread with the chest open, like the search; it adds about a tenth of a second in a release build.
- **Test steps:** the coastline is drawn into the map data now, but no map is shown on screen until group `maps-held`/`maps-view`. Nothing new to see in game; the check is with the map held in the later groups (orange outlines of the coast around the X).
- **Kit:** none.

### 4. Minecraft explorer maps skip structures another map already points to

After 03093a8b.

- **Vanilla (26.3, checked with javap):**
  - `ExplorationMapFunction.run` with `skip_existing_chunks` first runs `claimStructureAt`: `StructureManager.getStructureAt(pos, destination)` takes the first start of a destination structure, among those referenced in the chest's chunk, whose bounding box holds the chest, and adds a reference when it `isValid() && canBeReferenced()` (`references < getMaxReferences()`, which is 1). It looks no further when that start is already referenced.
  - `ChunkGenerator.getStructureGeneratingAt` with skip: `checkStructurePresence` reports a start that already has a reference as not present, and a start found in the chunk counts only when `tryAddReference` succeeds. So each skipping search claims the start it returns, and a claimed start is never found again by a skipping search.
  - Shipwreck and ocean ruin treasure maps pass `skip_existing_chunks: false`; camp maps and the secret chest's maps keep the default `true`.
- **Changed:**
  - `minecraft_terrain/src/locate.rs`: `Claim` (an optional `FnMut(structure, start chunk) -> bool`), threaded through `search_structures`, `nearest_ring`, `nearest_in_ring` and `generating_at`, which counts a start only when the claim takes it. `nearest_structure` passes none and works as before; new `nearest_unclaimed_structure` passes one. New `claim_structure_at(gen, targets, pos, claim)`: the start of a target structure, from the potential chunks of the 3 x 3 spread regions around the chest, whose bounding box holds the chest.
  - `minecraft_terrain/src/maps.rs`: `Claims` ((structure, start chunk) to references, `try_claim` up to `MAX_REFERENCES` 1, NBT `{claims: [{structure, chunk_x, chunk_z, references}]}`). `MapStore` keeps the claims per dimension, reads them from `dimensions/minecraft/<dim>/data/iw4l/map_claims.dat` when the world opens, and `take_writes` hands over the changed ones. `LootExplorer::with_claims` takes the level's claims; a skipping `explore` claims the chest's own structure, then searches with claims, and `Explored.claims` holds the result.
  - `minecraft_survival/src/menus.rs`: the explorer starts from the level's claims, and a kept roll stores the new claims with its maps, so the autosave and the closing save write them.
- **Tests:**
  - `maps::tests::claims_hold_one_reference_and_are_saved_beside_each_level`: one reference per start, the NBT round trip, the file in the overworld's `data/iw4l`, no file for an empty list, nothing written again unchanged, and the claims back after a reload.
  - `maps::tests::treasure_maps_that_skip_known_treasures_each_point_to_another` (seed 77, gated): from the nearest shipwreck, the first skipping treasure map points to the nearest treasure and the second to another; a map that keeps known treasures still points to the nearest; both treasures are claimed; after a save and a load a third skipping map points to neither.
  - `maps::tests::a_map_that_skips_known_structures_never_points_at_the_one_it_lies_in` (gated): from the middle of a shipwreck, a keeping search finds that wreck and claims nothing; a skipping one claims that wreck and points to another; outside any wreck only the target is claimed.
  - The `locate::tests` pass unchanged.
- **Decisions:**
  - Our chunks save no structure starts (`chunk_nbt.rs:246`), so the claims are kept in a fork file, `dimensions/minecraft/<dim>/data/iw4l/map_claims.dat`, which vanilla ignores. Vanilla keeps them as `references` on the start in the chunk.
  - `claim_structure_at` looks at the starts of the 3 x 3 spread regions around the chest instead of the chunk's structure references, which our chunks do not keep. A structure reaching more than one spread region from its start would be missed; none of the map structures does.
  - As in vanilla, a search that claims a start and then finds a closer one in a later placement of the same ring keeps both claims; the map destinations here have one placement each.
  - The claims are taken with the roll and kept only when the chest's loot is kept, like the map ids.
- **Test steps (survival, by day; the maps' targets can be seen once group `maps-held` lands):**
  1. Load a world with the kit. Run `/locate structure #minecraft:abandoned_camp` and `/tp` there.
  2. Open the camp's chests. A common chest now rolls its loot (arrows, bones, candles and the like) and often a camp map named after another biome's camp, such as a **Swamp Camp Map** (**maps-4_camp**). Dig around the camp for the secret chest: it can hold an explorer map such as a **Woodland Mansion Map**, a **Buried Trial Chambers Map** or a **Buried Ancient City Map**; opening it may pause the game for up to 2 seconds while the map searches (**maps-5_secret**).
  3. Save and quit. In the world folder, `dimensions/minecraft/overworld/data/iw4l/map_claims.dat` is there once a camp map was found (**maps-6_claims**, a screenshot of the folder). Shipwreck and ruin treasure maps alone do not make this file.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:iron_shovel,minecraft:diamond_pickaxe,minecraft:torch*64,minecraft:cobblestone*64,minecraft:cooked_beef*32,minecraft:oak_boat
```

### 5. Minecraft explorer map tests give slow searches time

After 0416f340.

- **Why:** in the full `minecraft_terrain` run (debug build, every test at once, other tracks building), `maps::tests::a_shipwreck_map_chest_rolls_a_treasure_map_to_the_nearest_buried_treasure` failed once: the treasure search ran past the game's 2 s budget, so the explorer gave up and the chest had no map. In the game the budget stays 2 s; only the tests wait longer.
- **Changed:** `minecraft_terrain/src/maps.rs`: `LootExplorer` keeps its own `search_time` (`SEARCH_TIME`, 2 s, unless `with_search_time` sets another). The tests that need a find use 120 s; the timeout test checks that a new explorer has the 2 s budget and gives up within a 100 ms one.
  - The same file: `packed` brackets its shift, `(colour << 2) | brightness`, as vanilla's `getPackedId` reads; the value is unchanged.
- **Tests:** the 12 `maps::tests` pass. The full `minecraft_terrain` run has 1416 passed with 3 ignored; `sections::tests::background_full_update_matches_the_synchronous_one` failed once while other tracks built and passed alone, as in the earlier groups. Full `minecraft_survival` (759), `minecraftoss-core` (37) and `render_anim` (212) pass, and `check -p launcher` passes.
- **Test steps and kit:** none (no change in game).

## Group lead-knots

### 1. Minecraft fences are aimed at by their post and arms like vanilla

After 94171133. This commit is not in the brief: it is what the knot punch needs. The hand ray took every fence as a full block, so from any side the fence was met before the knot, which sits inside that block.

- **Vanilla (26.3, checked with javap):** `FenceBlock` is `CrossCollisionBlock(4, 16, 4, 16, 24)`, and its outline is `makeShapes(4, 16, 4, 0, 16)`: the post `Block.column(4, 0, 16)` (6/16 to 10/16 across, a full block tall) and, for each connected side, the arm `Block.boxZ(4, 0, 16, 0, 8)` turned to that side (4/16 wide, a full block tall, from the edge to the middle).
- **Changed:** vendored `player/src/lib.rs`: new `fence_outline`, and `outline_shapes` uses it for every `*_fence` (13 woods and the nether brick fence; fence gates keep their own shape). The hand ray (`Player::target`) and `outline_bounds` now see the post and its arms; collision is unchanged.
- **Tests:** vendored `player::tests::a_fence_is_outlined_by_its_post_and_arms_and_the_ray_passes_beside_them`: a lone post, the north and east arms of a nether brick fence, the south and west arms of a pale oak fence, the bounds of an east-west fence, a ray beside a lone post that reaches the stone behind it, and a ray that stops on the post's south face.
- **Decision:** the selection box and the crack overlay (`outline_bounds`) now hug the post and arms. Walls, panes and iron bars still count as full blocks (left for later).
- **Test steps (survival, by day, flat grass):**
  1. Place a single oak fence post with a stone block right behind it. Aim just past the side of the post at the stone: the stone gets the outline and is mined, not the fence (**lk-1_beside_post**).
  2. Place a row of three fences. The middle one's outline is the post with an arm to each side, not a full block.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:oak_fence*16,minecraft:stone*16,minecraft:iron_pickaxe
```

### 2. Minecraft lead knots can be punched off

After 3188f4f8.

- **Vanilla (26.3, checked with javap):**
  - `LeashFenceKnotEntity` is a `BlockAttachedEntity` with a 0.375 x 0.5 box at (x + 0.5, y + 0.375, z + 0.5), so from (x + 0.3125, y + 0.375, z + 0.3125) to (x + 0.6875, y + 0.875, z + 0.6875). It sticks out 1/16 past each side of the post; from straight above the post's top is met first.
  - A left click is `Player.attack` → `cannotAttack` → `skipAttackInteraction` → `hurtOrSimulate(playerAttack, 0)`. `BlockAttachedEntity.hurtServer` kills the knot, and `LeashFenceKnotEntity.dropItem` only plays `item.lead.untied` (volume 1, pitch 1). Because `cannotAttack` returns early, the player gets no exhaustion and the weapon no wear; the client still resets the attack strength.
  - The mobs let go on their next tick (`Leashable.tickLeash` with a holder that can no longer interact): `dropLeash()`, one lead at each mob. Creative players punch knots the same way, and the leads still drop.
- **Changed:**
  - Vendored `entities/src/world/leashes.rs`: new `break_knot(pos)`: removes the knot, plays the untied sound at the knot, drops the leash (with a lead) of every mob tied to it and of every mob whose leash is still only saved to it, and returns whether there was anything. `tick_knots` calls it when the fence is gone, as before. Two private helpers, `saved_to_knot` and `drop_saved_leash`.
  - Vendored `entities/src/world.rs`: `KNOT_HEIGHT` and `KNOT_WIDTH` are re-exported with the other leash names.
  - `minecraft_terrain/src/leash_render.rs`: `KnotAction { Punch }`, `knot_box(pos)` (the vanilla knot box) and `knot_on_ray(knots, eye, look, reach)`, the nearest knot box on a ray, through `end_crystal::ray_box`.
  - `minecraft_terrain/src/server.rs`: `Command::Knot { player, pos, action }` next to `LeashFence`, `ServerHandle::knot_action` and `ServerSim::knot_action`, which breaks the knot and spawns the leads (`drop_leads`).
  - `minecraft_survival/src/entities.rs`: `knot_target` (the nearest knot on the ray when it is nearer than the block outline and than every mob, frame, player, End target, fireball and shulker bullet), `punch_knot`, which sends `KnotAction::Punch` for it, and `mob_distance` now counts knots, so neither creative breaking nor survival digging reaches the fence behind a knot. The old body of `mob_distance` is the private `target_distance`.
  - `minecraft_survival/src/actions.rs`: the attack press tries `punch_knot` first. A hit resets the attack strength and swings, with no exhaustion and no weapon wear; the player punch and the mob punch are skipped.
  - Friends' clicks run through the same `act` on the host, and the knot leaves their view with the mobs' `leash` look tag, so the protocol does not change.
- **Tests:**
  - Vendored `world::leashes::tests::punching_a_knot_frees_its_mobs_with_a_lead_each_and_the_untied_sound` (two cows on one knot, a third on another: the first knot goes, both its cows are free with a lead each at their feet, one untied sound at (3.5, 64.375, 0.5) in `friendly_volume`, the other knot stays, and a second punch or a punch at a bare fence does nothing) and `a_knot_known_only_from_a_save_can_be_punched_off_too`. `a_knot_lets_its_mobs_go_at_its_check_once_its_fence_is_gone` passes unchanged after the refactor.
  - `leash_render::tests::a_knot_is_hit_from_the_side_before_its_fence_post_but_not_from_above`: the knot box, a side ray meeting the knot at 2.3125 before the post (from the player crate's fence outline), a ray from above meeting the post's top first, a ray below the knot missing it, and a knot out of reach.
  - `server::tests::punching_a_knot_drops_a_lead_for_each_tied_cow_and_leaves_the_fence`: two cows tied to an oak fence; the punch leaves the fence, frees both cows, spawns two leads, saves no `leash` tag and plays only the untied sound; a second punch does nothing.
  - `actions::tests::a_left_click_at_a_knot_breaks_the_knot_and_not_the_fence` (gated on game data: swing, attack strength reset, no digging, no dig effect, no exhaustion) and `a_left_click_at_the_post_below_the_knot_still_digs_the_fence`.
- **Decisions:**
  - A knot stays a fence position, not an entity. The punch is a ray against the vanilla knot box, nearer than the block outline and every entity, within the entity reach.
  - The mobs let go at once instead of on their next tick, as `tick_knots` already did.
  - `KnotAction` lives in `leash_render.rs`, next to the knot box, as `FrameAction` lives in `item_frame.rs`.
  - No game events (`ENTITY_DIE`): track W wires game events. `Command::Knot` carries the player for that.
- **Left for later:** arrows, tridents and thrown items breaking knots; explosions breaking knots; MW2 bullets passing through knots; the `entity_drops` and `mob_griefing` rules on knots.
- **Test steps (survival, by day, flat grass):**
  1. Use the cow egg twice. Put a lead on each cow and right-click an oak fence post: a knot appears on the post with two ropes.
  2. Stand 2 blocks from the fence, aim at the knot from the side (the upper half of the post) and left-click once with the sword. The knot vanishes with the untie sound, a lead drops by each cow and both cows are free. The fence is still there and the hunger bar has not moved (**lk-2_knot_punched**: the fence without a knot and two leads on the ground).
  3. Tie a cow to the fence again. Left-click the bottom third of the post, below the knot: the fence is dug as before, and the cow goes free within 5 seconds.
  4. `/gamemode creative`. Tie a cow to a fence, left-click the knot: the knot goes, the fence stays and a lead drops. `/gamemode survival`.
  5. Tie a cow, punch the knot off, save and quit, load: the cow is loose and there is no knot.
  6. **Friend window:** the host ties a cow to a fence and the friend punches the knot. Both see the knot vanish within half a second and the lead drop (**lk-3_friend_punch**, the friend's view).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:lead*16,minecraft:cow_spawn_egg*4,minecraft:oak_fence*16,minecraft:iron_sword,minecraft:wheat*16,minecraft:cooked_beef*16
```

### 3. Minecraft shears cut every lead off a lead knot

After b64f8651.

- **Vanilla (26.3, checked with javap):** `LeashFenceKnotEntity.interact` with shears first runs `Entity.interact` (its sneaking branch needs a `Leashable`, which a knot is not) → `shearOffAllLeashConnections(player)` → `dropAllLeashConnections`: every mob in `Leashable.leashableInArea` (a 32-block box around the knot's centre) whose holder is the knot gets `dropLeash()`, one lead each, and the `SHEAR` game event fires. Then `ServerLevel.playSound(null, blockPosition(), SHEARS_SNIP, player's sound source)`, volume 1 and pitch 1 at the block centre, and `hurtAndBreak(1, player, hand)` (unbreaking applies; players with infinite materials take no wear). The knot discards itself once its last mob is gone, with no untied sound. With nothing tied, the click falls through to the knot's own tie and hand-back. The shears win over tying the player's own mobs, so a player who holds shears and leads a cow cuts the knot.
- **Changed:**
  - Vendored `entities/src/world/leashes.rs`: new `shear_knot(pos)`: every mob tied to the knot within the vanilla area, and every mob only saved to it, drops its leash with a lead; the snip plays at the block centre in `players_volume`; the knot goes through `forget_empty_knot`, with no untie. False when nothing was tied.
  - `minecraft_terrain/src/leash_render.rs`: `KnotAction::Shear`. `minecraft_terrain/src/server.rs`: `ServerSim::knot_action` runs `shear_knot` for it and spawns the leads.
  - `minecraft_survival/src/entities.rs`: `shear_knot(player, pos)`, tried on a copy of the view's mob world first, like `leash_fence`, and sent only when the copy cuts something.
  - `minecraft_survival/src/actions.rs`: on a use press with shears in either hand, the knot on the ray (`knot_target`, within the entity reach and nearer than the block and every entity) is sheared. In survival the shears wear one point through `durability::wear` (unbreaking applies), and broken shears go to `broken_items` for the break sound. The hand swings and the use ends there; tying at the fence, the bell and the block use are skipped for that click.
  - No protocol change.
- **Tests:**
  - Vendored `world::leashes::tests::shears_cut_every_lead_off_a_knot_with_a_snip_and_no_untie` (two tied cows and one saved to the knot are cut with three leads, a cow on another knot stays, one snip at (3.5, 64.5, 0.5) in `players_volume` and no untie) and `shears_on_an_empty_knot_do_nothing` (a bare fence, and a knot whose cow is beyond the 32-block box).
  - `server::tests::punching_a_knot_drops_a_lead_for_each_tied_cow_and_leaves_the_fence` now also shears: nothing to cut on the bare fence, then both cows tied again are cut with two more leads, only the snip sound, and the fence stays.
  - `actions::tests::shears_on_a_knot_cut_the_leads_and_wear_one_point` (gated on game data): a right-click at the knot wears fresh shears to damage 1 and swings; with the knot gone nothing wears; in creative the shears cut without wear.
- **Decisions:**
  - Shears cut on a fresh use press only, not on the repeat while use is held, so the shears can never wear twice before the host's view sees the knot gone.
  - Shears work from the offhand too, as vanilla tries each hand in turn.
  - The wear goes through `durability::wear`, so unbreaking can spare it as in vanilla. The older shear sites in `mob_actions.rs` (a lead off a mob, sheep, snow golems, mooshrooms) still wear one point flat; that is left as it was.
  - No `SHEAR` game event: track W wires game events.
- **Test steps (survival, by day, flat grass):**
  1. Tie both cows to a nether brick fence. Hold fresh shears and right-click the knot from the side: the snip sound, two leads drop and the knot is gone. There is no untie sound. The shears now show a durability bar (one point of 238 used; the bar itself still looks full) (**lk-4_shears**).
  2. Right-click a fence with no knot with the shears: nothing is cut and the shears do not wear. Then with an empty hand and a cow on your lead, right-click the fence: it ties as before.
  3. Lead a cow yourself, tie another cow to a fence, hold the shears and right-click the knot: the knot's cow is cut loose; your own cow stays on your lead.
  4. **Friend window:** the friend ties their own cow to a fence and cuts the knot with shears. The host sees the knot go and the lead drop, and the friend's fresh shears show a durability bar (**lk-5_friend_shears**, the friend's view).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:lead*16,minecraft:cow_spawn_egg*4,minecraft:oak_fence*16,minecraft:nether_brick_fence*8,minecraft:shears,minecraft:shears,minecraft:iron_sword,minecraft:wheat*16,minecraft:cooked_beef*16
```

## Group cat-chests

### 1. Minecraft cats sitting on a chest keep it shut

After 2085bded.

- **Vanilla (26.3, checked with javap):** `ChestBlock.isChestBlockedAt` is `isBlockedChestByBlock` (a redstone conductor above) or `isCatSittingOnChest`: `getEntitiesOfClass(Cat.class, new AABB(x, y + 1, z, x + 1, y + 2, z + 1))` and any of them `isInSittingPose()`. `AABB.intersects` is strict, so a box that only touches the space above does not count. Only the `Cat` class counts (kittens too, never ocelots), and lying is not sitting. The combiner tests the clicked half and, for a double chest, the other half; either blocked means no menu, while the use still succeeds (the arm swings, with no screen, sound or stat). Trapped and copper chests are `ChestBlock`s and block the same way. `EnderChestBlock` checks only the block above.
- **Changed:**
  - Vendored `entities/src/world/felines.rs`: new `EntityWorld::cat_sits_on(pos)`: any cat (`is_cat()`) with `sitting` whose body box (its position ± half its width, its feet to its height) strictly overlaps the block space above `pos`.
  - `minecraft_survival/src/entities.rs`: new `cat_on_chest(pos)`, which asks the view's mob world (the server's, copied each tick).
  - `minecraft_survival/src/actions.rs`: in the open check, `Station::Chest` (plain, trapped and copper chests) is now apart from `Station::Ender`. A chest stays shut when any of its cells has a conductor above it or a sitting cat on it; the ender chest keeps the conductor check only. The arm still swings.
  - Friends' clicks run through the same `act` on the host, so the cat keeps the chest shut for them too. No protocol change.
- **Tests:**
  - Vendored `world::felines::tests::a_sitting_cat_on_a_chest_is_found_but_not_a_standing_or_lying_cat_nor_an_ocelot` (also: not the chest beside it, not a block above it, and a sitting kitten counts) and `a_cat_sitting_on_the_floor_beside_a_chest_does_not_count` (and not one whose feet are at the top of the space above; a cat on the block beside whose body reaches over the chest does count, as in vanilla).
  - `actions::tests` (gated on game data): `a_chest_with_a_sitting_cat_on_it_stays_shut_but_the_arm_swings` (a plain, a trapped and a copper chest: shut with a swing and nothing built against it, then open once the cat stands), `either_half_of_a_double_chest_with_a_cat_on_it_keeps_both_shut` (the cat on the other half, then on the clicked half, then standing: both halves open), `an_ender_chest_ignores_cats`.
- **Decisions:**
  - Cats block chest menus only. Hoppers and comparators ignore them, as vanilla's `getContainer(..., true)` does.
  - The check reads the host's view of the mob world, which is at most one tick behind the server, as the knot punch and the fence tie do.
- **Test steps (survival, by day, flat grass):**
  1. Use the cat egg and tame the cat with cod until it sits with a collar. Right-click it with an empty hand so it stands.
  2. Place two chests side by side (a double chest) 2 blocks from the cat. Within about 10 seconds it walks onto one half and sits.
  3. Right-click the other half: the arm swings, the chest stays shut and makes no sound. Right-click the half under the cat: the same (**cc-1_cat_on_chest**).
  4. Right-click the cat twice (sit, then stand) and walk 15 blocks away so it follows you off the chest. Go back and open the chest: it opens.
  5. Cats only sit on plain chests by themselves (`#cats_can_sit_on`), so put one on the others by command. Place a trapped chest, jump onto it and run `/summon minecraft:cat ~ ~ ~ {Sitting:1b}` (it loads sitting), then step off: the trapped chest stays shut. Do the same on a copper chest: it stays shut. Do the same on an ender chest: it still opens (vanilla checks only the block above an ender chest).
  6. **Friend window:** the cat sits on the host's chest and the friend right-clicks it: it stays shut for the friend too, and the friend's arm swings (**cc-2_friend_cat_chest**, the friend's view).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cat_spawn_egg*4,minecraft:cod*64,minecraft:chest*4,minecraft:trapped_chest*2,minecraft:copper_chest*2,minecraft:ender_chest,minecraft:cooked_beef*16
```

### 2. Minecraft cats leave chests that someone has open

After d5559520.

- **Vanilla (26.3, checked with javap):** `CatSitOnBlockGoal.isValidTarget` needs air above, `#cats_can_sit_on` and its `BLOCK_TYPE_VALIDATION`: for `Blocks.CHEST` (the plain chest only) `ChestBlockEntity.getOpenCount(level, pos) < 1`, a furnace must be lit, and a bed must not be its head. `MoveToBlockGoal.canContinueToUse` checks `isValidTarget` again, so a sitting cat stands up and leaves once someone opens its chest. A double chest counts its openers on both halves (`CompoundContainer.startOpen`).
- **Changed:**
  - Vendored `player/src/lib.rs`: the `World` trait has a new default method `chest_open_count(pos)`, 0 for worlds without chests.
  - `minecraft_terrain/src/server_mobs.rs`: `MobWorld` answers it from the level's `chest_openers`.
  - Vendored `entities/src/feline_ai.rs`: `can_sit_on` takes a plain chest only while `chest_open_count(pos) < 1`. `BlockGoal::can_continue` already checks `valid` again, so a cat on a chest that is opened stands up.
  - `minecraft_survival/src/level.rs`: the server's level used to hear the openers of trapped chests only (for their redstone signal). `OpenerOutcome.trapped` is now `chest_openers` and covers plain chests too, so the cats can see them. The trapped chest's signal still reads only trapped chests (`trapped_chest_signal`), and a broken chest still forgets its openers. The callers in `level.rs` (a player leaving), `menus.rs` (`lid_effects`) and `render_anim/src/minecraft_world.rs` (a player leaving the level) use the new name.
  - Friends open chests through the host's `act` and menus, so their openers reach the cats the same way. No protocol change.
- **Tests:**
  - Vendored `world::felines::tests::a_tame_cat_does_not_sit_on_an_open_chest_and_gets_up_when_it_is_opened`: `can_sit_on` refuses a chest with an opener and takes it with none; a tame cat placed on an open chest never sits on it in 300 ticks; on the shut chest it sits, and within 4 ticks of the chest being opened it stands up. `a_tame_cat_lies_down_on_a_bed` passes unchanged.
  - `server::tests::the_mob_world_reports_a_chest_s_openers`: the mob world reads 0, then 2 after `set_chest_openers`, 0 beside the chest, 0 after the count goes back, and 0 once the chest is broken.
  - `level::tests::a_double_trapped_chest_counts_on_both_halves` now expects the plain chest's count too, and `a_barrel_shows_open_until_its_player_leaves_the_world` checks that a barrel gives none.
- **Decisions:**
  - The test of the mob world is in `server::tests`, not `server_mobs::tests` as the brief named it, because the level it needs comes from the `door_world` helper there.
  - Only plain and trapped chests send their openers to the server's level, the two kinds something there reads (the cats and the redstone signal). Copper chests do not; copper golems would need them.
  - A sitting cat keeps its chest shut (commit 1), so nobody can open the chest under it. In game the re-check only makes a cat on its way give up when someone opens the chest first. The test steps check the visible part: a cat does not sit on a chest that is open.
- **Test steps (survival, by day, flat grass):**
  1. Tame a cat and make it stand (as in step 1 of the commit before). Place a chest 2 blocks from it, open the chest and stay in its screen: the cat does not climb onto it (watch for 20 seconds). Close it: within about 10 seconds the cat walks onto it and sits (**cc-3_open_chest**, the cat beside the open chest).
  2. **Friend window:** the friend opens a fresh chest near the host's standing cat and keeps the screen open while the host watches the cat: it does not sit on it. The friend closes it: within about 10 seconds the cat sits on it (**cc-4_friend_open_chest**, the host's view).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cat_spawn_egg*4,minecraft:cod*64,minecraft:chest*4,minecraft:cooked_beef*16
```

## Group fw-burst

### 1. Minecraft firework rockets carry their stars

After 2dc233c9.

- **Vanilla (26.3, checked with javap):**
  - `FireworkRocketEntity.dealExplosionDamage`: the strength is `5.0F + explosions.size() * 2`; with no stars it is 0 and nobody is hurt. Every other `LivingEntity` within 5 blocks (feet to rocket) whose feet or middle the rocket sees takes `strength * sqrt((5 - distance) / 5)`.
  - `ClientLevel.createFireworks`: with no stars the client adds `poof` particles at the rocket with motion `(gaussian * 0.05, 0.005, gaussian * 0.05)` while `i < random.nextInt(3) + 2`, the bound drawn again on every pass (so 2 to 4 puffs), and no sound and no `Starter`.
  - `minecraft:fireworks {flight_duration, explosions}` and `minecraft:firework_explosion {shape, colors, fade_colors, has_trail, has_twinkle}`; the shapes are `small_ball`, `large_ball`, `star`, `creeper` and `burst`; at most 256 explosions. `Codec.BOOL` reads a number as `byteValue() != 0`.
- **Changed:**
  - `minecraft_terrain/src/firework.rs`: `Shape`, `Explosion { shape, colors, fade_colors, trail, twinkle }` (`Explosion::from_json`, `Explosion::burst(color)` for a villager's one-colour BURST star) and `Fireworks { flight, explosions }`, read from a stack by `fireworks(stack)`. `has_trail` and `has_twinkle` may be a bool or a number; an explosion with an unknown shape is skipped.
  - `minecraft_terrain/src/firework_rocket.rs`: `Rocket` keeps `explosions` and `owner: RocketOwner { Mob, Player, Nobody }` (it was `Option<u64>` and the colours of one star). `Rocket::launch(owner, position, fireworks, random)`. `damage_to` uses `5 + 2 * stars` and returns `None` with no stars. `Burst` carries `explosions` instead of `colors`.
  - `minecraft_terrain/src/firework_particles.rs`: the burst takes its spark colours from all its stars (the drawing is unchanged until `fw-shapes`); new `FireworkParticles::poofs()`, the puff motions of a starless burst.
  - `minecraft_survival/src/entity_view.rs` (the firework loop in `bursts()`): a starless burst adds those `poof` particles and plays no bang.
  - `minecraft_terrain/src/server.rs`: a celebrating villager's rocket is `RocketOwner::Mob` with one `Explosion::burst` star; a rocket's `PlayerHit.source` is its mob owner.
  - The `Burst` sent to friends changed (see Protocol); no `PROTOCOL` bump.
- **Tests:**
  - `firework::tests::a_rocket_carries_the_stars_of_its_fireworks`: a plain rocket (flight 1, no stars), a two-star rocket read from JSON with a number trail and a bool twinkle, an unknown shape skipped, a star with no colours, a removed component, and the villager star.
  - `firework_rocket::tests::a_starless_rocket_hurts_no_one_and_each_star_adds_two`: no damage and no stars in the burst without stars, 7 at the centre with one star and 11 with three.
  - `firework_particles::tests::a_starless_burst_puffs_two_to_four_times_and_rises_slowly` (2, 3 and 4 puffs all happen over 200 bursts, each rising 0.005, with no sparks).
  - `entity_view::tests::a_starless_firework_burst_only_puffs` (no sound, no sparks, 2 to 4 ambient puffs).
  - `server::tests::a_villager_s_rocket_climbs_bursts_under_a_roof_and_hurts_who_is_near` and the other rocket and burst tests pass, with the stars in place of the colours.
- **Decisions:**
  - The puffs use the burst view's own random (vanilla uses the client level's random).
  - A starred burst still draws today's BURST sparks in all its stars' colours, and a star without colours still draws black, until group `fw-shapes`.
- **Test steps:** nothing new to see yet. The only rockets in game are those of villagers celebrating a won raid, and they look and hurt as before (one star). Starred and plain rockets from the kit come with group `fw-launch` (steps FW-1 to FW-4 of the brief).
- **Kit:** none.

### 2. Minecraft firework rockets burst on mobs and players in their path

After 077c2210.

- **Vanilla (26.3, checked with javap):**
  - `FireworkRocketEntity.tick` for a rocket that is not attached: first `Projectile.tick` (`checkLeftOwner`: the rocket's box swept by its last motion and grown by 1 must touch none of the owner's root vehicle and its passengers); then, unless shot at an angle, `motion *= (h, 1, h)` with `h = 1.0` after a `horizontalCollision`, else 1.15, and `+0.04` in y; then `ProjectileUtil.getHitResultOnMoveVector` **before** the move; then `move(SELF, motion)` with block collision (a 0.25 x 0.25 box) and the motion put back; then `hitTargetOrDeflectSelf(hit)`; then the launch sound on `life == 0`, `life++`, and `explode` when `life > lifetime`.
  - `getHitResult`: a COLLIDER clip (no fluids) along the motion; the entity search box is the rocket's box swept by the motion and grown by 1; each candidate's box is grown by `computeMargin` (`max(0, min(0.3, (tickCount - 2) / 20))`) and clipped from the position to the block hit or the end of the motion; an entity beats the block. `getEntityHitResult(Level, ...)` has no check for a start inside a box, and `AABB.clip` finds nothing from inside, so a rocket that starts inside a mob passes through it.
  - `Projectile.canHitEntity`: `canBeHitByProjectile` (alive and pickable: mobs, players, item frames) and, until the rocket has left its owner, not `owner.isPassengerOfSameVehicle(target)` (the owner and everything in its vehicle).
  - `onHitEntity` always explodes; `onHitBlock` explodes only with stars. The burst is at the rocket's position after the move.
- **Changed:**
  - `minecraft_terrain/src/firework_rocket.rs`: `Rocket::tick(world, targets, owner_box)` in that order, with `left_owner` and `horizontal_collision` kept on the rocket, `bounds_at`/`bounds` (`SIZE` 0.25), `margin` (from `life + 1`, the tick count) and the private `hit_on_move`. The move goes through the player crate's `collision::move_body` (no step). A starless rocket that meets a block keeps flying and bursts only at the end of its life; a starred one bursts there. A rocket bursts once even if a hit and the end of its life come in the same tick.
  - `minecraft_terrain/src/server.rs` `tick_rockets`: each rocket's targets are `thrown::targets` (every mob store), the item frames and `thrown::player_targets`. A villager owner's box is flagged as the owner and used for the left-owner check; a player owner (from group `fw-launch`) uses `thrown::owner_box` and `skip_own_mount`, as thrown items do.
- **Tests:**
  - `firework_rocket::tests`: `a_rocket_bursts_on_the_first_tick_at_a_zombie_just_above_it` (at the end of its move, and the launch sound still comes), `a_rocket_launched_inside_a_zombie_rises_through_it`, `a_rocket_passes_its_owner_until_it_has_left_it` (and a stranger in the same place is hit; a far owner is left at once), `a_starless_rocket_under_a_roof_waits_out_its_lifetime` (pinned with its box under the roof), `a_rocket_against_a_wall_stops_speeding_up_sideways` (and a starred one bursts on the wall).
  - `server::tests::a_villager_s_rocket_passes_its_villager_and_bursts_on_a_mob_or_a_player_in_its_path`: a villager's rocket rises through its own villager and bursts on a zombie hovering 4 blocks above (hurt once) before its time is up; a rocket rising into another villager bursts there on the second tick; one rising into a hovering player bursts at the player's feet with a hit of about 7 blamed on the villager.
  - `server::tests::a_villager_s_rocket_climbs_bursts_under_a_roof_and_hurts_who_is_near` now expects the burst at 203.75, where the rocket's box meets the roof at 204 (it was the point where the old point clip met it).
  - The other rocket tests pass with the new `tick` arguments.
- **Decisions:**
  - The brief's "a target whose box holds the start point is hit at once" is not in 26.3's projectile hit test (it is in the other `getEntityHitResult`, used for the player's pick). A rocket placed at a mob's feet rises through that mob, as in vanilla, so the brief's test "a rocket launched at a zombie's feet bursts on the first tick" became "a rocket just under a zombie bursts on the first tick" plus "a rocket inside a zombie rises through it". Owner step FW-5 (group `fw-launch`) should click the ground just in front of a zombie that walks over it, or use the dispenser of FW-9, which shoots the rocket into the mob from outside.
  - Item frames are rocket targets too (they are pickable in vanilla); the frame itself is not hurt. End crystals and the dragon's parts are not targets yet.
  - The left-owner box of a villager owner is its own box, without its vehicle.
- **Test steps:** none new in game yet: players can only launch rockets from group `fw-launch` on (FW-5, FW-6 and FW-9 then show this commit). A celebrating villager's rocket now bursts on a mob or player right above it instead of flying through.
- **Kit:** none.

### 3. Mobs hurt by a firework burst are knocked back

After 4947f42c.

- **Vanilla (26.3, checked with javap):** `dealExplosionDamage` hurts each mob with `damageSources().fireworks(rocket, owner)`. `minecraft:fireworks` is not in `#no_knockback`, so on a full hit (outside the damage cooldown, not blocked) `LivingEntity.hurtServer` calls `dealDefaultKnockback`: the direct entity is the rocket, a `Projectile`, whose `calculateHorizontalHurtKnockbackDirection` is `(mob.x - rocket.x, mob.z - rocket.z)`, negated into `knockback(0.4000000059604645, rocket.x - mob.x, rocket.z - mob.z)`: away from the burst, halving the motion, up by 0.4 on the ground, scaled by `1 - knockback_resistance`. `#is_explosion` and `#panic_causes` hold `fireworks`, so animals panic. The owner is the source's entity: a player gets the kill credit, and the mob remembers who hurt it.
- **Changed:**
  - Vendored `entities/src/world.rs`:
    - New public `FireworkOwner { Mob, Player, Nobody }` (with `mob()`), next to the private `Blaster`.
    - `hurt_by_blast` is now a wrapper of the new private `hurt_by_typed_blast(key, damage, push, blaster, kind)`, whose wolf, cat, trader, villager and wither arms take `kind` where they took `blaster.damage_type()`. Every other caller passes `blaster.damage_type()`, so explosions behave as before.
    - New public `firework_blast(owner, id, damage, at)`: the typed blast hurt with `minecraft:fireworks` and no explosion push; for a player owner `blasted_by_player`, the hoglin anger and `credit(.., "minecraft:player", true)`, as `player_blast` does; a mob owner is the `Blaster::Mob` attacker (wolves, cats, traders, villagers); on a full hit `knock_back(id, 0.4F, at.x - x, at.z - z)`, which already holds the iron golem's and shulker's immunity and the ravager's and hoglin's resistance.
  - `minecraft_terrain/src/firework_rocket.rs`: `RocketOwner` is now that vendored `FireworkOwner` (re-exported under the old name).
  - `minecraft_terrain/src/server.rs` `rocket_blast`: mobs are hurt through `firework_blast` with the rocket's owner instead of `player_blast(None, .., 0.0)` (which used `minecraft:explosion`, no push and no owner).
- **Tests:**
  - Vendored `world::combat::tests`: `a_firework_burst_knocks_mobs_away_from_it` (a grounded zombie gets -0.4 in x, away from a burst at +x, and 0.4 up; an iron golem is hurt but not moved; a ravager gets 0.1; an unknown id is `None`), `a_second_firework_burst_inside_the_cooldown_does_not_push` (the second, bigger hit lands without a push; a player's burst angers the zombie at the player), `a_firework_burst_makes_a_cow_panic_and_credits_a_player_with_the_kill` (the cow's last damage panics it and pushes it away; a zombie killed by a player's burst is the player's kill; a burst with no owner angers no one).
  - `server::tests::a_villager_s_rocket_passes_its_villager_and_bursts_on_a_mob_or_a_player_in_its_path` now also checks that the zombie the rocket bursts on is knocked back at 0.4.
  - All tests of the touched crates pass: minecraftoss-entities 433, minecraft_terrain 1429 (3 ignored), minecraft_survival 766 (+1 in its second test binary).
- **Decisions:**
  - The burst's knockback comes from the rocket's position, as vanilla's projectile direction does, and a mob hit by two bursts inside its damage cooldown is pushed only by the first, as in vanilla.
  - A villager owner angers only the mobs whose blast arms already take a mob attacker (wolves, cats, traders, villagers), as `Blaster::Mob` does for other explosions; zombies and the like do not turn on the villager.
  - **For the merge with D0:** D0's `EntityKey::Family` arm in `hurt_by_blast` calls `families::Hurt::new(damage, blaster.damage_type(), source)`. After the merge it sits in `hurt_by_typed_blast` and should pass `kind` instead, so the new families see `minecraft:fireworks` from a burst.
- **Test steps:** the knockback shows once players can launch rockets (group `fw-launch`, steps FW-5 to FW-9 of the brief). Until then there is nothing new to see in game.
- **Kit:** none.

## Group fw-launch

### 1. Minecraft firework rockets launch from a clicked block

After b299a298.

- **Vanilla (26.3, checked with javap):** `FireworkRocketItem.useOn` passes while the player is fall flying (the elytra boost is `use`). Otherwise the server spawns `new FireworkRocketEntity(level, player, click.x + face.stepX * 0.15, click.y + face.stepY * 0.15, click.z + face.stepZ * 0.15, stack)` (owned by the player) and shrinks the stack by one; a creative player keeps it (`ServerPlayerGameMode.useItemOn` restores the count). The rocket sounds its launch on its first tick. In adventure mode `ItemStack.useOn` passes for want of `may_build`.
- **Changed:**
  - `minecraft_terrain/src/firework.rs`: `RocketLaunch { at, fireworks }` and `rocket_place(player, world, held, gliding)`: the block the player aims at within 4.5 blocks, the rocket at the click point plus 0.15 times the clicked face, with the stack's `Fireworks`. It does nothing while gliding or without a rocket.
  - `minecraft_terrain/src/bucket.rs`: `Change::LaunchRocket(RocketLaunch)`, next to `Change::Rocket`.
  - `minecraft_survival/src/actions.rs`: `rocket_place` joins the item-on-block chain just before `rocket_use`; its arm calls `entities.launch_rocket`.
  - `minecraft_survival/src/entities.rs`: `launch_rocket(player, launch, sent)` (next to `throw_eye`) sends the launch and waits for the level's answer, so the stack shrinks and the arm swings when the rocket is in the world, for the host and for friends (whose uses run through the same `actions::act` on the host).
  - `minecraft_terrain/src/server.rs`: `Command::LaunchRocket { owner, launch }` (next to `ThrowEye`), `ServerHandle::launch_rocket`, and `ServerSim::launch_rocket`, which pushes `Rocket::launch(RocketOwner::Player(owner), ..)` with a random drawn from the level's sound random, and answers the item use.
  - No protocol change: the launch travels inside the host.
- **Tests:**
  - `firework::tests::a_rocket_clicked_on_a_block_launches_just_off_the_clicked_face`: 0.15 above the top of the clicked grass with the stack's stars and flight; 0.15 off the west face of a wall; nothing while gliding, with nothing in reach, or with another item.
  - `actions::tests::a_rocket_clicked_on_the_ground_leaves_the_stack_and_rises_from_the_click` (needs the 26.3 data): a survival press at the stone floor waits for the level, the answer leaves 63 rockets, and the rocket view climbs from 0.15 above the click.
- **Decisions:**
  - A rocket clicked at a mob's feet starts inside the mob and rises through it, as in vanilla (fw-burst, commit 2). The owner's step for a burst near a mob therefore uses a roof (FW-5); the dispenser of the next commit shoots rockets into mobs from outside.
  - The launch, like the other item-on-block uses, needs `may_build` (no rockets from adventure mode).
- **Test steps** (survival, by day, open flat ground; the starred rockets need E1's kit parser; until group `fw-shapes` every starred burst draws the current spark burst in all its stars' colours with the bang):
  1. **FW-1. A plain rocket.** Put the plain rockets in the hotbar and right-click the grass 3 blocks ahead. A rocket rises from just above the click with its launch whoosh and white trail, and after 1 to 1.5 s pops in 2 to 4 grey puffs with no bang. The stack shows 63. **Screenshot FW-1.**
  2. **FW-2. A starred rocket.** Place the red rocket the same way. It rises and bursts into red sparks with a bang. Place the two-star rocket: it climbs higher before it bursts. Place the three-star rocket: it bursts lower.
  3. **FW-3. Creative keeps it.** `/gamemode creative`, place a plain rocket: it flies and the stack stays at its count. `/gamemode survival`.
  4. **FW-4. Gliding still boosts.** With the elytra on, glide from `/tp ~ ~30 ~` and right-click while looking at the ground below: you boost, and no rocket rises from the ground.
  5. **FW-5. Bursts near mobs.** Build a 3x3 cobblestone roof 3 blocks above the ground on four pillars. Hatch a zombie under it, and when it is 2 blocks from you, right-click the ground under the roof next to it with the red rocket. The rocket meets the roof at once and bursts: the zombie flashes red and hops away from the burst, and you lose about two hearts (you are within 5 blocks). **Screenshot FW-5.**
  6. **FW-6. Animals.** Hatch 3 cows under the roof and burst the red rocket among them the same way: all three are hurt, pushed away from the burst and run in panic.
  7. **FW-7. An iron golem.** Hatch an iron golem under the roof and burst the red rocket next to it: it flashes red, does not move, and turns on you (a player's rocket blames the player). Run. **Screenshot FW-7.**
- **Kit:** the plain rockets, the three starred rockets of the brief, eggs and the roof.

```
IW4L_MINECRAFT_KIT=minecraft:firework_rocket*64,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*64,minecraft:firework_rocket[fireworks={flight_duration:2,explosions:[{shape:star,colors:[I;14602026],fade_colors:[I;2437522]},{shape:creeper,colors:[I;4312372],has_twinkle:1b}]}]*64,minecraft:firework_rocket[fireworks={flight_duration:0,explosions:[{shape:burst,colors:[I;15790320,11743532]},{shape:small_ball,colors:[I;2437522]},{shape:large_ball,colors:[I;14602026]}]}]*64,minecraft:zombie_spawn_egg*16,minecraft:cow_spawn_egg*8,minecraft:iron_golem_spawn_egg*2,minecraft:elytra,minecraft:cobblestone*64,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

### 2. Minecraft dispensers shoot firework rockets

After 2f658af5.

- **Vanilla (26.3, checked with javap):** `FireworkRocketItem` is a `ProjectileItem`, so the dispenser uses `ProjectileDispenseBehavior`: `asProjectile` makes `new FireworkRocketEntity(level, stack.copyWithCount(1), x, y, z, true)` (shot at an angle, no owner) at `getEntityJustOutsideOfBlockPos`, the block centre plus the facing times 0.5000099999997474; `Projectile.spawnProjectileUsingShoot` then calls `shoot(facing, power 0.5, uncertainty 1.0)`: `getMovementToShoot` is the facing normalised plus three `triangle(0, 0.0172275 * uncertainty)` draws, scaled by the power; the stack shrinks by one. `playSound` sends level event 1004 in place of the dispense click: `entity.firework_rocket.shoot`, volume 1.0, pitch 1.2, at the dispenser. In `FireworkRocketEntity.tick` a rocket shot at an angle skips the 1.15 speed-up and the 0.04 lift, so it flies straight.
- **Changed:**
  - Vendored `world/src/level/dispenser.rs`: `firework_rocket` left `OTHER_BEHAVIOURS`; its new branch `dispense_rocket` takes one rocket into a `DispensedRocket { at, facing, stack }` and plays the shoot sound (volume 1, pitch 1.2) at the dispenser's centre.
  - Vendored `world/src/level/mod.rs`: `Level::dispensed_rockets`, beside `unsupported`.
  - `minecraft_terrain/src/firework_rocket.rs`: `Rocket::shot(owner, position, direction, power, uncertainty, fireworks, random)` (the launch, then `getMovementToShoot`), the `shot_at_angle` flag, and `tick` skips the speed-up and the lift for such a rocket. `SHOT_SPREAD`, `DISPENSE_POWER` and `DISPENSE_UNCERTAINTY`.
  - `minecraft_terrain/src/server.rs`: `ServerSim::tick` drains the queue right after the level tick (`shoot_dispensed_rockets`): each rocket is `Rocket::shot` with no owner, power 0.5 and uncertainty 1.0, with the stars of the dispensed stack, so it flies in the same tick's rocket step.
- **Tests:**
  - `firework_rocket::tests::a_rocket_shot_at_an_angle_flies_straight_without_lift_or_speed_up`: an east shot at power 0.5 keeps its velocity for its whole life and ends where the straight line says.
  - `server::tests::a_dispenser_shoots_a_rocket_level_into_a_zombie_and_a_plain_one_puffs_at_the_end_of_its_life` (needs the 26.3 data): two east-facing dispensers powered by redstone blocks shoot one rocket each, with the shoot sound at their centres, and keep 3 of 4; the rockets start at x + 1.00001 with the stack's stars; the red one stays level and bursts on a zombie 6 blocks away within 15 ticks (hurt once, knocked back 0.4); the plain one flies its whole life and puffs with no stars.
- **Decisions:**
  - Only the shoot sound replaces the click: the dispensers in this port play no click and no smoke yet for any item.
  - A dispensed rocket has no owner, so its burst blames no one (`minecraft:fireworks` with no attacker: "went off with a bang").
  - The rocket stays a billboard, as the brief leaves the angled rocket model for later.
- **Test steps** (survival, by day, flat ground):
  1. **FW-8. A dispenser.** A placed dispenser faces the player who places it. Stand at one end of a flat 10-block lane with your back to the lane, look straight ahead (level) and place the dispenser on the ground in front of you: its hole faces you and the lane behind you. Put a lever on its side and 4 red rockets in it. Hatch a zombie 7 blocks down the lane, and stand beside the dispenser, not in front of it. Flip the lever on. A rocket shoots out level, with no arc, and a sharp shoot sound instead of the click. It bursts on the zombie, which flashes red and hops back. The dispenser holds 3 rockets. **Screenshot FW-8.**
  2. **FW-9. A plain rocket from the dispenser.** Take the red rockets out, put plain rockets in, and flip the lever off and on. The rocket flies level 10 to 15 blocks and puffs in grey smoke with no bang.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:firework_rocket*64,minecraft:firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]*64,minecraft:dispenser*4,minecraft:lever*4,minecraft:zombie_spawn_egg*16,minecraft:cooked_beef*32
```
