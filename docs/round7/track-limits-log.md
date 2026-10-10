# Track LIMITS log: phase 4, the known limitations

Branch `limits`, worktree `C:\Dev\mc-wt\limits`. Briefs: the limitation lists in `docs/MINECRAFT-SURVIVAL.md` (Rendering, Movement, Survival and combat, Blocks and items, Mobs). Base: 4c4900e.

A commit can't name its own hash, so each entry is written with its commit and its hash is in `git log` under its subject.

## Group particles-sounds

Missing particles and sounds from the Rendering limitations. Every behaviour below was checked with javap on the 26.3 client jar.

### 1. Minecraft fire, lava and campfires make their ambient sounds

- **Changed:** `minecraft_terrain/src/display_ticks.rs` (`ClientLevel.animateTick`'s per-block step, which already ran for nether portals) now also runs:
  - `BaseFireBlock.animateTick` for `fire` and `soul_fire`: one time in 24, `block.fire.ambient` at the block centre, volume 1 + r, pitch 0.3 + 0.7 r.
  - `LavaFluid.animateTick` for `lava` with air above: one time in 100, `block.lava.pop` at a random spot of the top face (volume 0.2 + 0.2 r, pitch 0.9 + 0.15 r); one time in 200, `block.lava.ambient` at the block's corner (vanilla plays it at `getX()`, not the centre), same volume and pitch.
  - `CampfireBlock.animateTick` for lit `campfire` and `soul_campfire`: one time in 10, `block.campfire.crackle` at the centre, volume 0.5 + r, pitch 0.6 + 0.7 r.
  - The adapter already played every sound the display tick returns, so `render_anim` needed no change.
- **Tests:** `display_ticks`: `fire_roars_once_in_twenty_four_ticks`, `lava_under_air_pops_on_its_surface_and_hums_from_its_corner`, `covered_lava_is_silent`, `lit_campfires_crackle_once_in_ten_ticks`.
- **Decisions:** the display tick samples blocks around the player as vanilla does (667 tries in each of the 16 and 32 block cubes a tick), so the sounds come from nearby blocks at vanilla's rate without a list of fires or lava.
- **Test steps:**
  1. Stand next to a lava pool (or pour the lava bucket into a hole): within a few seconds the lava pops and hums. **Screenshot 1** of the pool.
  2. Light a netherrack block with flint and steel and stand by it: the fire crackles and roars about every second.
  3. Place the campfire and stand next to it: it crackles every half second or so. Douse it with the water bucket: it falls silent.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:lava_bucket,minecraft:water_bucket,minecraft:netherrack*4,minecraft:flint_and_steel,minecraft:campfire*2,minecraft:soul_campfire,minecraft:hay_block*2
```

### 2. Minecraft fire smokes and lava and campfires spit embers

- **Changed:**
  - New `minecraft_terrain/src/ambient_particles.rs`: `AmbientParticles`, the client's general particle list (cap 16384, like a `ParticleEngine` sheet), with vanilla's particle physics (`Particle.tick` and `Particle.move`: gravity, friction, the 0.2 box colliding with block collision shapes in `Direction.axisStepOrder`, `stoppedByCollision`, the ground's 0.7 and `speedUpWhenYMotionIsBlocked`) and three kinds so far:
    - `Smoke` and `LargeSmoke` (`SmokeParticle`, `LargeSmokeParticle`, `BaseAshSmokeParticle`): dark grey (0 to 0.3), rising (gravity -0.1, friction 0.96), lifetime 8 / (0.2 + 0.8 r) x scale, size 0.75 x scale and grown in over the first 1/32 of the life, sprites `generic_7` to `generic_0` by age; scale 1 and 2.5.
    - `Lava` (`LavaParticle`): leaps 0.05 to 0.45 up, falls (gravity 0.75, friction 0.999), lives 16 / (0.2 + 0.8 r) ticks, shrinks as 1 - t^2, is lit at block light 15 and drops a `Smoke` with its own motion while a random float is above its age fraction.
    - Quads face the camera like the poof's and go into the particle mesh (vanilla's `OPAQUE` sheet).
  - `display_ticks.rs` returns the particles of the display tick: `BaseFireBlock.animateTick`'s `large_smoke` (three in the upper half of a fire on a burnable or sturdy base, else two against each side it clings to, in vanilla's west, east, north, south, up order), `LavaFluid.animateTick`'s `lava` at the pop's spot, and `CampfireBlock.animateTick`'s `lava` one time in five from a lit campfire's centre (not from soul campfires, whose `spawnParticles` is false).
  - `minecraft_survival/src/entities.rs`: holds the list, feeds it the display tick, ticks it with the scene and draws it after the poof. `mesh.rs` reserves `particle/lava` in the atlas.
- **Tests:** `ambient_particles`: `smoke_is_dark_and_rises_until_it_fades`, `large_smoke_is_two_and_a_half_times_bigger_and_longer`, `smoke_grows_in_over_its_first_thirty_second_of_life`, `smoke_stops_under_a_ceiling`, `lava_leaps_falls_glows_and_trails_smoke`, `lava_lands_on_the_floor`. `display_ticks`: `fire_on_the_ground_smokes_three_puffs_a_tick_from_its_upper_half`, `fire_on_a_wall_smokes_two_puffs_against_each_burning_side`, `lava_spits_an_ember_with_each_pop`, `lit_campfires_spit_an_ember_one_time_in_five_and_soul_campfires_none`.
- **Decisions:**
  - Vanilla's fire picks its smoke by asking whether the block below burns or is sturdy (`canBurn`, `isFaceSturdy`). The fire's own `north`/`east`/`south`/`west`/`up` properties hold the same answer (`FireBlock.getStateForPlacement` and `updateShape` set them only when the base neither burns nor is sturdy, to whether each neighbour burns), so the display tick reads them and needs no block registry. Soul fire has no such properties and always smokes from its base, as `SoulFireBlock.canBurn` is always true.
  - The lava particle provider ignores the campfire's ember motion, as vanilla's `LavaParticle.Provider` does; the motion is still drawn so the random draws keep vanilla's order.
- **Test steps:**
  1. Light the netherrack: dark smoke rises from the fire all the time. **Screenshot 2.**
  2. Set fire to the side of a log wall (flint and steel on the side of a log with air below the fire): the smoke hugs the burning log.
  3. Stand by the lava pool: orange embers leap from the surface now and then, fall back and leave a short smoke trail. **Screenshot 3.**
  4. Place the campfire: embers jump from it every second or so. The soul campfire makes none.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:lava_bucket,minecraft:water_bucket,minecraft:netherrack*4,minecraft:oak_log*8,minecraft:flint_and_steel,minecraft:campfire*2,minecraft:soul_campfire
```
