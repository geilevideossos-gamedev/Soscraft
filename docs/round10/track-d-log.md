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

### 2. Minecraft lodestone compasses point at their lodestone

After 62c9ba69.

- **Vanilla (26.3 data and javap):**
  - `assets/minecraft/items/compass.json`: `condition` on `minecraft:has_component` `minecraft:lodestone_tracker`; `on_true` is `range_dispatch` `minecraft:compass` target `lodestone`, scale 32, 33 entries (compass_16 at 0, then 17..31, 00..16 at half steps); `on_false` the same table with target `spawn`.
  - `CompassAngle` / `CompassAngleState`: the lodestone target is the stack's `LodestoneTracker.target`, whatever `tracked` says; it spins (`noTargetWobble`) with no target, a target in another dimension or one within `1e-5` squared of the holder; otherwise the angle to the target's block centre plus the wobble, which follows only the holder's yaw. Each property in the model has its own wobble, so every lodestone compass shares one needle and the spawn compass has another.
- **Changed:**
  - `dials.rs`: `stack_key(stack)` gives a tracked compass a key of its own, `minecraft:compass@lodestone:<dimension>:<x>,<y>,<z>` (or `...@lodestone:none` without a target); `item_of` and `split` give back the plain id for such a key, so every place that splits an icon key (icons, dropped items, the vault's reward item) keeps working. `Dials::watch(keys)` takes the icon keys published in the last frame; `step` turns each watched lodestone key once a tick from its target, and `watch` drops the keys no longer shown. `leaf` evaluates `minecraft:condition` with `minecraft:has_component` `minecraft:lodestone_tracker` for the stack being keyed, and `compass` takes the `lodestone` target. `key()` of a lodestone key not turned yet shows the plain compass's frame for that frame (so its icon is drawn as an animated dial from the start).
  - `item_tint::of_stack` uses `stack_key`, so `McStack.icon`, the hotbar's animated icon refresh and dropped items key a tracked compass by its target.
  - `render_anim`: `publish` calls `dials.watch` with every icon key of the frame; `icon_source` names an icon by its plain item id (so `ui.names` stays keyed by the item); the first-person hand keys its stack with `dials.key(&stack_key(stack))`.
- **Tests:**
  - `dials::tests::a_lodestone_compass_points_at_its_lodestone_and_spins_without_one_or_elsewhere` (facing south with the lodestone north: the behind frame while a plain compass shows the spawn ahead; facing it: the straight-ahead frame; spins with a Nether target, without a target and standing in its centre).
  - `dials::tests::a_lodestone_compass_has_its_own_key_turned_each_tick_while_it_is_shown` (the keys and their parse, an unknown dimension spins, only lodestone keys are watched, the key resolves to a `compass_NN` frame other than the plain compass's, an unstepped key shows the plain frame, an unshown key is dropped).
  - `dials::tests::vanilla_compasses_and_clocks_get_their_frame_as_icon_and_held_model` (gated, with the real `compass.json`): a watched lodestone compass ahead shows `compass_16` while the spawn compass behind shows `compass_00`.
  - `item_tint::tests::a_lodestone_compass_keys_its_target_and_still_splits_to_the_compass`; `render_anim` `minecraft_inventory::tests::a_lodestone_compass_icon_is_keyed_by_its_target_but_named_by_its_item`.
- **Decisions:**
  - All lodestone compasses share one needle (`compass:lodestone`), as vanilla's one `CompassAngleState` per model property; the spawn compass keeps its own.
  - A lodestone key is turned from the frame after it first shows; in its first frame it shows the plain compass's needle.
  - Held items in the world (other players' hands, `body_render.rs`), item frames and dropped items still draw a compass with its default model, as they do for the plain compass; only icons and the first-person hand turn. The brief's `mesh.rs:1006` is the dropped items' mesh, which only needed to split the new key.
  - Each target shown gets its own animated icon cell, kept for the session like every other icon.
  - Merge note for track E1 (`of_stack_at`): `of_stack` becomes `of_stack_at(&crate::dials::stack_key(stack), stack)`, and the hand becomes `of_stack_at(dials.key(&stack_key(stack)), stack)`.
- **Test steps (world `trial10`, survival, Normal, Overworld, by day; the kit of commit 1):**
  1. Place a lodestone, lock a compass to it, walk 30 blocks away and turn around: the lodestone compass's needle points back at the lodestone in the hotbar and in your hand, while a plain compass in the next slot points to the world spawn (**324_lodestone_compass**).
  2. Lock the single lodestone compass to a second lodestone 20 blocks to the side: its needle swings round to the new one.
  3. Build a portal, go to the Nether, lock a plain compass to a lodestone there and walk away: it points at it. The Overworld lodestone compass spins at random (**325_nether_lodestone**).
  4. Back in the Overworld, hold the first lodestone compass and break its lodestone: the needle starts spinning; the name and glint stay.
  5. Save and quit, load the world: the remaining lodestone compasses still point at their lodestones.
- **Kit (the same line as commit 1):**

```
IW4L_MINECRAFT_KIT=minecraft:lodestone*4,minecraft:compass*8,minecraft:clock,minecraft:recovery_compass,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

### 3. Friends' compasses and clocks turn like the host's

After 64549426.

- **Vanilla (javap, 26.3):** the client's compass, recovery compass and clock read the local level: the world spawn from `ClientboundSetDefaultSpawnPositionPacket` (`LevelData.RespawnData`, sent when the player joins and whenever the world spawn changes), the last death from `CommonPlayerSpawnInfo.lastDeathLocation` (in the login and respawn packets), and the time from the client level. Every client turns its own needles.
- **Changed:**
  - `minecraft_net`: `ToClient::SpawnPosition { dimension, pos }` and `PlayerView.last_death: Option<(Dimension, [i32; 3])>`, both appended after the existing variants and fields (the codec's variant index is its place in the enum), so earlier indices stay as they are.
  - `minecraft_survival/src/session.rs`: each session remembers the world spawn it was sent; `Sessions::send_spawns` (called by the host right after `send_views`) sends it after the login and again whenever `/setworldspawn` or anything else moves it. `views.rs`: the player view carries the player's last death.
  - `minecraft_survival/src/remote.rs`: the friend keeps the world spawn it got; `RemoteWorld::dial_context(feet, yaw)` builds the dials' context from the friend's level (dimension, sun angle from its own sky clock), the world spawn and its own last death.
  - `render_anim/src/minecraft_remote.rs`: the friend steps its dials with that context, its own feet and yaw before publishing its inventory, so its compasses (the plain, recovery and lodestone ones) and clocks turn in the icons and the first-person hand, which reads the same dials.
- **Tests:**
  - `minecraft_net` `codec::tests::the_world_spawn_and_the_last_death_reach_the_friend_for_its_compasses` (index 35, round trips with and without a last death), and the new message in `every_message_to_the_client_round_trips`; `player_view()` carries a last death.
  - `minecraft_survival` `session::tests::each_friend_gets_the_world_spawn_after_its_login_and_again_when_it_moves` (gated: once per friend after its login, nothing when unchanged, a moved spawn reaches every friend).
  - `minecraft_survival` `remote::tests::the_friends_dials_know_the_world_spawn_and_its_own_last_death` (gated: no context without a level, no spawn until it is sent, the friend's dimension, feet and yaw, its last death, a new spawn).
- **Decisions:**
  - The world spawn is compared every host frame and sent only when it differs from the last one sent to that friend, which covers the join and `/setworldspawn` in one place.
  - Before the friend has a world spawn its compass spins (as a compass with no target).
  - The friend's publish path needs the whole friend setup (assets, the stream, a player state), so it has no `render_anim` test of its own: the context test and the dials tests cover both halves.
- **Protocol:** both changes change what friends decode (see "Protocol" below).
- **Test steps (both windows, survival, Overworld, by day, near the world spawn; the kit of commit 1 for both players):**
  1. Friend window: hold a plain compass and walk around the spawn: its needle points at the world spawn (it did not move before); hold the clock: it shows the sun and turns with the day (**326_friend_compass**, friend window).
  2. Host: `/setworldspawn ~ ~ ~` about 50 blocks away from the friend: the friend's plain compass swings round to point there.
  3. Friend: lock a compass to a lodestone and walk away: it points at the lodestone, in the hotbar and in the hand (**327_friend_lodestone_compass**, friend window).
  4. Host: `/kill <friend's name>`; the friend respawns and holds the recovery compass: it points to where they died. In the Nether the friend's clock and plain compass spin.
- **Kit (the same line as commit 1):**

```
IW4L_MINECRAFT_KIT=minecraft:lodestone*4,minecraft:compass*8,minecraft:clock,minecraft:recovery_compass,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

### 4. Minecraft lodestone checks keep their positions as block positions

After 9ff87228.

- **Changed:** `TickInput.lodestones` is `Vec<(u64, Vec<BlockPos>)>` (the same type through the crate's `BlockPos` alias), which clears the one clippy warning (`type_complexity`) the group added. Clippy on `minecraft_terrain`, `minecraft_survival` and `minecraft_net` finds nothing else new in the group's code (`minecraft_terrain` needs `-A clippy::approx_constant` for older files).
- **Tests:** none new; the lodestone tests and `check -p launcher` pass.
- **Test steps and kit:** none (no change in game).

## Group review

The adversarial review of the whole track (base `r9-d5-base`, so round 9's d5 commits too). Each entry is one defect the review found and its fix.

### 1. Minecraft arrows from an ominous trial hurt the same on every difficulty

After aba7374c.

- **Defect:** an arrow an ominous item spawner drops (group trial-ominous, commit 2, `ServerSim::drop_ominous_items`) is an entity-world arrow with owner id 0. The entity world reported every arrow that hits a player with `source: Some(owner_id)`, so this arrow came with `Some(0)`, and `minecraft_survival` `mob_hit` scaled its damage with the difficulty (`scales` is true for any source): on Hard an arrow from above did 4.5 instead of 3, on Easy 2.5.
- **Vanilla (javap, 26.3):** `DamageSource.scalesWithDifficulty` for the `arrow` damage type (`when_caused_by_living_non_player`) is true only when the causing entity is a `LivingEntity` that is not a player. The arrow from above is owned by the `OminousItemSpawner` (`spawnProjectile`), which is no living entity and is gone the same tick, so `AbstractArrow.onHitEntity` falls back to `arrow(this, this)`: no scaling, and the death line is "%1$s was shot by Arrow".
- **Changed:** vendored `entities/src/world.rs`: an arrow with owner 0 (no mob has id 0) hits a player with `source: None`. `mob_hit` then names the fork's ownerless arrow (`Mob { id: 0, type_id: "minecraft:arrow" }`, "shot by Arrow", as before) and leaves the damage as it is.
- **Tests:**
  - vendored `world::tests::an_arrow_without_an_owner_hits_a_player_with_no_source` (an arrow with owner 0 and its poison hits with no source; a skeleton's arrow still names the skeleton).
  - `minecraft_survival` `entities::tests::an_arrow_dropped_by_an_ominous_trial_hurts_the_same_on_every_difficulty` (3 on every difficulty, told as an arrow, its poison kept; a skeleton's arrow still does 4.5 on Hard).
- **Decisions:** none beyond vanilla.
- **Test steps (world `trial10`, survival, no armour, in the chamber; a fresh zombie spawner):**
  1. `/difficulty hard`, drink a plain ominous bottle and walk into sight of the spawner: it turns blue. Stand still and wait for an arrow to fall on you from above (the floating item is an arrow): it takes about 1.5 hearts, as on Normal, not 2 and a quarter (**328_ominous_arrow_hard**). If it kills you, the chat says "<name> was shot by Arrow". `/difficulty normal`.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:milk_bucket*2,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:golden_apple*16,minecraft:totem_of_undying,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:cobblestone*64,minecraft:ladder*32,minecraft:water_bucket
```

### 2. Ominous Minecraft trial spawners play the fire and wind charge sounds when they drop them

After 50782150.

- **Defect:** a fire charge or a wind charge that an ominous item spawner drops (group trial-ominous, commit 2) came down silently.
- **Vanilla (javap, 26.3):** `OminousItemSpawner.spawnProjectile` sends the item's `DispenseConfig.overrideDispenseEvent` as a level event at its block (data 0) before it shoots: `FireChargeItem` has 1018, `WindChargeItem` 1051; no other item of the table has one. `LevelEventHandler`: 1018 plays `entity.blaze.shoot` (hostile) at volume 2 and pitch `(r1 − r2) × 0.2 + 1`, 1051 plays `entity.wind_charge.throw` (blocks) at volume 0.5 and pitch `0.4 / (r × 0.4 + 0.8)`, both at the block centre.
- **Changed:** `trial_spawner.rs`: `dispense_sound(item, at, draw)` with the two sounds above (`FIRE_CHARGE_SOUND`, `WIND_CHARGE_SOUND`). `server.rs` `ServerSim::drop_ominous_items` plays it from the server with the item's drop (`end_sounds`, which go out in `Output.mob_sounds` to the host and the friends near it).
- **Tests:** `server::tests::an_ominous_item_spawner_warns_survives_a_save_and_drops_its_item_down` now also checks that exactly two sounds play when the four items drop: the wind charge's throw at its block centre (volume 0.5, blocks, pitch 0.33 to 0.5) and the fire charge's blaze shot at its block centre (volume 2, hostile, pitch 0.8 to 1.2); the arrow and the diamond make none.
- **Decisions:** the sound goes out as a server sound with its category, as the fork's other block and mob sounds, instead of a level event the client turns into a sound (the fork's level events do not have 1018 and 1051).
- **Test steps (world `trial10`, survival, Normal, full kit armour, in the chamber; a fresh zombie spawner):**
  1. Drink a plain ominous bottle and walk into sight of the spawner: it turns blue. Wait under the floating items: when a fire charge drops you hear a blaze's shot, when a wind charge drops the whoosh of a thrown wind charge (**329_ominous_charge_sounds**). Potions and arrows drop with only the spawner's own puff.
  2. Friend window, standing next to the host: the friend hears the same shots.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:milk_bucket*2,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:totem_of_undying,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:cobblestone*64,minecraft:ladder*32,minecraft:water_bucket
```

## Final verification

Clippy and the tests of every crate the track touched (`git diff --stat r9-d5-base..HEAD`): `frame`, `hud`, `minecraft_net`, `minecraft_survival`, `minecraft_terrain`, `render_anim`, `minecraftoss-entities` and `minecraftoss-world`.

### 1. Track D code passes clippy

After e9ea905e.

- **Clippy** (`clippy -p` on the eight crates, `--all-targets`, `-A clippy::approx_constant`): six warnings on lines the track added, all in `minecraft_terrain`; the older warnings stay. The vendored crates have none.
  - `trial_spawner.rs`: three `unwrap_or_default` calls, which the workspace's `clippy.toml` forbids, now name their fallback (`unwrap_or_else(BTreeMap::new)` for a block entity tag that is not a compound, `unwrap_or_else(|_| Vec::new())` for a reward table that does not roll), as `vault.rs` does.
  - Tests only: `dials.rs` (`cloned_ref_to_slice_refs`: `std::slice::from_ref`), `vault_display.rs` (`single_range_in_vec_init`: `vec!` as `item_frame.rs` does) and `server.rs`'s wither test (`useless_conversion`: the count is already `i32`).
  - Clippy on `minecraft_terrain` again finds nothing on the track's lines. The track's new `.rs` files all pass `rustfmt --check`, and its added lines hold no comments.
- **Tests:** none new; the same behaviour. `check -p launcher` passes.
- **Test steps and kit:** none (no change in game).

### 2. Track D log records the final test run and the combined kit

After 07d23491. Log only.

- **Tests** on the code of 07d23491, `test -p <crate> --locked` for each touched crate, all passed: `minecraft_terrain` 1470 (3 ignored; the known flaky `sections::tests::background_full_update_matches_the_synchronous_one` failed once in the full run and passed when run again, and the doc-tests ran on their own), `minecraft_survival` 766 + 1, `render_anim` 215, `minecraftoss-entities` 399 (every `check_*` bin builds), `minecraftoss-world` 103, `minecraft_net` 25, `hud` 186, `frame` 28.
- **Owner's test steps:** none new; the steps of each commit above stand, with the combined kit below.

## Group breezes

Brief D16, group breezes, on D0's registry (`docs/round10/track-d0.md`, D0-3, the plug-in checklist). This is the track's second segment: D0 merged into `minecraft-survival` (39654c55) and `track-d` was brought up to `minecraft-survival` (674afcc0) before it started.

### 1. Minecraft breezes live in the entity world and shoot wind charges

After 674afcc0.

- **Vanilla (javap, 26.3, and the 26.3 data):**
  - `EntityTypes.BREEZE`: `MONSTER`, `sized(0.6, 1.77)`, `eyeHeight(1.3452)`, `clientTrackingRange(10)`, `notInPeaceful`, not fire immune. `BREEZE_WIND_CHARGE`: `MISC`, `noLootTable`, `sized(0.3125, 0.3125)`, eye height 0, tracking range 4, update interval 10 (the same as `WIND_CHARGE`).
  - `SpawnPlacements`: the breeze is `ON_GROUND`, `MOTION_BLOCKING_NO_LEAVES`, `Monster.checkAnyLightMonsterSpawnRules` (not peaceful, then a spawner or a valid block below). No biome spawns it.
  - `Breeze.createAttributes`: movement speed 0.63, max health 30, follow range 24, attack damage 3. The constructor sets `xpReward` 10 and the pathfinding maluses `ON_TOP_OF_TRAPDOOR` -1 and `FIRE` -1. `getMaxHeadYRot` 30, `getHeadRotSpeed` 25, `getFiringYPosition` = y + height / 2 + 0.3, `getFluidJumpThreshold` = its eye height, `getMovementEmission` `EVENTS` (no step sounds), `canAttack` only a player or an iron golem (then `Monster.canAttack`), `isInvulnerableTo` any damage whose causing entity is a breeze. `playAmbientSound` is `playLocalSound` (the client's): `entity.breeze.idle_ground` on the ground, else `idle_air`, volume 1, pitch 1. Loot `entities/breeze`: 1-2 breeze rods (looting +1-2), killed by a player. Tags: `#fall_damage_immune`, `#deflects_projectiles`, `#no_anger_from_wind_charge`, `#can_turn_in_boats`.
  - The brain (`Breeze.BRAIN_PROVIDER`, `BreezeAi`): sensors `NEAREST_LIVING_ENTITIES`, `HURT_BY`, `NEAREST_PLAYERS` and `BREEZE_ATTACK_ENTITY_SENSOR` (the nearest living entity, not creative or spectator, that `Sensor.isEntityAttackable`: within 16 blocks, in sight unless it is already the attack target). The default activity is `FIGHT`; `updateActivity` takes `FIGHT` when there is an attack target and no walk target, else `IDLE`. Core: `Swim(0.8)`, `LookAtTargetSink(45, 90)`. Idle: 0 `StartAttacking(NEAREST_ATTACKABLE)`, 1 `StartAttacking` the entity behind `HURT_BY`, 2 `SlideToTargetSink(20, 40)` (a `MoveToTargetSink`: pose `SLIDING`; when it stops, pose `STANDING` and, with a target, `BREEZE_SHOOT` for 60 ticks), 3 `RunOne` of `DoNothing(20, 100)` (weight 1) and `RandomStroll.stroll(0.6)` (weight 2). Fight: 0 `StopAttackingIfTargetInvalid` (not attackable for 100 ticks), 1 `Shoot`, 2 `LongJump`, 3 `ShootWhenStuck`, 4 `Slide`.
  - `Shoot`: needs the target, `BREEZE_SHOOT`, no cooldown, charging, recovering, walk target or jump target, and pose `STANDING`; a target 16 blocks away or more (`distanceToSqr < 256` fails) makes it forget `BREEZE_SHOOT` instead. It runs 20 ticks: on start pose `SHOOTING`, charging for 15 ticks and `entity.breeze.inhale` (1, 1). Each tick it `lookAt`s the target's feet from its eyes; once charging and recovering are gone it sets recovering for 4 ticks and shoots a `BreezeWindCharge` from (x, firing height, z) towards (target x, target y + 0.3 of its height (0.8 when the target rides), target z) with `Projectile.shoot` power 0.7 and uncertainty 5 - 4 x the difficulty id, then `entity.breeze.shoot` (1.5, 1). On stop pose `STANDING`, cooldown 10 ticks, `BREEZE_SHOOT` forgotten. `MemorySlot.tick` clears a memory whose time is 0, else counts it down, so the charge leaves 16 ticks after the inhale, once per shoot.
  - `ShootWhenStuck`: a breeze that rides, is in water or levitates gets `BREEZE_SHOOT` for 60 ticks. `Slide`: on the ground, not in water, `STANDING`; with the target within the inner circle (4 blocks across and 10 up and down from its block's centre) `DefaultRandomPos.getPosAway(5, 5)` if it can see the spot from its feet (`BreezeUtil.hasLineOfSight`, 50 blocks) and the spot is farther from the target; otherwise one time in two `BreezeUtil.randomPointBehindTarget` (the target's head turn + 180 + 45 x a gaussian, 4 to 8 blocks) or `randomPointInMiddleCircle` (4 to 8 blocks short of the target on the line to it). The walk target has speed 0.6 and is reached within 1 block.
  - `AbstractWindCharge.onHitEntity`: the struck entity takes 1 (`wind_charge`, message `mob`, scaled when caused by a living non-player) with the owner as its cause, then the charge bursts; `BreezeWindCharge.explode`: radius 3, `entity.breeze.wind_burst`, blocks triggered only with `mob_griefing` (f142f9c1). `Projectile` saves `Owner`, `LeftOwner` and `HasBeenShot`, `AbstractHurtingProjectile` saves `acceleration_power`.
- **Changed (vendored entities crate):**
  - New `world/families/trial/breezes.rs` (family `breezes`, key 40) and `world/families/trial/breezes/brain.rs`, registered in `world/families/trial.rs`. The breeze is a brain family: the registry's `family_base_tick` and `family_after_move`, and between them its own step, as the hoglin's (the idle clock, the brain, ground navigation, move control, jump, look, travel, body turn). The brain keeps the memories above as the fork's `Slot`s and runs the sensors, `StartAttacking`, `StopAttackingIfTargetInvalid`, `Shoot`, `ShootWhenStuck`, `Slide`, `SlideToTargetSink`, the idle `RunOne` and `Swim` in vanilla's priority order. A shot goes into the breeze's `shots`; `breezes::take_shots` hands them to the server. Its `hurt` refuses a hit whose attacker is a breeze and remembers the last hit for `HURT_BY` (40 ticks). It answers `enemy`, gives 10 experience, makes no step sounds, and its base tick's ambient voice becomes `idle_ground` or `idle_air` at volume 1 and pitch 1.
  - `brain_behaviors.rs`: `MoveSink::try_start_for(min, max)` (the slide sink runs 20-40 ticks); `try_start` keeps 150-250.
- **Changed (vendored world crate):** `natural_spawner/families/trial.rs` gets the breeze row (0.6 x 1.77, on the ground, monster, follow range 24, no babies, the any-light rule above), so eggs, `/summon` and spawners can place it.
- **Changed (`minecraft_terrain`):**
  - `families/trial.rs` and new `families/trial/breezes.rs`: `TerrainFamily` (tracking range 10, shadow 0.5, head turn 30, the body turned to the saved yaw on load). Nothing is drawn yet (group breeze-render).
  - `thrown.rs`: `minecraft:breeze_wind_charge` is a wind charge; `Thrown.shooter_uuid` (a loaded charge's owner, found lazily), `Thrown::resumed` (a charge loaded in flight) and `left_owner()`; the breeze charge is drawn with the wind charge sprite.
  - New `breeze_charge.rs` (rustfmt-clean): the charge's tag (`id`, `Pos`, `Motion`, `Rotation`, `Owner`, `LeftOwner`, `HasBeenShot`, `acceleration_power`) both ways, and its hit on a player.
  - `wind.rs`: `Charge::of(item)`.
  - `server.rs`: `throw_breeze_charges` (next to the snow golems' snowballs) turns each shot into a thrown charge with the breeze as shooter. `tick_thrown` finds a loaded charge's breeze by its UUID before it flies, and a burst knows its charge: a breeze charge that hits a player is a `PlayerHitKind::Typed` hit (`minecraft:wind_charge`, message `mob`, 1, coming from the breeze's side, through the queue the mob tridents use); one that hits a mob goes through `mob_shot_mob` (so another breeze shrugs it off); then the radius 3 breeze burst. Breeze charges in flight are saved with their chunk, removed when it unloads, and loaded back.
- **Mob list:** row 52 becomes `live`.
- **Tests added:**
  - `world::families::trial::breezes::tests`: `a_breeze_is_a_tall_thirty_health_monster_with_its_kind` (size, eye height, health, key 40, `entity_type`, census, `mob_candidates`, `is_enemy`, the traits and voice; killed by a player: table `entities/breeze`, 10 experience), `a_breeze_shrugs_off_what_another_breeze_does_and_takes_the_rest` (no damage, sound or memory from a breeze's hit or charge; a player's hit lands and is remembered), `a_breeze_lands_from_a_long_fall_without_hurting` (20 blocks), `a_breeze_targets_survival_players_and_iron_golems_only` (not a creative player or a zombie beside it; an iron golem, a survival player), `a_breeze_slides_then_shoots_its_charge_at_a_player_within_sixteen_blocks` (it slides, then faces the player and shoots one charge from its firing height at 0.3 of the player's height, power 0.7, uncertainty -3 on Normal, the inhale before the shot sound at 1.5; nothing more during the cooldown), `a_breeze_charge_spreads_with_the_difficulty` (1, -3, -7 on Easy, Normal, Hard), `a_breeze_in_water_shoots_without_sliding_first`, `a_breeze_forgets_a_player_who_turns_creative`.
  - `world::families::trial::breezes::brain::tests`: `a_shot_comes_sixteen_ticks_after_the_inhale_and_once_per_shoot` (the inhale at the start, the shot 16 ticks later, still running at its 20th tick, then standing with a cooldown and no `BREEZE_SHOOT`), `a_target_beyond_sixteen_blocks_cancels_the_shot` (the memory is forgotten and it slides), `a_breeze_near_its_target_slides_away_and_far_from_it_slides_in` (speed 0.6, within 1 block; idle while it has a walk target; 4 to 8 blocks from a far target), `the_circles_and_the_spot_behind_a_target_follow_vanilla`.
  - `minecraft_terrain` `families::trial::breezes::tests`: `a_breeze_saves_and_loads_with_its_health_and_turn` (position, yaw, head and body turn, health, persistence, a custom name kept, the UUID), `a_friend_sees_a_breeze_from_its_look_tag` (tracking range 10 with or without `minecraft:`; added to a friend with its yaw; moved by `show_on`).
  - `breeze_charge::tests`: `a_breeze_charge_in_flight_saves_and_loads_with_its_owner`, `only_breeze_charges_are_saved`.
  - `server::tests` (gated on the 26.3 data): `a_breeze_charge_opens_a_door_only_with_mob_griefing` (a real breeze shooting at a player behind an oak door: the charge bursts on the door, which opens with `mob_griefing` and stays shut without it), `a_breeze_charge_hits_its_player_for_the_breeze_and_pushes_them` (one typed hit of 1 from the breeze, no thrower's hit, the burst pushes the player away), `a_loaded_breeze_charge_finds_its_breeze_again` (`Owner` loads as a UUID, the breeze is found before the charge flies, and it saves again with the same `Owner`).
  - The registry's generic tests now cover the breeze too: the eggs survey hatches `breeze_spawn_egg`, `every_registered_kind_is_simulated_and_summonable`, the hit keys, and the cage test (an empty cage until breeze-render).
- **Decisions:**
  - The group's two commits split the behaviours by their subjects: this one has `Shoot`, `ShootWhenStuck`, `Slide` and the slide sink (the shot needs them: `BREEZE_SHOOT` comes from the slide sink's stop or from being stuck); `LongJump`, deflection and the jump, slide, land, whirl and deflect sounds come with commit 2. The inhale and shoot sounds belong to `Shoot` and come now.
  - Brain timings that vanilla draws from the level's random or an unseeded one (the sensors' first scan, the length of `DoNothing` and of the slide sink, the `RunOne` shuffle, the charge's spread) are drawn from the breeze's own random or the server's sound random, as the hoglin and piglin brains do.
  - `LookAtTargetSink` and the `NEAREST_PLAYERS` sensor are left out: nothing in the breeze's brain sets a look target. The targeting range has no invisibility or sneaking factor (as the fork's other brains). A riding target is aimed at 0.3 of its height (the brain's view of a living entity does not know whether it rides).
  - The idle sound is played by the host for everyone (vanilla plays it on each client), at pitch 1 as vanilla.
  - A breeze charge whose breeze is gone hurts no mob, and hits a player without a source (vanilla still hurts for 1 with no cause). Other thrown things are still not saved with their chunk (unchanged); breeze charges are, as vanilla.
  - Thrown things and tridents still do not reach breezes (or silverfish and endermites): `thrown::targets` lists the old families until E2's `hit_boxes` lands with the registry's arm (D0-0). So a player's wind charge, snowball or trident passes through a breeze for now.
- **Protocol:** nothing new: a breeze reaches friends through the registry's look tag and `MobShow.family` (D0), its charges as `ThrownView`s with the item `minecraft:breeze_wind_charge`.
- **Vendored files:** patched `entities/src/brain_behaviors.rs` (patched by the fork before); D0's group files `entities/src/world/families/trial.rs` and `world/src/natural_spawner/families/trial.rs` get their first entries; added `entities/src/world/families/trial/breezes.rs` and `entities/src/world/families/trial/breezes/brain.rs` (two more added files).
- **Runs:** `test -p minecraftoss-entities -p minecraftoss-world --locked --lib` (450 and 114 passed), `test -p minecraft_terrain --locked --lib` (1515 passed, 3 ignored, with the 26.3 data), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The four new `.rs` files pass `rustfmt --check --edition 2021` on their staged content.
- **Also changed:** `server::tests::a_trial_spawner_picks_its_mob_and_waits_for_players_in_the_simulation_area` expected the breeze spawner to stay inactive ("breezes are not simulated yet"); it now wakes up like the zombie one, and an inline config of phantoms (still dormant) takes over the unsimulated case.
- **Test steps** (world `trial10`, survival, Normal, by day, flat open ground, `/gamerule mob_griefing true`). The breeze is not drawn until group breeze-render: you hear it and see its charges and what they do.
  1. Use a breeze egg on the ground 12 blocks away and stand still: after a moment you hear a sharp inhale and a whoosh, and a wind charge flies at you (**330_breeze_charge**). It knocks you back with a gust; your health drops by half a heart at most.
  2. Build a 1-block-thick cobblestone wall 3 high with an oak door in it, stand behind the closed door with the breeze on the other side: a charge bursting on the door opens it. `/gamerule mob_griefing false`, close the door: the bursts leave it shut. `/gamerule mob_griefing true`.
  3. Use a second breeze egg next to the first: their charges never hurt each other.
  4. `/gamemode creative`: the breezes stop shooting at you. `/gamemode survival`.
  5. Use an iron golem egg 6 blocks from a breeze: the breeze shoots at the golem too.
  6. With a charge in the air, save and quit, load: the charge flies on and bursts.
  7. Kill a breeze with the sword (swing where its charges come from, or let it come close): it drops 1-2 breeze rods and experience.
  8. Friend window: the friend stands near a breeze: its charges fly at the friend too and push the friend back, and the friend hears the inhale and the whoosh.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:breeze_spawn_egg*8,minecraft:iron_golem_spawn_egg*2,minecraft:oak_door*2,minecraft:cobblestone*64,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

### 2. Registered Minecraft mobs can turn projectiles back

After ae5bcf0e.

A hook commit (D0-3: a registry hook goes in the root file in its own commit, named for it): the breeze is the first registered mob that deflects projectiles.

- **Vanilla (javap, 26.3):** `Projectile.hitTargetOrDeflectSelf`: before an entity hit lands, `Entity.deflection(projectile)` is asked; anything but `ProjectileDeflection.NONE` deflects the projectile (unless the same entity deflected it last, `lastDeflectedBy`) and the hit never happens. `ProjectileDeflection.REVERSE`: the motion times -0.5 and the yaw turned by 170 + 20 x a random float. Every projectile goes through it: arrows and tridents (`AbstractArrow`), fireballs, small fireballs and wither skulls (`AbstractHurtingProjectile`), shulker bullets, and thrown items (`ThrowableProjectile`). After a deflection an arrow moves on with its new motion in the same tick (`AbstractArrow.tick` sets its position after the hit loop), then drag and gravity.
- **Changed (vendored entities crate, root files):**
  - `world/families.rs`: the `Family::deflect(mob, projectile) -> bool` hook (default: no deflection; the family plays its own sound), `FamilyStore::deflect`, the group macro's `deflect`, `Families::deflect(id, projectile)` and `EntityWorld::deflects_projectile(id, projectile)`.
  - `world.rs` (the arrow tick), `world/fireballs.rs` and `world/shulker_bullets.rs`: an arrow, fireball, small fireball, wither skull or shulker bullet that would hit a registered mob asks it first; a deflected one flies back at half speed instead of hitting. `projectile.rs`: `Arrow::deflect` (the yaw's random draw, -0.5, the move, drag and gravity).
  - `world/families/pests/probe.rs` (test only): the probe deflects when told to and records what it turned.
- **Changed (`minecraft_terrain`):** `thrown.rs` `Thrown::deflect` (alive again, -0.5, the yaw turned by 170-190, moved out by its new motion); `server.rs` `tick_thrown` asks a struck mob before the thrown thing lands on it. Thrown things reach registered mobs only once E2's hit boxes land (see commit 1), so this arm waits for that merge.
- **Tests added:** `world::families::tests::a_registered_mob_that_deflects_turns_arrows_and_fireballs_back` (a mob arrow hurts a plain probe and bounces off a deflecting one, which records `minecraft:arrow`, keeps its health and sends the arrow back out of its box; a blaze's small fireball neither hurts nor burns it and flies back), `thrown::tests::a_deflected_thrown_thing_flies_back_at_half_speed` (a snowball that hit a box flies back at half speed, turned by 170-190 degrees, and does not hit the box again).
- **Decisions:**
  - Instead of vanilla's `lastDeflectedBy` (which would need a field on every projectile), a deflected projectile is moved out by its new motion at once, as vanilla arrows do; fireballs, bullets and thrown things then start their next tick outside the mob's box, so they never hit or turn on the same mob twice in a row.
  - Tridents are left out: their hit is resolved inside the trident's own tick, and they reach registered mobs only with E2's hit boxes. Llama spit is left out too (no registered mob is in its target list). Both are under the group's "Left for later" (after commit 3).
- **Protocol:** none.
- **Vendored files:** patched `entities/src/projectile.rs`, `entities/src/world.rs`, `entities/src/world/fireballs.rs`, `entities/src/world/shulker_bullets.rs` (all patched by the fork before) and D0's `entities/src/world/families.rs`, `world/families/pests/probe.rs` and `world/families/tests.rs`.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (451 passed), `test -p minecraft_terrain --locked --lib` on the thrown, wind, pearl, snowball, breeze, egg, potion, splash, trident and deflect tests (164 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** none on their own (no registered mob deflects until commit 3); the steps of commit 3 cover it.

### 3. Minecraft breezes jump, slide and turn arrows back like vanilla

After 0c83047f.

- **Vanilla (javap, 26.3):**
  - `LongJump` (fight priority 2, 200 ticks): needs the attack target, no jump cooldown, no `BREEZE_SHOOT`, no walk target. `canRun`: on the ground or in water, not in water deeper than its eyes or in lava; a jump target already chosen runs at once; a target 24 blocks away or more (`closerThan(follow range)` fails) is forgotten; nothing within 4 blocks (`distanceTo - 4 <= 0`); its feet block not a honey block and the 4 blocks above it air or water; the spot is `BreezeUtil.randomPointBehindTarget` snapped down (or else up) 10 blocks onto a collision face, the block containing the hit plus one up; refused when the block under that spot `EntityType.isBlockDangerous` (fire, lava, magma, lit campfires, lava cauldrons, wither roses, sweet berry bushes, cactus, powder snow) or when neither the spot's centre nor the centre 4 blocks above it is in sight from its feet. Start: inhaling for 10 ticks, pose `INHALING`, `entity.breeze.charge` (hostile, 1, 1), it looks at the spot. Tick: once inhaling is over, the jump vector (`LongJumpUtil.calculateJumpVectorForAngle` over the angles 40, 55, 60, 75 and 80 shuffled, to the spot's bottom centre less half a block, gravity 0.08, at most 0.058333334 x follow range = 1.4, scaled by 0.95); none: `STANDING`; else `entity.breeze.jump` (1, 1), pose `LONG_JUMPING`, its yaw set to its body's, friction discarded, the motion set. On the ground again (or in water it is not leaving): `entity.breeze.land` (1, 1), `STANDING`, friction back, jump cooldown 2 ticks if it was hurt (`HURT_BY`) else 10, `BREEZE_SHOOT` for 100 ticks. It runs while the pose is not `STANDING` and there is no jump cooldown; on stop `STANDING`, the jump target, inhaling and leaving-water memories forgotten. `Shoot` also needs no jump target, `ShootWhenStuck` no jump target and no inhaling, `Slide` no jump cooldown.
  - `LivingEntity.travelInAir` with `discardFriction`: the motion keeps its sideways speed and loses only gravity (no 0.91 or 0.98 drag).
  - `SlideToTargetSink.start` plays `entity.breeze.slide` (1, 1). `Breeze.causeFallDamage`: `entity.breeze.land` after a fall of more than 3 blocks, then no damage (`#fall_damage_immune`). `Breeze.tick` (the client's): a whirl countdown of 1 to 80 ticks (`nextIntBetweenInclusive`), then `entity.breeze.whirl` at volume 0.8 + 0.2 x random and pitch 0.7 + 0.4 x random (`playLocalSound`).
  - `Breeze.deflection`: `NONE` for `wind_charge` and `breeze_wind_charge`, else (it is in `#deflects_projectiles`) `entity.breeze.deflect` (its sound source, 1, 1) and `ProjectileDeflection.REVERSE`.
  - `calculateOptimalJumpVector` adds lift for a breeze with Jump Boost; the fork's mob effects have no Jump Boost yet, so that lift is left out.
- **Changed (vendored entities crate):**
  - `world/families/trial/breezes/brain.rs`: `LongJump` with its memories (jump cooldown, inhaling, jump target, leaving water), `can_jump_from`, `snap_to_surface`, `dangerous`, `jump_vector`; the requirements above on `Shoot`, `ShootWhenStuck` and `Slide`; the slide sound.
  - `world/families/trial/breezes.rs`: the leap (yaw to the body's, the motion) and `discard_friction`, travelled without drag; the land sound after a fall of more than 3; the `deflect` hook (not for wind charges, with the deflect sound).
  - `movement.rs`: `ground_friction` is `pub(crate)` (the breeze undoes the drag it applied).
- **Changed (`minecraft_terrain`):** `families/trial/breezes.rs` `client_effects`: the whirl, counted per breeze in the viewer's `seen` map, so the host and every friend hear their own whirls.
- **Trial spawners:** breeze trial spawners run since commit 1 (D12's simulated check passes once the breeze is registered); this commit adds the test of a chamber's breeze trial.
- **Mob list:** row 52 keeps `live` (drawn with group breeze-render); "Left" now holds only the model and keyframes.
- **Tests added:**
  - `world::families::trial::breezes::tests`: `a_breeze_long_jumps_behind_its_target_and_lands_without_hurting` (inhaling, long jumping, standing; the charge, jump and land sounds in order; at most 1.4 fast, friction discarded in the air and back after; jump cooldown and `BREEZE_SHOOT` after it lands, no damage), `a_breeze_under_a_low_ceiling_slides_instead_of_jumping`, `a_breeze_forgets_a_target_beyond_its_follow_range` (sooner than the 100 ticks a target merely out of reach is kept), `a_breeze_turns_arrows_back_with_its_sound_but_not_wind_charges` (no deflection of either wind charge; an arrow does not hurt it, flies back, and the deflect sound plays at 1, 1 as hostile).
  - `world::families::trial::breezes::brain::tests`: `the_jump_vector_lands_short_of_its_target_within_the_speed_limit` (each angle that has a vector lands near the spot; none reaches 40 blocks; the limit is 1.4), `a_breeze_needs_four_blocks_of_air_or_water_and_no_honey_to_jump`, `a_jump_spot_sits_one_above_the_surface_and_hazards_refuse_it`.
  - Renamed, as `LongJump` now competes with the slide: `a_breeze_slides_or_jumps_then_shoots_its_charge_at_a_player_within_sixteen_blocks` (was `..._slides_then_shoots_...`), `a_target_beyond_sixteen_blocks_cancels_the_shot_for_a_jump_or_a_slide` (it long-jumps, or slides under a roof) and `a_breeze_near_its_target_slides_away_and_far_from_it_under_a_roof_slides_in`.
  - `minecraft_terrain` `families::trial::breezes::tests::a_breeze_whirls_now_and_then_for_every_viewer` (the whirl at the breeze, volume 0.8-1, pitch 0.7-1.1, 2 to 81 ticks apart).
  - `server::tests::a_breeze_trial_spawner_runs_its_trial_one_breeze_at_a_time` (gated: a spawner with `trial_chamber/breeze/normal` spawns one breeze, waits while it lives, a second after it is gone, then ejects and cools down; never two at once).
- **Decisions:**
  - LongJump's `HURT_BY` check uses the brain's hurt memory (damage in the last 40 ticks, refreshed by its sensor every 20).
  - The vanilla stop of `LongJump` leaves friction discarded if it times out in the air (200 ticks); the fork keeps that.
  - The whirl is a client sound in vanilla, so each viewer plays its own from its mirror of the breeze (D0's `client_effects`); the idle sound stays the host's (commit 1).
- **Protocol:** none.
- **Vendored files:** patched `entities/src/movement.rs` (patched by the fork before); the two breeze files again.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (458 passed), `test -p minecraft_terrain --locked --lib` (1518 passed, 3 ignored, with the 26.3 data), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The breeze files still pass `rustfmt --check --edition 2021` on their staged content.
- **Test steps** (world `trial10`, survival, Normal, by day, flat open ground; the breeze is drawn from group breeze-render on, until then you follow it by its sounds and charges):
  1. Use a breeze egg 10 blocks away and stand still: you hear it gather wind (a rising charge), then a whoosh as it leaps over to land behind or beside you with a thud, and its shots follow (**331_breeze_jump**). It takes no damage from its landings.
  2. Stand under a 3-high roof with the breeze under the same roof: it only slides (a swish) and shoots, it never leaps.
  3. Walk 30 blocks away from it: it stops shooting at you.
  4. Use a skeleton egg 15 blocks away at night (or under a roof) with a breeze between you and the skeleton: the skeleton's arrows that hit the breeze fly back with a whoosh and do not hurt it. A blaze's fireballs bounce off it the same way (Nether, or a blaze egg). Your own wind charges, snowballs and trident still pass through it until E2's hit boxes land.
  5. Stand near a breeze and listen: a soft whirl now and then, also in the friend's window.
  6. In a trial chamber's breeze room (`/locate structure minecraft:trial_chambers`): the breeze spawner wakes, sends one breeze at a time, two for one player, and gives its reward when both are gone (**332_breeze_spawner**). (The kit's trial spawner with a breeze egg runs vanilla's default trial instead: 6 breezes, 2 at once.)
  7. Friend window: the friend hears the jumps, slides and whirls and sees the arrows bounce.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:breeze_spawn_egg*8,minecraft:skeleton_spawn_egg*2,minecraft:blaze_spawn_egg*2,minecraft:trial_spawner,minecraft:iron_golem_spawn_egg*2,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:torch*32,minecraft:diamond_pickaxe+efficiency=5
```

### Left for later

- Tridents do not turn back on a breeze (their hit is resolved inside the trident's tick), and llama spit never reaches registered mobs; both wait for E2's hit boxes, after which a trident needs the deflection arm in `tick_tridents`.
- A player's thrown things (wind charges, snowballs, eggs, pearls, potions) and tridents pass through breezes until E2's `hit_boxes` carries the registry's arm (D0-0); the deflection in `tick_thrown` is ready for them.
- The breeze is not drawn yet (group breeze-render: model, wind and eyes layers, keyframes, particles, its pose for friends).

## Group breeze-render

Brief D16, group breeze-render: the breeze's model, its wind and eyes layers, its keyframes and its particles, for the host and every friend.

### 1. Registered Minecraft mobs can raise block dust

After a1ddd5ae.

A hook commit (D0-3: a missing registry hook goes in the root file in its own commit, named for it). The breeze's ground and jump-trail particles are vanilla `BLOCK` particles of the block it stands in or on, and nothing a family's `client_effects` reaches (`ClientFx`: ambient, spell, portal and poof particles) draws block textures. The warden's digging particles (track W) are the same kind.

- **Vanilla (javap, 26.3):** `Level.addParticle(new BlockParticleOption(BLOCK, state), x, y, z, 0, 0, 0)` makes a `TerrainParticle` through the 7-argument `Particle` constructor: a random direction of up to 0.4 per axis, a speed of `(r1 + r2 + 1) × 0.15 × 0.4` along it plus 0.1 up, half the usual size, tinted like the block's break particles. The provider makes none for air, a moving piston or a block whose render shape is invisible (liquids). `Entity.getInBlockState` is the block at the entity's block position; `getBlockStateOn` the block at `getOnPos` (`BlockPos.containing(x, y − 1.0E-5, z)` when nothing supports it).
- **Changed:**
  - `minecraft_terrain/src/block_particles.rs`: `BlockDust` (where the dust starts, the block the mob is in, the block below its feet, how many), `BlockDust::of(feet, at, count)` and `BlockDust::block` (the block it is in unless that is air, else the one below), and `BlockParticles::spawn_dust`, which spawns that many terrain particles at the point with no push, as the break particles' fragments already do (`spawn_fragment`, the 4096 cap).
  - `minecraft_terrain/src/families.rs` (D0's root file): the trait hook `TerrainFamily::block_dust(mobs, poses, out)` (default: none), the group macro's `block_dust` and the root `families::block_dust(world, poses, out)`, next to `client_effects`. A new hook instead of a new `ClientFx` field: a field would break every `ClientFx { .. }` literal in the other tracks' tests.
  - `minecraft_survival/src/entity_view.rs`: `EntityView.block_dust`, filled once per client tick right after the families' `client_effects`, for the host and every friend.
  - `render_anim`: `present_crumbs` (host and friend) takes the level's scene and hands each dust to `Mining::dust`, which spawns it with the break particles.
- **Tests added:** `block_particles::tests::block_dust_comes_from_the_block_it_is_in_or_else_the_block_below` (stone below the feet; a snow layer it stands in wins; cave air does not; nothing in the air; the cells for negative coordinates), `block_particles::tests::dust_starts_where_it_is_raised_and_flies_off_slowly` (20 fragments at the point, the 0.6 tint, at most 0.18 from a 0.1 lift; the 4096 cap).
- **Decisions:** the particles setting (all, decreased, minimal) is not applied, like the fork's other mob particles.
- **Protocol:** none (each viewer raises its own dust from its mirror of the mob).
- **Vendored files:** none.
- **Runs:** `test -p minecraft_terrain --locked --lib block_particles` (14 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** none on their own (no family raises dust before commit 2); the steps of commit 2 cover it.

### 2. Minecraft breezes are drawn with their wind and eyes

After fbf3b5cc.

- **Vanilla (javap, 26.3, and the textures in the jar):**
  - `BreezeModel.createBaseMesh`: `body` (no boxes) holds `rods` (offset 0, 8, 0) and `head` (offset 0, 4, 0). `rods` holds `rod_1` at (2.5981, −3, 1.5) turned (−2.7489, −1.0472, 3.1416), `rod_2` at (−2.5981, −3, 1.5) turned (−2.7489, 1.0472, 3.1416) and `rod_3` at (0, −3, −3) turned (0.3927, 0, 0), each a 2×8×2 box at (−1, 0, −3), uv 0,17. `head` is a 10×3×4 box at (−5, −5, −4.2), uv 4,24, and an 8×8×8 box at (−4, −8, −4), uv 0,0; its child `eyes` has the same two boxes at offset 0. `wind_body` holds `wind_bottom` (0, 24, 0: 5×7×5 at (−2.5, −7, −2.5), uv 1,83), its child `wind_mid` (0, −7, 0: 12×6×12, 8×6×8 and 5×6×5, uv 74,28, 78,32 and 49,71) and its child `wind_top` (0, −6, 0: 18×8×18, 12×8×12 and 5×8×5, uv 0,0, 6,6 and 105,57).
  - `createBodyLayer` keeps `head` and `rods` (32×32), `createWindLayer` keeps `wind_body` (128×128), `createEyesLayer` keeps `eyes` (32×32). `PartDefinition.retainPartsAndChildren` empties every other part but keeps its pose and children (`addOrReplaceChild` copies the old children), so each of the three models has the whole tree for the keyframes.
  - `setupAnim`: `resetPose`, then `BreezeAnimation` IDLE, SHOOT, SLIDE, SLIDE_BACK, INHALE and JUMP applied on the animation states idle, shoot, slide, slideBack, inhale and longJump at `ageInTicks`. The head does not follow the look. IDLE (2 s, looping) moves wind_top, wind_mid, head and rods; SHOOT 1.125 s, JUMP 0.5 s, INHALE 2 s, SLIDE 0.2 s, SLIDE_BACK 0.1 s. `WIND_TOP_SPEED` and the other wind speeds are declared but not used.
  - `BreezeRenderer`: shadow 0.5, `breeze.png` as `entityTranslucent` (no culling), the default death flip of 90 degrees. `BreezeWindLayer`: its own model of the wind layer with `breezeWind(breeze_wind.png, ageInTicks × 0.02 % 1, 0)` (pipeline `breeze_wind`: translucent, the mob's light, no cardinal shading, no overlay, no culling, the texture moved by the offset). `BreezeEyesLayer`: its own model of the eyes layer with `breezeEyes(breeze_eyes.png)` = `ENTITY_TRANSLUCENT_EMISSIVE` (no light map, cardinal shading, no culling). Both layers pass `NO_OVERLAY`, so only the head and rods flash red. `breeze.png` and `breeze_eyes.png` are only clear or opaque; `breeze_wind.png` has alpha 103 to 175.
  - `Breeze` on the client: `onSyncedDataUpdated(DATA_POSE)` stops shoot, idle, inhale and longJump, then starts shoot (SHOOTING), inhale (INHALING) or slide (SLIDING) at `tickCount` if stopped. `tick`: SLIDING raises 20 ground particles; SHOOTING, INHALING and STANDING raise one (`1 + nextInt(1)`) and reset the jump trail; LONG_JUMPING starts longJump and raises the jump trail (3 particles at the position plus the motion plus 0.1 up, for its first 5 ticks); then idle `startIfStopped(tickCount)`, and when not SLIDING with slide started, slideBack starts and slide stops. Ground particles are `BLOCK` particles of `getInBlockState` (or `getBlockStateOn` when that is air) at its feet, none while it rides.
- **Changed:**
  - Vendored `entities/src/world/families/trial/breezes.rs`: `Animations` (the pose they follow and the start ticks of the six animations) with `tick(pose, tick_count)`, the client logic above; the breeze's tick runs it every tick (also dead or without AI) before its brain, so a pose the brain took in tick T starts its animation at T and idle at T + 1, as vanilla's client sees the synced pose. `starts` and `set_starts` for friends.
  - New `minecraft_terrain/src/families/trial/breezes/render.rs` (rustfmt-clean): the 11 parts with their layers, `Clips` (the six `BreezeAnimation` definitions through D0's `keyframes::shared()` holder; with no jar installed it draws the rest pose), `part_poses`, `model_root`, `wind_shift` and `append_breezes`: head and rods in the model pass at the mob's light with the hurt overlay, the eyes in the same pass at full light without it, the wind in the translucent pass at the mob's light without shading, scrolled with the wither armour's face wrapping (`wither_render::wrap_face`, now `pub(crate)`).
  - `minecraft_terrain/src/families/trial/breezes.rs`: the three textures (D0's `families::textures()` puts them in the atlas), the shadow, `draw`, `show` and `show_on` (`FamilyShow`: a flag and the ticks since the start of each of the six animations in values 0 to 5, the pose in value 6; a friend's mirror sets its starts from its own tick count, so host and friend get the same elapsed ticks and each adds its partial tick), and `block_dust` (commit 1's hook): one dust a tick standing, shooting or inhaling, 20 sliding (none while it rides), 3 for the first 5 ticks of a leap at its feet plus its last movement plus 0.1 up.
  - Spawner and trial spawner cages draw the breeze through D0's generic arm in `spawner_display.rs`. A cage's breeze has no animation started (vanilla's display entity never ticks), so it shows the rest pose.
  - `families/tests.rs` (D0's): the texture test checks the pests' two textures first (`textures()[..2]`), the same one-line change as track M1's d7ae60a2, so the merge is clean.
- **Mob list:** row 52 is `done` (nothing left).
- **Tests added:**
  - Vendored `world::families::trial::breezes::tests::the_animations_follow_the_pose_like_vanillas_client` (idle from the first tick; slide from the tick the pose changed and idle restarted a tick later; slide back when the slide ends; shoot and inhale stop each other; long jump starts in the leap's own tick; slide back outlives other poses; `set_starts` round trip) and `a_breeze_animates_each_pose_a_tick_after_it_takes_it` (a breeze fighting a player for 400 ticks: every pose change starts its animation at the tick it happened and idle a tick later; it shoots and slides or leaps).
  - `families::trial::breezes::render::tests`: `a_breeze_has_eleven_parts_on_its_three_textures` (the part names in vanilla's order, 5 body, 2 eye and 7 wind boxes, the pivots of the head, eyes, a rod and the top of the wind, flip 90, shadow 0.5), `a_breeze_is_drawn_from_its_wind_up_to_its_head_with_glowing_eyes` (42 faces in the model pass and 42 in the translucent pass; the head's top at 1.751 and the wind from 0.001 to 1.3135 above the feet, 18 pixels across; in the dark the body is dark, shaded and without overlay, the eyes at full light, the wind dark and unshaded on its own texture; a breeze the viewer has not seen is not drawn), `its_wind_scrolls_a_fiftieth_of_its_texture_a_tick` (the shift, only along u, faces across the edge cut and kept inside the wind's region), `the_breeze_keyframes_from_the_jar_turn_its_wind_and_move_its_head` (gated on the jar: the six clips read and move only the model's bones, IDLE loops and turns the top of the wind and the head, the eyes stay on the head, a shot adds to the idle, the partial tick moves it on, the drawn breeze follows its keyframes).
  - `families::trial::breezes::tests`: `a_friend_sees_a_breeze_animate_from_the_same_ticks_in_its_pose` (a slide back, a shot and idle sent as flags 0b1011 and ticks 0, 1, 2 with the pose; the friend's mirror at another tick count gets the same elapsed ticks and the pose; the pose ids) and `a_breeze_raises_dust_where_it_stands_slides_and_leaps` (1 standing, shooting or inhaling, 20 sliding, the 3-particle trail 0.1 up in each of the first 5 ticks of a leap and none after, none for a breeze the viewer has not seen).
- **Decisions:**
  - The render file is `families/trial/breezes/render.rs` (D0-3: the model port beside the family's terrain file), not `src/breeze_render.rs` as the brief names it: D0-3 forbids new lines in terrain `lib.rs`.
  - Vanilla's body layer also draws the eyes' two boxes with `breeze.png` (it keeps `head` with its child `eyes`), exactly over the head's own boxes with the same texture; the fork leaves those duplicate faces out.
  - The eyes are drawn in the cut-out model pass at full light, and the body's `entityTranslucent` in the same pass: both textures are only clear or opaque, so the result is the same.
  - The animation start ticks live in the server's breeze, counted by its tick count (as track M1's rabbits). The pose sent to friends and used for the dust is the one the animations follow, a tick behind the brain, as vanilla's synced pose.
  - The keyframe holder is D0's (`keyframes::shared()`, installed by `render_anim` for host and friend); no second `AnimationLibrary` is made.
- **Protocol:** nothing new: the breeze's animations and pose ride in D0's `MobShow.family` (`FamilyShow`), already in the protocol.
- **Vendored files:** `entities/src/world/families/trial/breezes.rs` again (added by this track).
- **Runs:** `test -p minecraftoss-entities --locked --lib` (460 passed), `test -p minecraft_terrain --locked --lib` (1526 passed, 3 ignored, with the 26.3 data, so the jar-gated keyframe test ran), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The new `render.rs` and the two breeze files pass `rustfmt --check --edition 2021` on their staged content.
- **Test steps** (world `trial10`, survival, Normal, by day, flat open ground):
  1. Use a breeze egg 10 blocks away: a pale breeze stands on a whirling, see-through column of wind that swirls sideways, its head bobs, three rods turn around it and its eyes glow. It kicks up a little dust of the ground block at its feet (**319_breeze**).
  2. Watch it fight: its body, head and wind move with each slide, shot, intake of wind and leap (vanilla's animations); a slide throws up a cloud of ground dust and a leap leaves a short trail of dust.
  3. At night or in a closed cobblestone room without torches: its body and wind go dark, its eyes still glow (**323_breeze_eyes**).
  4. Hit it with the sword: the head and rods flash red, the wind and eyes do not. Kill it: it tips over sideways.
  5. Place the kit's trial spawner and use a breeze egg on it: a small breeze turns inside the cage (**324_breeze_cage**). The kit's mob spawner with a breeze egg shows one too. In a trial chamber's breeze room the waiting spawner shows its breeze.
  6. Friend window: the friend sees the same breeze, its wind, eyes, bobbing, slides, shots and leaps at the same moments, and its dust (**322_friend_breeze**, both windows).
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:breeze_spawn_egg*8,minecraft:trial_spawner,minecraft:spawner,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:oak_door,minecraft:torch*32,minecraft:diamond_pickaxe+efficiency=5
```

### Left for later (breeze-render)

- The particles setting (all, decreased, minimal) does not thin the breeze's dust, as for the fork's other mob particles.

## Group breeze-review

The adversarial review of the track's second segment (base `r10-d2-base`: groups breezes and breeze-render, ae5bcf0e to d91c7a8e). Each entry is one defect the review found and its fix.

### 1. Minecraft breezes notice players within their follow range and only in sight

After d91c7a8e.

- **Defect:** the breeze's brain used a fixed targeting range of 16 blocks and skipped the line of sight for its current target, in its attack sensor and in `StopAttackingIfTargetInvalid`. A breeze ignored a survival player 16 to 24 blocks away in plain sight, and it kept a player who hid behind a wall as its target for as long as the player stayed within 16 blocks, shooting at the wall or the door in between.
- **Vanilla (javap, 26.3):**
  - `Sensor.tick` calls `updateTargetingConditionRanges(entity)` before every scan: it sets the range of every shared `TargetingConditions` (`ATTACK_TARGET_CONDITIONS`, `ATTACK_TARGET_CONDITIONS_IGNORE_INVISIBILITY_TESTING` and the others) to the mob's `FOLLOW_RANGE`, 24 for a breeze; the 16 in `Sensor`'s static block is only their starting value. `NearestLivingEntitySensor` (the base of `BreezeAttackEntitySensor`) searches the bounding box inflated by `FOLLOW_RANGE`.
  - `Sensor.isEntityAttackable` uses `ATTACK_TARGET_CONDITIONS_IGNORE_INVISIBILITY_TESTING` for the current attack target. `ignoreInvisibilityTesting` only clears `testInvisible` (no `getVisibilityPercent`); `checkLineOfSight` stays true (only `ignoreLineOfSight` clears it), so `TargetingConditions.test` still asks `Mob.getSensing().hasLineOfSight`.
  - `BreezeAi.initFightActivity`: `StopAttackingIfTargetInvalid.create(Sensor.wasEntityAttackableLastNTicks(breeze, 100).negate())`; `rememberPositives` stays true for 100 checks after the last positive one.
- **Changed:** vendored `world/families/trial/breezes/brain.rs`: `TARGETING_RANGE` is `FOLLOW_RANGE` (24), and `attackable` always needs the line of sight from eye to eye, for the sensor and for the 100-tick memory of `StopAttackingIfTargetInvalid`. This is the rule the hoglin brain already uses (`targetable_within(.., FOLLOW_RANGE)`).
- **Decision:** vanilla's ranges are static and shared by every mob's sensors, so a behaviour that checks between two of the breeze's own scans can see another mob's follow range. The fork uses the breeze's own (the value its scans set).
- **Tests:**
  - `brain::tests::a_breeze_sees_players_within_its_follow_range_and_gives_up_one_out_of_sight`: a player 20 blocks away is targeted and one 25 blocks away is not; behind a wall the target is kept for 100 checks and given up on the 101st; the sensor does not pick its own target behind a wall.
  - `server::tests::a_breeze_charge_opens_a_door_only_with_mob_griefing` now stands the player on a block behind the door, so the breeze sees the player's head over the door (it no longer shoots at a player it cannot see) and its charge still bursts on the door.
- **Test steps** (world `trial10`, survival, Normal, by day, flat open ground, `/gamerule mob_griefing true`):
  1. Use a breeze egg 20 blocks away (count the blocks) and stand still in its sight: it comes for you, leaping or sliding closer, then shoots (**333_breeze_notices**). Before this fix it ignored you until you came within 16 blocks.
  2. While it shoots at you, step behind a 3-high cobblestone wall without gaps: after about 5 seconds it stops and idles. Step back into its sight: it comes for you again.
  3. This replaces step 2 of group breezes, commit 1 (a breeze no longer shoots at a player it cannot see): place an oak door on its own, without a wall. Stand on a cobblestone block 4 blocks behind the door, so your head shows over it, with the breeze about 6 blocks in front of the door. When a charge bursts on the door, the door opens. `/gamerule mob_griefing false`, close the door: the bursts leave it shut. `/gamerule mob_griefing true`.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:breeze_spawn_egg*8,minecraft:oak_door*2,minecraft:cobblestone*64,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32
```

## Combined kit

27 stacks. It holds everything the steps of the groups trial-spawners to review need (97db956b to e9ea905e: trial spawners and their eggs, the mob spawner steps, glass for the sight step, ominous trials with both bottles, vaults with both keys, lodestones with the Nether portal, the friend's clock and recovery compass), with enchanted diamond armour. Left out: the bow and arrows (the sword does), golden apples, milk (`/effect clear @s` does), the totem, the skeleton egg (egg step 2: use the husk egg), the ladder and the water bucket. The steps of group d5-review (bad8dd84 and 0ff5716c: wind charges, the wither and its skull) use their own kit lines above.

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:cobblestone*64,minecraft:glass*32,minecraft:trial_spawner*3,minecraft:zombie_spawn_egg*4,minecraft:husk_spawn_egg*2,minecraft:slime_spawn_egg*2,minecraft:spawner*2,minecraft:ominous_bottle[ominous_bottle_amplifier=1]*2,minecraft:ominous_bottle*4,minecraft:trial_key*6,minecraft:ominous_trial_key*3,minecraft:vault*2,minecraft:lodestone*4,minecraft:compass*8,minecraft:clock,minecraft:recovery_compass,minecraft:obsidian*14,minecraft:flint_and_steel
```

## Protocol

`minecraft_net::PROTOCOL` stays 6 (the release step bumps it).

- Group trial-ominous, commit 2: `EntitySnapshot` (sent to friends in `StateFrame.entities`) gains `ominous_items` (`trial_spawner::OminousItemView`: id, position, item, tint, age) after `items`. A friend on an older build cannot decode the new snapshot, so this needs the protocol bump at the release.

- Group lodestones, commit 3: `ToClient::SpawnPosition { dimension, pos }` (index 35, appended) and `PlayerView.last_death` (appended after `jump_bar`). A friend on an older build cannot decode the player view or the new message, so this rides in the release's protocol bump.

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
- `docs/MINECRAFT-SURVIVAL.md` and CLAUDE.md's "Finish survival" (lodestones, group lodestones, commit 2): a lodestone compass's needle points at its lodestone in the inventory, the hotbar and the hand, and spins in another dimension or once the lodestone is gone; every lodestone compass has its own icon. CLAUDE.md roadmap 1.3 and 2: lodestones are done ("Only lodestones are left" goes). Not yet: a compass held by another player, in an item frame or on the ground keeps the plain compass's look.
- `docs/MULTIPLAYER.md` and `docs/MINECRAFT-SURVIVAL.md` (group lodestones, commit 3): a friend's compasses (plain, recovery and lodestone) and clocks turn like the host's: the host sends the world spawn after the login and whenever it changes, and the friend's last death with its player view. Drop the limit that a friend's compass and clock never move.
- Release 1's known limits (Withers) and the round 9 log (group wither-fight, commit 1): drop "A skull loaded from a save has no owner and hurts as magic for 5". `docs/MINECRAFT-SURVIVAL.md`: fireballs, small fireballs and wither skulls keep their shooter through a save (`Owner`), found again when the shooter loads (group d5-review, commit 2).
- `docs/MINECRAFT-SURVIVAL.md` and the mob list (breezes, group breezes, commit 1): breezes live in the entity world: they come from eggs, `/summon` and trial spawners, target survival players and iron golems within 16 blocks, slide about and shoot wind charges (1 damage from the breeze, a radius 3 burst that opens doors and presses buttons only with `mob_griefing`), shrug off what other breezes do, take no fall damage, drop breeze rods, are saved with their charges in flight and are seen by friends. Not yet: the long jump, turning projectiles back, their other sounds (commit 2) and their model (group breeze-render). Thrown things and tridents pass through them until E2's hit boxes land.
- `docs/MINECRAFT-SURVIVAL.md` and the mob list (breezes, group breezes, commits 2 and 3): breezes long-jump over to land behind their target (a charge, a whoosh and a thud), slide about with a swish, whirl now and then for every viewer, forget a target 24 blocks away, and turn back arrows, fireballs, wither skulls and shulker bullets (not wind charges) with their deflect sound; a chamber's breeze trial spawner sends one breeze at a time, two for one player. Registered mobs can turn projectiles back (a registry hook). Not yet: their model (group breeze-render); tridents and a player's thrown things wait for E2's hit boxes.
- Registered mobs can raise block dust (group breeze-render, commit 1: a registry hook, `TerrainFamily::block_dust`).
- `docs/MINECRAFT-SURVIVAL.md` and the mob list (breezes, group breeze-render, commit 2): breezes are drawn like vanilla: a whirling, see-through column of wind under a bobbing head with glowing eyes and three turning rods, with vanilla's keyframes for idling, shooting, sliding, gathering wind and leaping, the ground dust they kick up, and the same for friends and in spawner cages. Drop "Not yet: their model".
- `docs/MINECRAFT-SURVIVAL.md` and the mob list (breezes, group breeze-review, commit 1): breezes notice survival players and iron golems they can see within 24 blocks (their follow range), shoot from within 16, and give up a target they have not seen for 5 seconds.

## Vendored files

Patched in round 10 (none added before group breezes): `entities/src/fireball.rs` and `entities/src/world/fireballs.rs` (group d5-review, commit 2), and `world/src/level/container.rs` (group trial-spawners, commit 2: `Level::set_block_entity_tag`). All three were already patched by the fork (the first two are in round 9's list, `docs/round9/track-d-log.md`, Vendored files; `container.rs` since c5efeed0), so the fork's counts do not change. Group trial-spawners-run, commit 1: `world/src/natural_spawner/spawning.rs` and `world/src/level/spawning.rs` (both patched by the fork before). Commit 2: `world/src/level/physics.rs`, patched by the fork for the first time (one more patched file). Commit 3: `world/src/level/spawning.rs` again and `world/src/level/dispenser.rs` (patched by the fork before, c987075b). Group trial-ominous, commit 1: `world/src/level/spawning.rs` again (`summon_trial_mob` no longer adds the mob itself). Commit 2: `world/src/level/entity.rs`, `world/src/level/mod.rs` and `entities/src/world.rs`, all patched by the fork before. Group vaults patches and adds no vendored file. Group lodestones patches and adds no vendored file. Group review, commit 1: `entities/src/world.rs` again (patched by the fork before). Group breezes, commit 1: patched `entities/src/brain_behaviors.rs` (patched by the fork before) and D0's group files `entities/src/world/families/trial.rs` and `world/src/natural_spawner/families/trial.rs` (added by D0); added `entities/src/world/families/trial/breezes.rs` and `entities/src/world/families/trial/breezes/brain.rs` (two more added files). Commit 2: `entities/src/projectile.rs`, `entities/src/world.rs`, `entities/src/world/fireballs.rs` and `entities/src/world/shulker_bullets.rs` (patched by the fork before) and D0's `entities/src/world/families.rs`, `world/families/pests/probe.rs` and `world/families/tests.rs`. Commit 3: `entities/src/movement.rs` (patched by the fork before) and the two breeze files. Group breeze-render, commit 2: `entities/src/world/families/trial/breezes.rs` again (added by this track). Group breeze-review, commit 1: `entities/src/world/families/trial/breezes/brain.rs` again (added by this track).
