# Round 10 mob list: the 38 mobs left after round 9

Rows 1-13 and 51 of `docs/round9/mobs.md` are done. This file holds the rest, one section per round 10 track; each track edits only its own section, in the same commit as the code. Status: `left`, `live` (in the entity world, saved, loaded, hatched and seen by friends), `drawn`, `done` (its vanilla behaviours, spawning and the owner's steps written). How a family plugs in: `docs/round10/track-d0.md`, D0-3 (the plug-in checklist). "Exists" lists only what goes beyond the default: a texture, a loot table, sounds, an unported model class and a `/summon` tag exist for every one.

## Hit keys

A MW2 bullet finds a mob through one number: the kind in the top byte, the entity id below (`crates/minecraft_survival/src/entities.rs` `encode`, `decode`). A registered family's byte is its `Family::KEY`.

| Bytes | Owner |
|---|---|
| 0-35 | round 9 and earlier (24 and 25 reserved, unused) |
| 36-39 | D0, group `pests`: 36 silverfish, 37 endermites, 39 the test probe |
| 40-43 | D, group `trial`: 40 breezes |
| 44-47 | W, group `deep_dark`: 44 wardens |
| 48-63 | M1, group `land` |
| 64-79 | M2, group `water` |
| 80-95 | M3, group `night` (camels, camel husks and the undead horses use the cow store's byte 11) |
| 96-99 | F, group `bosses`: 96 Elden Ring bosses |
| 100-255 | free |

## F: Elden Ring bosses (group `bosses`)

Not a vanilla mob: the boss of track F (`docs/round10/track-f.md`, `docs/ELDEN-RING-BOSSES.md`). His fight logic is the pure crate `crates/elden_boss`; he is saved as a vanilla `minecraft:marker` carrying `data.iw4l`.

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| F1 | Margit, the Fell Omen | `/summon iw4l:margit`, and later the altar | the brain, moves, phases and arena rules (`elden_boss`, segment f1) | walking, model, strikes, reward, MW2 resistances, bar, friends' show, altar | XL | `elden_bosses` | 96 | live | 03534171, "Minecraft weapons hurt Margit with no hurt cooldown, and his stance breaks" |

## D0: the registry's proof (group `pests`)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 42 | Silverfish | strongholds, infested blocks | stronghold spawner shows one; strongholds place infested stone bricks; sleep list; the `infested` potion is listed | the Infested effect | M | `silverfish` | 36 | done | bdbc8852, 757bfed8, 2842608b, 126b07a1, f8f03659, 85917c3c, "Minecraft silverfish leave a puff when they merge into stone" |
| 43 | Endermite | ender pearls | sleep list; the enderman's endermite target slot rolls with no prey | none | S | `endermites` | 37 | done | bdbc8852, 757bfed8, 2842608b, 126b07a1, f8f03659, "Endermites come from ender pearls" |

## D: trial chambers (group `trial`)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 52 | Breeze | trial chambers | wind charge burst (`wind.rs`), wind charges trigger blocks (f142f9c1), `BreezeAnimation` readable | | L | `breezes` | 40 | done | ae5bcf0e, 0c83047f, a1ddd5ae, fbf3b5cc, "Minecraft breezes are drawn with their wind and eyes" |

## W: the deep dark (group `deep_dark`)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 14 | Warden | deep dark, shriekers | dormant, sleep and fire lists; darkness effect; `WardenAnimation` readable; ancient cities generate with shriekers and loot | everything | XL | `wardens` | 44 | left | |

## M1: land animals (group `land`, bytes 48-63)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 15 | Rabbit | taiga, snowy, desert, meadow, cherry | spawn row, rules and group variants; `rabbits` family: saved, loaded, hatched and seen by friends; jump control, fleeing, panic, carrot temptation, breeding with the biome's colour, growth and the golden dandelion lock; carrot raids; the killer bunny (armour, bite, leap, targets, hostile sounds, name); wild cats and wolves hunt them; adult and baby models, 16 textures, hop and head-tilt keyframes | the hop's dust, the bite's crumbs and sound (level event 2001), W's `block_change` for the bite, name tags and the egg's variant component | M | `rabbits` | 48 | done | 8ad4c103, 39d9b5bc, e862fc31, d7ae60a2, 8afa0650, d008fa7e, ed19a463, ee0a08da, 9bfbc35e (M1 `rabbits-live`, `rabbits-drawn`, `rabbits-behaviour`) |
| 16 | Fox | taiga, snowy taiga, grove (dormant today) | spawn row and finalize; wolves hunt foxes | sleeping by day, pounce, trust, mouth item, berries, keyframes | L | `foxes` | 49 | left | |
| 17 | Goat | peaks, snowy slopes (dormant today) | size row (rules missing, never spawns) | brain, ram, long jump, screaming, horns, milking | L | `goats` | 50 | left | |
| 18 | Parrot | jungles (dormant today) | size row | flying (P3 from D0), imitation, dancing, cookies, shoulder | L | `parrots` | 51 | left | |
| 19 | Panda | jungles (dormant today) | spawn row with the animal rules | genes and personalities, bamboo, sneeze, roll | L | `pandas` | 52 | left | |
| 20 | Turtle | beaches (dormant today) | size row; zombies' baby-turtle target slot | home beach, eggs hatching, scute | L | `turtles` | 53 | left | |
| 21 | Armadillo | savanna, badlands (dormant today) | spawn row and finalize | brain, roll up, scute and brushing, keyframes | M | `armadillos` | 54 | left | |
| 22 | Polar bear | frozen biomes, frozen oceans | spawn row with its sea-ice rule, also at chunk generation; `polar_bears` family: saved, loaded, hatched and seen by friends (standing through `FamilyShow`); panic (cubs from any hit, adults from fire), cubs following adults, the standing warning and the six-point swipe, hit cubs calling adults, attacking players near cubs, hunting foxes, `NeutralMob` anger saved with `angry_at`, the cub's own ambient sound, the golden dandelion lock; the adult model scaled 1.2 and the cub's own model with their two textures, the standing pose lerped by the partial tick, the same pose for friends | `mustReach`, the 0.98 water slow-down, the standing bear's taller box, freeze immunity | M | `polar_bears` | 55 | done | 4d6f6a32, feed5dab, fa44b301, 84d52a57, 1c672729 (M1 `polar-bears-live`, `polar-bears-drawn`) |
| 46 | Sniffer | sniffer eggs | | egg hatching, digging seeds, brain, keyframes | M | `sniffers` | 56 | left | |
| 44 | Bee | flower forests, plains, nests | generator places nests with bees | flying (P3 from D0), nests and hives, pollination, honey, anger | L | `bees` | 57 | left | |

## M2: water and buckets (group `water`, bytes 64-79)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 25 | Squid | oceans, rivers | spawn row with the surface water rule, babies one time in twenty; `squids` family in the entity world: tentacle pulses, turning, tilting and rolling, the random heading, fleeing, the squirt with its ink counter, gills, growth, leads; saved, loaded, hatched and seen by friends (the host's tentacles, tilt and roll through `FamilyShow`), tracked at 8 chunks, the golden dandelion lock; adult and baby models without a death flip, in the spawner cage too; the ink cloud for every view from the squirt counter (black, drifting, fading, sinking in air) | impaling's extra damage (no mob has it yet) | M | `squids` | 64 | done | 2c0d74bf, ddb173f4, efd113a9, d11758b3, 93f77e9b, "Minecraft squids leave ink clouds and glow squids sparkle, for friends too" (M2 `squids-live`, `squids-drawn`) |
| 26 | Glow squid | underground water | spawn row with the dark deep water rule; shares the `squids` family, with its dark ticks after a squirt; tracked at 10 chunks (`TerrainFamily::tracking_range`), its own textures and full-bright light that dims to the water's for its dark ticks; teal glow ink and a glow sparkle a tick | impaling's extra damage (no mob has it yet) | S | `squids` | 64 | done | 2c0d74bf, ddb173f4, efd113a9, d11758b3, 93f77e9b, "Minecraft squids leave ink clouds and glow squids sparkle, for friends too" (M2 `squids-live`, `squids-drawn`) |
| 27 | Cod | oceans | category only; its bucket pours water and lets out a dormant cod | swimming (P2), schools, flopping, bucket | L | `fish` | 65 | left | |
| 28 | Salmon | oceans, rivers | category only; its bucket pours water and lets out a dormant salmon | sizes, bucket | L | `fish` | 65 | left | |
| 29 | Tropical fish | warm oceans, lush caves | category only; its bucket pours water and lets out a dormant fish | patterns, bucket | L | `fish` | 65 | left | |
| 30 | Pufferfish | warm oceans | category only; its bucket pours water and lets out a dormant pufferfish | puffing, poison, bucket | L | `fish` | 65 | left | |
| 31 | Dolphin | oceans | category only | breathing, treasure leading, dolphin's grace | L | `dolphins` | 66 | left | |
| 32 | Axolotl | lush caves (dormant today) | size row; its bucket pours water and lets out a dormant axolotl | brain, playing dead, variants, bucket | L | `axolotls` | 67 | left | |
| 33 | Frog | swamps (dormant today) | size row | brain, variants, tongue, froglights, keyframes | L | `frogs` | 68 | left | |
| 34 | Tadpole | frogspawn | its bucket pours water and lets out a dormant tadpole | hatching, bucket, growth | M | `tadpoles` | 69 | left | |
| 35 | Guardian | ocean monuments | mining fatigue effect | laser, spikes | L | `guardians` | 70 | left | |
| 36 | Elder guardian | ocean monuments | mining fatigue effect | elder curse, shares `guardians` | L | `guardians` | 70 | left | |
| 37 | Nautilus | oceans (26.3) | | brain, riding (D0 mount hooks), dash, keyframes | L | `nautiluses` | 71 | left | |
| 38 | Zombie nautilus | oceans (26.3) | | drowned rider, shares `nautiluses` | L | `nautiluses` | 71 | left | |
| 49 | Sulfur cube | sulfur caves (26.3) | `AbstractCubeMob` like the slime; `Bucketable`, `Shearable`; its bucket lets out a dormant cube | bucket, shearing, archetypes, fuse | M-L | `sulfur_cubes` | 72 | left | |

Key 79 is M2-1's `#[cfg(test)]` probe `minecraft:swim_probe` (`world/families/water/swimming/swim_probe.rs`); 73-78 stay free. M2-1's shared hooks, which make no mob live on their own: `Family::breath` with vanilla's lungs, the water animals' gills (squids, the four fish, tadpoles) and a family's own air (axolotls, nautiluses), the dolphin's 4800 air as lungs ("Registered Minecraft mobs breathe like vanilla water animals", after 4c42b0ae). `Family::swim_sounds` and `Family::quiet_steps`: a family's own swim and splash, and no steps for the fish, tadpoles and nautiluses, which still swim aloud ("Registered Minecraft mobs make their own swim and step sounds", after 0fa9ec48). M2-1's water paths, which M2's swimmers and M1's turtles use: the A* over any node evaluator ("Minecraft path search takes any node evaluator", after 287abc2f), vanilla's swim evaluator with the dolphin's breaching, the amphibious mode of the walk search with the shallow preference, `WaterNav` with `tick_water` in the navigation, and `water_bound_random_position` with an optional home ("Minecraft mobs find paths through water like vanilla swimmers and amphibians", after d0c0cd7b). M2-1's mob buckets: the seven bucket rows (`world/src/level/mob_bucket.rs`), emptying one pours its water (none for the sulfur cube) with its own splash and lets out its mob from the bucket's data with `FromBucket`, waiting dormant until its family is live ("Minecraft mob buckets pour their water and let out the mob they hold", after 166f11a8). Picking a mob up: `Bucketable` with its `PICKUP_ITEM` (a water bucket, an empty one for the sulfur cube) and `pickup::<S>`, which the fish, axolotl, tadpole and sulfur cube families call first in their `interact` ("Minecraft fish buckets carry the mob they caught like vanilla", after 1322d5fb). Dispensers empty the six water mob buckets the same way ("Dispensers empty fish buckets like vanilla", after 6c0c1ea0). `Family::OWN_DYING_TRAVEL`: a family whose class moves by its own `travel` keeps moving its dead mobs itself instead of the shared sinking and drag (squids, then dolphins and axolotls; "Registered Minecraft mobs can move by their own rules while dying", after 48f5c4bb). For every water row: water in an `InWater` mob's spawn box no longer refuses its spawn (vanilla's `isUnobstructed`), and `surface_water_animal` in `natural_spawner/families/water.rs` is the shared surface rule for squids, the fish and dolphins (after ddb173f4). `TerrainFamily::tracking_range(kind)`: a family's tracking range by kind, by default its `TRACKING_RANGE` (the glow squid's 10, then the elder guardian's; "Registered Minecraft mobs can have a tracking range per kind", after efd113a9).

## M3: night, built mobs and the horse family (group `night`, bytes 80-95)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 39 | Phantom | after 3 sleepless nights | `Player.time_since_rest`; custom spawners (P6); cats exist | `PhantomSpawner`, swoop, daylight burning, cats scare them; the terrain test `live_shulkers_stay_far_from_players_while_dormant_monsters_go` uses the phantom as its dormant monster and must switch kind or drop that half | M | `phantoms` | 80 | left | |
| 40 | Zombie horse | plains, savanna, snowy plains | spawn row, rules and the jockey finalize (dormant) | `HorseKind::ZombieHorse`, rider, rideable when tamed | M | cow store (`HorseKind`) | 11 | left | |
| 41 | Skeleton horse | lightning trap | | trap with natural lightning (P8, M3), swimming | M | cow store (`HorseKind`) | 11 | left | |
| 23 | Camel | deserts, desert villages (dormant today) | size row | `HorseKind::Camel`, two riders, sit, dash, keyframes | L | cow store (`HorseKind`) | 11 | left | |
| 24 | Camel husk | desert husk jockey (dormant today) | size row; the husk's finalize builds the jockey | monster camel with a husk rider | S | cow store (`HorseKind`) | 11 | left | |
| 45 | Allay | outposts, mansions | | brain, flying (P3 from D0), item collecting, note blocks (needs W's game events), duplication | L | `allays` | 81 | left | |
| 47 | Copper golem | built (26.3) | iron and snow golem builds (`golem_build.rs`) | build, sorting copper chests, oxidation, statue | L | `copper_golems` | 82 | left | |
| 48 | Creaking | pale garden | `player_views` from endermen | heart link, frozen while looked at, keyframes | L | `creakings` | 83 | left | |
| 50 | Happy ghast | dried ghast (26.3) | ghast model | ghastling, harness, flying, four riders (D0 mount hooks) | L | `happy_ghasts` | 84 | left | |
