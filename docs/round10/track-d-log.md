# Track D log: trial chambers, vaults, lodestones and breezes (round 10)

Branch `track-d`, worktree `C:\Dev\mc-wt\track-d`, target dir `D:\iw4l-wt\track-d`. Brief: `docs/round10/track-d.md`. Round 9's history: `docs/round9/track-d.md` and `docs/round9/track-d-log.md` (which stays as it is).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

Screenshots of this track are numbered from 301.

## Group d5-review

Brief D11: the review of round 9's partial d5 run (keyframes 81f6f353, withers d4f932de..ef9ad9de, wind charges f142f9c1).

### 1. Track D pins that a wind charge does not push a player flying in creative

After 382bf959.

- **Vanilla (javap, 26.3):** `SimpleExplosionDamageCalculator.getKnockbackMultiplier` returns 0 for a `Player` whose `getAbilities().flying` is set, and the calculator's own multiplier (1.22 for a player's wind charge, the default 1.0 for a breeze's) otherwise. Wind charges and breeze charges both explode with that calculator. Gliding with an elytra is not `abilities.flying`, so a gliding player is pushed.
- **Checked (every push path to a player, at 382bf959):**
  - Wind charges: `minecraft_terrain/src/wind.rs` `burst` returns `(player, push)` for every player in reach; `ServerSim::wind_burst` (`server.rs:1889-1896`) adds them to `players_pushed`; `run_tick` moves them to `Output.players_pushed` (`server.rs:4282`).
  - The other producers of `Output.players_pushed`: the entity world's player pushes (`server.rs:4283`, `EntityWorld::take_player_pushes`: ravagers and the other mob pushes), shield blocks (`server.rs:4133`, `Command::ShieldBlock`) and the dragon (`server.rs:2642`, `events.player_pushes`).
  - `minecraft_survival/src/entities.rs:301-304` routes every `players_pushed` entry to that player's `PlayerQueue.out.pushes`; the blast pushes of mob hits (creepers, fireballs, TNT) are added at `entities.rs:329` (`explosion_push`). Nothing else writes `out.pushes`.
  - `render_anim/src/minecraft_players.rs` `struck` applies `out.pushes` and the knockback of hits (`landed`) only through the one `pushed` check, for the host (`minecraft_world.rs:1245`) and for every friend (`step_friend`, `minecraft_players.rs:1661`: the host runs every player's step).
  - The level's own `wind_burst` (vendored `world/src/level/explosion.rs:349`) pushes only level entities (items, falling blocks), never players. `Body::push` (`actions.rs:112`) is only the riptide launch.
  - `MC_FLYING` is set only by creative and spectator flight (`movement_iw4/src/fly.rs:60-79`, `sim/src/frame.rs:529-539`); elytra gliding is `MC_GLIDING`. So the check matches vanilla's `abilities.flying`.
  - Result: no path bypasses the check. Release 1's known limit "A player flying in creative is still pushed by a burst" was wrong: the check is in the per-player step since cb4251e9, before f142f9c1.
- **Changed:** `render_anim/src/minecraft_players.rs`: the check moved unchanged into the small function `takes_pushes(alive, mode, pm_flags)`, so it can be tested.
- **Tests:** `render_anim` `minecraft_players::tests::a_wind_charge_does_not_push_a_player_flying_in_creative`: a player's burst one block away pushes the player out and up; `takes_pushes` keeps the push for a walking player, a creative player on the ground and a player gliding with an elytra, and drops it for a creative player flying, a spectator and a dead player.
- **Decisions:** none beyond the check; no code path changed.
- **Test steps (new world `IW4L_MINECRAFT_WORLD=trial10`, survival, Normal, by day, flat open ground):**
  1. `/gamemode creative`, double-tap jump to fly 5 blocks up, and throw a wind charge at a block right below you: the burst does not move you (**301_wind_flying**).
  2. Stop flying (double-tap jump), land, and throw one at your feet: it launches you. `/gamemode survival`.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:wind_charge*16,minecraft:cobblestone*64
```

### 2. Minecraft fireballs and wither skulls keep their shooter through a save

After bad8dd84.

- **Vanilla (javap, 26.3):**
  - `Projectile.addAdditionalSaveData` writes `Owner` with `EntityReference.store` (the owner's UUID as an int array), `LeftOwner` when set and `HasBeenShot`; `readAdditionalSaveData` reads `Owner` back with `EntityReference.read` as a UUID only.
  - `Projectile.getOwner` resolves that reference when asked (`EntityReference.getEntity(ref, level)`: `Level.getPlayerInAnyDimension` for a player, `Level.getEntityInAnyDimension` otherwise). An owner that is not loaded is `null`, and the projectile flies on without one. `AbstractHurtingProjectile.tick` asks every tick, so the owner is found as soon as it loads.
  - So a wither skull saved in flight still hits for 8 with Wither II and is told as "was shot by a skull from Wither", and a ghast's fireball still belongs to its ghast.
- **Changed:**
  - Vendored `entities/src/fireball.rs`: `Fireball.owner_uuid`, the saved owner still waiting to be found. `Fireball::shoot` leaves it empty, and `deflect` (a player batting a fireball back) clears it with the new owner.
  - Vendored `entities/src/world/fireballs.rs`: `tick_fireball` resolves a waiting owner at the start of every tick, before the flight and the hit, through the new `shooter_of_uuid`, and keeps it once found. A UUID the level knows for a player becomes `Shooter::Player`, a living mob's `Shooter::Mob`. Nothing is resolved at load.
  - `minecraft_terrain/src/server_mobs.rs`: `mob_tags_where` saves `Owner` for fireballs, small fireballs and wither skulls: the shooter's UUID (`uuid_of`, a player's under `PLAYER_TARGET`), or the UUID still waiting. `spawn_saved` loads it into `owner_uuid`.
  - `minecraft_survival/src/entities.rs`: `mob_hit` is `pub(crate)` for the death test.
- **Tests:**
  - `server_mobs::tests::a_wither_skull_saved_in_flight_still_belongs_to_its_wither`: a wither's skull is saved with the wither's UUID as `Owner`; loaded first into a fresh world it has no owner and the UUID waiting; with the wither loaded after it, the skull hits a player for 8 with Wither II and the wither as its source; in a world without the wither it hits for 5 with no source (magic).
  - `server_mobs::tests::a_ghast_fireball_saved_in_flight_still_belongs_to_its_ghast`: the same for a ghast's large fireball (6, the ghast as its source; no source without the ghast).
  - `server_mobs::tests::a_fireball_a_player_batted_back_keeps_that_player_through_a_save`: a fireball batted back by player 1 is saved with that player's UUID; reloaded, it waits while the level does not know the player, finds `Shooter::Player(1)` once it does, and is saved again with the same UUID.
  - `death::tests::a_reloaded_skull_kill_reads_shot_by_a_skull_from_wither`: a skull and its wither saved, loaded in that order, the skull's hit goes through `mob_hit` (8 on Normal, Wither II for 10 s) and its death line reads "%1$s was shot by a skull from Wither".
- **Decisions:**
  - The owner is resolved by `shooter_of_uuid`, not by `id_of_uuid` alone as shulker bullets do: the entity world's UUID map keeps the ids of mobs that died or unloaded, so `id_of_uuid` can name an id that is gone, and a fireball with a gone mob owner is removed (the fork's existing rule). `shooter_of_uuid` takes a map entry only for a player or a mob that is still there, and otherwise looks through the level's living entities. A dead or absent owner leaves the fireball ownerless, as vanilla's `null` owner.
  - A player is found through the UUIDs the level was told (every player present in that level this session). Vanilla looks in every dimension; the fork's levels each have their own entity world, so a mob owner in another dimension stays unknown, and a player owner is found once that player has been in this level.
  - Fireballs keep the fresh `UUID` they had; only `Owner` is new. `HasBeenShot` is not saved (the fork does not track it).
- **Test steps (world `trial10`, survival, Normal):**
  1. Take the armour off for this step (diamond armour cuts a skull's 8 damage to under a heart, so 4 and 2.5 hearts could not be told apart). `/tick rate 2` (the world runs ten times slower). Build a wither (four soul sand in a T, three wither skeleton skulls on top), run 20 blocks away and wait for the blast. When a black skull is in the air towards you, `/tick freeze`. Save and quit, load the world, `/tick rate 20`, `/tick unfreeze`: the skull hits you for about 4 hearts (not 2.5) with Wither II (black hearts) (**302_skull_after_reload**). If it kills you, the chat says "<name> was shot by a skull from Wither". Put the armour back on.
  2. Friend window: the friend joins again after the host has loaded the world (the host's save and quit ends the friend's session), sees the wither's next skulls fly and hears them hit.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:wind_charge*16,minecraft:soul_sand*8,minecraft:wither_skeleton_skull*6,minecraft:netherite_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:milk_bucket*2,minecraft:totem_of_undying,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

## Group trial-spawners

Brief D12, first half: the trial spawner's configs, its mob in the cage, its flames and hum, and the spawn egg. The trial itself (detection, spawning, rewards) is group `trial-spawners-run`.

### 1. Minecraft trial spawners show their mob spinning like vanilla

After 0ff5716c.

- **Vanilla (javap and the data in the 26.3 jar):**
  - `TrialSpawnerState`: inactive (no particles, spin −1, cannot spawn), waiting_for_players (small flames, spin 200, can spawn), active (flames and smoke, spin 1000, can spawn), waiting_for_reward_ejection and ejecting_reward (small flames, spin −1), cooldown (smoke inside and on top, spin −1). `hasSpinningMob` is spin ≥ 0.
  - `TrialSpawnerState$ParticleEmission`: small flames: one tick in two (`nextInt(2) == 0`) a `small_flame` (`soul_fire_flame` when ominous) at the block centre `offsetRandom(0.9)`; flames and smoke: a `smoke` and a `flame` (`soul_fire_flame`) at the same `offsetRandom(1.0)` point; smoke inside and on top: a point at `offsetRandom(0.9)`, a `smoke` there one tick in three, and when `gameTime % 20 == 0`, `nextInt(4) + 20` smokes at the centre of the top face (all at that one point). `Vec3.offsetRandom` adds `(nextFloat − 0.5) × f` on x, y, z in that order.
  - `TrialSpawner.tickClient`: the particles first; with a spinning mob, `oSpin = spin`, `spin = (spin + speed / (max(0, nextMobSpawnsAt − gameTime) + 200)) % 360`; when the state can spawn, `nextFloat() <= 0.02` plays `block.trial_spawner.ambient` (`ambient_ominous`) at the block centre, volume `nextFloat × 0.25 + 0.75`, pitch `nextFloat + 0.5`. The client tick has no player range.
  - `TrialSpawnerStateData.getUpdateTag`: the client gets `next_mob_spawns_at` only while active (otherwise 0), and `spawn_data`. `TrialSpawnerRenderer` shares the mob spawner's `extractSpawnerData` (scale, lerped spin × 10).
  - The inactive arm of `tickAndGetNext`: `getOrCreateDisplayEntity` → `getOrCreateNextSpawnData`: a saved `spawn_data` is kept; otherwise one is picked from the active config's `spawn_potentials` with the level random (`WeightedList.getRandom`: `nextInt(totalWeight)`), or an empty `SpawnData` (entity with no id) when there are none, and the block entity is marked updated. A mob with an id makes the state waiting_for_players (`setState`: `setBlockAndUpdate`, flags 3).
  - `TrialSpawnerConfig` keys and defaults (`TrialSpawnerConfig$Builder`): `spawn_range` 4, `total_mobs` 6, `simultaneous_mobs` 2, `total_mobs_added_per_player` 2, `simultaneous_mobs_added_per_player` 1, `ticks_between_spawn` 40, `spawn_potentials` empty, `loot_tables_to_eject` consumables then key (weight 1 each), `items_to_drop_when_ominous` `spawners/trial_chamber/items_to_drop_when_ominous`. Targets `floor(total + added × additional)` in float. Defaults are left out when written.
  - `TrialSpawner$FullConfig`: `normal_config`, `ominous_config` (registry id or inline), `target_cooldown_length` 36000, `required_player_range` 14, read with the strict `optionalFieldOf`: a config that does not read (an unknown id) makes the whole full config the default. The state data uses `lenientOptionalFieldOf`.
  - `FlameParticle$SmallFlameProvider` is the flame scaled by 0.5; `soul_fire_flame` is the flame with its own sprite (`particles/soul_fire_flame.json`).
- **Changed:**
  - New `minecraft_terrain/src/trial_spawner.rs`: `TrialState` (names, spin speed, particles, can spawn), `TrialConfig` (read from a tag or a data pack JSON, written back without defaults, targets, the weighted `spawn_data` pick), `registry_config` (`data/<ns>/trial_spawner/<path>.json`), `FullConfig` (both configs, range, cooldown; the whole default when a config does not read) and `wake_spawners`, the minimal server pass.
  - `wake_spawners`, called from `ServerSim::tick` right after `tick_spawners` (so `/tick freeze` stops it, as `run_tick` only calls `sim.tick()` when the tick runs normally): every trial spawner block entity in a chunk the level ticks (`Level.ticking_chunks`, the simulation distance around the players, the same chunks `tick_spawners` uses; every loaded chunk when no area is set, as for random ticks) whose block is `inactive` gets its `spawn_data` as vanilla picks it (marked changed, so the host's display and the friends get the tag), and turns `waiting_for_players` when that mob is a kind the game simulates.
  - `spawner_display.rs`: `is_displayed_block_entity` (mob spawners and trial spawners; vaults later) replaces `is_spawner` in `load_chunk`, `block_entities` and the host's forward filter (`minecraft_survival/src/entities.rs`); `is_displayed_block` keeps a friend's display when a block update changes the trial spawner's state (`minecraft_survival/src/remote.rs`, which removed every display whose block was not `minecraft:spawner`). A trial display reads `spawn_data.entity` and `next_mob_spawns_at`; `tick_trials(scene, game_time, particles)` takes the state and `ominous` from the block in the scene, emits the state's particles, spins the mob and returns the hum. `append` draws a trial spawner's mob only while its state spins one. The mob spawner tick skips trial displays.
  - `minecraft_survival/src/entity_view.rs`: `tick_scene` (host and friend, once per client tick, with the scene) runs `tick_trials` with the entity world's game time and plays the hums.
  - `ambient_particles.rs`: particle kinds `SmallFlame` (the flame at half size) and `SoulFireFlame` (the flame with the `soul_fire_flame` sprite, added to the atlas list).
- **Tests:**
  - `trial_spawner::tests::the_six_states_carry_vanillas_spin_particles_and_spawning`, `an_empty_config_has_vanillas_defaults_and_targets` (6/2 for one player, 8/3 for two, nothing written), `an_inline_config_reads_and_writes_back_the_same` (a 3:1 pick, a weightless entry dropped), `every_trial_chamber_config_in_the_data_reads` (gated on the data: 28 configs, the slime sizes 1 and 2 at weights 3 and 1, the ominous zombie's `equipment/trial_chamber_melee` with drop chance 0 and its rewards 3 and 7, targets 6/3 and 8/3 for `melee/zombie/normal`, 6/4 and 8/6 at 160 ticks for `slow_ranged/skeleton/normal`, 2/1 and 3/1 for the breeze), `a_full_config_reads_registry_ids_inline_configs_and_falls_back_whole`.
  - `spawner_display::tests::a_waiting_trial_spawner_shows_its_mob_and_spins_slower_than_an_active_one` (1° a tick waiting, 5° active, 1000/240 with the next spawn 40 ticks away, which only counts while active; drawn only waiting and active), `trial_flames_follow_the_state_and_burn_soul_blue_when_ominous`, `a_trial_spawner_hums_two_ticks_in_a_hundred_while_it_can_spawn`, `trial_spawners_load_with_chunks_and_follow_their_block_entity`, `spawners_and_trial_spawners_are_the_displayed_block_entities`.
  - `ambient_particles::tests::small_and_soul_flames_are_flames_with_their_own_size_and_sprite`.
  - `server::tests::a_trial_spawner_picks_its_mob_and_waits_for_players_in_the_simulation_area` (a zombie config wakes up with a zombie; a breeze config picks a breeze and stays inactive; the default config gets the empty spawn data and stays inactive; a spawner outside the simulation area is untouched; a waiting spawner keeps its mob).
  - `terrain::tests::trial_chambers_generate_with_trial_spawners_and_vaults` (gated like the ancient city test: seed 77's nearest trial chambers hold trial spawners whose normal and ominous configs are registry ids that read, and vaults keyed with a trial key or an ominous trial key).
- **Decisions:**
  - The trial display ticks in `EntityView::tick_scene`, not in `advance` as the brief said: only `tick_scene` has the scene, and the state and `ominous` are block properties. Mob spawners keep ticking in `advance`. When a frame runs several client ticks, each gets its own game time (the snapshot's time minus the ticks still to come), so the cooldown's 20-tick smoke burst is neither doubled nor lost.
  - A kind the game does not simulate (not in `server_mobs::SIMULATED_KINDS`) leaves the spawner inactive, as the brief decides; it still picks and saves its `spawn_data`, as vanilla does, so a breeze or silverfish spawner wakes up by itself once that mob lands.
  - A trial display draws a new mob when `spawn_data` names another one. Vanilla's client keeps the display entity it made first (`TrialSpawnerStateData.displayEntity` is never cleared on the client), which would keep showing the old mob after a spawn egg until the chunk reloads; the fork's mob spawner display already redraws on a change.
  - An unknown registry id in either config makes the whole full config the default, as vanilla's strict codec does.
  - The rolls of the `spawners/trial_chamber/*` reward tables and `equipment/trial_chamber_*` (brief D12's tests) move to group `trial-spawners-run`, which ejects the rewards and equips the mobs.
  - **For group `trial-spawners-run`:** the display's clock is `EntityView.world`'s game time. On the host that is the server entity world's tick count, which starts at 0 each session like `Level.game_time` (not loaded from `level.dat`). A friend's is the host's day clock (`session.rs` `time_of` sends `day.ticks` as `game_time`). `next_mob_spawns_at` and `cooldown_ends_at` are absolute level times saved in the block entity, so that group needs one clock for the server, the save and both displays before the active spin and the cooldown are right; until then a friend's active cage spins at the fastest rate.
- **Test steps (new world `IW4L_MINECRAFT_WORLD=trial10`, survival, Normal):**
  1. `/locate structure minecraft:trial_chambers`, `/tp` to it (it is underground, about y −40 to −20) and dig down with the pickaxe. Copper and tuff rooms. The zombie, husk, spider, skeleton, stray, bogged, cave spider, baby zombie and slime trial spawners each show a small mob turning slowly inside (about one turn every 2 seconds) with a small orange flame now and then, and a low hum now and then (**303_trial_spawner_waiting**).
  2. Breeze and silverfish trial spawners stay dark with no mob and no flames (those mobs are not in the game yet).
  3. Walking up to a spawner does nothing more yet: the trial (detection, mobs, rewards) is the next group.
  4. Friend window, in the same chamber: the friend sees the same mobs turning and the same flames, and hears the hum.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:cobblestone*64,minecraft:ladder*32,minecraft:water_bucket
```

### 2. Minecraft spawn eggs set a trial spawner's mob

After 97db956b.

- **Vanilla (javap, 26.3):** a spawn egg on a `Spawner` block entity calls `setEntityId`; `TrialSpawnerBlockEntity.setEntityId` is `TrialSpawner.overrideEntityToSpawn(type, level)`:
  - `TrialSpawnerStateData.reset()`: current mobs cleared, `spawn_data` dropped, and `resetStatistics`: detected players cleared, `total_mobs_spawned`, `next_mob_spawns_at` and `cooldown_ends_at` 0. `ejecting_loot_table` is kept.
  - `FullConfig.overrideEntity(type)`: both configs become inline copies (`Holder.direct`) through `TrialSpawnerConfig.withSpawning(type)`, which keeps every number and loot table and makes `spawn_potentials` that type alone (`WeightedList.of`: weight 1, a `SpawnData` holding only the id, no custom spawn rules, no equipment). The range and the cooldown length are kept.
  - Then `setState(INACTIVE)` (`setBlockAndUpdate`). The next tick picks the new mob and the spawner waits for players (commit 1's pass).
- **Changed:**
  - `trial_spawner.rs`: `TrialConfig::with_spawning`, `FullConfig::write` (both configs inline, range and cooldown length left out at their defaults), `override_entity` (the block entity after an egg) and `spawn_egg` (the tag replaced, the block set inactive with flags 3; `false` when the block entity is not a trial spawner).
  - `server.rs` `ServerSim::spawner_egg`: a trial spawner takes the egg through `spawn_egg`. The refusals before it stay as they were (a monster in peaceful; `spawner_blocks_work` off, with the vanilla chat line `advMode.notEnabled.spawner`), and anything that is neither spawner still answers "no monster spawner".
  - Vendored `world/src/level/container.rs`: `Level::set_block_entity_tag` replaces a block entity's whole tag and marks it changed, so the host's display and the friends get the new tag.
- **Tests:**
  - `trial_spawner::tests::an_egg_makes_both_configs_spawn_its_mob_and_resets_the_trial` (registry configs become inline copies with only the skeleton at weight 1 and every number kept, the state data goes except `ejecting_loot_table`, a cooldown length of 600 stays; a freshly placed spawner gets the default numbers and leaves the cooldown length out).
  - `server::tests::an_egg_sets_a_trial_spawners_mob` (a placed trial spawner stays inactive; a zombie egg makes both configs zombie-only, drops the spawn data and reports the block entity; the next tick it waits for players with a zombie; a skeleton egg on the waiting spawner sets it inactive and the next tick shows a skeleton; a creeper egg in peaceful and any egg with `spawner_blocks_work` off are refused and change nothing; an egg on a block without a spawner is refused).
- **Decisions:** none beyond vanilla; the game event `block_change` that vanilla's egg posts is track W's (game events).
- **Test steps (world `trial10`, survival, Normal):**
  1. 30 blocks from the chamber, place a trial spawner from the kit on stone. It stays dark (no mob). Use a zombie spawn egg on it: the egg is used up, and a small zombie turns inside with small orange flames (**306_trial_spawner_egg**). Zombies do not come out yet: the trial is the next group.
  2. Use a skeleton egg on the same spawner: the zombie inside becomes a skeleton.
  3. Use a husk egg on one of the chamber's trial spawners: it now shows a husk. A slime egg on another shows a slime.
  4. `/difficulty peaceful` and use a zombie egg on a trial spawner: nothing changes and the egg stays. `/difficulty normal`.
  5. `/gamerule spawner_blocks_work false` and use an egg: the chat says spawners are disabled and the egg stays. `/gamerule spawner_blocks_work true`.
  6. Friend window: the friend uses a slime egg on the host's placed trial spawner; both see the slime turning inside.
- **Kit (one line for the whole group):**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:trial_spawner*3,minecraft:zombie_spawn_egg*4,minecraft:skeleton_spawn_egg*2,minecraft:husk_spawn_egg*2,minecraft:slime_spawn_egg*2,minecraft:stone*16,minecraft:cobblestone*64,minecraft:ladder*32,minecraft:water_bucket
```

## Group trial-spawners-run

Brief D12, second half: the spawn rules of spawners, the visual line of sight, the trial itself with its rewards, and its particles and sounds.

### 1. Minecraft spawners spawn husks, strays and slimes without open sky or slime chunks

After 1294cf59.

- **Vanilla (javap, 26.3):**
  - `EntitySpawnReason.isSpawner` is true for `SPAWNER` and `TRIAL_SPAWNER`; `ignoresLightRequirements` only for `TRIAL_SPAWNER`.
  - `Monster.checkMonsterSpawnRules`: `(ignoresLightRequirements || isDarkEnoughToSpawn) && checkMobSpawnRules`, and `Mob.checkMobSpawnRules` is true for a spawner without looking at the block below.
  - `checkSurfaceMonstersSpawnRules` (husks) and `Stray.checkStraySpawnRules` add `canSeeSky` only when the reason is not a spawner.
  - `Slime.checkSlimeSpawnRules`: not peaceful, then for a spawner only `checkMobSpawnRules` (no swamp, no slime chunk).
  - `Drowned.checkDrownedSpawnRules` for a spawner: not peaceful and (ignores light or dark enough), no water test.
  - `BaseSpawner` (mob spawners) also asks `Mob.checkSpawnRules` (the walk target value: monsters need the dark); `TrialSpawner.spawnMob` never does, only `checkSpawnObstruction`.
- **Changed:** vendored `world/src/natural_spawner/spawning.rs`: `SpawnerReason` (`Spawner`, `TrialSpawner`) and `spawner_may_spawn(.., reason)`: monsters, husks, strays and drowned need the dark only from a mob spawner, slimes need only "not peaceful", and the walk test runs only for a mob spawner. `world/src/level/spawning.rs`'s mob spawner passes `Spawner`. Group commit 3's trial spawner passes `TrialSpawner`.
- **Tests:** vendored `natural_spawner::spawning::tests::spawners_spawn_husks_strays_and_slimes_without_open_sky_or_a_slime_chunk`: in a dark cave (no sky, outside a slime chunk) both reasons spawn husks, strays, slimes and zombies, while the natural rules still refuse husks, strays and slimes there and accept zombies; with block light 15 a mob spawner refuses husks, strays and zombies and a trial spawner takes them; slimes ignore the light, but not peaceful. The strider tests in `server::tests` pass the `Spawner` reason.
- **Decisions:** none beyond vanilla.
- **Test steps (world `trial10`, survival, Normal):**
  1. Deep underground (dig a room at y −30 with no sky above, no torches), place the kit's mob spawner and use a husk egg on it, stand 5 blocks away: husks come out in the dark (before this commit a husk spawner needed open sky) (**312_husk_spawner**).
  2. Break it, place another spawner in the same room and use a slime egg: slimes come out, also outside a slime chunk.
  3. Light the room with torches all around a husk spawner: it stops (a mob spawner still needs the dark for monsters).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:spawner*2,minecraft:husk_spawn_egg*2,minecraft:slime_spawn_egg*2,minecraft:torch*64,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:ladder*32
```

### 2. Minecraft line of sight passes through glass and iron bars like vanilla

After a877c6ae.

- **Vanilla (javap, 26.3):** `ClipContext.Block.VISUAL` tests `BlockState.getVisualShape`, which is the collision shape except where a block overrides `getVisualShape`. The overrides in 26.3 (every block class that names it):
  - `TransparentBlock` (glass, stained glass, tinted glass), `IronBarsBlock` (iron and copper bars, glass panes) and `PowderSnowBlock`: empty.
  - `MudBlock` and `SoulSandBlock`: a full cube (their collision is 14/16 high).
  - `SnowLayerBlock`: `SHAPES[layers]` (the collision is `SHAPES[layers - 1]`), so one layer is seen but not felt.
  - `FenceBlock`: `getShape`, the outline. `CrossCollisionBlock` builds both from the same post and arms (`FenceBlock(4, 16, 4, 16, 24)`): the outline is the collision shape cut at 16/16 instead of 24/16.
- **Changed:** vendored `world/src/level/physics.rs`: `ClipBlocks::Visual` and `Level::block_visual_shape` with the overrides above (block classes from the catalog, ancestors included), used by `clip_first_hit`. Its users are the trial spawner (next commit: a player seen from the spawner, a spawn spot that sees the spawner) and group trial-ominous's upward clip.
- **Tests:** `server::tests::the_visual_clip_passes_glass_and_iron_bars_and_stops_at_stone_and_a_fence_outline`: a ray along a row at y 200 passes glass, red stained glass, tinted glass, iron bars, a glass pane and powder snow with the visual clip (the collider clip stops at all but powder snow); stone and an oak fence stop both; a ray at 0.95 of the block is stopped by soul sand and mud only visually; a ray at 0.05 is stopped by one snow layer only visually; air stops neither.
- **Decisions:** none beyond vanilla.
- **Test steps:** none on its own; it is tested in game through the next commit (a trial spawner sees you through glass, step 3 there).

### 3. Minecraft trial spawners run their trial and eject rewards like vanilla

After 20ade5b0.

- **Vanilla (javap, 26.3):** as brief D12 lists (`TrialSpawner.tickServer`, `TrialSpawnerState.tickAndGetNext`, `TrialSpawnerStateData`, `PlayerDetector.NO_CREATIVE_PLAYERS`), checked again this run. The points the code follows exactly:
  - Each tick: `current_mobs` drops every UUID whose entity is not loaded, dead or more than 47 blocks away (`blockPosition().distSqr(pos) > 47²`); if any went, `next_mob_spawns_at = now + ticks_between_spawn`.
  - inactive → waiting_for_players once the next spawn data has a mob. waiting_for_players: `canSpawnInLevel` false resets the statistics; no mob → inactive; a detected player → active. active: rules off → statistics reset, waiting; additional players counted before detecting; all spawned and none tracked → `cooldown_ends_at = now + target_cooldown_length`, totals and next spawn 0, waiting_for_reward_ejection; else at `now >= next_mob_spawns_at` and fewer than the simultaneous target, `spawnMob`, and on success the UUID is tracked, counted, `next_mob_spawns_at = now + ticks_between_spawn` and a new `spawn_data` is picked. waiting_for_reward_ejection: `(float) now >= (float) (cooldown_ends_at − cooldown) + 40` plays `block.trial_spawner.open_shutter` → ejecting_reward. ejecting_reward: when `(float) (now − start) % 30 == 0`: no detected player left → `close_shutter`, the loot table is forgotten, cooldown; else the table is picked once (`loot_tables_to_eject`, weighted, level random), rolled and ejected, and one detected player removed. cooldown: detection (nothing gets through without an omen, group trial-ominous); a detected player → active; `now >= cooldown_ends_at` → `removeOminous`, `reset` (tracked mobs, spawn data and statistics), waiting_for_players.
  - `canSpawnInLevel`: `spawner_blocks_work`, not peaceful, `spawn_mobs`.
  - `tryDetectPlayers` only when `(pos.asLong() + gameTime) % 20 == 0`; players with `blockPosition().closerThan(pos, required_player_range)`, not creative, not spectator; the first scan needs sight from the block centre to the eye (`ClipContext` VISUAL, no fluid: sight when the hit block is the spawner or nothing is hit); once someone is detected, later scans need no sight. When anyone was added: `next_mob_spawns_at = max(now + 40, next_mob_spawns_at)` and level event 3013 (3019 when ominous) with the detected count.
  - `spawnMob`: the entity's saved `Pos`, else `x + (r1 − r2) × spawn_range + 0.5`, `y + nextInt(3) − 1`, `z + (r3 − r4) × spawn_range + 0.5` (level random); no collision in the type's box, sight from the spot to the block centre, `checkSpawnRules(TRIAL_SPAWNER)`, the entry's `custom_spawn_rules`; yaw `nextFloat() × 360`; `checkSpawnObstruction`; `finalizeSpawn(TRIAL_SPAWNER)` only for a tag that is just an id; `setPersistenceRequired`; level events 3011 at the spawner and 3012 at the mob (data 1 when ominous).
  - `ejectReward`: `getRandomItems` with the empty context (the table's own random sequence), each stack through `DefaultDispenseItemBehavior.spawnItem(level, stack, 2, UP, atBottomCenterOf(pos) + 1.2 up)`, then level event 3014 when anything came out.
  - The state data keys (`TrialSpawnerStateData$Packed`): `registered_players` and `current_mobs` (UUID int arrays), `cooldown_ends_at`, `next_mob_spawns_at`, `total_mobs_spawned`, `spawn_data`, `ejecting_loot_table`, each left out at its default.
  - `Level.getGameTime` is the world's `Time` from `level.dat`, the same in every dimension, and stops under `/tick freeze`.
- **Changed:**
  - `minecraft_terrain/src/trial_spawner.rs`: `StateData` (read and write of the keys above), `TrialPlayer`, the `TrialWorld` trait (time, players, sight, tracked mobs, rules, random, spawning, ejecting, sounds, level events), `TrialSpawner::tick` with the six states as above, and `LevelTrials`, the level's `TrialWorld`. `tick_spawners` runs every trial spawner block entity in a ticking chunk (sorted, as `wake_spawners` did, which it replaces), reads its configs through a cache of the registry configs (`FullConfig::read_with`), and writes back the state data (marked changed, so the host's display and the friends get it) and the block's `trial_spawner_state` and `ominous`.
  - `server.rs`: `ServerSim::tick_trial_spawners(players)` in `run_tick` right after `sim.tick()`, inside the `if normal` block and before `take_spawned`, so `/tick freeze` stops trials and new mobs join the entity world that tick. Players come from `TickInput.mob_players` (spectators left out; `attackable` is false for creative players), with their UUIDs. The living mobs of the level are gathered once by UUID with their block positions. `wake_spawners` left `ServerSim::tick`.
  - Vendored `world/src/level/spawning.rs`: `Level::summon_trial_mob` (the trial spawn rules of commit 1, `custom_spawn_rules`, the yaw, finalize only for an id-only tag, `PersistenceRequired`), pushed to `Level.spawned`. Vendored `world/src/level/dispenser.rs`: `spawn_item` is now `spawn_dispensed_item(position, direction, accuracy, stack)`, which the dispenser calls with accuracy 6 and the trial spawner with 2.
  - **One clock.** `ServerSim::set_game_time` (and `Command::GameTime`) sets the level's and the entity world's game time. `start_level` (`minecraft_survival/src/server.rs`) gives every level the world's game time when it starts (`Globals::game_time`: `level.dat`'s `Time` plus the ticks since load, the value `level.dat` is saved with). Both then count server ticks, so a trial spawner's `next_mob_spawns_at` and `cooldown_ends_at` mean the same after a save and load, and the host's display (the entity world's snapshot) reads the same clock. Friends get each level's game time in `ToClient::Time` (`send_time`; before it was the day clock) and count it on by client ticks between messages (`RemoteLevel.synced_game_time`, `render_anim/src/minecraft_remote.rs`), so a friend's active cage spins at the host's speed and the cooldown smoke bursts once a second (it repeated while the friend's clock stood still).
- **Tests:**
  - `trial_spawner::tests`: `a_waiting_spawner_detects_a_survival_player_in_range_and_sight_on_its_twenty_tick_phase` (creative, 14 blocks away and out of sight are not detected; a survival player is, only on the 20-tick phase, with event 3013 data 1 and the first spawn 40 ticks later; a second player out of sight joins at the next phase), `a_trial_spawns_its_mobs_in_turn_ejects_one_reward_per_player_and_cools_down` (two players: 8 mobs, at most 3 at once, at least 40 ticks apart, the first 40 ticks after detection, each with events 3011 and 3012, within the spawn range; then the shutter 40 ticks after the last death, two ejections of the same table 30 ticks apart, the close sound, cooldown 36000 ticks from the end, then waiting with the data reset and a new trial), `a_mob_farther_than_47_blocks_is_untracked_and_replaced_after_ticks_between_spawn` (47 blocks is kept, 48 is not; the replacement comes 40 ticks later), `peaceful_or_spawning_rules_off_send_a_trial_back_to_waiting`, `a_spawner_of_a_mob_the_game_does_not_simulate_stays_inactive` (a breeze spawner, the empty default config, and a waiting spawner whose next mob is a breeze), `the_state_data_round_trips_every_key`, `the_trial_chamber_reward_tables_roll_keys_and_consumables` (gated on the data: the key table gives one trial key, the ominous one an ominous trial key, consumables always give items, at least 4 kinds in 100 rolls).
  - `server::tests::a_trial_spawner_spawns_zombies_for_a_near_player_and_rewards_after_they_die` (gated: a zombie egg on a placed trial spawner on a platform at y 200, the clock at 1 000 000, a survival player 4 blocks away; each zombie removed as it comes: 6 persistent zombies within the spawn range, events 3013, 6 × 3011 and 3012, one 3014, the shutter sounds, the states waiting, active, waiting for ejection, ejecting, cooldown, a reward item on top of the spawner, `cooldown_ends_at` 36000 after the end; a creative player starts nothing). The older trial tests run `tick_trial_spawners` after `tick`.
- **Decisions:**
  - A mob kind the game does not simulate (not in `server_mobs::SIMULATED_KINDS`) counts as "no mob to spawn": such a spawner stays inactive, and a waiting or active one whose next mob is such a kind goes inactive (vanilla only asks for an id or any spawn potentials). Breeze and silverfish spawners wake up by themselves once those mobs land.
  - Ominous spawn data with `equipment` (only the ominous configs have it) is spawned without the equipment for now: spawners turn ominous only in group trial-ominous, which adds `EquipmentUser.equip` there.
  - Detection takes `attackable` players (alive and not creative or spectator). Vanilla also counts a dead player lying in range; a dead player respawns at once in the fork's flow, so the difference is a death screen's length.
  - The clock: each level starts from the world's game time and then counts its own server ticks. Vanilla has one counter for every dimension; here a level that is not ticking (no player in it) does not count, so a trial spawner's cooldown pauses while nobody is in its dimension. Trial chambers are only in the Overworld, so this only shows when every player stays in the Nether or the End. `level.dat`'s `Time` is still saved from the day clock as before.
  - The block entity is written back whenever its state data changed (vanilla only sends the client an update tag on `markUpdated` and state changes): friends get the whole tag, as for mob spawners.
  - Sounds go through `Level.sounds` like the other level sounds (heard by host and friends, category ambient).
- **Test steps (world `trial10`, survival, Normal, the chamber of group trial-spawners):**
  1. Walk into sight of a zombie trial spawner (within 14 blocks): it lights up (brighter block), the mob inside spins faster, and about 2 seconds later zombies come out one at a time (one a second), 6 in all, never more than 3 at once. The husk, spider and plain skeleton spawners behave the same; a "slow ranged" skeleton spawner (rooms with raised platforms) sends one every 8 seconds, up to 4 at once (**304_trial_spawner_active**).
  2. Kill them. About 2 seconds after the last one dies the shutter opens with a click and one reward (a trial key, or food and potions) pops out on top, then it goes dark and smokes (**305_trial_reward**). Walk up to it: nothing more happens (cooldown, 30 minutes).
  3. At a fresh spawner, stand behind a glass wall in its room: it detects you through the glass. Stand behind a stone wall instead: it does not.
  4. During a trial, lure one zombie 50 blocks away (or run so it follows you and come back without it): the spawner forgets it and sends a new one about 2 seconds later.
  5. `/gamemode creative` and walk to another fresh spawner: it stays waiting. `/gamemode survival`.
  6. 30 blocks from the chamber, place a trial spawner on stone and use a zombie egg on it: since you stand within 14 blocks in sight, zombies start at once: one every 2 seconds, 6 in all, never more than 2 at once (vanilla's default numbers) (**306_trial_spawner_egg**).
  7. Deep underground (dig a room at y −30, no sky), place trial spawners and use a husk egg on one and a slime egg on another: husks and slimes come out (no open sky or slime chunk needed). Put torches all around them: they still come (trial spawners ignore light).
  8. `/difficulty peaceful` while a trial runs: the zombies vanish and the spawner goes back to waiting. `/difficulty normal`: walking into sight starts a new trial.
  9. Start a trial, kill 2 of the zombies, then save and quit and load: the same spawner goes on where it was (it keeps its count and does not start over), and once all 6 have come and died it ejects the reward.
  10. Friend window, at a fresh spawner in the same chamber: the spawner spawns 8 zombies for two players (3 at once) and ejects two rewards (one per player, 1.5 seconds apart); the friend sees the light, the faster spin, the zombies and the rewards (**307_friend_trial**, both windows). The particles and sounds of detection, spawning and ejection come with the next commit.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:trial_spawner*3,minecraft:zombie_spawn_egg*4,minecraft:husk_spawn_egg*2,minecraft:slime_spawn_egg*2,minecraft:cobblestone*64,minecraft:glass*32,minecraft:ladder*32,minecraft:water_bucket
```

### 4. Minecraft trial spawners show their detect, spawn and eject particles and sounds

After 600beb1c.

- **Vanilla (javap, 26.3, `LevelEventHandler` and `TrialSpawner`'s client helpers):** every sound plays at the block centre with volume 1 and pitch `(r1 − r2) × 0.2 + 1`.
  - 3011: `addSpawnParticles`, 20 times a `smoke` and a flame (`TrialSpawner$FlameParticle.decode(data)`: 1 is the soul fire flame, anything else the flame) at one point `centre + (nextDouble − 0.5) × 2` on each axis, no motion. 3012: `block.trial_spawner.spawn_mob`, then the same. 3021: `block.trial_spawner.spawn_item`, then the same.
  - 3013: `block.trial_spawner.detect_player`, then `addDetectPlayerParticles`: `30 + min(data, 10) × 5` `trial_spawner_detection` particles at `x + 0.5 + (2 × nextFloat − 1) × 0.65`, `z` the same (drawn before `y`), `y + 0.1 + nextFloat × 0.8`, no motion. 3019: the same with `trial_spawner_detection_ominous`.
  - 3020: `block.trial_spawner.ominous_activate` at volume 0.3 for data 0 (the player) and 1 otherwise (the spawner), 30 ominous detection particles, then `addBecomeOminousParticles`: 20 times a `trial_omen` and a `soul_fire_flame` at one point around the centre (as 3011) with motion `nextGaussian × 0.02` on each axis.
  - 3014: `block.trial_spawner.eject_item`, then `addEjectItemParticles`: 20 times, at `pos + 0.4 + nextDouble × 0.2` on each axis with motion `nextGaussian × 0.02` each, a `small_flame` (its z motion × 0.25) and a `smoke`.
  - `TrialSpawnerDetectionParticle` (scale 1.5 from its provider): the 7-argument particle constructor with no motion, then x and z motion 0 and y × 0.9, friction 0.96, gravity −0.1, speeds up when its rise is blocked, collides, quad size × 0.75 × 1.5, lifetime `(int) (8 / randomBetween(0.5, 1) × 1.5)` at least 1, its 5 frames by age, block light 15, opaque, grows like smoke (`clamp((age + partial) / lifetime × 32, 0, 1)`), facing the camera around Y only.
  - `trial_omen` uses `SpellParticle$Provider`: x and z motion `0.5 − nextDouble` into the constructor with the given y, then y × 0.2 (x and z × 0.1 only when the given x and z were 0), friction 0.96, gravity −0.1, quad size × 0.75, lifetime `(int) (8 / (nextFloat × 0.8 + 0.2))`, no collision, translucent.
- **Changed:**
  - `level_events.rs`: the arms for 3011, 3012, 3013, 3014, 3019, 3020 and 3021 above (the host and friends both draw level events through `level_event`, the friends from `ToClient::LevelEvent`). 3020 and 3021 come from group trial-ominous; their effects are ready here.
  - `ambient_particles.rs`: particle kinds `TrialSpawnerDetection`, `TrialSpawnerDetectionOminous` and `TrialOmen` as above, with their sprites added to the atlas list.
  - `trial_spawner.rs`: the event ids 3020 and 3021 and the five sound names.
- **Tests:** `level_events::tests::trial_spawner_events_play_vanillas_sounds_and_particles` (3013 with 2 players: the detect sound and 40 detection particles in their box; 3019 with 25 players: 80; 3020: the activate sound at 0.3 for data 0 and 1.0 for data 1, 30 ominous detection particles, 20 trial omens and 20 soul flames; 3011: 20 smokes and 20 flames within a block of the centre, no sound; 3012 with data 1: the spawn sound and soul flames; 3021: the item sound; 3014: the eject sound, 20 small flames and 20 smokes in pairs at one point within 0.1 of the centre, the flame's z motion a quarter of the smoke's). `ambient_particles::tests::trial_detection_particles_rise_lit_through_their_frames_and_the_trial_omen_drifts` (lifetimes 12 to 24, no sideways motion, opaque, frames 0 to 4, full block light; the trial omen is translucent with its own sprite and lifetime 8 to 40; the sprites are in the atlas list).
- **Decisions:** the detection particle is drawn facing the camera like the other particles; vanilla turns it only around Y (`LOOKAT_Y`), which the particle mesher does not have. The tilt only shows when looking at it from far above or below.
- **Test steps (world `trial10`, survival, Normal, a fresh spawner in the chamber):**
  1. Walk into sight of a fresh zombie spawner: with the detect sound, a column of orange-gold sparks rises from it (about 35 of them, more with more players) (**313_trial_detect**).
  2. Each zombie that comes out appears in a puff of smoke and flames with a low spawn sound, and the spawner puffs too.
  3. After the last zombie dies, the reward pops out with a sound and a little burst of small flames and smoke at the top of the spawner (**314_trial_eject**).
  4. Friend window: the friend sees and hears the same detect, spawn and eject effects.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:trial_spawner*3,minecraft:zombie_spawn_egg*4,minecraft:husk_spawn_egg*2,minecraft:slime_spawn_egg*2,minecraft:cobblestone*64,minecraft:glass*32,minecraft:ladder*32,minecraft:water_bucket
```

## Group trial-ominous

Brief D13: Trial Omen, ominous trial spawners and the items they drop from above.

### 1. Minecraft trial spawners turn ominous with Trial Omen

After bd0ba68c.

- **Vanilla (javap, 26.3):**
  - `TrialSpawnerStateData.tryDetectPlayers`: after the 20-tick phase and the "ominous and in cooldown" return, the sighted scan runs; when the spawner is not ominous and someone was sighted, `findPlayerWithOminousEffect` walks the sighted list in order and returns the **first** player with Trial Omen, else the **last** one with Bad Omen (the bytecode overwrites its Bad Omen candidate on every match; the brief said the first). With one found: Bad Omen becomes Trial Omen (`transformBadOmenIntoTrialOmen`: Bad Omen removed, `MobEffectInstance(TRIAL_OMEN, 18000 × (amplifier + 1), 0)` added), level event 3020 data 0 at the block of the player's eyes, then `applyOminous`. In cooldown nothing goes further without such a change. The detection event 3013/3019 is **not** sent on the scan that turned the spawner ominous.
  - `TrialSpawner.applyOminous`: the block's `ominous=true`, level event 3020 data 1 at the spawner, then `TrialSpawnerStateData.resetAfterBecomingOminous`: each tracked mob gets level event 3012 data 0 (normal flames) at its block, drops its preserved equipment and is discarded; `spawn_data` is cleared when the ominous config has spawn potentials; `total_mobs_spawned` 0; tracked mobs cleared; `next_mob_spawns_at = now + ominous ticks_between_spawn`; `cooldown_ends_at = now + 160` (`ticksBetweenItemSpawners`, the first ominous item waits for it). `removeOminous` at the end of the cooldown sets `ominous=false` (the cooldown arm already did).
  - `TrialSpawnerState.tickAndGetNext` takes `activeConfig()` once at its start, before detection: a spawner that turns ominous during an active tick finishes that tick with the normal config.
  - `MobEffects.TRIAL_OMEN` has `withSoundOnAdded(event.mob_effect.trial_omen)`, played only when the effect is new (`onEffectAdded`).
  - `TrialSpawner.spawnMob`: after `setPersistenceRequired`, the spawn data's `equipment` goes through `Mob.equip(EquipmentTable)` → `EquipmentUser.equip(table, params, seed 0, slot_drop_chances)`: the table rolled with the mob's position and itself as `THIS_ENTITY`; each stack goes to its `equippable` slot if that slot was not filled by this roll, an item without `equippable` to the main hand if free, else nowhere; `EquipmentSlot.limit` keeps one item in an armour slot; the slot's drop chance is set when the table names it (`slot_drop_chances` is one float for every slot or a map).
- **Changed:**
  - `minecraft_terrain/src/server.rs`: `PlayerOmens.trial_omen`. `ServerSim::tick_trial_spawners(players, omens)` gives each trial player its Bad Omen and Trial Omen, removes the mobs the pass discarded from the entity world (and their saved tags), and returns the players whose Bad Omen it turned, after setting their `input.omens` entry to Trial Omen and no Bad Omen, so the later runs of a batch see the omen already turned. `run_tick` puts them in the new `Output.trial_omens`.
  - `minecraft_terrain/src/trial_spawner.rs`: `TrialPlayer.bad_omen` and `trial_omen`; `TrialWorld::turn_bad_omen` (the player's entry becomes Trial Omen at once, so the next spawner in the same pass sees it) and `discard_mob`; the omen check in `try_detect_players`, `become_ominous`, `apply_ominous` with the reset above; the active arm takes its config before detection. `TrialOutcome` (players turned, mobs discarded) is returned by `tick_spawners`. `equip`: the spawn data's equipment table rolled into the new mob's `equipment` and `drop_chances`. `TRIAL_OMEN_TICKS` 18000.
  - Vendored `world/src/level/spawning.rs`: `summon_trial_mob` returns the mob without adding it; `LevelTrials::spawn_mob` equips it and adds it to `Level.spawned`.
  - `minecraft_survival/src/survival.rs`: `omens()` fills `trial_omen`; `turn_bad_omen_into_trial` (Bad Omen removed, Trial Omen for 18000 × (amplifier + 1) at amplifier 0; true when the Trial Omen is new).
  - `minecraft_survival/src/entities.rs`: the omens sent each tick carry `trial_omen`; `PlayerOutputs.trial_omen` routed from `Output.trial_omens` to that player like the raid omens.
  - `render_anim/src/minecraft_players.rs` `settle` (the host's step and every friend's): turns that player's Bad Omen and plays `event.mob_effect.trial_omen` at their feet when the Trial Omen is new.
- **Tests:**
  - `trial_spawner::tests::a_spawner_that_sees_a_player_with_bad_omen_turns_ominous_and_gives_trial_omen` (out of sight nothing; in sight on the phase: turned once, the player's entry becomes Trial Omen, events 3020 at the eyes and at the spawner and no 3013, next spawn after the ominous config's 100 ticks, `cooldown_ends_at` 160 ticks on, the next mob picked from the ominous config).
  - `trial_omen_turns_a_spawner_ominous_as_it_is_and_comes_before_bad_omen` (Trial Omen is used without turning anyone; with two Bad Omen players the last one in sight is turned; a plain player gets the plain detection event).
  - `turning_ominous_mid_trial_discards_its_mobs_in_flames_and_starts_over` (both tracked zombies discarded with 3012 data 0 at their blocks, totals 0, the new player added, next spawn 40 and items 160 ticks on).
  - `an_omen_wakes_a_cooling_spawner_but_an_ominous_one_cools_down_and_turns_plain`.
  - `an_ominous_trial_ejects_from_its_ominous_reward_tables` (the ominous key table 3 times in 10 over 1000 ejections).
  - `trial_equipment_goes_to_its_armour_slot_or_the_main_hand` (slots and drop chances).
  - `server::tests::a_trial_spawner_turns_ominous_for_a_bad_omen_and_arms_its_new_mobs` (gated: the chamber's zombie configs; a plain trial starts, then a Bad Omen in the input turns it: the player is reported once and the input says Trial Omen, the spawner turns ominous with both 3020 events, the plain zombie leaves in flames and is gone from the entity world, no further request in 200 ticks, every new zombie holds an iron or diamond sword with drop chance 0, and the entity world's zombies hold their swords).
  - `survival::tests::a_trial_spawner_turns_bad_omen_into_trial_omen_for_eighteen_thousand_ticks_a_level` (Bad Omen IV becomes Trial Omen I for 72000 ticks; nothing without Bad Omen; a second turn keeps the longer omen and makes no sound).
- **Decisions:**
  - The host's survival turns the Bad Omen with the amplifier it has when the request arrives (vanilla reads the player's effect at that moment), so `Output.trial_omens` carries only player ids. A Bad Omen gone by then turns nothing, but the spawner is ominous anyway (the server saw the omen).
  - A discarded mob does not drop its preserved equipment (vanilla drops items with a drop chance above 1, which a trial mob only has when it picked something up during the trial). Logged as not vanilla.
  - An item's equipment slot comes from its name (helmets, heads, skulls and carved pumpkins to the head, chestplates and elytra to the chest, leggings, boots, everything else to the main hand): the level's item catalog has no `equippable` data. It covers every item of the three `equipment/trial_chamber*` tables.
  - The mobs a spawner discards leave the entity world right after the trial pass, in the same tick.
- **Test steps (world `trial10`, Normal, survival, full kit armour, in the chamber of group trial-spawners; use spawners that have not run yet):**
  1. Drink a plain ominous bottle: the Bad Omen icon shows at the top right. Walk into sight of a fresh zombie spawner: a deep chime, blue (soul) flames swirl at your head and at the spawner, and the Bad Omen icon becomes Trial Omen (15 minutes) (**308_trial_omen**).
  2. The spawner burns with blue flames. Its zombies come with iron or diamond swords and often iron, chainmail or diamond helmets and chestplates; at a skeleton spawner the skeletons come with bows. Kill them: no sword or armour drops. The items falling from above come with the next commit.
  3. Win the trial: it ejects an ominous trial key or ominous consumables (**310_ominous_reward**).
  4. While Trial Omen lasts, walk to a second fresh spawner: it turns blue at once, with the flames but no new chime.
  5. `/effect clear @s`. Start a plain trial at a fresh spawner, and while its zombies are out drink a level II bottle (Bad Omen II) and step into its sight: the zombies vanish in puffs of flame and the trial starts over, blue. The Trial Omen icon shows 30 minutes.
  6. Save and quit and load: the spawner is still blue and goes on.
  7. Friend window: the friend drinks a bottle and walks into sight of a third fresh spawner: their Bad Omen becomes Trial Omen with the chime, and both windows see the blue flames (**311_friend_ominous**).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle[ominous_bottle_amplifier=1]*2,minecraft:ominous_bottle*2,minecraft:milk_bucket*2,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:totem_of_undying,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:trial_spawner*2,minecraft:zombie_spawn_egg*4,minecraft:cobblestone*64,minecraft:ladder*32,minecraft:water_bucket
```

### 2. Ominous Minecraft trial spawners drop items from above

After 99f0787e.

- **Vanilla (javap, 26.3):**
  - `TrialSpawnerState.tickAndGetNext`, active arm: after detection, while the spawner is ominous, `spawnOminousOminousItemSpawner`. It takes `getDispensingItems` (built once per block entity: the active config's `items_to_drop_when_ominous` rolled with `getRandomItems(empty params, seed)`, seed = level seed + `BlockPos(floor(x / 30f), floor(y / 20f), floor(z / 30f)).asLong()`, each stack once at weight = its count), picks one with the level random every tick (nothing when the list is empty), and only when `gameTime >= cooldown_ends_at` looks for a spot: the detected players still in the level, alive, not creative or spectator, within `required_player_range` (squared, from the block centre to the feet); none → no spot. Then `selectEntityToSpawnItemAbove`: `nextBoolean()` picks the tracked mobs (alive, in range) when true, else those players; an empty pick gives nothing this time (no fallback), one is taken as is, more go through `Util.getRandom`. `calculatePositionAbove`: from the feet up by `bbHeight + 2 + nextInt(4)` (float), clipped (`VISUAL`, no fluid); the hit block's centre one block down (a miss counts as the end point's block); no spot when that block has a collision shape. There `OminousItemSpawner.create` (`spawn_item_after_ticks` = `nextIntBetweenInclusive(60, 120)` from the level random), `block.trial_spawner.spawn_item_begin` at the spot's block (volume 1, pitch `(r1 − r2) × 0.2 + 1`), and `cooldown_ends_at = now + 160`.
  - `OminousItemSpawner`: 0.25 × 0.25, `noPhysics`, `isIgnoringBlockTriggers`, never hurt; saves `item` (when not empty) and `spawn_item_after_ticks`; `tickCount` is not saved. Server tick: at `tickCount == spawn_item_after_ticks − 36` `block.trial_spawner.about_to_spawn_item` (neutral) at its block; at `tickCount >= spawn_item_after_ticks` `spawnItem` (nothing when empty), then `kill`. `spawnItem`: a `ProjectileItem` is shot straight down from its position with `Projectile.spawnProjectileUsingShoot` at its `DispenseConfig` power and uncertainty (arrows, tipped and spectral arrows, snowballs and eggs 1.1 and 6.0; fire charges and wind charges 1.0 and 6.6666665; splash and lingering potions and experience bottles 1.375 and 3.0), owned by the spawner; any other item becomes `ItemEntity(level, x, y, z, stack)` (motion `nextDouble × 0.2 − 0.1`, 0.2, `nextDouble × 0.2 − 0.1`, no pickup delay); then level event 3021 with data 1 and the item cleared. `ArrowItem.asProjectile` makes an `Arrow` from the stack, so an `arrow` carrying `potion_contents` (the table's poison and slowness arrows) hits with that potion at the arrow's duration scale (1 for `arrow`, 0.125 for `tipped_arrow`).
  - Client: every 5 game ticks 1 to 3 `ominous_spawning` particles from `position + 0.4 × (gauss − gauss)` on each axis flying into it. The provider is `FlyStraightTowardsParticle$OminousSpawnProvider`: lifetime `nextFloat × 5 + 25`, size `0.1 × (nextFloat × 0.5 + 0.2)` scaled by `randomBetween(3, 5)`, position `start + offset × (1 − age / lifetime)`, colour from `0xFF45AEFE` to white over its life, block light 15, opaque, no physics. `OminousItemSpawnerRenderer`: the item with its ground transform at full light (15728880), scaled `min(age, 50) / 50` while `age <= 50`, turned `wrapDegrees(age × 40)` about Y.
  - The 26.3 `items_to_drop_when_ominous` table rolls one lingering potion (wind charged, oozing, weaving, infested, strength, swiftness or slow falling) and one of an arrow, a poison arrow, a strong slowness arrow, 1-3 fire charges or 1-3 wind charges.
- **Changed:**
  - Vendored `world/src/level/entity.rs`: `EntityKind::OminousItemSpawner(OminousData { stack, spawn_after })`, `Entity::ominous_item_spawner` (0.25 wide and tall, no physics, no gravity), `ominous_data`, its tick (the warning sound, then the item handed to the new `Level.ominous_drops` with event 3021 data 1, and removal); pressure plates ignore it. Vendored `world/src/level/mod.rs`: `Level.ominous_drops`.
  - Vendored `entities/src/world.rs`: `EntityWorld::spawn_effect_arrow` (an arrow with its effect).
  - `minecraft_terrain/src/trial_spawner.rs`: the active arm's `spawn_ominous_item` and `item_spot` as above; `TrialWorld` gains `mob_body`, `dispensing_items`, `spot_above` and `spawn_ominous_item`; `TrialPlayer.feet`; `LevelTrials` rolls the list with the seed (`dispensing_seed`, `dispensing_list`) and keeps it in the server's `Dispensing` cache, clips upward and adds the entity with its sound. `dispense_config`, `arrow_effect`, `stack_tag`/`tag_stack`, `OminousItemView`, `ominous_item_views`, `ominous_item_scale`, `ominous_item_spin`.
  - `minecraft_terrain/src/server.rs`: `ServerSim::drop_ominous_items` right after the level's tick: arrows into the entity world with no owner and the potion's first effect, fire charges as small fireballs with no owner at the shot speed, the other projectiles through the new `Thrown::dispensed` (`thrown.rs`: no player owner, no mob shooter), anything else as an item entity. The trial players carry their feet, the trial mobs their feet and height. `EntitySnapshot.ominous_items`. A chunk that unloads takes its ominous item spawners with it (they are saved with it).
  - `minecraft_terrain/src/server_mobs.rs`: `minecraft:ominous_item_spawner` saved and loaded with the chunk's entities (`item`, `spawn_item_after_ticks`, `Motion`).
  - `server.rs` `tick_thrown` and `splash.rs`: an item thrown by no player (`MOB_THROWER` with no shooter) leaves a lingering cloud with no owner, and its direct hits do not blame a player (`PLAYER_TARGET + MOB_THROWER` overflowed).
  - `minecraft_terrain/src/ambient_particles.rs`: `Kind::OminousSpawning` and `AmbientParticles::ominous_spawning`; `mesh.rs` `append_ominous_items`; `mob_tracking.rs`: the ominous item spawner's tracking range 8.
  - `minecraft_survival/src/entity_view.rs` (host and friends): keeps the snapshot's ominous items, emits their particles every 5 game ticks in `tick_scene` and draws them; `mob_sender.rs` sends them to friends within 8 chunks.
- **Tests:**
  - `trial_spawner::tests::an_ominous_trial_drops_items_above_its_players_and_mobs_every_160_ticks_at_most` (a plain trial drops nothing; an ominous one drops at least 15 items in 4000 ticks, never closer than 160 ticks, above its player at 2.5 to 5.5 blocks over the feet or above its mobs, never above a player out of range; a low ceiling keeps the item down; an empty list drops nothing).
  - `the_dispensing_list_weighs_each_item_by_its_count_and_is_seeded_by_the_spawner_area`, `ominous_items_fly_with_vanillas_dispense_power_and_carry_their_potion` (powers and uncertainties, a poison arrow's 900 ticks and a strong slowness tipped arrow's 50, the grow and turn, the item tag round trip).
  - `server::tests::an_ominous_item_spawner_warns_survives_a_save_and_drops_its_item_down` (gated: four spawners saved and loaded with their items and 70 ticks; the warning 36 ticks before; at 70 each is gone with event 3021 data 1: a wind charge thrown down from its spot, a poison arrow falling at about 1.1 with poison for 900 ticks, a small fireball with no owner, a diamond as an item with vanilla's motion; the wind charge is a block lower after a thrown tick).
  - `server::tests::a_trial_spawner_turns_ominous_for_a_bad_omen_and_arms_its_new_mobs` now runs 400 ticks after the omen: the first item appears at least 160 ticks after it, the real table's list holds a lingering potion with its potion and one arrow, fire charge or wind charge at weight 1 to 3, the item is one of them, the `spawn_item_begin` sound plays, and the item has dropped by the end.
  - `ambient_particles::tests::ominous_spawning_particles_fly_into_their_item_and_turn_white`.
  - `mob_sender::tests::ominous_items_reach_friends_eight_chunks_away_with_their_tint_and_age` (the snapshot's codec round trip with an ominous item).
- **Decisions:**
  - The dispensing list is rolled once per spawner position and table and kept on the server (vanilla keeps it in the block entity until it unloads; the roll is seeded, so it is the same list).
  - An item spawner's age is not saved (vanilla neither): after a load it waits its whole 60 to 120 ticks again.
  - Arrows from above are entity-world arrows with no owner. They carry the first effect of their potion at the arrow's duration but always at level I (the fork's arrow effects have no level yet), so the strong slowness arrow slows at level I instead of IV, and they cannot be picked up (the fork's arrows have no pickup). Not vanilla, both logged.
  - A wind charge, snowball or egg from above that hits a mob directly does not hurt it (the fork's thrown items hurt mobs only for a player); the wind charge's burst still pushes. Not vanilla (vanilla's wind charge does 1 damage). Tridents and firework rockets, which the vanilla table never drops, fall as items.
  - The spot above a player uses the standing height 1.8 (vanilla uses the current box: 1.5 crouching).
- **Test steps (world `trial10`, Normal, survival, full kit armour, in the chamber; a fresh spawner):**
  1. Drink a plain ominous bottle and walk into sight of a fresh zombie spawner: it turns blue (commit 1). About 8 seconds later a small item appears 2 to 5 blocks above you or above one of its zombies, growing for 2.5 seconds, spinning and lit, with blue sparks flying into it, and a soft chime (**309_ominous_item**).
  2. About 1.5 seconds before it drops a rising sound plays; then it drops with a puff of blue flames: a lingering potion breaks into a cloud, an arrow (sometimes a poison or slowness arrow) or a wind charge flies down, or a fire charge falls burning. A new item follows every 8 seconds or more while the trial lasts.
  3. Stand under a low roof (two blocks of headroom): the item appears just under the roof.
  4. While an item floats above you, save and quit, then load: the item is still there and drops 3 to 6 seconds later; the spawner is still blue and goes on.
  5. Win the trial: no items fall once the trial is over (waiting for the reward and cooldown).
  6. Friend window: the friend sees the floating items, their sparks and their drops, and hears the sounds (**311_friend_ominous**).
- **Kit (the same line as commit 1):**

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle[ominous_bottle_amplifier=1]*2,minecraft:ominous_bottle*2,minecraft:milk_bucket*2,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:totem_of_undying,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:trial_spawner*2,minecraft:zombie_spawn_egg*4,minecraft:cobblestone*64,minecraft:ladder*32,minecraft:water_bucket
```

## Group vaults

Brief D14: vaults, their trial keys, rewards once per player and the turning reward item.

### 1. Minecraft vaults take trial keys and eject rewards once per player

After 1ce25bec.

- **Vanilla (javap, 26.3), checked again this run; where it differs from the brief, the javap reading is used:**
  - `VaultConfig`: `loot_table` (`chests/trial_chambers/reward`), `activation_range` 4.0, `deactivation_range` 4.5, `key_item` (`ItemStack.lenientOptionalFieldOf`: a config without it has **no** key), `override_loot_table_to_display`; all lenient fields. A config whose activation range is above its deactivation range fails to read, and a vault that cannot read its `config` (or has none) takes `VaultConfig.DEFAULT` (a trial key). The block entity catalog's placed vault carries `config.key_item` = one trial key.
  - `VaultServerData`: `rewarded_players` (a linked set of UUIDs; adding the 129th drops the oldest), `state_updating_resumes_at`, `items_to_eject`, `total_ejections_needed`, each left out at its default; `last_insert_fail_timestamp` is not saved. `VaultSharedData`: `display_item`, `connected_players`, `connected_particles_range` (4.5, never changed by the server).
  - `VaultBlockEntity$Server.tick`: when `gameTime % 20 == 0` and the state is active, the display item becomes a random item (`Util.getRandom`) of a roll of the display table (the override, else the loot table) with `ORIGIN` and the **level random** (`getRandomItems(params, random)`, so not the table's own sequence); empty when there is no key or the state is inactive. Then, when `gameTime >= state_updating_resumes_at`, `VaultState.tickAndGetNext`: inactive and active update the connected players (activation range when inactive, deactivation range when active: `PlayerDetector.INCLUDING_CREATIVE_PLAYERS`, every non-spectator whose block position is `closerThan` the vault, no line of sight, minus the rewarded players), pause 20 ticks, active if any is connected else inactive; unlocking pauses 20 and becomes ejecting; ejecting with nothing left sets `total_ejections_needed` 0 and updates the connected players with the deactivation range, else it takes the progress, pops the **last** item and ejects it, shows the next last item, pauses 20.
  - `ejectResultItem`: `DefaultDispenseItemBehavior.spawnItem(level, stack, 2, UP, bottom centre + 1.2 up)`, level event 3017 data 0, `block.vault.eject_item` (blocks) at volume 1 and pitch `0.8 + 0.4 × progress`; `ejectionProgress` is 1 for a single ejection, else `1 − (left − 1) / (total − 1)`.
  - `setVaultState`: the block (flags 3), then `onExit` of the old state (ejecting: `block.vault.close_shutter`) and `onEnter` of the new one: inactive clears the display item and sends level event 3016 (data 1 when ominous); active cycles a display item if it has none and sends 3015 (data 1 when ominous); unlocking plays `block.vault.insert_item`; ejecting plays `block.vault.open_shutter`. All server sounds are `playSound(null, pos, event, BLOCKS)` at the block centre, volume 1, pitch 1 unless said.
  - `VaultBlock.useItemOn`: an empty hand or a state other than active passes (`TRY_WITH_EMPTY_HAND`); otherwise the server runs `tryInsertKey` and the answer is `SUCCESS_SERVER` (the arm swings). `tryInsertKey`: no key configured or inactive: nothing; not the same item and components as the key, or fewer than its count: `block.vault.insert_item_fail`; the player already rewarded: `block.vault.reject_rewarded_player` (both at most once per 15 ticks, `now >= last + 15`, the timestamp starting at 0); the reward roll (`VAULT` params: `ORIGIN`, the player's luck, `THIS_ENTITY`, the key as `TOOL`, the table's own random sequence) empty: nothing; else the key is consumed by `key_item.count` (creative keeps it), `unlock` (items to eject, the last one shown, pause 14, state unlocking), the player joins `rewarded_players`, and the connected players are updated with the deactivation range.
  - `ServerPlayerGameMode.useItemOn`: a sneaking player holding anything skips the block's `useItemOn`; adventure mode does not (only item placement checks `mayBuild`).
- **Changed:**
  - New `minecraft_terrain/src/vault.rs` (rustfmt-clean): `VaultState`, `VaultConfig`, `ServerData`, `SharedData` (read and written as above, defaults left out), `Vault::tick` and `Vault::insert_key` against a `VaultWorld` trait (time, players, random, the display and reward rolls, ejection, sounds, level events), the 15-tick fail sound limit and the 128-player cap; `LevelVaults`, the level's `VaultWorld`; `tick_vaults` (every vault block entity in a chunk the level ticks, sorted; the tag and the block state written back only on a change) and `insert_key`; `key_use` and `key_item_use` (`Change::Vault`).
  - `bucket.rs`: `Change::Vault(Pos)`.
  - `server.rs`: `ServerSim::tick_vaults(players)` in `run_tick` right after `tick_trial_spawners`, inside the `if normal` block (so `/tick freeze` stops vaults); `ServerSim::vault_key` and `Command::VaultKey { player, pos, stack }`, answered with `(player, unlocked)` in `Output.item_uses`; `ServerSim::set_enchantments` (the vault rolls use the host's enchantment book); the vault sounds go out in `Output.mob_sounds` (category blocks).
  - `trial_spawner.rs`: `uuids` and `uuid_list` are `pub(crate)` for the vault. `containers.rs`: `loot_enchanting` wraps the enchantment book as a loot `Enchanting`.
  - `minecraft_survival/src/actions.rs`: the use chain offers a key to the targeted vault (5 blocks, as lecterns) when it is active and the hand holds something, unless the player sneaks with an item; the hand swings; `Entities::vault_key` sends the command and waits on `item_use_waiting`, so `finish_item_use` spends one key only when the server says the vault opened, and never in creative (`consumes_items`). `entities.rs`: `vault_key`; `Entities::new` gives the server the enchantment book.
- **Tests:**
  - `vault::tests::a_vault_lights_for_any_player_within_four_blocks_and_dims_beyond_four_and_a_half` (4 blocks along an axis stays inactive, 3 lights it with event 3015 and a display item; 4 keeps it lit, 5 dims it with 3016 and no display; no server sound).
  - `a_trial_key_unlocks_it_and_it_ejects_one_item_a_second_rising_in_pitch` (insert sound, the 14-tick pause, the open shutter, three items ejected last first 20 ticks apart at pitch 0.8, 1.0 and 1.2 with 3017, then the close shutter and 3016; a second key on the dim vault does nothing and plays nothing; it stays dim for its rewarded player).
  - `a_single_ejection_plays_at_the_highest_pitch`.
  - `a_rewarded_player_is_refused_while_another_keeps_it_lit` (after the ejection the second player keeps it active with 3015; the rewarded player's key gets the reject sound; the other player's key opens it).
  - `a_wrong_key_fails_with_its_sound_at_most_once_in_fifteen_ticks` (an ominous key, a stick, a named trial key and an empty stack; sounds at 1001 and 1016 only; nothing rolled).
  - `an_ominous_vault_wants_an_ominous_key_and_rolls_its_own_table` (3015 and 3016 with data 1; the trial key fails; the ominous key rolls `reward_ominous`).
  - `the_display_turns_over_every_twenty_ticks_only_while_active` (the override table, on lighting up and every 20 ticks while lit, never while dim).
  - `a_vault_without_a_key_shows_nothing_and_takes_nothing` (a `config` without `key_item`; ranges the wrong way round and no config read as the default).
  - `the_newest_128_rewarded_players_are_kept`, `the_tag_round_trips_and_leaves_defaults_out`, `a_key_is_offered_only_to_an_active_vault_from_a_full_hand` (no `Change::Vault` for an empty hand, a vault in any other state or another block).
  - `server::tests::a_vault_spends_a_key_only_when_it_opens` (gated: a placed vault lit by a creative player two blocks away with a display item from the real reward table; through `ServerHandle` and `Command::VaultKey`: an ominous key answers false with the fail sound, a trial key true, a second key false; 200 ticks eject the rolled rewards at rising pitch with as many 3017 events and dim it with 3016; every vault sound is in category blocks; another key answers false with no sound; the vault is inactive, the items lie on it and `rewarded_players` holds the player's UUID).
- **Decisions:**
  - Vault keys go through the block-use part of the use chain (as lectern books), not the bucket chain, which needs `may_build`: adventure players open vaults, as in vanilla.
  - A key spends one item (`finish_item_use`); vanilla spends `key_item.count`, which is 1 in every 26.3 vault and placed vault.
  - The players a vault sees are every non-spectator in its level (creative too, by block position, as `INCLUDING_CREATIVE_PLAYERS`). A key use between ticks updates the connected players from the last normal tick's positions.
  - The display roll seeds a fresh roll from the level random (`roll_seeded`); like vanilla it never draws the reward table's own random sequence, so the display does not change the rewards.
  - Rewards and the display use the host's enchantment book (enchanted bows, crossbows, axes, armour and books); the player's luck is 0, as the fork has no luck attribute.
  - `last_insert_fail_timestamp` is kept per vault position on the server and not saved, as vanilla.
  - `total_ejections_needed` is read back from the save. Vanilla's `VaultServerData.set` forgets it when the chunk loads, so after a reload in the middle of an ejection vanilla's remaining ejections play at `0.8 + 0.4 × items left`; the fork keeps rising to 1.2. Not vanilla, logged.
  - The key and vault sounds play from the server in category blocks for the host and friends; a key's fail or insert sound goes out with the next server tick.
  - The vault screenshots are numbered 315 to 318 (312 to 314 went to group trial-spawners-run).
- **Test steps (world `trial10`, survival, Normal, by day, open ground; the turning item, the flames and the particles come with the next commit):**
  1. Place a vault from the kit and step 6 blocks back: it stays dark. Walk to 3 blocks of it: it lights up (a brighter block). Step 5 blocks away: it goes dark again; come back to 3 blocks (**315_vault_active**).
  2. Right-click the lit vault with an ominous trial key: a dull failing sound, the key stays. Hold right-click: the sound repeats at most about once every three quarters of a second.
  3. Right-click it with a trial key: a click and one key is gone, about a second later the shutter opens with a sound, then items pop out on top one by one, one a second, each a little higher in pitch (**316_vault_open**).
  4. After the last item the shutter closes with a sound and the vault goes dark. Right-click it with a second key: nothing happens and the key stays. Step 6 blocks away and come back: it stays dark for you.
  5. `/locate structure minecraft:trial_chambers`, go there and find its vaults (in the corridors near the chamber's middle; an ominous vault has a darker frame). A plain vault opens with a trial key. An ominous one refuses the trial key with the failing sound, takes an ominous trial key and gives other rewards (often a block of emerald, iron or diamond, sometimes a heavy core or an enchanted golden apple) (**317_ominous_vault**).
  6. `/gamemode creative`, place the kit's second vault somewhere else and open it with a trial key: it opens and the key stays in your hand. `/gamemode survival`.
  7. Save and quit, load the world: the first vault stays dark for you.
  8. Friend window, both standing next to the first vault: it lights up because the friend has not opened it. The host right-clicks it with a trial key: a refusing sound, the key stays. The friend opens it with their own trial key and gets their rewards; then it goes dark for both (**318_friend_vault**, both windows).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:trial_key*6,minecraft:ominous_trial_key*3,minecraft:vault*2,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:cobblestone*64,minecraft:ladder*32
```

### 2. Minecraft vaults show their reward item turning

After 09a91421.

- **Vanilla (javap, 26.3); where it differs from the brief, the javap reading is used:**
  - `VaultBlockEntity.getUpdateTag` sends the client `shared_data` only. `VaultBlockEntity$Client.tick` runs every client tick for every loaded vault: `VaultClientData.updateDisplayItemSpin` (previous = current, current = `wrapDegrees(current + 10)`); when `gameTime % 20 == 0` the connection particles; then the idle particles: `nextFloat() <= 0.5` puts a `smoke` at a point inside the cage (`Mth.nextDouble` 0.1-0.9, 0.25-0.75, 0.1-0.9 from the lower corner) and, while it shows a display item, a `small_flame` (`soul_fire_flame` when ominous) at the same point (the brief said one time in two each 20 ticks); then, while it shows an item, `nextFloat() <= 0.02` plays `block.vault.ambient` at the block centre (blocks), volume `nextFloat × 0.25 + 0.75`, pitch `nextFloat + 0.5`.
  - `emitConnectionParticlesForNearbyPlayers`: for each connected UUID the client knows as a player whose block position is within `connected_particles_range` (`distSqr <= range²`), `Mth.nextInt(2, 5)` `vault_connection` particles at the keyhole (bottom centre + (step x × 0.5, 1.75, step z × 0.5) of the block's `facing`) with motion `keyhole.vectorTo(player position + half its height)` `offsetRandom(1)`.
  - `FlyTowardsPositionParticle$VaultConnectionProvider`: starts at keyhole + motion; each tick, with `t = age / lifetime`, the position is `keyhole + motion × (1 − t)` lowered by `1.2 × t⁴`, so it flies from the player into the vault; lifetime `nextFloat × 10 + 30`; size `0.1 × (nextFloat × 0.5 + 0.2)` scaled 1.5; colour `(0.9 f, 0.9 f, f)` with `f = nextFloat × 0.6 + 0.4`; glowing (block light 15); `LifetimeAlpha(0, 0.6, 0.25, 1)`: invisible for its first quarter, then fading in to 0.6, translucent; no physics; sprite `particle/vault_connection`.
  - `LevelEventHandler`: 3015 `emitActivationParticles` (the connection particles, then 20 times a `smoke` and a flame, `small_flame` or `soul_fire_flame` for data ≠ 0, at one point inside the cage, no motion) and `block.vault.activate` (volume 1, pitch `(r1 − r2) × 0.2 + 1`); 3016 `emitDeactivationParticles` (20 flames at the cage centre, 0.4-0.6 on each axis, motion `nextGaussian × 0.02`) and `block.vault.deactivate`; 3017 `TrialSpawner.addEjectItemParticles` (the 3014 particles without its sound; the server plays the vault's own eject sound).
  - `VaultRenderer`, while the vault shows a display item: translate (0.5, 0.4, 0.5), rotate about Y by `rotLerp(partial, previous, current)`, then `ItemEntityRenderer.renderMultipleFromCount` (1 copy for one item, 2 up to 16, 3 up to 32, 4 up to 48, else 5, with the item's ground transform) at the vault's light.
- **Changed:**
  - New `minecraft_terrain/src/vault_display.rs` (rustfmt-clean): `VaultDisplays`, loaded with chunks (entities and pending tags) and following block entity changes like the spawner displays; `tick` (the spin, the 20-tick connection particles, the idle smoke and flames, the hum), `activation` (event 3015's connection particles), `items` (the turned reward items within 64 blocks, with their tint key, count and glint); `inside_cage`, `center_of_cage`, `keyhole`, `flame`, `rendered_amount`, `cluster_offsets`.
  - `ambient_particles.rs`: `Kind::VaultConnection` as above, its sprite in the atlas list. `level_events.rs`: the arms for 3015, 3016 and 3017. `mesh.rs`: `ItemVisuals::append_vault_items` (the item's ground transform under the turn, one model per copy, the glint). `vault.rs`: the activate and deactivate sound names.
  - `minecraft_survival/src/entity_view.rs` (host and friends): `vaults` beside the spawner displays; `tick_scene` ticks them each client tick with the game time and the players' bodies by UUID (the rope holders: every player's feet and height, keyed by the UUID the server uses); `level_event` adds 3015's connection particles; the item mesh draws the reward items.
  - `entities.rs`: the host's view loads and unloads vault displays with its chunks and follows the server's block entity changes, and vault tags go to friends with the spawner tags (`ToClient::BlockEntity`). `remote.rs`: a friend's view follows those tags, its chunk loads and forgets, and drops a vault display when a block update replaces the vault.
- **Tests:**
  - `vault_display::tests::the_reward_item_turns_ten_degrees_a_tick_and_hides_without_one` (at (0.5, 0.4, 0.5) in the block, 25 degrees at half a tick after three ticks, wrapping at 180, nothing beyond 64 blocks or without a display item).
  - `idle_flames_burn_only_while_an_item_shows_and_turn_soul_blue_when_ominous` (smoke on about half the ticks inside the cage, a flame with each smoke only while an item shows, soul flames when ominous, nothing without the vault block), `a_lit_vault_hums_about_two_ticks_in_a_hundred`, `connection_particles_fly_from_the_keyhole_to_its_players_once_a_second` (only on the 20-tick phase, 2 to 5 for the player in range and none for one 5 blocks away or unknown, the keyhole of a south-facing vault, the activation event too), `vaults_load_with_chunks_and_follow_their_block_entity`, `a_cluster_draws_more_copies_for_bigger_stacks`, `the_reward_item_is_drawn_in_the_cage_turned_with_its_copies_and_glint` (a test pack's item: within a block of the cage centre, a quarter turn swaps its x and z extent, 3 copies for 20 items, the glint range).
  - `ambient_particles::tests::vault_connection_particles_fly_from_their_player_into_the_vault_and_fade_in` (starts at the player, lifetime 30 to 39, translucent and invisible at first, its size and colour, ends 1.2 below the keyhole at alpha 0.6, its sprite in the atlas list).
  - `level_events::tests::vault_events_play_vanillas_sounds_and_particles` (3015: the activate sound and 20 smoke and small flame pairs inside the cage, soul flames for data 1; 3016: the deactivate sound and 20 flames within 0.1 of the centre with small motion; 3017: no sound, 20 small flames and 20 smokes).
  - `minecraft_survival` `remote::tests::a_vault_streamed_to_the_friend_shows_its_reward_item_until_the_block_or_chunk_goes` (the item from the chunk, kept when the vault lights up, turned by a forwarded tag, hidden when dim, dropped when mined, back with a forwarded tag, gone with its chunk).
- **Decisions:**
  - The players' bodies for the connection particles are the view's rope holders (host: every player, friends included; a friend: every player in its snapshot), keyed by the server's player UUID. A connected UUID the view has no body for gets no particles, as vanilla's `getPlayerByUUID` returning null.
  - Event 3015's connection particles use the connected players the view last got; vanilla's level event also arrives before the block entity data of the same tick.
  - The cluster's extra copies use the fork's dropped-item offsets (seeded, at most 0.15 on each axis, none in depth for flat items) instead of vanilla's stacking of flat items in depth.
  - Reward items are drawn within 64 blocks, the block entity view distance.
- **Test steps (world `trial10`, survival, Normal, the kit of commit 1):**
  1. Place a vault and walk to 3 blocks of it: it lights up with a puff of smoke and small orange flames inside and a rising chime. An item from its reward table turns inside, a full turn about every 1.8 seconds, changing to another item every second; small flames and smoke drift inside it and now and then it hums (**319_vault_item**).
  2. Stand still next to it: about once a second a few faint glowing sparks fly from you into the top of the vault and sink into it (**320_vault_connection**).
  3. Step 5 blocks away: it goes dark with a puff of small flames from its middle and a falling sound, and the item is gone.
  4. Come back and open it with a trial key: while it ejects, the item inside shows the next reward to come out, and each one pops out with a little burst of flames and smoke on top.
  5. A trial chamber's ominous vault burns with blue (soul) flames instead of orange ones.
  6. Friend window, both next to a vault neither has opened: the friend sees the same turning item, flames and sparks (sparks fly from each player), and hears the hum (**321_friend_vault_item**, both windows).
- **Kit (the same line as commit 1):**

```
IW4L_MINECRAFT_KIT=minecraft:trial_key*6,minecraft:ominous_trial_key*3,minecraft:vault*2,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:cobblestone*64,minecraft:ladder*32
```

## Group lodestones

Brief D15: compasses lock onto lodestones, point at them, and friends' compasses and clocks turn.

### 1. Minecraft compasses lock onto lodestones like vanilla

After 73faeb44.

- **Vanilla (javap, 26.3), checked again this run:**
  - `CompassItem.useOn` on `Blocks.LODESTONE`: `playSound(null, pos, item.lodestone_compass.lock, PLAYERS, 1, 1)` (everyone near, at the block centre); the tracker `LodestoneTracker(Optional.of(GlobalPos(level dimension, pos)), true)`; a stack of exactly one outside creative (`!hasInfiniteMaterials && count == 1`) is changed in place; otherwise `transmuteCopy(COMPASS, 1)` (the other components kept) gets the tracker, the held stack is consumed by one (`consume`, which creative skips) and the copy goes to `Inventory.add` or is dropped (`Player.drop`). It returns `SUCCESS` (the arm swings); any other block falls through to `Item.useOn`. Only `minecraft:compass` is a `CompassItem` (the recovery compass is not).
  - `ItemStack.useOn` passes for a player that may not build (adventure mode without `can_place_on`), so adventure players cannot lock.
  - `LodestoneTracker.CODEC`: `target` is `GlobalPos.CODEC.optionalFieldOf("target")` (`{dimension, pos}`, pos a `BlockPos` int stream: an int array or a list), `tracked` is `Codec.BOOL.optionalFieldOf("tracked", true)`, so a tracked compass saves no `tracked` field and a lost lodestone leaves `{}`.
  - `CompassItem.getName`: `item.minecraft.lodestone_compass` whenever the component is present (a custom name still comes first, `ItemStack.getHoverName`); `isFoil` too (the glint already worked, `glint.rs`).
  - `CompassItem.inventoryTick` → `LodestoneTracker.tick(level)`: only when `tracked` and a target exists and its dimension is the level's: when `isInWorldBounds(pos)` fails (build height, or x or z outside ±30 000 000) or the POI manager has no lodestone POI there, the tracker becomes `(empty, true)`. Only the inventory's slots tick (not the carried stack or a crafting grid), and players tick under `/tick freeze`.
- **Changed:**
  - New `minecraft_terrain/src/lodestone.rs` (rustfmt-clean): `Target`, `Tracker`, `tracker`, `set_tracker` (written as vanilla's codec writes it), `with_tracker` (the transmuted copy), `lock_use` (a compass aimed at a lodestone within 4.5 blocks, the lock sound), `lock` (in place, or the copy handed back to drop), `tracked_targets` (a player's tracked targets in one dimension), `forget` (target removed, `tracked` kept), `in_world_bounds`, `name_key`.
  - `bucket.rs`: `Change::Lodestone(Pos)`. `actions.rs`: the chain link after the eye-of-ender frame use and its match arm (lock, toss what did not fit, the sound through `finish_item_use`, so every player near hears it: action effects go out as `Said::all`).
  - `server.rs`: `TickInput.lodestones` (each player's tracked targets in this level, dropped in `remove_player`), drained at the first `run_tick` of an input, outside the `if normal` block; `ServerSim::lodestones_gone` reports a target outside the world bounds, or one whose chunk is loaded and whose block is not a lodestone, in `Output.lodestones_gone`.
  - `minecraft_survival/src/entities.rs`: `Entities` keeps its level's dimension; its tick sends every actor's tracked targets in that dimension, routes the answers to that player's queue and calls `forget` on that player's inventory (host and friends alike, by player).
  - `render_anim/src/minecraft_inventory.rs`: `display_name` gives "Lodestone Compass" from the language file for a compass with the component, after a custom name and a book title.
- **Tests:**
  - `lodestone::tests`: `a_compass_on_a_lodestone_locks_with_its_sound_and_on_stone_does_nothing` (also not a clock or a recovery compass), `a_single_compass_tracks_in_place` (the saved component, glint and name), `a_stack_of_three_leaves_two_plain_and_adds_one_tracked`, `a_full_inventory_hands_the_tracked_compass_back_to_drop`, `creative_keeps_every_compass_and_adds_a_tracked_one` (a single one too), `relinking_moves_the_target_and_keeps_other_components` (a custom name stays; a stack of two tracked ones splits; a stone is left alone), `forget_clears_the_target_but_keeps_tracking_the_name_and_the_glint` (another dimension's target is kept), `only_tracked_targets_in_the_players_dimension_are_checked` (offhand included, `tracked: false` skipped, duplicates once), `a_tracker_reads_vanillas_forms` (`tracked` as a byte, no target, a short pos), `a_lodestone_outside_the_build_height_or_the_border_is_out_of_bounds`.
  - `server::tests::a_broken_lodestone_is_reported_only_when_its_chunk_is_loaded` (gated: a placed lodestone and one in an unloaded chunk are kept, targets above and below the build height are reported; once broken it is reported, once per input over a 3-tick frozen batch, per player; a player removed from the input is not checked).
  - `minecraft_survival` `entities::tests::each_players_compass_forgets_a_lodestone_that_is_not_there_in_a_loaded_chunk` (gated: the host's and the friend's compasses aimed at stone lose their target; the host's unloaded-chunk and Nether targets stay; `tracked` stays).
  - `player_nbt::tests::a_lodestone_compass_survives_save_and_load_with_its_pos_as_ints_or_a_list` (a saved player file round trip; vanilla's int array and a list load to the same stack; `{}` round trips).
  - `render_anim` `minecraft_inventory::tests::a_lodestone_compass_is_named_and_glints_and_a_custom_name_comes_first`.
- **Decisions:**
  - Fork decision (brief): a target in a chunk the level has not loaded is kept. Vanilla's POI manager can read unloaded POI sections; the fork keeps no lodestone POIs, so the host checks the block in its server level instead.
  - Locking goes through the bucket chain, like the other item uses on a block, so it needs `may_build` (adventure mode cannot lock, as vanilla) and the 4.5-block item reach of the other item-on-block uses.
  - The tracker is written like vanilla's codec (no `tracked` while true, `{}` once the lodestone is gone), so a compass locked here and the same compass loaded from a save are the same stack. Positions are saved as a list of ints, which vanilla reads like its int array.
  - The check runs for every tick input, drained at its first tick; a batch that runs one input several times reports once, and a newer input brings its own list, so nothing is carried. It also runs under `/tick freeze`, as vanilla players tick there.
  - Only the inventory's slots are checked, not the carried stack or the crafting grid, as vanilla's `Inventory.tick`.
- **Test steps (world `trial10`, survival, Normal, Overworld, by day; the needle points from commit 2 on):**
  1. Place a lodestone and right-click it holding the 8 compasses: a metallic click, one glinting compass named "Lodestone Compass" appears in the inventory and 7 plain ones stay in your hand (**322_lodestone_lock**).
  2. Put the 7 plain compasses away, hold the single lodestone compass and right-click a second lodestone: the click again, and no new compass appears (it changed in place).
  3. `/gamemode creative`, hold a plain compass and right-click a lodestone: you keep the compass and get one more lodestone compass. `/gamemode survival`.
  4. Fill every inventory slot with cobblestone except the one holding 2 plain compasses, right-click a lodestone: the lodestone compass drops on the ground in front of you.
  5. Hold a lodestone compass and break its lodestone with the pickaxe: it keeps its name and glint.
  6. Save and quit, load the world: the lodestone compasses are still there with their names.
  7. Friend window: the friend right-clicks a lodestone with their own compass: both players hear the click and the friend gets the lodestone compass (**323_friend_lodestone_lock**, both windows).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:lodestone*4,minecraft:compass*8,minecraft:clock,minecraft:recovery_compass,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

## Protocol

`minecraft_net::PROTOCOL` stays 6 (the release step bumps it).

- Group trial-ominous, commit 2: `EntitySnapshot` (sent to friends in `StateFrame.entities`) gains `ominous_items` (`trial_spawner::OminousItemView`: id, position, item, tint, age) after `items`. A friend on an older build cannot decode the new snapshot, so this needs the protocol bump at the release.

- Group trial-spawners-run, commit 3: `ToClient::Time.game_time` keeps its type but now carries the level's game time (the world's `Time`) instead of the day clock. The friend only uses it as its entity world's clock, so no bump is needed.

## State doc changes

- Release 1's known limits (`docs/round10/release-1-tests.md`, Wind charges) and the round 9 log (group breezes, commit 1, and "Left for later"): drop "A player flying in creative is still pushed by a burst". A player flying in creative was never pushed by a burst: every push goes through the flying check (group d5-review, commit 1).
- `docs/MINECRAFT-SURVIVAL.md` and `docs/round8/decisions.md`'s deferred trial spawners: trial spawners now show their mob turning in the cage with vanilla's flames and hum (soul flames when ominous), wake up from inactive with a mob picked from their config, and take spawn eggs like vanilla. Breeze and silverfish trial spawners stay dark until those mobs exist. The trial itself is still to come (group trial-spawners-run).
- `docs/MINECRAFT-SURVIVAL.md` (trial spawners, group trial-spawners-run): trial spawners run their trial like vanilla: they detect survival players in sight within 14 blocks (through glass and bars), spawn their mobs in turn (6, 2 at once, plus 2 and 1 per extra player; the chamber configs' numbers), forget mobs that die or go farther than 47 blocks, eject one reward per player from the top, and cool down for 30 minutes. Mob spawners and trial spawners spawn husks, strays and slimes without open sky or slime chunks, and trial spawners ignore light. Each level's game time now starts from `level.dat`'s `Time` (one clock for the server, the save and both displays). With the detection, spawn and ejection particles and sounds of vanilla (commit 4). Not yet: ominous trials (group trial-ominous).
- `docs/MINECRAFT-SURVIVAL.md` (trial spawners, group trial-ominous, commit 1): a trial spawner that sees a player with Bad Omen or Trial Omen turns ominous like vanilla: Bad Omen becomes Trial Omen (15 minutes a level), the spawner burns with soul flames, the mobs it had vanish and the trial starts over with the ominous config, whose mobs wear its swords, bows and armour (never dropped) and whose rewards are the ominous keys and consumables; it turns plain at the end of its cooldown. Not vanilla: a mob the spawner discards does not drop items it picked up.
- `docs/MINECRAFT-SURVIVAL.md` (trial spawners, group trial-ominous, commit 2): an ominous trial drops an item from above every 8 seconds or more, over a player or one of its mobs: a floating, spinning item with blue sparks that drops after 3 to 6 seconds as a lingering potion, an arrow (poison or slowness), a wind charge or a fire charge, like vanilla, saved with the chunk. Not vanilla: arrows from above slow or poison at level I and cannot be picked up; an item from above that hits a mob directly does not hurt it (its burst or cloud still acts).
- `docs/MINECRAFT-SURVIVAL.md` and CLAUDE.md's "Finish survival" (vaults, group vaults, commit 1): vaults work like vanilla: they light up for any player within 4 blocks they have not rewarded (creative too), take a trial key (an ominous trial key for an ominous vault) once per player UUID, eject their reward table's items one a second with rising pitch, and stay dark for players they rewarded (up to 128 kept); a wrong key or a rewarded player gets the failing sound at most once in 15 ticks; creative keeps the key. Not vanilla: the ejection pitch after a reload in mid-ejection.
- `docs/MINECRAFT-SURVIVAL.md` (vaults, group vaults, commit 2): a lit vault shows the item it will give turning inside it, with vanilla's smoke and small flames (soul flames when ominous), its hum, the sparks that fly from each connected player into it, and the activation, deactivation and ejection effects, for the host and friends.
- `docs/MINECRAFT-SURVIVAL.md` and CLAUDE.md's "Finish survival" (lodestones, group lodestones, commit 1): a compass right-clicked on a lodestone becomes a glinting "Lodestone Compass" like vanilla (a single compass in place, otherwise one more from the stack; creative keeps its compasses; a full inventory drops it), with the lock sound for everyone near, saved in the vanilla format. When its lodestone is gone (in a loaded chunk) it keeps its name and glint and loses its target. Lodestones leave CLAUDE.md's "Finish survival" list with commit 2.
- Release 1's known limits (Withers) and the round 9 log (group wither-fight, commit 1): drop "A skull loaded from a save has no owner and hurts as magic for 5". `docs/MINECRAFT-SURVIVAL.md`: fireballs, small fireballs and wither skulls keep their shooter through a save (`Owner`), found again when the shooter loads (group d5-review, commit 2).

## Vendored files

Patched in round 10 (none added): `entities/src/fireball.rs` and `entities/src/world/fireballs.rs` (group d5-review, commit 2), and `world/src/level/container.rs` (group trial-spawners, commit 2: `Level::set_block_entity_tag`). All three were already patched by the fork (the first two are in round 9's list, `docs/round9/track-d-log.md`, Vendored files; `container.rs` since c5efeed0), so the fork's counts do not change. Group trial-spawners-run, commit 1: `world/src/natural_spawner/spawning.rs` and `world/src/level/spawning.rs` (both patched by the fork before). Commit 2: `world/src/level/physics.rs`, patched by the fork for the first time (one more patched file). Commit 3: `world/src/level/spawning.rs` again and `world/src/level/dispenser.rs` (patched by the fork before, c987075b). Group trial-ominous, commit 1: `world/src/level/spawning.rs` again (`summon_trial_mob` no longer adds the mob itself). Commit 2: `world/src/level/entity.rs`, `world/src/level/mod.rs` and `entities/src/world.rs`, all patched by the fork before. Group vaults patches and adds no vendored file. Group lodestones, commit 1: none.
