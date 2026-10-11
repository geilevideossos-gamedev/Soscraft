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
