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
| 96-99 | F, group `bosses` |
| 100-255 | free |

## D0: the registry's proof (group `pests`)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 42 | Silverfish | strongholds, infested blocks | stronghold spawner shows one; strongholds place infested stone bricks; sleep list; the `infested` potion is listed | infested blocks, waking friends, merging into stone | M | `silverfish` | 36 | drawn | bdbc8852, 757bfed8, 2842608b, 126b07a1, "Minecraft silverfish and endermites are drawn like vanilla" |
| 43 | Endermite | ender pearls | sleep list; the enderman's endermite target slot rolls with no prey | 5% pearl roll | S | `endermites` | 37 | drawn | bdbc8852, 757bfed8, 2842608b, 126b07a1, "Minecraft silverfish and endermites are drawn like vanilla" |

## D: trial chambers (group `trial`)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 52 | Breeze | trial chambers | wind charge burst (`wind.rs`), wind charges trigger blocks (f142f9c1), `BreezeAnimation` readable | brain, wind charges, `BreezeModel`, keyframes | L | `breezes` | 40 | left | |

## W: the deep dark (group `deep_dark`)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 14 | Warden | deep dark, shriekers | dormant, sleep and fire lists; darkness effect; `WardenAnimation` readable; ancient cities generate with shriekers and loot | everything | XL | `wardens` | 44 | left | |

## M1: land animals (group `land`, bytes 48-63)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 15 | Rabbit | taiga, snowy, desert, meadow, cherry (dormant today) | spawn row, rules and group variants | jump control, carrot raids, biome variants, killer bunny, keyframes | M | `rabbits` | 48 | live | 8ad4c103, 39d9b5bc, "Minecraft rabbits breed and their babies grow up" (M1 `rabbits-live`) |
| 16 | Fox | taiga, snowy taiga, grove (dormant today) | spawn row and finalize; wolves hunt foxes | sleeping by day, pounce, trust, mouth item, berries, keyframes | L | `foxes` | 49 | left | |
| 17 | Goat | peaks, snowy slopes (dormant today) | size row (rules missing, never spawns) | brain, ram, long jump, screaming, horns, milking | L | `goats` | 50 | left | |
| 18 | Parrot | jungles (dormant today) | size row | flying (P3 from D0), imitation, dancing, cookies, shoulder | L | `parrots` | 51 | left | |
| 19 | Panda | jungles (dormant today) | spawn row with the animal rules | genes and personalities, bamboo, sneeze, roll | L | `pandas` | 52 | left | |
| 20 | Turtle | beaches (dormant today) | size row; zombies' baby-turtle target slot | home beach, eggs hatching, scute | L | `turtles` | 53 | left | |
| 21 | Armadillo | savanna, badlands (dormant today) | spawn row and finalize | brain, roll up, scute and brushing, keyframes | M | `armadillos` | 54 | left | |
| 22 | Polar bear | frozen biomes (dormant today) | size row | defends cubs, standing attack | M | `polar_bears` | 55 | left | |
| 46 | Sniffer | sniffer eggs | | egg hatching, digging seeds, brain, keyframes | M | `sniffers` | 56 | left | |
| 44 | Bee | flower forests, plains, nests | generator places nests with bees | flying (P3 from D0), nests and hives, pollination, honey, anger | L | `bees` | 57 | left | |

## M2: water and buckets (group `water`, bytes 64-79)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 25 | Squid | oceans, rivers | category only | swimming (P2), swim impulse, ink | M | `squids` | 64 | left | |
| 26 | Glow squid | underground water (dormant today) | spawn row and rules | glow ink, glow | S | `squids` | 64 | left | |
| 27 | Cod | oceans | category only | swimming (P2), schools, flopping, bucket | L | `fish` | 65 | left | |
| 28 | Salmon | oceans, rivers | category only | sizes, bucket | L | `fish` | 65 | left | |
| 29 | Tropical fish | warm oceans, lush caves | category only | patterns, bucket | L | `fish` | 65 | left | |
| 30 | Pufferfish | warm oceans | category only | puffing, poison, bucket | L | `fish` | 65 | left | |
| 31 | Dolphin | oceans | category only | breathing, treasure leading, dolphin's grace | L | `dolphins` | 66 | left | |
| 32 | Axolotl | lush caves (dormant today) | size row | brain, playing dead, variants, bucket | L | `axolotls` | 67 | left | |
| 33 | Frog | swamps (dormant today) | size row | brain, variants, tongue, froglights, keyframes | L | `frogs` | 68 | left | |
| 34 | Tadpole | frogspawn | | hatching, bucket, growth | M | `tadpoles` | 69 | left | |
| 35 | Guardian | ocean monuments | mining fatigue effect | laser, spikes | L | `guardians` | 70 | left | |
| 36 | Elder guardian | ocean monuments | mining fatigue effect | elder curse, shares `guardians` | L | `guardians` | 70 | left | |
| 37 | Nautilus | oceans (26.3) | | brain, riding (D0 mount hooks), dash, keyframes | L | `nautiluses` | 71 | left | |
| 38 | Zombie nautilus | oceans (26.3) | | drowned rider, shares `nautiluses` | L | `nautiluses` | 71 | left | |
| 49 | Sulfur cube | sulfur caves (26.3) | `AbstractCubeMob` like the slime; `Bucketable`, `Shearable` | bucket, shearing, archetypes, fuse | M-L | `sulfur_cubes` | 72 | left | |

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
