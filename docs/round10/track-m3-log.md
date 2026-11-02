# Track M3 log: night mobs, lightning and the horse family

Branch `track-m3`, worktree `C:\Dev\Soscraft\mc-wt\track-m3`, target dir `D:\iw4l-wt\track-m3`. Brief: `docs/round10/track-m3.md`. Mob list: `docs/round10/mobs.md` (the `## M3` rows only). Registry: `docs/round10/track-d0.md` (D0-2, D0-3).

A commit can't name its own hash, so each entry names the commit before it. Its own hash is in `git log` under its subject.

## Vendored files

Files under `third_party/minecraftoss` this track patched or added.

- Added (lightning-strikes 1): `world/src/level/lightning.rs` (with its tests), `entities/src/world/lightning.rs` (with its tests).
- Patched (lightning-strikes 1): `world/src/level/mod.rs` (`pub mod lightning;`, the `lightning_hits` field), `world/src/level/entity.rs` (`LightningData.seed` and `uuid`, `Entity::new` is `pub(super)`, the bolt tick's fire and hits, the copper walk's 3002), `world/src/level/fire.rs` (`fire_survives` and `can_spread_fire_around` are `pub(super)`), `world/src/level/redstone.rs` (the rod's 3002), `entities/src/inside_blocks.rs` (`Hazard::LightningBolt`), `entities/src/world.rs` (`mod lightning;` and its `pub use`), `entities/src/world/pushing.rs` (`exposed_mut` is `pub(super)`).

## Hooks

Root hooks this track added (M30, "M3's root hooks").

## Protocol

No commit of this track has changed a `ToClient`/`ToServer` message or the codec yet. `minecraft_net::PROTOCOL` is not bumped here. `Output.players_lightning` is the server thread's output to the host, not a network message; `Hazard` and `DamageCause` are not on the wire.

## Cow-store one-liners outside D0-3's list

None yet.

## State doc changes

For the orchestrator to copy at the merge of segment m3a:

- Under World simulation in `docs/MINECRAFT-SURVIVAL.md`: "Lightning (round 10, track M3): `/summon minecraft:lightning_bolt` strikes like vanilla's bolt. On Normal and Hard it lights its block and up to four around it, each re-flash lights its own block on any difficulty, and fire in an empty obsidian frame lights the portal. Every lit tick it hits every living thing in its box (3 blocks around, 3 below, 9 above): 5 damage that armour softens and shields do not block, with no knockback, and one more fire tick (eight seconds of fire for a mob or player that had just stopped burning). Animals panic, fire-immune mobs are hurt too, dropped items and experience orbs burn. A struck lightning rod powers for 8 ticks and sends the electric spark event (3002) along its axis; struck copper is cleaned with a spark on each cleaned block. Lightning deaths read 'was struck by lightning'."

## Left for later

- Advancements (`lightning_strike`, `channeled_lightning`): the fork has none.
- Arrows, tridents, fireballs and other projectiles in the bolt's box get no fire ticks (vanilla `Entity.thunderHit` sets fire on any entity; arrows then burn). End crystals, the ender dragon, item frames and armour stands are not struck (the End has no weather; frames and stands ignore lightning in vanilla).
- An item destroyed by lightning does not spill its contents (`ItemStack.onDestroyed`: bundles and shulker boxes) and posts no `entity_damage` game event; the fork's fire and lava hurts on items do neither either.
- Worn armour on zombies and skeletons does not soften hazards (lava, fire, lightning): their hazard hurt uses the natural armour only, as it did before this track.
- Vanilla's exact `getEntitiesOfClass` order for the hits: the entity world's order is used.

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

## Combined kit

The track's combined kit is in `docs/round10/track-m3.md` (M30, "Combined kit"); every group's kit above draws from it or from the section kits.
