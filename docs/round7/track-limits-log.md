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
  3. Pour lava next to a torch on the ground so it flows over it: the torch burns away with a hiss (lava destroys it without a drop, as `LavaFluid.beforeDestroyingBlock` does; corrected in entry 6).
  4. In the Nether (`/execute in minecraft:the_nether run tp @s ~ 100 ~` or a portal), pour the water bucket on the ground: a hiss and a puff of steam-smoke, and no water. **Screenshot 9.**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:water_bucket*2,minecraft:lava_bucket*2,minecraft:bucket*2,minecraft:torch*4,minecraft:cobblestone*32
```

### 6. Minecraft bone meal sparkles where it grows something

- **Changed:**
  - `server.rs` (`ServerSim::bone_meal`): a successful bone meal records level event 1505 with data 15 at the clicked block, or at the water block in front of the clicked face for `growWaterPlant`, as `BoneMealItem.useOn` does on the server.
  - `level_events.rs`: 1505 is `LevelEventHandler`'s `BoneMealItem.addGrowthParticles` plus `item.bone_meal.use` (volume 1, pitch 1) at the block centre. On a `BonemealableBlock` (the 35 classes that implement it in 26.3, matched through the block class chains of the block state catalog, so subclasses such as beetroots, carrots, kelp and cave vines count) the particle spot is the block, or the block below for `MangroveLeavesBlock` and `RootedDirtBlock` (`getParticlePos`). `NEIGHBOR_SPREADER`s (grass, nylium, moss and pale moss, netherrack) and water get `ParticleUtils.spawnParticles(count x 3, 3.0, 1.0, false)`: 45 tries over a 6 x 1 x 6 area, kept only above a non-air block. `GROWER`s get `spawnParticleInBlock`: 15 inside the block up to its outline shape's height. Each particle has a gaussian 0.02 motion.
  - `shape_height` holds the outline heights checked with javap: crops 2 + 2 x age sixteenths (carrots and potatoes 2 + age, torchflower crops 6 + 4 x age), stems 2 + 2 x age, saplings, bamboo saplings and seagrass 12, short grass, ferns and small dripleaf 13, short dry grass 10, fungi and kelp 9, mushrooms 6, twisting vines 15, sweet berry bushes 8 at age 0 and 16 after, air 16 (`spawnParticleInBlock` uses 1.0 for air).
  - `ambient_particles.rs`: `HappyVillager` (`SuspendedTownParticle` with the happy villager provider's white colour): sprite `glint`, size x (0.5 + 0.6 r), motion x 0.02, lifetime 20 / (0.2 + 0.8 r), counted down instead of aged, moving freely with friction 0.99. `particle/glint` joins the atlas list.
  - Level events now wait in `Entities` until `present_level_events(scene)`, which `minecraft_world.rs` calls right after `entities.tick` (one line), before the tick's block changes reach the client scene. So, as in vanilla, where the event packet arrives before the block update, the particles follow the block as it was before it grew: a sapling sparkles like a sapling even when it became a tree.
  - `berries.rs`: the bone meal use no longer plays `item.bone_meal.use` itself; the level event plays it, as in vanilla, so it sounds once and only when something grew.
- **Tests:** `level_events`: `bone_meal_on_a_crop_sparkles_fifteen_times_within_its_height`, `bone_meal_on_grass_spreads_forty_five_tries_over_the_ground_around`, `rooted_dirt_sparkles_below_itself_and_unbonemealable_blocks_only_sound`, `plant_heights_follow_the_vanilla_shapes`. `ambient_particles`: `happy_villager_sparkles_hang_in_the_air_and_count_down`. `server`: `bone_meal_grows_a_crop` checks the recorded event. `berries`: `bone_meal_asks_the_server_about_the_clicked_face_and_is_spent` now expects no sound from the use.
- **Decisions:** blocks whose outline is not in the table (cocoa, flower beds, sea pickles, pitcher crops, the dripleaf stem, glow lichen, mossy carpets) use the full block height; the sparkles then reach a little higher than vanilla's on those.
- **Test steps:**
  1. Plant wheat seeds on farmland (hoe the dirt next to water) and use bone meal on them: green stars sparkle low over the young wheat, and the bone meal sound plays once. **Screenshot 10.**
  2. Use bone meal on an oak sapling until it grows: each use sparkles inside the sapling, also the one that makes the tree.
  3. Use bone meal on a grass block in a meadow: stars rise from the ground in a wide square around it while grass and flowers appear. **Screenshot 11.**
  4. Use bone meal on stone: nothing happens, no sound.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:bone_meal*64,minecraft:wheat_seeds*16,minecraft:oak_sapling*4,minecraft:iron_hoe,minecraft:water_bucket,minecraft:dirt*16,minecraft:grass_block*8
```

### 7. Minecraft tools burst into pieces when they break

- **Changed:**
  - `Entities.broken_items` collects the items that broke in the local player's hands this frame: `minecraft_survival/src/level.rs` (`finish_item_use`, where a worn-out flint and steel already played `entity.item.break`) and `minecraft_world.rs` where a blocking shield wears out (one line each).
  - `minecraft_world.rs` turns them into `Mining::break_burst` after the item uses (three lines), which is `LivingEntity.spawnItemParticles(stack, 5)`: the existing eating crumbs (`BlockParticles::spawn_eating`, the same `ITEM` particle maths: 0.6 in front of the eyes, up to 0.15 to the side and 0.3 to 0.9 down, drifting along the look) with the item's particle texture, 5 of them.
- **Tests:** none new: the burst is the eating crumbs' code (`block_particles` tests) with count 5, and the two sites only push the item id.
- **Decisions:** the burst starts at the eyes and follows the yaw and pitch at the frame the break is seen, as vanilla's client does when it handles the break entity event. Only flint and steel and the shield wear out in the player's hands today (tools don't wear from mining or hitting yet).
- **Test steps:**
  1. Light netherrack with flint and steel over and over (64 uses): on the last one it breaks with the crack sound and grey-metal bits fall in front of you. **Screenshot 12** right at the break.
  2. Hold a shield up (right mouse) in front of a skeleton until the shield breaks: shield bits burst out the same way.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:flint_and_steel,minecraft:netherrack*8,minecraft:shield,minecraft:skeleton_spawn_egg
```

### 8. Minecraft deaths play the player death sound

- **Changed:**
  - `survival.rs`: `DEATH_SOUND` (`entity.player.death`, `Player.getDeathSound`) and `death_sound(random)`: volume 1 and the adult voice pitch (r - r) x 0.2 + 1 (`LivingEntity.makeSound` with `getSoundVolume` and `getVoicePitch`).
  - `minecraft_world.rs`: where a Minecraft death is handed to MW2 (the settle step that already builds the death message), the death sound plays once. A lethal fall no longer plays `entity.player.hurt` and a lethal drowning tick no longer plays `entity.player.hurt_drown` (two conditions), because `LivingEntity.hurtServer` plays the death sound instead of the hurt sound when the blow kills. Hazard burns already skipped the hurt sound when they killed, which is why lava and fire deaths were silent.
- **Tests:** `survival`: `the_burn_that_kills_is_heard_as_the_death_not_as_a_hurt` (every lava burn that leaves the player alive sounds `hurt_on_fire`, the one that kills does not; the death sound's volume and pitch range).
- **Decisions:** a totem that saves the player still plays only the totem sound, as `checkTotemDeathProtection` skips both sounds; `hold_off_death` runs before the settle step, so no death reaches it.
- **Test steps:**
  1. In survival, walk into a lava pool and stay: the hurt sounds play, then the player death sound when you die. **Screenshot 13** of the death screen.
  2. Stand in fire on netherrack until you die: same.
  3. Jump from 30 blocks: the big fall sound and the death sound, no hurt sound.
  4. With a totem of undying in the offhand, walk into lava: the totem pops with its own sound and no death sound.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:lava_bucket,minecraft:netherrack*4,minecraft:flint_and_steel,minecraft:totem_of_undying,minecraft:cobblestone*64
```

### 9. Minecraft rain splashes on the ground and patters

- **Changed:**
  - New `minecraft_terrain/src/rain.rs`: `rain_tick` is 26.3's `ClientLevel.tickWeatherEffects` (checked with javap; it moved there from `WeatherEffectRenderer`). With weather radius 10 it tries (int)(0.225 x 21 x 21 x rain^2) columns a tick (99 in full rain): a random column within 10 blocks of the camera, its `MOTION_BLOCKING` height, kept when above the world bottom, within 10 blocks up or down of the camera and raining there (`Biome.getPrecipitationAt`: precipitation and a height-adjusted temperature of 0.15 or more, from the vendored `BiomeTemperature` noises); a `rain` particle (or `smoke` over lava, magma or a lit campfire) at a random spot of the block below, at the top of its collision shape over that spot or its fluid surface, whichever is higher. Then `random.nextInt(3) < rainSoundTime++` plays `weather.rain` (0.2, pitch 1) at the last splash's block, or `weather.rain.above` (0.1, pitch 0.5) when that block is more than 1 above the camera and the camera's own column is covered.
  - `motion_blocking_height` (`Heightmap.Types.MOTION_BLOCKING`: a legacy-solid block other than cobwebs and bamboo saplings, or any fluid) uses the new `HandcraftedScene::column_top`, which looks at the column's edits and walks the generated chunk's non-empty sections from the top, so a tick stays cheap.
  - `ambient_particles.rs`: `Rain` (`WaterDropParticle`): a random `splash_0` to `splash_3`, hopping up 0.1 to 0.3 with sideways motion x 0.3, gravity 0.06 a tick, friction 0.98, box 0.01, lifetime 8 / (0.2 + 0.8 r) counted down; on the ground it vanishes half the time, and it vanishes inside a block's collision shape or fluid. `AmbientParticles::rain_tick` keeps vanilla's `rainSoundTime` and the temperature noises.
  - `entities.rs`: `rain_tick(scene, camera)` with the level's rain level; `minecraft_world.rs` runs it with the display tick each client tick and plays its sounds with the display tick's (two lines).
- **Tests:** `rain`: `full_rain_drops_ninety_nine_splashes_on_the_ground_around_the_camera`, `no_splashes_on_snow_or_ground_far_above_or_below`, `rain_on_lava_magma_or_a_lit_campfire_smokes_and_slabs_catch_it_halfway`, `the_rain_sound_comes_every_few_ticks_from_the_last_splash_spot`, `rain_heard_on_a_roof_above_the_camera_is_muffled`, `the_heightmap_finds_the_top_motion_blocking_block`. `ambient_particles`: `rain_splashes_hop_up_and_die_on_the_ground_or_in_water`.
- **Decisions:**
  - The camera is the player's eye; in third person vanilla uses the camera position, a few blocks off.
  - The temperature uses sea level 63, the overworld's, since only levels with weather get a rain level and only the overworld has weather.
  - The biome is the quart-resolution noise biome at the block, as the rain fog already uses, not vanilla's zoomed `BiomeManager` lookup; near biome borders the rain line can be up to a few blocks off.
  - The falling streaks (`WeatherEffectRenderer`) and the rain fog stay as they are: drawing streaks is a new pass in `render_gpu`, which this track leaves to the End track's sky work.
- **Test steps:**
  1. `/weather rain` in a plains biome: small water splashes pop up from the ground all around, and rain patters. **Screenshot 14** looking down at the grass.
  2. Stand under a roof (place a few blocks two above your head): the patter turns into the duller "rain on roof" sound now and then.
  3. Walk to a lava pool or place a magma block in the rain: small smoke puffs rise from them instead of splashes.
  4. In a snowy biome (`/locate biome minecraft:snowy_plains`, then `/tp` there): no splashes and no rain sound.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64,minecraft:magma_block*4,minecraft:lava_bucket,minecraft:campfire
```

### 10. Minecraft lava fog

- **Changed:**
  - New `minecraft_terrain/src/lava_fog.rs`: `in_lava(sky, spectator, fire_resistant, render_distance)` is `LavaFogEnvironment` (javap): fog colour `0x991A00` (`ARGB.vector3fFromRGB24(-6743808)`), environmental start and end 0.25 and 1.0, or 0 and 5 with Fire Resistance, or -8 and half the render distance for a spectator; the sky and cloud fog ends take the same end. The skybox becomes `None` (vanilla's `LevelRenderer` skips the sky pass in lava, so only the fog colour shows) and the clouds' alpha 0 (their fog end of 1 block hides them).
  - `minecraft_world.rs`: when the living player's eyes are in lava (`sim::voxel::eye_in_lava`, the same test the survival rules use), the sky state goes through `in_lava` before it is uploaded (one `let`). The shaders already fog terrain, mobs, particles and the sky with these values, so `render_gpu` needed no change.
- **Tests:** `lava_fog`: `lava_hides_everything_past_a_block_in_orange`, `fire_resistance_sees_five_blocks_and_spectators_half_the_view`.
- **Decisions:** the fog follows the eyes, as the rain fog and the rest of the fog already do; in third person vanilla asks the camera.
- **Test steps:**
  1. In survival, jump into the lava pool and look around: everything turns deep orange right in front of your face. **Screenshot 15** inside the lava.
  2. Drink a potion of fire resistance (or `/effect give @s minecraft:fire_resistance 60`) and dive again: you see about 5 blocks through the orange.
  3. `/gamemode spectator` and fly into lava: the fog starts far away and reaches half the view distance.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:lava_bucket*4,minecraft:cobblestone*32,minecraft:potion@fire_resistance*2
```

### 11. Minecraft level event tests pass clippy

- **Changed:** the two server tests that read the recorded level events (`bone_meal_grows_a_crop`, `a_poured_bucket_flows_and_an_empty_one_takes_the_source_back`) take the list with `std::mem::take` instead of `drain(..).collect()` (`clippy::drain_collect`). The group's other files show no clippy findings; the crate's existing ones (`spider_render.rs` and others) are untouched.
- **Tests:** the two tests, unchanged in what they check.
- **Test steps:** none, nothing changes in game.
- **Kit:** none.

## Group blocks-items

Block and item limitations from the Blocks and items, Survival and combat and Movement lists. Every behaviour below was checked with javap on the 26.3 client jar.

### 1. Minecraft pointed dripstone hangs from ceilings

- **Changed:**
  - `minecraftoss-player` `placement.rs`: pointed dripstone and sulfur spikes take 26.3's `SpeleothemBlock.getStateForPlacement` (javap; pointed dripstone now extends `SpeleothemBlock`). The default tip points away from the looking direction (`getNearestLookingVerticalDirection`: pitch below 0 is up), so looking up hangs a stalactite and looking down stands a stalagmite; where that side has no support (`isValidSpeleothemPlacement`: a sturdy face on the base, or the same block pointing the same way) the other direction is tried, and with neither the placement is refused. The thickness is `calculateSpeleothemThickness`: `tip_merge` against an opposite tip of the same block (only `tip` while sneaking, unless that tip is already merged), `tip` with nothing in front, `frustum` behind a tip, `middle` or `base` further up the column. Water source at the spot waterlogs it.
  - `minecraftoss-generator` `update.rs`: the `Speleothem` neighbour rule now ends like vanilla's `updateShape`, recalculating the thickness on an up or down update (keeping a merge only when it was already `tip_merge`), so a stalactite grows from tip to frustum, middle and base as tips are added below, and two opposite tips merge.
- **Tests:** `placement`: `pointed_dripstone_hangs_from_a_ceiling_and_stands_on_a_floor`, `pointed_dripstone_grows_thicker_and_merges_with_an_opposite_tip`. `server`: `a_stalactite_thickens_as_tips_are_added_below_and_merges_with_a_stalagmite`.
- **Decisions:** the speleothem tag check is the `Speleothem` update rule (pointed dripstone and sulfur spike, the tag's two blocks), so no new tag is loaded.
- **Test steps:**
  1. Stand under a stone ceiling, look up and place pointed dripstone on it: it hangs down as a stalactite tip. **Screenshot 1.**
  2. Place three more under it: the column turns into base, middle, frustum and tip from the top down.
  3. Under another hanging tip, build a floor so that two air blocks are left between floor and tip. Look down and place dripstone on the floor: it stands up. Then look up and place one in the one-block gap between the two tips: it hangs and merges with the standing tip into a joined pair. **Screenshot 2.**
  4. Sneak and repeat step 3 at another spot: the new tip stays separate from the standing one.
  5. Aim at the side of a lone wall block, with air above and below the spot: nothing is placed.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:pointed_dripstone*32,minecraft:stone*64
```

### 2. Minecraft farmland and dirt paths placed under a solid block become dirt

- **Changed:**
  - New `minecraft_terrain::placing::base_if_covered`: 26.3's `FarmlandBlock.getStateForPlacement` and `PathBlock.getStateForPlacement` (javap) place dirt when the block can't survive: the block above is legacy-solid (`BlockState.isSolid`, the registry's `LEGACY_SOLID` flag, so signs, pressure plates and the other `forceSolidOn` blocks count) and, for farmland, not in `#maintains_farmland` (crops, stems, moving pistons and fence gates) or, for a path, not a fence gate. It reads the block above from the client scene, the same place the survive check reads.
  - `render_anim/src/minecraft_world.rs`: the placed blocks go through it before they are written (one line).
- **Tests:** `placing`: `farmland_and_paths_placed_under_a_solid_block_become_dirt` (stone and a sign above make dirt; air, a torch, a fence gate and, for farmland, wheat keep it; other blocks pass through).
- **Decisions:** `PathBlock`'s `pushEntitiesUp` is left out: the placement is already refused when a body is in the way.
- **Test steps:**
  1. Stack two stone blocks, break the lower one and place farmland into the gap (aim at the side of the neighbouring ground): it becomes dirt. **Screenshot 3.**
  2. Do the same with a dirt path: dirt. A sign hung on a wall right above the gap also makes it dirt.
  3. Place a fence gate on a block, break the block under the gate and place farmland into that gap: it stays farmland. Again with a dirt path: it stays a path.
  4. Farmland and paths in the open still place as usual.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:farmland*16,minecraft:dirt_path*16,minecraft:stone*16,minecraft:oak_fence_gate*4,minecraft:torch*8,minecraft:oak_sign*4
```

### 3. Minecraft shovels douse campfires, and doused campfires smoke

- **Changed:**
  - `minecraft_terrain/src/campfire.rs`: `douse_use` is 26.3's `CampfireBlock.useItemOn` dousing branch (javap; in 26.3 it moved from `ShovelItem` into the campfire, keyed on the item tag `#douses_campfires`, which holds `#shovels`): a lit campfire or soul campfire the player aims at (block reach 4.5) becomes unlit, the shovel wears by 1 (`hurtAndBreak`, so not in creative) and level event 1009 with data 0 plays `block.fire.extinguish` at the block centre (volume 0.5, pitch 2.6 + (r - r) x 0.8). Sneaking skips it, as `useItemOn` is skipped for a sneaking player holding an item. `doused` tells whether an item use put a lit campfire out: a shovel's unlit write, or a water bucket emptied into it (the outcome carries the campfire douse sound, which an evaporating Nether pour does not).
  - `ambient_particles.rs`: `douse` is `CampfireBlock.douse` on the client: 20 times `makeParticles(signal, true)`, a cosy or signal smoke puff from the block and a `smoke` puff at y + 0.4 within a quarter block of the centre rising at 0.005.
  - `minecraft_survival` `entities.rs`: `campfire_douse` hands the burst to the ambient particles. `render_anim/src/minecraft_world.rs`: the shovel joins the item-use chain, and a douse found by `doused` bursts (three lines).
- **Tests:** `campfire`: `a_shovel_douses_a_lit_campfire_with_a_hiss_and_wears_by_one`, `a_douse_by_shovel_or_water_bucket_bursts_with_the_campfire_smoke`. `ambient_particles`: `a_doused_campfire_bursts_into_twenty_puffs_and_twenty_low_smokes`.
- **Decisions:**
  - A splash water potion's douse stays without the burst: vanilla runs `douse` there on the server only, where it makes no particles.
  - The shovel goes through the same item-use chain as buckets and flint and steel, which only runs where the player may build, so adventure players can't douse with a shovel (vanilla lets them). Recorded in the state doc's limitations.
- **Test steps:**
  1. Place a campfire (it lights) and right-click it with a shovel: it goes out with a hiss and a big burst of smoke. **Screenshot 4** right after the click.
  2. Hover the shovel in the inventory: it lost 1 durability (the bar under the icon moves after a few uses).
  3. Light it again with flint and steel, sneak and right-click it with the shovel: nothing happens.
  4. Place a campfire on a hay bale and douse it: the burst uses the tall signal smoke.
  5. Light it again and pour a water bucket on it: it goes out with the extinguish sound, the same burst, and is waterlogged.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:campfire*4,minecraft:iron_shovel,minecraft:wooden_shovel,minecraft:flint_and_steel,minecraft:hay_block*2,minecraft:water_bucket
```

### 4. The Minecraft state doc no longer lists shield knockback, use speed or trident charging as gaps

- **Changed:** `docs/MINECRAFT-SURVIVAL.md` only. Three listed gaps turned out to be vanilla 26.3 behaviour already, checked with javap:
  - Shield knockback to the attacker: 26.3 has none. `LivingEntity.blockUsingItem` calls `attacker.blockedByItem(defender, source, amount, fullyBlocked)`, which knocks the *defender* back by 0.5 only when the block was partial; the shield's `blocks_attacks` (`Items` lambda: one damage reduction, 90 degrees, base 0, factor 1) always blocks fully, and `Mob.doHurtTarget` skips its extra knockback when `hurtServer` fails. The fork already gives a blocked hit no knockback.
  - Use speed: `UseEffects.DEFAULT` is (can sprint false, vibrations true, speed 0.2) and only `Item.Properties.spear` sets another (sprint, speed 1). The fork slows every item use to 0.2 and stops sprinting (`net` client runtime, `MINECRAFT_ITEM_USE_SPEED`), and spears have no use yet.
  - Trident charging: `ui.using_item` includes `hand.charging`, so charging already slows like any use.
  - Axes disabling shields stays listed, reworded: `Player.blockUsingItem` disables the shield for the attacker's `getSecondsToDisableBlocking`, which comes from an axe's `weapon` component, and no simulated mob holds an axe, so it waits for players fighting each other.
- **Tests:** none, nothing changes in game.
- **Decisions:** the shield and use-speed items of the group are closed by verification instead of code.
- **Test steps:** none.
- **Kit:** none.

### 5. Minecraft armour protection enchantments reduce damage like vanilla

- **Changed:**
  - The listed gap was Fire Protection, but no armour enchantment protected at all, so this does all five the 26.3 way. New `minecraft_terrain/src/protection.rs`: `ProtectionBook::from_jar` reads every `data/minecraft/enchantment/*.json` and keeps its `minecraft:damage_protection` effects (`add` of a level-based value, with `damage_source_properties` tag requirements) and its `minecraft:attributes` `burning_time` modifiers (`add_multiplied_base`), and resolves the `damage_type` tags (nested tags included). `protection` is `EnchantmentHelper.getDamageProtection`: the summed values of every enchantment on the worn armour (the four armour slots, as the enchantments' `armor` slot group) whose tags match the damage type; damage in `#bypasses_effects` or `#bypasses_enchantments` gets none, as `LivingEntity.getDamageAfterMagicAbsorb` returns before it. `burning_time` is 1 plus the summed modifiers, clamped to the attribute's 0 to 1024, and `Protection::burning_ticks` is `LivingEntity.igniteForTicks`'s `ceil(ticks x burning_time)`.
  - `minecraftoss-player` `survival.rs`: `hurt_protected` applies the protection inside `actuallyHurt` after armour and Resistance and before absorption: `damage x (1 - clamp(points, 0, 20) / 25)` (`CombatRules.getDamageAfterMagicAbsorb`). `hurt` is `hurt_protected` with none.
  - `minecraft_survival`: `DamageCause::damage_type` gives each cause its vanilla damage type (`in_fire`, `campfire`, `lava`, `on_fire`, `fall`, `ender_pearl`, `mob_attack`, `arrow`, `explosion`, `fireball`, ...). `Survival::wear` takes the worn protection, and the hazard, fall, mob, generic and harming hurts use it; fire, lava and small-fireball ignitions are scaled by the burning time. `GameData` loads the book from the jar, and `minecraft_world.rs` hands the worn protection to the survival state each frame (one line).
- **Tests:** `protection`: `protections_add_their_levels_for_the_damage_types_they_match`, `fire_protection_shortens_burning_by_fifteen_percent_a_level`, `only_armour_slots_count`, `the_vanilla_protections_read_from_the_jar` (with `MINECRAFTOSS_ROOT`). `survival`: `fire_protection_softens_lava_and_shortens_the_burn_and_feather_falling_softens_falls`. `data`: no book without a jar.
- **Decisions:**
  - Drowning and the poison and wither effects stay unprotected: they hurt inside the vendored `SurvivalStatus` tick, which knows nothing of the armour. Recorded in the state doc.
  - Blast Protection's `explosion_knockback_resistance` is not applied: creeper and MW2 blasts push the player through the MW2 movement, not vanilla's explosion knockback.
- **Test steps:**
  1. Put on the diamond boots with Feather Falling IV and jump from 10 blocks onto stone: a little under 2 hearts lost. **Screenshot 5** of the hearts.
  2. Swap to the plain diamond boots and repeat: 3.5 hearts, as armour itself doesn't soften falls.
  3. Wear the Fire Protection IV chestplate and walk into lava for a moment, then out: the hits are smaller and the burning stops after about 6 seconds instead of 15. **Screenshot 6** while burning.
  4. Wear the Protection IV helmet and let a zombie hit you: a little less damage than without it.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_boots+feather_falling=4,minecraft:diamond_boots,minecraft:diamond_chestplate+fire_protection=4,minecraft:diamond_helmet+protection=4,minecraft:lava_bucket,minecraft:cobblestone*64,minecraft:zombie_spawn_egg
```
