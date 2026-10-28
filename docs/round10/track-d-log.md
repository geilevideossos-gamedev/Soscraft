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

## Protocol

No message of this track has changed yet. `minecraft_net::PROTOCOL` stays 6.

## State doc changes

- Release 1's known limits (`docs/round10/release-1-tests.md`, Wind charges) and the round 9 log (group breezes, commit 1, and "Left for later"): drop "A player flying in creative is still pushed by a burst". A player flying in creative was never pushed by a burst: every push goes through the flying check (group d5-review, commit 1).
- `docs/MINECRAFT-SURVIVAL.md` and `docs/round8/decisions.md`'s deferred trial spawners: trial spawners now show their mob turning in the cage with vanilla's flames and hum (soul flames when ominous), wake up from inactive with a mob picked from their config, and take spawn eggs like vanilla. Breeze and silverfish trial spawners stay dark until those mobs exist. The trial itself is still to come (group trial-spawners-run).
- Release 1's known limits (Withers) and the round 9 log (group wither-fight, commit 1): drop "A skull loaded from a save has no owner and hurts as magic for 5". `docs/MINECRAFT-SURVIVAL.md`: fireballs, small fireballs and wither skulls keep their shooter through a save (`Owner`), found again when the shooter loads (group d5-review, commit 2).

## Vendored files

Patched in round 10 (none added): `entities/src/fireball.rs` and `entities/src/world/fireballs.rs` (group d5-review, commit 2), and `world/src/level/container.rs` (group trial-spawners, commit 2: `Level::set_block_entity_tag`). All three were already patched by the fork (the first two are in round 9's list, `docs/round9/track-d-log.md`, Vendored files; `container.rs` since c5efeed0), so the fork's counts do not change. Group trial-spawners-run, commit 1: `world/src/natural_spawner/spawning.rs` and `world/src/level/spawning.rs` (both patched by the fork before). Commit 2: `world/src/level/physics.rs`, patched by the fork for the first time (one more patched file).
