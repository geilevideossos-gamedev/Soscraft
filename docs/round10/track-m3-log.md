# Track M3 log: night mobs, lightning and the horse family

Branch `track-m3`, worktree `C:\Dev\Soscraft\mc-wt\track-m3`, target dir `D:\iw4l-wt\track-m3`. Brief: `docs/round10/track-m3.md`. Mob list: `docs/round10/mobs.md` (the `## M3` rows only). Registry: `docs/round10/track-d0.md` (D0-2, D0-3).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Vendored files

Files under `third_party/minecraftoss` this track patched or added.

- Added (lightning-strikes 1): `world/src/level/lightning.rs` (with its tests), `entities/src/world/lightning.rs` (with its tests).
- Patched (lightning-strikes 1): `world/src/level/mod.rs` (`pub mod lightning;`, the `lightning_hits` field), `world/src/level/entity.rs` (`LightningData.seed` and `uuid`, `Entity::new` is `pub(super)`, the bolt tick's fire and hits, the copper walk's 3002), `world/src/level/fire.rs` (`fire_survives` and `can_spread_fire_around` are `pub(super)`), `world/src/level/redstone.rs` (the rod's 3002), `entities/src/inside_blocks.rs` (`Hazard::LightningBolt`), `entities/src/world.rs` (`mod lightning;` and its `pub use`), `entities/src/world/pushing.rs` (`exposed_mut` is `pub(super)`).
- Patched (lightning-strikes 2): `entities/src/world/families.rs` (the `thunder_hit` hook), `entities/src/world/families/pests/probe.rs` (test only: `thunder_answers`, `thunder_bolts` and its override). Changed again: `entities/src/world/lightning.rs` (the hook call and its test).
- Patched (lightning-turns 1): `world/src/natural_spawner/spawning.rs` (the witch's arm in `finalize_conversion`, after the villager's). Changed again: `entities/src/world/lightning.rs` (the turns, the creeper's charge, the mooshroom's flip and their tests).
- Added (phantoms-live 1): `entities/src/world/families/night/phantoms.rs`, `entities/src/world/families/night/phantoms/goals.rs`, `entities/src/world/families/night/phantoms/tests.rs`.
- Patched (phantoms-live 1): `entities/src/world/families/night.rs` (`pub mod phantoms;` and the `family_group!` entry), `world/src/natural_spawner/families/night.rs` (the phantom row, its rule, its finalize and its test).
- Changed again (phantoms-live 2): `entities/src/world/families/night/phantoms.rs`, `night/phantoms/goals.rs` and `night/phantoms/tests.rs` (burning, the cat search and their tests).
- Changed again (phantoms-spawner 1): `world/src/natural_spawner/families/night.rs` (`CreatureSpawns::spawn_phantoms` and its test; the test level gains darkness, sky light, sea level and a block layer up high). Patched: `world/src/level/spawning.rs` (`Level::spawn_phantoms` after `spawn_patrol`).

## Hooks

Root hooks this track added (M30, "M3's root hooks").

- **"Registered Minecraft mobs can answer lightning their own way"** (lightning-strikes 2, entities root `world/families.rs`): `Family::thunder_hit(mob, bolt: &Bolt) -> bool` after `fire_immune`, default `false` (`true`: the family took the whole hit, so the base hit is skipped); `FamilyStore::thunder_hit`, the `family_group!` dispatch and `Families::thunder_hit(key, id, bolt) -> Option<bool>` beside `deflect`. `EntityWorld::thunder_hit` asks it for each registered mob alive in the bolt's box. M1's turtles and M3's copper golems override it.

## Protocol

`minecraft_net::PROTOCOL` is not bumped here. `Output.players_lightning` is the server thread's output to the host, not a network message; `Hazard` and `DamageCause` are not on the wire. `ThunderOutcome` (lightning-turns) stays on the host; the charged creeper and the mooshroom's colour reach friends through the look tag (`powered`, `Type`), which is not a message change.

- lightning-drawn 1 ("Minecraft lightning bolts flash and thunder for every player", after 82a5cd0b): `EntitySnapshot` gains `lightning: Vec<minecraft_terrain::lightning::BoltView>` after `crystals`, with `BoltView { id: i32, position: DVec3, seed: i64, life: i32 }`. It rides in `StateFrame.entities`, so old and new builds cannot read each other's entity snapshots.
- phantoms-spawner 1: no message change. `TickInput.rest` is the host's own input to its server thread; friends' clocks stay on the host with their saved player.

## Cow-store one-liners outside D0-3's list

None yet.

## State doc changes

For the orchestrator to copy at the merge of segment m3a:

- Under World simulation in `docs/MINECRAFT-SURVIVAL.md`: "Lightning (round 10, track M3): `/summon minecraft:lightning_bolt` strikes like vanilla's bolt. On Normal and Hard it lights its block and up to four around it, each re-flash lights its own block on any difficulty, and fire in an empty obsidian frame lights the portal. Every lit tick it hits every living thing in its box (3 blocks around, 3 below, 9 above): 5 damage that armour softens and shields do not block, with no knockback, and one more fire tick (eight seconds of fire for a mob or player that had just stopped burning). Animals panic, fire-immune mobs are hurt too, dropped items and experience orbs burn. A struck lightning rod powers for 8 ticks and sends the electric spark event (3002) along its axis; struck copper is cleaned with a spark on each cleaned block. Lightning deaths read 'was struck by lightning'."
- Under World simulation in `docs/MINECRAFT-SURVIVAL.md`, after the lightning-turns line (lightning-drawn): "Every player sees and hears a bolt as vanilla draws it: a jagged blue-white bolt 128 blocks tall from its seed, drawn within 64 blocks of the camera and changing shape at each re-flash, with the thunder roll and the impact crack once per bolt within 16 chunks. While a bolt is lit the sky turns pale blue and the world is lit as by day. Friends get the host's bolts within 16 chunks (their view distance at most)."
- Under World simulation in `docs/MINECRAFT-SURVIVAL.md`, after the lightning line (lightning-turns): "Lightning turns a pig into a zombified piglin that never despawns, with a golden sword (one in twenty a golden spear; a baby pig gives a baby), and a villager into a witch that never despawns and can join raids, freeing its job site, bed and meeting point. Both keep their place, name, effects and lead, but not their saddle or held item. A struck creeper is charged (twice the blast, saved and seen by friends), and a mooshroom flips between red and brown once per bolt with its sound and takes no harm. On Peaceful pigs and villagers are only hurt."
- Under World simulation in `docs/MINECRAFT-SURVIVAL.md`, after the lightning-drawn line (lightning-glow 1): "A charged creeper wears vanilla's energy swirl: the blue `creeper_armor` texture scrolling diagonally over a shell two pixels outside its body, unlit and added onto what lies behind it, turning, walking and swelling with the creeper. Friends see it too."
- Under World simulation in `docs/MINECRAFT-SURVIVAL.md`, after the swirl line (lightning-glow 2): "A struck lightning rod sends 10 to 19 short white-violet sparks darting along its length, and every copper block a bolt cleans sparks on all six faces; friends within 64 blocks see them too."
- Under Mobs in `docs/MINECRAFT-SURVIVAL.md` (phantoms-live 1): "Phantoms (round 10, track M3) hatch from their egg and `/summon minecraft:phantom`, are saved with their size and anchor, and fly as vanilla's: they circle 5 to 15 blocks around a point five blocks above where they hatched, turning 4 degrees a tick and speeding up to 1.8 on the straight. Every 3 seconds (the first time after 1) they look for the highest player in sight within 16 blocks around and 64 up or down, climb 20 to 39 blocks above that player (never below sea level), and 10 ticks later dive with a screech and bite for 1 heart (a `size` set by command or save bites for 6 plus the size and grows 15 percent a size). A bite, a wall or any hit breaks the dive off: the phantom forgets the player and climbs 10 to 29 blocks above the ground before it looks again. Creative and spectator players and players under a roof are left alone. Friends see the size and the dive's pitch." (phantoms-drawn adds the model.)
- Under Mobs in `docs/MINECRAFT-SURVIVAL.md`, after the phantom line (phantoms-live 2): "Phantoms catch fire under the open day sky (not in rain or water), with or without AI. While a phantom dives it looks for cats every second: every cat within 16 blocks of it hisses, and the dive breaks off as if it had bitten. Ocelots do not count."
- Under Mobs in `docs/MINECRAFT-SURVIVAL.md`, after the burning line (phantoms-drawn 1): "Phantoms are drawn as vanilla draws them: the eight-part model with two glowing green eyes that stay lit in the dark, wings that beat 16 degrees up and down with the tail curling at each beat, a model 15 percent larger for each size, and the body pitched with its climbs and dives. They also show in monster spawner cages, wingtips poking out. Friends see the same."
- Under Mobs in `docs/MINECRAFT-SURVIVAL.md`, after the model line (phantoms-drawn 2): "Phantoms beat their wings with a soft flap sound every 2.4 seconds and trail small dark grey specks from both wingtips, for the host and every friend."
- Under Mobs in `docs/MINECRAFT-SURVIVAL.md`, after the flap line (phantoms-spawner 1): "Phantoms come for players who have not slept: in the Overworld, while the `spawn_phantoms` rule and monster spawning are on, the game looks once when the world loads and then every one to two minutes. At night (sky darkness 5 or more) each player standing in the open sky at or above sea level, not spectating, may draw them: a roll against the local difficulty (about half the looks on Normal, three in four on Hard, none on Peaceful), then a roll against the ticks since that player last slept or died (never within 72000 ticks, three days; half the time after six days, and more the longer they stay awake). Then 1 to difficulty + 1 phantoms appear together 20 to 34 blocks above, up to 10 blocks to each side, in air. Each player's clock is their own `minecraft:time_since_rest` stat, friends' too."

## Left for later

- Advancements (`lightning_strike`, `channeled_lightning`): the fork has none.
- Arrows, tridents, fireballs and other projectiles in the bolt's box get no fire ticks (vanilla `Entity.thunderHit` sets fire on any entity; arrows then burn). End crystals, the ender dragon, item frames and armour stands are not struck (the End has no weather; frames and stands ignore lightning in vanilla).
- An item destroyed by lightning does not spill its contents (`ItemStack.onDestroyed`: bundles and shulker boxes) and posts no `entity_damage` game event; the fork's fire and lava hurts on items do neither either.
- Worn armour on zombies and skeletons does not soften hazards (lava, fire, lightning): their hazard hurt uses the natural armour only, as it did before this track.
- Vanilla's exact `getEntitiesOfClass` order for the hits: the entity world's order is used.
- Lightning turns (lightning-turns): a player on a struck saddled pig gets off instead of riding the zombified piglin (vanilla moves the first passenger onto the new mob), and a struck mob's own vehicle is not handed on. A villager struck asleep in its bed: the witch does not take over its sleeping position, and the bed is left as a zombie infection leaves it. `hurtTime`, `yBodyRot`, the invulnerable time and `lastHurtByPlayerMemoryTime` are not carried to the new mob (vanilla's `ConversionType.convertCommon` copies them); the fork has no teams to copy.
- The villager's "Villager ... was struck by lightning ...." line goes to stderr (`eprintln!`), as the server's other messages do: `minecraft_terrain` has no `diag` dependency, and adding one would change `Cargo.lock` under every track.
- A second bolt in the same tick does not hit a mob a first bolt just made (the new tags join the level after `strike`); vanilla adds it at once. It is hit from the next lit tick on.
- Lightning drawn (lightning-drawn): vanilla's "Hide Lightning Flashes" accessibility option (`Options.hideLightningFlash`) and the entity distance option (`entityDistanceScaling`, which scales the 64-block draw range) have no setting in the fork yet; the flash always shows and the range is 64 blocks. A bolt never goes silent (`/summon` passes no tag to a bolt, so `Silent:1b` does nothing).
- Charged creeper swirl (lightning-glow 1): a creeper shown turning inside a monster spawner's cage (`spawner_display.rs`) never wears the swirl, even when the spawner's `SpawnData` holds `powered:1b`; the cage has no additive mesh.
- Lightning rod sparks (lightning-glow 2): the idle storm sparks of `LightningRodBlock.animateTick` (while thundering, when `level.random.nextInt(200) <= gameTime % 200` and the rod is the top block of its column, 1 or 2 sparks along its axis) are not shown. `display_ticks::animate_tick` gets only the scene, the player's block and the view's random; the rod arm beside `end_gateway` needs the client's thunder level, the game time and the level's own random passed in, which no caller has today. Natural thunderstorms come with `lightning-weather` (m3b), which is the place to add it.

- Phantoms (phantoms-live 1): a dying phantom sinks with gravity through its death tip (D0's `family_base_tick` moves every dead family mob with the ground travel; vanilla's phantom keeps `travelFlying`). M2's `OWN_DYING_TRAVEL` hook, once merged, lets the phantom keep its own travel. The player scan ignores `getVisibilityPercent` (sneaking, invisibility and a worn mob head shorten vanilla's 64-block range), as the fork's `NearestTargetGoal` does: `PlayerCandidate` carries none of them. `Silent:1b` does not silence the bite (`MobCore` has no silent flag). The server's `minecraft:flap` game event every 25 ticks waits for W's per-entity game events (M3-2 plan step 7). Scoreboard teams (`isAlliedTo`) do not exist in the fork.
- Phantom flaps (phantoms-drawn 2): the flap sound stays where the phantom flapped instead of following it (`EntityBoundSoundInstance`), and wing specks are not thinned by camera distance or a particle setting.

## Segment m3a

### Group lightning-strikes

#### 1. Minecraft lightning bolts set fires and hurt what they strike

After 7e860dfc.

- **javap (26.3 `client.jar`):**
  - `LightningBolt.<init>`: `life` 2, `seed = random.nextLong()`, `flashes = random.nextInt(3) + 1`; the `Entity` constructor draws the UUID first (`Mth.createInsecureUUID(random)`, two longs). `add_lightning` draws them in that order from `Level::entity_random` (the stand-in for the bolt's own unseeded random), the UUID through `SpawnLevel::next_uuid`.
  - `LightningBolt.tick`: at `life == 2` on the server, `spawnFire(4)` only on Normal and Hard, then `powerLightningRod`, `clearCopperOnLightningStrike` and `gameEvent(LIGHTNING_STRIKE)`; then `--life`; below 0 the bolt is discarded with no flashes left, else when `life < -nextInt(10)` it re-flashes (`--flashes`, `life = 1`, a new seed, `spawnFire(0)` with no difficulty check); while `life >= 0` and not visual-only, `getEntities(this, AABB(x-3, y-3, z-3, x+3, y+6+3, z+3), isAlive)` each get `thunderHit`.
  - `spawnFire(n)`: returns when visual-only or `!canSpreadFireAround(blockPosition())`; at the bolt's block, `BaseFireBlock.getState` is set with `setBlockAndUpdate` when the block `isAir` and the fire `canSurvive`; then `n` tries at `blockPosition().offset(nextInt(3)-1, nextInt(3)-1, nextInt(3)-1)` (each from the bolt's block, not chained).
  - `Entity.thunderHit`: `setRemainingFireTicks(r + 1)`, then `igniteForSeconds(8)` if that is 0, then `hurtServer(lightningBolt, 5)`. `Entity.igniteForTicks` sets the ticks only when fewer; `LivingEntity.igniteForTicks` scales them by `BURNING_TIME` first (the player's `Protection::burning_ticks`).
  - `ItemEntity.hurtServer`: `canBeHurtBy` (the `damage_resistant` types), `health -= amount`, discarded at 0 or less; `ExperienceOrb.hurtServer`: the same without the item check. Both start at 5 health.
  - `LightningRodBlock.onLightningStrike`: powered, neighbours, a tick in 8, then `levelEvent(3002, pos, facing.getAxis().ordinal())`. `randomStepCleaningCopper` posts `levelEvent(3002, pos, -1)` on each weathering copper block a walk step finds.
  - Damage type `lightning_bolt` (26.3 data): exhaustion 0.1, message `lightningBolt`, scaling `when_caused_by_living_non_player`; in `#panic_environmental_causes`, `#bypasses_shield` and `#no_knockback`, not in `#is_fire` or `#bypasses_armor`.
- **Changed:**
  - World crate: new `level/lightning.rs` with `Level::add_lightning(pos, visual_only)` (the visual-only bolt the skeleton trap uses), `lightning_fire(e, tries)` (`spawnFire`), `lightning_strike_entities(e)` (the level's items and orbs take `thunderHit`; every lit tick posts a `LightningHit { bolt, uuid, at }` to `Level::lightning_hits`), `take_lightning_hits`, `LightningHit::bounds`/`touches` and `ELECTRIC_SPARK` (3002). `LightningData` gains `seed` and `uuid`; `summon_lightning` calls `add_lightning(pos, false)`. The bolt tick calls `lightning_fire(e, 4)` on Normal and Hard before the rod, stores the re-flash seed and calls `lightning_fire(e, 0)`, and its old "lightning hitting entities" unsupported note became `lightning_strike_entities(e)`. The rod and the copper walk post 3002.
  - W's `lightning_strike` game event already sits in the bolt tick at `life == 2` (after the copper), so nothing moved.
  - Entities crate: `Hazard::LightningBolt` (damage type `minecraft:lightning_bolt`, panics, not fire, not bypassing armour); new `world/lightning.rs` with `Bolt { uuid, at }`, `ThunderOutcome` (its `Convert` arm is filled by `lightning-turns`) and `EntityWorld::thunder_hit(&Bolt, world)`: every living mob whose box meets the bolt's gets the fire bump, then `hurt_hazard(LightningBolt, 5)` through its `Exposed` (D0's generic family arm, every old family through `exposed_mut`, and bats and vexes, which `exposed_mut` leaves out). `world.rs` re-exports `Bolt` and `ThunderOutcome`.
  - Terrain: new `lightning.rs` with `BOLT`, `DAMAGE`, `PlayerStrike { player, at }`, `summon`, `summon_kinds` (the simulated kinds plus the bolt, sorted), `strike(level, mobs, states, tables, players, struck)` and `players_in(hit, players)`. `server.rs`: `crate::lightning::strike(..)` after `self.explode_tnt();` with `blast_players`; the `players_lightning` field after `players_struck`; `Output.players_lightning` after `players_struck`, filled in `run_tick`; the first lines of `ServerSim::summon` hand `minecraft:lightning_bolt` to `lightning::summon`.
  - Survival: new `lightning.rs` with `CAUSE`, `hit` (5 damage, from the bolt, `toward: None`, so no knockback and no shield block) and `burn` (the fire bump for players). `entities.rs`: a loop after the `players_struck` loop queues each `PlayerStrike`. `survival.rs`: the first line of `struck` calls `crate::lightning::burn`. `death.rs`: the `LightningBolt` arms after `Freeze` (`minecraft:lightning_bolt`, `lightningBolt`). `commands.rs`: `/summon` lists `lightning::summon_kinds()`.
- **Tests added:**
  - World crate `level::lightning::tests`: `a_bolt_on_normal_lights_its_block_and_up_to_four_more` (none on Easy at first, none for a visual bolt, none beyond the fire spread radius), `a_reflash_lights_only_its_own_block` (on Easy), `a_bolt_burns_items_and_orbs_inside_its_box_only` (netherite too), `a_bolt_reports_a_hit_each_lit_tick` (2 × (flashes + 1) hits, none for a visual bolt), `a_struck_rod_powers_and_sparks` (data 1 for an upright rod, -1 on cleaned copper), `a_bolt_in_an_empty_obsidian_frame_lights_the_portal`.
  - Entities crate `world::lightning::tests`: `a_struck_zombie_takes_five_and_one_fire_tick` (through its natural armour; 0 → 1, -1 → 160; 4 blocks off is outside), `armour_softens_lightning_and_blazes_are_hurt_too` (skeleton 15, zombie more, blaze 15, bat 1, vex 9), `a_struck_cow_panics`.
  - Terrain: `lightning::tests::summon_accepts_lightning_bolt`, `lightning::tests::a_bolt_strikes_the_living_players_in_its_box` (not spectators, the dead or those outside), `server::tests::a_summoned_bolt_hurts_mobs_and_players_in_its_box` (a full server tick: the cow under the bolt 10 → 5, the far one untouched, only the near player in `Output.players_lightning`, fire on the bolt's block).
  - Survival `lightning::tests`: `lightning_ignores_the_shield_and_knockback_and_lights_no_resting_player` (-20 → -19, -1 → 160, 100 → 101; other causes do nothing), `a_lightning_death_reads_struck_by_lightning`, `summon_lists_the_lightning_bolt`.
- **Decisions:**
  - Fire rule (M3-1 open decision 5): `thunderHit` ported literally. A mob at rest gets 1 fire tick, a player at rest (-20) none; the bolt's fire blocks do the burning.
  - The player hit keeps the bolt's position as `from` (the brief's choice: the MW2 damage direction points at the bolt). Vanilla's lightning damage source has no position; `toward: None` keeps the shield and the knockback out as `#bypasses_shield` and `#no_knockback` say.
  - Players are struck by the standing box (`thrown::PlayerBody::bounds`, 0.6 × 1.8), alive and not spectating, as the trident and wind charge hits measure them.
  - The hits run after the level tick (`strike` after `explode_tnt`), before the mobs tick; a mob the bolt kills drops its loot with the other deaths in `tick_mobs`.
  - `Level::add_lightning` returns the bolt's entity id; `ServerSim::summon` answers `Ok` on every difficulty (vanilla summons bolts on Peaceful).
- **Runs:** `test -p minecraftoss-world --locked` (141 passed), `test -p minecraftoss-entities -p minecraft_terrain -p minecraft_survival --locked --lib` (570, 1737 and 834 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** release build, survival, Normal, flat plains, by day, `/weather clear`, the kit below, the host is op. The bolt is not drawn yet and makes no sound (`lightning-drawn`); pigs, villagers, creepers and mooshrooms are not changed yet (`lightning-turns`). Stand still and type every `/summon` from the same place, so `~ ~ ~8` is always the same block ("the spot"). Name each screenshot by its label.
  1. `/summon minecraft:cow ~ ~ ~8`, then `/summon minecraft:lightning_bolt ~ ~ ~8`: the chat says "Summoned new Lightning Bolt", the grass at the spot and up to four blocks beside it catch fire, the cow flashes red, loses half its health and runs. **m3-strike-1**
  2. Walk to the spot (put the fire out with the water bucket), drop a stack of oak planks there and walk back. Throw 3 bottles o' enchanting at the spot, then the bolt: the planks and the orbs vanish. Drop planks 6 blocks further away first: they stay. **m3-strike-2**
  3. Put on the iron armour, raise the shield and `/summon minecraft:lightning_bolt ~ ~ ~`: a little over one heart is lost through the shield with no push, and the fire under you then lights you (step into water). **m3-strike-3**
  4. Take the armour off, eat down to 2 hearts (or take fall damage) and `/summon minecraft:lightning_bolt ~ ~ ~`: you die, "… was struck by lightning". **m3-strike-4**
  5. At the spot, build a pillar of 4 stone bricks, then a weathered copper block, then a lightning rod on top. Put a redstone lamp against one side of the copper block and weathered copper on the other three sides. From your place run `/summon minecraft:lightning_bolt ~ ~6 ~8`: the lamp blinks on for a moment and the copper turns orange (or less green) again. **m3-strike-5**
  6. Away from the pillar, stand a nether portal frame of 14 obsidian on the ground. At full health step inside it onto its bottom row, run `/summon minecraft:lightning_bolt ~ ~ ~`, then step out: the portal lights (you lose a heart or two). **m3-strike-6**
  7. `/difficulty easy`, then step 1 again: the cow is hurt, but only the block at the spot catches fire (a later flash of the bolt), never the blocks beside it. **m3-strike-7**
  8. Friend, two windows (`scripts\mp-local.ps1`): the friend stands at the spot and the host summons the bolt there: the friend loses two and a half hearts and burns from the fire under them. **m3-strike-8**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cow_spawn_egg*4,minecraft:zombie_spawn_egg*2,minecraft:blaze_spawn_egg*2,minecraft:lightning_rod*2,minecraft:weathered_copper*16,minecraft:redstone_lamp*2,minecraft:stone_bricks*64,minecraft:obsidian*14,minecraft:oak_planks*64,minecraft:experience_bottle*16,minecraft:water_bucket,minecraft:shield,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:cooked_beef*32,minecraft:iron_shovel
```

#### 2. Registered Minecraft mobs can answer lightning their own way

After cdd3cfed ("Minecraft lightning bolts set fires and hurt what they strike").

- **Checked first:** no `thunder_hit` hook was on `minecraft-survival` (M1's turtles wait for this one), so the commit was needed.
- **javap:** a jar scan for `thunderHit` overrides (M3-1) finds Pig, Villager, Creeper, MushroomCow, Turtle, CopperGolem, ArmorStand and `BlockAttachedEntity`; Turtle (M1) and CopperGolem (M3-4) are registered families, so they answer through this hook.
- **Changed (vendored entities crate, root `world/families.rs`):** `Family::thunder_hit(mob, bolt: &Bolt) -> bool` after `fire_immune`, default `false`; `true` means the family took the whole hit itself, so the base hit (fire bump and 5 damage) is skipped. `FamilyStore::thunder_hit(id, bolt)`, the `family_group!` dispatch `Group::thunder_hit(key, id, bolt)` and `Families::thunder_hit(key, id, bolt) -> Option<bool>`, each beside `deflect`. `EntityWorld::thunder_hit` (`world/lightning.rs`) asks the hook for a registered mob in the box before the base hit. D0's test probe (`families/pests/probe.rs`) records the bolts and answers with `thunder_answers`.
- **Tests added:** `world::lightning::tests::a_registered_mob_answers_lightning_through_its_hook` (a probe that answers false takes the base hit, one that answers true takes nothing; both saw the bolt's UUID).
- **Decisions:** the hook runs only for mobs inside the box and alive, as vanilla calls `thunderHit` on those only. A family that wants the base hit and more (the copper golem un-weathering) returns `false` after its own part.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (571 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** nothing new to see; registered mobs (silverfish, endermites, rabbits, polar bears, breezes) take the bolt like any mob. Release build, survival, Normal, by day: `/summon minecraft:polar_bear ~ ~ ~8`, then `/summon minecraft:lightning_bolt ~ ~ ~8`: the polar bear flashes red and is hurt, but lives (5 a hit, 30 health), and the grass under it catches fire. **m3-strike-hook**
- **Kit:** none new.

### Group lightning-turns

#### 1. Minecraft lightning turns pigs, villagers, creepers and mooshrooms like vanilla

After 1e178888 ("Registered Minecraft mobs can answer lightning their own way").

- **javap (26.3 `client.jar`):**
  - `Pig.thunderHit`: unless Peaceful, `convertTo(ZOMBIFIED_PIGLIN, ConversionParams.single(this, false, true), ..)`; its lambda calls `ZombifiedPiglin.populateDefaultEquipmentSlots(this.getRandom(), ..)` and `setPersistenceRequired`. A null result, or Peaceful, falls back to `Animal.thunderHit`.
  - `ZombifiedPiglin.populateDefaultEquipmentSlots`: main hand `random.nextInt(20) == 0 ? GOLDEN_SPEAR : GOLDEN_SWORD`, so the pig's own random gives a golden spear one time in twenty.
  - `Villager.thunderHit`: unless Peaceful, `LOGGER.info("Villager {} was struck by lightning {}.")`, then `convertTo(WITCH, single(this, false, false), ..)`; its lambda calls `Witch.finalizeSpawn(.., CONVERSION, null)`, `setPersistenceRequired` and the villager's `releaseAllPois`.
  - `Raider.finalizeSpawn`: `setCanJoinRaid(!is(WITCH) || reason != NATURAL)`, so a converted witch can join raids; then `PatrollingMonster`'s leader roll (a `nextFloat` from the level random; witches never lead) and `Mob`'s part.
  - `Creeper.thunderHit`: `Monster.thunderHit` (the base hit), then `DATA_IS_POWERED` true, on any difficulty.
  - `MushroomCow.thunderHit`: when the bolt's UUID is not `lastLightningBoltUUID`, it flips RED/BROWN, stores the UUID and plays `MOOSHROOM_CONVERT` at 2.0 and 1.0. It never calls the base hit (no damage, no fire, no panic), on any difficulty.
  - `ConversionType$1.convert` (SINGLE) and `convertCommon`: position and rotation, motion, the first passenger and the vehicle; equipment and drop chances only with `keepEquipment`; absorption, effects (copies), `setBaby` when the old one is a baby, age only between two `AgeableMob`s, `canPickUpLoot` only when preserved, left-handedness, NoAI, persistence, the custom name and its visibility, the on-fire flag, invulnerability, NoGravity, portal cooldown, Silent, tags, components and team; then fall distance, hurt time, body yaw, on-ground and the leash. `Mob.convertTo` adds the new mob and discards the old one, so a pig's saddle is lost.
  - `Entity.toString`: `%s['%s'/%d, l='%s', x=%.2f, y=%.2f, z=%.2f, ..]`, mirrored (without the level) in the villager's log line.
- **Changed:**
  - Entities crate (`world/lightning.rs`): `ThunderOutcome::Convert { id, into, held }`. `EntityWorld::thunder_hit` asks `thunder_turn` before the base hit and returns the outcomes.
    - Pig, unless Peaceful: it draws `nextInt(20)` from its own random and asks to become a zombified piglin holding a golden sword or spear, with no base hit.
    - Villager, unless Peaceful: it gives back its points of interest (`release_villager_pois`, as `zombie_killed_villager` does before an infection) and asks to become a witch, with no base hit.
    - Mooshroom, on any difficulty: it flips through `MushroomCowState::lightning_hit` (keyed by the bolt's UUID in hex, `Bolt::name`) and voices `entity.mooshroom.convert` at 2.0 and 1.0, with no base hit.
    - Creeper: the base hit, then `powered`.
  - World crate (`natural_spawner/spawning.rs`): `finalize_conversion` gains a witch arm after the villager's: `CanJoinRaid` true, the leader roll's `next_f32` from the level random, then `finalize_mob`.
  - Terrain (`lightning.rs`):
    - `strike` takes `&mut` `mob_tags`, sets the mob world's difficulty from the level's before each bolt's hits (the tick sets it only in `tick_mobs`, after the level tick) and returns the new tags.
    - For each `Convert` it builds the old mob's saved tag (`mob_tags_where`) and turns it with `turned_tag`. It removes the old mob and its tag at once, so a second bolt in the same tick cannot turn it again. It finalizes the witch (`Level::finalize_conversion`) after printing the villager line, and gives the zombified piglin a UUID from `SpawnLevel::next_uuid`.
    - `turned_tag` is `converted_zombie_tag` without `equipment` and `drop_chances`, plus `Pos`, `Motion`, `Rotation`, `OnGround`, `fall_distance`, `active_effects` and `AbsorptionAmount`, with the new `id` and `PersistenceRequired:1b`. The witch drops `CanPickUpLoot`. The piglin keeps the pig's (0 when unsaved), gets `IsBaby` from the pig's `Age` and its weapon in `equipment.mainhand`.
  - Terrain `server.rs`: the `strike` line in `ServerSim::tick` passes `&mut self.mob_tags` and hands each returned tag to `add_saved_entity`.
- **Tests added:**
  - Entities `world::lightning::tests`:
    - `a_struck_pig_or_villager_turns_unhurt_unless_peaceful`: a sword seed and a spear seed; both pigs and the villager unhurt; on Peaceful no outcome and 5 damage each.
    - `a_struck_villager_frees_its_job_site`: a farmer's composter ticket comes back.
    - `a_struck_creeper_is_charged_and_hurt`: on Peaceful too, 15 health, 1 fire tick, powered.
    - `a_mooshroom_flips_once_per_bolt_unharmed`: four hits of one bolt flip it once with one sound and no damage, fire or panic; a new bolt flips it back.
  - Terrain `lightning::tests`: `a_turned_pig_keeps_its_place_name_and_effects_but_not_its_saddle`, `a_turned_villager_drops_its_trades_hand_and_loot_pickup`, and `a_friend_sees_the_brown_mooshroom_and_the_charged_creeper` (two look updates; the friend's mooshroom is brown and its creeper charged).
  - Terrain `server::tests`, a full server tick each:
    - `a_struck_pig_becomes_a_persistent_zombified_piglin_with_a_golden_sword`: two pigs give two persistent piglins with a golden sword or spear at full health, and the baby gives a baby that keeps its name. On Peaceful the pig is hurt to 5.
    - `a_struck_villager_becomes_a_persistent_witch`: on Easy (no fire burst to burn the witch), persistent, can join raids, 26 health, a UUID and no villager data. On Peaceful the villager is hurt to 15.
    - `a_struck_creeper_is_charged_hurt_and_saved_charged`: 15 health, saved with `powered:1b`, loads charged.
- **Decisions:**
  - The golden spear one time in twenty (javap), where the brief says only "a golden sword": vanilla's `populateDefaultEquipmentSlots` for zombified piglins rolls it, from the pig's own random.
  - The witch's finalize lives in `finalize_conversion` (its own arm) rather than in terrain, so the level random's draws stay in vanilla's order (the UUID from the entity random, then the leader roll and `Mob`'s part).
  - Age: neither a zombified piglin nor a witch is an `AgeableMob`, so only `setBaby` carries over: `IsBaby` for the piglin. A baby villager gives an adult witch, as in vanilla.
  - The brief's `a_struck_villager_becomes_a_persistent_witch_and_frees_its_job_site` is split: the server test checks the witch, and the entity-crate test the freed job site (a server test would need a villager that has already claimed one).
  - `docs/round10/mobs.md` is not changed: lightning has no row, and pigs, villagers, creepers and mooshrooms are not M3 rows.
- **Runs:** `test -p minecraftoss-entities -p minecraftoss-world -p minecraft_terrain --locked --lib` (575, 141 and 1743 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** release build, survival, Normal, flat plains, by day, `/weather clear`, the kit below, the host is op. The bolt is not drawn yet and makes no thunder (`lightning-drawn`), and a charged creeper does not glow yet (`lightning-glow`). Stand still and type every `/summon` from the same place, so `~ ~ ~8` is always the same block ("the spot"). Put out fires with the water bucket between steps. Name each screenshot by its label.
  1. `/summon minecraft:pig ~ ~ ~8`, then `/summon minecraft:lightning_bolt ~ ~ ~8`: the pig becomes a zombified piglin holding a golden sword (one time in twenty a golden spear). It is not angry at you. **m3-turn-1**
  2. `/summon minecraft:pig ~ ~ ~8 {Age:-24000}`, then the bolt: a baby zombified piglin. **m3-turn-2**
  3. Walk 140 blocks away in a straight line and come back: both zombified piglins are still there. **m3-turn-3**
  4. Place a composter 2 blocks beside the spot and use a villager egg next to it. Wait until the villager wears the farmer's straw hat (up to a minute in the day). Then summon the bolt on the villager's block: it becomes a witch. **m3-turn-4** Put out the fire, step back and use a second villager egg 4 blocks from the composter: it becomes a farmer at the same composter, because the first villager freed it. **m3-turn-5**
  5. `/summon minecraft:mooshroom ~ ~ ~8`, then the bolt: it turns brown with a short sound and does not flash red from the bolt (the fire under it may still singe it). A second bolt turns it red again. **m3-turn-6**
  6. Dig a pit 1 wide and 3 deep at the spot and lay a stone slab on its floor. Run `/summon minecraft:creeper ~ ~-2.5 ~8` and `/summon minecraft:lightning_bolt ~ ~-2.5 ~8`. Dig a second pit the same way 5 blocks to the side and use a creeper egg in it. Save and quit, then load again. Walk up to each pit until its creeper hisses, then back off at once to 6 blocks: the struck creeper's crater is about twice as wide. **m3-turn-7**
  7. `/difficulty peaceful`, then steps 1 and 4 again: the pig and the villager flash red and lose health, but stay a pig and a villager. **m3-turn-8**
  8. Friend, two windows (`scripts\mp-local.ps1`): the host repeats steps 1 and 5; the friend sees the zombified piglin and the brown mooshroom. **m3-turn-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:pig_spawn_egg*4,minecraft:villager_spawn_egg*4,minecraft:creeper_spawn_egg*4,minecraft:mooshroom_spawn_egg*2,minecraft:composter*2,minecraft:stone_slab*8,minecraft:iron_shovel,minecraft:water_bucket,minecraft:cooked_beef*32
```

### Group lightning-drawn

#### 1. Minecraft lightning bolts flash and thunder for every player

After 82a5cd0b ("Minecraft lightning turns pigs, villagers, creepers and mooshrooms like vanilla").

- **javap (26.3 `client.jar`):**
  - `LightningBoltRenderer.submit`: 8 trunk points from `RandomSource.createThreadLocalInstance(seed)` (`SingleThreadedRandomSource`, the 48-bit LCG of our `LegacyRandom`), each step `nextInt(11) - 5` on x then z, walked from index 7 down; the tip is the sum. `lambda$submit$0`: 4 layers, each with a fresh random from the same seed; branch 0 is the trunk (segments 7 to 0, steps `nextInt(11) - 5`, starting at `point[7] - tip` so it lands on the bolt), branches 1 and 2 run segments 6 to 4 and 5 to 3 (steps `nextInt(31) - 15`) from the trunk point at their top; width `0.1 + layer * 0.2`, on the trunk times `segment * 0.1 + 1` at the top and `(segment - 1) * 0.1 + 1` at the foot. `quad`: 4 sides per segment with the flags (F,F,T,F), (T,F,T,T), (T,T,F,T), (F,T,F,F), the foot at `segment * 16` and the top at `(segment + 1) * 16`, colour (0.45, 0.45, 0.5, 0.3). 4 layers of 14 segments with 4 sides each: 224 quads, 128 blocks tall. `affectedByCulling` is false.
  - `LightningBolt.shouldRenderAtSqrDistance`: within `64 * Entity.viewScale` (1.0 by default).
  - `BlendFunction.LIGHTNING` is (SRC_ALPHA, ONE) and `ADDITIVE` (ONE, ONE). `rendertype_lightning.fsh` (26.3 jar): `vertexColor * ColorModulator * (1 - total_fog_value)`, no texture and no lightmap.
  - `LightningBolt.tick` on the client: at `life == 2`, unless silent, `entity.lightning_bolt.thunder` (WEATHER, 10000, `0.8 + nextFloat * 0.2`) and then `entity.lightning_bolt.impact` (WEATHER, 2.0, `0.5 + nextFloat * 0.2`); while `life >= 0`, `setSkyFlashTime(2)`. The renderer itself has no life check: the bolt is drawn for as long as it exists.
  - `ClientLevel.addEnvironmentAttributeLayers`: while `getSkyFlashTime() > 0`, `SKY_COLOR` is `ARGB.srgbLerp(0.22, value, (0.8, 0.8, 1.0))` (`ARGB.srgbLerp` on vectors is `Vector3fc.lerp`) and `SKY_LIGHT_FACTOR` is 1. `ClientLevel.tick` counts the flash time down by one. `Minecraft.tick` runs `GameRenderer.tick` (the camera's attribute probe samples) before `ClientLevel.tickEntities` and `ClientLevel.tick`.
- **Changed:**
  - Terrain: `lightning.rs` gains `BoltView { id, position, seed, life }` and `views(level)` (every live bolt of the level). New `lightning_render.rs`: `bolt_quads(seed)` (the renderer's port), `drawn(bolt, camera)` (64 blocks) and `append_bolts(mesh, bolts, atlas, camera)` (the white texel, colour (0.45, 0.45, 0.5, 0.3), unlit). `server.rs`: `EntitySnapshot.lightning` after `crystals`, filled from `lightning::views` in the snapshot line beside `crystals`. `mob_tracking::tracking_range`: `"lightning_bolt"` beside `"end_crystal"` (16 chunks). `environment.rs`: `DimensionEnvironment::flash_sky()` sets the flash to 2 ticks; `tick` counts it down, then, while it lasts, lerps the sampled sky colour 0.22 toward (0.8, 0.8, 1.0) and sets the sampled sky light factor to 1, so frames blend in and out between ticks as vanilla's probe does. `lib.rs`: `pub mod lightning_render;`.
  - Survival: `lightning.rs` gains `Bolts` (the view's bolts): `receive`, `tick(listener, random, sounds)` (thunder and impact once per bolt id within 16 chunks of the player, forgetting ids that left), `flashing()` (a tracked bolt with `life >= 0`) and `append` (the draw). `entity_view.rs`: the `bolts` field after `crystals`, `receive` after the crystals line, `tick` after `self.eyes.tick`, the draw after the crystal draw (into `out.additive`) and `pub fn sky_flash()`. `entities.rs`: `MobMeshes.additive` after `translucent`. `mob_sender.rs`: `LIGHTNING` after `CRYSTALS` and the bolts' row after `crystals:` in `near`, kept at the bolt's tracking range.
  - Render: `render_anim` (`minecraft_world.rs`, `minecraft_remote.rs`) fills a seventh entity mesh slot from `meshes.additive`; `SkyInputs.sky_flash` (host: the host's entity view, friend: `level.view.sky_flash()`) calls `flash_sky()` in `present_sky` before the environment ticks. `render_gpu` (`drawsurf/minecraft_world.rs`): `entity_meshes` and the GPU entity slots go from 6 to 7 (`ADDITIVE_SLOT` 6), and a seventeenth pipeline `additive_vertex`/`additive_fragment` with `additive_blend()` (ONE, ONE), depth test without depth write and no culling, drawn after the translucent terrain and the block cracks and before the weather. In the WGSL, `additive_vertex` passes the colour unlit and `additive_fragment` is texel times colour, times its alpha and `1 - fog_value`.
- **Tests added:**
  - Terrain `lightning_render::tests`: `a_bolt_is_224_quads_128_blocks_tall_from_its_seed` (0 to 128, the trunk lands on the bolt within 0.63, the top trunk segment tapers from 0.17 to 0.16 a side, every layer follows one path), `the_same_seed_draws_the_same_bolt`, `bolts_are_drawn_white_and_unlit_within_sixty_four_blocks` (a bolt 70 blocks off is not drawn; nothing without the white texel).
  - Terrain `environment::tests::a_sky_flash_brightens_the_sky_and_lights_the_world_for_two_ticks` (midnight in the plains: two renewed ticks pale blue and lit, dark again after, half lit halfway through the next tick, one tick when not renewed).
  - Terrain `server::tests::a_summoned_bolt_reaches_the_snapshot_with_its_seed_until_it_is_gone` (lives 1, 0, then one to three re-flashes each with a new seed, gone the tick after its last 0).
  - Survival `lightning::tests::a_new_bolt_thunders_once_and_flashes_while_lit` (thunder 10000 and impact 2.0 once per bolt, none for a bolt 300 blocks away, flashing at life 1 and 0, dark at -1, a new bolt thunders) and `mob_sender::tests::a_friend_gets_bolts_within_sixteen_chunks` (16 chunks, never past the friend's view distance, only in chunks the friend has; seed and life cross the state lane).
  - `render_gpu` `drawsurf::minecraft_world::tests::lightning_draws_on_its_own_additive_slot_unlit_and_faded_by_fog` (the entry points, slot 6 of 7, ONE/ONE for colour and alpha, the fragment premultiplies by alpha and fades by fog, the vertex is unlit); `glowing_mobs_draw_an_outline_through_walls_masked_by_their_own_shape` now checks 7 slots.
- **Decisions:**
  - The draw pass (M3-1 open decision 3): an additive pass, (ONE, ONE) with the colour premultiplied by its alpha in the shader, which adds exactly what vanilla's (SRC_ALPHA, ONE) adds; the charged creeper's energy swirl (`lightning-glow`, vanilla ADDITIVE) uses the same pass with alpha 1. It is unlit and fades by fog like `rendertype_lightning`.
  - Draw order: after the translucent terrain and the cracks, before the rain. The bolt does not write depth, so drawing it before water would let the water cover a bolt standing in front of it; drawn after, a bolt behind water shows a little too bright, the smaller error.
  - The bolt is drawn whenever the view holds it (vanilla's renderer has no life check) and changes shape with each re-flash's new seed; only the sky flash follows `life >= 0`. The brief said "draws and flashes while `life >= 0`"; the draw follows vanilla.
  - Bolts are drawn within 64 blocks of the camera (vanilla's `shouldRenderAtSqrDistance` at the default entity distance), and heard and flashed within 16 chunks of the player (the bolt's tracking range). The host's snapshot holds the whole level, so the host applies the 16 chunks itself; friends get only their 16 chunks (`near`), capped by their view distance as for every entity.
  - The flash enters the sampled attribute values in `DimensionEnvironment::tick` (the brief placed it in `sky_state`), as vanilla's time-based layer is sampled by the probe each tick; frames then blend from the last tick to this one. The countdown runs before the sample, so the sky flashes exactly the ticks a bolt is lit in the view, two per flash; vanilla's order (probe, bolt sets 2, level counts down) gives the same two samples one tick later, a tick the snapshot's own delay already covers.
  - Thunder and impact pitches come from the entity view's random (vanilla draws them from the client bolt's own unseeded random), thunder first. The audio engine's reach of 16 times the volume keeps the thunder at full volume across the tracking range.
  - `docs/round10/mobs.md` is not changed: lightning has no row.
- **Runs:** `test -p minecraft_terrain -p minecraft_survival -p render_gpu --locked --lib --no-fail-fast` (terrain 1748 of 1751 with 3 ignored, after two of the new tests were fixed and rerun with `-- bolt`; survival 836; render_gpu 42), `test -p render_anim --locked --lib` (224 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** release build, survival, Normal, flat plains, `/weather clear`, the kit below, the host is op. Stand still and type every `/summon` from the same place, so `~ ~ ~8` is always the same block ("the spot"). Put out fires with the water bucket between steps. Name each screenshot by its label.
  1. By day, `/summon minecraft:lightning_bolt ~ ~ ~8`: a jagged blue-white bolt reaches up out of sight from the spot, thunder rolls and a short crack sounds, and the sky flashes pale blue. The bolt stays about a second and jumps to a new shape once to three times. **m3-drawn-1**
  2. `/time set midnight`, then the bolt again: the whole landscape lights up as by day and the sky turns pale blue for a moment at each flash. **m3-drawn-2**
  3. `/summon minecraft:lightning_bolt ~ ~ ~70`: no bolt is drawn (vanilla draws them within 64 blocks), but the thunder rolls and the night lights up. Then `/summon minecraft:lightning_bolt ~ ~ ~40`: the bolt is drawn. **m3-drawn-3**
  4. Place a row of 5 light blue stained glass, 2 high, 4 blocks in front of you between you and the spot, and summon the bolt at the spot: the bolt shows through the glass, brighter than the glass around it. Pour the water bucket at the spot and summon the bolt on the water: it stays bright over the water. **m3-drawn-4**
  5. `/weather rain`, then the bolt: the rain falls in front of the bolt. **m3-drawn-5**
  6. Friend, two windows (`scripts\mp-local.ps1`), by night: the friend stands next to the host and the host summons the bolt at the spot. The friend sees the same bolt and the flash and hears the thunder. **m3-drawn-friend-1** The friend walks 300 blocks away and the host summons another bolt at the spot: the friend hears and sees nothing. **m3-drawn-friend-2**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:light_blue_stained_glass*16,minecraft:water_bucket,minecraft:iron_shovel,minecraft:cooked_beef*32
```

### Group lightning-glow

#### 1. Minecraft charged creepers glow with their energy swirl

After b4b3e8db ("Minecraft lightning bolts flash and thunder for every player").

- **javap (26.3 `client.jar`):**
  - `CreeperPowerLayer`: a `CreeperModel` baked from `ModelLayers.CREEPER_ARMOR`, texture `textures/entity/creeper/creeper_armor.png`, `isPowered` from the render state, `xOffset(t) = t * 0.01`.
  - `LayerDefinitions`: `CREEPER_ARMOR` is `CreeperModel.createBodyLayer(new CubeDeformation(2.0))`, the creeper's own boxes (head, body, four legs, texture 64 × 32) grown by 2 pixels on every side, their UVs unchanged.
  - `EnergySwirlLayer.submit`: only when powered; `RenderTypes.energySwirl(texture, xOffset(ageInTicks) % 1, ageInTicks * 0.01 % 1)`, colour `-8355712` (`0xFF808080`), `NO_OVERLAY`, submitted after the body (`order(1)`) under the body's pose, so it turns, walks, swells and tips over with it.
  - `RenderTypes.energySwirl`: `RenderPipelines.ENERGY_SWIRL` with `Sampler0` from the texture and `TextureTransform.OffsetTextureTransform(u, v)` (`Matrix4f.translation(u, v, 0)`: the UVs move by `+u, +v`). The texture's sampler is `AbstractTexture`'s default, REPEAT and NEAREST, so the scrolled texture wraps.
  - `RenderPipelines.ENERGY_SWIRL`: `core/entity` with `ALPHA_CUTOUT 0.1`, `EMISSIVE` (no lightmap), `NO_OVERLAY`, `NO_CARDINAL_LIGHTING`, `APPLY_TEXTURE_MATRIX`; no culling, blend `ADDITIVE`, `DepthStencilState.DEFAULT`. The 26.3 `entity.fsh` then runs `apply_fog`.
  - `creeper_armor.png` (26.3 jar): 64 × 32 with a palette whose index 0 is fully transparent (1404 of 2048 texels); every other texel is opaque.
- **Changed (terrain):**
  - `creeper_render.rs`: `POWER_TEXTURE` and `append_creeper_power(mesh, creepers, poses, atlas, partial)`: for each live powered creeper, its six boxes grown by 2 pixels with the body's UV sizes, under the body's rotation, swell scale and part poses, coloured 128/255 grey with alpha 1 and unshaded. The offset is `age_in_ticks * 0.01 % 1` on both axes. Each face is cut where its scrolled UVs pass 1 (at most four pieces), and each piece reads `creeper_armor` from the atlas at its wrapped coordinates, so the atlas draws what vanilla's repeating sampler draws. `swelling_of` and `posed_parts` are now shared by the body and the swirl. The module note "the charged creeper's energy layer is not drawn yet" is gone.
  - `mesh.rs`: `creeper_armor` joins the atlas after the creeper skin.
- **Changed (survival):** `entity_view.rs`: one line after `append_creepers` draws the swirl into `MobMeshes.additive`, the pass `lightning-drawn` added. Host and friends both draw it; friends know `powered` from the look tag (lightning-turns).
- **Tests added:** terrain `creeper_render::tests::a_charged_creeper_wears_the_scrolling_energy_layer` (a plain creeper has no swirl and the charge leaves the body unchanged; at age 0 the swirl is 36 faces in `creeper_armor`, grey 128/255 with alpha 1, 2 pixels below the feet and above the head; at age 25 the faces are cut where the texture repeats, cover the same area and stay in the region) and `creeper_render::tests::a_swirl_face_wraps_where_its_scrolled_texture_repeats` (a face at offset 0 is unchanged; at 0.25 it splits where the shifted u passes 1, the far piece reading the texture's left edge; UVs land in the atlas region; a face crossing both wraps becomes four pieces of the same area).
- **Decisions:**
  - The scroll (brief: "a repeating scroll along the armour glint's path"): the wrap is cut into the mesh on the CPU and the swirl reads the atlas on the additive pass, instead of `creeper_armor` as a GPU texture of its own with the repeat sampler and a vertex flag, as the armour glint does. The picture is the same (vanilla samples it REPEAT and NEAREST; the additive pass samples the atlas NEAREST at level 0). This needs no new binding, frame field, image conversion or texture load in `render_gpu`'s `drawsurf/minecraft_world.rs`, `render`'s `extract.rs` and the host and friend views in `render_anim`. The 44-byte entity vertex has no spare field for a flag either. A charged creeper is at most 144 quads.
  - Fog: vanilla's `apply_fog` mixes the swirl toward the fog colour before the additive blend, so a swirl deep in fog adds the fog colour. The shared additive pass fades it to nothing, as it does the bolt. Only a charged creeper at the far edge of the fog shows the difference.
  - Depth: vanilla's swirl writes depth; the additive pass only tests it. The shell sits 2 pixels outside the body, which writes depth, so only rain falling right behind the shell is drawn differently.
  - `docs/round10/mobs.md` is not changed: the creeper is not an M3 row and lightning has no row.
- **Runs:** `test -p minecraft_terrain --locked --lib` (1750 passed, 3 ignored), `test -p minecraft_survival --locked --lib` (836 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** release build, survival, Normal, flat plains, by day, `/weather clear`, the kit below, the host is op. Stand still and type every `/summon` from the same place, so `~ ~ ~8` is always the same block ("the spot"). Name each screenshot by its label.
  1. Dig a pit 1 wide and 3 deep at the spot and lay a stone slab on its floor (fire cannot sit on a bottom slab). Run `/summon minecraft:creeper ~ ~-2.5 ~8` and `/summon minecraft:lightning_bolt ~ ~-2.5 ~8`. Walk to the pit's edge (not closer) and look down: a blue swirl a little larger than the creeper wraps it and slides slowly and steadily on a diagonal. The creeper's green skin shows through the swirl's gaps. **m3-glow-1**
  2. `/time set midnight`: the creeper darkens with the night, the swirl stays as bright as by day. **m3-glow-2**
  3. Save and quit, then load again: the creeper still wears the swirl. **m3-glow-3**
  4. Dig a second pit the same way 5 blocks to the side and use a creeper egg in it: no swirl. **m3-glow-4**
  5. Stand at the edge of the first pit until the creeper hisses: the swirl swells and wobbles with it. Back off at once to 6 blocks before it blows. **m3-glow-5**
  6. Friend, two windows (`scripts\mp-local.ps1`): the host repeats step 1; the friend sees the same swirl on the creeper. **m3-glow-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:creeper_spawn_egg*2,minecraft:stone_slab*4,minecraft:iron_shovel,minecraft:water_bucket,minecraft:cooked_beef*16
```

#### 2. Minecraft lightning rods spark when struck

After 49deedb2 ("Minecraft charged creepers glow with their energy swirl").

- **javap (26.3 `client.jar`):**
  - `LevelEventHandler`, case 3002: when `0 <= data < Direction.Axis.VALUES.length`, `ParticleUtils.spawnParticlesAlongAxis(Axis.VALUES[data], level, pos, 0.125, ELECTRIC_SPARK, UniformInt.of(10, 19))`; otherwise `ParticleUtils.spawnParticlesOnBlockFaces(level, pos, ELECTRIC_SPARK, UniformInt.of(3, 5))`. No sound.
  - `ParticleUtils.spawnParticlesAlongAxis`: the count first (`UniformInt.sample`: `Mth.randomBetweenInclusive`, `nextInt(max - min + 1) + min`); each spark at the block's centre plus `Mth.nextDouble(-1, 1)` times 0.5 along the axis and times 0.125 across it (x, y, z in turn), then a speed `Mth.nextDouble(-1, 1)` along the axis only.
  - `ParticleUtils.spawnParticlesOnBlockFaces`: for each `Direction` (down, up, north, south, west, east) a count of 3 to 5, and for each spark first a speed of three `Mth.nextDouble(-0.5, 0.5)`, then the position: 0.55 out along the face's normal and `Mth.nextDouble(-0.5, 0.5)` along the other two axes, moving only along its face.
  - `GlowParticle$ElectricSparkProvider`: a `GlowParticle` made with zero speed, colour (1, 0.9, 1), the speed set to a quarter of the asked one, a lifetime of `nextInt(2) + 2`. `GlowParticle`: friction 0.96, `speedUpWhenYMotionIsBlocked`, `quadSize * 0.75`, no physics, layer OPAQUE, `getLightCoords` = `LightCoordsUtil.addSmoothBlockEmission(light, (age + partial) / lifetime)`. `particles/electric_spark.json`: the one sprite `minecraft:glow`.
- **Changed (terrain):**
  - `level_events.rs`: `ELECTRIC_SPARK` (3002) after `LECTERN_PAGE_TURN`, its arm after the lectern's, and `electric_sparks` (with the helper `between`, `Mth.nextDouble`): the axis sparks for data 0 to 2, the face sparks for anything else (the copper walk's -1), drawn in vanilla's order from the view's random.
  - `ambient_particles.rs`: `Kind::ElectricSpark` after `Kind::Crit`; `Particle::electric_spark` (the provider and `GlowParticle`); `glide` (`Particle.tick` without physics: through blocks, faster on x and z while its height holds); the `glow` sprite (added to `textures()`); its constant size; the flames' light rule (`glow`).
  - The world crate already posts 3002 (lightning-strikes 1), and level events reach friends within 64 blocks (`heard::Heard::Event`), so nothing else changed.
- **Tests added:** terrain `level_events::tests::a_struck_rod_sparks_along_its_axis_and_cleaned_copper_sparks_on_its_faces` (for each axis: 10 to 19 sparks, the first in vanilla's draw order, 0.5 along the axis and 0.125 across it, moving only along it, no sound; data -1 and 3: 3 to 5 on each of the six faces, 0.55 out, never moving off its face) and `ambient_particles::tests::an_electric_spark_fades_quickly` (2 or 3 ticks, colour (1, 0.9, 1), a quarter of the asked speed, three quarters of the base size, the `glow` sprite, opaque; it passes through stone and speeds up on x while its height holds; light from the block level at birth to 15 at its end).
- **Decisions:**
  - Kept in the commit: the struck rod and cleaned copper (event 3002). Left for later: the rod's idle storm sparks (`animateTick`), which need the client's thunder level, the game time and the level random in `display_ticks` (see "Left for later"). This keeps `display_ticks::animate_tick`'s signature for `lightning-weather` to extend with the storm.
  - `glide` is a new tick beside `coast` rather than a change to `coast`: `coast` drives the trial omen, whose `speed_up` is set, so a shared speed-up would change it.
  - `docs/round10/mobs.md` is not changed: lightning rods are not mobs.
- **Runs:** `test -p minecraft_terrain --locked --lib` (1752 passed, 3 ignored), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** release build, survival, Normal, flat plains, by day, `/weather clear`, the kit below, the host is op. Stand still and type every `/summon` from the same place, so `~ ~ ~8` is always the same block ("the spot"). Put out fires with the water bucket between steps. Name each screenshot by its label.
  1. At the spot, build a pillar of 4 stone bricks, then a weathered copper block, then a lightning rod on top, with weathered copper on all four sides of that copper block. From your place run `/summon minecraft:lightning_bolt ~ ~6 ~8`: small white-violet sparks dart up and down along the rod for a moment, and sparks flash on the faces of each copper block that turns orange again. **m3-spark-1**
  2. Place a stone brick on the ground at the spot and a lightning rod on its east face, so the rod lies flat, pointing east. Run `/summon minecraft:lightning_bolt ~1 ~1 ~8`: the sparks dart east and west along the lying rod. **m3-spark-2**
  3. Friend, two windows (`scripts\mp-local.ps1`): the friend stands next to the host and the host repeats step 1 on a new pillar; the friend sees the same sparks. **m3-spark-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:lightning_rod*4,minecraft:weathered_copper*16,minecraft:stone_bricks*16,minecraft:water_bucket,minecraft:cooked_beef*16
```

### Group phantoms-live

#### 1. Minecraft phantoms circle, swoop and bite like vanilla

After 32b231ee ("Minecraft lightning rods spark when struck").

- **javap (26.3 `client.jar`):**
  - `Phantom`: `extends Mob implements Enemy`; `xpReward` 5; `PhantomMoveControl` and an empty `PhantomLookControl`; `PhantomBodyRotationControl.clientTick` (`yHeadRot = yBodyRot; yBodyRot = yRot`). Goals: 1 `PhantomAttackStrategyGoal`, 2 `PhantomSweepAttackGoal`, 3 `PhantomCircleAroundAnchorGoal`; targets: 1 `PhantomAttackPlayerTargetGoal`. `travel` is `travelFlying(input, 0.2)`, `checkFallDamage` does nothing, the sound source is HOSTILE. `getDefaultDimensions` scales by `1 + 0.15 * size`; `updatePhantomSizeInfo` (attack damage `6 + size`) runs only from `onSyncedDataUpdated`, so only a changed size changes the damage. `setPhantomSize` clamps to 0..64. `finalizeSpawn`: `anchorPoint = blockPosition().above(5)`, `setPhantomSize(0)`, then `Mob.finalizeSpawn`. Saved: `anchor_pos` (`BlockPos.CODEC`, an int array, left out when null) and `size`.
  - `PhantomMoveControl.tick` (speed starts at 0.1): a horizontal collision turns `yRot` by 180 and drops the speed to 0.1; nothing more within 1e-5 of the target horizontally; the horizontal offset is scaled by `1 - |0.7 dy| / h`; `yRot = approachDegrees(wrap(yRot + 90), wrap(atan2(dz, dx) * 57.295776), 4) - 90`; a turn under 3 degrees speeds up by `0.005 * 1.8 / speed` toward 1.8, otherwise toward 0.2 by 0.025; `xRot = -(atan2(-dy, h) * 57.2957763671875)`; the wanted motion is `(speed cos(yRot + 90), speed sin(xRot), speed sin(yRot + 90))` times the offset's normalised absolute parts, and the motion moves a fifth of the way to it.
  - `PhantomAttackPlayerTargetGoal`: `nextScanTick` starts at `reducedTickDelay(20)` (10) and resets to `reducedTickDelay(60)` (30) at each scan; `getNearbyPlayers(forCombat().range(64), this, box.inflate(16, 64, 16))` sorted by `getY`, highest first; the first that passes `canAttack(DEFAULT)` (with line of sight) becomes the target. It continues while the target passes `DEFAULT`.
  - `PhantomAttackStrategyGoal` (no flags): `canUse` the target passes `DEFAULT`; `start`: `nextSweepTick = adjustedTickDelay(10)`, CIRCLE, `setAnchorAboveTarget`; `stop`: the anchor becomes `getHeightmapPos(MOTION_BLOCKING, anchor).above(10 + nextInt(20))`; `tick` in CIRCLE: when `--nextSweepTick <= 0`, SWOOP, `setAnchorAboveTarget`, `nextSweepTick = adjustedTickDelay((8 + nextInt(4)) * 20)` and `playSound(PHANTOM_SWOOP, 10, 0.95 + nextFloat * 0.1)`. `setAnchorAboveTarget`: nothing without an anchor; else the target's block `above(20 + nextInt(20))`, raised to `seaLevel + 1` when below sea level.
  - `PhantomSweepAttackGoal` (MOVE): `canUse` a target and SWOOP; `canContinueToUse` false for no target, a dead one, a spectator or creative player, or `!canUse()`, then the cat search (commit 2); `stop`: `setTarget(null)`, CIRCLE; `tick`: `moveTargetPoint = (x, getY(0.5), z)` of the target; when `box.inflate(0.2)` meets the target's box, `doHurtTarget`, CIRCLE and, unless silent, level event 1039; else a horizontal collision or `wasHurtRecently()` (`hurtTime > 0`) sets CIRCLE.
  - `PhantomCircleAroundAnchorGoal` (MOVE): `canUse` no target or CIRCLE; `start`: `distance = 5 + 10r`, `height = -4 + 9r`, `clockwise = nextBoolean ? 1 : -1`, `selectNext`; `tick`: `nextInt(adjustedTickDelay(350))` a new height, `nextInt(adjustedTickDelay(250))` one more block of distance (past 15 back to 5, and the direction flips), `nextInt(adjustedTickDelay(450))` a new `angle = 2 pi r` and `selectNext`; within distance² 4 of the target point `selectNext`; a target point below with a non-air block under it: `height = max(1, height)` and `selectNext`; above with a non-air block over it: `min(-1, height)` and `selectNext`. `selectNext`: no anchor becomes its own block; `angle += clockwise * 15 * 0.017453292`; the point is the anchor's corner plus `(distance cos, -4 + height, distance sin)`.
  - `LevelEventHandler` 1039: `entity.phantom.bite`, HOSTILE, 0.3, `0.9 + 0.1 * level.random.nextFloat()`, at the block's centre. `LivingEntity.wasHurtRecently` is `hurtTime > 0`. `Mob.checkDespawn` discards a type that is not `isAllowedInPeaceful` (the phantom's `notInPeaceful`) on Peaceful.
- **Changed:**
  - Entities crate: new family `phantoms` (key 80) in group `night`: `night/phantoms.rs` (the `Phantom` state with `size`, `attack_damage`, `speed` and its `MonsterAi`; `set_size`; `move_control`, the `PhantomMoveControl` port with `approach`, `approachDegrees` and `Mth`'s tables; the tick), `night/phantoms/goals.rs` (the four goals in `MonsterGoalContext` terms and the `MonsterSpec`; the shared state `PhantomGoals { phase, anchor, move_target, sea_level, hurt_recently, swoop, bit }` in `GoalExtra`) and `night/phantoms/tests.rs`. `night.rs`: `pub mod phantoms;` and `phantoms: phantoms::Phantom`.
  - The tick: `family_base_tick`; with AI, the vex's idle-clock reset within 32 blocks of a player, the trimmed velocity, the goal context filled from the core (the core's random lent to the goals), `MonsterAi::tick_goals` on full ticks (never `family_goal_step`), then the move control on the core's yaw, pitch (`look.pitch`, sent as `ServerMob.x_rot`) and velocity, the head yaw set to the yaw, and `travel_flying` with no input; then the swoop voice, the bite as `family_strike` with a `PlayerHitKind::Melee` hit of `attack_damage` from where it bit, the bite voice, and `family_after_move`.
  - World crate (`natural_spawner/families/night.rs`): the phantom row (0.9 x 0.5, `NoRestrictions`, `MotionBlockingNoLeaves`, `Rules::Family { creature: false }`, `Walk::Neutral`, removed far away, cluster 4, Monster, follow range 16, no babies), its rule (a spawner, or `mob_spawn_rules`: `Mob.checkMobSpawnRules`) and its finalize (`anchor_pos` five blocks up, `size` 0).
  - Terrain: `families/night/phantoms.rs` (`TerrainFamily`: tracking range 8, `VOLATILE` `anchor_pos`, shadow 0.75, no textures yet, `read` and `fields` for `size` and `anchor_pos`, `draw` empty until phantoms-drawn); `families/night.rs`: `mod phantoms;` and the type.
  - Terrain tests that used the phantom as an unsimulated mob: `server.rs`'s `live_shulkers_stay_far_from_players_while_dormant_monsters_go` is now `live_shulkers_stay_far_from_players_while_other_monsters_go` (decision 2); D's `a_trial_spawner_picks_its_mob_and_waits_for_players_in_the_simulation_area` and D0's `every_registered_kind_is_simulated_and_summonable` now use `minecraft:giant` as the kind nobody simulates.
- **Tests added:**
  - Entities `world::families::night::phantoms::tests`:
    - `a_phantom_is_nine_tenths_by_half_with_twenty_health_and_its_kind`: size, eye, health, key 80, census, `mob_candidates`, `is_enemy`, the traits, the goal names, follow range, loot table and 5 experience.
    - `a_bigger_phantom_grows_fifteen_percent_a_size_and_bites_for_six_plus_its_size`: size 0 keeps 2; size 4 is 1.44 x 0.8 with eye 0.28 and bites for 10; 70 clamps to 64; back to 0 it bites for 6.
    - `a_phantom_circles_five_to_fifteen_blocks_around_its_anchor_without_falling`: 600 ticks; the anchor stays at its hatching block, it goes round, stays between 70 and 86 and within 17.5 blocks, and takes no fall.
    - `its_move_control_turns_four_degrees_a_tick_and_speeds_to_one_point_eight`: 4 degrees a tick, 0.125 while turning, 1.8 on the straight, the velocity follows, it noses up to a higher target, a wall turns it 176 degrees with speed 0.125, and nothing changes over its target.
    - `a_phantom_picks_the_highest_player_in_sight_within_its_box`: none before tick 21, then the highest attackable player in its box over a lower one, a creative and a spectator one higher up, and one out of the box; never a player under a roof; the next scan 30 goal ticks after an empty one; creative ends it.
    - `a_phantom_swoops_ten_ticks_after_finding_a_target_bites_and_climbs_away`: the target at tick 21; the swoop sound (volume 10, pitch 0.95-1.05) 8 ticks later; a Melee hit of 2 with the bite sound (volume 0.3, pitch 0.9-1.0) at its block's centre; the target dropped; the anchor 10-29 above the ground; higher again 40 ticks later; the next dive only after the next scan.
    - `a_wall_or_a_hit_breaks_off_a_swoop`: the sweep goal alone with a wall, a recent hit and a spectator; in the world a hit mid-dive drops the target with no bite.
    - `its_anchor_never_sinks_below_sea_level`: a player at y 10 gives an anchor at 64, one at y 50 gives 70-89; the circle starts 5-15 blocks around its anchor.
  - World `natural_spawner::families::night::tests::a_phantom_row_spawns_from_spawners_and_finalizes_its_anchor_five_up`: the row's fields, the rule on grass and glass with and without a spawner, a summon's `anchor_pos` five up and `size` 0, a summon with a tag not finalized.
  - Terrain `families::night::phantoms::tests`: `a_phantom_saves_and_loads_with_its_size_and_anchor` (size 4 bites for 10 and is 1.44 x 0.8; health, persistence, yaw and anchor kept; a fresh one saves `size` 0 and no anchor; 900 clamps to 64) and `a_friend_sees_a_phantom_from_its_look_tag` (`anchor_pos` dropped, `size` kept, the friend's mirror 1.9 times as wide, tracking range 8, an empty family show, the pitch reaching `ServerMob.x_rot`).
- **Decisions:**
  - Decision 1 (bite damage): 2 at size 0 and `6 + size` after a size change, as the 26.3 server does.
  - Decision 2 (the `server.rs` test): the shulker half stays; the second mob is a live phantom that `check_despawn` removes 3000 blocks from the player and on Peaceful, and keeps near a player; the test is renamed `live_shulkers_stay_far_from_players_while_other_monsters_go`. The two other tests that needed an unsimulated kind (D's trial spawner, D0's simulated-kinds survey) use `minecraft:giant`, which no round 10 track simulates (D0 picked it for its test rows for the same reason).
  - Decision 4 (the bite sound): a mob voice with level event 1039's sound and volume at its block's centre, not a `level_events.rs` arm. Its pitch is drawn from the entity world's level random, as the evoker fangs' client-side sound is, so the phantom's own random keeps vanilla's sequence.
  - The goals read vanilla's raw `getTarget()` (`MonsterGoalContext.target`), not the filtered `target()`, so the circle goal's "no target" and the sweep goal's own checks behave as vanilla's. `canAttack(DEFAULT)` is `can_attack` plus line of sight (`sees`); `forCombat().range(64)` measures to the player's feet, without the visibility percent (left for later).
  - The fork's `Goal::stop` gets no world, so the strategy goal's `can_continue` reads the anchor column's `MOTION_BLOCKING` height when it is about to fail, and `stop` adds `10 + nextInt(20)` to it: the random draw stays where vanilla's is.
  - The next dive after a bite (the brief's "160-220 ticks"): vanilla's sweep goal clears the target when it stops, so the strategy goal stops one full tick later and its 160-220-tick `nextSweepTick` is never reached after a bite. The phantom dives again once the target goal's next scan (30 goal ticks, about 60 game ticks) finds the player, plus the 10-tick warm-up. The owner's steps say "every few seconds" for this.
  - The move control keeps `yRot` unwrapped as vanilla's `setYRot` does; `xRot` is clamped to ±90 as `setXRot` does. The body yaw and head yaw both follow `yRot` (`PhantomBodyRotationControl`).
  - Order after the move: the bite and the voices, then `family_after_move` (blocks and pushing), as vanilla's `aiStep` runs `applyEffectsFromBlocks` and `pushEntities` after `travel`.
  - The row's walk is `Walk::Neutral`: the phantom is a `Mob`, not a `PathfinderMob`, so vanilla's `checkSpawnRules` has no walk target value.
- **Runs:** `test -p minecraftoss-entities -p minecraftoss-world -p minecraft_terrain --locked --lib --no-fail-fast` (583, 142 and 1754 passed, 3 ignored), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** release build, survival, Normal, flat open ground above y 63, no roof, `/time set midnight`, the kit below, the host is op. The phantom is not drawn yet (phantoms-drawn), so follow it by its sounds. Name each screenshot by its label.
  1. Use a phantom egg on the ground 5 blocks away and stand still in the open: now and then a phantom screech sounds from above. About a second after it hatched a loud swoop screech sounds, and a few seconds later a crunching bite takes 1 heart. It bites again every few seconds. **m3p-live-1** (the health bar after the second bite)
  2. Build a 5 x 5 cobblestone roof 3 blocks high on four pillars, stand under it and wait 30 seconds: no swoop and no bite. Step out: it bites again within about 5 seconds. **m3p-live-2**
  3. `/gamemode creative` and wait 30 seconds: no bites. `/gamemode survival`.
  4. `/summon minecraft:phantom ~ ~1 ~3 {NoAI:1b}` and swing the diamond sword at chest height 3 blocks in front of you: it takes the hits (its hurt sound), dies on the third or fourth and drops experience, sometimes a phantom membrane (often with this Looting III sword). Do it again and kill the second one with the M4A1 from 10 blocks. **m3p-live-4**
  5. `/summon minecraft:phantom ~ ~8 ~ {size:6}`: its bite takes 6 hearts. Save and quit with it alive and load: it still bites for 6 hearts. **m3p-live-5**
  6. `/difficulty peaceful` with a phantom around: it is gone (no more screeches). `/difficulty normal`.
  7. Friend, two windows (`scripts\mp-local.ps1`): the friend builds a 5-block pillar of cobblestone and stands on it, the host stays on the ground 3 blocks away, and the host hatches a phantom: the friend is bitten, the host is not, until the friend comes down. **m3p-live-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:phantom_spawn_egg*16,minecraft:cobblestone*64,minecraft:diamond_sword+looting=3,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:water_bucket
```

#### 2. Minecraft phantoms burn by day and shy away from cats

After 7a93ae34 ("Minecraft phantoms circle, swoop and bite like vanilla").

- **javap (26.3 `client.jar`):**
  - `Mob.aiStep` calls `burnUndead` after `LivingEntity.aiStep` when the type is in `#burn_in_daylight` (the phantom is). `burnUndead` needs `isAlive()` and `isSunBurnTick()` (the `monsters_burn` environment attribute, the light roll, out of water and rain, the sky in view), then the head slot takes the sun or `igniteForSeconds(8)`. A NoAI mob still runs `aiStep`, so it burns too.
  - `PhantomSweepAttackGoal.canContinueToUse`, after the target checks and `canUse()`: when `tickCount > catSearchTick`, `catSearchTick = tickCount + 20`, `getEntitiesOfClass(Cat, getBoundingBox().inflate(16), ENTITY_STILL_ALIVE)`, every cat found `hiss()`es, and `isScaredOfCat = !cats.isEmpty()`. It returns `!isScaredOfCat`, so between searches the last answer stands. The search runs only while the phantom swoops.
  - `Cat.hiss()` is `makeSound(getSoundSet().hissSound())`: `Mob.makeSound` plays it at `getSoundVolume()` (1.0; `Cat` does not override it) and `getVoicePitch()` from the cat's random (`(r - r) * 0.2 + 1`, `+ 1.5` for a kitten). Ocelots are not `Cat`s.
- **Changed (entities crate):**
  - `night/phantoms/goals.rs`: `CatSeen { id, low, high }`, `PhantomGoals.cats` and `hiss`, `SweepAttackGoal { scared_of_cat, cat_search_tick }` and the search in its `can_continue` (every cat whose box meets the phantom's box grown by 16 goes into `hiss`).
  - `night/phantoms.rs`: `cats_of(world)` (the living cats of `felines()`, not ocelots, with their boxes), filled into the goals before they tick and cleared after. Each cat in `hiss` gets its sound set's hiss as a voice at its feet with its own voice pitch, through `feline_mut`; the cat code is unchanged. After `family_after_move`, `burn_undead` while the phantom is alive, with or without AI, from `EntityWorld.monsters_burn`, with no head item.
- **Tests added** (`world::families::night::phantoms::tests`):
  - `a_phantom_catches_fire_under_the_day_sky_but_not_in_rain_or_at_night`: under an open day sky it catches fire and is hurt within 300 ticks, without AI too; never in rain or at night.
  - `a_cat_within_sixteen_blocks_breaks_off_a_swoop_and_hisses`: the sweep goal alone: two cats in reach (one at the edge of the 16-block box) both hiss and it stops; no search for the next 20 ticks while it stays scared; a cat 20 blocks off does nothing. `cats_of` leaves out an ocelot and a dead cat. In the world (600 ticks): with a cat beside the player the cat hisses before any bite; with no cat, a cat 40 blocks off or an ocelot beside the player nothing hisses and the phantom bites.
- **Decisions:**
  - Cats need no `Prey::Kinds` (M30, correction 9): the phantom's own sweep goal looks for cats; `feline.rs`, `feline_ai.rs` and `world/felines.rs` are not edited.
  - The cat list is rebuilt from `felines()` each tick before the goals run, because goals only see `&dyn World`; it is cleared after, so a copied entity world carries no stale cats.
  - Burning runs after `family_after_move`, as vanilla's `Mob.aiStep` calls `burnUndead` after `LivingEntity.aiStep` (which moves the mob, applies the blocks and pushes), and only while alive.
- **Runs:** `test -p minecraftoss-entities --locked --lib` (585 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings). The terrain and world crates are unchanged by this commit.
- **Test steps:** release build, survival, Normal, flat open ground above y 63, no roof, the kit below, the host is op. The phantom is not drawn yet (phantoms-drawn), so follow it by its sounds. Name each screenshot by its label.
  1. `/weather clear`, `/time set midnight`, `/summon minecraft:phantom ~ ~2 ~4 {NoAI:1b}`, then `/time set noon`: within a few seconds the phantom's hurt sound comes about once a second, and soon its death sound (it drops no experience or membrane: no player killed it). **m3p-burn-1**
  2. `/weather rain` and step 1 again: no hurt sounds for 30 seconds. `/weather clear`. **m3p-burn-2**
  3. `/time set midnight`. Use a cat egg right beside you and stand next to the cat, then use a phantom egg 5 blocks away: the swoop screeches come, but most dives break off with a cat's hiss instead of a bite. **m3p-cat-1**
  4. Walk 40 blocks away from the cat with the phantom following: the bites come back. **m3p-cat-2**
  5. Use an ocelot egg beside you instead of a cat: the ocelot does not scare it, and it bites. **m3p-cat-3**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:phantom_spawn_egg*8,minecraft:cat_spawn_egg*4,minecraft:ocelot_spawn_egg*2,minecraft:golden_apple*8,minecraft:cooked_beef*32
```

### Group phantoms-drawn

#### 1. Minecraft phantoms are drawn like vanilla

After 6c28a475 ("Minecraft phantoms burn by day and shy away from cats").

- **javap (26.3 `client.jar`):**
  - `PhantomModel.createBodyLayer` (64 x 64): `body` (0,8) box(-3,-2,-8; 5,3,9), xRot -0.1, holding `tail_base` (3,20) box(-2,0,0; 3,2,6) at (0,-2,1) with `tail_tip` (4,29) box(-1,0,0; 1,1,6) at (0,0.5,6); `left_wing_base` (23,12) box(0,0,0; 6,2,9) at (2,-2,-8), zRot 0.1, with `left_wing_tip` (16,24) box(0,0,0; 13,1,9) at (6,0,0), zRot 0.1; `right_wing_base` and `right_wing_tip` mirrored (`CubeListBuilder.mirror`) at (-3,-2,-8) and (-6,0,0), boxes from -6 and -13, zRot -0.1; `head` (0,0) box(-4,-2,-5; 7,3,5) at (0,1,-7), xRot 0.2.
  - `PhantomModel.setupAnim`: `f = flapTime * 7.448451 * 0.017453292`; both left wing parts get `zRot = Mth.cos(f) * 16 * 0.017453292` (replacing their 0.1), the right parts its negative; both tail parts get `xRot = -(5 + Mth.cos(f * 2) * 5) * 0.017453292`.
  - `PhantomRenderer`: shadow 0.75, `textures/entity/phantom/phantom.png`, the `PhantomEyesLayer`; `extractRenderState`: `flapTime = getUniqueFlapTickOffset() + ageInTicks` (`id * 3`), `size`; `scale`: `s = 1 + 0.15 * size`, `scale(s, s, s)`, `translate(0, 1.3125, 0.1875)`; `setupRotations`: `MobRenderer`'s, then `Axis.XP` by `xRot`. No `getFlipDegrees` override (90). `LivingEntityRenderer.submit`: `setupRotations`, `scale(-1, -1, 1)`, the renderer's `scale`, `translate(0, -1.501, 0)`. The model's render type is `EntityModel`'s default `entityCutoutNoCull`.
  - `PhantomEyesLayer`: `RenderTypes.eyes(phantom_eyes.png)`; `RenderPipelines.EYES` is `EYES_SNIPPET` (`EMISSIVE`, `NO_OVERLAY`, `NO_CARDINAL_LIGHTING`) with TRANSLUCENT blending and no depth write.
  - `phantom_eyes.png` (26.3 jar): four opaque texels, (5,6), (6,6), (10,6) and (11,6), all on the head's north face (u 5 to 12, v 5 to 8). No other part carries eye texels.
- **Changed (terrain):**
  - New `families/night/phantoms/render.rs`: `PARTS` (the eight parts with their parents), `flap_offset`/`flap_time` (`id * 3 + age`), `flap` (wing and tail angles), `part_poses` (each part under its parent), `model_root` (the body turn with the death tip, the dive's pitch from `MobPose.head_pitch`, the flip, the size scale and the 1.3125/0.1875 lift) and `append_phantoms`: the model in the cell's light, then the head again in `phantom_eyes` at light 15, unshaded and without the hurt overlay.
  - `families/night/phantoms.rs`: `mod render;`, `TEXTURES` (phantom and phantom_eyes), `CAGE_MARGIN` 0.3, and `draw` into `out.models` (the mesh without back-face culling).
  - Friends draw it from their mirror world as the host does; the size comes in the look tag and the pitch in `ServerMob.x_rot` (phantoms-live), so nothing in the protocol changed.
- **Tests added** (terrain `families::night::phantoms::render::tests`):
  - `a_phantom_is_eight_cubes_on_a_64_texture`: the eight parts in order, each box's UVs inside 64 x 64, parents before children, the right wing mirrored; the drawn phantom is 54 faces (the eight cubes and the head again), the model in the phantom's atlas region and the cell's light, shaded and unhurt; the eyes in their own region at light 15, unshaded and without overlay, on the head's vertices; 48 faces with no eyes texture; nothing for a phantom the client has not seen.
  - `its_wings_flap_sixteen_degrees_and_its_tail_curls_with_the_flap_time`: 16 degrees and a 10-degree curl at flap time 0, level wings and a straight tail a quarter beat later, the other way half a beat later, the same again a full beat (48.3 ticks) later; `flap_time(7, 2.5)` is 23.5; the wing tip and tail tip hang from their bases under the body's -0.1; both wingtips move together, more than 18 pixels out.
  - `a_size_four_phantom_draws_one_point_six_times_larger_and_pitches_with_its_dive`: every vertex of a size 4 phantom is 1.6 times as far from its feet; the body's middle sits 0.24 above the feet; the wings span more than 2.3 blocks; at a dive pitch of about -35 degrees every vertex is the level model turned by that pitch about the feet, and the head is below the tail.
- **Decisions:**
  - The render file is `families/night/phantoms/render.rs` (a submodule, as M1's polar bears and rabbits and D's breezes do), not `phantoms_render.rs` beside the family file as the brief says: the group file `night.rs` keeps only registration lines. Its tests are `families::night::phantoms::render::tests`.
  - Eyes: only the head is drawn again (the texel check above). They go into the same cutout mesh as the body, at light 15, unshaded and with no hurt overlay, as `spider_render.rs` and `enderman_render.rs` draw their eyes. Vanilla blends them TRANSLUCENT without writing depth; the four texels are fully opaque, so the picture is the same.
  - `CAGE_MARGIN` 0.3: the phantom's 2.7-block wingspan at the cage scale of 0.53125 reaches about 0.23 past the cage's sides, as the wings do in vanilla's spawner.
  - The body's yaw is the client's generic `BodyRotationControl` (`client_mobs.rs`, on D0's never-edit list). Vanilla's `PhantomBodyRotationControl` sets the body to the yaw every tick; the generic control does the same whenever the phantom moves, which a phantom with AI always does. A still `NoAI` phantom's body turns to its head, and its head yaw is its yaw, within 20 ticks.
  - The flap time counts each viewer's own ticks (`ClientMob.tick_count`), as vanilla's client `tickCount` does, so two windows can show the wings at different points of the beat.
- **Runs:** `test -p minecraft_terrain --locked --lib` (1757 passed, 3 ignored), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** release build, survival, Normal, flat open ground above y 63, no roof, `/time set midnight`, the kit below, the host is op. Name each screenshot by its label.
  1. Use a phantom egg on the ground 5 blocks away: a dark blue-grey phantom with long wings and two small glowing green eyes circles 5 to 15 blocks around a point about 5 blocks above where it hatched. Its wings beat slowly up and down and its tail curls with each beat; it tips its body up when it climbs and down when it sinks. **m3p-drawn-1**
  2. Stand in the open and watch it dive at you: it points its head down at you as it comes, then levels out and climbs away. **m3p-drawn-2**
  3. Hit it with the sword: it flashes red, but its eyes stay green. Kill it: it tips over onto its side as it dies. **m3p-drawn-3**
  4. `/summon minecraft:phantom ~ ~3 ~5 {NoAI:1b,size:4}` and `/summon minecraft:phantom ~4 ~3 ~5 {NoAI:1b}`: the first is 1.6 times as large as the second, and both beat their wings in place. **m3p-drawn-4**
  5. Build a 3 x 3 cobblestone hut 3 blocks high with no light inside, stand in it and run `/summon minecraft:phantom ~ ~1 ~1 {NoAI:1b}`: its body is dark, its eyes glow bright green. **m3p-drawn-5**
  6. Place the spawner and use a phantom egg on it: a small phantom turns slowly inside the cage, its wingtips poking out of the cage's sides. **m3p-drawn-6**
  7. Friend, two windows (`scripts\mp-local.ps1`): the host hatches a phantom at night. The friend sees it with the same model and size and the same dives (its wings may beat at another point of the beat). **m3p-drawn-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:phantom_spawn_egg*8,minecraft:diamond_sword,minecraft:spawner,minecraft:cobblestone*64,minecraft:golden_apple*8,minecraft:cooked_beef*32
```

#### 2. Minecraft phantoms flap and trail specks from their wings like vanilla

After 240d4d55 ("Minecraft phantoms are drawn like vanilla").

- **javap (26.3 `client.jar`):**
  - `Phantom.tick` on the client, after `Mob.tick`: `c(t) = Mth.cos((getUniqueFlapTickOffset() + t) * 7.448451 * 0.017453292 + 3.1415927)`. When `c(tickCount) > 0` and `c(tickCount + 1) <= 0`, `playLocalSound(this, PHANTOM_FLAP, getSoundSource(), 0.95 + nextFloat * 0.05, 0.95 + nextFloat * 0.05)` (volume, then pitch). Then every tick `w = getBbWidth() * 1.48`, `dx = Mth.cos(yRot * 0.017453292) * w`, `dz = Mth.sin(yRot * 0.017453292) * w`, `dy = (0.3 + c(tickCount) * 0.45) * getBbHeight() * 2.5`, and two `MYCELIUM` particles with zero speed at `(x + dx, y + dy, z + dz)` and `(x - dx, y + dy, z - dz)`. `ClientLevel.tickNonPassenger` counts `tickCount` up before `tick`.
  - `c` falls through zero once a full beat, 360 / 7.448451 = 48.33 ticks, so the flap sounds every 48 or 49 ticks (2.4 s). `TICKS_PER_FLAP` (25, `Mth.ceil(180 / 7.448451)`) is the server's `isFlapping` for the `minecraft:flap` game event, half a beat.
  - `particles/mycelium.json` (26.3 jar): the one texture `minecraft:generic_0`. `SuspendedTownParticle.<init>` after the `SingleQuadParticle` base: grey `nextFloat * 0.1 + 0.2` on all three channels, `setSize(0.02, 0.02)`, `quadSize *= nextFloat * 0.6 + 0.5`, the speed times `(double) 0.02f`, `lifetime = (int) (20 / (nextFloat * 0.8 + 0.2))`; `tick`: `lifetime--` and removal at 0 or less, `move` without collisions, the speed times 0.99; layer OPAQUE. Its `Provider` (mycelium) keeps the grey, `HappyVillagerProvider` sets white.
- **Changed (terrain):**
  - `ambient_particles.rs`: `Kind::Mycelium` after `Kind::HappyVillager`, with `Particle::mycelium` after `happy_villager`, the `generic_0` sprite, the constant size, the `add` arm and the `linger` tick, each next to the happy villager's.
  - `families/night/phantoms/render.rs`: `FLAP_SOUND`, `SPECK_REACH`, `wing_beat(offset, tick)`, `flaps(offset, tick)` and `specks(at, y_rot, width, height, beat)`.
  - `families/night/phantoms.rs`: `client_effects`. For each phantom the viewer has, from that viewer's `ClientMob.tick_count`, position and `y_rot` and the mirror mob's box: the flap sound into `ClientFx.sounds` and the two specks into `ClientFx.ambient`. The host's and each friend's entity view run it, so every player hears and sees it.
- **Tests added** (terrain):
  - `ambient_particles::tests::mycelium_lingers_grey_for_twenty_to_a_hundred_ticks`: 40 seeds; grey 0.2 to 0.3, 20 to 100 ticks, speed under 0.006, size 0.05 to 0.22 and constant, opaque, `generic_0`; it drifts through stone with 0.99 drag and is gone the tick after its lifetime.
  - `families::night::phantoms::tests::a_phantom_flaps_once_every_wing_beat_for_every_viewer`: over 400 ticks at least 7 flaps 48 or 49 ticks apart, each where `flaps` says, at the phantom, volume and pitch 0.95 to 1.0; a viewer that came 10 ticks later hears every flap 10 ticks later; a viewer that never saw the phantom hears and sees nothing.
  - `families::night::phantoms::tests::a_phantom_trails_two_grey_specks_from_its_wingtips_each_tick`: at yaw 0, yaw 90 and size 4, two mycelium specks a tick, 1.48 widths either side across its facing, `(0.3 + 0.45 c) * 2.5` heights up, rising and falling with the beat; none for a phantom the viewer has not seen or one that is removed.
- **Decisions:**
  - The flap sound plays once a wing beat (every 48 or 49 ticks), vanilla's client crossing. The brief's "once every twenty-five ticks" (and its owner step's 1.25 s) is the rate of the server's `isFlapping` game event, which waits for W (see Left for later), so the test is named `a_phantom_flaps_once_every_wing_beat_for_every_viewer`.
  - Volume and pitch come from the entity view's random (vanilla: the client phantom's own random). The sound plays where the phantom was when it flapped; vanilla's `EntityBoundSoundInstance` follows the phantom while the short flap plays.
  - `Particle::mycelium` draws the grey from the view's random right after the base particle, as vanilla's constructor does. The shared `happy_villager` (E1's file), which skips that draw, is left as it is.
  - Specks are added at any distance from the camera, as the endermite's portal sparks are. Vanilla's `addParticle` drops particles more than 32 blocks from the camera and thins them on lower particle settings, which the fork has no setting for.
  - Row 39 of `docs/round10/mobs.md` stays `drawn`; `done` comes with the spawner (phantoms-spawner).
- **Runs:** `test -p minecraft_terrain --locked --lib` (1760 passed, 3 ignored), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** release build, survival, Normal, flat open ground above y 63, no roof, `/time set midnight`, the kit below, the host is op. Name each screenshot by its label.
  1. Use a phantom egg 5 blocks away: about every two and a half seconds a soft wing-flap sound comes from it as its wings sweep down, and small dark grey specks trail from both wingtips, hanging in the air for a second or more before they fade. **m3p-flap-1**
  2. `/summon minecraft:phantom ~ ~3 ~5 {NoAI:1b}`: it hangs in place, specks spill from both wingtips, rising and falling with the beat, and its flap sounds every two and a half seconds. `/summon minecraft:phantom ~5 ~3 ~5 {NoAI:1b,size:4}`: its specks start further out to the sides. **m3p-flap-2**
  3. Friend, two windows (`scripts\mp-local.ps1`): the host hatches a phantom at night; the friend hears its flaps and sees the specks trail from its wingtips. **m3p-flap-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:phantom_spawn_egg*8,minecraft:cooked_beef*16
```

### Group phantoms-spawner

#### 1. Phantoms come for Minecraft players who have not slept in three days

After 6b7491c7 ("Minecraft phantoms flap and trail specks from their wings like vanilla").

- **javap (26.3 `client.jar`):**
  - `PhantomSpawner.tick(level, spawnEnemies)`: returns without `spawnEnemies` or the `spawn_phantoms` rule; `--nextTick > 0` returns, else `nextTick += (60 + nextInt(60)) * 20` from the level random; returns when `getSkyDarken() < 5` and the dimension has sky light. Then for each player of `level.players()`: spectators are skipped; with sky light the block position needs `y >= getSeaLevel()` and `canSeeSky`; `getCurrentDifficultyAt(pos).isHarderThan(nextFloat * 3)`; `nextInt(Mth.clamp(TIME_SINCE_REST, 1, MAX_VALUE)) >= 72000`; the spot is `pos.above(20 + nextInt(15)).east(-10 + nextInt(21)).south(-10 + nextInt(21))` (drawn in that order) and must pass `NaturalSpawner.isValidEmptySpawnBlock(.., PHANTOM)`; then `1 + nextInt(difficulty.getDifficulty().getId() + 1)` phantoms, each `EntityType.create(NATURAL)`, `snapTo(spot, 0, 0)` (the spot's bottom centre), `finalizeSpawn(level, difficulty, NATURAL, groupData)` and `addFreshEntityWithPassengers`.
  - `BlockAndLightGetter.canSeeSky(pos)`: `getBrightness(SKY, pos) >= 15`. `DifficultyInstance.isHarderThan(f)`: `effectiveDifficulty > f`.
  - `ServerChunkCache.tickChunks` ticks the custom spawners only while mobs spawn, with `spawnEnemies` from `ServerLevel.isSpawningMonsters()` (the `spawn_mobs` and `spawn_monsters` rules; no difficulty check). `MinecraftServer.createLevels` gives only the Overworld its custom spawners, in the order `PhantomSpawner`, `PatrolSpawner`, `CatSpawner`, `VillageSiege`, `WanderingTraderSpawner`.
- **Changed:**
  - World crate (`natural_spawner/families/night.rs`): `CreatureSpawns::spawn_phantoms(level, context, players, sky_light)` runs the loop above for `(block, spectator, time_since_rest)` players; each phantom is `summon(.., "minecraft:phantom", spot centre, None, 0.0)`, which finalizes through the row (anchor five up, size 0, `Mob.finalizeSpawn`'s follow-range bonus). `level/spawning.rs`: `Level::spawn_phantoms(players)` after `spawn_patrol`, built the same way, passing the dimension's `has_sky_light`.
  - Terrain: `custom_spawners.rs` gains `PHANTOM_DELAY` and `PHANTOM_DELAY_SPREAD` (60 and 60 seconds), `PhantomSpawner { next_tick }` with `due(random)` after `PatrolSpawner`, and the fields `phantoms` (Overworld only) and `spawn_phantoms` after the patrol's. `server.rs`: `ServerSim::tick_phantom_spawner(players, rest)` after `tick_patrol_spawner` (gates `spawn_mobs`, `spawn_monsters` and the rule, `due` on the level random, the players' feet blocks with their clocks, `add_saved_entity` for each phantom); its call in `run_tick` right before the patrol's; the rule line after `spawn_patrols`; `TickInput.rest: Vec<(u64, i32)>` after `mob_vitals`, dropped in `remove_player`, and `rest: Vec::new()` in the test inputs `idle_input` and `eye_of_ender.rs`'s `tick_input` (`two_players` and the others build on `idle_input`).
  - Survival: `PlayerQueue.time_since_rest` after `uuid`; `PlayerQueues::rest(ids)` (each player's clock, by entity id); `Entities::set_time_since_rest(uuid, ticks)` after `set_players`, matching the queue by `opener_key`; the `rest:` line after `mob_vitals` in `Entities::tick`'s `TickInput`; and in `sleep::tick`, after `events.written.extend(written)`, every awake or sleeping player of that level hands their clock to that level's `Entities`.
- **Tests added:**
  - Terrain `custom_spawners::tests::the_phantom_spawner_looks_at_once_then_every_one_to_two_minutes`: due on the first tick; the first wait 1199 to 2379 ticks, then 1200 to 2380 in steps of 20, some under 1400 and some over 2200 in 200 looks; only the Overworld has one.
  - World `natural_spawner::families::night::tests::phantoms_come_for_the_sleepless_at_night_under_open_sky` (400 looks per case): none by day or at darkness 4; about half the looks on Normal (140 to 240) and most on Hard (240 to 320); groups of 1 to 3 on Normal and 1 to 4 on Hard, all seen, each group at one spot centre 20 to 34 up and within 10, finalized with the anchor five above the spot, size 0 and their own UUIDs; none at exactly 72000 ticks awake, rested, spectating, on Peaceful, with sky light 14, below sea level, or with stone or water at the spots; with a rested and a sleepless player only the sleepless one draws them.
  - Terrain `server::tests::a_sleepless_player_draws_phantoms_and_a_rested_one_does_not`: none by day, at 72000 ticks, for a rested player or one without a clock, for a spectator, with the rule off, or on Peaceful; then 1 to 3 phantoms around the sleepless player only, 220 to 234 up, sharing their spot, size 0, not persistent, anchored five above; never in the Nether (no spawner).
  - Survival `entities::tests::each_player_time_since_rest_reaches_the_server_tick`: two `sleep::tick`s hand 102 and 1000002 to the host's and friend's queues by UUID, the Nether player's stays 0; the friend's death resets only theirs; an unknown UUID makes no queue.
- **Decisions:**
  - The phantom spawner ticks right before the patrol spawner in `run_tick` (the brief said after it), so the level random is drawn in vanilla's spawner order (phantoms, patrols, cats, traders). The method sits after `tick_patrol_spawner` as the brief says.
  - `CreatureSpawns::spawn_phantoms` takes the dimension's sky light flag, as vanilla skips the darkness and open-sky checks without it. Only the Overworld has the spawner, so it is always true in play.
  - Players are rolled in `TickInput.mob_players` order (the host, then friends), the fork's stand-in for `level.players()`. Dead players are rolled too, as vanilla's list keeps them, and draw nothing because death resets their clock.
  - A player with no entry in `TickInput.rest` counts as 0 ticks awake, as an unset vanilla stat reads 0. A friend who just joined has no queue for one tick and so draws nothing that tick.
  - The group's difficulty for the count is the player's, as in vanilla; `summon` works out the spot's own difficulty for the finalize, which the phantom's finalize does not read.
  - The countdown is not saved, as vanilla's `nextTick` is not: after a load the spawner looks on the first tick.
  - On Peaceful the countdown still runs (vanilla's `spawnEnemies` has no difficulty check), but the effective difficulty is 0, so no roll passes.
- **Runs:** `test -p minecraftoss-world -p minecraft_terrain -p minecraft_survival --locked --lib --no-fail-fast` (143, 1762 with 3 ignored, and 837 passed), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** release build, survival, Normal, flat open ground above y 63 (plains), no roof nearby, the kit below, the host is op. On Normal each look of the spawner brings phantoms about half the time, and the spawner looks every one to two minutes, so allow up to about 5 minutes per step. Name each screenshot by its label.
  1. Save and quit. In the world folder, open `players\stats\<your UUID>.json` in Notepad (in single player it is the only file there). Find `"minecraft:time_since_rest":` and change the number after it to `1000000`, then save. Load the world, `/weather clear`, `/time set midnight`, and stand still in the open: 1 to 3 phantoms appear together high above you (20 to 34 blocks up), screech and dive at you. **m3p-insomnia-1**
  2. `/time set noon` and wait until every phantom has burned, then `/time set midnight`. Build a 5 x 5 cobblestone roof 3 blocks high on four pillars and stand under it for 5 minutes: no phantom comes. Step out into the open: they come again within about 5 minutes. **m3p-insomnia-2**
  3. Burn them by day again, `/gamerule spawn_phantoms false`, `/time set midnight` and stand in the open for 5 minutes: none come. `/gamerule spawn_phantoms true`. **m3p-insomnia-3**
  4. `/difficulty peaceful` with phantoms around: they vanish and none come for 5 minutes. `/difficulty normal`.
  5. Place the bed and sleep in it at night until morning. The next night, stand in the open for 5 minutes: no phantom comes (the clock started again at 0; it takes three days awake). **m3p-insomnia-4**
  6. Friend, two windows (`scripts\mp-local.ps1`), after the friend has joined the host's world once: with both games closed, set the number in the friend's stats file (the other file in the host's `players\stats`) to `1000000` and in yours to `0`. Load, `/time set midnight`; the friend stands in the open 40 blocks away from the host. Phantoms appear above the friend, not above the host, and dive at the friend; both windows see them. **m3p-insomnia-friend**
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:red_bed,minecraft:cobblestone*64,minecraft:diamond_sword+looting=3,minecraft:shield,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:water_bucket
```

### Final verification

#### 1. Track M3's lightning and swirl code passes clippy

After 740c204e ("Track M3's segment m3a log gets one combined kit line").

- **Clippy** on the six crates the segment touched (`minecraft_survival`, `minecraft_terrain`, `render_anim`, `render_gpu`, `minecraftoss-entities`, `minecraftoss-world`, `--all-targets`, `-A clippy::approx_constant`): three warnings on the segment's own lines, all in terrain; every other warning is older.
- **Changed (terrain):** `creeper_render.rs`: the swirl's quads (`append_creeper_power`) and the test helper `quad_area` walk `as_chunks::<4>()` instead of `chunks_exact(4)` (`clippy::chunks_exact_to_as_chunks`). `lightning.rs`: the villager line's `Pos` falls back to `unwrap_or(&[])` (`clippy.toml` disallows `unwrap_or_default`). Nothing a player sees changes.
- **Tests added:** none; the swirl and lightning tests cover the lines.
- **Runs:** clippy on `minecraft_terrain` again (none of the three left), `test -p minecraft_terrain -p minecraft_survival -p render_anim -p render_gpu -p minecraftoss-entities -p minecraftoss-world --locked --no-fail-fast` (terrain 1762 passed with 3 ignored, survival 837 and its `vanilla_world` 1, entities 585, world 143, render_anim 224, render_gpu 42; doc-tests empty), `check -p launcher --locked` (green; only the old render_frame and skate warnings).
- **Test steps:** nothing new to see; the steps of lightning-glow 1 (m3-glow-1 to m3-glow-5) and lightning-turns 1 step 4 (m3-turn-4, the villager that becomes a witch, whose turn writes the line with `Pos`) cover these lines.
- **Kit:** none new (the combined kit below).

## Combined kit

The whole track's kit (all segments, mostly the mobs still to come) is in `docs/round10/track-m3.md` (M30, "Combined kit").

Segment m3a (lightning and phantoms), one kit for every step above (27 stacks). To stay at 27 it leaves out a few things the commit kits name: dig the pits by hand (no iron shovel), drop 8 stone bricks instead of the oak planks in m3-strike-2, eat cooked beef instead of golden apples, and before m3-strike-6 run `/give @s minecraft:obsidian 14` (the host is op). Friends read the same `.env`.

```
IW4L_MINECRAFT_KIT=minecraft:phantom_spawn_egg*16,minecraft:cat_spawn_egg*2,minecraft:ocelot_spawn_egg*2,minecraft:cow_spawn_egg*4,minecraft:pig_spawn_egg*4,minecraft:villager_spawn_egg*4,minecraft:creeper_spawn_egg*4,minecraft:mooshroom_spawn_egg*2,minecraft:lightning_rod*4,minecraft:weathered_copper*32,minecraft:redstone_lamp*2,minecraft:stone_bricks*64,minecraft:experience_bottle*16,minecraft:composter*2,minecraft:stone_slab*8,minecraft:light_blue_stained_glass*16,minecraft:cobblestone*64,minecraft:spawner,minecraft:red_bed,minecraft:diamond_sword+looting=3,minecraft:shield,minecraft:water_bucket,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:cooked_beef*32
```
