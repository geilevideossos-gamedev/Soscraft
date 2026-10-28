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
