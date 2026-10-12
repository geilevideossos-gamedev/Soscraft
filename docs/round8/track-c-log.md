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

### 11. Minecraft tipped arrows give players their effect

Previous commit: c9fa0c37

- **Changed:**
  - `minecraft_survival` `entities.rs`: the hits `Entities::tick` hands back carry the arrow's effect (`PlayerHitKind::Arrow`'s `effect`), which the conversion used to drop.
  - `survival.rs`: `Survival::struck` takes a hit with the shield's share: fully blocked, it is `hurt_blocked`; otherwise the rest hurts, and only when that lands (not invulnerable, not in the damage cooldown) the arrow's effect is added (`Arrow.doPostHurtEffects` runs only after `hurtServer` returned true).
  - `render_anim` `minecraft_world.rs`: the hit loop calls `struck` with the effect.
- **Tests:** `minecraft_survival` `survival`: `a_tipped_arrow_gives_its_effect_only_when_it_hurts` (a poison arrow poisons for its 100 ticks, a fully shield-blocked one and a creative player get nothing, a partly blocked one hurts by what gets through).
- **Decisions:** the effect lands at amplifier 0, as every mob arrow in the port carries (strays' slowness, boggeds' poison); the shulker bullet's levitation in group 6 goes through the same path.
- **Test steps:** (survival, `/difficulty normal`, at night or underground; armour off)
  1. Use a bogged egg 12 blocks away and let it shoot you: when an arrow hurts you, your hearts turn green (poison) for a few seconds (**1**).
  2. Drink the milk, raise the shield toward the bogged and let it shoot: the arrows thud on the shield and you are not poisoned.
  3. Use a stray egg and let it shoot you: you walk slower for a few seconds after a hit.
  4. `/gamemode creative` and let them shoot: nothing happens. `/gamemode survival`.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:bogged_spawn_egg*2,minecraft:stray_spawn_egg*2,minecraft:shield,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cooked_beef*16
```

### Group c1-shulkers-1, combined kit

```
IW4L_MINECRAFT_KIT=minecraft:shulker_spawn_egg*16,minecraft:end_stone*64,minecraft:purpur_block*64,minecraft:diamond_pickaxe,minecraft:water_bucket,minecraft:iron_sword,minecraft:trident,minecraft:bogged_spawn_egg*2,minecraft:stray_spawn_egg*2,minecraft:shield,minecraft:milk_bucket,minecraft:milk_bucket,minecraft:cooked_beef*16
```

## Group c1-shulkers-2: shulker bullets, their drawing, the docs (c1 groups 6, 7 and 8)

### 12. Minecraft shulkers shoot homing bullets that make players levitate

Previous commit: 9f8b51e3

- **Changed:**
  - `minecraftoss-entities` `shulker_bullet.rs` (new): `ShulkerBullet`'s state (position, motion, move direction, flight steps, target delta, owner and target by ID, and by UUID until a saved one is found again) and vanilla's rules: `selectNextMoveDirection` (the block holding the target's middle, aimed at straight within 2 blocks; otherwise one step along an axis other than the given one toward it into an empty block, east before west, up before down, south before north, else a random free direction, 10 to 50 flight steps, speed 0.15), and `tick`: homing (`targetDelta × 1.025` clamped to ±1, motion eased by a fifth) or gravity 0.04 with no live target, the ray (blocks, then bodies grown by the projectile margin, a ray starting inside a box missing it), the full move, then the steering (flight steps run out, a block it can stand on ahead, or lined up with the target on its axis).
  - `world/shulker_bullets.rs` (new): spawning, ticking and hits. A player hit is a `PlayerHit` of 4 with the new `PlayerHitKind::ShulkerBullet`, credited to the shulker. A mob hit hurts by 4 through the arrow path (armour, knockback from the motion, endermen dodge) and gives levitation for 200 ticks when it lands; a shulker hit takes its shell armour, and when it lands on an open shulker without its half-health roll, `hitByShulkerBullet` teleports it and, with `n` live shulkers within 8 of where it sat, makes a copy of its colour there unless the level random's float is below `(n - 1) / 5`. Block hits play `entity.shulker_bullet.hit` and report two explosion puffs; any hurt (`hurt_shulker_bullet`) plays `entity.shulker_bullet.hurt`, reports fifteen crit puffs and pops it. Peaceful removes them (`checkDespawn`).
  - `monster_ai.rs`: `ShulkerAttackGoal` in place of the placeholder: with a live target outside peaceful it opens fully (raw peek 100), looks at it, fires 20 ticks after it starts and then every `20 + nextInt(10) × 10` ticks within 20 blocks, drops a target beyond, and closes when it stops. The shot plays `entity.shulker.shoot` at volume 2 with the shulker's two-float pitch.
  - `world/shulkers.rs`: the shots become bullets from the centre of the shulker's box, kept off its attach axis.
  - `world.rs`, `pushing.rs`: the bullet in every per-kind list (`EntityKey::ShulkerBullet`, positions, copies, removal, despawn, tick dispatch, type `minecraft:shulker_bullet`).
  - `minecraft_terrain` `server_mobs.rs`: `minecraft:shulker_bullet` loads and saves with `Steps`, `Dir` (`Direction.LEGACY_ID_CODEC`, absent when it has none), `TXD`/`TYD`/`TZD`, `Owner`, `Target` and `UUID`; `server.rs`: the `HurtShulkerBullet` command.
  - `minecraft_survival`: the hit conversion moved into `mob_hit` (unchanged for the other kinds); a bullet hit is `MobAttack::MobProjectile` (`minecraft:mob_projectile`, message `mob`: "was slain by Shulker"), scaled by difficulty, and carries levitation for 200 ticks into `Survival::struck`, so it lands only with the damage. MW2 bullets and blasts pop shulker bullets under key 27 (their 0.3125 box), and a punch pops one in reach.
- **Tests:**
  - `entities` `shulker_bullet`: `it_steps_along_one_axis_toward_the_target_but_never_along_its_shulker_s` (east or south, never up, flight steps 10 to 50 in tens), `near_the_target_it_aims_straight_at_it`, `walled_in_it_picks_a_random_way_out`, `it_homes_in_on_a_still_target_and_strikes_it`, `it_turns_when_the_block_ahead_is_solid_or_it_lines_up`, `without_a_target_it_falls_and_pops_on_the_ground`, `a_fresh_bullet_flies_out_of_its_own_shulker`.
  - `entities` `world::shulker_bullets`: `a_shulker_opens_and_fires_every_one_to_five_and_a_half_seconds`, `a_bullet_hurts_the_player_and_none_fly_in_peaceful`, `a_creative_player_is_struck_but_unhurt`, `a_struck_zombie_floats_up_for_ten_seconds`, `a_struck_open_shulker_jumps_away_and_a_lone_one_leaves_a_copy` (and a closed one stays), `any_hurt_pops_a_bullet_and_the_ground_stops_one`, `a_saved_target_is_found_again_by_its_uuid`.
  - `minecraft_terrain` `server`: `a_shulker_bullet_keeps_its_course_owner_and_target_through_a_save`.
  - `minecraft_survival`: `entities::shulker_bullets_take_bullet_key_27`, `entities::a_shulker_bullet_hurts_by_difficulty_and_lifts_for_ten_seconds` (3, 4 and 6 on easy, normal and hard, none in peaceful), `survival::a_shulker_bullet_lifts_the_player_only_when_it_hurts` (shield and creative give none), `death::a_shulker_bullet_slays_the_player_by_the_shulker`.
- **Decisions:**
  - Players already have UUIDs in the entity world (`set_player_uuid`), so a saved bullet's `Target` and `Owner` are kept as UUIDs and found again on its first tick (`EntityReference`): a target that is not there then is dropped and the bullet falls, as in vanilla.
  - A bullet that runs out of steps while lined up with its target on the two other axes picks a random free direction, which can turn it back; vanilla does the same, and such a bullet can strike its own shulker (and split it).
  - Skeleton arrows and blasts do not pop bullets yet (vanilla: arrows can hit them, explosions hurt them); MW2 guns, MW2 blasts and punches do.
  - The bullets are not drawn until the next commit, which also shows the puffs.
- **Test steps:** (survival, `/difficulty normal`, daytime, an end stone floor about 15 × 15; armour off; the bullets are invisible in this commit, so watch the effect)
  1. Use a shulker egg on the floor and stand 10 blocks away in its sight. It opens fully and you hear it fire every 1 to 5.5 seconds (**1**).
  2. Let a bullet hit you: 2 hearts, and you float up for 10 seconds, about 9 blocks, while you can strafe (**2**). Then you fall and lose about 3 hearts.
  3. Raise the shield toward the shulker: the hits thud on it and you don't float.
  4. `/difficulty easy`: a hit takes 1.5 hearts; `/difficulty hard`: 3 hearts. `/difficulty normal`.
  5. Use a zombie egg between you and the shulker: a bullet that strikes it lifts it up.
  6. Place two shulkers 3 blocks apart and stand so the far one's bullets pass the near one while it is open: a struck one teleports, and sometimes a copy of its colour appears where it sat (**3**).
  7. `/difficulty peaceful`: the shulkers close and stop firing. `/difficulty normal`.
  8. Let a bullet kill you at half a heart: the chat says "… was slain by Shulker".
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:shulker_spawn_egg*8,minecraft:zombie_spawn_egg*4,minecraft:shield,minecraft:end_stone*64,minecraft:purpur_block*64,minecraft:cooked_beef*32
```

### 13. Minecraft shulker bullets glow and leave an end rod trail

Previous commit: a9e98d20

- **Changed:**
  - `minecraftoss-entities` `shulker_bullet.rs`: the bullet turns halfway toward its motion each tick (`ProjectileUtil.rotateTowardsMovement(this, 0.5)`), keeping last tick's yaw and pitch for drawing, and gives its trail point (`position - motion + 0.15` up).
  - `minecraft_terrain` `shulker_render.rs`: `ShulkerBulletModel` (64 × 32: 8 × 8 × 2 at 0,0, 2 × 8 × 8 at 0,10, 8 × 2 × 8 at 20,0) posed as `ShulkerBulletRenderer`: lifted 0.15, spun by `sin(age × 0.1) × 180°` about Y, `cos(age × 0.1) × 180°` about X and `sin(age × 0.15) × 360°` about Z, scaled (-0.5, -0.5, 0.5), the model turned by the bullet's yaw and pitch; drawn full bright with `entity/shulker/spark` (cut out), then 1.5 times larger in the translucent pass with colour `0x26FFFFFF` (alpha 0.149). Interpolated with the partial tick. `mesh.rs` puts the spark sheet in the atlas.
  - `ambient_particles.rs`: `Kind::EndRod` (`EndRodParticle`: still, gravity 0.0125, friction 0.91, size × 0.75, 60 to 71 ticks, `glitter_7` down to `glitter_0` by age, fading out and toward `#F2DEC9` over its second half, full bright, translucent, no collision) and `Kind::Crit` (`CritParticle`: a tenth of the spread speed plus 0.4 of the motion, grey 0.6 to 0.9 turning red, gravity 0.5, friction 0.7, 4 to 10 ticks, growing in over its first thirty-second, ticked once when made, no collision); `puffs` spreads a count of particles by a Gaussian as `ClientPacketListener.handleParticleEvent` does for `sendParticles`.
  - `server.rs`: the tick output carries the bullets' puffs. `minecraft_survival` `entities.rs`: two explosion puffs (spread 0.2) where a bullet hits a block, fifteen crits where one is shot or punched, one end rod sparkle per bullet each client tick, and the bullets drawn with the mobs.
- **Tests:** `entities` `shulker_bullet::it_turns_halfway_toward_its_motion_each_tick_and_trails_behind`; `minecraft_terrain` `shulker_render::a_bullet_is_three_crossed_slabs_in_a_faint_bigger_glow` (2 × 3 cuboids, alpha 1 and 0.149, full bright, the glow 1.5 times the reach), `shulker_render::a_bullet_spins_with_its_age` (ages 0 and 10), `ambient_particles::end_rod_sparkles_drift_down_glitter_and_fade_out` (lifetime 60 to 71, the fall, the fade and the sprites), `ambient_particles::crits_fly_out_slowly_and_redden` (and two size-2 explosion puffs); `minecraft_survival` `entities::a_struck_bullet_sparks_and_one_that_lands_puffs`.
- **Decisions:** the end rod and crit particles live in `ambient_particles.rs` beside the other client particles rather than in a module of their own, since they share its mesh, atlas and tick; vanilla adds a new particle after the engine's tick, so the trail is added after the particles tick.
- **Test steps:** (survival, `/difficulty normal`, daytime, an end stone floor about 15 × 15)
  1. Use a shulker egg on the floor and stand 10 blocks away in its sight. Its bullets glow, spin and move in straight steps that turn at right angles, leaving a white sparkle trail that drifts down and fades (**1**).
  2. Let a bullet hit the floor: a small explosion puff and a pop.
  3. Shoot a bullet with the MW2 gun or punch it: it pops with red-white sparks (**2**).
  4. Walk behind a wall: a bullet that hits the wall puffs there.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:shulker_spawn_egg*4,minecraft:end_stone*64,minecraft:purpur_block*64,minecraft:cooked_beef*16
```

### 14. Docs: Minecraft shulkers and levitation

Previous commit: 69bf8539

- **Changed:** `docs/MINECRAFT-SURVIVAL.md`: shulkers and shulker bullets join the simulated mobs; a Shulkers part under The End (clinging, peeking, teleports, the shell, loot, bullets, levitation and slow falling); X6 no longer lists shulkers or levitation; a Shulkers limitation (not solid to the MW2 player, no push of players by an opening lid, no Looting, skeleton arrows and blasts don't pop bullets, no effect icons); `/effect` knows eleven simulated effects; shulkers leave the dormant list. `docs/NETHER-END.md`: the shulker row is done.
- **Tests:** none (docs only).
- **Test steps:** none.

### Group c1-shulkers-2, combined kit

```
IW4L_MINECRAFT_KIT=minecraft:shulker_spawn_egg*16,minecraft:zombie_spawn_egg*4,minecraft:shield,minecraft:end_stone*64,minecraft:purpur_block*64,minecraft:cooked_beef*32,minecraft:milk_bucket,minecraft:potion@slow_falling,minecraft:elytra,minecraft:firework_rocket*64,minecraft:ladder*16,minecraft:water_bucket
```

## Group c2-cities: End cities that generate and fill their chests (c2 group A)

### 15. Minecraft End cities generate on the outer islands

Previous commit: 51ec3c0c

- **Changed:**
  - Vendored `generator/src/structure/kinds/end_city.rs` (MinecraftOSS patch): the "Sentry" and "Elytra" data markers now also need vanilla's `Level.isInSpawnableBounds` (Y in -20,000,000 to 20,000,000, X and Z in -30,000,000 to 30,000,000), as `EndCityPiece.handleDataMarker` checks (javap). The elytra frame's position, yaw and facing moved into a pure `elytra_frame_placement` so they can be tested. `EndCityPiece::template_name` and `rotation` are public, so tests can tell which city has a ship and how the ship is turned.
  - Nothing else needed fixing: the port's start rule, pieces, chests, sentries and ship frame came out as vanilla describes on the first run.
- **Tests:**
  - generator `end_city`: `end_city_markers_follow_the_piece_rotation` (all four rotations: the frame's `Facing` is `rotation.rotate(SOUTH)`'s 3D value, its yaw the 2D value × 90, and its position the marker's centre moved 0.46875 against the facing, inside the marker's cell), `markers_outside_the_spawnable_bounds_add_nothing`.
  - `minecraft_terrain` `terrain`:
    - `end_cities_generate_on_the_outer_islands_with_their_chests_sentries_and_ship` (seed 77): `/locate`'s nearest city from (1000, 0, 0) (at 720, 1024) starts at Y 60 or more with a base floor; its chunks (the start's whole box) hold purpur, end stone bricks, end rods and magenta stained glass, no structure blocks, every chest has `LootTable` `minecraft:chests/end_city_treasure` and a non-zero `LootTableSeed`, and at least two shulkers stand at block centres on a whole Y. Then the structure chunks of the `end_cities` set within 100 chunks are walked by distance until a city has a ship (chunk 65, 66 for seed 77): one `minecraft:item_frame` holding one elytra, its `Facing` and yaw from the ship's rotation, `Pos` against its wall, its own cell air and its support solid, an `end_city_treasure` chest one block down on either side, and one `dragon_wall_head` with its `minecraft:skull` block entity.
    - `the_outer_end_islands_generate_beyond_the_main_island`: chunks at x 1100 to 1300 hold highlands or midlands columns and end stone, and no end spikes.
- **Decisions:**
  - The ship is found by walking the set's structure chunks (`is_structure_chunk`, then `starts_in`) rather than by repeated `/locate`, so the test reads the ship piece's own rotation; the ship's frame turns with the ship piece (its bridge's rotation), not with the city's first piece.
  - `/locate` returns the city at (720, 1024) although the city at chunk (65, 66) is a little nearer to (1000, 0): the search stops at the first ring that has a city, as vanilla's `findNearestMapStructure` does. Not changed.
  - The elytra frame is still saved as a dormant entity in this commit: it is kept and saved, but not drawn or hit (item frames come in c2 group B). The chests are tagged, but about nine in ten still open empty until the next commit (enchanted loot).
- **Test steps:** (a new world `c2test` with `IW4L_MINECRAFT_DIMENSION=the_end` in the `.env`, or travel to the End; seed doesn't matter)
  1. On the End platform type `/locate structure minecraft:end_city`. The chat names a spot more than 1000 blocks out. Type `/gamemode creative`, then `/tp <x> 120 <z>` with its x and z, and look down. A purple city of towers and bridges stands on an outer island; the banners on the tower tops are plain magenta (known) (**1**).
  2. Fly through the city: shulkers sit in the rooms and on the towers (they are live from c1, and shoot in survival).
  3. If no ship floats beside the city (a long purple boat with a black dragon head on its bow), fly about 300 blocks further out and `/locate` again. **2:** the ship with the dragon head.
  4. Inside the ship's lower deck there are two chests side by side with an empty spot on the wall between them (the elytra's frame is there but isn't drawn until item frames come).
- **Kit:** none needed (creative).

### 16. Minecraft structure chests roll enchanted gear and books

Previous commit: e7babe74

- **Changed:**
  - Vendored `core/src/loot.rs` (MinecraftOSS patch): a trait `Enchanting` (`enchant_with_levels` and `enchant_randomly`, both drawing from the loot random) and `LootParams.enchanting: Option<Arc<dyn Enchanting>>`, `None` by default so every other caller is unchanged. The functions `enchant_with_levels` (levels from its int provider, then `selectEnchantment` over its options) and `enchant_randomly` (one enchantment of the options, only ones that fit the item unless it is a book or `only_compatible` is false, at a level from 1 to its maximum) now work: a book becomes one `minecraft:enchanted_book` with `minecraft:stored_enchantments`, other items get `minecraft:enchantments`, each level raised to at least the new one and capped at 255 (`ItemStack.enchant`). `enchant_randomly` with nothing to pick leaves the stack alone, as vanilla does. Without an engine both pass the stack through instead of failing, so a missing engine never empties a whole chest. Other unported functions still fail as before.
  - Vendored `core/src/loot.rs`: new `LootTables::roll_seeded`. A chest with a `LootTableSeed` now rolls from `RandomSource.create(seed)` even when its table names a random sequence, as vanilla's `LootTable.fill` with `withOptionalRandomSeed` does (javap). Before, every chest table names its own sequence, so the seed was ignored and a fresh sequence from the world seed was used: every end city chest of a world (and every dungeon chest, and so on) held the same items.
  - Vendored `entities/src/enchant.rs`: `Enchantments::enchant_with_levels` and `enchant_randomly`, moved out of `trading.rs`, which now calls them (trades unchanged).
  - `minecraft_terrain` `containers.rs`: `structure_loot` takes the world's `Enchantments` and wraps them as the loot engine (`LootEnchanting`, with the core random adapted to `LootRandom`); it rolls with `roll_seeded`. `render_anim` passes `globals.data.enchantments`.
- **Tests:**
  - core `loot`: `the_enchanting_functions_write_vanilla_components` (a stub engine: a sword gets `minecraft:enchantments`, a stack of four books becomes one enchanted book with `minecraft:stored_enchantments`, `enchant_randomly` turns a book into an enchanted book and leaves a stick alone when nothing fits), `unknown_functions_still_fail` (`set_damage` still errors; `enchant_with_levels` without an engine passes the sword through).
  - `minecraft_terrain` `containers`: `an_end_city_chest_always_rolls_items` (seeds 1 to 200 all give items, more than 190 different chests, more than 20 enchanted diamond or iron pieces, each with only `#on_random_loot` enchantments that fit it at levels 1 to their maximum), `a_dungeon_book_becomes_an_enchanted_book` (the first seed whose dungeon chest rolls the book: one stored enchantment, no plain book left), `the_enchanting_functions_pass_through_without_an_engine` (end city chests still fill, their gear unenchanted, and the dungeon book stays a plain book). The existing `a_dungeon_chest_rolls_its_jar_loot_table` passes the new argument.
- **Decisions:**
  - The trait lives in core and the engine in `minecraft_terrain` (a newtype over `Arc<Enchantments>`), because the vendored `entities` crate does not depend on `minecraftoss-core`; adding that dependency would change `Cargo.lock`. The shared selection code still lives in `entities/src/enchant.rs`.
  - A chest without a seed keeps the fork's position-based fallback seed (vanilla would use the level's named sequence).
  - Still unported loot functions empty the chest that rolls them: `set_damage`, `exploration_map`, `filtered`, `set_potion`, `set_name`, `set_stew_effect`, `set_ominous_bottle_amplifier`, `set_instrument`, `set_enchantments` (desert pyramids, shipwrecks, ocean ruins and others). For track A (and B's bartering, which needs `enchant_randomly` and `set_potion` from this file: whoever lands second builds on the `Enchanting` hook).
- **Test steps:** (the city from commit 15, survival)
  1. Fly to an end city and open its chests (in the fat towers, the tall house towers and the ship). Every chest holds loot: diamonds, iron or gold ingots, emeralds, beetroot seeds, a saddle or horse armour, and often an enchanted diamond or iron sword, spear, tool or armour piece. Hover over it: its tooltip lists one or more enchantments (**1**). Sometimes a spire armour trim is there.
  2. Open a second chest: its items differ from the first (before, every end city chest of a world held the same items).
  3. Find a dungeon in the Overworld (`/locate` doesn't find dungeons; mob spawner rooms underground) and open its chests: any book in them is an enchanted book with one enchantment.
- **Kit:** none needed.

### Group c2-cities, combined kit

None: the steps use creative flight and the generated chests.

## Group c2-frames: item frames, the elytra in the ship's frame, leftovers (c2 groups B and C)

### 17. Minecraft item frames load, save and survive like vanilla

Previous commit: 612cbb19

- **Changed:**
  - `minecraft_terrain` `item_frame.rs` (new, pure): `ItemFrame` (`ItemFrame`, `GlowItemFrame`, `HangingEntity` and `BlockAttachedEntity` in javap) with the vanilla keys and defaults: `block_pos` (ignored when 16 or more blocks from the block holding `Pos`), `Facing` (`Direction.LEGACY_ID_CODEC`, default down), `Item` (one of the stack), `ItemRotation` (Java's signed remainder by 8), `ItemDropChance` (1.0), `Invisible`, `Fixed`, `Invulnerable`; unknown keys are kept as they came. The box (0.75 square, 0.0625 deep, its centre 0.46875 from the cell's centre against the facing), yaw and pitch (`setDirection`), the support behind it, `survives` (a fixed frame always; otherwise no block collision in the box, inside the world border, a `LEGACY_SOLID` support or a diode behind a wall frame, and no other frame with the same facing whose box meets this one, `canCoexist(true)`), the post-incremented check every 101 ticks (`ticksSinceLastCheck++ >= 100`) and `checkBelowWorld`, and `dropItem`'s drops (nothing from a fixed frame, nothing with `entity_drops` off or for a creative player, the frame item with its custom name, then the item when the frame's random float is below the drop chance), dropped 0.15 out from the wall (`HangingEntity.spawnAtLocation`, float steps).
  - `server.rs`: frames are a list in `ServerSim` beside the end crystals: saved chunk entities `item_frame` and `glow_item_frame` load into it (no longer dormant), are saved and unloaded with their chunk, and tick after the level. A frame that no longer survives plays its break sound (`MobSound`, category `neutral`, volume and pitch 1) and drops its frame and item as level items with the vanilla random motion and 10-tick pickup delay (`spawn_at_location`). `EntitySnapshot` gets `frames` (`FrameView`: id, glow, block, facing, position, item, glint, rotation, invisible), appended at the end. `/summon minecraft:item_frame` and `minecraft:glow_item_frame` go through the same load as a saved frame.
  - `server_mobs.rs`: `item_frame` and `glow_item_frame` join `SIMULATED_KINDS`, so `/summon` offers and accepts them (the existing `command_summons_do_not_reach_the_egg_results` now also summons both and checks they are not dormant).
- **Tests:**
  - `item_frame`: `a_frame_round_trips_its_vanilla_tag`, `the_box_sits_against_the_support_for_every_facing`, `defaults_fill_missing_keys`, `a_far_block_pos_is_ignored`, `frames_check_every_101_ticks`, `support_needs_a_solid_block_or_a_diode_on_a_wall`, `a_block_inside_the_frame_pops_it` (and past the world border), `a_fixed_frame_never_pops`, `two_frames_on_one_face_overlap_but_a_floor_frame_shares_the_cell`, `drops_follow_the_rules_and_the_drop_chance`.
  - `server`: `the_end_city_elytra_frame_is_live_not_dormant` (the catalog's default `item_frame` tag with the end city's keys), `a_frame_whose_wall_is_removed_drops_itself_and_its_item_within_101_ticks` (and nothing with `entity_drops` off, with the glow break sound), `frames_are_saved_with_their_chunk_and_come_back`.
- **Decisions:**
  - Frames are their own list in `ServerSim`, as the brief says, not mobs in the vendored entity world (vanilla frames are plain `Entity`s). All state is per level; nothing reads player 0.
  - `survives` checks block collisions only; vanilla's `noBlockCollision` also sees collidable entities (boats, shulkers), which frames ignore here.
  - The world border is vanilla's default (±29,999,984); the fork has no `/worldborder`.
  - The frames are not drawn yet (next commit).
- **Test steps:** (a new world, survival; frames are not drawn until the next commit, so watch the drops)
  1. Type `/setblock ~2 ~ ~ minecraft:stone`, then `/summon minecraft:item_frame ~2 ~1 ~ {Facing:1b,Item:{id:"minecraft:diamond",count:1}}`: a frame lies on top of that stone (not drawn yet); the chat confirms the summon.
  2. Mine that stone and wait up to 5 seconds: you hear the frame break, and an item frame and a diamond drop (**1**).
  3. Type `/gamerule entity_drops false` and repeat steps 1 and 2: the break sound, nothing drops. `/gamerule entity_drops true`.
  4. Fly to an End city ship (see commit 15): quit and start again near it. The elytra frame stays (it no longer goes dormant), and the ship's chests are still there.
- **Kit:** none needed (creative and commands).

### 18. Minecraft item frames are drawn with their item

Previous commit: fb61cbc8

- **Changed:**
  - `minecraft_terrain` `item_frame.rs`: `frame_pose` (`ItemFrameRenderer.submit`: the render offset undone, back to the cell's centre, X turned by 0 for a wall frame or `-90 × step` for a floor or ceiling frame, Y turned by `180 - toYRot` or 180), `model_pose` (then `-0.5` for the block model), `item_pose` (then 0.4375 out, or 0.5 for an invisible frame, turned about Z by `rotation × 45°`, half size), `light` (the frame's own cell; a glow frame's block light at least 5, `GLOW_FRAME_BRIGHTNESS`, and its item at full bright, `0xF000F0`) and `tracked` (drawn within 10 chunks, the frame's client tracking range).
  - `mesh.rs`: `ItemVisuals::append_frames` meshes the pack's `item_frame` or `glow_item_frame` blockstate model (`map=false`: the `template_item_frame` back and birch rim) under `model_pose` with entity shading, unless the frame is invisible, then the item through its `fixed` display transform under `item_pose`; an enchanted item gets the glint. `append_model` takes an optional light, so frames are lit at their own cell and a glow frame's item at full bright (every other caller passes none and is unchanged).
  - `minecraft_survival` `entities.rs`: keeps the snapshot's frames and draws those within range with the other entity items, after the open lids.
- **Tests:** `item_frame`: `frame_pose_puts_the_back_against_the_wall_for_all_six_facings` (the back's centre 1/64 off the support's face plus the 1/512 offset, the front facing out, the item 0.4375 out), `the_item_turns_45_degrees_per_rotation_about_the_facing`, `an_invisible_frame_moves_the_item_to_the_wall`, `a_glow_frame_raises_its_block_light_to_five_and_lights_its_item_fully`, `frames_are_drawn_within_ten_chunks`, `frames_mesh_the_pack_model_and_their_item` (with the 26.3 pack: the frame uses `block/item_frame` and birch planks, a glow frame `block/glow_item_frame`, the elytra adds its own faces, an invisible frame draws only its item, a glinting item is one glint range).
- **Decisions:**
  - The frame's block model is drawn with the entity shading the other entity items use. Vanilla draws it with a depth bias (`submitWithZOffset`) because its back and rim end exactly on the wall's face; here the model is moved 1/512 of a block out from the wall instead, which keeps the wall from showing through.
  - Maps in frames (the map model and `item_frame_map`) are not ported (the fork has no maps). The name tag of a framed item with a custom name is not drawn.
- **Test steps:** (a new world, creative; frames cannot be hung by hand until the next commit, so they are summoned)
  1. Stand on a flat floor facing north and type `/setblock ~ ~1 ~-2 minecraft:oak_planks`, then `/summon minecraft:item_frame ~ ~1 ~-1 {Facing:3b,Item:{id:"minecraft:elytra",count:1}}`. The frame lies flat on the south face of the planks with a birch rim and a leather back, and the elytra sits in it (**1**).
  2. `/summon minecraft:glow_item_frame ~1 ~1 ~-1 {Facing:3b,Item:{id:"minecraft:carrot",count:1},ItemRotation:2b}` beside a second planks block (`/setblock ~1 ~1 ~-2 minecraft:oak_planks`): a glow frame (cyan back) with the carrot turned 90 degrees.
  3. `/time set midnight` and cover the spot with a roof: the glow frame's carrot stays bright, the plain frame and its elytra go dark (**2**).
  4. `/summon minecraft:item_frame ~ ~ ~ {Facing:1b,Item:{id:"minecraft:diamond_sword",count:1,components:{"minecraft:enchantments":{"minecraft:sharpness":5}}}}`: a frame lies on the floor under you with a glinting sword (**3**).
  5. Fly to an End city ship: the elytra hangs in its frame on the wall between the two chests in the lower deck (**4**).
- **Kit:** none needed (creative and commands).

### 19. Minecraft item frames hang on block faces

Previous commit: a3e242ed

- **Changed:**
  - `minecraft_terrain` `item_frame.rs`: `frame_use` for `minecraft:item_frame` and `minecraft:glow_item_frame` aims at the block within 4.5 blocks and asks the server to hang a frame on the clicked face (`ItemOnBlock::HangFrame { face, glow }`, spent only when the server says yes); `hung_at` is the cell in front of the face.
  - `server.rs`: `HangFrame` builds the frame as `HangingEntityItem.useOn` does: in front of the clicked face, facing out of it, inside the build height, and only if it `survives` (support, nothing in the cell, no other frame on that face); then the place sound and the frame joins the level. A refused frame keeps the item (`item_uses` answers false).
  - `render_anim` `minecraft_world.rs`: one more use in the item-use chain, after end crystals.
- **Tests:** `item_frame`: `frames_hang_on_all_six_faces_with_vanilla_rotations` (the use asks for the clicked face, glow by the item; each of the six faces gives the vanilla yaw and pitch and hangs on the clicked block). `server`: `a_frame_is_refused_without_support_on_a_taken_face_or_inside_a_block` (and above the build height), `a_repeater_holds_a_wall_frame_but_not_a_floor_frame` (and keeps it through two checks), `two_frames_on_one_face_never_overlap`, `a_frame_and_a_floor_frame_share_a_cell`, `the_place_sound_is_the_glow_one_for_glow_frames`.
- **Decisions:**
  - The server decides, as for end crystals; the client never shows a frame before the server has it (one tick later).
  - The item stack's custom name and `entity_data` are not copied onto the new frame (`createDefaultStackConfig`): the server's use carries no stack. Adventure mode's `can_place_on` is not checked (the fork's game mode gate `may_build` already keeps adventure players from using it).
- **Test steps:** (a new world, survival, daytime)
  1. Place an oak planks block and right-click each of its four sides, its top and (from below) its bottom with item frames: each frame lies flat on its face with the place sound, and one frame goes per click (**1**).
  2. Right-click a side that already has a frame: no second frame, and the item stays in your hand.
  3. Place a repeater on the ground and right-click its side with a frame: it hangs. Right-click its top: nothing is placed.
  4. Right-click the ground with a glow item frame: it lies on the floor with the glow sound (a higher chime).
  5. Mine the planks block from step 1: within 5 seconds all six frames drop as items with break sounds.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:item_frame*16,minecraft:glow_item_frame*16,minecraft:oak_planks*64,minecraft:repeater*2,minecraft:diamond_pickaxe
```

### 20. Minecraft item frames take, turn and give back items

Previous commit: a87f9958

- **Changed:**
  - `minecraft_terrain` `item_frame.rs`: `FrameAction` (`Use` with the held stack, `Attack`, `Shot` for an MW2 bullet, `Blast`, each with whether the player has infinite materials) and `Frames::act`, vanilla's rules: `interact` (a fixed frame passes; a framed item turns one step with the rotate sound; an empty frame takes one of the held stack with the add sound, spent unless creative; an empty hand on an empty frame passes), `hurtServer` (a fixed or invulnerable frame ignores everything but a creative player, which breaks it outright with nothing dropped; a non-explosion hit on a framed item takes the item out with the remove sound, `dropItem` without the frame; anything else breaks the frame with the break sound and both drops) and `dropItem`. `FrameView` gets `fixed`; `use_passes` tells the client whether a use passes to the other hand, `on_ray` picks the nearest frame box along a look (frames have pick radius 0), `hanging_box` is the box shared by server and client, and `frame_item_use` is the pending use the client waits on (`Change::Frame` in `bucket.rs`).
  - `server.rs`: `Command::FrameAction` and `ServerSim::frame_action` play the outcome's sounds at the frame and drop its drops 0.15 out from the wall; a use with something in hand answers on `item_uses` (true only when the frame took the item), so the client spends it. `break_blasted_frames` runs before each tick's explosions leave the server: every blast that touches blocks (`ExplosionEffect.interacts`: vanilla's `shouldAffectBlocklikeEntities`) breaks the frames whose position lies within twice its power, with both drops.
  - `minecraft_survival` `entities.rs`: `punch` hits a frame first when it is nearer than both the block under the crosshair (`Blow.block`) and every other entity on the ray (vanilla's pick); `mob_distance` counts frames, so a creative click on a frame does not also break the block behind it. `use_frame` sends a use to the frame the player aims at and waits for the answer when an item was held. MW2 bullets and blasts hit frame boxes under the new bullet key 28 (`FRAME_KEY`), as `Shot` and `Blast` (`shoot` and `blast` take whether the player is creative).
  - `render_anim` `minecraft_world.rs`: right-click goes to the aimed frame before tables, containers and block uses (main hand, then the off hand when the main hand passes), swinging the hand; left-click passes the block distance and creative to `punch`; MW2 hits pass creative.
- **Tests:**
  - `item_frame`: `an_empty_frame_takes_one_item_and_a_full_frame_turns_it` (and creative keeps its stack), `eight_turns_bring_the_item_back`, `a_fixed_frame_ignores_use_and_survival_hits_but_not_creative` (and an invulnerable one), `a_hit_takes_the_item_out_and_a_second_hit_breaks_the_frame` (a punch and an MW2 bullet), `creative_hits_drop_nothing`, `entity_drops_off_drops_nothing`, `drop_chance_zero_keeps_the_item_from_dropping`, `a_blast_breaks_a_full_frame_with_both_drops`, `drops_appear_a_step_out_from_the_wall`, `glow_frames_use_their_own_sounds`, `the_crosshair_picks_the_nearest_frame_box_and_a_use_passes_only_on_an_empty_hand_and_frame`.
  - `server`: `a_bullet_pops_the_elytra_out_of_the_ship_frame` (the item 0.15 out with a 10-tick pickup delay, the empty frame stays, a second bullet breaks it), `tnt_beside_a_frame_breaks_it` (a blast that keeps blocks, as a creeper's with `mob_griefing` off, and one more than twice its power away leave it), `a_frame_use_answers_on_the_item_uses_only_when_an_item_was_held`.
  - `minecraft_survival` `entities`: `the_frame_key_round_trips`, `a_frame_behind_a_wall_is_not_hit` (and behind a nearer mob, and out of reach).
- **Decisions:**
  - MW2 bullets count as projectile hurts (the item comes out first, then the frame breaks); MW2 grenades and every level explosion count as explosions (the frame breaks with both drops). A creative player's bullets and grenades drop nothing, as a creative player's hits.
  - `Command::FrameAction` carries no player id: like `ItemOnBlock` and `MobAction`, its answer goes to the one local player on `item_uses`; track E adds player ids to all of these together. The frame state itself is per level, nothing reads player 0.
  - A frame hit still counts as an attack for the hand (the attack timer resets and the swing costs the usual 0.1 exhaustion); vanilla skips the exhaustion (`skipAttackInteraction`). Minor, left as is.
  - A level explosion whose direct source is in water still breaks frames (vanilla's `ignoreExplosion` checks the TNT's water); the level's explosion record does not carry its source.
  - While a held-item use is still waiting for the server, a second click on a frame does nothing.
- **Test steps:** (survival, a new world; armour off)
  1. Hang a frame on a wall and right-click it with a compass: the compass goes in with a sound and one leaves your hand. Right-click it 8 times: it turns 45 degrees each time with a click and comes back to where it started (**1**).
  2. Left-click the frame: the compass pops out with a sound and drops in front of it; the empty frame stays. Left-click again: the frame breaks and drops. Pick both up.
  3. Hang a frame, put a carrot in and shoot it once with the MW2 gun: the carrot pops out. Shoot again: the frame breaks and drops (**2**).
  4. Hang three filled frames on a wall around a TNT block 2 blocks away and light it: all three break and drop the frame and the item (**3**).
  5. `/gamerule mob_griefing false`, hang a filled frame and let a creeper blow up beside it: the frame stays. `/gamerule mob_griefing true`.
  6. `/gamemode creative` and left-click a filled frame: the item vanishes and nothing drops. Hit again: the frame vanishes, no drop. Right-click an empty frame with a stack: the item goes in and the stack keeps its count.
  7. Right-click an empty frame with an empty main hand and a torch in the off hand: the torch goes in.
  8. End city ship: in survival, right-click the elytra frame once (the elytra turns), then left-click it: the elytra pops out; pick it up (**4**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:item_frame*16,minecraft:glow_item_frame*8,minecraft:compass,minecraft:carrot*16,minecraft:torch*8,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:creeper_spawn_egg*2,minecraft:oak_planks*64,minecraft:diamond_pickaxe
```

### 21. Minecraft comparators read item frames

Previous commit: 88038810

- **Changed:**
  - Vendored `world/src/level/mod.rs` and `redstone.rs` (MinecraftOSS patch): `Level.frame_outputs` holds, per block, the item frames in it with their facing and analog output. The comparator's input (`ComparatorBlock.getInputSignal`): when the block behind is a redstone conductor and the signal is below 15, it now reads the block two behind and the item frame there facing the comparator's way, used only when exactly one such frame is in that block (`getItemFrame`), taking the larger of the frame's and the block's output. `set_frame_outputs` updates the map and, when asked, runs `updateNeighbourForOutputSignal` from the frame's block (as `ItemFrame.setItem` and `setRotation` do).
  - `minecraft_terrain` `item_frame.rs`: `analog_output` (`rotation % 8 + 1` with an item, 0 when empty).
  - `server.rs`: `sync_frame_outputs` keeps the level's map in step with the frames after every use, hit, hang and survival check (with the comparator update) and after chunk loads and unloads (without it, as vanilla does not update on load).
- **Tests:** `server`: `a_comparator_reads_the_frame_rotation_through_a_block` (a comparator behind a stone with a frame on the far side: 0 empty, then 1 to 8 as the arrow turns and 1 again; a second frame in the same block facing up is not read; 0 once the frame is empty).
- **Decisions:** the frames' outputs live in the level as plain data (block, facing, output), so the vendored redstone code needs no entity access; the server is the only writer.
- **Test steps:** (survival, a new world, daytime)
  1. Place a row on the ground, west to east: an oak planks block, then a comparator facing away from it (place it while looking east, away from the planks), then 8 redstone dust.
  2. Hang an item frame on the west side of the planks (the side away from the comparator) and put an arrow in: one dust lights.
  3. Right-click the frame 7 times: one more dust lights at each turn, 8 in all (**1**). One more turn: back to one.
  4. Left-click the arrow out: all the dust goes dark.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:item_frame*4,minecraft:arrow*4,minecraft:comparator*2,minecraft:redstone*16,minecraft:oak_planks*16
```

### 22. Minecraft thrown items and tridents knock items out of frames

Previous commit: 3219fc47

- **Changed:**
  - `minecraft_terrain` `item_frame.rs`: `Frames::targets` gives every frame's box as a projectile target under its own id range (`FRAME_TARGET`, below the players' `PLAYER_TARGET`), and `struck_frame` reads a struck target back.
  - `server.rs`: snowballs, eggs, ender pearls and wind charges that strike a frame hurt it (`onHitEntity` with their 0 or 1 damage): the item comes out, or an empty frame breaks, as an MW2 bullet does. Splash and lingering potions and experience bottles break on a frame without hurting it, as in vanilla. Tridents strike frames the same way and bounce off. A frame is never hurt as a mob (no knockback, no mob hit).
- **Tests:** `server`: `a_snowball_pops_a_framed_item_and_a_trident_breaks_the_empty_frame` (and a splash potion leaves the frame alone; the id ranges round-trip).
- **Decisions:** the brief's list said potions hurt frames; vanilla's `AbstractThrownPotion` does not hurt what it hits, so potions only break there. A frame struck by a creative player's snowball still drops (the server does not know the thrower's game mode yet; with track E's player state it can pass `infinite`).
- **Test steps:** (survival, a new world)
  1. Hang a frame on a wall, put a carrot in and throw a snowball at it: the carrot pops out and the frame stays (**1**).
  2. Throw a second snowball: the empty frame breaks and drops.
  3. Fill a frame again and throw a trident at it: the item pops out and the trident bounces off. Throw again: the frame breaks.
  4. Throw a splash potion of water at a filled frame: it splashes; the item stays.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:item_frame*8,minecraft:carrot*8,minecraft:snowball*16,minecraft:trident,minecraft:splash_potion@water*2,minecraft:oak_planks*16
```

### 23. Docs: Minecraft End cities and item frames

Previous commit: 1680fa64

- **Changed:**
  - `docs/MINECRAFT-SURVIVAL.md`: structure loot rolls enchanted gear and books from each chest's own seed; item frames and glow item frames join the simulated kinds; End cities and Item frames parts under The End; X6 no longer lists item frames; a limitation for End cities and item frames (banner patterns, end rod particles, maps, name tags, middle-click, pistons, named stacks, creative throws, TNT in water, attack exhaustion); a limitation listing the loot functions still missing, whose chests roll nothing. The levitation line's `×` and `−` were double-encoded by commit 14 and are fixed.
  - `docs/NETHER-END.md`: the end city and item frames are done; the dormant list notes that shulkers, end crystals and item frames are live.
- **Tests:** none (docs only).
- **Test steps:** none.

### Group c2-frames, combined kit

```
IW4L_MINECRAFT_KIT=minecraft:item_frame*16,minecraft:glow_item_frame*16,minecraft:compass,minecraft:carrot*16,minecraft:torch*8,minecraft:oak_planks*64,minecraft:repeater*2,minecraft:comparator*2,minecraft:redstone*16,minecraft:arrow*4,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:creeper_spawn_egg*2,minecraft:snowball*16,minecraft:trident,minecraft:splash_potion@water,minecraft:diamond_sword+sharpness=5,minecraft:diamond_pickaxe
```

## Group c3-sky-dragon-1: End flashes, the flash sound, dragon's breath particles (c3 groups 1-4)

### 24. The Minecraft End sky flashes like vanilla

Previous commit: 8ff863be

- **Changed:**
  - New `minecraft_terrain` `end_flash.rs`: `EndFlash`, vanilla's `EndFlashState`. Each 600-tick window of the End clock draws, from `LegacyRandom::new(window)` (the same LCG as `RandomSource.createThreadLocalInstance`), one thrown-away float, the offset (0-200), the duration (100 to `min(380, 600 - offset)`) and the two angles (x -60 to 10, y -180 to 180). The intensity is the table sine of `(t - offset)·π/duration` inside the flash, lerped by the partial tick. `direction_from_rotation` is `Vec3.directionFromRotation` with the table sine; `light_boost` adds the intensity, or a third of it under boss fog, to the sky light factor.
  - `environment.rs`: an End `DimensionEnvironment` holds an `EndFlash`; it is made fresh on the first tick after a load or a dimension switch (as vanilla makes a new `ClientLevel`) and ticked with the dimension's clock. `SkyState.end_flash` is `[intensity, x angle, y angle]` (zero outside the End).
  - `render_anim`: the celestial atlas has 10 slots of 64 px (sun and moons scaled up with nearest, which keeps their pixels), `environment/celestial/end_flash` in slot 9; the uniform's sky light factor gains `light_boost`; `EndFrame.end_flash` carries the flash to `render`, which copies it into the frame.
  - `render_gpu`: `end_portal` in the view uniform is now `[time, intensity, x angle, y angle]`; the celestial divisor is `CELESTIAL_SLOTS` (10); `end_flash` in the sky shader draws vanilla's quad (100 blocks out along the direction, half size 60, axes from the `YP(180 - y)·XP(-90 - x)` pose) over the End sky with the `OVERLAY` blend, `texel.rgb·texel.a·i²`.
- **Tests:**
  - `end_flash`: `window_zero_never_flashes`, `a_window_draws_its_flash_like_java_random` (windows 1, 2 and 7), `one_flash_every_600_ticks_rising_and_falling_as_a_sine`, `a_paused_clock_holds_the_flash`, `the_flash_lights_the_world_a_third_as_much_in_boss_fog`, `the_flash_points_where_direction_from_rotation_points`.
  - `environment`: `only_the_end_flashes` (and a first tick gives the same flash as a fresh `EndFlash`).
  - `render_gpu`: `the_end_flash_quad_centres_on_its_direction_and_spans_point_six` (the WGSL ported to a Rust helper, checked against the vanilla pose built with matrices on a grid of quad points and outside it), `the_celestial_atlas_has_ten_slots`.
  - `render_anim`: `celestial_tests::the_celestial_atlas_has_ten_slots` (with `MINECRAFTOSS_ROOT`: 640x64, the sun and the flash slots have pixels).
- **The jshell lines for the window values** (`jshell -q`):

```
for (long w : new long[]{1, 2, 7}) { java.util.Random r = new java.util.Random(w); r.nextFloat(); int off = r.nextInt(201); int dur = 100 + r.nextInt(Math.min(380, 600 - off) - 100 + 1); float xa = r.nextFloat() * 70.0f + -60.0f; float ya = r.nextFloat() * 360.0f + -180.0f; System.out.println(w + " " + off + " " + dur + " " + xa + " " + ya); }
```

  It printed `1 133 311 -31.479216 -105.22267`, `2 138 296 -59.709076 -1.1438904` and `7 29 334 -59.325974 -54.608513`.
- **Decisions:**
  - The window and the time in it use Java's truncating `/` and `%` (`ldiv`, `lrem`), not floor division as the brief said: that is vanilla, and a negative clock simply never flashes instead of panicking.
  - The flash lives in the level's `DimensionEnvironment`, beside the other presentation state the client camera ticks; it is never saved or sent. A friend's client recomputes it from the End clock (track E's per-level clock).
  - As in vanilla, the sky quad is part of the sky pass, so it is hidden while the dragon's boss bar makes world fog; the light still comes (a third of it).
- **Test steps:** (a new world; the End clock's first 30 seconds never flash, as in vanilla)
  1. In the `.env`: `IW4L_MINECRAFT_WORLD=c3fx`, `IW4L_MINECRAFT_DIMENSION=the_end`. Start on the obsidian platform and stand still for a minute and a half: about every 30 seconds the end stone turns faintly purple for 5 to 19 seconds, rising and fading. The sky stays the dark fog colour while the dragon's bar shows.
  2. `/gamemode creative`, `/tp @s 0 90 250` and fly there (more than 192 blocks from the island, so the bar goes and the End sky shows). Look around the sky for a minute: about every 30 seconds a soft purple glow appears somewhere between straight up and just below the horizon, swells and fades; the island and your blocks glow purple with it, three times as strongly as on the island during the fight (**1**, with the glow in view).
- **Kit:** none.

### 25. Minecraft End flashes rumble a second and a half after they start

Previous commit: d6ed4600

- **Changed:**
  - `end_flash.rs`: `EndFlash::tick(clock, credits_open)` returns the flash's angles 30 ticks (`SOUND_DELAY`) after a flash starts, unless the credits were open when it started (`ClientLevel.tick` skips the sound under `WinScreen`). `DimensionEnvironment::tick` takes `credits_open` and passes the due sound on.
  - `render_anim` `minecraft_sounds.rs`: `Sounds::play_without_falloff` queues an event with infinite attenuation, so its gain stays at its volume at any distance and only the pan places it (`SoundInstance.Attenuation.NONE`).
  - `render_anim` `minecraft_world.rs`: the 20 Hz environment tick passes `ui.credits.is_some()`; a due sound plays `minecraft:weather.end_flash` (volume 1, pitch 1) at the eye plus `direction_from_rotation(x, y)·10` blocks.
- **Tests:** `end_flash`: `the_sound_comes_thirty_ticks_after_the_flash_starts_and_never_during_the_credits` (four flashes in 3000 ticks, each sound 30 ticks after its start with its angles; none with the credits open; a flash that started under the credits stays silent after they close). `minecraft_sounds`: `a_sound_without_falloff_keeps_its_volume_far_away` (with `MINECRAFTOSS_ROOT`: one of the 8 `ambient/weather/end_flash` files, infinite attenuation, full gain 5000 units away).
- **Decisions:** the sound is placed once, when it starts (vanilla's `DirectionalSoundInstance` moves with the camera every tick). It lasts a few seconds, so only a fast turn shows the difference. A sound pending when the player leaves the End is dropped with the level's flash state.
- **Test steps:** (as entry 24, a new world in the End)
  1. Stand on the obsidian platform for a minute and a half. About 1.5 seconds after each purple glow starts, a deep rumble comes from one side, usually from above.
  2. Turn on the spot while a rumble plays: it stays on the side it came from.
- **Kit:** none.

### 26. Minecraft dragon's breath is drawn with vanilla's purple breath particles

Previous commit: d964b0ef

- **Changed:**
  - `minecraft_terrain` `ambient_particles.rs`: `Kind::DragonBreath` (`DragonBreathParticle`), appended at the end of `Kind`. Its draws in vanilla order: the base particle's lifetime (thrown away), the quad size, red 0.718-0.875, no green, blue 0.824-0.976, then quad size ×0.75 and a lifetime of 20 to 100 ticks; `add_breath(at, motion, power)` applies `setPower`. It has no physics (it moves straight through blocks and never lands), its sideways speed grows by 1.1 on a tick it did not rise or fall, sideways friction 0.96, sprites `generic_5` to `generic_7` by age, the size ramp over the first 1/32 of its life, opaque and lit by the level.
  - `breath_cloud(feet, radius, waiting)` (`AreaEffectCloud.clientTick`: while waiting, half the ticks puff 2 still particles within 0.2; once active, `ceil(π·r²)` particles over the disc with a slow drift and 0.01 rise), `breath_burst(at)` (level event 2006: 200 particles from the burst's block, 0.3 up, sprayed out with power up to 4) and `breath_trail(at)` (one still particle half a block above the fireball).
  - `minecraft_survival` `entities.rs`: dragon's breath clouds use `breath_cloud` (potion clouds keep the spell particles), fireball bursts use `breath_burst` instead of a potion splash, and each fireball in a snapshot leaves one trail particle (one snapshot per server tick).
- **Tests:** `ambient_particles`: `a_breath_particle_draws_its_lifetime_size_and_colour_in_vanilla_order` (against a mirror random for three seeds, sprites by age), `power_scales_the_motion_like_set_power`, `a_breath_particle_drifts_through_blocks_and_spreads_when_it_stops_rising`, `a_waiting_breath_cloud_puffs_two_or_none_and_an_active_one_fills_pi_r_squared`, `the_fireball_burst_sprays_two_hundred_breath_particles_from_its_block` (and the trail).
- **Decisions:** all breath particles share the ambient system's random instead of one random per particle (vanilla gives each particle its own `RandomSource.create()`); the draws per particle stay in vanilla's order.
- **Test steps:** (the End, a new world, `/difficulty normal`, diamond armour on)
  1. Walk onto the island and let the dragon fly over you: when it spits a fireball, the fireball leaves a trail of purple puffs behind it (**1**).
  2. Where the fireball bursts, purple puffs spray out low around the spot, and the cloud left on the ground is made of slowly drifting purple puffs, with no round sparkles (**2**).
  3. Throw a lingering potion of any kind: its cloud still uses the old round sparkles in the potion's colour.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:lingering_potion@regeneration*2
```

### 27. Minecraft ender dragon breathes purple from its mouth while it lands and flames

Previous commit: 94e6ed43

- **Changed:**
  - `minecraft_terrain` `ender_dragon.rs`: `DragonEvents.breath` with `DragonBreath { head, look, motion, kind }` and `BreathKind::{Landing, Flame}`. Every tick the dragon ends its phase step in `Landing` it breathes from its head (the head part's centre before the parts move, as `doClientTick` runs before them) along `head_look_vector` normalized, with its own motion (`DragonLandingPhase.doClientTick`). A flame breathes on flame ticks 2, 4, 6 and 8 (`DragonSittingFlamingPhase.doClientTick`). As in vanilla the look is never turned (the two `yRot` results are thrown away).
  - `server.rs`: the dragons' breath gathers in the server and goes to the client in the new `EntitySnapshot.dragon_breath` (appended at the end of the struct and of its literal), taken each snapshot like `breath_bursts`.
  - `ambient_particles.rs`: `mouth_breath`: a landing breath is 8 particles at the head plus `gaussian/2` on each axis, moving `(-look.x·0.08 + motion.x, -look.y·0.3 + motion.y, -look.z·0.08 + motion.z)`; a flame breath is 8 such points with 6 particles each, `(-look.x·0.08·j, -look.y·0.6, -look.z·0.08·j)` for `j` 0 to 5 (192 particles a flame).
  - `minecraft_survival` `entities.rs`: each snapshot's breath goes to `mouth_breath`.
  - Entry 26's step 3 said a lingering potion can be drunk; it can only be thrown.
- **Tests:**
  - `ender_dragon`: `a_landing_dragon_breathes_every_tick_from_its_head` (one landing breath on each landing tick and none outside, from the head's centre, a unit look pointing up so the breath falls), `a_flaming_dragon_breathes_on_its_second_to_eighth_flame_tick` (four flames, each with breath 1, 3, 5 and 7 ticks after it starts, the sitting look `view_vector(-45, yaw)`, no motion).
  - `ambient_particles`: `landing_breath_streams_eight_particles_back_along_the_look` (positions against a mirror random: three gaussians, then the particle's five floats), `flame_breath_fans_six_speeds_at_eight_points`.
  - `server`: `a_landing_dragon_perches_on_the_podium_breathes_flame_and_the_flame_goes_out_when_it_takes_off` also counts one landing breath per landing tick and four flame puffs before the flame lands.
- **Decisions:**
  - The breath is a server event drawn on the client, like the fireball bursts, so a friend's client (track E) gets the same puffs from the snapshot; the gaussians come from the client's particle random, not the dragon's, as the client's dragon random is not synced in vanilla either.
  - The phase that breathes is the dragon's phase after its phase step in that tick; vanilla's client runs the synced phase, which can lag a tick.
- **Test steps:** (the End, a new world, `/difficulty normal`, diamond armour on, golden apples at hand)
  1. Shoot all ten end crystals off the pillars with the MW2 gun. When the dragon comes in to land on the portal in the middle, purple breath streams from its mouth all the way down (**1**).
  2. When it sits and flames, four quick purple bursts pour from its mouth toward the ground at the start of each flame, before the purple cloud spreads on the ground (**2**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:diamond_sword+sharpness=5
```

### Group c3-sky-dragon-1, combined kit

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:diamond_sword+sharpness=5,minecraft:lingering_potion@regeneration*2
```

## Group c3-sky-dragon-2: the dragon's death particles, its knockback, the End music, docs (c3 groups 5-8 and 10)

### 28. Minecraft ender dragon bursts into explosions while it dies and where it breaks blocks

Previous commit: ef543055

- **Changed:**
  - `minecraft_terrain` `ender_dragon.rs`: `DragonEvents.explosions: Vec<(DVec3, bool)>` (`true` for an `explosion_emitter`). `tick_death` draws three offsets from the dragon's own random on death ticks 180 to 200 (`(f - 0.5)·8`, `(f - 0.5)·4`, `(f - 0.5)·8`) and pushes an emitter at the position plus `(a, 2 + b, c)`, before the tick's rise; the dying branch of `ai_step` draws the same three and pushes a plain `explosion` (`EnderDragon.aiStep`). These are vanilla's server draws, so the dragon's random stays in step with vanilla's. `tick` no longer runs `ai_step` once the dragon is removed (`LivingEntity.tick` checks `isRemoved` before `aiStep`).
  - `server.rs`: the dragons' explosions and their wall bursts (level event 2008, one `explosion` at the broken cell's centre) gather in the server and go to the client in the new `EntitySnapshot.dragon_explosions` (appended at the end of the struct and of its literal).
  - `minecraft_survival` `entities.rs`: each one is an ambient `explosion` (size 0, the 2.0 quad) or `explosion_emitter`.
- **Tests:**
  - `ender_dragon`: `a_dying_dragon_explodes_every_tick_and_bursts_from_tick_180_to_200` (199 plain, 21 emitters, offsets within ±4 and 2 ± 2), `the_dying_dragon_s_random_draws_three_floats_a_tick` (every explosion against a mirror of the dragon's random, and the random after the death).
  - `server`: `the_dragon_breaks_cobblestone_it_flies_through_but_not_end_stone` also sees plain explosions at the centres of cells inside the dragon's parts, and none where end stone holds it; `the_first_dragon_drops_12000_experience_and_a_later_one_500` sees 21 emitters and at least 199 plain explosions.
- **Decisions:** 199 plain explosions, not the brief's 200: on death tick 200 the dragon is removed in `tickDeath`, and vanilla skips `aiStep` for a removed entity.
- **Test steps:** (the End, a new world, `/difficulty normal`, diamond armour on)
  1. Build a wall of cobblestone and oak planks about 6 wide and 20 high under the dragon's circle (about 40 blocks from the centre, top at about 80 to 90). Where it tears through, small explosion flashes pop at the broken blocks (**1**).
  2. Kill the dragon. While it rises under the white rays, explosion flashes flicker around its body every tick, and in its last second big clusters of explosions burst (**2**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:diamond_sword+sharpness=5,minecraft:cobblestone*128,minecraft:oak_planks*64
```

### 29. Minecraft players' hits knock the flying ender dragon back

Previous commit: 8b6c9f09

- **Changed:**
  - `minecraft_terrain` `end_crystal.rs`: `EndDamage.from: Option<DVec3>`, where the hit came from; `EndDamage::attack(amount, player)` keeps its two arguments and `from_position(at)` sets it, explosions keep `None`.
  - `ender_dragon.rs` `hurt_part`: a full hit of kind `Attack` with a `from` pushes the dragon with `knockback(velocity, false, from.x − x, from.z − z)` (`LivingEntity.dealDefaultKnockback` with strength 0.4 from the dragon's own position, not the part's), unless it sits (`EnderDragon.knockback` returns while the phase is sitting). A killing hit still pushes it as it turns to dying, as vanilla's knockback runs inside `hurtServer` before the dragon sets its dying phase.
  - `minecraft_survival` `entities.rs`: `punch` passes the eye and `shoot` the shooter's position. The blame stays `PLAYER`, as before.
- **Tests:** `ender_dragon`: `a_hit_knocks_a_flying_dragon_away_from_the_attacker` (halved motion less 0.4 along the unit direction, y kept; no push without a source; a killing hit pushes), `a_sitting_or_dying_dragon_is_not_knocked_back`, `an_explosion_does_not_knock_the_dragon_back`, `only_a_full_hit_knocks_back_not_a_partial_one_in_the_cooldown`.
- **Decisions:**
  - Only a full hit knocks back. The brief said a partial hit in the invulnerability window does too, but javap of 26.3 `LivingEntity.hurtServer` shows the whole block with `broadcastDamageEvent`, `markHurt` and `dealDefaultKnockback` behind the "full hit" flag, which the partial branch clears.
  - `from_position` instead of a third argument to `attack`, so the 26 callers (most of them tests, some in files other tracks edit) stay as they are.
  - MW2 bullets knock the dragon back like a player's melee hit, from the shooter's position, since a bullet on a mob is already an attack from where it was fired. Grenades and other blasts are explosions, which `#no_knockback` exempts.
- **Test steps:** (the End, a new world, `/difficulty normal`, diamond armour on)
  1. Shoot the flying dragon from the side with a rapid-fire weapon as it passes: it lurches away from you with each burst that lands after its half-second cooldown.
  2. Hit its head with the sword when it swoops low: it jolts away from you.
  3. Shoot all ten crystals and wait for it to land on the portal; shoot it while it sits: it does not move.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:diamond_sword+sharpness=5
```

### 30. Minecraft fetches the End's three music tracks from Mojang

Previous commit: 429b4266

- **Changed:**
  - `assets` `minecraft_setup.rs`: `END_MUSIC` names `music/game/end/boss.ogg`, `the_end.ogg` and `alpha.ogg`; `wanted_asset(key)` is the first fetch's filter (`sounds.json`, English text and every sound but music and records) plus these three, so a first-time fetch already brings them. `begin_end_music(root)` runs once per process on a `minecraft-music` thread: it reads `root/asset-index.json` and, for each of the three tracks missing from the resource pack or with the wrong SHA-1, fetches `resources.download.minecraft.net/<hash[0..2]>/<hash>` with the existing `fetch` (curl, hash checked) into an `.ogg.partial` file and renames it into place. It logs `Minecraft End music ready`, or a warning; the game never waits for it or fails on it.
  - `minecraft_map.rs` `prepare`: with the files in place it starts `begin_end_music`; without them it starts the full fetch as before.
- **Tests:** `minecraft_setup`: `only_the_end_music_is_fetched_of_mojangs_music` (the three tracks, sounds and English text yes; menu music, records and other languages no), `the_end_music_lands_in_the_resource_pack_path`, `missing_end_music_is_fetched_by_hash_and_music_already_here_is_kept` (a stand-in index and fetcher: only the two missing tracks are asked for, by their object URL, through a `.partial` file; a second run fetches nothing; no index is an error).
- **Decisions:** only these three tracks (22 MB) are fetched. The rest of Mojang's music (259 MB) stays on their servers, so the overworld and the Nether stay silent. Nothing is committed: the files come from Mojang's servers at run time like every other asset. A fetch that fails offline is tried again on the next start.
- **Test steps:**
  1. Start the game once with an internet connection and open the Minecraft map. In `C:\Games\Mashup\2010-Rust-Rewrite-Mashup\iw4l-artifacts\minecraft-26.3\resourcepacks\local\minecraft-26.3\assets\minecraft\sounds\music\game\end\` there are now `alpha.ogg`, `boss.ogg` and `the_end.ogg` (about 7, 4 and 11 MB), and the game log says `Minecraft End music ready` (**1**, the folder).
  2. Start the game again: the log says `Minecraft End music ready` again at once, and the files' dates do not change.
- **Kit:** none.

### 31. Minecraft music streams from its Ogg file

Previous commit: d3eea377

- **Changed:**
  - New `audio` `minecraft_music.rs`: `McMusicStream`, an audio source that decodes Ogg Vorbis with Symphonia one packet at a time as rodio pulls samples, into one reused `SampleBuffer` (it grows only when a packet is larger), so a 13-minute track is never decoded whole. A decode error or a change of channels or rate ends the track.
  - `McMusic { wanted, finished }` with `McMusicTrack { key, bytes, volume }`, and the `play_minecraft_music` system: when `wanted` names another key than the voice playing, it stops that voice and starts the new one (not positional, volume `volume x MIX_GAIN x master volume`, kept up to date); when `wanted` is empty it stops; when the voice ends it sets `finished` to its key and clears `wanted`, so a track never restarts by itself. Registered from `minecraft.rs`'s `register` (`MIX_GAIN` is now `pub(crate)`), exported from `lib.rs`.
  - The track log's entry 30 lost the backslashes in its folder path when it was written; this commit puts them back.
- **Tests:** `minecraft_music`: `the_music_stream_decodes_the_same_samples_as_the_whole_file_decoder` (with `MINECRAFTOSS_ROOT`, on `ambient/weather/end_flash1.ogg`: the same channel count, rate and every sample as `decode_audio_bytes`), `a_stream_that_is_not_ogg_ends_at_once`.
- **Decisions:** the music volume is the game's master volume times the mix gain the other Minecraft sounds use (vanilla's defaults put music and sounds both at 100%). Nothing asks for music yet; the next commit adds vanilla's music manager.
- **Test steps:** none in game on its own (the next commit plays the music).
- **Kit:** none.

### 32. Minecraft plays the End music, the dragon's boss music and the credits music like vanilla

Previous commit: a72a4ff7

- **Changed:**
  - New `render_anim` `minecraft_music.rs`: vanilla's `Musics` (`CREDITS` `music.credits`, `END_BOSS` `music.dragon`, both 0/0 and replacing; `END` `music.end`, 6000 to 24000 and replacing), `situational` (`Minecraft.getSituationalMusic`: the credits first, then the boss music in the End while a shown boss bar plays music, then the End's music; nothing elsewhere) and `MusicManager`, vanilla's `MusicManager.tick` step by step: a delay of 100 at the start; no situational music keeps the delay at least 100 and lets the current track play on; a replacing track that differs stops the current one and draws `Mth.nextInt(0, minDelay / 2)`; a track that is no longer playing is forgotten and the delay drops to `nextInt(min(minDelay, 24000), min(maxDelay, 24000))`; the delay is capped at `maxDelay`; it counts down only with nothing playing and no level loading, and a start sets it to `i32::MAX`. `stopPlaying` (a dimension change, or the credits closing while their music plays) stops and sets the delay to the next song delay plus 100. Its random is a `LegacyRandom` seeded from the clock when the manager is made (`RandomSource.create()`).
  - `minecraft_sounds.rs`: `Sounds::music_file(packs, event)` resolves a music event's `sounds.json` entry to its file and volume, and logs once when the file is not in the pack.
  - `frame` `McBossBar.music` and a fourth `McBossBar::new` argument; the dragon's bar has it (`EnderDragonFight`'s `setPlayBossMusic(true)`).
  - `minecraft_world.rs`: `Runtime.music`; the `update` system takes `audio::McMusic`; the 20 Hz environment tick feeds the manager the local player's dimension, the credits, the boss bars' music flag, whether the current track has finished and the loading screen, and a start puts the track in `McMusic.wanted` (a missing file counts as finished at once); a stop clears it. `stop` resets the manager, and outside the Minecraft world nothing is wanted.
- **Tests:** `minecraft_music`: `the_situation_is_the_credits_then_the_boss_then_the_end`, `the_boss_music_starts_on_the_tick_its_bar_shows_and_loops`, `the_end_music_replaces_the_boss_music_within_3000_ticks_of_the_kill` (40 seeds), `arriving_in_the_end_without_a_fight_waits_6100_to_24000_ticks` (40 seeds), `the_credits_music_replaces_anything_and_stops_with_the_credits`, `a_dimension_change_stops_the_music`, `nothing_counts_down_while_a_level_loads`, `outside_the_end_nothing_new_starts_but_the_current_track_plays_on`. `minecraft_sounds`: `the_end_music_events_resolve_to_their_files_when_fetched` (with `MINECRAFTOSS_ROOT`). `frame`: the boss bar test passes and checks the new flag.
- **Decisions:**
  - Arriving in the End without a fight waits 6100 to 24000 ticks, not the brief's 24100: vanilla caps the delay at the track's `maxDelay` (24000) on the next tick.
  - A stop takes effect at once (vanilla's sound engine reports a stopped track as active until its next tick, which re-rolls the short delay for a tick or two).
  - No "now playing" toast, no music volume slider (vanilla's `fadePlaying`): the music follows the master volume. The manager is client state, driven by the local player only, as in vanilla.
- **Test steps:** (after entry 30's fetch; a new world in the End: `IW4L_MINECRAFT_WORLD=c3music`, `IW4L_MINECRAFT_DIMENSION=the_end`, `/difficulty normal`)
  1. As soon as the pink "Ender Dragon" bar shows, the boss music starts. When it ends (about 4 minutes), it starts again at once.
  2. Kill the dragon. When its bar goes, the boss music stops, and within two and a half minutes the calmer End music starts (**1**, the moment it starts, if you can).
  3. Place the 4 end crystals on the exit portal's rim (one on each side). When the new dragon's bar shows, the boss music cuts in at once.
  4. Kill that dragon too, then walk into the exit portal. The credits play their own music from the start. Press Escape: the music stops, and in the overworld no music plays.
  5. Go back to the End through a stronghold portal (or set `IW4L_MINECRAFT_DIMENSION=the_end` again with the dragon dead): no music for the first 5 minutes, then the End music within 20 minutes.
  6. Start without an internet connection on an install that never fetched the music: the game plays as before, without music, and the log says `Minecraft music … is not in the pack` once per track.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:diamond_sword+sharpness=5,minecraft:end_crystal*4
```

### 33. The Minecraft state doc covers the End flashes, the dragon's effects and the End music

Previous commit: 573af186

- **Changed:** `docs/MINECRAFT-SURVIVAL.md`: the End section describes the sky flashes, the breath particles (fireball trail and burst, clouds, the mouth while landing and flaming), the explosion particles where the dragon breaks blocks and while it dies, the knockback of players' hits and the music. The limitations lose "the End sky has no flashes" (track A edits the same line for the gateways: merge by hand), the dragon's missing knockback, mouth breath, fireball trail, wall explosion and death particles, "There is no boss music" and the credits' missing music; the dissolve and the overworld and Nether music stay listed.
- **The jshell lines** for the End flash test values are in entry 24.
- **Not done: c3 group 9** (the dragon's dissolve while it dies). It was optional and needs a second texture sample in the entity shader, a dissolve range in the chunk mesh and an atlas offset in `render_gpu`, in files tracks A and C2 also edit; it moves to a later round and stays in the limitations.
- **Tests:** none (docs only).
- **Test steps:** none beyond entries 28 to 32.
- **Kit:** none.

### Group c3-sky-dragon-2, combined kit

```
IW4L_MINECRAFT_KIT=minecraft:golden_apple*16,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:diamond_sword+sharpness=5,minecraft:cobblestone*128,minecraft:oak_planks*64,minecraft:end_crystal*4
```

## Group c5-rest-1: totem animation, trapped chest signal, book contents, lecterns (c5 groups 5-8)

### 34. Minecraft totems pop with vanilla's floating totem and glitter

Previous commit: fba23ac4

- **Changed:**
  - `render_anim` `minecraft_hand.rs`: `ItemActivation { ticks, off }` (vanilla `GameRenderer.displayItemActivation`: 40 ticks, offsets `nextFloat()·2 − 1`) and `activation_pose(ticks, partial, off, aspect)`, `ScreenEffectRenderer.renderItemActivationAnimation` step by step: t = (40 − ticks + partial)/40, the quintic curve times π, the translation by the offsets times 0.3·|sin 2a| (x also times the aspect) and z −10 + 9 sin a, scale 0.8, Y by 900·|sin a|, X and Z by 6 cos 8t degrees.
  - `minecraft_world.rs`: the activation lives in `Runtime` (client-local, as vanilla's), ticks with the hand's ticks and resets when the world closes. The `hold_off_death` wrapper now says whether a totem popped; both callers (the hazard tick and the MW2 damage pop) then call `pop_totem`, which starts the activation and the glitter. The hand view is built while the activation runs even with an empty hand, an MW2 gun or in third person (where only the totem is drawn), unless the HUD is hidden (`UiDraw`): the totem in its `fixed` display, full bright, after the held items in the same hand mesh.
  - `minecraft_terrain` `ambient_particles.rs`: `Kind::Totem` (vanilla `TotemParticle`: gravity 1.25, friction 0.6, size ×0.75, lifetime 60 + `nextInt(12)`, one in four yellow else green, `glitter_7` to `glitter_0` by age, translucent and full bright, alpha 1 − (age − lifetime/2)/lifetime after half its life, with block collision). `Body { feet, width, height }`, `totem_emitter(key, body)` and `tick_emitters(body_of)`: vanilla `TrackingEmitter`, 16 tries in the unit cube a tick, those inside the unit sphere start at (x + w·x/4, y + h·(0.5 + y/4), z + w·z/4) with motion (x, y + 0.2, z); once on creation and then each tick, 30 emissions in all, following the body as it moves. An emitter whose body is gone stops.
  - `minecraft_survival` `entities.rs`: `track_body(key, feet, width, height)` (the local player's, every frame, under its UUID key) and `totem_pop(key, ...)`; `tick_scene` ticks the emitters before the particles.
- **Tests:**
  - `minecraft_hand`: `the_activation_rises_spins_and_returns_like_vanilla` (at t = 0 and t = 1 the totem sits 10 in front with no offset; at t = 0.25, 0.5, 0.75 and a partial tick the position and the spin match the formula; at t = 0.5 it is about 1 block from the eye; 40 ticks then gone).
  - `ambient_particles`: `a_totem_emitter_runs_thirty_ticks_inside_the_body` (30 emissions of 1 to 16, none after; every start point inside the body box at the sphere offset and its motion; a lost body stops the emitter), `totem_glitter_is_mostly_green_and_fades_after_half_life` (about a quarter yellow in 2000, the colour ranges, lifetime 60 to 71, size, the first tick's motion, alpha 1 to half life then the vanilla fade).
- **Decisions:**
  - The totem drawn is always `totem_of_undying` (vanilla's `findTotem` falls back to it; it is the only item with `death_protection` the kit gives).
  - It is drawn in the hand pass with the hand's 70 degree projection, so an MW2 gun's view model can cover part of it when it swings close; vanilla draws it over its own hand.
  - The emitter keys bodies by the player's UUID key (`level::opener_key`); remote players' pops wait for track E **[E]**.
- **Test steps:** (survival, open ground)
  1. Put a totem in the offhand and `/tp ~ ~40 ~`. On landing the totem pops: a big totem flies from the middle of the screen towards you, spins and drops back over 2 seconds, and green and yellow glitter bursts around your body for about 1.5 seconds and follows you if you walk (**1**, totem mid-spin).
  2. Select an MW2 gun, put the second totem in the offhand and repeat: the floating totem shows the same way.
  3. Third totem: press F5 (third person) with the HUD shown and repeat: the floating totem shows and the glitter surrounds the body (**2**).
  4. Fourth totem: press F1 to hide the HUD and repeat: only the glitter shows, no floating totem.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:totem_of_undying,minecraft:totem_of_undying,minecraft:totem_of_undying,minecraft:totem_of_undying
```

### 35. Minecraft trapped chests power redstone by their number of viewers

Previous commit: 022c7b4e

- **Changed:**
  - Vendored `world` `level/mod.rs` (MinecraftOSS patch): `Level.chest_openers` (viewers per chest position, not saved, as in vanilla), dropped when the block entity at that position goes.
  - Vendored `level/redstone.rs` (MinecraftOSS patch): `Kinds.trapped_chest`; `own_signal` of a trapped chest is its viewers clamped to 0..15 (`TrappedChestBlock.getSignal`), `state_direct_signal` the same only for `Up` (it strongly powers the block below), and `set_chest_openers(pos, count)` stores the count and, when it changed on a trapped chest, updates the neighbours at the chest and below it (`TrappedChestBlockEntity.signalOpenCount`). The trapped chest is a signal source by its block state data; the comparator still reads its fill.
  - `minecraft_terrain` `server.rs`: `Command::ChestOpeners { pos, count }`, `ServerHandle::chest_openers`, `ServerSim::set_chest_openers`.
  - `minecraft_survival` `level.rs`: `OpenerOutcome.trapped` names a trapped chest's position and its viewer count for every opener event (open, close, the 5-tick recheck and a player leaving the level); `Entities::chest_openers` sends it. `render_anim` sends it from both opener loops. The count is per `Level` and per player key already, so friends' viewers add up **[E]**.
- **Tests:**
  - `server`: `a_trapped_chest_powers_by_viewers_and_strongly_below` (counts 1, 3, 20, 0 give 1, 3, 15, 0 on every side; the block below is strongly powered and the block beside is not; a plain chest gives nothing; a chest placed again starts at 0), `opening_a_trapped_chest_lights_a_lamp_beside_it` (a lamp beside and a lamp under the block below light on 1 viewer and go out 4 ticks after 0; the command reaches the server thread).
  - `level`: `a_double_trapped_chest_counts_on_both_halves` (both halves get the count on open, a second viewer, a viewer leaving and the close; a plain chest none).
- **Decisions:** the brief's vendored test lives in `minecraft_terrain`'s server tests, because the vendored `world` crate has no level fixture with the block data; it calls the vendored code directly.
- **Test steps:** (survival, flat ground)
  1. Place a trapped chest, a redstone lamp right beside it and a line of 15 redstone dust from its other side. Open the chest: the lamp lights and exactly one dust next to the chest is lit. Close it: both go off (**1**, open).
  2. Place a second trapped chest on a block of cobblestone with a redstone lamp under the cobblestone: opening the chest lights that lamp (strong power down).
  3. A comparator facing out of a trapped chest reads its fill like a normal chest: put one stack in, and the dust after the comparator lights 1 block.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:trapped_chest*4,minecraft:redstone_lamp*8,minecraft:redstone*64,minecraft:comparator*4,minecraft:cobblestone*64
```

### 36. /give and the kit take writable and written book contents

Previous commit: 6570c43c

- **Changed:**
  - New vendored `world` `book.rs` (a MinecraftOSS addition): `content(component, snbt)` reads `writable_book_content` and `written_book_content` with the core SNBT parser and checks them like vanilla's codecs (`WritableBookContent`: up to 100 pages of at most 1024 characters, each a string or `{raw, filtered}`; `WrittenBookContent`: `title` (at most 32, string or `{raw, filtered}`) and `author` required, `generation` 0 to 3, `pages` text components (string, compound or list, strings up to 32767) or `{raw, filtered}`, `resolved`). It returns the component in the shape vanilla saves (`{raw: ...}` pages and title, `generation` and `resolved` left out at their defaults). Also `page_count`, `has_content`, `raw_page` for the lectern, and `split_entries`/`entry_count`: kit lines split on commas outside `[]`, `{}` and quotes, and the `*count` is read only after the component list.
  - `minecraft_commands` `args.rs`: `ItemArg.components` (the parsed book components as JSON). The item component list takes both book components next to the enchantment ones; a bad one is `arguments.item.component.malformed` with the reason, at the value's start. Other components stay `arguments.item.component.unknown`. `kit.rs`: `kit_entries` uses the new splitter.
  - `minecraft_terrain` `crafting.rs`: `kit` uses the splitter, `kit_components(entry)` takes the `[...]` list off an entry (enchantments `+` and potions `@` are only read before it, so page text with `,`, `*`, `+` or `@` survives), and `with_components(stack, extra)` merges them into the stack. `give_kit` (the starting kit) uses both.
  - `minecraft_survival` `commands.rs`: `/give` and `/kit` put the components on the stack.
- **Tests:**
  - `world` `book`: `a_written_book_keeps_title_author_and_pages_in_the_saved_shape`, `writable_pages_are_plain_or_raw_strings`, `bad_book_content_is_refused`, `page_counts_and_content_follow_the_components`, `kit_entries_split_outside_components_and_quotes`.
  - `args`: `a_written_book_takes_title_author_and_pages`, `writable_pages_take_plain_and_raw_strings`, `bad_book_content_is_malformed`.
  - `kit`: `kit_entries_keep_commas_inside_components`; `crafting`: `give_kit_puts_book_pages_on_the_stack` (pages with `,`, `+`, `@` and `*`); `commands` (survival): `give_puts_book_pages_on_the_stack`.
- **Decisions:**
  - The parser lives in the vendored `world` crate because both `minecraft_commands` and `minecraft_terrain` already depend on it and it has the SNBT parser; that keeps `Cargo.lock` unchanged.
  - Unknown keys inside a book component are ignored, as vanilla's record codecs ignore them (the brief said to refuse them).
  - A kit entry with a bad book component gives the book without it (the kit has no error channel); `/give` reports it.
- **Test steps:** (survival)
  1. Type `/give @s minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}]`: a written book appears in the inventory (**1**, the chat line).
  2. Type `/give @s written_book[written_book_content={author:"Owner"}]`: the chat shows a red line that starts with "Malformed 'minecraft:written_book_content' component" and ends with "No key title", and nothing is given.
  3. With the kit below, the starting inventory holds a written book and a book and quill (the pages show on the lectern in the next entries).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:written_book[written_book_content={title:"Kit",author:"Owner",pages:["One, two + three","Page two"]}],minecraft:writable_book
```

### 37. Minecraft lecterns hold books, pulse on page turns and feed comparators

Previous commit: 92d24293

- **Changed:**
  - New vendored `world` `level/lectern.rs` (a MinecraftOSS addition), on the server thread's `Level` and the block entity it already saves (`Book`, `Page`): `lectern_place_book` (only on a lectern without a book: the book, page 0, comparators told, `has_book=true, powered=false`, the block below updated, `item.book.put` at the block; vanilla `tryPlaceBook`/`resetBookState`), `lectern_set_page` (clamped to the pages; a real change sets `powered=true`, updates below, schedules the block tick in 2 and pushes level event 1043; vanilla `setPage`/`signalPageChange`), `lectern_take_book`, `lectern_view`, `signal` (vanilla `getRedstoneSignal`: `floor(page/(count − 1)·14)`, or 14 with one page or none, plus 1 when the book has `writable_book_content` or `written_book_content`) and `drop_point`.
  - Vendored `level/redstone.rs`: `Kind::Lectern` (`LecternBlock`): own signal 15 while powered, direct signal 15 only up, the scheduled tick turns it off and updates below, the removal of a powered one updates below, and its comparator output reads the block entity (`has_analog_output` counts it). `level/drops.rs`: breaking a lectern with a book drops the book as one item at `(x + 0.5 + 0.25·facing x, y + 1, z + 0.5 + 0.25·facing z)` with the default pickup delay (vanilla `LecternBlockEntity.preRemoveSideEffects`), instead of logging it as unsupported. Vendored `core` `block_entity.rs`: `Page` is clamped to the book's pages on load (vanilla `loadAdditional`), with `lectern_book_pages` and `lectern_book_has_content`.
  - New `minecraft_terrain` `lectern.rs`: `BookStack`, `LecternAction` (`PlaceBook`, `Open`, `Button(i32)`, `Close`), `LecternView` and the button numbers; `pages_text` flattens pages to plain text for the screen (`text`, `translate` with its `fallback` or key, `keybind`, `extra`); `book_item_use`. `server.rs`: `Command::Lectern { player, pos, action }`; `PlaceBook` answers through `item_uses` like a bucket; the server keeps `lectern_viewers` per player id and answers in `Output.lecterns`: `Open` the view (or a closed one on an empty lectern), page buttons 1, 2 and 100 + n answer every viewer of that lectern when the page changed, button 3 takes the book (the taker's view carries it) and closes every viewer, and each tick closes viewers whose lectern lost its book (vanilla `stillValid`). `level_events.rs`: event 1043 plays `item.book.page_turn` at the block (1.0, 0.9 + r·0.1). `bucket.rs`: `Change::Lectern`.
  - `minecraft_survival` `entities.rs`: `lectern(pos, action, sent)` (the local player is server player `PLAYER` **[E]**), `book_stack`, and `lecterns` (this player's views).
  - `render_anim`: a right-click (main hand, not crouching with an item) on a lectern with a book sends `Open` and swings; one with a written book or book and quill in the hand on an empty lectern sends `PlaceBook` and takes the book from the hand when the server answers (creative keeps it). A taken book goes into the inventory or drops at the feet.
- **Tests:**
  - `lectern`: `the_comparator_reads_page_steps_like_vanilla` (3 pages: 1, 8, 15; one page: 15; an empty book and quill: 15; a written book without content: 14; no book: 0; 15 pages on page 7: 8), `written_pages_flatten_to_text`.
  - `server`: `a_book_placed_by_a_player_reaches_the_lectern_and_its_comparator` (through the server thread: placed, a second book refused, the client sees `has_book`; then a plain book refused, the book sound, page 0, the comparator reads 1), `a_page_turn_pulses_two_ticks` (only an opened viewer turns pages, the view's page, `powered` and signal 15 beside and strongly below for exactly two ticks, event 1043, comparator 8 and 15, no turn past the last page, buttons 1 and 100 + n, Close), `take_book_closes_every_viewer`, `a_broken_lectern_closes_its_viewers_and_drops_its_book_above` (the book at (8.5, 201, 8.25) for a north-facing lectern, pickup delay 10).
  - `core` `block_entity`: `page_is_clamped_on_load`; `level_events`: `a_lectern_page_turn_rustles_at_the_lectern`.
- **Decisions:**
  - A written book without `written_book_content` reads 14, not the brief's 0: javap of 26.3 `getRedstoneSignal` uses 1.0 when the page count is at most 1, and adds 1 only for a book with content.
  - A book with no pages keeps page 0 where vanilla's `Mth.clamp(page, 0, count − 1)` would give −1; no screen can ask for another page of it.
  - The Take Book button's `mayBuild` check is left to the client that sends it (the server thread does not know game modes yet) **[E]**.
  - The vendored `world` crate has no level fixture, so the lectern's level tests run in `minecraft_terrain`'s server tests against the vendored code.
  - The book screen comes in the next commit: until then a right-click on a lectern with a book opens nothing you can see.
  - `minecraft_survival`'s `the_end::tests::an_arrival_rebuilds_the_platform_once_its_chunks_are_in` failed once and passed on the rerun: it reads block changes from a server job and races the per-batch change output. It is older than this commit.
- **Test steps:** (survival; kit below)
  1. Place a lectern on the ground. Put a comparator on the ground facing away from the lectern's side, then 15 redstone dust in a line from it. Right-click the lectern with the written book: the book lies open on the lectern with the book sound and leaves your hand; the dust shows 1 lit block (**1**).
  2. Place the second lectern and right-click it with the book and quill: it lies on the lectern, and a comparator facing out of this lectern lights 15 dust (vanilla does the same for an empty book and quill).
  3. Crouch with a book in hand and right-click a lectern: nothing goes on it.
  4. Leave the world and load it again: both books are still on their lecterns and the dust still shows 1 and 15.
  5. Break the first lectern: the written book pops out on top of where it stood, next to the lectern item (**2**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}],minecraft:writable_book,minecraft:lectern*2,minecraft:comparator*2,minecraft:redstone*32,minecraft:redstone_lamp*2,minecraft:cobblestone*16
```

### Group c5-rest-1, combined kit

```
IW4L_MINECRAFT_KIT=minecraft:totem_of_undying,minecraft:totem_of_undying,minecraft:totem_of_undying,minecraft:totem_of_undying,minecraft:trapped_chest*4,minecraft:redstone_lamp*8,minecraft:redstone*64,minecraft:comparator*4,minecraft:cobblestone*64,minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}],minecraft:writable_book,minecraft:lectern*2
```

## Group c5-rest-2: the lectern book screen, rain and snow streaks, docs (c5 groups 9-11)

### 38. Minecraft lecterns open vanilla's book screen with page turning and Take Book

Previous commit: 268fac68

- **Changed:**
  - `frame` `minecraft_ui.rs`: `McScreen::Lectern`, `McBook { pages, page, take }` in `MinecraftUi.book`, and `MinecraftUi.ui_sounds` (UI sounds the HUD asks the world to play).
  - New `hud` `minecraft_book.rs`, vanilla `BookViewScreen` and `LecternScreen` (javap of 26.3): `textures/gui/book.png` (192 x 192) at left (width − 192)/2, top 2; page text from (left + 36, top + 30), split at 114 pixels with the gui font's word wrap (`McFont::split`), at most 128/9 = 14 lines of 9, black without shadow; `book.pageIndicator` ("Page %1$s of %2$s", page + 1 of at least 1) right-aligned at (left + 148, top + 16), black; the page buttons 23 x 13 at (left + 43, top + 157) back and (left + 116, top + 157) forward with `widget/page_backward`, `widget/page_forward` and their `_highlighted` sprites on hover, back only after page 0 and forward only before the last page; "Done" and "Take Book" 98 wide at (width/2 − 100, 196) and (width/2 + 2, 196), or one 200-wide "Done" for a player who may not build, drawn with `widget/button` (nine-slice) and `widget/button_highlighted`, white centred labels with shadow. A click on a page button presses vanilla's menu button 1 or 2 and plays `item.book.page_turn` as a UI sound; Page Up and Page Down (and the pad's shoulder buttons) press the same buttons without a sound, as vanilla's `keyPressed` does; "Take Book" is button 3 with `ui.button.click`; "Done" closes. `hud` `minecraft_inventory.rs` draws this screen instead of the slot screens while it is open; Esc closes it, the inventory key does not (vanilla's book screen is not a container screen).
  - `render_anim` `minecraft_gui.rs`: the atlas loads `gui/book.png` as the `book` sheet, the four page sprites and `widget/button`, `widget/button_highlighted`, `widget/button_disabled`, and the `book.`, `lectern.` and `gui.done` texts.
  - `render_anim` `minecraft_inventory.rs`: `InventoryUi` keeps the open lectern; `show_lectern` opens or refreshes the screen from a server view, `lectern_closed` closes it, the HUD's `Choose(n)` become `LecternAction::Button(n)` and closing sends `LecternAction::Close` once (`take_lectern_actions`).
  - `render_anim` `minecraft_world.rs`: the server's lectern views open the screen (`take` = the player's mode may build), refresh its page for every viewer when the page turns, and close it when the book is taken or the lectern loses its book; the screen also closes beyond reach or when the block is no longer a lectern with a book; the HUD's UI sounds are played; leaving a level sends the close.
- **Tests:**
  - `hud` `minecraft_book`: `book_layout_matches_vanilla` (gui size 427 x 240: the book at 117, 2, text at 153, 32, the indicator's right edge at 265, 18, back 160, 159, forward 233, 159, Done 113, 196 and Take Book 215, 196 at 98 wide, or Done 113, 196 at 200 wide), `page_buttons_hide_at_the_ends`, `clicks_and_page_keys_press_the_vanilla_menu_buttons`, `pages_wrap_at_114_pixels_and_14_lines`.
  - `render_anim` `minecraft_gui`: `the_book_sheet_and_page_sprites_load` (real 26.3 pack: the 256 sheet in the atlas, the 23 x 13 page sprites, the button's 200 x 20 nine-slice with border 3, the texts); `minecraft_inventory`: `a_lectern_screen_sends_its_buttons_and_closes_once`.
- **Decisions:**
  - The page shown is the server's: a click sends the button and the page changes when the server's view comes back, as vanilla's `LecternScreen` waits for the menu data.
  - The inventory key (E) does not close the book screen, because vanilla's `BookViewScreen` only closes on Esc (the brief said E too).
  - A view that arrives while another screen is open is refused and the server is told the screen closed.
  - "Take Book" shows for players whose game mode may build (survival and creative), as vanilla's `mayBuild`; the server still trusts the client on it **[E]**.
- **Test steps:** (survival; kit below)
  1. Place a lectern, put a comparator on the ground facing away from its side and 15 redstone dust in a line from the comparator, and a redstone lamp right beside the lectern on another side. Right-click the lectern with the written book: it lies on the lectern and the dust shows 1 lit.
  2. Right-click the lectern: the book screen opens with "Page one text.", "Page 1 of 3" at the top right, the forward arrow at the bottom right of the page and the Done and Take Book buttons under the book; the world keeps running behind it (**1**).
  3. Click the forward arrow: a page sound, page 2, the lamp flashes and the dust shows 8 lit. Click it again: page 3, the forward arrow is gone and 15 dust are lit (**2**, page 2 with the dust). Press Page Up: back to page 2, without a sound.
  4. Press E: the screen stays open. Press Esc: it closes. Open it again and click Done: it closes.
  5. Open it again and click Take Book: the screen closes, the written book is back in your inventory, the lectern is empty and the dust is dark.
  6. Put the book and quill on the second lectern and open it: "Page 1 of 1", an empty page and no arrows.
  7. `/gamemode adventure`, open the second lectern: only one wide Done button. `/gamemode survival`.
  8. Open a lectern and walk away from it: the screen closes about 8 blocks away, like a chest's.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}],minecraft:writable_book,minecraft:lectern*2,minecraft:comparator*2,minecraft:redstone*32,minecraft:redstone_lamp*2
```

### 39. Rain and snow fall around the camera like vanilla's weather renderer

Previous commit: c597fe17

- **Changed:**
  - `minecraft_terrain` `rain.rs`: `Precipitation { None, Rain, Snow }`, `precipitation_of` and `precipitation_at` (vanilla `Biome.getPrecipitationAt`: none without precipitation, snow below temperature 0.15, else rain); `rains_at` is now `precipitation_at == Rain`, so splashes and the rain sound are unchanged.
  - New `minecraft_terrain` `weather.rs`, vanilla `WeatherEffectRenderer` (javap of 26.3): `columns` walks the square of radius 10 around the camera block, z then x; the column runs from max(camera y − 10, height) to max(camera y + 10, height) of the `MOTION_BLOCKING` heightmap and is skipped when they are equal (a roof above the camera's reach) or the biome at the camera's height has no precipitation; each column seeds a legacy random with (x²·3121 + x·45238971) ^ (z²·418711 + z·13761) and takes its light at (x, max(camera y, height), z). Rain: speed 3 + `nextFloat`, v offset (−((time & 131071) + ((x²·3121 + x·45238971 + z²·418711 + z·13761) & 255) + partial)/32·speed) mod 32. Snow: u = `nextDouble` + t·0.01·`nextGaussian`, v = −((time & 511) + partial)/512 + `nextDouble` + t·`nextGaussian`·0.001, light (l·3 + 15)/4 per channel. `mesh` builds one camera-facing quad a column from vanilla's 32 x 32 size table (half widths −dz/len/2 and dx/len/2, none for the camera's own column, where vanilla's table is 0/0), v = y·0.25 + offset with the bottom height on the top edge, alpha lerp(min(dist²/r², 1), 1.0 rain or 0.8 snow, 0.5) times the rain level; rain columns first, then snow. `weather_texture` puts `environment/rain.png` and `environment/snow.png` side by side (128 x 256).
  - `minecraft_terrain` `ambient_particles.rs`: `precipitation_at` with the particles' biome temperature. `minecraft_survival` `entities.rs`: `partial_tick` and `weather_mesh` (the level's ticks and partial tick, the weather's rain level, the motion-blocking height, the level's light) **[E: per client camera and its level's weather]**.
  - `render_anim` `minecraft_world.rs`: the rain and snow texture loads with the world; `MinecraftWorldView.weather` is rebuilt each frame. `render` `extract.rs` copies the mesh and the texture (converted once). `render_gpu` `minecraft_world.rs`: `MinecraftWorldFrame.weather` (36-byte vertices) and `weather_texture` at binding 11, a 13th `weather` pipeline (no culling, depth test without depth write, alpha blend) whose fragment samples rain or snow by the vertex flag with the wrap done in the shader, multiplies by the lightmap and the alpha, discards below 0.1 (vanilla `particle.fsh`) and applies the terrain fog; it draws after the cracks and before the hand.
- **Tests:**
  - `weather`: `rain_columns_scroll_at_their_seeded_speed` and `snow_columns_drift_by_gaussians` (seeds, offsets, speed and u and v checked against jshell's `java.util.Random`, below), `quads_face_the_camera_and_fade_to_half_at_the_edge`, `roofed_columns_draw_nothing`, `only_cold_biomes_snow`, `the_texture_puts_snow_to_the_right_of_rain`, `the_vanilla_rain_and_snow_make_one_128_by_256_texture` (real 26.3 pack).
  - `render_gpu`: `weather_vertex_stride_matches_the_frame` (also the binding and the entry points); `terrain_shader_validates` covers the new WGSL.
  - jshell, with vanilla's formulas: rain (5, −3) at time 1000, partial 0.25: seed 222591356, offset 124, speed 3.7006378, v −2.0138092; rain (−12, 7) at 200000: v −29.427246; snow (−7, 12) at 123456, partial 0.5: u −1068.1278, v −250.92334; snow (3, 3) at 40: u 0.6006236, v 0.68376994.
- **Decisions:**
  - One draw with a per-vertex snow flag instead of two index ranges, and one 128 x 256 texture (rain left, snow right) wrapped in the shader instead of two bindings, which keeps the bind group to one new entry.
  - The weather clock is the level's tick count with the level's partial tick (`Entities::ticks`, `partial_tick`), as vanilla uses the level's game time.
  - The camera position is the player's eye; in third person vanilla centres on the detached camera, which is at most a few blocks away.
  - The radius is vanilla's default 10 (there is no weather radius option yet).
- **Test steps:** (survival; no kit needed beyond the blocks)
  1. In a plains or forest biome, `/weather rain`: rain streaks fall all around you up to about 10 blocks, fainter towards the edge (**1**). Look straight up: the streaks lean towards you from every side.
  2. Build a 5 x 5 roof 4 blocks above you and stand under it: no streaks fall inside, they still fall outside the roof's edge.
  3. `/time set midnight`: the streaks turn dark; place a torch next to you: the streaks near it are brighter.
  4. `/locate biome minecraft:snowy_plains` and `/tp` there (still `/weather rain`): slow white flakes drift down instead of streaks, without splashes (**2**).
  5. `/weather clear`: the streaks fade out with the rain over a few seconds.
  6. In the Nether and the End nothing falls.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:oak_planks*64,minecraft:torch*16
```

### 40. The Minecraft state doc covers powder snow, lecterns, trapped chests, totems, ladder jumps and weather streaks

Previous commit: fa1af90e

- **Changed:** `docs/MINECRAFT-SURVIVAL.md`:
  - Implemented: the rain and snow streaks and the powder snow fog (World and rendering); the ladder ground jump and powder snow movement (Movement); freezing and the totem activation (Survival and combat); trapped chest signals and lecterns with the book screen (Crafting, inventory, containers); book components on `/give` and in the kit format.
  - Saving: `freeze_damage` among the rules with an effect, `TicksFrozen` in the player file, lecterns' `Book` and `Page`, and "lectern books" dropped from what isn't simulated.
  - Limitations: the rain streaks, powder snow, ladder jump, totem, lectern and trapped chest lines are rewritten or removed; new ones for the fall-speed powder snow catch, mobs that don't freeze, the MW2 jump gate at ladders, books that can't be written and show plain text, the fixed weather radius, and the per-player state track E must still send (frost slowdown, other players' totem pops and snowflakes, lectern viewers).
- **Tests:** none (docs only).
- **Decisions:** the vendored-code count in CLAUDE.md (patched and added MinecraftOSS files) is left for the end of the track, as the c5 brief says.
- **Test steps:** none; read the changed sections.
- **Kit:** none.

### Group c5-rest-2, combined kit

```
IW4L_MINECRAFT_KIT=minecraft:written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one text.","Page two text.","Page three text."]}],minecraft:writable_book,minecraft:lectern*2,minecraft:comparator*2,minecraft:redstone*32,minecraft:redstone_lamp*2,minecraft:oak_planks*64,minecraft:torch*16
```

## Group c4-elytra: the elytra and the glide pose on the MW2 body (c4 groups 1-6)

### 41. Minecraft elytra wings are modelled and eased like vanilla

Previous commit: 237ef4c9

- **Changed:**
  - New `minecraft_terrain` `elytra_render.rs`, vanilla `ElytraModel`, `WingsLayer`, `ElytraAnimationState` and `AvatarRenderer.extractFlightData` (javap of 26.3):
    - `wings_texture`: only `minecraft:elytra` has a wings layer (`entity/equipment/wings/elytra`, 64 x 32).
    - `WingAngles` (the left wing's x, y, z rotation): `REST` 15, 0, -15 degrees; `CROUCH` 40, 5, -45; `SPREAD` 20, 0, -90. `target` while gliding lerps from rest to spread by k = 1 - (-normalised motion.y)^1.5 when falling (k = 1 otherwise), so a level or climbing glide spreads the wings and a vertical dive folds them back to rest.
    - `ElytraAnim`: starts at zero like vanilla's fields; `advance(seconds, gliding, crouching, motion)` runs whole 50 ms ticks (fall-fly ticks +1 or back to 0, then each angle moves 0.3 of the way to its target) and keeps the fraction; `wings()` lerps the old angles to the new by it; `scale()` is `fall_flying_scale(ticks + partial)` = clamp(t²/100, 0, 1) while gliding and 0 otherwise.
    - `flying_roll(look, motion)`: vanilla's signed angle between the horizontal motion and look, sign(look × motion) · acos(min(1, |cos|)), in map axes; `None` when either horizontal length squared is at most 1e-5.
    - `append_elytra`: the two wing boxes (10 x 20 x 2 at UV 22, 0, grown by 1) on a part frame in model pixels; the left wing pivots at (5, 0, 2) and turns Rz·Ry·Rx, the right one at (-5, 0, 2) with z and y negated and the texture mirrored. The 2 pixels are `WingsLayer`'s 0.125-block push toward the back. An enchanted elytra adds its range to `armor_glint`. Nothing is drawn when the atlas lacks the texture.
  - `mesh.rs`: the entity atlas takes the elytra texture when the pack has it.
- **Tests:** `elytra_render`: `only_the_elytra_has_wings`; `the_wings_fold_at_15_degrees_spread_to_90_and_crouch_at_40_and_45` (rest, crouch, level and climbing glide, a vertical dive back to rest, a 45-degree dive with k = 0.4054); `the_wings_ease_three_tenths_a_tick_from_zero` (one tick from zero gives x = 0.0785398, half a tick later the drawn wings are halfway, 30 + 20 ms frames match 50 ms ticks, a huge or NaN frame time is safe); `fall_flying_scale_is_ticks_squared_over_100`; `the_flying_roll_is_the_signed_angle_between_motion_and_look` (30 degrees left +0.5236, right -0.5236, at right angles ±1.5708, straight backward 0, 150 degrees +0.5236 because of the `abs`, a vertical dive `None`); `an_elytra_is_two_mirrored_wings_on_the_back` (12 faces, UVs inside the 22..46 x 0..22 panel, both boxes span x -6..6, y -1..21, z 1..5 at zero angles, the right wing is the left one mirrored with the same texture, every face faces out); `the_wings_turn_out_from_their_shoulder_pivots` (spread wings reach about 25 pixels out on each side, rest wings open at the bottom and swing back, the frame moves them); `an_enchanted_elytra_shimmers_with_the_armour_glint`.
- **Decisions:**
  - `append_elytra` takes no crouch flag: the brief keeps vanilla's crouch angles but not its 3-pixel crouch drop, because the MW2 spine already lowers when crouching.
  - Frame times are summed in ticks with a 1e-4 tick slack, so 30 + 20 ms frames make a whole tick despite float rounding; at most 100 ticks run in one frame (the ease has long settled by then).
- **Test steps:** none in game yet; nothing is drawn until the next commit.
- **Kit:** none.

### 42. A worn Minecraft elytra is drawn on the MW2 body's back

Previous commit: 05b42261

- **Changed:**
  - `minecraft_terrain` `body_render.rs`: `BodyRig.wings` (the wing angles, `None` draws them folded at rest). `append_body_gear` draws an elytra worn in the chest slot (`armor[1]`) on the torso's upper frame, the one the chestplate uses (from the neck, riding `j_spine4`, fitted to the MW2 torso by `GEAR_FIT`), with the armour's light, after the armour, and shimmering with the armour glint when enchanted. `armor_render.rs` is unchanged: the elytra stays outside `armor_asset`.
  - `render_anim` `minecraft_gear.rs`: `body_rig` leaves `wings` empty for now.
- **Tests:** `body_render`: `a_worn_elytra_is_drawn_on_the_back_and_shimmers_when_enchanted` (an enchanted elytra on a rig with a torso frame gives 12 faces, all in `armor_glint` and lit where the body is, the same as the wings at rest; spread angles move it; no torso frame or an elytra in the head slot draws nothing).
- **Decisions:**
  - `BodyRig` gets no `crouching` field (the brief had one): nothing in the drawing reads it, since vanilla's crouch drop of 3 pixels is left out (the MW2 spine already lowers) and the crouch angles come with the wing angles.
  - The wings use only the torso's upper frame, so they hang straight past the lower back as vanilla's do from the model root, rather than bending at the spine split.
- **Test steps:** (survival, `iw4l-survival.exe map minecraft:overworld`; kit below)
  1. Put the plain elytra in the chest armour slot and press F5. Back view: two grey wings folded on the back, from the shoulders to about the knees, slightly open at the bottom (**1**). F5 again for the front view: the wing edges show behind the arms (**2**).
  2. Open the inventory (E): the character in the box wears the wings (**3**).
  3. Swap in the Unbreaking III elytra: the wings shimmer with the broad armour glint. Put the diamond chestplate on instead: the wings are gone and the chestplate shows. Put the plain elytra back.
  4. Crouch and glide: nothing changes yet (the wings stay folded, the body upright); that comes with the next commits.
  5. For anything floating off the back, sunk into it, too big or turned wrongly, say which and by about how much.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:elytra,minecraft:elytra+unbreaking=3,minecraft:diamond_chestplate
```
