# B1: throwables

Owner, screenshot 5: "Bottles o' enchanting can't be thrown with right-click. Check every vanilla throwable: XP bottles, snowballs, eggs, ender pearls, splash and lingering potions, eyes of ender, tridents."

## 1. What exists today

- **Why the bottle o' enchanting does nothing.** `Throwable::of` (`minecraft_terrain/src/thrown.rs:35-42`) only knows the snowball, the three eggs and the ender pearl. The right-click chain in `use_hands` (`render_anim/src/minecraft_world.rs:1013-1330`) tries, in order:
  1. `bottle::bottle_use` (`:1119`), which handles only `glass_bottle` and drinkable `potion`;
  2. buckets;
  3. spawn eggs;
  4. throwables (`:1181-1207`).

  Bottles o' enchanting, splash and lingering potions, eyes of ender, tridents and wind charges match none of these and end in `HandUse::Passed`. The offhand check, `minecraft_offhand::has_use` (`:92-100`), misses them too.
- **Throwing today.**
  - The client checks `hand.pearl_cooldown` (`:771-773`) and calls `Entities::throw` (`minecraft_entities.rs:269-276`), which sends `Command::Throw` (`server.rs:1120`). The client then takes one item off the stack itself.
  - `Thrown::throw` (`thrown.rs:102-131`) reuses the vendored `potion::ThrownPotion` with a dummy `Potion::Healing`. It has a fixed power of 1.5, no pitch offset and a constant owner `u64::MAX`.
  - `ServerSim::tick_thrown` (`server.rs:701-739`) sees only the first player's box, hatches eggs and records pearl landings without an owner. It sends `thrown_view` (`:697`) in the snapshot.
  - Thrown items are not saved.
  - `append_sprites` (`thrown.rs:420-461`) draws an untinted item billboard. Every item texture is already in the atlas (`mesh.rs:636`).
- **Witch potion code we can reuse.**
  - `minecraftoss_entities::potion` has `ThrownPotion`, plus `splash_scale` and `splashed_effect`, which match vanilla. Its `Potion` enum has only the nine witch potions, without long or strong variants.
  - `EntityWorld::splash` (`entities/src/world/witches.rs:406-442`) puts the effects on mobs and queues a `PlayerSplash`. `splash_instant` (`:525`) is private.
  - **Gap:** `Output.player_splashes` and `Output.potion_breaks` (`server.rs:1184-1186`) are never read. That is why witch potions don't reach the player and break silently.
- **Effects.** Mobs have nine `MobEffect`s (`effects.rs:17-27`) and the player has nine `EffectKind`s (`player/src/survival.rs:36-48`), plus instant heal and harm. `drink_potion` (`minecraft_survival.rs:543-597`) maps the potion ids by hand; swiftness, water breathing, night vision, strength and leaping do nothing on the player.
- **Missing pieces.**
  - Nothing calls the stronghold ring list (`Placement::ring_chunks`, `generator/src/structure/placement.rs:200`).
  - There is no lightning, no trident model and no hold-and-release use (only `FoodUse` and the shield's `Blocking`).
  - Orbs can't be thrown in a given direction (`Level::award_experience`, `world/src/level/entity.rs:1472`).
  - The kit (`crafting.rs:402-470`) and `/give` (`args.rs:248`) only take enchantments, so splash and lingering potions can't be given.

## 2. Vanilla 26.3 (javap)

All throws use `Projectile.spawnProjectileFromRotation(pitchOffset, power, inaccuracy)` and add the thrower's motion. The throw sound plays at volume 0.5 and pitch `0.4/(r*0.4+0.8)`.

- **Bottle o' enchanting.**
  - `ExperienceBottleItem.use` throws with (-20, 0.7, 1) and plays `entity.experience_bottle.throw`.
  - `ThrownExperienceBottle` falls with gravity 0.07. `onHit` fires level events 2002 (colour 0x385DC6) and 1053.
  - It then calls `ExperienceOrb.awardWithDirection(hit, dir, 3+nextInt(5)+nextInt(5))`. `dir` is the face normal of a struck block, or minus the motion for a struck mob.
  - `awardWithDirection` flips any orb throw that points against `dir` and starts each orb `dir*0.25` out.
- **Splash and lingering potions.**
  - `ThrowablePotionItem.use` throws with (-20, 0.5, 1), with `entity.splash_potion.throw` or `entity.lingering_potion.throw`.
  - `AbstractThrownPotion` falls with gravity 0.05. When it breaks, it fires 2007 and 1054 for an instant potion, otherwise 2002 and 1053.
  - Events 1053 and 1054 play `entity.splash_potion.break` (volume 1, pitch 0.9-1.0). Events 2002 and 2007 spawn 8 `splash_potion` item particles and 100 `effect`/`instant_effect` particles in the potion's colour.
- **Splash effects.**
  - `ThrownSplashPotion.onHitAsPotion` moves the potion's box to the hit point and inflates it by 4/2/4. Any mob or player box within a squared box distance below 16 gets the scale `1-d/4`.
  - Instant effects heal `(int)(scale*(4<<amp)+0.5)` or hurt by `6<<amp` with magic damage.
  - Other effects last `(int)(scale*duration*potion_duration_scale+0.5)` ticks, and are skipped when they would end within 20 ticks.
- **Water** (`#douses_fire` and the other water tags).
  - Fire, lit candles and lit campfires go out (event 1009) in the cell in front of the hit face, the struck block and the cell's 4 horizontal neighbours.
  - Within a centre distance of 4, burning mobs and players are put out, and water-sensitive mobs take 1 magic damage.
- **Lingering cloud.**
  - `ThrownLingeringPotion` leaves an `AreaEffectCloud`: radius 3, -0.5 radius per use, 600 ticks, a 10-tick wait and -0.005 radius per tick.
  - Every 5 ticks, mobs and players inside the radius get the effect at 0.25 duration, or 0.5 scale for instant effects. Each one then waits 20 ticks before it can be affected again.
  - The cloud is gone below radius 0.5. It has no renderer, only particles.
- **Eye of ender.**
  - `EnderEyeItem.use` returns PASS when looking at an `end_portal_frame`.
  - It calls `findNearestMapStructure(#eye_of_ender_located, pos, 100, false)`. Without a result it returns CONSUME and the item is kept.
  - Ring placement picks the nearest ring chunk by chunk centre and aims at that chunk's corner.
  - The eye spawns at feet+0.9 and plays `entity.ender_eye.launch` (volume 1, pitch lerp 0.33-0.5).
  - `signalTo`: a target more than 12 blocks away horizontally is replaced by a point 12 blocks toward it and 8 up.
  - Movement: the horizontal speed eases toward the distance by 0.0025, the vertical by 0.015.
  - After 80 ticks it plays `entity.ender_eye.death` and drops the eye if `nextInt(5)>0`, otherwise it fires event 2003.
- **Trident.**
  - `TridentItem`: a throw needs at least 10 ticks of charge, has power 2.5 and costs 1 durability. Use is refused when the next damage would break it, or with Riptide outside water and rain.
  - Riptide: a push of 1.5+0.75 per level and `startAutoSpinAttack(20, 8)`.
  - `ThrownTrident` hits for 8. It deflects with motion x(0.02, 0.2, 0.02), hits only once, and has a water inertia of 0.99.
  - Loyalty L brings it back after a hit or 4 ticks in the ground: `v = v*0.95 + dir*0.05*L`.
  - Channeling summons lightning in thunder.
- **Wind charge.** It throws with (0, 1.5, 1) and has a 0.5 s cooldown. It bursts with radius 1.2 and knockback x1.22, without breaking blocks.

## 3. Commits

1. **The Minecraft kit and /give take potion contents.**
   - `crafting.rs`: the kit takes `id@potion`. `#` would start a `.env` comment.
   - `args.rs`: `/give` takes `potion_contents={potion:"minecraft:x"}`.
2. **Minecraft thrown items remember their thrower and their vanilla angle, power and gravity.**
   - `Throwable::{pitch_offset, power, gravity}`, and `Thrown.owner` replaces the `u64::MAX` constant.
   - `tick_thrown` takes every player's box, and each pearl landing carries its owner.
3. **Minecraft bottles o' enchanting are thrown and burst into experience like vanilla.**
   - `Throwable::ExperienceBottle` and `thrown::bottle_experience`.
   - Vendored `PotionHit.face`, and `Level::award_experience_toward`.
   - The break is queued in `potion_breaks`.
4. **Minecraft potions break with the vanilla glass sound and swirl.**
   - `Entities` reads `potion_breaks` and plays the sound.
   - A crumb burst from `BlockParticles`.
   - A new `spell_particles.rs` for the swirl.
   - Witch potions get the sound and swirl too.
5. **Minecraft splash potions are thrown and splash their effects like vanilla.**
   - A new `potions.rs` holds the 26.3 `Potions` table, and `drink_potion` uses it.
   - Vendored `EntityWorld::splash_effects` for mobs.
   - `Output.potion_splashes` feeds `MinecraftSurvival::splash_potion`, and witch `player_splashes` go the same way.
   - A tinted bottle sprite through `witch_render::append_potions`.
6. **Minecraft splash water bottles put out fire, campfires, candles and burning mobs and players like vanilla.** Uses `thrown::douse_cells`.
7. **Minecraft lingering potions leave a shrinking effect cloud like vanilla.** A new `effect_cloud.rs` holds `ServerSim.clouds`.
8. **Minecraft eyes of ender fly toward the nearest stronghold like vanilla.**
   - Vendored generator `nearest_ring_structure`.
   - The eye's flight, drop and break go in `thrown.rs`, with a portal particle trail.
   - With no stronghold the item is kept, which covers the Nether and the End.
9. **Minecraft tridents are drawn in the hand and in flight.** `special_models.rs`.
10. **Minecraft tridents charge and throw like vanilla.**
    - A `Charging` state beside `Blocking`; release is handled near `minecraft_world.rs:867`.
    - The flight reuses the arrow sim, `projectile::Arrow`.
    - The trident is saved, and only its owner can pick it up.
11. **Minecraft tridents with Loyalty return to their thrower.**
12. **Minecraft tridents with Riptide launch their thrower in water or rain.** Needs a pmove impulse in the MW2 player state.
13. **Minecraft wind charges burst and knock back like vanilla.** Cooldowns become per item.

Commits 1-4 fix screenshot 5.

## 4. Unit tests

- `thrown.rs`: `every_vanilla_throwable_is_recognised`, `a_bottle_o_enchanting_leaves_20_degrees_up_at_0_7`, `bottle_experience_is_3_to_11`, `water_douses_the_face_cell_struck_block_and_four_sides`.
- `entity.rs`: `directed_orbs_never_fly_into_the_face`. `server.rs`: `a_bottle_on_stone_leaves_orbs_worth_3_to_11`, `a_splash_two_blocks_off_gives_half_the_poison`.
- `potions.rs`: `strong_poison_is_432_ticks_level_2`. `effect_cloud.rs`: `the_cloud_is_gone_by_tick_610`, `a_victim_waits_20_ticks`.
- Eye: `a_far_target_aims_12_out_and_8_up`, `four_eyes_in_five_drop`, `no_stronghold_keeps_the_item`. Trident: `released_before_10_ticks_does_not_throw`, `loyalty_pulls_back`.
- `kit_gives_potion_contents`, `give_takes_potion_contents`.

## 5. In-game test steps

```
IW4L_MINECRAFT_KIT=experience_bottle*64,snowball*16,egg*16,ender_pearl*16,splash_potion@harming,splash_potion@strong_healing,splash_potion@poison,splash_potion@slowness,splash_potion@water,lingering_potion@poison,lingering_potion@regeneration,ender_eye*16,trident+loyalty=3,trident+riptide=3,trident,wind_charge*64,campfire,candle,flint_and_steel,zombie_spawn_egg*4,cow_spawn_egg*4
```

1. **Bottle o' enchanting at the ground.** It arcs up, breaks with the glass sound and a blue swirl, and gives 3-11 XP. Take screenshots of the XP bar before and after. Repeat from the offhand.
2. **Bottle at a wall.** The orbs come out of the wall's face.
3. **Snowball, egg and ender pearl.** They behave as before.
4. **Splash harming.** Throw it at a cow 1 block away and at one 3 blocks away: the far one takes less damage. On a zombie it heals.
5. **Splash poison and healing.** Poison at your own feet poisons you, less when thrown 3 blocks away. Strong healing heals you while you are hurt.
6. **Splash water.** Light a campfire, a candle and fire with flint and steel, then throw water: all three go out. It also puts out your own flames.
7. **Lingering poison.** The cloud shrinks over about 30 s. While you stand in it, you are poisoned again about once a second.
8. **Eye of ender.** It flies one way, sinks after about 4 s and usually drops. Compare the direction on the minimap after throwing from 3 places.
9. **Trident.** Hold right-click for 1 s and release: it hits a zombie, and the plain trident sticks in the ground and can be picked up. The Loyalty trident comes back. The Riptide trident launches you when you stand in water.
10. **Creative.** Bottles and tridents are not used up.
11. **Wind charge.** It bursts and pushes cows.

## 6. Risks and overlaps

- **Crate split.** The new state moves to `minecraft_survival`: `Thrown.owner`, clouds, eyes, `potions.rs` and splash handling. Today the client decides the throw and takes the item (`minecraft_world.rs:1181`); in host-authoritative play the server must do both.
- **Multiplayer.** The thrown snapshot is not tracked per player. Breaks, splashes and pearl landings must go through `minecraft_net`, keyed by player UUID.
- **The End track.** The stronghold locate is roadmap 1.5's first step, so it goes in the generator for reuse. Putting eyes into frames stays in that track. The locate must use the generator of the dimension the player is in.
- **Effects.** The player has no speed, water breathing, night vision or strength, so those splashes do nothing. `/effect` has the same gap.
- **Weather and movement.** There is no lightning yet, so Channeling has to wait. Riptide and the wind charge need an impulse in MW2 movement.
- **Saving.** Thrown items and clouds are not saved; tridents are, in commit 10.
- **Vendored patches.** `potion.rs`, `witches.rs`, `level/entity.rs` and the generator change.
