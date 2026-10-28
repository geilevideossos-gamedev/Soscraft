# Track D: trial chambers, vaults, lodestones and breezes

The owner's words for this track (round 10 brief):

> D. Finish track D: the rest of d5 (trial chambers and trial spawners, lodestones), d3 (deep dark, warden, sculk), and d4 (the remaining mobs), with d4 split into two or three tracks by mob group. Land the shared changes (mob list, spawn tables, render registry) first so the later merges stay small.

Round 10 splits that request. d3 (deep dark, warden, sculk) is track W (`docs/round10/track-w.md`), d4 is tracks M1-M3, and the shared mob registry is track D0 (`docs/round10/track-d0.md`). **This track (D) is what is left of round 9's d5**:

- **D11. d5 review:** the partial d5 run (keyframes 81f6f353, withers d4f932de..ef9ad9de, wind charges f142f9c1) stopped before its review.
- **D12. Trial spawners:** the cage, the egg, the spawn rules and the visual line of sight, the trial itself and its rewards, its particles and sounds (round 9's D7 groups `trial-spawners` and `trial-spawners-run`).
- **D13. Ominous trials:** Trial Omen, ominous spawners and the items they drop from above (round 9's `trial-ominous`).
- **D14. Vaults:** trial keys, rewards once per player, the turning reward item (round 9's `vaults`).
- **D15. Lodestones and compasses**, including friends' compasses and clocks (round 9's D8 `lodestones`).
- **D16. Breezes:** the mob and its drawing (round 9's `breezes` commit 2 and `breeze-render`). Its first commit, "Minecraft wind charges open doors and press buttons like vanilla", landed as f142f9c1.

The track continues the `track-d` branch. Groups run in this order: `d5-review`, `trial-spawners`, `trial-spawners-run`, `trial-ominous`, `vaults`, `lodestones`, then `breezes` and `breeze-render`. **The breeze groups go last and need D0**: they start only after D0 is merged into `minecraft-survival` and `minecraft-survival` is merged into `track-d` (a second segment, like track W's `w2`).

Paths and line numbers are as of 6e0e0573 (the track-d merge, checked out in `C:\Dev\mc-wt\track-d`). Tracks D0, W, M1-M3, E1, E2 and F run at the same time, so re-check every line before editing. Vanilla facts were checked this round with javap on the 26.3 `client.jar` and against the 26.3 data inside that jar; round 9's "javap during the work" items for this track are resolved below (marked **resolved**). No Mojang code or data is copied: numbers and behaviour only, and trial spawner configs, loot tables and animations are read at runtime.

Every section has the same parts: the owner's finding, the cause in the current code, what vanilla does, the plan as commits, the tests, and the owner's numbered in-game steps with screenshot points and a kit line. Screenshots of this track are numbered from 301 so they do not clash with round 9's (up to 261) or the round 10 release files.

---

## D0'. Shared points for the track

### Multiplayer rule
The host owns every level, every block entity and every rule. Friends run no AI and no block entity ticks: they draw trial spawners and vaults from the block entity tags the host forwards (`ToClient::BlockEntity`, `crates/minecraft_survival/src/session.rs:1434-1452` `send_spawners`, applied at `crates/minecraft_survival/src/remote.rs:413-420`) and from block states in their chunks, and they play level events from `ToClient::LevelEvent` (`crates/minecraft_survival/src/heard.rs:113`). The host runs every player's item uses (friends included), so vault keys and lodestone compasses work for friends through the same code. Everything a player earns is keyed by UUID (`ServerSim::player_uuid`, `crates/minecraft_terrain/src/server.rs:182`: `self.mobs.uuid_of(PLAYER_TARGET + id)`), never by player 0.

### Coordination with the other round 10 tracks
| Track | Shared with D |
|---|---|
| D0 | the mob registry; breezes plug into it (D16). `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`) may become D0's registry list: trial spawners ask "is this kind simulated" through whichever exists when they land |
| W | game events: trial spawners post `entity_place` (`TrialSpawner.spawnMob`); whichever of W and D merges second adds that one line (track W rule 3). The keyframe `AnimationLibrary` holder: W's `warden-render` or D's `breeze-render`, whichever lands first, creates it; the other reuses it. W also adds level events (3006, 3007) to `level_events.rs` and sculk particle kinds to `ambient_particles.rs`: keep both sides' arms and variants on merge |
| M1-M3 | silverfish (the trial chamber `small_melee/silverfish` spawners stay inactive until a track simulates silverfish) |
| E1 | new particle kinds in `crates/minecraft_terrain/src/ambient_particles.rs:42-59` (keep both sides' variants on merge) |
| E2 | `thrown.rs` targets (thrown items hitting mobs): breezes deflect them (D16) |

### Protocol
`minecraft_net::PROTOCOL` is 6 (`crates/minecraft_net/src/messages.rs:7`) and shipped in round 10 release 1. The first commit of this track that changes a serialized struct friends decode (the ominous item spawner in the entity snapshot, D13; `ToClient::SpawnPosition` and `PlayerView.last_death`, D15; breeze `MobShow` fields, D16) bumps it to 7, unless another round 10 track already bumped it since the last release, in which case D rides in that number. At most one bump per release; record the number in the track log and in `docs/round10/decisions.md` at merge time. Block entity tags and level events need no bump: their messages exist.

### Batching
`run_tick` (`server.rs:4195`) may run several times with one `TickInput` while a batch catches up (`:4161-4179`). Every new `TickInput` field of this track (the players' omens for trial spawners, the lodestone checks) is carried like `carry_omens` (`server.rs:3577-3592`), dropped in `remove_player` (`:3593-3603`) and drained once per input. The new server passes (trial spawners, vaults) run inside `run_tick`'s `if normal` block (`:4219-4226`), so `/tick freeze` stops them as it stops vanilla block entity ticks.

### Files and docs
- The track log is `docs/round10/track-d-log.md`: commits, decisions, vanilla numbers, test steps and one combined kit line. Round 9's log stays as it is.
- The mob list: the breeze row (52) in the file D0 names (`docs/round9/mobs.md` today) changes in the same commit as the code.
- Do not edit `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md`, `docs/PROGRESS.md` or `docs/round10/decisions.md`. Write their new wording in the log under "State doc changes" (trial spawners, vaults and lodestones leave "Finish survival"; breezes join the mob list).
- Count the vendored minecraftoss files the track patches and adds in the log ("Vendored files").

---

## D11. Review of the partial d5 run (group `d5-review`)

### The owner's finding
Not reported by the owner. The round 10 decision "track-d merged without its d5 review" (`docs/round10/decisions.md`) left keyframes, withers and wind charges unreviewed. The track runner's review of this track uses base `r9-d5-base`, so it covers those commits again at the end; this group fixes what a first reading found.

### What was read
- f142f9c1 (wind charges): `third_party/minecraftoss/world/src/level/redstone.rs:923-964` `wind_trigger` and `explosion.rs:169-185` `wind_charge_blocks` match vanilla's `onExplosionHit` arms, `SimpleExplosionDamageCalculator` resistances and `Util.shuffle` (javap). The door, trapdoor, gate, button, lever and candle sounds and pitches in `server.rs:1906-1916` match vanilla. No defect.
- 81f6f353 (keyframes): the sampler and reader match the javap'd `KeyframeAnimation` (round 9 log). No defect.
- d4f932de..ef9ad9de (withers): hurt order (`note_hurt` before the hit, as vanilla starts `destroyBlocksTick` before `LivingEntity.hurtServer`), boss bars per session (`session.rs` `send_wither_bars`) and the skull hit read correct. Two items below.

### Finding 1: a wind charge and a flying player (needs a check, then a test or a fix)
- Release 1's known limits say "A player flying in creative is still pushed by a burst" (`docs/round10/release-1-tests.md`, Known limits, Wind charges), copied from the round 9 log's group breezes decision.
- Vanilla (javap): `SimpleExplosionDamageCalculator.getKnockbackMultiplier` returns 0 for a `Player` whose `getAbilities().flying` is set; wind charges and breeze charges use that calculator.
- Current code: every push reaches the player through `PlayerQueue.out.pushes` (`crates/minecraft_survival/src/entities.rs:301-304`) and is applied in `crates/render_anim/src/minecraft_players.rs:903-913` only when `ps.pm_flags & MC_FLYING == 0`, for the host and for friends (the host runs every player's step). So the known limit is probably wrong.
- Checked for this brief: `wind.rs:131` `burst` → `server.rs:1896` `players_pushed` → `Output.players_pushed` (`server.rs:4282-4283`) → `entities.rs:301-304` `queue.out.pushes` → `minecraft_players.rs:906`. The other pushes in that loop (`landed`, `:765`, the knockback of mob hits; creeper blasts, `entities.rs:329`; shield blocks, `server.rs:4133`; dragon pushes, `server.rs:2642`) pass the same `pushed` filter (`:903-905`). `MC_FLYING` is only creative flight (`crates/movement_iw4/src/fly.rs:60-79`; elytra gliding is `MC_GLIDING`), so it matches vanilla's `abilities.flying`.
- Plan: re-check that no new push path appeared since. If all go through the flying filter, commit a test that pins it and write the correction in the log's "State doc changes" (the known-limit line goes). If one bypasses it, carry `flying` in `thrown::PlayerBody` (`crates/minecraft_terrain/src/thrown.rs:203-208`) and give it multiplier 0 in `wind.rs` `push` (`:96`).

### Finding 2: fireballs and wither skulls forget their shooter in a save (a vanilla gap)
- `server_mobs.rs:1896-1915` saves fireballs and wither skulls without `Owner`, and `:1135-1144` loads them with `owner = None` (the round 9 log calls this a known limit: "A skull loaded from a save has no owner and hurts as magic for 5"). Shulker bullets already save and load `Owner`: saved at `:1936-1938` (the live owner's UUID, else the UUID still waiting), loaded into `owner_uuid` at `:1154`, and resolved lazily by `id_of_uuid` when the bullet next ticks (vendored `entities/src/world/shulker_bullets.rs:230-249`).
- Vanilla (javap `Projectile.addAdditionalSaveData`/`readAdditionalSaveData`): `Owner` as an entity reference (UUID int array), `LeftOwner`, `HasBeenShot`; the owner resolves by UUID when it is loaded. So a skull saved in flight still hits for 8 with Wither II and "was shot by a skull from Wither", and a reloaded ghast fireball still credits its ghast.
- Plan: write `Owner` from the shooter's UUID (`EntityWorld::uuid_of`; a `Shooter::Player(id)`, a fireball a player batted back, is `uuid_of(PLAYER_TARGET + id)`), load it into a new `Fireball.owner_uuid` (vendored `entities/src/fireball.rs:96` holds `owner: Option<Shooter>`), and resolve it lazily like shulker bullets: by `id_of_uuid` before the fireball's hit, a player id mapping back to `Shooter::Player`. Do not resolve at load time: the skull's chunk can load before its wither's, and vanilla's `EntityReference` is lazy too. An owner never found stays `None` (magic 5, as today).

### Plan (group `d5-review`, 2 commits)
1. **"Track D pins that a wind charge does not push a player flying in creative"** (or, if a bypass exists, **"A Minecraft wind charge does not push a player flying in creative"**): as above.
2. **"Minecraft fireballs and wither skulls keep their shooter through a save"**: `server_mobs.rs` save and load arms; `wither_save.rs` if the skull fields live there; vendored `entities/src/fireball.rs` (`owner_uuid`) and the lazy resolve in `entities/src/world/fireballs.rs` before the hit.

### Tests
- `render_anim` or `minecraft_survival`: a burst push for a flying player is dropped, a walking player's is applied (whichever layer holds the filter; if `settle` is not testable, test the predicate after moving it into a small function).
- `server_mobs::tests::a_wither_skull_saved_in_flight_still_belongs_to_its_wither` (save with `mob_tags_where`, load the skull first and the wither after it: the skull's hit is the wither's, 8 damage; into a world without the wither: `None`, magic 5), the same for a ghast's large fireball, and a fireball a player batted back keeps that player as its owner.
- `minecraft_survival` `death::tests`: a reloaded skull's kill reads "was shot by a skull from Wither".

### Owner's steps
New world `IW4L_MINECRAFT_WORLD=trial10`, survival, Normal, by day, flat open ground.
1. `/gamemode creative`, double-tap jump to fly 5 blocks up, throw a wind charge at a block right below you: the burst does not move you (**301_wind_flying**). Stop flying, land, throw one at your feet: it launches you. `/gamemode survival`.
2. Take the armour off for this step (diamond armour cuts a skull's 8 damage to under a heart, so 4 and 2.5 hearts could not be told apart). `/tick rate 2` (the world runs ten times slower). Build a wither (four soul sand in a T, three wither skeleton skulls), run 20 blocks and wait for the blast. When a black skull is in the air towards you, `/tick freeze`. Save and quit, load the world, `/tick rate 20`, `/tick unfreeze`: the skull hits you for about 4 hearts (not 2.5) with Wither II (black hearts) (**302_skull_after_reload**). If it kills you, the chat says "<name> was shot by a skull from Wither". Put the armour back on.
3. Friend window: the friend joins again after the host has loaded the world (the host's save and quit ends the friend's session), and sees the wither's next skulls fly and hears them hit.

```
IW4L_MINECRAFT_KIT=minecraft:wind_charge*16,minecraft:soul_sand*8,minecraft:wither_skeleton_skull*6,minecraft:netherite_sword,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:milk_bucket*2,minecraft:totem_of_undying,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

---

## D12. Trial spawners (groups `trial-spawners`, `trial-spawners-run`)

### The owner's finding
"Trial chambers and trial spawners", asked in round 9 and again now. Round 8 deferred them (`docs/round8/decisions.md`: no mob in the cage, no spawning, eggs refused, **not vanilla**).

### Cause (current code)
- **Generation works but nothing tests it.** Trial chambers are a jigsaw structure with pool aliases (`third_party/minecraftoss/generator/src/structure/jigsaw.rs:331-345` parses `direct`, `random` and `random_group` aliases; 26.3's `worldgen/structure/trial_chambers.json` uses `random_group`). Their trial spawner and vault tags survive generation (`third_party/minecraftoss/core/src/block_entity.rs:294-311`; `vault_config` at `:519-535`). The 26.3 templates give trial spawners registry ids: `normal_config: "minecraft:trial_chamber/breeze/normal"`, `ominous_config: "minecraft:trial_chamber/breeze/ominous"` (read from `structure/trial_chambers/spawner/breeze/breeze.nbt`), and vaults `config: {key_item: trial_key, loot_table: "minecraft:chests/trial_chambers/reward"}` (`reward/vault.nbt`; `reward/ominous_vault.nbt` is the ominous one).
- **Nothing ticks them.** `Level::tick_spawners` takes only `minecraft:mob_spawner` (`third_party/minecraftoss/world/src/level/spawning.rs:397-413`, the filter at `:404`), called from `ServerSim::tick` (`server.rs:3154-3162`, at `:3158`), itself called from `run_tick` (`server.rs:4222`).
- **Nothing draws them.** `spawner_display.rs` keeps only mob spawners (`SPAWNER` at `crates/minecraft_terrain/src/spawner_display.rs:13`, `is_spawner` at `:104-106`, `load_chunk` `:504-521`, `block_entities` `:523-544`, the block check in `append` at `:589`); the host forwards only those to friends (`crates/minecraft_survival/src/entities.rs:1641-1643`).
- **Eggs are refused.** `spawn_egg.rs:10` already counts `minecraft:trial_spawner` as a spawner, so an egg on one goes to `ServerSim::spawner_egg` (`server.rs:1571-1586`), which answers "no monster spawner" at `:1581` (and the chat says so since round 9's small-not-vanilla commit 1).
- **Spawner spawn rules are too strict for some kinds.** `spawner_may_spawn` (`third_party/minecraftoss/world/src/natural_spawner/spawning.rs:700-727`) asks for open sky for `Rules::SurfaceMonster` (husks), and sends `Rules::Stray` and `Rules::Slime` to the natural rules (open sky, swamp or slime chunk). Vanilla skips all three for spawners (below). Today this only hits rare mob spawners; trial chambers have husk, stray and slime spawners underground.
- **Particles and level events are missing.** `ambient_particles.rs:42-59` has no `SmallFlame`, `SoulFireFlame`, `TrialSpawnerDetection`, `TrialOmen`, `OminousSpawning` or `VaultConnection`; `level_events.rs:87-139` handles none of 3011-3021.
- **Loot is ready.** Round 9's `loot-functions` ported `set_potion`, `set_enchantments`, `set_ominous_bottle_amplifier` and friends; the test `ancient_city_pyramid_and_trial_chamber_chests_roll_with_their_components` (`containers.rs:1921`) already rolls trial chamber chest tables, including `chests/trial_chambers/reward_ominous_common`. The `spawners/trial_chamber/*` and `spawners/ominous/trial_chamber/*` tables and `equipment/trial_chamber_*` have no test yet: D12 adds one.

### Vanilla 26.3 (javap, data in the jar)
**Configs.** 28 files under `data/minecraft/trial_spawner/trial_chamber/` (breeze, melee/{husk,spider,zombie}, ranged/{poison_skeleton,skeleton,stray}, slow_ranged/{poison_skeleton,skeleton,stray}, small_melee/{baby_zombie,cave_spider,silverfish,slime}, each `normal` and `ominous`). `TrialSpawnerConfig` fields and defaults (`TrialSpawnerConfig$Builder.<init>`): `spawn_range` 4, `total_mobs` 6.0, `simultaneous_mobs` 2.0, `total_mobs_added_per_player` 2.0, `simultaneous_mobs_added_per_player` 1.0, `ticks_between_spawn` 40, `spawn_potentials` empty, `loot_tables_to_eject` `spawners/trial_chamber/consumables` and `spawners/trial_chamber/key` at weight 1 each, `items_to_drop_when_ominous` `spawners/trial_chamber/items_to_drop_when_ominous`. Targets: `floor(total_mobs + total_mobs_added_per_player × additional)` and the same for simultaneous, `additional = max(0, detected − 1)`. `ticksBetweenItemSpawners()` is a constant 160. Examples: breeze normal is total 2 (+1), simultaneous 1 (+0.5), 20 ticks; zombie ominous gives each zombie the `equipment/trial_chamber_melee` table with `slot_drop_chances` 0 and ejects `spawners/ominous/trial_chamber/key` (weight 3) or `.../consumables` (7); slime small_melee spawns `Size` 1 (weight 3) or 2 (1).

**Block entity and state** (`TrialSpawner$FullConfig`, `TrialSpawnerStateData$Packed`): keys `normal_config`, `ominous_config` (a registry id or an inline config), `required_player_range` (default 14), `target_cooldown_length` (36000); state data `registered_players`, `current_mobs` (UUIDs), `cooldown_ends_at`, `next_mob_spawns_at`, `total_mobs_spawned`, `spawn_data`, `ejecting_loot_table`. The client update tag holds `spawn_data` and, only while ACTIVE, `next_mob_spawns_at`. Block properties `trial_spawner_state` and `ominous`. A missing config is the empty default (no potentials).

**States** (`TrialSpawnerState`: name, light, particles, spin speed, can spawn): inactive (0, none, −1, no), waiting_for_players (4, small flames, 200, yes), active (8, flames and smoke, 1000, yes), waiting_for_reward_ejection (8, small flames, −1, no), ejecting_reward (8, small flames, −1, no), cooldown (0, smoke inside and on top, −1, no). A spin speed of −1 means no mob in the cage.

**`TrialSpawner.tickServer`** each tick for a loaded trial spawner (`isOminous` from the block): untrack `current_mobs` whose entity is gone, dead, in another dimension or farther than 47 blocks (`BlockPos.distSqr > 47²`); if any was untracked, `next_mob_spawns_at = now + ticks_between_spawn`. Then `tickAndGetNext` (resolved):
- **inactive:** to waiting_for_players once a display entity exists (`getOrCreateNextSpawnData` picks `spawn_data` from the active config's potentials with the level random and marks the block entity changed; an id-less entity gives none).
- **waiting_for_players:** if `!canSpawnInLevel` → `resetStatistics` (detected players cleared, totals and both clocks 0), stay. If no mob to spawn (no `spawn_data` id and empty potentials) → inactive. Else `tryDetectPlayers`; any detected → active.
- **active:** `canSpawnInLevel` false → reset, waiting_for_players; no mob → inactive; `additional` counted before `tryDetectPlayers`; when ominous, `spawnOminousOminousItemSpawner` (D13); if `total_mobs_spawned >= target total`: when `current_mobs` is empty, `cooldown_ends_at = now + target_cooldown_length`, totals 0, `next_mob_spawns_at` 0 → waiting_for_reward_ejection; else if `now >= next_mob_spawns_at` and `current_mobs < target simultaneous`, `spawnMob`; on success add the UUID, count it, `next_mob_spawns_at = now + ticks_between_spawn`, and pick the next `spawn_data` from the potentials.
- **waiting_for_reward_ejection:** when `now >= (cooldown_ends_at − target_cooldown_length) + 40`: sound `block.trial_spawner.open_shutter` → ejecting_reward.
- **ejecting_reward:** every `TIME_BETWEEN_EACH_EJECTION` = 30 ticks (`(now − (cooldown_ends_at − cooldown)) % 30 == 0`): if no detected player is left: sound `block.trial_spawner.close_shutter`, `ejecting_loot_table` cleared → cooldown. Else roll `ejecting_loot_table` once (picked from `loot_tables_to_eject` the first time) and eject it, then remove one detected player.
- **cooldown:** `tryDetectPlayers` (D13: only an ominous change gets through); any detected → totals 0 → active; else when `now >= cooldown_ends_at`: `removeOminous`, `reset` (current mobs and `spawn_data` cleared, statistics reset) → waiting_for_players.

**`tryDetectPlayers`** only when `(pos.asLong() + gameTime) % 20 == 0`; nothing while cooldown and ominous. `PlayerDetector.NO_CREATIVE_PLAYERS`: players with `blockPosition().closerThan(pos, range)` (block distance squared `< range²`), not creative, not spectator; the first scan requires line of sight from the block centre to the eye (`ClipContext` VISUAL blocks, no fluids; sight when the hit block is the spawner itself or the ray misses). The D13 omen check runs here. Then, unless in cooldown without an omen change: if no player was detected before, add the sighted list, else add a second scan **without** line of sight; when that added anyone, `next_mob_spawns_at = max(now + 40, next_mob_spawns_at)` and level event 3013 (3019 when ominous) with data = detected count.

**`canSpawnInLevel`:** game rule `spawner_blocks_work`, then (unless overridden) not peaceful and game rule `spawn_mobs`.

**`spawnMob`** (resolved): the position is the entity's saved `Pos`, else `(x + (r1 − r2) × spawn_range + 0.5, y + nextInt(3) − 1, z + (r3 − r4) × spawn_range + 0.5)` with the level random; fail on collision in the type's spawn box, on no line of sight from the spawn spot to the block centre, on `SpawnPlacements.checkSpawnRules(TRIAL_SPAWNER)`, or on the entry's `custom_spawn_rules`. The entity loads from the tag with a random yaw `nextFloat() × 360`; a `Mob` fails on `checkSpawnObstruction`; it gets `finalizeSpawn(TRIAL_SPAWNER)` only when the tag is just an id, always `setPersistenceRequired`, and the entry's `equipment` (`EquipmentUser.equip`: each rolled item goes to its `equippable` slot if free, else the main hand if free, with the drop chance). Then level event 3011 at the spawner and 3012 at the mob (data 0 normal flames, 1 soul flames when ominous).

**Spawn rules for spawners** (resolved, javap `EntitySpawnReason`, `Monster`, `Stray`, `Slime`): `isSpawner` is true for SPAWNER and TRIAL_SPAWNER; `ignoresLightRequirements` only for TRIAL_SPAWNER. `checkMonsterSpawnRules` = not peaceful and (ignores light or dark enough) and `checkMobSpawnRules` (true for spawners). `checkSurfaceMonstersSpawnRules` and `checkStraySpawnRules` add open sky only when not a spawner. `checkSlimeSpawnRules`: not peaceful, then for spawners just `checkMobSpawnRules`. A trial spawner never asks `Mob.checkSpawnRules` (the walk target value), only `checkSpawnObstruction`.

**Rewards** (`ejectReward`): `getRandomItems` with the empty context; each stack via `DefaultDispenseItemBehavior.spawnItem(level, stack, 2, UP, atBottomCenterOf(pos) + 1.2 up)`: y − 0.125 for the Y axis, speed `nextDouble() × 0.1 + 0.2`, motion `triangle(0, 0.0172275 × 2)`, `triangle(0.2, 0.0172275 × 2)`, `triangle(0, 0.0172275 × 2)`; then level event 3014.

**Eggs** (`overrideEntityToSpawn`): state data `reset`, both configs become inline copies of themselves with `spawn_potentials` = that type alone (numbers and loot kept, no equipment), state inactive; the next tick makes it waiting_for_players.

**Client** (`tickClient`, `ParticleEmission`, `TrialSpawnerRenderer`): the cage mob spins `spin += speed / (max(0, next_mob_spawns_at − gameTime) + 200)` mod 360 and is drawn like a mob spawner's (scale `0.53125 / max(width, height)` when above 1, angle `lerp(spin) × 10`). Small flames: one time in two a `small_flame` (soul fire flame when ominous) at the centre `offsetRandom(0.9)`. Flames and smoke: a `smoke` and a `flame` (soul) at `offsetRandom(1.0)`. Smoke inside and top face: one time in three a `smoke`, and when `gameTime % 20 == 0` 20 to 23 smokes on the top face. When it can spawn, 2 % a tick `block.trial_spawner.ambient` (`ambient_ominous`), volume `nextFloat × 0.25 + 0.75`, pitch `nextFloat + 0.5`.

**Level events** (`LevelEventHandler`, all sounds at the block, volume 1, pitch `(r1 − r2) × 0.2 + 1`): 3011 spawn particles (20 × smoke plus the flame of `data` within the block); 3012 the same plus `block.trial_spawner.spawn_mob`; 3013 `detect_player` plus `30 + min(data, 10) × 5` `trial_spawner_detection` particles; 3019 the same with `trial_spawner_detection_ominous`; 3014 `eject_item` plus 20 small flames rising from the top; 3020 `block.trial_spawner.ominous_activate` (volume 0.3 for data 0, 1 for data 1) with ominous detection and `trial_omen` particles; 3021 `spawn_item` with soul flames.

### Plan
**Group `trial-spawners`**
1. **"Minecraft trial spawners show their mob spinning like vanilla"**
   - New `crates/minecraft_terrain/src/trial_spawner.rs` (rustfmt-clean): `TrialConfig` read from a registry id (`data/minecraft/trial_spawner/<path>.json` through the data pack the server already reads loot from) or an inline compound, with the defaults above; `TrialState` (name, light, spin speed, particle kind, can spawn) from the block property; `spawn_data` pick (weighted, level random).
   - A minimal server pass beside `tick_spawners` (`spawning.rs:397`): a trial spawner in a simulation chunk that is `inactive` and has a mob picks `spawn_data`, sets `waiting_for_players` and marks the block entity changed. Group `trial-spawners-run` replaces it with the full state machine.
   - `spawner_display.rs`: `is_displayed_block_entity` (mob spawners, trial spawners, and vaults for D14) replaces `is_spawner` in `load_chunk`, `block_entities` and the host's forward filter (`entities.rs:1642`); trial spawner displays read `spawn_data.entity` and `next_mob_spawns_at`, take the state and `ominous` from the block in the scene, spin by the formula above with the level game time (`SpawnerDisplays::tick` gains `game_time`; `entity_view.rs:146` passes the entity world's `game_time()`), draw the mob only while the state has a spin speed, and emit the state's particles and ambient sound. `append` (`:589`) accepts `trial_spawner` blocks.
   - New particle kinds `SmallFlame` and `SoulFireFlame` in `ambient_particles.rs` (vanilla `FlameParticle` providers; sprites from the particle atlas).
   - A generation test in `crates/minecraft_terrain/src/terrain.rs` beside `ancient_cities_generate_in_the_deep_dark_with_sculk_shriekers_and_loot_chests` (`:2739`): locate a trial chamber, find `minecraft:trial_spawner` block entities whose configs are registry ids that resolve, and `minecraft:vault` block entities with a `trial_key` or `ominous_trial_key`.
2. **"Minecraft spawn eggs set a trial spawner's mob"**
   - `ServerSim::spawner_egg` (`server.rs:1571-1586`): a trial spawner takes the egg as vanilla's `overrideEntityToSpawn` (both configs inline with that type, state data reset, `inactive`); the refusals for peaceful monsters and `spawner_blocks_work` stay as they are.

**Group `trial-spawners-run`**
1. **"Minecraft spawners spawn husks, strays and slimes without open sky or slime chunks"**
   - `spawner_may_spawn` (`spawning.rs:700-727`) takes the spawn reason (spawner or trial spawner): no sky test for `SurfaceMonster` and `Stray`, slimes and magma cubes only "not peaceful" for spawners, light ignored for trial spawners. Mob spawners keep their walk test (`BaseSpawner` asks `Mob.checkSpawnRules`); trial spawners skip it.
2. **"Minecraft line of sight passes through glass and iron bars like vanilla"**
   - A `ClipBlocks::Visual` mode in `third_party/minecraftoss/world/src/level/physics.rs:66-74` (`clip_first_hit` at `:352`): collision shapes, except vanilla's `getVisualShape` overrides (javap): `TransparentBlock` (glass, stained and tinted glass), `IronBarsBlock` (bars and panes) and powder snow are empty, mud and soul sand are full cubes, snow layers use their layer shape, fences use their outline shape. Split out of the trial commit so that commit stays one build agent's size; its only user is the next commit (and D13's upward clip).
3. **"Minecraft trial spawners run their trial and eject rewards like vanilla"**
   - `trial_spawner.rs`: `TrialSpawner` with `from_tag`/`write_tag` (all keys above, UUIDs as int arrays) and `tick(now, ominous, &mut impl TrialWorld) -> Vec<TrialEvent>` running the six states exactly as listed. `TrialWorld` asks: players near (UUID, feet, eye, alive, creative or spectator, line of sight), whether a tracked mob is alive and where, `spawn_mob(entry, pos)`, roll a loot table, the game rules and difficulty, the level random.
   - `Level::summon_trial_mob` beside `summon_mob` (`spawning.rs:206`): the position checks, the spawn rules with the trial reason, `custom_spawn_rules`, finalize only for an id-only tag, `PersistenceRequired`, the equipment table (`EquipmentUser.equip` slot rule) with its drop chances; returns the tag with its UUID.
   - `ServerSim::tick_trial_spawners(players)` called in `run_tick` right after `sim.tick()` (`server.rs:4222`), inside the `if normal` block and before `sim.take_spawned()` (`:4228`) so the new mobs join the entity world in the same tick, for trial spawners in simulation chunks only (as `tick_spawners` filters at `spawning.rs:403`). Players come from `TickInput.mob_players` (`PlayerCandidate`, `third_party/minecraftoss/entities/src/tempt.rs:6-27`: `attackable` is false for creative and spectator, `crates/minecraft_terrain/src/game_mode.rs:155`) with their UUIDs from `player_uuid`. Tracked mobs are found by UUID (`mob_with_uuid`, vendored `entities/src/world/riding.rs:752`); one not loaded counts as gone, as vanilla's `getEntity(uuid)`.
   - Line of sight: the `ClipBlocks::Visual` of commit 2, no fluids.
   - A kind the game does not simulate (not in `SIMULATED_KINDS`, or not in D0's registry once it exists) counts as "no mob to spawn", so that spawner stays `inactive` (fork decision: a dormant mob would never die and the trial would never end). Breeze and silverfish spawners wake up when those kinds land.
   - Rewards: generalise `Level::spawn_item` (`third_party/minecraftoss/world/src/level/dispenser.rs:125-135`, accuracy 6, from a dispenser face) into a `spawn_dispensed_item(position, direction, accuracy, stack)` used by both.
   - Sounds `open_shutter` and `close_shutter` through `Output.mob_sounds` (heard by host and friends like the wind charge sounds); block entity changes through the level's `take_changed_block_entities` → `Output.block_entities` (`server.rs:3330-3349`), forwarded to friends by the D12 filter.
4. **"Minecraft trial spawners show their detect, spawn and eject particles and sounds"**
   - `level_events.rs`: 3011, 3012, 3013, 3014, 3019, 3020, 3021 with the particles and sounds above; new particle kinds `TrialSpawnerDetection`, `TrialSpawnerDetectionOminous` and `TrialOmen` (vanilla `TrialSpawnerDetectionParticle` and the trial omen `SpellParticle`; sprites from the atlas). The server pushes them to `Output.level_events`; friends get them through `heard.rs`.

### Tests
- `trial_spawner::tests`: every config in the jar reads (28, gated on `MINECRAFTOSS_ROOT`: the slime weights, the zombie ominous equipment table and its loot weights 3 and 7); targets for the empty default config 6/2 for one player and 8/3 for two, for `melee/zombie/normal` 6/3 and 8/3 (simultaneous 3.0 + 0.5, 20 ticks), for `slow_ranged/skeleton/normal` 6/4 and 8/6 (160 ticks), breeze 2/1 and 3/1; the `spawners/trial_chamber/*` reward tables and `equipment/trial_chamber_melee` roll (gated); a waiting spawner detects a survival player in range and sight only on its 20-tick phase, never a creative or spectator one; the second player is added without sight; one mob per `ticks_between_spawn`, never more than the simultaneous target; all spawned and dead → `waiting_for_reward_ejection`, shutter 40 ticks later, one ejection per detected player every 30 ticks, then cooldown 36000 and back to waiting; a mob farther than 47 blocks is untracked and replaced after `ticks_between_spawn`; peaceful, `spawn_mobs` off or `spawner_blocks_work` off resets to waiting; an unsimulated kind stays inactive; the tag round-trips with every key.
- `spawning::tests` (vendored world): a mob spawner of husks spawns underground, a stray spawner without sky, a slime spawner outside a slime chunk; natural spawning still needs sky and slime chunks.
- `server::tests::a_trial_spawner_spawns_zombies_for_a_near_player_and_rewards_after_they_die` (model on `a_fortress_spawner_spawns_blazes_near_a_player_and_waits_between_rounds`, `server.rs:10023`): six zombies for one survival player, persistent, none for a creative one; killing them gives one reward roll that lands as items above the block; `an_egg_sets_a_trial_spawners_mob` (both configs, inactive then waiting).
- `spawner_display::tests`: a waiting trial spawner shows its mob and spins slower than an active one; inactive, ejecting and cooldown show none; the soul flames when ominous.
- `level_events::tests`: 3013 with data 2 gives 40 detection particles and the detect sound; 3020 with data 0 is quiet (0.3).
- `terrain::tests::trial_chambers_generate_with_trial_spawners_and_vaults` (gated like the ancient city test).
- `physics::tests`: the visual clip passes through glass and iron bars, stops at stone and at a fence's outline.

### Owner's steps
New world `IW4L_MINECRAFT_WORLD=trial10`, Normal difficulty, survival. Each step names its screenshot.
1. `/locate structure minecraft:trial_chambers`, `/tp` to it (it is underground, about y −40 to −20), dig down with the pickaxe. Copper and tuff rooms; trial spawners show a small mob turning inside with small orange flames. **303_trial_spawner_waiting**.
2. Walk into sight of a zombie spawner (within 14 blocks; zombie, husk, spider and plain skeleton spawners behave the same): a burst of particles and a "detect" sound, then it lights up, the mob spins faster, and about 2 seconds later mobs come out about one a second with a puff of flame, 6 in all, never more than 3 at once. (A "slow ranged" skeleton spawner, in the rooms with raised platforms, sends one every 8 seconds, up to 4 at once.) **304_trial_spawner_active**.
3. Kill them. About 2 seconds after the last one dies the shutter opens and one reward (a trial key or food and potions) pops out on top, then it goes dark and smokes. **305_trial_reward**. Walk up and look: it stays dark (cooldown, 30 minutes).
4. Lure one spawned mob 50 blocks away (or leave it behind a wall far off): the spawner forgets it and sends a new one.
5. `/gamemode creative`, walk to another spawner: it stays waiting. `/gamemode survival`.
6. 30 blocks from the chamber, place a trial spawner from the kit on stone and use a zombie egg on it: a small zombie spins inside, and since you stand within 14 blocks in sight, zombies start coming at once: one every 2 seconds, 6 in all, never more than 2 at once (a placed trial spawner has vanilla's default numbers). **306_trial_spawner_egg**.
7. Deep underground (dig a room at y −30, no sky above), place another trial spawner and use a husk egg, then a slime egg on a third: husks and slimes spawn (no open sky or slime chunk needed). Place the kit's mob spawner in the same room with no torches near it and use a husk egg: it also spawns husks in the dark.
8. `/difficulty peaceful` while a trial runs: the mobs vanish and the spawner goes back to waiting. `/difficulty normal`.
9. Save and quit while a trial runs, load: the same spawner continues with the mobs it had.
10. Friend window, in the same chamber: the spawner spawns 8 mobs for two players (3 at once), ejects two rewards (one per player), and the friend sees the flames, the spinning mob, the light and the particles and hears the sounds. **307_friend_trial** (both windows).

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:trial_spawner*3,minecraft:spawner,minecraft:zombie_spawn_egg*4,minecraft:skeleton_spawn_egg*2,minecraft:husk_spawn_egg*2,minecraft:slime_spawn_egg*2,minecraft:cobblestone*64,minecraft:glass*32,minecraft:ladder*32,minecraft:water_bucket,minecraft:milk_bucket,minecraft:spyglass
```

---

## D13. Ominous trials (group `trial-ominous`)

### The owner's finding
Part of "trial chambers and trial spawners". Bad Omen, Trial Omen and ominous bottles exist since round 9's `effects-omens` (`third_party/minecraftoss/player/src/survival.rs:54-58`, `crates/minecraft_survival/src/player_nbt.rs:37`), but nothing turns Bad Omen into Trial Omen and no spawner turns ominous.

### Cause (current code)
- `PlayerOmens` (`server.rs:124-127`) carries only `bad_omen` and the raid omens; the trial omen never reaches the server.
- The raid omen path shows the pattern: the server returns `(player, amplifier, pos)` in `Output.raid_omens` (`server.rs:3679`), `PlayerQueues::route` hands it to that player (`entities.rs:379-383`), and `render_anim/src/minecraft_players.rs:922-928` calls `Survival::turn_bad_omen` (`crates/minecraft_survival/src/survival.rs:225`) and plays the omen sound. `PlayerOmens` is built by struct literal at `server.rs:3588`, `:4214` and in tests (`:12500`, `:12505`): a new field touches each.
- No ominous item spawner entity: the level's entity kinds are items, lightning, TNT, falling blocks and orbs (`third_party/minecraftoss/world/src/level/entity.rs:89-95`); the snapshot friends get has no slot for it (`EntitySnapshot`, `server.rs:329-348`).
- No projectile dispensing: arrows, potions, fire charges and wind charges are listed as not ported in `dispenser.rs:17-27`.

### Vanilla 26.3 (javap, resolved)
- **Omen check** in `tryDetectPlayers`, on the sighted scan, when the spawner is not ominous: the first detected player (list order) that has Trial Omen, else the first with Bad Omen (`findPlayerWithOminousEffect`). Bad Omen becomes Trial Omen for `18000 × (amplifier + 1)` ticks at amplifier 0 (`transformBadOmenIntoTrialOmen`; Trial Omen plays `event.mob_effect.trial_omen` when added, `MobEffects` `withSoundOnAdded`). Then level event 3020 at the player's eye block with data 0, and `applyOminous`: block `ominous=true`, level event 3020 at the spawner with data 1, and `resetAfterBecomingOminous`: each current mob gets level event 3012 (normal flames), drops its preserved equipment and is discarded; `spawn_data` cleared when the ominous config has potentials; `total_mobs_spawned` 0; current mobs cleared; `next_mob_spawns_at = now + ominous ticks_between_spawn`; `cooldown_ends_at = now + 160`. In cooldown, a spawner turned ominous this way goes straight back to active (D12 cooldown arm); an ominous spawner in cooldown detects nobody.
- **Active config:** the ominous config while ominous, else the normal one. `removeOminous` at the end of the cooldown sets `ominous=false`.
- **Items from above** (`spawnOminousOminousItemSpawner`, each active tick while ominous): the dispensing list is built once from `items_to_drop_when_ominous` rolled with seed `level seed + BlockPos(floor(x / 30), floor(y / 20), floor(z / 30)).asLong()`, one entry per item at weight = its count; nothing when empty. When `now >= cooldown_ends_at`: a target among the detected players (alive, not creative, not spectator, within the required range) and the current mobs (alive, within range), chosen by `TrialSpawnerState.selectEntityToSpawnItemAbove` (resolved, javap): the level random's `nextBoolean()` takes the current mobs (resolved by UUID, non-null, within range of the block) when true and the players when false; an empty pick gives no target this time (no fallback to the other list), one entry is taken as is, more go through `Util.getRandom`; the spot is the target's position raised by `bbHeight + 2 + nextInt(4)`, clipped upward (VISUAL, no fluid) to the first block, taken as that block's centre one block down; no spot when that block has collision. There an `OminousItemSpawner` appears with a weighted pick of the list, sound `block.trial_spawner.spawn_item_begin` (pitch `(r1 − r2) × 0.2 + 1`), and `cooldown_ends_at = now + 160`.
- **`OminousItemSpawner`**: size 0.25, tracking range 8, no physics, saved as `item` and `spawn_item_after_ticks` (60 to 120, `nextIntBetweenInclusive`). At `spawnItemAfterTicks − 36` it plays `block.trial_spawner.about_to_spawn_item` (neutral); at `spawnItemAfterTicks` a `ProjectileItem` (arrows, tipped arrows, fire charges, wind charges, splash and lingering potions) is shot straight down with its `DispenseConfig` power and uncertainty (resolved, javap `createDispenseConfig`: the default is power 1.1, uncertainty 6.0, kept by arrows and tipped arrows; fire charges and wind charges power 1.0, uncertainty 6.6666665; splash and lingering potions (`ThrowablePotionItem`) power 1.1 × 1.25 = 1.375, uncertainty 6.0 × 0.5 = 3.0), owned by the spawner entity; any other item drops as an item entity; level event 3021 with data 1; then it is removed. Client: every 5 game ticks 1 to 3 `ominous_spawning` particles flying in from `0.4 × (gauss − gauss)` around it; the renderer draws the item at full light, scaled `min(age, 50) / 50`, turning 40 degrees a tick.
- The 26.3 `items_to_drop_when_ominous` table: one lingering potion (wind charged, oozing, weaving, infested, strength, swiftness, slow falling) and one of arrow, poison arrow, slowness IV arrow, 1-3 fire charges, 1-3 wind charges.

### Plan (group `trial-ominous`)
1. **"Minecraft trial spawners turn ominous with Trial Omen"**
   - `PlayerOmens` gains `trial_omen: bool`; `Survival::omens` (`survival.rs:218`) fills it.
   - `trial_spawner.rs`: the omen check, `apply_ominous`, `reset_after_becoming_ominous`, `remove_ominous`, the active config switch, and the cooldown re-activation; `TrialEvent::TurnOmen { player, amplifier }`.
   - `Output.trial_omens: Vec<(u64, i32)>` routed like `raid_omens` (`entities.rs:379`); `Survival::turn_bad_omen_into_trial` replaces Bad Omen by Trial Omen (18000 × (amplifier + 1), amplifier 0; nothing when Bad Omen is already gone, as `turn_bad_omen` at `survival.rs:225-235`) and the host plays `event.mob_effect.trial_omen` for that player (`minecraft_players.rs`, next to the raid omen sound at `:922-928`).
   - Batching: when the server asks to turn a player's Bad Omen, it also sets that player's entry in `input.omens` to Trial Omen and no Bad Omen (as `run_tick` rewrites `input.omens` after the raids at `server.rs:4212-4214`), so the later runs of the same batch see the omen already turned and do not ask twice.
   - Soul flames: the ominous block state already switches the display's particles (D12); the ominous config's equipment (`equipment/trial_chamber_melee`, `trial_chamber_ranged`) comes from `summon_trial_mob`.
2. **"Ominous Minecraft trial spawners drop items from above"**
   - Vendored `world/src/level/entity.rs`: `EntityKind::OminousItemSpawner { stack, spawn_after, age }` with save and load (`minecraft:ominous_item_spawner`, `item`, `spawn_item_after_ticks`), its tick (sounds, the drop), and level event 3021.
   - Projectile drops through the server's existing throwers: arrows into the entity world as arrows with no shooter, potions and wind charges through `thrown.rs`, fire charges as small fireballs; each with the javap'd dispense power and uncertainty, direction down.
   - `trial_spawner.rs`: the dispensing list, `select_entity_to_spawn_item_above`, the upward clip, and the 160-tick cadence.
   - `EntitySnapshot.ominous_items` (position, item, age) for host and friends; drawn by the item mesher in `crates/minecraft_survival/src/entity_view.rs` beside the item frames (`:1293-1301`) at full light with the scale and turn above; `OminousSpawning` particles every 5 game ticks. This changes the friend snapshot: protocol rule above.

### Tests
- `trial_spawner::tests`: a waiting spawner sighting a player with Bad Omen II turns ominous and asks to turn the omen into Trial Omen for 54000 ticks; a player who already has Trial Omen turns it ominous without a change; a player out of sight does not; current mobs are discarded with event 3012; the ominous config's potentials and loot are used; at the end of the cooldown it is plain again; ejecting rolls `spawners/ominous/trial_chamber/key` about 3 times in 10.
- `survival::tests`: Bad Omen IV becomes Trial Omen I for 90000 ticks; no Bad Omen, no change.
- `entity::tests` (vendored world): an ominous item spawner plays its warning at `spawn_after − 36`, drops a plain item as an item entity at `spawn_after` with event 3021, and survives a save and load with its ticks.
- `server::tests::an_ominous_trial_spawner_drops_a_wind_charge_from_above_its_player` (a fixed seed and a list holding only a wind charge: a thrown wind charge appears above the player 60-120 ticks after the cadence allows it, flying down).
- The codec round trip of the snapshot with an ominous item.

### Owner's steps
In the trial chamber of D12 (a fresh spawner that has not run yet), Normal difficulty, survival, full kit armour.
1. Drink an ominous bottle (Bad Omen icon top right). Walk into sight of a fresh spawner: a deep sound, blue (soul) flames swirl around you and the spawner; the Bad Omen icon turns into Trial Omen (15 minutes for level I, more for a stronger bottle). **308_trial_omen**.
2. The spawner now burns with blue flames. Mobs come with iron or diamond swords, bows and armour. Every 8 seconds or so a small item appears high above you or a mob, spinning and glowing with dark particles flying into it, and after 3-6 seconds it drops: a potion breaks into a lingering cloud, an arrow or wind charge flies down, a fire charge falls. **309_ominous_item**.
3. Win the trial: it ejects an ominous trial key or ominous consumables. **310_ominous_reward**.
4. Walk to a second fresh spawner while Trial Omen lasts: it turns blue at once.
5. While an item floats above you, save and quit, then load: the item is still there and drops when its time is up; the spawner is still blue and its trial goes on.
6. Friend window (the friend joins after step 5): the friend sees the blue flames, the floating items and their drops, and gets Trial Omen from their own bottle at a third fresh spawner. **311_friend_ominous**.

A spawner's 30-minute cooldown counts game ticks; `/time add` does not shorten it (vanilla too), so the return to a plain spawner is covered by the tests, not by a step.

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle[ominous_bottle_amplifier=1]*2,minecraft:ominous_bottle*2,minecraft:milk_bucket*2,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:totem_of_undying,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:trial_spawner*2,minecraft:zombie_spawn_egg*4,minecraft:cobblestone*64,minecraft:ladder*32,minecraft:water_bucket
```

---

## D14. Vaults (group `vaults`)

### The owner's finding
Part of "trial chambers": the reward rooms of every chamber hold vaults and ominous vaults.

### Cause (current code)
- Vault tags survive generation (`block_entity.rs:294-302`, `vault_config` `:519-535` keeps non-default config fields), but nothing ticks, opens or draws a vault. The block itself is drawn from its block states (`vault_state`, `ominous`, `facing`; `assets/minecraft/blockstates/vault.json`).
- A right-click with an item on a block goes through the use chain (`crates/minecraft_survival/src/actions.rs:1074-1134`) and `Change` (`crates/minecraft_terrain/src/bucket.rs:144-152`); nothing handles a vault. The server answers item uses with `Output.item_uses` (`server.rs:3658`), routed back to the player's `item_use_waiting` (`entities.rs:306-310`), which spends the item only on success.

### Vanilla 26.3 (javap, resolved)
- **Config** (`VaultConfig`): `loot_table` `chests/trial_chambers/reward`, `activation_range` 4.0, `deactivation_range` 4.5, `key_item` `trial_key`, optional `override_loot_table_to_display`; players by `PlayerDetector.INCLUDING_CREATIVE_PLAYERS` (any non-spectator within range, **no** line of sight).
- **Data:** `server_data` (`rewarded_players` up to 128 UUIDs, the oldest dropped beyond, `state_updating_resumes_at`, `items_to_eject`, `total_ejections_needed`), `shared_data` (`display_item`, `connected_players`, `connected_particles_range`). Light: inactive 6, active, unlocking and ejecting 12.
- **`VaultBlockEntity$Server.tick`:** every 20 game ticks while active, `display_item` cycles to a random item of a fresh roll of the display table (`LootContextParamSets.VAULT` with `ORIGIN`); when `now >= state_updating_resumes_at`, `VaultState.tickAndGetNext`:
  - inactive and active: connected players = detected within activation range (inactive) or deactivation range (active) that are not in `rewarded_players`; pause 20 ticks; active if any, else inactive.
  - unlocking: pause 20 → ejecting.
  - ejecting: when `items_to_eject` is empty, `markEjectionFinished`, then the connection update with deactivation range; else pop one item and eject it, `display_item` = the next, pause 20.
  - On a change: the block state, then `onExit`/`onEnter`: entering inactive clears the display item and plays level event 3016 (data 1 when ominous); entering active cycles a display item if none and plays 3015; entering unlocking plays `block.vault.insert_item`; entering ejecting plays `block.vault.open_shutter`, leaving it `block.vault.close_shutter`.
- **Ejection** (`ejectResultItem`): `spawnItem(level, stack, 2, UP, atBottomCenter + 1.2 up)` as the trial spawner's, level event 3017, sound `block.vault.eject_item` at pitch `0.8 + 0.4 × progress`, progress = 1 when one ejection is needed, else `1 − (left − 1) / (total − 1)`.
- **`VaultBlock.useItemOn`:** nothing with an empty hand or when not active (tries the empty hand). `tryInsertKey`: no key configured or inactive → nothing; the item not the same item and components as `key_item` or fewer than its count → `block.vault.insert_item_fail`; the player already rewarded → `block.vault.reject_rewarded_player`; both fail sounds at most once per 15 ticks (`last_insert_fail_timestamp`); the loot (`VAULT` params with origin, the player's luck, `THIS_ENTITY` and the key as `TOOL`) empty → nothing; else `ItemStack.consume(key count, player)` (creative keeps it), `unlock` (items to eject, display the first, pause 14 ticks, state unlocking), add the player to `rewarded_players`, update the connected players.
- **Client** (`VaultBlockEntity$Client`, `VaultRenderer`): the display item drawn at (0.5, 0.4, 0.5) as an item cluster, turning 10 degrees a tick; while active, idle `small_flame`/`soul_fire_flame` particles one time in two each 20 ticks and `vault_connection` particles from the keyhole to each connected player; 3015 and 3016 make 20 flames and `block.vault.activate`/`deactivate`; 3017 eject particles; 2 % a tick `block.vault.ambient`.

### Plan (group `vaults`)
1. **"Minecraft vaults take trial keys and eject rewards once per player"**
   - New `crates/minecraft_terrain/src/vault.rs` (rustfmt-clean): config, server and shared data `from_tag`/`write_tag` (the shapes `block_entity.rs:294-302` keeps), the four states, `tick(now, &mut impl VaultWorld)` and `insert_key(player_uuid, stack, creative) -> KeyAnswer`, the fail-sound rate limit, the 128-player cap.
   - `ServerSim::tick_vaults(players)` after `tick_trial_spawners` in `run_tick`, for vaults in simulation chunks; the display roll and the reward roll through the loot tables with the `VAULT` parameters.
   - The key: `Change::Vault(pos)` in `bucket.rs:144-152`, offered in the use chain (`actions.rs:1074-1134`) only when the block is a vault in state `active` and the hand is not empty; `Entities::vault_key(player, pos, sent)` sends `Command::VaultKey { player, pos, stack, creative }` and sets `item_use_waiting`; the server answers through `Output.item_uses` with `true` only on an unlock, so the key is spent only then and never in creative.
   - Sounds through `Output.mob_sounds`; level events 3015, 3016, 3017 through `Output.level_events`; the reward items through the generalised `spawn_dispensed_item` (D12).
2. **"Minecraft vaults show their reward item turning"**
   - `spawner_display.rs` (or a sibling `vault_display.rs`): vault displays read `shared_data.display_item` and `connected_players` from the tag (forwarded by the D12 filter), turn the item 10 degrees a tick with the partial tick, and emit the idle, activation and deactivation flames (soul when ominous) and the 2 % ambient sound.
   - `entity_view.rs`: the item drawn by the item mesher (as frames at `:1293-1301`), host and friends.
   - Level events 3015, 3016, 3017 in `level_events.rs`; a `VaultConnection` particle kind towards each connected player the client can see (players it cannot map from UUID to a body get none, recorded as a decision).

### Tests
- `vault::tests`: inactive → active when a player (creative too) comes within 4 blocks, back to inactive beyond 4.5; a trial key from a new UUID unlocks it (14-tick pause, then ejecting, one item every 20 ticks, pitch rising to 1.2); after the ejection a lone rewarded player leaves it inactive and a key does nothing (no sound, not spent); while another, unrewarded player keeps it active, a second key from the rewarded UUID is refused with the reject sound, and a key from the other UUID works; a wrong item or an ominous key on a plain vault fails with the fail sound at most once per 15 ticks; the ominous vault wants `ominous_trial_key` and rolls `reward_ominous`; the display cycles every 20 ticks only while active; 129 rewarded players keep the newest 128; the tag round-trips.
- `server::tests::a_vault_spends_a_key_only_when_it_opens` (survival: spent once, a second try keeps it; creative: kept) through `Command::VaultKey` and `Output.item_uses`.
- `actions` or `bucket` tests: an empty hand or an inactive vault gives no `Change::Vault`.
- The display test: the item turns 10 degrees a tick and is hidden while inactive.

### Owner's steps
In a trial chamber's reward room (vaults stand in the corridors near the chamber's middle; ominous vaults have a darker frame), or with the kit's vault placed on open ground.
1. Walk within 4 blocks of a vault: it lights up and an item turns inside it, changing every second. **312_vault_active**.
2. Before using a trial key, right-click the lit vault with an ominous trial key: a dull failing sound, the key stays. Click several times fast: the sound plays at most about once every three quarters of a second (15 ticks).
3. Right-click it with a trial key: a click, the key is gone, the shutter opens and items pop out one by one, each a little higher in pitch. **313_vault_open**.
4. After the last item it closes and goes dim, because it only lights for players it has not rewarded. Right-click it with a second key: nothing happens and the key stays. Step 5 blocks away and come back: it stays dim for you.
5. A chamber's ominous vault (darker frame; the kit's vaults are plain): it wants an ominous trial key, gives other rewards, and burns with blue flames. **314_ominous_vault**.
6. `/gamemode creative`, open a second vault (the kit's other one, placed elsewhere) with a trial key: the key stays in your hand. `/gamemode survival`.
7. Save and quit, load: the first vault stays dim for you.
8. Friend window (two windows, both near the first vault): it lights up for the friend and the item turns in both windows. While it is lit, the host right-clicks it with a trial key: a refusing sound, the key stays. The friend opens it with their own key and gets their rewards; then it goes dim for both. **315_friend_vault** (both windows).

```
IW4L_MINECRAFT_KIT=minecraft:trial_key*6,minecraft:ominous_trial_key*3,minecraft:vault*2,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:cobblestone*64,minecraft:ladder*32
```

---

## D15. Lodestones and compasses (group `lodestones`)

### The owner's finding
"Lodestones", the last open N6 item ("Only lodestones are left for Finish survival", CLAUDE.md roadmap 1.3).

### Cause (current code)
1. Nothing turns a compass into a lodestone compass: no link in the use chain (`actions.rs:1074-1134`, the arms at `:1156-1193`) and no `Change` for it (`bucket.rs:144-152`); no file writes `minecraft:lodestone_tracker`. The lodestone POI type exists (`third_party/minecraftoss/entities/src/poi.rs:94`, `:154`) but nothing keeps a lodestone POI.
2. The needle cannot point at a lodestone: `Dials` evaluates once per item id (`DIAL_ITEMS` at `crates/minecraft_terrain/src/dials.rs:9-13`, `step` at `:102-124`), `minecraft:condition` always takes `on_false` (`:210`), and `compass()` knows only `spawn` and `recovery` (`:151-184`).
3. Icons and held models are keyed by id: `item_tint::of_stack` (`crates/minecraft_terrain/src/item_tint.rs:24-26`) feeds `McStack.icon` (`crates/render_anim/src/minecraft_inventory.rs:462`, `:664`) and `icon_source` (`:769-772`) looks the dial up with `dials.key(base)`; the first-person hand uses `dials.key(&stack.id)` (`crates/render_anim/src/minecraft_hand.rs:195`); held items in the world use `of_stack` (`crates/minecraft_terrain/src/mesh.rs:1006`). The name comes from `display_name` (`minecraft_inventory.rs:929-941`), which knows no lodestone compass. The glint already works (`crates/minecraft_terrain/src/glint.rs:37`).
4. Nothing clears the target when the lodestone goes.
5. A friend's compass and clock never move: only the host steps the dials (`crates/render_anim/src/minecraft_world.rs:916`); the friend publishes its inventory without stepping (`crates/render_anim/src/minecraft_remote.rs:1330`), and never receives the world spawn or its last death (`Login`, `crates/minecraft_net/src/messages.rs:360-370`; `PlayerView`, `:481`; the host has them in `globals.spawn` and `Player.last_death`, `crates/minecraft_survival/src/player.rs:35`).

### Vanilla 26.3 (javap, data in the jar)
- **`CompassItem.useOn`** on `Blocks.LODESTONE`: sound `item.lodestone_compass.lock` (players, 1, 1) for everyone near; a tracker `{target: {dimension, pos: [x, y, z]}, tracked: true}`; a stack of exactly one outside creative (`!hasInfiniteMaterials && count == 1`) changes in place; otherwise `transmuteCopy(COMPASS, 1)` makes one tracked compass (keeping the other components), the held stack is consumed by one (creative keeps it), and the copy goes into the inventory or is dropped. Returns success; any other block falls through to `Item.useOn`. A compass that already tracks relinks the same way.
- **Name and glint:** `item.minecraft.lodestone_compass` whenever the component is present; `isFoil` too.
- **`inventoryTick` → `LodestoneTracker.tick`:** only when `tracked` and a target exists and the target's dimension is the player's level: when the position is outside the world's build height or the POI manager has no lodestone POI there, the target is removed (`{tracked: true}` stays, so the needle spins and the name and glint stay).
- **Model** (`assets/minecraft/items/compass.json`): `condition` on `minecraft:has_component` `minecraft:lodestone_tracker`; `on_true` is `range_dispatch` `minecraft:compass` target `lodestone`, scale 32, 33 entries; `on_false` the spawn compass. `CompassAngleState` spins at random with no target, another dimension, or a target within `1e-5` squared; our `Needle` does the rest.

### Plan (group `lodestones`)
1. **"Minecraft compasses lock onto lodestones like vanilla"**
   - New `crates/minecraft_terrain/src/lodestone.rs` (rustfmt-clean): `tracker(stack)`, `with_tracker(stack, target)`, `lock_use(player, world, stack) -> Option<ItemUse>` (a compass on `minecraft:lodestone`), `lock(inventory, slot, target, infinite) -> Option<ItemStack>` (in place, or the transmuted copy to add or drop), `forget(inventory, dimension, pos)` (targets removed, `tracked` kept).
   - `Change::Lodestone(Pos)` in `bucket.rs`; the chain link and match arm in `actions.rs`; the lock sound through `heard` so every player near hears it.
   - The tracker tick, host-owned for every player: each player's tracked targets in its current dimension go in `TickInput.lodestones` (with the batching rule above); the server thread answers in `Output.lodestones_gone` those whose chunk is loaded and whose block is not `minecraft:lodestone`, or that lie outside the build height; the main thread calls `forget` on that player's inventory. Fork decision: a target in an unloaded chunk is kept (vanilla's POI manager can read unloaded POI sections; the fork keeps no lodestone POIs).
   - `display_name` returns "Lodestone Compass" from the language file for a compass with the component.
2. **"Minecraft lodestone compasses point at their lodestone"**
   - `dials.rs`: `leaf` evaluates `minecraft:condition` with `property: minecraft:has_component` against the stack being keyed; `compass()` accepts the `lodestone` target from that stack; `stack_key(stack)` returns a virtual id (`minecraft:compass@lodestone:<dimension>:<x>,<y>,<z>`, or `...@lodestone:none` with no target) whose needle is stepped each tick like `DIAL_ITEMS` for every key seen in the last frame; `key()` resolves it to `minecraft:compass#minecraft:item/compass_NN`.
   - `item_tint::of_stack` (or a wrapper) returns that virtual id for a tracked compass so `McStack.icon`, `icon_source`, the first-person hand (`minecraft_hand.rs:195`) and held items in the world (`mesh.rs:1006`) all use the per-stack key. Track B's per-stack `McStack.icon` already exists; no second field.
3. **"Friends' compasses and clocks turn like the host's"**
   - The friend steps its dials before publishing (`minecraft_remote.rs:1330`) with a `DialContext` from its own dimension, feet, yaw and sun angle.
   - `ToClient::SpawnPosition { dimension, pos }` sent on login and whenever the world spawn changes (`/setworldspawn`); `PlayerView.last_death: Option<(Dimension, [i32; 3])>`. Codec indices appended; protocol rule above.

### Tests
- `lodestone::tests`: a single compass tracks in place; a stack of 3 leaves 2 plain and adds 1 tracked; a full inventory drops it; creative keeps all three and adds one; relinking moves the target; a compass on stone does nothing; `forget` clears the target, keeps `tracked`, the name and the glint.
- `server::tests::a_broken_lodestone_is_reported_only_when_its_chunk_is_loaded` (placed, then broken: reported; a target in an unloaded chunk: not reported; another dimension's target is never sent).
- `dials::tests`: a lodestone compass points at its lodestone (facing it: the straight-ahead frame), spins in another dimension and without a target; its key differs from the plain compass and resolves to a `compass_NN` model; a plain compass ignores the branch.
- `player_nbt::tests`: a lodestone compass survives save and load (pos as an int array and as a list).
- `render_anim`: named "Lodestone Compass" and glinting; a friend's published icons change after stepping.
- `minecraft_net` codec round trip of `SpawnPosition` and `PlayerView.last_death`.

### Owner's steps
Survival, Overworld, by day.
1. Place a lodestone, right-click it with the 8 compasses: a click, one glinting "Lodestone Compass" appears, 7 plain ones stay. Walk 30 blocks away and turn around: its needle points back at the lodestone in the hotbar and in your hand. **316_lodestone_compass**.
2. Put 7 compasses away, hold the single lodestone compass and right-click a second lodestone: it changes in place and now points there.
3. Build a portal, go to the Nether, link a plain compass to a lodestone there and walk away: it points. The Overworld one spins. **317_nether_lodestone**.
4. Back in the Overworld, hold the first lodestone compass and break its lodestone: the needle starts spinning; the name and glint stay.
5. Quit and restart: the remaining targets are kept.
6. Friend window: the friend links their own compass to a lodestone and sees it point; the friend's plain compass points to the world spawn and the friend's clock turns with the sun (both did not move before). **318_friend_compass** (friend window).

```
IW4L_MINECRAFT_KIT=minecraft:lodestone*4,minecraft:compass*8,minecraft:clock,minecraft:recovery_compass,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

---

## D16. Breezes (groups `breezes`, `breeze-render`; need D0)

### The owner's finding
"Trial chambers and trial spawners": breezes are the chambers' own mob (row 52 of the mob list). Breeze trial spawners stay inactive until breezes exist (D12 decision).

### Cause (current code)
- No breeze anywhere: not in `SIMULATED_KINDS` (`server_mobs.rs:433`), no store, no renderer; `breeze_spawn_egg` hatches nothing (`HATCHING_EGGS`, `server.rs:7717`, 50 kinds).
- Wind charges exist for players: `thrown.rs` `Throwable::WindCharge` (`:42`, `:54`), bursts in `wind.rs` with `Charge::Breeze` already defined (radius, knockback, burst sound, blocks only with `mob_griefing`, `wind.rs:18-48`) and `ServerSim::wind_burst` (`server.rs:1889`).
- The keyframe reader can read `BreezeAnimation` (round 9 test `every_animation_class_in_the_jar_reads`), but no renderer holds an `AnimationLibrary` yet (`crates/minecraft_terrain/src/keyframes.rs:761`; the jar from `crates/render_anim/src/minecraft_world.rs:360` `data_jar`).
- How a mob family plugs in: **through the D0 registry (`docs/round10/track-d0.md`, plug-in checklist)**: the family's own files plus one line per registry. Read the checklist before the first breeze commit and follow it exactly; it replaces round 9's chain of about 55 sites. The hit-key byte comes from D0's allocation (round 9 ended with 35 for withers, `entities.rs:2071`).

### Vanilla 26.3 (javap, data in the jar)
- **Type** (`EntityTypes.BREEZE`): monster, `sized(0.6, 1.77)`, eye height 1.3452, tracking range 10, not in peaceful. No natural spawn: it comes from trial spawners, eggs and `/summon`. Tags: `#fall_damage_immune` (it plays `entity.breeze.land` after a fall above 3 and takes no damage), `#deflects_projectiles`.
- **Attributes** (`Breeze.createAttributes`): movement speed 0.63, max health 30, follow range 24, attack damage 3. Loot `entities/breeze`: 1-2 breeze rods (looting +1-2) when killed by a player.
- **Behaviour** (`Breeze`, `BreezeAi`, `Shoot`, `LongJump`, `Slide`, `ShootWhenStuck`, `BreezeUtil`; javap each for the brain wiring): `canAttack` only players and iron golems (then `Monster.canAttack`); `isInvulnerableTo` any damage whose causing entity is a breeze; `deflection` is `NONE` for `wind_charge` and `breeze_wind_charge` and the reflecting deflection for everything else (the breeze is in `#deflects_projectiles`; `ProjectileDeflection.REVERSE` with `entity.breeze.deflect`); jump circles `JUMP_CIRCLE_INNER_RADIUS` 4, `MIDDLE` 8, `OUTER` 24 (`BreezeAi`); `withinInnerCircleRange(pos)` is `Vec3.closerThan(atCenterOf(blockPosition), 4.0, 10.0)`: within 4 blocks horizontally and 10 vertically (javap); `Shoot`: range up to 16 blocks (256 squared), a `BreezeWindCharge` from `getFiringYPosition` (feet + height / 2 + 0.3) at power 0.7 and uncertainty `5 − difficulty × 4` (`UNCERTAINTY_BASE` 5, `UNCERTAINTY_MULTIPLIER` 4), with its inhale and shoot timings; `LongJump`: needs 4 air blocks above, cooldown 10 ticks (2 when hurt), the allowed angles and the 1.4 maximum jump velocity scaled 0.058333334 by distance; `Slide`: speed multiplier 0.6 towards a spot 4 to 8 blocks away; `ShootWhenStuck`: after 60 ticks stuck. `BreezeWindCharge` (its own type `minecraft:breeze_wind_charge`, misc, no loot table): radius 3, size 0.3125, tracking range 4, update interval 10 (javap `EntityTypes`); its burst triggers blocks only with `mob_griefing` (f142f9c1). It saves and loads under its own id, with `Owner`.
- **Model and renderer** (`BreezeModel`, `BreezeRenderer`, `BreezeWindLayer`, `BreezeEyesLayer`): parts `body`, `head`, `eyes`, `rods` with three `rod_` children, `wind_body`, `wind_bottom`, `wind_mid`, `wind_top` (wind speeds 1.0, 0.8, 0.6); textures `entity/breeze/breeze.png`, `breeze_wind.png` (scrolled by `ageInTicks × 0.02`) and `breeze_eyes.png` (emissive); shadow 0.5; keyframes `BreezeAnimation` `IDLE`, `SHOOT`, `JUMP`, `INHALE`, `SLIDE`, `SLIDE_BACK` on the animation states `idle`, `shoot`, `longJump`, `inhale`, `slide`, `slideBack`. Particles: idle, slide (20), jump trail (3 for 5 ticks); whirl sound every 1 to 80 ticks.

### Plan
**Group `breezes`** (after D0 is merged into `track-d`)
1. **"Minecraft breezes live in the entity world and shoot wind charges"**
   - The family's files by the D0 checklist, for example vendored `entities/src/breeze.rs` (size, attributes, saved fields, hurt rules, `canAttack`, invulnerability, fall immunity, experience) and `entities/src/breeze_brain.rs` (the brain on the existing `brain_behaviors.rs` framework: Shoot, LongJump, Slide, ShootWhenStuck and the sensor), plus the registry lines D0 lists (save and load, egg, `/summon`, `tracking_range` 10, the friend's look tag, the census, the hit key).
   - The breeze's charge: the thrown wind charge (`thrown.rs`) with a mob shooter and `Charge::Breeze`, the vanilla power and uncertainty; it bursts like the player's, and is saved and loaded as `minecraft:breeze_wind_charge` with its `Owner` (the D11 lazy owner rule).
   - `HATCHING_EGGS` grows by `breeze`; the breeze row in the mob list.
2. **"Minecraft breezes jump, slide and turn arrows back like vanilla"**
   - Projectile deflection for arrows, tridents, snowballs and other thrown things (not wind charges) with the sound; the jump and slide movement; the whirl, idle, land, inhale and shoot sounds for host and friends.
   - Trial chambers' breeze spawners now run (D12's simulated check passes).

**Group `breeze-render`**
1. **"Minecraft breezes are drawn with their wind and eyes"**
   - New `crates/minecraft_terrain/src/breeze_render.rs` from `BreezeModel.createBodyLayer` and `setupAnim` on `posed_cube`, the wind layer in the translucent pass scrolled like the wither's armour (`wither_render.rs` `swirl_shift`), the eyes layer at full light, shadow 0.5, textures in `mesh.rs`, the D0 render registry line, spawner and trial spawner cages.
   - Keyframes from `BreezeAnimation` through one `AnimationLibrary::new(data_jar())` held by the client for host and friend; if W's `warden-render` (or an M track) already created the holder on `minecraft-survival`, reuse it. `MobShow` (or D0's per-family show) carries the animation start ticks; the particles drawn by each viewer.

### Tests
- The D0 family tests (size, health, kind, census, loot, save and load round trip, the friend's look tag, the hit key, the egg), plus a `minecraft:breeze_wind_charge` in flight saved and loaded with its owner.
- `breeze::tests`: invulnerable to another breeze's charge; no fall damage; attacks players and iron golems only; deflects an arrow back with the sound but not a wind charge.
- `breeze_brain::tests`: shoots within 16 blocks with the difficulty's uncertainty; long-jumps only with 4 air blocks above; slides 4 to 8 blocks.
- `server::tests::a_breeze_charge_opens_a_door_only_with_mob_griefing` (with a real breeze shooting); a breeze trial spawner runs its 2-breeze trial.
- `breeze_render::tests`: part count, the three textures, the wind layer scroll, the idle keyframe pose from the jar (gated).

### Owner's steps
Survival, Normal, in a small cobblestone room with an oak door and a stone button inside, by day.
1. Use a breeze egg: a pale, whirling breeze with glowing eyes and three spinning rods hops around. **319_breeze**.
2. Stand 10 blocks away with the door and the button between you: it inhales and shoots wind charges at you. They always knock you back. With `/gamerule mob_griefing true` (the default) a charge bursting next to the door opens it and one next to the button presses it; after `/gamerule mob_griefing false` they leave both alone. `/gamerule mob_griefing true`. **320_breeze_shoots**.
3. Shoot it with the bow: the arrows bounce back at you with a whoosh. A thrown wind charge does hit it.
4. It long-jumps over you and slides away to keep its distance; it lands without hurting itself.
5. Kill it with the sword: 1-2 breeze rods. Two breezes never hurt each other.
6. In a trial chamber's breeze room the spawner now runs: 2 breezes for one player, one at a time. **321_breeze_spawner**. (The kit's trial spawner with a breeze egg runs vanilla's default trial instead: 6 breezes, 2 at once.)
7. With a breeze alive, save and quit, then load: the breeze is still there with the health it had, and a charge that was in the air still bursts.
8. Friend window: the friend sees the breeze's wind, eyes and jumps, and its charges push the friend. **322_friend_breeze** (both windows).

```
IW4L_MINECRAFT_KIT=minecraft:breeze_spawn_egg*8,minecraft:wind_charge*16,minecraft:trial_spawner,minecraft:iron_golem_spawn_egg*2,minecraft:bow+power=5,minecraft:arrow*64,minecraft:trident,minecraft:snowball*16,minecraft:shield,minecraft:diamond_sword+sharpness=5,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:oak_door*2,minecraft:stone_button*2,minecraft:cobblestone*64,minecraft:torch*32
```

---

## Combined kit
One line for the whole track's main steps (27 stacks; checked against the 26.3 item list). These steps need their section's own kit instead: D11 (the wither), D12 step 7 (slime egg, mob spawner), D13 step 1 (the level II bottle), D14 step 6 (a second vault), D15 steps 1-3 (8 compasses, obsidian, flint and steel), D16 step 2 (door and button).

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:trial_spawner*3,minecraft:zombie_spawn_egg*4,minecraft:husk_spawn_egg*2,minecraft:breeze_spawn_egg*4,minecraft:ominous_bottle*2,minecraft:milk_bucket,minecraft:trial_key*4,minecraft:ominous_trial_key*2,minecraft:vault,minecraft:lodestone*2,minecraft:compass*4,minecraft:clock,minecraft:wind_charge*16,minecraft:cobblestone*64,minecraft:ladder*32
```

## Commit groups
| # | Group | Commits | Needs |
|---|---|---|---|
| 1 | `d5-review` | D11: 1-2 | nothing |
| 2 | `trial-spawners` | D12 group 1: 2 | nothing |
| 3 | `trial-spawners-run` | D12 group 2: 4 (spawn rules, visual clip, the trial, its events) | 2 |
| 4 | `trial-ominous` | D13: 2 | 3 |
| 5 | `vaults` | D14: 2 | 2 (the display filter) and 3 (`spawn_dispensed_item`, `tick_trial_spawners` to run after) |
| 6 | `lodestones` | D15: 3 | nothing |
| 7 | `breezes` | D16: 2 | D0 merged into `track-d` |
| 8 | `breeze-render` | D16: 1 | 7 |

## Rules
One commit per fix or feature, each with its tests. Clippy on the touched crates, `rustfmt --check` on new files only, never on existing ones, and no code comments. Cargo only through `scripts\cargo-slot.ps1`, gentle mode when the owner plays. Every commit writes its vanilla numbers, decisions, the owner's steps and its kit line in `docs/round10/track-d-log.md`.
