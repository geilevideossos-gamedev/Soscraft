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

### 3. Minecraft campfires smoke, and signal fires over hay

- **Changed:**
  - `ambient_particles.rs`: `CosySmoke` and `SignalSmoke` (`CampfireSmokeParticle`): a random one of `big_smoke_0` to `big_smoke_11`, size 3x, box 0.25, alpha 0.9 (cosy) or 0.95 (signal), lifetime 80 + 50 r or 280 + 50 r ticks, rising at the given 0.07 plus up to 0.002, gravity 3e-6, no friction, swaying by up to 1/5000 a tick on x and z, and losing 0.015 alpha a tick over the last 60. They go into the translucent entity mesh (vanilla's `TRANSLUCENT` sheet); `append_mesh` takes both meshes now.
  - `AmbientParticles::campfire` is `CampfireBlockEntity.particleTick`: with chance 0.11 a tick, 2 or 3 `makeParticles(signal, false)` (each at the centre +- r/3 on x and z and y + r + r), and for each food slot with chance 0.2, four `smoke` with motion (0, 0.0005, 0) at the slot's corner (`Direction.from2DDataValue((slot + facing) mod 4)`, 0.3125 in and 0.3125 clockwise). `campfire_tick` runs it for every lit campfire of the scene.
  - `scene.rs`: `HandcraftedScene::campfires` lists the campfires among the edited blocks and the generated chunks' block entities; `AmbientParticles` keeps the list until the scene's revision changes. `containers.rs`: `Containers::campfire_food` gives the occupied slots. `entities.rs` forwards `campfire_tick`; `minecraft_world.rs` calls it once per client tick beside the display tick (one line).
  - `third_party/minecraftoss/player/src/placement.rs`: campfires and soul campfires get `CampfireBlock.getStateForPlacement`'s state: `facing` the player's horizontal direction, `lit` unless placed in water, `signal_fire` over a hay bale, `waterlogged` in source water. Before, a placed campfire kept the default state (facing north, never a signal fire, lit even in water). The block update rule already turned `signal_fire` on and off when hay below changes.
- **Tests:** `ambient_particles`: `campfire_smoke_rises_for_its_life_and_fades_out_over_the_last_three_seconds`, `a_campfire_puffs_two_or_three_smokes_one_tick_in_nine`, `campfire_smoke_starts_over_the_fire_and_a_signal_fire_sends_signal_smoke`, `food_on_a_campfire_smokes_four_puffs_from_its_corner`, `only_lit_campfires_in_the_scene_smoke`. `minecraftoss-player` `placement`: `campfires_face_the_player_light_dry_and_signal_over_hay`.
- **Decisions:**
  - Vanilla draws campfire smoke with `addAlwaysVisibleParticle`, so it ignores the 32-block particle distance; the list covers every loaded campfire.
  - A campfire with no `facing` (none is placed so now) counts as north, the block's default state.
- **Test steps:**
  1. Place the campfire on the grass: grey smoke puffs rise about 6 to 8 blocks and fade. **Screenshot 4.**
  2. Place a hay bale and a campfire on top of it (looking east): the smoke climbs about three times as high. **Screenshot 5.** Its logs run the way you looked.
  3. Break the hay bale under that campfire and place it back (or place a campfire beside hay and move the hay under it with another bale): the column switches between short and tall smoke.
  4. Put raw beef and raw porkchop on a campfire: thin smoke rises from each piece's corner until it is cooked.
  5. Douse a campfire with the water bucket: the smoke stops. Relight it with flint and steel: it starts again.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:campfire*4,minecraft:soul_campfire,minecraft:hay_block*4,minecraft:beef*4,minecraft:porkchop*4,minecraft:water_bucket,minecraft:flint_and_steel
```

### 4. Minecraft TNT smokes on its fuse and explosions flash and scatter smoke

- **Changed:**
  - `ambient_particles.rs`: three more kinds. `Explosion` (`HugeExplosionParticle`): grey 0.4 to 1, size 2 x (1 - s / 2) for the size `s` it is given, lifetime 6 to 9, `explosion_0` to `explosion_15` by age, full bright, does not move. `ExplosionEmitter` (`HugeExplosionSeedParticle`): not drawn; for 8 ticks it adds 6 `Explosion`s a tick within +-4 blocks, sized by its age / 8. `Poof` (`ExplodeParticle`, as `poof_particles.rs` already had it for dead mobs): light grey, rises (gravity -0.1, friction 0.9), lifetime 16 / (0.2 + 0.8 r) + 2.
  - `AmbientParticles::explosion` is `ClientPacketListener.handleExplosion` plus `ClientExplosionTracker`: `explosion_emitter` for a blast of radius 2 or more that interacts with blocks (`ServerExplosion.isSmall` is false), else one `explosion` of size 1; on the next tick, min(total block count, 512) times: a blast picked by its block count, a random direction, a distance cbrt(r) x radius; if that block is air, `Level.DEFAULT_EXPLOSION_BLOCK_PARTICLES` (`poof` at half the offset or `smoke` at the full offset, one each by chance) with speed (0.5 / (d / radius + 0.1) x r x r + 0.3) along the direction.
  - The vendored level records each explosion's centre, radius, picked block count (`to_blow.len()`, which counts air as vanilla's list does) and whether it interacts with blocks (`Level.explosion_effects`, `explosion.rs` and one field in `level/mod.rs`). `server.rs` sends them as `Output.explosion_effects`; `weapon_blast` drops the MW2 blast's own record, so MW2 grenades and rockets get no Minecraft flash.
  - `entities.rs`: primed TNT adds a `smoke` at its position + 0.5 up every client tick while its fuse is above 0 (`PrimedTnt.tick` on the client), and every explosion effect goes to `AmbientParticles::explosion`. `mesh.rs` reserves `explosion_0` to `explosion_15` through `ambient_particles::textures`.
- **Tests:** `ambient_particles`: `a_big_blast_seeds_six_flashes_a_tick_for_eight_ticks`, `a_small_or_harmless_blast_is_one_flash`, `a_blast_scatters_poofs_and_smoke_through_the_air_it_reached`, `blast_particles_only_start_in_air`. `server`: `flint_primes_tnt_which_blows_up_with_a_bang` now also checks the TNT's explosion effect (radius 4, interacting, 745 picked positions in the test world, centre at the TNT's y + 0.06) and that an MW2 weapon blast sends none.
- **Decisions:**
  - The explosion's sound stays where it was (from `Output.explosions`, at the TNT's feet); only the particles use the recorded centre.
  - Particle status is vanilla's default `ALL`, so the tracker always runs.
- **Test steps:**
  1. Place TNT and light it with flint and steel: grey smoke puffs rise from its top while it flashes. **Screenshot 6** during the fuse.
  2. Stand back: the blast throws a burst of large grey flashes for under half a second, then light grey poofs and dark smoke drift out of the crater and rise. **Screenshot 7** right after the bang.
  3. Hit a creeper and let it blow up (with `/gamerule mob_griefing false` it breaks nothing and makes only one flash).
  4. Throw a frag grenade: only the MW2 explosion effect shows, no Minecraft flash.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:tnt*16,minecraft:flint_and_steel,minecraft:creeper_spawn_egg*2
```

### 5. Minecraft lava fizzes when water hardens it, and water steams away in the Nether

- **Changed:**
  - Vendored `level/fluid.rs`: the level records vanilla's level event 1501 (`LiquidBlock.fizz`, `LavaFluid.fizz`) in `Level.level_events` (one field in `level/mod.rs`) where lava turns into obsidian or cobblestone beside water and into basalt over soul soil beside blue ice (`shouldSpreadLiquid`), where lava flowing down into water makes stone (`LavaFluid.spreadTo`, which fizzes whether or not the water was a block of water), and where lava flowing into a block destroys it (`LavaFluid.beforeDestroyingBlock`). `server.rs` sends them as `Output.level_events`.
  - New `minecraft_terrain/src/level_events.rs`: the client's `LevelEventHandler` for the events handled so far. 1501 plays `block.lava.extinguish` at the block centre (volume 0.5, pitch 2.6 + (r - r) x 0.8) and puts 8 `large_smoke` at y + 1.2 over random spots of the block. `evaporation` is 26.3's `BucketItem.emptyContents` particle packet for water that evaporates: 8 `large_smoke` at random spots inside the block (`RandomizationType.ALTERNATIVE` with a 1 x 1 x 1 spread from the block corner and no speed).
  - `bucket.rs`: `evaporated(outcome)` names the block where a water bucket's use evaporated. `minecraft_survival/src/level.rs` (`finish_item_use`) puffs the steam once the server took the use, and `entities.rs` presents level events and the steam (`present`, `evaporation_smoke`).
- **Tests:** `level_events`: `lava_fizzes_with_a_hiss_and_eight_puffs_over_the_block`, `other_level_events_do_nothing_yet`, `evaporating_water_puffs_eight_smokes_inside_the_block`. `bucket`: `water_evaporates_where_the_dimension_says_so_but_lava_does_not` checks `evaporated`. `server`: `a_poured_bucket_flows_and_an_empty_one_takes_the_source_back` checks the fizz of the lava that hardens into obsidian.
- **Decisions:**
  - Level event 1009 (fire put out by a player or a splash) is only a sound in vanilla, so "fire extinguish" needs no particles. CampfireBlock.dowse's 20 smoke bursts are made by the client that douses (bucket or shovel); they come with the shovel dousing item of the "blocks and items" group, which owns that client path, and are recorded as missing in the state doc.
  - Level events are a list of (id, position, data) as vanilla sends them, so later effects (bone meal 1505) use the same path.
- **Test steps:**
  1. Pour the water bucket next to a lava pool: where the water meets lava, the lava turns to obsidian or cobblestone with a hiss and a puff of dark smoke. **Screenshot 8.**
  2. Pour lava from a ledge onto water below: stone forms with the same hiss and smoke.
  3. Pour lava next to a torch on the ground so it flows over it: the torch drops with a hiss.
  4. In the Nether (`/execute in minecraft:the_nether run tp @s ~ 100 ~` or a portal), pour the water bucket on the ground: a hiss and a puff of steam-smoke, and no water. **Screenshot 9.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:water_bucket*2,minecraft:lava_bucket*2,minecraft:bucket*2,minecraft:torch*4,minecraft:cobblestone*32
```
