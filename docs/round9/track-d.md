# Track D: finish survival

The owner's words for this track (round 9 brief, `docs/PROGRESS.md`):

> Track D, finish survival: the 52 missing mobs (survival order first: villagers, trading and villages, pillagers and raids, the warden, foxes...), trial chambers and trial spawners, lodestones and compasses, the friend limits (skating, first-person hand, music, Minecraft melee between players, portal overlay), and the open not-vanilla items (ridden strider under `/tick freeze`, TNT lit by another player with pvp off).

None of the 12 screenshots in `scrennshots/screenshots-03e0c9b/screenshots-03e0c9b/` is about track D. They cover the Nether, floating items, the dragon, end crystals, potion colours, the red overlay and guns in a chest (tracks A, B and C). Screenshot 4 (`4_items_schweben_friend.png`) shows track A's floating held item on another player's body, which is not D9's first-person hand. So every item below starts from the owner's words and the code, not from a screenshot.

Track D has ten parts:

- **D1:** a live bug: drowned vanish after a reload and friends never see them. Fix first.
- **D2:** how a mob is added, the shared prerequisites, and the list of all 52 missing mobs in the order a survival player meets them.
- **D3:** villages: trading (screen, host and friends), zombie villagers and curing, omens and effect icons, cats, the horse family and riding, wandering traders, built golems.
- **D4:** pillagers and raids.
- **D5:** the deep dark: darkness, vibrations, sculk, the warden.
- **D6:** the other mob batches: overworld animals, water, night and caves, late and built mobs.
- **D7:** trial chambers, trial spawners, vaults and breezes.
- **D8:** lodestones and compasses.
- **D9:** the friend limits: first-person hand and swing, portal overlay, music, Minecraft melee between players, skating.
- **D10:** the not-vanilla items: a ridden strider under `/tick freeze`, TNT and pvp, and a few cheap leftovers.

Paths and line numbers are as of f1dddf78 (53260221 only added docs), so re-check them before editing. Vanilla facts were checked with javap on the 26.3 `client.jar` and against the 26.3 data pack, unless marked "not verified". No Mojang code or data is copied: numbers and behaviour only.

Every section has the same parts: the owner's finding, the cause, what vanilla does, the plan, the tests, and the owner's in-game steps with a kit line. The commit subjects are the ones in each plan.

**Multiplayer rule for every item:** the host owns every mob, every level and every rule. All new state is per player and keyed by UUID (saved in `players/data/<uuid>.dat`) or per level (saved in that dimension's folder). Nothing keys on player 0. A friend runs no AI: it mirrors mobs from `MobAdd` look tags and per-tick `ServerMob` moves with `MobShow` (`crates/minecraft_survival/src/remote.rs:41`, `:55`). So every new mob must load from its look tag on the friend and show every drawn state through `MobShow`.

**Size and order.** This is the largest track of the round. The groups follow the owner's survival order: villages, then pillagers and raids, then the warden, then the common animals, water, night and late mobs, then trial chambers and lodestones, then the friend limits and the not-vanilla items. The orchestrator may run the track in segments (several track-runner runs over consecutive groups), with a review at the end of each, and may move the D9 and D10 groups earlier if the owner wants them in the first release; they do not depend on the mob groups.

---

## D1. Drowned vanish after a reload, and friends never see them

### The owner's finding
Not reported by the owner. Found while surveying the 52 missing mobs. It is a live bug in release 2: a zombie that drowns turns into a drowned, which disappears after save and reload, is never seen by a friend, and the drowned spawn egg does nothing.

### Cause
- A zombie that drowns converts in the tick: `third_party/minecraftoss/entities/src/world.rs:5433` `convert_drowning_zombie` → `:3442` `convert_zombie_to_drowned`.
- The drowned is drawn (`crates/minecraft_terrain/src/zombie_render.rs:79-80`, `:110-115`) and saved as `minecraft:drowned`, because `mob_tags_where` writes `zombie.kind.type_id()` (`crates/minecraft_terrain/src/server_mobs.rs:1293`).
- But `spawn_saved`'s zombie arm (`server_mobs.rs:714`) lists only zombie, husk, zombie_villager and zombified_piglin, and `minecraft:drowned` is not in `SIMULATED_KINDS` (`server_mobs.rs:409`). `spawn_saved` returns `None` (`:1122`, `_ => return None`). So:
  1. after a reload every drowned goes into `ServerSim::dormant` (`crates/minecraft_terrain/src/server.rs:61`, `:695`, `:774`): saved and counted, never ticked, drawn or hit;
  2. the friend never sees one: `mob_tracking::spawn_tracked` (`mob_tracking.rs:542`) calls the same `spawn_saved`;
  3. `ServerSim::hatch_egg` (`server.rs:925`) answers "drowned is not simulated" and keeps the egg.
- `natural_spawner.rs` `type_info` has no `drowned`, so natural drowned never spawn (`spawning.rs:496` `note_unsupported`).
- The fork has only `DrownedAttackGoal` (`world.rs:1511`), the swim control, amphibious navigation (`navigation.rs:206`, `:532`) and the conversion.

### Vanilla 26.3
- `Drowned` adds `DrownedGoToWaterGoal` (by day), `DrownedGoToBeachGoal` (at night), `DrownedSwimUpGoal`, and `DrownedTridentAttackGoal`, a `RangedAttackMob.performRangedAttack` that throws a trident.
- Spawning: `checkDrownedSpawnRules` and `isDeepEnoughToSpawn`, river and ocean biome checks with their own rolls (javap the numbers). Drowned spawn in oceans, rivers and dripstone caves.
- Equipment (`populateDefaultEquipmentSlots`): a trident or a fishing rod by chance, plus `NAUTILUS_SHELL_CHANCE` for a nautilus shell in the offhand.

### Plan
1. **"Minecraft drowned are saved, loaded and seen by friends"**
   - `server_mobs.rs`: add `"minecraft:drowned"` to `SIMULATED_KINDS`, and `"minecraft:drowned" => ZombieKind::Drowned` to the zombie arm of `spawn_saved` (`:714`).
   - Check that `mob_tags_where` keeps the drowned's hand items like a zombie's (trace the equipment write; the research did not).
   - `mob_tracking.rs` `tracking_range` already lists drowned at 8 (`:259`).
   - Grow `HATCHING_EGGS` (`server.rs:6358`) to 37 with `drowned`.
2. **"Minecraft drowned spawn in rivers and oceans with tridents and nautilus shells"**
   - `third_party/minecraftoss/world/src/natural_spawner.rs` `type_info`: `drowned` with `Placement::InWater` and a new `Rules::Drowned` (javap `checkDrownedSpawnRules`, `isDeepEnoughToSpawn`); extend `walk` and `remove_far` like monsters.
   - `natural_spawner/spawning.rs` finalize: the equipment roll (javap `populateDefaultEquipmentSlots`).
3. **"Minecraft drowned go to water by day, to the beach at night and throw tridents"**
   - The four goals next to the existing drowned code (`world.rs:1434-1720`). The trident throw goes through the existing thrown trident (`crates/minecraft_terrain/src/trident.rs`, `thrown.rs`).
   - A `MobShow` flag for the raised-trident arm pose, so the friend draws it too.

### Tests
- `server_mobs::tests::a_drowned_saves_and_loads_as_a_drowned`: spawn a zombie, force `convert_zombie_to_drowned`, save with `mob_tags_where`, load with `spawn_saved` into a new world: `ZombieKind::Drowned` with the same health and hand item. It returns `None` before the fix.
- `mob_tracking::tests::a_friend_sees_a_drowned`: `spawn_tracked` on its `look(tag)` returns true.
- `server::tests::every_spawn_egg_of_a_mob_the_game_has_hatches_that_mob` with `drowned` in `HATCHING_EGGS`.
- A `natural_spawner` test: a drowned spawns in a river column at night and never on dry land.
- `world::tests::a_drowned_with_a_trident_throws_it_from_range` and `a_drowned_heads_for_water_by_day_and_the_beach_by_night`.

### Owner's steps (idea)
1. At a river edge use the zombie egg on the shallow floor so the zombie is under water. After about 45 s it shakes and turns into a drowned. Screenshot.
2. Save and quit, load again: the drowned is still there and moves.
3. Use the drowned egg on the water floor a few times: a drowned each time, and the egg is used. Some hold a trident or a nautilus shell. From the shore 10 blocks away a trident drowned throws at you; block it with the shield. Screenshot.
4. By day drowned stay in the water. `/time set night`: they walk out onto the beach.
5. Friend window: the friend sees the same drowned and can hit them.

```
IW4L_MINECRAFT_KIT=minecraft:drowned_spawn_egg*8,minecraft:zombie_spawn_egg*4,minecraft:shield,minecraft:iron_sword,minecraft:water_bucket,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

---

## D2. How a mob is added, and the 52 missing mobs

### The owner's finding
"The 52 missing mobs, survival order first." Their spawn eggs do nothing, and the kinds that do spawn naturally sit dormant.

### Cause
- `spawn_saved` (`server_mobs.rs:530`) matches the id against the kinds that have an `EntityWorld` store and returns `None` for everything else (`:1122`). `SIMULATED_KINDS` (`:409`) lists the 36 mob kinds plus end crystal and item frames. The survey test `every_spawn_egg_of_a_mob_the_game_has_hatches_that_mob` (`server.rs:6364`) pins the 36 eggs in `HATCHING_EGGS` (`:6358`).
- There is no shared mob abstraction: each kind is a separate `Vec<XEntity>` in `EntityWorld` (`third_party/minecraftoss/entities/src/world.rs:2299-2338`). One new kind touches about 55 match or chain sites (round 8's hoglin commit 8f545881 touched 15 files).
- Kinds with a `type_info` entry but no simulation already spawn dormant in the owner's worlds: llama, panda, rabbit, fox, goat, frog, parrot, polar bear, turtle, camel, camel husk, armadillo, axolotl, glow squid and zombie-horse jockeys. Adding them makes those mobs appear at once.
- Kinds with no `type_info` never spawn (`note_unsupported`, `spawning.rs:496`): squid, fish, dolphin, drowned, pillagers at outposts, guardians, nautilus, sulfur cube.

### Vanilla 26.3
- Every one of the 52 has its own entity class (about 16 use `Brain`: allay, armadillo, axolotl, breeze, camel, camel husk, copper golem, creaking, frog, tadpole, goat, happy ghast, nautilus, zombie nautilus, sniffer, warden; the rest use goals), a model class in `net.minecraft.client.model.*`, textures under `textures/entity/<folder>` (some share: mule, skeleton horse and zombie horse under `horse/`, ocelot under `cat/`, vex under `illager/`, camel husk under `camel/`, zombie nautilus under `nautilus/`) and a loot table (93 under `loot_table/entities`).
- Keyframe animations in `client/animation/definitions` for armadillo, baby axolotl, rabbit, breeze, camel, copper golem, creaking, baby fox, frog, nautilus, sniffer and warden.
- Sources with no spawn-list entry: `PhantomSpawner`, `PatrolSpawner`, `WanderingTraderSpawner`, `CatSpawner`, mule breeding, the skeleton-horse lightning trap, endermites from pearls, built golems and the wither, vexes from evokers, ravagers from raids, tadpoles from frogspawn, sniffers from eggs, happy ghasts from dried ghasts, breezes from trial spawners, creakings from creaking hearts, wardens from shriekers, bees from nests and hives, camel husks and zombie nautiluses as jockey mounts.
- Exact attributes, sizes and goal priorities were not extracted per mob. **Each group javaps the entity class, `EntityTypes`, the model and the renderer first** and records the numbers in the track log, as `docs/round8/b2-hoglins.md` did.

### The mob pattern (round 8: piglins, hoglins, striders, shulkers, cave spiders)
Each kind or family goes in as a chain of commits:

1. **"X live in the entity world"** (hoglin 8f545881, shulker 116887f1)
   - `third_party/minecraftoss/entities/src/<mob>.rs` for state, attributes, saved fields, hurt and experience; AI in `<mob>_ai.rs`, or `<mob>_brain.rs` on `brain_behaviors.rs`.
   - `world/<mobs>.rs` with `XEntity`, `spawn_x`, `xs()`, `x_mut`.
   - `world.rs`: the field, `EntityKey` (`:1893`) and `MobHit` (`:1955`) variants, and the arms in `body`/`body_mut`, `damage`/`damage_mut`, `positions`, `bodies`, `mob_candidates` (`:3615`), `entity_type` (`:4158`), `census` (`:4974`), `check_despawn` (`:5027`), `resolve_voices` (`:714`), the death and loot list, the burning list, `mob_hits_mob`, the projectile hit and the chunk filter in `clone_where`.
   - `world/combat.rs` (knockback), `witches.rs` (effects), `pushing.rs`, `hazards.rs` (`exposed!` for fire and drowning), `riding.rs` (prune); `monster_ai.rs` `is_enemy` (`:332`) when iron golems hunt it.
2. **"X are saved and spawn"** (61ad7269)
   - `server_mobs.rs`: `SIMULATED_KINDS`, a `spawn_saved` arm, a `mob_tags_where` arm (`:1259`).
   - `natural_spawner.rs` `type_info` (size, placement, rules; without it natural spawning skips the kind and a spawner allows it anywhere), `spawning.rs` per-kind finalize (natural spawns and `/summon`), `generator/src/feature/entities.rs` `finalize_spawn` (`:111`) for structure mobs.
3. **"X are drawn like vanilla"** (2c1b64c3, c9fa0c37)
   - A new `crates/minecraft_terrain/src/<mob>_render.rs`, ported from the model's `createBodyLayer` and `setupAnim` with `posed_cube::{Cube, Skin, append, part_pose}`.
   - Textures in `mesh.rs:478+`, the pose push in `client_mobs.rs` (~`:1150-1240`), the shadow at `:1267+`, the render call in `crates/minecraft_survival/src/entity_view.rs` `meshes` (`:645+`), `boxes` (`:471`).
   - `crates/minecraft_survival/src/entities.rs` `boxes` (`:1470`) and `encode`/`decode` (`:1670-1735`) with a new hit-key byte. Bytes 0-28 are taken; track D takes 29 upward in order and records each in `docs/round9/mobs.md`.
4. **Behaviour commits**, one vanilla feature each; right-click goes in `crates/minecraft_terrain/src/mob_actions.rs`.
5. **Multiplayer:** `mob_tracking.rs` `tracking_range` (`:252`; vanilla `clientTrackingRange`: animals 10, monsters 8), `MobShow` fields (`:55`) for every synced value the model draws, `shows` (`:295`) and `show_on` (`:442`). `VOLATILE` (`:15`) strips Age, Health and Brain and `look()` adds `Baby`, so baby state comes from `ServerMob.baby`. Also `spawner_display.rs` (`:29`, `:350`) and the egg survey list.
6. **Docs:** the row in `docs/round9/mobs.md` and the track log.

**Batching rule:** one `EntityWorld` store per vanilla family, never one per kind, so each family pays the wiring once. Existing families: `ZombieKind` (drowned already in it), `SkeletonKind`, `HoglinKind`, `Spider::cave`, and horses and donkeys as `CowEntity` with `HorseState` (`world.rs:4515`, `horse.rs:11`). New ones:
- `HorseKind` += Mule, SkeletonHorse, ZombieHorse, Llama, TraderLlama, Camel, CamelHusk;
- `Feline { Cat, Ocelot }`; `Illager { Pillager, Vindicator, Evoker }`; `Squid { Squid, GlowSquid }`; `Fish { Cod, Salmon, TropicalFish, Pufferfish }`; `Guardian { Guardian, Elder }`; `Nautilus { Nautilus, ZombieNautilus }`; `Golem` += snow golem and copper golem.

### Shared prerequisites (each built once, in the first group that needs it)
- **P1. Keyframe animations** (group `keyframes`, before the warden's model). Move the linear sampler out of `bat_render.rs:36-135` into a new `crates/minecraft_terrain/src/keyframes.rs` with `LINEAR` and `CATMULLROM` and an `AnimationState` start tick. Read the `AnimationDefinition`s from the jar at runtime with `crates/minecraft_terrain/src/class_file.rs` instead of transcribing them (the warden alone is about 470 KB of javap). Fall back to hand transcription only if the reader cannot work, and record why.
- **P2. Water mobs** (group `squids-water`): `WaterBoundPathNavigation`, `SmoothSwimmingMoveControl`, fish flopping on land, air supply for water breathers, and buckets (`Bucketable`, the `*_bucket` items carrying `bucket_entity_data`).
- **P3. Flying mobs** (group `parrots`): `FlyingPathNavigation` and `FlyingMoveControl`, reused by bees, allays and happy ghasts.
- **P4. Riding** (group `riding`): generalise `carries()` (`server_mobs.rs:451`) and `riding.rs` beyond striders so jockeys load (zombie horse, camel husk, zombie nautilus, ravager riders); player riding for the horse family (taming, saddle, steering on the strider steering, the horse inventory screen); two seats on a camel later.
- **P5. Player effects** (group `effects-omens`): darkness, bad omen, raid omen, trial omen, hero of the village (later dolphin's grace), with HUD icons.
- **P6. Custom spawners on the host's level tick** (first in group `cats`): `CatSpawner`, then `WanderingTraderSpawner`, `PatrolSpawner`, `PhantomSpawner`. State that vanilla saves goes where vanilla saves it.
- **P7. Game events and vibrations** (groups `vibrations`, `vibrations-more`).
- **P8. Natural lightning** in thunderstorms (`ServerLevel.tickThunder`; only `summon_lightning` exists, `world/src/level/entity.rs:608`) (group `undead-horses`).
- **P9. The trading screen** (groups `trading-*`).
- **P10. Raids** (groups `raids`, `raid-villagers`).

### The 52 mobs in survival order
"Exists" lists only what goes beyond the default (texture, loot table, sounds, an unported model class and a `/summon` tag exist for all). Sizes: S, M, L, XL.

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Group |
|---|---|---|---|---|---|---|
| 1 | Drowned | oceans, rivers, drowning zombies | `ZombieKind::Drowned`, conversion, renderer, attack goal | D1 | M | `mobs-list-drowned-load`, `drowned` |
| 2 | Cat | villages, swamp huts | generator variant (`feature/entities.rs:150`), villagers look at cats | `Feline` store, taming, sitting, gifts, creepers and phantoms flee, collar model, `CatSpawner` | M | `cats` |
| 3 | Ocelot | jungles | shares `Feline` | trust by fish, scares creepers | S | `cats` |
| 4 | Mule | horse × donkey | `DonkeyModel` scaled in `horse_render.rs` | `HorseKind::Mule`, breeding result | S | `horse-family` |
| 5 | Llama | windswept hills, savanna plateau (dormant today) | horse family | variants, carpets, chest, caravans, spit, `LlamaModel` | M | `horse-family` |
| 6 | Trader llama | with wandering traders | horse family | `TraderLlama`, follows its trader on a lead | S | `horse-family`, `wandering-trader` |
| 7 | Wandering trader | near players | villager model, trading port | goals, own trades, invisibility at night, despawn, spawner | M | `wandering-trader` |
| 8 | Snow golem | built | iron golem build check (`golem_spawn.rs`) | build pattern, snowballs, snow trail, melting, shearing | M | `built-golems` |
| 9 | Pillager | outposts, patrols, raids | piglin crossbow fight and arm poses | `Illager` store, crossbow goal, outpost spawns | M | `pillagers` |
| 10 | Vindicator | mansions, raids, patrols | | iron axe, "Johnny", door breaking in raids | S | `patrols-vindicators` |
| 11 | Evoker | mansions, raids | | spells, fangs, vex summon, wololo, totem drop | M | `evokers-vexes` |
| 12 | Vex | from evokers | | no-clip flight, limited life, `VexModel` | M | `evokers-vexes` |
| 13 | Ravager | raids | | roar, stun on a blocked shield, breaks leaves, illager riders | M | `ravagers` |
| 14 | Warden | deep dark, shriekers | dormant, sleep and fire lists | everything (D5) | XL | `warden-*` |
| 15 | Rabbit | taiga, snowy, desert, meadow, cherry (dormant today) | | jump control, carrot raids, biome variants, killer bunny, keyframes | M | `rabbits` |
| 16 | Fox | taiga, snowy taiga, grove (dormant today) | wolves hunt foxes (`monster_ai.rs:2796`) | sleeping by day, pounce, trust, mouth item, berries, keyframes | L | `foxes` |
| 17 | Goat | peaks, snowy slopes (dormant today) | | brain, ram, long jump, screaming, horns, milking | L | `goats` |
| 18 | Parrot | jungles (dormant today) | | P3, imitation, dancing, cookies, shoulder | L | `parrots` |
| 19 | Panda | jungles (dormant today) | | genes and personalities, bamboo, sneeze, roll | L | `pandas` |
| 20 | Turtle | beaches (dormant today) | zombies target turtle eggs (`monster_ai.rs:2060`) | home beach, eggs hatching, scute | L | `turtles` |
| 21 | Armadillo | savanna, badlands (dormant today) | | brain, roll up, scute and brushing, keyframes | M | `armadillos-polar-bears` |
| 22 | Polar bear | frozen biomes (dormant today) | | defends cubs, standing attack | M | `armadillos-polar-bears` |
| 23 | Camel | deserts, desert villages (dormant today) | | `HorseKind::Camel`, two riders, sit, dash, keyframes | L | `camels` |
| 24 | Camel husk | desert husk jockey (dormant today) | `type_info` entry | monster camel with a husk rider | S | `camels` |
| 25 | Squid | oceans, rivers | | `Squid` store, swim impulse, ink | M | `squids-water` |
| 26 | Glow squid | underground water (dormant today) | `type_info` entry | glow ink, glow | S | `squids-water` |
| 27-30 | Cod, salmon, tropical fish, pufferfish | oceans, rivers | | P2, `Fish` store, buckets, patterns, puffing | L | `fish` |
| 31 | Dolphin | oceans | | breathing, treasure leading, dolphin's grace | L | `dolphins` |
| 32 | Axolotl | lush caves (dormant today) | `type_info` entry | brain, playing dead, variants, bucket | L | `axolotls` |
| 33 | Frog | swamps (dormant today) | | brain, variants, tongue, froglights, keyframes | L | `frogs-tadpoles` |
| 34 | Tadpole | frogspawn | | hatching, bucket, growth | M | `frogs-tadpoles` |
| 35-36 | Guardian, elder guardian | ocean monuments | mining fatigue effect | laser, spikes, elder curse | L | `guardians` |
| 37-38 | Nautilus, zombie nautilus | oceans (26.3) | | brain, riding, dash, drowned rider, keyframes | L | `nautiluses` |
| 39 | Phantom | after 3 sleepless nights | `Player.time_since_rest` | `PhantomSpawner`, swoop, daylight burning | M | `phantoms` |
| 40 | Zombie horse | plains, savanna, snowy plains | jockey built at `spawning.rs:988`, dormant | `HorseKind::ZombieHorse`, rider (P4), rideable when tamed | M | `undead-horses` |
| 41 | Skeleton horse | lightning trap | | trap with P8, swimming | M | `undead-horses` |
| 42 | Silverfish | strongholds, infested blocks | stronghold spawner shows one (`spawner_display.rs:724`) | infested blocks, waking friends, merging into stone | M | `silverfish-endermites` |
| 43 | Endermite | ender pearls | | 5% pearl roll, endermen target, 2400-tick life | S | `silverfish-endermites` |
| 44 | Bee | flower forests, plains, nests | generator places nests with bees (`region.rs:259-263`) | P3, nests and hives, pollination, honey, anger | L | `bees` |
| 45 | Allay | outposts, mansions | | brain, P3, item collecting, note blocks, duplication | L | `allays` |
| 46 | Sniffer | sniffer eggs | | egg hatching, digging seeds, brain, keyframes | M | `sniffers` |
| 47 | Copper golem | built (26.3) | | build, sorting copper chests, oxidation, statue | L | `copper-golems` |
| 48 | Creaking | pale garden | `player_views` from endermen (`world.rs:2324`) | heart link, frozen while looked at, keyframes | L | `creakings` |
| 49 | Sulfur cube | sulfur caves (26.3) | `AbstractCubeMob` like the slime | bucket, shearing, archetypes, fuse | M-L | `sulfur-cubes` |
| 50 | Happy ghast | dried ghast (26.3) | ghast model | ghastling, harness, P3, four riders | L | `happy-ghasts` |
| 51 | Wither | built boss | dragon boss bar | everything | XL | `wither-live`, `wither-fight` |
| 52 | Breeze | trial chambers | wind charge burst (`wind.rs`) | brain, wind charges, `BreezeModel`, keyframes | L | `breezes`, `breeze-render` |

### Plan
The first group writes `docs/round9/mobs.md`: one row per mob above with where it is met, what exists, what is left, its group, its hit-key byte and a status (`left`, `live`, `drawn`, `done`), plus the batching rule and the mob pattern in short. **Every mob group updates its rows in the same commit as the code.** The last group (`mobs-wrapup`) checks that every spawn egg in the game hatches.

### Tests (every family)
- `minecraftoss-entities` `world::tests`: `a_<mob>_is_<size>_with_<health>_and_its_kind` (dimensions, eye height, health, `entity_type`, loot table, census) and one behaviour test per vanilla feature.
- `minecraft_terrain`: `server_mobs::tests::a_<mob>_saves_and_loads_with_<fields>` (round trip through `mob_tags_where` → `spawn_saved`); `mob_tracking::tests::a_friend_sees_a_<mob>_from_its_look_tag` (keeps its `MobShow`); `<mob>_render::tests` (part count, textures, baby scale against the javap'd model); a `type_info` rules test; `HATCHING_EGGS` grows by each kind; `spawn_egg::tests` width check for odd sizes.
- `minecraft_survival`: `entities::tests::a_<mob>_hit_box_encodes_and_decodes`.

### Owner's steps (idea)
Every mob group's log entry has: hatch its egg, see it drawn and animated, hit it, see its main behaviour, save and reload, and the friend sees the same mob in the same pose and can hit it. Batch kits are in D6.

---

## D3. Villages

### The owner's finding
"Villagers, trading and villages" come first in the survival order. Right-clicking a villager does nothing today (round 8 decision: "there is no trading screen yet").

### Cause
**What already exists** (checked in code):
- Villages generate (`generator/src/structure/kinds/mod.rs:31`, test `locate.rs:466-491`), with villagers, one iron golem, animals, cats and rare zombie villages from the templates (`generator/src/feature/entities.rs:113-212`). Cats stay dormant.
- Villager AI is close to complete (`entities/src/villager_brain`): POIs, professions, beds, breeding, farming, doors, gossip, golem summoning. Restocking at workstations (`world/villagers.rs:171-201`), trading gossip (`:528`) and reputation prices (`world/merchants.rs:123-148`) are ported.
- Trading logic is ported and tested on the level thread: offers from `trade_set` (`entities/src/trading.rs`), `MerchantMenu` (`entities/src/merchant.rs`), `villager_interact`, `notify_trade`, `stillValid`, close (`world/merchants.rs:104-281`), the terrain API `MerchantView`, `MerchantOp`, `MerchantUpdate`, `merchant()`, `check_merchant()`, `player_left()` (`crates/minecraft_terrain/src/server.rs:128-180`, `:517-592`), `Command::Merchant` (`:2712`, `:3070`, `:3335`), `Output.merchant` (`:2888`, `:3472`), and a full-trade test (`:4290-4311`). Offers and gossips are saved (`server_mobs.rs:1148-1160`).

**Why trading does not work in game:**
1. `crates/minecraft_terrain/src/mob_actions.rs:305`: `MobHit::Villager(_) => false` in `would_interact`, so `Entities::use_on_mob` (`crates/minecraft_survival/src/entities.rs:770`) never sends a right-click on a villager.
2. `PlayerQueues::route` (`entities.rs:198-298`) reads only `slots`, `broken` and `mounted` from mob results (`:262-268`) and drops `MobResult.merchant` and `Output.merchant`.
3. There is no screen: no `McScreen::Merchant` (`crates/frame/src/minecraft_ui.rs:65-81`), no `MenuKind`/`MenuExtras` (`crates/minecraft_net/src/messages.rs:444-479`), no `Open::Merchant` (`crates/minecraft_survival/src/menus.rs:32-37`), the GUI loader lacks the villager sheet and `container/villager/` sprites (`crates/render_anim/src/minecraft_gui.rs:18-48`), and `MerchantOp` has no shift-click although `merchant_quick_move_inventory` exists (`world/merchants.rs:262`).

**Other gaps:**
- Zombie villagers: no conversion on a kill (`world.rs:5403-5411`), no curing (a golden apple on a zombie falls to `_ => return outcome` in `mob_actions.rs`), no conversion state in `Zombie` (`zombie.rs:83-84`), no level events 1026 and 1027 (`level_events.rs:83-125`).
- Player effects are a fixed enum of 15 (`third_party/minecraftoss/player/src/survival.rs:36-74`, mirrored in `crates/minecraft_survival/src/player_nbt.rs:18-34`, `crates/minecraft_survival/src/commands.rs:150-170`, `crates/minecraft_terrain/src/potions.rs:230-248`). No bad omen, raid omen, trial omen, hero of the village or darkness. `EntityWorld::set_player_hero` (`world/merchants.rs:76`) is never called. The HUD shows no effect icons.
- No leads anywhere; horses and donkeys cannot be ridden (only striders).
- Player-built iron golems are missing ("carved_pumpkin" is in no placement code).

### Vanilla 26.3
- **Trading** (`Villager.mobInteract`, `MerchantMenu`, `MerchantScreen`): an adult with offers opens the menu; a baby or one without offers shakes its head. Prices drop by reputation and by Hero of the Village (0.3 + 0.0625 × amplifier) × the first cost, at least 1. Each trade gives villager XP (levels at 10, 70, 150, 250), a level-up appends offers and gives Regeneration 200 ticks, an orb of 3-6 XP (8 more on a level-up), +2 trading gossip and entity event 14. Reputation events: cured +20 major and +25 minor positive, trade +2, hurt +25 minor negative, killed +25 major negative.
- **The screen:** 276×166 on a 512×256 `villager.png`; 7 offer buttons 88×20 at x 5, y 18 + 20i; scroller 6×27 at x 94, y 18, bar 139 high; XP bar at 136,16 (102 wide); price columns at 5 and 35, result at 68; title `<name> - <merchant.level.N>`, "Trades" on the left; sprites `container/villager/{trade_arrow, trade_arrow_out_of_stock, out_of_stock, discount_strikethrough, experience_bar_*, scroller, scroller_disabled}`. Slot positions (payments 136,37 and 162,37, result 220,37, inventory from 108,84, hotbar 108,142) are from memory: javap `MerchantMenu`.
- **Zombie villagers:** `Zombie.killedEntity` converts a killed villager on hard always and on normal on a `nextBoolean`, keeping VillagerData, gossips, offers and xp, with level event 1026. A golden apple on one with Weakness starts conversion: `nextInt(2401) + 3600` ticks, Weakness removed, Strength added, entity event 16 (cure sound). Each tick progresses by 1 plus iron bars and beds within 4 blocks (javap `getConversionProgress`). On finishing: a villager with the same data, the cured reputation event for the starter, Nausea 200 ticks, level event 1027.
- **Ominous bottle:** drinking gives Bad Omen for 120000 ticks at the `ominous_bottle_amplifier` value (javap `OminousBottleAmplifier.onConsume`, the use time and the sound).
- **Effect icons:** `Gui.renderEffects` draws the effect icons top right (beneficial in one row, harmful in the next), blinking in the last 10 seconds.
- **Cats:** taming with raw cod or salmon (1 in 3), sitting, lying on beds and chests, morning gifts from `gameplay/cat_morning_gift`, creepers and phantoms avoid them; `CatSpawner` spawns village cats near beds and swamp-hut cats (javap the timing).
- **The horse family:** mules from horse × donkey; llamas with variants, carpets, chests (strength 1-5), caravans and spit (`LlamaSpit`); trader llamas leashed to their trader. Player riding: taming by riding (temper), a saddle, steering, jumping (horses), the horse inventory (`HorseInventoryMenu`), chests on donkeys, mules and llamas.
- **Leads:** `LeadItem` leashes a mob to the player and ties it to a fence knot; the lead breaks beyond 10 blocks.
- **Wandering trader:** `WanderingTraderSpawner` checks every 1200 ticks, delay 24000, chance 25 rising by 25 up to 75, 1 in 10 player pick, 10 tries within 48 blocks, `despawnDelay` 48000, two trader llamas leashed to it, saved in `WanderingTraderData`, game rule `spawn_wandering_traders`. Its goals drink invisibility at night and milk at day. Offers from `trade_set/wandering_trader/{buying,common,uncommon}`.
- **Built golems:** `CarvedPumpkinBlock.trySpawnGolem`: a carved pumpkin on an iron T makes an iron golem; on two snow blocks a snow golem (snowballs, snow trail, melting in warm biomes, shearing off the pumpkin).

### Plan
**Group `trading-host`: "Minecraft villagers open their trading screen"** (level-thread menu, because it is vanilla-traced and tested)
- `mob_actions.rs:305`: `would_interact` for a villager returns `world.villager_interact(...) != Pass` on a clone, like the generic branch. Update the test at `:420-443`.
- `server.rs`: add `MerchantOp::QuickMove(usize)` → `merchant_quick_move_inventory`.
- `entities.rs`: `PlayerQueue` gains `merchant: Vec<MerchantUpdate>` and `merchant_waiting`; `route()` pushes `result.merchant` and every `Output.merchant` to its player's queue; apply each update's slots and cursor in the actor loop next to `mob_slots` (`:1401`), before picked items; add `Entities::merchant(player, op, ...)` and `take_merchant(player)` like `take_lecterns` (`:840`).
- `menus.rs`: `Open::Merchant(MerchantScreen)` (villager, offers, profession, level, xp, future xp, payment, result, waiting, `held: VecDeque<MenuAction>`). Container cells 0, 1, 2 are payment A, B and the result; `Choose(i)` → `Select(i)`; a shift-click on a player slot → `QuickMove`. **While an op is in flight, further clicks are held and replayed after the update**, so a reply never overwrites a newer change. Close sends `MerchantOp::Close`; also on a dimension switch, death, `update.closing` and `player_left`.
- A minimal screen so the commit is testable is allowed; group `trading-screen` makes it vanilla.

**Group `trading-screen`: "The Minecraft trading screen looks like vanilla"**
- `McScreen::Merchant`, `MinecraftUi.merchant: Option<McMerchant>` (offers with discounted cost A via `MerchantOffer::cost_a_count`, cost B, result, out of stock; selected; scroll; level; xp; future xp; title; progress bar shown).
- `render_anim/src/minecraft_gui.rs`: add `villager` to `CONTAINER_SHEETS` and `container/villager/` to `SPRITE_FOLDERS`.
- `hud/src/minecraft_screens.rs`: layout, offer buttons, strikethrough discounts, trade arrow or out of stock, scroller with wheel and drag (copy the stonecutter's `start_index` and `drag_offset`, `:680`), XP bar, labels, tooltips; controller pad focus like enchanting and stonecutter (`hud/src/minecraft_inventory.rs:536`).
- `render_anim/src/minecraft_inventory.rs`: `screen_for(Open::Merchant)` (`:580`).

**Group `trading-friend`: "Friends trade with Minecraft villagers"**
- `MenuKind::Merchant` and `MenuExtras::Merchant { villager, profession, level, xp, future_xp, show_progress, offers: Vec<NetOffer> }` (costs and result as Nbt, uses, max uses, special price, demand, price multiplier, reward flag). `menus.rs` snapshot builds them; `sync` already resends `MenuOpen` when extras change (`:1000-1041`); `mirror_open` (`:1233-1283`) builds `Open::Merchant`. The friend's clicks reach the same `menus::apply` on the host.
- `mob_tracking.rs` `VOLATILE`: add `Offers` and `Xp`.
- Codec tags and round-trip tests (`crates/minecraft_net/src/codec.rs:203-252`).

**Group `zombie-villagers`**
1. **"Zombies turn the villagers they kill into zombie villagers"**: at `world.rs:5403-5411`, after the difficulty gate, push `(zombie, villager)` to a new `infected` list. The terrain server builds the new tag from the villager's tag (id `minecraft:zombie_villager`, keeping VillagerData, VillagerDataFinalized, Offers, Gossips, Xp, CustomName, baby, equipment, PersistenceRequired), runs the zombie finalize with the CONVERSION reason, removes the villager, spawns the tag, sends level event 1026. Javap `ConversionType.SINGLE` for the UUID.
2. **"A golden apple cures a weakened Minecraft zombie villager"**: `Zombie` gains `villager_conversion_time` and `conversion_starter`, saved as `ConversionTime` and `ConversionPlayer`; the `mob_actions` arm; the countdown with iron bars and beds; the finish builds a villager tag the same way, with the cure gossip, Nausea 200 ticks, level event 1027. `MobShow.converting` (`mob_tracking.rs:52-62`) shakes it; entity event 16 plays the cure sound for everyone through `heard.rs`. Level-event sounds 1026 and 1027 in `level_events.rs`.

**Group `effects-omens`**
1. **"Minecraft players carry darkness, bad omen, raid omen, trial omen and hero of the village"**: `EffectKind` and `ALL` (20 kinds), `player_nbt.rs` EFFECTS, `commands.rs` `effect_kind`, `potions.rs` `player_effect`, `/effect`. Each tick the main thread puts every player's hero amplifier into `TickInput` beside `mob_players`; the level thread calls `set_player_hero`. Darkness only gets its kind here; its look is group `darkness`. Raid omen turning into a raid is group `raids`.
2. **"Minecraft ominous bottles give bad omen"**: a drinkable consumable with the javap'd time, effect and sound.
3. **"Minecraft effect icons show in the top right like vanilla"**: a new `crates/hud/src/minecraft_effects.rs` drawing `mob_effect/<id>` sprites from the GUI atlas, beneficial and harmful rows, blinking at the end. A friend's effects already arrive in `PlayerView.effects` (`messages.rs:365`, `:395`).

**Group `cats`**
1. **"Minecraft cats and ocelots live in the entity world and are drawn like vanilla"**: the `Feline` store, save and spawn (ocelots in jungles; structure cats through `finalize_spawn`), `AdultCatModel`, `BabyCatModel`, the collar layer, ocelot texture.
2. **"Minecraft cats are tamed with fish, sit, and scare creepers and phantoms; ocelots learn to trust"**: taming, owner UUID, sitting, following, lying on beds and chests, morning gifts, creeper avoidance (phantoms when they exist).
3. **"Minecraft villages and swamp huts get their cats"**: the first custom spawner (P6), a small hook in the host's level tick that later spawners reuse.

**Group `horse-family`**
1. **"Minecraft mules, llamas and trader llamas join the horse family"**: `HorseKind` += Mule, Llama, TraderLlama; the mule as the horse × donkey breeding result; llama variants, carpets, chest strength; spawning (llamas already spawn dormant); `LlamaSpit` projectile.
2. **"Minecraft llamas and trader llamas are drawn like vanilla and spit"**: `LlamaModel`, `LlamaSpitModel`, the carpet and trader layers; caravans.

**Group `riding`** (P4)
1. **"Minecraft players tame and ride horses, donkeys, mules and llamas"**: temper and taming by riding, saddle, steering on the strider steering, the horse jump bar, dismounting; generalise `carries()` and `riding.rs` so mob jockeys of any mount load. Replicates like the strider (`ServerMob.vehicle`, `players_pushed`, rejoin handling d0871eb2).
2. **"The Minecraft horse inventory holds the saddle, armour and chest"**: `HorseInventoryMenu` on the host, a `MenuKind` for friends, the screen.

**Group `wandering-trader`**
1. **"Minecraft leads tie mobs to players and fences"**: the `lead` item, leash knots on fences, the drawn lead, breaking beyond 10 blocks, saved `leash` tags.
2. **"Minecraft wandering traders visit with their own trades and two trader llamas"**: the entity (villager model, `wandering_trader` texture), offers from its trade sets through `TradeBook::offers_from`, despawn delay, invisibility potion at night and milk at day, the merchant screen without a progress bar, `WanderingTraderSpawner` with its saved data and game rule.

**Group `built-golems`**
1. **"Minecraft iron golems can be built from iron blocks and a carved pumpkin"**.
2. **"Minecraft snow golems are built, shoot snowballs and leave snow"**: the snow golem family member, `SnowGolemModel`, melting, shearing.

### Tests
- `mob_actions`: `would_interact` true for an adult villager with offers and for a baby; false for a sleeping one or one trading with another player.
- `server.rs`: extend the trade test at `:4290` with `QuickMove` and `Close` (payments back, overflow dropped at the feet); a level-up adds offers and 8 XP; `check_merchant` closes 8 blocks away.
- `entities`: a merchant update reaches only its player (copy `entities.rs:2332`); slots are applied before picked items.
- `menus`: payment, result, choose and shift-inventory clicks map to ops; held clicks replay in order; close sends Close; snapshot gives `MenuKind::Merchant`; mirror gives `Open::Merchant`.
- `minecraft_net` codec round trip of the merchant kind and extras with a discounted and an out-of-stock offer.
- `hud`: slot positions (after javap), offer row hit test, scroll index with 9 offers, strikethrough and out-of-stock choices.
- Zombie villagers: conversion gated by difficulty; data carried both ways; golden apple without weakness does nothing; conversion time 3600-6000; bars and beds speed it up; cheaper prices for the starter; `ConversionTime` and `ConversionPlayer` survive a reload; `dormant.rs:33-38` still keeps a curing zombie villager.
- Effects: the five kinds round trip through `player_nbt`; the bottle amplifier becomes the effect amplifier; hero reaches `set_player_hero` and lowers prices; the icon layout puts harmful effects in the second row.
- Cats, the horse family, riding, leads, the trader and golems follow the D2 family tests, plus: taming chance, a tamed cat follows its owner UUID, a creeper flees a cat, the spawner's timing; a mule from horse × donkey; a lead breaks at 10 blocks; the trader despawns after its delay and its llamas are leashed to it; the iron and snow golem patterns.

### Owner's steps (idea)
**Trading (host):**
1. `/locate structure minecraft:village_plains`, `/tp` there. Screenshot of villagers, the golem and animals.
2. Right-click a villager with a profession: the trading screen opens with "<Profession> - Novice", offers, two payment slots, the result and the XP bar. Screenshot.
3. Click an offer, take the result, shift-click it until you run out. After about 2 trades it becomes Apprentice with new offers and particles. Screenshot.
4. Use a trade until its arrow is crossed out. Screenshot. Next morning, after it works, the offer is back.
5. Walk 8 blocks away with the screen open: it closes and the payments return.
6. Hit a villager twice: prices rise. A baby villager shakes its head.
7. Friend: repeats 2-5 in the same village. Both click the same villager: only the first gets the screen. Screenshot from the friend.

```
IW4L_MINECRAFT_KIT=minecraft:emerald*64,minecraft:emerald*64,minecraft:wheat*64,minecraft:paper*64,minecraft:stick*64,minecraft:coal*64,minecraft:string*64,minecraft:rotten_flesh*64,minecraft:iron_ingot*32,minecraft:villager_spawn_egg*4,minecraft:composter,minecraft:lectern,minecraft:fletching_table,minecraft:smithing_table,minecraft:red_bed,minecraft:cobblestone*64,minecraft:cooked_beef*32
```

**Zombie villagers and omens:**
1. `/difficulty hard`. A villager and a zombie in a glass box at night: the villager becomes a zombie villager in the same outfit. Screenshot.
2. Splash weakness on it, then a golden apple: it shakes, red-tinted, with the cure sound. With iron bars and a bed next to it, after 2-5 minutes it becomes a villager. Screenshot. Its prices are much lower. Screenshot.
3. Drink an ominous bottle: the Bad Omen icon shows top right. `/effect give @s minecraft:hero_of_the_village 60`: the icon shows and prices drop.

```
IW4L_MINECRAFT_KIT=minecraft:zombie_villager_spawn_egg*4,minecraft:villager_spawn_egg*4,minecraft:zombie_spawn_egg*4,minecraft:splash_potion@weakness,minecraft:splash_potion@weakness,minecraft:golden_apple*16,minecraft:iron_bars*16,minecraft:red_bed,minecraft:glass*64,minecraft:oak_planks*64,minecraft:emerald*64,minecraft:ominous_bottle*2,minecraft:diamond_sword,minecraft:torch*32,minecraft:cooked_beef*32
```

**Cats, the horse family, riding, the trader and golems:**
1. Cat egg on grass. Hold raw cod and stay still, feed it until hearts: collar, sits on right-click, follows. A creeper egg near it: the creeper runs. Screenshot.
2. Horse and donkey eggs, tame both by riding, feed golden carrots: a baby mule. Or the mule egg. Screenshot.
3. Saddle a tamed horse: ride, steer, jump. Open its inventory: saddle and armour slots. A donkey takes a chest.
4. Llama egg: chest and white carpet on it; hit it and it spits.
5. Lead a cow to a fence and tie it.
6. Wandering trader egg: two trader llamas on leads come with it. Right-click it: its trading screen without an XP bar. At night it drinks invisibility. Screenshot.
7. Build an iron golem (4 iron blocks in a T, carved pumpkin on top) and a snow golem (2 snow blocks and a carved pumpkin). The snow golem leaves snow and throws snowballs at a zombie.
8. Friend: sees the same cats (sitting), mules, llamas, the trader; tames their own cat, which follows the friend, not the host. Screenshot of both windows.

```
IW4L_MINECRAFT_KIT=minecraft:cat_spawn_egg*4,minecraft:creeper_spawn_egg*2,minecraft:horse_spawn_egg*2,minecraft:donkey_spawn_egg*2,minecraft:mule_spawn_egg*2,minecraft:llama_spawn_egg*2,minecraft:wandering_trader_spawn_egg*2,minecraft:cod*32,minecraft:golden_carrot*16,minecraft:saddle*2,minecraft:iron_horse_armor,minecraft:chest*2,minecraft:white_carpet*2,minecraft:lead*4,minecraft:oak_fence*8,minecraft:emerald*64,minecraft:iron_block*8,minecraft:snow_block*4,minecraft:carved_pumpkin*4,minecraft:zombie_spawn_egg*2,minecraft:cow_spawn_egg*2,minecraft:shield,minecraft:iron_sword
```

---

## D4. Pillagers and raids

### The owner's finding
"Pillagers and raids" are second in the survival order.

### Cause
- Pillagers, vindicators, evokers, vexes, evoker fangs and ravagers have no entity, AI, renderer or egg support. In saves they stay dormant.
- Pillager outposts generate (jigsaw, legacy reducer at `generator/src/structure/placement.rs:47`), but their pillagers never spawn: the spawn overrides are read (`natural_spawner.rs:476-493`) and `type_info` has no `pillager` (`:267-312`), so it logs "unsupported" (`spawning.rs:497`).
- No `PatrolSpawner`, no raids: the raider goals are stubbed (`monster_ai.rs:181-195`, `:2323-2360`); villager PreRaid, Raid and Hide activities have no packages (`villager_brain/behaviors.rs:20-30`, `ReactToBell` is `Never` at `:122`, `SetRaidStatus` only draws the random at `:512-521`). The game rules `raids`, `spawn_patrols` and `spawn_wandering_traders` exist (`world/src/game_rules.rs:151-163`).
- The witch already exists as a raider (finalize at `spawning.rs:955-961`). Villagers already flee pillagers at 15 blocks (`villager_brain/mod.rs:566`). The piglin crossbow fight and arm poses can be reused (`piglin_brain/fight.rs`, `ArmPose::CrossbowCharge`/`CrossbowHold`, `MobShow.arm_pose`).
- **Loot functions:** `third_party/minecraftoss/core/src/loot.rs:608-698` fails on `set_damage`, `set_potion`, `set_stew_effect`, `set_ominous_bottle_amplifier`, `set_enchantments` and `set_components` ("loot function {other} is not supported", `:697`). Pillager captains, ancient city chests, trial chamber vaults and spawners, desert pyramids and shipwrecks all need some of them (`docs/MINECRAFT-SURVIVAL.md:435`). `entities/src/trading.rs:504`, `:537` already port `set_stew_effect` and `set_potion` for trades.

### Vanilla 26.3
- **Raid omen:** `BadOmenMobEffect` gives a non-spectator player inside a village (`ServerLevel.isVillage`), not on peaceful, RAID_OMEN for 600 ticks with the omen position. When raid omen ends, `Raids.createOrExtendRaid` starts the raid.
- **Waves** (`Raid$RaiderType`, wave index 0-7):

| Raider | Waves 0-7 |
|---|---|
| Vindicator | 0,0,2,0,1,4,2,5 |
| Evoker | 0,0,0,0,0,1,1,2 |
| Pillager | 0,4,3,3,4,4,4,2 |
| Witch | 0,0,0,0,3,0,0,1 |
| Ravager | 0,0,0,1,0,1,0,2 |

- Waves per difficulty (easy 3, normal 5, hard 7, plus a bonus wave at omen level above 1), Hero of the Village on victory (amplifier omen level − 1, about 48000 ticks) and the boss-bar range are from memory: javap `Raid`. Raids save per dimension in `data/raids.dat`.
- **PatrolSpawner:** next check in 12000 + nextInt(1200) ticks; needs a bright day, a 1 in 5 roll and a random non-spectator player not near a village (`isCloseToVillage(pos, 2)`); spawns 24-47 blocks away; group size from `ceil(effective difficulty)`; the first member is the captain with an ominous banner. Javap whether the "day 5" check is still there.
- **Pillager:** `RangedCrossbowAttackGoal`, `IllagerModel` with crossed arms. Killing a captain drops an ominous bottle (javap `Raider.die` and the 26.3 loot).
- **Vindicator:** iron axe; named "Johnny" it attacks everything. **Evoker:** spellcaster states, `EvokerFangs`, vex summons, wololo on blue sheep, totem drop. **Vex:** no-clip charge, limited life. **Ravager:** roar, stun on a blocked attack, breaks leaves with mob griefing, illager riders.
- **Villagers in raids:** they hide at the bell, ring it, celebrate a victory; heroes get gifts from `gameplay/hero_of_the_village/<profession>_gift`.

### Plan
**Group `loot-functions`**
1. **"Minecraft loot sets damage, potions, stew effects, enchantments, components and ominous bottle levels"**: port the six functions in `core/src/loot.rs` next to `enchant_randomly`, reusing `trading.rs`'s logic. Swap the test `unknown_functions_still_fail` to a still-unported function (`exploration_map`). Check the entity loot engine (`entities/src/loot.rs`) for the same functions.
2. **"Minecraft ancient cities generate with sculk shriekers and loot chests"** (test only): in `crates/minecraft_terrain/src/terrain.rs`, modelled on `end_cities_generate...` (`:2597`): find an ancient city in the deep dark, generate its chunks, assert reinforced deepslate, `sculk_shrieker[can_summon=true]`, `sculk_catalyst` and chests with `minecraft:chests/ancient_city`.

**Group `pillagers`**
1. **"Minecraft pillagers live in the entity world and shoot crossbows"**: the `Illager` store, raider and patrolling state, `RangedCrossbowAttackGoal` on the piglin crossbow code.
2. **"Minecraft pillagers are drawn like vanilla"**: `IllagerModel` with the crossbow poses.
3. **"Minecraft pillagers are saved and spawn at pillager outposts"**: `type_info`, finalize, the outpost overrides.

**Group `patrols-vindicators`**
1. **"Minecraft pillager patrols roam with a banner captain"**: `PatrolSpawner` (P6), the captain's ominous banner (components already built at `creative_tabs.rs:1348`), the captain's bottle drop.
2. **"Minecraft vindicators"**: the `Illager` arm with the iron axe and "Johnny".

**Group `evokers-vexes`**
1. **"Minecraft evokers cast fangs and summon vexes"**: spellcasting, the `EvokerFangs` entity and model, wololo, totem drop.
2. **"Minecraft vexes fly through walls and charge"**: the vex with its move control, limited life and `VexModel` with the charging texture.

**Group `ravagers`**
1. **"Minecraft ravagers roar, stun and carry illagers"**: entity, model, roar, stun on a blocked shield, leaf breaking, riders through P4.

**Group `raids`**
1. **"Minecraft raids come in waves with a boss bar and a horn"**: `Raid` per level saved to `data/raids.dat` in the dimension folder; raid omen at a village POI turns into a raid; waves from the table; the boss bar "Raid", "Raid - Victory", "Raid - Defeat" through `McBossBar` and `ToClient::BossBar` to players in range (the dragon's per-player bar is the template: `minecraft_survival/src/the_end/dragon_fight.rs`, `hud/src/minecraft_boss.rs`); `event.raid.horn`; raiders path to the centre; witches join; game rule `raids`.

**Group `raid-villagers`**
1. **"Villagers hide and ring bells in Minecraft raids, and heroes get gifts"**: PreRaid, Raid and Hide packages; `ReactToBell` and bell use (villagers hide, raiders glow); celebrating; Hero of the Village on victory; `GiveGiftToHero`.

### Tests
- `core::loot`: one test per function (`set_damage` 0.5 on an iron sword gives damage 125; `set_ominous_bottle_amplifier` uniform 0-4; `set_enchantments` on an item and a book; `set_components`); a survey test gated on `MINECRAFTOSS_ROOT` that every chest table rolls without an unsupported function except `exploration_map`.
- The ancient city generation test above.
- Illagers and the ravager follow the D2 family tests, plus: a pillager charges for the javap'd time then shoots; a captain carries the banner and drops a bottle; patrols never spawn near a village or at night; a blocked ravager is stunned; fangs damage.
- Raids: the wave table per difficulty, the bonus wave, `raids.dat` round trip, raid omen turning into a raid only inside a village, victory gives hero to every player in the raid, defeat after all villagers die.

### Owner's steps (idea)
1. `/locate structure minecraft:pillager_outpost`, `/tp` there: pillagers guard the tower and shoot crossbows. Screenshot.
2. Pillager, vindicator, evoker and ravager eggs: each fights you; the evoker casts fangs and summons vexes; the ravager roars after you block it with the shield. Screenshots.
3. Kill a patrol captain (banner on its head): it drops an ominous bottle.
4. Drink it, walk into a village: after 30 s the horn and the "Raid" bar. Waves come; villagers hide; ring the bell: raiders glow. Win: "Raid - Victory", Hero of the Village, cheaper trades, villagers throw gifts. Screenshot.
5. Friend: fights the same raid, sees the bar, both become heroes.

```
IW4L_MINECRAFT_KIT=minecraft:ominous_bottle*4,minecraft:bell,minecraft:pillager_spawn_egg*8,minecraft:vindicator_spawn_egg*4,minecraft:evoker_spawn_egg*2,minecraft:ravager_spawn_egg*2,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:crossbow,minecraft:shield,minecraft:totem_of_undying,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:diamond_leggings,minecraft:diamond_boots,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:cobblestone*64,minecraft:emerald*32
```

---

## D5. The deep dark and the warden

### The owner's finding
"The warden" is third in the survival order.

### Cause
**Exists:**
- Deep dark and ancient city generation are data-driven and should work (`generator/src/structure/mod.rs:316-342` loads `ancient_cities`; processors at `feature/template/processor.rs:325-347`; `feature/kinds/sculk.rs` ports `SculkPatchFeature` with shriekers `can_summon=true` at `:89`, `:219`). Not run in game, no test.
- Sculk blocks are plain blocks; their block entities keep only NBT defaults (`core/src/block_entity.rs:277-288`). The `spawn_wardens` rule is stored only (`game_rules.rs:164`). The player file keeps `warden_spawn_tracker` as an unknown key.

**Missing** (grep finds no game event, vibration, sonic or darkness logic):
1. No GameEvent system. 2. No sensor, calibrated sensor, shrieker or catalyst behaviour (`world/src/level/mod.rs:807-862` `tick_block` and `level/container.rs:475` `has_ticker` cover hoppers, pistons and daylight detectors only). 3. No warden entity; its egg does nothing. 4. No darkness: no `EffectKind`, no fog or lightmap term (`crates/render_anim/src/minecraft_world.rs:1620-1624` has only lava and powder snow fog; `crates/render_gpu/src/drawsurf/minecraft_world.rs:949-963` and `minecraft_light.rs:270-292` have no darkness; brightness is pinned at 0.5 in `environment[10].w`, `minecraft_world.rs:1648`). 5. Ancient city chests roll nothing (loot functions, group `loot-functions`). 6. No crouch flag in `PlayerCandidate` (`entities/src/tempt.rs:6-27`), no sonic boom in `PlayerHitKind` (`world.rs:584-600`), no warden pose in `MobShow`, no CATMULLROM keyframes.

### Vanilla 26.3
- **Game events:** default radius 16, `shriek` 32, `jukebox_*` 10. Frequencies: step, swim, flap 1; projectile_land, hit_ground, splash, bounce 2; item_interact_finish, projectile_shoot, instrument_play 3; entity_action, elytra_glide, unequip 4; entity_dismount, equip 5; entity_interact, shear, entity_mount 6; entity_damage 7; drink, eat 8; container_close, block_close, block_deactivate, block_detach 9; container_open, block_open, block_activate, block_attach, prime_fuse, note_block_play 10; block_change 11; block_destroy, fluid_pickup 12; block_place, fluid_place 13; entity_place, lightning_strike, teleport 14; entity_die, explode 15; resonate_N is N. Redstone output `max(1, 15 - floor((15/range) × distance))`.
- **Listener checks**, in order: listenable tag; sneaking (`#ignore_vibrations_sneaking`: hit_ground, projectile_shoot, step, swim, item_interact_start, item_interact_finish); dampening (`#dampens_vibrations`: wool, carpets; a warden; an item entity of such an item); range; occlusion (`#occludes_vibration_signals` = `#wool` on the ray); `canReceiveVibration`. `VibrationSelector` keeps the nearest per tick (ties: higher frequency); the vibration travels `floor(distance)` ticks with a `vibration` particle.
- **Sensor:** radius 8, active 30 ticks, cooldown 10, `power`, `last_vibration_frequency`, emits `sculk_sensor_tendrils_clicking` (which shriekers hear). **Calibrated:** radius 16, active 10, filters by input strength.
- **Shrieker:** 90-tick shriek, level event 3007, game event `shriek`. `canRespond` = `can_summon` and not peaceful and `spawn_wardens`. `WardenSpawnTracker.tryWarn` (per player, saved as `warden_spawn_tracker{ticks_since_last_warning, warning_level, cooldown_ticks}`, max 4, decays every 12000 ticks, 200-tick cooldown): nothing if a warden is within a 48-wide box; else all players within 16 plus the trigger player, none on cooldown, take the highest tracker, raise it, copy it to all. At level 4 summon a warden (20 tries, ±5/±6, ON_TOP_OF_COLLIDER, TRIGGERED), otherwise the nearby warden sound at ±10 and volume 5; then darkness within 40.
- **Warden:** 500 health, speed 0.3, knockback resistance 1, attack knockback 1.5, damage 30, follow range 24, size 0.9 × 2.9, fire-immune; melee disables shields 5 s. Darkness every 120 ticks to players within 20 for 260 ticks (only if under 200 left). Anger: vibration 35 (10 for a projectile over 30 blocks), +20 on hurt, max 150, CALM 0, AGITATED 40, ANGRY 80, decays 1 per 20 ticks; vibration cooldown 40, touch 20. Activities: EMERGE 134 ticks, DIG 100 (cooldown 1200), IDLE, INVESTIGATE, SNIFF 84, ROAR 84, FIGHT (melee cooldown 18, speed ×1.2). Sonic boom: 15 horizontal, 20 vertical, cooldown 40, 10 damage of `sonic_boom` (bypasses armour, enchantments and shields), push 0.5 up and 2.5 out × (1 − knockback resistance). Digs away after 1200 ticks without a target. Saves `anger` and `listener`. The heartbeat is client-side from the synced anger level.
- **Darkness:** blend 22 ticks. Fog end = lerp(blend, render distance, 15), start 0.75 × end. Lightmap: brightness = max(0, gamma − scale × blend); darkness scale = scale × max(0, 0.45 × blend × cos(ticks × π × 0.025)), subtracted before the clamp. Milk clears it.
- **Catalyst:** listens to `entity_die` within 8; takes the XP (none dropped), adds a cursor with that charge, `bloom` 8 ticks, `sculk_soul` particles; level spreader 10/4/10/5 over `#sculk_replaceable`, shriekers it grows have `can_summon=false`.
- Not verified, javap during the work: `entity_die` before XP drops; `checkSpawnRules` for a TRIGGERED warden; `SonicBoom.canStillUse`; fog colour darkening; the warden's navigation, tracking range and XP; the selector tie rule.

### Plan
Darkness and the game event core come first. All logic runs on the host in each level's `ServerSim`; the dispatcher lives there because the vendored crates cannot see each other (`minecraftoss-entities` depends only on `-player`, `-world` only on core and generator).

**Group `darkness`: "Minecraft darkness dims the view like vanilla"**
- A 22-tick blend per viewer; `lava_fog::darkness(sky, blend, render_distance)` (pattern `crates/minecraft_terrain/src/lava_fog.rs:47`) applied in `present_sky` after lava and powder snow; a darkness lane in the environment block, brightness max(0, 0.5 − blend), and `color -= darkness_scale` before the clamp in both lightmaps. The friend feeds the same from `PlayerView.effects`.

**Group `vibrations`: "Minecraft game events reach sculk sensors like vanilla"**
- New `crates/minecraft_terrain/src/game_events.rs` (ids, frequencies, radii, tags from the data pack) and `vibrations.rs` (selector, info, travel ticker, occlusion ray), owned by `ServerSim`. A `game_events` queue on `Level` (like `level.sounds`) and on `EntityWorld` (mob steps from `movement.rs:188-218`, damage, death, shooting); players' events in `TickInput.game_events` with player id and a sneaking flag (MW2 crouch and prone count as sneaking). Block listeners indexed per chunk section; their `listener` data stays in the block entity NBT (vanilla format).
- Sensor and calibrated sensor in a new vendored `world/src/level/sculk.rs`: phases, scheduled ticks, a redstone `Kind::SculkSensor`, comparator output.
- First sources: player step, swim, hit_ground (`render_anim/src/minecraft_players.rs:356-372`, `:412-420`, for host and friends); mob step and swim; block place and destroy (`server.rs:1559`, `:1640`); projectile shoot and land; explosions. The vibration particle reaches friends through `ToClient::Bursts`.

**Group `vibrations-more`: "More Minecraft actions make vibrations"**
- Containers, doors, trapdoors, gates, buttons, levers, plates, eat, drink, equip, damage, death, buckets, prime fuse, shear, entity place, teleport, elytra glide, mount, note blocks; MW2 guns (decision below); wool dampening and occlusion; amethyst resonance for calibrated sensors.

**Group `keyframes`: "Minecraft keyframe animations are shared by every mob model"** (P1, see D2).

**Group `warden-live`: "Minecraft wardens live in the entity world"**
- `entities/src/warden.rs`, `warden_brain.rs` (on the hoglin and villager brain framework), `world/wardens.rs`: attributes, poses, anger keyed by UUID, the vibration user (radius 16, `#warden_can_listen`), every activity, digging away, invulnerable while emerging or digging; save `anger` and `listener`; every per-kind list; extend `golem_spawn.rs` with ON_TOP_OF_COLLIDER; a crouch flag in `PlayerCandidate`.

**Group `warden-fight`: "Minecraft wardens attack with sonic booms and spread darkness"**
- Melee with difficulty scaling and the shield disable; `PlayerHitKind::SonicBoom { knockback }` (10 damage skipping armour, protection and shield) handled in `minecraft_survival/src/entities.rs` `mob_hit` and `death.rs` (`death.attack.sonic_boom`); the beam particles; darkness pulses through a new `Output.player_effects` applied per player.

**Group `warden-render`: "Minecraft wardens are drawn like vanilla"**
- `warden_render.rs`: `WardenModel` cubes, the tendril, heart and bioluminescent layers (check that the entity pass supports alpha-blended emissive layers; spider and enderman eyes are opaque), the keyframe animations through P1, digging particles. `MobShow` gets the pose with its start tick, the client anger level and the tendril and sonic ticks; the friend plays the heartbeat locally.

**Group `shriekers-catalysts`**
1. **"Minecraft sculk shriekers warn players and summon wardens"**: the shrieker in `level/sculk.rs`; `WardenSpawnTracker` per player on `Player`, ticked each player tick, saved in `player_nbt.rs` (no longer an unknown key); `TickInput` carries each tracker and `Output` returns the updated ones; `tryWarn` with the 16-block sync, `tryRespond`; `spawn_wardens` honoured; level event 3007 in `level_events.rs` for host and friends.
2. **"Minecraft sculk catalysts bloom when mobs die nearby"**: the catalyst listener takes `MobDeath.experience` and player death XP; bloom, sound, particles; the level spreader reused from `generator/src/feature/kinds/sculk.rs` with cursors saved in `cursors`.

### Tests
- Darkness: `/effect` accepts it; NBT round trip; the blend ramps over 22 ticks; `lava_fog::darkness` gives end 15 and start 11.25 at blend 1 and the render distance at 0; the lightmap scale at sample ticks; a friend's view gives the same fog.
- Vibrations: the frequency table equals javap; a step 5 blocks away activates a sensor after 5 ticks with the right power; a sneaking step does not, a sneaking block place does; wool occludes; a carpet step is dampened; nearest wins, ties by frequency; phases 30 then 10; listener NBT round-trips with an in-flight vibration; a calibrated sensor's 16 blocks; other dimensions never hear.
- Shrieker: each shriek raises the level with a 200-tick cooldown; two players within 16 both rise; one at 20 does not; level 4 summons within ±5/±6; `can_summon=false`, peaceful or `spawn_wardens=false` mean no response; decay; per-UUID save.
- Warden: emerge 134 ticks invulnerable; vibration +35 to that UUID; a hit +20 and target; picks the angrier of two players; melee 30 and shield disable; sonic boom 10 through armour and shield with the push; darkness every 120 ticks within 20; digs away after 1200 idle ticks; save and load; the friend keeps pose and anger from `MobShow`.
- Catalyst: a cow dying 5 blocks away blooms it and drops no XP; 10 blocks away drops orbs; never a `can_summon=true` shrieker; cursors round-trip.
- Keyframes: LINEAR and CATMULLROM samples against hand-computed values; the reader loads the warden's and the bat's definitions from the jar (gated on `MINECRAFTOSS_ROOT`).

### Owner's steps (idea)
1. `/locate structure minecraft:ancient_city`, `/tp`: a deep-dark city; 3 chests hold loot. Screenshot.
2. A sculk sensor with a redstone lamp next to it, walk 4 blocks away: ripple, click, glowing tendrils, the lamp lights. Crouch-walk: nothing. A wool wall: nothing. A snowball next to it: it activates. Screenshot.
3. `/effect give @s minecraft:darkness 30`: dark pulsing view, fog at about 15 blocks; milk clears it. Screenshot.
4. `/setblock ~ ~ ~3 minecraft:sculk_shrieker[can_summon=true]` with a sensor 2 blocks away. Trigger it 4 times, 10 s or more apart: a shriek, darkness and a distant warden sound each time; on the 4th a warden digs out, roars and sniffs. Screenshot.
5. Fight it: melee knocks the shield down; from 10 blocks behind a wall a sonic boom hurts through diamond armour; darkness every 6 s. Screenshot. Hide beyond 30 blocks for 60 s: it digs away.
6. A sculk catalyst with cows killed 2-5 blocks away: it blooms, no orbs, sculk spreads. Screenshot.
7. Friend: their steps trigger sensors; both get darkness; both warning levels rise together within 16 blocks; the friend sees the warden, hears its heartbeat, and a boom hurts them.
8. Save, quit, rejoin: the warden's anger, the shrieker and each player's warning level persist.

```
IW4L_MINECRAFT_KIT=minecraft:warden_spawn_egg*4,minecraft:sculk_sensor*16,minecraft:calibrated_sculk_sensor*4,minecraft:sculk_shrieker*8,minecraft:sculk_catalyst*4,minecraft:sculk*64,minecraft:white_wool*32,minecraft:white_carpet*16,minecraft:snowball*16,minecraft:redstone_lamp*4,minecraft:amethyst_block*8,minecraft:cow_spawn_egg*16,minecraft:stone*64,minecraft:diamond_sword+sharpness=5,minecraft:shield,minecraft:bow+power=5,minecraft:arrow*64,minecraft:milk_bucket,minecraft:cooked_beef*32,minecraft:diamond_chestplate+protection=4,minecraft:diamond_helmet,minecraft:totem_of_undying,minecraft:chest*2,minecraft:oak_door*4,minecraft:ender_pearl*16
```

---

## D6. The other mob batches

Every family follows the D2 pattern and tests. Below: what each batch adds beyond the default, the vanilla points to javap, and the owner's steps with a kit line. Kinds already dormant in the owner's worlds appear as soon as their family lands.

### Batch A: overworld animals
- **`rabbits`**: jump move control, carrot raids (crops), seven biome variants plus the killer bunny (`/summon` only), rabbit hide and foot loot, `RabbitModel` animated by keyframes (P1). Commits: "Minecraft rabbits hop and raid carrots", "Minecraft rabbits are drawn like vanilla".
- **`foxes`**: sleeping by day, pouncing, trusted UUIDs (bred foxes trust both parents' breeders), an item in the mouth, sweet berries, hunting chickens and rabbits, snow variant, `AdultFoxModel` and `BabyFoxModel` (baby keyframes). `MobShow`: sitting, sleeping, crouching, pouncing, held item. Commits: "Minecraft foxes live in the entity world and are drawn like vanilla", "Minecraft foxes sleep by day, pounce, carry items and trust their breeders".
- **`goats`**: brain, ramming (knocks players and mobs back, breaks horns off on logs and stone), long jump, screaming variant, milking, `GoatModel`. `MobShow`: ramming, lowered head.
- **`parrots`** (P3 first): flying navigation and move control; perching on shoulders (`ShoulderEntityLeft`/`Right` in each player file, drawn on the MW2 body for host and friend through track A's held-item fist rule), imitating nearby monsters' sounds, dancing to jukeboxes, cookies kill. Commits: "Minecraft mobs can fly with vanilla flying navigation", "Minecraft parrots fly, imitate and sit on shoulders".
- **`pandas`**: genes and seven personalities, bamboo eating, sneezing slime balls, rolling, sitting, `PandaModel`. `MobShow`: rolling, sitting, sneezing, lying.
- **`turtles`**: home beach, turtle egg blocks hatching at night, scutes when babies grow up, `TurtleModel`.
- **`armadillos-polar-bears`**: armadillo brain, rolling up when threatened, scute drop and brushing, keyframes; polar bear defending its cubs with the standing attack, `PolarBearModel`.
- **`camels`**: `HorseKind::Camel` with two seats (P4), sitting, dash, keyframes; camel husk as the monster camel carrying a husk (26.3 desert jockey).

```
IW4L_MINECRAFT_KIT=minecraft:rabbit_spawn_egg*4,minecraft:fox_spawn_egg*4,minecraft:goat_spawn_egg*2,minecraft:parrot_spawn_egg*2,minecraft:panda_spawn_egg*2,minecraft:turtle_spawn_egg*2,minecraft:armadillo_spawn_egg*2,minecraft:polar_bear_spawn_egg*2,minecraft:camel_spawn_egg*2,minecraft:camel_husk_spawn_egg*2,minecraft:carrot*32,minecraft:sweet_berries*32,minecraft:wheat_seeds*32,minecraft:bamboo*32,minecraft:seagrass*16,minecraft:cactus*8,minecraft:bucket*2,minecraft:brush,minecraft:saddle*2,minecraft:jukebox,minecraft:music_disc_cat,minecraft:chicken_spawn_egg*4,minecraft:iron_sword,minecraft:shield,minecraft:cooked_beef*32
```
Steps: hatch each; rabbits hop and eat planted carrots; foxes sleep at noon and pounce on a chicken, a fed fox carries an item; a goat rams you off a ledge, milk it with a bucket; a parrot lands on your shoulder when you crouch near it and dances at the jukebox, the friend sees it on your shoulder; a panda sneezes and rolls; turtle eggs on sand hatch at night; an armadillo rolls up when you sprint near it, brush it for a scute; a polar bear cub's parent stands up and attacks; two players ride one camel. Each with a screenshot.

### Batch S: water
- **`squids-water`** (P2 first): water-bound navigation, smooth swimming, flopping on land, air for water breathers, `Bucketable` and the `*_bucket` items with `bucket_entity_data`; squid and glow squid in a `Squid` store with the swim impulse, ink, glow ink, `SquidModel` (8 tentacles). Commits: "Minecraft water mobs swim, flop on land and fit in buckets", "Minecraft squids and glow squids".
- **`fish`**: cod, salmon (three sizes), tropical fish (two models, 22 patterns tinted by colour), pufferfish (three puff stages, poison on touch); `type_info` for water ambient.
- **`dolphins`**: breathing at the surface, leading players to treasure, dolphin's grace (P5 gains it), playing with items, `DolphinModel`.
- **`axolotls`**: brain, playing dead, hunting fish, five variants, bucket, regeneration for the player it helps.
- **`frogs-tadpoles`**: frog brain, three variants by biome temperature, tongue attack on small slimes and magma cubes making froglights, keyframes; frogspawn blocks hatching tadpoles, tadpole bucket, growth.
- **`guardians`**: laser with charge time and a beam renderer, spikes as thorns, `GuardianModel` (the elder ×2.35), the elder's mining-fatigue curse with the ghost particle, spawning at monuments.
- **`nautiluses`**: brain, saddle and armour riding (P4), dashing, the zombie variant with a drowned rider, `NautilusModel` with the coral layer, keyframes.

```
IW4L_MINECRAFT_KIT=minecraft:squid_spawn_egg*2,minecraft:glow_squid_spawn_egg*2,minecraft:cod_spawn_egg*2,minecraft:salmon_spawn_egg*2,minecraft:tropical_fish_spawn_egg*2,minecraft:pufferfish_spawn_egg*2,minecraft:dolphin_spawn_egg*2,minecraft:axolotl_spawn_egg*2,minecraft:frog_spawn_egg*2,minecraft:tadpole_spawn_egg*2,minecraft:guardian_spawn_egg*2,minecraft:elder_guardian_spawn_egg,minecraft:nautilus_spawn_egg*2,minecraft:zombie_nautilus_spawn_egg*2,minecraft:water_bucket*4,minecraft:bucket*8,minecraft:slime_ball*8,minecraft:magma_cube_spawn_egg*2,minecraft:saddle*2,minecraft:diamond_sword,minecraft:shield,minecraft:glass*64,minecraft:turtle_helmet,minecraft:cooked_beef*32
```
Steps: squids ink when hit; fish swim and flop on land, catch one in a bucket and pour it out elsewhere; a pufferfish puffs and poisons; dolphins jump beside your boat-less swim and give dolphin's grace; an axolotl plays dead and helps against a drowned; frogs eat a small magma cube and drop a froglight; tadpoles in a bucket; a guardian's laser charges and hits, the elder curses you with mining fatigue; ride a saddled nautilus; the friend sees all of them swim.

### Batch N: night and caves
- **`phantoms`**: `PhantomSpawner` per player from `Player.time_since_rest` (`player.rs:53`; vanilla rolls `clamp(stat)/24000` against 72000, needs open sky, 20-34 blocks up, group up to local difficulty + 1), circling and swooping, sizes, burning in daylight, avoiding cats, phantom membrane, `PhantomModel` with emissive eyes.
- **`undead-horses`**: `HorseKind::ZombieHorse` with its natural jockey (already built at `spawning.rs:988`) and rideable when tamed (26.3); natural lightning in thunderstorms (P8), which also turns pigs, villagers and mooshrooms like vanilla; `HorseKind::SkeletonHorse` and the `SkeletonTrapGoal`.
- **`silverfish-endermites`**: infested blocks releasing silverfish, waking friends, merging into stone, `SilverfishModel`; endermites from 5% of pearl landings, targeted by endermen, despawning after 2400 ticks, `EndermiteModel`.

```
IW4L_MINECRAFT_KIT=minecraft:phantom_spawn_egg*2,minecraft:zombie_horse_spawn_egg*2,minecraft:skeleton_horse_spawn_egg*2,minecraft:silverfish_spawn_egg*4,minecraft:endermite_spawn_egg*2,minecraft:infested_stone*16,minecraft:ender_pearl*16,minecraft:cat_spawn_egg*2,minecraft:saddle*2,minecraft:diamond_pickaxe,minecraft:diamond_sword,minecraft:shield,minecraft:bow,minecraft:arrow*64,minecraft:red_bed,minecraft:cooked_beef*32,minecraft:cobblestone*64
```
Steps: stay awake 3 nights (`/time add 24000` three times without sleeping): phantoms swoop at night under open sky and burn at dawn; a cat makes them avoid you; a zombie horse jockey spawns on plains (or egg) and can be ridden once tamed; `/weather thunder` long enough for a skeleton trap; mine infested stone: silverfish come out and wake others; throw pearls until an endermite appears.

### Batch G: late and built
- **`bees`**: P3, nest and hive block entities storing bees (saved), pollination and crop growth, honey level, anger and the stinger death, `BeeModel`.
- **`allays`**: brain, P3, collecting items matching its held one, note-block attraction, duplication with amethyst while dancing.
- **`sniffers`**: sniffer egg block hatching, digging up torchflower and pitcher seeds, brain, keyframes.
- **`copper-golems`**: build (copper block and carved pumpkin), sorting items from copper chests into chests, oxidation and waxing, turning into a statue block, keyframes.
- **`creakings`**: the creaking heart link, frozen while any player looks at it (per player, reuse `player_views`), invulnerable while its heart lives, keyframes.
- **`sulfur-cubes`**: the `AbstractCubeMob` family member: bucketable, shearable, archetypes with fuse, explosion and contact damage, picking up items, floating, `SulfurCubeModel` (inner and outer layers).
- **`happy-ghasts`**: dried ghast block → ghastling, harness, P3 flight, up to 4 riders (P4, multiplayer-relevant), `HappyGhastModel` with the harness.
- **`wither-live`** and **`wither-fight`**: the soul-sand and skull pattern, the invulnerable charge-up explosion, three heads, blue and black wither skulls, the half-health shield, block breaking, the boss bar (dragon template, per player in range), the nether star, `WitherBossModel`.

```
IW4L_MINECRAFT_KIT=minecraft:bee_spawn_egg*2,minecraft:bee_nest,minecraft:beehive,minecraft:allay_spawn_egg*2,minecraft:note_block*2,minecraft:amethyst_shard*8,minecraft:sniffer_egg*2,minecraft:copper_block*4,minecraft:carved_pumpkin*4,minecraft:copper_chest*2,minecraft:chest*2,minecraft:creaking_heart,minecraft:pale_oak_log*8,minecraft:sulfur_cube_spawn_egg*2,minecraft:shears,minecraft:dried_ghast*2,minecraft:white_harness,minecraft:soul_sand*8,minecraft:wither_skeleton_skull*3,minecraft:poppy*16,minecraft:glass_bottle*8,minecraft:netherite_sword+sharpness=5,minecraft:shield,minecraft:golden_apple*16,minecraft:cooked_beef*32
```
Steps: bees pollinate and fill a hive, harvest honey; an allay you hand an item collects matching drops; a sniffer egg hatches and the sniffer digs seeds; build a copper golem and watch it sort a copper chest; a creaking freezes while you look and dies when its heart breaks; shear a sulfur cube; grow a happy ghast and fly with the friend on it; build the wither (with the friend), its bar shows for both, kill it for a nether star. (Check the 26.3 ids `copper_chest`, `white_harness`, `dried_ghast` and `sulfur_cube_spawn_egg` against the item list before the owner's test.)

---

## D7. Trial chambers, vaults and breezes

### The owner's finding
"Trial chambers and trial spawners." Round 8 deferred trial spawners (`docs/round8/decisions.md`: no mob in the cage, no spawning, eggs refused, **not vanilla**).

### Cause
- Generation probably works (generic jigsaw with pool aliases, `structure/jigsaw.rs:331-430`; vault and trial spawner tags kept, `core/src/block_entity.rs:294-310`). No test locates one.
- `Level::tick_spawners` takes `minecraft:mob_spawner` only (`world/src/level/spawning.rs:296-312`), called from `ServerSim::tick` (`server.rs:2430-2433`). The display keeps only mob spawners (`spawner_display.rs:13`, `:92-94`), the host forwards only those to friends (`minecraft_survival/src/entities.rs:1353`), and eggs are refused on trial spawners (`server.rs:950-951`).
- Vaults have no simulation, no key insertion and no display item.
- Loot: the reward, consumable, ominous and equipment tables need the functions of group `loot-functions` (D4).
- No Bad or Trial Omen until group `effects-omens` (D3).
- No breeze (`SIMULATED_KINDS`). Wind charge bursts push but never trigger blocks (`crates/minecraft_terrain/src/wind.rs:89-120`).

### Vanilla 26.3
- **Trial spawner:** keys `normal_config` and `ominous_config` (a registry id from 28 `trial_spawner/*.json` or inline), `required_player_range` 14, `target_cooldown_length` 36000; state data `registered_players`, `current_mobs`, `cooldown_ends_at`, `next_mob_spawns_at`, `total_mobs_spawned`, `spawn_data`, `ejecting_loot_table`. Block properties `trial_spawner_state` (inactive, waiting_for_players, active, waiting_for_reward_ejection, ejecting_reward, cooldown; light 0/4/8/8/8/0) and `ominous`. Config defaults: spawn_range 4, total_mobs 6, simultaneous_mobs 2, +2 and +1 per extra player, ticks_between_spawn 40; targets `floor(base + added × max(0, registered − 1))`.
- **States:** waiting → active on a detected player (every 20 ticks by `(pos + gameTime) % 20`, not creative or spectator, within range, line of sight from the block centre to the eye); active spawns up to the targets (`spawnMob`: random offset `(r − r) × spawn_range + 0.5`, y ±1, `noCollision`, line of sight, spawn rules for TRIAL_SPAWNER, finalize only for an id-only tag, persistence, equipment loot, level event 3011); a mob farther than 47 blocks or dead is untracked; all spawned and dead → cooldown_ends_at and waiting_for_reward_ejection; after 40 ticks `open_shutter`, eject one reward per registered player, `close_shutter`, cooldown; cooldown ends → waiting. `canSpawnInLevel` = `spawner_blocks_work` and (not peaceful and `spawn_mobs`).
- **Ominous:** a detected player with Trial Omen, or Bad Omen turned into Trial Omen for 18000 × (amplifier + 1) ticks, triggers level event 3020 and `applyOminous`; ominous spawners drop items from above through an `OminousItemSpawner`.
- **Egg on a trial spawner:** reset, both configs' potentials replaced, state inactive.
- **Vault:** config (loot table `chests/trial_chambers/reward`, activation 4, deactivation 4.5, key `trial_key` or `ominous_trial_key`); server data `rewarded_players` (≤128 UUIDs), `state_updating_resumes_at`, `items_to_eject`, `total_ejections_needed`; shared data `display_item`, `connected_players`, `connected_particles_range`; states inactive, active, unlocking, ejecting; display item cycles every 20 ticks; a matching key from an unrewarded player unlocks it (14-tick delay), ejects one item at a time; a rewarded player or a wrong key gets the reject or fail sound, rate-limited to one per 15 ticks; creative keeps the key.
- **Breeze:** 30 health, speed 0.63, follow range 24, damage 3; brain with Shoot, LongJump, Slide and ShootWhenStuck; `BreezeWindCharge`; breeze rods; `BreezeModel` with wind and eyes layers. Sizes, charge radius and timings: javap.
- **Wind charge blocks:** `onExplosionHit` toggles wooden doors, trapdoors, gates, buttons, levers, bells and extinguishes candles; a breeze's charge only with mob griefing (`ServerExplosion.canTriggerBlocks`).
- Not extracted, javap during the work: eject velocities and `TIME_BETWEEN_EACH_EJECTION`, `ejectResultItem`, `OminousItemSpawner` rules.

### Plan
**Group `breezes`**
1. **"Minecraft wind charges open doors and press buttons like vanilla"**: `wind.rs::burst` also returns the blocks in reach; the server toggles them.
2. **"Minecraft breezes live in the entity world and shoot wind charges"**: the breeze module (pattern 8f545881), brain behaviours, `BreezeWindCharge` reusing `wind.rs`, saving, tracking, egg.

**Group `breeze-render`: "Minecraft breezes are drawn with their wind and eyes"** (keyframes, translucent scrolling wind layer, emissive eyes, the spawner display arm).

**Group `trial-spawners`**
1. **"Minecraft trial spawners show their mob spinning like vanilla"**: new `crates/minecraft_terrain/src/trial_spawner.rs` config reader (registry or inline, vanilla defaults); the server picks `spawn_data`; `spawner_display.rs` accepts trial spawners, reads `trial_spawner_state` and `ominous`, spin and particles (`docs/round8/a4-spawner-render.md` section 3); the friend filter at `entities.rs:1353` becomes `is_displayed_block_entity` (trial spawners and vaults).
2. **"Minecraft spawn eggs set a trial spawner's mob"** (`server.rs:950`).

**Group `trial-spawners-run`**
1. **"Minecraft trial spawners run their trial and eject rewards like vanilla"**: a pure `TrialSpawner` with `from_tag`/`write_tag` and `tick(now, &mut impl TrialWorld) -> Vec<TrialEvent>`; `ServerSim::tick_trial_spawners` after `tick_spawners`; `Level::summon_trial_mob` beside `summon_mob` (`spawning.rs:206`); players with UUIDs, creative and omen flags; `Output.omens` routed to that player.
2. **"Minecraft trial spawners show their detect, spawn and eject particles and sounds"**: level events 3011, 3013, 3014, 3019, 3020, 3021 for host and friends.

**Group `trial-ominous`: "Minecraft trial spawners turn ominous with Trial Omen and drop ominous items"** (`applyOminous`, `removeOminous`, the `OminousItemSpawner` as a level entity drawn as a floating item on host and friend).

**Group `vaults`**
1. **"Minecraft vaults take trial keys and eject rewards once per player"**: new `crates/minecraft_terrain/src/vault.rs`; `Command::VaultKey { player, pos, stack }`; the key is spent only on success through `item_use_waiting`; the vault tick next to the trial spawners.
2. **"Minecraft vaults show their reward item turning"**: a display like `spawner_display` with the dropped-item visuals.

Plus a generation test like the ancient city one: trial chambers contain trial spawners and vaults.

### Tests
- `trial_spawner`: every config from the data reads (28; slime weights; the ominous zombie has an equipment table); activation only for a survival player in range and sight, on the 20-tick phase; targets 6/2 for one player and 8/3 for two, one per 40 ticks; all dead → one reward per registered player, 40-tick shutter, cooldown 36000, back to waiting; a mob beyond 47 blocks is untracked; bad omen → trial omen 18000 × (amp + 1) and ominous; peaceful or `spawner_blocks_work` off resets; an unsimulated kind leaves it inactive; the tag round-trips.
- `server.rs`: a trial spawner spawns zombies for a near player and rewards after they die (model on the spawner test near `:6696`); an egg sets its mob.
- `spawner_display`: a waiting spawner turns and flickers; an inactive or cooling one shows no mob.
- `vault`: one unlock per UUID; another UUID accepted; wrong key fails rate-limited; the ominous vault wants the ominous key; activation at 4, deactivation at 4.5; display cycle; ejection; creative keeps the key.
- `wind`: a burst opens a wooden door and presses a button but not an iron door; a breeze burst only with mob griefing.
- Breezes: the D2 family tests.

### Owner's steps (idea)
New world `IW4L_MINECRAFT_WORLD=trial9`.
1. `/locate structure minecraft:trial_chambers`, `/tp`, dig down: copper rooms, orange-lit spawners showing their mob turning with small flames. Screenshot.
2. Walk into sight of a spawner in survival: active, mobs come out (6 for one player). Kill them: after about 2 s it ejects a key or consumables, then goes dark. Screenshot.
3. In creative near another spawner: it stays waiting.
4. A vault lights up and shows a turning reward; a trial key opens it, items drop one by one; a second key is refused with a click. Screenshot.
5. Drink an ominous bottle, walk into sight of a fresh spawner: Trial Omen, blue soul flames, armoured mobs, items drop from above. Win: an ominous key for an ominous vault. Screenshot.
6. Breezes hop and shoot wind charges; kill one for breeze rods. A wind charge opens a wooden door and presses a button.
7. A trial spawner from the creative tab with a zombie egg: a zombie spins inside, and zombies spawn when you walk up in survival.
8. Friend: in a chamber together the spawner spawns 8 mobs and ejects two rewards; each player opens each vault once; the friend sees states, flames and the vault item.

```
IW4L_MINECRAFT_KIT=minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword+sharpness=5,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:diamond_helmet+protection=4,minecraft:diamond_chestplate+protection=4,minecraft:diamond_leggings+protection=4,minecraft:diamond_boots+feather_falling=4,minecraft:golden_apple*16,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:trial_key*4,minecraft:ominous_trial_key*2,minecraft:ominous_bottle*2,minecraft:wind_charge*16,minecraft:zombie_spawn_egg*4,minecraft:breeze_spawn_egg*4,minecraft:trial_spawner,minecraft:oak_door*2,minecraft:stone_button*2,minecraft:cobblestone*64,minecraft:milk_bucket,minecraft:water_bucket,minecraft:ladder*32
```

---

## D8. Lodestones and compasses

### The owner's finding
"Lodestones and compasses", the last open N6 item.

### Cause
1. Nothing turns a compass into a lodestone compass: no item-use path handles a compass on `minecraft:lodestone` (use chain `crates/minecraft_survival/src/actions.rs:1067-1078`, `Change` enum `crates/minecraft_terrain/src/bucket.rs:144-152`); no file sets `minecraft:lodestone_tracker`.
2. The needle cannot point at a lodestone: `Dials` evaluates once per item id (`crates/minecraft_terrain/src/dials.rs:9-13`, `:114-121`), `minecraft:condition` always takes `on_false` (`:210`), `compass()` knows only `spawn` and `recovery` (`:152-156`).
3. Icons, held models and names are keyed by id (`render_anim/src/minecraft_inventory.rs:336-338`, `:482`, `:504`; `hud/src/minecraft_gui.rs:729-733`; `render_anim/src/minecraft_world.rs:1424`; `display_name` at `minecraft_inventory.rs:716-724`). The glint already works (`glint.rs:37`).
4. Nothing clears the tracker when the lodestone breaks.
5. A friend's compass and clock never animate: `dials.step` runs only on the host (`minecraft_world.rs:909`); the friend publishes without stepping (`minecraft_remote.rs:1073-1085`) and never receives the world spawn or its last death (`messages.rs:287-297`).

### Vanilla 26.3
- `CompassItem.useOn` on `Blocks.LODESTONE`: sound `item.lodestone_compass.lock`; tracker `{target:{dimension, pos:[x,y,z]}, tracked:true}`; a single compass outside creative changes in place; otherwise `transmuteCopy` gives one tracked compass (added or dropped) and one is consumed (not in creative). An existing lodestone compass relinks the same way.
- Named `item.minecraft.lodestone_compass`, glints.
- `inventoryTick`: if tracked, with a target in this dimension, and the position is out of bounds or the POI manager has no lodestone there, the target is cleared and `tracked` stays true.
- `items/compass.json`: `condition has_component lodestone_tracker` → `range_dispatch compass target=lodestone`. `CompassAngleState` spins at random with no target, another dimension or a target under 1e-5 squared away; our `Needle` already does the rest.

### Plan
**Group `lodestones`**
1. **"Minecraft compasses lock onto lodestones like vanilla"**: new `crates/minecraft_terrain/src/lodestone.rs` (`tracker`, `with_tracker`, `lock_use`, `lock`, `tick_tracker`); `Change::Lodestone(pos)`; the chain link at `actions.rs:1067` and the arm at `:1097`; the tracker tick once per hand tick for every slot against loaded chunks; the name in `display_name`.
2. **"Minecraft lodestone compasses point at their lodestone"**: `dials.rs` evaluates `has_component` conditions and the `lodestone` target; `stack_key(stack)` returns a virtual id (`minecraft:compass@lodestone:<dim>:<x>,<y>,<z>` or `...:none`) evaluated each tick; icons, the hand model and the HUD use the per-stack key. **Reuse track B's per-stack `McStack.icon` (B9) instead of adding a second field**; whichever track merges second adapts.
3. **"Friends' compasses and clocks turn like the host's"**: the friend steps its dials before publishing; `ToClient::SpawnPosition { dimension, pos }` on login and on change, and `last_death` in `PlayerView`.

### Tests
- `lodestone`: a single compass tracks in place; a stack of 3 gives 2 plain and 1 tracked, a full inventory returns it to toss; creative keeps the plain one; relinking; a compass on stone does nothing; the tracker forgets a broken lodestone only when loaded, tracked and in its dimension.
- `dials`: a lodestone compass points at its lodestone and spins without one or in another dimension; its key differs from the plain compass and resolves to `minecraft:compass#minecraft:item/compass_NN`; a plain compass ignores the branch.
- `player_nbt`: a lodestone compass survives save and load (pos as a List and as an IntArray).
- `render_anim`: named and glinting. Codec round trip of `SpawnPosition`.

### Owner's steps (idea)
1. Place a lodestone, right-click it with 8 compasses: a glinting "Lodestone Compass" and 7 plain ones. Walk 30 blocks and turn: it points back. Screenshot.
2. Relink the single lodestone compass: it changes in place.
3. In the Nether link a second compass and walk away: it points. The Overworld one spins. Screenshot.
4. Break the first lodestone while holding its compass: it spins and keeps name and glint.
5. Quit and restart: targets kept.
6. Friend: links their own compass; it points on the friend's screen; the friend's plain compass points at spawn and the clock turns.

```
IW4L_MINECRAFT_KIT=minecraft:lodestone*4,minecraft:compass*8,minecraft:clock,minecraft:obsidian*14,minecraft:flint_and_steel,minecraft:diamond_pickaxe+efficiency=5,minecraft:diamond_sword,minecraft:cooked_beef*32,minecraft:cobblestone*64
```

---

## D9. The friend limits

### The owner's finding
"The friend limits (skating, first-person hand, music, Minecraft melee between players, portal overlay)", listed in `docs/MULTIPLAYER.md` section 20.

### Cause
1. **First-person held item and swing.** The friend's `view.hand` holds only the totem activation (`crates/render_anim/src/minecraft_remote.rs:1449-1465`). `ui.holding_item` is set (`remote.rs:1059-1062`), which hides the MW2 viewmodel (`occupancy/fpv_present.rs:426`), so a friend holding an item sees nothing in hand. `ui.hand_swing` and `ui.offhand_busy` are never written on the friend (`fpv_present.rs:1143-1144`). Only the host draws held items (`minecraft_world.rs:1392-1433`). The swing state lives on the host's `Player.hand` (about 25 `hand.swing = Some(0.0)` sites in `actions.rs`, advanced at `minecraft_players.rs:465-470`), and no message carries it (`ToClient`, `messages.rs:159-269`). `UsingView` (`messages.rs:387-390`, `views.rs:104-148`) does not say what kind of use it is.
2. **Music** runs only in the host's `minecraft_world::update` (`:1524-1528`), which returns at once for a client (`:493-497`). Music means the three End tracks; nobody has Overworld or Nether music.
3. **Portal overlay.** `minecraft_portal::step` sets `ui.portal_overlay` only with a `MinecraftUi` (`render_anim/src/minecraft_portal.rs:61-66`); `step_friend` passes None (`minecraft_players.rs:1433-1442`); the friend never loads `ui.portal_frames` (`minecraft_world.rs:632`).
4. **Melee between players.** `punch` (`minecraft_survival/src/entities.rs:690-725`) never tests players; `actions::allows` refuses `Target::Player` (`actions.rs:164`), so `HandQueue::take` (`session.rs:426-434`) drops it; the friend's `aim()` ignores players (`remote.rs:891-929`). The pvp rule (`entities.rs:391`) covers only thrown items and potions. `DamageCause` has no player (`death.rs:75-93`). The victim side works already: `struck()` (`minecraft_players.rs:761-818`) applies shield, axe disable, armour, Protection, cooldown, totem and MW2 knockback for host and friends.
5. **Skating.** `skate::update` needs an `AuthorityWorld` (`render_anim/src/skate.rs:400-402`), which a friend never has; `board_allowed` is never set on the friend (`:430`); the host gives friends `skating: false` and `Skating::default()` (`minecraft_players.rs:679`, `:1474`); the state lane carries only `HandState` (`minecraft_host.rs:180-187`); other players never see a skater (`occupancy/remote_body.rs:445-447`, `:943-948`; `skate/rig.rs:60` takes `&SkateMode`); posing needs the viewer's own `rig.json` (`rig.rs:12-20`).

### Vanilla 26.3
- The client swings at once on attack (`Minecraft.startAttack`, `LocalPlayer.swing`), sends `ServerboundSwingPacket`; the server broadcasts `ClientboundAnimatePacket` to others, not back to the swinger; server-started swings reach the swinger. `missTime` 10 ticks after a miss (not creative). A swing restarts only if none is running or it is at least half way. Eating, blocking and the trident charge are drawn from the client's own use state.
- `MusicManager` is client-side; every client plays its own.
- `LocalPlayer.handlePortalTransitionEffect` runs from the client's own portal state, lerps the intensity, plays `block.portal.trigger` locally and closes screens that are not allowed in portals. Our `PortalEffect` (`minecraft_survival/src/portal.rs:173-205`) is the same curve.
- `Player.attack` treats players like any `LivingEntity`: damage × (0.2 + s² × 0.8); crit ×1.5; sprint knockback +0.5 then ×0.6 speed and sprint stops; a sword sweep hurts every living entity within 1 block of the target and 3 of the attacker for 1 × s with knockback 0.4; sounds `entity.player.attack.strong/weak/crit/sweep/knockback/nodamage`. `ServerPlayer.hurtServer` refuses a player source when `canHarmPlayer` is false (pvp off); creative players are invulnerable. Death message `death.attack.player`; the `.item` form only for a custom-named weapon. Sharpness +1 + 0.5 per level above 1; Knockback +1 per level.
- Skating has no vanilla equivalent; vehicles are client-moved and server-checked (`ServerboundMoveVehiclePacket`), which the plan follows.

### Plan
Bump `minecraft_net::PROTOCOL` once for the track (D3's trading-friend commit is the first message change), with codec tags and round-trip tests for each new message.

**Group `friend-hand`**
1. **"A friend sees their held items, eating, the shield and the trident charge in first person"**: move `minecraft_world.rs:1393-1432` into `minecraft_hand.rs` `held_hand_mesh(...)` (the host calls it, unchanged behaviour); `UsingView` gains `kind: UseKind { Eat { duration }, Block, Charge, Scope }`; the friend re-bases a local use counter and a new `show_hand()` replaces `remote.rs:1449-1465`, hiding the hand when skating, in F5, asleep, with the UI off, scoping or dead.
2. **"A friend's swing plays at once and follows the host's"**: `ToClient::Animate { client, offhand }`; `Hand.swing_predicted` set only by the attack-press swing (`actions.rs:370-371`); the host sends Animate for unpredicted swings started this frame; the friend predicts on attack press with the vanilla restart rule and `miss_time`.

**Group `friend-portal-music`**
1. **"A friend sees the purple portal overlay while standing in a portal"**: frames loaded on the friend; a pure `friend_portal_tick(effect, inside)`; inside from `portal::touched_portal` on the friend's own terrain; closes the inventory on the first rising tick; reset on a level rebuild and in `stop()`.
2. **"A friend hears the End, dragon and credits music"**: a `MusicManager` on the friend, `ResMut<audio::McMusic>` folded into an existing tuple (the system is at Bevy's 16-parameter limit); `MusicInput` from the friend's own level, credits and boss bars; `stop()` clears `wanted`.

**Group `pvp-melee`**
1. **"Minecraft melee hits other players, with sweeps, crits, knockback and the pvp rule"**: `Entities.player_boxes` per level (set each frame before any `act()`), `player_on_ray` (never the attacker); `punch()` picks the nearest of frame, End part, bullet, fireball, mob and player; a new `minecraft_survival/src/pvp.rs` with a pure `melee(attack, victim) -> PvpHit`; pvp off → nodamage sound, swing and cooldown reset; otherwise a Hit into the victim's `queue.out.hits`; sweeps reach players near the target, including when the target is a mob; attacker exhaustion and weapon wear; `allows()` passes `Target::Player` in reach; the friend's `aim()` tests the other bodies in the same band. Sharpness and Knockback through one helper used for mob hits too.
2. **"Players killed by Minecraft melee get vanilla death lines"**: `DamageCause::Player { client }`, `melee_text` with the `.item` form for custom names; `death_text` and `death_line` take a player-name lookup (callers at `minecraft_players.rs:958-981`); the kill card gets the killer.

**Group `friend-skating`: "A friend can skate: the host moves them from the friend's board"**
- `StateUp { Hand(HandState), Skate(SkateReport{ tick, on, airborne, origin, yaw }) }` on the state lane; a reliable off report; a `SkateBody` trait for `AuthorityWorld` (host) and `ClientPredictionState` (friend), with `ClientPrediction::set_external_motion` and `set_origin` (`net/src/client/predict.rs:204-282`); `board_allowed` made `pub(crate)` and computed on the friend; the host accepts a report only while alive, not switching, board allowed, within a plausibility step of the last origin and not stuck, else `ToClient::SkateRefused`; cleared on leave, death, dimension change and teleports; `step_friend` passes the friend's `Skating` and board gear.

**Group `skater-poses`: "Everyone sees every skater's pose and board"**
- `frame::SkatePoses` per client with the 22 mapped bones (`rig.rs:22-46`); `rig::pose` takes names and bones; the friend's report carries the pose; the host relays `Skaters` to players in the same level, including its own; each game interpolates; a viewer without `rig.json` sees the idle soldier with no board.

### Tests
- Codec: `Animate`, `UsingView` with kind, `StateUp`, `SkateRefused`, `Skaters`, `SpawnPosition`; the protocol number.
- `views.rs`: Eat 32 for a steak, 16 for dried kelp, Block, Charge, Scope.
- `actions.rs` and `pvp.rs`: `allows` passes a player in reach only; a hit queues one Hit with damage × scale; pvp off queues none; crit ×1.5; sprint knockback; axe `disable_shield_ticks == 100`; a sweep reaches a second player in range and skips one 4 blocks away; never self; spectators skipped; strength 0.5 gives 0.4; Sharpness 5 adds 3.0; Knockback 2 adds 1.0.
- `death.rs`: "Friend was slain by Host", the `.item` form only with a custom name.
- Portal: the friend overlay rises over 20 ticks, falls outside, 0 for a spectator, resets on a level change.
- Friend: swing predicted, not restarted by an echo in its first half, restarted after, suppressed by `miss_time`; `show_hand` hidden in F5, in bed and skating; the use counter counts eating down; music starts the dragon track with a boss bar and `stop()` clears it.
- Host: Animate only to the owner and only for unpredicted swings; a skate report too far or inside a wall is refused; a valid one moves the friend; leave, death and a dimension change clear it.
- `rig::pose` with names and bones gives the old matrices.

### Owner's steps (idea)
Same build in both windows (new protocol): `& scripts\mp-local.ps1 -Exe iw4l-survival.exe -World r9-friend -Fresh -HostSound`. `/op Friend`, `/gamerule pvp true`.
1. Friend: the diamond sword shows in the right hand; left-click into the air swings at once. Screenshot. Empty hand: the MW2 hand swings.
2. Friend eats beef: it rises to the mouth and bobs. Screenshot. Shield in the offhand raised. Screenshot. Trident pulled back and thrown. F5 hides the first-person item.
3. Friend steps into a lit portal: the purple swirl fades in with the sound, the inventory closes, stepping out fades it. Screenshot.
4. Host: `/execute in minecraft:the_end run tp @a 0 70 0`: the friend hears the dragon music when the bar shows; back in the Overworld it stops.
5. Both in iron armour. Friend hits the host 3 times at full charge: armour-reduced damage, knockback, strong-hit sound. Screenshot. Falling hit: crit. Sprint hit: bigger knockback. Host blocks with the shield: blocked; an axe disables it for 5 s. A sweep with two cows next to the host hits all three.
6. Host renames an iron sword "Excalibur" and kills the friend: "Friend was slain by <host> using [Excalibur]" once in both chats. Screenshot.
7. `/gamerule pvp false`: hits do nothing but the no-damage sound.
8. Skating (Skate 3 data, Skating switch on): friend presses J and skates stairs; the host sees the friend's soldier on a board in the skate pose. Screenshot of both. A 4-block drop hurts; deep water and a `/tp` pick the board up. Host skates: the friend sees it. Screenshot.

```
IW4L_MINECRAFT_KIT=minecraft:diamond_sword,minecraft:iron_axe,minecraft:shield,minecraft:cooked_beef*32,minecraft:trident,minecraft:spyglass,minecraft:obsidian*10,minecraft:flint_and_steel,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:iron_sword,minecraft:anvil,minecraft:experience_bottle*32,minecraft:cow_spawn_egg*4,minecraft:totem_of_undying,minecraft:golden_apple*4,minecraft:potion@swiftness,minecraft:cobblestone*64,minecraft:oak_stairs*32,minecraft:water_bucket,minecraft:dried_kelp*16,minecraft:iron_sword+knockback=2
```

---

## D10. The not-vanilla items

### The owner's finding
"A ridden strider under `/tick freeze`, TNT lit by another player with pvp off" (`docs/round8/decisions.md:23`, `:71`).

### Cause
**(1) Strider under freeze.** Three causes, all to fix:
- Server: `run_tick` (`crates/minecraft_terrain/src/server.rs:3451-3459`) ticks mobs only when `normal`; a frozen tick has `normal=false` (`tick_rate.rs:256`). The vendored `tick_with_players_where` (`entities/src/world.rs:5124-5139`) filters only by position and always bumps `game_time`. Steering still arrives (`server.rs:3448`) and dismounting works (`:3449`).
- Host view: `EntityView::advance` (`minecraft_survival/src/entity_view.rs:110-115`) ticks `ClientMobs` only when `runs_normally`; `ClientMob::tick` (`client_mobs.rs:648-656`) applies the moves.
- Seat and partials: `Entities::ride_seat` (`entities.rs:848-859`) places every rider (host and friends, `render_anim/src/minecraft_riding.rs:38-43`, `minecraft_players.rs:852`) from the frozen clock; mob meshes use the same frozen partial (`entity_view.rs:665`, striders at `:847`). The friend's game always ticks (`minecraft_remote.rs:1496`), so it follows once the host moves the strider.

**(2) TNT. The premise is wrong.** TNT hurts no mob and no player at all, with or without pvp (`docs/MINECRAFT-SURVIVAL.md:430` says so; `:374`, `docs/round8/decisions.md:71` and `docs/MULTIPLAYER.md:783`, `:1323` are wrong).
- `TntData {fuse, power}` has no owner (`world/src/level/entity.rs:46-51`), nor has `Explosion` (`level/explosion.rs:44-52`); `prime_tnt`, `ignite_tnt`, `new_primed_tnt` (`:61-80`) and chained TNT (`:392-400`) take none.
- `tick_tnt` explodes inside `level.tick` (`:103-113`); `Level::hurt_entities` (`:259-304`) reaches only level entities; `ServerSim::tick` (`server.rs:2429-2438`) only turns a vanished TNT into a sound and crystal blast. Nothing calls `mobs.blast` for TNT.
- `Command::ItemOnBlock { player, .. }` (`server.rs:3299-3300`) drops the player before `PrimeTnt` → `ignite_tnt` (`:1769`).
- `PlayerHit.source` (`world.rs:573-581`) names only a mob, so pvp could not be honoured; an end crystal popped by a player (`server.rs:2224` → `blast_by_player`, `world.rs:2875`) hurts players with pvp off too.
- An explosion with no source is told as a generic death (`minecraft_survival/src/death.rs:100-118`), where vanilla says "blew up".

### Vanilla 26.3
- `TickRateManager.isEntityFrozen(e)` = not running normally, not a player, and `countPlayerPassengers() <= 0` (indirect passengers count). So under freeze the whole stack under a player ticks fully; game time, weather and block ticks stay frozen. The client uses the same test, and a player-carrying vehicle is drawn with the live partial.
- `PrimedTnt` has an owner (saved as `owner`). `TntBlock.prime(level, pos, igniter, stack)` sets the igniting player (flint and steel, fire charges); redstone and neighbours pass null; chained TNT inherits `explosion.getIndirectSourceEntity()`. The explosion source is `explosion(tnt, owner)`.
- `ServerPlayer.hurtServer` refuses a player-sourced hit when `canHarmPlayer` is false: with pvp off a player's TNT hurts no player, the igniter included, but still hurts mobs (credited to the player). `ServerExplosion.hurtEntities` still pushes a spared player. `hurtEntities` runs before `interactWithBlocks`, so exposure is computed with the blocks still standing.
- `EndCrystal.hurtServer` explodes with `explosion(crystal, source.getEntity())`, so a popped crystal also spares players with pvp off.
- Death messages: `death.attack.explosion` "%1$s blew up"; with a player `death.attack.explosion.player`.

### Plan
**Group `strider-freeze`: "A Minecraft strider carrying a player keeps walking under /tick freeze"**
- Vendored `world/riding.rs`: `player_carriers()` (the whole stack under every player rider). `world.rs`: split `tick_with_players_where` into a private `tick_selected(.., frozen)`, keep the old wrapper, add `tick_frozen_with_players_where` (only carriers, no `game_time` bump, no piglin anger resolution, same epilogue). Audit `game_time` users first (`world.rs:5406`, `:3479`).
- `ServerSim::tick_ridden_mobs` sharing `tick_mobs` (`server.rs:808-854`); `run_tick` calls it in the frozen branch.
- `ClientMobs::tick_only(ids)` with the rider snapping shared with `tick()` (`client_mobs.rs:976-984`).
- `EntityView`: `ride_clock`, `ride_partial()`; carriers drawn with the ride partial; `ride_seat` uses it.

**Group `tnt`**
1. **"Minecraft TNT hurts mobs and players like vanilla, before its blocks break"**: `Level.defer_tnt_blasts` (default false so vendored tests stay) and `tnt_blasts`; `ServerSim::new` sets it; `ServerSim::tick` drains them after `level.tick()` instead of the scan at `:2430-2438`: mobs and players first (`mobs.blast` with a `blast_players` list filled by `run_tick`), death loot, then `level.explode`, then the crystal and sound blast as before.
2. **"Minecraft primed TNT remembers who lit it, and with pvp off a player's TNT or end crystal only pushes players"**: `owner` on `TntData` and `Explosion` (every literal), `ignite_tnt_by`, chained inheritance; `PrimeTnt` routed with the player; `PlayerHitKind::Explosion { knockback, by }` set from `Blaster::Player`; on the host, `spared_by_blasts_without_pvp` turns player-caused explosion hits into pushes when pvp is off (check that the push uses the same convention as `players_pushed`, `entities.rs:229`, and the hit at `:1590`).
3. **"An explosion with no attacker is told as \"blew up\""** (`death.rs`).

**Group `small-not-vanilla`** (each verified in code or javap by the research)
1. **"A Minecraft spawner that refuses an egg says so in chat"**: `advMode.notEnabled.spawner` to that player (`entities.rs:1395-1398` only logs).
2. **"The Minecraft enchanting seed is saved with the player"**: `XpSeed` in each player file (`player.rs:47`, `:123`, `menus.rs:703`, `player_nbt.rs:380`, `:528`), UUID-derived default when missing.
3. **"An MW2 gun hit wakes a sleeping Minecraft player"** (decisions:72), if the wake path is reachable from the host's MW2 damage hook; per player.
4. **"A ghast's or blaze's fireball makes a hoglin retaliate"** (decisions:29), if it is the missing `note_hoglin_hurt` call in the fireball hit path.

### Tests
- Entities: `player_carriers_is_the_stack_under_a_player`; `a_frozen_tick_moves_only_what_carries_a_player` (game time unchanged, the other strider and its tick count unchanged).
- `server.rs` with `frozen_sim()` and `run_ticks(.., false, n)` (like `primed_tnt_waits_while_frozen`, `:9544`): `a_strider_carrying_a_player_walks_while_frozen` while a cow stays put.
- `client_mobs`: `tick_only` moves only the listed vehicle and snaps its rider. `entity_view`: `ride_partial` moves while `partial` is fixed and they agree after unfreezing.
- TNT: `chained_tnt_inherits_the_owner`; `tnt_hurts_a_mob_and_a_player_before_its_blocks_break` (one hit with `by == None`, damage over 10, the cow hurt, one explosion; a wall it breaks still shields exposure); `tnt_lit_by_a_player_names_them_and_credits_their_kills`; `a_crystal_popped_by_a_player_names_them`; the existing TNT tests pass unchanged; `with_pvp_off_a_players_blast_only_pushes_players` (mirroring `entities.rs:2622`); "<name> blew up".
- Small items: the chat line reaches only that player; `XpSeed` round trip and default; a gun hit wakes; a fireball sets the hoglin's target.

### Owner's steps (idea)
**Strider:**
1. Netherrack 10×10, two lava buckets, two saddled striders, a cow 5 blocks away. Mount one with the fungus on a stick. `/tick freeze`. W: the strider walks smoothly; the cow and the other strider stay still. Screenshots at the start and after 5 s.
2. Shift off: it freezes too. Mount again: it walks. `/tick step 20` moves everything for a second; `/tick unfreeze` resumes all.
3. Friend on the second strider while frozen: both screens show it moving; the host's empty strider and the cow stay frozen. Screenshots of both.

**TNT** (tell the owner: TNT now hurts players and mobs everywhere, single player too):
1. TNT with a cow beside it, lit with flint and steel, you 3 blocks away: you lose hearts and are pushed, the cow dies and drops beef and XP. Screenshot. Behind a 3-block cobblestone wall: little or no damage.
2. `/gamerule pvp false`: the friend lights TNT 2 blocks from both of you: no hearts lost, both pushed, a cow beside it dies. Screenshot. A chain of 4 lit by the friend spares you too. TNT lit by a redstone block hurts both. Screenshot. `/gamerule pvp true`: the friend's TNT hurts both. Screenshot.
3. With pvp off the friend punches an end crystal: you are pushed but not hurt. Screenshot.
4. A TNT death reads "<name> blew up".
5. A spawner with `spawner_blocks_work` off refuses an egg with a chat line; the enchanting table's offers stay the same after a restart.

```
IW4L_MINECRAFT_KIT=minecraft:strider_spawn_egg*4,minecraft:saddle,minecraft:saddle,minecraft:warped_fungus_on_a_stick,minecraft:warped_fungus_on_a_stick,minecraft:lava_bucket,minecraft:lava_bucket,minecraft:netherrack*64,minecraft:cow_spawn_egg*8,minecraft:tnt*32,minecraft:flint_and_steel,minecraft:flint_and_steel,minecraft:redstone_block*4,minecraft:cobblestone*64,minecraft:end_crystal*4,minecraft:obsidian*8,minecraft:enchanting_table,minecraft:lapis_lazuli*32,minecraft:experience_bottle*32
```

---

## Shared points for the track
- **The mob list.** `docs/round9/mobs.md` belongs to track D. The first group writes it; every mob group updates its rows (status, hit-key byte, group, commits) in the same commit as the code.
- **The protocol.** `minecraft_net::PROTOCOL` goes up once in track D, in the first commit that changes a message (D3 `trading-friend`). If tracks A or B bumped it first, the merge takes the next number and renumbers the new message indices. Record the final number in the log.
- **`McStack`.** Track B adds `icon` and `lore`, track C touches it too. D8 reuses B's `icon`; whoever merges second keeps every field.
- **Effects.** `EffectKind` grows once in `effects-omens` (darkness, bad omen, raid omen, trial omen, hero of the village); `dolphins` adds dolphin's grace. Track B9 also edits `potions.rs` for colours: keep D's edits to added match arms.
- **Loot functions** are ported once in `loot-functions` for every user (raids, ancient cities, trial chambers, desert pyramids, shipwrecks).
- **Shared files.** `server.rs`, `server_mobs.rs`, `entities.rs`, `entity_view.rs`, `actions.rs`, `menus.rs`, `messages.rs`, `minecraft_remote.rs`, `minecraft_players.rs`, `death.rs` and `hud/src/minecraft_inventory.rs` are shared with tracks A, B and C. Keep every edit additive: new modules, match arms and chain links.
- **Hit-key bytes** 29 upward, one per new family, recorded in `docs/round9/mobs.md`.
- **Vendored patches.** Count the minecraftoss files patched and added by track D in the log, for the CLAUDE.md count at the end of the round.
- **State docs.** Do not edit `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md`, `docs/PROGRESS.md`, `docs/round9/decisions.md`, `docs/round8/*` or `docs/round8/friend-install.md`. Write their new wording in the track log under "State doc changes" (for example: lodestones and trial spawners leave "Finish survival", TNT now hurts, the friend needs Skate 3 data to skate).

## Rules
One commit per fix or feature, with tests. Run clippy, and rustfmt on new files. No code comments. Use gentle cargo mode. Owner steps and the kit line for each commit go in `docs/round9/track-d-log.md`.
