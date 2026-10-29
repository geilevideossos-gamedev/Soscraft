# Track M3: night mobs, lightning and the horse family

## The owner's request

> D. Finish track D: the rest of d5 (trial chambers and trial spawners, lodestones), d3 (deep dark, warden, sculk), and d4 (the remaining mobs), with d4 split into two or three tracks by mob group. **Land the shared changes (mob list, spawn tables, render registry) first so the later merges stay small.**

Round 10 splits d4 into three mob tracks: M1 (land animals), M2 (water and buckets) and M3, this track. Track D0 (`docs/round10/track-d0.md`) is the "shared changes first" part of the request: one registry that every new mob family plugs into, the spawn tables, the client and render registry, flying navigation and the mob list `docs/round10/mobs.md`.

M3 builds:
- the registry group `night` (hit-key bytes 80-95): phantoms (80), allays (81), copper golems (82), creakings (83) and happy ghasts (84);
- the horse family's new kinds, which stay in the cow store (byte 11, D0-2.5): zombie horses, skeleton horses, camels and camel husks;
- P8, natural lightning (D0-6). The skeleton trap needs it, and it also turns pigs, villagers, creepers and mooshrooms.

These are rows 23, 24, 39, 40, 41, 45, 47, 48 and 50 of `docs/round10/mobs.md`.

Paths and line numbers are in `C:\Dev\mashup-crafting` on `minecraft-survival` at `d1a3d028` unless a section says otherwise. Registry names (`Family`, `FamilyMob`, `MobCore`, `Registered`, `TerrainFamily`, `FamilyShow`, `FamilySpawn`, `family_group!`, `terrain_group!`, `family_base_tick`, `family_goal_step`, `family_after_move`, `family_strike`, `PlayerHitKind::Typed`, `keyframes::shared()`, `flying.rs`) are D0's design (track-d0.md D0-2 to D0-6). D0's code may grow fields, so read the merged code before the first commit. Tracks D0, D, W, E1, E2, F, M1 and M2 run at the same time: re-check every line before editing. Vanilla facts were checked with javap on the 26.3 `client.jar` and against the 26.3 data in that jar this round, unless marked "javap during the work". No Mojang code or data is copied: numbers and behaviour only.

---

## M30. Shared points for the track

### Order and waiting

- Branch `track-m3`, worktree `C:\Dev\mc-wt\track-m3`, target dir `D:\iw4l-wt\track-m3` (or `C:\iw4l-wt\track-m3` when D: is short, `decisions.md`). Log `docs/round10/track-m3-log.md`. Run specs `docs/round10/runs/m3a.json` to `m3g.json`.
- **Start:** the track starts on `minecraft-survival` once D0's segment d0a (`registry-core` to `silverfish-drawn`) is merged there. Each segment's base is tagged `r10-m3<x>-base` before its run starts.
- **Segments run in order, m3a to m3g** (M3-groups). Some need another track's segment first. **Before such a segment, the orchestrator merges `minecraft-survival` into `track-m3`** (after that track's merge), then tags the base and starts the run. If the needed segment is not merged yet, the run waits. m3c and m3d are the only pair that do not depend on each other, so they may swap.

| Segment | Needs on `minecraft-survival` first | Why |
|---|---|---|
| m3a | D0 d0a | the registry |
| m3b | nothing new (m3a before it) | E2's e2a and W's w1, if merged, only move single lines: Channeling beside E2's renamed trident arms, game event lines |
| m3c | E2 e2a (and m3b) | `copper-golems-sorting` needs `World::chest_open_count` (E2 `cat-chests`). E1's e1b (banners in `special_models.rs`) is soft: re-check lines for `copper-golems-statues` |
| m3d | E1 e1a (and m3b) | E1's `horse-fall`, `vehicle-hearts` and `horse-gallop` rewrite the horse lines the undead horses edit. E1 owns `horse.rs`, `world/equines.rs`, `world/riding.rs`, `world/emissions.rs` and `horse_render.rs` this round. E1's e1b `horse-dye` is soft |
| m3e | m3d | the same horse lines, and the jump bar in `hud/src/minecraft_hotbar.rs` beside E1's vehicle hearts. Camels extend `HorseKind::is_undead`, `despawn_rule()` and `equine_leashable` from m3d |
| m3f | D0 d0b (and m3e) | `flying.rs` (P3) for allays and the ghastling; camels' per-seat `ride_seat` for happy ghast riders |
| m3g | W w2 (with w1) (and m3b, m3f) | W4's `try_spawn_mob` and `ON_TOP_OF_COLLIDER` for creaking hearts; W's game events (P7) and W4's per-entity listener for the allay's note blocks |

- **Soft dependencies** are one-line hooks into another track's helper (game events, `puff_for` rows). When the helper is on `minecraft-survival`, add the line. When it is not, leave the line out and list it under "Left for later" in the log; the other track adds it if it merges second (W0 rule 3).

### The D0-3 checklist is the law

Follow `docs/round10/track-d0.md` D0-3 for every registered family. The rules that bite most:

1. **A registered family** (phantoms, allays, copper golems, creakings, happy ghasts) is its own files plus the registration lines in M3's three group files:
   - `third_party/minecraftoss/entities/src/world/families/night.rs`: `pub mod <family>;` and the entry in `family_group!(night 80..=95 { .. })`;
   - `crates/minecraft_terrain/src/families/night.rs`: `mod <family>;` and the type in `terrain_group!(night { .. })`;
   - `third_party/minecraftoss/world/src/natural_spawner/families/night.rs`: one `ROWS` row per kind, even for kinds that never spawn naturally (eggs and `/summon` read the size).

   Family files live under `world/families/night/` and `families/night/`.
2. **Never edit D0-3's "never edit" list** for a registered family. If a site seems to need an edit, the registry is missing a hook. Add the hook to D0's root file (`entities/src/world/families.rs`, `world/src/natural_spawner/families.rs` or `crates/minecraft_terrain/src/families.rs`) in its own commit named for the hook, and record it in the log.
3. **The one exception is the horse family** (D0-3, D0-2.5). Camels, camel husks, zombie horses and skeleton horses extend `HorseKind` in the cow store (byte 11). Only M3 edits the cow-store lines for them. The extra cow-store one-liners outside D0-3's list are named in M3-5, M3-6 and M3-7, and each one goes in the log.
4. **Flying goal families never call `family_goal_step`**, because its `monster_ai_step` walks on ground navigation. Phantoms and the adult happy ghast call `MonsterAi::tick_goals` themselves, as `world/vexes.rs` and `world/ghasts.rs` do. Brain families (allays, copper golems, creakings, ghastlings and camels) keep their brain in their state.
5. **Multiplayer:**
   - every drawn state goes through `FamilyShow` (`TerrainFamily::show`/`show_on`), sent as ticks since a start or as countdowns;
   - every saved field that changes while the mob lives goes in `TerrainFamily::VOLATILE`, or friends respawn the mob every 10 ticks;
   - friends load the mob from its look tag, and held or carried items reach them through it (`equipment` is not volatile), as for zombies and piglins;
   - nothing keys on player 0.
6. **Attacks and keyframes:** players are hit with `PlayerHitKind::Melee`, or with `Typed` when the attack has its own damage type. Keyframes are read only through `minecraft_terrain::keyframes::shared()`.
7. **Behaviour that needs a shared file** (blocks, items, player state) keeps its logic in a new M3 file and calls it with one line next to a related arm, never at the end of a list.
8. **Tests** go in the family's own files. D0's generic tests cover the eggs survey, the hit keys and the spawner cages for registered kinds. The horse kinds are not registered, so they go into `HATCHING_EGGS`, `SIMULATED_KINDS`, `VEHICLES`, `DRAWN_KINDS` and `tracking_range` directly.
9. **Two corrections to D0's notes:**
   - cats need no `Prey::Kinds` to scare phantoms: the phantom's sweep goal finds cats itself (M3-2);
   - the copper golem's MISC category needs no change to `FamilySpawn` (M3-4, decision 1).

### M3's root hooks

Each hook is its own commit at a fixed spot, so it never lands next to another M track's hook.

| Hook | Root file | Placed | Commit | Group |
|---|---|---|---|---|
| `Family::thunder_hit(mob, bolt) -> bool`, default false. Also its `family_group!` dispatch and `Families::thunder_hit(key, id, bolt) -> Option<bool>` | entities `families.rs` | after `fire_immune` | "Registered Minecraft mobs can answer lightning their own way" | `lightning-strikes` |
| `TerrainFamily::walk_target(distance: f32) -> f32`, default `(distance * 4.0).min(1.0)` | terrain `families.rs` | after `show_on` | "Registered Minecraft mobs can set their own walk animation speed" | `creakings-drawn` |
| `Family::voice_pitch(mob) -> Option<f32>`, default `None` | entities `families.rs` | before `voice` | "Registered Minecraft mobs can speak at a fixed pitch" | `happy-ghasts-live` |
| `Family::death_ticks`, only as M3-3's fallback (decision 4) | entities `families.rs` | after `despawn` | named for the hook | `creakings-live` |

If a hook with the same name and signature is already on `minecraft-survival` (another M track added it), skip the commit and use it. M3 adds no other root hook. The allay needs neither an ambient hook (no track adds one this round) nor M2's `OWN_DYING_TRAVEL` (M3-9), and the ghastling's shadow uses M1's `TerrainFamily::shadow_radius` (below).

### M3's own shared pieces (one owner each)

| Piece | Where | Written by | Used by |
|---|---|---|---|
| The visual-only bolt, `Level::add_lightning(pos, true)` | `world/src/level/lightning.rs` | `lightning-strikes` | the skeleton trap |
| `FlyingMoveSink`: a brain's walk and look targets over d0b's `FlyingPathNavigation`. It holds 150-250 ticks, paths again when the target moved more than 2 blocks, and waits 0-39 ticks when stuck | new `world/families/night/flying_sink.rs` | `allays-live` | allays, ghastlings |
| `stroll::air_and_water_random_position_within(world, profile, position, view, horizontal, vertical, random)`: `AirAndWaterRandomPos` with its own ranges (10 and 7 for `RandomStroll.fly`) | `third_party/minecraftoss/entities/src/stroll.rs`, right after `air_and_water_random_position` (`:315`), which stays as it is | `allays-live` | allays, ghastlings. M1's parrots and bees may call it; if an identical function from M1 is already on `minecraft-survival`, use that |
| `HorseKind::is_undead()`, the cow store's `despawn_rule()` (vendored `world.rs:5904`) and the `witches.rs:533-535`/`:668-669` lines | `horse.rs`, `world.rs`, `world/witches.rs` | `undead-horses-live` | `camels-live` adds the `CamelHusk` arms |
| `equine_leashable(id)`, called at `leashes.rs:338` | `world/equines.rs` | `undead-horses-ride` | camels add the mob-controlled husk |
| Per-seat `ride_seat` (`minecraft_survival/src/entities.rs:1102-1113`) and the forward seat in `ServerMob.vehicle` | survival and terrain | `camels-riding` | happy ghasts |
| `ambient_particles::Kind` entries: `Mycelium` next to `HappyVillager`, `Trail` next to `EndRod`, `WhiteSmoke` next to `Smoke`, and (optional) `ElectricSpark` next to `Crit` (M1 takes `Poof`, M2 `Flame`, E1 the end; no other brief names `Crit`) | `crates/minecraft_terrain/src/ambient_particles.rs` (E1's file) | `phantoms-drawn`, `creakings-crumble`, `happy-ghasts-ghastling`, `lightning-glow` commit 2 | |

M3 never edits `entities/src/brain_behaviors.rs`. If d0b already made `MoveSink` drive flying navigation, M3 uses that and does not write `FlyingMoveSink`.

### Pieces M3 takes from M1

M1 owns these (`docs/round10/track-m1.md`, its shared hooks and sites tables). M3 uses them once they are on `minecraft-survival` and never adds a second copy.

| Piece | M1 group (segment) | M3 users | If it is not merged when the M3 group runs |
|---|---|---|---|
| `world/families/animal_brain.rs`: `AnimalPanic`, `FollowTemptation`, `AnimalMakeLove`, `BabyFollowAdult` | `goats-live` (m1d) | copper golems (panic 1.5), camels (panic 4.0, temptation, love, following adults), allays (panic 2.5), ghastlings (panic 2.0) | the group writes what it needs privately in its own family file and logs it. The first M3 group that runs after M1's module is merged into `track-m3` switches every M3 user to it in its own commit, "Minecraft night mobs share the animal brain behaviours", and deletes M3's private copies |
| `TerrainFamily::shadow_radius(mob) -> f32` (default: D0's rule, halved for babies) | `armadillos-drawn` (m1e) | the ghastling's 0.475 shadow | the ghastling keeps D0's halved 1.0; left for later |
| `PlayerControls.sprinting` and survival `PlayerView.sprinting` | `armadillos-live` (m1e) | the camel's rider sprint. M3 adds only the line that passes `buttons::SPRINT` through `ride_input` | the rider sprint is left for later; the dash still lands |
| `ClientFx.dust` (block particles) | `sniffers-dig` (m1f) | the creaking's crumble (`creakings-crumble`) | the crumble is left for later |
| `Level::is_thundering` (thunder × rain, in `level/fire.rs`) | `pandas-thunder` (m1b) | `lightning-weather`'s fix at `level/spawning.rs:162` calls it | M3 writes thunder × rain inline at `:162`; the two never share a line |

### The other M tracks: shared sites outside the registry

| Site | M1 (land) | M2 (water) | M3 (this track) |
|---|---|---|---|
| D0's root `families.rs` files | `spared_blocks`, the gift hooks, `shadow_radius`, `ClientFx.item_crumbs`, `ClientFx.dust`, and the `animal_brain.rs` and `long_jump.rs` module lines | `OWN_DYING_TRAVEL` (squids); frogs use `walk_target` | the hooks above. **M3 owns `walk_target`.** Its creakings run earlier than M2's frogs; if M2 lands the same hook first, M3 skips its commit |
| `Family::thunder_hit` overrides | turtles (they wait for `lightning-strikes`) | | copper golems |
| `world/src/level/mod.rs` module lines and `tick_block` (`:839`) | `sniffer_egg`, `turtle_egg`, `beehive` after `wither_build` (`:21`); the sniffer egg after `wet_sponge` in `on_place` and after `FireBlock` (`:842`) in `tick_block`; `hive_alarms` after `bell_rings` (`:208`) | `mob_bucket` after `dispenser` (`:13`), `frogspawn` after `fluid`; frogspawn after `FrostedIceBlock` in `on_place` (`:705`) and after the coral arm in `tick_block` | modules: `creaking_heart` after `container` (`:12`), `lightning` after `fire` (`:19`), the two copper golem modules after `golem_build` (`:20`), `dried_ghast` after `ice` (`:23`). `tick_block`: the dried ghast after `FrostedIceBlock` (`:850`), the creaking heart before `ComposterBlock` (`:860`). Fields: `lightning_hits` after `level_events` (`:207`), `exploded_hearts` after `destroyed` (`:218`, init `:310`). M1 and M2 use other neighbours |
| `level/random_tick.rs` | (eggs) | | the dried ghast after `NetherPortalBlock` (`:323`); `block_random_pos` (`:27`) becomes `pub(super)` |
| `level/container.rs` | beehives | | `chest_positions_in` beside `container_at` (`:120`) |
| `entities/src/stroll.rs` | `air_random_position_towards` before `hover_random_position` (`:302`) | `water_bound_random_position` after `amphibious_random_position` (`:199-221`) | M3 owns `air_and_water_random_position_within` after `:315` |
| `level_events.rs` (E2's file) | 3009 and 2011 beside `BONE_MEAL_USE` | | M3's constants after `LECTERN_PAGE_TURN` (`:10`) and their arms after its arm (`:112`): 3002 (`lightning-glow` commit 2, optional), then 1010 and 1011 (`allays-dance`) |
| `player/src/lib.rs` `World` and `MobWorld` | `destroy_effect`, `biome_in_tag`, `raining`/`thundering`, `sea_level`, the hive methods, each after its own anchor | `nearest_structure`, `best_neighbor_signal`, `tnt_explodes` | M3's copper golem methods go next to E2's `chest_open_count` |
| `entities/src/brain_behaviors.rs` | brains | brains | not edited by M3 |
| `monster_ai.rs`, `feline_ai.rs`, `world/felines.rs` | M1 owns the target-slot lines (`monster_ai.rs:1287`, `:1309`, `:1948`, `:1965`, `:1971`; `feline_ai.rs:196`, `:200`, `:244`) | | not edited. Phantoms read `felines()` and push the hiss through `feline_mut` |
| `mount_inventory.rs` | | the nautilus arm next to the horse lookup | not edited. The camel's saddle-only screen comes from `HorseKind` and `equines.rs:790`. If a test shows it must change, the edit stays inside the horse branch, away from M2's arm |
| `world/riding.rs` horse arms | | (nautiluses use D0's generic arms) | M3 only (D0-2.5) |
| E1's `mob_particles::puff_for` | rows | rows | `("minecraft:camel", 18)`, `("minecraft:allay", 18)` with 3 hearts, `("minecraft:creaking", 60)`. Each row goes next to a related row (D0-12) |
| `player_nbt.rs`, `player/src/survival.rs`, buckets, brushes | shoulders, brushes | dolphin's grace, buckets | not edited |
| terrain `server.rs` | | | the phantom spawner beside the patrol's; lightning beside `explode_tnt` and `players_struck`; the trap after `cure_zombie_villagers`; creaking hearts beside every `gateways` line; `ItemOnBlock::WakeStatue` beside `HangFrame`; `TickInput.rest` after `mob_vitals` |

### The other round 10 tracks

| Track | What M3 shares with it | Rule |
|---|---|---|
| E1 | E1 owns `horse.rs`, `world/equines.rs`, `world/riding.rs`, `world/emissions.rs`, `horse_render.rs`, `mob_actions.rs`, `ambient_particles.rs`, `special_models.rs`, `mob_sender.rs`, the survival crate's `survival.rs` and `commands.rs`, `hud` and `render_gpu/src/drawsurf/minecraft_world.rs` this round | M3's horse segments (m3d, m3e) wait for e1a. Every other M3 line in those files is a one-liner beside a related arm, re-checked after each merge of `minecraft-survival` |
| E2 | E2 owns `world/leashes.rs`, `world/witches.rs`, `trident.rs`, `leash_render.rs`, `level_events.rs`, the survival crate's `death.rs`, `player/src/lib.rs` (`World::chest_open_count`) and `tick_tridents` | `copper-golems-sorting` (m3c) waits for e2a. Channeling (`lightning-weather`) goes beside E2's renamed hit arms once `ti1-targets-and-hurt` is merged |
| W | game events (P7), vibrations, `try_spawn_mob` with `ON_TOP_OF_COLLIDER`, and W4's per-entity listener | W owns `Level::game_event`, `EntityWorld::game_event` and the listener list. M3 adds one `game_event` line per M3 site only when W's helper is on `minecraft-survival`. The bolt's `lightning_strike`: vanilla posts it from the bolt's tick at `life == 2`, even for visual-only bolts. If W's line sits at `summon_lightning` (`world/src/level/entity.rs:609`) when `lightning-strikes` runs, that commit moves it into the bolt tick. `creakings-heart` and `allays-notes` wait for w2 |
| D | trial spawners ask `is_simulated` | nothing for M3 |
| F | registry group `bosses` | nothing for M3 |

### Protocol

M3 never bumps `minecraft_net::PROTOCOL` (`decisions.md`: the release step bumps it once). A commit that changes a serialized struct adds a line under "## Protocol" in the track log, naming the commit and the field. Such structs are a `ToClient`/`ToServer` message, `EntitySnapshot`, `MobShow`, `ServerMob` and the codec. The known changes are:
- `EntitySnapshot.lightning` (`lightning-drawn`);
- `MobShow.camel` (`camels-drawn`);
- `ServerMob.vehicle` (`camels-riding`).

`FamilyShow` values ride in D0's `MobShow.family` and need no note. `TickInput` stays on the host.

### Track log, mob list and commit rules

- **`docs/round10/track-m3-log.md`** holds:
  - each commit and the javap numbers it used;
  - decisions: each section's "Decisions" list, with the choice made;
  - vendored files, with patched and added counts;
  - the "## Protocol" lines;
  - the cow-store one-liners outside D0-3's list;
  - state doc wording for `docs/MINECRAFT-SURVIVAL.md`, under "State doc changes";
  - "Left for later";
  - each group's owner steps and kit line (own code block, at most 27 stacks);
  - the combined kit.
- **Docs a track run does not edit:** `CLAUDE.md`, `docs/MINECRAFT-SURVIVAL.md`, `docs/MULTIPLAYER.md`, `docs/PROGRESS.md` and `docs/round10/decisions.md`.
- **`docs/round10/mobs.md`:** M3 edits only its own section, "M3: night, built mobs and the horse family". A row's Status (`live`, `drawn`, `done`) and Commits columns change in the same commit as the code that earns them. The rows are 39 phantom, 40 zombie horse, 41 skeleton horse, 23 camel, 24 camel husk, 45 allay, 47 copper golem, 48 creaking and 50 happy ghast. Lightning has no row.
- **Every commit:**
  - one item, with a plain-English subject and its tests;
  - no code comments of any kind, and minimal diffs to existing files;
  - new files pass `rustfmt --check` on their staged content;
  - no Mojang files;
  - cargo only through `scripts\cargo-slot.ps1`.

  Clippy runs on the touched crates once per segment.

### Combined kit

One kit for the whole track (27 stacks). Each section keeps its own kit for its steps, and the class loadout's M4A1 is used for the bullet steps. Friends read the same `.env`.

```
IW4L_MINECRAFT_KIT=minecraft:phantom_spawn_egg*8,minecraft:zombie_horse_spawn_egg*4,minecraft:skeleton_horse_spawn_egg*4,minecraft:camel_spawn_egg*4,minecraft:camel_husk_spawn_egg*2,minecraft:allay_spawn_egg*4,minecraft:copper_golem_spawn_egg*4,minecraft:creaking_spawn_egg*4,minecraft:happy_ghast_spawn_egg*2,minecraft:saddle,minecraft:white_harness,minecraft:red_mushroom*16,minecraft:cactus*16,minecraft:emerald*32,minecraft:creaking_heart*2,minecraft:pale_oak_log*16,minecraft:copper_block*4,minecraft:carved_pumpkin*4,minecraft:chest*4,minecraft:lightning_rod*2,minecraft:dried_ghast*2,minecraft:water_bucket,minecraft:snowball*16,minecraft:shears,minecraft:iron_axe,minecraft:diamond_sword+looting=3,minecraft:cooked_beef*32
```

Every id comes from a section kit that was checked against the 26.3 item models (`assets/minecraft/items`).

---

## M3-1. Natural lightning, P8 (groups `lightning-strikes`, `lightning-turns`, `lightning-drawn`, `lightning-glow`, `lightning-weather`)

There is no mob in this section. It is the shared prerequisite P8 (D0-6), split into five groups (each one agent's work) in place of D0-8's single key `lightning`. The skeleton trap that needs it is M3-6.

Run order: `lightning-strikes`, `lightning-turns`, `lightning-drawn` and `lightning-glow` open segment m3a, and `lightning-weather` opens m3b (M3-groups). The strikes and the bolt's draw were one group each with three commits; at about 25-30 sites in 11-13 files each, they were split so that no agent carries the hits, the conversions, a new GPU pass and the creeper layer at once.

Lightning is not a registered family: it has no hit-key byte, no `TerrainFamily`, no `FamilyShow` and no keyframes. It reaches shared files only through the one-line hooks below (D0-3, "Behaviour that still needs a shared file").

### What exists today (`C:\Dev\mashup-crafting`, `minecraft-survival`)

- **The bolt** (`world/src/level/entity.rs`):
  - `:80-86` `LightningData { life, flashes, visual_only }`.
  - `:609` `summon_lightning` has no caller and throws its seed away (`:611`, `:639`).
  - `:619-650` `tick_lightning` has the flash timing, the rod (`:626` → `redstone.rs:1049`) and the copper cleaning (`:629`, `:654`). It has **no fire and no hits**: `:643-648` only push `"lightning hitting entities"` to `unsupported`.
  - Nothing ever sets `visual_only`, and `redstone.rs:1049-1054` lacks `levelEvent(3002)`.
- **The thunder roll.** `natural_spawner/tick.rs:259-260` already draws `nextInt(100000)` where vanilla does, then only calls `note_unsupported`.
- **Helpers to reuse:**
  - `random_tick.rs:27` `block_random_pos` (private).
  - `fire.rs:37` `fire_survives` and `:52` `can_spread_fire_around` (both private); `:71` `is_raining_at`.
  - `explosion.rs:193` `base_fire_state`.
  - `natural_spawner/spawning.rs:936` `difficulty_at` (D0 makes it `pub(super)`).
  - `level/spawning.rs:241` `raid_member`, the model for a finalized tag; `:314` `finalize_conversion`.
- **A bug.** `level/spawning.rs:162` sets `thundering` as `thunder_level > 0.9`. Vanilla's `isThundering` (javap) is `canHaveWeather && thunder × rain > 0.9`. The wrong value also feeds `dark_enough` (`spawning.rs:782`).
- **Mobs:**
  - `mooshroom.rs:93` `lightning_hit(uuid)` is ported but never called.
  - Creeper `powered` (`creeper.rs:10`) doubles the blast (`:66`), is saved (`server_mobs.rs:893`, `:1652`), and charged-creeper heads already roll (`loot.rs:243`). The energy layer is not drawn (`creeper_render.rs:7`).
  - Hazards reach every old family through `Exposed` (`world/hazards.rs:15`, `:208-245`; `pushing.rs:193`). `Hazard` is defined at `inside_blocks.rs:30`. Animals panic on any hazard that `panics()` (`hazards.rs:166`).
  - Conversions follow `server.rs:1163` `infect_villagers`. `server_mobs.rs:2185` `converted_zombie_tag` is `ConversionType.SINGLE` keeping equipment.
  - The rod POI (`entities/src/poi.rs:42`) is in terrain's `mobs.pois` (`server.rs:426`), which the world crate cannot see.
- **Players:** `blast_players` is set before the level tick (`server.rs:4221`). `Output.players_struck` (`:3642`) is applied at survival `entities.rs:318`, and hits land in `Survival::struck` (`survival.rs:851`). `death.rs:151` and `:193` name a `Hazard`'s type and message.
- **Commands.** `/summon minecraft:lightning_bolt` is refused: `minecraft_commands/src/commands.rs:871` takes only survival `commands.rs:147`'s kinds. Channeling has waited for lightning since round 7 (`docs/round7/b1-throwables.md:142`); tridents hit at `server.rs:1960-1966`.
- **Client:**
  - The snapshot (`server.rs:329-348`, built at `:4363`) is filtered for friends by `mob_sender.rs:160` `near`. The view's crystal rows (`entity_view.rs:45`, `:341`, `:1011`) are the pattern.
  - `present_sky` (`render_anim/src/minecraft_world.rs:1640`) serves the host (`:1565`) and the friend (`minecraft_remote.rs:1677`).
  - Volumes above 1 widen the reach (`audio/src/minecraft.rs:71`).
  - There is no additive pass (`render_gpu/src/drawsurf/minecraft_world.rs:1641-1717`, `entity_meshes: [_; 6]` at `:93`); the dragon rays use the translucent pass with the white texel (`dragon_render.rs:304`).

### Vanilla 26.3 (javap this round)

**The entity.** `EntityTypes.LIGHTNING_BOLT` is MISC, `noLootTable`, `noSave`, `sized(0,0)`, `clientTrackingRange(16)`, `noUpdateInterval`.
- `LightningBolt` saves nothing, its `getSoundSource` is WEATHER, and `hurtServer` returns false.
- It is born with `life` 2, `seed = nextLong()` and `flashes = nextInt(3)+1`.
- It renders within `64 × viewScale`.

**Server tick.**
1. At `life == 2`: `spawnFire(4)` on Normal and Hard; then, even when visual-only, `powerLightningRod`, `clearCopperOnLightningStrike` and `gameEvent(LIGHTNING_STRIKE)`.
2. `--life`.
3. If `life < 0`: with no flashes left it discards itself. Otherwise, when `life < -nextInt(10)`: `--flashes`, `life = 1`, a new seed, and `spawnFire(0)`. This call has no difficulty check (javap this review), so a re-flash lights its own block on Easy and Peaceful too.
4. While `life >= 0` and not visual-only, every `isAlive` entity in `AABB(x±3, y-3..y+9, z±3)` gets `thunderHit` each tick. That is two hits per flash, over 2-4 flashes.

**`spawnFire(n)`.**
- It needs the bolt not to be visual-only and `canSpreadFireAround(blockPosition)` (`fire_spread_radius_around_player`).
- At the bolt's block, if it `isAir` and the fire `canSurvive`, `BaseFireBlock.getState` is set with `setBlockAndUpdate`.
- Then `n` more tries at `offset(nextInt(3)-1, nextInt(3)-1, nextInt(3)-1)`.
- Fire's `onPlace` runs, so a bolt inside an empty obsidian frame lights the portal.

**Strike point and rods.** The strike point is `containing(x, y-1e-6, z)`. `LightningRodBlock.onLightningStrike` sets `powered`, updates neighbours, ticks in 8 and posts `levelEvent(3002, pos, axis)`. The copper walk posts `(3002, pos, -1)` for each cleaned block.

**`Entity.thunderHit`.**
- `setRemainingFireTicks(r+1)`; if that is 0, `igniteForSeconds(8)`. Then `hurtServer(lightningBolt, 5)`.
- `getFireImmuneTicks()` is 0, so a mob at rest gets 1 fire tick. Players rest at -20 (`survival.rs:668`), so the hit itself never lights them; the bolt's fire blocks do.
- **Port this literally.**

**Overrides** (a jar scan for `thunderHit` found exactly these):
- **Pig**, unless Peaceful: `convertTo(ZOMBIFIED_PIGLIN, single(this, keepEquipment false, preserveCanPickUpLoot true))`, a golden sword (`populateDefaultEquipmentSlots`) and persistence.
- **Villager**, unless Peaceful: logs "Villager {} was struck by lightning {}.", `convertTo(WITCH, single(this, false, false))`, `finalizeSpawn(CONVERSION)`, persistence and `releaseAllPois`.
- **Creeper**: the base hit, then `DATA_IS_POWERED`.
- **MushroomCow**: for a new bolt UUID (`lastLightningBoltUUID`), it flips red↔brown and plays `entity.mooshroom.convert` (2.0, 1.0). **No damage and no fire.**
- **Turtle** (M1): `Float.MAX_VALUE` damage.
- **CopperGolem** (M3 `copper-golems`): the base hit, then one weathering stage back.
- **ArmorStand**, **BlockAttachedEntity**: nothing.
- **Items and orbs** take the base hit and, with 5 health, die. Netherite resists only `#is_fire`.
- On Peaceful, or if a conversion fails, the pig and the villager take the base hit.

**`ConversionType.SINGLE`** copies position, motion, passengers, vehicle and leash; equipment and drop chances only with `keepEquipment`; absorption, effects, age, `canPickUpLoot` when preserved, handedness, NoAI, persistence, name, the on-fire flag, Invulnerable, NoGravity, Silent, tags and team.

**Damage type `lightning_bolt`:** exhaustion 0.1; message `lightningBolt`; scaling `when_caused_by_living_non_player` (so never scaled). It is in `#bypasses_shield`, `#no_knockback` and `#panic_environmental_causes`, and not in `#is_fire` or `#bypasses_armor`.

**`ServerLevel.tickThunder(chunk)`**, run from `tickSpawningChunk` in entity-ticking range:
1. Needs `isRaining && isThundering && nextInt(100000) == 0`.
2. `pos = findLightningTargetAround(getBlockRandomPos(minX, 0, minZ, 15))`. Nothing happens unless `isRainingAt(pos)`.
3. `trap = SPAWN_MOBS && nextDouble() < difficulty.effective × 0.01 && the block below is not in #lightning_rods`. The double is drawn only under `SPAWN_MOBS`.
4. A trap adds a `SKELETON_HORSE` (EVENT, no finalize, `setTrap(true)`, `setAge(0)`) at the block's integer corner.
5. A `LIGHTNING_BOLT` goes at `atBottomCenterOf(pos)` with `setVisualOnly(trap)`.

**`findLightningTargetAround(p)`.**
1. `c = getHeightmapPos(MOTION_BLOCKING, p)`.
2. First, the closest `LIGHTNING_ROD` POI within 128 whose `y == WORLD_SURFACE - 1` (Occupancy ANY); the target is the block above it.
3. Otherwise, the alive living entities in `encapsulatingFullBlocks(c, c.atY(maxY+1)).inflate(3)` that `canSeeSky`; spectators are included. One is picked with `nextInt(size)`, at its block position.
4. Otherwise `c`, or `c.above(2)` at `minY-1`.

**Client.** On the bolt's first client tick it plays `entity.lightning_bolt.thunder` (WEATHER, 10000, 0.8+r×0.2) and `.impact` (2.0, 0.5+r×0.2). While `life >= 0` it calls `setSkyFlashTime(2)`, which `ClientLevel.tick` counts down. While flashing, `SKY_COLOR` is `srgbLerp(0.22, value, (0.8, 0.8, 1.0))` and `SKY_LIGHT_FACTOR` is 1.

**`LightningBoltRenderer`.**
- `createThreadLocalInstance(seed)` (an LCG, our `LegacyRandom`), fresh for each of 4 layers of width `0.1 + 0.2 × layer`.
- Trunk segments 7..0 step `nextInt(11)-5` and taper by `seg × 0.1 + 1`; branches over 6..4 and 5..3 step `nextInt(31)-15`.
- 4 quads per 16-block segment: 224 quads, 128 blocks tall.
- Colour (0.45, 0.45, 0.5, 0.3), blend `SRC_ALPHA, ONE`, no cull.
- Port `submit`, `lambda$submit$0` and `quad` exactly.

**`CreeperPowerLayer`.** A `CreeperModel` from `CREEPER_ARMOR` (javap the deformation during the work) with `creeper_armor.png`, drawn as `energySwirl(tex, age×0.01 % 1, age×0.01 % 1)`, colour `0xFF808080`, blend ADDITIVE, no cull.

**Channeling** (`enchantment/channeling.json`):
- `post_attack`: thundering, the victim `can_see_sky`, and the direct attacker is a trident.
- `hit_block`: a trident on a `#lightning_rods` block that `can_see_sky`.
- Both run `summon_entity minecraft:lightning_bolt` and `item.trident.thunder` (5.0, 1.0). javap `SummonEntityEffect.apply` during the work.

**The skeleton trap:** `SkeletonHorse`, `SkeletonTrapGoal` and the four horsemen are in M3-6.

### Plan

**New files:**
- `world/src/level/lightning.rs`: `lightning_fire(e, n)`, the hits on level items and orbs, `LightningHit { bolt, uuid, at }`, `add_lightning(pos, visual_only)`, and `lightning_rod_near(c)`.
- `world/src/natural_spawner/thunder.rs`: `strike_thunder`.
- `entities/src/world/lightning.rs`: `Bolt { uuid, at }` and `EntityWorld::thunder_hit(&Bolt, peaceful, world, game_time) -> Vec<ThunderOutcome>` (`Convert { id, into }`).
- `crates/minecraft_terrain/src/lightning.rs`: `BOLT`, `BoltView { id, position, seed, life }`, `strike`, `views` and `channel`. These are free functions that take the `ServerSim` fields they need as arguments (`&mut Level`, `&mut EntityWorld`, the players), passed by the one-line calls in `server.rs`: `ServerSim`'s fields and helpers such as `add_saved_entity` are private to `server.rs`, so a function in another module cannot take `&mut ServerSim` and reach them. What a strike creates (converted mobs' tags) comes back as a return value, and the `server.rs` line hands each tag to `add_saved_entity`.
- `crates/minecraft_terrain/src/lightning_render.rs`: the bolt mesh.
- `crates/minecraft_survival/src/lightning.rs`: the player's `thunderHit`; the view's bolts (sound once per id, sky flash).

**Registry.**
- The base hit on registered mobs needs no code: `Hazard::LightningBolt` goes through D0's generic `impl Exposed for FamilyMob<S>` (`exposed_mut`'s family arm) into `S::hurt`.
- One hook goes into D0's root `families.rs`, in its own commit, after `fire_immune` (M30): `fn thunder_hit(mob, bolt: &Bolt) -> bool { false }` (true means the family handled the whole hit, so the base hit is skipped), with its `family_group!` dispatch and `Families::thunder_hit(key, id, bolt) -> Option<bool>`.
- M1's turtles and M3's copper golems override it.

**One-line hooks into shared files**, each next to a related line:

| Commit | Where | What |
|---|---|---|
| strikes 1 | `level/mod.rs` | `pub mod lightning;` after `pub mod fire;` (`:19`); `lightning_hits` after `level_events` (`:207`), with its init |
| strikes 1 | `level/entity.rs` | `LightningData` gains `seed` and `uuid`; `summon_lightning` calls `add_lightning(pos, false)`; `self.lightning_fire(e, 4)` before the rod check (`:624`); `self.lightning_fire(e, 0)` after the re-flash seed (`:639`); `:643-648` become `self.lightning_strike_entities(e)` |
| strikes 1 | `level/fire.rs`, `redstone.rs` | `:37` and `:52` become `pub(super)`. After `redstone.rs:1053`: `self.level_events.push((3002, pos, axis))`; `-1` for each cleaned block (`entity.rs:680-681`) |
| strikes 1 | `inside_blocks.rs` | `LightningBolt` after `Freeze` in the enum (`:43`), in `damage_type` (`:74`) and in `panics` (`:54`) |
| strikes 1 | `entities/src/world.rs` | `mod lightning;` and `pub use lightning::{Bolt, ThunderOutcome};` after `mod hazards;` (`:45`), the file's `mod x; pub use x::{..}` pattern: terrain builds `Bolt` and reads `ThunderOutcome`, and the root hook names `Bolt` in a public trait, so a private type would trip `private_interfaces` |
| strikes 1 | terrain `server.rs` | `crate::lightning::strike(..)` after `self.explode_tnt();` (`:3156`), players from `blast_players`; `Output.players_lightning` after `players_struck` (`:3642`), filled after `:4281`; the first line of `ServerSim::summon` (`:1549`) sends `BOLT` to `level.add_lightning` |
| strikes 1 | survival | `commands.rs:147` chains `lightning::BOLT`. `entities.rs`: a loop after the `players_struck` loop (`:318-324`) pushing `(5.0, Some(at), None, DamageCause::Hazard(Hazard::LightningBolt), ..)`; `toward: None` means no knockback and no shield block. `death.rs`: arms after `Freeze` (`:163`, `:206`). `survival.rs:856` `struck`, before `self.hurt`: `crate::lightning::burn(self, cause);`. `lib.rs` (both crates): `pub mod lightning;` |
| weather 1 | `natural_spawner.rs` | `mod thunder;` after `pub mod tick;` (`:14`); four defaulted `SpawnLevel` methods after `add_inhabited_time` (`:61`): `block_random_pos`, `raining_at`, `lightning_rod_near`, `add_lightning` |
| weather 1 | `tick.rs:260`, `level/spawning.rs`, `random_tick.rs:27` | `note_unsupported` → `Self::strike_thunder(level, context, input, chunk);`. In `level/spawning.rs`: four one-line impls after `:655`, and `:162` multiplies by rain (calling M1's `Level::is_thundering` if it is merged, M30). `random_tick.rs:27` becomes `pub(super)` |
| weather 2 | terrain `server.rs`, `trident.rs` | `crate::lightning::channel(..)` after the hits at `:1962` and `:1965`; one block-hit `TridentEvent` variant for rods |
| drawn 1 | terrain | `EntitySnapshot.lightning` after `crystals` (`:338`), set on a new line after `:4363`; `mob_tracking.rs:288` becomes `"end_crystal" \| "lightning_bolt" => 16` |
| drawn 1 | survival | `mob_sender.rs`: `LIGHTNING` after `CRYSTALS` (`:19`) and a row after `crystals:` (`:188`). `entity_view.rs`: a field after `crystals` (`:45`, `:94`), calls after `:341`, after `self.eyes.tick` (`:160`) and after the crystal draw (`:1011`). `MobMeshes.additive` (`entities.rs:117`) |
| drawn 1 | render | filled at `render_anim/minecraft_world.rs:1524` and `minecraft_remote.rs:2005`. `render_gpu`: `entity_meshes: [_; 7]` (`:93`, test `:1734`), a pipeline appended after `:1716`, `additive_fragment` in the WGSL |
| drawn 1 | sky | `environment.rs`: `sky_flash` and `flash_sky()`; `tick` (`:115`) counts down; `sky_state` (`:171`) lerps the sky and sets `sky_light_factor` 1. `SkyInputs.sky_flash` (`minecraft_world.rs:1596`) is set at `:1565` and `minecraft_remote.rs:1677`; one line in `present_sky` before `:1659` |

**Multiplayer.**
- The host runs every bolt, hit and conversion.
- Friends get bolts as a state view in `EntitySnapshot`, filtered at 16 chunks. A bolt lives at least 4 ticks, so a lost frame does not lose it.
- Each view draws and flashes while `life >= 0`, and plays thunder and impact once per new id, keeping a seen set like `fang_crits` (`entity_view.rs:237-240`).
- `powered` and the mooshroom's `Type` are not volatile, so a friend respawns that mob once from its look tag. Converted mobs are new mobs.
- No `FamilyShow` or `MobShow` change.
- **Protocol:** `EntitySnapshot.lightning`, recorded under "## Protocol" in the log. `Hazard` is not on the wire.

### Commit groups

1. **`lightning-strikes`** (segment m3a; needs D0 d0a):
   1. "Minecraft lightning bolts set fires and hurt what they strike": the fire; items and orbs; mobs through `Exposed` (fire bump and 5; armour counts, fire immunity does not, animals panic); players through the shield with no knockback, the fire rule and the death line; `/summon`; rod event 3002.
   2. "Registered Minecraft mobs can answer lightning their own way" (the hook).
2. **`lightning-turns`** (segment m3a, right after `lightning-strikes`):
   1. "Minecraft lightning turns pigs, villagers, creepers and mooshrooms like vanilla". The tag is `converted_zombie_tag` minus `equipment` and `drop_chances` (and minus `CanPickUpLoot` for the witch), plus effects, absorption and age. `EntityWorld::thunder_hit` returns the `ThunderOutcome::Convert`s, and `crate::lightning::strike` returns the new tags to its `server.rs` line, which hands each to `add_saved_entity`.
      - Pig: a golden sword and persistence, no finalize.
      - Villager: `finalize_conversion`, persistence, POIs released as in `infect_villagers`, a `diag::info!` line.
      - Creeper: `powered`. Mooshroom: `lightning_hit` and the sound.
3. **`lightning-drawn`** (segment m3a, after `lightning-turns`):
   1. "Minecraft lightning bolts flash and thunder for every player": the renderer; an additive pass (`ONE, ONE`, colour × 0.3, depth test without write, no cull); the flash, the sounds and the friend filter.
4. **`lightning-glow`** (segment m3a, right after `lightning-drawn`, whose additive pass it uses):
   1. "Minecraft charged creepers glow with their energy swirl": the armour model on the additive pass, with a repeating scroll along the armour glint's path (`mesh.rs:91`, `render_gpu/.../minecraft_world.rs:89-90`). Drop `creeper_render.rs:7`.
   2. Optional: "Minecraft lightning rods spark when struck" (event 3002 as `electric_spark`, its constant after `LECTERN_PAGE_TURN` in `level_events.rs` and its arm after that arm, M30; `Kind::ElectricSpark` next to `Kind::Crit` in `ambient_particles.rs`, M30; the storm `animateTick` of a rod at the surface, as a `lightning_rod` arm in `display_ticks.rs` beside `end_gateway` (`:77`)). Skip it to "Left for later" if the group runs long.
5. **`lightning-weather`** (segment m3b):
   1. "Minecraft thunderstorms strike lightning, drawn to lightning rods".
      - Inline at the roll, so `randValue` and the level random keep vanilla's order.
      - The rod search scans `WORLD_SURFACE` tops within 128 chunk by chunk. Targets come from `input.census` and `input.players`.
      - A trap gives a visual bolt and a `{SkeletonTrap:1b, Age:0}` skeleton horse tag via `add_entity`, dormant until `undead-horses-live` (M3-5); it springs from `skeleton-trap` (M3-6) on.
      - The `isThundering` fix.
   2. "Minecraft tridents with Channeling call down lightning in thunderstorms".

### Tests

- **strikes 1:**
  - `a_bolt_on_normal_lights_its_block_and_up_to_four_more` (no first burst on Easy, none at all when visual-only or beyond the radius), `a_reflash_lights_only_its_own_block` (on Easy too), `a_bolt_burns_items_and_orbs_inside_its_box_only`, `a_bolt_reports_a_hit_each_lit_tick`, `a_struck_rod_powers_and_sparks`, `a_bolt_in_an_empty_obsidian_frame_lights_the_portal`
  - `a_struck_zombie_takes_five_and_one_fire_tick` (0 → 1, -1 → 160), `armour_softens_lightning_and_blazes_are_hurt_too`, `a_struck_cow_panics`
  - `a_summoned_bolt_hurts_mobs_and_players_in_its_box`, `a_lightning_death_reads_struck_by_lightning`, `lightning_ignores_the_shield_and_knockback_and_lights_no_resting_player`, `summon_accepts_lightning_bolt`
- **strikes 2:** `a_registered_mob_answers_lightning_through_its_hook` (probe).
- **turns 1:** `a_struck_pig_becomes_a_persistent_zombified_piglin_with_a_golden_sword` (a baby stays a baby; Peaceful only hurts), `a_struck_villager_becomes_a_persistent_witch_and_frees_its_job_site`, `a_struck_creeper_is_charged_hurt_and_saved_charged`, `a_mooshroom_flips_once_per_bolt_unharmed`, `a_friend_sees_the_brown_mooshroom_and_the_charged_creeper`.
- **weather 1:** `a_strike_aims_at_the_nearest_surface_rod_within_128`, `a_buried_rod_is_ignored`, `without_a_rod_it_picks_a_living_thing_under_open_sky`, `no_bolt_in_a_desert_a_snowy_biome_or_under_a_roof`, `a_trap_roll_leaves_a_visual_bolt_and_a_trap_horse_tag`, `no_trap_without_spawn_mobs_or_above_a_rod`, `thundering_needs_rain_too`, `a_long_storm_strikes_inside_the_spawning_chunks` (seeded).
- **weather 2:** `a_channeling_trident_calls_lightning_on_a_mob_under_open_sky_in_thunder` (not in rain alone, not under a roof), `..._on_a_lightning_rod`.
- **drawn 1:** `a_bolt_is_224_quads_128_blocks_tall_from_its_seed`, `the_same_seed_draws_the_same_bolt`, `a_new_bolt_thunders_once_and_flashes_while_lit`, `a_sky_flash_brightens_the_sky_and_lights_the_world_for_two_ticks`, `a_friend_gets_bolts_within_sixteen_chunks`, the WGSL and slot-count tests.
- **glow 1:** `a_charged_creeper_wears_the_scrolling_energy_layer`.
- **glow 2 (optional):** `level_events::tests::a_struck_rod_sparks_along_its_axis_and_cleaned_copper_sparks_on_its_faces` (data = the axis, or -1) and `ambient_particles::tests::an_electric_spark_fades_quickly` (javap `ElectricSparkParticle` during the work).

### Owner's steps

**Setup:** a release build with segment m3a (`lightning-strikes`, `lightning-turns`, `lightning-drawn`, `lightning-glow`); steps 10 and 11 need `lightning-weather` (m3b). Survival, Normal, flat plains, by day; `/weather clear`; the kit below; the host is op. Pick a standing place and type every `/summon` from it, so `~ ~ ~8` ("the spot", 8 blocks towards +Z) is always the same block.

1. `/summon minecraft:cow ~ ~ ~8`, then `/summon minecraft:lightning_bolt ~ ~ ~8`: a jagged blue-white bolt reaches the sky, everything flashes, thunder rolls and cracks, and the cow flashes red and flees while the grass catches fire. **m3l1_bolt**
2. `/summon minecraft:pig ~ ~ ~8` and the bolt: a zombified piglin with a golden sword. `/summon minecraft:pig ~ ~ ~8 {Age:-24000}` and the bolt: a baby zombified piglin. **m3l2_pig**
3. `/summon minecraft:villager ~ ~ ~8` and the bolt: a witch.
4. `/summon minecraft:mooshroom ~ ~ ~8` and the bolt: it turns brown with a short sound, and the bolt itself does not hurt it (the fire the bolt leaves under it may still singe it). A second bolt turns it red. **m3l4_mooshroom**
5. Dig a pit 1 wide and 3 deep at the spot and lay a stone slab on its floor (fire cannot sit on a bottom slab, so the creeper does not burn). Run `/summon minecraft:creeper ~ ~-2.5 ~8` and `/summon minecraft:lightning_bolt ~ ~-2.5 ~8`: a scrolling blue energy layer wraps the creeper. **m3l5_charged** Save and quit, then load: it is still wrapped. Dig a second pit the same way 5 blocks to the side and use a creeper egg in it. Walk up to each pit until its creeper hisses, then back off at once to 6 blocks: the struck creeper's crater is about twice as wide as the plain one's.
6. Walk to the spot, drop a stack of planks there and walk back to your place. Throw 3 bottles o' enchanting at the spot, then the bolt: the items and orbs vanish. Planks dropped 6 blocks further away stay.
7. In iron armour, shield raised, `/summon minecraft:lightning_bolt ~ ~ ~`: about 2 hearts are lost through the shield with no push, and the fire under you lights you (step into water). Without armour at 2 hearts, a bolt kills you: "… was struck by lightning". **m3l7_death**
8. At the spot, build a pillar of 4 stone bricks, then a weathered copper block, then a lightning rod on top. Put a redstone lamp against one side of the copper block and weathered copper on the other three sides. From your place run `/summon minecraft:lightning_bolt ~ ~6 ~8`: the bolt hits the rod, the lamp blinks, and the copper turns orange again (with `lightning-glow`'s optional commit the rod also sparks). Leave the pillar for step 10.
9. Away from the pillar, stand a nether portal frame of 14 obsidian on the ground. At full health, step inside it onto its bottom row and run `/summon minecraft:lightning_bolt ~ ~ ~`, then step out at once: the portal lights (you lose a heart or two). **m3l9_portal**
10. `/weather thunder` and wait 2-3 minutes by the pillar (the highest point around): bolts fall across the plains, most within about 128 blocks hit the rod and blink the lamp, and cows in the open are sometimes struck. **m3l10_storm** No bolts fall in a desert or on snowy plains.
11. In the storm, throw the Channeling trident at a cow in the open: a bolt and the trident thunder sound; nothing under a roof. Thrown at the rod, a bolt hits the rod.
12. Friend, two windows (`scripts\mp-local.ps1`): the host repeats steps 1, 2 and 4, and the friend sees the bolt, the flash, the piglin and the brown mooshroom, and hears the thunder. **m3l12-host**, **m3l12-friend** A friend standing at the spot loses about 2 hearts and burns. In a storm both windows see the same bolts.

```
IW4L_MINECRAFT_KIT=minecraft:pig_spawn_egg*4,minecraft:villager_spawn_egg*2,minecraft:creeper_spawn_egg*2,minecraft:mooshroom_spawn_egg*2,minecraft:cow_spawn_egg*4,minecraft:stone_slab*8,minecraft:lightning_rod*4,minecraft:weathered_copper*16,minecraft:redstone_lamp*2,minecraft:trident+channeling=1,minecraft:stone_bricks*64,minecraft:scaffolding*64,minecraft:obsidian*14,minecraft:oak_planks*64,minecraft:experience_bottle*16,minecraft:water_bucket,minecraft:shield,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:cooked_beef*32,minecraft:iron_shovel,minecraft:diamond_sword,minecraft:torch*64
```

25 stacks; every id was checked against the 26.3 item models.

### Depends on

- **D0 d0a:**
  - the `exposed_mut` family arm;
  - the root `families.rs` (for the hook);
  - `difficulty_at` as `pub(super)` (`registry-spawning`);
  - `simulated_kinds()` at `commands.rs:147`.

  d0b is not needed.
- **W `vibrations-more`** (segment w1). Vanilla posts `lightning_strike` in the bolt's tick at `life == 2`, after the copper, even for visual-only bolts. That is not at `summon_lightning`, where `track-w.md` W0 places it. If W's line is on `minecraft-survival` when `lightning-strikes` runs, that commit moves it into the bolt tick. If not, M3 posts nothing and logs the spot for W (M30).
- **E2 `ti1-targets-and-hurt`** (segment e2a). Channeling's lines sit by `tick_tridents`' hit arms, which it renames (`hurt_by` → `hurt_for_player`). If e2a is merged when `lightning-weather` runs, place them beside the renamed arms.
- **E1:** nothing to wait for. `lightning-strikes` adds one-liners to E1's survival `commands.rs` and `survival.rs`, `lightning-drawn` to E1's `mob_sender.rs`, and `lightning-glow`'s optional commit to E1's `ambient_particles.rs`; each goes beside a related arm and is re-checked against whatever of E1 is on `minecraft-survival` (M30).
- **M3:** the skeleton trap (M3-6) uses the visual-only bolt. `copper-golems-care` (M3-4) and M1's turtles override `thunder_hit`. `lightning-turns` needs `lightning-strikes`' `thunder_hit` path; `lightning-glow` needs `lightning-drawn`'s additive pass.

### Open decisions (the recommended choice first)

1. **Where the strike runs.** Inline in the world crate, through four defaulted `SpawnLevel` methods, so the order of `randValue` and the level random stays vanilla's. The alternative is a terrain pass after the level tick.
2. **How the rod is found.** A world-crate scan of the `WORLD_SURFACE` tops within 128. It finds the same nearest rod, only breaking ties differently. The alternative is terrain's `mobs.pois`, worth it only if the scan measures over about 2 ms per strike.
3. **The draw pass.** An additive pass, as vanilla's `LIGHTNING` and `ENERGY_SWIRL` blends are; the charged creeper needs it too. The alternative is the dimmer translucent pass with the white texel, as the dragon rays use.
4. **Where the trap goes.** Its own group, `skeleton-trap` (M3-6), after the undead horse groups, rather than folded into them.
5. **The fire rule.** Port `thunderHit` literally: 1 fire tick for a mob at rest, none for players. The bolt's fire blocks do the burning, as in 26.3.
6. **The `isThundering` fix** (thunder × rain at `level/spawning.rs:162`): yes. It is vanilla, and it also corrects the monster light check during storms.

---

## M3-2. Phantoms (groups `phantoms-live`, `phantoms-drawn`, `phantoms-spawner`; registry group `night`, family `phantoms`, hit key 80)

Mob list row 39: today a phantom egg does nothing, `/summon minecraft:phantom` leaves a dormant tag, and three sleepless nights bring nothing.

### What exists today (`C:\Dev\mashup-crafting`, `minecraft-survival`)
- **Insomnia clock:** `Player.time_since_rest` (`crates/minecraft_survival/src/player.rs:54`) counts awake ticks and resets on death (`sleep.rs:476-480`) and in bed (`sleep.rs:272`). It is saved as the vanilla stat `minecraft:time_since_rest` in `players/stats/<uuid>.json` (`save.rs:22`, `:122`, `:135`, `:148`). Nothing reads it for phantoms.
- **Custom spawners (P6):** `CustomSpawners` (`crates/minecraft_terrain/src/custom_spawners.rs:29-61`, Overworld only). `PatrolSpawner` (`:157-171`) is the pattern: a countdown on the level random. Its tick is `ServerSim::tick_patrol_spawner` (`server.rs:1258-1288`), called at `server.rs:4223` beside the cat and trader spawners (`:4224-4225`). The rules are copied at `server.rs:3140-3141`. The `spawn_phantoms` rule exists (`third_party/minecraftoss/world/src/game_rules.rs:162`) but nothing reads it.
- **Lists that already name the phantom:** `MobCategory::of_type` Monster (`world/src/natural_spawner/spawning.rs:100`), `wither::is_undead` (`entities/src/wither.rs:90`), the sleep test's list of mobs that never stop a player sleeping (`crates/minecraft_survival/src/sleep.rs:915-926`; vanilla's phantom is not a `Monster`, so `MONSTERS` at `:29` rightly leaves it out and M3 adds nothing there), `NOT_IN_PEACEFUL` (`generator/src/feature/mod.rs:161`), and membranes repairing elytra (`crates/minecraft_terrain/src/stations.rs:431`).
- **Cats:** `FelineEntity` (`entities/src/world/felines.rs:32`), `felines()` (`:345`), `feline_mut` (`:349`), and each sound set's `hiss` (`entities/src/feline.rs:75`, `:89`).
- **Closest family, the vex** (`entities/src/world/vexes.rs`), flies with its own steering and goals. `VexEntity::tick_ai` (`:54-133`) fills `ai.state`, calls `ai.tick_goals(world, full)` (`:87-88`) and takes `ai.state.attack` (`:89`). Its strike is a `PlayerHitKind::Melee` hit (`:242-273`). `monster_idle_without_ai` is at `world.rs:1007`.
- **Flying physics:** `Body::travel_flying` (`entities/src/movement.rs:441-474`) is vanilla `travelFlying`: no gravity, drag 0.91, 0.8 in water and 0.5 in lava, and no fall distance. The ghast uses it (`world/ghasts.rs:117`).
- **Daylight burning:** `burn_undead` (`entities/src/world.rs:854-887`) is private to `world`, so `world/families/night/` can call it. Its flag is `EntityWorld.monsters_burn` (`:2559`, set at `:3421`); the zombie's call is at `:6312-6313`. `EntityWorld.sea_level` is at `:2569`.
- **Spawning helpers:**
  - `CreatureSpawns::summon` (`natural_spawner/spawning.rs:886`) and `spawn_patrol` (`:1038`);
  - `difficulty_at` (`:936`) and `mob_spawn_rules` (`:820`), which D0 widens to `pub(super)`;
  - `is_valid_empty_spawn_block` (`natural_spawner.rs:730`, private; public sibling `valid_spawn_block` at `:713`);
  - `Level::spawn_patrol` (`world/src/level/spawning.rs:227`).
- **Client:**
  - `ServerMob.x_rot` is synced and interpolated like vanilla (`client_mobs.rs:36`, `:234-257`, `:485-501`) and becomes `MobPose.head_pitch` (`:779`);
  - `mob_tracking::place` copies `MobShow.fire` (`mob_tracking.rs:589`);
  - eyes at full light: `spider_render.rs:61-90`;
  - `posed_cube` (`posed_cube.rs:40-47`), as `vex_render.rs:5` uses it;
  - `happy_villager` (`ambient_particles.rs:213-220`) is the `SuspendedTownParticle` port.
- **The test** `live_shulkers_stay_far_from_players_while_dormant_monsters_go` (`crates/minecraft_terrain/src/server.rs:8169`) uses the silverfish. D0 switches it to the phantom (D0-9.4), so M3 must change it again (decision 2).

### Vanilla 26.3 (javap this round)
**Type and attributes**
- `EntityTypes.PHANTOM`: `MONSTER`, `sized(0.9, 0.5)`, `eyeHeight(0.175)`, passenger attachment 0.3375, riding offset -0.125, `clientTrackingRange(8)`, `notInPeaceful`, not fire immune. Tags: `#undead`, `#burn_in_daylight`, `#fall_damage_immune`.
- Attributes come from `Monster.createMonsterAttributes()`: health 20, follow range 16, attack damage at its default 2.0.
- `Phantom extends Mob implements Enemy`, so it is not a `PathfinderMob` and has no navigation. `xpReward = 5`.
- Controls: `PhantomMoveControl`; `PhantomLookControl`, whose `tick` is empty; `PhantomBodyRotationControl.clientTick`: `yHeadRot = yBodyRot; yBodyRot = yRot`.
- `canBeLeashed` is `!(this instanceof Enemy)`, so it is false.

**Size** (`ID_SIZE` 0..64)
- The box and eye height scale by `1 + 0.15 × size`.
- `updatePhantomSizeInfo` sets attack damage `6 + size`, but it runs only from `onSyncedDataUpdated`. `SynchedEntityData.set` fires that only on a changed value, so a size-0 phantom keeps 2.0 on the server (decision 1).

**Saving and spawn**
- Saved: `anchor_pos` (int array `[x, y, z]`, left out when null) and `size` (int, default 0).
- `finalizeSpawn`: `anchorPoint = blockPosition().above(5)`, `setPhantomSize(0)`, then `Mob.finalizeSpawn`.

**Movement**
- `travel` is `travelFlying(input, 0.2)` with zero input. `checkFallDamage` does nothing and `onClimbable` is false.
- `Mob.aiStep` calls `burnUndead` after `LivingEntity.aiStep` (the fork's `burn_undead`).

**`PhantomMoveControl.tick`** (every tick; `speed` starts at 0.1)
1. On `horizontalCollision`: `yRot += 180`, `speed = 0.1`.
2. `d = moveTargetPoint - pos` and `h = |(dx, dz)|`. Nothing more unless `h > 1e-5`.
3. `f = 1 - |0.7 dy| / h`; `dx *= f`, `dz *= f`; recompute `h`; `l = |(dx, dy, dz)|`.
4. `yRot = approachDegrees(wrap(yRot + 90), wrap(atan2(dz, dx) × 57.295776), 4) - 90`; `yBodyRot = yRot`.
5. If `degreesDifferenceAbs(old, yRot) < 3`: `speed = approach(speed, 1.8, 0.005 × 1.8 / speed)`; otherwise `approach(speed, 0.2, 0.025)`.
6. `xRot = -(atan2(-dy, h) × 57.2957763671875)`.
7. Wanted motion: `(speed cos(yRot + 90°) |dx/l|, speed sin(xRot°) |dy/l|, speed sin(yRot + 90°) |dz/l|)`; `delta += (wanted - delta) × 0.2`.

**Goals**
- Goal selector: 1 `PhantomAttackStrategyGoal` (no flags), 2 `PhantomSweepAttackGoal` (MOVE), 3 `PhantomCircleAroundAnchorGoal` (MOVE). Target selector: 1 `PhantomAttackPlayerTargetGoal` (no flags).
- None updates every tick, so `adjustedTickDelay(n) = ceil(n / 2)` and they tick on full ticks.
- Shared state: `moveTargetPoint`, `anchorPoint` and `attackPhase` (`CIRCLE` or `SWOOP`).

*Target goal*
- `nextScanTick` starts at `reducedTickDelay(20)` and then resets to `reducedTickDelay(60)`.
- It scans the players in `box.inflate(16, 64, 16)` that pass `forCombat().range(64)`, sorted by `getY`, highest first. The first one that passes `canAttack(DEFAULT)` (attackable, not creative or spectator, in line of sight) becomes the target.
- It continues while the target passes `DEFAULT`. Its `stop` keeps the target.

*Strategy goal*
- `canUse`: the target passes `DEFAULT`.
- `start`: `nextSweepTick = adjustedTickDelay(10)`, phase `CIRCLE`, `setAnchorAboveTarget`.
- `stop`: the anchor becomes `heightmap(MOTION_BLOCKING, anchor).above(10 + nextInt(20))`.
- `tick` (in `CIRCLE`): when `--nextSweepTick <= 0`, it switches to `SWOOP`, sets the anchor above the target, sets `nextSweepTick = adjustedTickDelay((8 + nextInt(4)) × 20)`, and plays `entity.phantom.swoop` (10.0, `0.95 + 0.1r`).
- `setAnchorAboveTarget`: nothing when the anchor is null. Otherwise `target.blockPosition().above(20 + nextInt(20))`, raised to at least `seaLevel + 1`.

*Sweep goal*
- `canUse`: a target and `SWOOP`.
- It stops on: no target, a dead target, a creative or spectator player, or `!canUse()`. Every 20 ticks (`catSearchTick`) it lists the live cats in `box.inflate(16)`; **every one of them `hiss()`es**, and while any is there it stops.
- `stop`: `setTarget(null)` and `CIRCLE`.
- `tick`: `moveTargetPoint = (tx, target.getY(0.5), tz)`.
  - When `box.inflate(0.2)` meets the target, it calls `doHurtTarget` (its attack damage, a mob attack), sets `CIRCLE`, and sends level event 1039 unless silent (`entity.phantom.bite`, HOSTILE, 0.3, `0.9 + 0.1r`).
  - Otherwise `horizontalCollision` or `hurtTime > 0` sets `CIRCLE`.

*Circle goal*
- `canUse`: `target == null || CIRCLE`.
- `start`: `distance = 5 + 10r`, `height = -4 + 9r`, `clockwise = ±1`, then `selectNext`.
- `tick`:
  - `nextInt(175) == 0` draws a new height;
  - `nextInt(125) == 0` adds 1 to `distance`; above 15 it goes back to 5 and the direction flips;
  - `nextInt(225) == 0` draws a new `angle = 2πr` and calls `selectNext`;
  - within distance² 4 of the target point it calls `selectNext`;
  - the target point below and a solid block under it gives `height = max(1, height)` and `selectNext`; above and a solid block over it gives `min(-1, height)` and `selectNext`.
- `selectNext`: a null anchor becomes its own block. `angle += clockwise × 15°`, and `moveTargetPoint = anchor + (distance cos, -4 + height, distance sin)`.

**Sounds, flap and particles**
- Source HOSTILE. `entity.phantom.ambient`, `.hurt` and `.death` (interval 80), plus `.swoop`, `.bite` and `.flap`.
- `Cat.hiss()` plays its sound set's hiss at volume 1.0 and its voice pitch.
- Flap timing: `TICKS_PER_FLAP = ceil(24.166098) = 25`, offset `id × 3`.
- Server: `isFlapping` (`(offset + tickCount) % 25 == 0`) makes `Entity.processFlappingMovement` post game event `minecraft:flap` (P7).
- Client, with `c(t) = cos((offset + t) × 7.448451° + π)`:
  - when `c(tick) > 0 >= c(tick + 1)`, it plays the local sound `entity.phantom.flap` (volume and pitch `0.95 + 0.05r`);
  - every tick it adds two `mycelium` particles at `(x ± cos(yRot) × 1.48w, y + (0.3 + 0.45c) × 2.5h, z ± sin(yRot) × 1.48w)` with zero speed.
- `mycelium` is a `SuspendedTownParticle`: grey `0.2 + 0.1r`, quad × `(0.5 + 0.6r)`, life `20 / (0.8r + 0.2)`, × 0.99 drag, sprite `generic_0`.

**Model** (`PhantomModel.createBodyLayer`, 64 × 64)
- `body` (0,8) box(-3,-2,-8; 5,3,9), xRot -0.1. Its children:
  - `tail_base` (3,20) box(-2,0,0; 3,2,6) at (0,-2,1), with `tail_tip` (4,29) box(-1,0,0; 1,1,6) at (0,0.5,6);
  - `left_wing_base` (23,12) box(0,0,0; 6,2,9) at (2,-2,-8), zRot 0.1, with `left_wing_tip` (16,24) box(0,0,0; 13,1,9) at (6,0,0), zRot 0.1;
  - `right_wing_base` and `right_wing_tip`: the mirrors, at (-3,-2,-8) and (-6,0,0), boxes from -6 and -13, zRot -0.1;
  - `head` (0,0) box(-4,-2,-5; 7,3,5) at (0,1,-7), xRot 0.2.
- `setupAnim`: `f = flapTime × 7.448451°`. The left wing parts get `zRot = 16° cos f`, the right ones the negative. The tail parts get `xRot = -(5 + 5 cos 2f)°`.
- Render type `entityCutoutNoCull`.

**Renderer**
- Shadow 0.75, texture `textures/entity/phantom/phantom.png`.
- `PhantomEyesLayer`: `phantom_eyes.png`, full bright.
- `flapTime = id × 3 + ageInTicks`.
- `scale`: `s = 1 + 0.15 size`, `translate(0, 1.3125, 0.1875)`.
- `setupRotations`: the mob's own rotations, then `XP` by `xRot`.

**Loot, spawning and the spawner**
- Loot `entities/phantom`: `phantom_membrane` 0-1, plus 0-1 per Looting level, killed by a player.
- `SpawnPlacements`: `NO_RESTRICTIONS`, `MOTION_BLOCKING_NO_LEAVES`, `Mob.checkMobSpawnRules`. No biome or structure spawns it.
- `PhantomSpawner.tick(level, spawnEnemies)` runs only in the Overworld, while mob spawning runs:
  1. Return without `spawnEnemies` or without the rule `spawn_phantoms`.
  2. `--nextTick > 0` returns; otherwise `nextTick += (60 + nextInt(60)) × 20`.
  3. `skyDarken < 5` (with sky light) returns.
  4. Per non-spectator player, at block `p`:
     - it needs `p.y >= seaLevel` and `canSeeSky(p)`;
     - `difficultyAt(p).isHarderThan(3r)`;
     - `nextInt(clamp(time_since_rest, 1, MAX)) >= 72000`;
     - the spot `p.above(20 + nextInt(15)).east(nextInt(21) - 10).south(nextInt(21) - 10)` passes `isValidEmptySpawnBlock`;
     - then `1 + nextInt(difficultyId + 1)` phantoms at the spot's bottom centre, `finalizeSpawn(NATURAL)`.

### Plan
**Files the family creates** (D0-3):
- `entities/src/world/families/night/phantoms.rs`: `pub struct Phantom { size, attack_damage, speed, ai: Box<MonsterAi> }` and `impl Family` (`KEY = 80`, `KINDS = ["minecraft:phantom"]`). Hooks: `spawn` (`MobCore::new(kind, pos, 0.9, 0.5, 0.175, 20.0)`, `MonsterKind::Registered`), `tick`, `enemy` true, `monster` (the spec). The defaults stand for category, experience 5, despawn, `#undead`, the `entity.phantom.*` voices, not leashable and 0 seats. The file also holds the move control and the tests.
- `night/phantoms/goals.rs`: the four goals as `Goal<MonsterGoalContext>` and the `MonsterSpec` (vanilla goal names, eye 0.175, follow 16, a `walk` that changes nothing). Shared state goes in `GoalExtra` as `PhantomGoals { phase, anchor, move_target, sea_level, cats, hiss, swooped, collided, hurt_recently }`.
- `crates/minecraft_terrain/src/families/night/phantoms.rs` (`TerrainFamily`) and `phantoms_render.rs` beside it.
- In `world/src/natural_spawner/families/night.rs`: the row, and an `impl CreatureSpawns { pub fn spawn_phantoms(..) -> Vec<Tag> }` block. That module descends from `natural_spawner`, so it can call `difficulty_at`, `mob_spawn_rules` and `is_valid_empty_spawn_block`.

**Registration lines:** `pub mod phantoms;` and `phantoms: phantoms::Phantom,` in `family_group!(night 80..=95 { .. })` (empty in D0's worktree today); `mod phantoms;` and the type in `terrain_group!(night { .. })`; the row in `ROWS`.

**`Phantom::tick`** does not use `family_goal_step`, whose `monster_ai_step` (`world.rs:1022`) walks on ground navigation. In order:
1. `family_base_tick`. Reset `no_action_time` when a player is within 32 blocks (`vexes.rs:63-69`).
2. With AI:
   - fill `ai.state` and `PhantomGoals`: sea level from `world.sea_level`; live cats from `world.felines()` with `is_cat()`; `collided` from `body.horizontal_collision`; and the hurt time;
   - `ai.tick_goals(level, full)`;
   - the move control on `core.body.velocity` and `core.yaw`, with `core.look.head_yaw = core.yaw` and `core.look.pitch = xRot` (so every client gets the dive through `ServerMob.x_rot`);
   - `travel_flying(level, DVec3::ZERO, 0.2, yaw, fluid)`.

   Without AI: `monster_idle_without_ai`.
3. A bite (`ai.state.attack`):
   - `family_strike(id, target, attack_damage, PlayerHitKind::Melee { attacker, hunger_ticks: 0, lift: 0.0, throw: DVec3::ZERO, disable_shield_seconds: 0.0 })`;
   - the voice `Voice::Event("entity.phantom.bite", 0.3, 0.9 + 0.1r)` at the block centre (decision 4);
   - when `swooped`, the swoop voice.
4. For each id in `hiss`: `world.feline_mut(id)` pushes the cat's `sounds().hiss`. The cat code stays as it is. Vanilla cats run no goal against phantoms, so D0-3's "use `Prey::Kinds`" note is not needed.
5. `burn_undead(level, &mut core.random, &mut core.body, core.eye_height, world.monsters_burn, None)`.
6. `family_after_move`.
7. After W merges, one line: post `minecraft:flap` when `(id × 3 + tick_count) % 25 == 0`.

`set_size(n)` clamps; only on a changed value does it scale the body and eye and set `attack_damage = 6 + n` (2.0 at first).

**Spawn row:** `minecraft:phantom`, 0.9 × 0.5, `Placement::NoRestrictions`, `MotionBlockingNoLeaves`, `Rules::Family { creature: false }`, removed far away, cluster 4, Monster, follow 16, `baby_chance: None`.
- `rule`: spawner, or `mob_spawn_rules`.
- `finalize`: `anchor_pos` = the block plus 5 up, `size: 0`.

**Spawner (P6): one line each, next to the patrol's**
- `custom_spawners.rs`:
  - `PhantomSpawner { next_tick }` with `due(random)` after `PatrolSpawner` (`:171`);
  - the fields `phantoms` and `spawn_phantoms` after `patrols`/`spawn_patrols` (`:34-35`), set in `Default` (`:44-45`) and `for_dimension` (`:57-58`, Overworld).
- `server.rs`:
  - the rule line after `:3141`;
  - `ServerSim::tick_phantom_spawner(&mut self, players, rest)` after `:1288`: gates `spawn_mobs`, `spawn_monsters` and the rule; `due` on `self.level.random`; `self.level.spawn_phantoms(..)`; `add_saved_entity` for each tag;
  - its call after `:4223`.
- `world/src/level/spawning.rs`: `Level::spawn_phantoms(&mut self, players: &[(BlockPos, bool, i32)]) -> Vec<Tag>` after `spawn_patrol` (`:227`), built like `:228-236`.
- `CreatureSpawns::spawn_phantoms` runs the vanilla loop. `canSeeSky` is sky light 15 at the feet (javap `LevelReader.canSeeSky` during the work). The roll is `difficulty_at(..).effective > roll`. Each phantom is `self.summon(level, context, "minecraft:phantom", [x + 0.5, y, z + 0.5], None, 0.0)`, which finalizes through the row.
- **`time_since_rest` to the server** (decision 3):
  - `TickInput.rest: Vec<(u64, i32)>` after `mob_vitals` (`server.rs:3560`), plus `rest: Vec::new()` in the full test literals `idle_input` (`:11155`), `two_players` (`:11523`) and `eye_of_ender.rs:673`;
  - survival: `PlayerQueue.time_since_rest` after `uuid` (`entities.rs:192`), and `Entities::set_time_since_rest(uuid, ticks)` after `set_players` (`:533-542`, matching by `opener_key`);
  - the `rest:` line after `mob_vitals` in the `TickInput` literal (`:1519`);
  - one line in `sleep::tick` after `events.written.extend(written)` (`sleep.rs:726`) hands each sleeper's clock to `entities`.

**Client**
- `draw` ports the model on `posed_cube`: flap from `id × 3 + pose.age_in_ticks`, scale and translate by size, the whole body pitched by `pose.head_pitch`, into `out.models`.
- The eyes go in again at light 15, as `spider_render.rs:87-90` does. Keep only the parts whose `phantom_eyes.png` texels are not clear (check during the work).
- `client_effects`: the flap crossing and the two specks per tick.
- `Kind::Mycelium` sits next to `Kind::HappyVillager` in each `match` of `ambient_particles.rs` (`:51`, `:467` sprite `generic_0`, `:499`, `:1072`, `:1104`), built like `happy_villager` in grey. `Kind` is not serialized.

**Multiplayer:**
- `TRACKING_RANGE` 8; `VOLATILE = ["anchor_pos"]`.
- `size` stays in the look tag (a new size means a new tag, and the mirror respawns with it).
- No `FamilyShow` flags or values: the pitch comes in `ServerMob.x_rot`, burning in `MobShow.fire`, and the flap and specks from each client's `client_effects`.
- `TEXTURES` phantom and phantom_eyes; `SHADOW_RADIUS` 0.75 (javap `LivingEntityRenderer.getShadowRadius` during the work).
- No `## Protocol` entry: `TickInput` stays on the host.

**Keyframes:** none. `PhantomModel` is procedural, so `keyframes::shared()` is not used.

**COMMIT GROUPS**

| # | Group key | Commits |
|---|---|---|
| 1 | `phantoms-live` | 1. "Minecraft phantoms circle, swoop and bite like vanilla": the family, goals, move control, size, strike and sounds; the row; `TerrainFamily` `read`/`fields`/`VOLATILE`/`TRACKING_RANGE`, with `draw` empty until group 2; the `server.rs:8169` test; row 39 `live`. 2. "Minecraft phantoms burn by day and shy away from cats". |
| 2 | `phantoms-drawn` | 1. "Minecraft phantoms are drawn like vanilla": model, eyes, flap, size, pitch and cage; row `drawn`. 2. "Minecraft phantoms flap and trail specks from their wings like vanilla": `client_effects` and `Kind::Mycelium`. |
| 3 | `phantoms-spawner` | "Phantoms come for Minecraft players who have not slept in three days": `PhantomSpawner`, the rule, `Level::spawn_phantoms`, `TickInput.rest` and the hand-over; row `done`; the owner's steps and kit in the log. |

### Tests
- **1.1** (`world::families::night::phantoms::tests`):
  - `a_phantom_is_nine_tenths_by_half_with_twenty_health_and_its_kind`: size, eye, health, `entity_type`, census, `mob_candidates`, table and 5 experience, enemy, undead.
  - `a_bigger_phantom_grows_fifteen_percent_a_size_and_bites_for_six_plus_its_size`: size 4 gives 1.44 × 0.8 and 10 damage; size 0 keeps 2; 70 clamps to 64.
  - `a_phantom_circles_five_to_fifteen_blocks_around_its_anchor_without_falling`.
  - `its_move_control_turns_four_degrees_a_tick_and_speeds_to_one_point_eight`, including the wall turn of 180 with speed 0.1.
  - `a_phantom_picks_the_highest_player_in_sight_within_its_box`: Y order, the 16/64/16 box, creative and spectator skipped, a roof, scans at 20 ticks and then every 60.
  - `a_phantom_swoops_ten_ticks_after_finding_a_target_bites_and_climbs_away`: swoop voice, a Melee hit of 2, the bite voice, the target cleared, the anchor raised 10-29 above `MOTION_BLOCKING`, the next swoop after 160-220 ticks.
  - `a_wall_or_a_hit_breaks_off_a_swoop`.
  - `its_anchor_never_sinks_below_sea_level`.

  Also `natural_spawner::families::night::tests::a_phantom_row_spawns_from_spawners_and_finalizes_its_anchor_five_up`. Terrain: `a_phantom_saves_and_loads_with_its_size_and_anchor`, `a_friend_sees_a_phantom_from_its_look_tag` (`anchor_pos` dropped, `size` kept, `x_rot` carried) and the changed `server.rs:8169` test.
- **1.2:**
  - `a_phantom_catches_fire_under_the_day_sky_but_not_in_rain_or_at_night`;
  - `a_cat_within_sixteen_blocks_breaks_off_a_swoop_and_hisses`: checked every 20 ticks; a cat at 20 blocks, an ocelot and a dead cat do nothing.
- **2.1** (`phantoms_render::tests`):
  - `a_phantom_is_eight_cubes_on_a_64_texture`;
  - `its_wings_flap_sixteen_degrees_and_its_tail_curls_with_the_flap_time`;
  - `a_size_four_phantom_draws_one_point_six_times_larger_and_pitches_with_its_dive`.
- **2.2:**
  - `a_phantom_flaps_once_every_twenty_five_ticks_for_every_viewer`;
  - `a_phantom_trails_two_grey_specks_from_its_wingtips_each_tick`;
  - `ambient_particles::tests::mycelium_lingers_grey_for_twenty_to_a_hundred_ticks`.
- **3:**
  - `custom_spawners::tests::the_phantom_spawner_looks_at_once_then_every_one_to_two_minutes`.
  - `natural_spawner::families::night::tests::phantoms_come_for_the_sleepless_at_night_under_open_sky`, cases:
    - none by day, below sea level, under a roof, as a spectator, or at 72000;
    - most at 10^6;
    - 1 to `difficulty + 1` of them, 20-34 up and within ±10;
    - a solid or water spot refused.
  - `server::tests::a_sleepless_player_draws_phantoms_and_a_rested_one_does_not`: also none with the rule off, on peaceful or in the Nether.
  - `entities::tests::each_player_time_since_rest_reaches_the_server_tick`.

### Owner's steps (survival, Normal; flat open ground above y 63, no roof; `/time set midnight` unless a step says otherwise)
1. Use a phantom egg on the ground 5 blocks away: a dark blue winged phantom with glowing green eyes circles 5-15 blocks around a point about 5 blocks up. Its wings beat with a flap sound about every 1.25 s, and grey specks trail from its wingtips. **m3p1_phantom_circles**
2. Stand in its sight: within about 3 s it climbs 20-40 blocks above you, circles, then dives with a screech and bites for 1 heart (decision 1). It climbs away and dives again 8-11 s later. **m3p2_phantom_swoop**
3. Hit one with the sword mid-dive: it flashes and breaks off. Kill one each with the bow, the M4A1 and the Looting III sword: each leaves experience, and some drop phantom membranes (more often with Looting).
4. Use a cat egg beside you: dives near the cat stop short, the phantom pulls away, and the cat hisses. Walk 30 blocks from the cat: the dives come back. **m3p4_cat_scares_phantom**
5. Stand under a cobblestone roof: the phantom loses sight of you and only circles. Step out: it dives again.
6. `/time set day` with a phantom circling: it catches fire within seconds and burns to death. **m3p6_phantom_burns**
7. `/summon minecraft:phantom ~ ~8 ~ {size:6}`: a phantom 1.9 times larger whose bite takes 6 hearts. Save and quit with it and a small one alive, then load: both still circle, and the big one is still big.
8. Save and quit. In the world folder, open `players\stats\<your UUID>.json` (the only file there in single player) in Notepad and set `"minecraft:time_since_rest"` to `1000000`. Load, `/time set midnight`, and stand in the open: within 1-4 minutes 1-3 phantoms appear high above and attack. **m3p8_insomnia** Then check: in a cave or under a roof none come; `/gamerule spawn_phantoms false` stops them (set it back to true); `/difficulty peaceful` removes them.
9. Sleep in the bed until morning. The next night none come, even after 5 minutes.
10. Friend window (`scripts\mp-local.ps1`): the host hatches two phantoms at night. The friend sees them circle, flap and trail specks in the same places. The friend stands on a 5-block pillar, higher than the host: the phantoms go for the friend first and bite the friend. The friend shoots one with the M4A1: it dies in both windows. At `/time set day` both windows see them burn. **m3p10-host**, **m3p10-friend**
11. With the game closed, set the friend's stats file (the friend's UUID in the host's `players\stats`) to `1000000` too. At night, phantoms come for the friend standing alone in the open.

### Kit
```
IW4L_MINECRAFT_KIT=minecraft:phantom_spawn_egg*16,minecraft:cat_spawn_egg*4,minecraft:diamond_sword+looting=3,minecraft:iron_sword,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:red_bed,minecraft:cobblestone*64,minecraft:stone_slab*32,minecraft:ladder*32,minecraft:torch*64,minecraft:cooked_beef*32,minecraft:golden_apple*8,minecraft:diamond_helmet,minecraft:diamond_chestplate,minecraft:water_bucket,minecraft:spawner
```
18 stacks; the friend reads the same `.env`.

### Depends on
- **D0 d0a:**
  - `registry-core`;
  - `registry-hooks` (`MonsterKind::Registered`, `GoalExtra`, `family_strike`);
  - `registry-spawning` (rows, finalize, the `pub(super)` helpers);
  - `registry-client` and `registry-draw` (save and load, friends, `draw`, `client_effects`).

  **Not d0b:** the phantom steers itself and has no path navigation.
- **W:** optional, the `minecraft:flap` line only.
- **E1:** none. If E1 merged first with new particle kinds next to `HappyVillager`, put `Mycelium` after them.
- **Other M3 groups:** none. The phantom groups run after the four lightning groups in segment m3a, and nothing waits for them.

### Decisions for the track log
1. **Bite damage:** 2 for size 0, and `6 + size` after a size change. This is the 26.3 server's behaviour: `setPhantomSize(0)` never fires `onSyncedDataUpdated`, so `ATTACK_DAMAGE` stays at 2.0. The often-quoted 6 is not what the server does.
2. **The `server.rs:8169` test:** after round 10 no `of_type` monster stays dormant (silverfish in D0; phantom, zombie horse and camel husk in M3). Switching to the zombie horse would break again in `undead-horses`. Recommended: keep the shulker half, make the second mob a live phantom that `check_despawn` removes beyond 128 blocks and on peaceful, and rename the test `live_shulkers_stay_far_from_players_while_other_monsters_go`. The dormant rules keep their own tests (`dormant.rs:77-106`).
3. **`time_since_rest`** reaches the server through `TickInput.rest`, filled from `sleep::tick`, not through a new `PlayerView` field (about a dozen literals in shared files).
4. **The bite sound** is a mob voice with level event 1039's sound, volume and pitch, not a new `level_events.rs` arm.
5. **The countdown** lives in `CustomSpawners` beside the patrol's; the rolls live in M3's world group file.

---

## M3-3. Creakings and creaking hearts (groups `creakings-live`, `creakings-drawn`, `creakings-heart`, `creakings-crumble`)

Family `creakings` in group `night`, type `Creaking`, kind `minecraft:creaking`, hit-key byte **83**. It is a brain family (D0-2.4: the brain lives in its state, so it has no `MonsterSpec`). Mob list row 48. Facts were checked with javap on the 26.3 `client.jar` this round unless marked "javap during the work". Paths are in `C:\Dev\mashup-crafting` at `d1a3d028`; re-check every line before editing.

### What exists today

- **Gaze (endermen).**
  - `PlayerView { head_yaw, pitch, disguised }` is at `entities/src/enderman.rs:59`.
  - `is_looking_at_me` (`:87`) and `looking_at` (`:93`) use the enderman's narrow cone and a collider clip.
  - Every player's view reaches the entity world each tick, friends included. `minecraft_survival/src/entities.rs:1506` builds `mob_views` from all actors. `disguised` is hard-coded false at `:1514`. The views pass through `minecraft_terrain/src/server.rs:4238` to `set_player_views` (`entities/src/world/endermen.rs:268`).
  - They are stored in the private field `player_views` (`world.rs:2525`), which family files under `world/` can read.
  - `sight::line_of_sight` (`sight.rs:157`) is the collider version. `PlayerCandidate` (`tempt.rs:6`) has `position`, `eye_height`, `spectator` and `attackable`.
- **Closest brain:** the zoglin path of `hoglin_brain.rs`: `sense` (`:239`, every 20 ticks), `start_zoglin_idle` (`:397`), `update_zoglin_activity` (`:419`), `start_attacking` (`:435`), `look_sometimes` (`:458`), `idle_movement` (`:485`, RunOne), `fight` (`:699`, melee with a 40-tick cooldown).
- **Mobs that already react to creakings.** Pillagers, vindicators and evokers have the real `AvoidEntityGoal(Creaking, 8, 1.0, 1.2)` (`illager_ai.rs:167`, `:258`, `:322`). Round 9 noted that it "finds nothing until creakings exist". D0's `mob_candidates` arm feeds it with no edit.
- **Lists that already name the creaking:** `NOT_IN_PEACEFUL` (`generator/src/feature/mod.rs:160`) and `sleep::MONSTERS` (`minecraft_survival/src/sleep.rs:58`). The creaking is not in `#undead`.
- **The heart block, without its behaviour.**
  - Pale garden trees place a dormant natural heart in the trunk (`generator/src/feature/tree/decorator.rs:274`).
  - A neighbour change schedules a 1-tick block tick (`generator/src/feature/update.rs:649`). The level's `tick_block` (`world/src/level/mod.rs:839`) has no arm for it, so the state never changes.
  - The block entity keeps its `creaking` UUID (`core/src/block_entity.rs:289`).
  - A natural heart gives 20-24 experience to a player (`minecraft_terrain/src/experience.rs:52`). Its loot (1-3 resin clumps, or the heart with Silk Touch) rolls from the data pack.
  - A level explosion on a heart is logged as unsupported (`world/src/level/explosion.rs:409`).
  - `gameplay/creaking_active` is declared (`core/src/environment.rs:148`) and can be read through `Level::environment_flag` (`world/src/level/time.rs:190`, `pub(super)`). The pattern is `snow_golem_melts_at` at `:245`.
- **The store pattern to copy:** `end_gateway.rs` `Gateways` (`:99`, `load_chunk` `:121`, `unload_chunk` `:150`, `sync` `:108`, `tick` `:155`). `ServerSim` uses it at these lines of `server.rs`:
  - field `:84` and init `:457`;
  - load `:863`, `sync_pois` `:904` and unload `:2263`;
  - `collect_block_entity_changes` `:3334` and `:3340`;
  - `tick_gateways` `:2970`, called at `:4256`.

  Tags are written with `Level::set_block_entity_value` (`world/src/level/container.rs:105`), which cannot remove a key.
- **Spawning:**
  - `golem_spawn::try_spawn_golem` (`golem_spawn.rs:169`). Track W4 adds `try_spawn_mob(kind, reason, centre, attempts, xz, y, strategy, rules)` with `ON_TOP_OF_COLLIDER` there.
  - `ServerSim::summon` (`server.rs:1549`).
  - The `spawn_mobs` and `spawn_monsters` rules (`world/src/level/spawning.rs:30-32`).
  - `uuid_of` (`world.rs:4033`) and `mob_with_uuid` (`world/riding.rs:752`).
- **Client.**
  - Eye layers are drawn as the same cubes at full light (`spider_render.rs:61-89`).
  - `display_ticks::animate_block` matches block paths (`display_ticks.rs:75`, `end_gateway` arm at `:77`).
  - The walk target is `min(d × 4, 1)` for every mob (`walk_animation.rs:22`, called at `client_mobs.rs:691`).
  - `CreakingAnimation` already parses: the gated test at `keyframes.rs:1359` reads every `*Animation` class.
  - No `trail` or `block_crumble` particle exists.

### Vanilla 26.3 (javap)

**`EntityTypes.CREAKING`:** `MONSTER`, `sized(0.9, 2.7)`, `eyeHeight(2.3)`, `clientTrackingRange(8)`, `notInPeaceful`. `fireImmune` is not set on the type; the entity answers it (see below).
- `SpawnPlacements`: `ON_GROUND`, `MOTION_BLOCKING_NO_LEAVES`, `Monster.checkMonsterSpawnRules`.
- No biome spawns it; only hearts do.
- Loot `entities/creaking` is empty.

**`Creaking extends Monster`:**
- **Attributes:** health 1, speed 0.4, attack 3, follow range 32, step height 1.0625. `xpReward` 0. Navigation `setCanFloat(true)`.
- **Synced data:** `CAN_MOVE` (true), `IS_ACTIVE`, `IS_TEARING_DOWN`, `HOME_POS`. Saved: `home_pos` (an int array) only.
- **`setTransient(pos)`:** sets the home and the pathfinding maluses DAMAGING 8, POWDER_SNOW 8, LAVA 8, FIRE 0, FIRE_IN_NEIGHBOR 0. Heart-bound means it has a home.
- **`tick` (server):** with a home, if the block entity there is not a heart whose `isProtector(this)` holds, `setHealth(0)` (no `die`, no sound).
- **`aiStep`:** the invulnerability and attack countdowns tick down. Then `checkCanMove`. On a change it posts `entity_action` and plays `entity.creaking.unfreeze`, or calls `stopInPlace` and plays `entity.creaking.freeze`.
- **`checkCanMove`** (over the `NEAREST_PLAYERS` memory):
  - With no players, it deactivates if active and may move.
  - Each player is skipped unless `canAttack` (not creative, not peaceful) and not allied; such a player marks "an attackable player".
  - While active, a player with `#gaze_disguise_equipment` on the head is also skipped.
  - If the player `isLookingAtMe(this, 0.5, false, true, [eyeY, y + 0.5 × scale, (eyeY + y) / 2])`: an active creaking freezes. An inactive one within `distanceToSqr < 144` calls `activate(player)` and freezes.
  - After the loop, with no attackable player it deactivates if active. It may move.
- **`isLookingAtMe` with these arguments:** per gaze height, the dot product of the normalised view and direction is above 0.5 (a 60-degree cone, not scaled by distance), and there is a line of sight with `ClipContext.Block.VISUAL` within 128 blocks.
  - The 26.3 `getVisualShape` overrides are `TransparentBlock` (empty, so glass does not hide it), `IronBarsBlock`, `FenceBlock`, `MudBlock`, `SoulSandBlock`, `SnowLayerBlock` and `PowderSnowBlock` (javap each during the work).
- **`activate(player)`:** `ATTACK_TARGET`, `entity_action`, `entity.creaking.activate`, active. **`deactivate`:** the reverse, with `entity.creaking.deactivate`.
- **`hurtServer`:**
  - Not heart-bound, or a `#bypasses_invulnerability` source: `Monster.hurtServer` (one health).
  - Otherwise nothing happens if it is invulnerable to the source (fire while fire immune), the invulnerability countdown is above 0, or it is dying.
  - Nothing happens either unless the direct entity is living or a projectile, or there is a blamed player.
  - On a hit: invulnerability countdown 8, entity event 66, `entity_action`. If the home heart `isProtector(this)` and there is a blamed player, it calls `heart.creakingHurt()`. It plays `entity.creaking.sway` and returns true with no damage and no red flash.
- **`doHurtTarget`:** attack countdown 15, entity event 4, then the monster hit.
- **Pushing and knockback:** it can be pushed and knocked back only while it can move.
- **Fire, portals, walk target:** `fireImmune` when heart-bound. `canUsePortal` false when heart-bound. `getWalkTargetValue` 0.
- **`tickDeath`:**
  - Heart-bound and tearing down: `deathTime` + 1, and past 45 it calls `tearDown`.
  - Otherwise `Monster.tickDeath` (20 ticks and the poof).
- **`tearDown`:** `block_crumble` particles at the box centre, spread 0.3 × the box size: 100 of `pale_oak_wood` and 10 of the awake heart. Then the death sound and `remove(DISCARDED)`.
- **`creakingDeathEffects(source)`:** `die(source)` (kill credit, no loot, 0 experience) and `entity.creaking.twitch`.
- **`playerIsStuckInYou`:** a player's eye inside its box adds 1 to a counter, which resets otherwise. It is true once the counter passes 4.
- **`updateWalkAnimation(d)`:** `update(min(d × 25, 3), 0.4, 1)`. In 26.3 only `Camel`, `Frog` and `Creaking` override it.
- **`checkEyeBlink` (client):** once `deathTime > nextFlickerTime`, `nextFlickerTime = deathTime + between(glowing ? 2 : deathTime / 4, glowing ? 8 : deathTime / 2)` and the eyes toggle.
- **Sounds:**
  - ambient `entity.creaking.ambient`, silent while active;
  - hurt `entity.creaking.sway` when heart-bound, else `entity.hostile.hurt` (there is no `entity.creaking.hurt`);
  - death `entity.creaking.death`;
  - step `entity.creaking.step` at 0.15;
  - `attack` (the client plays it on event 4), `activate`, `deactivate`, `freeze`, `unfreeze`, `twitch`, `spawn`.
- **Entity events:** 66 (invulnerable shudder and hurt sound), 4 (attack animation and sound), 60 (the heart's spawn poof).

**Controls:** look, move, jump, body rotation and the navigation tick do nothing while it cannot move. `HomeNodeEvaluator` (a `WalkNodeEvaluator`, doors passable) marks a node `BLOCKED` when its squared distance to home is over 1024 and not below the mob's own.

**`CreakingAi`:**
- Sensors `NEAREST_LIVING_ENTITIES` and `NEAREST_PLAYERS` (javap `PlayerSensor`'s rate and ranges during the work).
- CORE: `Swim(0.8)` only while it can move, `LookAtTargetSink(45, 90)`, `MoveToTargetSink`.
- IDLE (10): `StartAttacking` (only while active, with target `NEAREST_VISIBLE_ATTACKABLE_PLAYER`), `SetEntityLookTargetSometimes(8, 30-60)`, and `RunOne` of stroll 0.3 (weight 2), `SetWalkTargetFromLookTarget(0.3, 3)` (weight 2) and `DoNothing(30, 60)` (weight 1).
- FIGHT (10, needs `ATTACK_TARGET`): `SetWalkTargetFromAttackTargetIfTargetOutOfReach(1.0)`, `MeleeAttack(canMove, 40)`, and `StopAttackingIfTargetInvalid` when the target is not in `NEAREST_VISIBLE_ATTACKABLE_PLAYERS`.
- `updateActivity`: IDLE while frozen, else FIGHT then IDLE.

**`CreakingHeartBlock`:** properties `axis` (y), `creaking_heart_state` (`uprooted` by default, `dormant`, `awake`) and `natural` (false).
- `hasRequiredLogs`: both neighbours along the heart's axis are `#pale_oak_logs` with the same axis.
- `updateState`: an uprooted heart with its logs becomes awake while `creaking_active`, else dormant. Placement uses it (the axis from the clicked face), and so does the 1-tick `tick` after a neighbour change.
- `animateTick`: while active, not uprooted, 1 in 16, and with logs on all six sides, the local sound `block.creaking_heart.idle`.
- `playerWillDestroy` → `removeProtector(playerAttack)`. A destroying explosion's `onExplosionHit` → `removeProtector(explosion source)`. Both also award the natural experience.
- An uprooted heart has no ticker.

**`CreakingHeartBlockEntity`:**
- **Every tick:**
  - It updates the comparator when the signal changed.
  - While the emitter runs (counting down from 100), each tick above 50 sends one orange trail from the heart to the creaking and one grey trail back.
  - Every 10th emitter tick, `block.creaking_heart.hurt` plays at `lerp(0.2 + 0.8 × (100 - e) / 100)` from the creaking towards the heart, with volume `e / 200 + 0.5`.
- **Every 20 + `nextInt(5)` ticks:**
  - `updateCreakingState`: with no logs and no creaking the heart becomes uprooted, which stops the ticker. Otherwise it is awake or dormant.
  - With a creaking, the heart tears it down (`removeProtector(null)`) in three cases:
    - `creaking_active` is off and the creaking is not persistent;
    - the creaking is more than 34 blocks from the heart's bottom centre;
    - `playerIsStuckInYou` holds.
  - Without a creaking, it spawns one only when all of these hold: the heart is awake, `spawn_mobs` and `spawn_monsters` are on, the difficulty is not peaceful, and a non-spectator player (creative counts) is within 32.
  - The spawn is `trySpawnMob(CREAKING, SPAWNER, 5 attempts, 16, 8, ON_TOP_OF_COLLIDER_NO_LEAVES, collision check true)`. It then posts `entity_place`, sends event 60, calls `setTransient(pos)`, and plays `entity.creaking.spawn` and `block.creaking_heart.spawn`.
- **The protector:** a removed creaking falls back to its UUID. A UUID is resolved with `getEntity` and cleared once `ticksExisted >= 30`; `ticksExisted` resets when a UUID is set.
- **`creakingHurt`:** only when the emitter is idle. It sends 20 grey trails, spreads resin 2-3 times when awake, and sets the emitter to 100.
- **`spreadResin`:**
  - It walks breadth-first through `#pale_oak_logs`: depth 2, at most 64 blocks.
  - Each log tries its sides in shuffled order: air becomes `resin_clump`, a water source becomes a waterlogged clump, and the face towards the log is set.
  - The first success stops the walk and plays `block.resin.place` with `block_place`.
- **Trails:** colour `0xFC7812` (towards the creaking) or `0x5F5F5F`, a random point in each box, lifetime `10 + nextInt(40)` (`TrailParticleOption`).
- **`removeProtector`:** with no source, `tearDown`. With a source, `creakingDeathEffects`, then tearing down and health 0. Any other removal of the heart (`preRemoveSideEffects`) is `removeProtector(null)`.
- **Comparator:** `15 - floor(clamp(dist, 0, 32) / 32 × 15)`.
- **Saved:** `creaking` (UUID).
- **`SpawnUtil` details:** a random offset in ±xz and ±y inside the border; the strategy scan downwards; a `noCollision` box check; then `checkSpawnRules`. For SPAWNER that means not peaceful and dark enough (only `TRIAL_SPAWNER` skips the light). Then `checkSpawnObstruction`, the add, and `playAmbientSound`.
- **`ON_TOP_OF_COLLIDER_NO_LEAVES`:** the spot has no collision, and the block below is not leaves and has a full top face.
- **`creaking_active`:** true from 12600 to 23401 (`timeline/day.json`), Overworld only.

**Client:**
- `CreakingRenderer`: shadow 0.6, `creaking.png`, `entityCutoutNoCull`.
- One emissive layer (`RenderTypes.eyes`, `creaking_eyes.png`) on a head-only model, alpha 1 when the eyes glow.
- `extractRenderState`: while tearing down, `deathTime` is 0 and there is no red overlay, and the eyes follow the flicker. Otherwise they glow while active.
- `CreakingModel` (64 × 64): `root` → `upper_body` → `head`, `body`, `right_arm`, `left_arm`; `root` → `left_leg`, `right_leg`. It has 16 boxes, 4 of them flat (two head flaps and two root planes). Record the javap'd `createMesh` numbers in the log.
- `setupAnim`: head `xRot` and `yRot`; `applyWalk(pos, speed, 1, 1)` only while it can move; then the attack, invulnerable and death `AnimationState`s (each started by `animateWhen`).
- Animations: `CREAKING_WALK` 1.125 s looping, `CREAKING_ATTACK` 0.7083 s looping, `CREAKING_INVULNERABLE` 0.2917 s, `CREAKING_DEATH` 2.25 s.

### Plan

**Files (D0-3):**
- `entities/src/world/families/night/creakings.rs`:
  - `pub struct Creaking`: home, can move, active, tearing down, death time, the countdowns, the stuck counter, link marks, pending heart calls, the heart's emitter copy, and the brain;
  - `impl Family for Creaking`;
  - an `impl EntityWorld` block with the heart API.
  - `night/creakings/brain.rs` holds `CreakingBrain`, shaped like the zoglin's.
- `world/src/natural_spawner/families/night.rs`: one row, `minecraft:creaking`:
  - 0.9 × 2.7, `OnGround`, `MotionBlockingNoLeaves`, not fire immune;
  - `Rules::Family { creature: false }`, monster walk, removed when far;
  - `Monster`, follow range 32, `baby_chance: None`, no `finalize`;
  - rule `checkMonsterSpawnRules`: not peaceful, `dark_enough`, and `spawner` or a valid block below.
- `world/src/level/creaking_heart.rs` (vendored, the block work):
  - the block tick, `has_required_logs`, `surrounded_by_pale_oak`, `creaking_active_at(pos)` and `update_creaking_heart_state(pos)`;
  - `creaking_heart_link(pos)` and `set_creaking_heart_link(pos, Option<[i32; 4]>)`, which write or remove the key through `self.chunks` and `changed_block_entities`;
  - `spread_resin(pos)`.
- `crates/minecraft_terrain/src/families/night/creakings.rs` (`impl TerrainFamily`), `creakings/render.rs` and `creakings/heart.rs` (`CreakingHearts` on the `Gateways` pattern, plus `tick`).

**Hooks (D0-2.2):**
- `KEY` 83, `KINDS`, `spawn`, `tick`.
- `hurt`: the rules above. It pushes its own `entity.hostile.hurt` or `entity.creaking.sway`.
- `experience` 0, `enemy` true.
- `fire_immune`, `pushable` and `knockback_resistance`: heart-bound, can move, and 1 while frozen.
- `voice` ("creaking", 1, hostile), `step_sound` `entity.creaking.step`.
- The rest keep their defaults; `removed` stands for `tearDown`.
- **The tick:**
  - `family_base_tick`;
  - it drops a `Voice::Ambient` pushed while active (vanilla still rolls and resets the timer);
  - the unlinked check, then the countdowns;
  - `check_can_move` over `players` and `self.player_views`, with a visual clip (a `sight::clip` variant that skips blocks with an empty visual shape, written in the family file);
  - the brain moves it only while it can move;
  - `family_after_move`. Melee uses `family_strike` with `PlayerHitKind::Melee`.
- **The 45-tick twitch:** while tearing down, the family keeps `core.damage`'s death clock under 20, counts its own `death_time`, and sets `core.removed` past 45. If that is not possible, it adds a root `Family::death_ticks` hook in its own commit.
- **Heart API on `EntityWorld`:**
  - `bind_creaking(id, home)`;
  - `link_creakings(&[(u64, Pos, i32)])`: it marks each heart-bound creaking linked or not and copies the emitter value. A creaking never marked yet is spared, so the one-tick lag never kills one that loaded or spawned this tick;
  - `creaking_links()`: id, home, centre, persistent, player stuck;
  - `take_creaking_heart_calls()`;
  - `tear_down_creaking(id, TearDown::Now | TearDown::Killed { by })`.

**Shared files (one line each, next to a related arm):**
- `world/src/level/mod.rs`:
  - `pub mod creaking_heart;` after `pub mod container;` (`:12`);
  - in `tick_block` (`:839`), the `CreakingHeartBlock` arm before `ComposterBlock` (`:860`).
- `world/src/level/explosion.rs:409`: the heart leaves the unsupported list. A destroying explosion pushes `pos` to a new `Level.exploded_hearts` field, declared after `destroyed` (`mod.rs:218`, init `:310`). See open decision 2.
- `crates/minecraft_terrain/src/server.rs`, each beside its `gateways` twin:
  - the field (`:84`), the init (`:457`), load (`:863`), the syncs (`:904`, `:3334`, `:3340`) and unload (`:2263`);
  - `ServerSim::tick_creaking_hearts`, three lines after `tick_gateways` (`:2970`): it passes the table, `level`, `mobs`, the players and a summon closure to `heart::tick`;
  - the call to `tick_creaking_hearts` after `sim.tick_gateways();` (`:4256`);
  - a "broken by a player" marker after `remove_blocks` in `player_breaks` (`:2271`) and `player_mines` (`:2276`). `Mined`, `SilkTouched` and `Blasted` all count, since a MW2 blast is the player's explosion.
- `display_ticks.rs:77`: a `creaking_heart` arm after `end_gateway` (the idle sound, keyed on the `awake` state).
- `entities/src/golem_spawn.rs` (W4's code): `ON_TOP_OF_COLLIDER_NO_LEAVES` next to `ON_TOP_OF_COLLIDER`, plus the collision flag.
- `creakings-crumble` only: `ambient_particles.rs` `Kind::Trail` next to `Kind::EndRod` (`:55`), with its arms beside `EndRod`'s.

**Heart tick (terrain):** `serverTick` as above, per loaded heart.
- It spawns through W's `try_spawn_mob` with reason SPAWNER, using the row's rule with `spawner = true` as the rules closure (the dark check), and `ServerSim::summon`.
- Then `bind_creaking`, event 60, and the two sounds.
- `ticksExisted`, `ticker` and the emitter live in the table and are not saved (vanilla does not save them).
- At the end of the tick, `link_creakings` runs, and the heart calls are drained into `creakingHurt`.

**Multiplayer:**
- `TRACKING_RANGE` 8.
- `FamilyShow` flags: 1 `CAN_MOVE`, 2 `ACTIVE`, 4 `TEARING_DOWN`, 8 `HEART_BOUND`, 16 `CRUMBLE` (the last show before an immediate tear-down, `creakings-crumble`).
- Values:
  - [0] ticks since the attack started, or -1;
  - [1] ticks since the shudder started, or -1;
  - [2] tear-down `death_time`, or -1;
  - [3], [4], [5] the home position;
  - [6] the heart's emitter;
  - [7] unused.
- `VOLATILE`: none (`home_pos` is set in the spawn tick and never changes).
- All sounds are host voices. The host tests every player's gaze, so a friend's look freezes the creaking too.

**Keyframes:**
- `keyframes::shared().definition("CreakingAnimation", "CREAKING_WALK" | "_ATTACK" | "_INVULNERABLE" | "_DEATH")`. Each start tick is the client's age minus the shown elapsed ticks.
- `draw` follows the `vex_render.rs:137` shape. While tearing down it draws a `MobPose` copy with `death_time` 0 and no red overlay.
- The eyes are the head cubes at full light while active.
- The tear-down flicker replays `checkEyeBlink` from death time 0 with a `LegacyRandom` seeded by the mob id, so every viewer sees the same flicker.

**Missing hook (walk speed):** add `TerrainFamily::walk_target(distance) -> f32` (default `(distance * 4.0).min(1.0)`) after `show_on` in D0's terrain root, read once in `client_mobs.rs` for registered cores, in its own root commit. M2's frogs need it too; M3 owns it (M30, open decision 3).

**Commit groups**

| # | Group key | Commits | Content |
|---|---|---|---|
| 1 | `creakings-live` | "Minecraft creakings freeze while players look at them" | the family (key 83), brain, gaze and activation, melee, the hurt rules with the heart-bound shudder, the link API, the twitch death, the home node rule, the spawn row; `mobs.md` row 48 `live` |
| 2 | `creakings-drawn` | 1. "Registered Minecraft mobs can set their own walk animation speed" (skip it if the root already has the hook). 2. "Minecraft creakings are saved, drawn and seen by friends like vanilla" | `TerrainFamily`, model, keyframes, eyes, flicker, shadow 0.6, cage; row `drawn` |
| 3 | `creakings-heart` | 1. "Minecraft creaking hearts wake at night and grow a creaking". 2. "Minecraft creakings die when their heart breaks and the heart bleeds resin when they are hit" | heart block tick, table, spawning, the dawn, 34-block and stuck tear-downs, the saved link; removal by players and explosions, `creakingHurt`, resin, emitter sounds; row `done` unless `creakings-crumble` follows |
| 4 | `creakings-crumble` | "Minecraft creakings and their hearts stream and crumble like vanilla" | trails (`Kind::Trail`), the crumble through M1's `ClientFx.dust` (M30; left for later if it is not merged), the spawn-poof fallback; row `done` |

Groups 3 and 4 run in segment m3g, after W's segment w2 is merged into `minecraft-survival` and `minecraft-survival` into `track-m3`. Group 3 alone touches about 20 sites in 8 files (every `gateways` twin in `server.rs`, `level/mod.rs`, `explosion.rs`, `golem_spawn.rs`, `display_ticks.rs`) plus three new files, so the client effects are their own group. If the segment runs long, `creakings-crumble` moves to "Left for later".

### Tests

`creakings-live`:
- `a_creaking_is_tall_with_one_health_and_its_kind`: size, eye height, health 1, speed, attack, follow range, step height, `entity_type`, census, `mob_candidates`, empty loot, 0 experience.
- `an_active_creaking_freezes_while_any_player_looks_and_moves_when_none_does`: the cone at three heights, two players, the freeze and unfreeze voices.
- `a_creaking_wakes_when_a_survival_player_looks_within_twelve_blocks`: 11.9 wakes it and 12.1 does not. Creative players, spectators and peaceful never wake or freeze it. It deactivates when no attackable player is within 32.
- `glass_does_not_hide_a_creaking_but_stone_does`.
- `a_frozen_creaking_is_not_pushed_or_knocked_back`.
- `a_creaking_hits_for_three_every_forty_ticks_with_its_attack_animation`.
- `an_unbound_creaking_dies_from_any_hit_and_gives_no_experience`.
- `a_heart_bound_creaking_shudders_instead_of_taking_damage`: 8 ticks, sway, a heart call only with a blamed player, fire immune, `generic_kill` still kills.
- `an_unlinked_heart_bound_creaking_dies_but_a_new_one_waits_for_its_heart`.
- `a_torn_down_creaking_twitches_forty_five_ticks_and_leaves_without_a_second_death`.
- `a_heart_bound_creaking_never_paths_past_thirty_two_blocks_from_home`.
- `illagers_flee_a_creaking` and `an_iron_golem_hunts_a_creaking`.
- World: `creakings_spawn_by_the_monster_rules_in_the_dark`.
- Only if the fallback root hook is needed (decision 4), its own commit: `families::tests::a_registered_family_can_keep_its_death_clock_running` (probe: a `death_ticks` of 45 keeps the mob in the store until tick 45, the default still 20).

`creakings-drawn`:
- `a_registered_family_walks_at_the_default_speed_unless_it_sets_its_own` (the hook commit).
- `a_creaking_saves_and_loads_with_its_home`.
- `a_friend_sees_a_creaking_from_its_look_tag` (every flag and value).
- `creakings::render::tests::a_creaking_has_sixteen_boxes_with_four_flat_ones`.
- `the_walk_runs_three_times_faster_at_full_speed`.
- `no_walk_keyframes_while_frozen`.
- `the_eyes_glow_only_while_active_and_flicker_the_same_for_every_viewer`.
- `a_torn_down_creaking_has_no_red_flash_or_death_roll`.
- `the_four_creaking_animations_read_from_the_jar` (gated).

`creakings-heart`, commit 1:
- `creaking_heart::tests::a_heart_between_two_logs_of_its_axis_wakes_at_night_and_sleeps_by_day`.
- `an_awake_heart_grows_a_creaking_within_sixteen_and_eight_of_a_player_within_thirty_two`: dark spots only, none on peaceful or with `spawn_monsters` off, spectators ignored, creative players count.
- `a_heart_tears_its_creaking_down_at_dawn_past_thirty_four_blocks_or_with_a_player_stuck_inside`: a persistent creaking stays at dawn.
- `a_heart_finds_its_creaking_again_after_loading` (the 30-tick grace).

`creakings-heart`, commit 2:
- `creaking_heart::tests::resin_spreads_breadth_first_on_the_heart_logs`.
- `breaking_a_heart_makes_its_creaking_twitch_then_crumble`: mined, blasted and TNT twitch; `/setblock` tears it down at once.
- `hitting_a_bound_creaking_makes_the_heart_bleed_resin_once_per_emitter`.

`creakings-crumble`:
- `a_hurt_creaking_streams_grey_to_its_heart_and_orange_back` and `a_torn_down_creaking_crumbles_into_pale_oak_and_heart_pieces`.
- `ambient_particles::tests::a_trail_flies_from_its_start_to_its_target_in_its_lifetime`.

### Owner's steps

Setup: release build, survival, **Normal**, flat open ground in the Overworld, the kit below, and the class loadout's M4A1. Steps 1-9 and 20 work from segment m3b (`creakings-live`, `creakings-drawn`). Steps 10-19 and 21 need segment m3g (`creakings-heart`; the streaks in step 11 and the bark pieces in steps 12-15 need `creakings-crumble`; without it a torn-down creaking still twitches, then vanishes with no pieces). Steps 10 to 19 are at night with no torches within 16 blocks of the heart, because the heart's spawn obeys the monster light check. Steps 20 and 21 need two windows (`scripts\mp-local.ps1`).

1. Run `/time set 1000`. Use a creaking egg 10 blocks away and step back to 20 blocks. A tall, thin, grey-brown bark creature appears, with a long head with two flat flaps, arms of different lengths, roots at its feet and dark eyes. It strolls slowly. **m3k1_creaking**
2. Walk to 8 blocks and look at it. It creaks, its eyes light orange and it stops dead. Turn your back: it creaks again and comes at you fast with heavy steps. Turn round: it freezes mid-step. **m3k2_frozen**
3. Keep your back turned until it reaches you. It swings both arms about every 2 seconds, for 1.5 hearts a hit.
4. Look at it through a 3 × 3 glass wall: it stays frozen. Behind a cobblestone wall, while you look at the wall, it walks round it.
5. Run `/gamemode creative`. It ignores your gaze and its eyes go dark. Then run `/gamemode survival`.
6. Hit one with the sword: it dies at once and gives no experience. Kill one with an arrow and one with the M4A1.
7. Use a pillager egg 5 blocks from a creaking: the pillager runs from it. Use an iron golem egg: the golem kills the creaking.
8. In creative, place a spawner and use a creaking egg on it. A small creaking turns in the cage. Return to survival and break the spawner.
9. Save and quit with an egg creaking alive, then load. It is still there and still freezes under your gaze.
10. Place a pale oak log, a creaking heart on its top face, and another log on top. Run `/time set 13000`: the heart's face turns orange (awake) within a second. Stand 10 blocks away. Within a few seconds a creaking appears within 16 blocks, with a puff, a creak and a thump from the heart. **m3k10_heart_spawn**
11. Hit that creaking with the sword. It shudders and creaks but takes no damage and does not flash red. Grey streaks fly into the heart and orange ones back for about 2.5 seconds, the heart groans, and 2 or 3 orange resin clumps grow on the logs. Bullets and arrows do the same, and it never dies. Lava at its feet does not burn it. **m3k11_heart_hurt**
12. Break the heart with the plain diamond axe. The creaking twitches for about 2 seconds with flickering eyes, then crumbles into pale bark pieces with a death creak. The heart drops 1-3 resin clumps. **m3k12_twitch**
13. Rebuild the heart. Walk 40 blocks away with the new creaking following you: past 34 blocks from the heart it crumbles at once.
14. With a new creaking, run `/time set 23500`. Within about a second it crumbles and the heart goes dark (dormant).
15. Run `/time set 13000`. With a creaking alive, light TNT beside the heart: the creaking twitches and crumbles.
16. With no creaking near, take away the top log. The heart turns plain grey (uprooted) and no creaking comes. Put the log back and it is awake again.
17. Break a heart with the Silk Touch axe: it drops the heart.
18. Save and quit with a heart and its creaking, then load. Hits still make the heart bleed resin, and breaking it still makes the creaking twitch.
19. (Optional) Run `/locate biome minecraft:pale_garden` and `/tp` there at night. Creakings come from hearts hidden in pale oak trunks. Breaking such a heart makes its creaking twitch and gives experience. **m3k19_pale_garden**
20. Two windows: The host wakes a creaking, then turns away while the friend looks at it: it stays frozen in both windows. When the friend also turns away, it walks in both, with the same eyes, pose and steps. **m3k20-host**, **m3k20-friend**
21. Two windows, at night: The friend hits a heart's creaking: both windows show the shudder, the streaks and the resin. The friend breaks the heart: both see the twitch and the crumble. **m3k21-host**, **m3k21-friend**

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:creaking_spawn_egg*8,minecraft:creaking_heart*4,minecraft:pale_oak_log*32,minecraft:diamond_axe,minecraft:diamond_axe+silk_touch=1,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:glass*32,minecraft:cobblestone*64,minecraft:spawner*2,minecraft:pillager_spawn_egg*2,minecraft:iron_golem_spawn_egg*2,minecraft:tnt*4,minecraft:flint_and_steel,minecraft:lava_bucket,minecraft:shield,minecraft:cooked_beef*32
```

That is 18 stacks, and every id is in the 26.3 `assets/minecraft/items`.

### Depends on

- **D0 d0a:** all of it, including `registry-spawning` (the widened `dark_enough` and `mob_spawn_rules`) and `registry-draw` (`keyframes::shared()`, `ClientFx`). Not d0b.
- **W segment w2:** `try_spawn_mob`, for `creakings-heart` only.
- **W's P7 game events:** one line each for `entity_action`, `entity_place` and `block_place`, added by whichever of W and M3 merges second.
- **W's `player` field on `PlayerMines`:** without it, a kill by breaking the heart gives no kill credit.
- **E1:** the `puff_for` row for event 60. Until then, `client_effects` puffs a newly seen heart-bound creaking.
- **Other M groups:** none. M2's frogs share the walk hook. M1's parrots imitate creakings on their side.

### Open decisions (recommended choice first)

1. **Heart split.** Recommended: the block work goes in a vendored `level/creaking_heart.rs`, and the link and spawning in a terrain table (as `end_gateway.rs` does). Only `ServerSim` sees both the level and the entity world. `block_entity_mut` is private to `level::container`, and `set_block_entity_value` cannot remove a key.
2. **Level explosions on a heart.** Recommended (vanilla): the `Level.exploded_hearts` list, so the creaking twitches. Fallback: the creaking is torn down at once, logged under left for later.
3. **Walk hook.** M3 owns `walk_target` (M30): this exact name and default, after `show_on`. M2's frogs use it. If M2 landed the same hook first, skip the commit.
4. **Twitch length.** Recommended: the family holds the death clock itself. Fallback: a root `Family::death_ticks` hook.
5. **Eye flicker.** Recommended: deterministic from the mob id, so every viewer sees the same flicker. Vanilla uses each client's own random.
6. **Heart idle sound.** Recommended: keyed on the `awake` state, which lags `creaking_active` by at most about a second.
7. **Link timing.** Recommended: `link_creakings` runs after the mobs each tick, and a creaking never marked is spared.

---

## M3-4. Copper golems (groups `copper-golems-live`, `copper-golems-drawn`, `copper-golems-sorting`, `copper-golems-care`, `copper-golems-statues`; registry group `night`, family `copper_golems`, hit key 82)

### The owner's finding
`docs/round9/release-2-tests.md:411`: "copper golems don't exist yet". Row 47 of `docs/round10/mobs.md`: build, sorting copper chests, oxidation, statue.

### What exists today (`minecraft-survival`, `d1a3d028`)
- **Golem builds:** `third_party/minecraftoss/world/src/level/golem_build.rs`: snow and iron `GolemPattern`s with one body block name each (`:19-33`), `find` (`:137`), `golem_tag` (`:174`), `carved_pumpkin_on_place` (`:191-199`, called from `level/mod.rs:707-708`), `spawn_golem_in_world` (`:201-226`, which pushes the tag to `Level.spawned`). `translate_and_rotate` and `GolemMatch` are `pub`. The module line is `level/mod.rs:20`.
- **Copper chests work as blocks:** placing and joining as the least weathered (`player/src/placement.rs:12-16`, `:200-235`), opening and sounds (`crates/minecraft_terrain/src/containers.rs:33-37`, `:182-191`), drawing (`special_models.rs:37-41`, `:199-228`), and level items (`world/src/level/container.rs:120` `container_at`, `:189` `container_item`, `:226` `container_set_changed`, `:233` `container_set_item`). `chest_blocked` (`:113-117`) checks only for a conductor above.
- **Chest lids:** `crates/minecraft_terrain/src/openers.rs` (`Openers` `:37`, `open` `:55`, `close` `:68`, `recheck` `:81`). Survival keeps one per level (`crates/minecraft_survival/src/level.rs:24`, `toggle_openers` `:344`, `opener_outcome` `:373`, which reports counts to the server for trapped chests only at `:383-385`). The host ticks it at `crates/render_anim/src/minecraft_world.rs:1371-1378`, through `crates/minecraft_survival/src/menus.rs` (`lids` `:1793`, `recheck_lids` `:1825`, private `lid_effects` `:1838`). The level's count map is `redstone.rs:305` `set_chest_openers`.
- **Copper weathering:**
  - Random ticks weather every `Weathering*` class (`world/src/level/random_tick.rs:325-329`, `:688`), so statues already weather.
  - Statues keep their block entity through weathering (`world/src/level/drops.rs:113-118`), and lightning cleans them (`player/src/lightning.rs:48-49`).
  - Axes scrape and unwax them through the jar's `block_transformer/axe.json` (`crates/minecraft_terrain/src/transform.rs`), and honeycomb waxes them (`player/data/waxable_blocks_26_3.txt:9`).
- **Statues are invisible.** Their block model has only a particle texture, and `special_models::block_model` (`:272`) and `item_model` (`:369`) have no statue arm.
- **Iron golem gift:** `OfferFlowerGoal` (`entities/src/golem_ai.rs:368-424`) offers only to villagers ("No copper golem takes the poppy here", `:422`). Its candidates are set from `world/golems.rs:265` at `:355`.
- **Reusable pieces:**
  - `InteractWithDoor` (`entities/src/villager_brain/behaviors.rs:3745-3797`, private) and the `throwItem` maths (`:1966-1981`).
  - `EntityWorld.level_random` (`entities/src/world.rs:2581`), `sight::clip` (`sight.rs:110`) and `WalkProfile.max_visited` (`walk_path.rs:41`).
  - The snow golem's pumpkin drawn as a block (`snow_golem_render.rs:93`) and the iron golem's poppy (`golem_render.rs:84`).
  - `keyframes.rs` `apply_walk` (`:328`) and `definition` (`:792`).
- **Missing:**
  - No entity and no `type_info` row (`natural_spawner.rs:271`). `MobCategory` (`natural_spawner/spawning.rs:25`) has no `Misc`.
  - Lightning does not hit entities (`world/src/level/entity.rs:647`).
  - Dispensers neither shear nor place carved pumpkins (`world/src/level/dispenser.rs:17-25`).
  - Level events 3003-3005 (wax on, wax off, scrape) have no presenter (`crates/minecraft_terrain/src/level_events.rs:87`).

### Vanilla 26.3 (javap this round)
- **Type:** `EntityTypes.COPPER_GOLEM` is `MISC`, `sized(0.49, 0.98)`, `eyeHeight(0.8125)`, `clientTrackingRange(10)`, not fire immune, not `notInPeaceful`. The class is `CopperGolem extends AbstractGolem implements ContainerUser, Shearable`.
- **Attributes:** `createAttributes` is speed 0.2 (`0.20000000298023224`), step height 1.0 and health 12; follow range stays 16.
- **Constructor:**
  - `setRequiredPathLength(48)` (`max_visited` 768), `setCanOpenDoors(true)`, `setPersistenceRequired()` and state IDLE.
  - Malus `FIRE_IN_NEIGHBOR` 16, `DAMAGING_IN_NEIGHBOR` 16, `FIRE` -1.
  - `TRANSPORT_ITEMS_COOLDOWN_TICKS` = entity random `nextInt(60, 100)`. It is not saved, so a loaded golem waits again.
- **`AbstractGolem`:** no ambient sound, ambient interval 120, `removeWhenFarAway` false.
- **Other numbers:** leash offset (0, 0.75 × eye, 0); `getWalkTargetValue` 0; not an `Enemy`, not ageable, 0 experience.
- **Saved fields:**
  - `next_weather_age` (long: -1 unset, -2 waxed) and `weather_state` (`unaffected|exposed|weathered|oxidized`).
  - `equipment.mainhand` (the cargo), `equipment.saddle` (`EQUIPMENT_SLOT_ANTENNA`) and `drop_chances`.
  - `Brain.memories`: `visited_block_positions`, `unreachable_transport_block_positions` (sets of `GlobalPos`) and `gaze_cooldown_ticks`. The transport cooldown has no codec.
  - Synced: `DATA_WEATHER_STATE` and `COPPER_GOLEM_STATE` (IDLE, GETTING_ITEM, GETTING_NO_ITEM, DROPPING_ITEM, DROPPING_NO_ITEM).
- **Tick:** the brain runs, then `updateActivity` (IDLE). The server's `tick` then runs `updateWeathering(level, level.getRandom(), gameTime)`, also with no AI.
- **Brain (`CopperGolemAi`):**
  - Sensors `NEAREST_LIVING_ENTITIES` and `HURT_BY`.
  - CORE: `AnimalPanic(1.5)`, `LookAtTargetSink(45, 90)`, `MoveToTargetSink`, `InteractWithDoor`, and `CountDownCooldownTicks` for gaze and transport.
  - IDLE:
    - 0: `TransportItemsBetweenContainers(1.0, #copper_chests → chest|trapped_chest, 32, 8)`.
    - 1: `SetEntityLookTargetSometimes(PLAYER, 6.0, 40-80)`.
    - 2: `RunOne` when `WALK_TARGET` is absent and the cooldown is present: `RandomStroll.stroll(1.0, 2, 2)` or `DoNothing(30, 60)`, weighted 1:1.
- **`TransportItemsBetweenContainers` constants:** interaction 60 ticks, visited memory 6000 ticks, at most 16 items carried, 10 visited, 50 unreachable, idle cooldown 140, queue distance 3.0, reach 0.5 (1.0 once the path is done), stays within 2.0.
- **When it runs:** cooldown absent, not panicking, not leashed; it never times out. `start` sets `setCanPathToTargetsBelowSurface(true)`; `stop` clears it and runs `onStartTravelling`.
- **Choosing a target:** when the target is invalid, it takes a new one.
  - Invalid: gone, unwanted, a changed block entity, `isChestBlockedAt`, or, while travelling, no usable path. A path is usable when its end node is within reach and a `COLLIDER` clip sees a face centre; a failed path marks the chest unreachable.
  - New target: the nearest (strictly `<` `distToCenterSqr`) `ChestBlockEntity` in loaded chunks within `32/16+1` chunks and inside the block box grown (32, 8, 32).
  - Wanted: copper chests with an empty hand, otherwise a chest or trapped chest. Neither half visited or unreachable, and not locked.
  - Found: `onStartTravelling` (the golem clears its chest and goes IDLE) and the chest is marked visited; more than 10 visited means cooldown. None: cooldown 140 and both sets are forgotten.
- **Travelling:**
  - Within 3.0 of a chest someone has open (`getEntitiesWithContainerOpen` of either half): stop and queue.
  - Within reach: start interacting with PICKUP_ITEM (empty hand, items in the chest), PICKUP_NO_ITEM, PLACE_ITEM (the chest is empty or holds the same item, `isSameItem`) or PLACE_NO_ITEM.
  - Otherwise: `setWalkAndLookTargetMemories(pos, 1.0, 0)`.
- **Interacting:**
  - More than 2.0 away means travelling again. Otherwise it counts ticks, looks at the chest and stands still.
  - Tick 1: `startOpen` and the golem state. Tick 9: the sound. Tick 60: `stopOpen`.
  - At 60 items move. Picking up takes `removeItem(firstNonEmpty, min(count, 16))` into the hand as a guaranteed drop and forgets both sets.
  - Putting down fills slots in order: the whole stack into the first empty slot, or a merge into the same item and components. A remainder keeps only the target dropped, and the next chest is tried. The NO_ITEM cases drop the target and keep the visited set.
- **Interaction states and sounds** (vanilla's field names swap `no_`):

  | Interaction | State | Sound |
  |---|---|---|
  | PICKUP_ITEM | GETTING_ITEM | `entity.copper_golem.no_item_get` |
  | PICKUP_NO_ITEM | GETTING_NO_ITEM | `.no_item_no_get` |
  | PLACE_ITEM | DROPPING_ITEM | `.item_drop` |
  | PLACE_NO_ITEM | DROPPING_NO_ITEM | `.item_no_drop` |

  `actuallyHurt` sets IDLE. `hasContainerOpen` is its opened chest (either half).
- **Weathering:**
  - -2 does nothing. -1 sets `gameTime + levelRandom.nextIntBetweenInclusive(504000, 552000)`.
  - When due and not oxidized, it moves to the next stage; the next due time is 0 at OXIDIZED, otherwise `+ random(504000..552000)`.
  - When oxidized, it becomes a statue if the block at its feet is air and `levelRandom.nextFloat() <= 0.0058`.
- **Becoming a statue (`turnToStatue`):**
  - The block becomes `oxidized_copper_golem_statue` with `copper_golem_pose` = entity random `nextInt(0, 4)` of STANDING, SITTING, RUNNING, STAR and `facing` = `fromYRot(yRot)`.
  - The block entity gets the custom name. `dropPreservedEquipment` drops the cargo and the poppy, `discard` removes the golem without loot, and `entity.copper_golem_become_statue` plays. A lead drops as an item, or with `entity_drops` off it vanishes.
- **`mobInteract`:**
  - Empty hand while it holds cargo: `throwItem` to the player (eyes − 0.3, 0.3 × the unit vector).
  - Shears while the antenna holds `#shearable_from_copper_golem` (poppy): `entity.copper_golem.shear` (players), the poppy `spawnAtLocation` 1.5 up, game event SHEAR, shears −1.
  - Honeycomb when not waxed: level event 3003, `item.honeycomb.wax_on`, -2, one used.
  - Axe when waxed: `item.axe.scrape`, 3004, -1. Axe when not unaffected: `item.axe.scrape`, 3005, one stage back, -1. Each axe use costs 1 durability.
  - Lightning (`thunderHit`): once per bolt UUID, one stage back and -1.
- **Sounds:** unaffected and exposed use `entity.copper_golem.{spin,hurt,death,step}`, weathered `entity.copper_golem_weathered.*`, oxidized `entity.copper_golem_oxidized.*`. The step is its own at 1, 1. `entity.copper_golem.spawn` plays from `finalizeSpawn` (eggs, `/summon`), `spawn(state)` (builds) and statue waking.
- **Build (`CarvedPumpkinBlock.trySpawnGolem`):**
  - The snow, then iron, then copper pattern is tried. The copper pattern is `^` over `#`, where `#` is `#copper`: 8 blocks, waxed included.
  - Both cells are cleared (2001). The golem spawns at the pumpkin cell's bottom centre + 0.05.
  - The copper cell becomes `CopperChestBlock.getFromCopperBlock(block, pumpkin facing)`: the matching chest (waxed for waxed copper), its type from `getChestType`, joined as the least weathered, flags 2.
  - The golem gets the block's age; for waxed copper, the unwaxed twin's (`WAX_OFF_BY_BLOCK`). It is never waxed.
- **Statue blocks:**
  - 4 waxed blocks plus 4 `WeatheringCopperGolemStatueBlock`s, with `facing`, `copper_golem_pose` and `waterlogged`. The shape is a 10 × 14 column, and the comparator reads `pose ordinal + 1`.
  - Using any item but an axe cycles the pose (honeycomb passes on weathering statues), plays `entity.copper_golem_become_statue` and posts `block_change`.
  - An axe on an UNAFFECTED weathering statue calls `removeStatue`: a new golem with the name, at the bottom centre, yaw from `facing`, with the spawn sound; the axe takes 1 durability and the block is removed. Other axe and honeycomb uses go to the items.
  - Loot: the statue keeps its name and `copper_golem_pose`.
- **Gift (`OfferFlowerGoal`):** the candidates are `#candidate_for_iron_golem_gift` (villager + `#accepts_iron_golem_gift` = copper golem). In `stop`, with the timer run out, a copper golem with an empty antenna that touches the box grown (6, 2, 6) gets a poppy as a guaranteed drop.
- **Model (`CopperGolemModel`, 64 × 64, root +24 y):** `body` with its children `head` (box, nose, antenna, knob; the 0.015 shrinks), `right_arm` and `left_arm`, plus the root legs. Port the cubes from `createBodyLayer`.
- **Animation (`setupAnim`):**
  - The head follows look. `COPPER_GOLEM_WALK`, or `_WALK_ITEM` plus `poseHeldItemArmsIfStill` when holding, runs through `applyWalk(pos, speed, 2.0, 2.5)` (0.8333 s loops).
  - `_IDLE` (3.5 s) and four 3.0 s loops follow: GETTING_ITEM → `CHEST_INTERACTION_NOITEM_GET`, GETTING_NO_ITEM → `NOITEM_NOGET`, DROPPING_ITEM → `ITEM_DROP`, DROPPING_NO_ITEM → `ITEM_NODROP`.
  - On the client, a state animation runs `startIfStopped` when the state changes. While IDLE it spins every `tickCount + random(200..240)`, and 10 ticks after the spin starts the stage's spin sound plays.
- **Renderer:**
  - Shadow 0.5, the skin by stage, and an emissive eyes layer with the stage's eyes texture on a second full model.
  - Cargo in hand: when IDLE, y ∓90° and (0, 0, 0.125); otherwise scale 0.55 and (-0.125, 0.3125, -0.1875).
  - The antenna block sits at the head then (0, -1.75, 0), and a custom head layer is added.
- **Statue renderer:** the four pose layers (`createBodyLayer`, `createRunningPoseBodyLayer`, `createSittingPoseBodyLayer`, `createStarPoseBodyLayer`) with the stage's skin, root zRot π, translated (0.5, 0, 0.5) and turned by `facing.getOpposite()`. Javap the sign during the work.
- **Loot and spawning:** `entities/copper_golem` drops 1-3 copper ingots, plus 0-1 per looting level. There is no biome spawner and no `SpawnPlacements` entry.

### Plan
**Files (D0-3).**
- Entities, `third_party/minecraftoss/entities/src/world/families/night/`:
  - `copper_golems.rs`: `pub struct CopperGolem` and `impl Family` (`KEY` 82, `KINDS` `["minecraft:copper_golem"]`), with tests.
  - `copper_golems/brain.rs`: CORE and IDLE, plus a local `InteractWithDoor`. `AnimalPanic` comes from M1's `world/families/animal_brain.rs` (M1 `goats-live`, M30) when it is on `minecraft-survival`; if it is not, write the panic privately in `copper_golems/brain.rs` and log it for the switch.
  - `copper_golems/transport.rs`: the behaviour on `World`.
- Terrain, `crates/minecraft_terrain/src/families/night/`:
  - `copper_golems.rs`: `impl TerrainFamily`. The group file declares it with `pub mod`.
  - `copper_golems/render.rs` and `copper_golems/statue.rs`.
- One row in `third_party/minecraftoss/world/src/natural_spawner/families/night.rs`.
- New level files `world/src/level/copper_golem_build.rs` and `copper_golem_statue.rs`, and new `crates/minecraft_survival/src/golem_chests.rs`.

**Registry hooks.**
- `category`: "misc". `voice`: `copper_golem`, `copper_golem_weathered` or `copper_golem_oxidized` by stage, neutral.
- `step_sound`: `entity.copper_golem.step`. The tick maps `Voice::Step` to the stage's step at 1, 1.
- `leashable`: yes, with `leash_walk` on its navigation. `panicking` comes from the brain. `death_drops`: the cargo and the poppy.
- No ambient sound: `core.ambient_interval` is 120, and the `Voice::Ambient` that `family_base_tick` pushes is dropped, so the roll is kept.
- Weathering and the statue roll use `world.level_random`; the pose uses `core.random`. The statue conversion sets `removed`.
- **Row:** 0.49 × 0.98, `Rules::Family { creature: true }`, not removed far away, follow range 16, `baby_chance: None`, no rule or finalize, `MobCategory::Creature` in the row while the entity's `category` answers "misc" (decision 1).

**One-line hooks into shared files.**
1. The vendored `World` (`player/src/lib.rs:95`) gets default methods `chests_in(min, max)`, `container_items(pos)` (combined, not blocked), `set_container_item(pos, slot, stack)` and `name_block_entity(pos, name)`.
   - They go next to E2's `chest_open_count` (e2a is merged before m3c).
   - `MobWorld` (`server_mobs.rs:169`) overrides each one through `Level`, with a new `chest_positions_in` beside `container_at` (`container.rs:120`).
2. `golem_build.rs:191-199` calls `self.try_copper_golem(pos)` after the `GOLEMS` loop. `level/mod.rs:20` gets two `pub mod` lines.
3. `redstone.rs`:
   - `handles_use` (`:975-982`) and `use_block_facing` (`:990`, before the match) handle `CopperGolemStatueBlock`.
   - `state_analog_output` gets a statue arm next to `CopperBulbBlock` (`:511`).
4. Survival `actions.rs`:
   - A statue pass filter beside `bone_meal_passes` (`:813`).
   - `copper_golems::statue::wake_use` next to `ender_eye::frame_use` (`:1125`). It sends `ItemOnBlock::WakeStatue` (`server.rs:31`), handled next to `HangFrame` (`:2501`), which pushes the golem tag to `Level.spawned`.
5. `special_models.rs`:
   - A statue arm next to `decorated_pot` (`:307`), and `"minecraft:copper_golem_statue"` next to `"minecraft:chest"` (`:373`).
   - The 8 golem textures by the chest loop (`:243`). `push_cube` (`:765`) and `Sheet` (`:87`) become `pub(crate)`.
6. Lids:
   - `render_anim/src/minecraft_world.rs:1378` calls `golem_chests::lids(..)` after `recheck_lids`. `lid_effects` becomes `pub(crate)`, and `level.rs:24` gets `golem_openers: BTreeMap<u64, (i32, i32, i32)>`.
   - Each host tick compares the host world's golem chests with that map and calls `Openers::open`/`close` with `golem_opener_key(id)`.
   - Lids, sounds, friends' block events and trapped-chest power follow. `recheck` leaves golem keys alone.
7. Gift: `world/golems.rs:355` adds copper golems to the candidates. `golem_ai.rs:422` records the gift, and one line in `world/golems.rs` beside the villager gift calls `copper_golems::take_gift(self, id)`, whose body (the antenna set through `family_mob_mut::<CopperGolem>` as a guaranteed drop) stays in the copper golem's file (D0-3, shared-file rule).
8. Lightning: `copper-golems-care` overrides `Family::thunder_hit`, the root hook that `lightning-strikes` (M3-1) adds and dispatches.

**Multiplayer.**
- `TRACKING_RANGE` 10, `SHADOW_RADIUS` 0.5. `TEXTURES`: 4 skins and 4 eyes. `VOLATILE`: `weather_state`, `next_weather_age`, `Brain`.
- `FamilyShow`:
  - `values[0]`: the stage, 0-3.
  - `values[1]`: the state, 0-4.
  - `values[2]`: ticks since the state began.
  - `values[3]`: ticks since the idle spin began, or -1.
  - `flags` bit 0: holding cargo.
- `draw` reads only what `show_on` writes. The cargo and poppy items come from the look tag (decision 4).

**Keyframes.** `keyframes::shared().definition("CopperGolemAnimation", ..)` for `COPPER_GOLEM_WALK`, `_WALK_ITEM`, `_IDLE` and the four `_CHEST_INTERACTION_*`.

**Commit groups**
| # | Group key | Commits |
|---|---|---|
| 1 | `copper-golems-live` | 1. "Minecraft copper golems live in the entity world": brain without transport, stage sounds, idle spin, weathering clock (no statue yet), saving, row, eggs and `/summon` with the spawn sound, `FamilyShow`, `mobs.md` row 47. 2. "Minecraft copper golems are built from a carved pumpkin on a copper block". |
| 2 | `copper-golems-drawn` | 1. "Minecraft copper golems are drawn like vanilla": model, skin by stage, glowing eyes, seven keyframe animations, cargo and poppy, shadow. |
| 3 | `copper-golems-sorting` | 1. "Minecraft copper golems carry items from copper chests to chests". 2. "Minecraft copper golems open chest lids like players". |
| 4 | `copper-golems-care` | 1. "Minecraft copper golems are waxed, scraped and sheared like vanilla": throw back, honeycomb, axe, shears and the lightning scrape. 2. "Iron golems give copper golems a poppy". |
| 5 | `copper-golems-statues` | 1. "Oxidized Minecraft copper golems turn into statues". 2. "Minecraft copper golem statues are drawn, posed and woken with an axe". |

### Tests
- **live:**
  - `copper_golems::tests`:
    - `a_copper_golem_is_small_twelve_health_misc_and_its_kind`: size, eye, health, speed, step, `entity_type`, census none, `mob_candidates`, table, 0 experience, persistent.
    - `it_panics_at_one_and_a_half_when_hit_and_goes_idle`
    - `its_sounds_follow_its_stage_and_it_has_no_ambient`
    - `its_idle_spin_comes_every_200_to_240_ticks_with_its_sound_ten_in`
    - `it_weathers_after_504000_to_552000_ticks_and_stops_at_oxidized`
    - `a_waxed_copper_golem_never_weathers`
    - `it_opens_a_wooden_door_on_its_path`
  - Terrain:
    - `a_copper_golem_saves_and_loads_with_its_stage_wax_cargo_poppy_and_memories`
    - `a_friend_sees_a_copper_golem_from_its_look_tag`
    - `a_fresh_copper_golem_greets_once_and_a_loaded_one_does_not`
  - World:
    - `the_copper_golem_row_has_its_size_spawns_nowhere_and_is_kept_in_peaceful`
    - `golem_build::tests::a_pumpkin_on_each_copper_block_builds_a_golem_on_its_copper_chest`: 8 blocks, waxed, facing, the 0.05 lift.
    - `the_snow_and_iron_golems_are_still_tried_first`
    - `a_new_copper_chest_joins_a_neighbour_as_the_least_weathered`
- **drawn:** `render::tests`:
  - `a_copper_golem_has_six_parts_on_a_64_texture`
  - `its_skin_and_glowing_eyes_follow_its_stage`
  - `it_walks_with_its_arms_out_when_it_carries` (gated)
  - `its_chest_animation_starts_when_the_state_changes`
  - `its_cargo_sits_in_its_hands_and_its_poppy_on_its_antenna`
- **sorting:**
  - `transport::tests`:
    - `it_waits_sixty_to_a_hundred_ticks_then_takes_sixteen_of_the_first_stack`
    - `it_opens_at_one_sounds_at_nine_and_moves_items_at_sixty`
    - `it_tries_the_nearest_chest_first_and_shakes_its_head_at_one_without_its_item`
    - `a_remainder_sends_it_to_the_next_chest`
    - `with_no_wanted_chest_it_rests_140_ticks_and_strolls`
    - `it_queues_three_blocks_from_an_open_chest`
    - `it_gives_up_after_ten_visited_or_fifty_unreachable`
    - `a_blocked_or_unseen_chest_is_skipped`
    - `a_leashed_or_panicking_golem_stops`
  - `server_mobs::tests::the_mob_world_lists_chests_and_moves_their_items`
  - `golem_chests::tests::a_copper_golem_opens_and_shuts_a_lid_and_powers_a_trapped_chest`
  - `golem_chests::tests::a_golem_that_dies_with_a_chest_open_lets_it_shut`
- **care:**
  - `an_empty_hand_gets_its_cargo_thrown_at_the_player`
  - `honeycomb_waxes_it_once_and_an_axe_unwaxes_then_scrapes_a_stage`
  - `shears_take_its_poppy_at_one_and_a_half_up`
  - `lightning_scrapes_one_stage_once_per_bolt`
  - `golem_ai::tests::an_iron_golem_gives_its_poppy_to_a_copper_golem_beside_it`
- **statues:**
  - `an_oxidized_golem_on_air_turns_to_a_statue_in_a_pose_facing_its_way`: drops, `removed`, no loot, name moved.
  - `copper_golem_statue::tests::a_statue_cycles_its_pose_and_its_comparator_reads_one_to_four`
  - `an_axe_on_an_unaffected_unwaxed_statue_wakes_a_named_golem`
  - `axes_and_honeycomb_on_other_statues_pass`
  - `random_tick::tests::a_statue_weathers_and_keeps_its_name`
  - `special_models::tests::a_statue_has_four_poses_and_an_item_icon`

### Owner's steps (survival, Normal, by day, flat open ground)
Steps 1 and 7, the hit, kill and lead parts of step 6, and the colours in step 11 work from segment m3b (`copper-golems-live`, `copper-golems-drawn`). The rest need segment m3c: sorting (steps 2-5) from `copper-golems-sorting`, the throw-back, wax, axe, shears and lightning (steps 6, 8, 10 and 13) from `copper-golems-care`, and statues (step 9) from `copper-golems-statues`.

1. Place a copper block and put a carved pumpkin on it, facing you. Both vanish, a copper chest facing you takes the block's place, and a small copper golem with a long antenna stands on it with a chime. **m3g1**
   - Exposed, weathered and oxidized copper give matching colours.
   - Waxed copper gives a waxed chest and a plain golem.
   - A snow golem still builds.
2. Put 64 cobblestone, 32 oak logs and 16 apples in its copper chest. Place chest A 6 blocks away holding 1 apple, and an empty chest B 10 blocks away.
   - Within 5 s the golem opens the copper chest (lid and copper sound), reaches in for 3 s and comes out holding 16 cobblestone; the lid shuts. **m3g2**
   - At A it shakes its head, then it puts the cobblestone into B.
   - Apples end up in A.
3. When the logs come up, it finds no chest, rests about 7 s holding them and wanders. Place an empty chest C: the logs go there.
4. Hold B open while it comes: it waits 3 blocks away until you close it. Replace B with a trapped chest that has a redstone lamp beside it: the lamp lights while the golem has it open. **m3g3**
5. Put C in an oak plank room with an oak door: the golem opens the door, goes in and shuts it.
6. Right-click a carrying golem with an empty hand: it throws the stack at you.
   - Hit one: it flashes, runs and stops sorting for a while.
   - Kill one: 1-3 copper ingots and its cargo drop.
   - Put a lead on one: it follows you and stops sorting.
7. `/summon minecraft:copper_golem ~ ~ ~3 {next_weather_age:1L}`: it turns exposed within a second. `/summon minecraft:copper_golem ~ ~ ~5 {weather_state:"weathered",next_weather_age:1L}`: it turns green, and its steps and hurt sounds turn rusty. **m3g4**
8. Use honeycomb on a weathered golem: you hear the wax sound. Then use the axe:
   - First use: a scrape sound, and it stays weathered.
   - Next three uses: exposed, then copper, then nothing.
9. `/summon minecraft:copper_golem ~ ~ ~4 {weather_state:"oxidized"}` and step back. Within about 30 s it becomes a green statue in a pose, facing its way. **m3g5**
   - An empty hand cycles the pose four times, and a comparator reads 1-4.
   - Three axe uses scrape it to copper, and the fourth wakes the golem with a chime. **m3g6**
   - A pickaxe drops it in its pose. The kit statue places facing you.
10. `/summon minecraft:copper_golem ~ ~ ~3 {equipment:{saddle:{id:"minecraft:poppy",count:1}}}`: a poppy sits on its antenna, and shears snip it off.
    - Optional: an iron golem beside a copper golem by day gives it a poppy within about 10 minutes. **m3g7**
11. Save and quit with a carrying golem, a green one, a waxed one and a statue, then load. All are unchanged, and the carrier finishes its trip.
12. Friend window: the friend sees the walking, carrying, lids, head-shakes and colours at the same moments. The friend's empty hand gets the stack, and the friend's axe scrapes. **m3g8-host**, **m3g8-friend**
13. `/summon minecraft:lightning_bolt` on a green golem: it turns weathered.

```
IW4L_MINECRAFT_KIT=minecraft:copper_golem_spawn_egg*8,minecraft:carved_pumpkin*16,minecraft:copper_block*8,minecraft:exposed_copper*4,minecraft:weathered_copper*4,minecraft:oxidized_copper*4,minecraft:waxed_copper_block*4,minecraft:copper_chest*4,minecraft:chest*8,minecraft:trapped_chest*2,minecraft:cobblestone*64,minecraft:oak_log*32,minecraft:apple*16,minecraft:honeycomb*16,minecraft:iron_axe,minecraft:shears,minecraft:comparator*2,minecraft:redstone*16,minecraft:redstone_lamp*2,minecraft:iron_golem_spawn_egg*2,minecraft:snow_block*4,minecraft:lead*2,minecraft:oak_door*2,minecraft:copper_golem_statue*2,minecraft:oak_planks*64,minecraft:iron_sword,minecraft:cooked_beef*16
```
27 stacks, all ids checked in the 26.3 jar's `assets/minecraft/items`.

### Depends on
- **D0 d0a**, all of it. d0b is not needed.
- **E2 `cat-chests`** (segment e2a, merged before m3c): `chest_open_count`.
  - It reads `level.chest_openers`, which survival fills only for trapped chests (`level.rs:383-385`). If E2 did not make plain and copper chests count, `copper-golems-sorting` commit 1 does.
- **W (P7)**: `container_open`/`container_close`, `shear` and `block_change`. Golem opens pass `None` as the source.
- **M3-1 `lightning-strikes`** (segment m3a): the `thunder_hit` hook for `copper-golems-care`.
- **E1**: none.

### Decisions to record (recommended)
1. **MISC without changing D0's `FamilySpawn`.** `FamilySpawn.category` stays a `MobCategory`: making it an `Option` would break every other track's rows. The row says `MobCategory::Creature`, which acts like vanilla's MISC everywhere the world crate reads it. The golem is always persistent, so the census never counts it, and no biome spawns it. `Creature` is friendly, so the golem is allowed in peaceful and a spawner's `custom_spawn_rules` (`level/spawning.rs:485`) do not take it for a monster, as `None` would. It may also spawn far from players, as MISC may. The entity side answers `category` "misc", which drives the census, `seen_all` and the despawn rule.
2. **The idle spin is drawn on the host** with the golem's own animation random, then sent in `FamilyShow`. Server draws stay vanilla's, and every viewer sees the same spin.
3. **Spawn sound:** a tag without `next_weather_age` (or -1) sets `greet` in `read`, and the first tick plays `entity.copper_golem.spawn`. Builds do not play it a second time.
4. **Cargo and poppy for friends** stay in the look tag. A friend's copy is made again at a pickup or put-down, at most twice per trip. Bit 0 switches the walk animation at once.
5. **The throw-back** is stored by `interact` and spawned on the next tick through `World::spawn_item`, with vanilla's position and velocity.
6. **Golem lids** are compared each host tick under golem keys. A golem that stops or dies with a chest open lets it shut on the next tick, like vanilla's recheck.

---

## M3-5. Zombie horses and skeleton horses (groups `undead-horses-live`, `undead-horses-ride`, `undead-horses-drawn`; cow store, hit key 11)

Mob list rows 40 and 41: their eggs do nothing, `/summon` leaves a dormant tag, and natural zombie horsemen (plains, savanna) stay dormant.

**Not registered.** Both extend `HorseKind` in the cow store, so they take no `night` byte (80-95) and no `TerrainFamily`. They follow D0-3's "The one exception": only M3 edits the cow-store lines. A few more cow-store one-liners outside that list are needed (decision 1).

The skeleton trap (its goal, its 18000-tick end and the four horsemen) is M3-6. This section only reads and saves the trap's two fields, so a trap horse from a storm or `/summon` lives from `undead-horses-live` on.

### What exists today (`C:\Dev\mashup-crafting`, `minecraft-survival` at `d1a3d028`)
- **Horse state.**
  - `HorseKind` (`third_party/minecraftoss/entities/src/horse.rs:11-17`) covers horse, donkey, mule, llama and trader llama.
  - Per-kind functions `:20-161`: `is_chested` `:45` is true for every non-horse kind, `wears_armor` `:61`, `passenger_attachment` `:65`, `food_tag`/`tempt_tag`/`eats` `:93-110`, `base_attributes` `:145`, `sound_family` `:153`.
  - `Feeding::of` `:281` already has the red mushroom (heal 3, temper 3).
- **`world/equines.rs`.**
  - `horse_event` `:42`, `horse_movement_sounds` `:67`, `ride_tick` `:135` (`travel_water` at `:182`).
  - `tick_trader_despawn` `:227`, with `despawned` `:106` covering trader llamas only.
  - The tempt fallback `:322-333`.
  - `horse_interact` `:630` (golden dandelion `:670`), `horse_rides_on_use` `:848`.
- **`cow_ai.rs`.**
  - `Species::horse` `:64` (tempt `HorseTempt`, which also accepts `main_hand_horse_tempt`).
  - `CowGoalContext` `:85`, whose only constructor is `world.rs:7809`; `CowGoalEffect` `:124`.
  - `HORSE_GOAL_NAMES` `:446` and `registered_horse_goals` `:547`, where `PanicGoal` stands in for `MountPanicGoal`.
- **Vendored `world.rs`, the cow store.**
  - `goal_names` `:930` and `spawn_horse` `:5255` fall back to the horse set.
  - `take_deaths` `:4694` gives adults their kind's table, animal experience and the worn saddle and armour.
  - `check_despawn` `:5904` uses `CREATURE`.
  - Cow tick sites: `water_breathing` `:7668`, ambient voice `:7690`, `travel_water` `:7744`/`:7928`, `Buck` arm `:7858`, `Stand` voice `:7861-7875`, `push_entities` `:7964`, `tick_age_and_love` `:8015`, `tick_trader_despawn` `:8050`.
  - `burn_undead` `:859` is private to `world`; the zombie's call is at `:6311-6313`.
- **`world/riding.rs`** is generic over `vehicle_horse` (`:213`).
  - `controlling_mob` `:259` lets a zombie with AI, or a skeleton, steer a horse, so both horsemen already work once the horse lives.
  - The jump sound is `entity.horse.jump` at `:698`.
- **Cow-store lines that treat every horse as a cow:**
  - `world/witches.rs:533-535` `mob_effects_mut` returns `(effects, false, false)`, so nothing in the store is undead.
  - `:668-669` `heal_mob` caps at 10 health, which is wrong for every horse.
  - `world/leashes.rs:338` accepts every cow for a lead.
- **Spawning.**
  - Zombie horse rows exist: `natural_spawner.rs:315`, `:334` (`Rules::Monster`), `:365`, `:367`; category Monster at `spawning.rs:99`.
  - Its finalize `spawning.rs:1274-1301` adds the iron-spear zombie **for every reason**. Summons pass `natural = false` (`:928`), and `hatch_egg` then splits the rider off (`spawn_unmounted`).
  - The skeleton horse has no row.
  - Helpers: `set_attribute_base` `natural_spawner.rs:1166`, `check_spawn_rules` `:761` (animal rules).
- **Terrain, cow-store lines.**
  - `server_mobs.rs`: `SIMULATED_KINDS` `:433`, `VEHICLES` `:489`, horse load arm `:618-664`, horse save `:1707-1748` (writes `ChestedHorse` for non-horses). The test at `:2433-2434` asserts that a zombie horse jockey stays dormant.
  - `mob_tracking.rs`: `tracking_range` `:289`, horse `MobShow.ridden` `:396`/`:596`, `VOLATILE` `:15`.
  - `horse_render.rs`: `texture_ids` `:24`, `saddle_texture` `:43`, `all_textures` `:72`, ears `:258-269`, baby `:278`, `layer_scale` `:329`. `append_horses` `:339` draws every non-llama horse.
  - `spawner_display.rs` `:19`/`:212`, `mob_actions.rs` horse arm `:176-190`, `HATCHING_EGGS` `server.rs:7717` (50).
- **Already right.**
  - `wither::is_undead` (`wither.rs:73-91`) and `dormant.rs` `ALLOWED_IN_PEACEFUL` (`:10`).
  - Shadow 0.75 (`client_mobs.rs:1347`) and MW2 hits through `MobHit::Cow`.
  - `mount_inventory.rs:31-55` is generic through `saddleable`/`wears_armor`.
  - `crate::blaze::triangle` (`blaze.rs:62`).
- **Fork limits that apply here.** Spears have no use (`docs/MINECRAFT-SURVIVAL.md:456`), and enchanted spawn equipment is `note_unsupported` (`spawning.rs:974-981`).

### Vanilla 26.3 (javap this round)
- **Types.**
  - `SKELETON_HORSE` is `CREATURE`; `ZOMBIE_HORSE` is `MONSTER` (allowed in peaceful). Both: `sized(1.3964844, 1.6)`, `eyeHeight(1.52)`, passenger attachment `1.31875`, tracking range 10, not fire immune.
  - Babies: `BABY_DIMENSIONS` is the type size with the passenger point at `height - 0.25`, scaled 0.7.
- **Attributes** (`createBaseHorseAttributes` plus their own).
  - Shared: jump 0.7, step 1, safe fall 6, fall multiplier 0.5, follow 16.
  - Skeleton horse: health 15, speed `0.20000000298023224`.
  - Zombie horse: health 25, speed `0.22499999403953552`.
- **`randomizeAttributes`**, run in `AbstractHorse.finalizeSpawn` after `AgeableMobGroupData(0.2)` when it has no group.
  - Skeleton horse: jump only, `0.4000000059604645 + 3 × nextDouble × 0.2`.
  - Zombie horse: jump `0.5 + 3 × nextDouble × 0.0666…`, then speed `(9 + 3 × nextDouble) / 42.16`. The fork already does this at `spawning.rs:1290-1295`.
- **`AbstractHorse` defaults that both keep.**
  - Cluster 6, max temper 100, volume 0.8, ambient every 400 ticks.
  - They rear (one hurt in three) and graze (`canEatGrass` true).
  - `canMate` is false, so they never breed. Experience is 1-3.
- **Foals.** `canAgeUp()` is false, so `AgeableMob.aiStep` never counts a foal up.
- **Tags.**
  - Both: `#undead` (inverted healing and harming, poison and regeneration refused, they breathe underwater), `#can_equip_saddle`, `#cannot_be_age_locked`.
  - Zombie horse only: `#burn_in_daylight`, `#can_wear_horse_armor`, `#can_float_while_ridden`, `#dismounts_underwater`.
  - `#zombie_horse_food` holds `red_mushroom` only.
- **Goals.**
  - `registerGoals`: 1 `RunAroundLikeCrazyGoal(1.2)`, 2 `BreedGoal`, 4 `FollowParentGoal`, 6 `WaterAvoidingRandomStrollGoal(0.7)`, 7 `LookAtPlayerGoal(6)`, 8 `RandomLookAroundGoal`, 9 `RandomStandGoal`.
  - Skeleton horse's `addBehaviourGoals`: **empty**. It has no float (it sinks), no panic and no tempt.
  - Zombie horse's `addBehaviourGoals`: 0 `FloatGoal`, 3 `TemptGoal(1.25, #zombie_horse_food)`.
  - `setTrap(true)` and `setTrap(false)` add and remove `SkeletonTrapGoal` (priority 1, no flags).
- **`SkeletonHorse`.**
  - Saves `SkeletonTrap` and `SkeletonTrapTime`.
  - `aiStep`: when it is not persistent and is a trap, `trapTime++ >= 18000` calls `discard()`, with no death.
  - `getWaterSlowDown` is 0.96.
  - `mobInteract` on a wild one returns `PASS`. A tame adult (`AbstractHorse.mobInteract`) opens its inventory on a crouching click, takes a saddle, and **otherwise mounts, food included**. It takes no armour.
  - Sounds: `ambient` (`ambient_water` with its eyes in water), `hurt`, `death`, no angry or eat sound.
  - Swim sound: off the ground, `swim`. On the ground and carrying someone, `gallopSoundCounter++` gives `gallop_water` when it is above 5 and divisible by 3, `swim` when above 5, and `step_water` at 5 or less. On the ground and carrying no one, `step_water`.
  - Swim volume: 0.3 on the ground, `min(0.1, v × 25)` off it. Jumping in water plays `jump_water` at 0.4.
- **`ZombieHorse`.**
  - `interact` always calls `setPersistenceRequired()`. `removeWhenFarAway` is true.
  - `canBeLeashed` is `isTamed || !isMobControlled` (no mob in the first seat).
  - `chargeSpeedModifier` 1.4. `sunProtectionSlot` is BODY: horse armour takes the sun.
  - `mobInteract`:
    - an adult, tame horse with a crouching click, or a horse carrying someone, goes to the horse's `mobInteract`;
    - otherwise a red mushroom calls `fedFood` (heal 3, temper +3, `entity.zombie_horse.eat`, the mouth opens);
    - any other item on a wild one calls `makeMad` (it rears and sounds `entity.zombie_horse.angry`);
    - an empty hand, or a tame horse with an item, goes to `AbstractHorse.mobInteract`: it mounts (even a wild one, tamed by temper) and takes a saddle and horse armour.
  - `finalizeSpawn`: **only for `NATURAL`**, a `JOCKEY` zombie finalized with the same reason gets an `iron_spear` and rides; then `AbstractHorse.finalizeSpawn` runs.
- **The skeleton trap** (`SkeletonTrapGoal` and where trap horses come from): M3-6.
- **Spawning.**
  - Both: `ON_GROUND`, `MOTION_BLOCKING_NO_LEAVES`.
  - Zombie horse: `checkMonsterSpawnRules`. Weight 5, count 1 in `plains`, `sunflower_plains`, `snowy_plains`, `savanna`, `savanna_plateau`, `windswept_savanna`.
  - Skeleton horse: a spawner reason needs `ignoresLightRequirements` or `getRawBrightness(pos, 0) > 8`; otherwise `checkAnimalSpawnRules`. No biome spawns it.
  - `SummonCommand` allows the zombie horse in peaceful.
- **Loot.**
  - `entities/zombie_horse`: rotten flesh `uniform(2, 3)`.
  - `entities/skeleton_horse`: bone `uniform(0, 2)`.
  - Both add Looting +0-1. A foal drops nothing.
- **Entity events:** 7 (hearts) and 6 (smoke).
- **Client.**
  - `UndeadHorseRenderer`: shadow 0.75, `HorseModel` (adult) and `BabyHorseModel` (foal).
  - Textures `entity/horse/horse_skeleton(_baby)` and `horse_zombie(_baby)`.
  - Layers: `HORSE_BODY` on `UNDEAD_HORSE_ARMOR`, and the `ZOMBIE_HORSE_SADDLE`/`SKELETON_HORSE_SADDLE` equipment layer on `EquineSaddleModel` (`entity/equipment/<layer>/saddle`).
  - `LayerDefinitions` bakes the undead body, armour and saddle **unscaled**, while the horse's are scaled 1.1, so undead horses are about 9% smaller.
  - No keyframe class.

### Plan
- **Files.** Nothing is registered.
  - The trap's terrain file, `families/night/skeleton_trap.rs`, is M3-6's.
  - The skeleton horse's `FamilySpawn` row (D0-4) goes in `third_party/minecraftoss/world/src/natural_spawner/families/night.rs`.
  - Everything else is arms and helpers in cow-store files.
- **Registry hooks.**
  - Only D0-4's row (`rule`, `finalize`).
  - D0-5.1's egg survey is unaffected: the cow-store kinds go into `HATCHING_EGGS` itself.
  - No `FamilyShow`, no `MobHit::Family`.
- **Exception-list edits.**
  - `horse.rs`: both variants in every per-kind function, `is_undead()`, and `HorseState.trap`/`trap_time` (read and saved here; M3-6 adds `sprung` and the expired trap in `despawned()`). `is_chested` becomes donkey, mule and the llamas only.
  - `equines.rs`.
  - `server_mobs.rs`: `SIMULATED_KINDS` (alphabetical), `VEHICLES`, the load pattern (adding `SkeletonTrap`/`SkeletonTrapTime`), and the save arm (neither `Variant` nor `ChestedHorse`; the trap keys for the skeleton horse).
  - `tracking_range`: the 10 group, next to `"horse" | "donkey"`.
  - `HATCHING_EGGS` (50 → 52, `skeleton_horse` and `zombie_horse` in sorted order), `spawner_display.rs`, `horse_render.rs`, the `mob_actions.rs` horse arm, and `riding.rs:698`.
- **One-liners outside the list** (decision 1). Each is the cow store's own line, next to a related arm and never at a list's end. The lines marked (M3-6) land in `skeleton-trap`:
  - (M3-6) `mob_tracking.rs` `VOLATILE`: `"SkeletonTrapTime",` after `"DespawnDelay"` (`:45`), with `[&str; 37]` becoming 38. Otherwise every friend respawns a ticking trap horse every `LOOK_TICKS`.
  - Vendored `world.rs`:
    - `:930` and `:5261`: arms for the undead goal sets;
    - `:7668`: `water_breathing` gains `|| …is_undead()`;
    - `:7690` and the `Stand` arms `:7861-7875`: `equines::ambient_voice`;
    - `:7744`, `:7928` and `equines.rs:182`: `travel_water` becomes `travel_water_slowed(.., equines::water_slow_down(horse))`. These neighbour E1's `horse-fall` lines `:7748`, `:7933` and `equines.rs:190`, so re-check after E1;
    - (M3-6) `CowGoalEffect::SpringTrap => horse_outputs.trap = true,` beside `Buck` (`:7858`);
    - after `:7964`: `self.burn_zombie_horse(index, &*world);`;
    - `:8015`: `tick_age_and_love_when(!undead)`;
    - (M3-6) after `:8050`: `entity.tick_skeleton_trap();`;
    - `:5904`: `CREATURE` becomes `e.despawn_rule()` (zombie horse `(128, true, true)`).
  - `cow.rs` `tick_age_and_love_when` and `movement.rs` `travel_water_slowed`, each beside its sibling.
  - `cow_ai.rs`:
    - the two goal lists and `registered_undead_horse_goals(kind, trap)`, beside the horse's;
    - (M3-6) `SkeletonTrapGoal`;
    - (M3-6) `CowGoalContext.trap` after `crazy`;
    - (M3-6) `CowGoalEffect::SpringTrap`;
    - `TemptItems::ZombieHorseFood`, which reads only `ctx.tempters`.
  - `witches.rs:533-535` returns `(.., undead, undead)`. `:668-669` uses `e.cow.max_health`, which also fixes horses.
  - `leashes.rs:338` adds `&& self.equine_leashable(e.id)`.
  - `spawning.rs:1276`: the rider's `if let` gains `.filter(|_| natural)`.
  - (M3-6) Terrain `server.rs`: `self.spring_skeleton_traps();` after `self.cure_zombie_villagers(players);` (`:1118`), and `fn spring_skeleton_traps` right after `fn cure_zombie_villagers` (`:1178`), which passes the fields to `crate::families::night::skeleton_trap::spring` (M3-6).
- **Multiplayer.**
  - No wire change, no `PROTOCOL` note.
  - The friend loads both kinds from the look tag through the horse arm of `spawn_saved`: coat, foal, saddle, armour, tame and trap state.
  - `MobShow.ridden` comes from the existing horse show.
  - The only per-tick saved field is `SkeletonTrapTime` (made volatile in M3-6).
  - The trap's mobs are ordinary tracked spawns. No keyframes.

#### Commit groups
| # | Group key | Commits |
|---|---|---|
| 1 | `undead-horses-live` | 1. "Minecraft zombie horses and skeleton horses live, save and hatch like vanilla" 2. "Minecraft undead horses burn, breathe underwater and heal from harming like vanilla" |
| 2 | `undead-horses-ride` | 1. "Minecraft zombie horses are tamed by riding, eat red mushrooms and wear horse armour" 2. "Minecraft skeleton horses obey only once tame and splash about underwater" |
| 3 | `undead-horses-drawn` | "Minecraft zombie horses and skeleton horses are drawn like vanilla" |

- **1.1 Live, saved and hatched.**
  - `HorseKind` numbers (above).
  - Sound families `zombie_horse` and `skeleton_horse` (no foal sounds); `horse_event` rows for the zombie horse's angry and eat.
  - Both goal lists; saving, loading, eggs, `/summon` and natural jockeys.
  - The skeleton horse row:
    - 1.3964844 × 1.6, `OnGround`, `MotionBlockingNoLeaves`, `Rules::Family { creature: true }`, `Walk::Animal`, kept, cluster 6;
    - `Creature`, follow range 16, `baby_chance: Some(0.2)`;
    - `rule`: a spawner needs `raw_brightness(pos, 0) > 8`, otherwise `check_spawn_rules`;
    - `finalize`: the jump roll.
  - The natural-only rider; the despawn rule; the jockey test now loads it mounted.
  - Rows 40 and 41 become `live`.
- **1.2 Undead rules.**
  - `burn_zombie_horse`: `burn_undead(.., monsters_burn && ZombieHorse, body_item.map(|_| false))`. Horse armour has no `max_damage` in 26.3, so it blocks the sun without wearing.
  - Undead breathing. The skeleton horse's drag is 0.96, and without `FloatGoal` it sinks.
  - Inverted potions, and poison and regeneration refused.
  - Foals never grow on their own, and no golden-dandelion lock.
- **2.1 Zombie horse use.**
  - The `mob_actions` horse arm's first line, `world.note_horse_interaction(id)`, makes it persistent.
  - `horse_interact` and `horse_rides_on_use` follow `ZombieHorse.mobInteract`.
  - `equine_leashable`.
  - The horseman steers through the existing `controlling_mob`, so it gets a test only.
- **2.2 Skeleton horse use and water.**
  - A wild one passes on everything. A tame adult takes a saddle and otherwise mounts.
  - `ambient_water`, `swim`/`step_water`/`gallop_water` with their volumes in `horse_movement_sounds`, and `jump_water` at `riding.rs:698`.
  - The gallop counter is E1's `gallop_counter`. Without E1's `horse-gallop`, play `step_water`/`swim` only and leave `gallop_water` for later.
  - `can_gallop` is true for both.
- **3.1 Drawn.**
  - The undead coats (no markings) and their saddles in `texture_ids`, `saddle_texture` and `all_textures`.
  - The horse ear and foal arms take both kinds.
  - `layer_scale` is 1.0 for undead adults (body, armour, saddle).
  - Spawner cages; rows become `drawn`.

### Tests
- **1.1**
  - `world::equines::tests::undead_horses_take_their_sizes_health_speed_jump_and_voices`.
  - `cow_ai::tests::undead_horses_run_vanilla_s_goal_lists`.
  - `undead_horses_never_breed`.
  - `zombie_horses_despawn_far_away_and_stay_on_peaceful_and_skeleton_horses_stay`.
  - `undead_horses_drop_flesh_and_bones_and_their_gear` (and nothing from a foal).
  - World: `natural_spawner::families::night::tests::a_zombie_horse_carries_a_spear_zombie_only_when_it_spawns_naturally`, `a_skeleton_horse_rolls_its_jump_and_spawns_from_spawners_only_in_light`.
  - Terrain: `server_mobs::tests::undead_horses_save_and_load_with_their_trap_tame_saddle_and_armour`; the egg survey grows by two; the jockey test is updated.
- **1.2**
  - `a_zombie_horse_burns_by_day_unless_it_wears_armour`.
  - `a_skeleton_horse_sinks_walks_the_bottom_faster_and_never_drowns`.
  - `harming_heals_an_undead_horse_to_its_full_health_and_poison_does_not_take`.
  - `an_undead_foal_never_grows_up_on_its_own_or_takes_a_golden_dandelion`.
- **2.1**
  - `a_zombie_horse_eats_red_mushrooms_rears_at_other_items_and_is_tamed_by_riding`.
  - `any_click_keeps_a_zombie_horse`.
  - `a_zombie_horse_wears_horse_armour_and_a_skeleton_horse_does_not`.
  - `a_zombie_horseman_steers_its_horse_and_its_horse_refuses_a_lead`.
  - `golden_carrots_do_not_tempt_a_zombie_horse`.
- **2.2**
  - `a_wild_skeleton_horse_ignores_every_click_and_a_tame_one_is_saddled_and_ridden_even_with_wheat`.
  - `a_skeleton_horse_sounds_like_water_on_the_lake_floor`.
  - `a_skeleton_horse_jumping_in_water_plays_jump_water`.
- **3.1**
  - `horse_render::tests::undead_horses_use_the_unscaled_horse_model_with_their_own_coats_and_saddles`.
  - `an_undead_foal_uses_the_baby_horse_model`.
  - `mob_tracking::tests::a_friend_sees_a_skeleton_trap_horse_and_a_zombie_horseman_from_their_look_tags` (the ticking `SkeletonTrapTime` part comes with M3-6, which makes it volatile).
  - The spawner cage test covers both.

### Owner's steps
Setup:
- Release build, survival, **Normal**.
- Flat grassland in the Overworld, with a lake at least 3 deep nearby.
- `/time set midnight` (zombie horses burn by day), and torches around you.
- The kit below.

Steps:
1. Use a zombie horse egg on the ground 6 blocks away. A grey-green rotting horse appears without a rider. It wanders, grazes and sometimes rears with a hollow groan, and it never attacks. **m3u1_zombie_horse**
2. Hold red mushrooms: it follows you. Hold a golden carrot instead: it ignores you.
3. Right-click it with a bone: it rears with an angry groan.
4. Right-click it with a red mushroom: it chews and the mushroom is used.
5. Right-click it with an empty hand: you mount it, and it throws you off a few times with a puff of smoke (E1's particles, in e1a).
6. Feed it red mushrooms between tries until it stays calm and shows hearts: it is tamed. **m3u2_zombie_horse_tamed**
7. Saddle it, ride it, and jump with the jump bar.
8. Right-click it with the iron horse armour: the armour shows, slightly smaller than on a horse. Crouch-right-click it: swap in the diamond armour.
9. `/time set noon` in the open, and hatch a second zombie horse beside the armoured one. The new one catches fire within seconds and keeps burning; the armoured one never burns. **m3u3_zombie_horse_sun**
10. `/time set midnight`. Hatch a fresh zombie horse without touching it, ride 140 blocks away and come back: it is gone, and the tamed one is still there.
11. `/summon minecraft:zombie_horse ~ ~ ~8 {Passengers:[{id:"minecraft:zombie",equipment:{mainhand:{id:"minecraft:iron_spear",count:1}}}]}`. A spear-holding zombie rides it straight at you and hits you; the horse goes where the zombie steers. **m3u4_zombie_horseman**
12. Right-click that horse with a lead: nothing attaches. Kill the zombie: the horse stays, wanders and now takes the lead.
13. Use a skeleton horse egg. A bony white horse appears. Right-click it with an empty hand, then the saddle, then wheat: nothing happens, and you cannot get on. **m3u5_skeleton_horse**
14. `/summon minecraft:skeleton_horse ~ ~ ~4 {Tame:1b}`. Saddle it, then right-click it holding wheat: you get on, and it does not eat.
15. Ride into the lake. It sinks and walks the bottom at a good pace with watery steps, and you stay on. Leave before your own air runs out: the horse has lost no health. **m3u6_skeleton_horse_underwater**
16. Hit a skeleton horse twice with the sword, then splash it with harming: it heals. Splash it with healing: it is hurt.
17. Splash the zombie horse with poison: no swirls, and no damage.
18. Kill the horses: a zombie horse drops 2-3 rotten flesh (plus its saddle and armour if worn), a skeleton horse 0-2 bones. Shoot one of each with the M4A1: they flash and die.
19. Save and quit, then load. The tamed horses keep their saddles, armour and taming.
20. In creative (`/gamemode creative`), place a spawner on grass by torchlight and use the skeleton horse egg on it. A small skeleton horse turns in the cage, and skeleton horses appear around it. **m3u8_spawner_cage** Then `/gamemode survival`.
21. Optional: spend a night in plains or savanna. Now and then a zombie horseman with a spear appears.

Friend (two windows):

22. The host runs `/summon minecraft:skeleton_horse ~ ~ ~3 {Tame:1b}`. The friend saddles it and rides it into the lake, and the host sees the friend on it under water. **m3u9_friend_skeleton_horse-host** and **m3u9_friend_skeleton_horse-friend**

### Kit
```
IW4L_MINECRAFT_KIT=minecraft:zombie_horse_spawn_egg*8,minecraft:skeleton_horse_spawn_egg*8,minecraft:red_mushroom*32,minecraft:saddle,minecraft:saddle,minecraft:iron_horse_armor,minecraft:diamond_horse_armor,minecraft:leather_horse_armor,minecraft:lead*4,minecraft:bone*8,minecraft:wheat*16,minecraft:golden_carrot*8,minecraft:splash_potion@harming,minecraft:splash_potion@healing,minecraft:splash_potion@poison,minecraft:spawner,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:shield,minecraft:water_bucket,minecraft:cobblestone*64,minecraft:torch*64,minecraft:cooked_beef*32,minecraft:golden_apple*8,minecraft:diamond_chestplate,minecraft:iron_helmet
```
27 stacks (saddles do not stack, so each one is its own entry). Every id has a 26.3 item model (`assets/minecraft/items`). The friend needs the same kit.

### Depends on
- **D0 d0a:** the `night` spawn-row file and `FamilySpawn` (`registry-spawning`), the terrain folder `families/night/`, and the egg-survey change (`registry-client`). d0b is not needed.
- **M3-1 lightning (P8, segments m3a and m3b):** its thunder roll leaves `{SkeletonTrap:1b,Age:0}` horse tags, which load live from `undead-horses-live` on. The trap itself is M3-6.
- **E1:**
  - `particles-events`: the 7/6 hearts and smoke need no row ("any other type").
  - `horse-gallop`: `can_gallop`, and the counter used in 2.2.
  - `horse-fall`, `vehicle-hearts` and dyed leather apply to the cow store as they are.
  - Segment m3d starts after e1a is merged (M30), so these groups add the undead arms to E1's code.
- **E2 and W:** no change needed.
- **M3 `camels`:** runs next in the same track and edits the same `HorseKind` matches, in order.

### Decisions for the track log
1. **Cow-store one-liners outside D0-3's exception list** (the list in the plan) are allowed. Each is the cow store's own line next to a related arm, never at a list's end, and each is recorded in the log. Three `travel_water` lines (`world.rs:7744`, `:7928`, `equines.rs:182`) sit next to E1's `horse-fall` lines; m3d starts after e1a is merged, so re-check them against E1's code.
2. **The zombie horseman's rider** is added only for natural spawns, as vanilla does: the `if let` at `spawning.rs:1276` gains `.filter(|_| natural)`. Eggs and `/summon` give a riderless horse. Today's code adds the spear zombie on every spawn, and an egg then splits it off.
3. **The skeleton horse's `gallop_water` sound** uses the gallop counter from E1's `horse-gallop`, which is in e1a and merged before m3d.
4. **Smaller deviations** go to "Left for later": `/summon` of a zombie horse in peaceful stays refused, `#dismounts_underwater` and `#can_float_while_ridden` stay unported for all horses, the skeleton horse's path search keeps floating, and spear charges stay out.

---

## M3-6. The skeleton trap (group `skeleton-trap`; cow store, hit key 11)

This is the last part of mob list row 41. A trap horse comes from two places: a thunderstorm's trap roll (M3-1, `lightning-weather`) leaves a `{SkeletonTrap:1b,Age:0}` skeleton horse, and so does `/summon minecraft:skeleton_horse ~ ~ ~ {SkeletonTrap:1b}`. From `undead-horses-live` on, a trap horse lives and saves and loads its two fields, but it never springs. This group adds the trap goal, the 18000-tick end and the four horsemen. It runs after the undead horse groups in segment m3d and uses M3-1's visual-only bolt.

### Vanilla 26.3 (javap this round)
- **`SkeletonHorse`:**
  - It saves `SkeletonTrap` and `SkeletonTrapTime`.
  - `setTrap(true)` adds `SkeletonTrapGoal` at priority 1 (no flags), and `setTrap(false)` removes it.
  - In `aiStep`, a trap horse that is not persistent counts `trapTime++`. At `>= 18000` it calls `discard()`: no death, no loot.
- **`SkeletonTrapGoal.canUse`:** `hasNearbyAlivePlayer(x, y, z, 10)`, a living non-spectator player with distance² < 100. Creative players count.
- **`tick`** (all in the same tick):
  1. `setTrap(false)`, `setTamed(true)`, `setAge(0)`.
  2. A visual-only bolt (TRIGGERED) at the horse.
  3. A finalized `TRIGGERED` skeleton rides the trap horse. It gets:
     - `setInvulnerableTime(60)` and persistence;
     - an `iron_helmet` if bareheaded;
     - its main hand and head enchanted again: `ENCHANTMENTS` cleared, then provider `mob_spawn_equipment` (`by_cost_with_difficulty`, cost `randomBetweenInclusive(5, 5 + (int)(special × 17))`, from `#on_mob_spawn_equipment`).
  4. Three times: a finalized `TRIGGERED` skeleton horse (invulnerable for 60, persistent, tame, age 0) with such a skeleton. It is pushed by `(triangle(0, 1.1485), 0, triangle(0, 1.1485))` from the trap horse's random.
- **Where traps come from:** `ServerLevel.tickThunder` (M3-1). It needs all of these:
  - thunder, and `nextInt(100000) == 0` per chunk;
  - the `spawn_mobs` rule on;
  - `nextDouble() < effectiveDifficulty × 0.01`;
  - no `#lightning_rods` block below.

  Then `SKELETON_HORSE.create(EVENT)` (no finalize) with `setTrap(true)` and `setAge(0)` goes at the block position, and the bolt is visual-only.

### Plan
- **Files.** New `crates/minecraft_terrain/src/families/night/skeleton_trap.rs`, declared by `pub(crate) mod skeleton_trap;` in M3's own `families/night.rs`, so terrain `lib.rs` is untouched. Nothing is registered.
- **Cow-store lines** (the lines marked (M3-6) in M3-5's list):
  - `horse.rs`: `HorseState.sprung: Option<f32>`, and `despawned()` also covers an expired trap.
  - `mob_tracking.rs` `VOLATILE`: `"SkeletonTrapTime",` after `"DespawnDelay"` (`:45`), so `[&str; 37]` becomes 38 (logged). Without it every friend respawns a ticking trap horse every `LOOK_TICKS`.
  - `cow_ai.rs`:
    - `SkeletonTrapGoal`, whose `can_start` needs `ctx.trap` and a living non-spectator in `ctx.players` with distance² < 100. It pushes `CowGoalEffect::SpringTrap`.
    - `CowGoalContext.trap` after `crazy`.
    - The trap goal in `registered_undead_horse_goals(kind, trap)`.
  - Vendored `world.rs`:
    - `CowGoalEffect::SpringTrap => horse_outputs.trap = true,` beside `Buck` (`:7858`);
    - `entity.tick_skeleton_trap();` after `:8050`.
  - Terrain `server.rs`: `self.spring_skeleton_traps();` after `self.cure_zombie_villagers(players);` (`:1118`), and a short `fn spring_skeleton_traps(&mut self)` right after `fn cure_zombie_villagers` (`:1178`). It calls `crate::families::night::skeleton_trap::spring(&mut self.level, &mut self.mobs)`, extends `self.mob_tags` with the riders `spring` spawned onto existing trap horses, and hands each horseman tag `spring` returns to `self.add_saved_entity`. `spring` cannot take `&mut ServerSim`: its fields and `add_saved_entity` are private to `server.rs`.
- **Springing.**
  - `horse_outputs` sets `trap = false`, `tame = true`, `age.set(0)` and `sprung = Some(yaw)`, and drops the goal with `retain_ids`.
  - `tick_skeleton_trap` runs the 18000 clock, but not for a persistent horse.
  - `equines.rs` gets `take_sprung_traps() -> Vec<(u64, DVec3, f32)>` and `set_hurt_cooldown(id, ticks)`, which goes through the private `damage_state_mut`.
  - `spring(level: &mut Level, mobs: &mut EntityWorld) -> Sprung { riders: Vec<(u64, Tag)>, horsemen: Vec<Tag> }` does this for each sprung trap:
    1. M3-1's visual-only bolt at the horse (`level.add_lightning(pos, true)`). Host and friends see and hear it; it lights nothing and hits nothing.
    2. `level.summon_mob("minecraft:skeleton", at, None, yaw)`, plus an `iron_helmet` if bareheaded and `PersistenceRequired:1b`. Then `server_mobs::spawn_saved_all(mobs, &tag)` (its `(id, tag)` pairs go into `riders`), and `mobs.force_riding` onto the trap horse with a hurt cooldown of 60.
    3. Three times: `summon_mob("minecraft:skeleton_horse", ..)` (the row's finalize), plus `Tame:1b`, `Age:0`, `PersistenceRequired:1b` and `Motion:[triangle, 0, triangle]` from the trap horse's random. The skeleton goes in as `Passengers`, and the whole tag into `horsemen`, which `spring_skeleton_traps` hands to `add_saved_entity`. Each horseman, horse and skeleton, gets the hurt cooldown of 60 too: `spring_skeleton_traps` finds both after the add by their UUIDs (`mob_with_uuid`, `riding.rs:752`) and calls `set_hurt_cooldown`.
  - On peaceful no skeletons come: `summon_mob` refuses monsters, where vanilla's appear and vanish the next tick.
  - Rows 40 and 41 become `done`. The steps and kit go into the log.
- **Multiplayer.** No wire change. The horsemen are ordinary tracked spawns, and the bolt reaches friends through `EntitySnapshot.lightning` (M3-1).

### Tests
- `world::equines::tests::a_trap_horse_springs_when_a_player_comes_within_ten_blocks`:
  - not at 10.0, but at 9.9;
  - not for a spectator, but for a creative player;
  - once only, leaving the horse tame and adult, with the goal gone.
- `a_lone_trap_horse_vanishes_after_18000_ticks_unless_persistent`: no death and no drops.
- `server::tests::a_sprung_trap_strikes_and_brings_four_skeleton_horsemen`:
  - one visual-only bolt;
  - four tame, persistent horses, each carrying a bow skeleton in an iron helmet;
  - three of the horses pushed, and the 60-tick hurt cooldown;
  - no skeletons on peaceful.
- `server::tests::a_storm_trap_horse_springs_too` (the `{SkeletonTrap:1b,Age:0}` tag that `lightning-weather` leaves).
- `mob_tracking::tests::a_friend_sees_the_four_horsemen`, and a ticking `SkeletonTrapTime` keeps the friend's look.

### Owner's steps (survival, Normal, flat grassland, `/time set midnight`, the kit below)
1. `/summon minecraft:skeleton_horse ~16 ~ ~ {SkeletonTrap:1b}` and walk towards it. About 10 blocks away a bolt flashes on it, with no fire and no hurt. Four skeleton horsemen in iron helmets with bows appear and shoot at you. **m3t1_skeleton_trap**
2. Kill a rider: its horse is tame. Saddle it and ride it.
3. `/summon minecraft:skeleton_horse ~30 ~ ~ {SkeletonTrap:1b,SkeletonTrapTime:17940}` and stay away. After about 3 seconds it vanishes, with no drops.
4. Summon a fresh trap horse 30 blocks away. Save and quit, then load. The tamed horses keep their saddles and taming, and the trap horse still springs when you walk up to it.
5. `/difficulty peaceful`, summon a trap horse 16 blocks away and walk up to it. The bolt flashes and four tame skeleton horses stand there, with no riders. Then `/difficulty normal`.
6. (Optional) `/weather thunder` on open plains and wait. Rarely, a bolt without fire leaves a lone skeleton horse. Walk up to it and it springs.
7. Friend window (`scripts\mp-local.ps1`): the host summons a trap horse 16 blocks from the friend, and the friend walks up to it. Both windows show the flash and the four horsemen, and the friend shoots one with the M4A1. **m3t7-host**, **m3t7-friend**

### Kit
```
IW4L_MINECRAFT_KIT=minecraft:skeleton_horse_spawn_egg*4,minecraft:saddle,minecraft:iron_sword,minecraft:bow,minecraft:arrow*64,minecraft:shield,minecraft:iron_helmet,minecraft:iron_chestplate,minecraft:iron_leggings,minecraft:iron_boots,minecraft:golden_apple*8,minecraft:cooked_beef*32,minecraft:torch*64,minecraft:cobblestone*64
```
14 stacks; every id is in the section kits checked against the 26.3 item models.

### Depends on
- **M3-5:** `undead-horses-live` (the live skeleton horse and its saved trap fields) and `undead-horses-ride` (a tame skeleton horse takes a saddle).
- **M3-1:** `lightning-strikes` (`Level::add_lightning(pos, true)`), `lightning-drawn` (the bolt is seen) and `lightning-weather` (natural trap horses).
- **Other tracks:** nothing.

### Decisions for the track log
1. **Where the horsemen are built.** In terrain's `skeleton_trap.rs`, from tags through `Level::summon_mob` and `add_saved_entity`, the way the horse arms already load jockeys. The world-crate `Level::trap_horseman` from the lightning research is not built.
2. **Their enchantments** stay unsupported (`note_unsupported`), like all enchanted spawn equipment in the fork (`spawning.rs:974-981`). The iron helmet and the bow come.
3. **Peaceful:** `summon_mob` refuses the skeletons. Vanilla's appear and vanish the next tick.

---

## M3-7. Camels and camel husks (groups `camels-live`, `camels-drawn`, `camels-brain`, `camels-riding`; not registered: `HorseKind` in the cow store, hit key 11)

Mob list rows 23 and 24. Today a camel or camel husk egg does nothing, `/summon` leaves a dormant tag, and desert-village camels and the husk's camel-husk jockeys stay dormant. D0-2.5 keeps camels out of the registry: they extend `HorseKind` in the cow store, which already has saddles, riding, the mount screen and the jump bar. Only M3 edits those lines (D0-3, "The one exception").

### What exists today (`C:\Dev\mashup-crafting`, `minecraft-survival`; vendored paths under `third_party/minecraftoss/`)
- **Horse family:**
  - `HorseKind` (`entities/src/horse.rs:11-17`) has its kind methods at `:19-162`, `HorseState` at `:166-208` and `RideJump` at `:514-550`.
  - `spawn_horse` (`world.rs:5255-5268`: goals, step height 1.0).
  - The cow tick (`world.rs:7636-8051`):
    - `horse_inputs` `:7654`, the player ride `:7726-7729`;
    - the goal tick `:7811-7815`, its effects `:7831-7877`;
    - the move control `:7886`, the look `:7918-7923`, the travel `:7924-7934`, the body rotation `:7941-7946`;
    - the `AbstractHorse` aiStep and tick tail `:8028-8049`.
  - Cow arms elsewhere in `world.rs`: the hurt rear `:2293-2298`, `census` `:5805-5808` (counts every cow-store mob as a cow or mooshroom) and `check_despawn` `:5904` (`CREATURE` for all).
- **`world/equines.rs`:** `horse_movement_sounds` `:67`, `ride_tick` `:135-218`, `horse_interact` `:630`, `horse_opens_inventory` `:790`, `set_mount_inventory` `:807` (horse saddle sound `:825`), `horse_rides_on_use` `:848`.
- **`world/riding.rs`:**
  - `passenger_point(vehicle)` `:227-257` has one seat for every rider.
  - `controlling_mob` `:259` already lets zombie-store riders (the husk) steer a cow-store vehicle, and `order_vehicle` `:576` passes their path down.
  - `start_riding` `:409-412` allows one horse rider. `jumpable_by` is at `:663`; `tick_ride_jumps` `:668-701` plays `entity.horse.jump`.
- **Spawning:**
  - `type_info` (`world/src/natural_spawner.rs`) has `camel` `:290` and `camel_husk` `:304` at 1.7 x 2.375. Both end in `Rules::Other` (`:354`, `:338`), which logs and spawns nothing (`spawning.rs:766-769`). Both have cluster 6 (`:367`).
  - `MobCategory::of_type`: husk Monster `spawning.rs:100`, camel Creature `:103`.
  - `finalize_husk` (`spawning.rs:1517-1569`) already builds the 1-in-10 camel-husk jockey (husk with an iron spear in front, parched behind).
  - Desert villages place `village/desert/camel_spawn` (`generator/src/feature/entities.rs:191-229`).
- **Kind lists:**
  - `wither::is_undead` has the camel husk (`entities/src/wither.rs:81`), and so does `dormant::ALLOWED_IN_PEACEFUL` (`crates/minecraft_terrain/src/dormant.rs:5`).
  - The cow arm of `mob_effects_mut` (`world/witches.rs:533-534`) answers "not undead".
- **Terrain (`crates/minecraft_terrain/src`):**
  - `server_mobs.rs`: `SIMULATED_KINDS` `:433`, `VEHICLES` `:489`, horse load `:618-668` and save `:1707-1748`. The test `a_spider_jockey_still_waits_dormant` (`:2411-2416`) uses `minecraft:camel` as its unsimulated rider.
  - `mob_tracking.rs`: `VOLATILE` `:15`, `MobShow.ridden` `:81`, `tracking_range` `:289`, and the cow arms of `shows` `:396-406` and `show_on` `:595-597`.
  - `client_mobs.rs`: `ServerMob.vehicle: Option<(u64, f64)>` `:58` (height only, so `seat_riders` `:997-1007` puts mob riders at the vehicle's centre). The bat's animation state is at `:399-401`, `:732-739` and `:790`; also the walk `:689-691`, the cow push `:1205`, the ravager head tweak `:1327` and the shadow `:1347`.
  - `walk_animation.rs:21` hardcodes the default walk (distance x 4, ease 0.4).
  - `horse_render.rs`: `append_horses` `:339-396` (llamas filtered `:350`), `all_textures` `:72`. `spawner_display.rs`: `DRAWN_KINDS` `:19`, horse arm `:212`. `keyframes.rs` (`apply_state` `:315`, `apply_walk` `:328`) and its user `bat_render.rs:163`.
- **Brains:** `brain_behaviors.rs` (`MoveSink` `:183`, `Ticker` `:149`). The closest animal brain is `hoglin_brain.rs`: `look_sometimes` `:458`, `idle_movement` `:485`, love `:616`/`:645`, `follow_adult` `:672`. `panic.rs` has the panic and water search.
- **Rider input:**
  - `ride_input` (`render_anim/src/minecraft_riding.rs:61-73`) has no sprint, and neither do `PlayerView` (`minecraft_survival/src/entities.rs:129-149`) and `PlayerControls` (`entities/src/controls.rs:2-15`). MW2 `buttons::SPRINT` is `playerstate_iw4/src/usercmd.rs:21`.
  - `Entities::ride_seat` (`entities.rs:1102-1113`) uses the one-seat `passenger_point`.
  - The jump bar (`hud/src/minecraft_hotbar.rs:181-183`) has no cooldown sprite.

### Vanilla 26.3 (javap this round)
**Type and attributes**
- `EntityTypes.CAMEL`: CREATURE, `sized(1.7, 2.375)`, eye 2.275, `clientTrackingRange(10)`.
- `CAMEL_HUSK`: MONSTER, the same numbers, not `notInPeaceful`.
- Both use `Camel.createAttributes`: the horse base (safe fall 6, fall multiplier 0.5, follow 16) with health 32, speed 0.09f, jump 0.42f and step 1.5.
- Navigation: `setCanFloat`, `setCanWalkOverFences`. `MAX_HEAD_Y_ROT` 30. `getAgeScale` 0.6 for babies.
- `AbstractHorse`: cluster 6, volume 0.8, ambient interval 400, grazes on grass (`canEatGrass` true). The camel never rears (`canPerformRearing` false) and `isTamed` is always true.
- **Sizes:**

  | | Width x height | Eye |
  |---|---|---|
  | Adult standing | 1.7 x 2.375 | 2.275 |
  | Adult sitting | 1.7 x 0.945 | 0.845 |
  | Baby standing | 0.95 x 1.4 | 1.38 |
  | Baby sitting | 0.95 x 0.425 | 0.41 |

**Pose clock** (synced long, saved `LastPoseTick`)
- Sitting is `LastPoseTick < 0`. `poseTime = gameTime - |LastPoseTick|`.
- A transition is a pose time below 40 while sitting, below 52 while standing. `refuseToMove` is sitting or in a transition.
- `sitDown`: the sit sound, pose SITTING, ENTITY_ACTION, and the tick becomes `-gameTime`.
- `standUp`: the stand sound, STANDING, and the tick becomes `gameTime`.
- `standUpInstantly`: no sound, and the tick becomes `max(0, gameTime - 53)`. `finalizeSpawn` does the same (after `initMemories`, which draws nothing), then `AbstractHorse.finalizeSpawn` (`AgeableMobGroupData(0.2)`).
- `canCamelChangePose`: the other pose's box is clear of blocks.
- **`tick`** (after `AbstractHorse.tick`):
  1. Dashing stops once the cooldown is below 50 and the camel is on the ground, in liquid or a passenger.
  2. The cooldown counts down; at 0 the server plays `dash_ready` (NEUTRAL, 1, 1).
  3. A refusing camel has its head clamped to its body.
  4. Sitting in water stands it up at once.
- **Controls:**
  - `travel`: refusing on the ground zeroes the horizontal motion and input.
  - `CamelMoveControl`: a MOVE_TO on a sitting camel that is not leashed, not in a transition and able to change pose calls `standUp` first.
  - `CamelLookControl`: no look while it has a controlling passenger.
  - `CamelBodyRotationControl.clientTick` (both sides): nothing while refusing.
  - `actuallyHurt` and `onElasticLeashPull` stand it up (at once, or with a transition).
- Walk animation: the target is `min(distance x 6, 1)` while STANDING and not dashing, else 0. Then `update(target, 0.2, baby ? 3 : 1)`.

**Brain** (`CamelAi`; sensors nearest living, hurt by, food temptations, nearest adult; activity always IDLE)
- **CORE:**
  - `Swim(0.8)`;
  - `CamelPanic(4.0)`: `AnimalPanic`, 100-120 ticks, not while mob-controlled; `start` calls `standUpInstantly`;
  - `LookAtTargetSink(45, 90)`, `MoveToTargetSink`, and the tempt and gaze cooldown counters.
- **IDLE:**
  - 0: `SetEntityLookTargetSometimes(PLAYER, 6, 30-60)`;
  - 1: `AnimalMakeLove(CAMEL)`;
  - 2: `RunOne` of `FollowTemptation` (speed 2.5, close enough 2.5 for a baby or 3.5 for an adult) and `BabyFollowAdult(5-16, 2.5)` (not while refusing);
  - 3: `RandomLookAround(150-250, 30, 0, 0)`;
  - 4: `RunOne` (only without a walk target), weight 1 each: `RandomStroll(2.0)` and `SetWalkTargetFromLookTarget(2.0, 3)` (neither while refusing), `RandomSitting(20)`, `DoNothing(30, 60)`.
- `RandomSitting` waits until the pose is at least 400 ticks old, and needs the camel out of water, unleashed, on the ground, with no controlling passenger and able to change pose. It stands a sitting camel up and sits a standing one down (unless it is panicking).

**Riding**
- `canAddPassenger` is `passengers <= 2`; a player gets on only while fewer than 2 ride.
- The controller is a player in front of a saddled camel (`AbstractHorse`), else a Mob in front (the husk).
- `getPassengerAttachmentPoint`:
  - `z` is 0.5, or with 2 or more passengers -0.7 for a back rider, plus 0.2 for an `Animal` rider.
  - `y = getBodyAnchorAnimationYOffset(first, 0, dims, scale)`. The point is `(0, y, z x scale)` turned by `-yRot`.
- `getBodyAnchorAnimationYOffset(first, partial, dims, s)`:
  - Start with `y = dims.height - (baby ? 0.09375 : 0.375)`, and let `A = 1.43s`, `B = A - 0.2s`, `C = A - B`.
  - **In a transition:**
    - `T` is 40 sitting or 52 standing. `S` is 28 sitting, or 24 (first) / 32 standing. `k` is 0.5 (first) / 0.1 sitting, or 0.6 / 0.35 standing.
    - `t = clamp(poseTime + partial, 0, T)`; it is early while `t < S`; `f = early ? t/S : (t-S)/(T-S)`; `M = A - kB`.
    - Sitting: `y += lerp(f, early ? A : M, early ? M : C)`. Standing: `y += lerp(f, early ? C-A : C-M, early ? C-M : 0)`.
  - **Sitting at rest:** `y += C`.
  - So an adult's seat is 2.0 high standing and 0.77 sitting.
- `tickRidden`: forward input on a sitting camel that is not in a transition calls `standUp`. While refusing, the input is zero and the camel keeps its own rotation.
- `getRiddenSpeed` is `(sprinting && cooldown 0 ? 0.1 : 0) + 0.09`. `canSprint` is true (`LocalPlayer.vehicleCanSprint`).
- **Dash:**
  - `canJump`: saddled and not refusing. `onPlayerJump`: only with a cooldown of 0 and on the ground, then the horse's pending scale.
  - `handleStartJump`: the dash sound (volume 0.8, voice pitch), ENTITY_ACTION, dashing.
  - `executeRidersJump(scale)`: add `lookXZ.normalize() x 22.2222 x scale x speed attribute x blockSpeedFactor`, and up `1.4285 x scale x getJumpPower()`. At full scale that is 2.0 forward and 0.6 up. Then cooldown 55, dashing, `needsSync`.
  - A synced DASH with a cooldown of 0 sets 55.
  - The client charges only at cooldown 0. `JumpableVehicleBar` draws `hud/jump_bar_cooldown` while the cooldown is above 0.

**Interaction** (`mobInteract`, in order)
1. Sneaking on an adult opens the mount screen; no taming is needed.
2. Item interactions: the saddle (adult; `entity.camel.saddle`), lead, name tag.
3. Food (`#camel_food` cactus; the husk's rabbit_foot) goes to `handleEating`:
   - a hurt camel heals 2;
   - a tame camel with age 0 that can fall in love goes into love (event 18);
   - a baby ages up 10 s (with a client happy-villager sparkle);
   - any of these plays the eat sound (pitch `1 + (r1 - r2) x 0.2`), EAT, and uses one item.
4. With fewer than 2 passengers, an adult rides.
5. A baby with a golden dandelion goes to `AbstractHorse`. Anything else fails.

`canMate`: two camels that can be parents; the child is a plain camel.

**Sounds:**
- Prefixes `entity.camel.` and `entity.camel_husk.`, with `ambient`, `hurt`, `death`, `eat`, `sit`, `stand`, `dash`, `dash_ready`, `saddle`, `step`, `step_sand`.
- Steps: `step_sand` on `#camel_sand_step_sound_blocks` (`#sand`, `#concrete_powders`), else `step`. Volume 1 (the husk 0.4), no gallop.

**Camel husk:**
- `removeWhenFarAway` true. It is mob-controlled when a Mob rides in front.
- `interact` sets persistence. It can be leashed only when not mob-controlled.
- It never mates, falls in love or is a baby.
- `chargeSpeedModifier` 4.0. It is `#zombies`, so undead, and does not burn in daylight.

**Loot and spawning:**
- `entities/camel` is empty. `entities/camel_husk` drops rotten flesh 2-3, plus 0-1 per Looting level. Experience is `Animal`'s 1-3.
- The desert creature list has `camel` (count 1, weight 1). Placements:
  - camel: ON_GROUND, `checkCamelSpawnRules` (below in `#camels_spawnable_on` = `#sand`, and raw brightness above 8);
  - camel husk: ON_GROUND, `Monster.checkSurfaceMonstersSpawnRules`. No biome spawns it.

**Client**
- `CamelRenderer`: shadow 0.7, `entity/camel/camel` (adult) and `camel_baby` (baby), saddle equipment `camel_saddle`.
- `CamelHuskRenderer`: the adult model only, `camel_husk`, `camel_husk_saddle`.
- `AdultCamelModel` (128 x 128, 12 cubes):
  - `body` (box 15 x 12 x 27 at (0,4,9.5)) with the children `hump`, `tail` and `head` (three boxes, with `left_ear` and `right_ear`);
  - four root legs, each 5 x 21 x 5 at (±4.9, 1, 9.5 / -10.5).
- `BabyCamelModel` (64 x 64, no hump) and `CamelSaddleModel` (the saddle on the body, bridle and reins on the head, `CubeDeformation(0.05)`, reins only while ridden).
- Copy the exact boxes from `createBodyLayer` with javap during the work.
- `setupAnim`:
  - head yaw clamped to ±30 and pitch to -25..45; while `jumpCooldown = max(cooldown - partial, 0) > 0`, the pitch gets `+45 x jumpCooldown / 55`, clamped to -25..70;
  - `walk.applyWalk(pos, speed, 2.0, 2.5)`;
  - then sit, sit pose, sit up, idle and dash `apply(state, ageInTicks)`.
- Keyframes: `CamelAnimation.CAMEL_{WALK,SIT,SIT_POSE,STANDUP,IDLE,DASH}` and `CamelBabyAnimation.CAMEL_BABY_*`.
- `setupAnimationStates` (client):
  - The idle animation restarts with a client-random timeout of `80 + random(40)`.
  - Visually sitting (`(poseTime < 0) != sitting`): sit up and dash stop. Sitting down (pose time 0-39) runs the sit animation; after that the sit pose.
  - Otherwise: dash `animateWhen(dashing)`, and sit up `animateWhen(transition && poseTime >= 0)`.

### Plan
**Not registered:** no `Family` or `TerrainFamily`, no `FamilyShow`, no `night` registration line. The D0 pieces used:
- D0-4 rows in `world/src/natural_spawner/families/night.rs` (a row may override a built-in kind);
- `keyframes::shared()` (D0-6.1);
- D0-2.5's seat index (if `registry-hooks` made `passenger_point` seat-aware, the camel arm takes the same seat; otherwise this track adds it);
- `is_simulated` through `SIMULATED_KINDS` (D0-5.1).

**New files** (under M3-owned parents, so no `lib.rs` or `world.rs` `mod` line):
- `entities/src/horse/camel.rs` (`pub mod camel;` in `horse.rs`) holds `CamelState`:
  - fields `last_pose_tick`, `dash_cooldown`, `dashing`, `game_time`, `brain`;
  - the sizes, the pose clock, sit and stand, `refuses_to_move`, `body_anchor_y`, `passenger_point(seat, count, animal_rider, yaw)`, `ridden_speed` and `dash_impulse`;
  - tests.
- `entities/src/horse/camel_brain.rs`: `CamelAi` on `brain_behaviors`, shaped like `hoglin_brain.rs`. Its panic, temptation, love and baby-following come from M1's `animal_brain.rs` when it is merged (M30); otherwise they stay private in `camel_brain.rs`.
- `entities/src/world/equines/camels.rs` (`mod camels;` in `equines.rs`):
  - the brain inputs and tick, and `camel_tick`;
  - the interact, inventory and ride checks, `camel_ride` and `camel_dash`;
  - tests.
- `crates/minecraft_terrain/src/horse_render/camel.rs` (`mod camel;` in `horse_render.rs`): the adult, baby and saddle models, the keyframes and the draw.

**`HorseKind`** gains `Camel` and `CamelHusk` with an arm in each exhaustive match. They are not chested (`is_chested` excludes them), never rear, graze, are saddleable and take no armour. Foods `camel_food`/`camel_husk_food`, sound families `camel`/`camel_husk`, attributes 32 / 0.09f / 0.42f, and only camel mates with camel. `HorseState.camel: Option<Box<CamelState>>`, and camels are always `tame`.

**One-line hooks** (each next to the named arm):

| Where | Hook |
|---|---|
| `world.rs:5261-5265` `spawn_horse` | a camel gets no goals and step 1.5 |
| `world.rs:7654` (beside `horse_inputs`) | the camel's brain inputs |
| `world.rs:7811-7815` | a camel runs its brain instead of `entity.goals` |
| `world.rs:7726-7729` | a ridden camel goes to `camel_ride` |
| before `world.rs:7886` | `CamelMoveControl`'s stand-up |
| `world.rs:7918` | `tick_limited(.., 30.0)`, no look while a player steers |
| `world.rs:7924` | refusing on the ground: no horizontal input or motion |
| `world.rs:7941` | no body rotation while refusing |
| `world.rs:8047` (beside `tick_animation`) | `camel_tick` |
| `world.rs:2293-2298` | a hurt camel stands up at once |
| `world.rs:5806` census | `horse.kind.type_id()`, so a camel husk counts as a monster |
| `world.rs:5904` `despawn_rule()` (from `undead-horses-live`) | the camel husk arm: 128, removable, allowed in peaceful |
| `horse.rs` `HorseKind::is_undead` | the `CamelHusk` arm; the `witches.rs:533` line is `undead-horses-live`'s |
| `equines.rs:630`, `:790`, `:848` | camel interact, inventory and ride checks |
| `equines.rs:825` | the camel's saddle sound |
| `riding.rs:227` | the camel seat by rider index |
| `riding.rs:410` | a camel takes riders while `<= 2` ride |
| `riding.rs:663` | a camel is jumpable only at cooldown 0 and not refusing |
| `riding.rs:694` | the camel dash sound and dashing |
| `equine_leashable` (`equines.rs`, from `undead-horses-ride`, called at `leashes.rs:338`) | no lead on a mob-controlled husk |
| `leashes.rs:829` (`pulled = true;` in the elastic pull, `:808-830`; `:850` is `leash_walk`) | a pulled sitting camel stands up (`onElasticLeashPull`) |

Terrain lines (M3's exception):
- `SIMULATED_KINDS` and `VEHICLES` +2; the horse load and save arms read and write `LastPoseTick`;
- `HATCHING_EGGS` 52 → 54 (the undead horses took it from 50 to 52 in m3d; `camel` and `camel_husk` go in sorted order after `bogged`), and `tracking_range` 10;
- `all_textures`, the camel arm in `append_horses`, and `DRAWN_KINDS` plus the `spawner_display.rs:212` arm;
- in `a_spider_jockey_still_waits_dormant` (`server_mobs.rs:2411`) the strider's camel rider becomes `minecraft:giant`, which stays dormant this round.

**Rows:**
- `minecraft:camel`: 1.7 x 2.375, `OnGround`, `MotionBlockingNoLeaves`, `Rules::SpawnableOn("minecraft:camels_spawnable_on")`, `Walk::Animal`, kept far away, cluster 6, Creature, follow 16, `baby_chance: Some(0.2)`.
- `minecraft:camel_husk`: the same box, `Rules::SurfaceMonster`, `Walk::Animal` (it is an `Animal`), removed far away, cluster 6, Monster, follow 16, `baby_chance: None`.

**Multiplayer:**
- `MobShow` gains `camel: [i32; 3]` (pose ticks, dash cooldown, dashing) after `ridden`, like `ravager`. The sitting flag goes in the existing `sitting`.
- The cow arm of `shows` fills them. The cow arm of `show_on` sets the mirror's pose, dash, box and eye, so the friend's bullets hit a sitting camel's low box. `VOLATILE` gains `"LastPoseTick"` (its length 38 → 39, after `skeleton-trap`'s `SkeletonTrapTime`).
- `client_mobs.rs`:
  - `receive` copies the camel show (next to `resting`, `:924`);
  - `ClientMob.camel: Option<Box<CamelClient>>` sits next to the bat fields: a `LegacyRandom` from the id, the idle timeout and five start ticks, run as vanilla `setupAnimationStates` next to the bat's;
  - the walk goes through a new `WalkAnimation::update_with(target, factor, scale)`; there is no body rotation while refusing;
  - `MobPose.camel`; the pose's eye in the cow push; `max_head_y_rot` 30 next to the ravager's; the shadow is `0.7 x getAgeScale`.
- `ServerMob.vehicle` becomes `Option<(u64, f64, f64)>` (height and forward). `seat_riders` turns the forward part by the vehicle's body yaw. That seats a camel husk's parched behind its husk, and also a ravager's rider at its forward seat.
- `ride_seat` asks the player's own seat.
- Protocol: `MobShow.camel` and `ServerMob.vehicle` go under "## Protocol".

**Keyframes:** `keyframes::shared().class("CamelAnimation")` and `("CamelBabyAnimation")`, with `apply_walk(.., 2.0, 2.5)` and `apply_state`. With no jar the camel stands still.

**E's horse lines:**
- E1's `horse-fall`/`horse-gallop` rewrite `equines.rs:67-80`, `emissions.rs:21` (`HorseStep`) and the fall lines `world.rs:7748`/`:7933`.
- Camels use E's `horse_fall` (`AbstractHorse.causeFallDamage`), with `can_gallop` false.
- Camel steps are one `HorseStep` variant (sand sound, other sound, volume). Segment m3e starts after e1a is merged, so E1's `HorseStep` is there.

**COMMIT GROUPS**

| # | Group key | Commits |
|---|---|---|
| 1 | `camels-live` | 1. "Minecraft camels and camel husks are saved, loaded and hatched": `HorseKind` arms, `CamelState` (pose clock, sizes, sit and stand, tick, hurt and water stand-up, look 30, refusing travel), `LastPoseTick` saved and loaded, the kind lists and eggs, census and despawn, the strider test. Rows 23-24 `live`. |
| 2 | `camels-drawn` | 1. "Minecraft camels are drawn with their keyframe animations": models, saddle layer, textures, keyframes, `MobShow.camel` with the `shows` arm, the `ClientMob` camel state and walk, shadow, head limit, cage. Rows `drawn`. 2. "Friends see Minecraft camels sit, stand and dash": the `show_on` arm, `VOLATILE`, the mirror's box and eye. |
| 3 | `camels-brain` | 1. "Minecraft camels wander, sit down, panic, eat cactus and breed like vanilla": the brain and feeding (heal, love, age up, husk rabbit feet). 2. "Minecraft camels spawn in deserts and camel husks carry husk jockeys": the rows, jockey loading and steering, peaceful and far despawn. |
| 4 | `camels-riding` | 1. "Two players can ride a Minecraft camel": seats by index, `start_riding`, `ride_seat`, the forward seat for mob riders, the saddle-only screen without taming, steering only when saddled, standing up on forward, no turning while sitting. 2. "Minecraft camels dash with the jump bar and run when their rider sprints": dash, cooldown and ready sound, `jumpable_by`, the cooldown sprite, sprint from `ride_input` into M1's `PlayerControls.sprinting` (M30). Rows `done`; the owner's steps and kit in the log. |

### Tests
- **1.1:**
  - `a_camel_is_1_7_by_2_375_with_32_health_and_its_kind` (eye, speed, jump, step, `entity_type`, census Creature and the husk's Monster, empty table, 1-3 experience, husk rotten flesh);
  - `a_sitting_camel_is_0_945_high_and_a_baby_0_95_by_1_4`;
  - `sitting_takes_40_ticks_and_standing_up_52`;
  - `a_hurt_or_wet_camel_stands_up_at_once`;
  - `a_refusing_camel_neither_walks_nor_turns_its_body`;
  - `a_camel_husk_despawns_far_away_but_stays_on_peaceful`;
  - `server_mobs::tests::a_camel_saves_and_loads_its_pose_tick_and_saddle`;
  - the eggs survey and the changed strider test.
- **2.1:**
  - `horse_render::camel::tests::an_adult_camel_is_twelve_cubes_on_a_128_texture_and_a_baby_on_64`;
  - `the_camel_keyframes_read_from_the_jar` (gated on `MINECRAFTOSS_ROOT`);
  - `the_head_turns_thirty_degrees_and_tilts_with_the_dash_cooldown`;
  - `the_reins_show_only_while_ridden`;
  - `client_mobs::tests::a_camel_starts_sit_sit_pose_and_stand_up_like_vanilla` and `a_camel_walk_swing_eases_at_a_fifth_and_stops_while_sitting`.
- **2.2:** `mob_tracking::tests::a_friend_sees_a_camel_sit_stand_and_dash` (`show_on` values, the low box, the eye) and `a_camel_look_tag_drops_its_pose_tick`.
- **3.1:**
  - `an_idle_camel_strolls_and_sits_after_400_ticks_in_a_pose`;
  - `a_hurt_camel_panics_at_four_times_its_speed`;
  - `a_camel_follows_cactus_and_stops_for_its_cooldown`;
  - `two_fed_camels_make_a_baby_that_follows_them`;
  - `cactus_heals_two_and_ages_a_baby_ten_seconds`;
  - `a_camel_husk_eats_rabbit_feet_but_never_breeds`.
- **3.2:** `natural_spawner::families::night::tests::camels_spawn_alone_on_bright_sand_only` and `a_camel_husk_needs_the_surface_monster_rules`; `server_mobs::tests::a_husk_jockey_loads_on_its_camel_husk_with_a_parched_behind_and_steers_it`.
- **4.1:**
  - `two_players_ride_a_camel_and_the_second_sits_behind` (2.0 high and 0.5 forward, -0.7 behind, 0.77 sitting, hand-computed transition points);
  - `a_third_player_cannot_board_but_a_third_mob_can`;
  - `only_a_saddled_camel_with_a_player_in_front_steers`;
  - `forward_stands_a_sitting_camel_up`;
  - `mount_inventory::tests::a_camel_opens_its_saddle_only_screen_without_taming`;
  - survival `entities::tests::each_rider_takes_its_own_seat`;
  - `client_mobs::tests::a_mob_rider_sits_at_its_forward_seat`.
- **4.2:**
  - `a_released_jump_dashes_two_blocks_a_tick_forward_and_cools_down_55` (sound, dashing cleared on landing after 5 ticks, the ready sound);
  - `no_dash_while_sitting_cooling_or_airborne`;
  - `a_sprinting_rider_runs_a_camel_at_0_19`;
  - `minecraft_hotbar::tests::the_jump_bar_shows_the_cooldown_sprite`;
  - the `ride_input` test with sprint.

### Owner's steps (survival, Normal, by day; `/locate biome minecraft:desert` and `/tp` there, or lay 20 x 20 sand)
1. Use a camel egg 6 blocks away. A tall tan camel with a hump turns its head a little, now and then twitches its head and ears, and ambles about. Within a minute or two it folds its legs and sits (`entity.camel.sit`, 2 s), and later rises (`entity.camel.stand`, about 2.5 s). **m3c1_camel_sits**
2. Hit it: a sitting camel stands at once and runs off fast for 5-6 s. Hold cactus: it follows; feed it and it eats and heals.
3. Use a second egg and feed both a cactus: hearts, then a small hump-less baby that follows them. Feed the baby a cactus: it grows a little. **m3c3_baby_camel**
4. Sneak and use an adult: a screen with only a saddle slot opens, with no taming. Put the saddle in: `entity.camel.saddle`, and a saddle and bridle show. **m3c4_camel_saddle**
5. Use it: you sit high on its back. W walks slowly; hold MW2 sprint and it runs about twice as fast. It climbs a 1-block step and a block-and-slab (1.5) step without jumping.
6. Hold jump and release: the camel lunges far forward and up (`entity.camel.dash`), with its head tilted. The bar stays grey for about 2.75 s, then `entity.camel.dash_ready` plays. Holding jump while it is grey fills nothing. **m3c6_camel_dash**
7. Wait until it sits under you: you sit low and mouse turns do not turn it. Press W: it stands (about 2.5 s), then walks.
8. Ride into water 2 deep: it floats with you. Push a sitting camel into water: it stands at once.
9. Friend window (`scripts\mp-local.ps1`): the host rides the saddled camel and the friend uses it too. The friend sits behind; only the host steers; a third use does nothing. **m3c9-host**, **m3c9-friend**
10. Friend window: the friend sees a camel sit, stand and dash in step with the host. The friend shoots a sitting camel with the M4A1: the bullet hits its low body, and it stands and runs.
11. `/summon minecraft:camel_husk ~ ~ ~6 {Passengers:[{id:"minecraft:husk"},{id:"minecraft:parched"}]}`: a grey-green camel husk carries a husk in front and a parched behind. The husk steers it at you and the parched shoots. Kill the camel husk with the Looting sword: rotten flesh drops, and the riders fall off and fight on. **m3c11_husk_jockey**
12. Summon another jockey and set `/difficulty peaceful`: the husk and parched vanish and the camel husk stays. `/difficulty normal`. Feed it a rabbit's foot: it eats and never breeds. Put a lead on it: the lead holds.
13. Leave a saddled camel sitting and another standing, save and quit, load: the first still sits saddled.
14. Use a camel egg on a placed spawner: a small camel turns in the cage.
15. (Optional) `/locate structure minecraft:village_desert`: its camels wander and sit.

### Kit
```
IW4L_MINECRAFT_KIT=minecraft:camel_spawn_egg*8,minecraft:camel_husk_spawn_egg*4,minecraft:husk_spawn_egg*4,minecraft:parched_spawn_egg*4,minecraft:saddle,minecraft:saddle,minecraft:cactus*64,minecraft:rabbit_foot*16,minecraft:lead*4,minecraft:golden_dandelion*4,minecraft:sand*64,minecraft:sandstone*64,minecraft:oak_slab*32,minecraft:water_bucket,minecraft:iron_sword,minecraft:diamond_sword+looting=3,minecraft:bow+power=5,minecraft:arrow*64,minecraft:shield,minecraft:spawner,minecraft:cooked_beef*32,minecraft:torch*64
```
22 stacks; every id is checked against the 26.3 item models.

### Depends on
- **D0 d0a:** `registry-spawning` (rows), `registry-hooks` (the seat index, if D0 adds it), `registry-client` (`is_simulated`, the eggs survey), `registry-draw` (`keyframes::shared()`). **Not d0b.**
- **E1 e1a** (merged before m3d, M30): `horse-fall` and `horse-gallop` touch the same horse lines. `particles-events`' `puff_for` takes one row (`minecraft:camel`, 18) for the love hearts, next to a related row.
- **W:** optional one-line game events (ENTITY_ACTION for sit, stand and dash; EAT) after P7.
- **Other M3 groups:** `undead-horses` runs first: `HorseKind` grows twice, and it adds the undead arm in `witches.rs` and the kind-dependent despawn expression. `happy-ghasts` reuses the seat index, `ride_seat` and the forward seat.

### Decisions for the track log
1. **The brain in the cow store.** `CamelState.brain` replaces the goal tick for camels, drives the cow's navigation through `MoveSink` as the hoglin's does, and keeps the cow tick's movement, look and travel. (D0-2.4 lists camels as a brain family; D0-2.5 keeps them in the cow store.)
2. **Shared lines beyond the exception list:** `MobShow`, `VOLATILE`, the cow arms of `shows`/`show_on`, `ClientMob`/`MobPose`, `walk_animation.rs`, `ServerMob.vehicle`, `seat_riders`, `ride_seat`, `leashes.rs`, `witches.rs`, the HUD and the sprint input. Recommended: treat them as M3's horse arms, next to the related arm, logged, with Protocol notes.
3. **A fresh camel's pose tick.** The rows have no game time, and vanilla's `max(0, gameTime - 53)` matters, because `RandomSitting` waits 400 pose ticks. Recommended: `spawn_saved` treats a missing or 0 `LastPoseTick` as a fresh spawn and sets `max(0, game_time - 53)`. The only difference: a camel saved in a world's first 53 ticks waits about 17 s longer for its first sit.
4. **The cooldown jump bar.** Recommended: a negative `jump_bar` means `hud/jump_bar_cooldown`, so there is no new wire field.
5. **Rider sprint.** `PlayerControls.sprinting` and `PlayerView.sprinting` are M1's (`armadillos-live`, M30); M3 adds only the line that passes `buttons::SPRINT` through `ride_input`. Without the flag a ridden camel never goes above 0.09, so if M1's field is not merged when `camels-riding` runs, the sprint is left for later.
6. **A third mob passenger.** Keep vanilla's `canAddPassenger` (`<= 2`); players board only while fewer than 2 ride.
7. **Check during the work:** `checkSurfaceMonstersSpawnRules` may skip the sky check for spawners, while `spawning.rs:712` asks for full sky light. Fix it in its own commit only if javap shows the difference.

---

## M3-8. Happy ghasts, ghastlings and dried ghasts (groups `happy-ghasts-live`, `happy-ghasts-riding`, `happy-ghasts-ghastling`)

Family `happy_ghasts`, kind `minecraft:happy_ghast`, group `night`, hit-key byte and `Family::KEY` **84** (mob list row 50). The adult runs goals; the ghastling runs a brain kept in the state (D0-2.4).

### What exists today (`C:\Dev\mashup-crafting`, `d1a3d028`)
- **No entity.** `minecraft:happy_ghast` is not in `SIMULATED_KINDS` (`crates/minecraft_terrain/src/server_mobs.rs:433`): an egg does nothing and a saved one stays dormant.
- **Ghast code to reuse** (26.3 shares `Ghast$GhastMoveControl`, `Ghast$RandomFloatAroundGoal` and `GhastModel.animateTentacles` with the happy ghast):
  - `entities/src/ghast.rs`: `face_toward` `:53`; `suitable_fly_to` `:57` (only the plain ±16 roll); `can_reach` `:69` (the non-careful sweep).
  - `entities/src/world/ghasts.rs:55-126` `tick_ai`: goals, float duration, `travel_flying` (`movement.rs:441`).
  - `crates/minecraft_terrain/src/ghast_render.rs`: `tentacle_swing` `:41`, `append_ghasts` `:45-102` (`cow_render::cube_scaled` with a model scale).
- **Dried ghast.**
  - The block, item, recipe and block loot load from the jar. Nether fossils place it (`generator/src/structure/kinds/nether_fossil.rs:108-121`) and piglins barter it (`entities/src/loot.rs:805`).
  - A water bucket waterlogs it with the soak sound (`bucket.rs:24`, `:431`).
  - Nothing hydrates it: its random tick lands in `unsupported` (`world/src/level/random_tick.rs:331`).
  - Its sound type's `place` is `intentionally_empty`, so placing it is silent (`crates/minecraft_survival/src/actions.rs:1342-1351`).
- **Harnesses.** The 16 `*_harness` items (`equippable_slot: body`) and their recipes exist; nothing equips them.
- **Riding.** The horse ride tick (`world/equines.rs:135`) reads `PlayerControls {yaw, pitch, forward, strafe, jumping}` (`controls.rs`, `world.rs:5242`). These come from MW2 keys for host and friends (`render_anim/src/minecraft_riding.rs:61`: ×0.98, the jump button). The MW2 pitch is positive looking down, like vanilla `xRot`.
- **Helpers.** `age.rs:6` `Age`; `animal.rs:12` `interact`; `World::rain_at` (`player/src/lib.rs:189`) and `block_in_tag` (`:238`); `EntityWorld.dimension` (`world.rs:2515`); cloud height 192.33 (`core/src/environment.rs:118`); leash snap 12 and elastic 6 (`leashes.rs:4-5`). `tempt::TemptState` (`tempt.rs:30`) hard-wires range 10 and cow food, so it is not reusable.

### Vanilla 26.3 (javap this round)
**Type and numbers.**
- `EntityTypes.HAPPY_GHAST`: `CREATURE`, 4.0 × 4.0, eye 2.6, `clientTrackingRange(10)`. Not fire immune, not `notInPeaceful`.
- Passenger attachments: seat 0 (0, 4, 1.7), 1 (−1.7, 4, 0), 2 (0, 4, −1.7), 3 (1.7, 4, 0), turned with the yaw. `ridingOffset(0.5)`.
- A baby is the type's size × 0.2375 (0.95), with eye height 0.46875.
- `createAttributes`: health 20, tempt range 16, flying speed 0.05, movement 0.05, follow range 16, camera distance 8.
- Loot `entities/happy_ghast` is empty. Experience is the creature default. `canFallInLove` false. Food `#happy_ghast_food` (the snowball). Cluster 1.

**Spawning and tags.**
- `SpawnPlacements`: `ON_GROUND`, `MOTION_BLOCKING_NO_LEAVES`, `Animal.checkAnimalSpawnRules`. No biome spawns it. `finalizeSpawn` is `AgeableMob`'s (baby chance 0.05).
- `#happy_ghast_tempt_items` = food + `#harnesses`.
- `#happy_ghast_avoids` = sweet berry bush, cactus, wither rose, magma block, fire, `#speleothems`.
- Also in `#can_equip_harness`, `#dismounts_underwater`, `#fall_damage_immune`, `#followable_friendly_mobs`.

**`HappyGhast` (server).**
- **Movement.** `travel`: `travelFlying(input, s, s, s)` with s = 0.05 × 5/3 = 0.0833, no gravity. No fall damage, no step sounds. An adult drowns; a baby breathes water.
- **Sounds.** Voice pitch always 1.0 (no random draw), source NEUTRAL, volume 1 for a baby and 4 for an adult. Ambient interval 120, ×6 while it has any passenger. `entity.ghastling.*` (baby) and `entity.happy_ghast.*` (adult).
- **Adult goals.**
  - 3: `HappyGhastFloatGoal`, a `FloatGoal` unless on still timeout.
  - 4: `TemptGoal$ForNonPathfinders(1.0, stop 7.0)`. Items: food only when harnessed or a baby, else the tempt items. It sets the wanted spot to `pos + (eye − pos) × nextDouble()`, and stops with `setWait`.
  - 5: `Ghast$RandomFloatAroundGoal(16)`.
- **Setup by age (`ageBoundaryReached`).**
  - Baby: `FlyingMoveControl(180, true)`, a plain `LookControl`, `BabyFlyingPathNavigation` (`FlyingPathNavigation`, no doors, `canFloat`, path length 48, `canMoveDirectly` = `isClearForMovementBetween(.., false)`), the still timeout cleared, no goals.
  - Adult: `GhastMoveControl(careful, stopped = isOnStillTimeout)`, `HappyGhastLookControl`, the goals, the brain cleared.
- **`GhastMoveControl.tick`.**
  - When stopped: `WAIT` and `stopInPlace` (velocity zero).
  - On `MOVE_TO`, when `floatDuration-- <= 0`: add `nextInt(5) + 2`. If `canReach`, velocity += normalize(delta) × 0.0833, else `WAIT`.
  - Careful `canReach`: every block of the moved box inflated by 1 must be air or collisionless, outside `#happy_ghast_avoids`, and hold no water or lava. Then the swept `forEachBlockIntersectedBetween` uses the same rules; water passes only when it is already in water, and lava likewise.
- **`RandomFloatAroundGoal(16)`.**
  - Runs when there is no wanted spot, or it is under 1 or over 60 blocks off; it never continues.
  - Up to 64 rolls of ±16 per axis. A roll is rejected outside the home. It is accepted when the spot is air and a non-air block lies within 1-15 blocks along one of the 6 directions.
  - Otherwise the last plain roll is kept. If `MOTION_BLOCKING` is below the target and above the minimum y, y becomes `y − |y − target.y|`.
- **Look and body.**
  - `HappyGhastLookControl`: on still timeout, it snaps the yaw to a quarter turn (`wrapDegrees90`). With a look-at cooldown it turns to the look spot. Otherwise it uses `Ghast.faceMovementDirection`.
  - While ridden, head = body = yaw.
- **`tick`.**
  - The still timeout counts down only after `tickCount > 60`.
  - `scanPlayerAboveGhast` sets it to 10. The box runs from (minX−1, maxY−1e−5, minZ−1) to (maxX+1, maxY + h/2, maxZ+1). It counts non-spectator players whose root vehicle is not a happy ghast, judged at the root's position.
  - `still_timeout` is saved.
- **`checkRestriction`** (not leashed, not ridden): home radius 32 (harnessed or baby) or 64. `setHomeTo(blockPos, r)` unless the home is within r + 16 and its radius is already r. Mob saves `home_pos` and `home_radius`.
- **`continuousHeal`.** +1 health every 20 ticks in clouds or where `precipitationAt` is not NONE, else every 600.
  - In clouds: the cloud colour alpha is not 0 and y + h ≥ the cloud height ≥ y − 4.
- **Ghastling brain (`HappyGhastAi`).**
  - CORE: `Swim(0.8)`, `AnimalPanic(2.0, 0)` (100-120 ticks; javap the int's lambda during the work), `LookAtTargetSink(45, 90)`, `MoveToTargetSink`, the temptation cooldown.
  - IDLE:
    - 1 `FollowTemptation(1.25, close 3)`
    - 2 `BabyFollowAdult(3..16, 1.1, NEAREST_VISIBLE_PLAYER)`: it follows players
    - 3 the same with adults
    - 4 `RunOne(RandomStroll.fly(1.0), SetWalkTargetFromLookTarget(1.0, 3))`
  - PANIC while panicking.

**Interaction and riding.**
- **`mobInteract`.**
  - A baby gets `Animal`'s: the snowball feeds it, the dandelion locks its age.
  - An adult takes the held harness first (`Equippable`: body slot, alive adult, empty slot). That calls `setItemSlot`, `setGuaranteedDrop(BODY)` (`drop_chances {body: 2.0}`) and plays `entity.happy_ghast.equip`; creative keeps the item.
  - Otherwise, when harnessed and not crouching, `doPlayerRide`.
- **`Mob.attemptToShearEquipment`** (before `mobInteract`). Shears with no passengers take the harness off and drop it at the top. The shears lose 1. It plays `entity.happy_ghast.unequip`.
- **Passengers.**
  - First rider: `harness_goggles_down`; still timeout 0, or at most 10 when a player is above.
  - A rider leaving: still timeout 10. The last rider leaving: `clearHome` and `harness_goggles_up`.
  - At most 4 riders.
- **Control.** `getControllingPassenger`: the first passenger when it is a player, the ghast is harnessed, and it is not on still timeout.
- **`getRiddenInput`.**
  - x = `xxa`.
  - When `zza ≠ 0`: z = cos(pitch), y = −sin(pitch), ×(−0.5) going backwards. Jump adds 0.5 to y.
  - The vector is ×3.9 × 0.05. Steady: forward 0.164 blocks/tick (3.3 blocks/s), back half that, jump alone 0.082/tick.
- **`tickRidden`.** yaw += `wrapDegrees(riderYaw − yaw)` × 0.08, pitch = rider × 0.5.
- **Dismount** at the top centre (D0's default). `canBeCollidedWith` holds for players at its top and while it is still, so players stand on it. Leash: elastic 10, snap 16, `setWait` on a pull, quad-leash holder (ropes).

**Client.**
- **`HappyGhastModel.createBodyLayer(inner, deform)`** (64², then scaled 4).
  - `body` at (0, 16, 0), cube (−8, −8, −8) 16³, uv (0, 0).
  - Baby only: `inner_body` at (0, 8, 0), cube (−8, −16, −8) 16³, deform −0.5, uv (0, 32).
  - Nine body-child tentacles, cubes (−1, 0, −1) 2 × L × 2, uv (0, 0), with pivots (x, 7, z) and lengths L:
    - row z −5: x −3.75, 1.25, 6.25 with L 5, 7, 4
    - row z 0: x −6.25, −1.25, 3.75 with L 5, 5, 7
    - row z 5: x −3.75, 1.25, 6.25 with L 8, 8, 5
  - The baby layer adds `BABY_TRANSFORMER` (×0.2375, so 0.95 overall).
  - `setupAnim`: body scale 0.9375 when wearing a harness, then tentacle xRot = 0.2 sin(age × 0.3 + i) + 0.4.
- **`HappyGhastHarnessModel`** (64², ×4).
  - `harness` at (0, 24, 0), cube (−8, −16, −8) 16³.
  - `goggles` at (0, 14, −5.5), cube (−8, −2.5, −2.5) 16 × 5 × 5, deform 0.15, uv (0, 32).
  - Ridden: xRot 0, y 14. Otherwise: xRot −0.7854, y 9.
- **`HappyGhastRenderer`.**
  - Shadow 2.0 × `ageScale`, so 0.475 for a ghastling, not half.
  - Textures `entity/ghast/happy_ghast`, `happy_ghast_baby`, and `entity/equipment/happy_ghast_body/<colour>_harness`.
  - `RopesLayer` only when it holds a leash and wears a harness. `entityCutoutNoCull`. No `*Animation` class. Riders hear a looping `entity.happy_ghast.riding`.

**`DriedGhastBlock`** (facing, `hydration` 0-3, waterlogged).
- **`randomTick`.** When waterlogged or hydration > 0, and no tick is scheduled there: schedule one in 5000 ticks.
- **`tick`.**
  - Waterlogged, below 3: +1 with `block.dried_ghast.transition` and `block_change`.
  - At 3: remove the block and create a `BREEDING` happy ghast (no `finalizeSpawn`) as a baby (Age −24000) at the bottom centre, yaw = facing (S 0, W 90, N 180, E 270). It plays `entity.ghastling.spawn`.
  - Dry: −1.
- **Placement.** Waterlogged in water, facing the player. `setPlacedBy` plays `place` or `place_in_water`.
- **Hatch time.** In water, four ticks of 5000 plus four random-tick waits (about 1365 each): about 25,500 ticks.
- **`animateTick`.**
  - Dry: 1 in 40, on soul sand or soul soil, plays `block.dried_ghast.ambient`. 1 in 6 makes `white_smoke` at the centre, motion (0, 0.02, 0).
  - Wet: 1 in 40 plays `ambient_water`. 1 in 6 makes `happy_villager` at ±(r×2−1)/3, y + 0.4, motion (0, r, 0).

### Plan
**Files (D0-3).**
- `entities/src/world/families/night/happy_ghasts.rs` (+ `happy_ghasts/{moving,riding,ghastling}.rs`): `pub struct HappyGhast`, `impl Family`, tests. The state holds:
  - `Age`;
  - the harness id and its guaranteed flag;
  - `still_timeout`, `home`;
  - the adult `Box<MonsterAi>`, the move-control state, the look cooldown;
  - the ghastling brain.
- `crates/minecraft_terrain/src/families/night/happy_ghasts.rs` (+ `render.rs`, `dried_ghast.rs`). It is `pub(crate) mod happy_ghasts;` in M3's `night.rs`, because two shared callers reach in.
- `world/src/level/dried_ghast.rs`, plus `pub mod dried_ghast;` next to `pub mod ice;` (`level/mod.rs:23`).
- A row in `natural_spawner/families/night.rs`:
  - 4.0 × 4.0, `OnGround`, `MotionBlockingNoLeaves`, `Rules::Family { creature: true }`, cluster 1;
  - `MobCategory::Creature`, follow range 16, `baby_chance: Some(0.05)`;
  - the animal rule, no `finalize`.
- The registration lines and the `mobs.md` row go in the group files only.

**Hooks (D0-2.2).**

| Hook | Value |
|---|---|
| `KEY` | 84 |
| `MOUNT` | true |
| `category` | `"creature"` |
| `hurt` | `hurt_plain` with the voices at pitch 1.0 |
| `death_drops` | the harness: guaranteed, or the 0.085 roll |
| `breathes_water` | baby |
| `leashable` | true (`leash_walk` empty: `shouldStayCloseToLeashHolder` false) |
| `panicking` | a ghastling in PANIC |
| `voice` | `("ghastling", 1.0, "neutral_volume")` or `("happy_ghast", 4.0, "neutral_volume")` |
| `seats` | 4 |
| `passenger_point(seat)` | the seat turned by −yaw (as the horse seat, `riding.rs:227-256`), ×0.2375 for a baby |
| `controller` | the first `Rider::Player` when harnessed, adult and not still |
| `vehicle_attachment` | −0.5 |

- Defaults stand for `experience`, `despawn`, `dismount` and `rider_jumps`.
- Each tick, `core.ambient_interval` = 120, or 720 with riders (`passengers_of`, `riding.rs:178`).
- `body.gravity = 0` (as `ghast.rs:29`).
- Vanilla has no steps. `step_sound` cannot say "silent" (`None` means the block's step), so after `family_after_move` the tick drops the `Voice::Step` voices the move pushed into `core.voices` (`world/emissions.rs:65`), as the copper golem rewrites its own.

**Tick, in vanilla order.**
1. `family_base_tick`.
2. The still countdown and the player-above scan (skip players whose `root_vehicle`, `riding.rs:718`, is a happy ghast).
3. The heal (`rain_at`; clouds at 192.33 in the Overworld only, read from `dimension`), then `Age::tick`.
4. With a controlling player, the ridden step.
5. Otherwise: a baby runs its brain; an adult runs `MonsterAi::tick_goals`, as `GhastEntity::tick_ai` does (not `family_goal_step`, which runs ground travel). Then the restriction, the careful move control, the look control, and `travel_flying(0, 0.0833)`.
6. `family_after_move`.
7. On an age boundary, swap the size, the eye height and the setup.

**Adult goals.**
- `MonsterKind::Registered` with a `MonsterSpec` with three goal names, no targets, eye 2.6, range 16.
- `Goal<MonsterGoalContext>` impls in the family file, sharing a `HappyGhastGoals` in `ctx.extra`: the wanted spot, the operation, the tempting players and the home.
- The tempting players come from `world.player_controls(p).holds(..)`, within range 16, with a calm-down of 50. javap `TemptGoal.canUse` for its targeting during the work.
- `moving.rs`: the careful reach is `ghast::can_reach` plus the inflated-box scan, with `block_in_tag(.., "minecraft:happy_ghast_avoids")` and the fluid rules.
- The full `getSuitableFlyToPosition(16)`. The ghast keeps its own `suitable_fly_to`.

**One-line hooks into shared files** (logic in the new files, each next to a related arm):
- `world/src/level/random_tick.rs`: `} else if class("DriedGhastBlock") { self.dried_ghast_random_tick(state, pos); }` after the `NetherPortalBlock` arm (`:323`), the other mob-hatching random tick.
- `world/src/level/mod.rs` `tick_block`: `} else if self.is_a(state, "DriedGhastBlock") { self.dried_ghast_tick(state, pos); }` after the `FrostedIceBlock` arm (`:850`).
  - The ghastling goes out through `self.spawned.push(tag)`, as `level/spawning.rs:564` does. The tag is `{id, Pos, Rotation: [yaw, 0f], Age: -24000}`, built directly since `BREEDING` skips `finalizeSpawn`.
  - It uses `has_block_tick_at` (`mod.rs:769`) and `schedule_block_tick` (`:752`).
- `crates/minecraft_terrain/src/display_ticks.rs`: `"dried_ghast" => crate::families::night::happy_ghasts::dried_ghast::animate(block, pos, random, out),` after the `campfire` arm (`:80`).
- `ambient_particles.rs`: `WhiteSmoke` after `Smoke` (`:43`), with its arms next to `Smoke`'s (javap `WhiteSmokeParticle`). Use E1's if E1 added one.
- `crates/minecraft_survival/src/actions.rs`: after the place-sound push (`:1346`), `effects.extend(..dried_ghast::placed_sound(..))`.
- `player/src/placement.rs`: a `"dried_ghast"` arm next to `"ender_chest"` (`:67`). Only if a test shows the generic path misses the facing or waterlogging.
- W's `game_event` calls, if W merged first: `block_change` after hydration changes, `shear`, `equip`.

**Multiplayer.**
- `FamilyShow`: flags bit 0 `RIDDEN` (goggles down), bit 1 `STILL`; `values[0]` = the harness colour by `DyeColor` id (white 0 … black 15), or −1.
- `VOLATILE`: `still_timeout`, `home_pos`, `home_radius`, `equipment`, `drop_chances`.
- `TRACKING_RANGE` 10.
- `read` and `fields`:
  - those keys, with `equipment {body: {id, count 1}}`;
  - `Age` and `ForcedAge`, as `server_mobs.rs:1536` writes them.
- Seats and a friend's steering go through D0-2.5 and the existing ride sync.

**Keyframes.** None; the tentacles use `tentacle_swing`.

**Render.** As `ghast_render.rs:45-102`: scale 4.0, or 0.95 for a baby. The harness layer has its own region; the body squeeze is a part scale about the body pivot. Everything goes to `out.models`.
- `TEXTURES`: 2 skins + 16 harnesses.
- `SHADOW_RADIUS` 2.0.

**Root hooks the family needs** (each its own small commit named for the hook; D0-12):
- `Family::voice_pitch(mob) -> Option<f32>` (default `None`, a random pitch) for the base tick's ambient roll, placed before `voice` (M30).
- No shadow hook: the ghastling's shadow (2.0 × 0.2375 = 0.475) goes through M1's `TerrainFamily::shadow_radius(mob)` (M1 `armadillos-drawn`, M30).

**COMMIT GROUPS**

| # | Group key | Needs | Commits |
|---|---|---|---|
| 1 | `happy-ghasts-live` | d0a | 1. Registered Minecraft mobs can speak at a fixed pitch. 2. Minecraft happy ghasts float about near the ground like vanilla. 3. Minecraft happy ghasts are saved, hatch from eggs and are drawn for host and friends. |
| 2 | `happy-ghasts-riding` | group 1 | 1. Minecraft happy ghasts wear harnesses like vanilla. 2. Minecraft happy ghasts carry four riders and fly where the first one looks. |
| 3 | `happy-ghasts-ghastling` | group 1; d0b for commit 2 | 1. Minecraft dried ghasts soak in water and hatch ghastlings like vanilla. 2. Minecraft ghastlings follow players, flee when hurt and grow up on snowballs. |

What each commit covers:
- **Group 1, commit 2**: the entity, adult goals, the careful move and look controls, the still timeout, the home, healing, age and the spawn row.
- **Group 1, commit 3**: the terrain registration, `show`, saving, and the adult and ghastling body.
- **Group 2, commit 1**: equipping, shears, the drop, the harness layer and goggles, the squeeze, the sounds, and the harness in `FamilyShow`.
- **Group 2, commit 2**: the `interact` mount, seats, controller, ridden input and slow turn, the goggle sounds and pose, ambient ×6.
- **Group 3, commit 1**: the block ticks, placing and its sounds, the display ticks, white smoke.
- **Group 3, commit 2**: the ghastling's shadow through M1's `shadow_radius` if it is merged, and the brain on D0's `FlyingMoveControl(180, true)` and `FlyingPathNavigation` (path length 48, can float, no doors), with M3's `FlyingMoveSink` and `air_and_water_random_position_within`, and M1's `AnimalPanic` (M30), feeding and the dandelion. Until it lands a ghastling hovers in place.

### Tests
- **Group 1, commit 1 (the hook; skipped with the commit if the hook is already there).** `families::tests::a_registered_family_with_a_fixed_pitch_speaks_at_it` (probe: `voice_pitch` `Some(1.0)` gives pitch 1.0 on the ambient, hurt and death voices and draws nothing from the mob's random; the default `None` keeps D0's random pitch).
- **Group 1, commit 2.**
  - `a_happy_ghast_is_four_blocks_with_twenty_health_and_its_kind`: size, eye, health, kind, census, candidates, empty loot, 1-3 experience. A ghastling is 0.95 with 0 experience.
  - `it_floats_to_spots_near_blocks_inside_its_home`: 64 bare, 32 harnessed, the heightmap drop.
  - `its_careful_move_refuses_cactus_fire_and_water_ahead`
  - `a_snowball_draws_it_to_seven_blocks_and_a_harness_only_while_bare`
  - `a_player_above_it_holds_it_still_and_square_after_the_load_grace`
  - `it_heals_every_second_in_rain_or_clouds_and_every_thirty_seconds_otherwise`
  - `an_adult_drowns_and_a_ghastling_does_not_and_neither_takes_fall_damage`
  - `its_voice_is_four_times_a_ghastlings_at_a_steady_pitch`
  - World crate: `a_happy_ghast_row_is_a_four_block_creature_that_never_spawns_naturally`
- **Group 1, commit 3.**
  - `a_happy_ghast_saves_and_loads_with_its_still_timeout_home_and_harness`
  - `a_friend_sees_a_happy_ghast_from_its_look_tag`
  - `a_happy_ghast_is_a_cube_over_nine_tentacles_at_scale_four`
  - `a_ghastling_adds_its_inner_body_at_0_95`
  - `its_textures_are_in_the_atlas`
- **Group 2, commit 1.**
  - `a_harness_goes_on_an_adult_once_and_always_drops`
  - `shears_cut_the_harness_off_only_without_riders`
  - `a_harnessed_ghast_shrinks_inside_its_harness_with_goggles_up`
- **Group 2, commit 2.**
  - `four_players_ride_it_at_the_four_vanilla_seats_turned_with_it`
  - `only_a_harnessed_ghast_answers_its_first_player_rider`
  - `the_rider_flies_it_at_3_3_blocks_a_second_and_climbs_with_jump`
  - `it_turns_eight_percent_of_the_way_to_the_riders_view_each_tick`
  - `a_rider_getting_off_lands_on_top_and_holds_it_still_for_ten_ticks`
  - `using_a_harnessed_ghast_mounts_and_crouching_does_not`
  - `a_friend_sees_the_riders_and_the_lowered_goggles`
- **Group 3, commit 1.**
  - `a_waterlogged_dried_ghast_gains_a_level_each_5000_ticks_and_hatches_at_three`
  - `a_dry_dried_ghast_loses_a_level_each_5000_ticks`
  - `placing_a_dried_ghast_in_water_waterlogs_it_and_faces_the_player`
  - `a_dried_ghast_smokes_on_soul_sand_and_sparkles_in_water`
- **Group 3, commit 2, with M1's `shadow_radius`.** `a_ghastlings_shadow_is_0_475`
- **Group 3, commit 2.**
  - `a_ghastling_follows_the_nearest_player_within_sixteen`
  - `a_snowball_draws_a_ghastling_at_1_25`
  - `a_hurt_ghastling_panics_100_to_120_ticks_at_double_speed`
  - `a_snowball_takes_a_tenth_off_a_ghastlings_wait_and_a_dandelion_locks_it`
  - `at_24000_ticks_it_becomes_a_four_block_happy_ghast_with_goals`

### Owner's steps (survival, Normal, Overworld, flat open ground by day)
1. Use a happy ghast egg on the ground. A big white cube with a calm face and nine swaying tentacles rises and drifts slowly near the ground. **m3h1_happy_ghast_egg**
2. Hold a snowball 15 blocks away. It comes and stops about 7 blocks off. Switch to the sword: it drifts away. The white harness in hand draws it too.
3. Use the white harness on it. An equip sound plays and a white frame appears with goggles raised on its forehead; its body sits slightly smaller inside. A second harness does nothing. **m3h3_harness_on**
4. Lure it low with a snowball and use it with an empty hand (not crouching). You sit on top and the goggles drop with a sound.
   - W flies where you look (up climbs, down sinks). S backs off at half speed, A and D strafe, Jump rises.
   - It turns slowly after your view, at about 3.3 blocks a second. **m3h4_flying**
5. Fly close to the ground and crouch. You land on its top and drop off (this fork cannot stand on mobs), and the goggles rise with a sound.
6. Friend window: while you sit in front, the friend uses it and takes the next of the four seats (front, two sides, back). Only the front rider steers. Get off: the friend becomes the front rider and steers. **m3h6-host**, **m3h6-friend**
7. With nobody aboard, use shears on it: the harness drops on its top and the shears lose 1. With a rider aboard, shears do nothing.
8. Hit it twice with the sword (it does not flee) and run `/weather rain`: it heals a heart about every 2 s. After `/weather clear`, it heals half a heart every 30 s.
9. Place a dried ghast on soul sand: a shrivelled ghast block faces you and puffs white smoke. Place another in a water pool, or pour a water bucket on it: a splash sound and green sparkles. **m3h9_dried_ghast**
10. Run `/tick sprint 2d`. The wet one swells through three looks, then a 1-block ghastling pops out, facing the way the block faced. The dry one stays as it was. **m3h10_ghastling_hatch**
11. The ghastling follows you, faster when you hold a snowball. Each snowball you feed it takes a tenth off its growing time. It does not drown, a harness will not go on, and a sword hit sends it fleeing for about 5 s.
12. Run `/tick sprint 1d`: it grows into a full happy ghast. **m3h12_ghastling_grown**
13. Save and quit, then load. The harness, the ghastling's age and their places are kept.
14. Kill a harnessed ghast: it drops its harness and 1-3 experience, nothing else.
15. In creative, use a happy ghast egg on a spawner: a small happy ghast turns in the cage. Run `/gamemode survival` afterwards.

```
IW4L_MINECRAFT_KIT=minecraft:happy_ghast_spawn_egg*4,minecraft:dried_ghast*4,minecraft:water_bucket,minecraft:water_bucket,minecraft:white_harness,minecraft:red_harness,minecraft:blue_harness,minecraft:snowball*16,minecraft:shears,minecraft:golden_dandelion*2,minecraft:lead*4,minecraft:soul_sand*8,minecraft:ghast_tear*8,minecraft:leather*3,minecraft:glass*2,minecraft:white_wool,minecraft:red_dye,minecraft:crafting_table,minecraft:spawner,minecraft:cobblestone*64,minecraft:scaffolding*64,minecraft:diamond_sword,minecraft:cooked_beef*16
```
(23 stacks. The tears, soul sand, leather, glass, wool and dye are for the dried ghast and harness recipes.)

### Depends on
- **d0a** (registry, mount hooks D0-2.5, spawn rows, saving, friends, draw): groups 1-2.
- **d0b** (P3 `flying.rs`): group 3 commit 2 only.
- **W (P7)**: the optional `game_event` lines.
- **E1**: no `puff_for` rows; a shared white smoke kind if E1 adds one; both edit `ambient_particles.rs`.
- **M3-9 `allays-live`** (earlier in segment m3f): `FlyingMoveSink` and `stroll::air_and_water_random_position_within` (`RandomStroll.fly`'s spot picker). **M1 `goats-live`:** `AnimalPanic` in `animal_brain.rs`, or a private one until it is merged. **M1 `armadillos-drawn`:** `TerrainFamily::shadow_radius` (M30).
- **M3-7 `camels-riding`**: the per-seat `ride_seat` and forward seats, so each rider sits at its own seat and a friend's view follows it.

### Decisions for the track log
1. **Getting off in mid-air.** Keep vanilla's dismount spot on top of the ghast. The MW2 player collides with blocks only, so in this fork the rider then falls. Standing on the ghast is left for later, and step 5 tells riders to get off near the ground.
2. **Voice pitch.** `Family::voice_pitch` (default `None`) in D0's root file, before `voice` (M30). Vanilla always uses pitch 1.0 with no random draw, but D0's base tick gives every ambient sound a random pitch.
3. **Ghastling shadow.** Through M1's `TerrainFamily::shadow_radius(mob)`: 2.0 × 0.2375 = 0.475 as in vanilla, instead of D0's halved 1.0. Without M1's hook it stays 1.0 (left for later).
4. **Leash lengths** stay the fork's fixed 6 and 12 (five places in `leashes.rs`, E2's file this round); vanilla's 10 and 16 for this mob are left for later.
5. **Healing in snow.** Use a snow query if one exists, otherwise rain only, logged as a known gap.
6. **Saving the harness** as `equipment.body`, plus `drop_chances.body` 2.0 when a player put it on, which matches vanilla's guaranteed drop. Writing these fields replaces the whole saved `equipment` compound, which is safe because vanilla happy ghasts carry nothing else.
7. **The adult's goals** run through `MonsterAi::tick_goals`, as the ghast's do, not through `family_goal_step`, which applies ground movement. Its careful move control stays in the family's files, so `monster_ai.rs` is not edited.

---

## M3-9. Allays (groups `allays-live`, `allays-drawn`, `allays-notes`, `allays-dance`)

Registry group `night`, family `allays`, kind `minecraft:allay`, hit-key byte **81**, row 45 of `docs/round10/mobs.md`.

- Every group needs D0 d0a. `allays-live` and `allays-drawn` run in segment m3f, after d0b (P3).
- `allays-notes` and `allays-dance` run in segment m3g, after W's w2 (game events, P7, and W4's per-entity listener). `allays-dance` is optional and last (decision 1).
- Paths are as of `minecraft-survival` `d1a3d028`. Re-check every line before editing.

### What exists today

- **No allay in `crates/`.** An egg or `/summon` leaves a dormant tag.
- **Generated allays are dormant tags.**
  - Mansions place 1-3 per "Group of Allays" (`generator/src/structure/kinds/woodland_mansion.rs:918`).
  - Outposts place them through the `feature_cage_with_allays` template (`generator/src/feature/entities.rs:191` `place_template_entity`, finalize `:117`).
- **Model to copy: the vex** (`crates/minecraft_terrain/src/vex_render.rs`):
  - nearly the same cubes (`:25-31`), `model_root` (`:97`), `hand_pose` (`:127`), `append_vexes` (`:137`);
  - full block light (`BLOCK_LIGHT` 15, `:16`) and the overlay marks (`cow_render.rs:336`).
  - Vexes draw into `out.models` (`minecraft_survival/src/entity_view.rs:1083`). The allay is translucent.
- **Brain pieces.**
  - `hoglin_brain.rs:188` is the smallest brain: sensor clocks (`SCAN_RATE` 20, `:18`, first tick `:206`), `MoveSink`, `Ticker`.
  - `brain_behaviors.rs` `Ticker` (`:149`) and `MoveSink` (`:183`). `MoveSink` drives `GroundNavigation` only (`:171`).
  - Private villager ports to copy: `Swim(0.8)` (`villager_brain/behaviors.rs:312`, threshold 0 below eye height 0.4), `LookAtTargetSink` (`:364`), `GoToWantedItem` (`:1724`).
- **Items in the world.**
  - The `World` trait has `items_in`/`item`/`take_item`/`spawn_item` (`player/src/lib.rs:105-117`, `WorldItem` `:86`), served at `server_mobs.rs:169-186`.
  - The piglin scan `world/piglins.rs:766` is vanilla's `NearestItemSensor`: 32/16 box, nearest first, `sight::line_of_sight` (`sight.rs:157`).
  - Throws: `piglin_throw` (`:862`), `PICKUP_DELAY` 10 (`:15`).
- **Movement.**
  - `Body::travel_flying` (`entities/src/movement.rs:441`): no gravity, no fall distance, drag 0.91, 0.8 in water and 0.5 in lava; one speed.
  - `stroll::land_random_position` (`stroll.rs:119`) gives the panic spot. `air_and_water_random_position` (`:315`) is fixed at 8/4; the allay needs 10/7.
- **Rules.**
  - `feline_ai::panic_cause` (`feline_ai.rs:103`) is `#panic_causes`.
  - `EntityWorld::uuid_of` (`world.rs:4033`; players at `PLAYER_TARGET + id`, `:4022`); `dimension` (`world.rs:2515`, `piglins.rs:613`).
- **Saving.**
  - `Brain.memories` in vanilla NBT: `piglin_save.rs:71` `read_memories` and `:115` `brain_tag` (value and ttl, UUID int arrays, `GlobalPos`).
  - Stacks: `piglin_save::stack_of` (`:39`) and `item_nbt::stack_tag` (`item_nbt.rs:238`). `equipment` and `Inventory` shapes: `piglin_save.rs:165-232`.
- **Interaction.**
  - `mob_actions.rs:149` `interact`; the `wolf_interact` pattern at `:348`.
  - `Inventory::add_item` (`player/src/inventory.rs:958`), `animal::consume_one` (`animal.rs:62`), `recipes.item_in_tag` (`player/src/crafting.rs:870`).
- **Leads.** The elastic pull applies to every leashed mob (`world/leashes.rs:808-830`); walking to the holder goes only through `leash_walk` (`:837`). The rope offsets are `leash_render.rs:72`.
- **Client.** `HeldItem` (`mesh.rs:1352`), `mth_cos` (`client_mobs.rs:1389`). The client walk is horizontal (`client_mobs.rs:685-691`), which is vanilla's for the allay (`omnidirectionalAirMover` false).
- **Listening.**
  - Note blocks play (`world/src/level/redstone.rs:1087` `play_note`) but post no game event (W3 adds `note_block_play`).
  - Jukebox discs are not simulated (`docs/MINECRAFT-SURVIVAL.md:351`), and no round 10 brief adds them.

### Vanilla 26.3 (javap this round)

**Type and entity**
- **`EntityTypes.ALLAY`:** `CREATURE`, `sized(0.35, 0.6)`, `eyeHeight(0.36)`, `ridingOffset(0.04)` (vehicle attachment −0.04), `clientTrackingRange(8)`, `updateInterval(2)`. Not fire immune, allowed in peaceful.
- **No spawning:** no `SpawnPlacements` entry, and none of the 68 biome files names it.
- **`Allay extends PathfinderMob implements InventoryCarrier, VibrationSystem`.** It is no `Animal`, so `xpReward` is 0: **no experience**.
- **`createAttributes`:** follow range 16, health 20, flying speed 0.1, movement speed 0.1, attack 2.
- **Controls:** `FlyingMoveControl(this, 20, true)`. `FlyingPathNavigation` with `setCanOpenDoors(false)`, `setCanFloat(true)` and `setRequiredPathLength(48)`.
- **`travel` = `travelFlying(input, getSpeed())`:** `moveRelative(0.02)` in water and lava, `getSpeed()` in the air; no gravity **even while dying**, so a dead allay hovers.
- **No falls or steps:** `checkFallDamage` and `playStepSound` are empty. `isFlapping` = `!onGround`.
- **Inventory:** `SimpleContainer(1)`. Synced `DATA_DANCING` (false) and `DATA_CAN_DUPLICATE` (true).
- **Saved:**
  - `Inventory`, `listener` (`VibrationSystem$Data`);
  - `DuplicationCooldown` (written as a long, read with `getIntOr`);
  - `Brain.memories` `minecraft:liked_player` (UUID), `minecraft:liked_noteblock` (`GlobalPos`), `minecraft:liked_noteblock_cooldown_ticks` and `minecraft:item_pickup_cooldown_ticks` (int);
  - `Mob`'s `equipment.mainhand` and `LeftHanded`.
- **Other overrides:** `removeWhenFarAway` false; `shouldStayCloseToLeashHolder` false; leash offset `(0, eye·0.6, width·0.1)`; `canDispenserEquipIntoSlot` false.

**Brain (`AllayAi`)**
- **Sensors:** `NEAREST_LIVING_ENTITIES`, `NEAREST_PLAYERS`, `HURT_BY`, `NEAREST_ITEMS`. `NearestItemSensor` takes the box grown 32/16/32, sorts by distance and keeps `wantsToPickUp`, within 32, in line of sight.
- **Activities:** CORE and IDLE only.
- **CORE**, in priority order:
  1. `Swim(0.8)`;
  2. `AnimalPanic(2.5)`: `#panic_causes`, `LandRandomPos(5, 4)`, 100-120 ticks, water when burning;
  3. `LookAtTargetSink(45, 90)`;
  4. `MoveToTargetSink()` (150-250);
  5. `CountDownCooldownTicks` for `LIKED_NOTEBLOCK_COOLDOWN_TICKS` and for `ITEM_PICKUP_COOLDOWN_TICKS`: minus 1 a tick while above 0, erased on stop.
- **IDLE**, in priority order:
  1. **`GoToWantedItem(always, 1.75, true, 32)`.** It needs the wanted item, no pickup cooldown, within 32, inside the world border, and `canPickUpLoot`. It sets the look target (the item's eyes) and the walk target (the item, 1.75, close enough 0).
  2. **`GoAndGiveItemsToTarget(getItemDepositPosition, 2.25, 20, throwItem, ITEM_PICKUP_COOLDOWN_TICKS, 60, hasItemToThrow)`.** A 20-tick `Behavior` that runs while the inventory is not empty and a target exists.
     - `start`: walk and look target, speed 2.25, close enough 3.
     - Each `tick` with the target within 3 of its eyes: `throwItem` and the pickup cooldown 60. That is **one item per tick**.
  3. **`StayCloseToTarget(getItemDepositPosition, not(hasWantedItem), 4, 16, 2.25)`:** beyond 16, look at the target and walk to it at 2.25, close enough 4.
  4. **`SetEntityLookTargetSometimes(6, 30-60)`.**
  5. **`RunOne`:** `RandomStroll.fly(1.0)` (weight 2: `AirAndWaterRandomPos(10, 7, −2, view, π/2)`), `SetWalkTargetFromLookTarget(1.0, 3)` (2), `DoNothing(30, 60)` (1).
- **`getItemDepositPosition`:**
  - the block above the liked note block, if it is within 1024 in this dimension, still a note block, and its cooldown is present;
  - otherwise the liked note block is erased and the target is the liked player (`EntityTracker`, eyes).
- **`getLikedPlayer`:** the UUID in this level, survival or creative, within 64.
- **`throwItem`:**
  1. Remove one item from the slot.
  2. `BehaviorUtils.throwItem(allay, stack, target + (0,1,0), (0.2, 0.3, 0.2), 0.2)`:
     - the item spawns at `(x, eyeY − 0.2, z)`;
     - its constructor draws two level-random doubles, then they are overwritten;
     - velocity `normalize(target' − position) × (0.2, 0.3, 0.2)`, thrower the allay, pickup delay 10.
  3. If `gameTime % 7 == 0` and `level.random.nextDouble() < 0.9`: `entity.allay.item_thrown`, volume 1, pitch `THROW_SOUND_PITCHES[nextInt(16)]` (0.5625, 0.625, 0.75, 0.9375, 1, 1, 1.125, 1.25, 1.5, 1.875, 2, 2.25, 2.5, 3, 3.75, 4).

**Picking up (`Mob.aiStep` after travel)**
- **When:** `canPickUpLoot` (no pickup cooldown and something in hand), alive, and `mob_griefing`.
- **Which items:** in its box grown by (1, 1, 1), not removed, not empty, no pickup delay, and `wantsToPickUp`.
- **`wantsToPickUp`:** hand not empty, `mob_griefing`, `canAddItem`, `isSameItem` (the item only) and equal `potion_contents`.
- **`InventoryCarrier.pickUpItem`:** add, `take` (the flight packet), then shrink or discard.

**`mobInteract`, in order**
1. **Dancing, a `#duplicates_allays` item (`amethyst_shard`) and `canDuplicate`:**
   - `duplicateAllay`: a persistent copy at its position, no finalize, both cooldowns 6000;
   - entity event **18**, `block.amethyst_block.chime` (2.0, 1.0, neutral);
   - one item used (not in creative).
2. **The allay's hand empty and the player's stack not:** it takes a copy of one item and one is used; `entity.allay.item_given` (2.0, 1.0); `LIKED_PLAYER` is the player's UUID.
3. **The allay holding something, the main hand used and empty:**
   - its hand clears, `entity.allay.item_taken` (2.0), and a swing;
   - every inventory stack is thrown at its own position (eyeY − 0.3, no velocity);
   - `LIKED_PLAYER` is erased;
   - `player.addItem(held)`: with a full inventory the item is lost.
4. **Otherwise:** pass.

**Other rules**
- **`hurtServer`:** a source whose entity is the liked player (hit, arrow, bullet) does nothing.
- **`aiStep`:** heal 1 when `tickCount % 10 == 0`; while dancing, every 20 ticks, stop if `shouldStopDancing`; the duplication cooldown −1 (`CAN_DUPLICATE` = cooldown 0).
- **Server `tick`:** the vibration ticker, then a panicking allay stops dancing. `setDancing(true)` is refused while panicking.
- **Client `tick`:**
  - holding ticks ±1 (0-5) by `hasItemInHand`;
  - while dancing: dancing ticks +1, then spinning ticks ±1 (0-15) by `isSpinning = dancingTicks % 55 < 15`;
  - not dancing: all 0.
- **Sounds:** ambient `entity.allay.ambient_with_item` or `ambient_without_item` (interval 80), `entity.allay.hurt` and `.death`; `getSoundVolume` 0.4, neutral; no step.
- **Death:** `entities/allay` has no pools. `dropEquipment` drops the inventory and the hand (not with Curse of Vanishing), even with `mob_drops` off.
- **`handleEntityEvent(18)`:** three hearts. Each draws three `gaussian·0.02`, then `getRandomX(1)`, `getRandomY() + 0.5`, `getRandomZ(1)`.

**Note blocks (`Allay$VibrationUser`, W)**
- **Listener:** radius 16 at the eye, tag `#allay_can_listen` (only `note_block_play`).
- **`canReceiveVibration`:** not no-AI; with no liked note block, yes; otherwise only from that block, within 1024.
- **On receive, `AllayAi.hearNoteblock`:** with none liked, like this block, cooldown 600. The same block again: cooldown 600.
- **`flap`:** `Entity.processFlappingMovement` posts it each step-distance over air while `isFlapping`.

**Dancing (`Allay$JukeboxListener`, radius 10)**
- **`jukebox_play`:** `setJukeboxPlaying(pos, true)`. If the allay is not dancing, it remembers the jukebox and dances.
- **`jukebox_stop_play`:** for its jukebox (or none), it forgets the jukebox and stops.
- **`shouldStopDancing`:** no jukebox, the jukebox centre not within 10 of its feet, or the block not a jukebox.
- **`JukeboxSongPlayer`:**
  - `play`: level event 1010 with the song id;
  - `tick`: stop at `ceil(length·20) + 20` ticks; every 20 ticks `jukebox_play` and a note particle at the bottom centre + (0, 1.2, 0);
  - `stop`: `jukebox_stop_play` and level event 1011.

**Client (`AllayRenderer`, `AllayModel`)**
- **Renderer:** shadow 0.4, `textures/entity/allay/allay.png`, `entityTranslucent`, `getBlockLightLevel` 15, `ItemInHandLayer`. No `*Animation` class, so no keyframes.
- **`createBodyLayer`** (32×32):

  | Part | Parent | UV | Box | Pose |
  |---|---|---|---|---|
  | root | | | | (0, 23.5, 0) |
  | head | root | 0,0 | (−2.5,−5,−2.5, 5,5,5) | (0, −3.99, 0) |
  | body | root | 0,10 + 0,16 | (−1.5,0,−1, 3,4,2) + (−1.5,0,−1, 3,5,2) at −0.2 | (0, −4, 0) |
  | right_arm | body | 23,0 | (−0.75,−0.5,−1, 1,4,2) at −0.01 | (−1.75, 0.5, 0) |
  | left_arm | body | 23,6 | (−0.25,−0.5,−1, 1,4,2) at −0.01 | (1.75, 0.5, 0) |
  | right/left_wing | body | 16,14 | (0,1,0, 0,5,8), not mirrored | (∓0.5, 0, 0.6) |

- **`setupAnim`.** `f` walk speed, `g` walk position, `a` age; `cos` is `Mth.cos` unless marked.
  - **Shared values:**
    - `h = a·20° + g`; `i = cos(h)·π·0.15 + f`;
    - `j = a·9°`; `k = min(f/0.3, 1)`; `l = 1 − k`;
    - `m = lerp(partial, holding0, holding)/5`.
  - **Dancing**, with `n = a·8° + f` and `p = lerp(spin0, spin)/15`:
    - root `yRot = 4π·p` while spinning, else unchanged;
    - root `zRot = cos(n)·16°·(1−p)`;
    - head `yRot = cos(n)·30°·(1−p)` and `zRot = cos(n)·14°·(1−p)`.
  - **Not dancing:** the head follows the look.
  - **Always:**
    - wings `xRot = 0.43633232·(1−k)`; `yRot` right `−π/4 + i`, left `π/4 − i`;
    - body `xRot = k·π/4`;
    - root `y += Math.cos(j)·0.25·l` (**java `Math.cos`**);
    - arms `xRot = m·lerp(k, −1.0471976, −1.134464)`;
    - `zRot = ±(0.43633232 − cos(j + 3π/2)·π·0.075·l·(1−m))` (left negative);
    - `yRot = ±0.27925268·m` (right positive).
- **`translateToHand`:** root, body, translate (0, 1/16, 3/16), rotate X by the right arm's `xRot`, scale 0.7, translate (1/16, 0, 0). Then `ItemInHandLayer`: X −90°, Y 180°, translate (±1/16, 0.125, −0.625).
  - A left-handed allay (5%, `Mob.finalizeSpawn`) uses `THIRD_PERSON_LEFT_HAND` at the same spot.

**Data:** `#allay_can_listen` = `note_block_play`; `#duplicates_allays` = `amethyst_shard`; sounds `entity.allay.{ambient_with_item, ambient_without_item, death, hurt, item_given, item_taken, item_thrown}`.

### Plan

**Files (D0-3)**
- **Entities:** `third_party/minecraftoss/entities/src/world/families/night/allays.rs`:
  - `pub struct Allay`:
    - `brain`, plus d0b's `navigation` and `move_control` (`FlyingMoveControl { max_turn: 20, hovers_in_place: true }`);
    - `hand` and `inventory: Option<ItemStack>`, `left_handed`;
    - `duplication_cooldown: i64`, `dancing`, `jukebox: Option<Pos>`;
    - `liked_player_id` (resolved each tick from the UUID) and `last_hurt: Option<(&'static str, i32)>` (`HURT_BY`);
    - the client counters `holding`/`holding_o`, `dancing_ticks`, `spinning`/`spinning_o`.
  - `impl Family`; `EntityWorld::allay_interact`, `allay_hears_note_block` and `allay_jukebox`; the tests.
- **Brain:** `night/allays/brain.rs`, `AllayBrain`:
  - the memories, as `Option`/`Slot` (`liked_player: Option<u128>`, `liked_noteblock: Option<GlobalPos>`, the two cooldowns, `is_panicking`, `walk_target`, `look_target`, `nearest_visible_wanted_item`, `nearest_visible_living`, `cant_reach_walk_target_since`);
  - four sensor clocks; CORE and IDLE in vanilla priority order, with `AnimalPanic` from M1's `animal_brain.rs` (M30);
  - M3's `FlyingMoveSink` (`world/families/night/flying_sink.rs`, written here and reused by the ghastling) over the flying navigation (decision 2).
- **Terrain:** `crates/minecraft_terrain/src/families/night/allays.rs` (`TerrainFamily`) and `night/allays/render.rs` (the model).
- **World:** one row in `world/src/natural_spawner/families/night.rs`.

**Registration lines**
- `pub mod allays;` and `allays: allays::Allay,` in `family_group!(night 80..=95 {..})`.
- `mod allays;` (`pub(crate)` from `allays-notes` on, for W's `vibrations.rs`) and `Allay` in `terrain_group!(night {..})`.
- The row `minecraft:allay`:
  - 0.35 × 0.6, `NoRestrictions`, `MotionBlockingNoLeaves`, not fire immune;
  - `Rules::Family { creature: true }`, `rule: None`, `remove_far: false`, cluster 4;
  - `MobCategory::Creature`, follow range 16;
  - `baby_chance: None` (no `AgeableMob`), `finalize: None` (`finalize_mob`: follow-range bonus, left-handed).

**Registry hooks (D0-2.2)**
- **`KEY` and `KINDS`:** `KEY = 81`, `KINDS = ["minecraft:allay"]`. `spawn`: `MobCore::new(kind, pos, 0.35, 0.6, 0.36, 20.0)`.
- **`tick`**, in vanilla order:
  1. `family_base_tick`.
  2. Resolve `liked_player_id` from `players` by `uuid_of(PLAYER_TARGET + id)`.
  3. The brain (sensors every 20, CORE, IDLE).
  4. Navigation, `FlyingMoveControl`, look and jump.
  5. `travel_flying` (speed 0.02 in water or lava).
  6. `play_movement`, with a step's block sound cleared so only the amethyst chime is left.
  7. `family_after_move`.
  8. Pickups through `items_in`/`take_item`.
  9. Heal, the 20-tick dance check, the cooldown.
  10. A panicking allay stops dancing.
  11. The client counters.
- **Overrides.**
  - `category`: `"creature"`.
  - `hurt`: refuse when `hurt.attacker == Some(Target::Player(liked))`; else record `last_hurt` and `hurt_plain`.
  - `experience`: 0.
  - `death_drops`: inventory plus hand, not with Curse of Vanishing.
  - `despawn`: `Despawn { removable: false, ..Despawn::of_category("creature") }`.
  - `leashable`: true. `leash_walk`: nothing (decision 6).
  - `panicking`: from the brain.
  - `voice`: `("allay", 0.4, "neutral_volume")`.
  - `vehicle_attachment`: −0.04.
- **Defaults kept:** undead, fire, `breathes_water` (false, so it can drown), enemy, push, knockback, loot, `monster` (`None`: a brain family).
- **No root hooks.** No track adds an ambient hook this round, and `OWN_DYING_TRAVEL` is M2's (M30). The allay swaps the `Voice::Ambient` that `family_base_tick` pushes for its own `entity.allay.ambient_with_item` or `ambient_without_item` voice, as copper golems and creakings drop theirs. It also sets `body.gravity = 0` at spawn, as the vex does (`vex.rs:68`), so D0's `dying_travel` leaves a dying allay hovering. If M2's `OWN_DYING_TRAVEL` is on `minecraft-survival` when `allays-live` runs, the allay may use it (and `travel_flying` while dying) instead of gravity 0.
- **`TerrainFamily`.**
  - `TRACKING_RANGE` 8.
  - `VOLATILE` `["Inventory", "listener", "DuplicationCooldown"]` (`Brain` is in the shared list).
  - `TEXTURES` `["minecraft:entity/allay/allay"]`. `SHADOW_RADIUS` 0.4. Other constants: the defaults.
  - `read`/`fields`: `equipment.mainhand`, `LeftHanded`, `Inventory`, `DuplicationCooldown` and the four memories (`piglin_save.rs` shapes). `listener` is left in the original tag; from `allays-notes`, W's mechanism writes it.
  - `interact`: calls `allay_interact(id, actor.player, actor.inventory, actor.selected, actor.infinite)`. It pushes `MobSound`s (2.0, neutral) and drops, and returns true unless it passes.

**One-line hooks in shared files (the logic stays in the allay files)**
1. **`entities/src/stroll.rs`:** a new `air_and_water_random_position_within(world, profile, position, view, horizontal, vertical, random)` right after `:315`, which stays untouched (decision 3).
2. **`leash_render.rs:78`:** `"allay" => (0.6, 0.1),` after the `"pig" | "wolf" | "strider"` line.
3. **E1's `mob_particles::puff_for`** (`allays-dance`): `("minecraft:allay", 18) => (Heart, 3, 0.5)` next to the ocelot row. E1's default 18 row gives 7.
4. **W's `crates/minecraft_terrain/src/vibrations.rs`** (`allays-notes`):
   - one line beside W4's warden listeners, `listeners.extend(crate::families::night::allays::vibration_listeners(world));` (radius 16 at the eye, `allay_can_listen`, `allay_can_hear`/`allay_hears_note_block`);
   - in `allays-dance`, one more line for the jukebox listener (radius 10, `allay_jukebox`);
   - the `listener` NBT follows W4's warden mechanism.
5. **The allay's own tick:** `EntityWorld::game_event("minecraft:flap", ..)` in its flapping step, once W is merged.

**Multiplayer**
- **`FamilyShow`:**
  - `flags` bit 0 is `DANCING`;
  - `values[0]` holding ticks, `[1]` dancing ticks, `[2]` spinning ticks;
  - the friend's `show_on` moves the old counters to `holding_o`/`spinning_o` first, so both sides lerp by the partial tick.
- **The held item** comes from the look tag. `equipment` is not volatile, so giving or taking respawns the friend's copy within `LOOK_TICKS`, as for zombies and piglins (decision 5).
- **Everything else is the host's:**
  - liked players are UUIDs, so a friend can be one, and nothing keys on player 0;
  - sounds go through the voices and the outcome;
  - the hearts come from event 18 through E1.
- **No serialized struct changes** (no `## Protocol` entry). **No keyframes.**

**COMMIT GROUPS**

| # | Group key | Commits | Needs |
|---|---|---|---|
| 1 | `allays-live` | 1. "Minecraft allays fly about and collect items like the one they hold" (family, brain, flying controls, pickups, throws, hurt rule, heal, sounds, lead, drops, no despawn, `stroll.rs` function). 2. "Minecraft allays take an item from a player, bring that player what they collect, and are saved and seen by friends" (`allay_interact`, the row, `TerrainFamily` with an empty `draw`). Row 45 becomes `live`. | d0a, d0b |
| 2 | `allays-drawn` | "Minecraft allays are drawn like vanilla" (model, translucent at block light 15, held item, bob, wings, arms, shadow, cage, leash offset). Row 45 becomes `drawn`. | 1 |
| 3 | `allays-notes` | "Minecraft allays drop what they collect at a note block they hear" (listener, liked note block, deposit above it, 600-tick memory, `listener` saved, `flap`). Row 45 becomes `done` (unless group 4 follows). Steps and kit go into the log. | 1, W `vibrations` and `vibrations-more` |
| 4 | `allays-dance` (optional) | 1. "Minecraft jukeboxes play music discs". 2. "Minecraft allays dance to jukeboxes and duplicate with amethyst shards". Row 45 becomes `done`. | 2, 3, E1 `particles-events` |

**Group 4, commit 1.** New vendored `world/src/level/jukebox.rs` (javap `JukeboxBlock` and `JukeboxBlockEntity` during the work):
- **Block entity:** insert and eject; `RecordItem` and `ticks_since_song_started` saved; the comparator output from the 26.3 `jukebox_song` data (`length_in_seconds`, `comparator_output`), read at load.
- **Ticking:** one line in `has_ticker` (`level/container.rs:475`) and one at the use-block site next to note blocks (`redstone.rs` `use_block_facing`).
- **Events:** `jukebox_play`/`jukebox_stop_play` through W's `Level::game_event`; level events 1010 and 1011 to host and friends.
- **Sound:** positional streamed disc music, one arm each in `minecraft_terrain/src/level_events.rs:87`, built on `crates/render_anim/src/minecraft_sounds.rs:146` `music_file`.

### Tests

**`allays-live` 1 (entities):**
- `an_allay_is_0_35_by_0_6_with_twenty_health_and_its_kind` (eye 0.36, `entity_type`, census as a creature, `mob_candidates`, empty table, 0 experience, kept far away and on peaceful)
- `an_allay_hovers_without_gravity_and_never_takes_fall_damage`
- `an_allay_flies_at_a_tenth_and_turns_twenty_degrees` (speeds 0.1, 0.175, 0.225, 0.25)
- `an_allay_heals_one_health_every_ten_ticks`
- `an_allay_holding_an_item_flies_to_a_matching_item_within_32_and_takes_it`. Refused: another item, a pickup delay, `mob_griefing` off, its 60-tick cooldown, a full slot, another potion.
- `an_allay_with_an_empty_hand_collects_nothing`
- `an_allay_throws_one_item_a_tick_within_three_blocks_of_its_player_then_waits_sixty` (spot, velocity, delay 10, the two level draws, the throw sound)
- `an_allay_stays_within_sixteen_blocks_of_its_player`
- `an_allay_forgets_a_player_beyond_64_or_spectating`
- `its_player_cannot_hurt_an_allay_but_others_can_and_it_panics` (melee, arrow, bullet; 2.5 for 100-120 ticks)
- `an_allay_drops_what_it_holds_and_carries_when_it_dies_even_without_mob_drops`
- `a_dying_allay_hovers`
- `an_allay_calls_with_or_without_an_item_at_four_tenths_and_never_steps`
- `a_lead_drags_an_allay_but_it_does_not_fly_to_the_holder`

**`allays-live` 2:**
- **Entities:**
  - `a_player_gives_an_allay_one_item_and_becomes_its_player` (used unless infinite, `item_given`, a second item ignored);
  - `an_empty_hand_takes_the_item_back_and_the_allay_drops_its_load` (`item_taken`, swing, load at the eyes − 0.3, player erased, the item lost when the inventory is full).
- **Terrain:**
  - `an_allay_saves_and_loads_with_its_item_load_player_and_cooldown` (an unknown `listener` kept);
  - `a_friend_sees_an_allay_from_its_look_tag` (volatile keys dropped, hand kept, the values);
  - `a_friend_gives_an_allay_an_item_and_it_brings_the_friend_what_it_collects` (`mob_actions::interact` with player 2: D0-9.4's `interact` test unless an earlier group added it).
- **World:** `the_allay_row_has_its_size_and_category_and_spawns_nowhere` (no baby draw).

**`allays-drawn`:**
- `an_allay_has_eight_cubes_on_a_32_texture_drawn_translucent_at_full_block_light`
- `an_allay_bobs_and_beats_its_wings_while_it_hovers_and_leans_forward_as_it_flies`
- `an_allay_lifts_its_arms_over_five_ticks_when_it_holds_something`
- `an_allay_holds_its_item_in_front_of_its_body` (against a hand-computed matrix)
- `the_friend_draws_the_same_holding_progress`

**`allays-notes`:**
- `an_allay_hears_a_note_block_within_sixteen_and_likes_it_for_600_ticks`
- `an_allay_hears_only_its_liked_note_block`
- `an_allay_drops_its_load_on_top_of_a_note_block_that_keeps_playing`
- `a_broken_or_silent_note_block_sends_the_load_back_to_the_player`
- `wool_all_round_hides_a_note_block_from_an_allay`
- `an_allay_listener_round_trips_its_nbt`
- `a_flying_allay_flaps_for_sensors`

**`allays-dance`:**
- `a_jukebox_plays_a_disc_for_its_length_and_calls_every_second`
- `an_allay_within_ten_blocks_of_a_playing_jukebox_dances_and_stops_when_it_stops_or_leaves`
- `a_dancing_allay_spins_fifteen_ticks_of_every_fifty_five`
- `an_amethyst_shard_duplicates_a_dancing_allay_once_every_6000_ticks`
- `a_panicking_allay_stops_dancing`
- `mob_particles` `an_allay_shows_three_hearts_half_a_block_up`

### Owner's steps

Setup: release build, survival, Normal, by day, flat open ground, `/gamerule mob_griefing true`, the kit below. Steps 12 to 14 and 22 need two windows (`scripts\mp-local.ps1`).

1. Use an allay egg on the ground 4 blocks away. A small blue winged spirit hovers there, bobbing, wings beating, glancing at you. It stays bright in shade. **m3a1_allay**
2. Use an emerald on it: one emerald leaves your stack with a soft chime. It holds the emerald in front of its body with both arms lifted. **m3a2_allay_holds**
3. Sprint 25 blocks away: it follows and settles within about 16 blocks.
4. Drop (Q) eight emeralds one at a time in a line 10 blocks away, and three gold ingots beside them. It flies to each emerald (each vanishes into it) and leaves the gold. Then it comes back and showers the emeralds onto you one after another, with a bright sound now and then. **m3a3_allay_brings**
5. `/gamerule mob_griefing false`, drop an emerald: it stays on the ground. `/gamerule mob_griefing true`: the allay takes it.
6. Punch it, shoot it with the M4A1 and with the bow: no red flash, no knockback.
7. Use an empty hand on it: its emerald goes into your inventory, anything it carried drops under it, and it stops following you.
8. Hand it an emerald, let it collect a few, kill it with the sword: the held and collected emeralds drop, and no experience orbs.
9. Put a lead on a fresh allay and walk off: the lead drags it along.
10. With an allay holding an emerald and following you, save and quit, then load: it still holds it, follows you and brings dropped emeralds.
11. `/gamemode creative`, place a spawner, use an allay egg on it: a tiny allay turns in the cage. `/gamemode survival`.
12. Friend: the friend hands a gold ingot to a fresh allay. Both windows show it holding the gold with its arms lifted. It follows the friend, not the host, and brings the friend's dropped gold to the friend. **m3a4_friend-host**, **m3a4_friend-friend**
13. The host punches the friend's allay: a red flash, and it darts away fast for about 5 s. The friend's punch does nothing.
14. `/time set midnight`: both windows show the allays equally lit, with no jump when the friend's allay gives back its item.

Note blocks (`allays-notes`):

15. Give an allay an emerald. Place a note block on open ground 8 blocks away, with air above, and use it once: the allay turns to it. Drop emeralds near you: it now drops them on top of the note block. Play the note block every 20 s: it keeps doing so. **m3a5_note_block**
16. Play only a second note block 5 blocks from the first: the allay ignores it.
17. Leave both note blocks silent for 40 s: the allay brings its finds to you again.
18. Wall in a third note block with white wool on all sides and play it beside a fresh allay holding an emerald: it does not go there.

Jukebox (`allays-dance`, only if built):

19. Place a jukebox 5 blocks from two allays and use the cat disc on it. The music plays and notes rise. The allays sway and spin on the spot about every 3 s. **m3a6_dance**
20. Use an amethyst shard on a dancing allay: a chime, three hearts and a second allay. A second shard on it right away does nothing. **m3a7_duplicate**
21. Use the jukebox to take the disc out: the music stops, and the dancing stops within a second.
22. Friend: the friend sees the dance, the hearts and the new allay, and hears the disc from the jukebox.

Optional:

23. `/locate structure minecraft:pillager_outpost`, `/tp` there. If the outpost has a dark-oak cage, the allays inside now move.

### Kit

```
IW4L_MINECRAFT_KIT=minecraft:allay_spawn_egg*8,minecraft:emerald*64,minecraft:gold_ingot*64,minecraft:note_block*4,minecraft:white_wool*32,minecraft:lead*2,minecraft:diamond_sword,minecraft:bow,minecraft:arrow*32,minecraft:amethyst_shard*8,minecraft:jukebox,minecraft:music_disc_cat,minecraft:spawner,minecraft:cobblestone*64,minecraft:oak_fence*32,minecraft:torch*32,minecraft:cooked_beef*16
```

17 stacks. Every id was checked against 26.3 `assets/minecraft/items`. The friend uses the same kit.

### Depends on

- **D0 d0a:**
  - the registry, `TerrainFamily`, `FamilyShow`, `families::interact`, the lead and `death_drops` arms, and the rows;
  - the private `EntityWorld` fields (`level_random`, `entity_events`, `mob_griefing`, `dimension`).
- **D0 d0b:** `FlyingMoveControl` and `FlyingPathNavigation`, for `allays-live`.
- **M1 `goats-live`:** `AnimalPanic` in `animal_brain.rs` (M30), or a private copy until it is merged. No ambient or dying-travel hook is needed.
- **W:**
  - `vibrations` (dispatch, occlusion) and `vibrations-more` (`note_block_play` from `redstone.rs:1087`), for `allays-notes`;
  - W4's per-entity listener and its `listener` NBT (segment w2);
  - `jukebox_play` dispatch, for `allays-dance`.
- **E1 `particles-events`:** the hearts row.
- **E2 `ti1-targets-and-hurt`:** snowballs, eggs and tridents hit allays through `hit_boxes`. No allay work.
- **No other M3 group.**

### Open decisions (recommended choice first)

1. **Dancing and duplication.**
   - Build them as the optional last group, with jukebox discs: vanilla duplicates only a dancing allay, and only a playing jukebox makes it dance.
   - Skip the group to "left for later" if the track runs long.
   - Dancing without a jukebox would not be vanilla.
2. **Walking the brain.** Write M3's `FlyingMoveSink` over d0b's flying navigation (the ghastling reuses it):
   - 150-250 ticks, a re-path when the target moved more than 2 blocks;
   - a 0-39 tick wait when stuck;
   - the `default_random_position_towards(10, 7)` fallback.
   - Do not make the shared `brain_behaviors::MoveSink` (hoglins, piglins) generic, unless d0b already did.
3. **Fly stroll at 10/7.** Add a new function beside `stroll.rs:315` and leave the 8/4 one untouched.
4. **Liked-player game mode.** Accept and log the adventure deviation rather than adding a field to the shared `PlayerCandidate`.
5. **The friend's held item.** Use the look tag (`equipment` not volatile, as for zombies and piglins), not an item index in `FamilyShow`.
6. **Leads.** `leash_walk` does nothing (vanilla `shouldStayCloseToLeashHolder` is false): the elastic pull alone drags the allay.
7. **The throw's level-random draws.** Keep the `ItemEntity` constructor's two `nextDouble` draws, so the level random stays in step with vanilla.

---

## M3-groups. Commit groups (track runner)

In run order. Every group updates `docs/round10/track-m3-log.md` and its rows in the M3 section of `docs/round10/mobs.md` in the same commit as the code.

| # | Key | Segment | Commits | Section |
|---|---|---|---|---|
| 1 | `lightning-strikes` | m3a | 1. Minecraft lightning bolts set fires and hurt what they strike. 2. Registered Minecraft mobs can answer lightning their own way. | M3-1 |
| 2 | `lightning-turns` | m3a | Minecraft lightning turns pigs, villagers, creepers and mooshrooms like vanilla. | M3-1 |
| 3 | `lightning-drawn` | m3a | Minecraft lightning bolts flash and thunder for every player. | M3-1 |
| 4 | `lightning-glow` | m3a | 1. Minecraft charged creepers glow with their energy swirl. 2. (optional) Minecraft lightning rods spark when struck. | M3-1 |
| 5 | `phantoms-live` | m3a | 1. Minecraft phantoms circle, swoop and bite like vanilla. 2. Minecraft phantoms burn by day and shy away from cats. | M3-2 |
| 6 | `phantoms-drawn` | m3a | 1. Minecraft phantoms are drawn like vanilla. 2. Minecraft phantoms flap and trail specks from their wings like vanilla. | M3-2 |
| 7 | `phantoms-spawner` | m3a | Phantoms come for Minecraft players who have not slept in three days. | M3-2 |
| 8 | `lightning-weather` | m3b | 1. Minecraft thunderstorms strike lightning, drawn to lightning rods. 2. Minecraft tridents with Channeling call down lightning in thunderstorms. | M3-1 |
| 9 | `creakings-live` | m3b | Minecraft creakings freeze while players look at them. | M3-3 |
| 10 | `creakings-drawn` | m3b | 1. Registered Minecraft mobs can set their own walk animation speed. 2. Minecraft creakings are saved, drawn and seen by friends like vanilla. | M3-3 |
| 11 | `copper-golems-live` | m3b | 1. Minecraft copper golems live in the entity world. 2. Minecraft copper golems are built from a carved pumpkin on a copper block. | M3-4 |
| 12 | `copper-golems-drawn` | m3b | Minecraft copper golems are drawn like vanilla. | M3-4 |
| 13 | `copper-golems-sorting` | m3c | 1. Minecraft copper golems carry items from copper chests to chests. 2. Minecraft copper golems open chest lids like players. | M3-4 |
| 14 | `copper-golems-care` | m3c | 1. Minecraft copper golems are waxed, scraped and sheared like vanilla. 2. Iron golems give copper golems a poppy. | M3-4 |
| 15 | `copper-golems-statues` | m3c | 1. Oxidized Minecraft copper golems turn into statues. 2. Minecraft copper golem statues are drawn, posed and woken with an axe. | M3-4 |
| 16 | `undead-horses-live` | m3d | 1. Minecraft zombie horses and skeleton horses live, save and hatch like vanilla. 2. Minecraft undead horses burn, breathe underwater and heal from harming like vanilla. | M3-5 |
| 17 | `undead-horses-ride` | m3d | 1. Minecraft zombie horses are tamed by riding, eat red mushrooms and wear horse armour. 2. Minecraft skeleton horses obey only once tame and splash about underwater. | M3-5 |
| 18 | `undead-horses-drawn` | m3d | Minecraft zombie horses and skeleton horses are drawn like vanilla. | M3-5 |
| 19 | `skeleton-trap` | m3d | Minecraft skeleton trap horses call down four skeleton horsemen. | M3-6 |
| 20 | `camels-live` | m3e | Minecraft camels and camel husks are saved, loaded and hatched. | M3-7 |
| 21 | `camels-drawn` | m3e | 1. Minecraft camels are drawn with their keyframe animations. 2. Friends see Minecraft camels sit, stand and dash. | M3-7 |
| 22 | `camels-brain` | m3e | 1. Minecraft camels wander, sit down, panic, eat cactus and breed like vanilla. 2. Minecraft camels spawn in deserts and camel husks carry husk jockeys. | M3-7 |
| 23 | `camels-riding` | m3e | 1. Two players can ride a Minecraft camel. 2. Minecraft camels dash with the jump bar and run when their rider sprints. | M3-7 |
| 24 | `happy-ghasts-live` | m3f | 1. Registered Minecraft mobs can speak at a fixed pitch. 2. Minecraft happy ghasts float about near the ground like vanilla. 3. Minecraft happy ghasts are saved, hatch from eggs and are drawn for host and friends. | M3-8 |
| 25 | `happy-ghasts-riding` | m3f | 1. Minecraft happy ghasts wear harnesses like vanilla. 2. Minecraft happy ghasts carry four riders and fly where the first one looks. | M3-8 |
| 26 | `allays-live` | m3f | 1. Minecraft allays fly about and collect items like the one they hold. 2. Minecraft allays take an item from a player, bring that player what they collect, and are saved and seen by friends. | M3-9 |
| 27 | `allays-drawn` | m3f | Minecraft allays are drawn like vanilla. | M3-9 |
| 28 | `happy-ghasts-ghastling` | m3f | 1. Minecraft dried ghasts soak in water and hatch ghastlings like vanilla. 2. Minecraft ghastlings follow players, flee when hurt and grow up on snowballs. | M3-8 |
| 29 | `creakings-heart` | m3g | 1. Minecraft creaking hearts wake at night and grow a creaking. 2. Minecraft creakings die when their heart breaks and the heart bleeds resin when they are hit. | M3-3 |
| 30 | `creakings-crumble` | m3g | Minecraft creakings and their hearts stream and crumble like vanilla. | M3-3 |
| 31 | `allays-notes` | m3g | Minecraft allays drop what they collect at a note block they hear. | M3-9 |
| 32 | `allays-dance` | m3g (optional) | 1. Minecraft jukeboxes play music discs. 2. Minecraft allays dance to jukeboxes and duplicate with amethyst shards. | M3-9 |

### Segments

Each segment is one track-runner run with its own review, clippy and tests. Each can merge on its own for a daily release, and each ends with whole mobs (live and drawn together), so a release never has an invisible mob.

| Segment | Groups | Run spec | Starts after | The owner can then test |
|---|---|---|---|---|
| m3a | 1-7: `lightning-strikes`, `lightning-turns`, `lightning-drawn`, `lightning-glow`, `phantoms-live`, `phantoms-drawn`, `phantoms-spawner` | `runs/m3a.json` | D0 d0a merged | summoned lightning, its conversions and charged creepers; phantoms, and the insomnia spawner |
| m3b | 8-12: `lightning-weather`, `creakings-live`, `creakings-drawn`, `copper-golems-live`, `copper-golems-drawn` | `runs/m3b.json` | m3a | thunderstorms, rods and Channeling; creakings from eggs; copper golems built and drawn |
| m3c | 13-15: `copper-golems-sorting`, `copper-golems-care`, `copper-golems-statues` | `runs/m3c.json` | m3b and E2 e2a (may swap with m3d) | copper golems sorting chests, waxing and scraping, statues |
| m3d | 16-19: `undead-horses-live`, `undead-horses-ride`, `undead-horses-drawn`, `skeleton-trap` | `runs/m3d.json` | m3b and E1 e1a (may swap with m3c) | zombie and skeleton horses, the skeleton trap |
| m3e | 20-23: `camels-live`, `camels-drawn`, `camels-brain`, `camels-riding` | `runs/m3e.json` | m3d | camels and camel husks |
| m3f | 24-28: `happy-ghasts-live`, `happy-ghasts-riding`, `allays-live`, `allays-drawn`, `happy-ghasts-ghastling` | `runs/m3f.json` | m3e and D0 d0b | happy ghasts with four riders, allays, dried ghasts and ghastlings |
| m3g | 29-32: `creakings-heart`, `creakings-crumble`, `allays-notes`, `allays-dance` (optional) | `runs/m3g.json` | m3f and W w2 | creaking hearts, allays at note blocks, jukeboxes and dancing |

---

## Left for later

Collected from the sections. Each item also goes into the track log's "Left for later" when its group ends, and into the state doc wording.

**M3-1. Natural lightning**

- Advancements (the fork has none).
- Vanilla clients time their flashes with their own random; here both views follow the server's `life` and `seed`.
- `hideLightningFlash`.
- A player riding a struck pig: vanilla moves the rider onto the piglin, here the rider is dismounted.
- End crystals (the End has no weather), armour stands, boats.
- Vanilla's exact `getEntitiesOfClass` order for the target pick: census order is used, with the same draws.
- Rod sparks, if the optional commit is skipped.

**M3-2. Phantoms**

- A head item taking the sun (`MobCore` has no equipment).
- Visibility scaling of the 64-block search (the fork's `NearestTargetGoal` has none either).
- The `Silent` tag.
- `/summon` riders (0 seats).
- Drawing phantoms at any distance (`shouldRenderAtSqrDistance`).

**M3-3. Creakings**

- **The carved pumpkin disguise:** `disguised` is always false (`minecraft_survival/src/entities.rs:1514`), for endermen too. Fix both in one commit.
- **The heart's comparator signal:** it needs a level-side analog store fed by terrain, which is shared redstone code.
- **Portal rules:** registered mobs do not travel through portals yet.
- **`creakings-crumble` (trails and crumble),** if the segment runs long.

**M3-4. Copper golems**

- The wax, unwax and scrape particles (3003-3005), which copper blocks lack too.
- Dispensers shearing golems and finishing builds.
- Cats blocking golems, unless E2's `cat-chests` (merged before m3c) already put cats into `chest_blocked`; check in `copper-golems-sorting`.
- `CustomHeadLayer`.
- Vanilla's `HashMap` tie order between equally near chests.

**M3-5. Zombie horses and skeleton horses**

- **Spears.** The spear charge (`chargeSpeedModifier` 1.4) and the spear's own attack: spears have no use in the fork, so the rider fights like a sword zombie.
- **Water rules, unported for every horse.**
  - `#dismounts_underwater` and `#can_float_while_ridden`.
  - The skeleton horse's path search keeps `can_float`, because `Cow::walk_profile` is per store.
- **Summoning in peaceful.** `/summon minecraft:zombie_horse` is refused in peaceful because `Level::summon_mob` checks the category, not `isAllowedInPeaceful`. That is a shared world-crate rule.
- **Tame skeleton foals** eating `#horse_food`. Foals come only from `/summon`.
- **Quad-leash offsets** and `onElasticLeashPull` for all horses.

**M3-6. The skeleton trap**

- **Enchantments.** The trap skeletons' `MOB_SPAWN_EQUIPMENT` enchantments; enchanted spawn equipment is unsupported everywhere.

**M3-7. Camels and camel husks**

- The husk's spear charge at x4 from a camel husk: `SpearUseGoal` is a `NeverGoal` (`monster_ai.rs:1950-1951`).
- The pose-dependent leash point (`getLeashOffset`; `leash_render.rs:72` is keyed by kind) and quad leash offsets.
- The baby's happy-villager sparkle when it eats (client-predicted, not an entity event), unless E1 takes it.
- Dismounting under water, and a spawn egg used on an adult making a baby (horses lack both).

**M3-8. Happy ghasts**

- **Standing on a still happy ghast.** The MW2 player collides with blocks only. It needs sim entity colliders; until then step 5 tells riders to get off low.
- **Leash lengths 10 and 16 with `setWait` on a pull.** The fork uses fixed 6 and 12 at five sites in `leashes.rs`. Also the ropes layer and quad leash: only boats use them, and the fork has none.
- **Rider-side extras.** The looping riding sound (there is no looping sound instance), `#dismounts_underwater` (shared with horses, pigs and camels), the `happy_ghast_one_cm` stat, and the rider camera distance.
- **Dispensers equipping harnesses.** Dispensers cannot see mobs.
- **Snow healing.** `rain_at` counts rain only; vanilla also counts snow.
- **Friends seeing feeding sparkles.** Vanilla shows them to the feeder only.
- **The ghastling's 0.475 shadow,** if M1's `shadow_radius` is not merged when `happy-ghasts-ghastling` runs.

**M3-9. Allays**

- **The pickup flight** (`Mob.take`). No mob shows it in the fork, so items vanish into the allay.
- **The held item's light.** It is lit by the world at the eye, not by block light 15, as for vexes (`HeldItem.light` is a position).
- **Adventure players** still count as an allay's player. `PlayerCandidate` (`entities/src/tempt.rs:6`) has no game mode.
- **Advancements** (`allay_deliver_item_to_player`, `allay_deliver_cake_to_note_block`). The fork has none.
- **Dancing, duplication and jukeboxes,** if `allays-dance` is skipped.
- **The dying drift.** A dying allay sinks a little: D0's `dying_travel` uses air drag (0.98 vertically) where vanilla's `travelFlying` uses 0.91 (no root hook, M3-9).

**All sections**

- Game event lines (P7) at M3 sites whose group ran before W's helper was on `minecraft-survival`. W adds them if it merges second (W0 rule 3). Until then, sculk sensors and wardens do not hear these sites. They are:
  - the phantom's and allay's `flap`;
  - the creaking's `entity_action`, `entity_place` and `block_place`;
  - the camel's sit, stand, dash and eat;
  - the copper golem's chest, shear and statue events;
  - the happy ghast's hydration, shear and equip events.
