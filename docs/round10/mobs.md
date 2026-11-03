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
| 14 | Warden | deep dark, shriekers | dormant, sleep and fire lists; darkness effect; `WardenAnimation` readable; ancient cities generate with shriekers and loot; `wardens` family: attributes, anger, its vibration listener at its eyes (kept in `minecraft_terrain`), brain (emerge, dig, investigate, sniff, idle), touches, digging away; saved with its anger, memories and listener, hatched, summoned, dug out by shriekers (`try_spawn_mob`, `ON_TOP_OF_COLLIDER`, the monster light rule), seen by friends (pose and anger in `FamilyShow`); roars at what angers it, fights it in melee (30, a raised shield down for 5 s), angers at whatever hurts it (100); sonic booms through walls (10 past armour and shields, the push, rings for every viewer), darkness pulses within 20 every 6 s; drawn for host, friends and spawner cages (`warden_render.rs`: the model, its five glowing layers on their own parts, the six `WardenAnimation` clips through the shared keyframe holder, the clips' clocks, the pose and the anger in `FamilyShow`); its heartbeat for every viewer and the ground thrown up while it digs or climbs out | | XL | `wardens` | 44 | done | e3495a40, f07e78d2, 4f1e81de, beeb9a11, 7f221998, "Minecraft wardens' hearts beat and their digging throws up the ground" (W `warden-render`) |

## M1: land animals (group `land`, bytes 48-63)

| # | Mob | Where the player meets it | Exists beyond the default | Left | Size | Family | Hit key | Status | Commits |
|---|---|---|---|---|---|---|---|---|---|
| 15 | Rabbit | taiga, snowy, desert, meadow, cherry | spawn row, rules and group variants; `rabbits` family: saved, loaded, hatched and seen by friends; jump control, fleeing, panic, carrot temptation, breeding with the biome's colour, growth and the golden dandelion lock; carrot raids; the killer bunny (armour, bite, leap, targets, hostile sounds, name); wild cats and wolves hunt them; adult and baby models, 16 textures, hop and head-tilt keyframes | the hop's dust, the bite's crumbs and sound (level event 2001), W's `block_change` for the bite, name tags and the egg's variant component | M | `rabbits` | 48 | done | 8ad4c103, 39d9b5bc, e862fc31, d7ae60a2, 8afa0650, d008fa7e, ed19a463, ee0a08da, 9bfbc35e (M1 `rabbits-live`, `rabbits-drawn`, `rabbits-behaviour`) |
| 16 | Fox | taiga, snowy taiga, grove | spawn row and finalize (built in); `foxes` family: saved, loaded, hatched and seen by friends (the flag byte through `FamilyShow`, the interested angle and crouch amount stepped from it); floating, the powder snow climb, panic, fleeing walking players, wild wolves and polar bears, seeking shelter from the sun and from thunder, sleeping by day under cover and waking when something walks near, in thunder or in water, perching and looking about, the village stroll at night, following its parent, the sleep and screech sounds, the lead, the mouth item saved and dropped on death, walking through berry bushes unhurt (the spared-blocks hook), a 5-block safe fall; wolves hunt foxes; the adult and cub models with their eight textures (the sleep skins with shut eyes), the walking legs and the cub's walk keyframes, the head tilt, crouch, sleep, sit, pounce and faceplant poses and the pounce tilt, the lead at the neck, the mouth item drawn at the nose on the ground display (laid aside while asleep, smaller for cubs), the same for friends | stalking, the pounce and faceplants, the bite, picking up, spitting and eating items, berries, breeding and trust, defending trusted players, the equip game events (W's w3) | L | `foxes` | 49 | drawn | 39a52395, e41f0c0a, 7459c74e (M1 `foxes-live`), a374c030, "Minecraft foxes show what they carry in their mouths" (M1 `foxes-drawn`) |
| 17 | Goat | peaks, snowy slopes (dormant today) | size row (rules missing, never spawns) | brain, ram, long jump, screaming, horns, milking | L | `goats` | 50 | left | |
| 18 | Parrot | jungles (dormant today) | size row | flying (P3 from D0), imitation, dancing, cookies, shoulder | L | `parrots` | 51 | left | |
| 19 | Panda | jungles, bamboo jungles | spawn row with the animal rules, its genes, the weak and lazy attributes and a cub one time in five; `pandas` family: saved, loaded, hatched and seen by friends (sit, back, roll, sneeze and scared flags, the unhappy counter and the held item through `FamilyShow`); genes and variants, panic from fire, breeding next to bamboo with genes from the parents, sulking without it, the bite back, aggressive pandas joining a fight, worried pandas fleeing players and monsters, bamboo temptation and feeding, growth and the golden dandelion lock; the adult and cub models with their 14 textures, every pose (sitting, eating paws, scared, on its back, rolling, sneezing, sulking) with the sit, back and roll amounts eased like vanilla, and the held item on the ground display, the same for friends (the sneeze, eat and roll counters through `FamilyShow`); sitting down to eat: the sit goal fetching bamboo and cake it sees, picking up a whole stack with `mob_griefing`, eating from the hand (its old item dropped), held still while it cannot act, munching with the eat sound and the held item's crumbs every fifth tick (for friends too), pausing after 80 ticks and finishing after 100 with the `eat` game event, dropping what it holds when it gets up, the held item's death drop, worried pandas never settling; moods: lazy pandas lying on their backs until a click gets them up, playful pandas and cubs somersaulting with vanilla's hops and rolling off ledges, cubs sneezing (weak ones often) with the pre-sneeze and sneeze sounds, the green sneeze cloud for every viewer, the adults within 10 blocks hopping and the `gameplay/panda_sneeze` gift with `mob_drops` (through the gift hook); thunder: worried pandas out of water sit with their faces covered and tremble while the level thunders (thunder times rain above 0.9, through the weather hook), stop eating, take no bamboo and cannot act, and stand up when it passes, the same for friends (the scared flag) | egg pandas pick up nothing (`CanPickUpLoot` false for eggs, fork-wide), dispensers filling the paw, the pickup's flight animation, the sneeze cloud drifting to a player's feet | L | `pandas` | 52 | done | 58f52911, 85551190, b6ec8731, f1972742, 8f047f0d, e7926cf6, 87275971, da88e0e0, f2fda2e4, "Minecraft worried pandas hide from thunder" (M1 `pandas-live`, `pandas-drawn`, `pandas-eating`, `pandas-moods`, `pandas-thunder`) |
| 20 | Turtle | beaches (dormant today) | size row; zombies' baby-turtle target slot | home beach, eggs hatching, scute | L | `turtles` | 53 | left | |
| 21 | Armadillo | savanna, badlands (dormant today) | spawn row and finalize | brain, roll up, scute and brushing, keyframes | M | `armadillos` | 54 | left | |
| 22 | Polar bear | frozen biomes, frozen oceans | spawn row with its sea-ice rule, also at chunk generation; `polar_bears` family: saved, loaded, hatched and seen by friends (standing through `FamilyShow`); panic (cubs from any hit, adults from fire), cubs following adults, the standing warning and the six-point swipe, hit cubs calling adults, attacking players near cubs, hunting foxes, `NeutralMob` anger saved with `angry_at`, the cub's own ambient sound, the golden dandelion lock; the adult model scaled 1.2 and the cub's own model with their two textures, the standing pose lerped by the partial tick, the same pose for friends | `mustReach`, the 0.98 water slow-down, the standing bear's taller box, freeze immunity | M | `polar_bears` | 55 | done | 4d6f6a32, feed5dab, fa44b301, 84d52a57, 1c672729 (M1 `polar-bears-live`, `polar-bears-drawn`) |
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
