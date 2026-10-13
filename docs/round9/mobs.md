# Round 9 mob list: the 52 missing Minecraft mobs

Track D's own file. Every mob group updates its rows (status, hit-key byte, commits) in the same commit as the code. The source is the D2 table of `docs/round9/track-d.md`; the order is the order a survival player meets the mobs.

Status: `left` (nothing built), `live` (in the entity world, saved, loaded and seen by friends), `drawn` (live and drawn like vanilla), `done` (drawn, its vanilla behaviours, spawning and the owner's steps written).

Sizes: S, M, L, XL. "Exists" lists only what goes beyond the default: a texture, a loot table, sounds, an unported model class and a `/summon` tag exist for every one of them.

## The list

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Group key | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Drowned | oceans, rivers, drowning zombies | `ZombieKind::Drowned`, conversion, renderer, attack goal; saved, loaded, hatched and seen by friends; natural spawns, trident, rod and nautilus shell with their drops; water, beach, swim-up and stroll goals, trident throw, raised arm | zombie nautilus jockeys (with row 38), the ocean ruin trident roll, the `trident_throwing` held model | M | `mobs-list-drowned-load`, `drowned` | 1 (zombie store) | done | "Minecraft drowned are saved, loaded and seen by friends" (after 80caed9e), "Minecraft drowned spawn in rivers and oceans with tridents and nautilus shells" (after 8fa0de3a), "Minecraft drowned go to water by day, to the beach at night and throw tridents" (after 93fe442f) |
| 2 | Cat | villages, swamp huts | `Feline` store (`world/felines.rs`, `feline.rs`, `feline_ai.rs`): saved, loaded, hatched and seen by friends; variant and voice drawn on `/summon` and eggs (black in swamp huts), structure cats come alive; float, panic, tempt with raw cod and salmon, flee from players while wild, leap and hunt, stroll and look; `AdultCatModel`, `BabyCatModel`, the collar layer, lying and relaxing poses (`cat_render.rs`) | taming, sitting, following, lying on beds and chests, gifts, creepers and phantoms flee, breeding, `CatSpawner` | M | `cats` | 29 (feline store) | drawn | "Minecraft cats and ocelots live in the entity world and are drawn like vanilla" (after c983b378) |
| 3 | Ocelot | jungles | shares `Feline`; spawns in jungles by its own rules (one in three, at or above sea level on grass or leaves, babies after the first of a group), tempted by raw cod and salmon, flees from players until it trusts, hunts chickens; `AdultOcelotModel`, `BabyOcelotModel` | trust by fish, scares creepers, breeding | S | `cats` | 29 (feline store) | drawn | "Minecraft cats and ocelots live in the entity world and are drawn like vanilla" (after c983b378) |
| 4 | Mule | horse x donkey | `DonkeyModel` scaled in `horse_render.rs` | `HorseKind::Mule`, breeding result | S | `horse-family` | | left | |
| 5 | Llama | windswept hills, savanna plateau (dormant today) | horse family | variants, carpets, chest, caravans, spit, `LlamaModel` | M | `horse-family` | | left | |
| 6 | Trader llama | with wandering traders | horse family | `TraderLlama`, follows its trader on a lead | S | `horse-family`, `wandering-trader` | | left | |
| 7 | Wandering trader | near players | villager model, trading port | goals, own trades, invisibility at night, despawn, spawner | M | `wandering-trader` | | left | |
| 8 | Snow golem | built | iron golem build check (`golem_spawn.rs`) | build pattern, snowballs, snow trail, melting, shearing | M | `built-golems` | | left | |
| 9 | Pillager | outposts, patrols, raids | piglin crossbow fight and arm poses | `Illager` store, crossbow goal, outpost spawns | M | `pillagers` | | left | |
| 10 | Vindicator | mansions, raids, patrols | | iron axe, "Johnny", door breaking in raids | S | `patrols-vindicators` | | left | |
| 11 | Evoker | mansions, raids | | spells, fangs, vex summon, wololo, totem drop | M | `evokers-vexes` | | left | |
| 12 | Vex | from evokers | | no-clip flight, limited life, `VexModel` | M | `evokers-vexes` | | left | |
| 13 | Ravager | raids | | roar, stun on a blocked shield, breaks leaves, illager riders | M | `ravagers` | | left | |
| 14 | Warden | deep dark, shriekers | dormant, sleep and fire lists | everything (D5) | XL | `warden-*` | | left | |
| 15 | Rabbit | taiga, snowy, desert, meadow, cherry (dormant today) | | jump control, carrot raids, biome variants, killer bunny, keyframes | M | `rabbits` | | left | |
| 16 | Fox | taiga, snowy taiga, grove (dormant today) | wolves hunt foxes (`monster_ai.rs:2796`) | sleeping by day, pounce, trust, mouth item, berries, keyframes | L | `foxes` | | left | |
| 17 | Goat | peaks, snowy slopes (dormant today) | | brain, ram, long jump, screaming, horns, milking | L | `goats` | | left | |
| 18 | Parrot | jungles (dormant today) | | P3, imitation, dancing, cookies, shoulder | L | `parrots` | | left | |
| 19 | Panda | jungles (dormant today) | | genes and personalities, bamboo, sneeze, roll | L | `pandas` | | left | |
| 20 | Turtle | beaches (dormant today) | zombies target turtle eggs (`monster_ai.rs:2060`) | home beach, eggs hatching, scute | L | `turtles` | | left | |
| 21 | Armadillo | savanna, badlands (dormant today) | | brain, roll up, scute and brushing, keyframes | M | `armadillos-polar-bears` | | left | |
| 22 | Polar bear | frozen biomes (dormant today) | | defends cubs, standing attack | M | `armadillos-polar-bears` | | left | |
| 23 | Camel | deserts, desert villages (dormant today) | | `HorseKind::Camel`, two riders, sit, dash, keyframes | L | `camels` | | left | |
| 24 | Camel husk | desert husk jockey (dormant today) | `type_info` entry | monster camel with a husk rider | S | `camels` | | left | |
| 25 | Squid | oceans, rivers | | `Squid` store, swim impulse, ink | M | `squids-water` | | left | |
| 26 | Glow squid | underground water (dormant today) | `type_info` entry | glow ink, glow | S | `squids-water` | | left | |
| 27 | Cod | oceans | | P2, `Fish` store, bucket | L | `fish` | | left | |
| 28 | Salmon | oceans, rivers | | `Fish` store, sizes, bucket | L | `fish` | | left | |
| 29 | Tropical fish | warm oceans, lush caves | | `Fish` store, patterns, bucket | L | `fish` | | left | |
| 30 | Pufferfish | warm oceans | | `Fish` store, puffing, poison, bucket | L | `fish` | | left | |
| 31 | Dolphin | oceans | | breathing, treasure leading, dolphin's grace | L | `dolphins` | | left | |
| 32 | Axolotl | lush caves (dormant today) | `type_info` entry | brain, playing dead, variants, bucket | L | `axolotls` | | left | |
| 33 | Frog | swamps (dormant today) | | brain, variants, tongue, froglights, keyframes | L | `frogs-tadpoles` | | left | |
| 34 | Tadpole | frogspawn | | hatching, bucket, growth | M | `frogs-tadpoles` | | left | |
| 35 | Guardian | ocean monuments | mining fatigue effect | `Guardian` store, laser, spikes | L | `guardians` | | left | |
| 36 | Elder guardian | ocean monuments | mining fatigue effect | elder curse, shares `Guardian` | L | `guardians` | | left | |
| 37 | Nautilus | oceans (26.3) | | brain, riding, dash, keyframes | L | `nautiluses` | | left | |
| 38 | Zombie nautilus | oceans (26.3) | | drowned rider, shares `Nautilus` | L | `nautiluses` | | left | |
| 39 | Phantom | after 3 sleepless nights | `Player.time_since_rest` | `PhantomSpawner`, swoop, daylight burning | M | `phantoms` | | left | |
| 40 | Zombie horse | plains, savanna, snowy plains | jockey built at `natural_spawner/spawning.rs:988`, dormant | `HorseKind::ZombieHorse`, rider (P4), rideable when tamed | M | `undead-horses` | | left | |
| 41 | Skeleton horse | lightning trap | | trap with P8, swimming | M | `undead-horses` | | left | |
| 42 | Silverfish | strongholds, infested blocks | stronghold spawner shows one (`spawner_display.rs:724`) | infested blocks, waking friends, merging into stone | M | `silverfish-endermites` | | left | |
| 43 | Endermite | ender pearls | | 5% pearl roll, endermen target, 2400-tick life | S | `silverfish-endermites` | | left | |
| 44 | Bee | flower forests, plains, nests | generator places nests with bees (`region.rs:259-263`) | P3, nests and hives, pollination, honey, anger | L | `bees` | | left | |
| 45 | Allay | outposts, mansions | | brain, P3, item collecting, note blocks, duplication | L | `allays` | | left | |
| 46 | Sniffer | sniffer eggs | | egg hatching, digging seeds, brain, keyframes | M | `sniffers` | | left | |
| 47 | Copper golem | built (26.3) | | build, sorting copper chests, oxidation, statue | L | `copper-golems` | | left | |
| 48 | Creaking | pale garden | `player_views` from endermen (`world.rs:2324`) | heart link, frozen while looked at, keyframes | L | `creakings` | | left | |
| 49 | Sulfur cube | sulfur caves (26.3) | `AbstractCubeMob` like the slime | bucket, shearing, archetypes, fuse | M-L | `sulfur-cubes` | | left | |
| 50 | Happy ghast | dried ghast (26.3) | ghast model | ghastling, harness, P3, four riders | L | `happy-ghasts` | | left | |
| 51 | Wither | built boss | dragon boss bar | everything | XL | `wither-live`, `wither-fight` | | left | |
| 52 | Breeze | trial chambers | wind charge burst (`wind.rs`) | brain, wind charges, `BreezeModel`, keyframes | L | `breezes`, `breeze-render` | | left | |

The last group (`mobs-wrapup`) checks that every spawn egg in the game hatches.

## Hit keys

A MW2 bullet finds a mob through one number: the kind in the top byte and the entity id below (`crates/minecraft_survival/src/entities.rs:1669` `encode`, `:1697` `decode`, the constants at `:1726-1733`).

- Taken: 0-16 (bat to ghast), 17 fireball, 18 blaze, 19 end crystal, 20 dragon, 21 piglin, 22 hoglin, 23 strider, 26 shulker, 27 shulker bullet, 28 item frame. 24 and 25 were reserved by round 8's track B and are unused, but they stay free for the other tracks.
- Track D takes **29 upward**, one byte per new `EntityWorld` store (one per vanilla family, not one per kind), in the order the groups land. The group writes its byte in the "Hit key" column of every row of the family.
- Drowned need none: they live in the zombie store and use byte 1.

| Byte | Store | Rows | Group |
|---|---|---|---|
| 29 | `felines` (`FelineEntity`, `MobHit::Feline`) | 2 cat, 3 ocelot | `cats` |
| 30 | (next free) | | |

## The batching rule

One `EntityWorld` store per vanilla family, never one per kind, so each family pays the wiring (about 55 match or chain sites) once. Extend an existing kind where vanilla shares a class.

- Existing families: `ZombieKind` (drowned already in it), `SkeletonKind`, `HoglinKind`, `Spider::cave`, horses and donkeys as `CowEntity` with `HorseState` (`third_party/minecraftoss/entities/src/horse.rs`), `Golem` (iron golem), the slime family (slime, magma cube).
- New families:
  - `HorseKind` += Mule, SkeletonHorse, ZombieHorse, Llama, TraderLlama, Camel, CamelHusk;
  - `Feline { Cat, Ocelot }`;
  - `Illager { Pillager, Vindicator, Evoker }`;
  - `Squid { Squid, GlowSquid }`;
  - `Fish { Cod, Salmon, TropicalFish, Pufferfish }`;
  - `Guardian { Guardian, Elder }`;
  - `Nautilus { Nautilus, ZombieNautilus }`;
  - `Golem` += snow golem and copper golem;
  - the slime family += sulfur cube (`AbstractCubeMob`).

## The mob pattern

Each kind or family goes in as a chain of commits, as round 8's piglins, hoglins, striders, shulkers and cave spiders did. Before writing a mob, javap its entity class, `EntityTypes`, the model and the renderer (and its goals or brain) and record the numbers in the track log, as `docs/round8/b2-hoglins.md` did.

Line numbers below are as of 24dfdf71; re-check them before editing.

1. **"X live in the entity world"** (hoglin 8f545881, shulker 116887f1)
   - `third_party/minecraftoss/entities/src/<mob>.rs`: state, attributes, saved fields, hurt and experience. AI in `<mob>_ai.rs`, or `<mob>_brain.rs` on `brain_behaviors.rs`.
   - `world/<mobs>.rs`: `XEntity`, `spawn_x`, `xs()`, `x_mut`.
   - `world.rs`: the store in `EntityWorld` (`:2298`), `EntityKey` (`:1893`) and `MobHit` (`:1955`) variants, and the arms in `body_mut` (`:4266`), `damage_state` (`:4315`), `bodies_mut` (`:4241`), `positions` (`:4774`), `mob_candidates` (`:3615`), `mob_hits_mob` (`:3693`), `entity_type` (`:4158`), `burning` (`:4806`), `clone_where` (`:4846`), `census` (`:4974`), `check_despawn` (`:5027`), `remove_where` (`:5088`), `resolve_voices` (`:714`), the death and loot list (`take_deaths`, `:3905`) and the projectile hit.
   - `world/combat.rs` (knockback), `witches.rs` (effects), `pushing.rs`, `hazards.rs` (`exposed!` for fire and drowning), `riding.rs` (prune); `monster_ai.rs` `is_enemy` (`:332`) when iron golems hunt it.
2. **"X are saved and spawn"** (61ad7269)
   - `crates/minecraft_terrain/src/server_mobs.rs`: `SIMULATED_KINDS` (`:409`), a `spawn_saved` arm (`:530`; unknown kinds fall to `_ => return None` at `:1122` and wait dormant), a `mob_tags_where` arm (`:1259`). Jockeys load only through `carries` (`:451`, striders today; P4 widens it).
   - `third_party/minecraftoss/world/src/natural_spawner.rs` `type_info` (`:267`): size, placement and rules. Without it natural spawning skips the kind (`natural_spawner/spawning.rs:497` `note_unsupported`) and a spawner allows it anywhere.
   - `natural_spawner/spawning.rs` per-kind finalize (`finalize_monster` `:893`, natural spawns and `/summon`), `third_party/minecraftoss/generator/src/feature/entities.rs` `finalize_spawn` (`:113`) for structure mobs.
3. **"X are drawn like vanilla"** (2c1b64c3, c9fa0c37)
   - A new `crates/minecraft_terrain/src/<mob>_render.rs`, ported from the model's `createBodyLayer` and `setupAnim` with `posed_cube::{Cube, Skin, append, part_pose}`. Templates: `hoglin_render.rs`, `piglin_render.rs`, `horse_render.rs`, `villager_render.rs`, `bat_render.rs`.
   - Textures in `mesh.rs` (`build_internal`, `:478+`), the pose push in `client_mobs.rs` (the per-store loops near `:1186-1240`), the shadow in `shadow_casters` (`:1263`), the render call in `crates/minecraft_survival/src/entity_view.rs` `meshes` (`:645`) and `boxes` (`:471`).
   - `crates/minecraft_survival/src/entities.rs` `boxes` (`:1470`) and `encode`/`decode` (`:1669`, `:1697`) with the family's hit-key byte (29 up, see above).
4. **Behaviour commits**, one vanilla feature each. Right-click goes in `crates/minecraft_terrain/src/mob_actions.rs` (`interact` `:132`, `would_interact` `:302`).
5. **Multiplayer:** `crates/minecraft_terrain/src/mob_tracking.rs` `tracking_range` (`:252`; vanilla `clientTrackingRange`: animals 10, monsters 8), `MobShow` fields (`:54`) for every synced value the model draws, `shows` (`:295`) and `show_on` (`:442`). `VOLATILE` (`:15`) strips Age, Health and Brain and `look()` (`:279`) adds `Baby`, so baby state comes from `ServerMob.baby`. The friend loads every kind from its look tag: `spawn_tracked` (`:542`) calls `spawn_saved`. Also `spawner_display.rs` (the kind list at `:29`, the render match near `:350`) and the egg survey: `HATCHING_EGGS` in `crates/minecraft_terrain/src/server.rs` (`:6358`, test `:6364`).
6. **Docs:** the rows above and the track log (`docs/round9/track-d-log.md`): javap numbers, tests, the owner's numbered steps and the kit line.

Every kind must: load on the friend from its look tag, have a `tracking_range` entry, show every drawn state through `MobShow`, get a hit-key byte (or share its family's), join `HATCHING_EGGS`, and work for host and friend alike. The friend runs no AI. Textures, model data, loot, trades and animations are read at runtime from the 26.3 jar and packs.

Tests for every family:
- `minecraftoss-entities` `world::tests`: `a_<mob>_is_<size>_with_<health>_and_its_kind` and one behaviour test per vanilla feature.
- `minecraft_terrain`: `server_mobs::tests::a_<mob>_saves_and_loads_with_<fields>`, `mob_tracking::tests::a_friend_sees_a_<mob>_from_its_look_tag`, `<mob>_render::tests`, a `type_info` rules test, `HATCHING_EGGS` grows by each kind.
- `minecraft_survival`: `entities::tests::a_<mob>_hit_box_encodes_and_decodes`.

## Shared prerequisites

Each is built once, in the first group that needs it.

| | Prerequisite | Built in group | Used by |
|---|---|---|---|
| P1 | Keyframe animations: the linear sampler moves out of `bat_render.rs` into a new `crates/minecraft_terrain/src/keyframes.rs` with `LINEAR`, `CATMULLROM` and an `AnimationState` start tick; the `AnimationDefinition`s are read from the jar at runtime with `class_file.rs` | `keyframes` (before the warden's model) | warden, armadillo, axolotl, rabbit, breeze, camel, copper golem, creaking, fox, frog, nautilus, sniffer |
| P2 | Water mobs: `WaterBoundPathNavigation`, `SmoothSwimmingMoveControl`, fish flopping on land, air supply for water breathers, buckets (`Bucketable`, `*_bucket` items with `bucket_entity_data`) | `squids-water` | squids, fish, dolphins, axolotls, tadpoles, guardians, nautiluses |
| P3 | Flying mobs: `FlyingPathNavigation` and `FlyingMoveControl` | `parrots` | parrots, bees, allays, happy ghasts |
| P4 | Riding: `carries()` and `riding.rs` beyond striders so jockeys load (zombie horse, camel husk, zombie nautilus, ravager riders); player riding for the horse family (taming, saddle, steering, the horse inventory screen); two camel seats later | `riding` | horse family, camels, ravagers, nautiluses, happy ghasts |
| P5 | Player effects with HUD icons: darkness, bad omen, raid omen, trial omen, hero of the village (later dolphin's grace) | `effects-omens` | raids, the warden, trial chambers, dolphins |
| P6 | Custom spawners on the host's level tick: `CatSpawner`, `WanderingTraderSpawner`, `PatrolSpawner`, `PhantomSpawner`; their state saved where vanilla saves it | `cats` (first) | cats, wandering traders, patrols, phantoms |
| P7 | Game events and vibrations | `vibrations`, `vibrations-more` | the warden, sculk, allays |
| P8 | Natural lightning in thunderstorms (`ServerLevel.tickThunder`) | `undead-horses` | skeleton horse trap, charged creepers |
| P9 | The trading screen | `trading-*` | villagers, wandering traders |
| P10 | Raids | `raids`, `raid-villagers` | pillagers, vindicators, evokers, ravagers, witches in raids |
