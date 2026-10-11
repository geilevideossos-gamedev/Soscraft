# Track C log: shulkers, end cities, end sky and dragon effects, elytra bodies, open limitations

Branch `track-c`, worktree `C:\Dev\mc-wt\track-c`. Briefs: `c1-shulkers.md`, `c5-limitations.md`, `c2-end-cities.md`, `c3-end-sky-dragon-fx.md`, `c4-elytra-body.md`. Order: c1 groups 1-3, c5 groups 1-4, c1 groups 4-8, c2, c3, c5 groups 5-11, c4. Base: 0bcbfb17.

A commit can't name its own hash, so each entry names the previous commit's hash; the last entry's hash is in `git log` under its subject.

## Group c1-levitation: levitation and slow falling (c1 groups 1-3)

### 1. Minecraft players and mobs can have levitation and slow falling

Previous commit: 0bcbfb17

- **Changed:**
  - `minecraftoss-player` `survival.rs`: `EffectKind::Levitation` and `EffectKind::SlowFalling`, appended after `Wither` so the indices of the older kinds don't move; the effect array holds 12. Both count down in `tick_effects` with no work of their own.
  - `minecraft_survival`: `/effect give` and `/effect clear` know `minecraft:levitation` and `minecraft:slow_falling` (`commands.rs`), `clear_effects` removes them (so milk and the totem clear them, `survival.rs`), and the player file saves and reads them under their vanilla ids (`player_nbt.rs`).
  - `minecraft_terrain` `potions.rs`: `player_effect` maps `levitation` and `slow_falling`, so drinking or being splashed by a slow falling potion now gives the effect (before, it did nothing).
  - `minecraftoss-entities` `effects.rs`: `MobEffect::Levitation` and `MobEffect::SlowFalling` with their ids, in `ALL`, with no per-tick work. Splash and lingering potions of slow falling now give mobs the effect, and a mob's `active_effects` keep both through a save. `potion.rs` gives both their vanilla colours (`levitation` 13565951, `slow_falling` 15978425) for the witch potion match.
- **Tests:**
  - `commands`: `levitation_and_slow_falling_are_given_and_cleared_by_command` (`/effect give @s minecraft:levitation 10 1` gives amplifier 1 for 200 ticks, slow falling 30 s gives 600 ticks, `/effect clear @s` removes both).
  - `survival`: `slow_falling_potions_last_like_vanilla_and_a_totem_clears_levitation` (long slow falling 4800 ticks, a shorter one doesn't cut it, the totem clears levitation and slow falling, levitation counts down and ends).
  - `player_nbt`: `levitation_and_slow_falling_are_saved_and_read_back`.
  - `minecraft_terrain` `server`: `levitation_and_slow_falling_on_a_mob_round_trip_through_its_save`.
- **Decisions:**
  - The new kinds are appended at the end of both enums. Tracks A and B also append kinds (haste, mining fatigue, conduit power, nausea); whoever merges second keeps every kind and grows the arrays to match.
  - Nothing moves yet: the effects only exist, count down, save and clear. The movement comes in the next two commits.
- **Test steps:** (any world, survival, open ground)
  1. `/effect give @s minecraft:levitation 10 1`, then quit to the menu and load the world again within 10 seconds, then `/effect clear @s`: no error, the command is accepted each time.
  2. Drink the slow falling potion, quit and load: the world loads normally (the effect is in the player file; you can't see it yet, there are no effect icons).
  3. Throw the splash slow falling potion at a zombie: it is accepted like any splash potion.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:potion@slow_falling,minecraft:potion@long_slow_falling,minecraft:splash_potion@slow_falling,minecraft:zombie_spawn_egg*2
```

### 2. Minecraft levitation lifts players and slow falling slows their fall

Previous commit: 53ecf14a

- **Changed:**
  - `playerstate_iw4`: a new `mc_float` field (bits 0-15 the levitation level, amplifier + 1, 0 for none; bit 16 slow falling), replicated and exact in the PS netfields after `mc_rocket_time`. No `pm_flags` bit is used.
  - `sim`: `set_float(id, levitation, slow_falling)` on the frame and the carrier, like `set_may_glide`.
  - `movement_iw4/src/float.rs` (new): the closed forms of vanilla's per-tick vertical steps, `v' = 0.98F * (0.8 v + 0.01 L)` for levitation and `v' = 0.98F * (v - 0.01)` for slow falling, over fractional ticks. The distance moved in a frame is the exact sum of vanilla's tick moves (each tick moves with the old speed, then the speed changes), which adds up across frames, so the path doesn't depend on the frame rate.
  - `single.rs`: with levitation the walk branch is replaced by the air branch, after leaving the ground (ground entity, walking and ground plane cleared); the climbable branch leaves the ground too. Water, lava, creative flight, cobwebs and noclip ignore both effects.
  - `air.rs`: the air move hands its slide to `float::air_slide`, which with levitation (or slow falling while not rising) sets the vertical speed from the closed form and slides with no MW2 gravity; otherwise the old MW2 gravity slide runs unchanged. Horizontal stays MW2 air control.
  - `climb.rs`: on a climbable, levitation runs the closed form after the move, and a climb up (jump or pushing into the climbable) gives one tick of levitation from 0.2, as vanilla's `handleOnClimbable` and `travelInAir` do.
  - `glide.rs`: levitation stops and forbids gliding (`canGlide`), and slow falling lowers the glide's gravity to 0.01 while not rising.
  - `render_anim/src/minecraft_elytra.rs` feeds `set_float` each frame from the player's own `survival.status` (per player), and `may glide` is false while levitating. The fall peak restarts at the feet while either effect is on (`minecraft_survival::elytra::float_peak`, `floating`), also in the fall block of `minecraft_world.rs`, like vanilla's `resetFallDistance`.
- **Tests:**
  - `movement_iw4` `float`: `the_closed_form_matches_vanilla_tick_by_tick` (8.864 and 44.32 blocks in 200 ticks, 32.67 units a second per level, 1.656 blocks of slow fall in 20 ticks, terminal 352.8 units a second), `short_frames_add_up_to_whole_ticks`, `the_float_field_keeps_the_level_and_slow_falling_apart`.
  - `movement_iw4` `glide`: `levitation_stops_the_glide_and_slow_falling_sinks_it_with_gravity_0_01`.
  - `sim` `voxel` (full pmove): `levitation_lifts_a_player_off_the_floor_like_vanilla_at_any_frame_rate` (50 ms frames within 0.01 block, 16 ms frames within 0.1, steady 32.67 units a second), `a_levitating_player_leaves_the_floor_at_once_and_a_ceiling_stops_the_rise`, `slow_falling_sinks_at_most_ten_blocks_a_second_and_leaves_a_jump_alone`, `levitation_wins_over_slow_falling_and_does_nothing_in_water_or_flight`.
  - `sim` `frame`: `levitation_and_slow_falling_reach_the_player_state`. `net` `delta`: `the_float_field_reaches_the_client`.
  - `minecraft_survival` `elytra`: `floating_restarts_the_fall_and_a_fall_after_it_hurts_from_where_it_ended`.
- **Decisions:**
  - Vanilla's order is kept: a tick moves with the old speed and then changes it. From rest the first tick doesn't move, so a levitating player leaves the floor on the second tick (0.35 units up, past Q3's 0.25 ground trace). At frame rates above 20 the first frames can come out a hair below zero; the move is clamped so a rising player never moves down.
  - The float moves skip MW2's end-of-frame velocity rounding (like swimming, climbing and gliding already do). Rounding to whole units a second would eat the small early speeds at high frame rates and stall the rise.
  - A move blocked by a ceiling or the floor keeps the slide's clipped speed (about 0), like vanilla's collision; the next frame starts the formula from there.
  - Levitating on the ground uses the air move, so a jump does nothing while levitating (vanilla could still jump). Slow falling on a climbable keeps the climb's own fall (capped at 0.15 a tick), so it doesn't slow a ladder slide further. Both differences are small and recorded here.
  - `PlayerState` grew a field: host and friends must run the same build.
- **Test steps:** (survival, `/difficulty normal`, open ground, armour off)
  1. `/effect give @s minecraft:levitation 10 0`: you float straight up for 10 seconds and can strafe while floating; you end about 9 blocks up (**1**). Then you fall and lose about 3 hearts (**2**).
  2. `/effect give @s minecraft:levitation 10 0` again, then drink the milk while floating: you fall at once.
  3. `/effect give @s minecraft:levitation 5 4`: you rise fast, about 4.5 blocks a second (**3**). `/effect clear @s`.
  4. Build a 20-block purpur tower, drink the slow falling potion and jump off: you sink gently (about 10 blocks a second at most) and take no damage (**4**). Jumping on the ground still goes as high as before. While slow falling, `/effect give @s minecraft:levitation 3 0`: you rise (levitation wins).
  5. Equip the elytra, jump off the tower and glide. `/effect give @s minecraft:levitation 3 0`: the glide stops and you rise (**5**). Drink the long slow falling potion and glide again: the glide sinks more slowly.
  6. Pour the water buckets into a 2-deep hole, stand in it and `/effect give @s minecraft:levitation 5 0`: you don't rise. Place a ladder column on the tower and stand at it: you float up the ladder.
  7. `/gamemode creative`, fly, `/effect give @s minecraft:levitation 5 0`: flight is unchanged. `/gamemode survival`.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:milk_bucket,minecraft:milk_bucket,minecraft:potion@slow_falling,minecraft:potion@long_slow_falling,minecraft:elytra,minecraft:firework_rocket*16,minecraft:purpur_block*64,minecraft:ladder*16,minecraft:water_bucket,minecraft:water_bucket,minecraft:cooked_beef*16
```

### 3. Minecraft mobs float with levitation and fall slowly with slow falling

Previous commit: 1aeb79a9

- **Changed:**
  - `minecraftoss-entities` `movement.rs`: `Body.levitation` (the amplifier) and `Body.slow_falling`, set by `Body::float_from(&MobEffects)`. `travel_air_jumping` (which `travel_air` also runs) resets the fall distance while either is set (`LivingEntity.aiStep` resets it just before `travel`, checked with javap), and its vertical step is vanilla's `travelInAir`: with levitation `vy += (0.05 (a + 1) - vy) * 0.2`, else with slow falling and `vy <= 0` the gravity is at most 0.01, then `vy *= 0.98F`.
  - `world.rs` and `world/*.rs`: every mob kind with effects (bats, zombies, skeletons, creepers, spiders, chickens, pigs, sheep, cows and their kin, blazes, endermen, ghasts, iron golems, slimes, villagers, witches, wolves) calls `float_from` right after its effects tick, so the flags follow the effects each tick.
- **Tests:** `world`: `levitation_lifts_a_zombie_and_slow_falling_lets_it_down_unhurt` (8.864 blocks in 200 ticks, slow fall never faster than 0.49 a tick, lands from 300 blocks with full health, while a plain fall from 29 blocks hurts).
- **Decisions:**
  - Mobs that move with `travel_flying` keep that travel, which ignores both effects, as vanilla's `FlyingMob.travel` does; every mob that moves with `travel_air` or `travel_air_jumping` floats.
  - Nothing gives mobs levitation in survival yet except `/effect` (not wired to mobs) and shulker bullets (group 6 of c1); splash and lingering slow falling potions do reach mobs now.
- **Test steps:**
  1. Stand on a 6-block tower edge with zombies below and around it. Throw the splash slow falling potion at zombies on the tower, then push or lure one off: it sinks gently and lands unhurt (**6**).
  2. Spawn a zombie at the top of the tower and let it walk off without the potion: it lands hurt (red flash).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:splash_potion@slow_falling,minecraft:splash_potion@slow_falling,minecraft:zombie_spawn_egg*4,minecraft:cobblestone*64,minecraft:diamond_sword
```

### Group c1-levitation, combined kit

```
IW4L_MINECRAFT_KIT=minecraft:milk_bucket,minecraft:milk_bucket,minecraft:potion@slow_falling,minecraft:potion@long_slow_falling,minecraft:splash_potion@slow_falling,minecraft:splash_potion@slow_falling,minecraft:elytra,minecraft:firework_rocket*16,minecraft:purpur_block*64,minecraft:ladder*16,minecraft:water_bucket,minecraft:water_bucket,minecraft:cooked_beef*16,minecraft:zombie_spawn_egg*4,minecraft:cobblestone*64,minecraft:diamond_sword
```

## Group c5-snow-ladder: ladder ground jump, powder snow, freezing, overlays (c5 groups 1-4)

### 4. Minecraft ladders take a ground jump like vanilla

Previous commit: e1ab7780

- **Changed:** `movement_iw4` `climb.rs`: on the ground at a climbable, `climb_move` runs the MW2 jump gate the way `stuck_move` does (a stale JUMPING flag is cleared, `jump::check` with the 500 ms gate and a fresh press, `ps.jump_time` stamped at the frame start). For the 50 ms after the launch the vertical speed is vanilla's jump power, 0.42 blocks a tick, and a sprinting player gets the 0.2 push along the facing (which the climb's ±0.15 clamp then caps, as vanilla's `handleOnClimbable` does). After the launch tick the speed is the climb rise, 0.1176 a tick, as vanilla sets 0.2 when the jump key was held at the end of that tick. The override only runs while the JUMPING flag is set, so an old `jump_time` alone never lifts anyone.
- **Tests:** `movement_iw4` `climb`: `a_ground_jump_at_a_ladder_rises_the_vanilla_first_tick` (0.42 in the first 50 ms, then 0.1176 a tick while held, at 20, 125 and 333 frames a second, within 1 %), `a_tap_at_a_ladder_lifts_about_half_a_block` (peak 0.574 at 50 ms frames, between 0.5 and 0.6 at 16, 8 and 3 ms; the player slides back down), `the_jump_waits_for_the_gate` (a jump 200 ms after the last one, or a jump key held from before, only climbs: 0 the first tick, 0.1176 the next; a sprint jump rises 0.42 and moves at most the climb speed sideways).
- **Decisions:**
  - The coast after a tap uses the climb's existing per-frame gravity, which matches vanilla tick for tick at 20 frames a second and follows a smooth curve at higher rates, so the hop peaks a little lower there (about 0.51 to 0.56 blocks instead of 0.57). The ladder slide's speed is unchanged.
  - The MW2 jump gate stays: a second ground jump needs a fresh press and 500 ms, like every other MW2 jump in the port (vanilla's 10-tick delay applies only while the key is held on the ground, and a ladder lifts the player off the ground at once).
- **Test steps:** (survival, flat ground)
  1. Put a ladder on a wall 6 blocks high. Stand at its foot and tap jump once: you hop up about half a block and slide back down. Before this round a tap only lifted you a hand's width (**1**, mid-hop).
  2. Hold jump at the foot: the first step up is a quick hop, then the steady climb, the same speed as before.
  3. Sprint into the ladder and jump: the same hop, no faster sideways.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ladder*32,minecraft:oak_planks*64
```

### 5. Minecraft powder snow holds, sinks and slows players, and leather boots walk on it

Previous commit: 5de91941

- **Changed:**
  - `sim` `voxel.rs`: a new shape flag `POWDER_SNOW` (`0x8000`, the last free bit; track D takes `0x7800`), set by `render_anim`'s `shape_id` for `minecraft:powder_snow`. Its shape depends on the body moving (`Mover`), as vanilla's `getCollisionShape` does for an entity: a box 0.9 high while falling faster than a 2.5-block free fall under the pmove's gravity (`Mover.falling`), a full block for a powder snow walker whose feet are above the block and who isn't sneaking, else nothing. Bullets, projectiles and other traces without a mover pass through. `stuck_in` holds a body whose feet cell is powder snow with vanilla's `(0.9, 1.5, 0.9)` multipliers, after cobwebs and berry bushes.
  - `playerstate_iw4`: `pm_flags::MC_POWDER_WALKER` (`0x0080_0000`, free in the whole workspace; B3's `MC_RIDING` is `0x0100_0000`). `sim` `set_powder_snow_walker` on the frame and the carrier; `render_anim` sets it every frame from the boots slot (`walks_on_powder_snow`: `minecraft:leather_boots` in slot 36, from that player's own inventory).
  - Climbing out: `ClimbSample.powder`, set when the feet cell is powder snow and the mover is a walker (`voxel::climb_as` with the pmove's mover; `voxel::climb` without one stays for the footsteps). It counts as climbing, so a walker rises 0.2 a tick (scaled by the stuck 1.5) on jump or when pushing into a wall, as vanilla's `handleRelativeFrictionAndCalculateMovement` does, and the ladder clamp (`hold`) is skipped, so sneaking doesn't hold a walker in place.
  - Vendored `inside_blocks.rs`: powder snow gives `Stuck((0.9F, 1.5, 0.9F))` only when it is the body's feet block (`getInBlockState`), then the extinguish as before. Mobs now sink slowly in powder snow too (they had no collision with it already).
  - Landing: a fall of 4 blocks or more into powder snow, or onto it in leather boots, plays `entity.player.small_fall` (under 7) or `big_fall`, with no damage (`powder_snow_landing`).
  - Snowflakes: `ambient_particles::Kind::Snowflake` (gravity 0.225, friction 1, motion jitter ±0.05, size 0.1 × (1 + r²), lifetime 16 / (0.2 + 0.8 r) + 2, `generic_7` to `generic_0` by age, opaque, motion × (0.95, 0.9, 0.95) each tick). Each tick the player moved sideways with the feet in powder snow, every powder snow block the body is in (`powder_snow_flakes`) gets a coin flip for one flake at its top, with motion (±1/12 r, 0.05, ±1/12 r) (`Entities::powder_snow_flake`).
- **Tests:**
  - `sim` `voxel`: `powder_snow_catches_a_fall_at_nine_tenths`, `leather_boots_stand_on_powder_snow_until_sneaking`, `powder_snow_holds_only_through_the_feet_cell`, `powder_snow_needs_no_collision_without_a_mover`; `frame`: `leather_boots_mark_a_powder_snow_walker`.
  - `movement_iw4` `stuck`: `powder_snow_sinks_at_the_vanilla_rate` (0.1176 a tick at 20, 40 and 100 frames a second, sideways × 0.9), `a_walker_climbs_out_of_powder_snow_at_two_tenths` (0.1764 a tick with the stuck scale, also when pushing into a wall; a sneaking walker sinks).
  - Vendored `inside_blocks`: `powder_snow_sticks_only_by_the_feet`.
  - `minecraft_terrain` `ambient_particles`: `snowflakes_live_and_slow_like_vanilla`.
  - `minecraft_survival` `survival`: `leather_boots_walk_on_powder_snow_and_landing_in_it_sounds_from_four_blocks`, `snowflakes_come_from_each_snow_block_the_body_is_in_while_the_feet_are`.
- **Decisions:**
  - The falling shape uses the fall speed (faster than a 2.5-block drop from rest), not vanilla's fall distance; a fall slowed by water or webs just before won't be caught.
  - The walker bit is a `pm_flags` bit so the client's prediction sees it; friends will need the same build.
  - Snowflakes are made on the player's own client from its own movement; vanilla sends them from the server to everyone near **[E]**.
- **Test steps:** (survival, flat ground)
  1. Dig a pit 1 × 1 and 3 deep and fill it with the three powder snow buckets. Scoop the top block back up with an empty bucket and pour it back in.
  2. Walk into the pit: you sink slowly, about 2 blocks a second, and moving is a little slower. Small white flakes rise when you move inside (**2**, inside looking out).
  3. Without boots, jump: you can't climb out of 3 deep. Get out by scooping the blocks up and pillaring with cobblestone, then pour them back.
  4. Put on the leather boots: walk over the powder snow as if it were solid (**4**). Crouch: you sink in. Inside with boots, hold jump or walk into the pit wall: you climb out at a steady pace.
  5. Take the boots off, `/tp ~ ~12 ~` above the pit: you land on the snow surface with a fall sound and no damage, then sink in.
  6. Shoot an MW2 gun through powder snow: the bullets pass through it.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:bucket,minecraft:leather_boots,minecraft:cobblestone*64
```

### 6. Minecraft powder snow freezes players like vanilla: frost, damage and melting

Previous commit: 6b565157

- **Changed:**
  - Vendored `inside_blocks.rs`: `Hazard::Freeze` (`minecraft:freeze`, through armour, not fire, no panic), `BlockEffect::Freeze` and `BlockEffect::MeltIfBurning(pos)`. `Kind::Freeze` is first in the apply order, as `InsideBlockEffectType` declares it. The collector gained a `before` list, flushed ahead of its kind (vanilla's `runBefore`). Every powder snow block the body is in adds the melt before the extinguish, then freeze and extinguish. Mobs ignore both new effects (`world/hazards.rs`).
  - `minecraft_survival` `survival.rs`: `Survival` keeps `ticks_frozen` and `in_powder_snow` per player. Freeze adds 1 (up to 140) unless the player can't freeze; ClearFreeze (fire, lava) sets 0; after the effects the frost melts by 2 unless the player is in powder snow and can freeze. Every 40th tick while at 140 it deals 1 `minecraft:freeze` damage through armour with `entity.player.hurt_freeze` and no exhaustion, unless the new `freeze_damage` rule (`SurvivalRules.freeze_damage`, read from the game rules) is off. The extinguish hiss also plays when the frost was cleared during the effects. A player can't freeze while wearing any leather armour piece (`freeze_immune`: boots, leggings, chestplate, helmet or horse armour in slots 36-39 and 41, vanilla's `#freeze_immune_wearables`) or in spectator. A burning player melts every powder snow block it is in (`take_melted`); `render_anim` writes air there through the same block-write path as Frost Walker and shows the block's break particles and sound (`Entities.destroyed`). `Vitals` gains `frozen` (percent) and `fully_frozen`.
  - `death.rs`: `minecraft:freeze` with the message id `freeze` ("%1$s froze to death", ".player": "%1$s was frozen to death by %2$s").
  - `player_nbt.rs`: `TicksFrozen` (int) is owned, written only above 0, read with default 0; `Survival::restore` takes it and respawning clears it.
  - Frost slowdown: `sim` keeps a per-client Minecraft move scale (`set_minecraft_move_scale`, on the client's match state, default 1) and feeds it to `WalkMoveContext.weapon_move_scale`, which was always 1 before. MW2 walking and climbing already scale by it, and `stuck_move` now scales its ground walk and sprint speeds by it (air speed stays, as vanilla's flying speed isn't an attribute). `render_anim` sets 1 - 0.5 × percent frozen while frozen with a block 0.2 below the feet (`frost_move_scale`), else 1.
- **Tests:**
  - `minecraft_survival` `survival`: `seven_seconds_in_powder_snow_freeze_a_player`, `frozen_players_take_one_every_two_seconds` (none before tick 160, 4 in 280 ticks, through iron armour), `thawing_takes_two_ticks_per_tick` (quietly), `leather_armour_keeps_a_player_from_freezing`, `the_freeze_damage_rule_stops_the_damage`, `fire_clears_the_frost_with_the_extinguish_sound`, `a_burning_player_melts_the_powder_snow_it_enters`.
  - `death`: `freezing_to_death_has_the_vanilla_message`. `player_nbt`: `ticks_frozen_round_trips_and_is_left_out_at_zero`.
  - `sim` `frame`: `frost_sets_a_minecraft_move_scale_per_client`.
  - `movement_iw4` `stuck`: `frost_slows_the_walk_but_not_the_drift` (half speed on the ground at a move scale of 0.5, the air drift unchanged).
  - Vendored `inside_blocks`: `powder_snow_freezes_before_it_extinguishes` (freeze, clear, ignite, fire hurt, melt, extinguish); `powder_snow_sticks_only_by_the_feet` now also expects the freeze and the melt.
- **Decisions:**
  - The move scale is not replicated, so a joined friend's client would mispredict a frozen player's speed until track E sends it **[E]**. It lives next to the client's other per-client state in `sim`, not in `PlayerState`, and leaves `ps.move_speed_scale_multiplier` alone because MW2's `_weapons.gsc` writes that one.
  - Freezing and frost are ticked from `render_anim` for the local player, like the other hazards **[E]**.
  - A burning mob doesn't melt powder snow (vanilla does with `mob_griefing`), and mobs don't freeze.
- **Test steps:** (survival, `/difficulty normal`; the same pit as in commit 5)
  1. Stand inside the 3-deep powder snow pit, head in the snow, armour off: after about 8 seconds you start taking half a heart every 2 seconds with a crackling hurt sound (the frost overlay and frozen hearts come in the next commit).
  2. Stay until the frost kills you: the chat says "<name> froze to death". Respawn, `/gamerule freeze_damage false` and go back in: no damage. `/gamerule freeze_damage true`.
  3. Get out (scoop the blocks up and pillar out): for about 3.5 seconds walking is slower, half speed right after full frost.
  4. Put on the leather helmet only and stand in the snow again: no damage, no slowdown afterwards.
  5. Set yourself on fire (place netherrack, light it with flint and steel, step in) and jump into the powder snow: the fire goes out with a hiss and the snow block you are in melts away with its break sound.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:bucket,minecraft:leather_helmet,minecraft:flint_and_steel,minecraft:netherrack*16,minecraft:cobblestone*64
```

### 7. Frozen Minecraft players see the powder snow outline, frozen hearts and powder snow fog

Previous commit: 2795707e

- **Changed:**
  - `frame`: `McVitals` gains `frozen` (percent frozen) and `fully_frozen`, filled from the player's own `Vitals`; `MinecraftUi.freeze_outline` holds `textures/misc/powder_snow_outline.png`, loaded once when the world is ready (`minecraft_portal::strip`, the portal strip loader made general).
  - `hud` `minecraft_freeze.rs` (new): the outline drawn over the whole screen in white with alpha = percent frozen while it is above 0, in every camera mode, hidden with the rest of the Minecraft HUD like the portal overlay. It sits above the fire overlay and below the portal overlay, as vanilla's `Hud.extractCameraOverlays` draws it before the portal.
  - `hud` `minecraft_vitals.rs`: hearts use `frozen_full`, `frozen_half` and their blinking sprites when fully frozen, unless poisoned or withered (vanilla's `HeartType.forPlayer` order); absorbing hearts and the containers stay as they are.
  - `minecraft_terrain` `lava_fog.rs`: `in_powder_snow` (colour `#9FBECC`, fog from 0 to 2 blocks, sky and cloud fog ends the same, no clouds or skybox; spectators -8 to half the render distance), used by `render_anim` when the block at the eye is powder snow and the eye is not in lava.
- **Tests:** `hud` `minecraft_vitals`: `fully_frozen_players_get_frozen_hearts_but_poison_wins`; `hud` `minecraft_freeze`: `the_outline_fades_with_the_frost`; `minecraft_terrain` `lava_fog`: `powder_snow_fog_is_two_blocks_of_pale_blue`.
- **Decisions:** the outline's alpha follows whole ticks of frost (vanilla's percent is per tick too); the fog follows the eye's block, not the third-person camera's.
- **Test steps:** (survival, the 3-deep powder snow pit from commit 5)
  1. Walk into the pit: with your head in the snow the screen turns pale blue fog about 2 blocks deep (**2**, inside looking out).
  2. Stay inside: over 7 seconds a frosty border fades in over the whole screen; when it is full the hearts turn frozen (blue-white) (**3**, full frost with the hearts visible). Press F5: the border stays.
  3. Get out: the border fades over about 3.5 seconds and the hearts turn red again as soon as it starts fading.
  4. Drink a poison potion inside the snow (or `/effect give @s minecraft:poison 10`): the hearts are green, not frozen.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:bucket,minecraft:cobblestone*64
```

### Group c5-snow-ladder, combined kit

```
IW4L_MINECRAFT_KIT=minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:powder_snow_bucket,minecraft:bucket,minecraft:leather_boots,minecraft:leather_helmet,minecraft:flint_and_steel,minecraft:netherrack*16,minecraft:ladder*32,minecraft:oak_planks*64,minecraft:cobblestone*64
```

## Group c1-shulkers-1: live shulkers, their model, tipped arrow effects (c1 groups 4, 5 and 6a)

### 8. Minecraft shulkers are live mobs that cling to blocks, peek and teleport

Previous commit: 39e15602

- **Changed:**
  - `minecraftoss-entities` `shulker.rs` (new): the shulker's state (`attach_face`, raw `Peek` 0-100, the eased peek and last tick's, colour, `covered`, the 6-tick teleport lerp) and vanilla's pure rules: `physical_peek`, `progress_box`/`progress_delta_box` (the unit box stretched toward the opening side, `Shulker.getProgressDeltaAabb`), `can_stay_at` (own cell air, the neighbour's face sturdy toward it, room for the fully open lid, no other shulker in the way), `find_attachable_surface` (down, up, north, south, west, east), `teleport_somewhere` (5 tries within ±8, above the bottom, on air, beside a sturdy face, never onto `#shulker_does_not_teleport_to`), and `ShulkerLookControl` (pitch 0, the head yaw measured in the plane of the attach face, back toward body yaw 0 when nothing is looked at).
  - `world/shulkers.rs` (new): `ShulkerEntity`, `spawn_shulker`, `tick_shulker`: fluids, fire, suffocation, drowning, effects, then the goals, then (as `Shulker.tick` after `super.tick`) the attachment check (a new face, else a teleport with `entity.shulker.teleport`, the peek reset, the target dropped and a teleport report), then the peek step (0.05 a tick toward raw / 100) and, while it grows, the push of every other mob body in the newly covered slab along the opening side. It never moves otherwise: no motion, no knockback, no blast push.
  - `monster_ai.rs`: `MonsterKind::Shulker` with its goals (look at players within 8 horizontally, an attack goal placeholder for group 6, `ShulkerPeekGoal`: 1 in 20 on full ticks when it can stay and has no target, raw peek 30 for 20, 40 or 60 ticks; random look) and targets (`HurtByTargetGoal`, nearest player in sight within 16, not in peaceful). Shulkers are `Enemy`s, so iron golems go for them.
  - `world.rs`, `combat.rs`, `pushing.rs`, `hazards.rs`, `fireballs.rs`, `witches.rs`: the shulker in every per-kind list (keys, bodies, damage states, voices, census, despawn with the golem rule `(128, false, true)`, removal, tick dispatch). Arrows, fireballs and the ray pick use its progress box, so wall and ceiling shulkers are hit where they are. In this commit it takes plain damage.
  - `minecraft_terrain`: `minecraft:shulker` is simulated; `spawn_saved` reads `AttachFace` (`Direction.LEGACY_ID_CODEC` byte), `Peek`, `Color` and the `minecraft:covered` armour modifier, `mob_tags` writes them back (the modifier only while covered). The client mob list gets the shulker with its head yaw and a 180° head range; teleports reach the client as level event 2016's portal particles (`egg_teleports`).
  - `minecraft_survival` `entities.rs`: MW2 bullets hit the shulker's progress box under key 26.
- **Tests:**
  - `entities` `shulker`: `the_physical_peek_eases_in_and_out`, `the_box_stretches_toward_the_opening_side` (every face at peek 0 and 1, and the newly covered slab), `a_shulker_clings_to_a_sturdy_face_with_room_to_open`, `teleports_land_within_eight_blocks_on_air_beside_a_sturdy_face` (never onto bedrock), `the_peek_moves_a_twentieth_a_tick_and_a_floor_shulker_grows` (raw 30 in 6 ticks), `a_teleport_draws_from_the_old_block_over_six_ticks`, `the_head_turns_in_the_plane_of_its_face`.
  - `entities` `world::shulkers`: `a_lone_shulker_peeks_now_and_then_with_its_sounds`, `a_shulker_whose_floor_goes_clings_to_a_wall_or_teleports`, `water_or_a_block_over_the_lid_sends_it_away`, `a_teleport_resets_the_peek_and_drops_the_target`, `a_shulker_turns_its_head_to_a_nearby_player_and_never_moves`, `a_far_climber_is_acquired_like_any_player_in_sight`, `an_opening_shulker_pushes_a_mob_in_its_way`, `shulkers_stay_far_away_and_in_peaceful`.
  - `minecraft_terrain` `server`: `a_shulker_keeps_its_face_peek_colour_and_shell_through_a_save`, `an_end_city_sentry_becomes_a_live_shulker_on_the_floor`; `live_shulkers_stay_far_from_players_while_dormant_monsters_go` and the gateway test now expect a live shulker instead of a dormant one.
  - `minecraft_survival` `entities`: `shulkers_take_bullet_key_26`.
- **Decisions:**
  - **Targeting differs from the brief, toward vanilla:** javap shows `NearestAttackableTargetGoal.findTarget` asks `getNearestPlayer` for players and uses `getTargetSearchArea` only for other mob types, so `ShulkerNearestAttackGoal`'s axis-shaped box never limits players. A shulker takes any player within 16 blocks in sight, at any height. The brief's "only within 4 blocks of its height" was wrong.
  - The body stays vanilla's position (the block's bottom centre) with a 1-wide box, 1 + peek high on a floor; places that need the real box (MW2 bullets, arrows, fireballs, the ray pick) use `progress_box`. Explosions, sweeps and pushing keep the body box, which matches for floor shulkers.
  - Shulkers are not solid to other mobs (vanilla's `canBeCollidedWith`); an opening lid pushes mobs out of its way, and players are not pushed or blocked (brief decision).
  - Sturdy faces come from the block's collision boxes (the World's authored rule), and `#shulker_does_not_teleport_to` is bedrock in the authored tag list.
  - The open and close sounds come with the peek in this commit; the hurt, death and ambient voices come in the next.
- **Test steps:** (survival, `/difficulty normal`, daytime, an end stone floor about 15 × 15; the shulker is not drawn until commit 10, so this commit is tested by ear and by hit markers)
  1. Use a shulker egg on the floor 10 blocks away and walk 20 blocks off. Every few seconds you hear the shulker open and close its shell.
  2. Shoot where it sits with an MW2 gun: hit markers on the block it sits in.
  3. Mine the end stone under it while standing 20 blocks away: you hear the teleport sound and purple portal particles spring from where it was toward where it went.
  4. Pour water on it: it teleports the same way.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:shulker_spawn_egg*8,minecraft:end_stone*64,minecraft:diamond_pickaxe,minecraft:water_bucket
```

### 9. Minecraft shulkers hide in their shell: armour, arrow bounces, hurt teleports, loot and sounds

Previous commit: 116887f1

- **Changed:**
  - `world/shulkers.rs`: `hurt` plays `entity.shulker.hurt_closed` while closed, else `entity.shulker.hurt` (and `death` on the killing blow), resets the ambient clock to -120 (`AbstractGolem`), and when a hit lands below half health draws 1 in 4 to teleport. `hurt_armored` applies the `minecraft:covered` armour of 20 while covered. The ambient roll runs each tick as for every mob (interval 120); `entity.shulker.ambient` only plays while open.
  - The armour applies to player hits and MW2 bullets (`hurt_by_player`), mob hits, blasts, arrows, tridents and the block hazards that do not bypass armour; drowning, suffocation, fire and magic bypass it.
  - A closed shulker refuses arrows (the arrow's existing "not accepted" path turns it back at a tenth of its speed) and thrown tridents (`thrown::hurt_by`, an `AbstractArrow` in vanilla). Snowballs, eggs and tridents now find shulkers at their progress box (`thrown::targets`).
  - Deaths: table `minecraft:shulker` (one `shulker_shell` at 0.5 unenchanted), 5 experience when a player killed it. Healing and harming potions reach shulkers.
- **Tests:** `entities` `world::shulkers`: `the_shell_armour_comes_only_with_the_first_close` (a fresh one takes 10, a closed one after its first close 4, an open one 10), `a_closed_shulker_turns_arrows_away_and_an_open_one_takes_them`, `below_half_health_one_hit_in_four_teleports_it` (seeded rolls between 10 and 32 of 80; never above half health), `hits_and_blasts_never_move_it`, `its_voices_follow_its_shell`, `a_dead_shulker_leaves_five_experience_and_its_shell_about_half_the_time` (400 rolls of the real loot table from `MINECRAFTOSS_ROOT`'s `client.jar`, 160 to 240 shells). `minecraft_terrain` `thrown`: `a_closed_shulker_turns_tridents_away_and_is_struck_where_its_shell_is`.
- **Decisions:**
  - Vanilla teleports inside `hurtServer`; the entity world's hurt paths have no world to search, so the roll happens in the hurt and the teleport at the start of the shulker's next tick (at most one tick later). A failed search is not retried, as in vanilla.
  - Looting isn't supported anywhere in the fork yet (the killer's weapon doesn't reach the loot context, and the evaluator refuses `random_chance_with_enchanted_bonus` with looting). Shulkers drop shells at the unenchanted 50 % for any weapon, so the brief's Looting III step is dropped.
  - Players can't shoot bows in the fork, so only skeleton arrows and tridents meet the arrow rule in game.
  - A teleport resets the peek without the close sound and without putting the armour back, as vanilla's teleport writes `DATA_PEEK` directly.
- **Test steps:** (survival, `/difficulty normal`, the same end stone floor; shulkers are drawn from commit 10)
  1. Use a shulker egg on the floor. Shoot it with the MW2 gun while it is closed, after it has opened and closed at least once: it takes little damage and you hear a hollow, closed hurt sound. Wait until it peeks and shoot again: full damage and the plain hurt sound (**1**).
  2. Keep shooting it below half health: now and then it teleports away with portal particles and a sound (**2**).
  3. Kill 8 shulkers with the iron sword: about half drop a shulker shell, and each leaves experience orbs (**3**).
  4. Throw the trident at a closed shulker: it bounces off. Throw it at an open one: it hits.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:shulker_spawn_egg*16,minecraft:iron_sword,minecraft:trident,minecraft:end_stone*64
```

### 10. Minecraft shulkers are drawn with their shell, lid and head

Previous commit: 17e4a5d3

- **Changed:**
  - `minecraft_terrain` `shulker_render.rs` (new): `ShulkerModel` (64 × 64: the lid 16 × 12 × 16 at UV 0,0 and the base 16 × 8 × 16 at 0,28, both at 24; the head 6 × 6 × 6 at 0,52 at 12) with `setupAnim`: the lid at `16 + sin((0.5 + peek) π) × 8` plus the open wobble `sin(age × 0.1) × 0.7` past half open, turned by `h⁴ π / 8` above a peek of 0.3, the head turned by `yHeadRot - 180`. `ShulkerRenderer`'s pose: no body turn (its body yaw stays 0), the death tip, then the opening side's `Direction.getRotation` about (0, 0.5, 0); the peek interpolated with the partial tick; the 6-tick teleport offset from the old block; the sheet per `Color` (`shulker_<dye>`, else the plain `shulker`). No shadow (`MobRenderer` radius 0). The red hurt overlay as for every mob.
  - `special_models.rs`: the dye list is shared with the renderer.
  - `minecraft_survival` `entities.rs`: `meshes()` draws the shulkers.
- **Tests:** `minecraft_terrain` `shulker_render`: `a_shulker_is_a_lid_a_base_and_a_head` (3 cuboids, the base on the block's lower half, a closed lid topping the block), `the_lid_rises_and_turns_as_it_peeks` (24, 20.70 and 8 at 0, 0.3 and 1; no turn at 0.3, `h⁴π/8` above; the wobble only past half open), `the_head_follows_the_head_yaw`, `a_ceiling_shulker_is_the_floor_one_turned_over_about_the_block_centre` (and a wall one lies on its side), `its_colour_picks_its_sheet` (14 red, 0 white, none, 16 and 200 the plain sheet). The 6-tick teleport offset is tested in commit 8.
- **Decisions:** the shulker is drawn at its own block position (vanilla gives it no position interpolation) with the client mob only lending the eased head yaw and the hurt and death state.
- **Test steps:** (survival, `/difficulty normal`, daytime, an end stone floor about 15 × 15)
  1. Use a shulker egg on the floor 10 blocks away and walk 20 blocks off: a purple shulker sits closed on the block. Every few seconds it lifts its lid a little and closes it again, with the open and close sounds (**1**).
  2. `/summon minecraft:shulker ~3 ~ ~ {Color:14b}` and `/summon minecraft:shulker ~5 ~ ~ {Color:0b}`: a red and a white shulker (**2**).
  3. Build a wall and a ceiling of purpur and use eggs on the wall face and on the ceiling's underside: they cling to the wall and hang from the ceiling, lid away from the block (**3**).
  4. Stand 6 blocks from a floor shulker in its sight: its head turns toward you (inside the shell, under the lid).
  5. Shoot one below half health until it teleports: it slides from its old block to the new one in a quarter of a second, with portal particles (**4**).
  6. Quit with shulkers of three colours on a floor, a wall and a ceiling. Restart: all three keep their colour and face (**5**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:shulker_spawn_egg*16,minecraft:end_stone*64,minecraft:purpur_block*64,minecraft:diamond_pickaxe
```
