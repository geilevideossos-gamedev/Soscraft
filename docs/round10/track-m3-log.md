# Track M3 log: night mobs, lightning and the horse family

Branch `track-m3`, worktree `C:\Dev\Soscraft\mc-wt\track-m3`, target dir `D:\iw4l-wt\track-m3`. Brief: `docs/round10/track-m3.md`. Mob list: `docs/round10/mobs.md` (the `## M3` rows only). Registry: `docs/round10/track-d0.md` (D0-2, D0-3).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Vendored files

Files under `third_party/minecraftoss` this track patched or added.

- Added (lightning-strikes 1): `world/src/level/lightning.rs` (with its tests), `entities/src/world/lightning.rs` (with its tests).
- Patched (lightning-strikes 1): `world/src/level/mod.rs` (`pub mod lightning;`, the `lightning_hits` field), `world/src/level/entity.rs` (`LightningData.seed` and `uuid`, `Entity::new` is `pub(super)`, the bolt tick's fire and hits, the copper walk's 3002), `world/src/level/fire.rs` (`fire_survives` and `can_spread_fire_around` are `pub(super)`), `world/src/level/redstone.rs` (the rod's 3002), `entities/src/inside_blocks.rs` (`Hazard::LightningBolt`), `entities/src/world.rs` (`mod lightning;` and its `pub use`), `entities/src/world/pushing.rs` (`exposed_mut` is `pub(super)`).
- Patched (lightning-strikes 2): `entities/src/world/families.rs` (the `thunder_hit` hook), `entities/src/world/families/pests/probe.rs` (test only: `thunder_answers`, `thunder_bolts` and its override). Changed again: `entities/src/world/lightning.rs` (the hook call and its test).
- Patched (lightning-turns 1): `world/src/natural_spawner/spawning.rs` (the witch's arm in `finalize_conversion`, after the villager's). Changed again: `entities/src/world/lightning.rs` (the turns, the creeper's charge, the mooshroom's flip and their tests).

## Hooks

Root hooks this track added (M30, "M3's root hooks").

- **"Registered Minecraft mobs can answer lightning their own way"** (lightning-strikes 2, entities root `world/families.rs`): `Family::thunder_hit(mob, bolt: &Bolt) -> bool` after `fire_immune`, default `false` (`true`: the family took the whole hit, so the base hit is skipped); `FamilyStore::thunder_hit`, the `family_group!` dispatch and `Families::thunder_hit(key, id, bolt) -> Option<bool>` beside `deflect`. `EntityWorld::thunder_hit` asks it for each registered mob alive in the bolt's box. M1's turtles and M3's copper golems override it.

## Protocol

`minecraft_net::PROTOCOL` is not bumped here. `Output.players_lightning` is the server thread's output to the host, not a network message; `Hazard` and `DamageCause` are not on the wire. `ThunderOutcome` (lightning-turns) stays on the host; the charged creeper and the mooshroom's colour reach friends through the look tag (`powered`, `Type`), which is not a message change.

- lightning-drawn 1 ("Minecraft lightning bolts flash and thunder for every player", after 82a5cd0b): `EntitySnapshot` gains `lightning: Vec<minecraft_terrain::lightning::BoltView>` after `crystals`, with `BoltView { id: i32, position: DVec3, seed: i64, life: i32 }`. It rides in `StateFrame.entities`, so old and new builds cannot read each other's entity snapshots.

## Cow-store one-liners outside D0-3's list

None yet.

## State doc changes

For the orchestrator to copy at the merge of segment m3a:

- Under World simulation in `docs/MINECRAFT-SURVIVAL.md`: "Lightning (round 10, track M3): `/summon minecraft:lightning_bolt` strikes like vanilla's bolt. On Normal and Hard it lights its block and up to four around it, each re-flash lights its own block on any difficulty, and fire in an empty obsidian frame lights the portal. Every lit tick it hits every living thing in its box (3 blocks around, 3 below, 9 above): 5 damage that armour softens and shields do not block, with no knockback, and one more fire tick (eight seconds of fire for a mob or player that had just stopped burning). Animals panic, fire-immune mobs are hurt too, dropped items and experience orbs burn. A struck lightning rod powers for 8 ticks and sends the electric spark event (3002) along its axis; struck copper is cleaned with a spark on each cleaned block. Lightning deaths read 'was struck by lightning'."
- Under World simulation in `docs/MINECRAFT-SURVIVAL.md`, after the lightning-turns line (lightning-drawn): "Every player sees and hears a bolt as vanilla draws it: a jagged blue-white bolt 128 blocks tall from its seed, drawn within 64 blocks of the camera and changing shape at each re-flash, with the thunder roll and the impact crack once per bolt within 16 chunks. While a bolt is lit the sky turns pale blue and the world is lit as by day. Friends get the host's bolts within 16 chunks (their view distance at most)."
- Under World simulation in `docs/MINECRAFT-SURVIVAL.md`, after the lightning line (lightning-turns): "Lightning turns a pig into a zombified piglin that never despawns, with a golden sword (one in twenty a golden spear; a baby pig gives a baby), and a villager into a witch that never despawns and can join raids, freeing its job site, bed and meeting point. Both keep their place, name, effects and lead, but not their saddle or held item. A struck creeper is charged (twice the blast, saved and seen by friends), and a mooshroom flips between red and brown once per bolt with its sound and takes no harm. On Peaceful pigs and villagers are only hurt."
- Under World simulation in `docs/MINECRAFT-SURVIVAL.md`, after the lightning-drawn line (lightning-glow 1): "A charged creeper wears vanilla's energy swirl: the blue `creeper_armor` texture scrolling diagonally over a shell two pixels outside its body, unlit and added onto what lies behind it, turning, walking and swelling with the creeper. Friends see it too."

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

## Combined kit

The track's combined kit is in `docs/round10/track-m3.md` (M30, "Combined kit"); every group's kit above draws from it or from the section kits.
